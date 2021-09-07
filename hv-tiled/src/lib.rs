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
    pub properties: Properties,
}

impl MapData {
    pub fn from_lua(map_table: &LuaTable) -> Result<Self, Error> {
        let render_order = match map_table.get::<_, LuaString>("renderorder")?.to_str()? {
            "right-down" => RenderOrder::RightDown,
            r => return Err(anyhow!("Got an unsupported renderorder: {}", r)),
        };

        let orientation = match map_table.get::<_, LuaString>("orientation")?.to_str()? {
            "orthogonal" => Orientation::Orthogonal,
            o => return Err(anyhow!("Got an unsupported orientation: {}", o)),
        };

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
            properties: Properties::from_lua(map_table)?,
            orientation,
            render_order,
        })
    }
}

#[derive(Debug, Clone)]
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
    properties: Properties,
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
                            // Need to offset by 1 here since tiled renders maps top right to bottom left, but we do bottom left to top right
                            (top - ((y_cord + 1) * map_data.tileheight)) as f32,
                        )),
                );
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
    pub id: u32,
    pub tile_type: Option<String>,
    pub probability: f32,
    pub properties: Properties,
    pub objectgroup: Option<ObjectGroup>,
    pub animation: Option<Animation>,
}

impl Tile {
    pub fn from_lua(tile_table: &LuaTable) -> Result<Self, Error> {
        let objectgroup = match tile_table.get::<_, LuaTable>("objectGroup") {
            Ok(t) => Some(ObjectGroup::from_lua(&t)?),
            Err(_) => None,
        };

        Ok(Tile {
            id: tile_table.get("id")?,
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
    pub fn new(it: &LuaTable) -> Result<Self, Error> {
        Ok(Image {
            source: it.get::<_, LuaString>("image")?.to_str()?.to_owned(),
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
    pub tilecount: Option<u32>,
    pub columns: u32,
    pub tiles: Vec<Tile>,
    pub properties: Properties,
    pub images: Vec<Image>,
}

impl Tileset {
    pub fn from_lua(ts: &LuaTable) -> Result<Tileset, Error> {
        let mut tiles = Vec::new();
        for tile_table in ts.get::<_, LuaTable>("tiles")?.sequence_values() {
            tiles.push(Tile::from_lua(&tile_table?)?);
        }

        Ok(Tileset {
            name: ts.get::<_, LuaString>("name")?.to_str()?.to_owned(),
            first_gid: ts.get("firstgid")?,
            tile_width: ts.get("tilewidth")?,
            tile_height: ts.get("tileheight")?,
            spacing: ts.get("spacing")?,
            margin: ts.get("margin")?,
            columns: ts.get("columns")?,
            images: vec![Image::new(ts)?],
            tilecount: ts.get("tilecount")?,
            properties: Properties::from_lua(ts)?,
            tiles,
        })
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
    pub fn new(tilesets: Vec<Tileset>, engine: &Engine) -> Result<Self, Error> {
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
    pub fn from_lua(t: &LuaTable) -> Result<Layer, Error> {
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
            properties: Properties::from_lua(t)?,
            encoding,
            layer_type,
            width,
            height,
        })
    }
}
