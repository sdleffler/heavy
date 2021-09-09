use std::ops;

use crate::{
    graphics::{CachedTexture, Instance},
    math::*,
};

/// An index of a subtexture in an atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubTextureId(usize);

/// A "slice" of a texture [`Atlas`], representing an area of the atlas and its associated render
/// data.
#[derive(Debug, Clone)]
pub struct SubTexture {
    /// The UV coordinates of this slice.
    pub uvs: Box2<f32>,
    /// The offset at which this slice should be drawn. Useful if several related slices have
    /// different sizes due to being compressed/packed in a texture atlas.
    pub offset: Vector2<f32>,
}

impl SubTexture {
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
pub struct Atlas {
    texture: CachedTexture,
    slices: Vec<SubTexture>,
}

impl Atlas {
    /// Create a new texture atlas for a given texture.
    pub fn new(texture: CachedTexture) -> Self {
        Self {
            texture,
            slices: Vec::new(),
        }
    }

    /// Insert a slice into the atlas.
    pub fn insert(&mut self, slice: SubTexture) -> SubTextureId {
        let id = self.slices.len();
        self.slices.push(slice);
        SubTextureId(id)
    }
}

impl ops::Index<SubTextureId> for Atlas {
    type Output = SubTexture;

    fn index(&self, index: SubTextureId) -> &Self::Output {
        &self.slices[index.0]
    }
}
