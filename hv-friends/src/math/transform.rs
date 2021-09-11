//! # The [`Transform`] trait and [`Tx`] copyable dynamic transform type.
//!
//! Contains Lua-friendly transform math for 2D and 3D usage.

use hv_core::{prelude::*, xsbox::CopyBox};
use std::{
    any::Any,
    fmt,
    ops::{Div, DivAssign, Mul, MulAssign},
};

use crate::math::*;

macro_rules! lhs_from_matrix_unchecked_def {
    ($name:ident($lhs:ty, $rhs:ty)  $(, $morename:ident($morelhs:ty, $morerhs:ty))* $(,)?) => {
        fn $name(&self, tx: &$rhs) -> Tx<T> {
            Tx::new(<$lhs>::from_matrix_unchecked(self.to_homogeneous_mat4()) * tx)
        }

        lhs_from_matrix_unchecked_def!($( $morename($morelhs, $morerhs) ),*);
    };
    () => {};
}

macro_rules! rhs_from_matrix_unchecked_def {
    ($name:ident($rhs_in:ty, $rhs_out:ty)  $(, $morename:ident($morerhsin:ty, $morerhsout:ty))* $(,)?) => {
        fn $name(&self, tx: &$rhs_in) -> Tx<T> {
            Tx::new(self * <$rhs_out>::from_matrix_unchecked(tx.to_homogeneous_mat4()))
        }

        rhs_from_matrix_unchecked_def!($( $morename($morerhsin, $morerhsout) ),*);
    };
    () => {};
}

macro_rules! mul_def {
    ($name:ident($ty:ty)  $(, $morename:ident($morety:ty))* $(,)?) => {
        fn $name(&self, tx: &$ty) -> Tx<T> {
            Tx::new(self * tx)
        }

        mul_def!($( $morename($morety) ),*);
    };
    () => {};
}

macro_rules! def_3d {
    ($ty:ty) => {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        mul_def! {
            transform3(Transform3<T>),
            projective3(Projective3<T>),
            affine3(Affine3<T>),
        }

        rhs_from_matrix_unchecked_def! {
            transform2(Transform2<T>, Transform3<T>),
            projective2(Projective2<T>, Projective3<T>),
            affine2(Affine2<T>, Affine3<T>),
            similarity2(Similarity2<T>, Affine3<T>),
            isometry2(Isometry2<T>, Affine3<T>),
        }

        fn reset(&mut self) {
            *self = Self::identity();
        }

        fn scale2(&self, v: &Vector2<T>) -> Tx<T> {
            Tx::new(
                self * <$ty>::from_matrix_unchecked(homogeneous_mat3_to_mat4(
                    &Matrix3::new_nonuniform_scaling(v),
                )),
            )
        }

        fn translate2(&self, v: &Vector2<T>) -> Tx<T> {
            Tx::new(
                self * <$ty>::from_matrix_unchecked(homogeneous_mat3_to_mat4(
                    &Matrix3::new_translation(v),
                )),
            )
        }

        fn rotate2(&self, angle: T) -> Tx<T> {
            Tx::new(
                self * <$ty>::from_matrix_unchecked(homogeneous_mat3_to_mat4(
                    &UnitComplex::new(angle).to_homogeneous(),
                )),
            )
        }
    };
}

macro_rules! def_2d {
    ($ty:ty as $as_3d:ty) => {
        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }

        lhs_from_matrix_unchecked_def! {
            transform3($as_3d, Transform3<T>),
            projective3($as_3d, Projective3<T>),
            affine3($as_3d, Affine3<T>),
        }

        mul_def! {
            transform2(Transform2<T>),
            projective2(Projective2<T>),
            affine2(Affine2<T>),
            similarity2(Similarity2<T>),
            isometry2(Isometry2<T>),
        }

        fn reset(&mut self) {
            *self = Self::identity();
        }

        fn scale2(&self, v: &Vector2<T>) -> Tx<T> {
            Tx::new(
                self * na::convert_unchecked::<_, Affine2<T>>(Matrix3::new_nonuniform_scaling(v)),
            )
        }

        fn translate2(&self, v: &Vector2<T>) -> Tx<T> {
            Tx::new(self * Translation2::from(*v))
        }

        fn rotate2(&self, angle: T) -> Tx<T> {
            Tx::new(self * UnitComplex::new(angle))
        }
    };
}

macro_rules! impl_convert {
    ($name:ident($to:ty) $(, $($more:tt)*)?) => {
        fn $name(&self) -> Option<$to> {
            Some(na::convert_ref(self))
        }

        $(impl_convert!($($more)*);)?
    };
    () => {};
}

macro_rules! impl_try_convert {
    ($name:ident($to:ty) $(, $($more:tt)*)?) => {
        fn $name(&self) -> Option<$to> {
            na::try_convert_ref(self)
        }

        $(impl_try_convert!($($more)*);)?
    };
    () => {};
}

macro_rules! impl_fail_convert {
    ($name:ident($to:ty) $(, $($more:tt)*)?) => {
        fn $name(&self) -> Option<$to> {
            None
        }

        $(impl_fail_convert!($($more)*);)?
    };
    () => {};
}

macro_rules! impl_convert_from_mat4 {
    ($name:ident($to:ty) $(, $($more:tt)*)?) => {
        fn $name(&self) -> Option<$to> {
            na::try_convert(self.to_homogeneous_mat4())
        }

        $(impl_convert_from_mat4!($($more)*);)?
    };
    () => {};
}

macro_rules! impl_convert_delegated {
    ($name:ident($to:ty) $(, $($more:tt)*)?) => {
        fn $name(&self) -> Option<$to> {
            self.0.$name()
        }

        $(impl_convert_delegated!($($more)*);)?
    };
    () => {};
}

/// The `Transform` trait is used to describe and dispatch the various types of transformations we
/// might want to use from Lua or Rust. In order for a `Transform` to be used with the [`Tx`] type,
/// it must also be [`Copy`], though we cannot add that to the `Transform` trait because it would
/// cease to be object-safe. (This is still enforced by a bound on [`Tx::new`], though.)
pub trait Transform<T: RealField + Copy>: fmt::Debug + Send + Sync + Any {
    #[doc(hidden)]
    fn as_any(&self) -> &dyn Any;

    #[doc(hidden)]
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// In-place, right-multiply this transform onto `to`. In other words, something like:
    /// ```text
    /// *to *= self;
    /// ```
    ///
    /// Internally, this must dispatch to whatever `.transform` method is most appropriate.
    /// `append_to` mostly exists as a way to dispatch as such, and you usually should have no need
    /// to use it directly; the [`Tx`] type implements the necessary [`Mul`] and [`MulAssign`]
    /// traits which use this internally.
    fn append_to(&self, to: &mut Tx<T>);

    /// Like [`Transform::append_to`], but rather than just right-multiplying `self` on we first
    /// take the inverse of self. So, something like:
    /// ```text
    /// *to *= self.inverse();
    /// ```
    ///
    /// Like `append_to`, this mostly exists as a way to dispatch things and has a default
    /// implementation which should likely be overriden for some performance gain, and also is
    /// internally used by [`Tx`]'s [`Div`] and [`DivAssign`] implementations so you should never
    /// need to use this directly.
    fn append_inverse_to(&self, to: &mut Tx<T>) {
        self.inverse().append_to(to);
    }

    /// Convert this [`Transform`] to the most general form we support, an arbitrary 4x4
    /// transformation matrix.
    fn to_homogeneous_mat4(&self) -> Matrix4<T>;

    /// Right-multiply by a [`Transform2`]. The default implementation dispatches to
    /// [`Transform::transform3`], converting the [`Transform2`] into a 3x3 transformation matrix
    /// and augmenting it with the proper terms to convert it into a 4x4 transformation matrix
    /// before converting that matrix to a [`Transform3`].
    fn transform2(&self, tx: &Transform2<T>) -> Tx<T> {
        self.transform3(&Transform3::from_matrix_unchecked(
            homogeneous_mat3_to_mat4(&tx.to_homogeneous()),
        ))
    }

    /// Right-multiply by a [`Transform3`].
    fn transform3(&self, tx: &Transform3<T>) -> Tx<T>;

    /// Transform a 2D point. The default implementation dispatches to
    /// [`Transform::transform_point3`], using a Z coordinate of zero and then discarding the Z
    /// coordinate of the result.
    fn transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.transform_point3(&Point3::new(pt.x, pt.y, T::zero()))
            .xy()
    }

    /// Transform a 3D point. The default implementation (which should likely be overridden for some
    /// performance gain) converts this transform to a [`Matrix4`] and calls
    /// [`Matrix4::transform_point`].
    fn transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        self.to_homogeneous_mat4().transform_point(pt)
    }

    /// Transform a 2D point by the inverse of this transform. Similarly to
    /// [`Transform::transform_point2`], the default implementation dispatches to
    /// [`Transform::inverse_transform_point3`] using a Z coordinate of zero and then discarding the
    /// Z coordinate of the result.
    fn inverse_transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.inverse_transform_point3(&Point3::new(pt.x, pt.y, T::zero()))
            .xy()
    }

    /// Transform a 3D point by the inverse of this transform.
    fn inverse_transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        self.inverse_transform_point2(&pt.xy())
            .coords
            .push(pt.z)
            .into()
    }

    /// Transform a 2D vector. This does not take into account any translational components of the
    /// transform, treating the vector as a direction with magnitude.
    fn transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.transform_vector3(&v.push(T::zero())).xy()
    }

    /// Transform a 3D vector. This does not take into account any translational components of the
    /// transform, treating the vector as a direction with magnitude.
    fn transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        self.to_homogeneous_mat4().transform_vector(v)
    }

    /// Transform a 2D vector by the inverse of this transform. For a 2D transform this will always
    /// be the inverse of [`Transform::transform_vector2`].
    fn inverse_transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.inverse_transform_vector3(&v.push(T::zero())).xy()
    }

    /// Transform a 3D vector by the inverse of this transform. For a 3D transform this will always
    /// be the inverse of [`Transform::transform_vector3`].
    fn inverse_transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        self.inverse_transform_vector2(&v.xy()).push(v.z)
    }

    /// Transform a position and rotation by this transform. This is like a combination transform of
    /// point and vector, treating the direction of the [`Position2`] as a vector. This should
    /// probably be overridden for performance whenever possible, as the default implementation has
    /// to convert the angle of the [`Position2`] to a unit vector and then normalize the
    /// transformed vector before converting it back to the internal [`UnitComplex`] rotation
    /// encoding used by [`Position2`].
    fn transform_position2(&self, p: &Position2<T>) -> Position2<T> {
        let new_tr = Translation2::from(self.transform_point2(&p.center()).coords);
        let new_dir = self
            .transform_vector2(&Vector2::new(
                p.rotation.cos_angle(),
                p.rotation.sin_angle(),
            ))
            .normalize();
        let new_angle = UnitComplex::from_cos_sin_unchecked(new_dir.x, new_dir.y);

        Position2::from(Isometry2::from_parts(new_tr, new_angle))
    }

    /// For a 2D transform, this will always be the inverse of [`Transform::transform_position2`].
    fn inverse_transform_position2(&self, p: &Position2<T>) -> Position2<T> {
        let new_tr = Translation2::from(self.inverse_transform_point2(&p.center()).coords);
        let new_dir = self
            .inverse_transform_vector2(&Vector2::new(
                p.rotation.cos_angle(),
                p.rotation.sin_angle(),
            ))
            .normalize();
        let new_angle = UnitComplex::from_cos_sin_unchecked(new_dir.x, new_dir.y);

        Position2::from(Isometry2::from_parts(new_tr, new_angle))
    }

    /// Calculate the inverse of this transform.
    fn inverse(&self) -> Tx<T>;

    /// Reset this transform to the identity.
    fn reset(&mut self);

    /// Right-multiply by a [`Projective2`].
    fn projective2(&self, tx: &Projective2<T>) -> Tx<T>;
    /// Right-multiply by a [`Projective3`].
    fn projective3(&self, tx: &Projective3<T>) -> Tx<T>;
    /// Right-multiply by an [`Affine2`].
    fn affine2(&self, tx: &Affine2<T>) -> Tx<T>;
    /// Right-multiply by an [`Affine3`].
    fn affine3(&self, tx: &Affine3<T>) -> Tx<T>;
    /// Right-multiply by a [`Similarity2`].
    fn similarity2(&self, sim: &Similarity2<T>) -> Tx<T>;
    /// Right-multiply by an [`Isometry2`].
    fn isometry2(&self, iso: &Isometry2<T>) -> Tx<T>;
    /// Right-multiply by a 2D rotation given as an angle.
    fn rotate2(&self, f: T) -> Tx<T>;
    /// Right-multiply by a 2D vector of scaling factors.
    fn scale2(&self, v: &Vector2<T>) -> Tx<T>;
    /// Right-multiply by a 2D translation.
    fn translate2(&self, v: &Vector2<T>) -> Tx<T>;

    /// Try to convert this [`Transform`] to a [`Transform2`]. In the case that the transform is
    /// inconvertible (if it is a transform with 3D components, for example) this will return
    /// `None`.
    fn to_transform2(&self) -> Option<Transform2<T>>;

    /// Try to convert this [`Transform`] to a [`Transform3`].
    fn to_transform3(&self) -> Option<Transform3<T>>;

    /// Try to convert this [`Transform`] to a [`Projective2`]. In the case that the transform is
    /// inconvertible (if it is a non-invertible transform or has 3D components, for example) this
    /// will return `None`.
    fn to_projective2(&self) -> Option<Projective2<T>>;

    /// Try to convert this [`Transform`] to a [`Projective3`]. In the case that the transform is
    /// inconvertible (if it is non-invertible, for example) this will return `None`.
    fn to_projective3(&self) -> Option<Projective3<T>>;

    /// Try to convert this [`Transform`] to an [`Affine2`]. In the case that the transform is
    /// inconvertible (if it has projective components or is non-invertible or has 3D components,
    /// for example) this will return `None`.
    fn to_affine2(&self) -> Option<Affine2<T>>;

    /// Try to convert this [`Transform`] to an [`Affine3`]. In the case that the transform is
    /// inconvertible (if it has projective components or is non-invertible, for example) this will
    /// return `None`.
    fn to_affine3(&self) -> Option<Affine3<T>>;

    /// Try to convert this [`Transform`] to a [`Similarity2`]. In the case that the transform is
    /// inconvertible (if it has affine or projective or 3D components or is non-invertible, for
    /// example) this will return `None`.
    fn to_similarity2(&self) -> Option<Similarity2<T>>;

    /// Try to convert this [`Transform`] to an [`Isometry2`]. In the case that the transform is
    /// inconvertible (if it has scaling or affine or projective or 3D components or is
    /// non-invertible, for example) this will return `None`.
    fn to_isometry2(&self) -> Option<Isometry2<T>>;
}

impl<T: RealField + Copy> dyn Transform<T> {
    /// Try to downcast this [`Transform`] to a specific transform type.
    pub fn downcast_ref<U: Transform<T> + Copy>(&self) -> Option<&U> {
        self.as_any().downcast_ref()
    }

    /// Try to mutably downcast this [`Transform`] to a specific transform type.
    pub fn downcast_mut<U: Transform<T> + Copy>(&mut self) -> Option<&mut U> {
        self.as_any_mut().downcast_mut()
    }
}

impl<T: RealField + Copy> Transform<T> for Transform3<T> {
    fn append_to(&self, to: &mut Tx<T>) {
        *to = to.transform3(self);
    }

    fn to_homogeneous_mat4(&self) -> Matrix4<T> {
        self.to_homogeneous()
    }

    fn transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        self.transform_point(pt)
    }

    fn inverse_transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        na::try_convert_ref::<Self, Projective3<T>>(self)
            .expect("uninvertible transform!")
            .inverse_transform_point(pt)
    }

    fn transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        self.transform_vector(v)
    }

    fn inverse_transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        na::try_convert_ref::<Self, Projective3<T>>(self)
            .expect("uninvertible transform!")
            .inverse_transform_vector(v)
    }

    fn inverse(&self) -> Tx<T> {
        Tx::new(
            na::try_convert_ref::<Self, Projective3<T>>(self)
                .expect("uninvertible transform!")
                .inverse(),
        )
    }

    def_3d!(Transform3<T>);

    impl_convert! {
        to_transform3(Transform3<T>),
    }

    impl_try_convert! {
        to_projective3(Projective3<T>),
        to_affine3(Affine3<T>),
    }

    impl_fail_convert! {
        to_transform2(Transform2<T>),
        to_projective2(Projective2<T>),
        to_affine2(Affine2<T>),
        to_similarity2(Similarity2<T>),
        to_isometry2(Isometry2<T>),
    }
}

impl<T: RealField + Copy> Transform<T> for Projective3<T> {
    fn append_to(&self, to: &mut Tx<T>) {
        *to = to.projective3(self);
    }

    fn to_homogeneous_mat4(&self) -> Matrix4<T> {
        self.to_homogeneous()
    }

    fn transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        self.transform_point(pt)
    }

    fn inverse_transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        self.inverse_transform_point(pt)
    }

    fn transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        self.transform_vector(v)
    }

    fn inverse_transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        self.inverse_transform_vector(v)
    }

    fn inverse(&self) -> Tx<T> {
        Tx::new(Projective3::inverse(*self))
    }

    def_3d!(Projective3<T>);

    impl_convert! {
        to_transform3(Transform3<T>),
        to_projective3(Projective3<T>),
    }

    impl_try_convert! {
        to_affine3(Affine3<T>),
    }

    impl_fail_convert! {
        to_transform2(Transform2<T>),
        to_projective2(Projective2<T>),
        to_affine2(Affine2<T>),
        to_similarity2(Similarity2<T>),
        to_isometry2(Isometry2<T>),
    }
}

impl<T: RealField + Copy> Transform<T> for Affine3<T> {
    fn append_to(&self, to: &mut Tx<T>) {
        *to = to.affine3(self);
    }

    fn to_homogeneous_mat4(&self) -> Matrix4<T> {
        self.to_homogeneous()
    }

    fn transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        self.transform_point(pt)
    }

    fn inverse_transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        self.inverse_transform_point(pt)
    }

    fn transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        self.transform_vector(v)
    }

    fn inverse_transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        self.inverse_transform_vector(v)
    }

    fn inverse(&self) -> Tx<T> {
        Tx::new(Affine3::inverse(*self))
    }

    def_3d!(Affine3<T>);

    impl_convert! {
        to_transform3(Transform3<T>),
        to_projective3(Projective3<T>),
        to_affine3(Affine3<T>),
    }

    impl_fail_convert! {
        to_transform2(Transform2<T>),
        to_projective2(Projective2<T>),
        to_affine2(Affine2<T>),
        to_similarity2(Similarity2<T>),
        to_isometry2(Isometry2<T>),
    }
}

impl<T: RealField + Copy> Transform<T> for Transform2<T> {
    fn append_to(&self, to: &mut Tx<T>) {
        *to = to.transform2(self);
    }

    fn to_homogeneous_mat4(&self) -> Matrix4<T> {
        homogeneous_mat3_to_mat4(&self.to_homogeneous())
    }

    fn transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.transform_point(pt)
    }

    fn inverse_transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        na::try_convert_ref::<Self, Projective2<T>>(self)
            .expect("uninvertible transform!")
            .inverse_transform_point(pt)
    }

    fn transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.transform_vector(v)
    }

    fn inverse_transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        na::try_convert_ref::<Self, Projective2<T>>(self)
            .expect("uninvertible transform!")
            .inverse_transform_vector(v)
    }

    fn inverse(&self) -> Tx<T> {
        Tx::new(
            na::try_convert_ref::<Self, Projective2<T>>(self)
                .expect("uninvertible transform!")
                .inverse(),
        )
    }

    def_2d!(Transform2<T> as Transform3<T>);

    impl_convert_from_mat4! {
        to_transform3(Transform3<T>),
        to_projective3(Projective3<T>),
        to_affine3(Affine3<T>),
    }

    impl_convert! {
        to_transform2(Transform2<T>),
    }

    impl_try_convert! {
        to_projective2(Projective2<T>),
        to_affine2(Affine2<T>),
        to_similarity2(Similarity2<T>),
        to_isometry2(Isometry2<T>),
    }
}

impl<T: RealField + Copy> Transform<T> for Projective2<T> {
    fn append_to(&self, to: &mut Tx<T>) {
        *to = to.projective2(self);
    }

    fn to_homogeneous_mat4(&self) -> Matrix4<T> {
        homogeneous_mat3_to_mat4(&self.to_homogeneous())
    }

    fn transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.transform_point(pt)
    }

    fn inverse_transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.inverse_transform_point(pt)
    }

    fn transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.transform_vector(v)
    }

    fn inverse_transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.inverse_transform_vector(v)
    }

    fn inverse(&self) -> Tx<T> {
        Tx::new(Projective2::inverse(*self))
    }

    def_2d!(Projective2<T> as Projective3<T>);

    impl_convert_from_mat4! {
        to_transform3(Transform3<T>),
        to_projective3(Projective3<T>),
        to_affine3(Affine3<T>),
    }

    impl_convert! {
        to_transform2(Transform2<T>),
        to_projective2(Projective2<T>),
    }

    impl_try_convert! {
        to_affine2(Affine2<T>),
        to_similarity2(Similarity2<T>),
        to_isometry2(Isometry2<T>),
    }
}

impl<T: RealField + Copy> Transform<T> for Affine2<T> {
    fn append_to(&self, to: &mut Tx<T>) {
        *to = to.affine2(self);
    }

    fn to_homogeneous_mat4(&self) -> Matrix4<T> {
        homogeneous_mat3_to_mat4(&self.to_homogeneous())
    }

    fn transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        let out2 = self.transform_point(&pt.xy());
        Point3::new(out2.x, out2.y, pt.z)
    }

    fn transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.transform_point(pt)
    }

    fn inverse_transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.inverse_transform_point(pt)
    }

    fn transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.transform_vector(v)
    }

    fn inverse_transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.inverse_transform_vector(v)
    }

    fn inverse(&self) -> Tx<T> {
        Tx::new(Affine2::inverse(*self))
    }

    def_2d!(Affine2<T> as Affine3<T>);

    impl_convert_from_mat4! {
        to_transform3(Transform3<T>),
        to_projective3(Projective3<T>),
        to_affine3(Affine3<T>),
    }

    impl_convert! {
        to_transform2(Transform2<T>),
        to_projective2(Projective2<T>),
        to_affine2(Affine2<T>),
    }

    impl_try_convert! {
        to_similarity2(Similarity2<T>),
        to_isometry2(Isometry2<T>),
    }
}

impl<T: RealField + Copy> Transform<T> for Similarity2<T> {
    fn append_to(&self, to: &mut Tx<T>) {
        *to = to.similarity2(self);
    }

    fn to_homogeneous_mat4(&self) -> Matrix4<T> {
        homogeneous_mat3_to_mat4(&self.to_homogeneous())
    }

    fn transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.transform_point(pt)
    }

    fn inverse_transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.inverse_transform_point(pt)
    }

    fn transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.transform_vector(v)
    }

    fn inverse_transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.inverse_transform_vector(v)
    }

    fn inverse(&self) -> Tx<T> {
        Tx::new(self.inverse())
    }

    def_2d!(Similarity2<T> as Affine3<T>);

    impl_convert_from_mat4! {
        to_transform3(Transform3<T>),
        to_projective3(Projective3<T>),
        to_affine3(Affine3<T>),
    }

    impl_convert! {
        to_transform2(Transform2<T>),
        to_projective2(Projective2<T>),
        to_affine2(Affine2<T>),
        to_similarity2(Similarity2<T>),
    }

    impl_try_convert! {
        to_isometry2(Isometry2<T>),
    }
}

impl<T: RealField + Copy> Transform<T> for Isometry2<T> {
    fn append_to(&self, to: &mut Tx<T>) {
        *to = to.isometry2(self);
    }

    fn to_homogeneous_mat4(&self) -> Matrix4<T> {
        homogeneous_mat3_to_mat4(&self.to_homogeneous())
    }

    fn transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.transform_point(pt)
    }

    fn inverse_transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.inverse_transform_point(pt)
    }

    fn transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.transform_vector(v)
    }

    fn inverse_transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.inverse_transform_vector(v)
    }

    fn inverse(&self) -> Tx<T> {
        Tx::new(self.inverse())
    }

    def_2d!(Isometry2<T> as Affine3<T>);

    impl_convert_from_mat4! {
        to_transform3(Transform3<T>),
        to_projective3(Projective3<T>),
        to_affine3(Affine3<T>),
    }

    impl_convert! {
        to_transform2(Transform2<T>),
        to_projective2(Projective2<T>),
        to_affine2(Affine2<T>),
        to_similarity2(Similarity2<T>),
        to_isometry2(Isometry2<T>),
    }
}

/// A [`Transform`] representing the identity/no-op transform. Can be more general and efficient
/// than using a specific transform type's identity value.
#[derive(Debug, Clone, Copy)]
pub struct Identity;

impl<T: RealField + Copy> Transform<T> for Identity {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn append_to(&self, _to: &mut Tx<T>) {}

    fn inverse(&self) -> Tx<T> {
        Tx::new(Identity)
    }

    fn reset(&mut self) {}

    fn to_homogeneous_mat4(&self) -> Matrix4<T> {
        Matrix4::identity()
    }

    fn transform2(&self, tx: &Transform2<T>) -> Tx<T> {
        Tx::new(*tx)
    }

    fn transform3(&self, tx: &Transform3<T>) -> Tx<T> {
        Tx::new(*tx)
    }

    fn projective2(&self, tx: &Projective2<T>) -> Tx<T> {
        Tx::new(*tx)
    }

    fn projective3(&self, tx: &Projective3<T>) -> Tx<T> {
        Tx::new(*tx)
    }

    fn affine2(&self, tx: &Affine2<T>) -> Tx<T> {
        Tx::new(*tx)
    }

    fn affine3(&self, tx: &Affine3<T>) -> Tx<T> {
        Tx::new(*tx)
    }

    fn similarity2(&self, sim: &Similarity2<T>) -> Tx<T> {
        Tx::new(*sim)
    }

    fn isometry2(&self, iso: &Isometry2<T>) -> Tx<T> {
        Tx::new(*iso)
    }

    fn scale2(&self, v: &Vector2<T>) -> Tx<T> {
        Tx::new(Affine2::from_matrix_unchecked(
            Matrix3::new_nonuniform_scaling(v),
        ))
    }

    fn rotate2(&self, angle: T) -> Tx<T> {
        Tx::new(Isometry2::rotation(angle))
    }

    fn translate2(&self, v: &Vector2<T>) -> Tx<T> {
        Tx::new(Isometry2::translation(v.x, v.y))
    }

    fn transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        *pt
    }

    fn transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        *pt
    }

    fn inverse_transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        *pt
    }

    fn inverse_transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        *pt
    }

    fn transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        *v
    }

    fn transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        *v
    }

    fn inverse_transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        *v
    }

    fn inverse_transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        *v
    }

    fn to_transform2(&self) -> Option<Transform2<T>> {
        Some(Transform2::identity())
    }

    fn to_transform3(&self) -> Option<Transform3<T>> {
        Some(Transform3::identity())
    }

    fn to_projective2(&self) -> Option<Projective2<T>> {
        Some(Projective2::identity())
    }

    fn to_projective3(&self) -> Option<Projective3<T>> {
        Some(Projective3::identity())
    }

    fn to_affine2(&self) -> Option<Affine2<T>> {
        Some(Affine2::identity())
    }

    fn to_affine3(&self) -> Option<Affine3<T>> {
        Some(Affine3::identity())
    }

    fn to_similarity2(&self) -> Option<Similarity2<T>> {
        Some(Similarity2::identity())
    }

    fn to_isometry2(&self) -> Option<Isometry2<T>> {
        Some(Isometry2::identity())
    }
}

impl<T: RealField + Copy> Default for Tx<T> {
    fn default() -> Self {
        Self::identity()
    }
}

/// A general, dynamically dispatched, [`Copy`] 2D and 3D transform type.
///
/// Internally, this uses a [`CopyBox`] to allow for a copyable [`Transform<T>`] trait object. It is
/// always slightly larger (by one pointer) than a [`Matrix4`], but internally can store any
/// transform which satisfies [`Transform`]. Currently, the implemented transforms skew heavily
/// towards 2D rather than 3D use but eventually [`Tx`] will be a fully-featured general 2D or 3D
/// transform type useful for conversions between Rust and Lua and general transform-oriented math
/// in Lua.
#[derive(Debug, Clone, Copy)]
pub struct Tx<T: RealField + Copy>(CopyBox<dyn Transform<T>, [T; 16]>);

impl<T: RealField + Copy> Tx<T> {
    /// Create a new [`Tx`] from a given [`Transform`].
    pub fn new(tx: impl Transform<T> + Copy + 'static) -> Self {
        Self(hv_core::xsbox!(tx))
    }

    /// Create a new [`Tx`] containing an identity transform.
    ///
    /// Internally, uses [`Identity`] to represent the most general possible identity transform.
    pub fn identity() -> Self {
        Self::new(Identity)
    }
}

impl<T: RealField + Copy> Transform<T> for Tx<T> {
    fn as_any(&self) -> &dyn Any {
        self.0.as_any()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self.0.as_any_mut()
    }

    fn append_to(&self, to: &mut Tx<T>) {
        self.0.append_to(to);
    }

    fn to_homogeneous_mat4(&self) -> Matrix4<T> {
        self.0.to_homogeneous_mat4()
    }

    fn transform2(&self, tx: &Transform2<T>) -> Tx<T> {
        self.0.transform2(tx)
    }

    fn transform3(&self, tx: &Transform3<T>) -> Tx<T> {
        self.0.transform3(tx)
    }

    fn transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.0.transform_point2(pt)
    }

    fn transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        self.0.transform_point3(pt)
    }

    fn inverse_transform_point2(&self, pt: &Point2<T>) -> Point2<T> {
        self.0.inverse_transform_point2(pt)
    }

    fn inverse_transform_point3(&self, pt: &Point3<T>) -> Point3<T> {
        self.0.inverse_transform_point3(pt)
    }

    fn transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.0.transform_vector2(v)
    }

    fn transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        self.0.transform_vector3(v)
    }

    fn inverse_transform_vector2(&self, v: &Vector2<T>) -> Vector2<T> {
        self.0.inverse_transform_vector2(v)
    }

    fn inverse_transform_vector3(&self, v: &Vector3<T>) -> Vector3<T> {
        self.0.inverse_transform_vector3(v)
    }

    fn inverse(&self) -> Tx<T> {
        self.0.inverse()
    }

    fn reset(&mut self) {
        self.0.reset()
    }

    fn projective2(&self, tx: &Projective2<T>) -> Tx<T> {
        self.0.projective2(tx)
    }

    fn projective3(&self, tx: &Projective3<T>) -> Tx<T> {
        self.0.projective3(tx)
    }

    fn affine2(&self, tx: &Affine2<T>) -> Tx<T> {
        self.0.affine2(tx)
    }

    fn affine3(&self, tx: &Affine3<T>) -> Tx<T> {
        self.0.affine3(tx)
    }

    fn similarity2(&self, sim: &Similarity2<T>) -> Tx<T> {
        self.0.similarity2(sim)
    }

    fn isometry2(&self, iso: &Isometry2<T>) -> Tx<T> {
        self.0.isometry2(iso)
    }

    fn rotate2(&self, f: T) -> Tx<T> {
        self.0.rotate2(f)
    }

    fn translate2(&self, v: &Vector2<T>) -> Tx<T> {
        self.0.translate2(v)
    }

    fn scale2(&self, v: &Vector2<T>) -> Tx<T> {
        self.0.scale2(v)
    }

    impl_convert_delegated! {
        to_transform2(Transform2<T>),
        to_transform3(Transform3<T>),
        to_projective2(Projective2<T>),
        to_projective3(Projective3<T>),
        to_affine2(Affine2<T>),
        to_affine3(Affine3<T>),
        to_similarity2(Similarity2<T>),
        to_isometry2(Isometry2<T>),
    }
}

impl<T: RealField + Copy> MulAssign for Tx<T> {
    fn mul_assign(&mut self, rhs: Self) {
        rhs.append_to(self);
    }
}

impl<T: RealField + Copy> Mul for Tx<T> {
    type Output = Tx<T>;

    fn mul(mut self, rhs: Self) -> Self::Output {
        self *= rhs;
        self
    }
}

impl<'a, T: RealField + Copy> Mul<&'a Tx<T>> for Tx<T> {
    type Output = Tx<T>;

    fn mul(self, rhs: &'a Tx<T>) -> Self::Output {
        self * (*rhs)
    }
}

impl<'a, T: RealField + Copy> Mul<Tx<T>> for &'a Tx<T> {
    type Output = Tx<T>;

    fn mul(self, rhs: Tx<T>) -> Self::Output {
        (*self) * rhs
    }
}

impl<'a, 'b, T: RealField + Copy> Mul<&'b Tx<T>> for &'a Tx<T> {
    type Output = Tx<T>;

    fn mul(self, rhs: &'b Tx<T>) -> Self::Output {
        (*self) * (*rhs)
    }
}

impl<T: RealField + Copy> DivAssign for Tx<T> {
    fn div_assign(&mut self, rhs: Self) {
        rhs.append_inverse_to(self);
    }
}

impl<T: RealField + Copy> Div for Tx<T> {
    type Output = Tx<T>;

    fn div(mut self, rhs: Self) -> Self::Output {
        self /= rhs;
        self
    }
}

impl<'a, T: RealField + Copy> Div<&'a Tx<T>> for Tx<T> {
    type Output = Tx<T>;

    fn div(self, rhs: &'a Tx<T>) -> Self::Output {
        self / (*rhs)
    }
}

impl<'a, T: RealField + Copy> Div<Tx<T>> for &'a Tx<T> {
    type Output = Tx<T>;

    fn div(self, rhs: Tx<T>) -> Self::Output {
        (*self) / rhs
    }
}

impl<'a, 'b, T: RealField + Copy> Div<&'b Tx<T>> for &'a Tx<T> {
    type Output = Tx<T>;

    fn div(self, rhs: &'b Tx<T>) -> Self::Output {
        (*self) / (*rhs)
    }
}

impl<T: RealField + Copy + for<'lua> FromLua<'lua> + for<'lua> ToLua<'lua>> LuaUserData for Tx<T> {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        crate::lua::add_clone_methods(methods);

        methods.add_meta_method(LuaMetaMethod::Mul, |_, this, rhs: Tx<T>| Ok(this * rhs));

        crate::lua::simple_mut(methods, "apply", |lhs, rhs: Tx<T>| (*lhs) *= rhs);
        crate::lua::simple(methods, "inverse", |this, ()| this.inverse());
        crate::lua::simple(methods, "inverse_transform_point2", |this, (x, y)| {
            let out = this.inverse_transform_point2(&Point2::new(x, y));
            (out.x, out.y)
        });
        crate::lua::simple(methods, "inverse_transform_position2", |this, pos2| {
            this.inverse_transform_position2(&pos2)
        });
        crate::lua::simple(methods, "inverse_transform_vector2", |this, (x, y)| {
            let out = this.inverse_transform_vector2(&Vector2::new(x, y));
            (out.x, out.y)
        });
        crate::lua::simple_mut(methods, "reset", |lhs, ()| lhs.reset());
        crate::lua::simple_mut(methods, "rotate2", |lhs, angle| *lhs = lhs.rotate2(angle));
        crate::lua::simple_mut(methods, "scale2", |lhs, (x, maybe_y): (T, Option<T>)| {
            *lhs = lhs.scale2(&Vector2::new(x, maybe_y.unwrap_or(x)))
        });
        crate::lua::simple_mut(methods, "set_transformation", |lhs, rhs| *lhs = rhs);
        crate::lua::simple(methods, "transform_point2", |this, (x, y)| {
            let out = this.transform_point2(&Point2::new(x, y));
            (out.x, out.y)
        });
        crate::lua::simple(methods, "transform_position2", |this, pos2| {
            this.transform_position2(&pos2)
        });
        crate::lua::simple(methods, "transform_vector2", |this, (x, y)| {
            let out = this.transform_vector2(&Vector2::new(x, y));
            (out.x, out.y)
        });
        crate::lua::simple_mut(methods, "translate2", |lhs, (x, y)| {
            *lhs = lhs.translate2(&Vector2::new(x, y))
        });
    }
}
