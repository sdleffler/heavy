use hv_core::{
    conf::Conf,
    engine::{Engine, EventHandler, LuaExt, Resource},
    filesystem::Filesystem,
    input::{InputBinding, InputState, KeyCode, MouseButton},
    prelude::*,
    spaces::{
        object_table::{ObjectTableComponent, ObjectTableRegistry, UpdateHookComponent},
        Space, Spaces,
    },
    util::RwLockExt,
};
use hv_friends::{
    graphics::{
        Canvas, ClearOptions, Color, DrawMode, Drawable, GraphicsLock, GraphicsLockExt, Instance,
        Mesh, MeshBuilder,
    },
    math::*,
    Position, Velocity,
};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Axes {
    Horz,
    Vert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Buttons {
    Swing,
    Crush,
    Dash,
    Focus,
}

fn default_input_binding() -> InputBinding<Axes, Buttons> {
    InputBinding::new()
        .bind_key_to_axis(KeyCode::W, Axes::Vert, 1.)
        .bind_key_to_axis(KeyCode::S, Axes::Vert, -1.)
        .bind_key_to_axis(KeyCode::A, Axes::Horz, -1.)
        .bind_key_to_axis(KeyCode::D, Axes::Horz, 1.)
        .bind_key_to_button(KeyCode::Space, Buttons::Dash)
        .bind_key_to_button(KeyCode::LeftShift, Buttons::Focus)
        .bind_mouse_to_button(MouseButton::Left, Buttons::Swing)
        .bind_mouse_to_button(MouseButton::Right, Buttons::Crush)
}

struct Game {
    space: Resource<Space>,
    input_binding: InputBinding<Axes, Buttons>,
    input_state: InputState<Axes, Buttons>,
    gfx_resource: Resource<GraphicsLock>,
    mesh: Mesh,
    paused: bool,
    canvas: Canvas,
}

impl Game {
    fn new(engine: &Engine) -> Result<Self> {
        let spaces = engine.get::<Spaces>();
        let space = spaces.borrow_mut().create_space();
        let gfx_resource = engine.get::<GraphicsLock>();

        let mesh;
        let canvas;

        {
            let mut gfx = gfx_resource.lock();

            gfx.set_projection(Orthographic3::new(0., 640., 0., 480., -1., 1.).to_homogeneous());

            mesh = MeshBuilder::new(gfx.state.null_texture.clone())
                .polygon(
                    DrawMode::fill(),
                    &[
                        Point2::new(-4., -4.),
                        Point2::new(4., 0.),
                        Point2::new(-4., 4.),
                        Point2::new(-4., 0.),
                    ],
                    Color::WHITE,
                )?
                .build(&mut gfx);

            canvas = Canvas::new(&mut gfx, 640, 480);
        }

        engine.lua().globals().set("space", space.clone())?;

        Ok(Self {
            space,
            input_binding: default_input_binding(),
            input_state: InputState::new(),
            gfx_resource,
            mesh,
            paused: true,
            canvas,
        })
    }

    fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        if self.input_state.get_button_pressed(Buttons::Dash) {
            self.paused = !self.paused;
        }

        if !self.paused {
            for (_, (Position(pos), Velocity(vel))) in self
                .space
                .borrow_mut()
                .query_mut::<(&mut Position, &mut Velocity)>()
            {
                pos.integrate_mut(vel, dt);
            }

            {
                let lua = engine.lua();
                let object_table_registry = lua.resource::<ObjectTableRegistry>()?;
                for (object, (object_table,)) in self
                    .space
                    .borrow_mut()
                    .query_mut::<(&ObjectTableComponent,)>()
                    .with::<UpdateHookComponent>()
                {
                    let maybe_table = object_table_registry
                        .borrow()
                        .by_index(object_table.index)
                        .map(|entry| lua.registry_value::<LuaTable>(entry.key()))
                        .transpose()?;

                    if let Some(table) = maybe_table {
                        if let Some(update) = table.get::<_, Option<LuaFunction>>("update")? {
                            update.call(table)?;
                        }
                    } else {
                        log::error!(
                            "{:?} has object table component but no corresponding table entry",
                            object
                        );
                    }
                }
            }

            engine
                .lua()
                .globals()
                .get::<_, LuaTable>("hv")?
                .call_function("update", dt)?;
        }

        self.input_state.update(dt);

        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        {
            let mut gfx = self.gfx_resource.lock();

            gfx.begin_render_pass(
                Some(&self.canvas.render_pass),
                Some(ClearOptions {
                    color: Some(Color::BLACK),
                    ..ClearOptions::default()
                }),
            );
            gfx.apply_default_pipeline();
            gfx.apply_modelview();

            for (_, (Position(pos),)) in self.space.borrow_mut().query_mut::<(&Position,)>() {
                self.mesh.draw(
                    &mut gfx,
                    Instance::new()
                        .translate2(pos.translation.vector)
                        .rotate2(pos.rotation.angle()),
                );
            }
        }

        engine
            .lua()
            .globals()
            .get::<_, LuaTable>("hv")?
            .call_function("draw", ())?;

        {
            let mut gfx = self.gfx_resource.lock();
            // let mesh = MeshBuilder::new(gfx.state.null_texture.clone())
            //     .circle(DrawMode::fill(), Point2::origin(), 2., 0.1, Color::WHITE)
            //     .build(&mut gfx);

            // for (_, (projectile,)) in self.space.borrow_mut().query_mut::<(&ProjectileState,)>() {
            //     let tx = projectile.tx();
            //     mesh.draw(
            //         &mut gfx,
            //         InstanceParam::new()
            //             .translate2(tx.translation.vector)
            //             .rotate2(tx.rotation.angle())
            //             .color(projectile.color),
            //     );
            // }

            gfx.end_render_pass();
            gfx.begin_render_pass(None, Some(ClearOptions::default()));
            gfx.apply_default_pipeline();
            gfx.apply_modelview();

            gfx.draw(&self.canvas, None);

            gfx.end_render_pass();
            gfx.mq.commit_frame();
        }
        Ok(())
    }
}

struct GameHandler {
    inner: Option<Game>,
}

impl GameHandler {
    fn new() -> Self {
        Self { inner: None }
    }
}

impl EventHandler for GameHandler {
    fn init(&mut self, engine: &Engine) -> Result<()> {
        self.inner = Some(Game::new(engine)?);
        engine.lua().load(mlua::chunk! { require("main") }).exec()?;
        Ok(())
    }

    fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        self.inner.as_mut().unwrap().update(engine, dt)
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        self.inner.as_mut().unwrap().draw(engine)
    }

    fn key_down_event(
        &mut self,
        _engine: &Engine,
        keycode: KeyCode,
        _keymods: hv_core::input::KeyMods,
        _repeat: bool,
    ) {
        let inner_mut = self.inner.as_mut().unwrap();
        if let Some(effect) = inner_mut.input_binding.resolve_keycode(keycode) {
            log::debug!("kd effect: {:?}", effect);
            inner_mut.input_state.update_effect(effect, true);
        }
    }

    fn key_up_event(
        &mut self,
        _engine: &Engine,
        keycode: KeyCode,
        _keymods: hv_core::input::KeyMods,
    ) {
        let inner_mut = self.inner.as_mut().unwrap();
        if let Some(effect) = inner_mut.input_binding.resolve_keycode(keycode) {
            inner_mut.input_state.update_effect(effect, false);
        }
    }

    fn mouse_button_down_event(&mut self, _engine: &Engine, button: MouseButton, x: f32, y: f32) {
        let inner_mut = self.inner.as_mut().unwrap();
        if let Some(effect) = inner_mut
            .input_binding
            .resolve_mouse_button(button, Point2::new(x, y))
        {
            inner_mut.input_state.update_effect(effect, true);
        }
    }

    fn mouse_button_up_event(&mut self, _engine: &Engine, button: MouseButton, x: f32, y: f32) {
        let inner_mut = self.inner.as_mut().unwrap();
        if let Some(effect) = inner_mut
            .input_binding
            .resolve_mouse_button(button, Point2::new(x, y))
        {
            inner_mut.input_state.update_effect(effect, false);
        }
    }
}

fn main() {
    hv_friends::link_me();
    hv_rain::link_me();

    simple_logger::SimpleLogger::new()
        .with_module_level("rustyline", log::LevelFilter::Warn)
        .init()
        .unwrap();

    let conf = Conf {
        window_width: 640 * 2,
        window_height: 480 * 2,
        filesystem: Filesystem::from_project_dirs(Path::new("examples/foo"), "foo", "Shea Leffler")
            .unwrap(),
        ..Conf::default()
    };

    Engine::run(conf, GameHandler::new())
}
