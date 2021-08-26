use anyhow::*;
use arc_swap::{ArcSwap, Cache};
use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
    sync::Arc,
};

pub type Guard<T> = arc_swap::Guard<Arc<T>>;

pub trait Key: Eq + Hash {}
impl<T: Eq + Hash> Key for T {}

pub trait Loader<K, T> {
    fn load(&mut self, key: &K) -> Result<Handle<T>>;
}

pub struct SwappableCache<K: Key, T, L: Loader<K, T>> {
    loader: L,
    map: HashMap<K, Handle<T>>,
}

impl<K: Key, T, L: Loader<K, T>> SwappableCache<K, T, L> {
    pub fn new(loader: L) -> Self {
        Self {
            loader,
            map: HashMap::new(),
        }
    }

    pub fn get_or_load(&mut self, key: K) -> Result<Handle<T>> {
        match self.map.entry(key) {
            Entry::Occupied(occupied) => Ok(occupied.get().clone()),
            Entry::Vacant(vacant) => {
                let loaded = self.loader.load(vacant.key())?;
                Ok(vacant.insert(loaded).clone())
            }
        }
    }

    pub fn reload(&mut self, key: &K) -> Result<()> {
        let reloaded = self.loader.load(key)?;
        self.map[key].inner.store(reloaded.inner.load_full());

        Ok(())
    }

    pub fn reload_all(&mut self) -> Result<()> {
        for (key, handle) in self.map.iter_mut() {
            handle.inner.store(self.loader.load(key)?.inner.load_full());
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Handle<T> {
    inner: Arc<ArcSwap<T>>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Handle<T> {
    pub fn new(t: T) -> Self {
        Self {
            inner: Arc::new(ArcSwap::new(Arc::new(t))),
        }
    }

    pub fn from_arc(arc: Arc<T>) -> Self {
        Self {
            inner: Arc::new(ArcSwap::new(arc)),
        }
    }

    pub fn into_cached(self) -> CacheRef<T> {
        CacheRef {
            inner: Cache::new(self.inner),
        }
    }

    pub fn ptr_eq(lhs: &Self, rhs: &Self) -> bool {
        Arc::ptr_eq(&lhs.inner.load(), &rhs.inner.load())
    }
}

#[derive(Debug)]
pub struct CacheRef<T> {
    inner: Cache<Arc<ArcSwap<T>>, Arc<T>>,
}

impl<T> Clone for CacheRef<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> CacheRef<T> {
    pub fn new_uncached(object: T) -> Self {
        Self {
            inner: Cache::new(Arc::new(ArcSwap::new(Arc::new(object)))),
        }
    }

    pub fn get(&self) -> Guard<T> {
        self.inner.arc_swap().load()
    }

    pub fn get_cached(&mut self) -> &T {
        self.inner.load()
    }

    pub fn ptr_eq(lhs: &Self, rhs: &Self) -> bool {
        Arc::ptr_eq(&lhs.inner.arc_swap().load(), &rhs.inner.arc_swap().load())
    }

    pub fn ptr_eq_cached(lhs: &mut Self, rhs: &mut Self) -> bool {
        Arc::ptr_eq(lhs.inner.load(), rhs.inner.load())
    }
}
