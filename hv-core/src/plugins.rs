//! Heavy's Lua plugin registry, used for registering your Rust-to-Lua bindings. Plugins generate
//! Lua tables which are automatically loaded into an `Engine`'s Lua context, as entries under the
//! global table `hv.plugins`. You usually will not access plugins directly, but instead through
//! using the bundled Lua APIs that come with whatever library registered them.

use crate::{engine::Engine, error::*, mlua::prelude::*};

#[doc(hidden)]
pub use crate::inventory::*;

/// A plugin represents a Lua interface to be bound at the initialization time of the `Engine`.
/// Specifically, when the `Engine` is first loaded, all plugins will have their `open` methods
/// called, and the resulting tables will be stored in the global table `hv.plugins`, as values
/// paired with their names as keys.
pub trait Plugin: 'static {
    /// The name of this plugin, used as a string key for the `hv.plugins` table.
    fn name(&self) -> &'static str;

    /// "Open" the plugin and retrieve a table to be stored in `hv.plugins[name]`. Excellent for
    /// binding Rust functions to Lua functions to be exposed to Lua code bundled with a plugin.
    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>>;

    /// Any initialization which needs to happen after all plugins are opened should go here. This
    /// can include things like quickly running `require("my_library.whatever")` if there is Lua
    /// initialization that this plugin needs done before the user's code begins to run.
    ///
    /// An example of initialization which might need to be done is any classes which need to be
    /// registered with `binser`. If a class is not registered with binser before we attempt to
    /// serialize or deserialize it, then binser will throw an error. As these classes are
    /// registered usually right next to the definition of the class itself, it is a good idea to
    /// use `require` to ensure that any Lua code bundled with your plugin has properly initialized
    /// everything it needs to.
    fn load<'lua>(&self, _lua: &'lua Lua, _engine: &Engine) -> Result<()> {
        Ok(())
    }
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

    fn load<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<()> {
        for plugin in registered_plugins() {
            log::trace!("loading registered plugin: `{}`", plugin.name());
            plugin.load(lua, engine)?;
        }

        Ok(())
    }
}

inventory::submit!(ModuleWrapper::new(PluginModule));
