use hv_core::{
    components::DynamicComponentConstructor,
    engine::Engine,
    prelude::*,
    spaces::{Object, SpaceCache},
};
use hv_friends::math::*;
use serde::*;

use crate::level::TalismanSerdePlugin;

/// A marker component used for hiding internal components from the GUI editor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Hidden;

inventory::submit!(TalismanSerdePlugin::serde::<Hidden>("talisman.Hidden"));

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Visible(pub bool);

inventory::submit!(TalismanSerdePlugin::serde::<Visible>("talisman.Visible"));

/// Object name component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Name(pub String);

inventory::submit!(TalismanSerdePlugin::serde::<Name>("talisman.Name"));

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

inventory::submit!(TalismanSerdePlugin::lua::<Class>("talisman.Class"));

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Parent(pub Object);

inventory::submit!(TalismanSerdePlugin::serde::<Parent>("talisman.Parent"));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Children(pub Vec<Object>);

inventory::submit!(TalismanSerdePlugin::serde::<Children>("talisman.Children"));

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

// mod object_properties {
//     use hv_core::spaces::{Object, Space};
//     use hv_egui::egui;
//     use hv_friends::Position;

//     use crate::editor::{EditResult, ObjectProperty, ObjectPropertyPlugin};

//     use super::*;

//     struct NameLop;

//     impl<'a> ObjectProperty<'a> for NameLop {
//         type Query = &'a mut Name;
//         type State = String;

//         const NAME: &'static str = "talisman.Name";
//         const DISPLAY: &'static str = "Name";

//         fn init<'q>(&self, _object: Object, name: &'q mut Name, _lua: &Lua) -> Result<Self::State> {
//             Ok(name.0.clone())
//         }

//         fn edit<'q>(
//             &self,
//             _object: Object,
//             name: &'q mut Name,
//             _lua: &Lua,
//             ui: &mut egui::Ui,
//             state: &mut Self::State,
//         ) -> Result<EditResult> {
//             let response = ui.text_edit_singleline(state);

//             if response.lost_focus() && ui.input().key_down(egui::Key::Enter) {
//                 name.0.clone_from(state);

//                 Ok(EditResult::MarkUndoPoint(format!(
//                     "Set object name to '{}'",
//                     state
//                 )))
//             } else {
//                 Ok(EditResult::Unedited)
//             }
//         }

//         fn remove(&self, space: &mut Space, object: Object, _lua: &Lua) -> Result<()> {
//             space.remove_one::<Name>(object)?;
//             Ok(())
//         }
//     }

//     inventory::submit!(ObjectPropertyPlugin::new(NameLop));

//     struct ClassLop;

//     struct ClassLopState {
//         chunk: String,
//         value: String,
//         error: Option<String>,
//     }

//     impl<'a> ObjectProperty<'a> for ClassLop {
//         type Query = &'a mut Class;
//         type State = ClassLopState;

//         const NAME: &'static str = "talisman.Class";
//         const DISPLAY: &'static str = "Class";

//         fn init<'q>(
//             &self,
//             _object: Object,
//             class: &'q mut Class,
//             lua: &Lua,
//         ) -> Result<Self::State> {
//             let to_string_value = lua
//                 .globals()
//                 .call_function("tostring", lua.registry_value::<LuaValue>(&class.key)?)?;

//             Ok(ClassLopState {
//                 chunk: class.chunk.clone().unwrap_or_default(),
//                 value: to_string_value,
//                 error: None,
//             })
//         }

//         fn edit<'q>(
//             &self,
//             object: Object,
//             class: &'q mut Class,
//             lua: &Lua,
//             ui: &mut egui::Ui,
//             state: &mut Self::State,
//         ) -> Result<EditResult> {
//             let response =
//                 ui.add(egui::TextEdit::singleline(&mut state.chunk).hint_text(&state.value));
//             let err_popup_id = ui.make_persistent_id((object, "err_popup"));

//             if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
//                 match lua.load(&state.chunk).eval::<LuaValue>() {
//                     Ok(value) => {
//                         let key = lua.create_registry_value(value)?;
//                         class.key = key;

//                         *state = ObjectProperty::init(self, object, class, lua)?;

//                         return Ok(EditResult::MarkUndoPoint(format!(
//                             "Set object class to {}",
//                             state.value
//                         )));
//                     }
//                     Err(err) => {
//                         state.error = Some(format!("{:?}", err));
//                         ui.memory().toggle_popup(err_popup_id);
//                     }
//                 }
//             }

//             if let Some(error) = state.error.as_ref() {
//                 egui::popup_below_widget(ui, err_popup_id, &response, |ui| {
//                     ui.colored_label(egui::Rgba::RED, error);
//                 });
//             }

//             Ok(EditResult::Unedited)
//         }

//         fn remove(&self, space: &mut Space, object: Object, _lua: &Lua) -> Result<()> {
//             space.remove_one::<Class>(object)?;
//             Ok(())
//         }
//     }

//     inventory::submit!(ObjectPropertyPlugin::new(ClassLop));

//     struct PositionLop;

//     struct PositionState {
//         actual: Isometry2<f32>,

//         x: f32,
//         y: f32,
//         theta: f32,

//         dirty: bool,
//     }

//     impl<'a> ObjectProperty<'a> for PositionLop {
//         type Query = &'a mut Position;
//         type State = PositionState;

//         const NAME: &'static str = "hv.friends.Position";
//         const DISPLAY: &'static str = "Position";

//         fn init<'q>(
//             &self,
//             _object: Object,
//             Position(pos): &'q mut Position,
//             _lua: &Lua,
//         ) -> Result<Self::State> {
//             Ok(PositionState {
//                 actual: **pos,
//                 x: pos.translation.vector.x,
//                 y: pos.translation.vector.y,
//                 theta: pos.rotation.angle(),
//                 dirty: false,
//             })
//         }

//         fn edit<'q>(
//             &self,
//             object: Object,
//             Position(pos): &'q mut Position,
//             _lua: &Lua,
//             ui: &mut egui::Ui,
//             state: &mut Self::State,
//         ) -> Result<EditResult> {
//             egui::Grid::new((object, "Position grid"))
//                 .num_columns(2)
//                 .show(ui, |ui| {
//                     ui.label("translation:");
//                     let (x_response, y_response) = ui
//                         .horizontal(|ui| {
//                             ui.label("(");
//                             let x_response = ui
//                                 .add(egui::DragValue::new(&mut state.x).speed(1.).max_decimals(2));
//                             ui.label(",");
//                             let y_response = ui
//                                 .add(egui::DragValue::new(&mut state.y).speed(1.).max_decimals(2));
//                             ui.label(")");
//                             (x_response, y_response)
//                         })
//                         .inner;
//                     ui.end_row();
//                     ui.label("rotation:");
//                     let theta_response = ui.add(
//                         egui::DragValue::new(&mut state.theta)
//                             .speed(0.01)
//                             .max_decimals(3),
//                     );
//                     ui.end_row();

//                     let all_response = x_response.union(y_response).union(theta_response);
//                     if all_response.changed() {
//                         state.dirty = true;

//                         // Preview
//                         *pos = Position2::new(Point2::new(state.x, state.y), state.theta);
//                     }

//                     if state.dirty && (all_response.drag_released() || all_response.lost_focus()) {
//                         state.actual = **pos;
//                         state.dirty = false;

//                         Ok(EditResult::MarkUndoPoint(format!(
//                             "Set position of object #`{}`",
//                             object.slot()
//                         )))
//                     } else {
//                         Ok(EditResult::Unedited)
//                     }
//                 })
//                 .inner
//         }

//         fn remove(&self, space: &mut Space, object: Object, _lua: &Lua) -> Result<()> {
//             space.remove_one::<Position>(object)?;
//             Ok(())
//         }
//     }

//     inventory::submit!(ObjectPropertyPlugin::new(PositionLop));
// }
