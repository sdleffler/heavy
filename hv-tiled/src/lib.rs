use hv_core::{
    prelude::*,
};

use std::collections::HashMap;

pub use tiled;

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

#[derive(Debug)]
pub struct Tile(u32);

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
    data: Vec<Tile>
}

impl Layer {
    pub fn from_lua_table(t: LuaTable) -> Result<Layer, Error> {
        let layer_type = match t.get::<_, LuaString>("type")?.to_str()? {
            "tilelayer" => LayerType::Tile,
            s => return Err(anyhow!("Got an unsupported tilelayer type: {}", s))
        };

        let encoding = match t.get::<_, LuaString>("encoding")?.to_str()? {
            "lua" => Encoding::Lua,
            e => return Err(anyhow!("Got an unsupported encoding type: {}", e))
        };

        let width = t.get("width")?;
        let height = t.get("height")?;
        let mut tile_data = Vec::with_capacity(width * height);

        for tile in t.get::<_, LuaTable>("data")?.sequence_values() {
            tile_data.push(Tile(tile?));
        }

        let mut properties = HashMap::new();

        for pair_res in t.get::<_, LuaTable>("properties")?.pairs() {
            let pair = pair_res?;
            let val = match pair.1 {
                LuaValue::Boolean(b) => Property::Bool(b),
                LuaValue::Integer(i) => Property::Int(i),
                LuaValue::Number(n) => Property::Float(n),
                LuaValue::String(s) => Property::String(s.to_str()?.to_owned()),
                l => return Err(anyhow!("Got an unexpected value in the properties section: {:?}", l)),
            };
            properties.insert(pair.0, val);
        }

        Ok(Layer {
            id : t.get("id")?,
            name : t.get::<_, LuaString>("name")?.to_str()?.to_owned(),
            x : t.get("x")?,
            y : t.get("y")?,
            visible: t.get("visible")?,
            opacity : t.get("opacity")?,
            offset_x : t.get("offsetx")?,
            offset_y : t.get("offsety")?,
            data : tile_data,
            encoding: encoding,
            layer_type : layer_type,
            width : width,
            height: height,
            properties : properties,
        })
    }
}