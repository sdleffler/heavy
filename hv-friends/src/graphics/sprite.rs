use anyhow::*;
use aseprite::SpritesheetData;
use hv_core::{
    engine::{Engine, EngineRef, LuaResource},
    mlua::prelude::*,
    mq,
    swappable_cache::{CacheRef, Guard, Handle, Loader, SwappableCache},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Read, mem, ops, path::Path};
use thunderdome::{Arena, Index};

use crate::{
    graphics::{CachedTexture, Drawable, DrawableMut, Graphics, Instance, InstanceProperties},
    math::*,
};

#[derive(Debug, Clone)]
pub struct Sprite {
    pub params: Instance,
    pub texture: CachedTexture,
}

impl Sprite {
    pub fn new(texture: impl Into<CachedTexture>, params: Instance) -> Self {
        Self {
            params,
            texture: texture.into(),
        }
    }
}

impl DrawableMut for Sprite {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        let params = Instance {
            tx: instance.tx * self.params.tx,
            ..self.params
        };
        self.texture.draw_mut(ctx, params);
    }
}

/// FIXME(sleffy): same issue as the SpriteBatch implementation, ignoring
/// the passed-in src/color params
impl Drawable for Sprite {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        let params = Instance {
            tx: instance.tx * self.params.tx,
            ..self.params
        };
        self.texture.get().draw(ctx, params);
    }
}

impl LuaUserData for Sprite {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        crate::lua::add_drawable_methods(methods);
    }
}

/// Represents the index of a `Sprite` within a `SpriteBatch`
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SpriteId(Index);

impl<'lua> ToLua<'lua> for SpriteId {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        LuaLightUserData(self.0.to_bits() as *mut _).to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for SpriteId {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        LuaLightUserData::from_lua(lua_value, lua).map(|lud| Self(Index::from_bits(lud.0 as u64)))
    }
}

pub struct SpriteBatchIter<'a> {
    iter: thunderdome::Iter<'a, Instance>,
}

impl<'a> Iterator for SpriteBatchIter<'a> {
    type Item = (SpriteId, &'a Instance);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(i, v)| (SpriteId(i), v))
    }
}

pub struct SpriteBatchIterMut<'a> {
    iter: thunderdome::IterMut<'a, Instance>,
}

impl<'a> Iterator for SpriteBatchIterMut<'a> {
    type Item = (SpriteId, &'a mut Instance);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(i, v)| (SpriteId(i), v))
    }
}

#[derive(Debug)]
pub struct SpriteBatch {
    sprites: Arena<Instance>,
    // Used to store the result of converting InstanceParams to InstanceProperties
    instances: Vec<InstanceProperties>,
    /// Capacity is used to store the length of the buffers inside of mq::Bindings
    capacity: usize,
    bindings: mq::Bindings,
    dirty: bool,
    texture: CachedTexture,
}

impl ops::Index<SpriteId> for SpriteBatch {
    type Output = Instance;

    #[inline]
    fn index(&self, index: SpriteId) -> &Self::Output {
        &self.sprites[index.0]
    }
}

impl ops::IndexMut<SpriteId> for SpriteBatch {
    #[inline]
    fn index_mut(&mut self, index: SpriteId) -> &mut Self::Output {
        self.dirty = true;
        &mut self.sprites[index.0]
    }
}

impl SpriteBatch {
    pub fn new<T>(ctx: &mut Graphics, texture: T) -> Self
    where
        T: Into<CachedTexture>,
    {
        const DEFAULT_SPRITEBATCH_CAPACITY: usize = 64;
        Self::with_capacity(ctx, texture, DEFAULT_SPRITEBATCH_CAPACITY)
    }

    pub fn with_capacity<T>(ctx: &mut Graphics, texture: T, capacity: usize) -> Self
    where
        T: Into<CachedTexture>,
    {
        let mut texture = texture.into();

        let instances = mq::Buffer::stream(
            &mut ctx.mq,
            mq::BufferType::VertexBuffer,
            capacity * mem::size_of::<InstanceProperties>(),
        );

        let bindings = mq::Bindings {
            vertex_buffers: vec![ctx.state.quad_bindings.vertex_buffers[0], instances],
            index_buffer: ctx.state.quad_bindings.index_buffer,
            images: vec![texture.get_cached().handle],
        };

        Self {
            sprites: Arena::new(),
            instances: Vec::new(),
            capacity,
            bindings,
            dirty: true,
            texture,
        }
    }

    #[inline]
    pub fn insert(&mut self, param: Instance) -> SpriteId {
        self.dirty = true;
        SpriteId(self.sprites.insert(param))
    }

    #[inline]
    pub fn remove(&mut self, index: SpriteId) {
        self.dirty = true;
        self.sprites.remove(index.0);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.dirty = true;
        self.sprites.clear();
    }

    #[inline]
    pub fn texture(&self) -> &CachedTexture {
        &self.texture
    }

    #[inline]
    pub fn set_texture(&mut self, texture: impl Into<CachedTexture>) {
        let mut new_texture = texture.into();

        if !CachedTexture::ptr_eq_cached(&mut self.texture, &mut new_texture) {
            self.dirty = true;
            self.texture = new_texture;
        }
    }

    pub fn flush(&mut self, ctx: &mut Graphics) {
        let texture = self.texture.get_cached();

        if !self.dirty && texture.handle == self.bindings.images[0] {
            return;
        }

        self.instances.clear();
        self.instances.extend(self.sprites.iter().map(|(_, param)| {
            param
                .scale2(param.src.extents())
                .scale2(Vector2::new(
                    texture.width() as f32,
                    texture.height() as f32,
                ))
                .to_instance_properties()
        }));

        if self.instances.len() > self.capacity {
            let new_capacity = self.instances.len().checked_next_power_of_two().unwrap();
            let new_buffer = mq::Buffer::stream(
                &mut ctx.mq,
                mq::BufferType::VertexBuffer,
                new_capacity * mem::size_of::<InstanceProperties>(),
            );

            let old_buffer = mem::replace(&mut self.bindings.vertex_buffers[1], new_buffer);
            old_buffer.delete();

            self.capacity = new_capacity;
        }

        self.bindings.vertex_buffers[1].update(&mut ctx.mq, &self.instances);
        self.bindings.images[0] = texture.handle;

        self.dirty = false;
    }

    pub fn iter(&self) -> SpriteBatchIter<'_> {
        SpriteBatchIter {
            iter: self.sprites.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> SpriteBatchIterMut<'_> {
        SpriteBatchIterMut {
            iter: self.sprites.iter_mut(),
        }
    }
}

/// TODO: FIXME(sleffy) maybe? This implementation ignores the color and src parameters
/// of the `InstanceParam`. Not sure there's much to be done about that, though, since
/// the spritebatch has its own instance parameters.
impl DrawableMut for SpriteBatch {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.flush(ctx);

        ctx.push_multiplied_transform(instance.tx.to_homogeneous());
        ctx.mq.apply_bindings(&self.bindings);
        ctx.apply_transforms();
        // 6 here because a quad is 6 vertices
        ctx.mq.draw(0, 6, self.instances.len() as i32);
        ctx.pop_transform();
        ctx.apply_transforms();
    }
}

impl LuaUserData for SpriteBatch {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        crate::lua::add_drawable_methods(methods);

        methods.add_method_mut("insert", |_, this, instance| Ok(this.insert(instance)));

        methods.add_method_mut("remove", |_, this, sprite_id| {
            this.remove(sprite_id);
            Ok(())
        });

        methods.add_method_mut("clear", |_, this, ()| {
            this.clear();
            Ok(())
        });

        methods.add_meta_method(LuaMetaMethod::Index, |_, this, sprite_id| {
            Ok(this[sprite_id])
        });

        methods.add_meta_method_mut(LuaMetaMethod::NewIndex, |_, this, (sprite_id, instance)| {
            this[sprite_id] = instance;
            Ok(())
        });

        methods.add_method_mut("set_texture", |_, this, texture: CachedTexture| {
            this.set_texture(texture);
            Ok(())
        });

        methods.add_method("texture", |_, this, ()| Ok(this.texture().clone()));
    }
}

#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(transparent)]
pub struct TagId(u32);

impl<'lua> ToLua<'lua> for TagId {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        self.0.to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for TagId {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        u32::from_lua(lua_value, lua).map(TagId)
    }
}

#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(transparent)]
pub struct FrameId(u32);

impl<'lua> ToLua<'lua> for FrameId {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        self.0.to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for FrameId {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        u32::from_lua(lua_value, lua).map(FrameId)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Direction {
    Forward,
    Reverse,
    Pingpong,
}

impl From<aseprite::Direction> for Direction {
    fn from(ad: aseprite::Direction) -> Self {
        match ad {
            aseprite::Direction::Forward => Self::Forward,
            aseprite::Direction::Reverse => Self::Reverse,
            aseprite::Direction::Pingpong => Self::Pingpong,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    pub from: u32,
    pub to: u32,
    pub direction: Direction,
}

#[derive(Debug, Clone, Copy)]
pub enum NextFrame {
    /// Returned if this is just the next frame ID.
    Stepped(FrameId),
    Wrapped(FrameId),
}

impl Tag {
    pub fn first_frame(&self) -> FrameId {
        match self.direction {
            Direction::Forward | Direction::Pingpong => FrameId(self.from),
            Direction::Reverse => FrameId(self.to),
        }
    }

    pub fn last_frame(&self) -> FrameId {
        match self.direction {
            Direction::Forward | Direction::Pingpong => FrameId(self.to),
            Direction::Reverse => FrameId(self.from),
        }
    }

    /// Returns `Err` if this next frame would loop the animation, `Ok` otherwise.
    pub fn next_frame(&self, FrameId(current): FrameId) -> Result<FrameId, FrameId> {
        match self.direction {
            Direction::Forward if current == self.to => Err(FrameId(self.from)),
            Direction::Reverse if current == self.from => Err(FrameId(self.to)),
            Direction::Pingpong if current == self.to => {
                Err(FrameId(na::max(self.to - 1, self.from)))
            }
            Direction::Pingpong if current == self.from => {
                Err(FrameId(na::min(self.from + 1, self.to)))
            }
            Direction::Forward => Ok(FrameId(current + 1)),
            Direction::Reverse => Ok(FrameId(current - 1)),
            Direction::Pingpong => todo!("pingpong is broken!"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Frame {
    pub frame: Box2<u32>,
    pub frame_source: Box2<u32>,
    pub source_size: Vector2<u32>,
    pub offset: Vector2<f32>,
    pub uvs: Box2<f32>,
    pub duration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteSheet {
    pub image: String,
    pub tag_ids: HashMap<String, TagId>,
    pub tags: Vec<Tag>,
    pub frames: Vec<Frame>,
    pub size: Vector2<u32>,
}

impl ops::Index<TagId> for SpriteSheet {
    type Output = Tag;

    fn index(&self, TagId(id): TagId) -> &Self::Output {
        &self.tags[id as usize]
    }
}

impl ops::Index<SpriteTag> for SpriteSheet {
    type Output = Tag;

    fn index(&self, sprite_tag: SpriteTag) -> &Self::Output {
        &self[sprite_tag.tag_id]
    }
}

impl ops::Index<FrameId> for SpriteSheet {
    type Output = Frame;

    fn index(&self, FrameId(id): FrameId) -> &Self::Output {
        &self.frames[id as usize]
    }
}

impl ops::Index<SpriteFrame> for SpriteSheet {
    type Output = Frame;

    fn index(&self, SpriteFrame(FrameId(id)): SpriteFrame) -> &Self::Output {
        &self.frames[id as usize]
    }
}

impl SpriteSheet {
    pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = String::new();
        reader.read_to_string(&mut buf)?;
        Self::from_json(&buf)
    }

    pub fn from_json(s: &str) -> Result<Self> {
        let spritesheet_data = serde_json::from_str::<SpritesheetData>(s)?;
        let dims = spritesheet_data.meta.size;
        let size = Vector2::new(dims.w, dims.h);

        let mut frames = Vec::new();
        for ase_frame in spritesheet_data.frames.into_iter() {
            let fr = ase_frame.frame;
            let sb = ase_frame.sprite_source_size;
            let ss = ase_frame.source_size;

            let duration = ase_frame.duration;
            let frame = Box2::new(fr.x, fr.y, fr.w, fr.h);
            let frame_source = Box2::new(sb.x, sb.y, sb.w, sb.h);
            let source_size = Vector2::new(ss.w, ss.h);
            // `bw` is border width. only nonzero if there's padding added to the spritesheet.
            let bw = Vector2::new(fr.w - sb.w, fr.h - sb.h).cast::<i32>() / 2;
            let offset = Vector2::new(sb.x as i32 - bw.x, sb.y as i32 - bw.y).cast::<f32>()
                - Vector2::new(ss.w, ss.h).cast::<f32>() / 2.;
            let uvs = Box2::new(
                fr.x as f32 / size.x as f32,
                fr.y as f32 / size.y as f32,
                fr.w as f32 / size.x as f32,
                fr.h as f32 / size.y as f32,
            );

            frames.push(Frame {
                frame,
                frame_source,
                source_size,
                offset,
                uvs,
                duration,
            });
        }

        let mut tags = vec![Tag {
            name: String::new(),
            from: 0,
            to: frames.len() as u32 - 1,
            direction: Direction::Forward,
        }];

        for frame_tag in spritesheet_data.meta.frame_tags.into_iter().flatten() {
            tags.push(Tag {
                name: frame_tag.name,
                from: frame_tag.from,
                to: frame_tag.to,
                direction: Direction::from(frame_tag.direction),
            });
        }

        let tag_ids = tags
            .iter()
            .enumerate()
            .map(|(i, tag)| (tag.name.clone(), TagId(i as u32)))
            .collect::<HashMap<_, _>>();

        Ok(Self {
            image: spritesheet_data
                .meta
                .image
                .ok_or_else(|| anyhow!("no image path"))?,
            tag_ids,
            tags,
            frames,
            size,
        })
    }

    pub fn update_animation(&self, dt: f32, tag: &mut SpriteTag, frame: &mut SpriteFrame) {
        if let Some((new_tag, maybe_new_frame)) = self.update_animation_inner(dt, tag, frame) {
            *tag = new_tag;

            if let Some(new_frame) = maybe_new_frame {
                *frame = new_frame;
            }
        }
    }

    fn update_animation_inner(
        &self,
        dt: f32,
        tag: &SpriteTag,
        SpriteFrame(frame): &SpriteFrame,
    ) -> Option<(SpriteTag, Option<SpriteFrame>)> {
        if !tag.is_paused {
            let mut new_tag = SpriteTag {
                remaining: tag.remaining - dt * 1_000.,
                ..*tag
            };

            if new_tag.remaining < 0. {
                match self[new_tag.tag_id].next_frame(*frame) {
                    Err(_) if !tag.should_loop => Some((
                        SpriteTag {
                            is_paused: true,
                            ..new_tag
                        },
                        Some(SpriteFrame(self[new_tag.tag_id].last_frame())),
                    )),
                    Ok(new_frame) | Err(new_frame) => {
                        new_tag.remaining += self[new_frame].duration as f32;
                        Some((new_tag, Some(SpriteFrame(new_frame))))
                    }
                }
            } else {
                Some((new_tag, None))
            }
        } else {
            None
        }
    }

    pub fn get_tag<K: AsRef<str>>(&self, s: K) -> Option<TagId> {
        self.tag_ids.get(s.as_ref()).copied()
    }

    pub fn at_tag(&self, tag_id: TagId, should_loop: bool) -> (SpriteFrame, SpriteTag) {
        let tag = &self[tag_id];
        let ff = tag.first_frame();
        (
            SpriteFrame(ff),
            SpriteTag {
                tag_id,
                remaining: self[ff].duration as f32,
                is_paused: false,
                should_loop,
            },
        )
    }
}

#[derive(Debug, Clone)]
pub struct CachedSpriteSheet {
    handle: CacheRef<SpriteSheet>,
}

impl CachedSpriteSheet {
    pub fn new_uncached(sprite_sheet: SpriteSheet) -> Self {
        Self {
            handle: CacheRef::new_uncached(sprite_sheet),
        }
    }

    pub fn get(&self) -> Guard<SpriteSheet> {
        self.handle.get()
    }

    pub fn get_cached(&mut self) -> &SpriteSheet {
        self.handle.get_cached()
    }
}

impl LuaUserData for CachedSpriteSheet {}

/// Component holding the string name of a spritesheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SpriteName(pub String);

/// Component holding the current frame ID of a sprite.
#[derive(Debug, Copy, Clone, Default)]
pub struct SpriteFrame(pub FrameId);

/// Component holding the state of a running animation at a given tag.
#[derive(Debug, Copy, Clone)]
pub struct SpriteTag {
    /// The index of the currently running animation/tag.
    pub tag_id: TagId,
    /// Remaining time for this frame, in milliseconds.
    pub remaining: f32,
    /// Whether this animation is running or paused.
    pub is_paused: bool,
    /// Whether this animation should loop, or pause on the last frame.
    pub should_loop: bool,
}

impl Default for SpriteTag {
    fn default() -> Self {
        Self {
            tag_id: TagId::default(),
            remaining: 0.,
            is_paused: false,
            should_loop: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpriteAnimationState {
    pub sheet: CachedSpriteSheet,
    pub frame: SpriteFrame,
    pub tag: SpriteTag,
}

impl LuaUserData for SpriteAnimationState {}

pub struct FilesystemSpriteSheetLoader {
    engine: EngineRef,
}

impl FilesystemSpriteSheetLoader {
    pub fn new(engine: &Engine) -> Self {
        Self {
            engine: engine.downgrade(),
        }
    }
}

impl<P: AsRef<Path>> Loader<P, SpriteSheet> for FilesystemSpriteSheetLoader {
    fn load(&mut self, key: &P) -> Result<Handle<SpriteSheet>> {
        let engine = self.engine.upgrade();
        let mut file = engine.fs().open(key)?;
        let sprite_sheet = SpriteSheet::from_reader(&mut file)?;
        Ok(Handle::new(sprite_sheet))
    }
}

pub struct SpriteSheetCache {
    inner: SwappableCache<String, SpriteSheet, FilesystemSpriteSheetLoader>,
}

impl LuaUserData for SpriteSheetCache {}

impl LuaResource for SpriteSheetCache {
    const REGISTRY_KEY: &'static str = "HV_FRIENDS_SPRITE_SHEET_CACHE";
}

impl SpriteSheetCache {
    pub fn new(engine: &Engine) -> Self {
        Self {
            inner: SwappableCache::new(FilesystemSpriteSheetLoader::new(engine)),
        }
    }

    pub fn get_or_load(&mut self, path: impl Into<String>) -> Result<CachedSpriteSheet> {
        Ok(CachedSpriteSheet {
            handle: self.inner.get_or_load(path.into())?.into_cached(),
        })
    }

    pub fn reload_all(&mut self) -> Result<()> {
        self.inner.reload_all()
    }
}

// impl LuaComponentInterface for SpriteAnimation {
//     fn accessor<'lua>(lua: &'lua Lua, entity: Entity) -> LuaResult<LuaValue<'lua>> {
//         SpriteAnimationAccessor(entity).to_lua(lua)
//     }

//     fn bundler<'lua>(
//         lua: &'lua Lua,
//         args: LuaValue<'lua>,
//         builder: &mut EntityBuilder,
//     ) -> LuaResult<()> {
//         let table = LuaTable::from_lua(args, lua)?;
//         let path = table
//             .get::<_, LuaString>("path")
//             .log_error_err(module_path!())?;

//         let tmp = lua.fetch_one::<DefaultCache>()?;
//         let mut sprite_sheet = tmp
//             .borrow()
//             .get::<SpriteSheet>(&Key::from_path(path.to_str()?))
//             .to_lua_err()?;

//         let should_loop = table.get::<_, Option<bool>>("should_loop")?.unwrap_or(true);

//         let tag_id = match table
//             .get::<_, Option<LuaString>>("tag")
//             .log_warn_err(module_path!())?
//         {
//             Some(tag_name) => sprite_sheet.load_cached().get_tag(tag_name.to_str()?),
//             None => None,
//         }
//         .unwrap_or_default();
//         let (frame, tag) = sprite_sheet.load_cached().at_tag(tag_id, should_loop);

//         builder.add(SpriteAnimation {
//             frame,
//             tag,
//             sheet: sprite_sheet,
//         });
//         Ok(())
//     }
// }
