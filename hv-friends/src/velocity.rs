use hv_core::{
    components::DynamicComponentConstructor,
    engine::Engine,
    prelude::*,
    spaces::{serialize::Serializable, Object, SpaceCache},
};
use serde::*;

use crate::math::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Velocity(pub Velocity2<f32>);

hv_core::serializable!(Serializable::serde::<Velocity>("friends.Velocity"));

impl LuaUserData for Velocity {}

pub(crate) fn open<'lua>(lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error> {
    let create_velocity_constructor = lua
        .create_function(|_, velocity| Ok(DynamicComponentConstructor::copy(Velocity(velocity))))?;

    let mut space_cache = SpaceCache::new(engine);
    let has_velocity = lua.create_function_mut(move |_, object: Object| {
        Ok(space_cache
            .get_space(object.space())
            .borrow()
            .query_one::<&Velocity>(object)
            .to_lua_err()?
            .get()
            .is_some())
    })?;

    let mut space_cache = SpaceCache::new(engine);
    let get_velocity2 =
        lua.create_function_mut(move |_, (obj, out): (Object, LuaAnyUserData)| {
            let space = space_cache.get_space(obj.space());
            let velocity = space.borrow().get::<Velocity>(obj).to_lua_err()?.0;
            *out.borrow_mut::<Velocity2<f32>>()? = velocity;
            Ok(())
        })?;

    let mut space_cache = SpaceCache::new(engine);
    let set_velocity2 =
        lua.create_function_mut(move |_, (obj, vel): (Object, Velocity2<f32>)| {
            let space = space_cache.get_space(obj.space());
            space.borrow().get_mut::<Velocity>(obj).to_lua_err()?.0 = vel;
            Ok(())
        })?;

    Ok(lua
        .load(mlua::chunk! {
            {
                create_velocity_constructor = $create_velocity_constructor,
                has_velocity = $has_velocity,
                get_velocity2 = $get_velocity2,
                set_velocity2 = $set_velocity2,
            }
        })
        .eval()?)
}
