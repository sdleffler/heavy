use std::convert::TryFrom;

use hv_core::{
    prelude::*,
    spaces::{serialize, Object, Space},
};
use na::{Isometry2, RealField};
use nc::{
    pipeline::{
        CollisionGroups, CollisionObjectSlabHandle, CollisionObjectUpdateFlags, GeometricQueryType,
    },
    shape::{Ball, Compound, ConvexPolygon, Cuboid, HeightField, Polyline, Segment, ShapeHandle},
    world::CollisionWorld,
};
use serde::*;

use crate::{Position, Velocity};

mod compound_helper {
    use std::marker::PhantomData;

    use serde::ser::SerializeSeq;

    use super::*;

    pub fn serialize<N, S>(compound: &Compound<N>, serializer: S) -> Result<S::Ok, S::Error>
    where
        N: RealField + Copy + Serialize,
        S: Serializer,
    {
        let shapes = compound.shapes();
        let mut tuple_ser = serializer.serialize_seq(Some(shapes.len()))?;
        for (iso, shape) in shapes {
            let serializable_shape =
                SerializableShape::try_from(shape).map_err(serde::ser::Error::custom)?;
            tuple_ser.serialize_element(&(iso, serializable_shape))?;
        }
        tuple_ser.end()
    }

    pub fn deserialize<'de, N, D>(deserializer: D) -> Result<Compound<N>, D::Error>
    where
        N: RealField + Copy + Deserialize<'de>,
        D: Deserializer<'de>,
    {
        #[allow(clippy::type_complexity)]
        struct Visitor<'de, N>
        where
            N: RealField + Copy + Deserialize<'de>,
        {
            _marker: PhantomData<fn() -> (Isometry2<N>, ShapeHandle<N>, &'de ())>,
        }

        impl<'de, N> serde::de::Visitor<'de> for Visitor<'de, N>
        where
            N: RealField + Copy + Deserialize<'de>,
        {
            type Value = Vec<(Isometry2<N>, ShapeHandle<N>)>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "sequence of locally transformed shapes")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut shapes = Vec::new();
                while let Some((iso, serializable_shape)) =
                    seq.next_element::<(Isometry2<N>, SerializableShape<N>)>()?
                {
                    shapes.push((iso, serializable_shape.into()));
                }
                Ok(shapes)
            }
        }

        let shapes = deserializer.deserialize_seq(Visitor {
            _marker: PhantomData,
        })?;

        Ok(Compound::new(shapes))
    }
}

mod shape_handle_helper {
    use super::*;

    pub fn serialize<N, S>(shape_handle: &ShapeHandle<N>, serializer: S) -> Result<S::Ok, S::Error>
    where
        N: RealField + Copy + Serialize,
        S: Serializer,
    {
        SerializableShape::try_from(shape_handle)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }

    pub fn deserialize<'de, N, D>(deserializer: D) -> Result<ShapeHandle<N>, D::Error>
    where
        N: RealField + Copy + Deserialize<'de>,
        D: Deserializer<'de>,
    {
        SerializableShape::deserialize(deserializer).map(ShapeHandle::from)
    }
}

mod query_type_helper {
    use nc::pipeline::GeometricQueryType;

    use super::*;

    #[derive(Serialize, Deserialize)]
    #[serde(remote = "GeometricQueryType")]
    pub enum GeometricQueryTypeDef<N: RealField + Copy> {
        Contacts(N, N),
        Proximity(N),
    }

    impl<N> From<GeometricQueryTypeDef<N>> for nc::pipeline::GeometricQueryType<N>
    where
        N: RealField + Copy,
    {
        fn from(gqt: GeometricQueryTypeDef<N>) -> Self {
            match gqt {
                GeometricQueryTypeDef::Contacts(n, m) => {
                    nc::pipeline::GeometricQueryType::Contacts(n, m)
                }
                GeometricQueryTypeDef::Proximity(n) => {
                    nc::pipeline::GeometricQueryType::Proximity(n)
                }
            }
        }
    }
}

mod collision_groups_helper {
    use super::*;

    #[derive(Serialize, Deserialize)]
    #[serde(remote = "CollisionGroups")]
    pub struct CollisionGroupsDef {
        #[serde(getter = "membership")]
        membership: u32,
        #[serde(getter = "whitelist")]
        whitelist: u32,
        #[serde(getter = "blacklist")]
        blacklist: u32,
    }

    fn membership(cg: &CollisionGroups) -> u32 {
        unsafe { std::mem::transmute::<CollisionGroups, CollisionGroupsDef>(*cg) }.membership
    }

    fn whitelist(cg: &CollisionGroups) -> u32 {
        unsafe { std::mem::transmute::<CollisionGroups, CollisionGroupsDef>(*cg) }.whitelist
    }

    fn blacklist(cg: &CollisionGroups) -> u32 {
        unsafe { std::mem::transmute::<CollisionGroups, CollisionGroupsDef>(*cg) }.blacklist
    }

    impl From<CollisionGroupsDef> for CollisionGroups {
        fn from(cg: CollisionGroupsDef) -> Self {
            unsafe { std::mem::transmute(cg) }
        }
    }
}

#[derive(Serialize, Deserialize)]
enum SerializableShape<N: RealField + Copy> {
    Ball(Ball<N>),

    #[serde(with = "compound_helper")]
    Compound(Compound<N>),

    Cuboid(Cuboid<N>),
    HeightField(HeightField<N>),
    Polygon(ConvexPolygon<N>),
    Polyline(Polyline<N>),
    Segment(Segment<N>),
}

impl<'a, N: RealField + Copy> TryFrom<&'a ShapeHandle<N>> for SerializableShape<N> {
    type Error = Error;

    fn try_from(value: &'a ShapeHandle<N>) -> Result<Self, Self::Error> {
        if let Some(ball) = value.downcast_ref::<Ball<N>>().copied() {
            Ok(SerializableShape::Ball(ball))
        } else if let Some(compound) = value.downcast_ref::<Compound<N>>().cloned() {
            Ok(SerializableShape::Compound(compound))
        } else if let Some(cuboid) = value.downcast_ref::<Cuboid<N>>().copied() {
            Ok(SerializableShape::Cuboid(cuboid))
        } else if let Some(height_field) = value.downcast_ref::<HeightField<N>>().cloned() {
            Ok(SerializableShape::HeightField(height_field))
        } else if let Some(polygon) = value.downcast_ref::<ConvexPolygon<N>>().cloned() {
            Ok(SerializableShape::Polygon(polygon))
        } else if let Some(polyline) = value.downcast_ref::<Polyline<N>>().cloned() {
            Ok(SerializableShape::Polyline(polyline))
        } else if let Some(segment) = value.downcast_ref::<Segment<N>>().copied() {
            Ok(SerializableShape::Segment(segment))
        } else {
            Err(anyhow!("unsupported shape!"))
        }
    }
}

impl<N> From<SerializableShape<N>> for ShapeHandle<N>
where
    N: RealField + Copy,
{
    fn from(shape: SerializableShape<N>) -> Self {
        match shape {
            SerializableShape::Ball(ball) => ShapeHandle::new(ball),
            SerializableShape::Compound(compound) => ShapeHandle::new(compound),
            SerializableShape::Cuboid(cuboid) => ShapeHandle::new(cuboid),
            SerializableShape::HeightField(height_field) => ShapeHandle::new(height_field),
            SerializableShape::Polygon(polygon) => ShapeHandle::new(polygon),
            SerializableShape::Polyline(polyline) => ShapeHandle::new(polyline),
            SerializableShape::Segment(segment) => ShapeHandle::new(segment),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[must_use = "colliders will be leaked if not removed from their collision space!"]
pub struct Collider {
    #[serde(with = "shape_handle_helper")]
    shape: ShapeHandle<f32>,
    local_tx: Isometry2<f32>,
    #[serde(with = "collision_groups_helper::CollisionGroupsDef")]
    collision_groups: CollisionGroups,
    #[serde(with = "query_type_helper::GeometricQueryTypeDef")]
    query_type: GeometricQueryType<f32>,
    handle: Option<CollisionObjectSlabHandle>,
    #[serde(skip)]
    update_flags: CollisionObjectUpdateFlags,
}

hv_core::serializable!(serialize::with_serde::<Collider>("friends.Collider"));

impl Collider {
    pub fn new(local_tx: Isometry2<f32>, shape: ShapeHandle<f32>) -> Self {
        Self {
            shape,
            local_tx,
            collision_groups: CollisionGroups::new(),
            query_type: GeometricQueryType::Proximity(0.1),
            handle: None,
            update_flags: CollisionObjectUpdateFlags::empty(),
        }
    }
}

pub struct CollisionSpace {
    space: Shared<Space>,
    world: CollisionWorld<f32, Object>,
}

impl CollisionSpace {
    pub fn new(space: Shared<Space>) -> Self {
        Self {
            space,
            world: CollisionWorld::new(0.1),
        }
    }

    pub fn update(&mut self, dt: f32) -> Result<()> {
        for (object, (collider, Position(pos), maybe_velocity)) in self
            .space
            .borrow_mut()
            .query_mut::<(&mut Collider, &Position, Option<&Velocity>)>()
        {
            let collision_object = match collider.handle {
                Some(handle) => {
                    let mut_ref = self
                        .world
                        .objects
                        .get_mut(handle)
                        .expect("initialized collider should have object handle");
                    mut_ref.set_position(**pos);
                    mut_ref
                }
                None => {
                    let (handle, mut_ref) = self.world.add(
                        (**pos) * collider.local_tx,
                        collider.shape.clone(),
                        collider.collision_groups,
                        collider.query_type,
                        object,
                    );
                    collider.handle = Some(handle);
                    mut_ref
                }
            };

            if let Some(Velocity(vel)) = maybe_velocity {
                collision_object.set_predicted_position(Some(*pos.integrate(vel, dt)));
            }

            if !collider.update_flags.is_empty() {
                if collider
                    .update_flags
                    .contains(CollisionObjectUpdateFlags::QUERY_TYPE_CHANGED)
                {
                    collision_object.set_query_type(collider.query_type);
                }

                if collider
                    .update_flags
                    .contains(CollisionObjectUpdateFlags::COLLISION_GROUPS_CHANGED)
                {
                    collision_object.set_collision_groups(collider.collision_groups);
                }

                if collider
                    .update_flags
                    .contains(CollisionObjectUpdateFlags::SHAPE_CHANGED)
                {
                    collision_object.set_shape(collider.shape.clone());
                }

                collider.update_flags = CollisionObjectUpdateFlags::empty();
            }
        }

        Ok(())
    }

    pub fn remove(&mut self, collider: Collider) {
        self.world.remove(
            collider
                .handle
                .as_ref()
                .map(|h| std::slice::from_ref(h))
                .unwrap_or_default(),
        );
    }
}
