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
    pub fn translate2(&mut self, v: &Vector2<f32>) -> &mut Self {
        *self.top_mut() *= Translation3::from(v.push(0.)).to_homogeneous();
        self
    }

    #[inline]
    pub fn scale2(&mut self, v: &Vector2<f32>) -> &mut Self {
        *self.top_mut() *= Matrix3::from_diagonal(&v.push(1.)).to_homogeneous();
        self
    }

    #[inline]
    pub fn rotate2(&mut self, angle: f32) -> &mut Self {
        *self.top_mut() *= UnitComplex::new(angle).to_homogeneous().to_homogeneous();
        self
    }

    #[inline]
    pub fn push(&mut self, tx: impl Into<Option<Matrix4<f32>>>) {
        self.ts.push(tx.into().unwrap_or(*self.top()));
    }

    #[inline]
    pub fn pop(&mut self) {
        self.ts.pop().expect("popped empty transform stack");
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
}
