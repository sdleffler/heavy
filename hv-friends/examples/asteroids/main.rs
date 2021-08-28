use std::path::Path;

use hv_core::{conf::Conf, engine::Engine, filesystem::Filesystem};
use hv_friends::SimpleHandler;

fn main() {
    hv_friends::link_me();

    let conf = Conf {
        filesystem: Filesystem::from_project_dirs(
            Path::new("examples/asteroids"),
            "asteroids",
            "Shea Leffler",
        )
        .unwrap(),
        ..Conf::default()
    };

    Engine::run(conf, SimpleHandler::new("main"))
}
