#![feature(option_get_or_insert_default)]

use hv_core::{
    conf::Conf,
    engine::{Engine, EventHandler, LuaExt},
    filesystem::Filesystem,
    input::{InputBinding, InputState, KeyCode, MouseButton},
    prelude::*,
    spaces::{
        object_table::{ObjectTableComponent, ObjectTableRegistry, UpdateHookComponent},
        Space, Spaces,
    },
};
use hv_friends::{
    camera::{Camera, CameraParameters},
    graphics::{
        Canvas, ClearOptions, Color, DrawMode, DrawableMut, GraphicsLock, GraphicsLockExt,
        Instance, Mesh, MeshBuilder,
    },
    math::*,
    scene::{DynamicScene, SceneStack},
    Position, Velocity,
};
use std::path::Path;

use crate::{combat_geometry::CombatGeometry, player::PlayerController};

mod box_geometry;
mod combat_geometry;
mod player;
mod scenes;

const INTERNAL_RESOLUTION: (u32, u32) = (640, 480);

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

#[ctor::ctor]
fn link_me() {
    hv_fmod::link_me();
    hv_friends::link_me();
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
    space: Shared<Space>,
    input_binding: InputBinding<Axes, Buttons>,
    input_state: InputState<Axes, Buttons>,
    gfx_resource: Shared<GraphicsLock>,
    mesh: Mesh,
    world_canvas: Canvas,
    static_canvas: Canvas,
    camera: Camera,
}

impl Game {
    fn new(engine: &Engine) -> Result<Self> {
        let spaces = engine.get::<Spaces>();
        let space = spaces.borrow_mut().create_space();
        let gfx_resource = engine.get::<GraphicsLock>();
        let (mesh, world_canvas, static_canvas);

        {
            let mut gfx = gfx_resource.lock();

            gfx.set_projection(
                Orthographic3::new(
                    0.,
                    INTERNAL_RESOLUTION.0 as f32,
                    0.,
                    INTERNAL_RESOLUTION.1 as f32,
                    -1.,
                    1.,
                )
                .to_homogeneous(),
            );

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

            world_canvas = Canvas::new(&mut gfx, INTERNAL_RESOLUTION.0, INTERNAL_RESOLUTION.1);
            static_canvas = Canvas::new(&mut gfx, INTERNAL_RESOLUTION.0, INTERNAL_RESOLUTION.1);
        }

        let camera = Camera::new(CameraParameters::new(Vector2::new(
            INTERNAL_RESOLUTION.0,
            INTERNAL_RESOLUTION.1,
        )));

        engine.lua().globals().set("space", space.clone())?;

        Ok(Self {
            space,
            input_binding: default_input_binding(),
            input_state: InputState::new(),
            gfx_resource,
            mesh,
            world_canvas,
            static_canvas,
            camera,
        })
    }

    fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        engine
            .get::<hv_mymachine::Console>()
            .borrow_mut()
            .poll(&engine.lua())?;

        for (_, (pos, vel, pc, cg)) in self.space.borrow_mut().query_mut::<(
            &mut Position,
            &mut Velocity,
            &mut PlayerController,
            &mut CombatGeometry,
        )>() {
            pc.update(pos, vel, cg, &self.input_state)?;
        }

        for (_, (Position(pos), Velocity(vel))) in self
            .space
            .borrow_mut()
            .query_mut::<(&mut Position, &mut Velocity)>()
        {
            pos.integrate_mut(vel, dt);
        }

        let player_pos = self
            .space
            .borrow_mut()
            .query_mut::<(&Position,)>()
            .into_iter()
            .next()
            .map(|(_, (&pos,))| pos);

        if let Some(Position(pos)) = player_pos {
            self.camera.set_subject_pos(pos.translation.vector.into());
        }

        self.camera.update(dt);

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
                        update.call((table, dt))?;
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

        self.input_state.update(dt);

        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        {
            let mut gfx = self.gfx_resource.lock();

            gfx.begin_render_pass(
                Some(&self.world_canvas.render_pass),
                Some(ClearOptions::default()),
            );

            gfx.apply_default_pipeline();

            gfx.modelview_mut().push(homogeneous_mat3_to_mat4(
                &self.camera.world_to_screen_tx().to_homogeneous(),
            ));

            gfx.apply_modelview();

            for (_, Position(pos)) in self.space.borrow_mut().query_mut::<&Position>() {
                self.mesh.draw_mut(
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

            for (_, (Position(pos), cg)) in self
                .space
                .borrow_mut()
                .query_mut::<(&Position, &CombatGeometry)>()
            {
                let mut mesh_builder = MeshBuilder::new(gfx.state.null_texture.clone());
                cg.append_debug_polygons_to_mesh(&mut mesh_builder)?;
                let mut mesh = mesh_builder.build(&mut gfx);
                mesh.draw_mut(
                    &mut gfx,
                    Instance::new()
                        .translate2(pos.translation.vector)
                        .rotate2(pos.rotation.angle())
                        .color(Color::new(1., 1., 1., 0.5)),
                );
            }

            gfx.modelview_mut().pop();
            gfx.end_render_pass();

            gfx.begin_render_pass(None, Some(ClearOptions::default()));
            gfx.apply_default_pipeline();
            gfx.apply_modelview();

            gfx.draw(&self.world_canvas, None);
            // gfx.draw(&self.static_canvas, None);

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
    simple_logger::SimpleLogger::new()
        .with_module_level("rustyline", log::LevelFilter::Warn)
        .init()
        .unwrap();

    let conf = Conf {
        window_width: INTERNAL_RESOLUTION.0 * 2,
        window_height: INTERNAL_RESOLUTION.1 * 2,
        filesystem: Filesystem::from_project_dirs(Path::new(""), "belltower", "Shea Leffler")
            .unwrap(),
        ..Conf::default()
    };

    Engine::run(conf, GameHandler::new())

    // Engine::run(
    //     conf,
    //     SceneStack::with_init(|stack, engine| {
    //         stack.push(DynamicScene::new(hv_talisman::EditorScene::new(engine)?));
    //         Ok(())
    //     }),
    // )
}
