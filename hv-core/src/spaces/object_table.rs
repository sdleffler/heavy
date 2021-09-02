//! Lua representation of and interaction with [`Object`]s.
//!
//! Object tables are how [`Object`]s go from Rust to Lua and back. An `Object` can only be
//! converted to a Lua value if it has an [`ObjectTableComponent`], which represents a unique Lua
//! table representing the object. The conversion both ways is automatic, and relies on the
//! [`FromLua`] and [`ToLua`] implementations for [`Object`]. There are no restrictions on object
//! tables besides that they must be a Lua table; any table can be one, and if the table is
//! serializable with `binser`, then it is even possible for the object table to be serialized and
//! deserialized with the rest of the [`Space`].
//!
//! [`Space`]: crate::spaces::Space

use std::collections::{HashMap, HashSet};

use crate::{
    components::{ComponentWrapper, DynamicComponentConstructor},
    engine::{Engine, LuaExt, LuaResource},
    error::*,
    mlua::prelude::*,
    plugins::Plugin,
    shared::{Shared, Weak},
    spaces::Object,
};

use thunderdome::{Arena, Index};

/// An `ObjectTableComponent` connects an [`Object`] to its corresponding Lua table (its "object
/// table".) Constructing an `ObjectTableComponent` can be done without linking the Lua side to the
/// Rust side, by constructing an entry in the [`ObjectTableRegistry`] with a `None` object; so, as
/// long as you have a Lua table, you can construct an `ObjectTableComponent`, but until the object
/// field in the [`ObjectTableEntry`] is properly set, conversions from Lua to Rust will fail.
#[derive(Debug)]
pub struct ObjectTableComponent {
    index: ObjectTableIndex,
    weak_ref: Weak<ObjectTableRegistry>,
}

impl ObjectTableComponent {
    /// The index of the entry in the [`ObjectTableRegistry`]. You can use this or the relevant
    /// [`Object`] to index the [`ObjectTableRegistry`], though the latter will only work if it has
    /// a "full" entry (object field in the entry is set.)
    pub fn index(&self) -> ObjectTableIndex {
        self.index
    }
}

impl Drop for ObjectTableComponent {
    fn drop(&mut self) {
        if let Some(mut write) = self
            .weak_ref
            .try_upgrade()
            .as_ref()
            .and_then(|s| s.try_borrow_mut())
        {
            write.remove(self.index);
        }
    }
}

impl<'a, 'lua> ToLua<'lua> for &'a ObjectTableComponent {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        let object_table_registry_shared = lua.get_resource::<ObjectTableRegistry>()?;
        let object_table_registry = object_table_registry_shared.borrow();
        object_table_registry
            .by_index(self.index)
            .map(|entry| lua.registry_value::<LuaTable>(entry.key()))
            .transpose()
            .and_then(|opt| opt.to_lua(lua))
    }
}

impl<'lua> FromLua<'lua> for ObjectTableComponent {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let table = LuaTable::from_lua(lua_value, lua)?;
        let lua_object_table: LuaTable = lua.named_registry_value(HV_LUA_OBJECT_TABLE)?;
        let maybe_index: Option<ObjectTableIndex> = lua_object_table.get(table.clone())?;
        let otr_shared = lua.get_resource::<ObjectTableRegistry>()?;

        match maybe_index {
            Some(index) => Ok(ObjectTableComponent {
                index,
                weak_ref: Shared::downgrade(&otr_shared),
            }),
            None => Ok(otr_shared
                .borrow_mut()
                .insert_partial_entry(lua, table)
                .to_lua_err()?),
        }
    }
}

const HV_LUA_OBJECT_TABLE: &str = "HV_LUA_OBJECT_TABLE";

impl<'lua> ToLua<'lua> for Object {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        let object_table_registry_shared = lua.get_resource::<ObjectTableRegistry>()?;
        let object_table_registry = object_table_registry_shared.borrow();
        object_table_registry
            .by_object(self)
            .map(|entry| lua.registry_value::<LuaTable>(entry.key()))
            .transpose()
            .and_then(|opt| opt.to_lua(lua))
    }
}

impl<'lua> FromLua<'lua> for Object {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let lua_object_table: LuaTable = lua.named_registry_value(HV_LUA_OBJECT_TABLE)?;
        let otable_index = lua_object_table.get(lua_value)?;
        let otr_shared = lua.get_resource::<ObjectTableRegistry>()?;
        let otr = otr_shared.borrow();

        match otr
            .by_index(otable_index)
            .and_then(ObjectTableEntry::object)
        {
            Some(object) => Ok(object),
            None => Err(LuaError::external("no matching object for table")),
        }
    }
}

/// An `ObjectTableEntry` links an [`ObjectTableComponent`] to its corresponding Lua value, as well
/// as the [`Object`] it belongs to (if set.) If an entry doesn't have its `object` field set, it is
/// considered a "partial" entry, and converting the corresponding Lua table back to Rust will fail.
#[derive(Debug)]
pub struct ObjectTableEntry {
    key: LuaRegistryKey,
    object: Option<Object>,
}

impl ObjectTableEntry {
    /// The registry key for the Lua table this entry corresponds to.
    pub fn key(&self) -> &LuaRegistryKey {
        &self.key
    }

    /// The object this entry corresponds to, if set.
    pub fn object(&self) -> Option<Object> {
        self.object
    }
}

/// The [`ObjectTableRegistry`] handles mapping Lua "object tables" to [`Object`]s. There are
/// actually two data structures that handle this mapping: the first is the [`ObjectTableRegistry`],
/// which maps [`Object`]s to [`ObjectTableIndex`]s and [`ObjectTableIndex`]s to
/// [`ObjectTableEntry`]s, and then [`ObjectTableEntry`]s contain [`LuaRegistryKey`]s which map to
/// the actual Lua object tables themselves. The *second* data structure is a Lua table which lives
/// in the Lua registry under a string key `"HV_LUA_OBJECT_TABLE"`; this maps Lua tables to
/// [`ObjectTableIndex`]s which are stored as light userdata. In total, the path to go from
/// [`Object`] to Lua table and back takes two steps each way:
///
/// ### Rust to Lua
///
/// 1. [`Object`] is checked in its corresponding [`Space`] for an [`ObjectTableComponent`], and
///    extracts its corresponding [`ObjectTableIndex`].
/// 2. The extracted [`ObjectTableIndex`] is looked up in the [`ObjectTableRegistry`], and gets the
///    corresponding [`LuaRegistryKey`] from its [`ObjectTableEntry`], which is then converted to
///    the Lua object table.
///
/// ### Lua to Rust
///
/// 1. The table is looked up in the `"HV_LUA_OBJECT_TABLE"` and mapped to its corresponding
///    [`ObjectTableIndex`], if present.
/// 2. The extracted [`ObjectTableIndex`] is looked up in the [`ObjectTableRegistry`], and if its
///    [`ObjectTableEntry`] contains a corresponding [`Object`], then the conversion succeeds.
///
/// [`Space`]: crate::spaces::Space
#[derive(Debug)]
pub struct ObjectTableRegistry {
    objects: Arena<ObjectTableEntry>,
    tables: HashMap<Object, ObjectTableIndex>,
    weak: Weak<ObjectTableRegistry>,
    cleanup: HashSet<LuaRegistryKey>,
}

impl LuaUserData for ObjectTableRegistry {}

impl LuaResource for ObjectTableRegistry {
    const REGISTRY_KEY: &'static str = "HV_RUST_OBJECT_TABLE";
}

/// An index of an [`ObjectTableEntry`]. Conversion between Rust and Lua is achieved by converting
/// first to an [`ObjectTableIndex`], no matter which way the conversion is going (see also
/// [`ObjectTableRegistry`].)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectTableIndex(Index);

impl<'lua> ToLua<'lua> for ObjectTableIndex {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        LuaLightUserData(self.0.to_bits() as *mut _).to_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for ObjectTableIndex {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let lud = LuaLightUserData::from_lua(lua_value, lua)?;
        Ok(Self(Index::from_bits(lud.0 as u64)))
    }
}

impl ObjectTableRegistry {
    /// Create an empty [`ObjectTableRegistry`].
    pub fn new() -> Shared<Self> {
        let this = Shared::new(Self {
            objects: Arena::new(),
            tables: HashMap::new(),
            weak: Weak::new(),
            cleanup: HashSet::new(),
        });
        this.borrow_mut().weak = Shared::downgrade(&this);
        this
    }

    /// Create a "full" object table entry, linking both ways (Rust to Lua and Lua to Rust.)
    pub fn insert<'lua>(
        &mut self,
        lua: &'lua Lua,
        table: LuaTable<'lua>,
        object: Object,
    ) -> Result<ObjectTableComponent> {
        let otc = self.insert_partial_entry(lua, table)?;
        self.link_partial_entry_to_object(object, otc.index)?;
        Ok(otc)
    }

    /// Create a "partial" object table entry, linking Rust to Lua (but not the other way around.)
    pub fn insert_partial_entry<'lua>(
        &mut self,
        lua: &'lua Lua,
        table: LuaTable<'lua>,
    ) -> Result<ObjectTableComponent> {
        let key = lua.create_registry_value(table.clone())?;
        let index = ObjectTableIndex(self.objects.insert(ObjectTableEntry { object: None, key }));
        let lot = lua.named_registry_value::<_, LuaTable>(HV_LUA_OBJECT_TABLE)?;
        lot.set(table, index)?;

        if !self.cleanup.is_empty() {
            for dead_index in self.cleanup.drain() {
                lot.set::<LuaValue, _>(lua.registry_value(&dead_index)?, LuaNil)?;
            }
        }

        Ok(ObjectTableComponent {
            index,
            weak_ref: self.weak.clone(),
        })
    }

    /// Complete a "partial" entry, linking Lua back to Rust.
    pub fn link_partial_entry_to_object(
        &mut self,
        object: Object,
        index: ObjectTableIndex,
    ) -> Result<()> {
        // TODO(sleffy): error handling
        assert!(self.tables.insert(object, index).is_none());
        self.objects[index.0].object = Some(object);
        Ok(())
    }

    /// Look up an object table by its index.
    pub fn by_index(&self, index: ObjectTableIndex) -> Option<&ObjectTableEntry> {
        self.objects.get(index.0)
    }

    /// Look up an object table by its [`Object`] - this takes an extra step through a table mapping
    /// [`Object`]s directly to [`ObjectTableIndex`]s, but has the advantage that if you don't have
    /// the corresponding [`Space`] nearby to access, you can still grab the Lua object table.
    ///
    /// [`Space`]: crate::spaces::Space
    pub fn by_object(&self, object: Object) -> Option<&ObjectTableEntry> {
        self.tables
            .get(&object)
            .and_then(|&index| self.by_index(index))
    }

    /// Remove an object table entry by index from the registry. Unfortunately this cannot also
    /// remove the entry in the Lua object table registry as well, because doing so requires access
    /// to the Lua state which is likely to be already borrowed. So, we postpone cleanup of the
    /// actual Lua object table until the next insert; on the next insert, any postponed registry
    /// keys requiring cleanup are removed from the Lua object table.
    pub fn remove(&mut self, index: ObjectTableIndex) {
        if let Some(entry) = self.objects.remove(index.0) {
            if let Some(object) = entry.object {
                self.tables.remove(&object);
            }
            self.cleanup.insert(entry.key);
        }
    }
}

struct ObjectTableComponentPlugin;

impl Plugin for ObjectTableComponentPlugin {
    fn name(&self) -> &'static str {
        "ObjectTable"
    }

    fn open<'lua>(&self, lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>> {
        let otable_resource = ObjectTableRegistry::new();
        engine.insert_wrapped(otable_resource.clone());
        lua.insert_resource(otable_resource.clone())?;
        lua.set_named_registry_value(HV_LUA_OBJECT_TABLE, lua.create_table()?)?;

        let otr_weak = otable_resource.downgrade();
        let object_table_new = lua.create_function(move |lua, table: LuaTable| {
            let key = lua.create_registry_value(table)?;
            let weak_ref = otr_weak.clone();
            Ok(DynamicComponentConstructor::new(
                move |lua: &Lua, object| {
                    let table = lua.registry_value(&key)?;
                    let component = weak_ref.upgrade().borrow_mut().insert(lua, table, object)?;

                    Ok(component)
                },
            ))
        })?;

        Ok(lua
            .load(mlua::chunk! {
                local ObjectTable = {}

                function ObjectTable:new(t)
                    return $object_table_new(t)
                end

                return setmetatable(ObjectTable, { __call = ObjectTable.new })
            })
            .eval()?)
    }
}

inventory::submit!(ComponentWrapper::new(ObjectTableComponentPlugin));

impl LuaUserData for ObjectTableComponent {}

/// A convenient component for requiring an "update" call on an object with an
/// [`ObjectTableComponent`]. This is currently just a marker component.
#[derive(Debug, Clone, Copy)]
pub struct UpdateHookComponent;

struct UpdateHookComponentPlugin;

impl Plugin for UpdateHookComponentPlugin {
    fn name(&self) -> &'static str {
        "UpdateHook"
    }

    fn open<'lua>(&self, lua: &'lua Lua, _engine: &Engine) -> Result<LuaTable<'lua>> {
        let object_table_update_new = lua
            .create_function(|_, ()| Ok(DynamicComponentConstructor::copy(UpdateHookComponent)))?;

        Ok(lua
            .load(mlua::chunk! {
                return setmetatable({}, { __call = $object_table_update_new })
            })
            .eval()?)
    }
}

inventory::submit!(ComponentWrapper::new(UpdateHookComponentPlugin));
