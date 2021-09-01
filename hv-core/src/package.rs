//! Lua module loading through the [`Filesystem`](crate::filesystem::Filesystem).

use std::io::Read;

use crate::{
    engine::Engine,
    error::*,
    mlua::prelude::*,
    plugins::{ModuleWrapper, Plugin},
};

/// The Lua registry key holding the package module table.
pub const HV_PACKAGE: &str = "HV_PACKAGE";

/// The default value for `hv.package.path`.
pub const HV_DEFAULT_PATH: &str = "/?.lua:/?/init.lua:/scripts/?.lua:/scripts/?/init.lua";

/// A `LoadedModule` contains the path to the module as well as a function to complete the loading
/// process. Calling the function executes the loaded module file.
#[derive(Debug, Clone)]
pub struct LoadedModule<'lua> {
    pub path: String,
    pub loaded: LuaFunction<'lua>,
}

/// Load a Lua file as a function.
pub fn load<'lua>(engine: &Engine, lua: &'lua Lua, module: &str) -> LuaResult<LoadedModule<'lua>> {
    let package = lua.named_registry_value::<_, LuaTable>(HV_PACKAGE)?;
    let package_path = package.get::<_, LuaString>("path")?;
    let segments = package_path.to_str()?.split(':');
    let module_replaced = module.replace(".", "/");
    let mut tried = Vec::new();

    for segment in segments {
        let path = segment.replace('?', &module_replaced);
        let mut file = match engine.fs().open(&path) {
            Ok(file) => file,
            Err(err) => {
                tried.push(err.to_string());
                continue;
            }
        };
        let mut buf = String::new();
        file.read_to_string(&mut buf).to_lua_err()?;
        let loaded = lua
            .load(&buf)
            .set_name(&module)?
            .into_function()
            .with_context(|| anyhow!("error while loading module {}", module))
            .to_lua_err()?;

        return Ok(LoadedModule { path, loaded });
    }

    // FIXME: better error reporting here; collect errors from individual module attempts
    // and log them?
    Err(anyhow!(
        "module {} not found: {}\n",
        module,
        tried.join("\n"),
    ))
    .to_lua_err()
}

/// Lua-exposed function for loading a module from sludge's `Filesystem`.
///
/// Unlike [`load`], this function will not re-load/execute a module which is already loaded and
/// present in the module cache.
///
/// Similar to Lua's built-in `require`, this will search along paths found in
/// `sludge.package.path`, which is expected to be a colon-separated list of
/// paths to search, where any `?` characters found are replaced by the module
/// path being searched for. The default value of `sludge.package.path` is "/?.lua",
/// which will simply search for any Lua files found in the VFS.
///
/// The limitations of opening files through this `require` are the same as opening
/// any file through the `Filesystem`.
pub fn require<'lua>(engine: &Engine, lua: &'lua Lua, module: String) -> LuaResult<LuaValue<'lua>> {
    let package = lua.named_registry_value::<_, LuaTable>(HV_PACKAGE)?;
    let loaded_modules = package.get::<_, LuaTable>("modules")?;
    if let Some(module) = loaded_modules.get::<_, Option<LuaValue>>(module.as_str())? {
        Ok(module)
    } else {
        let loaded_module = load(engine, lua, &module)?;
        let loaded_value: LuaValue = loaded_module.loaded.call(())?;
        loaded_modules.set(module, loaded_value.clone())?;
        Ok(loaded_value)
    }
}

struct PackageModule;

impl Plugin for PackageModule {
    fn name(&self) -> &'static str {
        "package"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error> {
        let table = lua.create_table()?;
        table.set("path", HV_DEFAULT_PATH)?;

        let engine_ref = engine.downgrade();
        let req_fn = lua.create_function(move |lua, module: String| {
            require(&engine_ref.upgrade(), lua, module)
        })?;

        table.set("require", req_fn.clone())?;
        lua.globals().set("require", req_fn)?;

        let engine_ref = engine.downgrade();
        let load_fn = lua.create_function(move |lua, module: LuaString| {
            load(&engine_ref.upgrade(), lua, module.to_str()?).map(|lm| (lm.loaded, lm.path))
        })?;

        table.set("load", load_fn.clone())?;

        let modules = lua.create_table()?;
        table.set("modules", modules.clone())?;

        lua.set_named_registry_value(HV_PACKAGE, table.clone())?;

        Ok(table)
    }
}

inventory::submit! {
    ModuleWrapper::new(PackageModule)
}
