use crate::{graphics::LinearColor, math::*};
use hv_core::mq;

pub const BASIC_VERTEX: &str = include_str!("basic_es300.glslv");
pub const BASIC_FRAGMENT: &str = include_str!("basic_es300.glslf");

pub fn meta() -> mq::ShaderMeta {
    mq::ShaderMeta {
        images: vec!["t_Texture".to_string()],
        uniforms: mq::UniformBlockLayout {
            uniforms: vec![mq::UniformDesc::new("u_MVP", mq::UniformType::Mat4)],
        },
    }
}

#[repr(C)]
pub struct Uniforms {
    pub mvp: Matrix4<f32>,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Vertex {
    pub pos: Vector3<f32>,
    pub uv: Vector2<f32>,
    pub color: LinearColor,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InstanceProperties {
    pub src: Vector4<f32>,
    pub tx: Matrix4<f32>,
    pub color: LinearColor,
}
