//! The core functionality of the Heavy game framework.

#![feature(coerce_unsized, unsize)]
#![feature(ptr_metadata)]
#![warn(missing_docs)]

pub extern crate hecs;
pub extern crate inventory;
pub extern crate miniquad as mq;
pub extern crate mlua;
pub extern crate nalgebra as na;

mod logger;
mod package;
mod path_clean;
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
pub mod timer;
pub mod xsbox;

pub mod error {
    //! Reexport of the [`mod@anyhow`] crate.
    //!
    //! This is your standard solution for error handling/representation in the Heavy crate family,
    //! and helps ensure compatibility between error types and makes throwing and handling errors
    //! much smoother than having unique error types everywhere.

    pub use anyhow::*;
}

pub mod prelude {
    //! Common types which you'll use almost everywhere when working with Heavy.
    //!
    //! If you're willing to use glob imports, a `use hv_core::prelude::*;` won't hurt.

    pub use crate::{
        engine::LuaExt,
        error::*,
        inventory,
        mlua::{self, prelude::*, Variadic as LuaVariadic},
        shared::Shared,
    };
}

pub mod api {
    #![doc = include_str!("../doc/std.md")]
}
