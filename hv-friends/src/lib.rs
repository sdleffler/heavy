#![feature(float_interpolation)]

use anyhow::*;
use hv_core::{
    engine::Engine,
    mlua::{self, prelude::*},
    plugins::Plugin,
};

pub extern crate nalgebra as na;
pub extern crate ncollide2d as nc;

#[macro_use]
mod lua;

mod position;
mod velocity;

pub mod camera;
pub mod graphics;
pub mod math;
pub mod scene;

pub use position::*;
pub use velocity::*;

#[doc(hidden)]
pub fn link_me() {}

struct HvFriendsPlugin;

impl Plugin for HvFriendsPlugin {
    fn name(&self) -> &'static str {
        "friends"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>> {
        engine
            .fs()
            .add_zip_file(std::io::Cursor::new(include_bytes!(
                "../resources/scripts.zip"
            )))?;

        let graphics = crate::graphics::open(lua, engine)?;
        let position = crate::position::open(lua, engine)?;
        let velocity = crate::velocity::open(lua, engine)?;
        let math = crate::math::open(lua, engine)?;

        Ok(lua
            .load(mlua::chunk! {
                {
                    graphics = $graphics,
                    math = $math,
                    position = $position,
                    velocity = $velocity,
                }
            })
            .eval()?)
    }
}

hv_core::plugin!(HvFriendsPlugin);
