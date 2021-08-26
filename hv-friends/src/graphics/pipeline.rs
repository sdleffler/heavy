use anyhow::*;
use hibitset::{AtomicBitSet, DrainableBitSet};
use hv_core::{
    engine::Resource,
    mlua::{self, prelude::*},
    mq,
    util::RwLockExt,
};
use std::{
    ops,
    sync::{Arc, RwLock},
};
use thunderdome::{Arena, Index};

use crate::graphics::{BlendMode, Graphics, GraphicsLock, GraphicsLockExt};

/// Indicates whether or not a buffer should be indexed per-vertex or per-instance. Per-instance
/// steps are useful for holding transforms/different parameters when drawing many instances at once.
#[derive(Debug, Clone, Copy)]
pub enum VertexStep {
    PerInstance,
    PerVertex,
}

impl From<VertexStep> for mq::VertexStep {
    fn from(step: VertexStep) -> Self {
        match step {
            VertexStep::PerInstance => mq::VertexStep::PerInstance,
            VertexStep::PerVertex => mq::VertexStep::PerVertex,
        }
    }
}

impl LuaUserData for VertexStep {}

/// The layout of a vertex buffer object. There are other options here under the hood but for now
/// all we care about is whether the buffer is per-vertex or per-instance.
#[derive(Debug, Clone, Copy)]
pub struct BufferLayout {
    pub step: VertexStep,
}

impl From<BufferLayout> for mq::BufferLayout {
    fn from(layout: BufferLayout) -> Self {
        mq::BufferLayout {
            step_func: layout.step.into(),
            ..Default::default()
        }
    }
}

impl BufferLayout {
    pub fn vertex() -> Self {
        Self {
            step: VertexStep::PerVertex,
        }
    }

    pub fn instance() -> Self {
        Self {
            step: VertexStep::PerInstance,
        }
    }
}

impl LuaUserData for BufferLayout {}

#[derive(Clone, Copy, Debug)]
pub enum VertexFormat {
    Float1,
    Float2,
    Float3,
    Float4,
    Byte1,
    Byte2,
    Byte3,
    Byte4,
    Short1,
    Short2,
    Short3,
    Short4,
    Int1,
    Int2,
    Int3,
    Int4,
    Mat4,
}

impl VertexFormat {
    pub fn size(&self) -> i32 {
        match self {
            VertexFormat::Float1 => 1,
            VertexFormat::Float2 => 2,
            VertexFormat::Float3 => 3,
            VertexFormat::Float4 => 4,
            VertexFormat::Byte1 => 1,
            VertexFormat::Byte2 => 2,
            VertexFormat::Byte3 => 3,
            VertexFormat::Byte4 => 4,
            VertexFormat::Short1 => 1,
            VertexFormat::Short2 => 2,
            VertexFormat::Short3 => 3,
            VertexFormat::Short4 => 4,
            VertexFormat::Int1 => 1,
            VertexFormat::Int2 => 2,
            VertexFormat::Int3 => 3,
            VertexFormat::Int4 => 4,
            VertexFormat::Mat4 => 16,
        }
    }

    #[allow(clippy::identity_op)]
    pub fn byte_len(&self) -> i32 {
        match self {
            VertexFormat::Float1 => 1 * 4,
            VertexFormat::Float2 => 2 * 4,
            VertexFormat::Float3 => 3 * 4,
            VertexFormat::Float4 => 4 * 4,
            VertexFormat::Byte1 => 1,
            VertexFormat::Byte2 => 2,
            VertexFormat::Byte3 => 3,
            VertexFormat::Byte4 => 4,
            VertexFormat::Short1 => 1 * 2,
            VertexFormat::Short2 => 2 * 2,
            VertexFormat::Short3 => 3 * 2,
            VertexFormat::Short4 => 4 * 2,
            VertexFormat::Int1 => 1 * 4,
            VertexFormat::Int2 => 2 * 4,
            VertexFormat::Int3 => 3 * 4,
            VertexFormat::Int4 => 4 * 4,
            VertexFormat::Mat4 => 16 * 4,
        }
    }
}

impl From<VertexFormat> for mq::VertexFormat {
    fn from(vertex_format: VertexFormat) -> Self {
        match vertex_format {
            VertexFormat::Float1 => mq::VertexFormat::Float1,
            VertexFormat::Float2 => mq::VertexFormat::Float2,
            VertexFormat::Float3 => mq::VertexFormat::Float3,
            VertexFormat::Float4 => mq::VertexFormat::Float4,
            VertexFormat::Byte1 => mq::VertexFormat::Byte1,
            VertexFormat::Byte2 => mq::VertexFormat::Byte2,
            VertexFormat::Byte3 => mq::VertexFormat::Byte3,
            VertexFormat::Byte4 => mq::VertexFormat::Byte4,
            VertexFormat::Short1 => mq::VertexFormat::Short1,
            VertexFormat::Short2 => mq::VertexFormat::Short2,
            VertexFormat::Short3 => mq::VertexFormat::Short3,
            VertexFormat::Short4 => mq::VertexFormat::Short4,
            VertexFormat::Int1 => mq::VertexFormat::Int1,
            VertexFormat::Int2 => mq::VertexFormat::Int2,
            VertexFormat::Int3 => mq::VertexFormat::Int3,
            VertexFormat::Int4 => mq::VertexFormat::Int4,
            VertexFormat::Mat4 => mq::VertexFormat::Mat4,
        }
    }
}

impl LuaUserData for VertexFormat {}

#[derive(Debug, Clone)]
pub struct VertexAttribute {
    pub name: String,
    pub ty: VertexFormat,
    pub buffer_index: usize,
}

impl VertexAttribute {
    pub fn new(name: impl Into<String>, ty: VertexFormat, buffer_index: usize) -> Self {
        Self {
            name: name.into(),
            ty,
            buffer_index,
        }
    }
}

impl From<VertexAttribute> for mq::VertexAttribute {
    fn from(vertex_attribute: VertexAttribute) -> Self {
        mq::VertexAttribute {
            name: Box::leak(vertex_attribute.name.into_boxed_str()),
            format: vertex_attribute.ty.into(),
            buffer_index: vertex_attribute.buffer_index,
        }
    }
}

impl LuaUserData for VertexAttribute {}

#[derive(Debug, Clone)]
pub struct PipelineLayout {
    pub buffer_layouts: Vec<BufferLayout>,
    pub attributes: Vec<VertexAttribute>,
}

impl Default for PipelineLayout {
    fn default() -> Self {
        Self {
            buffer_layouts: vec![BufferLayout::vertex(), BufferLayout::instance()],
            attributes: vec![
                VertexAttribute::new("a_Pos", VertexFormat::Float3, 0),
                VertexAttribute::new("a_Uv", VertexFormat::Float2, 0),
                VertexAttribute::new("a_VertColor", VertexFormat::Float4, 0),
                VertexAttribute::new("a_Src", VertexFormat::Float4, 1),
                VertexAttribute::new("a_Tx", VertexFormat::Mat4, 1),
                VertexAttribute::new("a_Color", VertexFormat::Float4, 1),
            ],
        }
    }
}

impl LuaUserData for PipelineLayout {}

#[derive(Debug, Clone, Copy)]
pub enum UniformType {
    Float1,
    Float2,
    Float3,
    Float4,
    Int1,
    Int2,
    Int3,
    Int4,
    Mat4,
}

impl From<UniformType> for mq::UniformType {
    fn from(ty: UniformType) -> Self {
        match ty {
            UniformType::Float1 => mq::UniformType::Float1,
            UniformType::Float2 => mq::UniformType::Float2,
            UniformType::Float3 => mq::UniformType::Float3,
            UniformType::Float4 => mq::UniformType::Float4,
            UniformType::Int1 => mq::UniformType::Int1,
            UniformType::Int2 => mq::UniformType::Int2,
            UniformType::Int3 => mq::UniformType::Int3,
            UniformType::Int4 => mq::UniformType::Int4,
            UniformType::Mat4 => mq::UniformType::Mat4,
        }
    }
}

impl LuaUserData for UniformType {}

#[derive(Debug, Clone)]
pub struct UniformDesc {
    pub name: String,
    pub ty: UniformType,
    pub len: usize,
}

impl UniformDesc {
    pub fn new(name: impl Into<String>, ty: UniformType) -> Self {
        Self {
            name: name.into(),
            ty,
            len: 1,
        }
    }

    pub fn array(name: impl Into<String>, ty: UniformType, len: usize) -> Self {
        Self {
            name: name.into(),
            ty,
            len,
        }
    }
}

impl From<UniformDesc> for mq::UniformDesc {
    fn from(desc: UniformDesc) -> Self {
        mq::UniformDesc::new(&desc.name, desc.ty.into()).array(desc.len)
    }
}

impl LuaUserData for UniformDesc {}

#[derive(Debug, Clone)]
pub struct ShaderLayout {
    pub uniforms: Vec<UniformDesc>,
    pub images: Vec<String>,
}

impl From<ShaderLayout> for mq::ShaderMeta {
    fn from(layout: ShaderLayout) -> Self {
        mq::ShaderMeta {
            uniforms: mq::UniformBlockLayout {
                uniforms: layout
                    .uniforms
                    .into_iter()
                    .map(mq::UniformDesc::from)
                    .collect(),
            },
            images: layout.images,
        }
    }
}

impl Default for ShaderLayout {
    fn default() -> Self {
        Self {
            images: vec!["t_Texture".to_string()],
            uniforms: vec![UniformDesc::new("u_MVP", UniformType::Mat4)],
        }
    }
}

impl LuaUserData for ShaderLayout {}

#[derive(Debug)]
pub(crate) struct ShaderRegistry {
    registry: Arena<mq::Shader>,
    cleanup: Arc<RwLock<AtomicBitSet>>,
}

impl ShaderRegistry {
    pub fn new() -> Self {
        Self {
            registry: Arena::new(),
            cleanup: Arc::new(RwLock::new(AtomicBitSet::new())),
        }
    }

    fn insert(&mut self, _mq: &mut mq::Context, handle: mq::Shader) -> OwnedShader {
        let registry = &mut self.registry;
        let mut cleanup = self.cleanup.borrow_mut();
        for (_, _shader) in cleanup
            .drain()
            .filter_map(|slot| registry.remove_by_slot(slot))
        {
            log::warn!("Shader object \"destroyed\" (shader object leaked; no way to currently destroy shaders)");
        }

        let registry_index = self.registry.insert(handle);
        let registry_cleanup = self.cleanup.clone();

        OwnedShader {
            handle,
            registry_index,
            registry_cleanup,
        }
    }
}

#[derive(Debug)]
pub struct OwnedShader {
    pub handle: mq::Shader,
    registry_index: Index,
    registry_cleanup: Arc<RwLock<AtomicBitSet>>,
}

impl Drop for OwnedShader {
    fn drop(&mut self) {
        self.registry_cleanup
            .borrow()
            .add_atomic(self.registry_index.slot());
    }
}

#[derive(Debug, Clone)]
pub struct Shader {
    inner: Arc<OwnedShader>,
}

impl Shader {
    pub fn new(
        gfx: &mut Graphics,
        vertex: &str,
        fragment: &str,
        layout: ShaderLayout,
    ) -> Result<Self> {
        let handle = mq::Shader::new(&mut gfx.mq, vertex, fragment, layout.into())?;
        Ok(Self {
            inner: Arc::new(gfx.state.shaders.insert(&mut gfx.mq, handle)),
        })
    }
}

impl ops::Deref for Shader {
    type Target = OwnedShader;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl LuaUserData for Shader {}

#[derive(Debug, Clone, Copy)]
pub struct PipelineParams {
    // pub cull_face: CullFace,
    // pub front_face_order: FrontFaceOrder,
    // pub depth_test: Comparison,
    pub depth_write: bool,
    pub depth_write_offset: Option<(f32, f32)>,
    // pub color_blend: Option<BlendState>,
    // pub alpha_blend: Option<BlendState>,
    // pub stencil_test: Option<StencilState>,
    pub color_write: (bool, bool, bool, bool),
    // pub primitive_type: PrimitiveType,
}

impl Default for PipelineParams {
    fn default() -> Self {
        Self {
            depth_write: true,
            depth_write_offset: None,
            color_write: (true, true, true, true),
        }
    }
}

impl LuaUserData for PipelineParams {}

#[derive(Debug)]
pub(crate) struct PipelineRegistry {
    registry: Arena<mq::Pipeline>,
    cleanup: Arc<RwLock<AtomicBitSet>>,
}

impl PipelineRegistry {
    pub fn new() -> Self {
        Self {
            registry: Arena::new(),
            cleanup: Arc::new(RwLock::new(AtomicBitSet::new())),
        }
    }

    fn insert(
        &mut self,
        _mq: &mut mq::Context,
        handle: mq::Pipeline,
        layout: PipelineLayout,
        shader: Shader,
    ) -> OwnedPipeline {
        let registry = &mut self.registry;
        let mut cleanup = self.cleanup.borrow_mut();
        for (_, _pipeline) in cleanup
            .drain()
            .filter_map(|slot| registry.remove_by_slot(slot))
        {
            log::warn!("Pipeline object \"destroyed\" (pipeline object leaked; no way to currently destroy pipeline objects)");
        }

        let registry_index = self.registry.insert(handle);
        let registry_cleanup = self.cleanup.clone();

        OwnedPipeline {
            handle,
            layout,
            shader,
            registry_index,
            registry_cleanup,
        }
    }
}

#[derive(Debug)]
pub struct OwnedPipeline {
    pub handle: mq::Pipeline,
    pub layout: PipelineLayout,
    pub shader: Shader,
    registry_index: Index,
    registry_cleanup: Arc<RwLock<AtomicBitSet>>,
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub shared: Arc<OwnedPipeline>,
}

impl ops::Deref for Pipeline {
    type Target = OwnedPipeline;

    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

impl Pipeline {
    pub fn new(
        gfx: &mut Graphics,
        layout: PipelineLayout,
        shader: Shader,
        _params: Option<PipelineParams>,
    ) -> Result<Self> {
        let buffer_layouts = layout
            .buffer_layouts
            .iter()
            .copied()
            .map(mq::BufferLayout::from)
            .collect::<Vec<_>>();

        let vertex_attributes = layout
            .attributes
            .iter()
            .cloned()
            .map(mq::VertexAttribute::from)
            .collect::<Vec<_>>();

        let handle = mq::Pipeline::with_params(
            &mut gfx.mq,
            &buffer_layouts,
            &vertex_attributes,
            shader.handle,
            mq::PipelineParams {
                color_blend: Some(BlendMode::default().into()),
                depth_test: mq::Comparison::LessOrEqual,
                depth_write: true,
                ..mq::PipelineParams::default()
            },
        );

        Ok(Self {
            shared: Arc::new(
                gfx.state
                    .pipelines
                    .insert(&mut gfx.mq, handle, layout, shader),
            ),
        })
    }
}

impl LuaUserData for Pipeline {}

#[derive(Debug)]
pub struct Uniforms {
    bytes: Vec<u8>,
    descs: Vec<UniformDesc>,
    offsets: Vec<usize>,
}

impl Uniforms {
    pub fn new(shader_layout: &ShaderLayout) -> Self {
        use crevice::std140::*;

        let mut sizer = Sizer::new();
        let mut offsets = Vec::new();
        for desc in shader_layout.uniforms.iter() {
            assert_eq!(desc.len, 1, "array uniforms not yet implemented");
            let offset = match desc.ty {
                UniformType::Float1 => sizer.add::<f32>(),
                UniformType::Float2 => sizer.add::<Vec2>(),
                UniformType::Float3 => sizer.add::<Vec3>(),
                UniformType::Float4 => sizer.add::<Vec4>(),
                UniformType::Int1 => sizer.add::<i32>(),
                UniformType::Mat4 => sizer.add::<Mat4>(),
                _ => unimplemented!(),
            };
            offsets.push(offset);
        }

        let mut bytes = Vec::new();
        bytes.resize(sizer.len(), 0);

        Self {
            bytes,
            descs: shader_layout.uniforms.clone(),
            offsets,
        }
    }

    pub fn get_uniform_index_by_name(&mut self, name: &str) -> Option<usize> {
        self.descs.iter().position(|desc| desc.name == name)
    }

    pub fn set_uniform_by_name<T: Copy>(&mut self, name: &str, value: &T) {
        let uniform_index = self
            .descs
            .iter()
            .position(|desc| desc.name == name)
            .expect("no such uniform");
        self.set_uniform_by_index(uniform_index, value);
    }

    pub fn set_uniform_by_index<T: Copy>(&mut self, index: usize, value: &T) {
        let bytes_at_offset = &mut self.bytes[self.offsets[index]..];
        unsafe {
            bytes_at_offset.align_to_mut::<T>().1[0] = *value;
        }
    }

    pub fn set_uniform_by_index_from_lua(
        &mut self,
        index: usize,
        lua: &Lua,
        value: LuaValue,
    ) -> Result<(), Error> {
        let ty = self.descs[index].ty;

        match ty {
            UniformType::Float1 => {
                self.set_uniform_by_index(index, &lua.from_value::<f32>(value)?);
            }
            UniformType::Float2 => {
                self.set_uniform_by_index(index, &lua.from_value::<[f32; 2]>(value)?);
            }
            UniformType::Float3 => {
                self.set_uniform_by_index(index, &lua.from_value::<[f32; 3]>(value)?);
            }
            UniformType::Float4 => {
                self.set_uniform_by_index(index, &lua.from_value::<[f32; 4]>(value)?);
            }
            UniformType::Mat4 => {
                self.set_uniform_by_index(index, &lua.from_value::<[f32; 16]>(value)?);
            }
            _ => unimplemented!(),
        }

        Ok(())
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}

impl LuaUserData for Uniforms {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut(
            "set_uniform_by_name",
            |lua, this, (name, value): (LuaString, LuaValue)| {
                let index = this
                    .get_uniform_index_by_name(name.to_str()?)
                    .expect("no such uniform");
                this.set_uniform_by_index_from_lua(index, lua, value)
                    .to_lua_err()
            },
        );

        methods.add_method_mut(
            "set_uniform_by_index",
            |lua, this, (index, value): (usize, LuaValue)| {
                this.set_uniform_by_index_from_lua(index, lua, value)
                    .to_lua_err()
            },
        );
    }
}

pub(super) fn open<'lua>(
    lua: &'lua Lua,
    gfx_lock: &Resource<GraphicsLock>,
) -> Result<LuaTable<'lua>, Error> {
    let pipeline = lua.create_table()?;

    pipeline.set("VertexFormat", {
        let float1 = VertexFormat::Float1;
        let float2 = VertexFormat::Float2;
        let float3 = VertexFormat::Float3;
        let float4 = VertexFormat::Float4;
        let byte1 = VertexFormat::Byte1;
        let byte2 = VertexFormat::Byte2;
        let byte3 = VertexFormat::Byte3;
        let byte4 = VertexFormat::Byte4;
        let short1 = VertexFormat::Short1;
        let short2 = VertexFormat::Short2;
        let short3 = VertexFormat::Short3;
        let short4 = VertexFormat::Short4;
        let int1 = VertexFormat::Int1;
        let int2 = VertexFormat::Int2;
        let int3 = VertexFormat::Int3;
        let int4 = VertexFormat::Int4;
        let mat4 = VertexFormat::Mat4;
        lua.load(mlua::chunk! {
            {
                Float1 = $float1,
                Float2 = $float2,
                Float3 = $float3,
                Float4 = $float4,
                Byte1 = $byte1,
                Byte2 = $byte2,
                Byte3 = $byte3,
                Byte4 = $byte4,
                Short1 = $short1,
                Short2 = $short2,
                Short3 = $short3,
                Short4 = $short4,
                Int1 = $int1,
                Int2 = $int2,
                Int3 = $int3,
                Int4 = $int4,
                Mat4 = $mat4,
                nil
            }
        })
        .eval::<LuaTable>()?
    })?;

    pipeline.set(
        "create_vertex_attribute_object",
        lua.create_function(
            move |_lua, (name, ty, buffer_index): (String, VertexFormat, usize)| {
                Ok(VertexAttribute {
                    name,
                    ty,
                    buffer_index,
                })
            },
        )?,
    )?;

    pipeline.set("UniformType", {
        let float1 = UniformType::Float1;
        let float2 = UniformType::Float2;
        let float3 = UniformType::Float3;
        let float4 = UniformType::Float4;
        let int1 = UniformType::Int1;
        let int2 = UniformType::Int2;
        let int3 = UniformType::Int3;
        let int4 = UniformType::Int4;
        let mat4 = UniformType::Mat4;
        lua.load(mlua::chunk! {
            {
                Float1 = $float1,
                Float2 = $float2,
                Float3 = $float3,
                Float4 = $float4,
                Int1 = $int1,
                Int2 = $int2,
                Int3 = $int3,
                Int4 = $int4,
                Mat4 = $mat4,
                nil
            }
        })
        .eval::<LuaTable>()?
    })?;

    pipeline.set(
        "create_uniform_desc_object",
        lua.create_function(
            move |_lua, (name, ty, len): (String, UniformType, Option<usize>)| {
                Ok(UniformDesc {
                    name,
                    ty,
                    len: len.unwrap_or(1),
                })
            },
        )?,
    )?;

    pipeline.set(
        "create_shader_layout_object",
        lua.create_function(move |_lua, (uniforms, images)| Ok(ShaderLayout { uniforms, images }))?,
    )?;

    pipeline.set("VertexStep", {
        let per_instance = VertexStep::PerInstance;
        let per_vertex = VertexStep::PerVertex;

        lua.load(mlua::chunk! {
            {
                PerInstance = $per_instance,
                PerVertex = $per_vertex,
                nil
            }
        })
        .eval::<LuaTable>()?
    })?;

    pipeline.set(
        "create_buffer_layout_object",
        lua.create_function(move |_lua, step| Ok(BufferLayout { step }))?,
    )?;

    let gfx = gfx_lock.clone();
    pipeline.set(
        "create_shader_object",
        lua.create_function(
            move |_lua, (vertex, fragment, layout): (LuaString, LuaString, Option<ShaderLayout>)| {
                let g = &mut gfx.lock();
                Shader::new(g, vertex.to_str()?, fragment.to_str()?, layout.unwrap_or_default()).to_lua_err()
            },
        )?,
    )?;

    pipeline.set(
        "create_pipeline_layout_object",
        lua.create_function(move |_lua, (buffer_layouts, attributes)| {
            Ok(PipelineLayout {
                buffer_layouts,
                attributes,
            })
        })?,
    )?;

    let gfx = gfx_lock.clone();
    pipeline.set(
        "create_pipeline_object",
        lua.create_function(
            move |_lua,
                  (pipeline_layout, shader, maybe_params): (
                Option<PipelineLayout>,
                Shader,
                Option<PipelineParams>,
            )| {
                let g = &mut gfx.lock();
                Pipeline::new(g, pipeline_layout.unwrap_or_default(), shader, maybe_params)
                    .to_lua_err()
            },
        )?,
    )?;

    pipeline.set(
        "create_uniforms_object",
        lua.create_function(|_lua, shader_layout| Ok(Uniforms::new(&shader_layout)))?,
    )?;

    Ok(pipeline)
}
