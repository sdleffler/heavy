use hv_core::{
    components::DynamicComponentConstructor,
    engine::Engine,
    prelude::*,
    spaces::{serialize, Object, SpaceCache},
};
use serde::*;

use crate::math::Position2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position(pub Position2<f32>);

hv_core::serializable!(serialize::with_serde::<Position>("friends.Position"));

impl LuaUserData for Position {}

pub(crate) fn open<'lua>(lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error> {
    let create_position_constructor = lua
        .create_function(|_, position| Ok(DynamicComponentConstructor::copy(Position(position))))?;

    let mut space_cache = SpaceCache::new(engine);
    let has_position = lua.create_function_mut(move |_, object: Object| {
        Ok(space_cache
            .get_space(object.space())
            .borrow()
            .query_one::<&Position>(object)
            .to_lua_err()?
            .get()
            .is_some())
    })?;

    let mut space_cache = SpaceCache::new(engine);
    let get_position2 =
        lua.create_function_mut(move |_, (obj, out): (Object, LuaAnyUserData)| {
            let space = space_cache.get_space(obj.space());
            let position = space.borrow().get::<Position>(obj).to_lua_err()?.0;
            *out.borrow_mut::<Position2<f32>>()? = position;
            Ok(())
        })?;

    let mut space_cache = SpaceCache::new(engine);
    let set_position2 =
        lua.create_function_mut(move |_, (obj, pos): (Object, Position2<f32>)| {
            let space = space_cache.get_space(obj.space());
            space.borrow().get_mut::<Position>(obj).to_lua_err()?.0 = pos;
            Ok(())
        })?;

    Ok(lua
        .load(mlua::chunk! {
            {
                create_position_constructor = $create_position_constructor,
                has_position = $has_position,
                get_position2 = $get_position2,
                set_position2 = $set_position2,
            }
        })
        .eval()?)
}
