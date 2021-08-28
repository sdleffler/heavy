use std::collections::HashMap;

use hv_core::{
    engine::{Engine, EngineRef, WeakResourceCache},
    swappable_cache::{CacheRef, Guard, Handle, Loader, SwappableCache},
};
use ordered_float::NotNan;

use crate::{
    graphics::{
        CachedTexture, Color, Drawable, DrawableMut, Graphics, GraphicsLock, GraphicsLockExt,
        Instance, OwnedTexture, SpriteBatch,
    },
    math::*,
};
use {
    hv_core::prelude::*,
    image::{Rgba, RgbaImage},
    std::io::Read,
};

#[derive(Debug, Clone)]
pub struct Font {
    inner: rusttype::Font<'static>,
}

// AsciiSubset refers to the subset of ascii characters which give alphanumeric characters plus symbols
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharacterListType {
    AsciiSubset,
    Ascii,
    ExtendedAscii,
    Cyrillic,
    Thai,
    Vietnamese,
    Chinese,
    Japanese,
}

#[derive(Debug, Clone, Copy)]
struct CharInfo {
    vertical_offset: f32,
    horizontal_offset: f32,
    advance_width: f32,
    uvs: Box2<f32>,
    scale: Vector2<f32>,
    width: f32,
    height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ThresholdFunction {
    Above(NotNan<f32>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontAtlasKey {
    pub path: String,
    pub size: u32,
    pub char_list_type: CharacterListType,
    pub threshold: Option<NotNan<f32>>,
}

impl FontAtlasKey {
    pub fn new<S: Into<String> + ?Sized>(
        path: S,
        size: u32,
        char_list_type: CharacterListType,
    ) -> Self {
        Self {
            path: path.into(),
            size,
            char_list_type,
            threshold: None,
        }
    }

    /// Will panic if the threshold is not a number.
    pub fn with_threshold<S: Into<String> + ?Sized>(
        path: S,
        size: u32,
        char_list_type: CharacterListType,
        threshold: f32,
    ) -> Self {
        Self {
            path: path.into(),
            size,
            char_list_type,
            threshold: Some(NotNan::new(threshold).expect("NaN threshold!")),
        }
    }
}

/// `FontTexture` is a texture generated using the *_character_list functions.
/// It contains a texture representing all of the rasterized characters
/// retrieved from the *_character_list function. `font_map` represents a
/// a mapping between a character and its respective character texture
/// located within `font_texture`.
#[derive(Debug)]
pub struct FontAtlas {
    font_texture: CachedTexture,
    font_map: HashMap<char, CharInfo>,
    line_gap: f32,
}

impl FontAtlas {
    pub(crate) fn from_rusttype_font<F: FnMut(f32) -> f32>(
        ctx: &mut Graphics,
        rusttype_font: &rusttype::Font,
        height_px: f32,
        char_list_type: CharacterListType,
        mut threshold: F,
    ) -> Result<FontAtlas> {
        use rusttype as rt;

        let font_scale = rt::Scale::uniform(height_px);
        let inval_bb = rt::Rect {
            min: rt::Point { x: 0, y: 0 },
            max: rt::Point {
                x: (height_px / 4.0) as i32,
                y: 0,
            },
        };
        const MARGIN: u32 = 2;
        let char_list = Self::get_char_list(char_list_type)?;
        let chars_per_row = ((char_list.len() as f32).sqrt() as u32) + 1;
        let mut glyphs_and_chars = char_list
            .iter()
            .map(|c| {
                (
                    rusttype_font
                        .glyph(*c)
                        .scaled(font_scale)
                        .positioned(rt::Point { x: 0.0, y: 0.0 }),
                    *c,
                )
            })
            .collect::<Vec<(rt::PositionedGlyph, char)>>();
        glyphs_and_chars
            .sort_unstable_by_key(|g| g.0.pixel_bounding_box().unwrap_or(inval_bb).height());

        let mut texture_height = glyphs_and_chars
            .last()
            .unwrap()
            .0
            .pixel_bounding_box()
            .unwrap_or(inval_bb)
            .height() as u32;
        let mut current_row = 0;
        let mut widest_row = 0u32;
        let mut row_sum = 0u32;

        // Sort the glyphs by height so that we know how tall each row should be in the atlas
        // Sums all the widths and heights of the bounding boxes so we know how large the atlas will be
        let mut char_rows = Vec::new();
        let mut cur_row = Vec::with_capacity(chars_per_row as usize);

        for (glyph, c) in glyphs_and_chars.iter().rev() {
            let bb = glyph.pixel_bounding_box().unwrap_or(inval_bb);

            if current_row > chars_per_row {
                current_row = 0;
                texture_height += bb.height() as u32;
                if row_sum > widest_row {
                    widest_row = row_sum;
                }
                row_sum = 0;
                char_rows.push(cur_row.clone());
                cur_row.clear();
            }

            cur_row.push((glyph, *c));
            row_sum += bb.width() as u32;
            current_row += 1;
        }
        // Push remaining chars
        char_rows.push(cur_row);

        let texture_width = widest_row + (chars_per_row * MARGIN);
        texture_height += chars_per_row * MARGIN;

        let mut texture = RgbaImage::new(texture_width as u32, texture_height as u32);
        let mut texture_cursor = Point2::<u32>::new(0, 0);
        let mut char_map: HashMap<char, CharInfo> = HashMap::new();
        let v_metrics = rusttype_font.v_metrics(font_scale);

        for row in char_rows {
            let first_glyph = row.first().unwrap().0;
            let height = first_glyph
                .pixel_bounding_box()
                .unwrap_or(inval_bb)
                .height() as u32;

            for (glyph, c) in row {
                let bb = glyph.pixel_bounding_box().unwrap_or(inval_bb);
                let h_metrics = glyph.unpositioned().h_metrics();

                char_map.insert(
                    c,
                    CharInfo {
                        vertical_offset: v_metrics.descent + bb.min.y as f32,
                        uvs: Box2::new(
                            texture_cursor.x as f32 / texture_width as f32,
                            (texture_cursor.y + height) as f32 / texture_height as f32,
                            bb.width() as f32 / texture_width as f32,
                            -bb.height() as f32 / texture_height as f32,
                        ),
                        advance_width: h_metrics.advance_width,
                        horizontal_offset: h_metrics.left_side_bearing,
                        scale: Vector2::repeat(1. / height_px),
                        width: bb.width() as f32,
                        height: bb.height() as f32,
                    },
                );

                glyph.draw(|x, y, v| {
                    let x: u32 = texture_cursor.x as u32 + x;
                    let y: u32 = texture_cursor.y as u32 + (height - y);
                    let c = (threshold(v).clamp(0., 1.) * 255.0) as u8;
                    let color = Rgba([255, 255, 255, c]);
                    texture.put_pixel(x, y, color);
                });

                texture_cursor.x += (bb.width() as u32) + MARGIN;
            }
            texture_cursor.y += height + MARGIN;
            texture_cursor.x = 0;
        }

        let texture_obj =
            OwnedTexture::from_rgba8(ctx, texture_width as u16, texture_height as u16, &texture);

        Ok(FontAtlas {
            font_texture: CachedTexture::from(texture_obj),
            font_map: char_map,
            line_gap: v_metrics.ascent - v_metrics.descent + v_metrics.line_gap,
        })
    }

    pub fn from_reader<R: Read>(
        ctx: &mut Graphics,
        mut font: R,
        height_px: f32,
        char_list_type: CharacterListType,
    ) -> Result<FontAtlas> {
        use rusttype as rt;

        let mut bytes_font = Vec::new();
        font.read_to_end(&mut bytes_font)?;
        let rusttype_font = rt::Font::try_from_bytes(&bytes_font[..])
            .ok_or_else(|| anyhow!("Unable to create a rusttype::Font using bytes_font"))?;

        Self::from_rusttype_font(ctx, &rusttype_font, height_px, char_list_type, |v| v)
    }

    fn get_char_list(char_list_type: CharacterListType) -> Result<Vec<char>> {
        let char_list = match char_list_type {
            CharacterListType::AsciiSubset => [0x20..0x7F].iter(),
            CharacterListType::Ascii => [0x00..0x7F].iter(),
            CharacterListType::ExtendedAscii => [0x00..0xFF].iter(),
            CharacterListType::Cyrillic => [
                0x0020u32..0x00FF, // Basic Latin + Latin Supplement
                0x0400u32..0x052F, // Cyrillic + Cyrillic Supplement
                0x2DE0u32..0x2DFF, // Cyrillic Extended-A
                0xA640u32..0xA69F, // Cyrillic Extended-B
            ]
            .iter(),
            CharacterListType::Thai => [
                0x0020u32..0x00FF, // Basic Latin
                0x2010u32..0x205E, // Punctuations
                0x0E00u32..0x0E7F, // Thai
            ]
            .iter(),

            CharacterListType::Vietnamese => [
                0x0020u32..0x00FF, // Basic Latin
                0x0102u32..0x0103,
                0x0110u32..0x0111,
                0x0128u32..0x0129,
                0x0168u32..0x0169,
                0x01A0u32..0x01A1,
                0x01AFu32..0x01B0,
                0x1EA0u32..0x1EF9,
            ]
            .iter(),
            CharacterListType::Chinese => bail!("Chinese fonts not yet supported"),
            CharacterListType::Japanese => bail!("Japanese fonts not yet supported"),
        };
        char_list
            .cloned()
            .flatten()
            .map(|c| {
                std::char::from_u32(c)
                    .ok_or_else(|| anyhow!("Unable to convert u32 \"{}\" into char", c))
            })
            .collect::<Result<Vec<char>>>()
    }

    // pub fn font_texture(&self) -> Guard<OwnedTexture> {
    //     self.font_texture.get()
    // }
}

impl Drawable for FontAtlas {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        self.font_texture.draw(ctx, instance);
    }
}

impl DrawableMut for FontAtlas {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.draw(ctx, instance);
    }
}

#[derive(Debug, Clone)]
pub struct CachedFontAtlas {
    inner: CacheRef<FontAtlas>,
}

impl CachedFontAtlas {
    pub fn new_uncached(font_atlas: FontAtlas) -> Self {
        Self {
            inner: CacheRef::new_uncached(font_atlas),
        }
    }

    pub fn get(&self) -> Guard<FontAtlas> {
        self.inner.get()
    }

    pub fn get_cached(&mut self) -> &FontAtlas {
        self.inner.get_cached()
    }
}

const DEFAULT_TEXT_BUFFER_SIZE: usize = 64;

#[derive(Debug)]
pub struct Text {
    batch: SpriteBatch,
}

impl Text {
    pub fn new(ctx: &mut Graphics) -> Self {
        Self::with_capacity(ctx, DEFAULT_TEXT_BUFFER_SIZE)
    }

    pub fn with_capacity(ctx: &mut Graphics, capacity: usize) -> Self {
        Text {
            batch: SpriteBatch::with_capacity(ctx, ctx.state.null_texture.clone(), capacity),
        }
    }

    pub fn from_layout(layout: &mut TextLayout, gfx: &mut Graphics) -> Text {
        // The last word's end should be pointing to the last char
        let sprite_batch_size = match layout.words.last() {
            Some(w) => w.end,
            None => 0,
        };
        let mut text = Text::with_capacity(gfx, sprite_batch_size);
        text.apply_layout(layout);
        text
    }

    pub fn apply_layout(&mut self, layout: &mut TextLayout) {
        let font_atlas = layout.font_atlas.get_cached();
        let question_mark = &font_atlas.font_map[&'?'];
        self.batch.clear();
        self.batch.set_texture(font_atlas.font_texture.clone());
        for layout_c in layout.chars.iter() {
            let c_info = font_atlas
                .font_map
                .get(&layout_c.c)
                .unwrap_or(question_mark);
            let i_param = Instance::new()
                .src(c_info.uvs)
                .color(layout_c.color)
                .translate2(Vector2::new(layout_c.coords.mins.x, layout_c.coords.mins.y));
            self.batch.insert(i_param);
        }
    }
}

impl DrawableMut for Text {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.batch.draw_mut(ctx, instance);
    }
}

// end - ending index of current word within TextLayout.chars (we always
// start at 0 and will use the previous word's end to figure out the size
// of the next word)
// width - width of the given word in pixels (used to determine whether
// or not we should start a new line)
struct Word {
    end: usize,
    width: f32,
}

impl Word {
    fn from_str(
        text: &str,
        font_map: &HashMap<char, CharInfo>,
        mut upper_bound: usize,
    ) -> Vec<Self> {
        let mut buffer = Vec::new();
        for word in text.split(' ') {
            upper_bound += word.len();
            buffer.push(Word {
                end: upper_bound,
                width: word
                    .chars()
                    .map(|c| font_map.get(&c).unwrap_or(&font_map[&'?']).advance_width)
                    .sum(),
            })
        }
        buffer
    }
}

#[derive(Debug)]
pub struct LayoutCharInfo {
    pub coords: Box2<f32>,
    pub color: Color,
    pub c: char,
}

pub struct TextLayout {
    chars: Vec<LayoutCharInfo>,
    words: Vec<Word>,
    font_atlas: CachedFontAtlas,
    cursor: Point2<f32>,
    space_width: f32,
}

impl TextLayout {
    pub fn new(mut font_atlas: CachedFontAtlas) -> Self {
        let space_width = font_atlas.get_cached().font_map[&' '].advance_width;
        TextLayout {
            font_atlas,
            chars: Vec::new(),
            words: Vec::new(),
            cursor: Point2::new(0., 0.),
            space_width,
        }
    }

    pub fn chars(&self) -> &[LayoutCharInfo] {
        &self.chars
    }

    pub fn clear(&mut self) {
        self.chars.clear();
        self.words.clear();
        self.cursor = Point2::new(0., 0.);
    }

    pub fn push_str<T>(&mut self, text: &str, colors: T)
    where
        T: IntoIterator<Item = Color>,
        T::IntoIter: Clone,
    {
        let color_iter = colors.into_iter();
        if let Some(upper_bound) = color_iter.size_hint().1 {
            assert!(
                upper_bound < text.len(),
                "Passed in less colors than the number of chars you tried to push!"
            );
        }
        let font_atlas = self.font_atlas.get_cached();
        self.words.append(&mut Word::from_str(
            text,
            &font_atlas.font_map,
            self.words.last().unwrap_or(&Word { end: 0, width: 0. }).end,
        ));
        let question_mark = &font_atlas.font_map[&'?'];
        let mut chars = text.chars();
        for (c, color) in chars.by_ref().zip(color_iter) {
            if c.is_whitespace() {
                self.cursor.x += self.space_width;
                continue;
            }
            let c_info = font_atlas.font_map.get(&c).unwrap_or(question_mark);
            self.chars.push(LayoutCharInfo {
                coords: Box2::new(
                    self.cursor.x,
                    self.cursor.y - c_info.vertical_offset,
                    c_info.width,
                    c_info.height,
                ),
                color,
                c,
            });
            self.cursor.x += c_info.advance_width - c_info.horizontal_offset;
        }
        assert_eq!(
            chars.next(),
            None,
            "Ended up with less colors than chars! Did not push entire new string"
        );
    }

    pub fn push_wrapping_str<T>(&mut self, text: &str, colors: T, line_width: f32)
    where
        T: IntoIterator<Item = Color>,
        T::IntoIter: Clone,
    {
        let font_atlas = self.font_atlas.get_cached();
        let question_mark = font_atlas.font_map[&'?'];
        let new_words = Word::from_str(
            text,
            &font_atlas.font_map,
            self.words.last().unwrap_or(&Word { end: 0, width: 0. }).end,
        );

        let mut start = match self.words.last() {
            Some(w) => w.end,
            None => 0usize,
        };

        let mut char_iter = text.chars();
        let mut colors_iter = colors.into_iter();

        for word in new_words.iter() {
            if word.width + self.cursor.x > line_width as f32 {
                self.cursor.x = 0.;
                self.cursor.y += font_atlas.line_gap;
            }

            for _ in 0..(word.end - start) {
                let c = char_iter
                    .next()
                    .expect("Somehow got more words than chars that existed!");
                let color = colors_iter.next().expect(
                    "Should've gotten more colors, but didn't! Did you pass in enough colors?",
                );
                let c_info = font_atlas.font_map.get(&c).unwrap_or(&question_mark);
                self.chars.push(LayoutCharInfo {
                    coords: Box2::new(
                        self.cursor.x,
                        self.cursor.y - c_info.vertical_offset,
                        c_info.width,
                        c_info.height,
                    ),
                    color,
                    c,
                });
                self.cursor.x += c_info.advance_width - c_info.horizontal_offset;
            }

            start = word.end;
            // Advance the char and color iterators to get rid of the space
            char_iter.next();
            colors_iter.next();
            self.cursor.x += self.space_width;
        }
    }
}

pub struct FontLoader {
    engine: EngineRef,
}

impl Loader<String, Font> for FontLoader {
    fn load(&mut self, key: &String) -> Result<Handle<Font>> {
        use rusttype as rt;
        let engine = self.engine.upgrade();
        let mut file = engine.fs().open(key)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        let font = rt::Font::try_from_vec(buf).ok_or_else(|| anyhow!("error parsing font"))?;
        Ok(Handle::new(Font { inner: font }))
    }
}

pub struct FontAtlasLoader {
    font_cache: SwappableCache<String, Font, FontLoader>,
    engine: EngineRef,
    weak_gfx_cache: WeakResourceCache<GraphicsLock>,
}

impl Loader<FontAtlasKey, FontAtlas> for FontAtlasLoader {
    fn load(&mut self, key: &FontAtlasKey) -> Result<Handle<FontAtlas>> {
        let engine = self.engine.upgrade();
        let mut font = self.font_cache.get_or_load(key.path.clone())?.into_cached();
        let gfx_lock = self.weak_gfx_cache.get::<_, Error>(|| Ok(engine.get()))?;
        let gfx = &mut gfx_lock.lock();
        let atlas = match key.threshold {
            Some(t) => FontAtlas::from_rusttype_font(
                gfx,
                &font.get_cached().inner,
                key.size as f32,
                key.char_list_type,
                |v| if v > *t { 1. } else { 0. },
            )?,
            None => FontAtlas::from_rusttype_font(
                gfx,
                &font.get_cached().inner,
                key.size as f32,
                key.char_list_type,
                |v| v,
            )?,
        };

        Ok(Handle::new(atlas))
    }
}

pub struct FontCache {
    inner: SwappableCache<FontAtlasKey, FontAtlas, FontAtlasLoader>,
}

impl FontCache {
    pub fn new(engine: &Engine) -> Self {
        let font_loader = FontLoader {
            engine: engine.downgrade(),
        };

        let font_atlas_loader = FontAtlasLoader {
            font_cache: SwappableCache::new(font_loader),
            engine: engine.downgrade(),
            weak_gfx_cache: WeakResourceCache::new(),
        };

        Self {
            inner: SwappableCache::new(font_atlas_loader),
        }
    }

    pub fn get_or_load(&mut self, key: FontAtlasKey) -> Result<CachedFontAtlas> {
        Ok(CachedFontAtlas {
            inner: self.inner.get_or_load(key)?.into_cached(),
        })
    }
}
