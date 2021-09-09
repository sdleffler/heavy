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

struct MarioBros {
    tile_layer_batches: Vec<hv_tiled::TileLayerBatch>,
    x_scroll: u32,
    map: hv_tiled::Map,
    timer: TimeContext,
}

impl MarioBros {
    pub fn new(engine: &Engine) -> Result<Self, Error> {
        // let space = engine.get::<Spaces>().borrow_mut().create_space();
        let map = hv_tiled::Map::new("/mario_bros_1-1.lua", engine, None)?;

        let tileset_atlas = hv_tiled::TilesetAtlas::new(&map.tilesets, engine)?;

        let mut tile_layer_batches = Vec::with_capacity(map.tile_layers.len());

        for tile_layer in map.tile_layers.iter() {
            tile_layer_batches.push(hv_tiled::TileLayerBatch::new(
                tile_layer,
                &tileset_atlas,
                engine,
                &map.meta_data,
            ));
        }

        let mut simple_handler = SimpleHandler::new("main");
        simple_handler.init(engine)?;

        Ok(MarioBros {
            tile_layer_batches,
            x_scroll: 0,
            timer: TimeContext::new(),
            map,
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
                > ((self.map.meta_data.width * self.map.meta_data.tilewidth)
                    - (engine.mq().screen_size().0 as u32 / 4))
            {
                self.x_scroll = 0;
            }
        }
        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        let graphics_lock = engine.get::<GraphicsLock>();
        for layer_batch in self.tile_layer_batches.iter_mut() {
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
