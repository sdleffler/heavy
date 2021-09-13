//! A hot-swappable resource cache with smart pointer types to access values in it.

use arc_swap::{ArcSwap, Cache};
use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
    sync::Arc,
};

use crate::error::*;

/// Types which can be converted from mutable references to immutable references. Similar to an
/// unholy mixture of [`AsRef`] and [`AsMut`]. Can be used to generically quantify over `T`,
/// [`Shared<T>`], and [`CacheRef<T>`].
///
/// [`Shared<T>`]: crate::shared::Shared
pub trait AsCached<T> {
    /// Get an immutable reference to a `T` from this type, potentially using a cache along the way
    /// to improve access speed.
    fn as_cached(&mut self) -> &T;
}

impl<T> AsCached<T> for T {
    fn as_cached(&mut self) -> &T {
        self
    }
}

impl<T> AsCached<T> for Handle<T> {
    fn as_cached(&mut self) -> &T {
        self.get_cached()
    }
}

/// The type of a guarded immutable reference to a cached value.
pub type Guard<T> = arc_swap::Guard<Arc<T>>;

/// Trait synonym for types which can be used as keys in a [`SwappableCache`].
pub trait Key: Eq + Hash {}
impl<T: Eq + Hash> Key for T {}

/// Describes an object which can be used to load a cached value.
pub trait Loader<K, T> {
    /// Load the value corresponding to a given key.
    fn load(&mut self, key: &K) -> Result<UncachedHandle<T>>;
}

/// A customizable cache for loading values which may later be re-loaded and swapped out with new
/// values.
pub struct SwappableCache<K: Key, T, L: Loader<K, T>> {
    loader: L,
    map: HashMap<K, UncachedHandle<T>>,
}

impl<K: Key, T, L: Loader<K, T>> SwappableCache<K, T, L> {
    /// Construct an empty cache with the given loader.
    pub fn new(loader: L) -> Self {
        Self {
            loader,
            map: HashMap::new(),
        }
    }

    /// Returns the cached value if present or loads a fresh value and returns a handle to it.
    pub fn get_or_load(&mut self, key: K) -> Result<UncachedHandle<T>> {
        match self.map.entry(key) {
            Entry::Occupied(occupied) => Ok(occupied.get().clone()),
            Entry::Vacant(vacant) => {
                let loaded = self.loader.load(vacant.key())?;
                Ok(vacant.insert(loaded).clone())
            }
        }
    }

    /// Re-load the value corresponding to the given key. After reloading, all handles for this key
    /// will point to the newly loaded value rather than the old one.
    pub fn reload(&mut self, key: &K) -> Result<()> {
        let reloaded = self.loader.load(key)?;
        self.map[key].inner.store(reloaded.inner.load_full());

        Ok(())
    }

    /// Reload all keys.
    pub fn reload_all(&mut self) -> Result<()> {
        for (key, handle) in self.map.iter_mut() {
            handle.inner.store(self.loader.load(key)?.inner.load_full());
        }

        Ok(())
    }
}

/// A shared handle to a possibly cached value.
#[derive(Debug)]
pub struct UncachedHandle<T> {
    inner: Arc<ArcSwap<T>>,
}

impl<T> Clone for UncachedHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> UncachedHandle<T> {
    /// Construct a new [`Handle`] from the value it wraps, whether inside or outside a cache.
    pub fn new(t: T) -> Self {
        Self {
            inner: Arc::new(ArcSwap::new(Arc::new(t))),
        }
    }

    /// Construct a new [`Handle`] from an [`Arc`], reusing the [`Arc`] for the internal shared
    /// reference.
    pub fn from_arc(arc: Arc<T>) -> Self {
        Self {
            inner: Arc::new(ArcSwap::new(arc)),
        }
    }

    /// Load the cached value this handle refers to. If you're going to call this a lot, consider a
    /// [`CacheRef<T>`] instead.
    pub fn load(&self) -> Guard<T> {
        self.inner.load()
    }

    /// Convert this [`Handle`] into a [`CacheRef`], allowing for speedier repeated access.
    pub fn into_cached(self) -> Handle<T> {
        Handle {
            inner: Cache::new(self.inner),
        }
    }

    /// Check if two handles refer to the same value.
    pub fn ptr_eq(lhs: &Self, rhs: &Self) -> bool {
        Arc::ptr_eq(&lhs.inner.load(), &rhs.inner.load())
    }
}

/// Similar to an [`UncachedHandle<T>`] but with an added cached reference which allows for faster
/// access.
#[derive(Debug)]
pub struct Handle<T> {
    inner: Cache<Arc<ArcSwap<T>>, Arc<T>>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Handle<T> {
    /// Construct a [`CacheRef`] which actually does not belong to a cache.
    pub fn new_uncached(object: T) -> Self {
        Self {
            inner: Cache::new(Arc::new(ArcSwap::new(Arc::new(object)))),
        }
    }

    /// Get the inner value. Returns a guard dereferencing to the inner value. If you have mutable
    /// access to the [`CacheRef<T>`], you should use [`CacheRef::get_cached`] instead, as it's
    /// signicantly faster.
    pub fn get(&self) -> Guard<T> {
        self.inner.arc_swap().load()
    }

    /// Quickly check that the cached value is valid and does not need to be reloaded, and then
    /// a reference to it. This is faster than [`CacheRef::get`] and [`Handle::load`], but requires
    /// mutable access to the [`CacheRef<T>`].
    pub fn get_cached(&mut self) -> &T {
        self.inner.load()
    }

    /// Check if two [`CacheRef`]s point to the same value.
    pub fn ptr_eq(lhs: &Self, rhs: &Self) -> bool {
        Arc::ptr_eq(&lhs.inner.arc_swap().load(), &rhs.inner.arc_swap().load())
    }

    /// Check if two [`CacheRef`]s point to the same value, but faster, but requiring mutable
    /// access.
    pub fn ptr_eq_cached(lhs: &mut Self, rhs: &mut Self) -> bool {
        Arc::ptr_eq(lhs.inner.load(), rhs.inner.load())
    }
}
