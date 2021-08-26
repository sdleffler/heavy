use hv_core::{
    engine::{Engine, EngineRef, Resource},
    prelude::*,
    spaces::{
        object_table::{ObjectTableComponent, ObjectTableRegistry},
        Space, Spaces,
    },
};
use hv_egui::{
    egui::{self, ScrollArea},
    Egui,
};
use hv_friends::{
    graphics::{Canvas, ClearOptions, Graphics, GraphicsLock, GraphicsLockExt},
    math::*,
    scene::{EngineEvent, Scene},
};
use thunderdome::{Arena, Index};

mod objects;

use crate::{scenes::editor::objects::Objects, INTERNAL_RESOLUTION};

pub struct LevelEditor {
    path: Option<String>,

    space: Resource<Space>,
    objects: Objects,
    show_objects: bool,

    lua_level_class: LuaRegistryKey,
    lua_level_object_class: LuaRegistryKey,

    lua_level: LuaRegistryKey,
}

impl LevelEditor {
    pub fn new(engine: &Engine, lua: &Lua, bt: LuaTable) -> Result<Self> {
        let space = engine.get::<Spaces>().borrow_mut().create_space();
        let objects = Objects::new(engine);

        let lua_level_class: LuaTable = bt.get("Level")?;
        let lua_level_object_class: LuaTable = bt.get("LevelObject")?;

        let lua_level: LuaTable = lua_level_class.call_method("new", ())?;

        Ok(Self {
            path: None,

            space,
            objects,
            show_objects: true,

            lua_level_class: lua.create_registry_value(lua_level_class)?,
            lua_level_object_class: lua.create_registry_value(lua_level_object_class)?,

            lua_level: lua.create_registry_value(lua_level)?,
        })
    }

    pub fn menu(&mut self, engine: &Engine, ui: &mut egui::Ui) -> Result<()> {
        egui::menu::menu(ui, "Level", |ui| {
            let _ = ();
            Ok::<_, Error>(())
        })
        .transpose()?;

        Ok(())
    }

    pub fn show(&mut self, engine: &Engine, ctx: &egui::CtxRef) -> Result<()> {
        if self.show_objects {
            egui::SidePanel::right("objects")
                .show(ctx, |ui| {
                    self.objects
                        .add(&engine.lua(), &mut self.space.borrow_mut(), ui)?;

                    Ok::<_, Error>(())
                })
                .inner?;
        }

        Ok(())
    }

    pub fn draw(&mut self, graphics: Graphics) -> Result<()> {
        Ok(())
    }
}

pub struct Editor {
    world_canvas: Canvas,

    gfx_lock: Resource<GraphicsLock>,
    egui: Resource<Egui>,

    belltower: LuaRegistryKey,

    open_levels: Arena<LevelEditor>,
    current_level: Option<Index>,
}

impl Editor {
    pub fn new(engine: &Engine) -> Result<Self> {
        let gfx_lock = engine.get();

        let world_canvas = Canvas::new(
            &mut gfx_lock.lock(),
            INTERNAL_RESOLUTION.0,
            INTERNAL_RESOLUTION.1,
        );

        let egui = Egui::new(engine)?;
        let mut open_levels = Arena::new();

        let lua = &engine.lua();
        let belltower: LuaTable = lua
            .load(mlua::chunk! { require("belltower.level") })
            .eval()?;

        let level = LevelEditor::new(engine, lua, belltower.clone())?;

        {
            let space = &mut level.space.borrow_mut();
            let object = space.reserve_object();
            let table = lua
                .load(mlua::chunk! {
                    local level = require("belltower.level")
                    return level.LevelObject:new()
                })
                .eval::<LuaTable>()?;
            let otc = engine
                .get::<ObjectTableRegistry>()
                .borrow_mut()
                .insert(lua, table, object)?;
            space.insert_one(object, otc)?;
        }

        open_levels.insert(level);

        Ok(Self {
            world_canvas,
            gfx_lock,
            egui,
            belltower: lua.create_registry_value(belltower)?,
            open_levels,
            current_level: None,
        })
    }

    pub fn file_menu(&mut self, engine: &Engine, ui: &mut egui::Ui) -> Result<()> {
        egui::menu::menu(ui, "File", |ui| {
            if ui.button("New").clicked() {
                let lua = &engine.lua();
                let bt = lua.registry_value(&self.belltower)?;
                self.current_level =
                    Some(self.open_levels.insert(LevelEditor::new(engine, lua, bt)?));
            } else if ui.button("Open").clicked() {
                log::debug!("open!");
            }

            Ok::<_, Error>(())
        })
        .transpose()?;

        Ok(())
    }

    pub fn view_menu(&mut self, engine: &Engine, ui: &mut egui::Ui) -> Result<()> {
        egui::menu::menu(ui, "View", |ui| {
            ui.label("Open editors:");
            ui.indent("Open editors:", |ui| {
                for (index, open_level) in self.open_levels.iter() {
                    let button = egui::Button::new(format!(
                        "({}) {}",
                        index.slot(),
                        open_level.path.as_deref().unwrap_or("untitled")
                    ))
                    .enabled(self.current_level != Some(index));

                    if ui.add(button).clicked() {
                        self.current_level = Some(index);
                    }
                }

                Ok::<_, Error>(())
            })
            .inner?;

            Ok::<_, Error>(())
        })
        .transpose()?;

        Ok(())
    }

    pub fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        if matches!(self.current_level, Some(current) if !self.open_levels.contains(current)) {
            self.current_level = None;
        }

        if self.current_level.is_none() && !self.open_levels.is_empty() {
            self.current_level = Some(self.open_levels.iter().next().unwrap().0);
        }

        let egui_resource = self.egui.clone();
        let egui = &mut egui_resource.borrow_mut();
        egui.begin_frame(engine);
        let ctx = egui.egui_ctx();

        egui::TopBottomPanel::top("menubar")
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    self.file_menu(engine, ui)?;

                    if let Some(current) = self.current_level {
                        self.open_levels[current].menu(engine, ui)?;
                    }

                    self.view_menu(engine, ui)?;

                    Ok::<_, Error>(())
                })
                .inner?;

                Ok::<_, Error>(())
            })
            .inner?;

        if let Some(current) = self.current_level {
            self.open_levels[current].show(engine, ctx)?;
        }

        egui.end_frame(engine);
        Ok(())
    }

    pub fn draw(&mut self, engine: &Engine) -> Result<()> {
        {
            let gfx = &mut self.gfx_lock.lock();

            gfx.begin_render_pass(None, Some(ClearOptions::default()));
            gfx.apply_default_pipeline();
            gfx.set_projection(
                Orthographic3::new(
                    0.,
                    INTERNAL_RESOLUTION.0 as f32,
                    0.,
                    INTERNAL_RESOLUTION.1 as f32,
                    -1.,
                    1.,
                )
                .to_homogeneous(),
            );
            gfx.apply_transforms();
        }

        self.egui.borrow_mut().draw(engine);

        {
            let gfx = &mut self.gfx_lock.lock();

            gfx.end_render_pass();
            gfx.mq.commit_frame();
        }
        Ok(())
    }
}

impl Scene<EngineRef, EngineEvent> for Editor {
    fn update(
        &mut self,
        scene_stack: &mut hv_friends::scene::SceneStack<EngineRef, EngineEvent>,
        ctx: &mut EngineRef,
    ) -> Result<()> {
        self.update(&ctx.upgrade(), 1. / 60.)?;
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineRef) -> Result<()> {
        self.draw(&ctx.upgrade())?;
        Ok(())
    }

    fn event(&mut self, ctx: &mut EngineRef, event: EngineEvent) -> Result<()> {
        use EngineEvent::*;
        let engine = ctx.upgrade();

        match event {
            KeyDown {
                keycode, keymods, ..
            } => self
                .egui
                .borrow_mut()
                .key_down_event(&engine, keycode, keymods),
            KeyUp { keycode, keymods } => self.egui.borrow_mut().key_up_event(keycode, keymods),
            Char { character, .. } => self.egui.borrow_mut().char_event(character),
            MouseMotion { x, y } => self.egui.borrow_mut().mouse_motion_event(&engine, x, y),
            MouseWheel { x, y } => self.egui.borrow_mut().mouse_wheel_event(&engine, x, y),
            MouseButtonDown { button, x, y } => self
                .egui
                .borrow_mut()
                .mouse_button_down_event(&engine, button, x, y),
            MouseButtonUp { button, x, y } => self
                .egui
                .borrow_mut()
                .mouse_button_up_event(&engine, button, x, y),
        }

        Ok(())
    }

    fn draw_previous(&self) -> bool {
        false
    }
}
