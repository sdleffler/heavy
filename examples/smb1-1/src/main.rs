use std::path::Path;

use hv_core::{
    components::DynamicComponentConstructor,
    conf::Conf,
    engine::{Engine, EventHandler},
    filesystem::Filesystem,
    input::{GamepadAxis, GamepadButton, InputBinding, InputState, KeyCode, KeyMods},
    prelude::*,
    spaces::{Object, Space, Spaces},
    timer::TimeContext,
};

use hv_friends::{
    collision::Collider,
    graphics::{
        Color, DrawMode, DrawableMut, GraphicsLock, GraphicsLockExt, Instance, MeshBuilder,
    },
    math::*,
    Position, SimpleHandler, Velocity,
};
use hv_tiled::{BoxExt, CoordSpace};

const TIMESTEP: f32 = 1. / 60.;

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
struct RequiresUpdate;

struct SmbOneOne {
    space: Shared<Space>,
    input_binding: InputBinding<Axis, Button>,
    input_state: Shared<InputState<Axis, Button>>,
    layer_batches: Vec<hv_tiled::LayerBatch>,
    x_scroll: f32,
    map: hv_tiled::Map,
    timer: TimeContext,

    to_update: Vec<Object>,
}

impl SmbOneOne {
    pub fn new(engine: &Engine) -> Result<Self, Error> {
        let space = engine.get::<Spaces>().borrow_mut().create_space();
        let input_state = Shared::new(InputState::new());
        let lua = engine.lua();

        {
            let button = lua.create_table()?;
            button.set("A", Button::A)?;
            button.set("B", Button::B)?;
            button.set("Start", Button::Start)?;
            button.set("Left", Button::Left)?;
            button.set("Right", Button::Right)?;
            button.set("Down", Button::Down)?;
            button.set("Up", Button::Up)?;

            let input_state = input_state.clone();
            let space = space.clone();

            let requires_update = DynamicComponentConstructor::copy(RequiresUpdate);

            let chunk = mlua::chunk! {
                {
                    input = $input_state,
                    button = $button,
                    space = $space,

                    RequiresUpdate = $requires_update,
                }
            };

            lua.globals()
                .set("rust", lua.load(chunk).eval::<LuaTable>()?)?;
        }
        drop(lua);

        let map = hv_tiled::Map::new("/maps/mario_bros_1-1.lua", engine, Some("maps/"))?;

        let tileset_atlas = hv_tiled::TilesetAtlas::new(&map.tilesets, engine)?;

        let mut layer_batches = Vec::with_capacity(map.layers.len());

        for layer in map.layers.iter() {
            layer_batches.push(hv_tiled::LayerBatch::new(
                layer,
                &tileset_atlas,
                engine,
                &map.meta_data,
            ));
        }

        let mut simple_handler = SimpleHandler::new("main");
        simple_handler.init(engine)?;

        Ok(SmbOneOne {
            input_binding: default_input_bindings(),
            input_state,
            space,
            layer_batches,
            x_scroll: 0.,
            map,
            timer: TimeContext::new(),

            to_update: Vec::new(),
        })
    }
}

impl EventHandler for SmbOneOne {
    fn update(&mut self, engine: &Engine, _dt: f32) -> Result<()> {
        let lua = engine.lua();

        self.timer.tick();
        let mut counter = 0;
        while self.timer.check_update_time_forced(60, &mut counter) {
            for (obj, ()) in self
                .space
                .borrow_mut()
                .query_mut::<()>()
                .with::<RequiresUpdate>()
            {
                self.to_update.push(obj);
            }

            for obj_to_update in self.to_update.drain(..) {
                let table = LuaTable::from_lua(obj_to_update.to_lua(&lua)?, &lua)?;
                table.call_method("update", ())?;
            }

            for (_obj, (Position(pos), Velocity(vel), collider)) in self
                .space
                .borrow_mut()
                .query_mut::<(&mut Position, &mut Velocity, &Collider)>()
            {
                // First, resolve X-axis collisions and movement.
                pos.translation.vector.x += vel.linear.x * TIMESTEP;

                let mut aabb = collider.compute_aabb(pos);
                let pixel_aabb = aabb.floor_to_u32();

                for (tile, x, y) in self.map.get_tiles_in_bb_in_layer(
                    pixel_aabb,
                    *self.map.layer_map.get("Foreground").unwrap(),
                    CoordSpace::Pixel,
                ) {
                    let mut tile_bb = Box2::<f32>::invalid();
                    if let Some(object_group) = self
                        .map
                        .tilesets
                        .get_tile(&tile)
                        .and_then(|t| t.objectgroup.as_ref())
                    {
                        for object in &object_group.objects {
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
                                // If the collision is in the direction we're moving, stop.
                                vel.linear.x = 0.;

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
                    *self.map.layer_map.get("Foreground").unwrap(),
                    CoordSpace::Pixel,
                ) {
                    let mut tile_bb = Box2::<f32>::invalid();
                    if let Some(object_group) = self
                        .map
                        .tilesets
                        .get_tile(&tile)
                        .and_then(|t| t.objectgroup.as_ref())
                    {
                        for object in &object_group.objects {
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

                                // TODO: Collision state (touching left/right)
                            }
                        }
                    }
                }
            }

            self.x_scroll += 0.25;
            if self.x_scroll
                > ((self.map.meta_data.width * self.map.meta_data.tilewidth)
                    - (engine.mq().screen_size().0 as u32 / 4)) as f32
            {
                self.x_scroll = 0.;
            }

            self.input_state.borrow_mut().update(TIMESTEP);
        }

        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        let graphics_lock = engine.get::<GraphicsLock>();
        let mut gfx = graphics_lock.lock();

        gfx.modelview_mut()
            .origin()
            .scale2(Vector2::new(4.0, 4.0))
            .translate2(Vector2::new(self.x_scroll * -1.0, 0.0));

        for layer_batch in self.layer_batches.iter_mut() {
            layer_batch.draw_mut(&mut gfx, Instance::new());
        }

        let mut space = self.space.borrow_mut();
        let mut mesh = MeshBuilder::new(gfx.state.null_texture.clone())
            .rectangle(
                DrawMode::fill(),
                Box2::from_half_extents(Point2::origin(), Vector2::new(8., 8.)),
                Color::RED,
            )
            .build(&mut gfx);

        for (_, Position(pos)) in space.query_mut::<&Position>() {
            mesh.draw_mut(&mut gfx, Instance::new().translate2(pos.center().coords));
        }

        Ok(())
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
        // quiet
    }

    fn mouse_motion_event(&mut self, _engine: &Engine, _x: f32, _y: f32) {
        // quiet
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

    Engine::run(conf, SmbOneOne::new)
}
