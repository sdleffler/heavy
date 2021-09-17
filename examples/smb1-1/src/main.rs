use std::path::Path;

use hv_core::{
    components::DynamicComponentConstructor,
    conf::Conf,
    engine::{Engine, EngineRefCache, EventHandler},
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
struct Unloaded;

struct SmbOneOne {
    space: Shared<Space>,
    tile_layer_batches: hv_tiled::TileLayerBatches,
    x_scroll: f32,
    map: hv_tiled::Map,
    ts_render_data: TilesetRenderData,
    to_update: Vec<Object>,
    to_collide: Vec<(Object, Object)>,
    to_headbutt: Vec<(Object, (u32, u32, TileId))>,
    to_load: Vec<Object>,

    goomba_batch: SpriteBatch<CachedTexture>,
    koopa_batch: SpriteBatch<CachedTexture>,
    mario_batch: SpriteBatch<CachedTexture>,

    goomba_sheet: CachedSpriteSheet,
    koopa_sheet: CachedSpriteSheet,
    mario_sheet: CachedSpriteSheet,
}

impl SmbOneOne {
    fn new(engine: &Engine, input_state: Shared<InputState<Axis, Button>>) -> Result<Shared<Self>> {
        let space = engine.get::<Spaces>().borrow_mut().create_space();
        let lua = engine.lua();
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

        {
            let button = lua.create_table()?;
            button.set("A", Button::A)?;
            button.set("B", Button::B)?;
            button.set("Start", Button::Start)?;
            button.set("Left", Button::Left)?;
            button.set("Right", Button::Right)?;
            button.set("Down", Button::Down)?;
            button.set("Up", Button::Up)?;

            let space = space.clone();

            let requires_update = DynamicComponentConstructor::copy(RequiresLuaUpdate);
            let goomba_marker = DynamicComponentConstructor::copy(GoombaMarker);
            let koopa_marker = DynamicComponentConstructor::copy(KoopaMarker);
            let player_marker = DynamicComponentConstructor::copy(PlayerMarker);
            let unloaded = DynamicComponentConstructor::copy(Unloaded);

            let goomba_sheet = goomba_sheet.clone();
            let koopa_sheet = koopa_sheet.clone();
            let mario_sheet = mario_sheet.clone();

            let chunk = mlua::chunk! {
                {
                    input = $input_state,
                    button = $button,
                    space = $space,

                    sprite_sheets = {
                        goomba = $goomba_sheet,
                        koopa = $koopa_sheet,
                        mario = $mario_sheet,
                    },

                    RequiresLuaUpdate = $requires_update,
                    GoombaMarker = $goomba_marker,
                    KoopaMarker = $koopa_marker,
                    PlayerMarker = $player_marker,
                    Unloaded = $unloaded,
                }
            };

            lua.globals()
                .set("rust", lua.load(chunk).eval::<LuaTable>()?)?;
        }
        drop(lua);

        let map = hv_tiled::Map::new("/maps/mario_bros_1-1.lua", engine, Some("maps/"))?;

        let ts_render_data = hv_tiled::TilesetRenderData::new(&map.tilesets, engine)?;

        let tile_layer_batches =
            hv_tiled::TileLayerBatches::new(&map.tile_layers, &ts_render_data, &map, engine);

        let gfx_lock = engine.get::<GraphicsLock>();
        let mut gfx = gfx_lock.lock();
        let goomba_batch = SpriteBatch::new(&mut gfx, goomba_texture);
        let koopa_batch = SpriteBatch::new(&mut gfx, koopa_texture);
        let mario_batch = SpriteBatch::new(&mut gfx, mario_texture);
        drop(gfx);

        Ok(Shared::new(SmbOneOne {
            space,
            tile_layer_batches,
            x_scroll: 0.,
            map,
            ts_render_data,
            to_update: Vec::new(),
            to_collide: Vec::new(),
            to_headbutt: Vec::new(),
            to_load: Vec::new(),

            goomba_batch,
            koopa_batch,
            mario_batch,

            goomba_sheet,
            koopa_sheet,
            mario_sheet,
        }))
    }
}

impl SmbOneOne {
    fn load_nearby_objects(&mut self, engine: &Engine, lua: &Lua) -> Result<()> {
        for (obj, (Position(pos), _)) in self
            .space
            .borrow_mut()
            .query_mut::<(&Position, &Unloaded)>()
        {
            // Load the enemies in right before they come on screen
            if (pos.translation.vector.x)
                <= ((self.x_scroll + engine.mq().screen_size().0 / 4.0)
                    + 8.0
                    + LOAD_DISTANCE_IN_PIXELS)
            {
                self.to_load.push(obj);
            }
        }

        for obj_to_load in self.to_load.drain(..) {
            self.space
                .borrow_mut()
                .remove_one::<Unloaded>(obj_to_load)?;
            let table = LuaTable::from_lua(obj_to_load.to_lua(lua)?, lua)?;
            table.call_method("on_load", ())?;
        }

        Ok(())
    }

    fn run_required_lua_updates(&mut self, _engine: &Engine, lua: &Lua, dt: f32) -> Result<()> {
        for (obj, ()) in self
            .space
            .borrow_mut()
            .query_mut::<()>()
            .with::<RequiresLuaUpdate>()
            .without::<Unloaded>()
        {
            self.to_update.push(obj);
        }

        for obj_to_update in self.to_update.drain(..) {
            let table = LuaTable::from_lua(obj_to_update.to_lua(lua)?, lua)?;
            table.call_method("update", dt)?;
        }

        Ok(())
    }

    fn integrate_object_positions(&mut self, _engine: &Engine, lua: &Lua, dt: f32) -> Result<()> {
        // Query: integrate positions for all objects w/o colliders.
        for (_, (Position(pos), Velocity(vel))) in self
            .space
            .borrow_mut()
            .query_mut::<(&mut Position, &Velocity)>()
            .without::<Collider>()
        {
            pos.integrate_mut(vel, dt);
        }

        // Query: handle collisions between blocks and objects with positions, velocities, and
        // colliders. In addition, collect "headbutt" events to be dispatched to Lua once the
        // query is finished and the borrows are released.
        self.to_headbutt.clear();
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
            let pixel_aabb = aabb.floor_to_u32();

            for (tile, x, y) in self.map.get_tiles_in_bb_in_layer(
                pixel_aabb,
                *self.map.tile_layer_map.get("Foreground").unwrap(),
                CoordSpace::Pixel,
            ) {
                let mut tile_bb = Box2::<f32>::invalid();
                if let Some(object_group) = self.map.get_obj_grp_from_tile_id(&tile) {
                    for object in self.map.get_objs_from_obj_group(object_group) {
                        tile_bb.merge(&Box2::new(
                            object.x + (x * self.map.meta_data.tilewidth) as f32,
                            object.y + (y * self.map.meta_data.tileheight) as f32,
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
            let pixel_aabb = aabb.floor_to_u32();

            for (tile, x, y) in self.map.get_tiles_in_bb_in_layer(
                pixel_aabb,
                *self.map.tile_layer_map.get("Foreground").unwrap(),
                CoordSpace::Pixel,
            ) {
                let mut tile_bb = Box2::<f32>::invalid();
                if let Some(object_group) = self.map.get_obj_grp_from_tile_id(&tile) {
                    for object in self.map.get_objs_from_obj_group(object_group) {
                        tile_bb.merge(&Box2::new(
                            object.x + (x * self.map.meta_data.tilewidth) as f32,
                            object.y + (y * self.map.meta_data.tileheight) as f32,
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
                            } else if overlap.y.signum() > 0. && maybe_player.is_some() {
                                self.to_headbutt.push((player_object, (x, y, tile)));
                            }
                        }
                    }
                }
            }

            player_object
                .to_table(lua)?
                .set("is_grounded", is_grounded)?;
        }

        // Dispatch any headbutt events gathered from the previous query.
        for (player_object, (x, y, _tile)) in self.to_headbutt.drain(..) {
            LuaTable::from_lua(player_object.to_lua(lua)?, lua)?
                .call_method("on_headbutt_block", (x, y))?;
        }

        Ok(())
    }

    fn dispatch_object_on_object_collisions(&mut self, _engine: &Engine, lua: &Lua) -> Result<()> {
        // Collect any object-on-object collisions events, for later dispatch to Lua.
        //
        // TODO: we may want to modify this query loop to also collect enemy-on-enemy
        // collision events (so goombas and koopas and so on can change direction when they
        // hit each other and spinning koopa shells can kill other enemies, etc.)
        self.to_collide.clear();
        for (object1, (Position(pos1), collider1)) in self
            .space
            .borrow()
            .query::<(&Position, &Collider)>()
            .without::<Unloaded>()
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
                    self.to_collide.push((object1, object2));
                }
            }
        }

        // Dispatch collected player-on-enemy collision events.
        for (object1, object2) in self.to_collide.drain(..) {
            LuaTable::from_lua(object1.to_lua(lua)?, lua)?
                .call_method("on_collide_with_object", object2)?;
        }

        Ok(())
    }

    fn update_object_sprite_batches(&mut self, engine: &Engine, lua: &Lua) -> Result<()> {
        self.goomba_batch.clear();
        let goomba_sheet = self.goomba_sheet.get_cached();
        for (_, (Position(pos), animation)) in self
            .space
            .borrow_mut()
            .query_mut::<(&Position, &mut SpriteAnimation)>()
            .with::<GoombaMarker>()
        {
            let frame = goomba_sheet[animation.animation.frame_id];
            self.goomba_batch.insert(
                Instance::new()
                    .translate2(pos.center().coords - Vector2::new(8., 8.))
                    .src(frame.uvs)
                    .translate2(frame.offset),
            );
        }

        self.koopa_batch.clear();
        let koopa_sheet = self.koopa_sheet.get_cached();
        for (_, (Position(pos), animation)) in self
            .space
            .borrow_mut()
            .query_mut::<(&Position, &mut SpriteAnimation)>()
            .with::<KoopaMarker>()
        {
            let frame = koopa_sheet[animation.animation.frame_id];
            self.koopa_batch.insert(
                Instance::new()
                    .translate2(pos.center().coords - Vector2::new(8., 8.))
                    .src(frame.uvs)
                    .translate2(frame.offset),
            );
        }

        self.mario_batch.clear();
        let mario_sheet = self.mario_sheet.get_cached();
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
                + (self.map.meta_data.tilewidth as f32 / 2.0);
            if scroll < 0.0 {
                self.x_scroll = 0.0;
            } else {
                self.x_scroll = scroll;
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

            self.mario_batch.insert(instance.translate2(frame.offset));
        }

        Ok(())
    }

    fn update(&mut self, engine: &Engine, lua: &Lua, dt: f32) -> Result<()> {
        self.tile_layer_batches
            .update_all_batches(dt, &self.ts_render_data);

        if lua.globals().get("is_player_dead")? {
            for (obj, ()) in self
                .space
                .borrow()
                .query::<()>()
                .with::<PlayerMarker>()
                .iter()
            {
                self.to_update.push(obj);
            }

            for obj in self.to_update.iter() {
                let table = LuaTable::from_lua(obj.to_lua(lua)?, lua)?;
                table.call_method("update", ())?;
            }
        } else {
            self.load_nearby_objects(engine, lua)?;
            self.run_required_lua_updates(engine, lua, dt)?;
            self.integrate_object_positions(engine, lua, dt)?;
            self.dispatch_object_on_object_collisions(engine, lua)?;
        }

        self.update_object_sprite_batches(engine, lua)?;

        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        let graphics_lock = engine.get::<GraphicsLock>();
        let mut gfx = graphics_lock.lock();
        let scale = 4.0;

        gfx.modelview_mut()
            .origin()
            .translate2((Vector2::new(self.x_scroll * -1.0, 0.0) * scale).map(|t| t.floor()));
        gfx.modelview_mut().push(None);
        gfx.modelview_mut().scale2(Vector2::new(4.0, 4.0));

        self.tile_layer_batches.draw_mut(&mut gfx, Instance::new());
        self.goomba_batch.draw_mut(&mut gfx, Instance::new());
        self.koopa_batch.draw_mut(&mut gfx, Instance::new());
        self.mario_batch.draw_mut(&mut gfx, Instance::new());

        gfx.modelview_mut().pop();

        Ok(())
    }
}

impl LuaUserData for SmbOneOne {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        let mut engine_cache = EngineRefCache::new();
        methods.add_method_mut("update", move |lua, this, dt| {
            this.update(&engine_cache.get(lua), lua, dt).to_lua_err()?;
            Ok(())
        });

        let mut engine_cache = EngineRefCache::new();
        methods.add_method_mut("draw", move |lua, this, ()| {
            this.draw(&engine_cache.get(lua)).to_lua_err()?;
            Ok(())
        });

        let mut engine_cache = EngineRefCache::new();
        methods.add_method_mut("load_nearby_objects", move |lua, this, ()| {
            this.load_nearby_objects(&engine_cache.get(lua), lua)
                .to_lua_err()?;
            Ok(())
        });

        let mut engine_cache = EngineRefCache::new();
        methods.add_method_mut("run_required_lua_updates", move |lua, this, dt| {
            this.run_required_lua_updates(&engine_cache.get(lua), lua, dt)
                .to_lua_err()?;
            Ok(())
        });

        let mut engine_cache = EngineRefCache::new();
        methods.add_method_mut("integrate_object_positions", move |lua, this, dt| {
            this.integrate_object_positions(&engine_cache.get(lua), lua, dt)
                .to_lua_err()?;
            Ok(())
        });

        let mut engine_cache = EngineRefCache::new();
        methods.add_method_mut(
            "dispatch_object_on_object_collisions",
            move |lua, this, ()| {
                this.dispatch_object_on_object_collisions(&engine_cache.get(lua), lua)
                    .to_lua_err()?;
                Ok(())
            },
        );

        let mut engine_cache = EngineRefCache::new();
        methods.add_method_mut("update_object_sprite_batches", move |lua, this, ()| {
            this.update_object_sprite_batches(&engine_cache.get(lua), lua)
                .to_lua_err()?;
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
