//! Object tables are how [`Object`]s go from Rust to Lua and back. An `Object` can only be
//! converted to a Lua value if it has an [`ObjectTableComponent`], which represents a unique Lua
//! table representing the object in Lua. The conversion both ways is automatic, and is implemented
//! in the [`FromLua`] and [`ToLua`] implementations for [`Object`]. There are no restrictions on
//! object tables; any table can be one, and if the table is serializable with `binser`, then it is
//! even possible for the table to be serialized and deserialized with the rest of the [`Space`].
//!
//! [`Space`]: crate::spaces::Space

use std::collections::{HashMap, HashSet};

use crate::{
    components::DynamicComponentConstructor,
    engine::{Engine, LuaExt, LuaResource},
    error::*,
    mlua::prelude::*,
    plugins::Plugin,
    shared::{Shared, Weak},
    spaces::Object,
};

use thunderdome::{Arena, Index};

pub struct ObjectTableComponent {
    pub index: ObjectTableIndex,
    pub weak_ref: Weak<ObjectTableRegistry>,
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
        let object_table_registry_shared = lua.resource::<ObjectTableRegistry>()?;
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
        let otr_shared = lua.resource::<ObjectTableRegistry>()?;

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
        let object_table_registry_shared = lua.resource::<ObjectTableRegistry>()?;
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
        let otr_shared = lua.resource::<ObjectTableRegistry>()?;
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

pub struct ObjectTableEntry {
    key: LuaRegistryKey,
    object: Option<Object>,
}

impl ObjectTableEntry {
    pub fn key(&self) -> &LuaRegistryKey {
        &self.key
    }

    pub fn object(&self) -> Option<Object> {
        self.object
    }
}

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

    pub fn by_index(&self, index: ObjectTableIndex) -> Option<&ObjectTableEntry> {
        self.objects.get(index.0)
    }

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
        lua.register(otable_resource.clone())?;
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

crate::component!(ObjectTableComponentPlugin);

impl LuaUserData for ObjectTableComponent {}

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

crate::component!(UpdateHookComponentPlugin);
