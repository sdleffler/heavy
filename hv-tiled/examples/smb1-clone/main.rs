use std::path::Path;

use hv_core::{
    engine::{Engine, EventHandler},
    filesystem::Filesystem,
    conf::Conf,
    prelude::*,
    // spaces::{Object, Space, Spaces},
};

use hv_tiled;
use std::io::Read;

struct MarioBros;

impl MarioBros {
    pub fn new(engine: &Engine) -> Result<Self> {
        // let space = engine.get::<Spaces>().borrow_mut().create_space();
        let mut fs = engine.fs();
        // let tileset_path = fs.open(Path::new("/NES - Super Mario Bros - Tileset.tsx"))?;
        // let tileset = hv_tiled::tiled::parse_tileset(tileset_path, 1)?;
        let lua = engine.lua();
        let mut tiled_lua_map = fs.open(Path::new("/mario_bros_1-1.lua"))?;
        let mut tiled_buffer: Vec<u8> = Vec::new();
        tiled_lua_map.read_to_end(&mut tiled_buffer)?;
        let lua_chunk = lua.load(&tiled_buffer);
        let tiled_lua_table = lua_chunk.eval::<LuaTable>()?;
        let mut tiled_layers = Vec::new();

        for layer in tiled_lua_table.get::<_, LuaTable>("layers")?.sequence_values::<LuaTable>() {
            tiled_layers.push(hv_tiled::Layer::from_lua_table(layer?)?);
        }

        Ok(MarioBros)
    }
}

impl EventHandler for MarioBros {
    fn update(&mut self, _engine: &Engine, _dt: f32) -> Result<()> {
        Ok(())
    }

    fn draw(&mut self, _engine: &Engine) -> Result<()> {
        Ok(())
    }
}

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();

    let conf = Conf {
        filesystem: Filesystem::from_project_dirs(
            Path::new("examples/smb1-clone"),
            "smb1-clone",
            "Maxim Veligan",
        )
        .unwrap(),
        ..Conf::default()
    };

    Engine::run(conf, MarioBros::new)
}