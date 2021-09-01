use std::convert::TryFrom;

use hv_core::{prelude::*, spaces::serialize};
use na::Isometry2;
use parry2d::shape::{
    Ball, Compound, ConvexPolygon, Cuboid, HeightField, Polyline, Segment, SharedShape,
};
use serde::*;

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
    HeightField(HeightField),
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
        } else if let Some(height_field) = value.downcast_ref::<HeightField>().cloned() {
            Ok(ClosedShape::HeightField(height_field))
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
            ClosedShape::HeightField(height_field) => SharedShape::new(height_field),
            ClosedShape::Polygon(polygon) => SharedShape::new(polygon),
            ClosedShape::Polyline(polyline) => SharedShape::new(polyline),
            ClosedShape::Segment(segment) => SharedShape::new(segment),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[must_use = "colliders will be leaked if not removed from their collision space!"]
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
}
