//! Functionality for working with components across the Rust/Lua boundary.
//!
//! If you need to add a component to an object from Lua, you'll want a
//! [`DynamicComponentConstructor`]. This is essentially the data necessary to add the component but
//! wrapped in an extra layer which tells Rust *how* to add the component. For unfortunate technical
//! reasons, the component type itself cannot be the Lua userdata passed to functions like
//! `Space::spawn`.

use smallbox::{space::S4, SmallBox};

use crate::{
    engine::Engine,
    error::*,
    hecs::{ColumnBatchType, Component, EntityBuilder},
    mlua::prelude::*,
    plugins::{ModuleWrapper, Plugin},
    spaces::{Object, Space},
};

pub trait ComponentConstructor: Send + 'static {
    type Component: Component;

    fn construct(&self, _lua: &Lua, _object: Object) -> Result<Self::Component>;
}

impl<F, C> ComponentConstructor for F
where
    C: Component,
    F: Send + 'static + for<'lua> Fn(&'lua Lua, Object) -> Result<C>,
{
    type Component = C;

    fn construct(&self, lua: &Lua, object: Object) -> Result<Self::Component> {
        (self)(lua, object)
    }
}

trait ErasedComponentConstructor: Send + 'static {
    fn add_to_column_batch_type(&self, column_batch_type: &mut ColumnBatchType);

    fn add_to_object_builder(
        &self,
        lua: &Lua,
        object: Object,
        builder: &mut EntityBuilder,
    ) -> Result<()>;

    fn insert_on_object(&self, lua: &Lua, object: Object, space: &mut Space) -> Result<()>;
}

impl<T: ComponentConstructor> ErasedComponentConstructor for T {
    fn add_to_column_batch_type(&self, column_batch_type: &mut ColumnBatchType) {
        column_batch_type.add::<T::Component>();
    }

    fn add_to_object_builder(
        &self,
        lua: &Lua,
        object: Object,
        builder: &mut EntityBuilder,
    ) -> Result<()> {
        builder.add(self.construct(lua, object)?);
        Ok(())
    }

    fn insert_on_object(&self, lua: &Lua, object: Object, space: &mut Space) -> Result<()> {
        space.insert_one(object, self.construct(lua, object)?)?;
        Ok(())
    }
}

pub struct DynamicComponentConstructor {
    erased: SmallBox<dyn ErasedComponentConstructor, S4>,
}

impl DynamicComponentConstructor {
    pub fn new(constructor: impl ComponentConstructor) -> Self {
        Self {
            erased: smallbox::smallbox!(constructor),
        }
    }

    pub fn copy<T: Component + Copy>(value: T) -> Self {
        Self::new(move |_: &Lua, _| Ok(value))
    }

    pub fn clone<T: Component + Clone>(value: T) -> Self {
        Self::new(move |_: &Lua, _| Ok(value.clone()))
    }

    pub fn add_to_column_batch_type(&self, column_batch_type: &mut ColumnBatchType) {
        self.erased.add_to_column_batch_type(column_batch_type);
    }

    pub fn add_to_object_builder(
        &self,
        lua: &Lua,
        object: Object,
        builder: &mut EntityBuilder,
    ) -> Result<()> {
        self.erased.add_to_object_builder(lua, object, builder)
    }

    pub fn insert_on_object(&self, lua: &Lua, object: Object, space: &mut Space) -> Result<()> {
        self.erased.insert_on_object(lua, object, space)
    }
}

impl LuaUserData for DynamicComponentConstructor {}

#[doc(hidden)]
pub struct ComponentWrapper {
    object: Box<dyn Plugin>,
}

impl ComponentWrapper {
    pub fn new<T: Plugin>(plugin: T) -> Self {
        Self {
            object: Box::new(plugin),
        }
    }
}

#[macro_export]
macro_rules! component {
    ($e:expr) => {
        const _: () = {
            use $crate::inventory;
            $crate::inventory::submit!($crate::components::ComponentWrapper::new($e));
        };
    };
}

inventory::collect!(ComponentWrapper);

pub(crate) fn registered_components() -> impl Iterator<Item = &'static dyn Plugin> {
    inventory::iter::<ComponentWrapper>
        .into_iter()
        .map(|wrapper| &*wrapper.object)
}

struct ComponentsModule;

impl Plugin for ComponentsModule {
    fn name(&self) -> &'static str {
        "components"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>> {
        let table = lua.create_table()?;

        for plugin in registered_components() {
            let opened = plugin.open(lua, engine)?;
            let mut parent = table.clone();
            let mut segments = plugin.name().split('.');
            let mut current = segments
                .next()
                .expect("component paths must have at least one identifier!");

            log::trace!("opening registered component plugin: `{}`", plugin.name());

            for next in segments {
                parent = match parent.get::<_, Option<LuaTable>>(current)? {
                    Some(t) => t,
                    None => {
                        let t = lua.create_table()?;
                        parent.set(current, t.clone())?;
                        t
                    }
                };

                current = next;
            }

            parent.set(current, opened)?;
        }

        log::trace!("all registered components opened.");

        Ok(table)
    }
}

inventory::submit!(ModuleWrapper::new(ComponentsModule));
