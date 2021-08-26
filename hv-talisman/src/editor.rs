use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    sync::{Arc, RwLock},
};

use bincode::Options;
use hv_core::{
    engine::{Engine, EngineRef, LuaResource, Resource, WeakResourceCache},
    hecs::{Bundle, EntityBuilder},
    prelude::*,
    spaces::{Object, Space, Spaces},
};
use hv_egui::{
    egui::{self, Response},
    Egui,
};
use hv_friends::{
    camera::{Camera, CameraParameters},
    graphics::{
        Canvas, ClearOptions, Color, DrawableMut, GraphicsLock, GraphicsLockExt, Instance,
        MeshBuilder,
    },
    math::*,
    scene::{EngineEvent, Scene, SceneStack},
    Position,
};
use hv_mymachine::Console;
use thunderdome::{Arena, Index};

use crate::{
    components::Visible, editor::object_tree::ObjectTree, level::Level, modes::LevelEditorMode,
};

mod object_tree;

pub use object_tree::{ObjectProperty, ObjectPropertyPlugin};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UndoNode(Index);

pub struct UndoState {
    label: String,
    lua_object_diff: Vec<u8>,
    ecs_object_diff: Vec<u8>,
    prev_state: Option<Index>,
}

impl UndoState {
    fn new(index: Option<Index>, label: String) -> Self {
        Self {
            label,
            lua_object_diff: Vec::new(),
            ecs_object_diff: Vec::new(),
            prev_state: index,
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}

pub struct UndoTracker {
    level: Arc<RwLock<Level>>,
    space: Arc<RwLock<Space>>,
    current_lua_objects: Vec<u8>,
    current_ecs_objects: Vec<u8>,
    prev_lua_objects: Vec<u8>,
    prev_ecs_objects: Vec<u8>,
    states: Arena<UndoState>,
    current_state: Option<Index>,
}

impl UndoTracker {
    pub fn new(lua: &Lua, level: Arc<RwLock<Level>>) -> Result<Self> {
        let space = level.borrow().space.clone();
        let mut this = Self {
            level,
            space,
            current_lua_objects: Vec::new(),
            current_ecs_objects: Vec::new(),
            prev_lua_objects: Vec::new(),
            prev_ecs_objects: Vec::new(),
            states: Arena::new(),
            current_state: None,
        };

        // Ensure that the current objects are properly initialized so that we don't get problems if
        // we have to undo back to zero.
        let mut lua_object_writer = bincode::Serializer::new(
            &mut this.current_lua_objects,
            bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes(),
        );
        let mut ecs_object_writer = bincode::Serializer::new(
            &mut this.current_ecs_objects,
            bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes(),
        );
        this.level
            .borrow()
            .serialize_into(lua, &mut ecs_object_writer, &mut lua_object_writer)?;

        Ok(this)
    }

    pub fn mark(&mut self, lua: &Lua, label: String) -> Result<UndoNode> {
        // These are the diffs necessary to REVERT to the old state.
        let mut new_state = UndoState::new(self.current_state, label);
        bidiff::simple_diff(
            &self.current_lua_objects,
            &self.prev_lua_objects,
            &mut new_state.lua_object_diff,
        )?;
        bidiff::simple_diff(
            &self.current_ecs_objects,
            &self.prev_ecs_objects,
            &mut new_state.ecs_object_diff,
        )?;

        std::mem::swap(&mut self.current_lua_objects, &mut self.prev_lua_objects);
        std::mem::swap(&mut self.current_ecs_objects, &mut self.prev_ecs_objects);

        self.current_lua_objects.clear();
        self.current_ecs_objects.clear();

        let mut lua_object_writer = bincode::Serializer::new(
            &mut self.current_lua_objects,
            bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes(),
        );
        let mut ecs_object_writer = bincode::Serializer::new(
            &mut self.current_ecs_objects,
            bincode::DefaultOptions::new()
                .with_fixint_encoding()
                .allow_trailing_bytes(),
        );
        self.level
            .borrow()
            .serialize_into(lua, &mut ecs_object_writer, &mut lua_object_writer)?;

        let new_index = self.states.insert(new_state);
        self.current_state = Some(new_index);

        Ok(UndoNode(new_index))
    }

    pub fn undo(&mut self, lua: &Lua, node: UndoNode) -> Result<()> {
        assert_eq!(self.current_state, Some(node.0));
        let state = self.states.remove(node.0).expect("invalid undo state!");
        self.current_state = state.prev_state;

        log::trace!(
            "reloading from {} bytes of ECS object data and {} of Lua object data",
            self.prev_ecs_objects.len(),
            self.prev_lua_objects.len()
        );

        // Deserialize the level from the previous state, restoring it to the state it was before
        // the undo node was logged.
        let level = Level::deserialize_into(
            self.space.clone(),
            lua,
            &mut bincode::Deserializer::from_slice(
                &self.prev_ecs_objects,
                bincode::DefaultOptions::new()
                    .with_fixint_encoding()
                    .allow_trailing_bytes(),
            ),
            &mut bincode::Deserializer::from_slice(
                &self.prev_lua_objects,
                bincode::DefaultOptions::new()
                    .with_fixint_encoding()
                    .allow_trailing_bytes(),
            ),
        )?;

        // Swap the current and previous objects; the previous are now the current.
        std::mem::swap(&mut self.current_lua_objects, &mut self.prev_lua_objects);
        std::mem::swap(&mut self.current_ecs_objects, &mut self.prev_ecs_objects);

        self.prev_lua_objects.clear();
        self.prev_ecs_objects.clear();

        // Revert the now current objects to the previous-previous state by applying the diff stored
        // the undo state.
        std::io::copy(
            &mut bipatch::Reader::new(
                state.lua_object_diff.as_slice(),
                std::io::Cursor::new(&self.current_lua_objects),
            )?,
            &mut self.prev_lua_objects,
        )?;

        std::io::copy(
            &mut bipatch::Reader::new(
                state.ecs_object_diff.as_slice(),
                std::io::Cursor::new(&self.current_ecs_objects),
            )?,
            &mut self.prev_ecs_objects,
        )?;

        *self.level.borrow_mut() = level;

        Ok(())
    }

    pub fn undo_last(&mut self, lua: &Lua) -> Result<()> {
        self.undo(lua, self.current_state.map(UndoNode).unwrap())
    }

    pub fn get_last(&self) -> Option<&UndoState> {
        self.current_state.map(|index| &self.states[index])
    }
}

#[derive(Debug)]
pub enum EditResult {
    MarkUndoPoint(String),
    Unedited,
}

pub trait UiComponent<'a, T: 'a>: Send + Sync + 'static {
    fn show(&mut self, ctx: &mut LevelContext, ui: T) -> Result<egui::Response>;
}

pub struct WindowComponent {
    state: Box<dyn for<'a> UiComponent<'a, &'a egui::CtxRef>>,
}

pub struct WidgetComponent {
    state: Box<dyn for<'a> UiComponent<'a, &'a mut egui::Ui>>,
}

pub struct LevelContext {
    pub engine: EngineRef,
    pub graphics_lock: Resource<GraphicsLock>,
    pub egui: Resource<egui::CtxRef>,
    pub level: Arc<RwLock<Level>>,
    pub undo_tracker: Arc<RwLock<UndoTracker>>,
    pub camera: Arc<RwLock<Camera>>,
    pub editor_space: Arc<RwLock<Space>>,
    pub window_to_canvas_tx: Transform2<f32>,
    pub canvas_response: Option<Response>,
    pub selected_objects: HashSet<Object>,
    pub dt: f32,

    spawn_queue: Vec<EntityBuilder>,
    despawn_queue: Vec<Object>,
}

impl LevelContext {
    pub fn spawn_editor_object(&mut self, bundle: impl Bundle) {
        let mut builder = EntityBuilder::new();
        builder.add_bundle(bundle);
        self.spawn_queue.push(builder);
    }

    pub fn despawn_editor_object(&mut self, object: Object) {
        self.despawn_queue.push(object);
    }

    pub fn get_closest_interactable_object_to_hover_pos(&self, radius: f32) -> Option<Object> {
        let pos = self.canvas_response.as_ref()?.hover_pos()?;
        self.get_closest_interactable_object_to_point(&Point2::new(pos.x, pos.y), radius)
    }

    pub fn get_closest_interactable_object_to_interact_pointer_pos(
        &self,
        radius: f32,
    ) -> Option<Object> {
        let pos = self.canvas_response.as_ref()?.interact_pointer_pos()?;
        self.get_closest_interactable_object_to_point(&Point2::new(pos.x, pos.y), radius)
    }

    pub fn get_closest_interactable_object_to_point(
        &self,
        point: &Point2<f32>,
        radius: f32,
    ) -> Option<Object> {
        let canvas_interact_pos = self.window_to_canvas_tx.transform_point(point);
        let world_to_canvas_tx = *self.camera.borrow().world_to_screen_tx();

        let space = self.level.borrow().space.clone();
        let mut closest: Option<(Object, f32)> = None;
        for (obj, pos) in space
            .borrow_mut()
            .query_mut::<(&Position, Option<&Visible>)>()
            .into_iter()
            .filter_map(|(obj, (pos, maybe_visible))| {
                if matches!(maybe_visible, None | Some(Visible(true))) {
                    Some((obj, pos))
                } else {
                    None
                }
            })
        {
            let object_pos = Point2::from(pos.isometry.translation.vector);
            let canvas_object_pos = world_to_canvas_tx.transform_point(&object_pos);
            let distance = na::distance_squared(&canvas_object_pos, &canvas_interact_pos);

            if distance > radius.powi(2) {
                continue;
            }

            let is_closer = match closest {
                None => true,
                Some((_, closest_dist)) => distance < closest_dist,
            };

            if is_closer {
                closest = Some((obj, distance));
            }
        }

        closest.map(|(obj, _)| obj)
    }
}

pub struct LevelEditor {
    path: Option<String>,

    level: Arc<RwLock<Level>>,
    object_tree: ObjectTree,

    undo_tracker: Arc<RwLock<UndoTracker>>,

    show_objects: bool,

    camera: Arc<RwLock<Camera>>,

    editor_space: Arc<RwLock<Space>>,

    mode_ctx: LevelContext,
    modes: BTreeMap<String, Box<dyn LevelEditorMode>>,
    current_mode: Option<String>,
}

impl LevelEditor {
    pub fn new(engine: &Engine, lua: &Lua, level: Arc<RwLock<Level>>) -> Result<Self> {
        let (w, h) = engine.mq().screen_size();
        let camera = Arc::new(RwLock::new(Camera::new(CameraParameters::new(
            Vector2::new(w as u32, h as u32),
        ))));
        let undo_tracker = Arc::new(RwLock::new(UndoTracker::new(lua, level.clone())?));
        let editor_space = engine.get::<Spaces>().borrow_mut().create_space();
        let mut mode_ctx = LevelContext {
            engine: engine.downgrade(),
            level: level.clone(),
            graphics_lock: engine.get(),
            egui: engine.get(),
            camera: camera.clone(),
            editor_space: editor_space.clone(),
            window_to_canvas_tx: Transform2::identity(),
            undo_tracker: undo_tracker.clone(),
            canvas_response: None,
            selected_objects: HashSet::new(),
            dt: 0.,

            spawn_queue: Vec::new(),
            despawn_queue: Vec::new(),
        };

        let mut modes = crate::modes::modes();
        let current_mode = crate::modes::default_mode();

        if let Some(default_mode) = current_mode.as_ref() {
            modes
                .get_mut(default_mode)
                .unwrap()
                .enter(&mut mode_ctx, &mut [])?;
        }

        Ok(Self {
            path: None,

            level,
            object_tree: ObjectTree::new(&mut mode_ctx),

            undo_tracker,

            show_objects: true,

            camera,
            editor_space,

            mode_ctx,
            modes,
            current_mode,
        })
    }

    pub fn menu(&mut self, engine: &Engine, ui: &mut egui::Ui) -> Result<()> {
        egui::menu::menu(ui, "Edit", |ui| {
            let undo_button = match self.undo_tracker.borrow().get_last() {
                Some(state) => egui::Button::new(format!("Undo ({})", state.label())).enabled(true),
                None => egui::Button::new("Undo (none)").enabled(false),
            };

            if ui.add(undo_button).clicked() {
                self.undo_tracker.borrow_mut().undo_last(&engine.lua())?;
            }

            Ok::<_, Error>(())
        })
        .transpose()?;

        egui::menu::menu(ui, "Mode", |ui| {
            let previous_mode = self.current_mode.clone();
            for mode in self.modes.keys() {
                ui.selectable_value(&mut self.current_mode, Some(mode.to_owned()), mode);
            }

            if previous_mode != self.current_mode {
                if let Some(prev) = previous_mode.as_ref() {
                    self.modes
                        .get_mut(prev)
                        .unwrap()
                        .exit(&mut self.mode_ctx, &mut [])?;
                }

                if let Some(new) = self.current_mode.as_ref() {
                    self.modes
                        .get_mut(new)
                        .unwrap()
                        .enter(&mut self.mode_ctx, &mut [])?;
                }
            }

            Ok::<_, Error>(())
        })
        .transpose()?;

        egui::menu::menu(ui, "View", |ui| {
            ui.checkbox(&mut self.show_objects, "Show objects");

            Ok::<_, Error>(())
        })
        .transpose()?;

        Ok(())
    }

    pub fn show_update(&mut self, engine: &Engine, ctx: &egui::CtxRef, dt: f32) -> Result<()> {
        self.camera.borrow_mut().update(dt);

        if let Some(mode) = self.current_mode.as_ref() {
            self.modes
                .get_mut(mode)
                .unwrap()
                .update(&mut self.mode_ctx, &mut [])?;
        }

        if self.show_objects {
            egui::SidePanel::right("objects")
                .show(ctx, |ui| {
                    egui::ScrollArea::auto_sized().show(ui, |ui| {
                        self.object_tree
                            .show(&mut self.mode_ctx, &engine.lua(), ui)?;

                        // let space_resource = self.level.borrow().space.clone();

                        // self.all_objects.clear();
                        // self.all_objects.extend(space_resource.borrow().iter());
                        // self.all_objects.sort_by_key(Object::slot);

                        // ui.heading(format!("objects ({})", self.all_objects.len()));

                        // self.level_object_properties.update();
                        // let mut undo_tracker = self.undo_tracker.borrow_mut();

                        // for &object in &self.all_objects {
                        //     self.level_object_properties.show_object(
                        //         &space_resource,
                        //         object,
                        //         &engine.lua(),
                        //         ui,
                        //         &mut undo_tracker,
                        //     )?;
                        // }

                        Ok::<_, Error>(())
                    })
                })
                .inner?;
        }

        let mut editor_space_mut = self.editor_space.borrow_mut();
        for object in self.mode_ctx.despawn_queue.drain(..) {
            editor_space_mut.despawn(object)?;
        }

        for mut builder in self.mode_ctx.spawn_queue.drain(..) {
            editor_space_mut.spawn(builder.build());
        }

        for (_, window) in editor_space_mut.query_mut::<&mut WindowComponent>() {
            window.state.show(&mut self.mode_ctx, ctx)?;
        }

        self.mode_ctx.dt = dt;

        if let Some(mode) = self.current_mode.as_ref() {
            self.modes
                .get_mut(mode)
                .unwrap()
                .show(&mut self.mode_ctx, &mut [])?;
        }

        Ok(())
    }

    pub fn draw(&mut self, graphics: &Resource<GraphicsLock>) -> Result<()> {
        let mut gfx = graphics.lock();
        let space = self.level.borrow().space.clone();
        let mut mesh = MeshBuilder::new(gfx.state.null_texture.clone())
            .line(&[Point2::origin(), Point2::new(0., 32.)], 2., Color::GREEN)?
            .line(&[Point2::origin(), Point2::new(32., 0.)], 2., Color::RED)?
            .build(&mut gfx);

        gfx.push_multiplied_transform(self.camera.borrow().view_tx());
        gfx.apply_transforms();

        for (_, (pos, maybe_visible)) in space
            .borrow_mut()
            .query_mut::<(&Position, Option<&Visible>)>()
        {
            if !maybe_visible.map(|&Visible(b)| b).unwrap_or(true) {
                continue;
            }

            mesh.draw_mut(
                &mut gfx,
                Instance::new()
                    .translate2(pos.isometry.translation.vector)
                    .rotate2(pos.isometry.rotation.angle())
                    .scale2(Vector2::repeat(
                        self.camera.borrow().screen_to_world_tx().scaling(),
                    )),
            );
        }

        gfx.pop_transform();
        drop(gfx);

        if let Some(mode) = self.current_mode.as_ref() {
            self.modes
                .get_mut(mode)
                .unwrap()
                .draw(&mut self.mode_ctx, &mut [])?;
        }

        Ok(())
    }

    pub fn event(&mut self, event: EngineEvent) -> Result<()> {
        if let Some(mode) = self.current_mode.as_ref() {
            self.modes
                .get_mut(mode)
                .unwrap()
                .event(&mut self.mode_ctx, &mut [], event)?;
        }

        Ok(())
    }
}

impl LuaUserData for LevelEditor {}

pub struct Editor {
    resolution: (u32, u32),
    world_canvas: Canvas,
    events: VecDeque<EngineEvent>,

    gfx_lock: Resource<GraphicsLock>,
    egui_resource: Resource<Egui>,
    egui_ctx: Resource<egui::CtxRef>,

    open_levels: Arena<Arc<RwLock<LevelEditor>>>,
    current_level: Option<Index>,

    open_file_dialog_is_open: bool,
    open_file_dialog_state: String,
}

impl Editor {
    pub fn new(engine: &Engine) -> Result<Resource<Self>> {
        if let Some(this) = engine.try_get::<Self>() {
            return Ok(this);
        }

        let (w, h) = engine.mq().screen_size();
        let resolution = (w as u32, h as u32);
        let gfx_lock = engine.get();

        let world_canvas = Canvas::new(&mut gfx_lock.lock(), resolution.0, resolution.1);

        let egui_resource = Egui::new(engine)?;
        let egui_ctx = engine.get::<egui::CtxRef>();

        let open_levels = Arena::new();

        let this = engine.insert(Self {
            resolution,
            world_canvas,
            events: VecDeque::new(),
            gfx_lock,
            egui_resource,
            egui_ctx,
            open_levels,
            current_level: None,

            open_file_dialog_is_open: false,
            open_file_dialog_state: String::new(),
        });

        engine.lua().register(this.clone())?;

        Ok(this)
    }

    pub fn open_level(
        &mut self,
        engine: &Engine,
        lua: &Lua,
        level: Arc<RwLock<Level>>,
    ) -> Result<()> {
        let index = self
            .open_levels
            .insert(Arc::new(RwLock::new(LevelEditor::new(engine, lua, level)?)));
        self.current_level = Some(index);

        Ok(())
    }

    pub fn file_menu(&mut self, engine: &Engine, ui: &mut egui::Ui) -> Result<()> {
        egui::menu::menu(ui, "File", |ui| {
            if ui.button("New").clicked() {
                let lua = &engine.lua();
                self.current_level = Some(self.open_levels.insert(Arc::new(RwLock::new(
                    LevelEditor::new(engine, lua, Arc::new(RwLock::new(Level::empty(engine))))?,
                ))));
            } else if ui.button("Open").clicked() {
                self.open_file_dialog_is_open = true;
            }

            Ok::<_, Error>(())
        })
        .transpose()?;

        Ok(())
    }

    pub fn view_menu(&mut self, _engine: &Engine, ui: &mut egui::Ui) -> Result<()> {
        egui::menu::menu(ui, "Window", |ui| {
            ui.label("Open editors:");
            ui.indent("Open editors:", |ui| {
                for (index, open_level) in self.open_levels.iter() {
                    let button = egui::Button::new(format!(
                        "({}) {}",
                        index.slot(),
                        open_level.borrow().path.as_deref().unwrap_or("untitled")
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

    pub fn show_open_file_dialog(&mut self, engine: &Engine, ctx: &egui::CtxRef) -> Result<()> {
        let mut open = self.open_file_dialog_is_open;
        let mut path = std::mem::take(&mut self.open_file_dialog_state);
        egui::Window::new("Open level...")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Path:");
                    let is_valid = engine.fs().is_file(&path);
                    let text_edit = egui::TextEdit::singleline(&mut path)
                        .id_source("Open level path")
                        .text_color_opt(if is_valid {
                            Some(egui::Rgba::from_gray(0.7).into())
                        } else {
                            None
                        });

                    let err_popup_id = ui.make_persistent_id("Open level dialogue error popup");

                    let response = ui.add(text_edit);
                    if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                        let lua = &engine.lua();
                        let file_result = engine.fs().open(&path);
                        let level_result = file_result.and_then(|file| {
                            Level::deserialize_from_single_binary_chunk(
                                engine.get::<Spaces>().borrow_mut().create_space(),
                                lua,
                                file,
                            )
                        });
                        match level_result {
                            Ok(level) => {
                                self.open_level(engine, lua, Arc::new(RwLock::new(level)))?;
                            }
                            Err(err) => {
                                let mut memory = ui.memory();
                                memory.toggle_popup(err_popup_id);
                                memory.id_data_temp.insert(err_popup_id, Arc::new(err));
                            }
                        }
                    }

                    egui::popup_below_widget(ui, err_popup_id, &response, |ui| {
                        let maybe_err = ui
                            .memory()
                            .id_data_temp
                            .get::<Arc<Error>>(&err_popup_id)
                            .cloned();
                        if let Some(err) = maybe_err {
                            ui.label(format!("{:?}", err));
                        }
                    });

                    Ok::<_, Error>(())
                });
            });
        self.open_file_dialog_is_open = open;
        self.open_file_dialog_state = path;
        Ok(())
    }

    pub fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        if matches!(self.current_level, Some(current) if !self.open_levels.contains(current)) {
            self.current_level = None;
        }

        if self.current_level.is_none() && !self.open_levels.is_empty() {
            self.current_level = Some(self.open_levels.iter().next().unwrap().0);
        }

        self.egui_resource.borrow_mut().begin_frame(engine);
        let egui_ctx = self.egui_ctx.borrow().clone();

        egui::TopBottomPanel::top("menubar")
            .show(&egui_ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    self.file_menu(engine, ui)?;

                    if let Some(current) = self.current_level {
                        self.open_levels[current].borrow_mut().menu(engine, ui)?;
                    }

                    self.view_menu(engine, ui)?;

                    Ok::<_, Error>(())
                })
                .inner?;

                Ok::<_, Error>(())
            })
            .inner?;

        self.show_open_file_dialog(engine, &egui_ctx)?;

        if let Some(current) = self.current_level {
            self.open_levels[current]
                .borrow_mut()
                .show_update(engine, &egui_ctx, dt)?;
        }

        let (canvas_response, ratio, image_dims) = egui::CentralPanel::default()
            .show(&egui_ctx, |ui| {
                let available = ui.available_size();
                let image_dims = Vector2::new(
                    self.world_canvas.color_buffer.width(),
                    self.world_canvas.color_buffer.height(),
                )
                .cast::<f32>();

                let ratio = (available.x / image_dims.x).min(available.y / image_dims.y);
                let canvas_response = ui
                    .centered_and_justified(|ui| {
                        ui.add(
                            egui::Image::new(
                                egui::TextureId::User(
                                    self.world_canvas.color_buffer.handle.gl_internal_id() as u64,
                                ),
                                (
                                    self.world_canvas.color_buffer.width() as f32 * ratio,
                                    self.world_canvas.color_buffer.height() as f32 * ratio,
                                ),
                            )
                            .sense(egui::Sense::click_and_drag()),
                        )
                    })
                    .inner;
                (canvas_response, ratio, image_dims)
            })
            .inner;

        let canvas_rect = canvas_response.rect;
        let canvas_center = canvas_rect.center();
        let window_to_canvas_tx = Translation2::new(0., image_dims.y)
            * Transform2::from_matrix_unchecked(Matrix3::new_nonuniform_scaling(&Vector2::new(
                1., -1.,
            )))
            * Similarity2::from_scaling(ratio.recip())
            * Translation2::from(image_dims * ratio / 2.)
            * Translation2::new(canvas_center.x, canvas_center.y).inverse();

        if let Some(current) = self.current_level {
            while let Some(event) = self.events.pop_front() {
                match event {
                    EngineEvent::MouseButtonDown { button, x, y } if canvas_response.hovered() => {
                        let canvas_point = window_to_canvas_tx.transform_point(&Point2::new(x, y));
                        self.open_levels[current].borrow_mut().event(
                            EngineEvent::MouseButtonDown {
                                button,
                                x: canvas_point.x,
                                y: canvas_point.y,
                            },
                        )?;
                    }
                    EngineEvent::MouseButtonUp { button, x, y } => {
                        let canvas_point = window_to_canvas_tx.transform_point(&Point2::new(x, y));
                        self.open_levels[current].borrow_mut().event(
                            EngineEvent::MouseButtonUp {
                                button,
                                x: canvas_point.x,
                                y: canvas_point.y,
                            },
                        )?;
                    }
                    EngineEvent::MouseMotion { x, y } => {
                        let canvas_vector =
                            window_to_canvas_tx.transform_vector(&Vector2::new(x, y));
                        self.open_levels[current]
                            .borrow_mut()
                            .event(EngineEvent::MouseMotion {
                                x: canvas_vector.x,
                                y: canvas_vector.y,
                            })?;
                    }
                    EngineEvent::MouseWheel { .. } => {
                        self.open_levels[current].borrow_mut().event(event)?;
                    }
                    EngineEvent::KeyDown { .. }
                    | EngineEvent::KeyUp { .. }
                    | EngineEvent::Char { .. }
                        if egui_ctx.memory().focus().is_none() =>
                    {
                        self.open_levels[current].borrow_mut().event(event)?;
                    }
                    _ => {}
                }
            }

            let mut level_mut = self.open_levels[current].borrow_mut();
            level_mut.mode_ctx.canvas_response = Some(canvas_response);
            level_mut.mode_ctx.window_to_canvas_tx = window_to_canvas_tx;
        } else {
            self.events.clear();
        }

        self.egui_resource.borrow_mut().end_frame(engine);

        Ok(())
    }

    pub fn draw(&mut self, engine: &Engine) -> Result<()> {
        {
            let mut gfx = self.gfx_lock.lock();

            gfx.set_projection(
                Orthographic3::new(
                    0.,
                    self.resolution.0 as f32,
                    self.resolution.1 as f32,
                    0.,
                    -1.,
                    1.,
                )
                .to_homogeneous(),
            );

            gfx.apply_default_pipeline();
            gfx.apply_transforms();
            gfx.begin_render_pass(
                Some(&self.world_canvas.render_pass),
                Some(ClearOptions {
                    color: Some(Color::BLACK),
                    ..ClearOptions::default()
                }),
            );

            drop(gfx);
            if let Some(current) = self.current_level {
                self.open_levels[current]
                    .borrow_mut()
                    .draw(&self.gfx_lock)?;
            }
            let mut gfx = self.gfx_lock.lock();

            gfx.end_render_pass();
            gfx.begin_render_pass(None, Some(ClearOptions::default()));
            gfx.apply_default_pipeline();
            gfx.apply_transforms();
        }

        self.egui_resource.borrow_mut().draw(engine);

        {
            let gfx = &mut self.gfx_lock.lock();

            gfx.end_render_pass();
            gfx.mq.commit_frame();
        }
        Ok(())
    }

    pub fn event(&mut self, engine: &Engine, event: EngineEvent) -> Result<()> {
        use EngineEvent::*;
        self.events.push_back(event);
        match event {
            KeyDown {
                keycode, keymods, ..
            } => self
                .egui_resource
                .borrow_mut()
                .key_down_event(engine, keycode, keymods),
            KeyUp { keycode, keymods } => self
                .egui_resource
                .borrow_mut()
                .key_up_event(keycode, keymods),
            Char { character, .. } => self.egui_resource.borrow_mut().char_event(character),
            MouseMotion { x, y } => self
                .egui_resource
                .borrow_mut()
                .mouse_motion_event(engine, x, y),
            MouseWheel { x, y } => self
                .egui_resource
                .borrow_mut()
                .mouse_wheel_event(engine, x, y),
            MouseButtonDown { button, x, y } => self
                .egui_resource
                .borrow_mut()
                .mouse_button_down_event(engine, button, x, y),
            MouseButtonUp { button, x, y } => self
                .egui_resource
                .borrow_mut()
                .mouse_button_up_event(engine, button, x, y),
        }

        Ok(())
    }
}

impl LuaUserData for Editor {}

impl LuaResource for Editor {
    const REGISTRY_KEY: &'static str = "TALISMAN_EDITOR";
}

pub struct EditorScene {
    pub editor: Resource<Editor>,
}

impl EditorScene {
    pub fn new(engine: &Engine) -> Result<Self> {
        Editor::new(engine).map(|editor| Self { editor })
    }
}

impl Scene<EngineRef, EngineEvent> for EditorScene {
    fn update(
        &mut self,
        _scene_stack: &mut SceneStack<EngineRef, EngineEvent>,
        ctx: &mut EngineRef,
    ) -> Result<()> {
        let engine = ctx.upgrade();
        engine.get::<Console>().borrow_mut().poll(&engine.lua())?;
        self.editor.borrow_mut().update(&ctx.upgrade(), 1. / 60.)?;
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineRef) -> Result<()> {
        self.editor.borrow_mut().draw(&ctx.upgrade())?;
        Ok(())
    }

    fn event(&mut self, ctx: &mut EngineRef, event: EngineEvent) -> Result<()> {
        let engine = ctx.upgrade();
        self.editor.borrow_mut().event(&engine, event)?;
        Ok(())
    }

    fn draw_previous(&self) -> bool {
        false
    }
}

pub(crate) fn open<'lua>(lua: &'lua Lua, engine: &Engine) -> Result<LuaTable<'lua>, Error> {
    let mut weak_resource_cache = WeakResourceCache::<Editor>::new();
    let get_editor = lua
        .create_function_mut(move |lua, ()| weak_resource_cache.get(|| lua.resource::<Editor>()))?;

    let mut weak_resource_cache = WeakResourceCache::<Editor>::new();
    let get_current_level = lua.create_function_mut(move |lua, ()| {
        let editor = weak_resource_cache.get(|| lua.resource::<Editor>())?;
        let editor_ref = editor.borrow();
        Ok(editor_ref
            .current_level
            .and_then(|current| editor_ref.open_levels.get(current).cloned())
            .map(|lv_editor| lv_editor.borrow().level.clone()))
    })?;

    let weak_engine = engine.downgrade();
    let mut weak_resource_cache = WeakResourceCache::<Editor>::new();
    let open_level = lua.create_function_mut(move |lua, level: Arc<RwLock<Level>>| {
        let editor = weak_resource_cache.get(|| lua.resource::<Editor>())?;
        let mut editor_mut = editor.borrow_mut();
        editor_mut
            .open_level(&weak_engine.upgrade(), lua, level)
            .to_lua_err()?;
        Ok(())
    })?;

    lua.load(mlua::chunk! {
        {
            get_current_level = $get_current_level,
            get_editor = $get_editor,
            open_level = $open_level,
        }
    })
    .eval()
    .map_err(Into::into)
}
