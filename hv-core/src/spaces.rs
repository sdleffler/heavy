//! An entity-component system focused on having multiple "worlds" ([`Space`]s) in the same running
//! program.
//!
//! It is built on the [`hecs`] ECS, but adds space IDs to [`Object`]s so that they cannot be used
//! with the wrong `Space`.

use std::{cell::RefCell, fmt};

use crate::{
    engine::{LuaExt, LuaResource},
    error::*,
    mlua::prelude::*,
    plugins::{ModuleWrapper, Plugin},
    shared::Shared,
};

use {
    hecs::{Archetype, ArchetypesGeneration, ColumnBatch, MissingComponent, Ref, RefMut},
    thunderdome::{Arena, Index},
};

pub use hecs::{Bundle, Component, DynamicBundle, Query};
use hecs::{QueryItem, QueryOne, With, Without};
use serde::{Deserialize, Serialize};

mod lua;

pub mod object_table;
pub mod serialize;

pub use self::lua::SpaceCache;

/// Possible errors when attempting to access a specific component on an object.
#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("no such object")]
    NoSuchObject,
    #[error("missing component {_0}")]
    MissingComponent(MissingComponent),
    #[error("wrong space")]
    WrongSpace,
}

/// Possible errors when attempting to access an object.
#[derive(Debug, thiserror::Error)]
pub enum ObjectError {
    #[error("no such object")]
    NoSuchObject,
    #[error("wrong space")]
    WrongSpace,
}

impl From<hecs::ComponentError> for ComponentError {
    fn from(hecs_err: hecs::ComponentError) -> Self {
        match hecs_err {
            hecs::ComponentError::NoSuchEntity => Self::NoSuchObject,
            hecs::ComponentError::MissingComponent(specifics) => Self::MissingComponent(specifics),
        }
    }
}

impl From<hecs::NoSuchEntity> for ObjectError {
    fn from(_: hecs::NoSuchEntity) -> Self {
        ObjectError::NoSuchObject
    }
}

/// A resource holding shared references to all spaces spawned from it.
///
/// The [`Spaces`] registry allows us to, given an [`Object`], access the [`Space`] it corresponds
/// to as long as we have access to the engine's global resources. It also acts as an arbiter for
/// producing space IDs which are unique within the [`Engine`]. This is important because our
/// [`Object`], unlike [`hecs::Entity`], is specifically tied to its [`Space`] and so contains a
/// [`SpaceId`]. The [`Spaces`] resource is what initially creates this [`SpaceId`] when
/// constructing a new [`Space`].
pub struct Spaces {
    registry: Arena<Shared<Space>>,
}

impl Spaces {
    fn new() -> Self {
        Self {
            registry: Arena::new(),
        }
    }

    /// Create an empty [`Space`] with a fresh [`SpaceId`].
    pub fn create_space(&mut self) -> Shared<Space> {
        let space = Shared::new(Space::new());
        let space_id = self.registry.insert(space.clone());
        self.registry[space_id].borrow_mut().id = SpaceId(space_id);
        space
    }

    /// Get a shared reference to the [`Space`] with this ID. Will panic if the space does not
    /// exist.
    pub fn get_space(&self, space_id: SpaceId) -> Shared<Space> {
        self.registry[space_id.0].clone()
    }
}

impl LuaUserData for Spaces {}

impl LuaResource for Spaces {
    const REGISTRY_KEY: &'static str = "HV_SPACES";
}

/// An ID uniquely identifying a [`Space`] within the [`Spaces`] resource that originated it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpaceId(Index);

impl SpaceId {
    fn invalid() -> Self {
        Self(Index::from_bits(1 << 32))
    }
}

impl<'lua> ToLua<'lua> for SpaceId {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        LuaLightUserData(self.0.to_bits() as *mut _).to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for SpaceId {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self(Index::from_bits(
            LuaLightUserData::from_lua(lua_value, lua)?.0 as u64,
        )))
    }
}

/// A unique identifier for a set of [`Component`]s in a [`Space`].
///
/// It contains three parts: a [`SpaceId`], a generation, and an index (where the last two are
/// actually part of [`hecs::Entity`].) It represents a single object in a [`Space`] which can at
/// most one component of any type which satisfies the [`Component`] trait. For more information,
/// see [`Space`].
///
/// ## Serialization
///
/// [`Object`] is both [`Serialize`] and [`Deserialize`], with a significant caveat: ***when
/// serialized, [`Object`]s do not retain their [`SpaceId`].*** Both serializing and deserializing
/// an [`Object`] must be done within a specific sort of context which provides a current
/// [`SpaceId`]; during serialization, this [`SpaceId`] will be asserted to match the object's space
/// ID, and if that assertion fails, the serialization will fail. During deserialization, a
/// deserialized [`Object`]'s [`SpaceId`] is set to whatever space ID is in the context it's being
/// deserialized in. This context is created with the function [`with_space_id`].
///
/// ***You almost certainly do not need to do this yourself. This functionality is used within
/// [`serialize`], and if possible, one should avoid dealing with serialized [`Object`]s outside the
/// context of a serialized [`Space`].*** Whenever possible, rely on the [`serialize`] module!
/// [`Object`]s are highly special due to their relationship to Lua and their potential
/// corresponding "object tables", and this machinery exists to help with that.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Object {
    space: SpaceId,
    entity: hecs::Entity,
}

impl Object {
    /// The ID of the [`Space`] this object was spawned in.
    pub fn space(&self) -> SpaceId {
        self.space
    }

    /// The internal index of the entity inside the [`Space`].
    ///
    /// As this index lacks any generational information, it could potentially refer to a dead
    /// entity. Accessing via a slot is unsafe, but can be done with
    /// [`Space::find_find_object_from_id`].
    pub fn slot(&self) -> u32 {
        self.entity.id()
    }

    /// The internal [`hecs::Entity`].
    ///
    /// This contains the generation and index components of the [`Object`]. If you need to
    /// serialize an object outside of the context of a single [`Space`], you should probably be
    /// serializing this.
    pub fn entity(&self) -> hecs::Entity {
        self.entity
    }
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{:?}",
            self.space.0.to_bits() & 0xFFFFFFFF,
            self.space.0.to_bits() >> 32,
            self.entity
        )
    }
}

std::thread_local! {
    #[doc(hidden)]
    pub static CURRENT_SPACE_ID: RefCell<Option<SpaceId>> = RefCell::new(None);
}

impl Serialize for Object {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let current_space_id = CURRENT_SPACE_ID.with(|current| *current.borrow());
        if Some(self.space) != current_space_id {
            return Err(serde::ser::Error::custom(
                "hv Objects must be serialized within a context providing a current space ID!",
            ));
        }

        self.entity.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Object {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let current_space_id = match CURRENT_SPACE_ID.with(|current| *current.borrow()) {
            Some(id) => id,
            None => return Err(serde::de::Error::custom(
                "hv Objects must be deserialized within a context providing a current space ID!",
            )),
        };

        Ok(Self {
            space: current_space_id,
            entity: hecs::Entity::deserialize(deserializer)?,
        })
    }
}

/// Create a context in which `Object`s may be serialized and deserialized.
///
/// You will usually never need this yourself; it's useful strictly for implementing serialization
/// of [`Space`]s and enforcing a couple of rules about them - namely, that spaces can serialize
/// [`Object`]s which they own, and deserializing a space comes with a [`SpaceId`] to use for all
/// objects within it.
pub fn with_space_id<T>(space_id: SpaceId, f: impl FnOnce() -> T) -> T {
    CURRENT_SPACE_ID.with(|current| current.replace(Some(space_id)));
    let t = f();
    CURRENT_SPACE_ID.with(|current| current.replace(None));
    t
}

/// A raw entity, equivalent to an [`Object`] without its [`SpaceId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawEntity(hecs::Entity);

/// An iterator over all objects in a space.
pub struct Iter<'a> {
    id: SpaceId,
    inner: hecs::Iter<'a>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Object;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|entity| Object {
            space: self.id,
            entity: entity.entity(),
        })
    }
}

/// An iterator returning entities spawned from [`Space::spawn_batch`].
pub struct SpawnBatchIter<'a, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    id: SpaceId,
    inner: hecs::SpawnBatchIter<'a, I>,
}

impl<'a, I> Iterator for SpawnBatchIter<'a, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    type Item = Object;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.inner.next()?;
        Some(Object {
            space: self.id,
            entity,
        })
    }
}

/// An iterator returning entities spawned from [`Space::spawn_column_batch`].
pub struct SpawnColumnBatchIter<'a> {
    id: SpaceId,
    inner: hecs::SpawnColumnBatchIter<'a>,
}

impl<'a> Iterator for SpawnColumnBatchIter<'a> {
    type Item = Object;

    fn next(&mut self) -> Option<Self::Item> {
        let entity = self.inner.next()?;
        Some(Object {
            space: self.id,
            entity,
        })
    }
}

/// An iterator returning objects which satisfy a given query over a [`Space`].
pub struct QueryIter<'q, Q: Query> {
    space: SpaceId,
    inner: hecs::QueryIter<'q, Q>,
}

impl<'q, Q: Query> Iterator for QueryIter<'q, Q> {
    type Item = (Object, QueryItem<'q, Q>);

    fn next(&mut self) -> Option<Self::Item> {
        let space = self.space;
        let (entity, q) = self.inner.next()?;
        Some((Object { entity, space }, q))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'q, Q: Query> ExactSizeIterator for QueryIter<'q, Q> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

/// A successful borrow of a given query on a [`Space`], created with [`Space::query`].
pub struct QueryBorrow<'w, Q: Query> {
    space: SpaceId,
    inner: hecs::QueryBorrow<'w, Q>,
}

impl<'w, Q: Query> QueryBorrow<'w, Q> {
    /// Iterate over the returned query items.
    pub fn iter(&mut self) -> QueryIter<'_, Q> {
        QueryIter {
            space: self.space,
            inner: self.inner.iter(),
        }
    }

    /// Efficiently filter the query such that it only returns objects with a `T` component.
    pub fn with<T: Component>(self) -> QueryBorrow<'w, With<T, Q>> {
        QueryBorrow {
            space: self.space,
            inner: self.inner.with(),
        }
    }

    /// Efficiently filter the query such that it only returns objects without a `T` component.
    pub fn without<T: Component>(self) -> QueryBorrow<'w, Without<T, Q>> {
        QueryBorrow {
            space: self.space,
            inner: self.inner.without(),
        }
    }
}

/// A mutably borrowed query on a [`Space`], created with [`Space::query_mut`].
pub struct QueryMut<'q, Q: Query> {
    space: SpaceId,
    inner: hecs::QueryMut<'q, Q>,
}

impl<'q, Q: Query> QueryMut<'q, Q> {
    /// Efficiently filter the query such that it only returns objects with a `T` component.
    pub fn with<T: Component>(self) -> QueryMut<'q, With<T, Q>> {
        QueryMut {
            space: self.space,
            inner: self.inner.with(),
        }
    }

    /// Efficiently filter the query such that it only returns objects without a `T` component.
    pub fn without<T: Component>(self) -> QueryMut<'q, Without<T, Q>> {
        QueryMut {
            space: self.space,
            inner: self.inner.without(),
        }
    }
}

impl<'q, Q: Query> IntoIterator for QueryMut<'q, Q> {
    type Item = <QueryIter<'q, Q> as Iterator>::Item;
    type IntoIter = QueryIter<'q, Q>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIter {
            space: self.space,
            inner: self.inner.into_iter(),
        }
    }
}

/// A container for [`Object`]s and their components.
///
pub struct Space {
    id: SpaceId,

    #[doc(hidden)]
    pub ecs: hecs::World,
}

impl Space {
    fn new() -> Self {
        Self {
            id: SpaceId::invalid(),
            ecs: hecs::World::new(),
        }
    }

    /// Get the unique identifier for this [`Space`].
    pub fn id(&self) -> SpaceId {
        self.id
    }

    fn wrap_entity(&self, entity: hecs::Entity) -> Object {
        Object {
            space: self.id,
            entity,
        }
    }

    /// Spawn an object with a given set of components.
    pub fn spawn(&mut self, components: impl DynamicBundle) -> Object {
        let e = self.ecs.spawn(components);
        self.wrap_entity(e)
    }

    /// Spawn an object with a given [`hecs::Entity`]. If an object exists with that entity inside
    /// the [`Space`], all components belonging to it will be dropped and overwritten with the new
    /// object and its components.
    pub fn spawn_at(&mut self, handle: hecs::Entity, components: impl DynamicBundle) -> Object {
        self.ecs.spawn_at(handle, components);
        self.wrap_entity(handle)
    }

    /// Spawn a number of entities which are statically known to have the same type. This is much
    /// more efficient than calling [`Space::spawn`] many times, because it can allocate all the
    /// necessary space for the batch in one go.
    pub fn spawn_batch<I>(&mut self, iter: I) -> SpawnBatchIter<I::IntoIter>
    where
        I: IntoIterator,
        I::Item: Bundle + 'static,
    {
        let id = self.id;
        let inner = self.ecs.spawn_batch(iter);
        SpawnBatchIter { id, inner }
    }

    /// An even more efficient batch spawning method than [`Space::spawn_batch`], and capable of
    /// being called with a dynamically typed batch. This is roughly what's used under the hood when
    /// deserializing a [`World`].
    pub fn spawn_column_batch(&mut self, batch: ColumnBatch) -> SpawnColumnBatchIter {
        let id = self.id;
        let inner = self.ecs.spawn_column_batch(batch);
        SpawnColumnBatchIter { id, inner }
    }

    /// Reserve a number of [`Object`] handles for later usage, such as inserting components onto
    /// them. This does not immediately create the [`Object`]s; it only reserves their identifiers.
    /// The objects will "become real" immediately before an operation like [`Space::insert`] or
    /// [`Space::despawn`], and can be used by inserting components on them with [`Space::insert`]
    /// or [`Space::insert_one`].
    pub fn reserve_objects(
        &self,
        count: u32,
    ) -> impl Iterator<Item = Object> + ExactSizeIterator + '_ {
        let id = self.id;
        self.ecs
            .reserve_entities(count)
            .map(move |entity| Object { space: id, entity })
    }

    /// Despawn an [`Object`], dropping all components attached to it. Any further attempt to access
    /// a despawned [`Object`] will result in an error indicating the object is not found within the
    /// space.
    pub fn despawn(&mut self, object: Object) -> Result<(), ObjectError> {
        if self.id != object.space {
            Err(ObjectError::WrongSpace)
        } else {
            self.ecs.despawn(object.entity).map_err(ObjectError::from)
        }
    }

    /// Reserve a single [`Object`]; see [`Space::reserve_objects`].
    pub fn reserve_object(&self) -> Object {
        self.wrap_entity(self.ecs.reserve_entity())
    }

    /// Reserve storage for `additional` more objects with the given components.
    pub fn reserve<T: Bundle + 'static>(&mut self, additional: u32) {
        self.ecs.reserve::<T>(additional);
    }

    /// Clear the [`Space`], despawning all objects in it and dropping all components attached to
    /// them. The allocated memory inside the space is preserved and can be re-used.
    pub fn clear(&mut self) {
        self.ecs.clear()
    }

    /// Test whether an [`Object`] refers to a live object in this space.
    pub fn contains(&self, object: Object) -> bool {
        object.space == self.id && self.ecs.contains(object.entity)
    }

    /// Attempt to borrow all objects which satisfy the given query, and if successful, return a
    /// [`QueryBorrow`] representing the query, convertible into a [`QueryIter`].
    pub fn query<Q: Query>(&self) -> QueryBorrow<'_, Q> {
        QueryBorrow {
            space: self.id,
            inner: self.ecs.query(),
        }
    }

    /// Similar to [`Space::query`], but skips the dynamic borrow checking step because a mutable
    /// borrow on the [`Space`] means that we're guaranteed no one else is accessing it right now.
    pub fn query_mut<Q: Query>(&mut self) -> QueryMut<'_, Q> {
        QueryMut {
            space: self.id,
            inner: self.ecs.query_mut(),
        }
    }

    /// Execute a query on a single object. Will panic if borrowing the object would conflict with
    /// another currently executed query borrow.
    pub fn query_one<Q: Query>(&self, object: Object) -> Result<QueryOne<'_, Q>, ObjectError> {
        if self.id != object.space {
            return Err(ObjectError::WrongSpace);
        }

        self.ecs.query_one(object.entity).map_err(ObjectError::from)
    }

    /// Similar to [`Space::query_one`], but skips the dynamic borrow checking step because a
    /// mutable borrow on the [`Space`] means that we're guaranteed no one else is accessing it
    /// right now.
    pub fn query_one_mut<Q: Query>(
        &mut self,
        object: Object,
    ) -> Result<QueryItem<'_, Q>, ObjectError> {
        if self.id != object.space {
            return Err(ObjectError::WrongSpace);
        }

        self.ecs
            .query_one_mut::<Q>(object.entity)
            .map_err(|_| ObjectError::NoSuchObject)
    }

    /// Attempt to get a reference to a single component on a specific object. Panics if this would
    /// violate the borrowing rules.
    pub fn get<T: Component>(&self, object: Object) -> Result<Ref<'_, T>, ComponentError> {
        if self.id != object.space {
            return Err(ComponentError::WrongSpace);
        }

        self.ecs
            .get::<T>(object.entity)
            .map_err(ComponentError::from)
    }

    /// Attempt to get a mutable reference to a single component on a specific object. Panics if
    /// this would violate the borrowing rules.
    pub fn get_mut<T: Component>(&self, object: Object) -> Result<RefMut<'_, T>, ComponentError> {
        if self.id != object.space {
            return Err(ComponentError::WrongSpace);
        }

        self.ecs
            .get_mut::<T>(object.entity)
            .map_err(ComponentError::from)
    }

    // pub fn entity(&self, entity: Entity) -> Result<EntityRef<'_>, NoSuchEntity>

    /// Recover a full [`Object`] handle given the raw index of an object.
    ///
    /// # Safety
    ///
    /// Using this method with an [`Object`] that is already despawned or was never created will
    /// result in undefined behavior.
    pub unsafe fn find_object_from_id(&self, id: u32) -> Object {
        let entity = self.ecs.find_entity_from_id(id);
        self.wrap_entity(entity)
    }

    /// Recover a full [`Object`] handle given a raw [`hecs::Entity`].
    ///
    /// Unlike [`Space::find_object_from_id`], this method is safe and under normal conditions will
    /// never produce undefined behavior. Will return `None` if the entity is not contained in the
    /// underlying [`hecs::World`]; otherwise, will attach the space's ID to the entity, creating an
    /// [`Object`] referring to the given entity within the [`Space`].
    pub fn find_object_from_entity(&self, entity: hecs::Entity) -> Option<Object> {
        if self.ecs.contains(entity) {
            Some(Object {
                space: self.id,
                entity,
            })
        } else {
            None
        }
    }

    /// Iterate over all [`Object`]s in the [`Space`].
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            id: self.id,
            inner: self.ecs.iter(),
        }
    }

    /// Insert a set of components on a given [`Object`]. If any component is already on the object,
    /// the older component will be dropped and replaced with the new value.
    pub fn insert(
        &mut self,
        object: Object,
        components: impl DynamicBundle,
    ) -> Result<(), ObjectError> {
        if self.id != object.space {
            return Err(ObjectError::WrongSpace);
        }

        self.ecs
            .insert(object.entity, components)
            .or(Err(ObjectError::WrongSpace))
    }

    /// Insert a single component on a given [`Object`]. Slightly faster than [`Space::insert`] if
    /// you're only inserting one component.
    pub fn insert_one(
        &mut self,
        object: Object,
        component: impl Component,
    ) -> Result<(), ObjectError> {
        if self.id != object.space {
            return Err(ObjectError::WrongSpace);
        }

        self.ecs
            .insert_one(object.entity, component)
            .or(Err(ObjectError::WrongSpace))
    }

    /// Remove a bundle of components from a given [`Object`]. If successful, the entire bundle is
    /// returned.
    pub fn remove<T: Bundle + 'static>(&mut self, object: Object) -> Result<T, ComponentError> {
        if self.id != object.space {
            return Err(ComponentError::WrongSpace);
        }

        self.ecs.remove(object.entity).map_err(ComponentError::from)
    }

    /// Remove a single component from a given [`Object`].
    pub fn remove_one<T: Component>(&mut self, object: Object) -> Result<T, ComponentError> {
        if self.id != object.space {
            return Err(ComponentError::WrongSpace);
        }

        self.ecs
            .remove_one(object.entity)
            .map_err(ComponentError::from)
    }

    /// Borrows the `T` component of the given [`Object`], bypassing all safety checks.
    ///
    /// # Safety
    ///
    /// The object must have been spawned from this [`Space`], and no unique borrow of the same
    /// component of the object may be live at the same time as the returned reference.
    pub unsafe fn get_unchecked<T: Component>(&self, object: Object) -> Result<&T, ComponentError> {
        self.ecs
            .get_unchecked(object.entity)
            .map_err(ComponentError::from)
    }

    /// Mutably borrows the `T` component of the given [`Object`], bypassing all safety checks.
    ///
    /// # Safety
    ///
    /// The object must have been spawned from this [`Space`], and no borrow of the same component
    /// of the object may be live at the same time as the returned reference.
    pub unsafe fn get_unchecked_mut<T: Component>(
        &self,
        object: Object,
    ) -> Result<&mut T, ComponentError> {
        self.ecs
            .get_unchecked_mut(object.entity)
            .map_err(ComponentError::from)
    }

    /// Convert any reserved entities into empty entities that can be accessed. Implicitly called by
    /// [`Space::spawn`], [`Space::insert`], [`Space::insert_one`], [`Space::remove`],
    /// [`Space::remove_one`], and [`Space::despawn`].
    pub fn flush(&mut self) {
        self.ecs.flush();
    }

    /// Inspect the archetypes that objects in this [`Space`] are organized into.
    ///
    /// Useful for dynamically scheduling queries, efficient serialization, and other similar tasks
    /// which need to understand what sets of components exist in the [`Space`].
    pub fn archetypes(&self) -> impl ExactSizeIterator<Item = &Archetype> + '_ {
        self.ecs.archetypes()
    }

    /// Returns an opaque value which can be used to determine when the [`Space::archetypes`] have
    /// changed. Useful for when you need to know when to update something calculated from
    /// [`Space::archetypes`].
    pub fn archetypes_generation(&self) -> ArchetypesGeneration {
        self.ecs.archetypes_generation()
    }

    /// The number of currently live objects.
    pub fn len(&self) -> u32 {
        self.ecs.len()
    }

    /// Checks whether any objects are live at all.
    pub fn is_empty(&self) -> bool {
        self.ecs.is_empty()
    }
}

impl LuaUserData for Space {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        use crate::spaces::lua::*;

        methods.add_meta_method(LuaMetaMethod::Len, spaces_len());
        methods.add_method_mut("spawn", spaces_spawn());
        methods.add_method_mut("insert", spaces_insert());
        methods.add_method_mut("despawn", spaces_despawn());
        methods.add_method_mut("clear", spaces_clear());
        methods.add_method("id", |_, this, ()| Ok(this.id));
    }
}

struct SpacesPlugin;

impl Plugin for SpacesPlugin {
    fn name(&self) -> &'static str {
        "spaces"
    }

    fn open<'lua>(
        &self,
        lua: &'lua Lua,
        engine: &crate::engine::Engine,
    ) -> Result<LuaTable<'lua>, Error> {
        let spaces_resource = engine.insert(Spaces::new());
        lua.register(spaces_resource.clone())?;

        let sp_res = spaces_resource;
        let create_space = lua.create_function(move |_, ()| {
            let mut sr = sp_res.borrow_mut();
            Ok(sr.create_space())
        })?;

        Ok(lua
            .load(mlua::chunk! {
                {
                    create_space = $create_space,
                }
            })
            .eval()?)
    }
}

inventory::submit!(ModuleWrapper::new(SpacesPlugin));
