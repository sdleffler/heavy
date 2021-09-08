use hv_core::{engine::Engine, prelude::*};

use hv_friends::{
    graphics::{
        CachedTexture, Color, Drawable, DrawableMut, Graphics, GraphicsLock, GraphicsLockExt,
        Instance, OwnedTexture, SpriteBatch,
    },
    math::Box2,
    math::Vector2,
};

use std::{collections::HashMap, io::Read, path::Path};

#[derive(Debug, Clone)]
pub enum LayerType {
    Tile,
}

// TODO: This type was pulled from the Tiled crate, but the Color and File variants
// are never constructed. This might be a bug depending on what the "properties"
// table contains
#[derive(Debug, PartialEq, Clone)]
pub enum Property {
    Bool(bool),
    Float(f64),
    Int(i64),
    Color(u32),
    String(String),
    File(String),
}

#[derive(Debug, Clone)]
pub struct Properties(HashMap<String, Property>);

impl Properties {
    pub fn from_lua(props: &LuaTable) -> Result<Self, Error> {
        let mut properties = HashMap::new();
        let props_t = props.get::<_, LuaTable>("properties")?;

        for pair_res in props_t.pairs() {
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
        Ok(Properties(properties))
    }
}

#[derive(Debug, Clone)]
pub enum Encoding {
    Lua,
}

#[derive(Debug, Clone)]
pub enum Orientation {
    Orthogonal,
    Isometric,
}

#[derive(Debug, Clone)]
pub enum RenderOrder {
    RightDown,
    RightUp,
    LeftDown,
    LeftUp,
}

#[derive(Debug, Clone, Eq, PartialEq, Copy, Hash)]
pub struct TileId(u32, usize);

impl TileId {
    pub fn to_index(&self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            Some((self.0 - 1) as usize)
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LayerId {
    // global layer id and local layer id
    // global layer id is set by tiled, local layer id is generated sequentially in the order
    // that the layers are parsed
    glid: u32,
    llid: u32,
}

#[derive(Debug, Clone)]
pub struct MapMetaData {
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
    pub properties: Properties,
}

impl MapMetaData {
    pub fn from_lua(map_table: &LuaTable) -> Result<Self, Error> {
        let render_order = match map_table.get::<_, LuaString>("renderorder")?.to_str()? {
            "right-down" => RenderOrder::RightDown,
            r => return Err(anyhow!("Got an unsupported renderorder: {}", r)),
        };

        let orientation = match map_table.get::<_, LuaString>("orientation")?.to_str()? {
            "orthogonal" => Orientation::Orthogonal,
            o => return Err(anyhow!("Got an unsupported orientation: {}", o)),
        };

        Ok(MapMetaData {
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
            properties: Properties::from_lua(map_table)?,
            orientation,
            render_order,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Layer {
    layer_type: LayerType,
    id: LayerId,
    name: String,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    visible: bool,
    opacity: f64,
    offset_x: usize,
    offset_y: usize,
    properties: Properties,
    encoding: Encoding,
    data: Vec<TileId>,
}

impl Layer {
    pub fn from_lua(t: &LuaTable, llid: u32, tile_buffer: &[TileId]) -> Result<Layer, Error> {
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

        for tile in t
            .get::<_, LuaTable>("data")?
            .sequence_values::<LuaInteger>()
        {
            tile_data.push(tile_buffer[tile? as usize]);
        }

        Ok(Layer {
            id: LayerId {
                glid: t.get("id")?,
                llid,
            },
            name: t.get::<_, LuaString>("name")?.to_str()?.to_owned(),
            x: t.get("x")?,
            y: t.get("y")?,
            visible: t.get("visible")?,
            opacity: t.get("opacity")?,
            offset_x: t.get("offsetx")?,
            offset_y: t.get("offsety")?,
            data: tile_data,
            properties: Properties::from_lua(t)?,
            encoding,
            layer_type,
            width,
            height,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Map {
    pub meta_data: MapMetaData,
    pub layers: Vec<Layer>,
    pub tilesets: Tilesets,
    pub layer_map: HashMap<String, LayerId>,
}

#[derive(Debug, Clone)]
pub enum CoordSpace {
    Pixel,
    Tile,
}

impl Map {
    pub fn new(map_path: &str, engine: &Engine, path_prefix: Option<&str>) -> Result<Map, Error> {
        let mut fs = engine.fs();
        let lua = engine.lua();
        let mut tiled_lua_map = fs.open(Path::new(map_path))?;

        drop(fs);

        let mut tiled_buffer: Vec<u8> = Vec::new();
        tiled_lua_map.read_to_end(&mut tiled_buffer)?;
        let lua_chunk = lua.load(&tiled_buffer);
        let tiled_lua_table = lua_chunk.eval::<LuaTable>()?;
        let meta_data = MapMetaData::from_lua(&tiled_lua_table)?;

        let mut tilesets = Vec::new();
        let mut tile_buffer = vec![TileId(0, 0)];

        for (tileset, i) in tiled_lua_table
            .get::<_, LuaTable>("tilesets")?
            .sequence_values::<LuaTable>()
            .zip(0..)
        {
            let tileset = Tileset::from_lua(&tileset?, path_prefix, i)?;
            tile_buffer.reserve(tileset.tilecount as usize);
            for tile_id_num in tileset.first_gid..tileset.tilecount {
                tile_buffer.push(TileId(tile_id_num, i));
            }
            tilesets.push(tileset);
        }

        let mut layers = Vec::new();
        let mut layer_map = HashMap::new();

        for (layer, i) in tiled_lua_table
            .get::<_, LuaTable>("layers")?
            .sequence_values::<LuaTable>()
            .zip(0..)
        {
            let layer = Layer::from_lua(&layer?, i, &tile_buffer)?;

            layer_map.insert(layer.name.clone(), layer.id);

            layers.push(layer);
        }

        drop(tiled_lua_table);
        drop(lua);

        Ok(Map {
            meta_data,
            layers,
            tilesets: Tilesets(tilesets),
            layer_map,
        })
    }

    pub fn get_tile_at(
        &self,
        x: usize,
        y: usize,
        coordinate_space: CoordSpace,
    ) -> Vec<(TileId, LayerId)> {
        let mut tile_layer_buff = Vec::new();
        let (x, y) = match coordinate_space {
            CoordSpace::Pixel => (x / self.meta_data.tilewidth, y / self.meta_data.tileheight),
            CoordSpace::Tile => (x, y),
        };
        let offset = (self.meta_data.height * self.meta_data.width) - self.meta_data.width;

        for layer in self.layers.iter() {
            // We subtract top from y * self.meta_data.width since tiled stores it's tiles top left
            // to bottom right, and we want to index bottom left to top right
            if let Some(tile_id) = layer.data.get((offset - (y * self.meta_data.width)) + x) {
                // TODO: there should be a better way to ID a layer than this
                if tile_id.to_index().is_some() {
                    tile_layer_buff.push((*tile_id, layer.id));
                }
            }
        }
        tile_layer_buff
    }

    pub fn get_tile_in_layer(
        &self,
        x: usize,
        y: usize,
        layer: LayerId,
        coordinate_space: CoordSpace,
    ) -> Option<TileId> {
        for (tile, g_layer) in self.get_tile_at(x, y, coordinate_space) {
            if layer == g_layer {
                return Some(tile);
            }
        }
        None
    }

    pub fn get_tiles_in_bb(
        &self,
        bb: Box2<f32>,
        coordinate_space: CoordSpace,
    ) -> impl Iterator<Item = (Vec<(TileId, LayerId)>, usize, usize)> + '_ {
        let box_in_tiles = match coordinate_space {
            CoordSpace::Pixel => (
                (
                    (bb.mins.x / (self.meta_data.tilewidth as f32)).floor() as usize,
                    (bb.mins.y / (self.meta_data.tileheight as f32)).floor() as usize,
                ),
                (
                    (bb.maxs.x / (self.meta_data.tilewidth as f32)).ceil() as usize,
                    (bb.maxs.y / (self.meta_data.tileheight as f32)).ceil() as usize,
                ),
            ),

            CoordSpace::Tile => (
                (bb.mins.x as usize, bb.mins.y as usize),
                (bb.maxs.x as usize, bb.maxs.y as usize),
            ),
        };
        ((box_in_tiles.0 .1)..(box_in_tiles.1 .1)).flat_map(move |y| {
            ((box_in_tiles.0 .0)..(box_in_tiles.1 .0))
                .map(move |x| (self.get_tile_at(x, y, CoordSpace::Tile), x, y))
        })
    }

    pub fn get_tiles_in_bb_in_layer(
        &self,
        bb: Box2<f32>,
        layer_id: LayerId,
        coordinate_space: CoordSpace,
    ) -> impl Iterator<Item = (TileId, usize, usize)> + '_ {
        let box_in_tiles = match coordinate_space {
            CoordSpace::Pixel => (
                (
                    (bb.mins.x / (self.meta_data.tilewidth as f32)).floor() as usize,
                    (bb.mins.y / (self.meta_data.tileheight as f32)).floor() as usize,
                ),
                (
                    (bb.maxs.x / (self.meta_data.tilewidth as f32)).ceil() as usize,
                    (bb.maxs.y / (self.meta_data.tileheight as f32)).ceil() as usize,
                ),
            ),

            CoordSpace::Tile => (
                (bb.mins.x as usize, bb.mins.y as usize),
                (bb.maxs.x as usize, bb.maxs.y as usize),
            ),
        };
        ((box_in_tiles.0 .1)..(box_in_tiles.1 .1)).flat_map(move |y| {
            ((box_in_tiles.0 .0)..(box_in_tiles.1 .0)).filter_map(move |x| {
                self.get_tile_in_layer(x, y, layer_id, CoordSpace::Tile)
                    .map(|t| (t, x, y))
            })
        })
    }
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
        map_meta_data: &MapMetaData,
    ) -> Self {
        // We need 1 sprite batch per texture
        let mut batches = Vec::with_capacity(ts_atlas.textures.len());
        let graphics_lock = engine.get::<GraphicsLock>();

        for texture in ts_atlas.textures.iter() {
            let mut acquired_lock = GraphicsLockExt::lock(&graphics_lock);
            batches.push(SpriteBatch::new(&mut acquired_lock, texture.clone()));
            drop(acquired_lock);
        }
        let top = layer.height * map_meta_data.tileheight;

        for y_cord in 0..layer.height {
            for x_cord in 0..layer.width {
                let tile = layer.data[y_cord * layer.width + x_cord];
                // Tile indices start at 1, 0 represents no tile, so we offset the tile by 1
                // first, and skip making the instance param if the tile is 0
                if let Some(index) = tile.to_index() {
                    let uvs = ts_atlas.render_data[index];
                    batches[tile.1].insert(
                        Instance::new()
                            .src(uvs)
                            .color(Color::new(1.0, 1.0, 1.0, layer.opacity as f32))
                            .translate2(Vector2::new(
                                (x_cord * map_meta_data.tilewidth) as f32,
                                // Need to offset by 1 here since tiled renders maps top right to bottom left, but we do bottom left to top right
                                (top - ((y_cord + 1) * map_meta_data.tileheight)) as f32,
                            )),
                    );
                }
            }
        }
        LayerBatch(batches)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ObjectShape {
    Rect { width: f32, height: f32 },
    Ellipse { width: f32, height: f32 },
    Polyline { points: Vec<(f32, f32)> },
    Polygon { points: Vec<(f32, f32)> },
    Point(f32, f32),
}

impl ObjectShape {
    pub fn from_lua(shape_table: &LuaTable) -> Result<Self, Error> {
        match shape_table.get::<_, LuaString>("shape")?.to_str()? {
            "rectangle" => Ok(ObjectShape::Rect {
                width: shape_table.get("width")?,
                height: shape_table.get("height")?,
            }),
            "ellipse" => Ok(ObjectShape::Ellipse {
                width: shape_table.get("width")?,
                height: shape_table.get("height")?,
            }),
            s if s == "point" || s == "polygon" || s == "polyline" => {
                Err(anyhow!("{} objects aren't supported yet, ping Maxim", s))
            }
            e => Err(anyhow!("Got an unsupported shape type: {}", e)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Object {
    pub id: u32,
    pub name: String,
    pub obj_type: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub gid: Option<u32>,
    pub visible: bool,
    pub shape: ObjectShape,
    pub properties: Properties,
}

impl Object {
    pub fn from_lua(obj_table: &LuaTable) -> Result<Self, Error> {
        Ok(Object {
            id: obj_table.get("id")?,
            name: obj_table.get::<_, LuaString>("name")?.to_str()?.to_owned(),
            obj_type: obj_table.get::<_, LuaString>("type")?.to_str()?.to_owned(),
            x: obj_table.get("x")?,
            y: obj_table.get("y")?,
            width: obj_table.get("width")?,
            height: obj_table.get("height")?,
            shape: ObjectShape::from_lua(obj_table)?,
            properties: Properties::from_lua(obj_table)?,
            rotation: obj_table.get("rotation")?,
            visible: obj_table.get("visible")?,
            gid: obj_table.get("gid").ok(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ObjectGroup {
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    pub objects: Vec<Object>,
    pub color: Option<Color>,
    /**
     * Layer index is not preset for tile collision boxes
     */
    pub layer_index: Option<u32>,
    pub properties: Properties,
}

impl ObjectGroup {
    pub fn from_lua(objg_table: &LuaTable) -> Result<Self, Error> {
        let name = objg_table.get("name")?;
        let opacity = objg_table.get("opacity")?;
        let visible = objg_table.get("visible")?;
        let color = objg_table.get("color").ok();
        let layer_index = objg_table.get("layer_index").ok();
        let properties = Properties::from_lua(objg_table)?;

        let mut objects = Vec::new();
        for object in objg_table.get::<_, LuaTable>("objects")?.sequence_values() {
            objects.push(Object::from_lua(&object?)?);
        }

        Ok(ObjectGroup {
            name,
            opacity,
            visible,
            color,
            layer_index,
            properties,
            objects,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Animation;

#[derive(Debug, Clone)]
pub struct Tile {
    pub id: TileId,
    pub tile_type: Option<String>,
    pub probability: f32,
    pub properties: Properties,
    pub objectgroup: Option<ObjectGroup>,
    pub animation: Option<Animation>,
}

impl Tile {
    pub fn from_lua(tile_table: &LuaTable, tileset_num: usize) -> Result<Self, Error> {
        let objectgroup = match tile_table.get::<_, LuaTable>("objectGroup") {
            Ok(t) => Some(ObjectGroup::from_lua(&t)?),
            Err(_) => None,
        };

        Ok(Tile {
            // We have to add 1 here, because Tiled Data stores TileIds + 1, so for consistency,
            // we add 1 here
            id: TileId(
                tile_table.get::<_, LuaInteger>("id")? as u32 + 1,
                tileset_num,
            ),
            tile_type: tile_table.get("type").ok(),
            probability: tile_table.get("probability").unwrap_or(0.0),
            animation: None,
            properties: match tile_table.get("properties") {
                Ok(t) => Properties::from_lua(&t)?,
                Err(_) => Properties(HashMap::new()),
            },
            objectgroup,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Image {
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub trans_color: Option<Color>,
}

impl Image {
    pub fn new(it: &LuaTable, prefix: Option<&str>) -> Result<Self, Error> {
        Ok(Image {
            source: prefix.unwrap_or("").to_owned() + it.get::<_, LuaString>("image")?.to_str()?,
            width: it.get("imagewidth")?,
            height: it.get("imageheight")?,
            trans_color: match it.get::<_, LuaString>("transparentcolor") {
                Ok(s) => {
                    log::warn!(
                    "Transparent colors aren't supported, courtesy of Shea, add support yourself. Color: {}", s.to_str()?
                );
                    None
                }
                _ => None,
            },
        })
    }
}

#[derive(Debug, Clone)]
pub struct Tileset {
    pub first_gid: u32,
    pub name: String,
    pub tile_width: u32,
    pub tile_height: u32,
    pub spacing: u32,
    pub margin: u32,
    pub tilecount: u32,
    pub columns: u32,
    pub tiles: HashMap<TileId, Tile>,
    pub properties: Properties,
    pub images: Vec<Image>,
}

impl Tileset {
    pub fn from_lua(
        ts: &LuaTable,
        path_prefix: Option<&str>,
        tileset_number: usize,
    ) -> Result<Tileset, Error> {
        let mut tiles = HashMap::new();
        for tile_table in ts.get::<_, LuaTable>("tiles")?.sequence_values() {
            let tile = Tile::from_lua(&tile_table?, tileset_number)?;
            tiles.insert(tile.id, tile);
        }

        Ok(Tileset {
            name: ts.get::<_, LuaString>("name")?.to_str()?.to_owned(),
            first_gid: ts.get("firstgid")?,
            tile_width: ts.get("tilewidth")?,
            tile_height: ts.get("tileheight")?,
            spacing: ts.get("spacing")?,
            margin: ts.get("margin")?,
            columns: ts.get("columns")?,
            images: vec![Image::new(ts, path_prefix)?],
            tilecount: ts.get("tilecount")?,
            properties: Properties::from_lua(ts)?,
            tiles,
        })
    }

    fn get_tile(&self, tile_id: &TileId) -> Option<&Tile> {
        self.tiles.get(tile_id)
    }
}

#[derive(Debug, Clone)]
pub struct Tilesets(Vec<Tileset>);

impl Tilesets {
    pub fn get_tile(&self, tile_id: &TileId) -> Option<&Tile> {
        self.0[tile_id.1].get_tile(tile_id)
    }
}

pub struct TilesetAtlas {
    // Box2<f32> is the uvs, the second usize is an index into a texture vec
    // that relates the uv to the texture
    render_data: Vec<Box2<f32>>,
    textures: Vec<CachedTexture>,
}

impl TilesetAtlas {
    pub fn new(tilesets: &Tilesets, engine: &Engine) -> Result<Self, Error> {
        let mut textures = Vec::with_capacity(tilesets.0.len());
        let mut render_data = Vec::new();

        for tileset in tilesets.0.iter() {
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

            let rows = tileset.tilecount / tileset.columns;
            let top = (rows * (tileset.spacing + tileset.tile_height)) + tileset.margin;
            for row in 1..=rows {
                for column in 0..tileset.columns {
                    render_data.push(Box2::new(
                        (tileset.margin
                            + ((column * tileset.tile_width) + column * tileset.spacing))
                            as f32
                            / texture_obj.width() as f32,
                        (tileset.spacing
                            + (top
                                - (tileset.margin
                                    + ((row * tileset.tile_height) + row * tileset.spacing))))
                            as f32
                            / texture_obj.height() as f32,
                        tileset.tile_width as f32 / texture_obj.width() as f32,
                        tileset.tile_height as f32 / texture_obj.height() as f32,
                    ));
                }
            }
            textures.push(CachedTexture::from(texture_obj));
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
