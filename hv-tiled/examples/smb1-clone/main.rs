use std::path::Path;

use hv_core::{
    conf::Conf,
    engine::{Engine, EventHandler},
    filesystem::Filesystem,
    prelude::*,
    timer::TimeContext,
    // spaces::{Object, Space, Spaces},
};

use hv_friends::{
    graphics::{DrawableMut, GraphicsLock, GraphicsLockExt, Instance},
    math::Vector2,
    SimpleHandler,
};

use std::io::Read;

struct MarioBros {
    layer_batches: Vec<hv_tiled::LayerBatch>,
    x_scroll: usize,
    map_data: hv_tiled::MapData,
    timer: TimeContext,
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

        for layer in tiled_lua_table
            .get::<_, LuaTable>("layers")?
            .sequence_values::<LuaTable>()
        {
            tiled_layers.push(hv_tiled::Layer::from_lua_table(&layer?)?);
        }

        let mut tilesets = Vec::new();

        for tileset in tiled_lua_table
            .get::<_, LuaTable>("tilesets")?
            .sequence_values::<LuaTable>()
        {
            tilesets.push(hv_tiled::Tileset::from_lua(&tileset?)?);
        }

        drop(tiled_lua_table);
        drop(lua);

        let tileset_atlas = hv_tiled::TilesetAtlas::new(tilesets, engine)?;

        let mut layer_batches = Vec::with_capacity(tiled_layers.len());

        for layer in tiled_layers.iter() {
            layer_batches.push(hv_tiled::LayerBatch::new(
                layer,
                &tileset_atlas,
                engine,
                &map_data,
            ));
        }

        let mut simple_handler = SimpleHandler::new("main");
        simple_handler.init(engine)?;

        Ok(MarioBros {
            layer_batches,
            x_scroll: 0,
            map_data,
            timer: TimeContext::new(),
        })
    }
}

impl EventHandler for MarioBros {
    fn update(&mut self, engine: &Engine, _dt: f32) -> Result<()> {
        self.timer.tick();
        let mut counter = 0;
        while self.timer.check_update_time_forced(60, &mut counter) {
            self.x_scroll += 1;
            if self.x_scroll
                > ((self.map_data.width * self.map_data.tilewidth)
                    - (engine.mq().screen_size().0 as usize / 4))
            {
                self.x_scroll = 0;
            }
        }
        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        let graphics_lock = engine.get::<GraphicsLock>();
        for layer_batch in self.layer_batches.iter_mut() {
            layer_batch.draw_mut(
                &mut GraphicsLockExt::lock(&graphics_lock),
                Instance::default()
                    .scale2(Vector2::new(4.0, 4.0))
                    .translate2(Vector2::new((self.x_scroll as f32) * -1.0, 0.0)),
            );
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
        window_width: 1024,
        window_height: 960,
        ..Conf::default()
    };

    Engine::run(conf, MarioBros::new)
}
