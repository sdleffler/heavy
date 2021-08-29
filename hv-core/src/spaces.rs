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

pub use self::lua::SpaceCache;

#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("no such object")]
    NoSuchObject,
    #[error("missing component {_0}")]
    MissingComponent(MissingComponent),
    #[error("wrong space")]
    WrongSpace,
}

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

pub struct Spaces {
    registry: Arena<Shared<Space>>,
}

impl Spaces {
    fn new() -> Self {
        Self {
            registry: Arena::new(),
        }
    }

    pub fn create_space(&mut self) -> Shared<Space> {
        let space = Shared::new(Space::new());
        let space_id = self.registry.insert(space.clone());
        self.registry[space_id].borrow_mut().id = SpaceId(space_id);
        space
    }

    pub fn get_space(&self, space_id: SpaceId) -> Shared<Space> {
        self.registry[space_id.0].clone()
    }
}

impl LuaUserData for Spaces {}

impl LuaResource for Spaces {
    const REGISTRY_KEY: &'static str = "HV_SPACES";
}

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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Object {
    space: SpaceId,
    entity: hecs::Entity,
}

impl Object {
    pub fn space(&self) -> SpaceId {
        self.space
    }

    pub fn slot(&self) -> u32 {
        self.entity.id()
    }

    #[doc(hidden)]
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

/// Create a context in which `Object`s may be serialized and deserialized, around the provided
/// function. You will usually never need this yourself; it's useful strictly for implementing
/// serialization of `Space`s and enforcing a couple of rules about them - namely, that spaces can
/// serialize `Object`s which they own, and deserializing a space comes with a space ID to use for
/// all objects within it.
pub fn with_space_id<T>(space_id: SpaceId, f: impl FnOnce() -> T) -> T {
    CURRENT_SPACE_ID.with(|current| current.replace(Some(space_id)));
    let t = f();
    CURRENT_SPACE_ID.with(|current| current.replace(None));
    t
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RawEntity(hecs::Entity);

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

pub struct QueryBorrow<'w, Q: Query> {
    space: SpaceId,
    inner: hecs::QueryBorrow<'w, Q>,
}

impl<'w, Q: Query> QueryBorrow<'w, Q> {
    pub fn iter(&mut self) -> QueryIter<'_, Q> {
        QueryIter {
            space: self.space,
            inner: self.inner.iter(),
        }
    }

    pub fn with<T: Component>(self) -> QueryBorrow<'w, With<T, Q>> {
        QueryBorrow {
            space: self.space,
            inner: self.inner.with(),
        }
    }

    pub fn without<T: Component>(self) -> QueryBorrow<'w, Without<T, Q>> {
        QueryBorrow {
            space: self.space,
            inner: self.inner.without(),
        }
    }
}

pub struct QueryMut<'q, Q: Query> {
    space: SpaceId,
    inner: hecs::QueryMut<'q, Q>,
}

impl<'q, Q: Query> QueryMut<'q, Q> {
    pub fn with<T: Component>(self) -> QueryMut<'q, With<T, Q>> {
        QueryMut {
            space: self.space,
            inner: self.inner.with(),
        }
    }

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

    pub fn id(&self) -> SpaceId {
        self.id
    }

    fn wrap_entity(&self, entity: hecs::Entity) -> Object {
        Object {
            space: self.id,
            entity,
        }
    }

    pub fn spawn(&mut self, components: impl DynamicBundle) -> Object {
        let e = self.ecs.spawn(components);
        self.wrap_entity(e)
    }

    pub fn spawn_at(&mut self, handle: RawEntity, components: impl DynamicBundle) -> Object {
        self.ecs.spawn_at(handle.0, components);
        self.wrap_entity(handle.0)
    }

    pub fn spawn_batch<I>(&mut self, iter: I) -> SpawnBatchIter<I::IntoIter>
    where
        I: IntoIterator,
        I::Item: Bundle + 'static,
    {
        let id = self.id;
        let inner = self.ecs.spawn_batch(iter);
        SpawnBatchIter { id, inner }
    }

    pub fn spawn_column_batch(&mut self, batch: ColumnBatch) -> SpawnColumnBatchIter {
        let id = self.id;
        let inner = self.ecs.spawn_column_batch(batch);
        SpawnColumnBatchIter { id, inner }
    }

    pub fn reserve_objects(
        &self,
        count: u32,
    ) -> impl Iterator<Item = Object> + ExactSizeIterator + '_ {
        let id = self.id;
        self.ecs
            .reserve_entities(count)
            .map(move |entity| Object { space: id, entity })
    }

    pub fn despawn(&mut self, object: Object) -> Result<(), ObjectError> {
        if self.id != object.space {
            Err(ObjectError::WrongSpace)
        } else {
            self.ecs.despawn(object.entity).map_err(ObjectError::from)
        }
    }

    pub fn reserve_object(&self) -> Object {
        self.wrap_entity(self.ecs.reserve_entity())
    }

    pub fn reserve<T: Bundle + 'static>(&mut self, additional: u32) {
        self.ecs.reserve::<T>(additional);
    }

    pub fn clear(&mut self) {
        self.ecs.clear()
    }

    pub fn contains(&self, object: Object) -> bool {
        object.space == self.id && self.ecs.contains(object.entity)
    }

    pub fn query<Q: Query>(&self) -> QueryBorrow<'_, Q> {
        QueryBorrow {
            space: self.id,
            inner: self.ecs.query(),
        }
    }

    pub fn query_mut<Q: Query>(&mut self) -> QueryMut<'_, Q> {
        QueryMut {
            space: self.id,
            inner: self.ecs.query_mut(),
        }
    }

    pub fn query_one<Q: Query>(&self, object: Object) -> Result<QueryOne<'_, Q>, ObjectError> {
        if self.id != object.space {
            return Err(ObjectError::WrongSpace);
        }

        self.ecs.query_one(object.entity).map_err(ObjectError::from)
    }

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

    pub fn get<T: Component>(&self, object: Object) -> Result<Ref<'_, T>, ComponentError> {
        if self.id != object.space {
            return Err(ComponentError::WrongSpace);
        }

        self.ecs
            .get::<T>(object.entity)
            .map_err(ComponentError::from)
    }

    pub fn get_mut<T: Component>(&self, object: Object) -> Result<RefMut<'_, T>, ComponentError> {
        if self.id != object.space {
            return Err(ComponentError::WrongSpace);
        }

        self.ecs
            .get_mut::<T>(object.entity)
            .map_err(ComponentError::from)
    }

    // pub fn entity(&self, entity: Entity) -> Result<EntityRef<'_>, NoSuchEntity>

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn find_object_from_id(&self, id: u32) -> Object {
        let entity = self.ecs.find_entity_from_id(id);
        self.wrap_entity(entity)
    }

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

    pub fn iter(&self) -> Iter<'_> {
        Iter {
            id: self.id,
            inner: self.ecs.iter(),
        }
    }

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

    pub fn remove<T: Bundle + 'static>(&mut self, object: Object) -> Result<T, ComponentError> {
        if self.id != object.space {
            return Err(ComponentError::WrongSpace);
        }

        self.ecs.remove(object.entity).map_err(ComponentError::from)
    }

    pub fn remove_one<T: Component>(&mut self, object: Object) -> Result<T, ComponentError> {
        if self.id != object.space {
            return Err(ComponentError::WrongSpace);
        }

        self.ecs
            .remove_one(object.entity)
            .map_err(ComponentError::from)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_unchecked<T: Component>(&self, object: Object) -> Result<&T, ComponentError> {
        self.ecs
            .get_unchecked(object.entity)
            .map_err(ComponentError::from)
    }

    #[allow(clippy::missing_safety_doc)]
    pub unsafe fn get_unchecked_mut<T: Component>(
        &self,
        object: Object,
    ) -> Result<&mut T, ComponentError> {
        self.ecs
            .get_unchecked_mut(object.entity)
            .map_err(ComponentError::from)
    }

    pub fn flush(&mut self) {
        self.ecs.flush();
    }

    pub fn archetypes(&self) -> impl ExactSizeIterator<Item = &Archetype> + '_ {
        self.ecs.archetypes()
    }

    pub fn archetypes_generation(&self) -> ArchetypesGeneration {
        self.ecs.archetypes_generation()
    }

    pub fn len(&self) -> u32 {
        self.ecs.len()
    }

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
