use hv_core::{mlua::prelude::*, mq};

use crate::graphics::{Drawable, DrawableMut, Graphics, Instance, RenderPass, SharedTexture};

#[derive(Debug)]
pub struct Canvas {
    pub render_pass: RenderPass,
    pub color_buffer: SharedTexture,
    pub depth_buffer: SharedTexture,
}

impl AsRef<RenderPass> for Canvas {
    fn as_ref(&self) -> &RenderPass {
        &self.render_pass
    }
}

impl Canvas {
    pub fn new(ctx: &mut Graphics, width: u32, height: u32) -> Self {
        let color_img = SharedTexture::from(mq::Texture::new_render_texture(
            &mut ctx.mq,
            mq::TextureParams {
                width,
                height,
                format: mq::TextureFormat::RGBA8,
                filter: mq::FilterMode::Nearest,
                ..Default::default()
            },
        ));

        let depth_img = SharedTexture::from(mq::Texture::new_render_texture(
            &mut ctx.mq,
            mq::TextureParams {
                width,
                height,
                format: mq::TextureFormat::Depth,
                filter: mq::FilterMode::Nearest,
                ..Default::default()
            },
        ));

        let render_pass = RenderPass::from_parts(ctx, color_img.handle, Some(depth_img.handle));

        Self {
            render_pass,
            color_buffer: color_img,
            depth_buffer: depth_img,
        }
    }
}

impl DrawableMut for Canvas {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.color_buffer.draw_mut(ctx, instance);
    }
}

impl Drawable for Canvas {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        self.color_buffer.draw(ctx, instance);
    }
}

impl LuaUserData for Canvas {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("render_pass", |_, this| Ok(this.render_pass.clone()));
    }
}
