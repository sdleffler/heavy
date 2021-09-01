//! The core functionality of the Heavy game framework.

#![feature(ptr_metadata)]

pub extern crate hecs;
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
pub mod shared;
pub mod spaces;
pub mod swappable_cache;
pub mod xsbox;

pub mod error {
    //! This module reexports the `anyhow` crate as a standard solution for error representation,
    //! for crates in the Heavy family.

    pub use anyhow::*;
}

pub mod prelude {
    //! Common types which you'll use almost everywhere when working with Heavy. If you're willing
    //! to use glob imports, a `use hv_core::prelude::*;` won't hurt.

    pub use crate::{
        engine::LuaExt,
        error::*,
        inventory,
        mlua::{self, prelude::*, Variadic as LuaVariadic},
        shared::Shared,
        util::RwLockExt,
    };
}

pub mod api {
    #![doc = include_str!("../doc/std.md")]
}
