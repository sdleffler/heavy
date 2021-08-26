use std::sync::{Arc, RwLock};

use hv_core::{
    engine::{EngineRef, Resource},
    prelude::*,
    spaces::Space,
};
use hv_friends::{
    camera::Camera,
    graphics::GraphicsLock,
    scene::{Scene, SceneStack},
};

pub struct GameplayContext<'a> {
    lua: &'a Lua,
    dt: f32,
    space: Resource<Space>,
    gfx_lock: Resource<GraphicsLock>,
    camera: Arc<RwLock<Camera>>,
}
