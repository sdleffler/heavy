use crate::filesystem::Filesystem;

#[derive(Debug)]
pub struct Conf {
    pub filesystem: Filesystem,
    pub window_title: String,
    pub window_width: u32,
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
