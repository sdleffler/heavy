use std::ops;

use crate::{
    graphics::{CachedTexture, Instance},
    math::*,
};

/// An index of a subtexture in an atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubTextureId(usize);

/// A "slice" of a texture [`Atlas`], representing an area of the atlas and its associated render
/// data. Optionally it can also contain some userdata.
#[derive(Debug, Clone)]
pub struct SubTexture<T> {
    /// The UV coordinates of this slice.
    pub uvs: Box2<f32>,
    /// The offset at which this slice should be drawn. Useful if several related slices have
    /// different sizes due to being compressed/packed in a texture atlas.
    pub offset: Vector2<f32>,
    /// Any associated userdata.
    pub userdata: T,
}

impl<T> SubTexture<T> {
    /// Convert this slice into a properly scaled instance with the correct UVs.
    pub fn to_instance(&self) -> Instance {
        Instance::new()
            .src(self.uvs)
            .translate2(self.offset)
            .scale2(self.uvs.extents())
    }
}

/// A texture atlas containing a texture and any number of "slices" of that textures, representing
/// images stored in it.
#[derive(Debug, Clone)]
pub struct Atlas<T> {
    texture: CachedTexture,
    slices: Vec<SubTexture<T>>,
}

impl<T> Atlas<T> {
    /// Create a new texture atlas for a given texture.
    pub fn new(texture: CachedTexture) -> Self {
        Self {
            texture,
            slices: Vec::new(),
        }
    }

    /// Insert a slice into the atlas.
    pub fn insert(&mut self, slice: SubTexture<T>) -> SubTextureId {
        let id = self.slices.len();
        self.slices.push(slice);
        SubTextureId(id)
    }
}

impl<T> ops::Index<SubTextureId> for Atlas<T> {
    type Output = SubTexture<T>;

    fn index(&self, index: SubTextureId) -> &Self::Output {
        &self.slices[index.0]
    }
}
