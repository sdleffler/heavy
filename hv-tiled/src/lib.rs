use hv_core::{engine::Engine, prelude::*};

use hv_friends::{
    graphics::{
        CachedTexture, Color, Drawable, DrawableMut, Graphics, GraphicsLock, GraphicsLockExt,
        Instance, OwnedTexture, SpriteBatch,
    },
    math::Box2,
    math::Vector2,
};

use std::{collections::HashMap, ops, path::Path};

use tiled;

#[derive(Debug)]
pub enum LayerType {
    Tile,
}

// TODO: This type was pulled from the Tiled crate, but the Color and File variants
// are never constructed. This might be a bug depending on what the "properties"
// table contains
#[derive(Debug)]
pub enum Property {
    Bool(bool),
    Float(f64),
    Int(i64),
    Color(u32),
    String(String),
    File(String),
}

#[derive(Debug)]
pub enum Encoding {
    Lua,
}

pub enum Orientation {
    Orthogonal,
    Isometric,
}

pub enum RenderOrder {
    RightDown,
    RightUp,
    LeftDown,
    LeftUp,
}

#[derive(Debug, Clone, Copy)]
pub struct TileId(u32);

pub struct MapData {
    pub tsx_ver: String,
    pub lua_ver: String,
    pub tiled_ver: String,
    pub orientation: Orientation,
    pub render_order: RenderOrder,
    pub width: usize,
    pub height: usize,
    pub tilewidth: usize,
    pub tileheight: usize,
    pub nextlayerid: usize,
    pub nextobjectid: usize,
    pub properties: HashMap<String, Property>,
}

impl MapData {
    pub fn from_lua_table(map_table: &LuaTable) -> Result<Self, Error> {
        let render_order = match map_table.get::<_, LuaString>("renderorder")?.to_str()? {
            "right-down" => RenderOrder::RightDown,
            r => return Err(anyhow!("Got an unsupported renderorder: {}", r)),
        };

        let orientation = match map_table.get::<_, LuaString>("orientation")?.to_str()? {
            "orthogonal" => Orientation::Orthogonal,
            o => return Err(anyhow!("Got an unsupported orientation: {}", o)),
        };

        let mut properties = HashMap::new();

        for pair_res in map_table.get::<_, LuaTable>("properties")?.pairs() {
            let pair = pair_res?;
            let val = match pair.1 {
                LuaValue::Boolean(b) => Property::Bool(b),
                LuaValue::Integer(i) => Property::Int(i),
                LuaValue::Number(n) => Property::Float(n),
                LuaValue::String(s) => Property::String(s.to_str()?.to_owned()),
                l => {
                    return Err(anyhow!(
                        "Got an unexpected value in the properties section: {:?}",
                        l
                    ))
                }
            };
            properties.insert(pair.0, val);
        }

        Ok(MapData {
            width: map_table.get("width")?,
            height: map_table.get("height")?,
            tilewidth: map_table.get("tilewidth")?,
            tileheight: map_table.get("tileheight")?,
            tsx_ver: map_table
                .get::<_, LuaString>("version")?
                .to_str()?
                .to_owned(),
            lua_ver: map_table
                .get::<_, LuaString>("luaversion")?
                .to_str()?
                .to_owned(),
            tiled_ver: map_table
                .get::<_, LuaString>("tiledversion")?
                .to_str()?
                .to_owned(),
            nextlayerid: map_table.get::<_, LuaInteger>("nextlayerid")? as usize,
            nextobjectid: map_table.get::<_, LuaInteger>("nextobjectid")? as usize,
            orientation,
            properties,
            render_order,
        })
    }
}

#[derive(Debug)]
pub struct Layer {
    layer_type: LayerType,
    id: usize,
    name: String,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    visible: bool,
    opacity: f64,
    offset_x: usize,
    offset_y: usize,
    properties: HashMap<String, Property>,
    encoding: Encoding,
    data: Vec<TileId>,
}

pub struct LayerBatch(Vec<SpriteBatch>);

impl DrawableMut for LayerBatch {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        for batch in self.0.iter_mut() {
            batch.draw_mut(ctx, instance);
        }
    }
}

impl LayerBatch {
    pub fn new(
        layer: &Layer,
        ts_atlas: &TilesetAtlas,
        engine: &Engine,
        map_data: &MapData,
    ) -> Self {
        // We need 1 sprite batch per texture
        let mut batches = Vec::with_capacity(ts_atlas.textures.len());
        let graphics_lock = engine.get::<GraphicsLock>();

        for texture in ts_atlas.textures.iter() {
            let mut acquired_lock = GraphicsLockExt::lock(&graphics_lock);
            batches.push(SpriteBatch::new(&mut acquired_lock, texture.clone()));
            drop(acquired_lock);
        }
        let top = layer.height * map_data.tileheight;

        for y_cord in 0..layer.height {
            for x_cord in 0..layer.width {
                let tile = layer.data[y_cord * layer.width + x_cord];
                // Tile indices start at 1, 0 represents no tile, so we offset the tile by 1
                // first, and skip making the instance param if the tile is 0
                if tile.0 == 0 {
                    continue;
                }

                let real_id: TileId = TileId(tile.0 - 1u32);

                let (uvs, tileset_id) = ts_atlas[real_id];
                batches[tileset_id].insert(
                    Instance::new()
                        .src(uvs)
                        .color(Color::new(1.0, 1.0, 1.0, layer.opacity as f32))
                        .translate2(Vector2::new(
                            (x_cord * map_data.tilewidth) as f32,
                            (top - (y_cord * map_data.tileheight)) as f32,
                        )),
                );
            }
        }
        LayerBatch(batches)
    }
}

pub struct TilesetAtlas {
    // Box2<f32> is the uvs, the second usize is an index into a texture vec
    // that relates the uv to the texture
    render_data: Vec<(Box2<f32>, usize)>,
    textures: Vec<CachedTexture>,
}

impl ops::Index<TileId> for TilesetAtlas {
    type Output = (Box2<f32>, usize);

    #[inline]
    fn index(&self, index: TileId) -> &Self::Output {
        &self.render_data[(index.0 as usize)]
    }
}

impl TilesetAtlas {
    pub fn new(tilesets: Vec<tiled::Tileset>, engine: &Engine) -> Result<Self, Error> {
        let mut textures = Vec::with_capacity(tilesets.len());
        let mut render_data = Vec::new();

        for (i, tileset) in (0..).zip(tilesets.iter()) {
            if tileset.images.len() > 1 {
                return Err(anyhow!(
                    "Multiple images per tilesets aren't supported yet. Expected 1 image, got {}",
                    tileset.images.len()
                ));
            }

            let mut fs = engine.fs();
            let mut tileset_img_path = fs.open(&mut Path::new(
                &("/".to_owned() + &tileset.images[0].source),
            ))?;
            let graphics_lock = engine.get::<GraphicsLock>();
            let mut acquired_lock = GraphicsLockExt::lock(&graphics_lock);
            let texture_obj = OwnedTexture::from_reader(&mut acquired_lock, &mut tileset_img_path)?;

            drop(acquired_lock);

            if let Some(tile_count) = tileset.tilecount {
                let rows = tile_count / tileset.columns;
                let top = (rows * (tileset.spacing + tileset.tile_height)) + tileset.margin;
                for row in 1..=rows {
                    for column in 0..tileset.columns {
                        render_data.push((
                            Box2::new(
                                (tileset.margin
                                    + ((column * tileset.tile_width) + column * tileset.spacing))
                                    as f32
                                    / texture_obj.width() as f32,
                                (tileset.spacing
                                    + (top
                                        - (tileset.margin
                                            + ((row * tileset.tile_height)
                                                + row * tileset.spacing))))
                                    as f32
                                    / texture_obj.height() as f32,
                                tileset.tile_width as f32 / texture_obj.width() as f32,
                                tileset.tile_height as f32 / texture_obj.height() as f32,
                            ),
                            i,
                        ));
                    }
                }
                textures.push(CachedTexture::from(texture_obj));
            } else {
                return Err(anyhow!("Tile count was None for some reason! Check the tiled map,
                                    and if it's indeed missing, let Maxim Veligan (maximveligan.gmail.com) know"));
            }
        }

        Ok(TilesetAtlas {
            render_data,
            textures,
        })
    }
}

impl Drawable for TilesetAtlas {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        let mut y_offset = 0.0;
        for texture in self.textures.iter() {
            texture.draw(ctx, instance.translate2(Vector2::new(0.0, y_offset)));
            y_offset += texture.get().height() as f32;
        }
    }
}

impl DrawableMut for TilesetAtlas {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.draw(ctx, instance);
    }
}

impl Layer {
    pub fn from_lua_table(t: LuaTable) -> Result<Layer, Error> {
        let layer_type = match t.get::<_, LuaString>("type")?.to_str()? {
            "tilelayer" => LayerType::Tile,
            s => return Err(anyhow!("Got an unsupported tilelayer type: {}", s)),
        };

        let encoding = match t.get::<_, LuaString>("encoding")?.to_str()? {
            "lua" => Encoding::Lua,
            e => return Err(anyhow!("Got an unsupported encoding type: {}", e)),
        };

        let width = t.get("width")?;
        let height = t.get("height")?;
        let mut tile_data = Vec::with_capacity(width * height);

        for tile in t.get::<_, LuaTable>("data")?.sequence_values() {
            tile_data.push(TileId(tile?));
        }

        let mut properties = HashMap::new();

        for pair_res in t.get::<_, LuaTable>("properties")?.pairs() {
            let pair = pair_res?;
            let val = match pair.1 {
                LuaValue::Boolean(b) => Property::Bool(b),
                LuaValue::Integer(i) => Property::Int(i),
                LuaValue::Number(n) => Property::Float(n),
                LuaValue::String(s) => Property::String(s.to_str()?.to_owned()),
                l => {
                    return Err(anyhow!(
                        "Got an unexpected value in the properties section: {:?}",
                        l
                    ))
                }
            };
            properties.insert(pair.0, val);
        }

        Ok(Layer {
            id: t.get("id")?,
            name: t.get::<_, LuaString>("name")?.to_str()?.to_owned(),
            x: t.get("x")?,
            y: t.get("y")?,
            visible: t.get("visible")?,
            opacity: t.get("opacity")?,
            offset_x: t.get("offsetx")?,
            offset_y: t.get("offsety")?,
            data: tile_data,
            encoding: encoding,
            layer_type: layer_type,
            width: width,
            height: height,
            properties: properties,
        })
    }
}

pub fn get_tileset(t: LuaTable, engine: &Engine) -> Result<tiled::Tileset, Error> {
    let filename = "/".to_owned() + t.get::<_, LuaString>("filename")?.to_str()?;
    let gid = t.get("firstgid")?;
    let mut fs = engine.fs();
    let tileset_path = fs.open(Path::new(&filename))?;
    match tiled::parse_tileset(tileset_path, gid) {
        Ok(t) => Ok(t),
        Err(e) => Err(anyhow!("Got an error when parsing {}: {:?}", filename, e)),
    }
}
