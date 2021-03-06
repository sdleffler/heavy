use {
    nalgebra::SimdPartialOrd,
    num_traits::{Bounded, NumAssign, NumAssignRef, NumCast},
    serde::{Deserialize, Serialize},
};

use std::{
    mem,
    ops::{Add, AddAssign, Deref, DerefMut, Mul, MulAssign, Sub, SubAssign},
};

use hv_core::{engine::Engine, prelude::*};
pub use mint;

use na::{Storage, Vector, U3};
pub use nalgebra::{
    self as na, Affine2, Affine3, Complex, Isometry2, Isometry3, Matrix2, Matrix3, Matrix4,
    Orthographic3, Perspective3, Point2, Point3, Projective2, Projective3, Quaternion, RealField,
    Rotation2, Rotation3, Scalar, Similarity2, Similarity3, Transform2, Transform3, Translation2,
    Translation3, Unit, UnitComplex, UnitQuaternion, UnitVector2, UnitVector3, UnitVector4,
    Vector2, Vector3, Vector4,
};

use num::Signed;
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

#[repr(C)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position2<N: RealField + Copy>(Isometry2<N>);

impl<N: RealField + Copy> From<Isometry2<N>> for Position2<N> {
    fn from(iso: Isometry2<N>) -> Self {
        Self(iso)
    }
}

impl<N: RealField + Copy> Deref for Position2<N> {
    type Target = Isometry2<N>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N: RealField + Copy> DerefMut for Position2<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<N: RealField + Copy> Position2<N> {
    pub fn new(coords: Point2<N>, angle: N) -> Self {
        Self(Isometry2::new(coords.coords, angle))
    }

    pub fn translation(x: N, y: N) -> Self {
        Self(Isometry2::translation(x, y))
    }

    /// Semi-implicit Euler integration.
    pub fn integrate2_mut(
        &mut self,
        velocity: &mut Velocity2<N>,
        acceleration: &Velocity2<N>,
        dt: N,
    ) {
        let dv = (*acceleration) * dt;
        velocity.linear += dv.linear;
        velocity.angular += dv.angular;
        self.integrate_mut(velocity, dt);
    }

    pub fn integrate(mut self, velocity: &Velocity2<N>, dt: N) -> Self {
        self.integrate_mut(velocity, dt);
        self
    }

    pub fn integrate_mut(&mut self, velocity: &Velocity2<N>, dt: N) {
        let integrated = velocity.integrate(dt);
        self.translation *= integrated.translation;
        self.rotation *= integrated.rotation;
    }

    pub fn center(&self) -> Point2<N> {
        Point2::from(self.0.translation.vector)
    }

    pub fn to_isometry(&self) -> Isometry2<N> {
        self.0
    }
}

impl<N: RealField + Copy + for<'lua> FromLua<'lua> + for<'lua> ToLua<'lua>> Position2<N> {
    pub fn lua_new(lua: &Lua, args: LuaMultiValue) -> LuaResult<Self> {
        match args.len() {
            0 => Ok(Position2::from(Isometry2::identity())),
            1 => {
                let pos = Position2::from_lua_multi(args, lua)?;
                Ok(pos)
            }
            2 => {
                let (x, y) = FromLuaMulti::from_lua_multi(args, lua)?;
                Ok(Position2::translation(x, y))
            }
            3 => {
                let (x, y, angle) = FromLuaMulti::from_lua_multi(args, lua)?;
                Ok(Position2::new(Point2::new(x, y), angle))
            }
            _ => Err(LuaError::external(anyhow!(
                "could not construct position from args"
            ))),
        }
    }
}

/// A velocity structure combining both the linear and angular velocities of a point.
#[repr(C)]
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
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
pub struct Box2<N: Numeric + Send> {
    pub mins: Point2<N>,
    pub maxs: Point2<N>,
}

impl From<parry2d::bounding_volume::AABB> for Box2<f32> {
    fn from(aabb: parry2d::bounding_volume::AABB) -> Self {
        Self {
            mins: aabb.mins,
            maxs: aabb.maxs,
        }
    }
}

impl<N: Numeric + Send> Box2<N> {
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

    #[inline(always)]
    pub fn x(self) -> N {
        self.mins.x
    }

    #[inline(always)]
    pub fn y(self) -> N {
        self.mins.y
    }

    #[inline(always)]
    pub fn w(self) -> N {
        self.extents().x
    }

    #[inline(always)]
    pub fn h(self) -> N {
        self.extents().y
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
    pub fn intersection(&self, other: &Self) -> Self {
        let new_mins = self.mins.coords.sup(&other.mins.coords);
        let new_maxes = self.maxs.coords.inf(&other.maxs.coords);
        Self {
            mins: Point2::from(new_mins),
            maxs: Point2::from(new_maxes),
        }
    }

    #[inline]
    pub fn overlap(&self, other: &Self) -> Vector2<N>
    where
        N: Signed,
    {
        let x = if self.mins.x <= other.mins.x && self.maxs.x >= other.mins.x {
            self.maxs.x - other.mins.x
        } else if self.mins.x <= other.maxs.x && self.maxs.x >= other.maxs.x {
            self.mins.x - other.maxs.x
        } else {
            N::zero()
        };

        let y = if self.mins.y <= other.mins.y && self.maxs.y >= other.mins.y {
            self.maxs.y - other.mins.y
        } else if self.mins.y <= other.maxs.y && self.maxs.y >= other.maxs.y {
            self.mins.y - other.maxs.y
        } else {
            N::zero()
        };

        Vector2::new(x, y)
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

impl Box2<f32> {
    #[inline]
    pub fn to_aabb(&self) -> parry2d::bounding_volume::AABB {
        parry2d::bounding_volume::AABB::new(self.mins, self.maxs)
    }
}

impl<N: Numeric + Send + for<'lua> FromLua<'lua> + for<'lua> ToLua<'lua>> Box2<N> {
    pub fn lua_from_corners(
        _: &Lua,
        (min_x, min_y, max_x, max_y): (N, N, N, N),
    ) -> LuaResult<Self> {
        Ok(Self::from_corners(
            Point2::new(min_x, min_y),
            Point2::new(max_x, max_y),
        ))
    }

    pub fn lua_from_extents(_: &Lua, (x, y, w, h): (N, N, N, N)) -> LuaResult<Self> {
        Ok(Self::from_extents(Point2::new(x, y), Vector2::new(w, h)))
    }

    pub fn lua_from_half_extents(_: &Lua, (x, y, w, h): (N, N, N, N)) -> LuaResult<Self> {
        Ok(Self::from_half_extents(
            Point2::new(x, y),
            Vector2::new(w, h),
        ))
    }

    pub fn lua_invalid(_: &Lua, (): ()) -> LuaResult<Self> {
        Ok(Self::invalid())
    }

    pub fn lua_huge(_: &Lua, (): ()) -> LuaResult<Self> {
        Ok(Self::huge())
    }
}

impl<N: Numeric + Send + for<'lua> FromLua<'lua> + for<'lua> ToLua<'lua>> LuaUserData for Box2<N> {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("intersects", |_, this, other: Self| {
            Ok(this.intersects(&other))
        });

        methods.add_method("contains", |_, this, other: Self| Ok(this.contains(&other)));

        methods.add_method_mut("merge", |_, this, other: Self| {
            this.merge(&other);
            Ok(())
        });

        methods.add_method("merged", |_, this, other: Self| Ok(this.merged(&other)));

        methods.add_method("center", |_, this, ()| {
            let pt = this.center();
            Ok((pt.x, pt.y))
        });

        methods.add_method("mins", |_, this, ()| Ok((this.mins.x, this.mins.y)));
        methods.add_method("maxs", |_, this, ()| Ok((this.maxs.x, this.maxs.y)));

        methods.add_method("extents", |_, this, ()| {
            let exts = this.extents();
            Ok((exts.x, exts.y))
        });

        methods.add_method("half_extents", |_, this, ()| {
            let hexts = this.half_extents();
            Ok((hexts.x, hexts.y))
        });
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

impl<T: RealField + Copy> Position2<T> {
    pub fn to_matrix4(&self) -> Matrix4<T> {
        homogeneous_mat3_to_mat4(&self.to_homogeneous())
    }
}

impl<T: RealField + Copy + for<'lua> ToLua<'lua> + for<'lua> FromLua<'lua>> LuaUserData
    for Position2<T>
{
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        add_field!(fields, t.x => t.0.translation.vector.x);
        add_field!(fields, t.y => t.0.translation.vector.y);

        add_getter!(fields, t.angle => t.0.rotation.angle());
        add_setter!(fields, t.angle = angle => t.0.rotation = UnitComplex::new(angle));
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        add_clone_methods(methods);

        simple_mut(methods, "init", |t, (x, y, a)| {
            t.0 = Isometry2::new(Vector2::new(x, y), a)
        });

        simple_mut(methods, "set_coords", |t, (x, y)| {
            t.0.translation = Translation2::new(x, y)
        });

        simple_mut(methods, "set_angle", |t, a| {
            t.0.rotation = UnitComplex::new(a)
        });

        simple_mut(methods, "add_coords", |t, (x, y)| {
            t.0.translation *= Translation2::new(x, y)
        });

        simple_mut(methods, "add_angle", |t, angle| {
            t.0.rotation *= UnitComplex::new(angle)
        });

        simple(methods, "to_transform", |t, ()| Tx::new(t.0));

        simple_mut(methods, "transform_mut", |t, tx: Tx<T>| {
            *t = tx.transform_position2(t)
        });

        simple_mut(methods, "inverse_transform_mut", |t, tx: Tx<T>| {
            *t = tx.inverse_transform_position2(t)
        });
    }
}

impl<T: RealField + Copy + for<'lua> ToLua<'lua> + for<'lua> FromLua<'lua>> LuaUserData
    for Velocity2<T>
{
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        add_field!(fields, t.x => t.linear.x);
        add_field!(fields, t.y => t.linear.y);
        add_field!(fields, t.angular => t.angular);
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        add_clone_methods(methods);

        methods.add_method_mut("init", |_, this, (x, y, angular)| {
            *this = Velocity2::new(Vector2::new(x, y), angular);
            Ok(())
        });

        methods.add_method_mut("set_linear", |_, this, (x, y)| {
            this.linear = Vector2::new(x, y);
            Ok(())
        });

        methods.add_method_mut("add_linear", |_, this, (x, y)| {
            this.linear += Vector2::new(x, y);
            Ok(())
        });

        methods.add_method_mut("set_angular", |_, this, angular| {
            this.angular = angular;
            Ok(())
        });

        methods.add_method_mut("add_angular", |_, this, angular| {
            this.angular += angular;
            Ok(())
        });
    }
}

pub(crate) fn open<'lua>(lua: &'lua Lua, _engine: &Engine) -> Result<LuaTable<'lua>> {
    let create_position2_object_from_identity =
        lua.create_function(move |_lua, ()| Ok(Position2::<f32>(Isometry2::identity())))?;
    let create_position2_object = lua.create_function(move |_lua, (x, y, angle)| {
        Ok(Position2::<f32>(Isometry2::new(Vector2::new(x, y), angle)))
    })?;

    let create_velocity2_object_from_zero =
        lua.create_function(move |_lua, ()| Ok(Velocity2::<f32>::zero()))?;
    let create_velocity2_object = lua.create_function(move |_lua, (x, y, angular)| {
        Ok(Velocity2::<f32>::new(Vector2::new(x, y), angular))
    })?;

    let create_transform_identity =
        lua.create_function(move |_lua, ()| Ok(Tx::<f32>::identity()))?;
    let create_transform_isometry2 = lua.create_function(move |_lua, (x, y, angle)| {
        Ok(Tx::<f32>::new(Isometry2::new(Vector2::new(x, y), angle)))
    })?;
    let create_transform_rotation2 =
        lua.create_function(move |_lua, angle| Ok(Tx::<f32>::new(Isometry2::rotation(angle))))?;
    let create_transform_translation2 =
        lua.create_function(move |_lua, (x, y)| Ok(Tx::<f32>::new(Isometry2::translation(x, y))))?;

    let create_box2_from_corners = lua.create_function(Box2::<f32>::lua_from_corners)?;
    let create_box2_from_extents = lua.create_function(Box2::<f32>::lua_from_extents)?;
    let create_box2_from_half_extents = lua.create_function(Box2::<f32>::lua_from_half_extents)?;
    let create_box2_invalid = lua.create_function(Box2::<f32>::lua_invalid)?;
    let create_box2_huge = lua.create_function(Box2::<f32>::lua_huge)?;

    Ok(lua
        .load(mlua::chunk! {
            {
                create_position2_object = $create_position2_object,
                create_position2_object_from_identity = $create_position2_object_from_identity,

                create_velocity2_object = $create_velocity2_object,
                create_velocity2_object_from_zero = $create_velocity2_object_from_zero,

                create_transform_identity = $create_transform_identity,
                create_transform_isometry2 = $create_transform_isometry2,
                create_transform_rotation2 = $create_transform_rotation2,
                create_transform_translation2 = $create_transform_translation2,

                create_box2_from_corners = $create_box2_from_corners,
                create_box2_from_extents = $create_box2_from_extents,
                create_box2_from_half_extents = $create_box2_from_half_extents,
                create_box2_invalid = $create_box2_invalid,
                create_box2_huge = $create_box2_huge,
            }
        })
        .eval()?)
}
