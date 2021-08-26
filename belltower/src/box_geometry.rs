use hv_friends::{math::*, nc::shape::Cuboid};
use std::ops;
use thunderdome::{Arena, Index};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoxIndex(Index);

#[derive(Debug, Clone, Copy)]
pub struct BoxCollider<T> {
    pub cuboid: Cuboid<f32>,
    pub tx: Isometry2<f32>,
    pub properties: T,
}

impl<T> BoxCollider<T> {
    pub fn to_points(&self) -> [Point2<f32>; 4] {
        let r = self.cuboid.half_extents;
        let (x, y) = (r.x, r.y);
        [
            self.tx * Point2::new(-x, -y),
            self.tx * Point2::new(-x, y),
            self.tx * Point2::new(x, y),
            self.tx * Point2::new(x, -y),
        ]
    }
}

pub struct BoxGeometry<T> {
    boxes: Arena<BoxCollider<T>>,
}

impl<T> BoxGeometry<T> {
    pub fn new() -> Self {
        Self {
            boxes: Arena::new(),
        }
    }

    pub fn insert(&mut self, collider: BoxCollider<T>) -> BoxIndex {
        BoxIndex(self.boxes.insert(collider))
    }

    pub fn remove(&mut self, index: BoxIndex) -> Option<BoxCollider<T>> {
        self.boxes.remove(index.0)
    }

    pub fn clear(&mut self) {
        self.boxes.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = (BoxIndex, &BoxCollider<T>)> {
        self.boxes
            .iter()
            .map(|(index, elem)| (BoxIndex(index), elem))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (BoxIndex, &mut BoxCollider<T>)> {
        self.boxes
            .iter_mut()
            .map(|(index, elem)| (BoxIndex(index), elem))
    }
}

impl<T> ops::Index<BoxIndex> for BoxGeometry<T> {
    type Output = BoxCollider<T>;

    fn index(&self, index: BoxIndex) -> &Self::Output {
        &self.boxes[index.0]
    }
}

impl<T> ops::IndexMut<BoxIndex> for BoxGeometry<T> {
    fn index_mut(&mut self, index: BoxIndex) -> &mut Self::Output {
        &mut self.boxes[index.0]
    }
}
