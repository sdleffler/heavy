use crate::*;

// TODO: implement this struct. How do we want to draw objects?
// pub struct ObjectLayerBatch;

#[derive(Debug, Clone)]
pub struct SpriteSheetState {
    anim_state: AnimationState,
    sprite_tag: TagId,
}

#[derive(Debug, Clone, Copy)]
pub enum TileRenderData {
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
    tile_width: u32,
    tile_height: u32,
}

impl TilesetRenderData {
    pub fn new(
        tile_width: u32,
        tile_height: u32,
        tilesets: &Tilesets,
        engine: &Engine,
    ) -> Result<Self, Error> {
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
            tile_height,
            tile_width,
            uvs,
            textures_and_spritesheets,
            tile_to_tag_map,
        })
    }

    pub fn get_render_data_for_tile(
        &self,
        tile: TileId,
    ) -> (TileRenderData, &SpriteSheet, &CachedTexture) {
        let tile = TileId(tile.0 - 1, tile.1);
        let render_data = if let Some(tag) = self.tile_to_tag_map.get(&tile) {
            TileRenderData::Animated(*tag)
        } else {
            TileRenderData::Static(self.uvs[tile.0 as usize])
        };
        let (ss, ct) = &self.textures_and_spritesheets[tile.1.tileset_id() as usize];
        (render_data, ct, ss)
    }

    pub fn get_tileset_texture_and_spritesheet(
        &self,
        tileset_id: u32,
    ) -> (&SpriteSheet, &CachedTexture) {
        let (ss, ct) = &self.textures_and_spritesheets[tileset_id as usize];
        (ct, ss)
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

pub struct TileLayerBatches {
    batches: Vec<TileLayerBatch>,
    _render_orientation: Orientation,
}

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

        TileLayerBatches {
            batches,
            _render_orientation: map.meta_data.orientation.clone(),
        }
    }

    pub fn update_all_batches(&mut self, dt: f32, ts_render_data: &TilesetRenderData) {
        for tile_layer_batch in self.batches.iter_mut() {
            tile_layer_batch.update_batches(dt, ts_render_data);
        }
    }

    pub fn get_layer(&self, layer_id: TileLayerId) -> &TileLayerBatch {
        &self.batches[layer_id.llid as usize]
    }

    pub fn get_layer_mut(&mut self, layer_id: TileLayerId) -> &mut TileLayerBatch {
        &mut self.batches[layer_id.llid as usize]
    }

    pub fn get_tile_batch_layers(&mut self) -> impl Iterator<Item = &mut TileLayerBatch> + '_ {
        self.batches.iter_mut()
    }

    fn set_tile(
        &mut self,
        addition: &TileAddition,
        ts_render_data: &TilesetRenderData,
    ) -> Option<SpriteId> {
        // Remove the existing sprite id and any animated metadata associatd with it
        let ret_val = if self.batches[addition.layer_id.llid as usize]
            .sprite_id_map
            .contains_key(&(addition.x, addition.y))
        {
            self.remove_tile(&TileRemoval {
                id: addition.changed_id.unwrap(),
                layer_id: addition.layer_id,
                x: addition.x,
                y: addition.y,
            })
        } else {
            None
        };

        // Insert the new tile into the sprite sheet
        let index = addition.new_id.to_index().unwrap();
        let tile_batch = &mut self.batches[addition.layer_id.llid as usize];
        let sprite_id = tile_batch.sprite_batches[addition.new_id.1.tileset_id() as usize].insert(
            Instance::new()
                .src(ts_render_data.uvs[index])
                .color(Color::new(1.0, 1.0, 1.0, tile_batch.opacity as f32))
                .translate2(Vector2::new(
                    (addition.x * ts_render_data.tile_width as i32) as f32,
                    // TODO: make sure that this is correct, we subtract one because our origin is 1 unit
                    // lower than tiled's system
                    ((addition.y - 1) * ts_render_data.tile_height as i32) as f32,
                )),
        );

        // If it's an animated tile, add it to the sprite sheet state hashmap so that it'll get updated correctly
        if let Some(t) = ts_render_data.tile_to_tag_map.get(&addition.new_id) {
            let anim_state = ts_render_data.textures_and_spritesheets
                [addition.new_id.1.tileset_id() as usize]
                .1
                .at_tag(*t, true);
            tile_batch.sprite_sheet_info[addition.new_id.1.tileset_id() as usize].insert(
                sprite_id,
                SpriteSheetState {
                    anim_state,
                    sprite_tag: *t,
                },
            );
        }

        // Insert the new sprite id, we unwrap() here to trigger a panic in the event
        // that we somehow inserted a tile that already existed
        let res = tile_batch
            .sprite_id_map
            .insert((addition.x, addition.y), sprite_id);
        assert!(res.is_none(),
                 "There is a bug in the hv_tiled remove_tile function, remove_tile should've removed the tile, but instead we got {:?}", res.unwrap());

        ret_val
    }

    fn remove_tile(&mut self, removal: &TileRemoval) -> Option<SpriteId> {
        let tile_batch = &mut self.batches[removal.layer_id.llid as usize];
        if let Some(old_sprite_id) = tile_batch.sprite_id_map.remove(&(removal.x, removal.y)) {
            // Attempt to remove the sprite sheet info if it exists since we don't want to update animation info for a sprite that doesn't exist
            tile_batch.sprite_sheet_info[removal.id.1.tileset_id() as usize].remove(&old_sprite_id);
            tile_batch.sprite_batches[removal.id.1.tileset_id() as usize].remove(old_sprite_id);
            Some(old_sprite_id)
        } else {
            None
        }
    }

    pub fn resolve_delta(
        &mut self,
        change: &TileChange,
        ts_render_data: &TilesetRenderData,
    ) -> Option<SpriteId> {
        match change {
            TileChange::TileAddition(a) => self.set_tile(a, ts_render_data),
            TileChange::TileRemoval(r) => self.remove_tile(r),
        }
    }

    pub fn resolve_deltas<'a>(
        &mut self,
        change_iter: impl Iterator<Item = &'a TileChange>,
        ts_render_data: &TilesetRenderData,
    ) -> Vec<Option<SpriteId>> {
        change_iter
            .map(|change| self.resolve_delta(change, ts_render_data))
            .collect()
    }
}

impl DrawableMut for TileLayerBatches {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        for tile_layer in self.batches.iter_mut() {
            if tile_layer.visible {
                for batch in tile_layer.sprite_batches.iter_mut() {
                    batch.draw_mut(
                        ctx,
                        instance
                            .translate2(Vector2::new(tile_layer.offset_x, -tile_layer.offset_y)),
                    );
                }
            }
        }
    }
}

pub struct TileLayerBatch {
    sprite_sheet_info: Vec<HashMap<SpriteId, SpriteSheetState>>,
    pub sprite_id_map: HashMap<(i32, i32), SpriteId>,
    sprite_batches: Vec<SpriteBatch<CachedTexture>>,
    pub visible: bool,
    pub opacity: f64,
    _x: f32,
    _y: f32,
    pub offset_x: f32,
    pub offset_y: f32,
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
                    let tile = chunk.0[(tile_y * CHUNK_SIZE + tile_x) as usize];
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

                        let (pixel_x, pixel_y) = match map_meta_data.orientation {
                            Orientation::Orthogonal => (
                                (tile_x_global * map_meta_data.tilewidth as i32) as f32,
                                (tile_y_global * map_meta_data.tileheight as i32) as f32,
                            ),
                            Orientation::Isometric => (
                                ((tile_x_global + tile_y_global) * map_meta_data.tilewidth as i32)
                                    as f32
                                    / 2.0,
                                (((tile_x_global + (-tile_y_global))
                                    * map_meta_data.tileheight as i32)
                                    as f32
                                    / -2.0),
                            ),
                        };

                        let sprite_id = sprite_batches[tile.1.tileset_id() as usize].insert(
                            Instance::new()
                                .src(ts_render_data.uvs[index])
                                .color(Color::new(1.0, 1.0, 1.0, layer.opacity as f32))
                                .translate2(Vector2::new(pixel_x, pixel_y))
                                .scale2(Vector2::new(scale_x, scale_y))
                                .translate2(Vector2::new(trans_fix_x, trans_fix_y))
                                .scale2(Vector2::new(1.0, y_scale))
                                .translate2(Vector2::new(x_trans, y_trans))
                                .rotate2(rotation),
                        );

                        // Todo: I think the reason why be add 1 here is due to the render data
                        // being offset by 1 from the actual map data, but this needs to be checked
                        sprite_id_map.insert((tile_x_global, tile_y_global + 1), sprite_id);

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
            _x: (layer.x * (map_meta_data.tilewidth as i32)) as f32,
            _y: (layer.y * (map_meta_data.tileheight as i32)) as f32,
            offset_x: layer.offset_x as f32,
            offset_y: layer.offset_y as f32,
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
}
