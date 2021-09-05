use std::path::Path;

use hv_core::{
    engine::{Engine, EventHandler},
    filesystem::Filesystem,
    conf::Conf,
    prelude::*,
    // spaces::{Object, Space, Spaces},
};

use hv_friends::{SimpleHandler, graphics:: {Drawable, DrawableMut, GraphicsLock, GraphicsLockExt, Instance}};

use hv_tiled;
use std::io::Read;

struct MarioBros {
    tileset_atlas: hv_tiled::TilesetAtlas,
    layer_batches: Vec<hv_tiled::LayerBatch>,
}

impl MarioBros {
    pub fn new(engine: &Engine) -> Result<Self, Error> {
        // let space = engine.get::<Spaces>().borrow_mut().create_space();
        let mut fs = engine.fs();
        let lua = engine.lua();
        let mut tiled_lua_map = fs.open(Path::new("/mario_bros_1-1.lua"))?;
        drop(fs);
        let mut tiled_buffer: Vec<u8> = Vec::new();
        tiled_lua_map.read_to_end(&mut tiled_buffer)?;
        let lua_chunk = lua.load(&tiled_buffer);
        let tiled_lua_table = lua_chunk.eval::<LuaTable>()?;
        let map_data = hv_tiled::MapData::from_lua_table(&tiled_lua_table)?;

        let mut tiled_layers = Vec::new();

        for layer in tiled_lua_table.get::<_, LuaTable>("layers")?.sequence_values::<LuaTable>() {
            tiled_layers.push(hv_tiled::Layer::from_lua_table(layer?)?);
        }

        let mut tilesets = Vec::new();

        for tileset in tiled_lua_table.get::<_, LuaTable>("tilesets")?.sequence_values::<LuaTable>() {
            tilesets.push(hv_tiled::get_tileset(tileset?, engine)?);
        }

        drop(tiled_lua_table);
        drop(lua);

        let tileset_atlas = hv_tiled::TilesetAtlas::new(tilesets, engine)?;

        let mut layer_batches = Vec::with_capacity(tiled_layers.len());

        for layer in tiled_layers.iter() {
            layer_batches.push(hv_tiled::LayerBatch::new(layer, &tileset_atlas, engine, &map_data));
        }

        let mut simple_handler = SimpleHandler::new("main");
        simple_handler.init(engine)?;

        Ok(MarioBros {
            tileset_atlas,
            layer_batches,
        })
    }
}

impl EventHandler for MarioBros {
    fn update(&mut self, _engine: &Engine, _dt: f32) -> Result<()> {
        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        let graphics_lock = engine.get::<GraphicsLock>();
        for layer_batch in self.layer_batches.iter_mut() {
            layer_batch.draw_mut(&mut GraphicsLockExt::lock(&graphics_lock), Instance::default());
        }
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