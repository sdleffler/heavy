use std::path::Path;

use hv_core::conf::Conf;
use hv_core::engine::{Engine, SimpleHandler};
use hv_core::filesystem::Filesystem;

use {anyhow::*, hv_core::plugins::Plugin, mlua::prelude::*};

struct FooPlugin;

impl Plugin for FooPlugin {
    fn name(&self) -> &'static str {
        "foo"
    }

    fn open<'lua>(&self, lua: &'lua Lua, _engine: &Engine) -> Result<LuaTable<'lua>> {
        let print = lua.create_function(|_, s: LuaString| {
            println!("{}", s.to_str()?);
            Ok(())
        })?;

        let table = lua.create_table_from(vec![("print", print)])?;
        Ok(table)
    }
}

hv_core::plugin!(FooPlugin);

fn main() {
    let conf = Conf {
        filesystem: Filesystem::from_project_dirs(Path::new("examples/foo"), "foo", "Shea Leffler")
            .unwrap(),
        ..Conf::default()
    };

    Engine::run(conf, SimpleHandler::new("foo"))
}
