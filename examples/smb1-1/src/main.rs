use std::path::Path;

use atomic_refcell::AtomicRefCell;
use hv_core::{
    components::DynamicComponentConstructor,
    conf::Conf,
    engine::{Engine, EngineRef, EventHandler},
    filesystem::Filesystem,
    input::{GamepadAxis, GamepadButton, InputBinding, InputState, KeyCode, KeyMods, MouseButton},
    prelude::*,
    spaces::{Object, Space, Spaces},
    timer::TimeContext,
};

use hv_friends::{
    collision::Collider,
    graphics::{
        sprite::{CachedSpriteSheet, SpriteAnimation, SpriteSheetCache},
        texture::TextureCache,
        CachedTexture, DrawableMut, GraphicsLock, GraphicsLockExt, Instance, SpriteBatch,
    },
    math::*,
    parry2d, Position, SimpleHandler, Velocity,
};
use hv_tiled::{BoxExt, CoordSpace, TileId, TilesetRenderData};

const TIMESTEP: f32 = 1. / 60.;
const LOAD_DISTANCE_IN_PIXELS: f32 = 32.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Button {
    A,
    B,
    Start,
    Left,
    Right,
    Down,
    Up,
}

impl LuaUserData for Button {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Axis {}

impl LuaUserData for Axis {}

fn default_input_bindings() -> InputBinding<Axis, Button> {
    InputBinding::new()
        .bind_gamepad_button_to_button(GamepadButton::West, Button::B)
        .bind_gamepad_button_to_button(GamepadButton::North, Button::A)
        .bind_gamepad_button_to_button(GamepadButton::Start, Button::Start)
        .bind_gamepad_button_to_button(GamepadButton::DPadLeft, Button::Left)
        .bind_gamepad_button_to_button(GamepadButton::DPadRight, Button::Right)
        .bind_gamepad_button_to_button(GamepadButton::DPadDown, Button::Down)
        .bind_gamepad_button_to_button(GamepadButton::DPadUp, Button::Up)
        .bind_key_to_button(KeyCode::Z, Button::B)
        .bind_key_to_button(KeyCode::X, Button::A)
        .bind_key_to_button(KeyCode::Enter, Button::Start)
        .bind_key_to_button(KeyCode::Left, Button::Left)
        .bind_key_to_button(KeyCode::Right, Button::Right)
        .bind_key_to_button(KeyCode::Down, Button::Down)
        .bind_key_to_button(KeyCode::Up, Button::Up)
        .bind_key_to_button(KeyCode::A, Button::Left)
        .bind_key_to_button(KeyCode::D, Button::Right)
        .bind_key_to_button(KeyCode::S, Button::Down)
        .bind_key_to_button(KeyCode::W, Button::Up)
    // TODO: bind gamepad axis to button
}

#[derive(Debug, Clone, Copy)]
struct RequiresLuaUpdate;

#[derive(Debug, Clone, Copy)]
struct GoombaMarker;

#[derive(Debug, Clone, Copy)]
struct KoopaMarker;

#[derive(Debug, Clone, Copy)]
struct PlayerMarker;

#[derive(Debug, Clone, Copy)]
struct ItemMarker(u32);

#[derive(Debug, Clone, Copy)]
struct Unloaded;

#[allow(clippy::type_complexity)]
struct SmbOneOne {
    input_state: Shared<InputState<Axis, Button>>,
    button_table: LuaRegistryKey,
    sprite_sheets_table: LuaRegistryKey,

    space: Shared<Space>,
    tile_layer_batches: AtomicRefCell<hv_tiled::TileLayerBatches>,
    x_scroll: AtomicRefCell<f32>,

    map_initial_state: hv_tiled::Map,
    map: AtomicRefCell<hv_tiled::Map>,
    ts_render_data: TilesetRenderData,
    to_update: AtomicRefCell<Vec<Object>>,
    to_collide: AtomicRefCell<Vec<(Object, Object)>>,
    to_headbutt: AtomicRefCell<Vec<(Object, (i32, i32, TileId, Option<u32>))>>,
    to_load: AtomicRefCell<Vec<Object>>,

    goomba_batch: AtomicRefCell<SpriteBatch<CachedTexture>>,
    koopa_batch: AtomicRefCell<SpriteBatch<CachedTexture>>,
    mario_batch: AtomicRefCell<SpriteBatch<CachedTexture>>,
    item_batch: AtomicRefCell<SpriteBatch<CachedTexture>>,

    goomba_sheet: CachedSpriteSheet,
    koopa_sheet: CachedSpriteSheet,
    mario_sheet: CachedSpriteSheet,
}

impl SmbOneOne {
    fn new(engine: &Engine, input_state: Shared<InputState<Axis, Button>>) -> Result<Shared<Self>> {
        let space = engine.get::<Spaces>().borrow_mut().create_space();
        let mut texture_cache = engine.get::<TextureCache>().owned_borrow_mut();
        let goomba_texture = texture_cache.get_or_load("/sprite_sheets/goomba-sheet.png")?;
        let koopa_texture = texture_cache.get_or_load("/sprite_sheets/koopa.png")?;
        let mario_texture = texture_cache.get_or_load("/sprite_sheets/mario_ss.png")?;
        drop(texture_cache);

        let mut sprite_sheet_cache = engine.get::<SpriteSheetCache>().owned_borrow_mut();
        let goomba_sheet = sprite_sheet_cache.get_or_load("/sprite_sheets/goomba.json")?;
        let koopa_sheet = sprite_sheet_cache.get_or_load("/sprite_sheets/koopa.json")?;
        let mario_sheet = sprite_sheet_cache.get_or_load("/sprite_sheets/mario_ss.json")?;
        drop(sprite_sheet_cache);

        let button_table;
        let sprite_sheets_table;

        {
            let lua = engine.lua();

            let button = lua.create_table()?;
            button.set("A", Button::A)?;
            button.set("B", Button::B)?;
            button.set("Start", Button::Start)?;
            button.set("Left", Button::Left)?;
            button.set("Right", Button::Right)?;
            button.set("Down", Button::Down)?;
            button.set("Up", Button::Up)?;
            button_table = lua.create_registry_value(button.clone())?;

            let sprite_sheets = lua.create_table()?;
            sprite_sheets.set("goomba", goomba_sheet.clone())?;
            sprite_sheets.set("koopa", koopa_sheet.clone())?;
            sprite_sheets.set("mario", mario_sheet.clone())?;
            sprite_sheets_table = lua.create_registry_value(sprite_sheets.clone())?;
        }

        let map =
            hv_tiled::lua_parser::parse_map("/maps/mario_bros_1-1.lua", engine, Some("maps/"))?;

        let ts_render_data = hv_tiled::TilesetRenderData::new(
            map.meta_data.tilewidth,
            map.meta_data.tileheight,
            &map.tilesets,
            engine,
        )?;

        let tile_layer_batches = AtomicRefCell::new(hv_tiled::TileLayerBatches::new(
            &map.tile_layers,
            &ts_render_data,
            &map,
            engine,
        ));

        let gfx_lock = engine.get::<GraphicsLock>();
        let mut gfx = gfx_lock.lock();
        let goomba_batch = AtomicRefCell::new(SpriteBatch::new(&mut gfx, goomba_texture));
        let koopa_batch = AtomicRefCell::new(SpriteBatch::new(&mut gfx, koopa_texture));
        let mario_batch = AtomicRefCell::new(SpriteBatch::new(&mut gfx, mario_texture));
        let item_batch = AtomicRefCell::new(SpriteBatch::new(
            &mut gfx,
            ts_render_data
                .get_tileset_texture_and_spritesheet(0)
                .1
                .clone(),
        ));
        drop(gfx);

        Ok(Shared::new(SmbOneOne {
            input_state,
            button_table,
            sprite_sheets_table,

            space,
            tile_layer_batches,
            x_scroll: AtomicRefCell::new(0.),
            map_initial_state: map.clone(),
            map: AtomicRefCell::new(map),
            ts_render_data,
            to_update: AtomicRefCell::new(Vec::new()),
            to_collide: AtomicRefCell::new(Vec::new()),
            to_headbutt: AtomicRefCell::new(Vec::new()),
            to_load: AtomicRefCell::new(Vec::new()),

            goomba_batch,
            koopa_batch,
            mario_batch,
            item_batch,

            goomba_sheet,
            koopa_sheet,
            mario_sheet,
        }))
    }
}

impl SmbOneOne {
    fn load_nearby_objects(&self, engine: &Engine, lua: &Lua) -> Result<()> {
        for (obj, (Position(pos), _)) in self
            .space
            .borrow_mut()
            .query_mut::<(&Position, &Unloaded)>()
        {
            // Load the enemies in right before they come on screen
            if (pos.translation.vector.x)
                <= ((*self.x_scroll.borrow() + engine.mq().screen_size().0 / 4.0)
                    + 8.0
                    + LOAD_DISTANCE_IN_PIXELS)
            {
                self.to_load.borrow_mut().push(obj);
            }
        }

        for obj_to_load in self.to_load.borrow_mut().drain(..) {
            self.space
                .borrow_mut()
                .remove_one::<Unloaded>(obj_to_load)?;
            let table = LuaTable::from_lua(obj_to_load.to_lua(lua)?, lua)?;
            table.call_method("on_load", ())?;
        }

        Ok(())
    }

    fn run_required_lua_updates(&self, _engine: &Engine, lua: &Lua, dt: f32) -> Result<()> {
        let mut to_update = self.to_update.borrow_mut();

        for (obj, ()) in self
            .space
            .borrow_mut()
            .query_mut::<()>()
            .with::<RequiresLuaUpdate>()
            .without::<Unloaded>()
        {
            to_update.push(obj);
        }

        for obj_to_update in to_update.drain(..) {
            let table = LuaTable::from_lua(obj_to_update.to_lua(lua)?, lua)?;
            table.call_method("update", dt)?;
        }

        Ok(())
    }

    fn integrate_objects_without_colliders(
        &self,
        _engine: &Engine,
        _lua: &Lua,
        dt: f32,
    ) -> Result<()> {
        // Query: integrate positions for all objects w/o colliders.
        for (_, (Position(pos), Velocity(vel))) in self
            .space
            .borrow_mut()
            .query_mut::<(&mut Position, &Velocity)>()
            .without::<Collider>()
        {
            pos.integrate_mut(vel, dt);
        }
        Ok(())
    }

    fn integrate_object_positions(&self, _engine: &Engine, lua: &Lua, _dt: f32) -> Result<()> {
        let mut to_headbutt = self.to_headbutt.borrow_mut();
        let map = self.map.borrow();

        // Query: handle collisions between blocks and objects with positions, velocities, and
        // colliders. In addition, collect "headbutt" events to be dispatched to Lua once the
        // query is finished and the borrows are released.
        to_headbutt.clear();
        for (player_object, (Position(pos), Velocity(vel), collider, maybe_player)) in
            self.space.borrow_mut().query_mut::<(
                &mut Position,
                &mut Velocity,
                &Collider,
                Option<&PlayerMarker>,
            )>()
        {
            let mut is_grounded = false;

            // First, resolve X-axis collisions and movement.
            pos.translation.vector.x += vel.linear.x * TIMESTEP;

            let mut aabb = collider.compute_aabb(pos);
            let pixel_aabb = aabb.floor_to_i32();

            for (tile, x, y) in map.get_tiles_in_bb(
                pixel_aabb,
                *map.tile_layer_map.get("Foreground").unwrap(),
                CoordSpace::Pixel,
            ) {
                let mut tile_bb = Box2::<f32>::invalid();
                if let Some(object_group) = map.get_obj_grp_from_tile_id(&tile) {
                    for object in map.get_objs_from_obj_group(object_group) {
                        tile_bb.merge(&Box2::new(
                            object.x + (x * map.meta_data.tilewidth as i32) as f32,
                            object.y + (y * map.meta_data.tileheight as i32) as f32,
                            object.width,
                            object.height,
                        ));
                    }
                }

                if aabb.intersects(&tile_bb) {
                    let overlap = aabb.overlap(&tile_bb);
                    let intersection = aabb.intersection(&tile_bb);

                    // Only process this collision if we are more than "touching".
                    if intersection.extents().x > 0. && intersection.extents().y > 0. {
                        pos.translation.vector.x -= overlap.x;
                        aabb = collider.compute_aabb(pos);

                        if vel.linear.x.signum() == overlap.x.signum() {
                            if maybe_player.is_none() {
                                // If we're not a player, swap the direction
                                vel.linear.x *= -1.;
                            } else {
                                // If the collision is in the direction we're moving, stop.
                                vel.linear.x = 0.;
                            }

                            // TODO: Collision state (touching left/right)
                        }
                    }
                }
            }

            // Second, resolve Y-axis collisions and movement.
            pos.translation.vector.y += vel.linear.y * TIMESTEP;

            let mut aabb = collider.compute_aabb(pos);
            let pixel_aabb = aabb.floor_to_i32();

            // This is a specialized variant of the collision checks from before where we only look
            // for Y collisions happening "above" the player. This is our "block headbutt" check,
            // and it tries to find the collision candidate above the player which is closest to the
            // player's coordinate; this is so that there's no mysterious behavior where the player
            // can't headbutt a block because they're just barely touching an adjacent block or
            // something. "Distance" used for picking these candidates is just X axis distance; no
            // need to consider Y.
            //
            // The biggest difference is that this loop does not change the player's velocity or
            // position. Its only job is to check for headbutts. Resolution is taken care of for all
            // collider + position objects after this if block.
            if maybe_player.is_some() {
                let mut closest = None;

                // This is most likely overkill - we only really need to check the tiles above the
                // player. But that would depend on the player's hitbox, which will change when
                // transforming from big to small or vice versa, and this is general enough to cover
                // all the possibilities.
                for (tile, x, y) in map.get_tiles_in_bb(
                    pixel_aabb,
                    *map.tile_layer_map.get("Foreground").unwrap(),
                    CoordSpace::Pixel,
                ) {
                    let mut tile_bb = Box2::<f32>::invalid();
                    if let Some(object_group) = map.get_obj_grp_from_tile_id(&tile) {
                        for object in map.get_objs_from_obj_group(object_group) {
                            tile_bb.merge(&Box2::new(
                                object.x + (x * map.meta_data.tilewidth as i32) as f32,
                                object.y + (y * map.meta_data.tileheight as i32) as f32,
                                object.width,
                                object.height,
                            ));
                        }
                    }

                    if aabb.intersects(&tile_bb) {
                        let overlap = aabb.overlap(&tile_bb);
                        let intersection = aabb.intersection(&tile_bb);

                        // If we're colliding, our velocity is positive, we're colliding from the
                        // bottom, and we're the player (in this loop), then register a headbutt
                        // candidate.
                        if intersection.extents().x > 0.
                            && intersection.extents().y > 0.
                            && vel.linear.y.signum() > 0.
                            && overlap.y.signum() > 0.
                        {
                            // Woo that's a long string to get out an `Option<u32>` containing
                            // `Some` if the tileset has a tile this tile should turn into when it
                            // gets hit!
                            let hittable = map
                                .tilesets
                                .get_tile(&tile)
                                .unwrap()
                                .properties
                                .get_property("hittable")
                                .map(hv_tiled::Property::as_int)
                                .transpose()?
                                .copied()
                                .map(|x| x as u32);

                            let distance = (pos.center().x
                                - (x as f32 + 0.5) * (map.meta_data.tilewidth as f32))
                                .abs();

                            match closest {
                                Some((_, _, _, _, cdistance)) if cdistance <= distance => {}
                                _ => closest = Some((x, y, tile, hittable, distance)),
                            }
                        }
                    }
                }

                // If we were headbutting a block, then `closest` now contains the closest headbutt
                // candidate.
                if let Some((x, y, tile, hittable, _)) = closest {
                    to_headbutt.push((player_object, (x, y, tile, hittable)));
                }
            }

            for (tile, x, y) in map.get_tiles_in_bb(
                pixel_aabb,
                *map.tile_layer_map.get("Foreground").unwrap(),
                CoordSpace::Pixel,
            ) {
                let mut tile_bb = Box2::<f32>::invalid();
                if let Some(object_group) = map.get_obj_grp_from_tile_id(&tile) {
                    for object in map.get_objs_from_obj_group(object_group) {
                        tile_bb.merge(&Box2::new(
                            object.x + (x * map.meta_data.tilewidth as i32) as f32,
                            object.y + (y * map.meta_data.tileheight as i32) as f32,
                            object.width,
                            object.height,
                        ));
                    }
                }

                if aabb.intersects(&tile_bb) {
                    let overlap = aabb.overlap(&tile_bb);
                    let intersection = aabb.intersection(&tile_bb);

                    if intersection.extents().x > 0. && intersection.extents().y > 0. {
                        pos.translation.vector.y -= overlap.y;
                        aabb = collider.compute_aabb(pos);

                        if vel.linear.y.signum() == overlap.y.signum() {
                            vel.linear.y = 0.;

                            // TODO: Collision state (touching up/down)
                            if overlap.y.signum() < 0. {
                                is_grounded = true;
                            }
                        }
                    }
                }
            }

            player_object
                .to_table(lua)?
                .set("is_grounded", is_grounded)?;
        }

        // Release the map borrow so that Lua can modify it.
        drop(map);

        // Dispatch any headbutt events gathered from the previous query.
        for (player_object, (x, y, tile, hittable)) in to_headbutt.drain(..) {
            LuaTable::from_lua(player_object.to_lua(lua)?, lua)?
                .call_method("on_headbutt_block", (x, y, tile.to_index(), hittable))?;
        }

        Ok(())
    }

    fn dispatch_object_on_object_collisions(&self, _engine: &Engine, lua: &Lua) -> Result<()> {
        let mut to_collide = self.to_collide.borrow_mut();
        // Mario can only collide with 1 enemy per frame, so we limit the amount of objects here
        // to just 1

        // Collect any object-on-object collisions events, for later dispatch to Lua.
        to_collide.clear();
        for (object1, (Position(pos1), collider1)) in self
            .space
            .borrow()
            .query::<(&Position, &Collider)>()
            .without::<Unloaded>()
            .without::<PlayerMarker>()
            .iter()
        {
            for (object2, (Position(pos2), collider2)) in self
                .space
                .borrow()
                .query::<(&Position, &Collider)>()
                .without::<Unloaded>()
                .iter()
                .filter(|&(object2, _)| object1 != object2)
            {
                if parry2d::query::intersection_test(
                    &(pos1.to_isometry() * collider1.local_tx),
                    collider1.shape.as_ref(),
                    &(pos2.to_isometry() * collider2.local_tx),
                    collider2.shape.as_ref(),
                )? {
                    to_collide.push((object1, object2));
                }
            }
        }

        let space = self.space.borrow();
        let mut mario_query = space
            .query::<(&Position, &Collider)>()
            .with::<PlayerMarker>();

        let (object1, (Position(pos1), collider1)) = mario_query.iter().next().unwrap();

        for (object2, (Position(pos2), collider2)) in self
            .space
            .borrow()
            .query::<(&Position, &Collider)>()
            .without::<Unloaded>()
            .iter()
            .filter(|&(object2, _)| object1 != object2)
        {
            if parry2d::query::intersection_test(
                &(pos1.to_isometry() * collider1.local_tx),
                collider1.shape.as_ref(),
                &(pos2.to_isometry() * collider2.local_tx),
                collider2.shape.as_ref(),
            )? {
                to_collide.push((object1, object2));
                break;
            }
        }

        drop(mario_query);
        drop(space);

        // Dispatch collected player-on-enemy collision events.
        for (object1, object2) in to_collide.drain(..) {
            LuaTable::from_lua(object1.to_lua(lua)?, lua)?
                .call_method("on_collide_with_object", object2)?;
        }

        Ok(())
    }

    fn update_object_sprite_batches(&self, engine: &Engine, lua: &Lua) -> Result<()> {
        {
            let mut goomba_batch = self.goomba_batch.borrow_mut();
            goomba_batch.clear();
            let goomba_sheet = self.goomba_sheet.get();
            for (_, (Position(pos), animation)) in self
                .space
                .borrow_mut()
                .query_mut::<(&Position, &mut SpriteAnimation)>()
                .with::<GoombaMarker>()
            {
                let frame = goomba_sheet[animation.animation.frame_id];
                goomba_batch.insert(
                    Instance::new()
                        .translate2(pos.center().coords - Vector2::new(8., 8.))
                        .src(frame.uvs)
                        .translate2(frame.offset),
                );
            }
        }

        {
            let mut koopa_batch = self.koopa_batch.borrow_mut();
            koopa_batch.clear();
            let koopa_sheet = self.koopa_sheet.get();
            for (_, (Position(pos), animation)) in self
                .space
                .borrow_mut()
                .query_mut::<(&Position, &mut SpriteAnimation)>()
                .with::<KoopaMarker>()
            {
                let frame = koopa_sheet[animation.animation.frame_id];
                koopa_batch.insert(
                    Instance::new()
                        .translate2(pos.center().coords - Vector2::new(8., 8.))
                        .src(frame.uvs)
                        .translate2(frame.offset),
                );
            }
        }

        {
            let mut mario_batch = self.mario_batch.borrow_mut();
            mario_batch.clear();
            let mario_sheet = self.mario_sheet.get();
            for (object, (Position(pos), animation)) in self
                .space
                .borrow_mut()
                .query_mut::<(&mut Position, &mut SpriteAnimation)>()
                .with::<PlayerMarker>()
            {
                // 8 is just faster than doing self.map.meta_data.tilewidth as f32 / 2.0
                if pos.translation.vector.x - 8.0 <= 0.0 {
                    pos.translation.vector.x = 8.0;
                }

                let scroll = pos.translation.vector.x - (engine.mq().screen_size().0 / 8.0)
                    + (self.map.borrow().meta_data.tilewidth as f32 / 2.0);
                if scroll < 0.0 {
                    *self.x_scroll.borrow_mut() = 0.0;
                } else {
                    *self.x_scroll.borrow_mut() = scroll;
                }

                let frame = mario_sheet[animation.animation.frame_id];
                let facing_dir: i32 = object.to_table(lua)?.get("facing_direction")?;

                let mut instance = Instance::new()
                    .translate2(pos.center().coords - Vector2::new(8., 8.))
                    .src(frame.uvs);

                // If facing left, flip.
                if facing_dir == -1 {
                    instance = instance
                        .translate2(Vector2::new(16., 0.))
                        .scale2(Vector2::new(-1., 1.));
                }

                mario_batch.insert(instance.translate2(frame.offset));
            }
        }

        {
            let mut item_batch = self.item_batch.borrow_mut();
            item_batch.clear();
            for (_object, (Position(pos), ItemMarker(id))) in
                self.space
                    .borrow_mut()
                    .query_mut::<(&Position, &ItemMarker)>()
            {
                let (trd, _, _) = self
                    .ts_render_data
                    .get_render_data_for_tile(TileId::new(*id, 0, false, false, false));

                let uvs = match trd {
                    hv_tiled::TileRenderData::Static(uvs) => uvs,
                    _ => panic!("oh no"),
                };

                item_batch.insert(
                    Instance::new()
                        .translate2(pos.center().coords - Vector2::new(8., 8.))
                        .src(uvs),
                );
            }
        }

        Ok(())
    }

    fn update_normal(&self, engine: &Engine, lua: &Lua, dt: f32) -> Result<()> {
        self.tile_layer_batches
            .borrow_mut()
            .update_all_batches(dt, &self.ts_render_data);

        self.load_nearby_objects(engine, lua)?;
        self.run_required_lua_updates(engine, lua, dt)?;
        self.integrate_object_positions(engine, lua, dt)?;
        self.dispatch_object_on_object_collisions(engine, lua)?;

        Ok(())
    }

    fn update_player_died(&self, _engine: &Engine, lua: &Lua, dt: f32) -> Result<()> {
        self.tile_layer_batches
            .borrow_mut()
            .update_all_batches(dt, &self.ts_render_data);

        let mut to_update = self.to_update.borrow_mut();

        for (obj, ()) in self
            .space
            .borrow()
            .query::<()>()
            .with::<PlayerMarker>()
            .iter()
        {
            to_update.push(obj);
        }

        for obj in to_update.drain(..) {
            let table = LuaTable::from_lua(obj.to_lua(lua)?, lua)?;
            table.call_method("update", ())?;
        }

        Ok(())
    }

    fn draw(&self, engine: &Engine) -> Result<()> {
        let graphics_lock = engine.get::<GraphicsLock>();
        let mut gfx = graphics_lock.lock();
        let scale = 4.0;

        gfx.modelview_mut().origin().translate2(
            (Vector2::new(*self.x_scroll.borrow() * -1.0, 0.) * scale).map(|t| t.floor()),
        );
        gfx.modelview_mut().push(None);
        gfx.modelview_mut().scale2(Vector2::new(4.0, 4.0));

        let tiled_instance = Instance::new().translate2(Vector2::new(0., 16.));
        let sky_layer = self.map.borrow().tile_layer_map["Sky"];
        let bg_layer = self.map.borrow().tile_layer_map["Background"];
        let fg_layer = self.map.borrow().tile_layer_map["Foreground"];
        let mut tile_layer_batches = self.tile_layer_batches.borrow_mut();

        tile_layer_batches
            .get_layer_mut(sky_layer)
            .draw_mut(&mut gfx, tiled_instance);
        tile_layer_batches
            .get_layer_mut(bg_layer)
            .draw_mut(&mut gfx, tiled_instance);
        self.item_batch
            .borrow_mut()
            .draw_mut(&mut gfx, Instance::new());
        self.goomba_batch
            .borrow_mut()
            .draw_mut(&mut gfx, Instance::new());
        self.koopa_batch
            .borrow_mut()
            .draw_mut(&mut gfx, Instance::new());
        self.mario_batch
            .borrow_mut()
            .draw_mut(&mut gfx, Instance::new());
        tile_layer_batches
            .get_layer_mut(fg_layer)
            .draw_mut(&mut gfx, tiled_instance);

        gfx.modelview_mut().pop();

        Ok(())
    }
}

impl LuaUserData for SmbOneOne {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("input_state", |_, this| Ok(this.input_state.clone()));
        fields.add_field_method_get("space", |_, this| Ok(this.space.clone()));
        fields.add_field_method_get("button", |lua, this| {
            lua.registry_value::<LuaValue>(&this.button_table)
        });
        fields.add_field_method_get("sprite_sheets", |lua, this| {
            lua.registry_value::<LuaValue>(&this.sprite_sheets_table)
        });
        fields.add_field_method_get("RequiresLuaUpdate", |_, _| {
            Ok(DynamicComponentConstructor::copy(RequiresLuaUpdate))
        });
        fields.add_field_method_get("GoombaMarker", |_, _| {
            Ok(DynamicComponentConstructor::copy(GoombaMarker))
        });
        fields.add_field_method_get("KoopaMarker", |_, _| {
            Ok(DynamicComponentConstructor::copy(KoopaMarker))
        });
        fields.add_field_method_get("PlayerMarker", |_, _| {
            Ok(DynamicComponentConstructor::copy(PlayerMarker))
        });
        fields.add_field_method_get("ItemMarker", |lua, _| {
            lua.create_function(|_, id| Ok(DynamicComponentConstructor::copy(ItemMarker(id))))
        });
        fields.add_field_method_get("Unloaded", |_, _| {
            Ok(DynamicComponentConstructor::copy(Unloaded))
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        let get_engine = |lua: &Lua| -> LuaResult<EngineRef> {
            Ok((*lua.get_resource::<EngineRef>()?.borrow()).clone())
        };

        methods.add_method("update_normal", move |lua, this, dt| {
            this.update_normal(&get_engine(lua)?.upgrade(), lua, dt)
                .to_lua_err()?;
            Ok(())
        });

        methods.add_method("update_player_died", move |lua, this, dt| {
            this.update_player_died(&get_engine(lua)?.upgrade(), lua, dt)
                .to_lua_err()?;
            Ok(())
        });

        methods.add_method("draw", move |lua, this, ()| {
            this.draw(&get_engine(lua)?.upgrade()).to_lua_err()?;
            Ok(())
        });

        methods.add_method("load_nearby_objects", move |lua, this, ()| {
            this.load_nearby_objects(&get_engine(lua)?.upgrade(), lua)
                .to_lua_err()?;
            Ok(())
        });

        methods.add_method("run_required_lua_updates", move |lua, this, dt| {
            this.run_required_lua_updates(&get_engine(lua)?.upgrade(), lua, dt)
                .to_lua_err()?;
            Ok(())
        });

        methods.add_method(
            "integrate_objects_without_colliders",
            move |lua, this, dt| {
                this.integrate_objects_without_colliders(&get_engine(lua)?.upgrade(), lua, dt)
                    .to_lua_err()?;
                Ok(())
            },
        );

        methods.add_method("integrate_object_positions", move |lua, this, dt| {
            this.integrate_object_positions(&get_engine(lua)?.upgrade(), lua, dt)
                .to_lua_err()?;
            Ok(())
        });

        methods.add_method(
            "dispatch_object_on_object_collisions",
            move |lua, this, ()| {
                this.dispatch_object_on_object_collisions(&get_engine(lua)?.upgrade(), lua)
                    .to_lua_err()?;
                Ok(())
            },
        );

        methods.add_method("update_object_sprite_batches", move |lua, this, ()| {
            this.update_object_sprite_batches(&get_engine(lua)?.upgrade(), lua)
                .to_lua_err()?;
            Ok(())
        });

        methods.add_method(
            "set_tile",
            move |_lua, this, (x, y, tile_id, tileset_id): (i32, i32, u32, u32)| {
                let tile_id = TileId::new(tile_id, tileset_id, false, false, false);
                let layer_id = *this.map.borrow().tile_layer_map.get("Foreground").unwrap();
                let map_set =
                    this.map
                        .borrow_mut()
                        .set_tile(x, y, layer_id, tile_id, CoordSpace::Tile);
                this.tile_layer_batches
                    .borrow_mut()
                    .set_tile(&map_set, &this.ts_render_data);

                Ok(())
            },
        );

        methods.add_method("remove_tile", move |_lua, this, (x, y): (i32, i32)| {
            let layer_id = *this.map.borrow().tile_layer_map.get("Foreground").unwrap();
            if let Some(map_set) =
                this.map
                    .borrow_mut()
                    .remove_tile(x, y, CoordSpace::Tile, layer_id)
            {
                this.tile_layer_batches.borrow_mut().remove_tile(&map_set);
            }

            Ok(())
        });

        methods.add_method("reset_map", move |lua, this, ()| {
            this.map.borrow_mut().clone_from(&this.map_initial_state);
            let engine = lua.get_resource::<EngineRef>()?;
            *this.tile_layer_batches.borrow_mut() = hv_tiled::TileLayerBatches::new(
                &this.map.borrow().tile_layers,
                &this.ts_render_data,
                &this.map.borrow(),
                &engine.borrow().upgrade(),
            );

            Ok(())
        });
    }
}

struct SmbOneOneEventHandler {
    simple_handler: SimpleHandler,
    input_binding: InputBinding<Axis, Button>,
    input_state: Shared<InputState<Axis, Button>>,
    timer: TimeContext,
    inner: Shared<SmbOneOne>,
}

impl SmbOneOneEventHandler {
    fn new(engine: &Engine) -> Result<Self> {
        let input_state = Shared::new(InputState::new());
        Ok(Self {
            simple_handler: SimpleHandler::new("main"),
            timer: TimeContext::new(),
            inner: SmbOneOne::new(engine, input_state.clone())?,
            input_state,
            input_binding: default_input_bindings(),
        })
    }
}

impl EventHandler for SmbOneOneEventHandler {
    fn init(&mut self, engine: &Engine) -> Result<()> {
        engine.lua().globals().set("game", self.inner.clone())?;
        self.simple_handler.init(engine)
    }

    fn update(&mut self, engine: &Engine, _dt: f32) -> Result<()> {
        self.timer.tick();
        let mut counter = 0;
        while self.timer.check_update_time_forced(60, &mut counter) {
            self.simple_handler.update(engine, TIMESTEP)?;
            self.input_state.borrow_mut().update(TIMESTEP);
        }

        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        self.simple_handler.draw(engine)
    }

    fn key_down_event(&mut self, _: &Engine, keycode: KeyCode, _: KeyMods, _: bool) {
        if let Some(effect) = self.input_binding.resolve_keycode(keycode) {
            self.input_state.borrow_mut().update_effect(effect, true);
        }
    }

    fn key_up_event(&mut self, _: &Engine, keycode: KeyCode, _: KeyMods) {
        if let Some(effect) = self.input_binding.resolve_keycode(keycode) {
            self.input_state.borrow_mut().update_effect(effect, false);
        }
    }

    fn gamepad_button_down_event(&mut self, _: &Engine, button: GamepadButton, _: bool) {
        if let Some(effect) = self.input_binding.resolve_gamepad_button(button) {
            self.input_state.borrow_mut().update_effect(effect, true);
        }
    }

    fn gamepad_button_up_event(&mut self, _engine: &Engine, button: GamepadButton) {
        if let Some(effect) = self.input_binding.resolve_gamepad_button(button) {
            self.input_state.borrow_mut().update_effect(effect, false);
        }
    }

    fn gamepad_axis_changed_event(&mut self, _: &Engine, axis: GamepadAxis, position: f32) {
        if let Some(effect) = self.input_binding.resolve_gamepad_axis(axis, position) {
            self.input_state
                .borrow_mut()
                .update_effect(effect, position.abs() > f32::EPSILON);
        }
    }

    fn char_event(&mut self, _engine: &Engine, _character: char, _keymods: KeyMods, _repeat: bool) {
        // self.inner
        //     .borrow_mut()
        //     .char_event(engine, character, keymods, repeat)
    }

    fn mouse_motion_event(&mut self, _engine: &Engine, _x: f32, _y: f32) {
        // self.inner.borrow_mut().mouse_motion_event(engine, x, y)
    }

    fn mouse_wheel_event(&mut self, _engine: &Engine, _x: f32, _y: f32) {
        // self.inner.borrow_mut().mouse_wheel_event(engine, x, y)
    }

    fn mouse_button_down_event(
        &mut self,
        _engine: &Engine,
        _button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
        // self.inner
        //     .borrow_mut()
        //     .mouse_button_down_event(engine, button, x, y)
    }

    fn mouse_button_up_event(&mut self, _engine: &Engine, _button: MouseButton, _x: f32, _y: f32) {
        // self.inner
        //     .borrow_mut()
        //     .mouse_button_up_event(engine, button, x, y)
    }

    fn resize_event(&mut self, _engine: &Engine, _width: f32, _height: f32) {
        // self.inner.borrow_mut().resize_event(engine, width, height)
    }
}

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();

    let conf = Conf {
        filesystem: Filesystem::from_project_dirs(Path::new(""), "smb1-1", "Heavy Orbit").unwrap(),
        window_width: 1024,
        window_height: 960,
        ..Conf::default()
    };

    Engine::run(conf, SmbOneOneEventHandler::new)
}
