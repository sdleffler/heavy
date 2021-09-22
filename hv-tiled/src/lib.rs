pub mod lua_parser;
pub mod object_layer;
pub mod render;
pub mod tile_layer;

use crate::lua_parser::ColorExt;
use crate::object_layer::*;
pub use crate::render::*;
use crate::tile_layer::*;

use hv_core::{engine::Engine, prelude::*};

use hv_friends::{
    graphics::{
        sprite::{AnimationState, Direction, Frame, SpriteSheet, Tag, TagId},
        CachedTexture, Color, Drawable, DrawableMut, Graphics, GraphicsLock, GraphicsLockExt,
        Instance, SpriteBatch, SpriteId, Texture,
    },
    math::Box2,
    math::Vector2,
};

use std::{collections::HashMap, io::Read, path::Path};

const EMPTY_TILE: TileId = TileId(0, TileMetaData(0));
const CHUNK_SIZE: u32 = 16;

#[derive(Debug, Clone)]
pub enum LayerType {
    Tile,
    Object,
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
    Color(String),
    File(String),
}

macro_rules! as_rust_type {
    ( $fun_name:ident, $return_type:ty, $error_name: literal, $enum_var:ident ) => {
        pub fn $fun_name(&self) -> Result<$return_type> {
            match self {
                Property::$enum_var(e) => Ok(e),
                p => Err(anyhow!("Attempted to get a {} from a {:?}", $error_name, p)),
            }
        }
    };
}

impl Property {
    as_rust_type!(as_bool, &bool, "bool", Bool);
    as_rust_type!(as_float, &f64, "float", Float);
    as_rust_type!(as_int, &i64, "int", Int);
    as_rust_type!(as_str, &str, "string", String);
    as_rust_type!(as_obj_id, &ObjectId, "object", Obj);
    as_rust_type!(as_file, &str, "file", File);

    pub fn as_color(&self) -> Result<Color> {
        match self {
            Property::Color(c) => Ok(Color::from_tiled_hex(c)?),
            p => Err(anyhow!("Attempted to get a color from a {:?}", p)),
        }
    }
}

pub trait BoxExt {
    fn floor_to_i32(self) -> Box2<i32>;
    fn to_pixel_space(self, map_md: &MapMetaData) -> Box2<i32>;
}

impl BoxExt for Box2<f32> {
    fn floor_to_i32(self) -> Box2<i32> {
        Box2::new(
            self.mins.x as i32,
            self.mins.y as i32,
            (self.maxs.x - self.mins.x) as i32,
            (self.maxs.y - self.mins.y) as i32,
        )
    }

    fn to_pixel_space(self, map_md: &MapMetaData) -> Box2<i32> {
        self.floor_to_i32().to_pixel_space(map_md)
    }
}

impl BoxExt for Box2<i32> {
    fn floor_to_i32(self) -> Box2<i32> {
        self
    }

    fn to_pixel_space(self, map_md: &MapMetaData) -> Box2<i32> {
        Box2::new(
            self.mins.x / (map_md.tilewidth as i32),
            self.mins.y / (map_md.tileheight as i32),
            (self.maxs.x - self.mins.x) / (map_md.tilewidth as i32),
            (self.maxs.y - self.mins.y) / (map_md.tileheight as i32),
        )
    }
}

#[derive(Debug, Clone)]
pub struct Properties(HashMap<String, Property>);

impl Properties {
    pub fn get_property(&self, key: &str) -> Option<&Property> {
        self.0.get(key)
    }
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

bitfield::bitfield! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
    pub struct TileMetaData(u32);
    pub flipx,                   _ : 31;
    pub flipy,                   _ : 30;
    pub diag_flip,               _ : 29;
    pub tileset_id, set_tileset_id : 28, 0;
}

impl TileMetaData {
    pub fn new(tileset_id: u32, flipx: bool, flipy: bool, diagonal_flip: bool) -> TileMetaData {
        assert_eq!(tileset_id >> 29, 0);
        TileMetaData(
            (flipx as u32) << 31 | (flipy as u32) << 30 | (diagonal_flip as u32) << 29 | tileset_id,
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Copy, Hash)]
pub struct TileId(u32, TileMetaData);

impl TileId {
    pub fn to_index(&self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            Some((self.0 - 1) as usize)
        }
    }

    // Input the tile id here as is found in tiled
    pub fn new(
        tile_id: u32,
        tileset_id: u32,
        flipx: bool,
        flipy: bool,
        diagonal_flip: bool,
    ) -> TileId {
        // If any of the top 3 bits of the tileset_id are stored, panic. We can't have
        // tileset ids that are larger than 29 bits due to the top 3 bits being reserved for
        // flip data
        TileId(
            tile_id + 1,
            TileMetaData::new(tileset_id, flipx, flipy, diagonal_flip),
        )
    }
}

#[derive(Debug, Clone)]
pub struct MapMetaData {
    pub tsx_ver: String,
    pub lua_ver: Option<String>,
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

#[derive(Debug, Clone)]
pub struct Map {
    pub meta_data: MapMetaData,
    pub tile_layers: Vec<TileLayer>,
    pub object_layers: Vec<ObjectLayer>,
    pub tilesets: Tilesets,
    pub tile_layer_map: HashMap<String, TileLayerId>,
    pub object_layer_map: HashMap<String, ObjectLayerId>,
    obj_slab: slab::Slab<Object>,
    obj_id_to_ref_map: HashMap<ObjectId, ObjectRef>,
}

#[derive(Debug, Clone)]
pub struct MapRemoval {
    id: TileId,
    layer_id: TileLayerId,
    x: i32,
    y: i32,
}

impl MapRemoval {
    pub fn get_contents(&self) -> (&TileId, &TileLayerId, &i32, &i32) {
        (&self.id, &self.layer_id, &self.x, &self.y)
    }
}

#[derive(Debug, Clone)]
pub struct MapAddition {
    changed_id: Option<TileId>,
    new_id: TileId,
    layer_id: TileLayerId,
    x: i32,
    y: i32,
}

impl MapAddition {
    pub fn get_contents(&self) -> (&Option<TileId>, &TileLayerId, &i32, &i32) {
        (&self.changed_id, &self.layer_id, &self.x, &self.y)
    }
}

#[derive(Debug, Clone)]
pub enum CoordSpace {
    Pixel,
    Tile,
}

impl Map {
    pub fn remove_tile(
        &mut self,
        x: i32,
        y: i32,
        coordinate_space: CoordSpace,
        layer_id: TileLayerId,
    ) -> Option<MapRemoval> {
        let (x, y) = match coordinate_space {
            CoordSpace::Pixel => (
                x / (self.meta_data.tilewidth) as i32,
                y / (self.meta_data.tileheight as i32),
            ),
            CoordSpace::Tile => (x, y),
        };

        if let Some(tile_id) = self.tile_layers[layer_id.llid as usize]
            .data
            .remove_tile(x, y)
        {
            assert!(tile_id.to_index().is_some());
            Some(MapRemoval {
                id: tile_id,
                layer_id,
                x,
                y,
            })
        } else {
            None
        }
    }

    pub fn set_tile(
        &mut self,
        x: i32,
        y: i32,
        layer_id: TileLayerId,
        tile: TileId,
        coordinate_space: CoordSpace,
    ) -> MapAddition {
        let (x, y) = match coordinate_space {
            CoordSpace::Pixel => (
                x / (self.meta_data.tilewidth as i32),
                y / (self.meta_data.tileheight as i32),
            ),
            CoordSpace::Tile => (x, y),
        };

        let layer = &mut self.tile_layers[layer_id.llid as usize];
        let changed_id = layer.data.set_tile(x, y, tile);
        MapAddition {
            new_id: tile,
            changed_id,
            layer_id,
            x,
            y,
        }
    }

    pub fn get_tile(
        &self,
        x: i32,
        y: i32,
        layer_id: TileLayerId,
        coordinate_space: CoordSpace,
    ) -> Option<TileId> {
        let (x, y) = match coordinate_space {
            CoordSpace::Pixel => (
                x / (self.meta_data.tilewidth as i32),
                y / (self.meta_data.tileheight as i32),
            ),
            CoordSpace::Tile => (x, y),
        };

        let layer = &self.tile_layers[layer_id.llid as usize];

        match layer.data.get_tile(x, y) {
            Some(t_id) if t_id.to_index().is_some() => Some(t_id),
            Some(_) | None => None,
        }
    }

    pub fn get_tiles_in_bb(
        &self,
        bb: Box2<i32>,
        layer_id: TileLayerId,
        coordinate_space: CoordSpace,
    ) -> impl Iterator<Item = (TileId, i32, i32)> + '_ {
        assert!(bb.is_valid());
        let box_in_tiles = match coordinate_space {
            CoordSpace::Pixel => (
                (
                    (bb.mins.x as f32 / (self.meta_data.tilewidth) as f32).floor() as i32,
                    (bb.mins.y as f32 / (self.meta_data.tileheight) as f32).floor() as i32,
                ),
                (
                    (bb.maxs.x as f32 / (self.meta_data.tilewidth as f32)).ceil() as i32,
                    (bb.maxs.y as f32 / (self.meta_data.tileheight as f32)).ceil() as i32,
                ),
            ),

            CoordSpace::Tile => ((bb.mins.x, bb.mins.y), (bb.maxs.x, bb.maxs.y)),
        };
        ((box_in_tiles.0 .1)..=(box_in_tiles.1 .1)).flat_map(move |y| {
            ((box_in_tiles.0 .0)..=(box_in_tiles.1 .0)).filter_map(move |x| {
                self.get_tile(x, y, layer_id, CoordSpace::Tile)
                    .map(|t| (t, x, y))
            })
        })
    }

    pub fn get_obj(&self, obj_ref: &ObjectRef) -> &Object {
        &self.obj_slab[obj_ref.0]
    }

    pub fn get_objs_from_obj_group<'a>(
        &'a self,
        obj_group: &'a ObjectGroup,
    ) -> impl Iterator<Item = &'a Object> + 'a {
        obj_group.get_obj_refs().map(move |o| &self.obj_slab[o.0])
    }

    pub fn get_obj_grp_from_tile_id(&self, tileid: &TileId) -> Option<&ObjectGroup> {
        self.tilesets
            .get_tile(tileid)
            .and_then(|t| t.objectgroup.as_ref())
    }

    pub fn get_obj_grp_from_layer_id(&self, obj_layer_id: &ObjectLayerId) -> &ObjectGroup {
        &self.object_layers[obj_layer_id.llid as usize]
    }
}

#[derive(Debug, Clone)]
// The u32 here represents the duration, TileId is which TileId is associated with said duration
pub struct Animation(Vec<(TileId, u32)>);

#[derive(Debug, Clone)]
pub struct Tile {
    pub id: TileId,
    pub tile_type: Option<String>,
    pub probability: f32,
    pub properties: Properties,
    pub objectgroup: Option<ObjectGroup>,
    pub animation: Option<Animation>,
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
    fn get_tile(&self, tile_id: &TileId) -> Option<&Tile> {
        self.tiles.get(tile_id)
    }
}

#[derive(Debug, Clone)]
pub struct Tilesets(Vec<Tileset>);

impl Tilesets {
    pub fn get_tile(&self, tile_id: &TileId) -> Option<&Tile> {
        self.0[tile_id.1.tileset_id() as usize].get_tile(tile_id)
    }
}
