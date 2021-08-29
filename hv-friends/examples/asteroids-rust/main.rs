use std::{
    path::Path,
    sync::{Arc, RwLock},
};

use hv_core::{
    conf::Conf,
    engine::{Engine, EngineRef, EventHandler, LazyHandler},
    filesystem::Filesystem,
    prelude::*,
    spaces::Space,
};
use hv_friends::{Position, SimpleHandler, Velocity};

pub struct Asteroids {
    space: Arc<RwLock<Space>>,
}

impl Asteroids {
    pub fn new(engine: &Engine) -> Result<Self> {
        todo!()
    }
}

impl EventHandler for Asteroids {
    fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        let mut space = self.space.borrow_mut();

        for (_, (Position(pos), Velocity(vel))) in space.query_mut::<(&mut Position, &Velocity)>() {
            pos.integrate_mut(vel, dt);
        }

        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        Ok(())
    }
}

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();

    let conf = Conf {
        filesystem: Filesystem::from_project_dirs(
            Path::new("examples/asteroids-rust"),
            "asteroids-rust",
            "Shea Leffler",
        )
        .unwrap(),
        ..Conf::default()
    };

    Engine::run(conf, LazyHandler::new(Asteroids::new))
}
