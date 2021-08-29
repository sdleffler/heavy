use std::{collections::BTreeMap, fmt::Write};

use hv_core::{
    engine::Engine,
    prelude::*,
    spaces::{
        object_table::{ObjectTableComponent, ObjectTableRegistry},
        Object, Space,
    },
};
use hv_egui::egui::{self, ScrollArea};

struct ObjectEditBuffers {
    name: String,
    original_name: String,
    object_class: String,
    original_object_class: String,
    live: bool,
}

impl ObjectEditBuffers {
    fn new(name: String, object_class: String) -> Self {
        Self {
            original_name: name.clone(),
            name,
            original_object_class: object_class.clone(),
            object_class,
            live: true,
        }
    }
}

pub struct Objects {
    object_table_resource: Shared<ObjectTableRegistry>,
    bufs: BTreeMap<Object, ObjectEditBuffers>,
}

impl Objects {
    pub fn new(engine: &Engine) -> Self {
        Self {
            object_table_resource: engine.get(),
            bufs: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, lua: &Lua, space: &mut Space, ui: &mut egui::Ui) -> Result<()> {
        ui.heading("Objects");
        ui.separator();

        self.bufs.retain(|_, v| {
            let old = v.live;
            v.live = false;
            old
        });

        let mut label_buf = String::new();
        ScrollArea::auto_sized().show(ui, |ui| {
            for (object, table_component) in space.query_mut::<&ObjectTableComponent>() {
                let index = table_component.index;
                let entry: LuaTable = match self.object_table_resource.borrow().by_index(index) {
                    Some(e) => lua.registry_value(e.key())?,
                    None => continue,
                };

                let maybe_name: Option<LuaString> = entry.get("_name")?;
                let maybe_object_class: Option<LuaString> =
                    entry.call_method("get_object_class_name", ())?;

                label_buf.clear();

                if let Some(name) = maybe_name.as_ref() {
                    write!(&mut label_buf, "{} ({})", name.to_str()?, object.slot())?;
                } else {
                    write!(&mut label_buf, "{}", object.slot())?;
                }

                if let Some(maybe_object_class) = maybe_object_class.as_ref() {
                    write!(&mut label_buf, ": {}", maybe_object_class.to_str()?)?;
                }

                let body = |ui: &mut egui::Ui| {
                    let buf = &mut self.bufs.entry(object).or_insert_with(|| {
                        ObjectEditBuffers::new(
                            maybe_name
                                .map(|l| l.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                            maybe_object_class
                                .map(|l| l.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                        )
                    });

                    buf.live = true;

                    ui.horizontal(|ui| {
                        ui.label("name");

                        let response = ui.text_edit_singleline(&mut buf.name);

                        if response.lost_focus() {
                            if ui.input().key_pressed(egui::Key::Enter) {
                                entry.set("_name", &*buf.name)?;

                                // HACK: kill the buffer off to reset its state to have the new
                                // contents
                                buf.live = false;
                            } else {
                                buf.name.clone_from(&buf.original_name);
                            }
                        }

                        Ok::<_, Error>(())
                    })
                    .inner?;

                    ui.horizontal(|ui| {
                        ui.label("object_class");

                        let response = ui.text_edit_singleline(&mut buf.object_class);

                        if response.lost_focus() {
                            if ui.input().key_pressed(egui::Key::Enter) {
                                match lua.load(&buf.object_class).eval::<LuaValue>() {
                                    Ok(value) => {
                                        entry.set("_object_class", value)?;

                                        // HACK: kill the buffer off to reset its state to have the
                                        // new contents
                                        buf.live = false;
                                    }
                                    Err(err) => log::error!("bad Lua expression: {}", err),
                                }
                            } else {
                                buf.object_class.clone_from(&buf.original_object_class);
                            }
                        }

                        Ok::<_, Error>(())
                    })
                    .inner?;

                    Ok::<_, Error>(())
                };

                egui::CollapsingHeader::new(&label_buf)
                    .id_source(object)
                    .show(ui, body)
                    .body_returned
                    .transpose()?;
            }

            Ok(())
        })
    }
}
