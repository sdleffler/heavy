//! Lua interface to the Rust [`log`] crate.

use crate::{
    engine::Engine,
    error::*,
    mlua::prelude::*,
    plugins::{ModuleWrapper, Plugin},
};

struct LoggerModule;

impl Plugin for LoggerModule {
    fn name(&self) -> &'static str {
        "logger"
    }

    fn open<'lua>(&self, lua: &'lua mlua::Lua, _engine: &Engine) -> Result<LuaTable<'lua>> {
        let log_info = lua.create_function(|_, (target, formatted): (LuaString, LuaString)| {
            let target = target.to_string_lossy();
            let formatted = formatted.to_string_lossy();
            log::info!(target: &*target, "{}", formatted);
            Ok(())
        })?;

        Ok(lua
            .load(mlua::chunk! {
                {
                    info = $log_info
                }
            })
            .eval()?)
    }
}

inventory::submit!(ModuleWrapper::new(LoggerModule));
