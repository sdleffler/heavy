use na::{TProjective, Transform};

use crate::math::*;

#[derive(Debug, Clone)]
pub struct TransformStack {
    ts: Vec<Matrix4<f32>>,
}

impl Default for TransformStack {
    fn default() -> Self {
        Self::new()
    }
}

impl TransformStack {
    pub fn new() -> Self {
        Self {
            ts: vec![Matrix4::identity()],
        }
    }

    #[inline]
    pub fn top(&self) -> &Matrix4<f32> {
        self.ts.last().unwrap()
    }

    #[inline]
    pub fn top_mut(&mut self) -> &mut Matrix4<f32> {
        self.ts.last_mut().unwrap()
    }

    #[inline]
    pub fn push(&mut self, tx: impl Into<Option<Matrix4<f32>>>) -> &mut Self {
        self.ts.push(tx.into().unwrap_or(*self.top()));
        self
    }

    #[inline]
    pub fn pop(&mut self) -> &mut Self {
        self.ts.pop().expect("popped empty transform stack");
        self
    }

    #[inline]
    pub fn scope<T, F>(&mut self, thunk: F) -> T
    where
        F: FnOnce(&mut TransformStack) -> T,
    {
        self.push(None);
        let result = thunk(self);
        self.pop();
        result
    }

    pub fn apply_transform(&mut self, tx: Matrix4<f32>) -> &mut Self {
        *self.top_mut() *= tx;
        self
    }

    pub fn inverse_transform_point2(&self, screen: Point2<f32>) -> Point2<f32> {
        Transform::<_, TProjective, 3>::from_matrix_unchecked(*self.top())
            .inverse_transform_point(&Point3::from(screen.coords.push(0.)))
            .xy()
    }

    pub fn origin(&mut self) -> &mut Self {
        *self.top_mut() = Matrix4::identity();
        self
    }

    pub fn replace_transform(&mut self, tx: Matrix4<f32>) -> &mut Self {
        *self.top_mut() = tx;
        self
    }

    pub fn rotate2(&mut self, angle: f32) -> &mut Self {
        *self.top_mut() *= homogeneous_mat3_to_mat4(&Rotation2::new(angle).to_homogeneous());
        self
    }

    pub fn scale2(&mut self, scale: Vector2<f32>) -> &mut Self {
        *self.top_mut() *= Matrix4::new_nonuniform_scaling(&scale.push(1.));
        self
    }

    pub fn shear2(&mut self, shear: Vector2<f32>) -> &mut Self {
        let shear_mat2 = Matrix2::new(1., shear.x, shear.y, 1.);
        *self.top_mut() *= homogeneous_mat3_to_mat4(&shear_mat2.to_homogeneous());
        self
    }

    pub fn transform_point2(&self, point: Point2<f32>) -> Point2<f32> {
        self.top()
            .transform_point(&Point3::from(point.coords.push(0.)))
            .xy()
    }

    pub fn translate2(&mut self, translation: Vector2<f32>) -> &mut Self {
        let top_mut = self.top_mut();
        *top_mut *= homogeneous_mat3_to_mat4(
            &Translation2::new(translation.x, translation.y).to_homogeneous(),
        );
        self
    }
}
