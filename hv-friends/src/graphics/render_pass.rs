use hibitset::{AtomicBitSet, DrainableBitSet};
use hv_core::{mlua::prelude::*, mq, util::RwLockExt};
use std::{
    ops::Deref,
    sync::{Arc, RwLock},
};
use thunderdome::{Arena, Index};

use crate::graphics::{Graphics, SharedTexture};

#[derive(Debug)]
pub(crate) struct RenderPassRegistry {
    registry: Arena<mq::RenderPass>,
    cleanup: Arc<RwLock<AtomicBitSet>>,
}

impl RenderPassRegistry {
    pub(crate) fn new() -> Self {
        Self {
            registry: Arena::new(),
            cleanup: Arc::new(RwLock::new(AtomicBitSet::new())),
        }
    }

    fn insert(&mut self, mq: &mut mq::Context, handle: mq::RenderPass) -> OwnedRenderPass {
        let registry = &mut self.registry;
        let mut cleanup = self.cleanup.borrow_mut();
        for (_, render_pass) in cleanup
            .drain()
            .filter_map(|slot| registry.remove_by_slot(slot))
        {
            render_pass.delete(mq);
        }

        let registry_index = self.registry.insert(handle);
        let registry_cleanup = self.cleanup.clone();

        OwnedRenderPass {
            handle,
            registry_index,
            registry_cleanup,
        }
    }
}

#[derive(Debug)]
pub struct OwnedRenderPass {
    pub handle: mq::RenderPass,
    registry_index: Index,
    registry_cleanup: Arc<RwLock<AtomicBitSet>>,
}

impl Drop for OwnedRenderPass {
    fn drop(&mut self) {
        self.registry_cleanup
            .borrow()
            .add_atomic(self.registry_index.slot());
    }
}

#[derive(Debug, Clone)]
pub struct RenderPass {
    shared: Arc<OwnedRenderPass>,
}

impl Deref for RenderPass {
    type Target = OwnedRenderPass;

    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

impl RenderPass {
    pub fn new(
        ctx: &mut Graphics,
        color_img: SharedTexture,
        depth_img: impl Into<Option<SharedTexture>>,
    ) -> Self {
        Self::from_parts(ctx, color_img.handle, depth_img.into().map(|di| di.handle))
    }

    pub fn from_parts(
        ctx: &mut Graphics,
        color_img: mq::Texture,
        depth_img: Option<mq::Texture>,
    ) -> Self {
        let handle = mq::RenderPass::new(&mut ctx.mq, color_img, depth_img);
        Self {
            shared: Arc::new(ctx.state.render_passes.insert(&mut ctx.mq, handle)),
        }
    }
}

impl LuaUserData for RenderPass {}
