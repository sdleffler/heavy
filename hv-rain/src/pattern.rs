use anyhow::*;
use hecs::EntityBuilder;
use hv_core::{
    components::DynamicComponentConstructor,
    engine::{LuaExt, LuaResource, Resource},
    mlua::{prelude::*, Variadic as LuaVariadic},
    spaces::Space,
    util::RwLockExt,
};
use hv_friends::{graphics::Color, math::*};
use std::{
    collections::HashMap,
    sync::{RwLock, Weak},
};
use thunderdome::{Arena, Index};

use crate::{graphics::ProjectileSprite, ProjectileState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShotTypeIndex(Index);

impl<'lua> ToLua<'lua> for ShotTypeIndex {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        LuaLightUserData(self.0.to_bits() as *mut _).to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for ShotTypeIndex {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let lud = LuaLightUserData::from_lua(lua_value, lua)?;
        Ok(Self(Index::from_bits(lud.0 as u64)))
    }
}

pub trait ShotType: Send + Sync + 'static {
    fn spawn(
        &self,
        lua: &Lua,
        slots: &Arena<LuaRegistryKey>,
        space: &mut Space,
        shots: &[Parameters],
    ) -> Result<()>;
}

pub struct LuaComponentFunctionShotType {
    lua_fn: LuaRegistryKey,
}

impl ShotType for LuaComponentFunctionShotType {
    fn spawn(
        &self,
        lua: &Lua,
        _slots: &Arena<LuaRegistryKey>,
        space: &mut Space,
        shots: &[Parameters],
    ) -> Result<()> {
        let component_fn = lua.registry_value::<LuaFunction>(&self.lua_fn)?;

        let mut builder = EntityBuilder::new();
        for shot in shots {
            let object = space.reserve_object();
            let components: LuaVariadic<LuaAnyUserData> = component_fn.call(())?;

            builder.add(ProjectileState::from_parameters(shot));

            for component in components.iter() {
                let dynamic_component = component.borrow::<DynamicComponentConstructor>()?;
                dynamic_component
                    .add_to_object_builder(lua, object, &mut builder)
                    .to_lua_err()?;
            }

            space.insert(object, builder.build()).to_lua_err()?;
        }

        Ok(())
    }
}

impl<'lua> FromLua<'lua> for LuaComponentFunctionShotType {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let f = LuaFunction::from_lua(lua_value, lua)?;
        let rk = lua.create_registry_value(f)?;
        Ok(Self { lua_fn: rk })
    }
}

pub struct ShotTypeRegistry {
    shot_types: Arena<Box<dyn ShotType>>,
}

impl LuaResource for ShotTypeRegistry {
    const REGISTRY_KEY: &'static str = "HV_DANMAKU_SHOT_TYPE_REGISTRY";
}

impl LuaUserData for ShotTypeRegistry {}

impl ShotTypeRegistry {
    pub(crate) fn new() -> Self {
        Self {
            shot_types: Arena::new(),
        }
    }

    pub fn register(&mut self, shot_type: Box<dyn ShotType>) -> ShotTypeIndex {
        ShotTypeIndex(self.shot_types.insert(shot_type))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Parameters {
    pub origin: Isometry2<f32>,

    pub linear_tx: Isometry2<f32>,
    pub linear_vel: Velocity2<f32>,
    pub linear_accel: Velocity2<f32>,

    pub polar_tx: Isometry2<f32>,
    pub polar_vel: Velocity2<f32>,
    pub polar_accel: Velocity2<f32>,

    pub color: Color,
    pub sprite: Option<ProjectileSprite>,

    pub lua_value: Option<Index>,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            origin: Isometry2::identity(),

            linear_tx: Isometry2::identity(),
            linear_vel: Velocity2::zero(),
            linear_accel: Velocity2::zero(),

            polar_tx: Isometry2::identity(),
            polar_vel: Velocity2::zero(),
            polar_accel: Velocity2::zero(),

            color: Color::WHITE,
            sprite: None,

            lua_value: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Frame {
    params: Parameters,
    shot_type: Option<ShotTypeIndex>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            params: Default::default(),
            shot_type: None,
        }
    }
}

pub struct Barrage {
    space: Weak<RwLock<Space>>,
    stack: Vec<Frame>,
    batches: HashMap<ShotTypeIndex, Vec<Parameters>>,
    lua_slots: Arena<LuaRegistryKey>,
}

impl Barrage {
    pub fn new(space: &Resource<Space>) -> Self {
        Self {
            space: Resource::downgrade(space),
            stack: vec![Default::default()],
            batches: HashMap::new(),
            lua_slots: Arena::new(),
        }
    }

    pub fn push(&mut self) {
        let top = *self.stack.last().expect("empty stack");
        self.stack.push(top);
    }

    pub fn pop(&mut self) {
        self.stack.pop();

        if self.stack.is_empty() {
            self.stack.push(Default::default());
        }
    }

    pub fn top_params(&self) -> &Parameters {
        &self.stack.last().expect("empty stack").params
    }

    pub fn top_params_mut(&mut self) -> &mut Parameters {
        &mut self.stack.last_mut().expect("empty stack").params
    }

    pub fn set_lua_value(&mut self, registry_key: LuaRegistryKey) {
        let index = self.lua_slots.insert(registry_key);
        self.top_params_mut().lua_value = Some(index);
    }

    pub fn append_origin(&mut self, tx: &Isometry2<f32>) {
        let top = self.top_params_mut();
        top.origin *= tx;
    }

    pub fn prepend_origin(&mut self, tx: &Isometry2<f32>) {
        let top = self.top_params_mut();
        top.origin = tx * top.origin;
    }

    pub fn append_rotation_to_origin(&mut self, rot: &UnitComplex<f32>) {
        self.top_params_mut().origin.append_rotation_mut(rot);
    }

    pub fn append_linear_tx(&mut self, tx: &Isometry2<f32>) {
        let top = self.top_params_mut();
        top.linear_tx *= tx;
    }

    pub fn prepend_linear_tx(&mut self, tx: &Isometry2<f32>) {
        let top = self.top_params_mut();
        top.linear_tx = tx * top.linear_tx;
    }

    pub fn set_linear_tx(&mut self, tx: &Isometry2<f32>) {
        self.top_params_mut().linear_tx = *tx;
    }

    pub fn add_polar_tx(&mut self, tx: &Isometry2<f32>) {
        let top = self.top_params_mut();
        top.polar_tx = tx.translation * top.polar_tx * tx.rotation;
    }

    pub fn set_polar_tx(&mut self, tx: &Isometry2<f32>) {
        self.top_params_mut().polar_tx = *tx;
    }

    pub fn add_linear_velocity(&mut self, vel: &Velocity2<f32>) {
        let top = self.top_params_mut();
        top.linear_vel += vel.transformed(&top.origin);
    }

    pub fn set_linear_velocity(&mut self, vel: &Velocity2<f32>) {
        let top = self.top_params_mut();
        top.linear_vel = vel.transformed(&top.origin);
    }

    pub fn add_linear_velocity_wrt_world(&mut self, vel: &Velocity2<f32>) {
        self.top_params_mut().linear_vel += *vel;
    }

    pub fn set_linear_velocity_wrt_world(&mut self, vel: &Velocity2<f32>) {
        self.top_params_mut().linear_vel = *vel;
    }

    pub fn add_polar_velocity(&mut self, vel: &Velocity2<f32>) {
        self.top_params_mut().polar_vel += *vel;
    }

    pub fn set_polar_velocity(&mut self, vel: &Velocity2<f32>) {
        self.top_params_mut().polar_vel = *vel;
    }

    pub fn add_linear_acceleration(&mut self, accel: &Velocity2<f32>) {
        let top = self.top_params_mut();
        top.linear_accel += accel.transformed(&top.origin);
    }

    pub fn set_linear_acceleration(&mut self, accel: &Velocity2<f32>) {
        let top = self.top_params_mut();
        top.linear_accel = accel.transformed(&top.origin);
    }

    pub fn add_linear_acceleration_wrt_world(&mut self, accel: &Velocity2<f32>) {
        self.top_params_mut().linear_accel += *accel;
    }

    pub fn set_linear_acceleration_wrt_world(&mut self, accel: &Velocity2<f32>) {
        self.top_params_mut().linear_accel = *accel;
    }

    pub fn add_polar_acceleration(&mut self, accel: &Velocity2<f32>) {
        self.top_params_mut().polar_accel += *accel;
    }

    pub fn set_polar_acceleration(&mut self, accel: &Velocity2<f32>) {
        self.top_params_mut().polar_accel = *accel;
    }

    pub fn set_color(&mut self, color: Color) {
        self.top_params_mut().color = color;
    }

    pub fn set_shot_type(&mut self, shot_type: ShotTypeIndex) {
        self.stack.last_mut().expect("empty stack").shot_type = Some(shot_type);
    }

    pub fn set_sprite(&mut self, sprite: &Option<ProjectileSprite>) {
        self.top_params_mut().sprite = *sprite;
    }

    pub fn fire(&mut self) {
        let top = self.stack.last().expect("empty stack");
        self.batches
            .entry(top.shot_type.expect("no shot type set"))
            .or_default()
            .push(top.params);
    }

    pub fn flush(&mut self, lua: &Lua) -> Result<()> {
        let strong = self.space.upgrade().unwrap();
        let space = &mut strong.borrow_mut();
        let st_registry_resource = lua.resource::<ShotTypeRegistry>()?;
        let st_registry = st_registry_resource.borrow();
        for (&shot_type, shots) in self.batches.iter_mut() {
            st_registry.shot_types[shot_type.0].spawn(lua, &self.lua_slots, space, shots)?;
            shots.clear();
        }
        self.lua_slots.clear();
        self.stack.clear();
        self.stack.push(Default::default());
        Ok(())
    }
}

impl LuaUserData for Barrage {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("push", |_, this, ()| {
            this.push();
            Ok(())
        });

        methods.add_method_mut("pop", |_, this, ()| {
            this.pop();
            Ok(())
        });

        methods.add_method_mut("fire", |_, this, ()| {
            this.fire();
            Ok(())
        });

        methods.add_method_mut("set_lua_value", |lua, this, lua_value: LuaValue| {
            this.set_lua_value(lua.create_registry_value(lua_value)?);
            Ok(())
        });

        methods.add_method_mut("prepend_origin", |_, this, tx: Position2<f32>| {
            this.prepend_origin(&tx);
            Ok(())
        });

        methods.add_method_mut("append_origin", |_, this, tx: Position2<f32>| {
            this.append_origin(&tx);
            Ok(())
        });

        methods.add_method_mut(
            "append_rotation_to_origin",
            |_, this, tx: Position2<f32>| {
                this.append_rotation_to_origin(&tx.rotation);
                Ok(())
            },
        );

        methods.add_method_mut("prepend_linear_tx", |_, this, tx: Position2<f32>| {
            this.prepend_linear_tx(&tx);
            Ok(())
        });

        methods.add_method_mut("append_linear_tx", |_, this, tx: Position2<f32>| {
            this.append_linear_tx(&tx);
            Ok(())
        });

        methods.add_method_mut("set_linear_tx", |_, this, tx: Position2<f32>| {
            this.set_linear_tx(&tx);
            Ok(())
        });

        methods.add_method_mut("add_polar_tx", |_, this, tx: Position2<f32>| {
            this.add_polar_tx(&tx);
            Ok(())
        });

        methods.add_method_mut("set_polar_tx", |_, this, tx: Position2<f32>| {
            this.set_polar_tx(&tx);
            Ok(())
        });

        methods.add_method_mut("add_linear_velocity", |_, this, vel| {
            this.add_linear_velocity(&vel);
            Ok(())
        });

        methods.add_method_mut("set_linear_velocity", |_, this, vel| {
            this.set_linear_velocity(&vel);
            Ok(())
        });

        methods.add_method_mut("add_linear_velocity_wrt_world", |_, this, vel| {
            this.add_linear_velocity_wrt_world(&vel);
            Ok(())
        });

        methods.add_method_mut("set_linear_velocity_wrt_world", |_, this, vel| {
            this.set_linear_velocity_wrt_world(&vel);
            Ok(())
        });

        methods.add_method_mut("add_polar_velocity", |_, this, vel| {
            this.add_polar_velocity(&vel);
            Ok(())
        });

        methods.add_method_mut("set_polar_velocity", |_, this, vel| {
            this.set_polar_velocity(&vel);
            Ok(())
        });

        methods.add_method_mut("add_linear_acceleration", |_, this, accel| {
            this.add_linear_acceleration(&accel);
            Ok(())
        });

        methods.add_method_mut("set_linear_acceleration", |_, this, accel| {
            this.set_linear_acceleration(&accel);
            Ok(())
        });

        methods.add_method_mut("add_linear_acceleration_wrt_world", |_, this, accel| {
            this.add_linear_acceleration_wrt_world(&accel);
            Ok(())
        });

        methods.add_method_mut("set_linear_acceleration_wrt_world", |_, this, accel| {
            this.set_linear_acceleration_wrt_world(&accel);
            Ok(())
        });

        methods.add_method_mut("add_polar_acceleration", |_, this, accel| {
            this.add_polar_acceleration(&accel);
            Ok(())
        });

        methods.add_method_mut("set_polar_acceleration", |_, this, accel| {
            this.set_polar_acceleration(&accel);
            Ok(())
        });

        methods.add_method_mut("set_color", |_, this, (r, g, b, a)| {
            this.set_color(Color::new(r, g, b, a));
            Ok(())
        });

        methods.add_method_mut("set_shot_type", |_, this, shot_type| {
            this.set_shot_type(shot_type);
            Ok(())
        });

        methods.add_method_mut("set_sprite", |_, this, sprite| {
            this.set_sprite(&sprite);
            Ok(())
        });

        methods.add_method_mut("flush", |lua, this, ()| {
            this.flush(lua).to_lua_err()?;
            Ok(())
        });
    }
}
