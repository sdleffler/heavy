//! Serialization functionality for [`Space`]s.
//!
//! Not all components in a space have to be serializable in order to serialize the space, but just
//! know that those unserializable components (or components not registered w/ a [`ComponentSerde`]
//! instance) will simply not be stored and will not be present when the space is deserialized.
//!
//! For common cases, [`ComponentSerde`]s can be constructed with the [`with_lua`] and
//! [`with_serde`] functions, and then registered with the [`serializable!`] macro.
//!
//! ## Registering serde-compatible components with the serializer
//!
//! Registering serde-compatible components is very simple and just needs a single call to the
//! [`serializable!`] macro:
//!
//! ```rust
//! # use hv_core::{spaces::serialize, na::Vector2, serializable};
//! # use serde::*;
//! #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
//! pub struct Coordinates(pub Vector2<f32>);
//!
//! // The string name here can be whatever you want, but it's a good idea to have it be something
//! // "my_plugin_or_module_or_crate_or_whatever.ComponentType", as that's the convention that most
//! // Heavy crates use.
//! serializable!(serialize::with_serde::<Coordinates>("my.CoordinatesComponent"));
//! ```
//!
//! ## Registering Lua-convertible components with the serializer
//!
//! Registering Lua-convertible components (types which implement [`ToLua`] and [`FromLua`]) is also
//! very easy:
//!
//! ```rust
//! # use hv_core::{spaces::serialize, prelude::*, serializable};
//! #[derive(Debug)]
//! pub struct SomeLuaTable(pub LuaRegistryKey);
//!
//! impl<'a, 'lua> ToLua<'lua> for &'a SomeLuaTable {
//!     fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
//!         lua.registry_value(&self.0)
//!     }
//! }
//!
//! impl<'lua> FromLua<'lua> for SomeLuaTable {
//!     fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
//!         lua.create_registry_value(lua_value).map(Self)
//!     }
//! }
//!
//! serializable!(serialize::with_lua::<SomeLuaTable>("my.SomeLuaTableComponent"));
//! ```
//!
//! *Note the slight snag on the [`ToLua`] implementation!* There must be a [`ToLua`] implementation
//! for `&T`, not just `T`! (It's actually okay if there isn't one for `T` but that would be a
//! little strange if you already have it for `&T`.)
//!
//! If your Lua value contains references to other Lua values being serialized in the same space,
//! those references *will* be preserved! The underlying serialization used is from `binser`, which
//! can be found in the Lua module [`std.binser`](crate::api). If there are unserializable Lua
//! values such as C functions or userdata or resources which shouldn't be serialized but instead
//! taken from global constants once deserialized, `binser` provides functionality to deal with
//! this.
//!
//! ## Registering components requiring finalization
//!
//! Finalization allows you to run a function on the entire [`Space`] being deserialized after all
//! the deserialized components are added to it (but not necessarily finalized themselves.)  For
//! example, here is how Heavy serializes [`ObjectTableComponent`]s; not the use of
//! [`with_finalizer`] to re-iterate over all the objects w/ [`ObjectTableComponent`]s later, to
//! finish linking them up (note that the implementation of [`FromLua`] for [`ObjectTableComponent`]
//! creates a partial entry automatically, which is difficult to show here, but should at least
//! provide an example of how finalization can be useful with a deserializer.)
//!
//! ```rust
//! # use hv_core::{spaces::{serialize::*, object_table::*}, prelude::*, serializable};
//! serializable!(with_finalizer(
//!     with_lua::<ObjectTableComponent>("hv.ObjectTable"),
//!     |lua, space| {
//!         // While object tables can be serialized without issue, loading them back up incurs a
//!         // problem: they are linked to object IDs, and not even hecs entity IDs are available at
//!         // deserialization time. So we "partially" insert them when deserializing, and then
//!         // link them to their respective objects after we're done loading everything else.
//!         let object_table_registry = lua.get_resource::<ObjectTableRegistry>()?;
//!         for (object, otc) in space.query_mut::<&ObjectTableComponent>() {
//!             object_table_registry
//!                 .borrow_mut()
//!                 .link_partial_entry_to_object(object, otc.index())?;
//!         }
//!
//!         Ok(())
//!     }
//! ));
//! ```

use std::{
    collections::{BTreeMap, VecDeque},
    fmt,
    io::{Read, Write},
    marker::PhantomData,
};

use crate::{
    engine::LuaExt,
    error::*,
    hecs::{Archetype, ColumnBatchBuilder, ColumnBatchType},
    mlua::prelude::*,
    prelude::Shared,
    spaces::{
        object_table::{ObjectTableComponent, ObjectTableRegistry},
        Component, Space,
    },
};

use bincode::Options;
use serde::{de::DeserializeSeed, ser::SerializeTuple, *};
use thunderdome::Arena;

/// Describes the interface required to register a component type to be serialized. Most of the time
/// you will not need to implement this trait yourself; it will be enough to use one of the
/// convenience functions provided in this module:
///
/// - [`with_serde`], if your component implements [`Serialize`] and [`Deserialize`]
/// - [`with_lua`], if your object implements [`ToLua`] and [`FromLua`]
/// - [`with_finalizer`], if you want to add a finalizer to a constructed [`ComponentSerde`] which
///   doesn't have one (such as one made with [`with_serde`] or [`with_lua`].)
///
/// [`ComponentSerde`] instances can be registered with the [`serializable!`] macro.
pub trait ComponentSerde {
    /// A unique string name identifying this component type. This value *is* serialized and is used
    /// to look up a component type on deserialization, so it should not be carelessly thought out.
    fn name(&self) -> &'static str;

    /// The component type itself, used for determining whether or not this [`ComponentSerde`]
    /// applies to a given set of components or not. If an [`Archetype`] doesn't have the
    /// corresponding component, then this [`ComponentSerde`] won't be run on it.
    type Component: Component;

    /// Deserialize all components of this type for a given "column" (archetype.) The deserializer
    /// must hold a tuple of length `count` containing all serialized components of this type for
    /// the given archetype.
    fn deserialize_components<'de, D>(
        &self,
        count: u32,
        column_batch_builder: &mut ColumnBatchBuilder,
        serde_ctx: &mut SerdeContext,
        deserializer: D,
    ) -> Result<()>
    where
        D: Deserializer<'de>,
        D::Error: Send + Sync + 'static;

    /// Serialize all components of this type for a given "column" (archetype.) The components
    /// should be serialized as a tuple of the same number of entities in the archetype.
    fn serialize_components<F>(
        &self,
        archetype: &Archetype,
        serde_ctx: &mut SerdeContext,
        serialize: F,
    ) -> Result<()>
    where
        F: FnOnce(&dyn erased_serde::Serialize) -> Result<()>;

    /// Called once a space is fully deserialized for all components. If you have any little bits of
    /// linkage to finish up with deserialized components, this is the place. For example,
    /// [`ObjectTableComponent`]s are deserialized into "partial" object table entries, so the
    /// [`ComponentSerde::finalize`] implementation for [`ObjectTableComponent`] completes those
    /// entries with [`ObjectTableRegistry::link_partial_entry_to_object`].
    fn finalize(&self, _lua: &Lua, _space: &mut Space) -> Result<()> {
        Ok(())
    }
}

trait ErasedComponentSerde {
    fn name(&self) -> &'static str;
    fn contained_in(&self, archetype: &Archetype) -> bool;
    fn add_to_column_batch_type(&self, column_batch_type: &mut ColumnBatchType);

    fn deserialize_components<'de>(
        &self,
        count: u32,
        column_batch_builder: &mut ColumnBatchBuilder,
        serde_ctx: &mut SerdeContext,
        deserializer: &mut dyn erased_serde::Deserializer<'de>,
    ) -> Result<()>;

    /// Serialize all components corresponding to this serializer/deserializer, *one element per
    /// serialized ID,* where a single element serialized into this tuple is *all of the components
    /// of the type corresponding to the serialized ID, as a collection.*
    fn serialize_components(
        &self,
        archetype: &Archetype,
        serde_ctx: &mut SerdeContext,
        serializer: &mut dyn FnMut(&dyn erased_serde::Serialize) -> Result<()>,
    ) -> Result<()>;

    fn finalize(&self, lua: &Lua, space: &mut Space) -> Result<()>;
}

impl<T: ComponentSerde> ErasedComponentSerde for T {
    fn name(&self) -> &'static str {
        T::name(self)
    }

    fn contained_in(&self, archetype: &Archetype) -> bool {
        archetype.has::<T::Component>()
    }

    fn add_to_column_batch_type(&self, column_batch_type: &mut ColumnBatchType) {
        column_batch_type.add::<T::Component>();
    }

    fn deserialize_components<'de>(
        &self,
        count: u32,
        column_batch_builder: &mut ColumnBatchBuilder,
        serde_ctx: &mut SerdeContext,
        deserializer: &mut dyn erased_serde::Deserializer<'de>,
    ) -> Result<()> {
        T::deserialize_components(self, count, column_batch_builder, serde_ctx, deserializer)
            .map_err(Error::from)
    }

    fn serialize_components(
        &self,
        archetype: &Archetype,
        serde_ctx: &mut SerdeContext,
        serializer: &mut dyn FnMut(&dyn erased_serde::Serialize) -> Result<()>,
    ) -> Result<()> {
        T::serialize_components(self, archetype, serde_ctx, move |obj| serializer(obj))
    }

    fn finalize(&self, lua: &Lua, space: &mut Space) -> Result<()> {
        T::finalize(self, lua, space)
    }
}

#[doc(hidden)]
pub struct Serializable {
    inner: Box<dyn ErasedComponentSerde>,
}

inventory::collect!(Serializable);

impl Serializable {
    #[doc(hidden)]
    pub fn new(cs: impl ComponentSerde + 'static) -> Self {
        Self {
            inner: Box::new(cs),
        }
    }
}

/// Register a [`ComponentSerde`] instance.
///
/// Under the hood, this macro uses [`inventory::submit!`], so it does not actually generate code
/// that needs to run or be placed in a function! It is enough to place a [`serializable!`] call at
/// the top level of a module.
#[macro_export]
macro_rules! serializable {
    ($e:expr) => {
        const _: () = {
            use $crate::inventory;
            $crate::inventory::submit!(
                $crate::spaces::serialize::Serializable::new($e)
            );
        };
    };
}

/// Construct a [`ComponentSerde`] instance for a type which can be freely converted to and from a
/// Lua value. On serialization, the type will be converted to Lua and then serialized using
/// along with all other Lua objects in the [`SerdeContext`], preserving any references to other
/// tables or Lua objects being serialized in the same space.
pub fn with_lua<T: Component + for<'lua> FromLua<'lua>>(
    name: &'static str,
) -> impl ComponentSerde<Component = T>
where
    for<'a, 'lua> &'a T: ToLua<'lua>,
{
    struct LuaShim<T: Component + for<'lua> FromLua<'lua>>
    where
        for<'a, 'lua> &'a T: ToLua<'lua>,
    {
        name: &'static str,
        _phantom: PhantomData<fn() -> T>,
    }

    impl<T: Component + for<'lua> FromLua<'lua>> ComponentSerde for LuaShim<T>
    where
        for<'a, 'lua> &'a T: ToLua<'lua>,
    {
        fn name(&self) -> &'static str {
            self.name
        }

        type Component = T;

        fn serialize_components<F>(
            &self,
            archetype: &Archetype,
            serde_ctx: &mut SerdeContext,
            serialize: F,
        ) -> Result<(), Error>
        where
            F: FnOnce(&dyn erased_serde::Serialize) -> Result<(), Error>,
        {
            let slots = archetype
                .get::<T>()
                .expect("already checked")
                .iter()
                .map(|t| serde_ctx.serialize_lua_value(t.to_lua(serde_ctx.lua)?))
                .collect::<Result<Vec<_>>>()?;
            serialize(&slots)
        }

        fn deserialize_components<'de, D>(
            &self,
            count: u32,
            column_batch_builder: &mut ColumnBatchBuilder,
            serde_ctx: &mut SerdeContext,
            deserializer: D,
        ) -> Result<(), Error>
        where
            D: Deserializer<'de>,
            D::Error: Send + Sync + 'static,
        {
            let slots = Vec::<u32>::deserialize(deserializer)?;

            assert_eq!(
                slots.len(),
                count as usize,
                "mismatch in expected component count"
            );

            log::trace!(
                "preparing to deserialize {} slots for Lua-encoded values of ID {}...",
                count,
                self.name,
            );

            let mut out = column_batch_builder.writer::<T>().expect("already checked");

            for slot in slots {
                let value = serde_ctx.deserialize_lua_value(slot)?;
                let _ = out.push(T::from_lua(value, serde_ctx.lua)?);
            }

            log::trace!("done.");

            Ok(())
        }
    }

    LuaShim::<T> {
        name,
        _phantom: PhantomData,
    }
}

/// Serialize this object with its serde [`Serialize`] and [`Deserialize`] implementations. Most of
/// the time this will be what you want for adding serialization to your components.
pub fn with_serde<T: Component + Serialize + for<'de> Deserialize<'de>>(
    name: &'static str,
) -> impl ComponentSerde<Component = T> {
    struct SerdeShim<T: Serialize + for<'de> Deserialize<'de>> {
        name: &'static str,
        _phantom: PhantomData<fn() -> T>,
    }

    impl<T: Component + Serialize + for<'de> Deserialize<'de>> ComponentSerde for SerdeShim<T> {
        fn name(&self) -> &'static str {
            self.name
        }

        type Component = T;

        fn deserialize_components<'de, D>(
            &self,
            count: u32,
            column_batch_builder: &mut ColumnBatchBuilder,
            _serde_ctx: &mut SerdeContext,
            deserializer: D,
        ) -> Result<(), Error>
        where
            D: Deserializer<'de>,
            D::Error: Send + Sync + 'static,
        {
            struct ColumnVisitor<'a, T> {
                object_count: u32,
                out: &'a mut ColumnBatchBuilder,
                _phantom: PhantomData<fn() -> T>,
            }

            impl<'de, 'a, T> serde::de::Visitor<'de> for ColumnVisitor<'a, T>
            where
                T: Component + Deserialize<'de>,
            {
                type Value = ();

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    write!(
                        formatter,
                        "a set of {} {} values",
                        self.object_count,
                        std::any::type_name::<T>()
                    )
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<(), A::Error>
                where
                    A: serde::de::SeqAccess<'de>,
                {
                    let mut out = self.out.writer::<T>().expect("unexpected component type");

                    log::trace!(
                        "preparing to deserialize {} components of serde-encodable type {}...",
                        self.object_count,
                        std::any::type_name::<T>()
                    );

                    while let Some(component) = seq.next_element()? {
                        if out.push(component).is_err() {
                            return Err(de::Error::invalid_value(
                                de::Unexpected::Other("extra component"),
                                &self,
                            ));
                        }
                    }

                    if out.fill() < self.object_count {
                        return Err(de::Error::invalid_length(out.fill() as usize, &self));
                    }

                    log::trace!("done.");

                    Ok(())
                }
            }

            Ok(deserializer.deserialize_tuple(
                count as usize,
                ColumnVisitor::<T> {
                    object_count: count,
                    out: column_batch_builder,
                    _phantom: PhantomData,
                },
            )?)
        }

        fn serialize_components<F>(
            &self,
            archetype: &Archetype,
            _serde_ctx: &mut SerdeContext,
            serialize: F,
        ) -> Result<(), Error>
        where
            F: FnOnce(&dyn erased_serde::Serialize) -> Result<(), Error>,
        {
            use std::cell::RefCell;

            struct SerializeColumn<I>(RefCell<I>);

            impl<I> Serialize for SerializeColumn<I>
            where
                I: ExactSizeIterator,
                I::Item: Serialize,
            {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: Serializer,
                {
                    let mut iter = self.0.borrow_mut();
                    let mut tuple = serializer.serialize_tuple(iter.len())?;
                    for x in &mut *iter {
                        tuple.serialize_element(&x)?;
                    }
                    tuple.end()
                }
            }

            serialize(&SerializeColumn(RefCell::new(
                archetype.get::<T>().expect("already checked").iter(),
            )))
        }
    }

    SerdeShim::<T> {
        name,
        _phantom: PhantomData,
    }
}

/// Add a finalizer function to a [`ComponentSerde`]. Most useful when you need to make one more
/// pass over every object containing your component in the world, with the object ID available.
pub fn with_finalizer(
    cs: impl ComponentSerde,
    f: impl Fn(&Lua, &mut Space) -> Result<()>,
) -> impl ComponentSerde {
    struct FinalizedShim<S: ComponentSerde, F: Fn(&Lua, &mut Space) -> Result<()>> {
        cs: S,
        f: F,
    }

    impl<S: ComponentSerde, G: Fn(&Lua, &mut Space) -> Result<()>> ComponentSerde
        for FinalizedShim<S, G>
    {
        type Component = S::Component;

        fn name(&self) -> &'static str {
            self.cs.name()
        }

        fn deserialize_components<'de, D>(
            &self,
            count: u32,
            column_batch_builder: &mut ColumnBatchBuilder,
            serde_ctx: &mut SerdeContext,
            deserializer: D,
        ) -> Result<()>
        where
            D: Deserializer<'de>,
            D::Error: Send + Sync + 'static,
        {
            self.cs
                .deserialize_components(count, column_batch_builder, serde_ctx, deserializer)
        }

        fn serialize_components<F>(
            &self,
            archetype: &Archetype,
            serde_ctx: &mut SerdeContext,
            serialize: F,
        ) -> Result<()>
        where
            F: FnOnce(&dyn erased_serde::Serialize) -> Result<()>,
        {
            self.cs
                .serialize_components(archetype, serde_ctx, serialize)
        }

        fn finalize(&self, lua: &Lua, space: &mut Space) -> Result<()> {
            (self.f)(lua, space)
        }
    }

    FinalizedShim { cs, f }
}

serializable!(with_finalizer(
    with_lua::<ObjectTableComponent>("hv.ObjectTable"),
    |lua, space| {
        // While object tables can be serialized without issue, loading them back up incurs a
        // problem: they are linked to object IDs, and not even hecs entity IDs are available at
        // deserialization time. So we "partially" insert them when deserializing, and then
        // link them to their respective objects after we're done loading everything else.
        log::trace!(
            "linking Lua object table entries from components to their owning Rust objects..."
        );
        let object_table_registry = lua.get_resource::<ObjectTableRegistry>()?;
        for (object, otc) in space.query_mut::<&ObjectTableComponent>() {
            object_table_registry
                .borrow_mut()
                .link_partial_entry_to_object(object, otc.index())?;
        }

        Ok(())
    }
));

/// Context and state available to serializers and deserializers. The [`SerdeContext`] handles
/// serialization/deserialization of Lua objects and also provides access to the Lua context, which
/// can be useful for grabbing stored global resources such as the [`ObjectTableRegistry`].
pub struct SerdeContext<'a> {
    serdes: BTreeMap<&'static str, &'static dyn ErasedComponentSerde>,
    lua: &'a Lua,
    lua_objects: Arena<LuaRegistryKey>,

    components: VecDeque<&'static dyn ErasedComponentSerde>,

    serialize: LuaFunction<'a>,
    deserialize: LuaFunction<'a>,
}

impl<'a> SerdeContext<'a> {
    /// Construct an empty [`SerdeContext`] for serialization. During serialization, all components
    /// are serialized first before any Lua values, which populates the [`SerdeContext`] with all
    /// the Lua data; then, the [`SerdeContext`] puts all that Lua data into a single Lua table and
    /// serializes it all at once, preserving references between Lua objects.
    pub fn new(lua: &'a Lua) -> Result<Self> {
        let serdes = inventory::iter::<Serializable>
            .into_iter()
            .map(|bsp| (bsp.inner.name(), &*bsp.inner))
            .collect();

        let binser: LuaTable = lua.load(mlua::chunk! { require("std.binser") }).eval()?;
        let serialize = binser.get("serialize")?;
        let deserialize = binser.get("deserializeN")?;

        Ok(Self {
            serdes,
            lua,
            lua_objects: Arena::new(),

            components: VecDeque::new(),

            serialize,
            deserialize,
        })
    }

    /// Construct a [`SerdeContext`] for deserialization, populating it with Lua values from a
    /// serialized table. This must be used before deserializing Rust component data, as otherwise
    /// the Rust components will not be able to look up any Lua values they contain.
    pub fn with_lua_objects<'de, D: Deserializer<'de>>(
        lua: &'a Lua,
        deserializer: D,
    ) -> Result<Self>
    where
        D::Error: Send + Sync + 'static,
    {
        let mut this = Self::new(lua)?;

        let s = Vec::<u8>::deserialize(deserializer)?;
        let table: LuaTable = this.deserialize.call((lua.create_string(&s)?, 1))?;

        for value_res in table.clone().sequence_values::<LuaValue>() {
            this.lua_objects
                .insert(lua.create_registry_value(value_res?)?);
        }

        assert_eq!(
            table.len()? as usize,
            this.lua_objects.len(),
            "length mismatch"
        );

        Ok(this)
    }

    /// Reference to the internal Lua context.
    pub fn lua(&self) -> &'a Lua {
        self.lua
    }

    /// Serialize all Lua objects in this context by adding them all to a single Lua table and
    /// serializing that table with `binser`.
    pub fn dump_lua_objects<S: Serializer>(&self, serializer: S) -> Result<S::Ok>
    where
        S::Error: Send + Sync + 'static,
    {
        let table = self
            .lua
            .create_table_with_capacity(self.lua_objects.len() as i32, 0)?;

        for pair in self.lua_objects.iter().map(|(index, key)| {
            Ok::<_, Error>((index.slot() + 1, self.lua.registry_value::<LuaValue>(key)?))
        }) {
            let (i, value) = pair?;
            table.set(i, value)?;
        }

        assert_eq!(
            table.len()? as usize,
            self.lua_objects.len(),
            "something is wrong - the table has gaps in it!"
        );

        let lua_string = self.serialize.call::<_, LuaString>(table)?;
        log::trace!("lua string byte length: {}", lua_string.as_bytes().len());
        Ok(lua_string.as_bytes().serialize(serializer)?)
    }

    /// Add a Lua value to be serialized, receiving an index with which to deserialize it later.
    pub fn serialize_lua_value(&mut self, value: LuaValue<'a>) -> Result<u32> {
        Ok(self
            .lua_objects
            .insert(self.lua.create_registry_value(value)?)
            .slot())
    }

    /// Retrieve a deserialized Lua value with the index it was serialized with.
    pub fn deserialize_lua_value(&mut self, slot: u32) -> Result<LuaValue<'a>> {
        Ok(self
            .lua
            .registry_value(self.lua_objects.get_by_slot(slot).expect("bad slot!").1)?)
    }
}

impl<'a> hecs::serialize::column::SerializeContext for SerdeContext<'a> {
    fn component_count(&self, archetype: &Archetype) -> usize {
        archetype.len() as usize
    }

    fn serialize_component_ids<S: SerializeTuple>(
        &mut self,
        archetype: &Archetype,
        out: &mut S,
    ) -> Result<(), S::Error> {
        self.components.clear();
        for &bt_serde in self.serdes.values().filter(|bs| bs.contained_in(archetype)) {
            out.serialize_element(bt_serde.name())?;
            self.components.push_back(bt_serde);
        }

        Ok(())
    }

    fn serialize_components<S: SerializeTuple>(
        &mut self,
        archetype: &Archetype,
        out: &mut S,
    ) -> Result<(), S::Error> {
        while let Some(bt_serde) = self.components.pop_back() {
            bt_serde
                .serialize_components(archetype, self, &mut |value| {
                    out.serialize_element(value)
                        .map_err(|err| anyhow!("{:?}", err))
                })
                .map_err(|err| ser::Error::custom(anyhow!("{:?}", err)))?;
        }

        Ok(())
    }
}

impl<'a> hecs::serialize::column::DeserializeContext for SerdeContext<'a> {
    fn deserialize_component_ids<'de, A>(&mut self, mut seq: A) -> Result<ColumnBatchType, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        self.components.clear();
        log::trace!("beginning deserializing column component IDs:");
        let mut batch = ColumnBatchType::new();
        while let Some(id) = seq.next_element::<String>()? {
            let bt_serde = *self
                .serdes
                .get(id.as_str())
                .expect("no such component name registered!");
            bt_serde.add_to_column_batch_type(&mut batch);
            self.components.push_back(bt_serde);
            log::trace!("component ID: {}", id);
        }

        Ok(batch)
    }

    fn deserialize_components<'de, A>(
        &mut self,
        count: u32,
        mut seq: A,
        batch: &mut ColumnBatchBuilder,
    ) -> Result<(), A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        /// Deserializer for a single component type, for use in [`DeserializeContext::deserialize_components()`]
        struct DeserializeColumn<'a, 'lua> {
            serde_ctx: &'a mut SerdeContext<'lua>,
            bt_serde: &'static dyn ErasedComponentSerde,
            count: u32,
            batch: &'a mut ColumnBatchBuilder,
        }

        impl<'de, 'a, 'lua> DeserializeColumn<'a, 'lua> {
            /// Construct a deserializer for `entity_count` `T` components, writing into `batch`
            pub fn new(
                serde_ctx: &'a mut SerdeContext<'lua>,
                bt_serde: &'static dyn ErasedComponentSerde,
                count: u32,
                batch: &'a mut ColumnBatchBuilder,
            ) -> Self {
                Self {
                    serde_ctx,
                    bt_serde,
                    count,
                    batch,
                }
            }
        }

        impl<'de, 'a, 'lua> de::DeserializeSeed<'de> for DeserializeColumn<'a, 'lua> {
            type Value = ();

            fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
            where
                D: Deserializer<'de>,
            {
                self.bt_serde
                    .deserialize_components(
                        self.count,
                        self.batch,
                        self.serde_ctx,
                        &mut <dyn erased_serde::Deserializer<'de>>::erase(deserializer),
                    )
                    .map_err(|err| de::Error::custom(anyhow!("{:?}", err)))
            }
        }

        log::trace!(
            "beginning component data deserialization, entity count: {}",
            count
        );
        while let Some(bt_serde) = self.components.pop_back() {
            log::trace!("deserializing component {}", bt_serde.name());
            seq.next_element_seed(DeserializeColumn::new(self, bt_serde, count, batch))?
                .ok_or_else(|| de::Error::custom("an entire component column is missing!"))?;
        }

        Ok(())
    }
}

/// Deserialize a space from two separate deserializers containing the Rust objects and Lua values.
/// Most useful if you're doing something which requires storing serialized [`Space`]s in memory, as
/// otherwise serializing/deserializing from a single chunk of bytes is less efficient.
///
/// # Locking behavior
///
/// Transient mutable borrows: [`Space`] (as passed in as parameter)
pub fn deserialize_separate<'de, D, E>(
    space: &Shared<Space>,
    lua: &Lua,
    objects: D,
    lua_values: E,
) -> Result<()>
where
    D: Deserializer<'de>,
    E: Deserializer<'de>,
    D::Error: Send + Sync + 'static,
    E::Error: Send + Sync + 'static,
{
    struct DeserializeWorld<'a, 'lua> {
        serde_ctx: &'a mut SerdeContext<'lua>,
    }

    impl<'de, 'a, 'lua> DeserializeSeed<'de> for DeserializeWorld<'a, 'lua> {
        type Value = hecs::World;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            hecs::serialize::column::deserialize(self.serde_ctx, deserializer)
        }
    }

    let mut space_mut = space.borrow_mut();
    crate::spaces::with_space_id(space_mut.id(), || {
        log::trace!("deserializing Lua values in preparation for main space deserialization...");
        let mut serde_ctx = SerdeContext::with_lua_objects(lua, lua_values)?;
        log::trace!(
            "deserialized Lua values ({} slots deserialized)",
            serde_ctx.lua_objects.len()
        );

        let world = DeserializeWorld {
            serde_ctx: &mut serde_ctx,
        }
        .deserialize(objects)?;
        space_mut.ecs = world;

        log::trace!(
            "deserialized {} objects. running finalizers...",
            space_mut.len()
        );

        for cs in &serde_ctx.components {
            cs.finalize(lua, &mut space_mut)?;
        }

        log::trace!("finished deserializing.");
        Ok(())
    })
}

/// Serialize a space to two separate serializers, one for Rust objects and the other for Lua
/// values.
///
/// # Locking behavior
///
/// Transient immutable borrows: [`Space`] (as passed in as parameter)
pub fn serialize_separate<S, T>(
    shared_space: &Shared<Space>,
    lua: &Lua,
    objects: S,
    lua_values: T,
) -> Result<(S::Ok, T::Ok)>
where
    S: Serializer,
    S::Error: Send + Sync + 'static,
    T: Serializer,
    T::Error: Send + Sync + 'static,
{
    let space_id = shared_space.borrow().id();
    let mut serde_ctx = SerdeContext::new(lua)?;
    let count = shared_space.borrow().len();
    log::trace!(
        "preparing to serialize {} ECS objects while generating Lua value slots...",
        count
    );
    let ok_s = crate::spaces::with_space_id(space_id, || {
        hecs::serialize::column::serialize(&shared_space.borrow().ecs, &mut serde_ctx, objects)
    })?;
    log::trace!(
        "done. preparing to serialize {} Lua value slots...",
        serde_ctx.lua_objects.len()
    );
    let ok_t = serde_ctx.dump_lua_objects(lua_values)?;
    log::trace!("done serializing.");

    Ok((ok_s, ok_t))
}

/// Serialize a space to a writer as a single chunk of bytes.
///
/// # Locking behavior
///
/// Transient immutable borrows: [`Space`] (as passed in as parameter)
pub fn serialize_whole<W: Write>(space: &Shared<Space>, lua: &Lua, writer: W) -> Result<()> {
    let mut lua_object_buf = Vec::new();
    let mut ecs_object_buf = Vec::new();
    let mut lua_object_writer = bincode::Serializer::new(
        &mut lua_object_buf,
        bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes(),
    );
    let mut ecs_object_writer = bincode::Serializer::new(
        &mut ecs_object_buf,
        bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes(),
    );
    serialize_separate(space, lua, &mut ecs_object_writer, &mut lua_object_writer)?;
    bincode::serialize_into(writer, &(ecs_object_buf, lua_object_buf))?;
    Ok(())
}

/// Deserialize a space as a single chunk of bytes, from a reader.
///
/// # Locking behavior
///
/// Transient mutable borrows: [`Space`] (as passed in as parameter)
pub fn deserialize_whole<R: Read>(space: &Shared<Space>, lua: &Lua, reader: R) -> Result<()> {
    let (ecs_object_buf, lua_object_buf): (Vec<u8>, Vec<u8>) = bincode::deserialize_from(reader)?;

    deserialize_separate(
        space,
        lua,
        &mut bincode::Deserializer::from_slice(
            &ecs_object_buf,
            bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes(),
        ),
        &mut bincode::Deserializer::from_slice(
            &lua_object_buf,
            bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes(),
        ),
    )
}
