#![feature(option_get_or_insert_default)]
#![feature(entry_insert)]
#![feature(toowned_clone_into)]

use hv_core::{engine::Engine, plugins::Plugin, prelude::*};

pub mod components;
pub mod level;

pub use crate::level::Level;

struct TalismanPlugin;

impl Plugin for TalismanPlugin {
    fn name(&self) -> &'static str {
        "talisman"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error> {
        engine
            .fs()
            .add_zip_file(std::io::Cursor::new(include_bytes!(
                "../resources/scripts.zip"
            )))?;

        let components = components::open(lua, engine)?;
        let level = level::open(lua, engine)?;

        lua.load(mlua::chunk! {
            {
                components = $components,
                level = $level,
            }
        })
        .eval()
        .map_err(Into::into)
    }
}

hv_core::plugin!(TalismanPlugin);
