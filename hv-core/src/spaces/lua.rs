use hecs::EntityBuilder;
use mlua::{prelude::*, Variadic as LuaVariadic};

use crate::{
    components::DynamicComponentConstructor,
    engine::{Engine, EngineRef},
    shared::{Shared, Weak},
    spaces::{object_table::ObjectTableComponent, Object, Space, SpaceId, Spaces},
};

macro_rules! lua_fn {
    (Fn<$lua:lifetime>($args:ty) -> $ret:ty) => { impl 'static + for<$lua> Fn(&$lua Lua, $args) -> LuaResult<$ret> };
    (FnMut<$lua:lifetime>($args:ty) -> $ret:ty) => { impl 'static + for<$lua> FnMut(&$lua Lua, $args) -> LuaResult<$ret> };
    (Fn<$lua:lifetime>($this:ty, $args:ty) -> $ret:ty) => { impl 'static + for<$lua> Fn(&$lua Lua, $this, $args) -> LuaResult<$ret> };
    (FnMut<$lua:lifetime>($this:ty, $args:ty) -> $ret:ty) => { impl 'static + for<$lua> FnMut(&$lua Lua, $this, $args) -> LuaResult<$ret> }
}

pub fn spaces_len() -> lua_fn!(Fn<'lua>(&Space, ()) -> u32) {
    |_, space, _| Ok(space.len())
}

pub fn spaces_spawn(
) -> lua_fn!(FnMut<'lua>(&mut Space, LuaVariadic<LuaAnyUserData<'lua>>) -> Object) {
    let mut builder = EntityBuilder::new();
    move |lua, space, components| {
        let object = space.reserve_object();

        for component in components {
            let dynamic_component = component.borrow::<DynamicComponentConstructor>()?;
            dynamic_component
                .add_to_object_builder(lua, object, &mut builder)
                .to_lua_err()?;
        }

        space.insert(object, builder.build()).to_lua_err()?;
        Ok(object)
    }
}

pub fn spaces_insert(
) -> lua_fn!(FnMut<'lua>(&mut Space, (Object, LuaVariadic<LuaAnyUserData<'lua>>)) -> Object) {
    let mut builder = EntityBuilder::new();
    move |lua, space, (object, components)| {
        for component in components {
            let dynamic_component = component.borrow::<DynamicComponentConstructor>()?;
            dynamic_component
                .add_to_object_builder(lua, object, &mut builder)
                .to_lua_err()?;
        }

        space.insert(object, builder.build()).to_lua_err()?;
        Ok(object)
    }
}

pub fn spaces_despawn() -> lua_fn!(FnMut<'lua>(&mut Space, Object) -> ()) {
    |_, space, object| {
        space.despawn(object).to_lua_err()?;
        Ok(())
    }
}

pub fn spaces_queue_spawn() -> lua_fn!(Fn<'lua>(&Space, LuaVariadic<LuaAnyUserData<'lua>>) -> Object)
{
    move |lua, space, components| {
        let mut command_buffer = space.command_buffer.write().unwrap();
        let mut builder = command_buffer.get_builder();
        let object = space.reserve_object();

        for component in components {
            let dynamic_component = component.borrow::<DynamicComponentConstructor>()?;
            dynamic_component
                .add_to_object_builder(lua, object, &mut builder)
                .to_lua_err()?;
        }

        command_buffer.insert_builder(object, builder);
        Ok(object)
    }
}

pub fn spaces_queue_insert(
) -> lua_fn!(Fn<'lua>(&Space, (Object, LuaVariadic<LuaAnyUserData<'lua>>)) -> Object) {
    move |lua, space, (object, components)| {
        let mut command_buffer = space.command_buffer.write().unwrap();
        let mut builder = command_buffer.get_builder();

        for component in components {
            let dynamic_component = component.borrow::<DynamicComponentConstructor>()?;
            dynamic_component
                .add_to_object_builder(lua, object, &mut builder)
                .to_lua_err()?;
        }

        command_buffer.insert_builder(object, builder);
        Ok(object)
    }
}

pub fn spaces_queue_despawn() -> lua_fn!(Fn<'lua>(&Space, Object) -> ()) {
    |_, space, object| {
        space.queue_despawn(object);
        Ok(())
    }
}

pub fn spaces_clear() -> lua_fn!(FnMut<'lua>(&mut Space, ()) -> ()) {
    |_, space, ()| {
        space.clear();
        Ok(())
    }
}

pub fn spaces_objects() -> lua_fn!(Fn<'lua>(&Space, ()) -> Vec<Object>) {
    |_, space, ()| {
        Ok(space
            .query::<()>()
            .with::<ObjectTableComponent>()
            .iter()
            .map(|(obj, _)| obj)
            .collect())
    }
}

/// A specialized cache for [`Space`]s to reduce access to the [`Spaces`] resource.
pub struct SpaceCache {
    weak_engine: EngineRef,
    weak_spaces: Weak<Spaces>,
    cached_space_id: Option<SpaceId>,
    weak_cached_space: Weak<Space>,
}

impl SpaceCache {
    /// Create a new cache for the [`Spaces`] resource in the given engine.
    pub fn new(engine: &Engine) -> Self {
        Self {
            weak_engine: engine.downgrade(),
            weak_spaces: Weak::new(),
            cached_space_id: None,
            weak_cached_space: Weak::new(),
        }
    }

    /// Get a space by ID. If this is the same as the last space that was accessed with this cache,
    /// then the cached space is returned.
    pub fn get_space(&mut self, space_id: SpaceId) -> Shared<Space> {
        if self.cached_space_id != Some(space_id) {
            let strong_spaces = match self.weak_spaces.try_upgrade() {
                Some(strong) => strong,
                None => {
                    let strong = self.weak_engine.upgrade().get::<Spaces>();
                    self.weak_spaces = strong.downgrade();
                    strong
                }
            };

            let space = strong_spaces.borrow().get_space(space_id);
            self.weak_cached_space = space.downgrade();
            space
        } else {
            self.weak_cached_space.upgrade()
        }
    }
}
