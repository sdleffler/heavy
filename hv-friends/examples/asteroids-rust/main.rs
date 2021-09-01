use std::path::Path;

use hv_core::{
    components::DynamicComponentConstructor,
    conf::Conf,
    engine::{Engine, EventHandler},
    filesystem::Filesystem,
    prelude::*,
    spaces::{Object, Space, Spaces},
};
use hv_friends::{
    graphics::{
        ClearOptions, Color, DrawMode, DrawableMut, GraphicsLock, GraphicsLockExt, Instance, Mesh,
        MeshBuilder,
    },
    math::*,
    Position, SimpleHandler, Velocity,
};

const CIRCLE_MESH_RADIUS: f32 = 64.;
const ARENA_WIDTH: f32 = 800.;
const ARENA_HEIGHT: f32 = 680.;

#[derive(Debug, Clone, Copy)]
pub struct Circle {
    radius: f32,
    color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct Asteroid;

#[derive(Debug, Clone, Copy)]
pub struct Player;

#[derive(Debug, Clone, Copy)]
pub struct Bullet {
    time_left: f32,
}

pub struct Asteroids {
    simple_handler: SimpleHandler,
    space: Shared<Space>,
    circle: Mesh,
    to_remove: Vec<Object>,
    to_destroy: Vec<Object>,
}

impl Asteroids {
    pub fn new(engine: &Engine) -> Result<Self> {
        let space = engine.get::<Spaces>().borrow_mut().create_space();

        let lua = engine.lua();
        let make_circle = lua.create_function(|_, (radius, r, g, b)| {
            Ok(DynamicComponentConstructor::copy(Circle {
                radius,
                color: Color::new(r, g, b, 1.),
            }))
        })?;
        let make_asteroid = DynamicComponentConstructor::copy(Asteroid);
        let make_player = DynamicComponentConstructor::copy(Player);
        let make_bullet = lua.create_function(|_, time_limit| {
            Ok(DynamicComponentConstructor::copy(Bullet {
                time_left: time_limit,
            }))
        })?;
        let space_ref = space.clone();

        lua.load(mlua::chunk! {
            hf = require("hf")

            asteroids_rust = {
                make_circle = $make_circle,
                make_asteroid = $make_asteroid,
                make_player = $make_player,
                make_bullet = $make_bullet,
                space = $space_ref,
                arenaWidth = $ARENA_WIDTH,
                arenaHeight = $ARENA_HEIGHT,
            }
        })
        .exec()?;

        let gfx_lock = engine.get::<GraphicsLock>();
        let mut gfx = gfx_lock.lock();
        let circle = MeshBuilder::new(gfx.state.null_texture.clone())
            .circle(
                DrawMode::fill(),
                Point2::origin(),
                CIRCLE_MESH_RADIUS,
                1.,
                Color::WHITE,
            )
            .build(&mut gfx);

        drop(gfx);
        drop(lua);

        let mut simple_handler = SimpleHandler::new("main");
        simple_handler.init(engine)?;

        Ok(Self {
            simple_handler,
            space,
            circle,
            to_remove: Vec::new(),
            to_destroy: Vec::new(),
        })
    }
}

impl EventHandler for Asteroids {
    fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        let mut space = self.space.borrow_mut();

        for (_, (Position(pos), Velocity(vel))) in space.query_mut::<(&mut Position, &Velocity)>() {
            pos.integrate_mut(vel, dt);
            let center = pos.center();
            let new_center = Point2::new(
                center.x.rem_euclid(ARENA_WIDTH),
                center.y.rem_euclid(ARENA_HEIGHT),
            );
            pos.translation.vector = new_center.coords;
        }

        for (bullet_object, (Position(bullet_pos), bullet, bullet_circle)) in
            space.query::<(&Position, &mut Bullet, &Circle)>().iter()
        {
            bullet.time_left -= dt;

            if bullet.time_left <= 0. {
                self.to_remove.push(bullet_object);
            } else {
                for (asteroid_object, (Position(asteroid_pos), asteroid_circle)) in space
                    .query::<(&Position, &Circle)>()
                    .with::<Asteroid>()
                    .iter()
                {
                    if na::distance_squared(&bullet_pos.center(), &asteroid_pos.center())
                        < (bullet_circle.radius + asteroid_circle.radius).powi(2)
                    {
                        self.to_remove.push(bullet_object);
                        self.to_destroy.push(asteroid_object);
                    }
                }
            }
        }

        for (player_object, (Position(player_pos), player_circle)) in space
            .query::<(&Position, &Circle)>()
            .with::<Player>()
            .iter()
        {
            for (_, (Position(asteroid_pos), asteroid_circle)) in space
                .query::<(&Position, &Circle)>()
                .with::<Asteroid>()
                .iter()
            {
                if na::distance_squared(&player_pos.center(), &asteroid_pos.center())
                    < (player_circle.radius + asteroid_circle.radius).powi(2)
                {
                    self.to_destroy.push(player_object);
                }
            }
        }

        let num_asteroids = space
            .query_mut::<()>()
            .with::<Asteroid>()
            .into_iter()
            .count();
        drop(space);

        let lua = engine.lua();
        let globals = lua.globals();

        for to_destroy in self.to_destroy.drain(..) {
            let value = to_destroy.to_lua(&lua)?;

            if let Ok(table) = LuaTable::from_lua(value, &lua) {
                let () = table.call_method("destroy", ())?;
            }
        }

        let mut space = self.space.borrow_mut();
        for to_remove in self.to_remove.drain(..) {
            let _ = space.despawn(to_remove);
        }
        drop(space);

        if num_asteroids == 0 {
            globals.call_function("reset", ())?;
        }

        drop(globals);
        drop(lua);

        self.simple_handler.update(engine, dt)
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        // This time, we don't delegate to the `SimpleHandler`.

        let gfx_lock = engine.get::<GraphicsLock>();

        let mut gfx = gfx_lock.lock();
        gfx.begin_render_pass(None, Some(ClearOptions::default()));

        for (_, (Position(pos), circle)) in self
            .space
            .borrow_mut()
            .query_mut::<(&Position, &Circle)>()
            .without::<Player>()
        {
            for (i, j) in (-1..=1).flat_map(|i| (-1..=1).map(move |j| (i, j))) {
                self.circle.draw_mut(
                    &mut gfx,
                    Instance::new()
                        .translate2(Vector2::new(
                            i as f32 * ARENA_WIDTH,
                            j as f32 * ARENA_HEIGHT,
                        ))
                        .translate2(pos.translation.vector)
                        .scale2(Vector2::repeat(circle.radius / CIRCLE_MESH_RADIUS))
                        .color(circle.color),
                );
            }
        }

        for (_, (Position(pos), circle)) in self
            .space
            .borrow_mut()
            .query_mut::<(&Position, &Circle)>()
            .with::<Player>()
        {
            for (i, j) in (-1..=1).flat_map(|i| (-1..=1).map(move |j| (i, j))) {
                self.circle.draw_mut(
                    &mut gfx,
                    Instance::new()
                        .translate2(Vector2::new(
                            i as f32 * ARENA_WIDTH,
                            j as f32 * ARENA_HEIGHT,
                        ))
                        .translate2(pos.translation.vector)
                        .scale2(Vector2::repeat(circle.radius / CIRCLE_MESH_RADIUS))
                        .color(circle.color),
                );

                self.circle.draw_mut(
                    &mut gfx,
                    Instance::new()
                        .translate2(Vector2::new(
                            i as f32 * ARENA_WIDTH,
                            j as f32 * ARENA_HEIGHT,
                        ))
                        .translate2(pos.translation.vector)
                        .rotate2(pos.rotation.angle())
                        .translate2(Vector2::x() * 20.)
                        .scale2(Vector2::repeat(5. / CIRCLE_MESH_RADIUS))
                        .color(Color::from_rgb(0, 255, 255)),
                );
            }
        }

        drop(gfx);

        engine
            .lua()
            .globals()
            .get::<_, LuaTable>("hv")?
            .call_function("draw", ())?;

        let mut gfx = gfx_lock.lock();
        gfx.end_render_pass();
        gfx.commit_frame();

        Ok(())
    }

    fn key_down_event(
        &mut self,
        engine: &Engine,
        keycode: hv_core::input::KeyCode,
        keymods: hv_core::input::KeyMods,
        repeat: bool,
    ) {
        self.simple_handler
            .key_down_event(engine, keycode, keymods, repeat)
    }

    fn key_up_event(
        &mut self,
        engine: &Engine,
        keycode: hv_core::input::KeyCode,
        keymods: hv_core::input::KeyMods,
    ) {
        self.simple_handler.key_up_event(engine, keycode, keymods)
    }
}

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();

    let conf = Conf {
        filesystem: Filesystem::from_project_dirs(
            Path::new("examples/asteroids-rust"),
            "asteroids-rust",
            "Shea Leffler",
        )
        .unwrap(),
        ..Conf::default()
    };

    Engine::run(conf, Asteroids::new)
}
