use {
    anyhow::*,
    mlua::{prelude::*, LuaSerdeExt},
    nalgebra::SimdPartialOrd,
    num_traits::{Bounded, NumAssign, NumAssignRef, NumCast},
    serde::{de::DeserializeOwned, Deserialize, Serialize},
};

use std::{
    mem,
    ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign},
};

use hv_core::engine::Engine;
pub use mint;

use na::{Storage, Vector, U3};
pub use nalgebra::{
    self as na, Affine2, Affine3, Complex, Isometry2, Isometry3, Matrix2, Matrix3, Matrix4,
    Orthographic3, Perspective3, Point2, Point3, Projective2, Projective3, Quaternion, RealField,
    Rotation2, Rotation3, Scalar, Similarity2, Similarity3, Transform2, Transform3, Translation2,
    Translation3, Unit, UnitComplex, UnitQuaternion, Vector2, Vector3, Vector4,
};

pub use num_traits as num;

use crate::lua::*;

pub mod transform;
pub use transform::*;

pub trait Numeric:
    NumAssign + NumAssignRef + NumCast + Scalar + Copy + PartialOrd + SimdPartialOrd + Bounded
{
}
impl<T> Numeric for T where
    T: NumAssign + NumAssignRef + NumCast + Scalar + Copy + PartialOrd + SimdPartialOrd + Bounded
{
}

/// A velocity structure combining both the linear angular velocities of a point.
#[repr(C)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Velocity2<N: RealField + Copy> {
    /// The linear velocity.
    pub linear: Vector2<N>,
    /// The angular velocity.
    pub angular: N,
}

impl<N: RealField + Copy> Velocity2<N> {
    /// Create velocity from its linear and angular parts.
    #[inline]
    pub fn new(linear: Vector2<N>, angular: N) -> Self {
        Velocity2 { linear, angular }
    }

    /// Create a purely angular velocity.
    #[inline]
    pub fn angular(w: N) -> Self {
        Velocity2::new(na::zero(), w)
    }

    /// Create a purely linear velocity.
    #[inline]
    pub fn linear(vx: N, vy: N) -> Self {
        Velocity2::new(Vector2::new(vx, vy), N::zero())
    }

    /// Create a zero velocity.
    #[inline]
    pub fn zero() -> Self {
        Self::new(na::zero(), N::zero())
    }

    /// Computes the velocity required to move from `start` to `end` in the given `time`.
    pub fn between_positions(start: &Isometry2<N>, end: &Isometry2<N>, time: N) -> Self {
        let delta = end / start;
        let linear = delta.translation.vector / time;
        let angular = delta.rotation.angle() / time;
        Self::new(linear, angular)
    }

    /// Compute the displacement due to this velocity integrated during the time `dt`.
    pub fn integrate(&self, dt: N) -> Isometry2<N> {
        (*self * dt).to_transform()
    }

    /// Compute the displacement due to this velocity integrated during a time equal to `1.0`.
    ///
    /// This is equivalent to `self.integrate(1.0)`.
    pub fn to_transform(&self) -> Isometry2<N> {
        Isometry2::new(self.linear, self.angular)
    }

    /// This velocity seen as a slice.
    ///
    /// The linear part is stored first.
    #[inline]
    pub fn as_slice(&self) -> &[N] {
        self.as_vector().as_slice()
    }

    /// This velocity seen as a mutable slice.
    ///
    /// The linear part is stored first.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [N] {
        self.as_vector_mut().as_mut_slice()
    }

    /// This velocity seen as a vector.
    ///
    /// The linear part is stored first.
    #[inline]
    pub fn as_vector(&self) -> &Vector3<N> {
        unsafe { mem::transmute(self) }
    }

    /// This velocity seen as a mutable vector.
    ///
    /// The linear part is stored first.
    #[inline]
    pub fn as_vector_mut(&mut self) -> &mut Vector3<N> {
        unsafe { mem::transmute(self) }
    }

    /// Create a velocity from a vector.
    ///
    /// The linear part of the velocity is expected to be first inside of the input vector.
    #[inline]
    pub fn from_vector<S: Storage<N, U3>>(data: &Vector<N, U3, S>) -> Self {
        Self::new(Vector2::new(data[0], data[1]), data[2])
    }

    /// Create a velocity from a slice.
    ///
    /// The linear part of the velocity is expected to be first inside of the input slice.
    #[inline]
    pub fn from_slice(data: &[N]) -> Self {
        Self::new(Vector2::new(data[0], data[1]), data[2])
    }

    /// Compute the velocity of a point that is located at the coordinates `shift` relative to the point having `self` as velocity.
    #[inline]
    pub fn shift(&self, shift: &Vector2<N>) -> Self {
        Self::new(
            self.linear + Vector2::new(-shift.y, shift.x) * self.angular,
            self.angular,
        )
    }

    /// Rotate each component of `self` by `rot`.
    #[inline]
    pub fn rotated(&self, rot: &Rotation2<N>) -> Self {
        Self::new(rot * self.linear, self.angular)
    }

    /// Transform each component of `self` by `iso`.
    #[inline]
    pub fn transformed(&self, iso: &Isometry2<N>) -> Self {
        Self::new(iso * self.linear, self.angular)
    }
}

impl<N: RealField + Copy> Add<Velocity2<N>> for Velocity2<N> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        Velocity2::new(self.linear + rhs.linear, self.angular + rhs.angular)
    }
}

impl<N: RealField + Copy> AddAssign<Velocity2<N>> for Velocity2<N> {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.linear += rhs.linear;
        self.angular += rhs.angular;
    }
}

impl<N: RealField + Copy> Sub<Velocity2<N>> for Velocity2<N> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Velocity2::new(self.linear - rhs.linear, self.angular - rhs.angular)
    }
}

impl<N: RealField + Copy> SubAssign<Velocity2<N>> for Velocity2<N> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.linear -= rhs.linear;
        self.angular -= rhs.angular;
    }
}

impl<N: RealField + Copy> Mul<N> for Velocity2<N> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: N) -> Self {
        Velocity2::new(self.linear * rhs, self.angular * rhs)
    }
}

impl<N: RealField + Copy> MulAssign<N> for Velocity2<N> {
    #[inline]
    fn mul_assign(&mut self, rhs: N) {
        *self = Velocity2::new(self.linear * rhs, self.angular * rhs);
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(into = "Box2Proxy<N>", from = "Box2Proxy<N>")]
pub struct Box2<N: Numeric> {
    pub mins: Point2<N>,
    pub maxs: Point2<N>,
}

impl<N: Numeric + RealField> From<ncollide2d::bounding_volume::AABB<N>> for Box2<N> {
    fn from(aabb: ncollide2d::bounding_volume::AABB<N>) -> Self {
        Self {
            mins: aabb.mins,
            maxs: aabb.maxs,
        }
    }
}

impl<N: Numeric> Box2<N> {
    pub fn new(x: N, y: N, w: N, h: N) -> Self {
        Self {
            mins: Point2::new(x, y),
            maxs: Point2::new(x + w, y + h),
        }
    }

    pub fn from_corners(mins: Point2<N>, maxs: Point2<N>) -> Self {
        Self { mins, maxs }
    }

    pub fn from_extents(mins: Point2<N>, extents: Vector2<N>) -> Self {
        Self {
            mins,
            maxs: mins + extents,
        }
    }

    pub fn from_half_extents(center: Point2<N>, half_extents: Vector2<N>) -> Self {
        Self {
            mins: center - half_extents,
            maxs: center + half_extents,
        }
    }

    pub fn invalid() -> Self {
        Self {
            mins: Vector2::repeat(N::max_value()).into(),
            maxs: Vector2::repeat(N::min_value()).into(),
        }
    }

    pub fn huge() -> Self {
        Self {
            mins: Vector2::repeat(N::min_value()).into(),
            maxs: Vector2::repeat(N::max_value()).into(),
        }
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        na::partial_le(&self.mins, &self.maxs)
    }

    #[inline]
    pub fn center(&self) -> Point2<N> {
        self.mins + self.half_extents()
    }

    #[inline]
    pub fn to_aabb(&self) -> ncollide2d::bounding_volume::AABB<N>
    where
        N: RealField,
    {
        ncollide2d::bounding_volume::AABB::new(self.mins, self.maxs)
    }

    #[inline]
    pub fn extents(&self) -> Vector2<N> {
        self.maxs.coords - self.mins.coords
    }

    #[inline]
    pub fn half_extents(&self) -> Vector2<N> {
        self.extents() / num::cast::<_, N>(2).unwrap()
    }

    #[inline]
    pub fn merge(&mut self, other: &Self) {
        *self = self.merged(other);
    }

    #[inline]
    pub fn merged(&self, other: &Self) -> Self {
        let new_mins = self.mins.coords.inf(&other.mins.coords);
        let new_maxes = self.maxs.coords.sup(&other.maxs.coords);
        Self {
            mins: Point2::from(new_mins),
            maxs: Point2::from(new_maxes),
        }
    }

    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        na::partial_le(&self.mins, &other.maxs) && na::partial_ge(&self.maxs, &other.mins)
    }

    #[inline]
    pub fn contains(&self, other: &Self) -> bool {
        na::partial_le(&self.mins, &other.mins) && na::partial_ge(&self.maxs, &other.maxs)
    }

    #[inline]
    pub fn loosen(&mut self, margin: N) {
        assert!(margin >= na::zero());
        let margin = Vector2::repeat(margin);
        self.mins -= margin;
        self.maxs += margin;
    }

    #[inline]
    pub fn loosened(&self, margin: N) -> Self {
        assert!(margin >= na::zero());
        let margin = Vector2::repeat(margin);
        Self {
            mins: self.mins - margin,
            maxs: self.maxs + margin,
        }
    }

    #[inline]
    pub fn tighten(&mut self, margin: N) {
        assert!(margin >= na::zero());
        let margin = Vector2::repeat(margin);
        self.mins += margin;
        self.maxs -= margin;
        assert!(na::partial_le(&self.mins, &self.maxs));
    }

    #[inline]
    pub fn tightened(&self, margin: N) -> Self {
        assert!(margin >= na::zero());
        let margin = Vector2::repeat(margin);
        Self {
            mins: self.mins + margin,
            maxs: self.maxs - margin,
        }
    }

    #[inline]
    pub fn from_points<'a, I>(pts: I) -> Self
    where
        I: IntoIterator<Item = &'a Point2<N>>,
    {
        let mut iter = pts.into_iter();

        let p0 = iter.next().expect("iterator must be nonempty");
        let mut mins: Point2<N> = *p0;
        let mut maxs: Point2<N> = *p0;

        for pt in iter {
            mins = mins.inf(pt);
            maxs = maxs.sup(pt);
        }

        Self { mins, maxs }
    }

    #[inline]
    pub fn transformed_by(&self, tx: &Matrix4<N>) -> Self
    where
        N: RealField,
    {
        let tl = Point3::new(self.mins.x, self.mins.y, N::zero());
        let tr = Point3::new(self.maxs.x, self.mins.y, N::zero());
        let br = Point3::new(self.maxs.x, self.maxs.y, N::zero());
        let bl = Point3::new(self.mins.x, self.maxs.y, N::zero());

        Self::from_points(&[
            tx.transform_point(&tl).xy(),
            tx.transform_point(&tr).xy(),
            tx.transform_point(&br).xy(),
            tx.transform_point(&bl).xy(),
        ])
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename = "Box2")]
struct Box2Proxy<N: Numeric> {
    x: N,
    y: N,
    w: N,
    h: N,
}

impl<N: Numeric> From<Box2<N>> for Box2Proxy<N> {
    fn from(b: Box2<N>) -> Self {
        Self {
            x: b.mins.x,
            y: b.mins.y,
            w: b.maxs.x - b.mins.x,
            h: b.maxs.y - b.mins.y,
        }
    }
}

impl<N: Numeric> From<Box2Proxy<N>> for Box2<N> {
    fn from(b: Box2Proxy<N>) -> Self {
        Self::new(b.x, b.y, b.w, b.h)
    }
}

impl<'lua, N> ToLua<'lua> for Box2<N>
where
    N: Numeric + Serialize + ToLua<'lua>,
{
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        lua.to_value(&self)
    }
}

impl<'lua, N> FromLua<'lua> for Box2<N>
where
    N: Numeric + DeserializeOwned + FromLua<'lua>,
{
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        lua.from_value(lua_value)
    }
}

#[rustfmt::skip]
pub fn homogeneous_mat3_to_mat4<T: RealField + Copy>(mat3: &Matrix3<T>) -> Matrix4<T> {
    Matrix4::new(
        mat3[(0, 0)], mat3[(0, 1)],    T::zero(), mat3[(0, 2)],
        mat3[(1, 0)], mat3[(1, 1)],    T::zero(), mat3[(1, 2)],
          T::zero(),     T::zero(),     T::one(),    T::zero(),
        mat3[(2, 0)], mat3[(2, 1)],    T::zero(), mat3[(2, 2)],
    )
}

#[derive(Debug, Clone, Copy)]
pub struct HvVector2<T: RealField + Copy>(pub Vector2<T>);

impl<T: RealField + Copy + for<'lua> ToLua<'lua> + for<'lua> FromLua<'lua>> LuaUserData
    for HvVector2<T>
{
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        add_field!(fields, t.x => t.0.x);
        add_field!(fields, t.y => t.0.y);
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        use HvVector2 as HvV2;

        add_clone_methods(methods);

        simple_mut(methods, "init", |t, (x, y)| t.0 = Vector2::new(x, y));
        simple(methods, "norm", |t, ()| t.0.norm());

        lh_binop(methods, "add", |HvV2(lhs), HvV2(rhs)| HvV2(lhs + rhs));
        lh_binop(methods, "sub", |HvV2(lhs), HvV2(rhs)| HvV2(lhs - rhs));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HvIsometry2<T: RealField + Copy>(pub Isometry2<T>);

impl<T: RealField + Copy> HvIsometry2<T> {
    pub fn to_matrix4(&self) -> Matrix4<T> {
        homogeneous_mat3_to_mat4(&self.0.to_homogeneous())
    }
}

impl<T: RealField + Copy + for<'lua> ToLua<'lua> + for<'lua> FromLua<'lua>> LuaUserData
    for HvIsometry2<T>
{
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        add_field!(fields, t.x => t.0.translation.vector.x);
        add_field!(fields, t.y => t.0.translation.vector.y);

        add_getter!(fields, t.angle => t.0.rotation.angle());
        add_setter!(fields, t.angle = angle => t.0.rotation = UnitComplex::new(angle));

        add_getter!(fields, t.translation => HvVector2(t.0.translation.vector));
        add_setter!(fields, t.translation = v: HvVector2<T> => t.0.translation.vector = v.0);
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        use HvIsometry2 as HvI2;

        add_clone_methods(methods);

        simple_mut(methods, "init", |t, (x, y, a)| {
            t.0 = Isometry2::new(Vector2::new(x, y), a)
        });

        simple_mut(methods, "set_translation", |t, (x, y)| {
            t.0.translation = Translation2::new(x, y)
        });

        simple_mut(methods, "set_rotation", |t, a| {
            t.0.rotation = UnitComplex::new(a)
        });

        lh_binop(methods, "mul", |HvI2(lhs), HvI2(rhs)| HvI2(lhs * rhs));
        lh_binop(methods, "div", |HvI2(lhs), HvI2(rhs)| HvI2(lhs / rhs));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HvVelocity2<T: RealField + Copy>(pub Velocity2<T>);

impl<T: RealField + Copy + for<'lua> ToLua<'lua> + for<'lua> FromLua<'lua>> LuaUserData
    for HvVelocity2<T>
{
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        add_field!(fields, t.x => t.0.linear.x);
        add_field!(fields, t.y => t.0.linear.y);
        add_field!(fields, t.angular => t.0.angular);

        add_getter!(fields, t.linear => HvVector2(t.0.linear));
        add_setter!(fields, t.linear = v: HvVector2<T> => t.0.linear = v.0);
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        add_clone_methods(methods);

        methods.add_method_mut("init", |_, this, (x, y, angular)| {
            this.0 = Velocity2::new(Vector2::new(x, y), angular);
            Ok(())
        });

        methods.add_method_mut("set_linear", |_, this, (x, y)| {
            this.0.linear = Vector2::new(x, y);
            Ok(())
        });

        methods.add_method_mut("set_angular", |_, this, angular| {
            this.0.angular = angular;
            Ok(())
        });
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HvMatrix4<T: RealField + Copy>(pub Matrix4<T>);

impl<T: RealField + Copy + for<'lua> ToLua<'lua> + for<'lua> FromLua<'lua>> LuaUserData
    for HvMatrix4<T>
{
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        add_clone_methods(methods);

        methods.add_method_mut("apply", |_, this, HvMatrix4(mat)| {
            this.0 *= mat;
            Ok(())
        });

        methods.add_method("inverse", |_, this, ()| {
            Ok(this.0.try_inverse().map(HvMatrix4))
        });

        methods.add_method(
            "inverse_transform_point2",
            |_, this, (x, y): (T, T)| match this
                .0
                .try_inverse()
                .map(|inv| inv.transform_point(&Point3::new(x, y, T::zero())))
                .map(|pt| (pt.x, pt.y))
            {
                Some((x, y)) => Ok((Some(x), Some(y))),
                None => Ok((None, None)),
            },
        );

        methods.add_method_mut("isometry2", |_, this, HvIsometry2(iso): HvIsometry2<T>| {
            let iso_mat = homogeneous_mat3_to_mat4(&iso.to_homogeneous());
            this.0 *= iso_mat;
            Ok(())
        });

        methods.add_method_mut("reset", |_, this, ()| {
            this.0 = Matrix4::identity();
            Ok(())
        });

        methods.add_method_mut("rotate2", |_, this, angle| {
            this.0 *= homogeneous_mat3_to_mat4(&UnitComplex::new(angle).to_homogeneous());
            Ok(())
        });

        methods.add_method_mut("scale2", |_, this, (x, maybe_y): (T, Option<T>)| {
            let y = maybe_y.unwrap_or(x);
            this.0
                .append_nonuniform_scaling_mut(&Vector3::new(x, y, T::one()));
            Ok(())
        });

        methods.add_method_mut("set_matrix", |_, this, (m11, m12, m13, m14, m21, m22, m23, m24, m31, m32, m33, m34, m41, m42, m43, m44)| {
            this.0 = Matrix4::new(m11, m12, m13, m14, m21, m22, m23, m24, m31, m32, m33, m34, m41, m42, m43, m44);
            Ok(())
        });

        methods.add_method_mut("set_transformation", |_, this, HvMatrix4(rhs)| {
            this.0 = rhs;
            Ok(())
        });

        methods.add_method_mut("shear2", |_, this, (x, y)| {
            this.0 *=
                homogeneous_mat3_to_mat4(&Matrix2::new(T::one(), x, y, T::one()).to_homogeneous());
            Ok(())
        });

        methods.add_method_mut("transform_point", |_, this, (x, y)| {
            let out = this.0.transform_point(&Point3::new(x, y, T::zero())).xy();
            Ok((out.x, out.y))
        });

        methods.add_method_mut("translate2", |_, this, (x, y)| {
            this.0
                .append_translation_mut(&Vector3::new(x, y, T::zero()));
            Ok(())
        });

        methods.add_meta_method(LuaMetaMethod::Mul, |_, HvMatrix4(lhs), HvMatrix4(rhs)| {
            Ok(HvMatrix4(lhs * rhs))
        });
    }
}

pub(crate) fn open<'lua>(lua: &'lua Lua, _engine: &Engine) -> Result<LuaTable<'lua>> {
    let create_isometry2_object_from_identity =
        lua.create_function(move |_lua, ()| Ok(HvIsometry2::<f32>(Isometry2::identity())))?;
    let create_isometry2_object = lua.create_function(move |_lua, (x, y, angle)| {
        Ok(HvIsometry2::<f32>(Isometry2::new(
            Vector2::new(x, y),
            angle,
        )))
    })?;

    let create_velocity2_object_from_zero =
        lua.create_function(move |_lua, ()| Ok(HvVelocity2::<f32>(Velocity2::zero())))?;
    let create_velocity2_object = lua.create_function(move |_lua, (x, y, angular)| {
        Ok(HvVelocity2::<f32>(Velocity2::new(
            Vector2::new(x, y),
            angular,
        )))
    })?;

    let create_matrix4_object_from_identity =
        lua.create_function(move |_lua, ()| Ok(HvMatrix4::<f32>(Matrix4::identity())))?;

    let create_transform_object = lua.create_function(move |_lua, ()| Ok(Tx::<f32>::identity()))?;

    Ok(lua
        .load(mlua::chunk! {
            {
                create_isometry2_object = $create_isometry2_object,
                create_isometry2_object_from_identity = $create_isometry2_object_from_identity,

                create_velocity2_object = $create_velocity2_object,
                create_velocity2_object_from_zero = $create_velocity2_object_from_zero,

                create_matrix4_object_from_identity = $create_matrix4_object_from_identity,

                create_transform_object = $create_transform_object,
            }
        })
        .eval()?)
}
