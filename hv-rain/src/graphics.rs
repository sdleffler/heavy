use hv_core::{engine::LuaResource, mlua::prelude::*};
use hv_friends::graphics::{
    pipeline::Pipeline,
    sprite::{CachedSpriteSheet, SpriteFrame, SpriteTag},
    CachedTexture, SpriteBatch,
};
use std::ops;
use thunderdome::{Arena, Index};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectileSpriteBatchId(pub(crate) Index);

impl<'lua> ToLua<'lua> for ProjectileSpriteBatchId {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        LuaLightUserData(self.0.to_bits() as *mut _).to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for ProjectileSpriteBatchId {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        LuaLightUserData::from_lua(lua_value, lua).map(|lud| Self(Index::from_bits(lud.0 as u64)))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ProjectileSprite {
    pub batch_id: ProjectileSpriteBatchId,
    pub tag: SpriteTag,
    pub frame: SpriteFrame,
}

impl LuaUserData for ProjectileSprite {}

pub struct ProjectileSpriteBatch {
    pub sheet: CachedSpriteSheet,
    pub sprites: SpriteBatch<CachedTexture>,
    pub pipeline: Option<Pipeline>,
}

pub struct ProjectileSpriteRegistry {
    pub defs: Arena<ProjectileSpriteBatch>,
}

impl Default for ProjectileSpriteRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectileSpriteRegistry {
    pub fn new() -> Self {
        Self { defs: Arena::new() }
    }

    pub fn clear_batches(&mut self) {
        for (_, batch) in self.defs.iter_mut() {
            batch.sprites.clear();
        }
    }
}

impl ops::Index<ProjectileSpriteBatchId> for ProjectileSpriteRegistry {
    type Output = ProjectileSpriteBatch;

    fn index(&self, index: ProjectileSpriteBatchId) -> &Self::Output {
        &self.defs[index.0]
    }
}

impl ops::IndexMut<ProjectileSpriteBatchId> for ProjectileSpriteRegistry {
    fn index_mut(&mut self, index: ProjectileSpriteBatchId) -> &mut Self::Output {
        &mut self.defs[index.0]
    }
}

impl LuaUserData for ProjectileSpriteRegistry {}

impl LuaResource for ProjectileSpriteRegistry {
    const REGISTRY_KEY: &'static str = "HV_DANMAKU_PROJECTILE_SPRITE_REGISTRY";
}
