use std::path::Path;

use hv_core::{
    conf::Conf,
    engine::{Engine, EventHandler},
    filesystem::Filesystem,
    prelude::*,
    spaces::{Space, Spaces},
    timer::TimeContext,
};

use hv_friends::{
    graphics::{
        Color, DrawMode, DrawableMut, GraphicsLock, GraphicsLockExt, Instance, MeshBuilder,
    },
    math::*,
    Position, SimpleHandler,
};

use std::io::Read;

struct SmbOneOne {
    space: Shared<Space>,
    layer_batches: Vec<hv_tiled::LayerBatch>,
    x_scroll: usize,
    map_data: hv_tiled::MapData,
    timer: TimeContext,
}

impl SmbOneOne {
    pub fn new(engine: &Engine) -> Result<Self, Error> {
        let space = engine.get::<Spaces>().borrow_mut().create_space();
        let mut fs = engine.fs();
        let lua = engine.lua();
        lua.globals().set("space", space.clone())?;
        let mut tiled_lua_map = fs.open(Path::new("/maps/mario_bros_1-1.lua"))?;
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
            tiled_layers.push(hv_tiled::Layer::from_lua_table(layer?)?);
        }

        let mut tilesets = Vec::new();

        for tileset in tiled_lua_table
            .get::<_, LuaTable>("tilesets")?
            .sequence_values::<LuaTable>()
        {
            tilesets.push(hv_tiled::get_tileset(tileset?, engine)?);
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

        Ok(SmbOneOne {
            space,
            layer_batches,
            x_scroll: 0,
            map_data,
            timer: TimeContext::new(),
        })
    }
}

impl EventHandler for SmbOneOne {
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
        let mut gfx = graphics_lock.lock();

        gfx.modelview_mut()
            .origin()
            .scale2(Vector2::new(4.0, 4.0))
            .translate2(Vector2::new((self.x_scroll as f32) * -1.0, 0.0));

        for layer_batch in self.layer_batches.iter_mut() {
            layer_batch.draw_mut(&mut gfx, Instance::new());
        }

        let mut space = self.space.borrow_mut();
        let mut mesh = MeshBuilder::new(gfx.state.null_texture.clone())
            .rectangle(
                DrawMode::fill(),
                Box2::from_half_extents(Point2::origin(), Vector2::new(8., 8.)),
                Color::RED,
            )
            .build(&mut gfx);

        for (_, Position(pos)) in space.query_mut::<&Position>() {
            mesh.draw_mut(&mut gfx, Instance::new().translate2(pos.center().coords));
        }

        Ok(())
    }
}

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();

    let conf = Conf {
        filesystem: Filesystem::from_project_dirs(Path::new(""), "smb1-1", "Heavy Orbit").unwrap(),
        window_width: 1024,
        window_height: 960,
        ..Conf::default()
    };

    Engine::run(conf, SmbOneOne::new)
}
