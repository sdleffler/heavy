use hv_core::{engine::Engine, plugins::Plugin, prelude::*};

pub mod components;

struct TalismanPlugin;

impl Plugin for TalismanPlugin {
    fn name(&self) -> &'static str {
        "talisman"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error> {
        engine.fs().add_zip_file(
            std::io::Cursor::new(include_bytes!("../resources/scripts.zip")),
            Some(std::path::PathBuf::from("hv-talisman/resources/scripts")),
        )?;

        let components = components::open(lua, engine)?;

        lua.load(mlua::chunk! {
            {
                components = $components,
            }
        })
        .eval()
        .map_err(Into::into)
    }

    fn load<'lua>(&self, lua: &'lua Lua, _engine: &Engine) -> Result<()> {
        let chunk = mlua::chunk! {
            talisman = require("talisman")
        };
        lua.load(chunk).exec()?;

        Ok(())
    }
}

hv_core::plugin!(TalismanPlugin);
