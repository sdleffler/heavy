use hv_friends::graphics::Color;
use hv_friends::graphics::DrawMode;
use hv_friends::graphics::Mesh;
use hv_friends::{graphics::MeshBuilder, math::Box2};
use hv_tiled::BoxExt;
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
    tile_layer_batches: hv_tiled::TileLayerBatches,
    x_scroll: f32,
    map: hv_tiled::Map,
    timer: TimeContext,
    ts_render_data: TilesetRenderData,
    mb: Mesh,
}

impl MarioBros {
    pub fn new(engine: &Engine) -> Result<Self, Error> {
        // let space = engine.get::<Spaces>().borrow_mut().create_space();
        let map = hv_tiled::lua_parser::parse_map("/mario_bros_1-1.lua", engine, None)?;

        let ts_render_data = hv_tiled::TilesetRenderData::new(&map.tilesets, engine)?;

        let tile_layer_batches =
            hv_tiled::TileLayerBatches::new(&map.tile_layers, &ts_render_data, &map, engine);

        let mut simple_handler = SimpleHandler::new("main");
        simple_handler.init(engine)?;

        let bb = Box2::new(
            0.,
            0.,
            2. * map.meta_data.tilewidth as f32,
            2. * map.meta_data.tileheight as f32,
        );

        let graphics_lock = engine.get::<GraphicsLock>();
        let mut gfx = graphics_lock.lock();

        let mb = MeshBuilder::new(gfx.state.null_texture.clone())
            .rectangle(DrawMode::stroke(2.), bb, Color::WHITE)
            .build(&mut gfx);

        for (tile, x, y) in map.get_tiles_in_bb_in_layer(
            bb.floor_to_i32(),
            *map.tile_layer_map.get("Foreground").unwrap(),
            hv_tiled::CoordSpace::Pixel,
        ) {
            println!("x {}, y {}, tile {:?}", x, y, tile);
        }

        Ok(MarioBros {
            tile_layer_batches,
            x_scroll: 0.0,
            timer: TimeContext::new(),
            map,
            ts_render_data,
            mb,
        })
    }
}

impl EventHandler for MarioBros {
    fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        self.timer.tick();
        let mut counter = 0;

        while self.timer.check_update_time_forced(60, &mut counter) {
            self.tile_layer_batches
                .update_all_batches(dt, &self.ts_render_data);

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

        let x = engine.lua().globals().get::<_, LuaNumber>("x")?;
        let y = engine.lua().globals().get::<_, LuaNumber>("y")?;

        gfx.modelview_mut()
            .origin()
            .translate2((Vector2::new(x, y) * scale).map(|t: f64| (t as f32).floor()));
        gfx.modelview_mut().push(None);
        gfx.modelview_mut().scale2(Vector2::new(4.0, 4.0));

        for tile_layer_batch in self.tile_layer_batches.get_tile_batch_layers() {
            tile_layer_batch.draw_mut(&mut gfx, Instance::new());
        }

        self.mb.draw_mut(&mut gfx, Instance::new());

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
