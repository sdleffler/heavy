/*
Much of the code in this file is drawn from the ggez project and then heavily modified. As such, here is the corresponding license notification:

The MIT License (MIT)

Copyright (c) 2016-2017 ggez-dev

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHERDEALINGS IN THE
SOFTWARE.
 */

use anyhow::*;
use hv_core::mq;
use lyon::tessellation::{self as t, FillOptions, StrokeOptions};
use std::mem;

use crate::{
    graphics::{
        CachedTexture, Color, Drawable, DrawableMut, Graphics, Instance, InstanceProperties,
        LinearColor, Vertex,
    },
    math::*,
};

/// Specifies whether a mesh should be drawn
/// filled or as an outline.
#[derive(Debug, Copy, Clone)]
pub enum DrawMode {
    /// A stroked line with given parameters, see `StrokeOptions` documentation.
    Stroke(StrokeOptions),
    /// A filled shape with given parameters, see `FillOptions` documentation.
    Fill(FillOptions),
}

impl DrawMode {
    /// Constructs a DrawMode that draws a stroke with the given width
    pub fn stroke(width: f32) -> DrawMode {
        DrawMode::Stroke(StrokeOptions::default().with_line_width(width))
    }

    /// Constructs a DrawMode that fills shapes with default fill options.
    pub fn fill() -> DrawMode {
        DrawMode::Fill(FillOptions::default())
    }
}

#[derive(Debug, Copy, Clone)]
struct VertexBuilder {
    color: LinearColor,
}

impl t::FillVertexConstructor<Vertex> for VertexBuilder {
    #[inline]
    fn new_vertex(&mut self, vertex: t::FillVertex) -> Vertex {
        let point = vertex.position();
        Vertex {
            pos: Vector3::new(point.x, point.y, 0.),
            uv: Vector2::new(point.x, point.y),
            color: self.color,
        }
    }
}

impl t::StrokeVertexConstructor<Vertex> for VertexBuilder {
    #[inline]
    fn new_vertex(&mut self, vertex: t::StrokeVertex) -> Vertex {
        let point = vertex.position();
        Vertex {
            pos: Vector3::new(point.x, point.y, 0.),
            uv: Vector2::zeros(),
            color: self.color,
        }
    }
}

#[derive(Debug)]
pub struct MeshBuilder {
    pub buffer: t::geometry_builder::VertexBuffers<Vertex, u16>,
    pub texture: CachedTexture,
}

impl MeshBuilder {
    pub fn new<T>(texture: T) -> Self
    where
        T: Into<CachedTexture>,
    {
        Self {
            buffer: t::VertexBuffers::new(),
            texture: texture.into(),
        }
    }

    pub fn clear(&mut self) {
        self.buffer.vertices.clear();
        self.buffer.indices.clear();
    }

    /// Create a new mesh for a line of one or more connected segments.
    pub fn line<P>(&mut self, points: &[P], width: f32, color: Color) -> Result<&mut Self>
    where
        P: Into<mint::Point2<f32>> + Clone,
    {
        self.polyline(DrawMode::stroke(width), points, color)
    }

    /// Create a new mesh for a series of connected lines.
    pub fn polyline<P>(&mut self, mode: DrawMode, points: &[P], color: Color) -> Result<&mut Self>
    where
        P: Into<mint::Point2<f32>> + Clone,
    {
        ensure!(
            points.len() >= 2,
            "MeshBuilder::polyline() got a list of < 2 points"
        );
        self.polyline_inner(mode, points, false, color)
    }

    /// Create a new mesh for a circle.
    ///
    /// For the meaning of the `tolerance` parameter, [see here](https://docs.rs/lyon_geom/0.11.0/lyon_geom/#flattening).
    pub fn circle<P>(
        &mut self,
        mode: DrawMode,
        point: P,
        radius: f32,
        tolerance: f32,
        color: Color,
    ) -> &mut Self
    where
        P: Into<mint::Point2<f32>>,
    {
        {
            let point = point.into();
            let buffers = &mut self.buffer;
            let vb = VertexBuilder {
                color: LinearColor::from(color),
            };
            match mode {
                DrawMode::Fill(fill_options) => {
                    let builder = &mut t::BuffersBuilder::new(buffers, vb);
                    let mut tessellator = t::FillTessellator::new();
                    let _ = tessellator.tessellate_circle(
                        t::math::point(point.x, point.y),
                        radius,
                        &fill_options.with_tolerance(tolerance),
                        builder,
                    );
                }
                DrawMode::Stroke(options) => {
                    let builder = &mut t::BuffersBuilder::new(buffers, vb);
                    let mut tessellator = t::StrokeTessellator::new();
                    let _ = tessellator.tessellate_circle(
                        t::math::point(point.x, point.y),
                        radius,
                        &options.with_tolerance(tolerance),
                        builder,
                    );
                }
            };
        }
        self
    }

    /// Create a new mesh for a closed polygon.
    /// The points given must be in clockwise order,
    /// otherwise at best the polygon will not draw.
    pub fn polygon<P>(&mut self, mode: DrawMode, points: &[P], color: Color) -> Result<&mut Self>
    where
        P: Into<mint::Point2<f32>> + Clone,
    {
        ensure!(
            points.len() >= 3,
            "MeshBuilder::polygon() got a list of < 3 points"
        );

        self.polyline_inner(mode, points, true, color)
    }

    fn polyline_inner<P>(
        &mut self,
        mode: DrawMode,
        points: &[P],
        is_closed: bool,
        color: Color,
    ) -> Result<&mut Self>
    where
        P: Into<mint::Point2<f32>> + Clone,
    {
        {
            assert!(points.len() > 1);
            let buffers = &mut self.buffer;
            let points = points
                .iter()
                .cloned()
                .map(|p| {
                    let mint_point: mint::Point2<f32> = p.into();
                    t::math::point(mint_point.x, mint_point.y)
                })
                .collect::<Vec<_>>();
            let vb = VertexBuilder {
                color: LinearColor::from(color),
            };
            let polygon = lyon::path::Polygon {
                points: &points,
                closed: is_closed,
            };
            match mode {
                DrawMode::Fill(options) => {
                    let builder = &mut t::BuffersBuilder::new(buffers, vb);
                    let tessellator = &mut t::FillTessellator::new();
                    tessellator.tessellate_polygon(polygon, &options, builder)
                }
                DrawMode::Stroke(options) => {
                    let builder = &mut t::BuffersBuilder::new(buffers, vb);
                    let tessellator = &mut t::StrokeTessellator::new();
                    tessellator.tessellate_polygon(polygon, &options, builder)
                }
            }
            .map_err(|e| anyhow!("error during tessellation: {:?}", e))?;
        }
        Ok(self)
    }

    /// Create a new mesh for a rectangle.
    pub fn rectangle(&mut self, mode: DrawMode, bounds: Box2<f32>, color: Color) -> &mut Self {
        {
            let buffers = &mut self.buffer;
            let extents = bounds.extents();
            let rect = t::math::rect(bounds.mins.x, bounds.mins.y, extents.x, extents.y);
            let vb = VertexBuilder {
                color: LinearColor::from(color),
            };
            match mode {
                DrawMode::Fill(fill_options) => {
                    let builder = &mut t::BuffersBuilder::new(buffers, vb);
                    let tessellator = &mut t::FillTessellator::new();
                    let _ = tessellator.tessellate_rectangle(&rect, &fill_options, builder);
                }
                DrawMode::Stroke(options) => {
                    let builder = &mut t::BuffersBuilder::new(buffers, vb);
                    let tessellator = &mut t::StrokeTessellator::new();
                    let _ = tessellator.tessellate_rectangle(&rect, &options, builder);
                }
            };
        }
        self
    }

    /// Creates a `Mesh` from a raw list of triangles defined from vertices
    /// and indices.  You may also
    /// supply an `Image` to use as a texture, if you pass `None`, it will
    /// just use a pure white texture.
    ///
    /// This is the most primitive mesh-creation method, but allows you full
    /// control over the tesselation and texturing.  It has the same constraints
    /// as `Mesh::from_raw()`.
    pub fn raw<V, T>(&mut self, verts: &[V], indices: &[u16], texture: T) -> &mut Self
    where
        V: Into<Vertex> + Clone,
        T: Into<Option<CachedTexture>>,
    {
        assert!(self.buffer.vertices.len() + verts.len() < (std::u16::MAX as usize));
        assert!(self.buffer.indices.len() + indices.len() < (std::u16::MAX as usize));
        let next_idx = self.buffer.vertices.len() as u16;
        // Can we remove the clone here?
        // I can't find a way to, because `into()` consumes its source and
        // `Borrow` or `AsRef` aren't really right.
        let vertices = verts.iter().cloned().map(|v: V| -> Vertex { v.into() });
        let indices = indices.iter().map(|i| (*i) + next_idx);
        self.buffer.vertices.extend(vertices);
        self.buffer.indices.extend(indices);

        if let Some(tex) = texture.into() {
            self.texture = tex;
        }

        self
    }

    pub fn update(&self, ctx: &mut Graphics, mesh: &mut Mesh) {
        let vertex_buffer = mq::Buffer::immutable(
            &mut ctx.mq,
            mq::BufferType::VertexBuffer,
            &self.buffer.vertices,
        );

        let index_buffer = mq::Buffer::immutable(
            &mut ctx.mq,
            mq::BufferType::IndexBuffer,
            &self.buffer.indices,
        );

        let instance = mq::Buffer::stream(
            &mut ctx.mq,
            mq::BufferType::VertexBuffer,
            mem::size_of::<InstanceProperties>(),
        );

        let aabb = if self.buffer.vertices.is_empty() {
            Box2::invalid()
        } else {
            Box2::from_points(
                &self
                    .buffer
                    .vertices
                    .iter()
                    .map(|v| Point2::from(v.pos.xy()))
                    .collect::<Vec<_>>(),
            )
        };

        mesh.texture = self.texture.clone();

        mesh.bindings.vertex_buffers.clear();
        mesh.bindings.vertex_buffers.push(vertex_buffer);
        mesh.bindings.vertex_buffers.push(instance);

        mesh.bindings.index_buffer = index_buffer;

        mesh.bindings.images.clear();
        mesh.bindings.images.push(mesh.texture.get_cached().handle);

        mesh.len = self.buffer.indices.len() as i32;
        mesh.aabb = aabb;
    }

    pub fn build(&self, ctx: &mut Graphics) -> Mesh {
        let vertex_buffer = mq::Buffer::immutable(
            &mut ctx.mq,
            mq::BufferType::VertexBuffer,
            &self.buffer.vertices,
        );

        let index_buffer = mq::Buffer::immutable(
            &mut ctx.mq,
            mq::BufferType::IndexBuffer,
            &self.buffer.indices,
        );

        let instance = mq::Buffer::stream(
            &mut ctx.mq,
            mq::BufferType::VertexBuffer,
            mem::size_of::<InstanceProperties>(),
        );

        let aabb = if self.buffer.vertices.is_empty() {
            Box2::invalid()
        } else {
            Box2::from_points(
                &self
                    .buffer
                    .vertices
                    .iter()
                    .map(|v| Point2::from(v.pos.xy()))
                    .collect::<Vec<_>>(),
            )
        };

        Mesh {
            texture: self.texture.clone(),
            bindings: mq::Bindings {
                vertex_buffers: vec![vertex_buffer, instance],
                index_buffer,
                images: vec![self.texture.get().handle],
            },
            len: self.buffer.indices.len() as i32,
            aabb,
        }
    }
}

#[derive(Debug)]
pub struct Mesh {
    /// The shared reference to the texture, so that it doesn't get dropped and deleted.
    /// The inner data is already in `bindings` so this is really just to keep it from
    /// being dropped.
    pub texture: CachedTexture,
    pub bindings: mq::Bindings,
    pub len: i32,
    pub aabb: Box2<f32>,
}

impl DrawableMut for Mesh {
    fn draw_mut(&mut self, ctx: &mut Graphics, instance: Instance) {
        self.draw(ctx, instance);
    }
}

impl Drawable for Mesh {
    fn draw(&self, ctx: &mut Graphics, param: Instance) {
        self.bindings.vertex_buffers[1].update(&mut ctx.mq, &[param.to_instance_properties()]);
        ctx.apply_modelview();
        ctx.mq.apply_bindings(&self.bindings);
        ctx.mq.draw(0, self.len, 1);
    }
}
