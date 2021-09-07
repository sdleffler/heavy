use std::convert::TryFrom;

use hv_core::{
    components::DynamicComponentConstructor, engine::Engine, prelude::*, spaces::serialize,
};
use na::Isometry2;
use parry2d::shape::{
    Ball, Compound, ConvexPolygon, Cuboid, HalfSpace, Polyline, Segment, SharedShape,
};
use serde::*;

use crate::math::*;

mod compound_helper {
    use serde::ser::SerializeSeq;

    use super::*;

    pub fn serialize<S>(compound: &Compound, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let shapes = compound.shapes();
        let mut tuple_ser = serializer.serialize_seq(Some(shapes.len()))?;
        for (iso, shape) in shapes {
            let serializable_shape =
                ClosedShape::try_from(shape).map_err(serde::ser::Error::custom)?;
            tuple_ser.serialize_element(&(iso, serializable_shape))?;
        }
        tuple_ser.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Compound, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[allow(clippy::type_complexity)]
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Vec<(Isometry2<f32>, SharedShape)>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "sequence of locally transformed shapes")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut shapes = Vec::new();
                while let Some((iso, serializable_shape)) =
                    seq.next_element::<(Isometry2<f32>, ClosedShape)>()?
                {
                    shapes.push((iso, serializable_shape.into()));
                }
                Ok(shapes)
            }
        }

        let shapes = deserializer.deserialize_seq(Visitor)?;

        Ok(Compound::new(shapes))
    }
}

mod shape_handle_helper {
    use super::*;

    pub fn serialize<S>(shape_handle: &SharedShape, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ClosedShape::try_from(shape_handle)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SharedShape, D::Error>
    where
        D: Deserializer<'de>,
    {
        ClosedShape::deserialize(deserializer).map(SharedShape::from)
    }
}

#[derive(Clone, Serialize, Deserialize)]
enum ClosedShape {
    Ball(Ball),

    #[serde(with = "compound_helper")]
    Compound(Compound),

    Cuboid(Cuboid),
    HalfSpace(HalfSpace),
    // HeightField(HeightField),
    Polygon(ConvexPolygon),
    Polyline(Polyline),
    Segment(Segment),
}

impl<'a> TryFrom<&'a SharedShape> for ClosedShape {
    type Error = Error;

    fn try_from(value: &'a SharedShape) -> Result<Self, Self::Error> {
        if let Some(ball) = value.downcast_ref::<Ball>().copied() {
            Ok(ClosedShape::Ball(ball))
        } else if let Some(compound) = value.downcast_ref::<Compound>().cloned() {
            Ok(ClosedShape::Compound(compound))
        } else if let Some(cuboid) = value.downcast_ref::<Cuboid>().copied() {
            Ok(ClosedShape::Cuboid(cuboid))
        } else if let Some(half_space) = value.downcast_ref::<HalfSpace>().cloned() {
            Ok(ClosedShape::HalfSpace(half_space))
        // } else if let Some(height_field) = value.downcast_ref::<HeightField>().cloned() {
        //     Ok(ClosedShape::HeightField(height_field))
        } else if let Some(polygon) = value.downcast_ref::<ConvexPolygon>().cloned() {
            Ok(ClosedShape::Polygon(polygon))
        } else if let Some(polyline) = value.downcast_ref::<Polyline>().cloned() {
            Ok(ClosedShape::Polyline(polyline))
        } else if let Some(segment) = value.downcast_ref::<Segment>().copied() {
            Ok(ClosedShape::Segment(segment))
        } else {
            Err(anyhow!("unsupported shape!"))
        }
    }
}

impl From<ClosedShape> for SharedShape {
    fn from(shape: ClosedShape) -> Self {
        match shape {
            ClosedShape::Ball(ball) => SharedShape::new(ball),
            ClosedShape::Compound(compound) => SharedShape::new(compound),
            ClosedShape::Cuboid(cuboid) => SharedShape::new(cuboid),
            ClosedShape::HalfSpace(half_space) => SharedShape::new(half_space),
            // ClosedShape::HeightField(height_field) => SharedShape::new(height_field),
            ClosedShape::Polygon(polygon) => SharedShape::new(polygon),
            ClosedShape::Polyline(polyline) => SharedShape::new(polyline),
            ClosedShape::Segment(segment) => SharedShape::new(segment),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Collider {
    #[serde(with = "shape_handle_helper")]
    pub shape: SharedShape,
    pub local_tx: Isometry2<f32>,
}

hv_core::serializable!(serialize::with_serde::<Collider>("friends.Collider"));

impl Collider {
    pub fn new(local_tx: Isometry2<f32>, shape: SharedShape) -> Self {
        Self { shape, local_tx }
    }

    pub fn compute_local_aabb(&self) -> Box2<f32> {
        self.shape.compute_local_aabb().into()
    }

    pub fn compute_aabb(&self, position: &Isometry2<f32>) -> Box2<f32> {
        self.shape.compute_aabb(position).into()
    }

    pub fn compute_swept_aabb(
        &self,
        start_pos: &Isometry2<f32>,
        end_pos: &Isometry2<f32>,
    ) -> Box2<f32> {
        self.shape.compute_swept_aabb(start_pos, end_pos).into()
    }

    pub fn lua_compute_local_aabb(_: &Lua, this: &Self, (): ()) -> LuaResult<Box2<f32>> {
        Ok(this.compute_local_aabb())
    }

    pub fn lua_compute_aabb(_: &Lua, this: &Self, tx: Tx<f32>) -> LuaResult<Box2<f32>> {
        Ok(this.compute_aabb(
            &tx.to_isometry2()
                .ok_or_else(|| anyhow!("could not convert transform to Isometry2").to_lua_err())?,
        ))
    }

    pub fn lua_compute_swept_aabb(
        _: &Lua,
        this: &Self,
        (start_tx, end_tx): (Tx<f32>, Tx<f32>),
    ) -> LuaResult<Box2<f32>> {
        let start_pos = start_tx
            .to_isometry2()
            .ok_or_else(|| anyhow!("could not convert start position to Isometry2").to_lua_err())?;
        let end_pos = end_tx
            .to_isometry2()
            .ok_or_else(|| anyhow!("could not convert end position to Isometry2").to_lua_err())?;
        Ok(this.compute_swept_aabb(&start_pos, &end_pos))
    }

    pub fn lua_ball(lua: &Lua, (radius, more): (f32, LuaMultiValue)) -> LuaResult<Self> {
        Ok(Self {
            shape: SharedShape::ball(radius),
            local_tx: Position2::lua_new(lua, more)?.to_isometry(),
        })
    }

    pub fn lua_compound(lua: &Lua, args: LuaMultiValue) -> LuaResult<Self> {
        let mut args_vec = args.into_vec();
        let mut shapes = Vec::new();
        while let Some(LuaValue::UserData(ud)) = args_vec.last() {
            if let Ok(collider) = ud.borrow::<Collider>() {
                shapes.push((collider.local_tx, collider.shape.clone()));
            } else {
                break;
            }

            args_vec.pop();
        }

        let pos = Position2::lua_new(lua, LuaMultiValue::from_vec(args_vec))?;

        Ok(Self {
            shape: SharedShape::compound(shapes),
            local_tx: pos.to_isometry(),
        })
    }

    pub fn lua_cuboid(lua: &Lua, (hx, hy, more): (f32, f32, LuaMultiValue)) -> LuaResult<Self> {
        Ok(Collider {
            shape: SharedShape::cuboid(hx, hy),
            local_tx: Position2::lua_new(lua, more)?.to_isometry(),
        })
    }

    pub fn lua_halfspace(lua: &Lua, (nx, ny, more): (f32, f32, LuaMultiValue)) -> LuaResult<Self> {
        Ok(Collider {
            shape: SharedShape::halfspace(UnitVector2::new_normalize(Vector2::new(nx, ny))),
            local_tx: Position2::lua_new(lua, more)?.to_isometry(),
        })
    }

    pub fn lua_convex_hull(
        lua: &Lua,
        (vertex_coords, more): (Vec<f32>, LuaMultiValue),
    ) -> LuaResult<Self> {
        if vertex_coords.len() % 2 == 1 {
            return Err(LuaError::external(anyhow!(
                "expected an even number of vertex coordinates!"
            )));
        }

        let mut vertices = Vec::new();
        for xy in vertex_coords.chunks_exact(2) {
            vertices.push(Point2::new(xy[0], xy[1]));
        }

        Ok(Collider {
            shape: SharedShape::convex_hull(&vertices)
                .ok_or_else(|| anyhow!("failed to compute convex hull of vertex coordinates"))
                .to_lua_err()?,
            local_tx: Position2::lua_new(lua, more)?.to_isometry(),
        })
    }

    pub fn lua_convex_polyline(
        lua: &Lua,
        (vertex_coords, more): (Vec<f32>, LuaMultiValue),
    ) -> LuaResult<Self> {
        if vertex_coords.len() % 2 == 1 {
            return Err(LuaError::external(anyhow!(
                "expected an even number of vertex coordinates!"
            )));
        }

        let mut vertices = Vec::new();
        for xy in vertex_coords.chunks_exact(2) {
            vertices.push(Point2::new(xy[0], xy[1]));
        }

        Ok(Collider {
            shape: SharedShape::convex_polyline(vertices)
                .ok_or_else(|| anyhow!("coordinates were too close to collinear!"))
                .to_lua_err()?,
            local_tx: Position2::lua_new(lua, more)?.to_isometry(),
        })
    }

    pub fn lua_polyline(
        lua: &Lua,
        (vertex_coords, more): (Vec<f32>, LuaMultiValue),
    ) -> LuaResult<Self> {
        if vertex_coords.len() % 2 == 1 {
            return Err(LuaError::external(anyhow!(
                "expected an even number of vertex coordinates!"
            )));
        }

        let mut vertices = Vec::new();
        for xy in vertex_coords.chunks_exact(2) {
            vertices.push(Point2::new(xy[0], xy[1]));
        }

        Ok(Collider {
            shape: SharedShape::polyline(vertices, None),
            local_tx: Position2::lua_new(lua, more)?.to_isometry(),
        })
    }

    pub fn lua_segment(
        lua: &Lua,
        (ax, ay, bx, by, more): (f32, f32, f32, f32, LuaMultiValue),
    ) -> LuaResult<Self> {
        Ok(Collider {
            shape: SharedShape::segment(Point2::new(ax, ay), Point2::new(bx, by)),
            local_tx: Position2::lua_new(lua, more)?.to_isometry(),
        })
    }
}

impl LuaUserData for Collider {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("compute_local_aabb", Self::lua_compute_local_aabb);
        methods.add_method("compute_aabb", Self::lua_compute_aabb);
        methods.add_method("compute_swept_aabb", Self::lua_compute_swept_aabb);
    }
}

pub(crate) fn open<'lua>(lua: &'lua Lua, _engine: &Engine) -> Result<LuaTable<'lua>> {
    let create_ball = lua.create_function(Collider::lua_ball)?;
    let create_compound = lua.create_function(Collider::lua_compound)?;
    let create_cuboid = lua.create_function(Collider::lua_cuboid)?;
    let create_halfspace = lua.create_function(Collider::lua_halfspace)?;
    let create_convex_hull = lua.create_function(Collider::lua_convex_hull)?;
    let create_convex_polyline = lua.create_function(Collider::lua_convex_polyline)?;
    let create_polyline = lua.create_function(Collider::lua_polyline)?;
    let create_segment = lua.create_function(Collider::lua_segment)?;

    let create_collider_component = lua.create_function(|_, collider: Collider| {
        Ok(DynamicComponentConstructor::clone(collider))
    })?;

    let intersection_test = lua.create_function(
        |_,
         (pos1, collider1, pos2, collider2): (
            Position2<f32>,
            Collider,
            Position2<f32>,
            Collider,
        )| {
            parry2d::query::intersection_test(
                &pos1,
                collider1.shape.as_ref(),
                &pos2,
                collider2.shape.as_ref(),
            )
            .to_lua_err()
        },
    )?;

    let chunk = mlua::chunk! {
        create_ball = $create_ball,
        create_compound = $create_compound,
        create_cuboid = $create_cuboid,
        create_halfspace = $create_halfspace,
        create_convex_hull = $create_convex_hull,
        create_convex_polyline = $create_convex_polyline,
        create_polyline = $create_polyline,
        create_segment = $create_segment,

        create_collider_component = $create_collider_component,

        intersection_test = $intersection_test,
    };

    Ok(lua.load(chunk).eval()?)
}
