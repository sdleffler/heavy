use anyhow::*;
use hv_core::{engine::Resource, mlua::prelude::*, mq};

use crate::graphics::{Buffer, CachedTexture, GraphicsLock};

#[derive(Debug, Clone)]
pub struct Bindings {
    mq: mq::Bindings,
    pub vertex_buffers: Vec<Buffer>,
    pub index_buffer: Buffer,
    pub textures: Vec<CachedTexture>,
}

impl Bindings {
    pub fn new(
        vertex_buffers: Vec<Buffer>,
        index_buffer: Buffer,
        mut textures: Vec<CachedTexture>,
    ) -> Self {
        Self {
            mq: mq::Bindings {
                vertex_buffers: vertex_buffers.iter().map(|vbo| vbo.handle).collect(),
                index_buffer: index_buffer.handle,
                images: textures
                    .iter_mut()
                    .map(|tex| tex.get_cached().handle)
                    .collect(),
            },

            vertex_buffers,
            index_buffer,
            textures,
        }
    }

    pub(crate) fn update(&mut self) -> &mq::Bindings {
        if self.mq.vertex_buffers.len() != self.vertex_buffers.len() {
            self.mq.vertex_buffers.clear();
            self.mq
                .vertex_buffers
                .extend(self.vertex_buffers.iter().map(|vbo| vbo.handle));
        } else {
            for (mq_vbo, vbo) in self.mq.vertex_buffers.iter_mut().zip(&self.vertex_buffers) {
                *mq_vbo = vbo.handle;
            }
        }

        self.mq.index_buffer = self.index_buffer.handle;

        if self.mq.images.len() != self.textures.len() {
            self.mq.images.clear();
            self.mq
                .images
                .extend(self.textures.iter_mut().map(|tex| tex.get_cached().handle));
        } else {
            for (mq_image, tex) in self.mq.images.iter_mut().zip(self.textures.iter_mut()) {
                *mq_image = tex.get_cached().handle;
            }
        }

        &self.mq
    }
}

impl LuaUserData for Bindings {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        add_getter!(fields, t.index_buffer => t.index_buffer.clone());
        add_setter!(fields, t.index_buffer = new_buf => t.index_buffer = new_buf);
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("vertex_buffer_at_index", |_lua, this, i: usize| {
            Ok(this.vertex_buffers[i].clone())
        });

        methods.add_method("vertex_buffers", |lua, this, ()| {
            this.vertex_buffers.as_slice().to_lua(lua)
        });

        methods.add_method("texture_at_index", |_lua, this, i: usize| {
            Ok(this.textures[i].clone())
        });

        methods.add_method("textures", |lua, this, ()| {
            this.textures.as_slice().to_lua(lua)
        });

        crate::lua::simple_mut(
            methods,
            "set_texture_at_index",
            |this, (index, texture): (usize, CachedTexture)| this.textures[index] = texture,
        );

        crate::lua::simple_mut(
            methods,
            "set_vertex_buffer_at_index",
            |this, (index, vbo): (usize, Buffer)| this.vertex_buffers[index] = vbo,
        );
    }
}

pub(super) fn open<'lua>(
    lua: &'lua Lua,
    _shared_gfx: &Resource<GraphicsLock>,
) -> Result<LuaTable<'lua>> {
    let buffer = lua.create_table()?;

    buffer.set(
        "create_bindings_object",
        lua.create_function_mut(
            move |_lua,
                  (vertex_buffers, index_buffer, textures): (
                Vec<Buffer>,
                Buffer,
                Vec<CachedTexture>,
            )| { Ok(Bindings::new(vertex_buffers, index_buffer, textures)) },
        )?,
    )?;

    Ok(buffer)
}
