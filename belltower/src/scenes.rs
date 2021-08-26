use std::sync::{Arc, RwLock};

use hv_core::{
    engine::{Engine, EngineRef, Resource},
    prelude::*,
    spaces::Space,
};
use hv_friends::{
    camera::{Camera, CameraParameters},
    graphics::{Canvas, GraphicsLock},
    math::*,
    scene::{Scene, SceneStack},
};

// pub mod gameplay;
pub mod editor;
pub mod level;

use crate::INTERNAL_RESOLUTION;

pub enum SceneEvent {}

pub struct SceneContext {
    engine: EngineRef,
    dt: f32,
}

pub struct GameplayScene {
    space: Resource<Space>,
    gfx_lock: Resource<GraphicsLock>,
    camera: Arc<RwLock<Camera>>,
}

impl GameplayScene {
    pub fn new(engine: &Engine) -> Self {
        Self {
            space: engine.get(),
            gfx_lock: engine.get(),
            camera: Arc::new(RwLock::new(Camera::new(CameraParameters::new(
                Vector2::new(INTERNAL_RESOLUTION.0, INTERNAL_RESOLUTION.1),
            )))),
        }
    }
}

impl Scene<SceneContext, SceneEvent> for GameplayScene {
    fn update(
        &mut self,
        scene_stack: &mut SceneStack<SceneContext, SceneEvent>,
        ctx: &mut SceneContext,
    ) -> Result<()> {
        todo!()
    }

    fn draw(&mut self, ctx: &mut SceneContext) -> Result<()> {
        todo!()
    }

    fn event(&mut self, ctx: &mut SceneContext, event: SceneEvent) -> Result<()> {
        match event {}
    }
}
