#![feature(drain_filter)]

use smallvec::SmallVec;

use hv_core::{
    components::DynamicComponentConstructor,
    engine::{Engine, LuaExt, WeakResourceCache},
    plugins::Plugin,
    prelude::*,
    shared::Weak,
    spaces::{Object, Space},
};
use hv_friends::{
    graphics::{
        pipeline::{Pipeline, PipelineLayout, Shader, ShaderLayout},
        sprite::CachedSpriteSheet,
        CachedTexture, Color, DrawableMut, Graphics, GraphicsLock, GraphicsLockExt, Instance,
        SpriteBatch,
    },
    math::*,
};

use crate::{
    graphics::{
        ProjectileSprite, ProjectileSpriteBatch, ProjectileSpriteBatchId, ProjectileSpriteRegistry,
    },
    pattern::{Barrage, LuaComponentFunctionShotType, Parameters, ShotTypeRegistry},
    sm::{StateIndex, StateMachine, StateRegistry},
};

pub mod graphics;
pub mod pattern;
pub mod sm;

#[derive(Debug, Clone, Copy)]
pub struct ProjectileGroupMarker<const N: u8>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectileGroup(pub u8);

#[derive(Debug, Clone, Copy)]
pub struct ProjectileState {
    pub time: f32,

    pub origin: Isometry2<f32>,

    pub linear_tx: Isometry2<f32>,
    pub linear_vel: Velocity2<f32>,
    pub linear_accel: Velocity2<f32>,

    pub polar_tx: Isometry2<f32>,
    pub polar_vel: Velocity2<f32>,
    pub polar_accel: Velocity2<f32>,

    pub color: Color,
    pub sprite: Option<ProjectileSprite>,

    sm_init: bool,
    kill: bool,
}

impl ProjectileState {
    pub fn from_parameters(params: &Parameters) -> Self {
        Self {
            time: 0.,
            origin: params.origin,
            linear_tx: params.linear_tx,
            linear_vel: params.linear_vel,
            linear_accel: params.linear_accel,
            polar_tx: params.polar_tx,
            polar_vel: params.polar_vel,
            polar_accel: params.polar_accel,
            color: params.color,
            sprite: params.sprite,
            sm_init: false,
            kill: false,
        }
    }

    pub fn tx(&self) -> Isometry2<f32> {
        let oriented_polar_vector = self
            .origin
            .rotation
            .transform_vector(&self.polar_tx.translation.vector);
        Isometry2::from_parts(
            self.origin.translation
                * self.linear_tx.translation
                * Translation2::from(oriented_polar_vector),
            self.origin.rotation * self.linear_tx.rotation * self.polar_tx.rotation,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LinearVelocity;

/// Linear "polar" movement.
#[derive(Debug, Clone, Copy)]
pub struct PolarVelocity;

/// Linear acceleration.
#[derive(Debug, Clone, Copy)]
pub struct LinearAcceleration;

#[derive(Debug, Clone, Copy)]
pub struct PolarAcceleration;

#[derive(Debug, Clone)]
pub struct ProjectileTrail {
    pub prev: SmallVec<[Isometry2<f32>; 256]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bullet(Object);

pub struct Danmaku {
    space: Weak<Space>,
}

impl Danmaku {
    pub fn new(space: &Shared<Space>) -> Result<Self> {
        Ok(Self {
            space: Shared::downgrade(space),
        })
    }

    pub fn update(&self, lua: &Lua, dt: f32) -> Result<()> {
        let space = &mut self.space.borrow_mut();
        let state_registry_resource = lua.get_resource::<StateRegistry>()?;
        let state_registry = &state_registry_resource.borrow();
        let sprite_registry_resource = lua.get_resource::<ProjectileSpriteRegistry>()?;

        for (_, (projectile, state_machine)) in
            space.query_mut::<(&mut ProjectileState, &mut StateMachine)>()
        {
            if !projectile.sm_init {
                projectile.sm_init = true;
                state_registry.enter(lua, projectile, state_machine);
            }

            state_registry.update(lua, dt, projectile, state_machine);
        }

        {
            let sprite_registry = &mut sprite_registry_resource.borrow_mut();
            sprite_registry.clear_batches();

            for (
                _,
                (projectile, (maybe_lin_accel, maybe_polar_accel, maybe_lin_vel, maybe_polar_vel)),
            ) in space.query_mut::<(
                &mut ProjectileState,
                (
                    Option<&LinearAcceleration>,
                    Option<&PolarAcceleration>,
                    Option<&LinearVelocity>,
                    Option<&PolarVelocity>,
                ),
            )>() {
                if maybe_lin_accel.is_some() {
                    projectile.linear_vel += projectile.linear_accel * dt;
                }

                if maybe_lin_vel.is_some() {
                    let integrated = projectile.linear_vel.integrate(dt);
                    projectile.linear_tx =
                        integrated.translation * projectile.linear_tx * integrated.rotation;
                }

                if maybe_polar_accel.is_some() {
                    projectile.polar_vel += projectile.polar_accel * dt;
                }

                if maybe_polar_vel.is_some() {
                    let integrated = projectile.polar_vel.integrate(dt);
                    projectile.polar_tx =
                        integrated.rotation * projectile.polar_tx * integrated.translation;
                }

                if projectile.sprite.is_some() {
                    let tx = projectile.tx();
                    let sprite = projectile.sprite.as_mut().unwrap();
                    let batch = &mut sprite_registry[sprite.batch_id];
                    let sheet = batch.sheet.get_cached();
                    sheet.update_animation(dt, &mut sprite.animation_state);
                    let frame = &sheet[sprite.animation_state.frame_id];

                    batch.sprites.insert(
                        Instance::new()
                            .src(frame.uvs)
                            .translate2(tx.translation.vector)
                            .rotate2(tx.rotation.angle())
                            .translate2(frame.offset)
                            .color(projectile.color),
                    );
                }
            }
        }

        Ok(())
    }

    pub fn draw(&self, lua: &Lua, gfx: &mut Graphics) -> Result<()> {
        let sprite_registry_resource = lua.get_resource::<ProjectileSpriteRegistry>()?;
        let sprite_registry = &mut sprite_registry_resource.borrow_mut();

        gfx.push_pipeline();
        for (_, batch) in sprite_registry.defs.iter_mut() {
            match batch.pipeline.as_ref() {
                Some(pl) => gfx.apply_pipeline(pl),
                None => gfx.apply_default_pipeline(),
            }

            batch.sprites.draw_mut(gfx, Instance::new());
        }
        gfx.pop_pipeline();

        Ok(())
    }
}

impl LuaUserData for Danmaku {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("create_barrage_object", |_, this, ()| {
            Ok(Barrage::new(&this.space.upgrade()))
        });

        methods.add_method("update", |lua, this, dt| {
            this.update(lua, dt).to_lua_err()?;
            Ok(())
        });

        methods.add_method("draw", |lua, this, ()| {
            let gfx_lock = lua.get_resource::<GraphicsLock>()?;
            this.draw(lua, &mut gfx_lock.lock()).to_lua_err()?;
            Ok(())
        });
    }
}

struct HvRainPlugin;

impl Plugin for HvRainPlugin {
    fn name(&self) -> &'static str {
        "rain"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>> {
        engine.fs().add_zip_file(
            std::io::Cursor::new(include_bytes!("../resources/scripts.zip")),
            Some(std::path::PathBuf::from("hv-rain/resources/scripts")),
        )?;

        let shot_type_registry = engine.insert(ShotTypeRegistry::new());
        lua.insert_resource(shot_type_registry.clone())?;

        let state_registry = engine.insert(StateRegistry::new());
        lua.insert_resource(state_registry.clone())?;

        let sprite_registry = engine.insert(ProjectileSpriteRegistry::new());
        lua.insert_resource(sprite_registry.clone())?;

        let create_danmaku_object =
            lua.create_function_mut(move |_lua, space| Danmaku::new(&space).to_lua_err())?;

        let weak_registry = Shared::downgrade(&shot_type_registry);
        let create_shot_type_from_component_fn =
            lua.create_function(move |lua, component_fn: LuaFunction| {
                let lcfst =
                    LuaComponentFunctionShotType::from_lua(LuaValue::Function(component_fn), lua)?;
                Ok(weak_registry.borrow_mut().register(Box::new(lcfst)))
            })?;

        let weak_registry = Shared::downgrade(&sprite_registry);
        let create_projectile_sprite_batch = lua.create_function(
            move |lua,
                  (texture, sheet, pipeline): (
                CachedTexture,
                CachedSpriteSheet,
                Option<Pipeline>,
            )| {
                let registry = &mut weak_registry.borrow_mut();
                let gfx_lock = lua.get_resource::<GraphicsLock>()?;
                let batch = SpriteBatch::new(&mut gfx_lock.lock(), texture);

                Ok(ProjectileSpriteBatchId(registry.defs.insert(
                    ProjectileSpriteBatch {
                        sheet,
                        sprites: batch,
                        pipeline,
                    },
                )))
            },
        )?;

        let get_state_registry = lua.create_function(move |_, ()| Ok(state_registry.clone()))?;

        let linear_velocity_component_constructor =
            DynamicComponentConstructor::new(|_: &Lua, _| Ok(LinearVelocity));
        let polar_velocity_component_constructor =
            DynamicComponentConstructor::new(|_: &Lua, _| Ok(PolarVelocity));
        let linear_acceleration_component_constructor =
            DynamicComponentConstructor::new(|_: &Lua, _| Ok(LinearAcceleration));
        let polar_acceleration_component_constructor =
            DynamicComponentConstructor::new(|_: &Lua, _| Ok(PolarAcceleration));

        let state_machine_component_constructor = lua.create_function(|_, index: StateIndex| {
            Ok(DynamicComponentConstructor::new(move |_: &Lua, _| {
                Ok(StateMachine::new(index))
            }))
        })?;

        let mut weak_gfx_cache = WeakResourceCache::<GraphicsLock>::new();
        let mut color_bullet_pipeline = None;
        let get_color_bullet_pipeline =
            lua.create_function_mut(move |lua, ()| match color_bullet_pipeline.clone() {
                Some(pl) => Ok(pl),
                None => {
                    let gfx_lock = weak_gfx_cache.get(|| lua.get_resource())?;
                    let gfx = &mut gfx_lock.lock();

                    let bullet_shader = Shader::new(
                        gfx,
                        include_str!("graphics/bullet_es300.glslv"),
                        include_str!("graphics/bullet_es300.glslf"),
                        ShaderLayout::default(),
                    )
                    .to_lua_err()?;

                    let bullet_pipeline =
                        Pipeline::new(gfx, PipelineLayout::default(), bullet_shader, None)
                            .to_lua_err()?;
                    color_bullet_pipeline = Some(bullet_pipeline.clone());

                    Ok(bullet_pipeline)
                }
            })?;

        let weak_registry = Shared::downgrade(&sprite_registry);
        let projectile_sprite_component_constructor = lua.create_function(
            move |_,
                  (projectile_sprite, tag, should_loop): (
                ProjectileSpriteBatchId,
                LuaString,
                Option<bool>,
            )| {
                let registry = &mut weak_registry.borrow_mut();
                let sheet = registry[projectile_sprite].sheet.get_cached();
                let tag_id = sheet
                    .get_tag(tag.to_str()?)
                    .ok_or_else(|| anyhow!("no such tag"))
                    .to_lua_err()?;
                let animation_state = sheet.at_tag(tag_id, should_loop.unwrap_or(true));

                Ok(ProjectileSprite {
                    batch_id: projectile_sprite,
                    animation_state,
                })
            },
        )?;

        Ok(lua
            .load(mlua::chunk! {
                {
                    create_danmaku_object = $create_danmaku_object,
                    create_projectile_sprite_batch = $create_projectile_sprite_batch,
                    create_shot_type_from_component_fn = $create_shot_type_from_component_fn,
                    linear_velocity_component_constructor = $linear_velocity_component_constructor,
                    polar_velocity_component_constructor = $polar_velocity_component_constructor,
                    linear_acceleration_component_constructor = $linear_acceleration_component_constructor,
                    polar_acceleration_component_constructor = $polar_acceleration_component_constructor,
                    state_machine_component_constructor = $state_machine_component_constructor,
                    projectile_sprite_component_constructor = $projectile_sprite_component_constructor,
                    get_state_registry = $get_state_registry,
                    get_color_bullet_pipeline = $get_color_bullet_pipeline,
                    nil
                }
            })
            .eval()?)
    }

    fn load<'lua>(&self, lua: &'lua Lua, _engine: &Engine) -> Result<()> {
        let chunk = mlua::chunk! {
            rain = require("rain")
        };
        lua.load(chunk).exec()?;

        Ok(())
    }
}

hv_core::plugin!(HvRainPlugin);

pub fn link_me() {}
