//! A convenient reference-counted smart pointer type w/ support for concurrent interior mutability.

use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::mlua::prelude::*;

pub struct Shared<T: ?Sized> {
    inner: Arc<RwLock<T>>,
}

pub struct Weak<T: ?Sized> {
    inner: std::sync::Weak<RwLock<T>>,
}

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
    #[inline]
    pub fn new(t: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(t)),
        }
    }
}

impl<T> Weak<T> {
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: std::sync::Weak::new(),
        }
    }
}

impl<T: ?Sized> Shared<T> {
    #[inline]
    pub fn downgrade(&self) -> Weak<T> {
        Weak {
            inner: Arc::downgrade(&self.inner),
        }
    }

    #[inline]
    pub fn borrow_blocking(&self) -> Ref<'_, T> {
        Ref(self.inner.read().unwrap())
    }

    #[inline]
    pub fn borrow_mut_blocking(&self) -> RefMut<'_, T> {
        RefMut(self.inner.write().unwrap())
    }

    #[inline]
    pub fn borrow(&self) -> Ref<'_, T> {
        Ref(self.inner.try_read().unwrap())
    }

    #[inline]
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        RefMut(self.inner.try_write().unwrap())
    }

    #[inline]
    pub fn try_borrow(&self) -> Option<Ref<'_, T>> {
        self.inner.try_read().ok().map(Ref)
    }

    #[inline]
    pub fn try_borrow_mut(&self) -> Option<RefMut<'_, T>> {
        self.inner.try_write().ok().map(RefMut)
    }

    #[inline]
    pub fn owned_borrow(self) -> OwnedRef<'static, T> {
        let guard = self.borrow();
        OwnedRef {
            borrower: unsafe { std::mem::transmute::<Ref<T>, Ref<'static, T>>(guard) },
            _owner: self,
        }
    }

    #[inline]
    pub fn owned_borrow_mut(self) -> OwnedRefMut<'static, T> {
        let guard = unsafe { std::mem::transmute::<&Self, &'static Self>(&self) }.borrow_mut();
        OwnedRefMut {
            borrower: guard,
            _owner: self,
        }
    }

    #[inline]
    pub fn owned_borrow_blocking(self) -> OwnedRef<'static, T> {
        let guard = self.borrow_blocking();
        OwnedRef {
            borrower: unsafe { std::mem::transmute::<Ref<T>, Ref<'static, T>>(guard) },
            _owner: self,
        }
    }

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
    #[inline]
    pub fn try_upgrade(&self) -> Option<Shared<T>> {
        self.inner.upgrade().map(|inner| Shared { inner })
    }

    #[inline]
    pub fn upgrade(&self) -> Shared<T> {
        self.try_upgrade().unwrap()
    }

    #[inline]
    pub fn borrow(&self) -> OwnedRef<'_, T> {
        let upgraded = self.upgrade();
        let guard = upgraded.borrow();
        OwnedRef {
            borrower: unsafe { std::mem::transmute::<Ref<T>, Ref<'_, T>>(guard) },
            _owner: upgraded,
        }
    }

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

    #[inline]
    pub fn try_borrow(&self) -> Option<OwnedRef<'_, T>> {
        let upgraded = self.try_upgrade()?;
        let guard = upgraded.try_borrow()?;
        Some(OwnedRef {
            borrower: unsafe { std::mem::transmute::<Ref<T>, Ref<'_, T>>(guard) },
            _owner: upgraded,
        })
    }

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

    #[inline]
    pub fn borrow_blocking(&self) -> OwnedRef<'_, T> {
        let upgraded = self.upgrade();
        let guard = upgraded.borrow_blocking();
        OwnedRef {
            borrower: unsafe { std::mem::transmute::<Ref<T>, Ref<'_, T>>(guard) },
            _owner: upgraded,
        }
    }

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

    #[inline]
    pub fn owned_borrow(&self) -> OwnedRef<'static, T> {
        self.upgrade().owned_borrow()
    }

    #[inline]
    pub fn owned_borrow_mut(&self) -> OwnedRefMut<'static, T> {
        self.upgrade().owned_borrow_mut()
    }

    #[inline]
    pub fn owned_borrow_blocking(&self) -> OwnedRef<'static, T> {
        self.upgrade().owned_borrow_blocking()
    }

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

pub struct Ref<'a, T: ?Sized + 'a>(RwLockReadGuard<'a, T>);

pub struct RefMut<'a, T: ?Sized + 'a>(RwLockWriteGuard<'a, T>);

pub struct OwnedRef<'a, T: ?Sized + 'a> {
    borrower: Ref<'a, T>,
    _owner: Shared<T>,
}

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
