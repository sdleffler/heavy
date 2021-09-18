pub mod lua_parser;

use crate::lua_parser::ColorExt;

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

const FLIPPED_HORIZONTALLY_FLAG: u32 = 0x80000000;
const FLIPPED_VERTICALLY_FLAG: u32 = 0x40000000;
const FLIPPED_DIAGONALLY_FLAG: u32 = 0x20000000;
const UNSET_FLAGS: u32 = 0x1FFFFFFF;

const CHUNK_SIZE: u32 = 16;

const EMPTY_TILE: TileId = TileId(0, TileMetaData(0));

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
    Color(u32),
    File(String),
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

#[derive(Debug, Clone)]
pub enum Encoding {
    Lua,
    Base64,
}

#[derive(Debug, Clone)]
pub enum Compression {
    ZLib,
    GZip,
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
    fn new(tileset_id: u32, flipx: bool, flipy: bool, diagonal_flip: bool) -> TileMetaData {
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct TileLayerId {
    // global layer id and local layer id
    // global layer id is set by tiled, local layer id is generated sequentially in the order
    // that the layers are parsed
    glid: u32,
    llid: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ObjectLayerId {
    // global layer id and local layer id
    // global layer id is set by tiled, local layer id is generated sequentially in the order
    // that the layers are parsed
    glid: u32,
    llid: u32,
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
pub struct Chunk {
    // Contains the coordinates of the chunk in tile space
    tile_x: i32,
    tile_y: i32,
    data: Vec<TileId>,
}

impl Chunk {
    fn new(x: i32, y: i32) -> Self {
        Chunk {
            tile_x: x,
            tile_y: y,
            data: vec![EMPTY_TILE; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Chunks(HashMap<(i32, i32), Chunk>);

impl Chunks {
    pub fn new() -> Self {
        Chunks(HashMap::default())
    }

    pub fn set_tile(&mut self, x: i32, y: i32, tile: TileId) -> Option<TileId> {
        let (chunk_x, tile_x) = (
            x.div_euclid(CHUNK_SIZE as i32),
            x.rem_euclid(CHUNK_SIZE as i32) as u32,
        );
        let (chunk_y, tile_y) = (
            y.div_euclid(CHUNK_SIZE as i32),
            y.rem_euclid(CHUNK_SIZE as i32) as u32,
        );
        let chunk = self
            .0
            .entry((chunk_x, chunk_y))
            .or_insert_with(|| Chunk::new(x, y));
        let index = (tile_y * CHUNK_SIZE + tile_x) as usize;
        let tile_id = chunk.data[index];
        chunk.data[index] = tile;
        if tile_id != EMPTY_TILE {
            Some(tile_id)
        } else {
            None
        }
    }

    pub fn get_tile(&self, x: i32, y: i32) -> Option<TileId> {
        let y = -y;
        let (chunk_x, tile_x) = (
            x.div_euclid(CHUNK_SIZE as i32),
            x.rem_euclid(CHUNK_SIZE as i32) as u32,
        );
        let (chunk_y, tile_y) = (
            y.div_euclid(CHUNK_SIZE as i32),
            y.rem_euclid(CHUNK_SIZE as i32) as u32,
        );

        self.0.get(&(chunk_x, chunk_y)).and_then(|chunk| {
            match chunk.data[((CHUNK_SIZE * tile_y) + tile_x) as usize] {
                EMPTY_TILE => None,
                t => Some(t),
            }
        })
    }
}

pub fn to_chunks(data: &[TileId], width: u32, height: u32) -> Chunks {
    let mut chunks = Chunks::default();
    for y in 0..height {
        for x in 0..width {
            chunks.set_tile(x as i32, y as i32, data[(y * width + x) as usize]);
        }
    }
    chunks
}

#[derive(Debug, Clone)]
pub struct TileLayer {
    layer_type: LayerType,
    id: TileLayerId,
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
    data: Chunks,
}

impl TileLayer {
    fn parse_tile_data(
        encoding: &Encoding,
        compression: &Option<Compression>,
        t: &LuaTable,
        tile_buffer: &[TileId],
    ) -> Result<Vec<TileId>, Error> {
        // TODO: Vector capacity can be pre-calculated here, optimize this
        let mut tile_data = Vec::new();

        match encoding {
            Encoding::Lua => {
                for tile in t
                    .get::<_, LuaTable>("data")?
                    .sequence_values::<LuaInteger>()
                {
                    tile_data.push(tile? as u32);
                }
            }

            Encoding::Base64 => {
                let str_data = t.get::<_, LuaString>("data")?.to_str()?.to_owned();

                let decoded_bytes = base64::decode_config(str_data, base64::STANDARD)?;

                let level_bytes = match compression {
                    Some(c) => match c {
                        Compression::GZip => {
                            let mut d = flate2::read::GzDecoder::new(decoded_bytes.as_slice());
                            let mut s = Vec::new();
                            d.read_to_end(&mut s).unwrap();
                            s
                        }
                        Compression::ZLib => {
                            let mut d = flate2::read::ZlibDecoder::new(decoded_bytes.as_slice());
                            let mut s = Vec::new();
                            d.read_to_end(&mut s).unwrap();
                            s
                        }
                    },
                    None => decoded_bytes,
                };

                for i in (0..level_bytes.len()).step_by(4) {
                    let val = level_bytes[i] as u32
                        | (level_bytes[i + 1] as u32) << 8
                        | (level_bytes[i + 2] as u32) << 16
                        | (level_bytes[i + 3] as u32) << 24;
                    tile_data.push(val);
                }
            }
        }

        let mut tile_ids = Vec::with_capacity(tile_data.len());

        for mut tile in tile_data.into_iter() {
            // For each tile, we check the flip flags and set the metadata with them.
            // We then unset the flip flags in the tile ID
            let flipx = (tile & FLIPPED_HORIZONTALLY_FLAG) != 0;
            let flipy = (tile & FLIPPED_VERTICALLY_FLAG) != 0;
            let diag_flip = (tile & FLIPPED_DIAGONALLY_FLAG) != 0;

            tile &= UNSET_FLAGS;

            let mut tile_id = tile_buffer[tile as usize];

            tile_id.1 = TileMetaData::new(tile_id.1.tileset_id(), flipx, flipy, diag_flip);
            tile_ids.push(tile_id);
        }

        Ok(tile_ids)
    }
}

type ObjectLayer = ObjectGroup;

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
pub enum CoordSpace {
    Pixel,
    Tile,
}

impl Map {
    pub fn get_tile_at(
        &self,
        x: i32,
        y: i32,
        coordinate_space: CoordSpace,
    ) -> Vec<(TileId, TileLayerId)> {
        let mut tile_layer_buff = Vec::new();
        let (x, y) = match coordinate_space {
            CoordSpace::Pixel => (
                x / (self.meta_data.tilewidth) as i32,
                y / (self.meta_data.tileheight as i32),
            ),
            CoordSpace::Tile => (x, y),
        };

        for layer in self.tile_layers.iter() {
            // We subtract top from y * self.meta_data.width since tiled stores it's tiles top left
            // to bottom right, and we want to index bottom left to top right

            if let Some(tile_id) = layer.data.get_tile(x, y) {
                // TODO: there should be a better way to ID a layer than this
                if tile_id.to_index().is_some() {
                    tile_layer_buff.push((tile_id, layer.id));
                }
            }
        }
        tile_layer_buff
    }

    pub fn get_tile_in_layer(
        &self,
        x: i32,
        y: i32,
        layer: TileLayerId,
        coordinate_space: CoordSpace,
    ) -> Option<TileId> {
        let (x, y) = match coordinate_space {
            CoordSpace::Pixel => (
                x / (self.meta_data.tilewidth as i32),
                y / (self.meta_data.tileheight as i32),
            ),
            CoordSpace::Tile => (x, y),
        };

        let layer = &self.tile_layers[layer.llid as usize];

        match layer.data.get_tile(x, y) {
            Some(t_id) if t_id.to_index().is_some() => Some(t_id),
            Some(_) | None => None,
        }
    }

    pub fn get_tiles_in_bb(
        &self,
        bb: Box2<i32>,
        coordinate_space: CoordSpace,
    ) -> impl Iterator<Item = (Vec<(TileId, TileLayerId)>, i32, i32)> + '_ {
        assert!(bb.is_valid());
        let box_in_tiles = match coordinate_space {
            CoordSpace::Pixel => (
                (
                    (bb.mins.x as f32 / (self.meta_data.tilewidth as f32)).floor() as i32,
                    (bb.mins.y as f32 / (self.meta_data.tileheight as f32)).floor() as i32,
                ),
                (
                    (bb.maxs.x as f32 / (self.meta_data.tilewidth as f32)).ceil() as i32,
                    (bb.maxs.y as f32 / (self.meta_data.tileheight as f32)).ceil() as i32,
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
                self.get_tile_in_layer(x, y, layer_id, CoordSpace::Tile)
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

    // pub fn set_tile(x: u32, y: u32, tile_id: TileId) -> Option<TileId> { None }
}

// TODO: implement this struct. How do we want to draw objects?
pub struct ObjectLayerBatch;

#[derive(Debug, Clone)]
pub struct SpriteSheetState {
    anim_state: AnimationState,
    sprite_tag: TagId,
}

pub struct TileLayerBatches(Vec<TileLayerBatch>);

impl TileLayerBatches {
    pub fn new(
        tile_layers: &[TileLayer],
        ts_render_data: &TilesetRenderData,
        map: &Map,
        engine: &Engine,
    ) -> Self {
        let mut batches = Vec::with_capacity(tile_layers.len());
        for tile_layer in tile_layers.iter() {
            batches.push(TileLayerBatch::new(
                tile_layer,
                ts_render_data,
                engine,
                &map.meta_data,
            ));
        }
        TileLayerBatches(batches)
    }

    pub fn update_all_batches(&mut self, dt: f32, ts_render_data: &TilesetRenderData) {
        for tile_layer_batch in self.0.iter_mut() {
            tile_layer_batch.update_batches(dt, ts_render_data);
        }
    }

    pub fn get_layer(&self, layer_id: TileLayerId) -> &TileLayerBatch {
        &self.0[layer_id.llid as usize]
    }

    pub fn get_layer_mut(&mut self, layer_id: TileLayerId) -> &mut TileLayerBatch {
        &mut self.0[layer_id.llid as usize]
    }

    pub fn get_tile_batch_layers(&mut self) -> impl Iterator<Item = &mut TileLayerBatch> + '_ {
        self.0.iter_mut()
    }
}

impl DrawableMut for TileLayerBatches {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        for tile_layer in self.0.iter_mut() {
            if tile_layer.visible {
                for batch in tile_layer.sprite_batches.iter_mut() {
                    batch.draw_mut(
                        ctx,
                        instance.translate2(Vector2::new(tile_layer.offx, tile_layer.offy)),
                    );
                }
            }
        }
    }
}

pub struct TileLayerBatch {
    _id: TileLayerId,
    sprite_sheet_info: Vec<HashMap<SpriteId, SpriteSheetState>>,
    pub sprite_id_map: HashMap<(i32, i32), SpriteId>,
    sprite_batches: Vec<SpriteBatch<CachedTexture>>,
    pub visible: bool,
    pub opacity: f64,
    pub offx: f32,
    pub offy: f32,
}

impl DrawableMut for TileLayerBatch {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        for batch in self.sprite_batches.iter_mut() {
            batch.draw_mut(ctx, instance);
        }
    }
}

impl TileLayerBatch {
    pub fn new(
        layer: &TileLayer,
        ts_render_data: &TilesetRenderData,
        engine: &Engine,
        map_meta_data: &MapMetaData,
    ) -> Self {
        // We need 1 sprite batch per texture
        let mut sprite_batches = Vec::with_capacity(ts_render_data.textures_and_spritesheets.len());
        let mut ss_state = vec![HashMap::new(); ts_render_data.textures_and_spritesheets.len()];
        let mut sprite_id_map = HashMap::new();

        let graphics_lock = engine.get::<GraphicsLock>();

        for (texture, _) in ts_render_data.textures_and_spritesheets.iter() {
            let mut acquired_lock = GraphicsLockExt::lock(&graphics_lock);
            sprite_batches.push(SpriteBatch::new(&mut acquired_lock, texture.clone()));
            drop(acquired_lock);
        }

        for ((chunk_x, chunk_y), chunk) in layer.data.0.iter() {
            for tile_y in 0..CHUNK_SIZE {
                for tile_x in 0..CHUNK_SIZE {
                    let tile = chunk.data[(tile_y * CHUNK_SIZE + tile_x) as usize];
                    // Tile indices start at 1, 0 represents no tile, so we offset the tile by 1
                    // first, and skip making the instance param if the tile is 0
                    if let Some(index) = tile.to_index() {
                        let (scale_x, trans_fix_x) = if tile.1.flipx() {
                            (-1.0, -1.0 * map_meta_data.tilewidth as f32)
                        } else {
                            (1.0, 0.0)
                        };

                        let (scale_y, trans_fix_y) = if tile.1.flipy() {
                            (-1.0, -1.0 * map_meta_data.tileheight as f32)
                        } else {
                            (1.0, 0.0)
                        };

                        let (rotation, y_scale, x_trans, y_trans) = if tile.1.diag_flip() {
                            (
                                std::f32::consts::FRAC_PI_2,
                                -1.0,
                                map_meta_data.tilewidth as f32,
                                map_meta_data.tileheight as f32 * -1.0,
                            )
                        } else {
                            (0.0, 1.0, 0.0, 0.0)
                        };

                        let tile_x_global = (chunk_x * CHUNK_SIZE as i32) + tile_x as i32;
                        let tile_y_global = (((chunk_y * -1) - 1) * CHUNK_SIZE as i32)
                            + (CHUNK_SIZE - tile_y) as i32
                            - 1;

                        let sprite_id = sprite_batches[tile.1.tileset_id() as usize].insert(
                            Instance::new()
                                .src(ts_render_data.uvs[index])
                                .color(Color::new(1.0, 1.0, 1.0, layer.opacity as f32))
                                .translate2(Vector2::new(
                                    (tile_x_global * map_meta_data.tilewidth as i32) as f32,
                                    (tile_y_global * map_meta_data.tileheight as i32) as f32,
                                ))
                                .scale2(Vector2::new(scale_x, scale_y))
                                .translate2(Vector2::new(trans_fix_x, trans_fix_y))
                                .scale2(Vector2::new(1.0, y_scale))
                                .translate2(Vector2::new(x_trans, y_trans))
                                .rotate2(rotation),
                        );

                        sprite_id_map.insert((tile_x_global, tile_y_global), sprite_id);

                        if let Some(t) = ts_render_data.tile_to_tag_map.get(&tile) {
                            let anim_state = ts_render_data.textures_and_spritesheets
                                [tile.1.tileset_id() as usize]
                                .1
                                .at_tag(*t, true);
                            ss_state[tile.1.tileset_id() as usize].insert(
                                sprite_id,
                                SpriteSheetState {
                                    anim_state,
                                    sprite_tag: *t,
                                },
                            );
                        }
                    }
                }
            }
        }

        TileLayerBatch {
            sprite_sheet_info: ss_state,
            visible: layer.visible,
            opacity: layer.opacity,
            offx: (layer.x * map_meta_data.tilewidth) as f32,
            offy: (layer.y * map_meta_data.tileheight) as f32,
            _id: layer.id,
            sprite_batches,
            sprite_id_map,
        }
    }

    pub fn update_batches(&mut self, dt: f32, ts_render_data: &TilesetRenderData) {
        for (i, batch) in self.sprite_batches.iter_mut().enumerate() {
            for (sprite_index, ss_state) in self.sprite_sheet_info[i].iter_mut() {
                let sprite_sheet = &ts_render_data.textures_and_spritesheets[i].1;
                if let Some(new_frame_id) =
                    sprite_sheet.update_animation(dt, &mut ss_state.anim_state)
                {
                    batch[*sprite_index].src = sprite_sheet[new_frame_id].uvs;
                }
            }
        }
    }

    // pub fn set_tile(
    //     &mut self,
    //     x: u32,
    //     y: u32,
    //     tile: TileId,
    //     ts_render_data: &TilesetRenderData,
    //     map: &mut Map,
    // ) -> Option<(SpriteId, TileId)> {
    //     // Insert the new tile into the sprite sheet
    //     let index = tile.to_index().unwrap();
    //     let sprite_id = self.sprite_batches[tile.1.tileset_id() as usize].insert(
    //         Instance::new()
    //             .src(ts_render_data.uvs[index])
    //             .color(Color::new(1.0, 1.0, 1.0, self.opacity as f32))
    //             .translate2(Vector2::new(
    //                 (x * map.meta_data.tilewidth) as f32,
    //                 // Need to offset by 1 here since tiled renders maps top right to bottom left, but we do bottom left to top right
    //                 (y * map.meta_data.tileheight) as f32,
    //             )),
    //     );

    //     // If it's an animated tile, add it to the sprite sheet state hashmap so that it'll get updated correctly
    //     if let Some(t) = ts_render_data.tile_to_tag_map.get(&tile) {
    //         let anim_state = ts_render_data.textures_and_spritesheets[tile.1.tileset_id() as usize]
    //             .1
    //             .at_tag(*t, true);
    //         self.sprite_sheet_info[tile.1.tileset_id() as usize].insert(
    //             sprite_id,
    //             SpriteSheetState {
    //                 anim_state,
    //                 sprite_tag: *t,
    //             },
    //         );
    //     }

    //     // We first remove the tile, as if we insert first, remove will fail due to insert updating
    //     // values in place
    //     let ret_val = self.remove_tile(x, y, map);

    //     // Update the layer with the new tile id
    //     let layer = &mut map.tile_layers[self.id.llid as usize];

    //     let top = (layer.height * map.meta_data.width - 1) - map.meta_data.height;

    //     layer.data[(top - (y * layer.width + x)) as usize] = tile;

    //     // Insert the new sprite id, we unwrap() here to trigger a panic in the event
    //     let res = self.sprite_id_map.insert((x, y), sprite_id);
    //     assert!(res.is_none(),
    //             "There is a bug in the hv_tiled remove_tile function, remove_tile should've removed the tile, but instead we got {:?}", res.unwrap());

    //     ret_val
    // }

    // pub fn remove_tile(&mut self, x: u32, y: u32, map: &mut Map) -> Option<(SpriteId, TileId)> {
    //     let layer = &mut map.tile_layers[self.id.llid as usize];

    //     let top = (layer.width * layer.height) - layer.width;

    //     let tile_ref = &mut layer.data[(top - (y * layer.width) + x) as usize];
    //     let old_tile = *tile_ref;

    //     if let Some(old_sprite_id) = self.sprite_id_map.remove(&(x, y)) {
    //         // Attempt to remove the sprite sheet info if it exists since we don't want to update animation info for a sprite that doesn't exist
    //         self.sprite_sheet_info[old_tile.1.tileset_id() as usize].remove(&old_sprite_id);
    //         self.sprite_batches[tile_ref.1.tileset_id() as usize].remove(old_sprite_id);

    //         *tile_ref = EMPTY_TILE;
    //         Some((old_sprite_id, old_tile))
    //     } else {
    //         None
    //     }
    // }
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

#[derive(Debug, Clone)]
enum Halign {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone)]
enum Valign {
    Top,
    Center,
    Bottom,
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

#[derive(Debug, Clone)]
pub enum ObjGroupType {
    ObjectGroup,
}

#[derive(Debug, Clone)]
pub struct ObjectGroup {
    pub name: String,
    pub opacity: f32,
    pub visible: bool,
    pub draworder: DrawOrder,
    pub object_refs: Vec<ObjectRef>,
    pub color: Color,
    pub id: ObjectLayerId,
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
    pub fn get_obj_refs(&self) -> impl Iterator<Item = &ObjectRef> + '_ {
        self.object_refs.iter()
    }
}

#[derive(Debug, Clone)]
// The u32 here represents the duration, TileId is which TileId is assocated with said duration
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

#[derive(Debug, Clone)]
pub enum TileRenderType {
    Static(Box2<f32>),
    Animated(TagId),
}

pub struct TilesetRenderData {
    // Box2<f32> is the uvs
    uvs: Vec<Box2<f32>>,
    // We pair the Texture and the related SpriteSheet of that texture
    textures_and_spritesheets: Vec<(CachedTexture, SpriteSheet)>,
    // Relates a TileId to a TagId, which is used to get the relevant sprite sheet info
    tile_to_tag_map: HashMap<TileId, TagId>,
}

impl TilesetRenderData {
    pub fn new(tilesets: &Tilesets, engine: &Engine) -> Result<Self, Error> {
        let mut textures_and_spritesheets = Vec::with_capacity(tilesets.0.len());
        let mut uvs = Vec::new();
        let mut tile_to_tag_map = HashMap::new();

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
            let texture_obj = Texture::from_reader(&mut acquired_lock, &mut tileset_img_path)?;

            drop(acquired_lock);

            let rows = tileset.tilecount / tileset.columns;
            let top = (rows * (tileset.spacing + tileset.tile_height)) + tileset.margin;
            for row in 1..=rows {
                for column in 0..tileset.columns {
                    uvs.push(Box2::new(
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

            let mut sprite_sheet = SpriteSheet::new();

            for (_, tile) in tileset.tiles.iter() {
                if let Some(animation) = &tile.animation {
                    let from = sprite_sheet.next_frame_id();

                    for (tile_id, duration) in animation.0.iter() {
                        sprite_sheet.insert_frame(Frame {
                            source: None,
                            offset: Vector2::new(0.0, 0.0),
                            uvs: uvs[tile_id.0 as usize],
                            duration: *duration,
                        });
                    }

                    let tag_id = sprite_sheet.insert_tag(Tag {
                        name: None,
                        from,
                        to: sprite_sheet.last_frame_id(),
                        direction: Direction::Forward,
                    });

                    tile_to_tag_map.insert(tile.id, tag_id);
                }
            }

            textures_and_spritesheets.push((CachedTexture::from(texture_obj), sprite_sheet));
        }

        Ok(TilesetRenderData {
            uvs,
            textures_and_spritesheets,
            tile_to_tag_map,
        })
    }
}

impl Drawable for TilesetRenderData {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        let mut y_offset = 0.0;
        for (texture, _) in self.textures_and_spritesheets.iter() {
            texture.draw(ctx, instance.translate2(Vector2::new(0.0, y_offset)));
            y_offset += texture.get().height() as f32;
        }
    }
}

impl DrawableMut for TilesetRenderData {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.draw(ctx, instance);
    }
}
