use hv_core::{
    engine::{Engine, EngineRef, LuaResource},
    mq,
    prelude::*,
    swappable_cache::{CacheRef, Guard, Handle, Loader, SwappableCache},
};
use std::{io::Read, ops::Deref, path::Path, sync::Arc};

use crate::{
    graphics::{
        Drawable, DrawableMut, FilterMode, Graphics, GraphicsLock, GraphicsLockExt, Instance,
    },
    math::*,
};

#[derive(Debug)]
pub struct OwnedTexture {
    pub handle: mq::Texture,
}

impl OwnedTexture {
    /// Create a texture from a given buffer of RGBA image data.
    pub fn from_rgba8(ctx: &mut Graphics, width: u16, height: u16, bytes: &[u8]) -> Self {
        let tex = mq::Texture::from_rgba8(ctx.mq_mut(), width, height, bytes);
        tex.set_filter(ctx.mq_mut(), mq::FilterMode::Nearest);
        Self::from_inner(tex)
    }

    /// Parse a buffer containing the raw contents of an image file such as a PNG, GIF, etc.
    pub fn from_memory(ctx: &mut Graphics, buffer: &[u8]) -> Result<Self> {
        let rgba_image = image::load_from_memory(buffer)?.to_rgba8();
        Ok(Self::from_rgba8(
            ctx,
            rgba_image.width() as u16,
            rgba_image.height() as u16,
            &rgba_image.to_vec(),
        ))
    }

    /// Parse a reader such as a `File` into a texture.
    pub fn from_reader<R: Read>(ctx: &mut Graphics, reader: &mut R) -> Result<Self> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Self::from_memory(ctx, &buf)
    }

    pub fn from_inner(handle: mq::Texture) -> Self {
        Self { handle }
    }

    pub fn set_filter_mode(&self, ctx: &mut Graphics, filter_mode: FilterMode) {
        self.handle.set_filter(
            ctx.mq_mut(),
            match filter_mode {
                FilterMode::Nearest => mq::FilterMode::Nearest,
                FilterMode::Linear => mq::FilterMode::Linear,
            },
        );
    }

    pub fn width(&self) -> u32 {
        self.handle.width
    }

    pub fn height(&self) -> u32 {
        self.handle.height
    }
}

impl DrawableMut for OwnedTexture {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.draw(ctx, instance);
    }
}

impl Drawable for OwnedTexture {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        ctx.state.quad_bindings.vertex_buffers[1].update(
            &mut ctx.mq,
            &[instance
                .scale2(Vector2::new(self.width() as f32, self.height() as f32))
                .to_instance_properties()],
        );
        ctx.state.quad_bindings.images[0] = self.handle;
        ctx.mq.apply_bindings(&ctx.state.quad_bindings);
        ctx.apply_modelview();
        ctx.mq.draw(0, 6, 1);
    }
}

impl LuaUserData for OwnedTexture {}

impl Drop for OwnedTexture {
    fn drop(&mut self) {
        self.handle.delete();
    }
}

#[derive(Debug, Clone)]
pub struct SharedTexture {
    pub shared: Arc<OwnedTexture>,
}

impl Deref for SharedTexture {
    type Target = OwnedTexture;

    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

impl From<mq::Texture> for SharedTexture {
    fn from(texture: mq::Texture) -> Self {
        Self {
            shared: Arc::new(OwnedTexture::from_inner(texture)),
        }
    }
}

impl From<OwnedTexture> for SharedTexture {
    fn from(owned: OwnedTexture) -> Self {
        Self {
            shared: Arc::new(owned),
        }
    }
}

impl DrawableMut for SharedTexture {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.draw(ctx, instance);
    }
}

impl Drawable for SharedTexture {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        self.shared.draw(ctx, instance);
    }
}

#[derive(Debug, Clone)]
pub struct CachedTexture {
    pub cached: CacheRef<OwnedTexture>,
}

impl From<mq::Texture> for CachedTexture {
    fn from(texture: mq::Texture) -> Self {
        Self {
            cached: CacheRef::new_uncached(OwnedTexture::from_inner(texture)),
        }
    }
}

impl From<OwnedTexture> for CachedTexture {
    fn from(owned: OwnedTexture) -> Self {
        Self {
            cached: CacheRef::new_uncached(owned),
        }
    }
}

impl From<Handle<OwnedTexture>> for CachedTexture {
    fn from(handle: Handle<OwnedTexture>) -> Self {
        Self {
            cached: handle.into_cached(),
        }
    }
}

impl CachedTexture {
    pub fn get(&self) -> Guard<OwnedTexture> {
        self.cached.get()
    }

    pub fn get_cached(&mut self) -> &OwnedTexture {
        self.cached.get_cached()
    }

    pub fn to_shared(&self) -> SharedTexture {
        SharedTexture {
            shared: self.cached.get().clone(),
        }
    }

    pub fn ptr_eq(lhs: &Self, rhs: &Self) -> bool {
        CacheRef::ptr_eq(&lhs.cached, &rhs.cached)
    }

    pub fn ptr_eq_cached(lhs: &mut Self, rhs: &mut Self) -> bool {
        CacheRef::ptr_eq_cached(&mut lhs.cached, &mut rhs.cached)
    }
}

impl DrawableMut for CachedTexture {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.get_cached().draw(ctx, instance);
    }
}

impl Drawable for CachedTexture {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        self.get().draw(ctx, instance);
    }
}

impl LuaUserData for CachedTexture {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        crate::lua::add_drawable_methods(methods);
    }
}

pub struct FilesystemTextureLoader {
    engine_ref: EngineRef,
    gfx_lock: Shared<GraphicsLock>,
}

impl FilesystemTextureLoader {
    pub fn new(engine: &Engine, gfx_lock: &Shared<GraphicsLock>) -> Self {
        Self {
            engine_ref: engine.downgrade(),
            gfx_lock: gfx_lock.clone(),
        }
    }
}

impl<P: AsRef<Path>> Loader<P, OwnedTexture> for FilesystemTextureLoader {
    fn load(&mut self, key: &P) -> Result<Handle<OwnedTexture>> {
        let engine = self.engine_ref.upgrade();
        let mut file = engine.fs().open(key)?;
        let texture = OwnedTexture::from_reader(&mut self.gfx_lock.lock(), &mut file)?;
        Ok(Handle::new(texture))
    }
}

pub struct TextureCache {
    inner: SwappableCache<String, OwnedTexture, FilesystemTextureLoader>,
}

impl LuaUserData for TextureCache {}

impl LuaResource for TextureCache {
    const REGISTRY_KEY: &'static str = "HV_FRIENDS_TEXTURE_CACHE";
}

impl TextureCache {
    pub fn new(engine: &Engine, gfx_lock: &Shared<GraphicsLock>) -> Self {
        Self {
            inner: SwappableCache::new(FilesystemTextureLoader::new(engine, gfx_lock)),
        }
    }

    pub fn get_or_load(&mut self, path: impl Into<String>) -> Result<CachedTexture> {
        self.inner.get_or_load(path.into()).map(CachedTexture::from)
    }

    pub fn reload_all(&mut self) -> Result<()> {
        self.inner.reload_all()
    }
}
