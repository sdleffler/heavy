//! # Love2D-like graphics API
//!
//! Provides a Love2D-like graphics API to both Lua and Rust, with Love2D-esque graphics state on
//! the Lua side provided by [`LuaGraphicsState`] and a more flexible and speed-oriented API on the
//! Rust side. Comes with default shaders for doing most 2D game rendering.
//!
//! # Locking behavior and the [`Engine`]
//!
//! Internally, in order to access the graphics context, the [`GraphicsLock`] type must lock both
//! its graphics state *and* the window/graphics context type stored in [`Engine`]. This locking
//! behavior is mutex-based and not [`RwLock`]-based, so care must be taken accordingly. 

use std::{
    mem,
    ptr::NonNull,
    sync::{Arc, Mutex, MutexGuard, RwLock},
};

use hv_core::{
    components::DynamicComponentConstructor,
    engine::{Engine, EngineRef, LuaExt, LuaResource},
    mq::{self, PassAction},
    prelude::*,
    shared::{RefMut, Shared},
};
use serde::*;

use crate::{
    graphics::{
        bindings::Bindings,
        lua::{LuaDrawMode, LuaGraphicsState},
        pipeline::{Pipeline, PipelineRegistry, ShaderRegistry},
        render_pass::RenderPassRegistry,
        sprite::{CachedSpriteSheet, SpriteAnimationState, SpriteSheetCache},
        texture::TextureCache,
    },
    math::*,
};

pub mod basic;
pub mod bindings;
pub mod buffer;
pub mod canvas;
mod color;
mod lua;
pub mod mesh;
pub mod pipeline;
pub mod render_pass;
pub mod sprite;
pub mod text;
pub mod texture;
mod transform_stack;

pub use basic::{InstanceProperties, Uniforms, Vertex};
pub use buffer::{Buffer, BufferElement, BufferFormat, BufferType, OwnedBuffer};
pub use canvas::Canvas;
pub use color::{Color, LinearColor};
pub use mesh::{DrawMode, Mesh, MeshBuilder};
pub use render_pass::{OwnedRenderPass, RenderPass};
pub use sprite::{Sprite, SpriteBatch, SpriteId};
pub use texture::{CachedTexture, OwnedTexture, SharedTexture};
pub use transform_stack::TransformStack;

fn quad_vertices() -> [Vertex; 4] {
    [
        Vertex {
            pos: Vector3::new(0., 0., 0.),
            uv: Vector2::new(0., 0.),
            color: Color::WHITE.into(),
        },
        Vertex {
            pos: Vector3::new(1., 0., 0.),
            uv: Vector2::new(1., 0.),
            color: Color::WHITE.into(),
        },
        Vertex {
            pos: Vector3::new(1., 1., 0.),
            uv: Vector2::new(1., 1.),
            color: Color::WHITE.into(),
        },
        Vertex {
            pos: Vector3::new(0., 1., 0.),
            uv: Vector2::new(0., 1.),
            color: Color::WHITE.into(),
        },
    ]
}

fn quad_indices() -> [u16; 6] {
    [0, 1, 2, 0, 2, 3]
}

/// Represents the parameters available for a single instance using the default shaders and render
/// pipeline.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Instance {
    /// The source rectangle in UV coordinates for which this instance's vertices' UV coordinates
    /// should be interpreted in. Defaults to `Box2::new(0., 0., 1., 1.)`, which in essence is the
    /// "identity" value.
    pub src: Box2<f32>,
    /// The local transform of this instance. Defaults to `Transform3::identity()`.
    pub tx: Transform3<f32>,
    /// The color of this instance. Defaults to [`Color::WHITE`], which in essence is the "identity"
    /// value.
    pub color: Color,
}

impl Default for Instance {
    fn default() -> Self {
        Self {
            src: Box2::new(0., 0., 1., 1.),
            tx: Transform3::identity(),
            color: Color::WHITE,
        }
    }
}

impl Instance {
    /// Construct a new `Instance` with default parameters.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method for setting the source rectangle of an `Instance`.
    #[inline]
    pub fn src(self, src: Box2<f32>) -> Self {
        Self { src, ..self }
    }

    /// Builder method for setting the color of an `Instance`.
    #[inline]
    pub fn color(self, color: Color) -> Self {
        Self { color, ..self }
    }

    /// Builder method for right-multiplying a 2D rotation onto the transform of an `Instance`.
    #[inline]
    pub fn rotate2(self, angle: f32) -> Self {
        Self {
            tx: self.tx
                * Transform3::from_matrix_unchecked(homogeneous_mat3_to_mat4(
                    &Rotation2::new(angle).to_homogeneous(),
                )),
            ..self
        }
    }

    /// Builder method for right-multiplying a 2D translation onto the transform of an `Instance`.
    #[inline]
    pub fn translate2(self, v: Vector2<f32>) -> Self {
        Self {
            tx: self.tx * Translation3::new(v.x, v.y, 0.),
            ..self
        }
    }

    /// Builder method for right-multiplying a potentially non-uniform 2D scaling onto the transform
    /// of an `Instance`.
    #[inline]
    pub fn scale2(self, v: Vector2<f32>) -> Self {
        Self {
            tx: self.tx
                * Transform3::from_matrix_unchecked(Matrix4::from_diagonal(&v.push(1.).push(1.))),
            ..self
        }
    }

    /// Builder method for right-multiplying a 3D translation onto the transform of an `Instance`.
    #[inline]
    pub fn translate3(self, v: Vector3<f32>) -> Self {
        Self {
            tx: self.tx * Translation3::from(v),
            ..self
        }
    }

    /// Builder method for right-multiplying a general 3D transform onto the transform of an
    /// `Instance`.
    #[inline]
    pub fn transform3(self, tx: &Transform3<f32>) -> Self {
        Self {
            tx: self.tx * tx,
            ..self
        }
    }

    /// Calculate the AABB resulting from transforming the given AABB by the internal transform of
    /// this `Instance`. The result is a conservative approximation which would contain any shape
    /// the original AABB was calculated from if that shape were to be transformed by this
    /// instance's transform (see [`Box2::transformed_by`]).
    #[inline]
    pub fn transform_aabb(&self, aabb: &Box2<f32>) -> Box2<f32> {
        aabb.transformed_by(self.tx.matrix())
    }

    /// Convert this `Instance` to the corresponding [`InstanceProperties`], for use in an instance
    /// buffer with the default ([`basic`]) shader and vertex/uniform types.
    #[inline]
    pub fn to_instance_properties(&self) -> InstanceProperties {
        let mins = self.src.mins;
        let extents = self.src.extents();
        InstanceProperties {
            src: Vector4::new(mins.x, mins.y, extents.x, extents.y),
            tx: *self.tx.matrix(),
            color: LinearColor::from(self.color),
        }
    }
}

impl LuaUserData for Instance {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("reset", |_, this, ()| {
            *this = Instance::new();
            Ok(())
        });

        methods.add_method_mut("src", |_, this, (x, y, w, h)| {
            *this = this.src(Box2::new(x, y, w, h));
            Ok(())
        });

        methods.add_method_mut("rotate2", |_, this, angle: f32| {
            *this = this.rotate2(angle);
            Ok(())
        });

        methods.add_method_mut("scale2", |_, this, (x, y)| {
            *this = this.scale2(Vector2::new(x, y));
            Ok(())
        });

        methods.add_method_mut("translate2", |_, this, (x, y): (f32, f32)| {
            *this = this.translate2(Vector2::new(x, y));
            Ok(())
        });

        methods.add_method_mut("isometry2", |_, this, hv_iso: Position2<f32>| {
            *this = this.transform3(&Transform3::from_matrix_unchecked(hv_iso.to_matrix4()));
            Ok(())
        });

        methods.add_method_mut("color", |_, this, color| {
            *this = this.color(color);
            Ok(())
        });
    }
}

/// An object which can be drawn with a mutable borrow. This is strictly more general than
/// [`Drawable`], as it can allow the object being drawn to update itself or deal with cached state.
pub trait DrawableMut {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance);
}

/// An object which can be drawn with an immutable borrow. Not everything can be drawn with an
/// immutable borrow; [`SpriteBatch`] for example will only push a new buffer to the GPU if it is
/// modified, and has to deal with resetting a `dirty` flag and using its internal memory to perform
/// the resulting buffer manipulations.
pub trait Drawable: DrawableMut {
    fn draw(&self, ctx: &mut Graphics, instance: Instance);
}

impl DrawableMut for () {
    fn draw_mut(&mut self, _ctx: &mut Graphics, _instance: Instance) {}
}

impl Drawable for () {
    fn draw(&self, _ctx: &mut Graphics, _instance: Instance) {}
}

impl<T: Drawable + ?Sized> DrawableMut for Arc<T> {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        match Arc::get_mut(self) {
            Some(this) => this.draw_mut(ctx, instance),
            None => T::draw(self, ctx, instance),
        }
    }
}

impl<T: Drawable + ?Sized> Drawable for Arc<T> {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        T::draw(self, ctx, instance)
    }
}

impl<T: DrawableMut + ?Sized> DrawableMut for Mutex<T> {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.get_mut().unwrap().draw_mut(ctx, instance)
    }
}

impl<T: DrawableMut + ?Sized> Drawable for Mutex<T> {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        self.lock().unwrap().draw_mut(ctx, instance)
    }
}

impl<T: DrawableMut + ?Sized> DrawableMut for RwLock<T> {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.get_mut().unwrap().draw_mut(ctx, instance)
    }
}

impl<T: DrawableMut + ?Sized> Drawable for RwLock<T> {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        self.try_write().unwrap().draw_mut(ctx, instance)
    }
}

impl<T: DrawableMut + ?Sized> DrawableMut for Shared<T> {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.borrow_mut().draw_mut(ctx, instance)
    }
}

impl<T: DrawableMut + ?Sized> Drawable for Shared<T> {
    fn draw(&self, ctx: &mut Graphics, instance: Instance) {
        self.borrow_mut().draw_mut(ctx, instance)
    }
}

/// Represents a filter mode for a texture/other image operations.
///
/// If you are doing pixel art, you probably want to be using [`FilterMode::Nearest`]. We currently
/// do not support more complex filtering than [`FilterMode::Linear`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FilterMode {
    Nearest,
    Linear,
}

impl<'lua> ToLua<'lua> for FilterMode {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        lua.to_value(&self)
    }
}

impl<'lua> FromLua<'lua> for FilterMode {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        lua.from_value(lua_value)
    }
}

/// `BlendEquation` represents the different types of equations that can be used to blend colors.
#[derive(Debug, Clone, Copy)]
pub enum BlendEquation {
    /// Represents an equation which adds together the source and destination colors.
    Add,
    /// Represents an equation which subtracts the destination color from the source color.
    Sub,
    /// Represents an equation which subtracts the source color from the destination color.
    ReverseSub,
}

impl From<BlendEquation> for mq::Equation {
    fn from(beq: BlendEquation) -> Self {
        match beq {
            BlendEquation::Add => mq::Equation::Add,
            BlendEquation::Sub => mq::Equation::Subtract,
            BlendEquation::ReverseSub => mq::Equation::ReverseSubtract,
        }
    }
}

/// `BlendFactor` represents the different factors that can be used when blending two colors.
#[derive(Debug, Clone, Copy)]
pub enum BlendFactor {
    /// Multiply the parameter by zero (ignore).
    Zero,
    /// Multiply the parameter by one (no-op).
    One,
    /// Component-wise multiply the parameter by the source color.
    SourceColor,
    /// Multiply the parameter by the source alpha component.
    SourceAlpha,
    /// Component-wise multiply the parameter by the destination color.
    DestinationColor,
    /// Multiply the parameter by the destination alpha component.
    DestinationAlpha,
    /// Component-wise multiply the parameter by `1 - <source color component>`.
    OneMinusSourceColor,
    /// Multiply the parameter by `1 - <source alpha component>`.
    OneMinusSourceAlpha,
    /// Component-wise multiply the parameter by `1 - <destination color component>`.
    OneMinusDestinationColor,
    /// Multiply the parameter by `1 - <destination alpha component>`.
    OneMinusDestinationAlpha,
    /// Honestly no idea what this does, but miniquad/OpenGL support it. I'll have to look it up
    /// sometime.
    SourceAlphaSaturate,
}

impl From<BlendFactor> for mq::BlendFactor {
    fn from(bf: BlendFactor) -> Self {
        use {
            mq::{BlendFactor as MqBf, BlendValue as MqBv},
            BlendFactor::*,
        };

        match bf {
            Zero => MqBf::Zero,
            One => MqBf::One,
            SourceColor => MqBf::Value(MqBv::SourceColor),
            SourceAlpha => MqBf::Value(MqBv::SourceAlpha),
            DestinationColor => MqBf::Value(MqBv::DestinationColor),
            DestinationAlpha => MqBf::Value(MqBv::DestinationAlpha),
            OneMinusSourceColor => MqBf::OneMinusValue(MqBv::SourceColor),
            OneMinusSourceAlpha => MqBf::OneMinusValue(MqBv::SourceAlpha),
            OneMinusDestinationColor => MqBf::OneMinusValue(MqBv::DestinationColor),
            OneMinusDestinationAlpha => MqBf::OneMinusValue(MqBv::DestinationAlpha),
            SourceAlphaSaturate => MqBf::SourceAlphaSaturate,
        }
    }
}

/// `BlendMode` represents a formula used when blending a "source" color with a "destination" color,
/// for example when drawing onto a buffer which already has color data in it.
#[derive(Debug, Copy, Clone)]
pub struct BlendMode {
    eq: BlendEquation,
    src: BlendFactor,
    dst: BlendFactor,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::new(
            BlendEquation::Add,
            BlendFactor::SourceAlpha,
            BlendFactor::OneMinusSourceAlpha,
        )
    }
}

impl BlendMode {
    /// Create a new blend mode from an equation, source factor, and destination factor. The blend
    /// equation that this represents looks like this:
    ///
    /// ```text
    /// match eq {
    ///     BlendEquation::Add => src * source_color + dst * destination_color,
    ///     BlendEquation::Sub => src * source_color - dst * destination_color,
    ///     BlendEquation::ReverseSub => dst * destination_color - src * source_color,
    /// }
    /// ```
    /// 
    /// The default blend mode is `BlendMode::new(BlendEquation::Add, BlendFactor::SourceAlpha,
    /// BlendFactor::OneMinusSourceAlpha)`.
    pub fn new(eq: BlendEquation, src: BlendFactor, dst: BlendFactor) -> Self {
        Self { eq, src, dst }
    }
}

impl From<BlendMode> for mq::BlendState {
    fn from(bm: BlendMode) -> Self {
        mq::BlendState::new(bm.eq.into(), bm.src.into(), bm.dst.into())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ClearOptions {
    pub color: Option<Color>,
    pub depth: Option<f32>,
    pub stencil: Option<i32>,
}

impl Default for ClearOptions {
    fn default() -> Self {
        Self {
            color: Some(Color::ZEROS),
            depth: Some(1.),
            stencil: None,
        }
    }
}

impl LuaUserData for ClearOptions {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("color", |_lua, this| Ok(this.color));
        fields.add_field_method_get("depth", |_lua, this| Ok(this.depth));
        fields.add_field_method_get("stencil", |_lua, this| Ok(this.stencil));

        fields.add_field_method_set("color", |_lua, this, color| {
            this.color = color;
            Ok(())
        });

        fields.add_field_method_set("depth", |_lua, this, depth| {
            this.depth = depth;
            Ok(())
        });

        fields.add_field_method_set("stencil", |_lua, this, stencil| {
            this.stencil = stencil;
            Ok(())
        });
    }
}

pub struct GraphicsState {
    default_pipeline: mq::Pipeline,
    pub null_texture: CachedTexture,
    projection: Matrix4<f32>,
    modelview: TransformStack,
    modelview_dirty: bool,
    quad_bindings: mq::Bindings,
    render_passes: RenderPassRegistry,
    shaders: ShaderRegistry,
    pipelines: PipelineRegistry,
    pipeline_stack: Vec<Option<Pipeline>>,
}

impl GraphicsState {
    fn new(mq: &mut mq::Context) -> Result<Self> {
        let shader = mq::Shader::new(
            mq,
            basic::BASIC_VERTEX,
            basic::BASIC_FRAGMENT,
            basic::meta(),
        )?;

        let pipeline = mq::Pipeline::with_params(
            mq,
            &[
                mq::BufferLayout::default(),
                mq::BufferLayout {
                    step_func: mq::VertexStep::PerInstance,
                    ..mq::BufferLayout::default()
                },
            ],
            &[
                mq::VertexAttribute::with_buffer("a_Pos", mq::VertexFormat::Float3, 0),
                mq::VertexAttribute::with_buffer("a_Uv", mq::VertexFormat::Float2, 0),
                mq::VertexAttribute::with_buffer("a_VertColor", mq::VertexFormat::Float4, 0),
                mq::VertexAttribute::with_buffer("a_Src", mq::VertexFormat::Float4, 1),
                mq::VertexAttribute::with_buffer("a_Tx", mq::VertexFormat::Mat4, 1),
                mq::VertexAttribute::with_buffer("a_Color", mq::VertexFormat::Float4, 1),
            ],
            shader,
            mq::PipelineParams {
                color_blend: Some(BlendMode::default().into()),
                depth_test: mq::Comparison::LessOrEqual,
                depth_write: true,
                ..mq::PipelineParams::default()
            },
        );

        let mut null_texture =
            CachedTexture::from(mq::Texture::from_rgba8(mq, 1, 1, &[0xFF, 0xFF, 0xFF, 0xFF]));

        let quad_vertices =
            mq::Buffer::immutable(mq, mq::BufferType::VertexBuffer, &quad_vertices());
        let quad_indices = mq::Buffer::immutable(mq, mq::BufferType::IndexBuffer, &quad_indices());

        let instances = mq::Buffer::stream(
            mq,
            mq::BufferType::VertexBuffer,
            mem::size_of::<InstanceProperties>(),
        );

        let quad_bindings = mq::Bindings {
            vertex_buffers: vec![quad_vertices, instances],
            index_buffer: quad_indices,
            images: vec![null_texture.get_cached().handle],
        };

        Ok(Self {
            default_pipeline: pipeline,
            null_texture,
            projection: Matrix4::identity(),
            modelview: TransformStack::new(),
            modelview_dirty: true,
            quad_bindings,
            render_passes: RenderPassRegistry::new(),
            shaders: ShaderRegistry::new(),
            pipelines: PipelineRegistry::new(),
            pipeline_stack: Vec::new(),
        })
    }
}

pub struct GraphicsLock {
    engine_ref: EngineRef,
    state: GraphicsState,
}

impl GraphicsLock {
    pub fn new(engine: &Engine) -> Result<Shared<Self>> {
        let engine_ref = engine.downgrade();
        let this = Self {
            engine_ref,
            state: GraphicsState::new(&mut engine.mq())?,
        };
        Ok(engine.insert(this))
    }
}

impl LuaUserData for GraphicsLock {}

impl LuaResource for GraphicsLock {
    const REGISTRY_KEY: &'static str = "HV_FRIENDS_GRAPHICS_LOCK";
}

pub trait GraphicsLockExt {
    fn lock(&self) -> Graphics;
}

impl GraphicsLockExt for Shared<GraphicsLock> {
    fn lock(&self) -> Graphics {
        // First, we lock the resource, and then extract its dereferenced location into a pointer.
        let mut write_guard = self.borrow_mut();
        let mut write_nonnull =
            unsafe { NonNull::new_unchecked(&mut *write_guard as *mut GraphicsLock) };

        // Second, we convert the pointer into a mutable borrow which allows the write guard to be
        // moved, rather than a mutable borrow which is still borrowing the write guard. We give
        // this borrow the same lifetime as the returned guard/the immutable borrow of the resource
        // itself.
        let write_borrow_mut = unsafe { write_nonnull.as_mut() };

        // Third, borrow the `GraphicsCtx` behind the write guard once and then immediately release
        // it after upgrading the engine reference. We *cannot* hold this borrow without violating
        // Rust's aliasing rules, as because in step four...
        let strong_owner = write_borrow_mut.engine_ref.upgrade();
        let guard = unsafe {
            std::mem::transmute::<MutexGuard<mq::Context>, MutexGuard<mq::Context>>(
                strong_owner.mq(),
            )
        };

        // Fourth, last but not least, we return the mutex guard we extract from the engine ref,
        // lengthen its lifetime to be the same as the rest of the lock, and then once again mutably
        // borrow the data behind the write guard; this is the only active borrow in existence to
        // that data, therefore not violating Rust's aliasing rules.
        //
        // The guard struct's field ordering ensures that the guards inside are dropped in the
        // correct order, as well; first the mutable borrow of the `GraphicsState` is released, then
        // the mutex guard on the `mq` object, then the strong reference to the engine object, and
        // finally the write guard of the original resource. The original resource's strong
        // reference is still guaranteed to be around because the lock's original construction takes
        // out an immutable borrow on it.
        Graphics {
            state: &mut write_borrow_mut.state,
            mq: guard,
            _strong_owner: strong_owner,
            _write_guard: write_guard,
        }
    }
}

// Note that the field ordering of this struct is VERY IMPORTANT! These fields MUST be dropped in
// order or else we risk UB!
pub struct Graphics<'a> {
    pub state: &'a mut GraphicsState,
    pub mq: MutexGuard<'a, mq::Context>,
    _write_guard: RefMut<'a, GraphicsLock>,
    _strong_owner: Engine<'a>,
}

impl<'a> Graphics<'a> {
    #[inline]
    pub fn mq(&self) -> &mq::Context {
        &*self.mq
    }

    #[inline]
    pub fn mq_mut(&mut self) -> &mut mq::Context {
        &mut *self.mq
    }

    #[inline]
    pub fn modelview(&self) -> &TransformStack {
        &self.state.modelview
    }

    #[inline]
    pub fn modelview_mut(&mut self) -> &mut TransformStack {
        self.state.modelview_dirty = true;
        &mut self.state.modelview
    }

    #[inline]
    pub fn apply_modelview(&mut self) {
        if self.state.modelview_dirty {
            let mvp = self.state.projection * self.state.modelview.top();
            self.mq.apply_uniforms(&basic::Uniforms { mvp });
            self.state.modelview_dirty = false;
        }
    }

    #[inline]
    pub fn set_projection<M>(&mut self, projection: M)
    where
        M: Into<Matrix4<f32>>,
    {
        self.state.projection = projection.into();
    }

    #[inline]
    pub fn push_pipeline(&mut self) {
        let top = self.state.pipeline_stack.last().and_then(|x| x.clone());
        self.state.pipeline_stack.push(top);
    }

    #[inline]
    pub fn apply_default_pipeline(&mut self) {
        self.mq.apply_pipeline(&self.state.default_pipeline);
    }

    #[inline]
    pub fn apply_pipeline(&mut self, pipeline: &Pipeline) {
        self.mq.apply_pipeline(&pipeline.handle);
    }

    #[inline]
    pub fn pop_pipeline(&mut self) {
        let top = self.state.pipeline_stack.pop().and_then(|x| x);

        match top {
            Some(pipeline) => self.apply_pipeline(&pipeline),
            None => self.apply_default_pipeline(),
        }
    }

    #[inline]
    pub fn apply_bindings(&mut self, bindings: &mut Bindings) {
        self.mq.apply_bindings(bindings.update());
    }

    #[inline]
    pub fn begin_render_pass(
        &mut self,
        pass: Option<&RenderPass>,
        clear_options: Option<ClearOptions>,
    ) {
        self.mq.begin_pass(
            pass.map(|rp| rp.handle),
            match clear_options {
                None => PassAction::Nothing,
                Some(options) => PassAction::Clear {
                    color: options.color.map(|c| (c.r, c.g, c.b, c.a)),
                    depth: options.depth,
                    stencil: options.stencil,
                },
            },
        );
    }

    #[inline]
    pub fn end_render_pass(&mut self) {
        self.mq.end_render_pass();
    }

    #[inline]
    pub fn commit_frame(&mut self) {
        self.mq.commit_frame();
    }

    #[inline]
    pub fn clear(&mut self, options: ClearOptions) {
        self.mq.clear(
            options.color.map(|c| (c.r, c.g, c.b, c.a)),
            options.depth,
            options.stencil,
        );
    }

    #[inline]
    pub fn draw(&mut self, drawable: &impl Drawable, params: impl Into<Option<Instance>>) {
        drawable.draw(self, params.into().unwrap_or_default());
    }
}

pub(crate) fn open<'lua>(lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>> {
    let gfx_lock = GraphicsLock::new(engine)?;
    lua.insert_resource(gfx_lock.clone())?;

    let texture_cache = engine.insert(TextureCache::new(engine, &gfx_lock));
    lua.insert_resource(texture_cache.clone())?;

    let clone = texture_cache.clone();
    let load_texture_from_filesystem = lua.create_function(move |_, path: LuaString| {
        let cache = &mut clone.borrow_mut();
        cache.get_or_load(path.to_str()?).to_lua_err()
    })?;

    let sprite_sheet_cache = engine.insert(SpriteSheetCache::new(engine));
    lua.insert_resource(sprite_sheet_cache.clone())?;

    let clone = sprite_sheet_cache.clone();
    let load_sprite_sheet_from_filesystem = lua.create_function(move |_, path: LuaString| {
        let cache = &mut clone.borrow_mut();
        cache.get_or_load(path.to_str()?).to_lua_err()
    })?;

    let reload_textures =
        lua.create_function(move |_, ()| texture_cache.borrow_mut().reload_all().to_lua_err())?;

    let reload_sprite_sheets = lua
        .create_function(move |_, ()| sprite_sheet_cache.borrow_mut().reload_all().to_lua_err())?;

    let create_instance_object = lua.create_function(move |_, ()| Ok(Instance::new()))?;

    let gfx = gfx_lock.clone();
    let create_sprite_batch_object = lua.create_function(
        move |_, (texture, maybe_capacity): (CachedTexture, Option<usize>)| match maybe_capacity {
            Some(capacity) => Ok(SpriteBatch::with_capacity(
                &mut gfx.lock(),
                texture,
                capacity,
            )),
            None => Ok(SpriteBatch::new(&mut gfx.lock(), texture)),
        },
    )?;

    let sprite_animation_state =
        |_, (mut sprite_sheet, tag, should_loop): (CachedSpriteSheet, LuaString, Option<bool>)| {
            let sheet = sprite_sheet.get_cached();
            let tag_id = sheet
                .get_tag(tag.to_str()?)
                .ok_or_else(|| anyhow!("no such tag"))
                .to_lua_err()?;
            let (frame, tag) = sheet.at_tag(tag_id, should_loop.unwrap_or(true));

            Ok(SpriteAnimationState {
                sheet: sprite_sheet,
                frame,
                tag,
            })
        };

    let create_sprite_animation_state_object = lua.create_function(sprite_animation_state)?;
    let create_sprite_animation_state_component_constructor =
        lua.create_function(move |lua, (sprite_sheet, tag, should_loop)| {
            Ok(DynamicComponentConstructor::clone(sprite_animation_state(
                lua,
                (sprite_sheet, tag, should_loop),
            )?))
        })?;

    let gfx = gfx_lock.clone();
    let apply_modelview = lua.create_function(move |_, ()| {
        gfx.lock().apply_modelview();
        Ok(())
    })?;

    let gfx = gfx_lock.clone();
    let apply_default_pipeline = lua.create_function(move |_, ()| {
        gfx.lock().apply_default_pipeline();
        Ok(())
    })?;

    let gfx = gfx_lock.clone();
    let apply_pipeline = lua.create_function(move |_, pipeline: Pipeline| {
        gfx.lock().apply_pipeline(&pipeline);
        Ok(())
    })?;

    let gfx = gfx_lock.clone();
    let begin_render_pass = lua.create_function(
        move |_, (pass, clear_options): (Option<RenderPass>, Option<ClearOptions>)| {
            gfx.lock().begin_render_pass(pass.as_ref(), clear_options);
            Ok(())
        },
    )?;

    let gfx = gfx_lock.clone();
    let end_render_pass = lua.create_function(move |_, ()| {
        gfx.lock().end_render_pass();
        Ok(())
    })?;

    let bindings = crate::graphics::bindings::open(lua, &gfx_lock)?;
    let buffer = crate::graphics::buffer::open(lua, &gfx_lock)?;
    let pipeline = crate::graphics::pipeline::open(lua, &gfx_lock)?;

    let lgs = LuaGraphicsState::new(&mut gfx_lock.lock());

    let circle = lua.create_function(self::lua::circle(lgs.clone(), gfx_lock.clone()))?;
    let line = lua.create_function(self::lua::line(lgs.clone(), gfx_lock.clone()))?;
    let points = lua.create_function(self::lua::points(lgs.clone(), gfx_lock.clone()))?;
    let polygon = lua.create_function(self::lua::polygon(lgs.clone(), gfx_lock.clone()))?;
    let print = lua.create_function(self::lua::print(lgs.clone(), gfx_lock.clone()))?;

    let clear = lua.create_function(self::lua::clear(lgs.clone(), gfx_lock.clone()))?;
    let present = lua.create_function(self::lua::present(gfx_lock.clone()))?;

    let set_color = lua.create_function(self::lua::set_color(lgs))?;

    let apply_transform = lua.create_function(self::lua::apply_transform(gfx_lock.clone()))?;
    let inverse_transform_point =
        lua.create_function(self::lua::inverse_transform_point(gfx_lock.clone()))?;
    let origin = lua.create_function(self::lua::origin(gfx_lock.clone()))?;
    let pop = lua.create_function(self::lua::pop(gfx_lock.clone()))?;
    let push = lua.create_function(self::lua::push(gfx_lock.clone()))?;
    let replace_transform = lua.create_function(self::lua::replace_transform(gfx_lock.clone()))?;
    let rotate = lua.create_function(self::lua::rotate(gfx_lock.clone()))?;
    let scale = lua.create_function(self::lua::scale(gfx_lock.clone()))?;
    let shear = lua.create_function(self::lua::shear(gfx_lock.clone()))?;
    let transform_point = lua.create_function(self::lua::transform_point(gfx_lock.clone()))?;
    let translate = lua.create_function(self::lua::translate(gfx_lock))?;

    let draw_mode_fill = LuaDrawMode::Fill;
    let draw_mode_line = LuaDrawMode::Line;

    Ok(lua
        .load(mlua::chunk! {
            {
                load_sprite_sheet_from_filesystem = $load_sprite_sheet_from_filesystem,
                load_texture_from_filesystem = $load_texture_from_filesystem,
                reload_textures = $reload_textures,
                reload_sprite_sheets = $reload_sprite_sheets,

                create_instance_object = $create_instance_object,
                create_sprite_batch_object = $create_sprite_batch_object,
                create_sprite_animation_state_object = $create_sprite_animation_state_object,
                create_sprite_animation_state_component_constructor = $create_sprite_animation_state_component_constructor,

                apply_modelview = $apply_modelview,

                apply_default_pipeline = $apply_default_pipeline,
                apply_pipeline = $apply_pipeline,
                begin_render_pass = $begin_render_pass,
                end_render_pass = $end_render_pass,

                bindings = $bindings,
                buffer = $buffer,
                pipeline = $pipeline,
                
                circle = $circle,
                line = $line,
                points = $points,
                polygon = $polygon,
                print = $print,

                clear = $clear,
                present = $present,

                set_color = $set_color,

                apply_transform = $apply_transform,
                inverse_transform_point = $inverse_transform_point,
                origin = $origin,
                pop = $pop,
                push = $push,
                replace_transform = $replace_transform,
                rotate = $rotate,
                scale = $scale,
                shear = $shear,
                transform_point = $transform_point,
                translate = $translate,

                DrawMode = {
                    Fill = $draw_mode_fill,
                    Line = $draw_mode_line,
                },
            }
        })
        .eval()?)
}
