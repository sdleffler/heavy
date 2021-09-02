//! A convenient reference-counted smart pointer type w/ support for concurrent interior mutability.

use std::{
    fmt,
    marker::Unsize,
    ops::{CoerceUnsized, Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::mlua::prelude::*;

/// A "strong" shared reference to a value with interior mutability.
pub struct Shared<T: ?Sized> {
    inner: Arc<RwLock<T>>,
}

/// A "weak" shared reference to a value with interior mutability.
pub struct Weak<T: ?Sized> {
    inner: std::sync::Weak<RwLock<T>>,
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Shared<U>> for Shared<T> {}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.inner.try_read() {
            Ok(t) => t.fmt(f),
            Err(_) => write!(f, "Shared<{}>(_)", std::any::type_name::<T>()),
        }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Weak<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T: ?Sized> Clone for Shared<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: ?Sized> Clone for Weak<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Default> Default for Shared<T> {
    #[inline]
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> Default for Weak<T> {
    #[inline]
    fn default() -> Self {
        Weak::new()
    }
}

impl<T> Shared<T> {
    /// Construct a new [`Shared<T>`].
    #[inline]
    pub fn new(t: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(t)),
        }
    }
}

impl<T> Weak<T> {
    /// Construct a new [`Weak<T>`] which cannot be upgraded.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: std::sync::Weak::new(),
        }
    }
}

impl<T: ?Sized> Shared<T> {
    /// Construct a weak reference to the same shared value.
    #[inline]
    pub fn downgrade(&self) -> Weak<T> {
        Weak {
            inner: Arc::downgrade(&self.inner),
        }
    }

    /// If this is the only strong reference to this value and there are no weak references to this
    /// value, mutably borrow the interior value.
    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        Arc::get_mut(&mut self.inner).map(|rwlock| rwlock.get_mut().unwrap())
    }

    /// Immutably borrow the interior value. Panics if the value is already mutably borrowed.
    #[inline]
    pub fn borrow(&self) -> Ref<'_, T> {
        Ref(self.inner.try_read().unwrap())
    }

    /// Mutably borrow the interior value. Panics if the value is already borrowed.
    #[inline]
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        RefMut(self.inner.try_write().unwrap())
    }

    /// Attempt to immutably borrow the interior value. Returns `None` if the value is already
    /// mutably borrowed.
    #[inline]
    pub fn try_borrow(&self) -> Option<Ref<'_, T>> {
        self.inner.try_read().ok().map(Ref)
    }

    /// Attempt to mutably borrow the interior value. Returns `None` if the value is already
    /// borrowed.
    #[inline]
    pub fn try_borrow_mut(&self) -> Option<RefMut<'_, T>> {
        self.inner.try_write().ok().map(RefMut)
    }

    /// Convert this strong reference into an object which represents both a strong reference to the
    /// interior value *and* an ongoing immutable borrow of it. The immutable borrow is released
    /// when the [`OwnedRef`] is dropped. Panics if the interior value is already mutably borrowed.
    #[inline]
    pub fn owned_borrow(self) -> OwnedRef<'static, T> {
        let guard = self.borrow();
        OwnedRef {
            borrower: unsafe { std::mem::transmute::<Ref<T>, Ref<'static, T>>(guard) },
            _owner: self,
        }
    }

    /// Convert this strong reference into an object which represents both a strong reference to the
    /// interior value *and* an ongoing mutable borrow of it. The mutable borrow is released when
    /// the [`OwnedRef`] is dropped. Panics if the interior value is already borrowed.
    #[inline]
    pub fn owned_borrow_mut(self) -> OwnedRefMut<'static, T> {
        let guard = unsafe { std::mem::transmute::<&Self, &'static Self>(&self) }.borrow_mut();
        OwnedRefMut {
            borrower: guard,
            _owner: self,
        }
    }

    /// Like [`Shared::borrow`] but if the value is already mutably borrowed, then instead of
    /// panicking, block the current thread until the mutable borrow ends.
    #[inline]
    pub fn borrow_blocking(&self) -> Ref<'_, T> {
        Ref(self.inner.read().unwrap())
    }

    /// Like [`Shared::borrow_mut`], but if the value is already borrowed, then instead of
    /// panicking, block the current thread until the borrow ends.
    #[inline]
    pub fn borrow_mut_blocking(&self) -> RefMut<'_, T> {
        RefMut(self.inner.write().unwrap())
    }

    /// Combination of [`Shared::borrow_blocking`] and [`Shared::owned_borrow`].
    #[inline]
    pub fn owned_borrow_blocking(self) -> OwnedRef<'static, T> {
        let guard = self.borrow_blocking();
        OwnedRef {
            borrower: unsafe { std::mem::transmute::<Ref<T>, Ref<'static, T>>(guard) },
            _owner: self,
        }
    }

    /// Combination of [`Shared::borrow_mut_blocking`] and [`Shared::owned_borrow_mut`].
    #[inline]
    pub fn owned_borrow_mut_blocking(self) -> OwnedRefMut<'static, T> {
        let guard =
            unsafe { std::mem::transmute::<&Self, &'static Self>(&self) }.borrow_mut_blocking();
        OwnedRefMut {
            borrower: guard,
            _owner: self,
        }
    }
}

impl<T: ?Sized> Weak<T> {
    /// Try to upgrade this weak reference to a strong reference. Returns `None` if the weak
    /// reference was constructed with [`Weak::new`] or if the value the reference points to has
    /// already been dropped.
    #[inline]
    pub fn try_upgrade(&self) -> Option<Shared<T>> {
        self.inner.upgrade().map(|inner| Shared { inner })
    }

    /// Upgrade this weak reference to a strong reference. Will panic if the weak reference was
    /// constructed with [`Weak::new`] or if the value the reference points to has already been
    /// dropped.
    #[inline]
    pub fn upgrade(&self) -> Shared<T> {
        self.try_upgrade().unwrap()
    }

    /// Attempt to upgrade this weak reference to a strong reference and then immutably borrow it.
    /// If successful, returns a reference which owns an upgraded strong reference and an immutable
    /// reference to the interior value. Will panic if the interior value is already mutably
    /// borrowed.
    #[inline]
    pub fn borrow(&self) -> OwnedRef<'_, T> {
        let upgraded = self.upgrade();
        let guard = upgraded.borrow();
        OwnedRef {
            borrower: unsafe { std::mem::transmute::<Ref<T>, Ref<'_, T>>(guard) },
            _owner: upgraded,
        }
    }

    /// Attempt to upgrade this weak reference to a strong reference and then mutably borrow it. If
    /// successful, returns a reference which owns an upgraded strong reference and a mutable
    /// reference to the interior value. Will panic if the interior value is already borrowed.
    #[inline]
    pub fn borrow_mut(&self) -> OwnedRefMut<'_, T> {
        let upgraded = self.upgrade();
        let guard =
            unsafe { std::mem::transmute::<&Shared<T>, &'_ Shared<T>>(&upgraded) }.borrow_mut();
        OwnedRefMut {
            borrower: guard,
            _owner: upgraded,
        }
    }

    /// Attempt to upgrade this weak reference to a strong reference and then immutably borrow it.
    /// If successful, returns a reference which owns an upgraded strong reference and an immutable
    /// reference to the interior value. Returns `None` if the weak reference can't be upgraded or
    /// the interior value is already mutably borrowed.
    #[inline]
    pub fn try_borrow(&self) -> Option<OwnedRef<'_, T>> {
        let upgraded = self.try_upgrade()?;
        let guard = upgraded.try_borrow()?;
        Some(OwnedRef {
            borrower: unsafe { std::mem::transmute::<Ref<T>, Ref<'_, T>>(guard) },
            _owner: upgraded,
        })
    }

    /// Attempt to upgrade this weak reference to a strong reference and then mutably borrow it. If
    /// successful, returns a reference which owns an upgraded strong reference and a mutable
    /// reference to the interior value. Returns `None` if the weak reference can't be upgraded or
    /// the interior value is already borrowed.
    #[inline]
    pub fn try_borrow_mut(&self) -> Option<OwnedRefMut<'_, T>> {
        let upgraded = self.try_upgrade()?;
        let guard = unsafe { std::mem::transmute::<&Shared<T>, &'_ Shared<T>>(&upgraded) }
            .try_borrow_mut()?;
        Some(OwnedRefMut {
            borrower: guard,
            _owner: upgraded,
        })
    }

    /// Like [`Weak::borrow`] but if the value is already mutably borrowed, then instead of
    /// panicking, block the current thread until the mutable borrow ends. Will panic if the weak
    /// reference can't be upgraded.
    #[inline]
    pub fn borrow_blocking(&self) -> OwnedRef<'_, T> {
        let upgraded = self.upgrade();
        let guard = upgraded.borrow_blocking();
        OwnedRef {
            borrower: unsafe { std::mem::transmute::<Ref<T>, Ref<'_, T>>(guard) },
            _owner: upgraded,
        }
    }

    /// Like [`Weak::borrow_mut`] but if the value is already borrowed, then instead of
    /// panicking, block the current thread until the borrow ends. Will panic if the weak reference
    /// can't be upgraded.
    #[inline]
    pub fn borrow_mut_blocking(&self) -> OwnedRefMut<'_, T> {
        let upgraded = self.upgrade();
        let guard = unsafe { std::mem::transmute::<&Shared<T>, &'_ Shared<T>>(&upgraded) }
            .borrow_mut_blocking();
        OwnedRefMut {
            borrower: guard,
            _owner: upgraded,
        }
    }

    /// Combination of [`Weak::upgrade`] followed by [`Shared::owned_borrow`].
    #[inline]
    pub fn owned_borrow(&self) -> OwnedRef<'static, T> {
        self.upgrade().owned_borrow()
    }

    /// Combination of [`Weak::upgrade`] followed by [`Shared::owned_borrow_mut`].
    #[inline]
    pub fn owned_borrow_mut(&self) -> OwnedRefMut<'static, T> {
        self.upgrade().owned_borrow_mut()
    }

    /// Combination of [`Weak::upgrade`] followed by [`Shared::owned_borrow_blocking`].
    #[inline]
    pub fn owned_borrow_blocking(&self) -> OwnedRef<'static, T> {
        self.upgrade().owned_borrow_blocking()
    }

    /// Combination of [`Weak::upgrade`] followed by [`Shared::owned_borrow_mut_blocking`].
    #[inline]
    pub fn owned_borrow_mut_blocking(&self) -> OwnedRefMut<'static, T> {
        self.upgrade().owned_borrow_mut_blocking()
    }
}

impl<'lua, T: LuaUserData + Send + Sync + 'static> ToLua<'lua> for Shared<T> {
    #[inline]
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        ToLua::to_lua(self.inner, lua)
    }
}

impl<'lua, T: LuaUserData + 'static> FromLua<'lua> for Shared<T> {
    #[inline]
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            inner: FromLua::from_lua(lua_value, lua)?,
        })
    }
}

/// An immutable borrow of a value inside a [`Shared<T>`].
pub struct Ref<'a, T: ?Sized + 'a>(RwLockReadGuard<'a, T>);

/// A mutable borrow of a value inside a [`Shared<T>`].
pub struct RefMut<'a, T: ?Sized + 'a>(RwLockWriteGuard<'a, T>);

/// An immutable borrow of a value inside a [`Shared<T>`] which owns the strong reference that the
/// value is borrowed from.
pub struct OwnedRef<'a, T: ?Sized + 'a> {
    borrower: Ref<'a, T>,
    _owner: Shared<T>,
}

/// A mutable borrow of a value inside a [`Shared<T>`] which owns the strong reference that the
/// value is borrowed from.
pub struct OwnedRefMut<'a, T: ?Sized + 'a> {
    borrower: RefMut<'a, T>,
    _owner: Shared<T>,
}

impl<'a, T: ?Sized + 'a> Deref for Ref<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T: ?Sized + 'a> Deref for RefMut<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T: ?Sized + 'a> Deref for OwnedRef<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.borrower
    }
}

impl<'a, T: ?Sized + 'a> Deref for OwnedRefMut<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.borrower
    }
}

impl<'a, T: ?Sized + 'a> DerefMut for RefMut<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a, T: ?Sized + 'a> DerefMut for OwnedRefMut<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.borrower
    }
}

impl<'a, T: fmt::Debug + ?Sized + 'a> fmt::Debug for Ref<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'a, T: fmt::Debug + ?Sized + 'a> fmt::Debug for RefMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'a, T: fmt::Debug + ?Sized + 'static> fmt::Debug for OwnedRef<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<'a, T: fmt::Debug + ?Sized + 'static> fmt::Debug for OwnedRefMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}
