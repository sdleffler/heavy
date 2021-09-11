//! # Heavy Friends - Love2D-like APIs and more for the Heavy framework
//!
//! *Slow down upon the teeth of Orange*
//! *In the midst of this slumber, there is a glow*
//! *That is the heliocentric theory of Green*
//! *Huge falling shadows, swallowing me up*
//! *Heaviness brought together, lifting me up*
//!
//! Heavy Friends (`hv-friends`) builds upon the `hv-core` crate, adding graphics APIs, useful math
//! constructs, basic collision testing, scene stacks, camera math helpers, and more, as well as Lua
//! integration for all of the above. Its API is heavily inspired by the Rust `ggez` crate and Lua
//! Love2D framework.

#![feature(float_interpolation)]

use hv_core::{
    engine::{Engine, EventHandler},
    input::{KeyCode, KeyMods},
    plugins::Plugin,
    prelude::*,
};

pub extern crate nalgebra as na;
pub extern crate parry2d;

#[macro_use]
mod lua;

mod keyboard;
mod position;
mod velocity;

pub mod camera;
pub mod collision;
pub mod graphics;
pub mod math;
pub mod scene;

use na::Orthographic3;
pub use position::*;
pub use velocity::*;

use crate::{
    graphics::{ClearOptions, GraphicsLock, GraphicsLockExt},
    keyboard::EngineKeyboardState,
};

#[doc(hidden)]
pub fn link_me() {}

/// A simple event handler for projects which are entirely or almost entirely Lua-controlled. It
/// loads a Lua file by using the `hv.package.require` function, runs it, and then calls the Lua
/// hooks it finds in the `hv` table (`hv.update`, `hv.draw`, `hv.load`, etc.) which are named much
/// like their Love2D equivalents.
pub struct SimpleHandler {
    entrypoint: String,
}

impl SimpleHandler {
    /// Create a new `SimpleHandler` which loads the given module as its "main" Lua entrypoint.
    pub fn new(s: impl AsRef<str>) -> Self {
        Self {
            entrypoint: s.as_ref().to_owned(),
        }
    }
}

impl EventHandler for SimpleHandler {
    fn init(&mut self, engine: &Engine) -> Result<()> {
        let entrypoint = self.entrypoint.as_str();
        engine
            .lua()
            .load(mlua::chunk! {
                hf = require("hf")
                require($entrypoint)
            })
            .exec()?;

        let gfx_lock = engine.get::<GraphicsLock>();
        let mut gfx = gfx_lock.lock();
        let (w, h) = gfx.mq.screen_size();
        gfx.set_projection(Orthographic3::new(0., w, 0., h, -1., 1.).to_homogeneous());
        gfx.apply_default_pipeline();
        gfx.begin_render_pass(None, Some(ClearOptions::default()));
        drop(gfx);

        engine
            .lua()
            .globals()
            .get::<_, LuaTable>("hv")?
            .call_function("load", ())?;

        Ok(())
    }

    fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        engine
            .lua()
            .globals()
            .get::<_, LuaTable>("hv")?
            .call_function("update", dt)?;
        Ok(())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        let gfx_lock = engine.get::<GraphicsLock>();

        let mut gfx = gfx_lock.lock();
        gfx.begin_render_pass(None, Some(ClearOptions::default()));
        drop(gfx);

        engine
            .lua()
            .globals()
            .get::<_, LuaTable>("hv")?
            .call_function("draw", ())?;

        let mut gfx = gfx_lock.lock();
        gfx.end_render_pass();
        gfx.commit_frame();

        Ok(())
    }

    fn key_down_event(
        &mut self,
        engine: &Engine,
        keycode: KeyCode,
        _keymods: KeyMods,
        repeat: bool,
    ) {
        engine
            .get::<EngineKeyboardState>()
            .borrow_mut()
            .set_key_state(keycode, true, repeat);
    }

    fn key_up_event(&mut self, engine: &Engine, keycode: KeyCode, _keymods: KeyMods) {
        engine
            .get::<EngineKeyboardState>()
            .borrow_mut()
            .set_key_state(keycode, false, false);
    }
}

struct HvFriendsPlugin;

impl Plugin for HvFriendsPlugin {
    fn name(&self) -> &'static str {
        "friends"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>> {
        engine.fs().add_zip_file(
            std::io::Cursor::new(include_bytes!("../resources/scripts.zip")),
            Some(std::path::PathBuf::from("hv-friends/resources/scripts")),
        )?;

        let collision = crate::collision::open(lua, engine)?;
        let graphics = crate::graphics::open(lua, engine)?;
        let keyboard = crate::keyboard::open(lua, engine)?;
        let position = crate::position::open(lua, engine)?;
        let velocity = crate::velocity::open(lua, engine)?;
        let math = crate::math::open(lua, engine)?;

        Ok(lua
            .load(mlua::chunk! {
                {
                    collision = $collision,
                    graphics = $graphics,
                    keyboard = $keyboard,
                    math = $math,
                    position = $position,
                    velocity = $velocity,
                }
            })
            .eval()?)
    }

    fn load<'lua>(&self, lua: &'lua Lua, _engine: &Engine) -> Result<()> {
        let chunk = mlua::chunk! {
            hf = require("hf")
        };
        lua.load(chunk).eval()?;

        Ok(())
    }
}

hv_core::plugin!(HvFriendsPlugin);
