use std::path::Path;

use hv_core::{conf::Conf, engine::Engine, filesystem::Filesystem};
use hv_friends::SimpleHandler;

fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();

    let conf = Conf {
        filesystem: Filesystem::from_project_dirs(
            Path::new("examples/asteroids-lua"),
            "asteroids-lua",
            "Shea Leffler",
        )
        .unwrap(),
        ..Conf::default()
    };

    Engine::run(conf, |_| Ok(SimpleHandler::new("main")))
}
