use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub trait RwLockExt {
    type Inner: ?Sized;

    fn borrow(&self) -> RwLockReadGuard<Self::Inner>;
    fn borrow_mut(&self) -> RwLockWriteGuard<Self::Inner>;
}

impl<T: ?Sized> RwLockExt for RwLock<T> {
    type Inner = T;

    fn borrow(&self) -> RwLockReadGuard<Self::Inner> {
        self.try_read().unwrap()
    }

    fn borrow_mut(&self) -> RwLockWriteGuard<Self::Inner> {
        self.try_write().unwrap()
    }
}
