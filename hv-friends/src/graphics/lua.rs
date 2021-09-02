use std::sync::Arc;

use hv_core::{engine::WeakResourceCache, prelude::*};

use crate::{
    graphics::{
        text::{CachedFontAtlas, CharacterListType, FontAtlas, Text, TextLayout},
        CachedTexture, ClearOptions, Color, DrawMode, DrawableMut, Graphics, GraphicsLock,
        GraphicsLockExt, Instance, Mesh, MeshBuilder, Vertex,
    },
    math::*,
};

macro_rules! lua_fn {
    (Fn<$lua:lifetime>($args:ty) -> $ret:ty) => { impl 'static + for<$lua> Fn(&$lua Lua, $args) -> LuaResult<$ret> };
    (FnMut<$lua:lifetime>($args:ty) -> $ret:ty) => { impl 'static + for<$lua> FnMut(&$lua Lua, $args) -> LuaResult<$ret> };
    (Fn<$lua:lifetime>($this:ty, $args:ty) -> $ret:ty) => { impl 'static + for<$lua> Fn(&$lua Lua, $this, $args) -> LuaResult<$ret> };
    (FnMut<$lua:lifetime>($this:ty, $args:ty) -> $ret:ty) => { impl 'static + for<$lua> FnMut(&$lua Lua, $this, $args) -> LuaResult<$ret> }
}

impl LuaUserData for DrawMode {}

#[derive(Debug, Clone)]
pub struct PointBuffer(pub Arc<Vec<Point2<f32>>>);

impl LuaUserData for PointBuffer {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        crate::lua::simple_mut(methods, "push", |this, (x, y)| {
            Arc::make_mut(&mut this.0).push(Point2::new(x, y))
        });
    }
}

#[derive(Debug, Clone)]
pub struct VertexBuffer(pub Arc<Vec<Vertex>>);

impl LuaUserData for VertexBuffer {}

#[derive(Debug, Clone)]
pub struct IndexBuffer(pub Arc<Vec<u16>>);

impl LuaUserData for IndexBuffer {}

impl LuaUserData for MeshBuilder {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut(
            "line",
            |_, this, (points, width, color): (PointBuffer, f32, Color)| {
                this.line(&points.0, width, color).to_lua_err()?;
                Ok(())
            },
        );

        methods.add_method_mut(
            "polyline",
            |_, this, (draw_mode, points, color): (DrawMode, PointBuffer, Color)| {
                this.polyline(draw_mode, &points.0, color).to_lua_err()?;
                Ok(())
            },
        );

        methods.add_method_mut(
            "circle",
            |_, this, (draw_mode, x, y, radius, tolerance, color)| {
                this.circle(draw_mode, Point2::new(x, y), radius, tolerance, color);
                Ok(())
            },
        );

        methods.add_method_mut(
            "polygon",
            |_, this, (draw_mode, points, color): (DrawMode, PointBuffer, Color)| {
                this.polygon(draw_mode, &points.0, color).to_lua_err()?;
                Ok(())
            },
        );

        methods.add_method_mut("rectangle", |_, this, (draw_mode, x, y, w, h, color)| {
            this.rectangle(draw_mode, Box2::new(x, y, w, h), color);
            Ok(())
        });

        methods.add_method_mut(
            "raw",
            |_, this, (vertices, indices, texture): (VertexBuffer, IndexBuffer, Option<CachedTexture>)| {
                this.raw(&vertices.0, &indices.0, texture);
                Ok(())
            },
        );

        let mut weak_resource_cache = WeakResourceCache::<GraphicsLock>::new();
        methods.add_method_mut("build", move |lua, this, ()| {
            let gfx_lock = weak_resource_cache.get(|| lua.get_resource::<GraphicsLock>())?;
            let mesh = this.build(&mut gfx_lock.lock());
            Ok(mesh)
        });
    }
}

impl LuaUserData for Mesh {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        crate::lua::add_drawable_methods(methods);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LuaDrawMode {
    Fill,
    Line,
}

impl<'lua> ToLua<'lua> for LuaDrawMode {
    #[allow(clippy::zero_ptr)]
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        match self {
            Self::Fill => LuaLightUserData(0 as *mut _).to_lua(lua),
            Self::Line => LuaLightUserData(1 as *mut _).to_lua(lua),
        }
    }
}

impl<'lua> FromLua<'lua> for LuaDrawMode {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let i = LuaLightUserData::from_lua(lua_value, lua)?.0 as u64;
        match i {
            0 => Ok(Self::Fill),
            1 => Ok(Self::Line),
            _ => Err(anyhow!("invalid draw mode!")).to_lua_err(),
        }
    }
}

pub(crate) struct LuaGraphicsState {
    line_width: f32,
    point_size: f32,
    color: Color,
    bg_color: Color,
    mesh_builder: MeshBuilder,
    mesh: Option<Mesh>,
    // font: CachedFontAtlas,
    text_layout: TextLayout,
    text: Text,
}

impl LuaGraphicsState {
    pub fn new(gfx: &mut Graphics) -> Shared<Self> {
        let font = CachedFontAtlas::new_uncached(
            FontAtlas::from_reader(
                gfx,
                std::io::Cursor::new(include_bytes!("../../resources/default_font.ttf")),
                20.,
                CharacterListType::Ascii,
            )
            .expect("error loading default font"),
        );
        let text_layout = TextLayout::new(font);
        let text = Text::new(gfx);

        Shared::new(Self {
            line_width: 1.,
            point_size: 1.,
            color: Color::WHITE,
            bg_color: Color::ZEROS,
            mesh_builder: MeshBuilder::new(gfx.state.null_texture.clone()),
            mesh: None,
            // font,
            text_layout,
            text,
        })
    }

    pub fn circle(
        &mut self,
        gfx: &mut Graphics,
        lua_draw_mode: LuaDrawMode,
        point: Point2<f32>,
        radius: f32,
    ) -> Result<()> {
        let mode = match lua_draw_mode {
            LuaDrawMode::Fill => DrawMode::fill(),
            LuaDrawMode::Line => DrawMode::stroke(self.line_width),
        };

        self.mesh_builder
            .circle(mode, point, radius, 0.1, self.color);

        let mesh = match &mut self.mesh {
            Some(mesh) => {
                self.mesh_builder.update(gfx, mesh);
                mesh
            }
            None => self.mesh.insert(self.mesh_builder.build(gfx)),
        };

        self.mesh_builder.clear();
        mesh.draw_mut(gfx, Instance::new());

        Ok(())
    }

    pub fn line(&mut self, gfx: &mut Graphics, points: &[Point2<f32>]) -> Result<()> {
        self.mesh_builder
            .line(points, self.line_width, self.color)?;

        let mesh = match &mut self.mesh {
            Some(mesh) => {
                self.mesh_builder.update(gfx, mesh);
                mesh
            }
            None => self.mesh.insert(self.mesh_builder.build(gfx)),
        };

        self.mesh_builder.clear();
        mesh.draw_mut(gfx, Instance::new());

        Ok(())
    }

    pub fn points(&mut self, gfx: &mut Graphics, points: &[Point2<f32>]) -> Result<()> {
        for point in points {
            self.mesh_builder.rectangle(
                DrawMode::fill(),
                Box2::from_half_extents(*point, Vector2::repeat(self.point_size / 2.)),
                self.color,
            );
        }

        let mesh = match &mut self.mesh {
            Some(mesh) => {
                self.mesh_builder.update(gfx, mesh);
                mesh
            }
            None => self.mesh.insert(self.mesh_builder.build(gfx)),
        };

        self.mesh_builder.clear();
        mesh.draw_mut(gfx, Instance::new());

        Ok(())
    }

    pub fn polygon(
        &mut self,
        gfx: &mut Graphics,
        lua_draw_mode: LuaDrawMode,
        points: &[Point2<f32>],
    ) -> Result<()> {
        let mode = match lua_draw_mode {
            LuaDrawMode::Fill => DrawMode::fill(),
            LuaDrawMode::Line => DrawMode::stroke(self.line_width),
        };

        self.mesh_builder.polygon(mode, points, self.color)?;

        let mesh = match &mut self.mesh {
            Some(mesh) => {
                self.mesh_builder.update(gfx, mesh);
                mesh
            }
            None => self.mesh.insert(self.mesh_builder.build(gfx)),
        };

        self.mesh_builder.clear();
        mesh.draw_mut(gfx, Instance::new());

        Ok(())
    }

    pub fn print(&mut self, gfx: &mut Graphics, text: &str, instance: Instance) -> Result<()> {
        self.text_layout.clear();
        self.text_layout
            .push_str(text, std::iter::repeat(Color::WHITE));
        self.text.apply_layout(&mut self.text_layout);
        self.text.draw_mut(gfx, instance);

        Ok(())
    }
}

pub(crate) fn circle(
    lgs: Shared<LuaGraphicsState>,
    gfx_lock: Shared<GraphicsLock>,
) -> lua_fn!(Fn<'lua>((LuaDrawMode, f32, f32, f32)) -> ()) {
    move |_, (mode, x, y, radius)| {
        lgs.borrow_mut()
            .circle(&mut gfx_lock.lock(), mode, Point2::new(x, y), radius)
            .to_lua_err()
    }
}

pub(crate) fn line(
    lgs: Shared<LuaGraphicsState>,
    gfx_lock: Shared<GraphicsLock>,
) -> lua_fn!(Fn<'lua>(PointBuffer) -> ()) {
    move |_, point_buffer| {
        lgs.borrow_mut()
            .line(&mut gfx_lock.lock(), &point_buffer.0)
            .to_lua_err()
    }
}

pub(crate) fn points(
    lgs: Shared<LuaGraphicsState>,
    gfx_lock: Shared<GraphicsLock>,
) -> lua_fn!(Fn<'lua>(PointBuffer) -> ()) {
    move |_, point_buffer| {
        lgs.borrow_mut()
            .points(&mut gfx_lock.lock(), &point_buffer.0)
            .to_lua_err()
    }
}

pub(crate) fn polygon(
    lgs: Shared<LuaGraphicsState>,
    gfx_lock: Shared<GraphicsLock>,
) -> lua_fn!(Fn<'lua>((LuaDrawMode, PointBuffer)) -> ()) {
    move |_, (mode, point_buffer)| {
        lgs.borrow_mut()
            .polygon(&mut gfx_lock.lock(), mode, &point_buffer.0)
            .to_lua_err()
    }
}

pub(crate) fn print(
    lgs: Shared<LuaGraphicsState>,
    gfx_lock: Shared<GraphicsLock>,
) -> lua_fn!(Fn<'lua>((LuaString<'lua>, LuaVariadic<f32>)) -> ()) {
    move |_, (text, params): (LuaString, LuaVariadic<f32>)| {
        let mut ps = params.into_iter();
        let x = ps.next().unwrap_or(0.);
        let y = ps.next().unwrap_or(0.);
        let r = ps.next().unwrap_or(0.);
        let sx = ps.next().unwrap_or(1.);
        let sy = ps.next().unwrap_or(sx);
        let ox = ps.next().unwrap_or(0.);
        let oy = ps.next().unwrap_or(0.);

        let mut lgs_mut = lgs.borrow_mut();
        let instance = Instance::new()
            .color(lgs_mut.color)
            .translate2(Vector2::new(x, y))
            .scale2(Vector2::new(sx, sy))
            .rotate2(r)
            .translate2(Vector2::new(ox, oy));

        lgs_mut
            .print(&mut gfx_lock.lock(), text.to_str()?, instance)
            .to_lua_err()?;

        Ok(())
    }
}

pub(crate) fn clear(
    lgs: Shared<LuaGraphicsState>,
    gfx_lock: Shared<GraphicsLock>,
) -> lua_fn!(Fn<'lua>(LuaMultiValue<'lua>) -> ()) {
    move |lua, values: LuaMultiValue| {
        if values.is_empty() {
            gfx_lock.lock().clear(ClearOptions {
                color: Some(lgs.borrow().bg_color),
                depth: Some(1.),
                stencil: None,
            });
        } else {
            let (r, g, b, maybe_a, maybe_stencil, maybe_depth): (_, _, _, Option<f32>, _, _) =
                FromLuaMulti::from_lua_multi(values, lua)?;
            gfx_lock.lock().clear(ClearOptions {
                color: Some(Color::new(r, g, b, maybe_a.unwrap_or(1.))),
                depth: maybe_depth,
                stencil: maybe_stencil,
            });
        }

        Ok(())
    }
}

pub(crate) fn present(gfx_lock: Shared<GraphicsLock>) -> lua_fn!(Fn<'lua>(()) -> ()) {
    move |_, ()| {
        gfx_lock.lock().commit_frame();
        Ok(())
    }
}

pub(crate) fn set_color(
    lgs: Shared<LuaGraphicsState>,
) -> lua_fn!(Fn<'lua>((f32, f32, f32, Option<f32>)) -> ()) {
    move |_, (r, g, b, maybe_a)| {
        lgs.borrow_mut().color = Color::new(r, g, b, maybe_a.unwrap_or(1.));
        Ok(())
    }
}

pub(crate) fn apply_transform(gfx_lock: Shared<GraphicsLock>) -> lua_fn!(Fn<'lua>(Tx<f32>) -> ()) {
    move |_, tx| {
        gfx_lock
            .lock()
            .modelview_mut()
            .apply_transform(tx.to_homogeneous_mat4());
        Ok(())
    }
}

pub(crate) fn inverse_transform_point(
    gfx_lock: Shared<GraphicsLock>,
) -> lua_fn!(Fn<'lua>((f32, f32)) -> (f32, f32)) {
    move |_, (x, y)| {
        let out = gfx_lock
            .lock()
            .modelview()
            .inverse_transform_point2(Point2::new(x, y));
        Ok((out.x, out.y))
    }
}

pub(crate) fn origin(gfx_lock: Shared<GraphicsLock>) -> lua_fn!(Fn<'lua>(()) -> ()) {
    move |_, ()| {
        gfx_lock.lock().modelview_mut().origin();
        Ok(())
    }
}

pub(crate) fn pop(gfx_lock: Shared<GraphicsLock>) -> lua_fn!(Fn<'lua>(()) -> ()) {
    move |_, ()| {
        gfx_lock.lock().modelview_mut().pop();
        Ok(())
    }
}

pub(crate) fn push(gfx_lock: Shared<GraphicsLock>) -> lua_fn!(Fn<'lua>(()) -> ()) {
    move |_, ()| {
        gfx_lock.lock().modelview_mut().push(None);
        Ok(())
    }
}

pub(crate) fn replace_transform(
    gfx_lock: Shared<GraphicsLock>,
) -> lua_fn!(Fn<'lua>(Tx<f32>) -> ()) {
    move |_, tx| {
        gfx_lock
            .lock()
            .modelview_mut()
            .replace_transform(tx.to_homogeneous_mat4());
        Ok(())
    }
}

pub(crate) fn rotate(gfx_lock: Shared<GraphicsLock>) -> lua_fn!(Fn<'lua>(f32) -> ()) {
    move |_, angle| {
        gfx_lock.lock().modelview_mut().rotate2(angle);
        Ok(())
    }
}

pub(crate) fn scale(gfx_lock: Shared<GraphicsLock>) -> lua_fn!(Fn<'lua>((f32, Option<f32>)) -> ()) {
    move |_, (x, maybe_y)| {
        gfx_lock
            .lock()
            .modelview_mut()
            .scale2(Vector2::new(x, maybe_y.unwrap_or(x)));
        Ok(())
    }
}

pub(crate) fn shear(gfx_lock: Shared<GraphicsLock>) -> lua_fn!(Fn<'lua>((f32, f32)) -> ()) {
    move |_, (x, y)| {
        gfx_lock.lock().modelview_mut().shear2(Vector2::new(x, y));
        Ok(())
    }
}

pub(crate) fn transform_point(
    gfx_lock: Shared<GraphicsLock>,
) -> lua_fn!(Fn<'lua>((f32, f32)) -> (f32, f32)) {
    move |_, (x, y)| {
        let out = gfx_lock
            .lock()
            .modelview()
            .transform_point2(Point2::new(x, y));
        Ok((out.x, out.y))
    }
}

pub(crate) fn translate(gfx_lock: Shared<GraphicsLock>) -> lua_fn!(Fn<'lua>((f32, f32)) -> ()) {
    move |_, (x, y)| {
        gfx_lock
            .lock()
            .modelview_mut()
            .translate2(Vector2::new(x, y));
        Ok(())
    }
}
