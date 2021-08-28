#![feature(ptr_metadata)]

#[doc(hidden)]
pub extern crate hecs;
#[doc(hidden)]
pub extern crate inventory;
pub extern crate miniquad as mq;
pub extern crate mlua;
pub extern crate nalgebra as na;

mod logger;
mod package;
mod path_clean;
pub mod util;
mod vfs;

pub mod components;
pub mod conf;
pub mod engine;
pub mod filesystem;
pub mod input;
pub mod plugins;
pub mod spaces;
pub mod swappable_cache;
pub mod xsbox;

pub mod prelude {
    pub use crate::{engine::LuaExt, util::RwLockExt};

    pub use anyhow::*;
    pub use inventory;
    pub use mlua::{self, prelude::*, Variadic as LuaVariadic};
}
