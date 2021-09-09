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
    Object,
}

impl LayerType {
    pub fn from_lua(t: &LuaTable) -> Result<Self, Error> {
        match t.get::<_, LuaString>("type")?.to_str()? {
            "objectgroup" => Ok(LayerType::Object),
            "tilelayer" => Ok(LayerType::Tile),
            s => Err(anyhow!("Unsupported layer type: {}", s)),
        }
    }
}

// TODO: This type was pulled from the Tiled crate, but the Color and File variants
// are never constructed. This might be a bug depending on what the "properties"
// table contains
#[derive(Debug, PartialEq, Clone)]
pub enum Property {
    Bool(bool),
    Float(f64),
    Int(i64),
    String(String),
    Obj(ObjectId),
    Color(u32),
    File(String),
}

pub trait BoxExt {
    fn floor_to_u32(self) -> Box2<u32>;
    fn to_pixel_space(self, map_md: &MapMetaData) -> Box2<u32>;
}

impl BoxExt for Box2<f32> {
    fn floor_to_u32(self) -> Box2<u32> {
        Box2::new(
            self.mins.x as u32,
            self.mins.y as u32,
            (self.maxs.x - self.mins.x) as u32,
            (self.maxs.y - self.mins.y) as u32,
        )
    }

    fn to_pixel_space(self, map_md: &MapMetaData) -> Box2<u32> {
        self.floor_to_u32().to_pixel_space(map_md)
    }
}

impl BoxExt for Box2<u32> {
    fn floor_to_u32(self) -> Box2<u32> {
        self
    }

    fn to_pixel_space(self, map_md: &MapMetaData) -> Box2<u32> {
        Box2::new(
            self.mins.x / map_md.tilewidth,
            self.mins.y / map_md.tileheight,
            (self.maxs.x - self.mins.x) / map_md.tilewidth,
            (self.maxs.y - self.mins.y) / map_md.tileheight,
        )
    }
}

pub trait ColorExt {
    fn from_tiled_hex(hex: &str) -> Result<Color, Error>;
    fn from_tiled_lua_table(c_t: &LuaTable) -> Result<Color, Error>;
}

impl ColorExt for Color {
    fn from_tiled_hex(hex: &str) -> Result<Color, Error> {
        Ok(Color::from_rgb_u32(u32::from_str_radix(
            hex.trim_start_matches('#'),
            16,
        )?))
    }

    fn from_tiled_lua_table(c_t: &LuaTable) -> Result<Color, Error> {
        match c_t.get::<_, LuaTable>("color") {
            Ok(t) => {
                let mut iter = t.sequence_values();
                let r = iter
                    .next()
                    .ok_or_else(|| anyhow!("Should've gotten a value for R, got nothing"))??;
                let g = iter
                    .next()
                    .ok_or_else(|| anyhow!("Should've gotten a value for G, got nothing"))??;
                let b = iter
                    .next()
                    .ok_or_else(|| anyhow!("Should've gotten a value for B, got nothing"))??;
                Ok(Color::from_rgb(r, g, b))
            }
            Err(_) => Ok(Color::BLACK),
        }
    }
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
                LuaValue::Table(t) => Property::Obj(ObjectId::new(t.get("id")?, false)), // I believe tables will only come through for Object properties
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
    pub width: u32,
    pub height: u32,
    pub tilewidth: u32,
    pub tileheight: u32,
    pub nextlayerid: u32,
    pub nextobjectid: u32,
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
            nextlayerid: map_table.get::<_, LuaInteger>("nextlayerid")? as u32,
            nextobjectid: map_table.get::<_, LuaInteger>("nextobjectid")? as u32,
            properties: Properties::from_lua(map_table)?,
            orientation,
            render_order,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TileLayer {
    layer_type: LayerType,
    id: LayerId,
    name: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    visible: bool,
    opacity: f64,
    offset_x: u32,
    offset_y: u32,
    properties: Properties,
    encoding: Encoding,
    data: Vec<TileId>,
}

impl TileLayer {
    pub fn from_lua(t: &LuaTable, llid: u32, tile_buffer: &[TileId]) -> Result<TileLayer, Error> {
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
        let mut tile_data = Vec::with_capacity((width * height) as usize);

        for tile in t
            .get::<_, LuaTable>("data")?
            .sequence_values::<LuaInteger>()
        {
            tile_data.push(tile_buffer[tile? as usize]);
        }

        Ok(TileLayer {
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

type ObjectLayer = ObjectGroup;

#[derive(Debug, Clone)]
pub struct Map {
    pub meta_data: MapMetaData,
    pub tile_layers: Vec<TileLayer>,
    pub object_layers: Vec<ObjectLayer>,
    pub tilesets: Tilesets,
    pub layer_map: HashMap<String, LayerId>,
    obj_slab: slab::Slab<Object>,
    obj_id_to_ref_map: HashMap<ObjectId, ObjectRef>,
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
        let mut obj_slab = slab::Slab::new();

        for (tileset, i) in tiled_lua_table
            .get::<_, LuaTable>("tilesets")?
            .sequence_values::<LuaTable>()
            .zip(0..)
        {
            let tileset = Tileset::from_lua(&tileset?, path_prefix, i, &mut obj_slab)?;
            tile_buffer.reserve(tileset.tilecount as usize);
            for tile_id_num in tileset.first_gid..tileset.tilecount {
                tile_buffer.push(TileId(tile_id_num, i));
            }
            tilesets.push(tileset);
        }

        let mut tile_layers = Vec::new();
        let mut object_layers = Vec::new();

        let mut layer_map = HashMap::new();
        let mut obj_id_to_ref_map = HashMap::new();

        for (layer, i) in tiled_lua_table
            .get::<_, LuaTable>("layers")?
            .sequence_values::<LuaTable>()
            .zip(0..)
        {
            let layer = layer?;
            let layer_type = LayerType::from_lua(&layer)?;
            match layer_type {
                LayerType::Tile => {
                    let tile_layer = TileLayer::from_lua(&layer, i, &tile_buffer)?;
                    layer_map.insert(tile_layer.name.clone(), tile_layer.id);
                    tile_layers.push(tile_layer);
                }
                LayerType::Object => {
                    let (obj_group, obj_ids_and_refs) =
                        ObjectGroup::from_lua(&layer, i, true, &mut obj_slab)?;
                    for (obj_id, obj_ref) in obj_ids_and_refs.iter() {
                        obj_id_to_ref_map.insert(obj_id.clone(), *obj_ref);
                    }
                    layer_map.insert(obj_group.name.clone(), obj_group.id);
                    object_layers.push(obj_group);
                }
            }
        }

        drop(tiled_lua_table);
        drop(lua);

        Ok(Map {
            meta_data,
            tile_layers,
            tilesets: Tilesets(tilesets),
            object_layers,
            layer_map,
            obj_slab,
            obj_id_to_ref_map,
        })
    }

    pub fn get_tile_at(
        &self,
        x: u32,
        y: u32,
        coordinate_space: CoordSpace,
    ) -> Vec<(TileId, LayerId)> {
        let mut tile_layer_buff = Vec::new();
        let (x, y) = match coordinate_space {
            CoordSpace::Pixel => (x / self.meta_data.tilewidth, y / self.meta_data.tileheight),
            CoordSpace::Tile => (x, y),
        };
        let offset = (self.meta_data.height * self.meta_data.width) - self.meta_data.width;

        for layer in self.tile_layers.iter() {
            // We subtract top from y * self.meta_data.width since tiled stores it's tiles top left
            // to bottom right, and we want to index bottom left to top right
            if let Some(tile_id) = layer
                .data
                .get(((offset - (y * self.meta_data.width)) + x) as usize)
            {
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
        x: u32,
        y: u32,
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
        bb: Box2<u32>,
        coordinate_space: CoordSpace,
    ) -> impl Iterator<Item = (Vec<(TileId, LayerId)>, u32, u32)> + '_ {
        let box_in_tiles = match coordinate_space {
            CoordSpace::Pixel => (
                (
                    (bb.mins.x / (self.meta_data.tilewidth)),
                    (bb.mins.y / (self.meta_data.tileheight)),
                ),
                (
                    (bb.maxs.x as f32 / (self.meta_data.tilewidth as f32)).ceil() as u32,
                    (bb.maxs.y as f32 / (self.meta_data.tileheight as f32)).ceil() as u32,
                ),
            ),

            CoordSpace::Tile => ((bb.mins.x, bb.mins.y), (bb.maxs.x, bb.maxs.y)),
        };
        ((box_in_tiles.0 .1)..=(box_in_tiles.1 .1)).flat_map(move |y| {
            ((box_in_tiles.0 .0)..=(box_in_tiles.1 .0))
                .map(move |x| (self.get_tile_at(x, y, CoordSpace::Tile), x, y))
        })
    }

    pub fn get_tiles_in_bb_in_layer(
        &self,
        bb: Box2<u32>,
        layer_id: LayerId,
        coordinate_space: CoordSpace,
    ) -> impl Iterator<Item = (TileId, u32, u32)> + '_ {
        let box_in_tiles = match coordinate_space {
            CoordSpace::Pixel => (
                (
                    (bb.mins.x / (self.meta_data.tilewidth)),
                    (bb.mins.y / (self.meta_data.tileheight)),
                ),
                (
                    (bb.maxs.x as f32 / (self.meta_data.tilewidth as f32)).ceil() as u32,
                    (bb.maxs.y as f32 / (self.meta_data.tileheight as f32)).ceil() as u32,
                ),
            ),

            CoordSpace::Tile => ((bb.mins.x, bb.mins.y), (bb.maxs.x, bb.maxs.y)),
        };
        ((box_in_tiles.0 .1)..=(box_in_tiles.1 .1)).flat_map(move |y| {
            ((box_in_tiles.0 .0)..=(box_in_tiles.1 .0)).filter_map(move |x| {
                self.get_tile_in_layer(x, y, layer_id, CoordSpace::Tile)
                    .map(|t| (t, x, y))
            })
        })
    }

    pub fn get_obj(&self, obj_ref: &ObjectRef) -> &Object {
        &self.obj_slab[obj_ref.0]
    }
}

// TODO: implement this struct. How do we want to draw objects?
pub struct ObjectLayerBatch;

pub struct TileLayerBatch(Vec<SpriteBatch>);

impl DrawableMut for TileLayerBatch {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        for batch in self.0.iter_mut() {
            batch.draw_mut(ctx, instance);
        }
    }
}

impl TileLayerBatch {
    pub fn new(
        layer: &TileLayer,
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
                let tile = layer.data[(y_cord * layer.width + x_cord) as usize];
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
        TileLayerBatch(batches)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ObjectShape {
    Rect,
    Ellipse,
    Polyline { points: Vec<(f32, f32)> },
    Polygon { points: Vec<(f32, f32)> },
    Point,
}

impl ObjectShape {
    pub fn from_string(s: &str) -> Result<Self, Error> {
        match s {
            "rectangle" => Ok(ObjectShape::Rect),
            "ellipse" => Ok(ObjectShape::Ellipse),
            "point" => Ok(ObjectShape::Point),
            s if s == "polygon" || s == "polyline" => {
                Err(anyhow!("{} objects aren't supported yet, ping Maxim", s))
            }
            e => Err(anyhow!("Got an unsupported shape type: {}", e)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DrawOrder {
    TopDown,
    Index,
}

impl DrawOrder {
    fn from_lua(t: &LuaTable) -> Result<Self, Error> {
        match t.get::<_, LuaString>("draworder")?.to_str()? {
            "topdown" => Ok(DrawOrder::TopDown),
            "index" => Ok(DrawOrder::Index),
            s => Err(anyhow!("Unsupported draw order: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
enum Halign {
    Left,
    Center,
    Right,
    Justify,
}

impl Halign {
    pub fn from_lua(t: &LuaTable) -> Result<Self, Error> {
        match t.get::<_, LuaString>("halign") {
            Ok(s) => match s.to_str()? {
                "left" => Ok(Halign::Left),
                "center" => Ok(Halign::Center),
                "right" => Ok(Halign::Right),
                "justify" => Ok(Halign::Justify),
                s => Err(anyhow!("Unsupported halign value: {}", s)),
            },
            Err(_) => Ok(Halign::Left),
        }
    }
}

#[derive(Debug, Clone)]
enum Valign {
    Top,
    Center,
    Bottom,
}

impl Valign {
    pub fn from_lua(t: &LuaTable) -> Result<Self, Error> {
        match t.get::<_, LuaString>("valign") {
            Ok(s) => match s.to_str()? {
                "top" => Ok(Valign::Top),
                "center" => Ok(Valign::Center),
                "bottom" => Ok(Valign::Bottom),
                s => Err(anyhow!("Unsupported valign value: {}", s)),
            },
            Err(_) => Ok(Valign::Top),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Text {
    wrapping: bool,
    text: String,
    fontfamily: String,
    pixelsize: u32,
    color: Color,
    bold: bool,
    italic: bool,
    underline: bool,
    strikeout: bool,
    kerning: bool,
    halign: Halign,
    valign: Valign,
}

impl Text {
    pub fn from_lua(t_table: &LuaTable) -> Result<Self, Error> {
        let fontfamily = match t_table.get::<_, LuaString>("fontfamily") {
            Ok(s) => s.to_str()?.to_owned(),
            Err(_) => "sans-serif".to_owned(),
        };

        Ok(Text {
            text: t_table.get::<_, LuaString>("text")?.to_str()?.to_owned(),
            pixelsize: t_table.get("pixelsize").unwrap_or(16),
            wrapping: t_table.get("wrapping").unwrap_or(false),
            color: Color::from_tiled_lua_table(t_table)?,
            bold: t_table.get("bold").unwrap_or(false),
            italic: t_table.get("italic").unwrap_or(false),
            underline: t_table.get("underline").unwrap_or(false),
            strikeout: t_table.get("strikeout").unwrap_or(false),
            kerning: t_table.get("kerning").unwrap_or(true),
            halign: Halign::from_lua(t_table)?,
            valign: Valign::from_lua(t_table)?,
            fontfamily,
        })
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct ObjectId {
    id: u32,
    from_obj_layer: bool,
}

impl ObjectId {
    fn new(id: u32, from_obj_layer: bool) -> Self {
        ObjectId { id, from_obj_layer }
    }

    pub fn tainted_new(id: u32) -> Self {
        ObjectId {
            id,
            from_obj_layer: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectRef(usize);

#[derive(Debug, Clone)]
pub struct Object {
    pub id: ObjectId,
    pub name: String,
    pub obj_type: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub gid: Option<u32>,
    pub visible: bool,
    pub properties: Properties,
    pub shape: Option<ObjectShape>,
    pub text: Option<Text>,
}

// For some reason, in the lua encoding, text is stored under shape
// Why????? In any case I made this type to store both a text and an
// actual shape object
enum LuaShapeResolution {
    Text(Text),
    ObjectShape(ObjectShape),
}

impl Object {
    pub fn from_lua(obj_table: &LuaTable, from_obj_layer: bool) -> Result<Self, Error> {
        let lua_shape_res = match obj_table.get::<_, LuaString>("shape")?.to_str()? {
            "text" => LuaShapeResolution::Text(Text::from_lua(obj_table)?),
            s => LuaShapeResolution::ObjectShape(ObjectShape::from_string(s)?),
        };

        let (shape, text) = match lua_shape_res {
            LuaShapeResolution::ObjectShape(s) => (Some(s), None),
            LuaShapeResolution::Text(t) => (None, Some(t)),
        };
        Ok(Object {
            id: ObjectId::new(obj_table.get("id")?, from_obj_layer),
            name: obj_table.get::<_, LuaString>("name")?.to_str()?.to_owned(),
            obj_type: obj_table.get::<_, LuaString>("type")?.to_str()?.to_owned(),
            x: obj_table.get("x")?,
            y: obj_table.get("y")?,
            width: obj_table.get("width")?,
            height: obj_table.get("height")?,
            properties: Properties::from_lua(obj_table)?,
            rotation: obj_table.get("rotation")?,
            visible: obj_table.get("visible")?,
            gid: obj_table.get("gid").ok(),
            shape,
            text,
        })
    }
}

#[derive(Debug, Clone)]
pub enum ObjGroupType {
    ObjectGroup,
}

impl ObjGroupType {
    pub fn from_lua(t: &LuaTable) -> Result<Self, Error> {
        match t.get::<_, LuaString>("type")?.to_str()? {
            "objectgroup" => Ok(ObjGroupType::ObjectGroup),
            s => Err(anyhow!("Unsupported object group type: {}", s)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ObjectGroup {
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    pub draworder: DrawOrder,
    pub object_refs: Vec<ObjectRef>,
    pub color: Color,
    pub id: LayerId,
    pub obj_group_type: ObjGroupType,
    /**
     * Layer index is not preset for tile collision boxes
     */
    pub layer_index: Option<u32>,
    pub properties: Properties,
    pub tintcolor: Option<Color>,
    pub off_x: u32,
    pub off_y: u32,
}

impl ObjectGroup {
    pub fn from_lua(
        objg_table: &LuaTable,
        llid: u32,
        from_obj_layer: bool,
        slab: &mut slab::Slab<Object>,
    ) -> Result<(Self, Vec<(ObjectId, ObjectRef)>), Error> {
        let mut obj_ids_and_refs = Vec::new();

        for object in objg_table.get::<_, LuaTable>("objects")?.sequence_values() {
            let object = Object::from_lua(&object?, from_obj_layer)?;

            obj_ids_and_refs.push((object.id.clone(), ObjectRef(slab.insert(object))));
        }

        let color = match objg_table.get::<_, LuaString>("color") {
            Ok(s) => Color::from_tiled_hex(s.to_str()?)?,
            Err(_) => Color::from_rgb(0xA0, 0xA0, 0x0A4),
        };

        Ok((
            ObjectGroup {
                id: LayerId {
                    glid: objg_table.get("id")?,
                    llid,
                },
                name: objg_table.get("name")?,
                opacity: objg_table.get("opacity")?,
                visible: objg_table.get("visible")?,
                layer_index: objg_table.get("layer_index").ok(),
                properties: Properties::from_lua(objg_table)?,
                draworder: DrawOrder::from_lua(objg_table)?,
                obj_group_type: ObjGroupType::from_lua(objg_table)?,
                tintcolor: objg_table.get("tintcolor").ok(),
                off_x: objg_table.get("offsetx").unwrap_or(0),
                off_y: objg_table.get("offsety").unwrap_or(0),
                object_refs: obj_ids_and_refs.iter().map(|i| i.1).collect(),
                color,
            },
            obj_ids_and_refs,
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
// The u32 here represents the duration, TileId is which TileId is assocated with said duration
pub struct Animation(Vec<(u32, TileId)>);

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
    pub fn from_lua(
        tile_table: &LuaTable,
        tileset_num: usize,
        slab: &mut slab::Slab<Object>,
    ) -> Result<Self, Error> {
        let objectgroup = match tile_table.get::<_, LuaTable>("objectGroup") {
            Ok(t) => Some(ObjectGroup::from_lua(&t, u32::MAX, false, slab)?.0),
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
            properties: match tile_table.get::<_, LuaTable>("properties") {
                Ok(_) => Properties::from_lua(tile_table)?,
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
    // Note that although this is parsed, it's not actually used lmao TODO
    pub trans_color: Option<Color>,
}

impl Image {
    pub fn new(it: &LuaTable, prefix: Option<&str>) -> Result<Self, Error> {
        Ok(Image {
            source: prefix.unwrap_or("").to_owned() + it.get::<_, LuaString>("image")?.to_str()?,
            width: it.get("imagewidth")?,
            height: it.get("imageheight")?,
            trans_color: match it.get::<_, LuaString>("transparentcolor") {
                Ok(s) => Some(Color::from_tiled_hex(s.to_str()?)?),
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
        slab: &mut slab::Slab<Object>,
    ) -> Result<Tileset, Error> {
        let mut tiles = HashMap::new();
        for tile_table in ts.get::<_, LuaTable>("tiles")?.sequence_values() {
            let tile = Tile::from_lua(&tile_table?, tileset_number, slab)?;
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
