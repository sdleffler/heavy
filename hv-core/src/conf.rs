//! Configuration options for starting an `Engine`.

use crate::filesystem::Filesystem;

#[derive(Debug)]
pub struct Conf {
    /// The filesystem object to be used by the [`Engine`](crate::engine::Engine). Setting this with
    /// custom settings allows you to "mount" new directories onto it, add ZIP files, set an
    /// "offset" for your resource directory, and more; please refer to the [`Filesystem`] type for
    /// more options.
    pub filesystem: Filesystem,
    /// The window's title.
    pub window_title: String,
    /// The width of the window in pixels.
    pub window_width: u32,
    /// The height of the window in pixels.
    pub window_height: u32,
}

impl Default for Conf {
    fn default() -> Self {
        Self {
            filesystem: Filesystem::new(),
            window_title: "HEAVY \\m/".to_string(),
            window_width: 800,
            window_height: 680,
        }
    }
}
