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
use hv_tiled::TilesetRenderData;

struct MarioBros {
    tile_layer_batches: Vec<hv_tiled::TileLayerBatch>,
    x_scroll: f32,
    map: hv_tiled::Map,
    timer: TimeContext,
    ts_render_data: TilesetRenderData,
}

impl MarioBros {
    pub fn new(engine: &Engine) -> Result<Self, Error> {
        // let space = engine.get::<Spaces>().borrow_mut().create_space();
        let map = hv_tiled::Map::new("/mario_bros_1-1.lua", engine, None)?;

        let ts_render_data = hv_tiled::TilesetRenderData::new(&map.tilesets, engine)?;

        let mut tile_layer_batches = Vec::with_capacity(map.tile_layers.len());

        for tile_layer in map.tile_layers.iter() {
            tile_layer_batches.push(hv_tiled::TileLayerBatch::new(
                tile_layer,
                &ts_render_data,
                engine,
                &map.meta_data,
            ));
        }

        let mut simple_handler = SimpleHandler::new("main");
        simple_handler.init(engine)?;

        Ok(MarioBros {
            tile_layer_batches,
            x_scroll: 0.0,
            timer: TimeContext::new(),
            map,
            ts_render_data,
        })
    }
}

impl EventHandler for MarioBros {
    fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        self.timer.tick();
        let mut counter = 0;

        while self.timer.check_update_time_forced(60, &mut counter) {
            for tile_layer_batch in self.tile_layer_batches.iter_mut() {
                tile_layer_batch.update_batches(dt, &self.ts_render_data);
            }

            self.x_scroll += 1.0;
            if self.x_scroll
                > ((self.map.meta_data.width as f32 * self.map.meta_data.tilewidth as f32)
                    - (engine.mq().screen_size().0 / 4.0))
            {
                self.x_scroll = 0.0;
            }
        }
        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        let graphics_lock = engine.get::<GraphicsLock>();
        let mut gfx = graphics_lock.lock();
        let scale = 4.0;

        gfx.modelview_mut()
            .origin()
            .translate2((Vector2::new(self.x_scroll * -1.0, 0.0) * scale).map(|t| t.floor()));
        gfx.modelview_mut().push(None);
        gfx.modelview_mut().scale2(Vector2::new(4.0, 4.0));

        for tile_layer_batch in self.tile_layer_batches.iter_mut() {
            tile_layer_batch.draw_mut(&mut gfx, Instance::new());
        }

        gfx.modelview_mut().pop();

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
