use std::{collections::HashMap, str::FromStr};

use hv_core::{engine::Engine, input::KeyCode, prelude::*};

#[derive(Debug, Clone, Copy, Default)]
struct EngineKeyState {
    is_down: bool,
    is_repeat: bool,
}

/// Used for providing input state to Lua; "normal" input from the Rust side should not use this,
/// but rather should rely on the event handler's methods and the `InputState`/`InputBinding` types.
#[derive(Debug, Default)]
pub struct EngineKeyboardState {
    is_key_down: HashMap<KeyCode, EngineKeyState>,
    key_repeat_enabled: bool,
}

impl EngineKeyboardState {
    pub fn set_key_state(&mut self, key: KeyCode, down: bool, repeat: bool) {
        let entry = self.is_key_down.entry(key).or_default();

        if self.key_repeat_enabled || !repeat {
            entry.is_repeat = repeat;
            entry.is_down = down
        } else {
            entry.is_repeat = false;
        }
    }

    pub fn is_key_down(&self, key: KeyCode) -> bool {
        self.is_key_down
            .get(&key)
            .map(|ks| ks.is_down)
            .unwrap_or(false)
    }
}

pub(crate) fn open<'lua>(lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error> {
    let keyboard_state = engine.insert(EngineKeyboardState::default());
    let is_down = lua.create_function(move |_, key: LuaString| {
        let key_variant = KeyCode::from_str(key.to_str()?).to_lua_err()?;
        Ok(keyboard_state.borrow().is_key_down(key_variant))
    })?;

    Ok(lua
        .load(mlua::chunk! {
            {
                is_down = $is_down,
            }
        })
        .eval()?)
}
