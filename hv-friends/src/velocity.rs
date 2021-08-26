use anyhow::*;
use hv_core::{
    components::DynamicComponentConstructor,
    engine::Engine,
    mlua::prelude::*,
    spaces::{Object, SpaceCache},
    util::RwLockExt,
};
use serde::*;

use crate::math::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Velocity {
    pub velocity: Velocity2<f32>,
}

impl LuaUserData for Velocity {}

pub(crate) fn open<'lua>(lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error> {
    let create_velocity_constructor = lua.create_function(|_, HvVelocity2(velocity)| {
        Ok(DynamicComponentConstructor::copy(Velocity { velocity }))
    })?;

    let mut space_cache = SpaceCache::new(engine);
    let get_velocity2 =
        lua.create_function_mut(move |_, (obj, out): (Object, LuaAnyUserData)| {
            let space = space_cache.get_space(obj.space());
            let velocity = space.borrow().get::<Velocity>(obj).to_lua_err()?.velocity;
            out.borrow_mut::<HvVelocity2<f32>>()?.0 = velocity;
            Ok(())
        })?;

    let mut space_cache = SpaceCache::new(engine);
    let set_velocity2 = lua.create_function_mut(
        move |_, (obj, HvVelocity2(vel)): (Object, HvVelocity2<f32>)| {
            let space = space_cache.get_space(obj.space());
            space
                .borrow()
                .get_mut::<Velocity>(obj)
                .to_lua_err()?
                .velocity = vel;
            Ok(())
        },
    )?;

    Ok(lua
        .load(mlua::chunk! {
            {
                create_velocity_constructor = $create_velocity_constructor,
                get_velocity2 = $get_velocity2,
                set_velocity2 = $set_velocity2,
                nil
            }
        })
        .eval()?)
}
