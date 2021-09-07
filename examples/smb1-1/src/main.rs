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
    graphics::{
        Color, DrawMode, DrawableMut, GraphicsLock, GraphicsLockExt, Instance, MeshBuilder,
    },
    math::*,
    Position, SimpleHandler, Velocity,
};

use std::io::Read;

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
    x_scroll: usize,
    map_data: hv_tiled::MapData,
    timer: TimeContext,

    to_update: Vec<Object>,
}

impl SmbOneOne {
    pub fn new(engine: &Engine) -> Result<Self, Error> {
        let space = engine.get::<Spaces>().borrow_mut().create_space();
        let input_state = Shared::new(InputState::new());
        let mut fs = engine.fs();
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

        let mut tiled_lua_map = fs.open(Path::new("/maps/mario_bros_1-1.lua"))?;
        drop(fs);
        let mut tiled_buffer: Vec<u8> = Vec::new();
        tiled_lua_map.read_to_end(&mut tiled_buffer)?;
        let lua_chunk = lua.load(&tiled_buffer);
        let tiled_lua_table = lua_chunk.eval::<LuaTable>()?;
        let map_data = hv_tiled::MapData::from_lua(&tiled_lua_table)?;

        let mut tiled_layers = Vec::new();

        for layer in tiled_lua_table
            .get::<_, LuaTable>("layers")?
            .sequence_values::<LuaTable>()
        {
            tiled_layers.push(hv_tiled::Layer::from_lua(&layer?)?);
        }

        let mut tilesets = Vec::new();

        for tileset in tiled_lua_table
            .get::<_, LuaTable>("tilesets")?
            .sequence_values::<LuaTable>()
        {
            tilesets.push(hv_tiled::Tileset::from_lua(&tileset?)?);
        }

        drop(tiled_lua_table);
        drop(lua);

        let tileset_atlas = hv_tiled::TilesetAtlas::new(tilesets, engine)?;

        let mut layer_batches = Vec::with_capacity(tiled_layers.len());

        for layer in tiled_layers.iter() {
            layer_batches.push(hv_tiled::LayerBatch::new(
                layer,
                &tileset_atlas,
                engine,
                &map_data,
            ));
        }

        let mut simple_handler = SimpleHandler::new("main");
        simple_handler.init(engine)?;

        Ok(SmbOneOne {
            input_binding: default_input_bindings(),
            input_state,
            space,
            layer_batches,
            x_scroll: 0,
            map_data,
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

            for (_obj, (Position(pos), Velocity(vel))) in self
                .space
                .borrow_mut()
                .query_mut::<(&mut Position, &Velocity)>()
            {
                pos.integrate_mut(vel, 1. / 60.);
            }

            // self.x_scroll += 1;
            if self.x_scroll
                > ((self.map_data.width * self.map_data.tilewidth)
                    - (engine.mq().screen_size().0 as usize / 4))
            {
                self.x_scroll = 0;
            }
        }
        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        let graphics_lock = engine.get::<GraphicsLock>();
        let mut gfx = graphics_lock.lock();

        gfx.modelview_mut()
            .origin()
            .scale2(Vector2::new(4.0, 4.0))
            .translate2(Vector2::new((self.x_scroll as f32) * -1.0, 0.0));

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
