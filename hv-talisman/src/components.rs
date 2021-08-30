use hv_core::{
    components::DynamicComponentConstructor,
    engine::Engine,
    prelude::*,
    spaces::{serialize::Serializable, Object, SpaceCache},
};
use hv_friends::math::*;
use serde::*;

/// A marker component used for hiding internal components from the GUI editor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Hidden;

hv_core::serializable!(Serializable::serde::<Hidden>("talisman.Hidden"));

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Visible(pub bool);

hv_core::serializable!(Serializable::serde::<Visible>("talisman.Visible"));

/// Object name component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Name(pub String);

hv_core::serializable!(Serializable::serde::<Name>("talisman.Name"));

#[derive(Debug)]
pub struct Class {
    pub chunk: Option<String>,
    pub key: LuaRegistryKey,
}

impl<'a, 'lua> ToLua<'lua> for &'a Class {
    fn to_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        lua.registry_value(&self.key)
    }
}

impl<'lua> FromLua<'lua> for Class {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            chunk: None,
            key: lua.create_registry_value(lua_value)?,
        })
    }
}

hv_core::serializable!(Serializable::lua::<Class>("talisman.Class"));

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Parent(pub Object);

hv_core::serializable!(Serializable::serde::<Parent>("talisman.Parent"));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Children(pub Vec<Object>);

hv_core::serializable!(Serializable::serde::<Children>("talisman.Children"));

#[derive(Debug)]
pub struct Sprite {
    pub path: String,
    pub local_tx: Similarity2<f32>,
}

pub(crate) fn open<'lua>(lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error> {
    let create_name_constructor =
        lua.create_function(|_, name: String| Ok(DynamicComponentConstructor::clone(Name(name))))?;

    let mut space_cache = SpaceCache::new(engine);
    let get_name_string = lua.create_function_mut(move |lua, object: Object| {
        let shared_space = space_cache.get_space(object.space());
        let mut space = shared_space.borrow_mut();
        lua.create_string(&space.query_one_mut::<&Name>(object).to_lua_err()?.0)
    })?;

    let mut space_cache = SpaceCache::new(engine);
    let set_name_string =
        lua.create_function_mut(move |_lua, (object, name_string): (Object, LuaString)| {
            let shared_space = space_cache.get_space(object.space());
            let mut space = shared_space.borrow_mut();
            let name = space.query_one_mut::<&mut Name>(object).to_lua_err()?;
            let name_str = name_string.to_str()?;

            name.0.clear();
            name.0.push_str(name_str);

            Ok(())
        })?;

    let create_class_constructor = lua.create_function(|lua, value: LuaValue| {
        let key = lua.create_registry_value(value)?;
        Ok(DynamicComponentConstructor::new(move |lua: &Lua, _| {
            let value = lua.registry_value::<LuaValue>(&key)?;
            let re_keyed = lua.create_registry_value(value)?;
            Ok(Class {
                chunk: None,
                key: re_keyed,
            })
        }))
    })?;

    let mut space_cache = SpaceCache::new(engine);
    let get_class_value = lua.create_function_mut(move |lua, object: Object| {
        let shared_space = space_cache.get_space(object.space());
        let mut space = shared_space.borrow_mut();
        lua.registry_value::<LuaValue>(&space.query_one_mut::<&Class>(object).to_lua_err()?.key)
    })?;

    let mut space_cache = SpaceCache::new(engine);
    let set_class_value =
        lua.create_function_mut(move |lua, (object, value): (Object, LuaValue)| {
            let shared_space = space_cache.get_space(object.space());
            let mut space = shared_space.borrow_mut();
            let class = space.query_one_mut::<&mut Class>(object).to_lua_err()?;

            class.chunk = None;
            class.key = lua.create_registry_value(value)?;

            Ok(())
        })?;

    Ok(lua
        .load(mlua::chunk! {
            {
                class = {
                    create_constructor = $create_class_constructor,
                    get = $get_class_value,
                    set = $set_class_value,
                },
                name = {
                    create_constructor = $create_name_constructor,
                    get = $get_name_string,
                    set = $set_name_string,
                },
            }
        })
        .eval()?)
}
