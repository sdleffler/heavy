use hv_core::{
    components::DynamicComponentConstructor,
    hecs::{Query, QueryItem, QueryOne, With},
    prelude::*,
    spaces::{Object, Space},
};
use hv_egui::egui;
use serde::*;
use std::{
    any::{Any, TypeId},
    collections::{hash_map::Entry, HashMap},
    fmt::Write,
    sync::RwLock,
};
use thunderdome::{Arena, Index};

use crate::{
    components::{Children, Hidden, Name, Parent, Visible},
    editor::{EditResult, LevelContext, UndoTracker},
    level::TalismanSerdePlugin,
};

pub trait ObjectProperty<'a>: Send + Sync + 'static {
    type Query: Query + 'a;
    type State: Send + Sync + 'static;

    const NAME: &'static str;
    const DISPLAY: &'static str;

    fn init<'q>(
        &self,
        object: Object,
        query: QueryItem<'q, Self::Query>,
        lua: &Lua,
    ) -> Result<Self::State>;

    fn edit<'q>(
        &self,
        object: Object,
        query: QueryItem<'q, Self::Query>,
        lua: &Lua,
        ui: &mut egui::Ui,
        state: &mut Self::State,
    ) -> Result<EditResult>;

    fn remove(&self, space: &mut Space, object: Object, lua: &Lua) -> Result<()>;
}

trait DynamicObjectProperty: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn display(&self) -> &'static str;

    fn init(
        &self,
        space: &mut Space,
        object: Object,
        lua: &Lua,
    ) -> Result<Option<Box<dyn Any + Send + Sync>>>;

    fn edit(
        &self,
        space: &mut Space,
        object: Object,
        lua: &Lua,
        ui: &mut egui::Ui,
        state: &mut (dyn Any + Send + Sync),
    ) -> Result<EditResult>;

    fn remove(&self, space: &mut Space, object: Object, lua: &Lua) -> Result<()>;
}

impl<'a, T: ObjectProperty<'a>> DynamicObjectProperty for T {
    fn name(&self) -> &'static str {
        T::NAME
    }

    fn display(&self) -> &'static str {
        T::DISPLAY
    }

    fn init(
        &self,
        space: &mut Space,
        object: Object,
        lua: &Lua,
    ) -> Result<Option<Box<dyn Any + Send + Sync>>> {
        match space.query_one_mut::<T::Query>(object) {
            Ok(q_item) => Ok(Some(Box::new(ObjectProperty::init(
                self, object, q_item, lua,
            )?))),
            Err(err) => {
                log::trace!(
                    "lop {} init failed on object {:?}, skipping; err: {}",
                    self.name(),
                    object,
                    err
                );
                Ok(None)
            }
        }
    }

    fn edit(
        &self,
        space: &mut Space,
        object: Object,
        lua: &Lua,
        ui: &mut egui::Ui,
        state: &mut (dyn Any + Send + Sync),
    ) -> Result<EditResult> {
        ObjectProperty::edit(
            self,
            object,
            space.query_one_mut::<T::Query>(object)?,
            lua,
            ui,
            state
                .downcast_mut::<T::State>()
                .expect("level object property UI state type mismatch!!"),
        )
    }

    fn remove(&self, space: &mut Space, object: Object, lua: &Lua) -> Result<()> {
        ObjectProperty::remove(self, space, object, lua)
    }
}

pub struct ObjectPropertyPlugin {
    inner: Box<dyn DynamicObjectProperty>,
}

impl ObjectPropertyPlugin {
    pub fn new(lop: impl for<'a> ObjectProperty<'a>) -> Self {
        Self {
            inner: Box::new(lop),
        }
    }
}

inventory::collect!(ObjectPropertyPlugin);

pub struct LopState {
    index: Index,
    state: Box<dyn Any + Send + Sync>,
    live: bool,
}

#[derive(Default)]
struct AddComponentState {
    text: String,
    err: Option<String>,
}

pub struct ObjectHeaderState {
    component_types: Vec<TypeId>,

    lop_states: Vec<LopState>,
    live: bool,

    has_children: bool,
    is_visible: bool,

    add_component_state: Option<AddComponentState>,
}

pub struct ObjectProperties {
    lops: Arena<&'static dyn DynamicObjectProperty>,
    data: HashMap<Object, ObjectHeaderState>,
}

impl Default for ObjectProperties {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectProperties {
    pub fn new() -> Self {
        let mut lops = Arena::new();

        for plugin in inventory::iter::<ObjectPropertyPlugin> {
            lops.insert(&*plugin.inner);
        }

        Self {
            lops,
            data: HashMap::new(),
        }
    }

    pub fn update(&mut self) {
        let Self { data, .. } = self;

        data.retain(|_, header_state| {
            let live_this_update = header_state.live;
            header_state.live = false;
            live_this_update
        });
    }

    pub fn show_object(
        &mut self,
        space: &Shared<Space>,
        object: Object,
        object_name: &str,
        lua: &Lua,
        ui: &mut egui::Ui,
        undo_tracker: &mut UndoTracker,
    ) -> Result<()> {
        let Self { lops, data } = self;
        let mut space_mut = Some(space.borrow_mut());

        let entry = data.entry(object);
        let is_dirty = match &entry {
            Entry::Vacant(_) => true,
            Entry::Occupied(occupied) => space_mut
                .as_ref()
                .unwrap()
                .ecs
                .entity(object.entity())
                .unwrap()
                .component_types()
                .ne(occupied.get().component_types.iter().copied()),
        };

        let header_state = if is_dirty {
            let mut lop_states = Vec::new();
            for (index, lop) in lops.iter() {
                if let Some(state) = lop.init(space_mut.as_mut().unwrap(), object, lua)? {
                    lop_states.push(LopState {
                        index,
                        state,
                        live: true,
                    });
                }
            }

            let has_children = space_mut
                .as_mut()
                .unwrap()
                .query_one_mut::<With<Children, ()>>(object)
                .is_ok();

            let is_visible = space_mut
                .as_mut()
                .unwrap()
                .query_one_mut::<&Visible>(object)
                .ok()
                .map(|&Visible(b)| b)
                .unwrap_or(true);

            lop_states.sort_by_key(|state| lops[state.index].name());
            entry
                .insert(ObjectHeaderState {
                    component_types: space_mut
                        .as_ref()
                        .unwrap()
                        .ecs
                        .entity(object.entity())
                        .unwrap()
                        .component_types()
                        .collect(),
                    lop_states,
                    live: true,
                    has_children,
                    is_visible,
                    add_component_state: None,
                })
                .into_mut()
        } else {
            match entry {
                Entry::Occupied(occupied) => occupied.into_mut(),
                Entry::Vacant(_) => unreachable!(),
            }
        };

        header_state
            .lop_states
            .sort_by_key(|state| lops[state.index].name());

        header_state.live = true;

        egui::Grid::new("grid")
            .striped(true)
            .num_columns(2)
            .show(ui, |ui| {
                for lop_state in &mut header_state.lop_states {
                    let lop = &lops[lop_state.index];

                    let lop_display = lop.display();
                    let menu_popup_id =
                        ui.make_persistent_id((object, lop_display, "menu_popup_id"));
                    let label_response = ui.button(lop_display);
                    if label_response.clicked() {
                        ui.memory().toggle_popup(menu_popup_id);
                    }

                    match lop.edit(
                        space_mut.as_mut().unwrap(),
                        object,
                        lua,
                        ui,
                        lop_state.state.as_mut(),
                    ) {
                        Ok(edit_result) => {
                            lop_state.live = true;

                            match edit_result {
                                EditResult::MarkUndoPoint(label) => {
                                    drop(space_mut.take());
                                    undo_tracker.mark(lua, label)?;
                                    space_mut = Some(space.borrow_mut());
                                }
                                EditResult::Unedited => {}
                            }
                        }
                        Err(err) => {
                            log::error!("{}", err);
                        }
                    }

                    egui::popup_below_widget(ui, menu_popup_id, &label_response, |ui| {
                        ui.set_min_width(200.);

                        if ui.button("Remove component").clicked() {
                            lop.remove(space_mut.as_mut().unwrap(), object, lua)?;
                            drop(space_mut.take());
                            undo_tracker.mark(
                                lua,
                                format!(
                                    "Remove component `{}` from object #{}",
                                    lop_display,
                                    object.slot()
                                ),
                            )?;
                            space_mut = Some(space.borrow_mut());
                        }

                        Ok::<_, Error>(())
                    })
                    .transpose()?;

                    ui.end_row();
                }

                Ok::<_, Error>(())
            })
            .inner?;

        let states = &mut header_state.lop_states;
        let mut i = 0;
        while i < states.len() {
            let state = &mut states[i];
            let live_this_update = state.live;
            state.live = false;

            if live_this_update {
                i += 1;
            } else {
                states.remove(i);
            }
        }

        let menu_popup_id = ui.make_persistent_id((object.slot(), "popup menu"));

        let edit_button_response = ui.button("Edit...");
        if edit_button_response.clicked() {
            ui.memory().toggle_popup(menu_popup_id);
        }

        egui::popup_below_widget(ui, menu_popup_id, &edit_button_response, |ui| {
            ui.set_min_width(200.);
            if ui.button("Add component").clicked() {
                header_state.add_component_state = Some(AddComponentState::default());
            }

            ui.separator();

            if ui
                .checkbox(&mut header_state.has_children, "Parent")
                .clicked()
            {
                if !header_state.has_children {
                    space_mut
                        .as_mut()
                        .unwrap()
                        .insert_one(object, Children(Vec::new()))?;

                    drop(space_mut.take());
                    undo_tracker.mark(lua, format!("Make object {} a parent", object_name))?;
                } else {
                    let space_mut_tmp = &mut *space_mut.as_mut().unwrap();
                    let Children(children) = space_mut_tmp.remove_one(object)?;

                    if let Ok(Parent(parent)) = space_mut_tmp.remove_one(object) {
                        for &child in &children {
                            space_mut_tmp.insert_one(child, Parent(parent))?;
                        }

                        let Children(ref mut parent_children) =
                            space_mut_tmp.query_one_mut::<&mut Children>(parent)?;
                        parent_children.extend_from_slice(&children);
                    } else {
                        for child in children {
                            space_mut_tmp.remove_one::<Parent>(child)?;
                        }
                    }

                    drop(space_mut.take());
                    undo_tracker
                        .mark(lua, format!("Remove children from object {}", object_name))?;
                }

                space_mut = Some(space.borrow_mut());
            }

            ui.separator();

            if ui
                .checkbox(&mut header_state.is_visible, "Visible")
                .clicked()
            {
                space_mut
                    .as_mut()
                    .unwrap()
                    .insert_one(object, Visible(header_state.is_visible))?;

                drop(space_mut.take());
                undo_tracker.mark(lua, format!("Change visibility of object {}", object_name))?;
                space_mut = Some(space.borrow_mut());
            }

            Ok::<_, Error>(())
        })
        .transpose()?;

        if let Some(add_component_state) = header_state.add_component_state.as_mut() {
            let response = ui.add(
                egui::TextEdit::singleline(&mut add_component_state.text)
                    .hint_text("<constructor>"),
            );

            if response.lost_focus() {
                add_component_state.err = None;
            }

            if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                match lua.load(&add_component_state.text).eval::<LuaAnyUserData>() {
                    Ok(ud) => {
                        let dcc = ud.borrow::<DynamicComponentConstructor>()?;
                        dcc.insert_on_object(lua, object, space_mut.as_deref_mut().unwrap())?;
                        drop(space_mut.take());
                        undo_tracker.mark(
                            lua,
                            format!(
                                "Add component `{}` to object #{}",
                                add_component_state.text,
                                object.slot()
                            ),
                        )?;
                    }
                    Err(err) => {
                        add_component_state.err = Some(format!("{:?}", err));
                    }
                }
            }

            if let Some(err) = add_component_state.err.as_ref() {
                ui.label(err);
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct ObjectTreeNode {
    pub object: Object,
    pub egui_id: egui::Id,
    pub parent: Option<Object>,
    pub children: Vec<Object>,
}

#[derive(Debug, Default)]
struct TempState {
    source: Option<Object>,
    source_name: Option<String>,
    target: Option<Option<Object>>,
    target_name: Option<String>,

    label_buf: String,
    child_buf: Vec<Object>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct ObjectTreeRoots {
    objects: Vec<Object>,
}

inventory::submit!(TalismanSerdePlugin::serde::<ObjectTreeRoots>(
    "talisman.ObjectTreeRoots"
));

pub struct ObjectTree {
    roots_object: Object,
    roots: Vec<Object>,
    object_properties: RwLock<ObjectProperties>,
}

const EXPECTED_PARENT_TO_HAVE_CHILDREN_ERR: &str = "a child object w/ a parent component \
    should have a parent with a `Children` component, but the parent object did not have a \
    `Children` component!";

const EXPECTED_PARENT_TO_CONTAIN_CHILD_ERR: &str = "a parent object didn't contain an object \
    claiming to be its child!";

impl ObjectTree {
    pub fn new(ctx: &mut LevelContext) -> Self {
        let space = ctx.level.borrow().space.clone();
        let mut space_mut = space.borrow_mut();
        let mut roots_query = space_mut.query_mut::<&ObjectTreeRoots>().into_iter();

        let (roots_object, roots) = if let Some((object, roots_component)) = roots_query.next() {
            let tmp = object;
            if roots_query.next().is_some() {
                log::warn!("Level contains multiple objects with `ObjectTreeRoot` components!");
            }
            (tmp, roots_component.objects.clone())
        } else {
            (
                space_mut.spawn((ObjectTreeRoots::default(), Hidden)),
                Vec::new(),
            )
        };

        Self {
            roots_object,
            roots,
            object_properties: RwLock::new(ObjectProperties::new()),
        }
    }

    fn show_node(
        &self,
        space_resource: &Shared<Space>,
        lua: &Lua,
        ui: &mut egui::Ui,
        undo_tracker: &mut UndoTracker,
        temp_state: &mut TempState,
        object: Option<Object>,
    ) -> Result<()> {
        type QueryType<'a> = (Option<&'a Name>, Option<&'a Children>);

        temp_state.label_buf.clear();
        let space = space_resource.borrow();
        let mut query: Option<QueryOne<QueryType>> = None;

        let child_tmp;
        let children;
        if let Some(object) = object {
            query = Some(space.query_one(object)?);
            let (name, children_tmp) = query
                .as_mut()
                .unwrap()
                .get()
                .expect("query is always satisfiable");

            if let Some(Name(name)) = name {
                write!(&mut temp_state.label_buf, "{} (#{})", name, object.slot())?;
            } else {
                write!(&mut temp_state.label_buf, "#{}", object.slot())?;
            }

            children = children_tmp;
        } else {
            child_tmp = Children(self.roots.clone());
            children = Some(&child_tmp);

            write!(&mut temp_state.label_buf, "Root")?;
        };

        let base_id = egui::Id::new("tree_node");
        let egui_id = match object {
            Some(object) => base_id.with(object.slot()),
            None => base_id.with("root"),
        };

        // We want to be able to open collapsible headers, so the drag selector must be pressed in
        // order to actually drag anything.
        let drag_selector = ui.input().modifiers.ctrl;
        let is_anything_being_dragged = ui.memory().is_anything_being_dragged() && drag_selector;
        let is_being_dragged =
            ui.memory().is_being_dragged(egui_id) && object.is_some() && drag_selector;

        ui.set_min_width(ui.available_width());

        if !is_being_dragged {
            let margin = egui::Vec2::splat(4.0);

            let outer_rect_bounds = ui.available_rect_before_wrap();
            let inner_rect = outer_rect_bounds.shrink2(margin);
            let where_to_put_background = ui.painter().add(egui::Shape::Noop);
            let mut content_ui = ui.child_ui(inner_rect, *ui.layout());
            let has_children = children.is_some();

            content_ui.set_min_width(inner_rect.width());

            let node_response = if let Some(Children(children)) = children {
                let children = children.clone();
                drop(query);
                drop(space);

                let label_buf = std::mem::take(&mut temp_state.label_buf);
                let response = content_ui.collapsing(&label_buf, |ui| {
                    // Root node will never have any properties, so don't show them.
                    if let Some(object) = object {
                        ui.scope(|ui| {
                            self.object_properties.borrow_mut().show_object(
                                space_resource,
                                object,
                                &label_buf,
                                lua,
                                ui,
                                undo_tracker,
                            )
                        })
                        .inner?;
                    }

                    if !children.is_empty() {
                        ui.separator();

                        for child in children {
                            ui.scope(|ui| {
                                self.show_node(
                                    space_resource,
                                    lua,
                                    ui,
                                    undo_tracker,
                                    temp_state,
                                    Some(child),
                                )
                            })
                            .inner?;
                        }
                    }
                    Ok::<_, Error>(())
                });

                response.body_returned.transpose()?;
                response.header_response
            } else {
                drop(query);
                drop(space);

                let response = content_ui.collapsing(&temp_state.label_buf, |ui| {
                    ui.scope(|ui| {
                        self.object_properties.borrow_mut().show_object(
                            space_resource,
                            object.expect("root node will never not have children"),
                            &temp_state.label_buf,
                            lua,
                            ui,
                            undo_tracker,
                        )
                    })
                    .inner
                });

                response.body_returned.transpose()?;
                response.header_response
            };

            let node_response =
                content_ui.interact(node_response.rect, egui_id, egui::Sense::drag());

            if node_response.hovered() {
                if is_anything_being_dragged {
                    temp_state.target = Some(object);
                    temp_state.target_name = Some(temp_state.label_buf.clone());
                } else {
                    content_ui.output().cursor_icon = egui::CursorIcon::Grab;
                }
            }

            let outer_rect =
                egui::Rect::from_min_max(outer_rect_bounds.min, content_ui.min_rect().max + margin);
            let (rect, response) = ui.allocate_at_least(outer_rect.size(), egui::Sense::hover());

            let style = if is_anything_being_dragged && response.hovered() {
                if has_children {
                    ui.visuals().widgets.active
                } else {
                    ui.visuals().widgets.hovered
                }
            } else {
                ui.visuals().widgets.inactive
            };

            let mut fill = style.bg_fill;
            let mut stroke = style.bg_stroke;
            if is_being_dragged && has_children {
                // gray out:
                fill = egui::color::tint_color_towards(fill, ui.visuals().window_fill());
                stroke.color =
                    egui::color::tint_color_towards(stroke.color, ui.visuals().window_fill());
            }

            ui.painter().set(
                where_to_put_background,
                egui::Shape::Rect {
                    corner_radius: style.corner_radius,
                    fill,
                    stroke,
                    rect,
                },
            );
        } else {
            ui.output().cursor_icon = egui::CursorIcon::Grabbing;

            let layer_id = egui::LayerId::new(egui::Order::Tooltip, egui_id);
            let response = ui
                .with_layer_id(layer_id, |ui| ui.label(&temp_state.label_buf))
                .response;

            if let Some(pointer_pos) = ui.input().pointer.interact_pos() {
                let delta = pointer_pos - response.rect.center();
                ui.ctx().translate_layer(layer_id, delta);
            }

            debug_assert!(object.is_some());
            temp_state.source = object;
            temp_state.source_name = Some(temp_state.label_buf.clone());
        }

        Ok(())
    }

    pub fn show(&mut self, ctx: &mut LevelContext, lua: &Lua, ui: &mut egui::Ui) -> Result<()> {
        self.object_properties.get_mut().unwrap().update();
        let space = ctx.level.borrow().space.clone();

        {
            let mut space_mut = space.borrow_mut();
            space_mut
                .query_one_mut::<&ObjectTreeRoots>(self.roots_object)
                .expect("invalid roots_object")
                .objects
                .clone_into(&mut self.roots);

            self.roots.retain(|&root| space_mut.contains(root));
            for (object, _) in space_mut
                .query_mut::<()>()
                .without::<Parent>()
                .without::<Hidden>()
            {
                if !self.roots.contains(&object) {
                    self.roots.push(object);
                }
            }
        }

        let mut temp_state = TempState::default();
        self.show_node(
            &space,
            lua,
            ui,
            &mut ctx.undo_tracker.borrow_mut(),
            &mut temp_state,
            None,
        )?;

        if let (Some(source), Some(target)) = (temp_state.source, temp_state.target) {
            if ui.input().pointer.any_released() {
                let mut space_mut = space.borrow_mut();

                // Source and target name fields must be non-empty if we have source and target
                // objects.
                let source_name = temp_state.source_name.unwrap();
                let target_name = temp_state.target_name.unwrap();

                // Move the indices around according to the drop.
                let source_parent = space_mut.query_one_mut::<&Parent>(source).ok().copied();

                let (source_parent, source_index) =
                    if let Some(Parent(source_parent)) = source_parent {
                        let Children(source_parent_children) = space_mut
                            .query_one_mut::<&mut Children>(source_parent)
                            .with_context(|| Error::msg(EXPECTED_PARENT_TO_HAVE_CHILDREN_ERR))?;
                        // Remove the source object from its parent's children list.
                        let pos = source_parent_children
                            .iter()
                            .position(|&object| object == source)
                            .ok_or_else(|| Error::msg(EXPECTED_PARENT_TO_CONTAIN_CHILD_ERR))?;
                        source_parent_children.remove(pos);
                        (Some(source_parent), pos)
                    } else {
                        // If it doesn't have a parent, then it should be in the root node list.
                        let pos = self
                            .roots
                            .iter()
                            .position(|&object| object == source)
                            .ok_or_else(|| Error::msg(EXPECTED_PARENT_TO_CONTAIN_CHILD_ERR))?;
                        self.roots.remove(pos);
                        (None, pos)
                    };

                let message_buf = match target {
                    // If this *isn't* an insertion on the root node...
                    Some(target) => {
                        if let Ok(Children(children)) =
                            space_mut.query_one_mut::<&mut Children>(target)
                        {
                            // If the target node can have children, then add the source node to the
                            // new target node parent's `Children` component, and set its' `Parent`
                            // component accordingly.
                            children.push(source);
                            space_mut.insert_one(source, Parent(target))?;

                            format!(
                                "Object hierarchy: Move object {} to the end of object {}'s children",
                                source_name, target_name
                            )
                        } else if let Some(Parent(target_parent)) =
                            space_mut.query_one_mut::<&Parent>(target).ok().copied()
                        {
                            // If the new target node *can't* have children, but does have a parent,
                            // then this is a rearranging operation on that parent's children.
                            let Children(target_parent_children) = space_mut
                                .query_one_mut::<&mut Children>(target_parent)
                                .with_context(|| {
                                    Error::msg(EXPECTED_PARENT_TO_HAVE_CHILDREN_ERR)
                                })?;
                            let pos = target_parent_children
                                .iter()
                                .position(|&child| child == target)
                                .expect("child must be present in the parent's child list");

                            // This check produces some special behavior when you drag a node onto
                            // the node below it. Without this, the dragged node would actually go
                            // right back to where it was, while you might expect it to *actually*
                            // skip ahead and swap with the node below! So we check to see if we're
                            // in this situation, and if so, skip ahead one.
                            if source_parent == Some(target_parent) && source_index == pos {
                                target_parent_children.insert(pos + 1, source);
                            } else {
                                target_parent_children.insert(pos, source);
                            }

                            space_mut.insert_one(source, Parent(target_parent))?;

                            format!(
                                "Object hierarchy: Move object {} adjacent to object {}",
                                source_name, target_name
                            )
                        } else {
                            // If the new target node can't have children and doesn't have a parent,
                            // then this is a rearranging operation on the root nodes. There isn't
                            // really any meaning to this since the order currently isn't stored,
                            // but it might be some day.
                            let pos = self
                                .roots
                                .iter()
                                .position(|&child| child == target)
                                .unwrap_or_else(|| self.roots.len());

                            // Same check as in the non-root case, just avoid putting it back where
                            // it was.
                            if source_parent == None && source_index == pos {
                                self.roots.insert(pos + 1, source);
                            } else {
                                self.roots.insert(pos, source);
                            }

                            let _ = space_mut.remove_one::<Parent>(source);

                            format!(
                                "Object hierarchy: Move object {} adjacent to object {}",
                                source_name, target_name
                            )
                        }
                    }
                    // If this *is* an insertion on the root node...
                    None => {
                        // Easy, just push it on the end.
                        self.roots.push(source);
                        format!("Object hierarchy: Move object {} to root", source_name)
                    }
                };

                // Push the object tree roots to the undo tracker so that they're included when we
                // next undo.
                space_mut
                    .query_one_mut::<&mut ObjectTreeRoots>(self.roots_object)?
                    .objects
                    .clone_from(&self.roots);

                drop(space_mut);
                ctx.undo_tracker.borrow_mut().mark(lua, message_buf)?;
            }
        }

        Ok(())
    }
}
