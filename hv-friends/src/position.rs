use anyhow::*;
use hv_core::{
    components::DynamicComponentConstructor,
    engine::Engine,
    mlua::prelude::*,
    spaces::{Object, SpaceCache},
    util::RwLockExt,
};
use nalgebra::Isometry2;
use serde::*;

use crate::math::HvIsometry2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub isometry: Isometry2<f32>,
}

impl LuaUserData for Position {}

pub(crate) fn open<'lua>(lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error> {
    let create_position_constructor = lua.create_function(|_, HvIsometry2(isometry)| {
        Ok(DynamicComponentConstructor::copy(Position { isometry }))
    })?;

    let mut space_cache = SpaceCache::new(engine);
    let get_isometry2 =
        lua.create_function_mut(move |_, (obj, out): (Object, LuaAnyUserData)| {
            let space = space_cache.get_space(obj.space());
            let isometry = space.borrow().get::<Position>(obj).to_lua_err()?.isometry;
            out.borrow_mut::<HvIsometry2<f32>>()?.0 = isometry;
            Ok(())
        })?;

    let mut space_cache = SpaceCache::new(engine);
    let set_isometry2 = lua.create_function_mut(
        move |_, (obj, HvIsometry2(iso)): (Object, HvIsometry2<f32>)| {
            let space = space_cache.get_space(obj.space());
            space
                .borrow()
                .get_mut::<Position>(obj)
                .to_lua_err()?
                .isometry = iso;
            Ok(())
        },
    )?;

    Ok(lua
        .load(mlua::chunk! {
            {
                create_position_constructor = $create_position_constructor,
                get_isometry2 = $get_isometry2,
                set_isometry2 = $set_isometry2,
            }
        })
        .eval()?)
}
