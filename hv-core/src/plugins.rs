use crate::engine::Engine;

use {anyhow::*, mlua::prelude::*};

#[doc(hidden)]
pub use inventory::*;

pub trait Plugin: 'static {
    fn name(&self) -> &'static str;
    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error>;
}

pub(crate) struct ModuleWrapper {
    object: Box<dyn Plugin>,
}

impl ModuleWrapper {
    pub fn new<T: Plugin>(plugin: T) -> Self {
        Self {
            object: Box::new(plugin),
        }
    }
}

inventory::collect!(ModuleWrapper);

#[doc(hidden)]
pub struct PluginWrapper {
    object: Box<dyn Plugin>,
}

impl PluginWrapper {
    pub fn new<T: Plugin>(plugin: T) -> Self {
        Self {
            object: Box::new(plugin),
        }
    }
}

#[macro_export]
macro_rules! plugin {
    ($e:expr) => {
        const _: () = {
            use $crate::inventory;
            $crate::inventory::submit!($crate::plugins::PluginWrapper::new($e));
        };
    };
}

inventory::collect!(PluginWrapper);

pub(crate) fn registered_plugins() -> impl Iterator<Item = &'static dyn Plugin> {
    inventory::iter::<PluginWrapper>
        .into_iter()
        .map(|wrapper| &*wrapper.object)
}

pub(crate) fn registered_modules() -> impl Iterator<Item = &'static dyn Plugin> {
    inventory::iter::<ModuleWrapper>
        .into_iter()
        .map(|wrapper| &*wrapper.object)
}

struct PluginModule;

impl Plugin for PluginModule {
    fn name(&self) -> &'static str {
        "plugins"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error> {
        let table = lua.create_table()?;

        for plugin in registered_plugins() {
            log::trace!("opening registered plugin: `{}`", plugin.name());
            let opened = plugin.open(lua, engine)?;
            table.set(plugin.name(), opened)?;
        }

        Ok(table)
    }
}

inventory::submit!(ModuleWrapper::new(PluginModule));
