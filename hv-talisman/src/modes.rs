use std::collections::BTreeMap;

use hv_core::{prelude::*, spaces::Object};
use hv_egui::egui;
use hv_friends::{
    graphics::{Color, DrawMode, DrawableMut, GraphicsLockExt, Instance, MeshBuilder},
    math::*,
    scene::EngineEvent,
    Position,
};

use crate::editor::LevelContext;

pub enum Transition {
    Push(Box<dyn LevelEditorMode>),
    Pop,
    To(Box<dyn LevelEditorMode>),
    Noop,
}

pub trait LevelEditorMode: Send + Sync {
    fn enter(
        &mut self,
        _ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<()> {
        Ok(())
    }

    fn exit(
        &mut self,
        _ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<()> {
        Ok(())
    }

    fn update(
        &mut self,
        _ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<Transition> {
        Ok(Transition::Noop)
    }

    fn show(
        &mut self,
        _ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<Transition> {
        Ok(Transition::Noop)
    }

    fn draw(
        &mut self,
        _ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<()> {
        Ok(())
    }

    fn event(
        &mut self,
        _ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
        _event: EngineEvent,
    ) -> Result<Transition> {
        Ok(Transition::Noop)
    }
}

#[derive(Default)]
pub struct ModeStack {
    stack: Vec<Box<dyn LevelEditorMode>>,
}

impl ModeStack {
    fn do_op(&mut self, ctx: &mut LevelContext, op: Transition) -> Result<bool> {
        match op {
            Transition::Noop => return Ok(false),
            Transition::Pop => self.pop(ctx),
            Transition::Push(transition) => self.push(ctx, transition),
            Transition::To(transition) => self.to(ctx, transition),
        }?;

        Ok(true)
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.stack.clear();
    }

    pub fn push(&mut self, ctx: &mut LevelContext, mode: Box<dyn LevelEditorMode>) -> Result<()> {
        self.exit(ctx)?;
        self.stack.push(mode);
        self.enter(ctx)?;
        Ok(())
    }

    pub fn pop(&mut self, ctx: &mut LevelContext) -> Result<()> {
        self.exit(ctx)?;
        self.stack.pop();
        self.enter(ctx)?;
        Ok(())
    }

    pub fn to(&mut self, ctx: &mut LevelContext, mode: Box<dyn LevelEditorMode>) -> Result<()> {
        self.exit(ctx)?;
        self.stack.pop();
        self.stack.push(mode);
        self.enter(ctx)?;
        Ok(())
    }

    pub fn enter(&mut self, ctx: &mut LevelContext) -> Result<()> {
        self.stack
            .as_mut_slice()
            .split_last_mut()
            .map(|(last, rest)| last.enter(ctx, rest))
            .unwrap_or(Ok(()))
    }

    pub fn exit(&mut self, ctx: &mut LevelContext) -> Result<()> {
        self.stack
            .as_mut_slice()
            .split_last_mut()
            .map(|(last, rest)| last.exit(ctx, rest))
            .unwrap_or(Ok(()))
    }

    pub fn show(&mut self, ctx: &mut LevelContext) -> Result<()> {
        let op = self
            .stack
            .as_mut_slice()
            .split_last_mut()
            .map(|(last, rest)| last.show(ctx, rest))
            .unwrap_or(Ok(Transition::Noop))?;
        if self.do_op(ctx, op)? {
            self.show(ctx)
        } else {
            Ok(())
        }
    }

    pub fn update(&mut self, ctx: &mut LevelContext) -> Result<()> {
        let op = self
            .stack
            .as_mut_slice()
            .split_last_mut()
            .map(|(last, rest)| last.update(ctx, rest))
            .unwrap_or(Ok(Transition::Noop))?;
        if self.do_op(ctx, op)? {
            self.update(ctx)
        } else {
            Ok(())
        }
    }

    pub fn draw(&mut self, ctx: &mut LevelContext) -> Result<()> {
        self.stack
            .as_mut_slice()
            .split_last_mut()
            .map(|(last, rest)| last.draw(ctx, rest))
            .unwrap_or(Ok(()))
    }

    pub fn event(&mut self, ctx: &mut LevelContext, event: EngineEvent) -> Result<()> {
        let op = self
            .stack
            .as_mut_slice()
            .split_last_mut()
            .map(|(last, rest)| last.event(ctx, rest, event))
            .unwrap_or(Ok(Transition::Noop))?;
        if self.do_op(ctx, op)? {
            self.event(ctx, event)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct ContextMenuSubMode {
    pub position: egui::Pos2,
    pub object: Option<Object>,
}

impl LevelEditorMode for ContextMenuSubMode {
    fn show(
        &mut self,
        ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<Transition> {
        let egui_ctx = (*ctx.egui.borrow()).clone();
        let response = egui::Window::new("pan mode right click menu")
            .fixed_pos(self.position)
            .resizable(false)
            .title_bar(false)
            .show(&egui_ctx, |ui| {
                if ui.button("New object").clicked() {
                    ctx.level.borrow().space.borrow_mut().spawn(());

                    ctx.undo_tracker
                        .borrow_mut()
                        .mark(&ctx.engine.upgrade().lua(), "Create object".to_owned())?;

                    return Ok(Some(Transition::Pop));
                }

                if ui.button("New object at cursor").clicked() {
                    let canvas_position = ctx
                        .window_to_canvas_tx
                        .transform_point(&Point2::new(self.position.x, self.position.y));
                    let world_position = ctx
                        .camera
                        .borrow()
                        .screen_to_world_tx()
                        .transform_point(&canvas_position);

                    let position =
                        Position(Position2::translation(world_position.x, world_position.y));

                    ctx.level.borrow().space.borrow_mut().spawn((position,));

                    ctx.undo_tracker.borrow_mut().mark(
                        &ctx.engine.upgrade().lua(),
                        "Create object at cursor".to_owned(),
                    )?;

                    return Ok(Some(Transition::Pop));
                }

                ui.separator();

                if ui
                    .add(
                        egui::Button::new("Delete object at cursor").enabled(self.object.is_some()),
                    )
                    .clicked()
                {
                    let space = ctx.level.borrow().space.clone();
                    let _ = space
                        .borrow_mut()
                        .despawn(self.object.expect("would not be enabled if not some"));

                    ctx.undo_tracker.borrow_mut().mark(
                        &ctx.engine.upgrade().lua(),
                        "Delete object at cursor".to_owned(),
                    )?;

                    return Ok(Some(Transition::Pop));
                }

                if ui.button("Delete selected").clicked() {
                    let space = ctx.level.borrow().space.clone();
                    let mut space_mut = space.borrow_mut();
                    for &object in &ctx.selected_objects {
                        let _ = space_mut.despawn(object);
                    }

                    ctx.selected_objects.clear();

                    ctx.undo_tracker.borrow_mut().mark(
                        &ctx.engine.upgrade().lua(),
                        "Delete multiple objects".to_owned(),
                    )?;

                    return Ok(Some(Transition::Pop));
                }

                Ok::<_, Error>(None)
            });

        if let Some(response) = response {
            if let Some(Some(transition)) = response.inner.transpose()? {
                return Ok(transition);
            } else if response.response.clicked_elsewhere() {
                return Ok(Transition::Pop);
            }
        } else {
            return Ok(Transition::Pop);
        }

        Ok(Transition::Noop)
    }
}

#[derive(Debug, Default)]
struct PanSubMode;

impl LevelEditorMode for PanSubMode {
    fn update(
        &mut self,
        ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<Transition> {
        if let Some(response) = ctx.canvas_response.as_ref() {
            if response.dragged() {
                let window_delta = response.drag_delta();
                let canvas_delta = ctx
                    .window_to_canvas_tx
                    .transform_vector(&Vector2::new(window_delta.x, window_delta.y));
                let mut camera_mut = ctx.camera.borrow_mut();
                let camera_delta = camera_mut
                    .screen_to_world_tx()
                    .transform_vector(&canvas_delta);
                let pos = camera_mut.subject_pos();

                // The dragging wants to move the WORLD, not the subject!! If we added here, then
                // the camera would follow the position of the subject and the world would go in the
                // other direction... which is not intuitive (imo.)
                camera_mut.set_subject_pos(pos - camera_delta);
            } else {
                return Ok(Transition::Pop);
            }
        }

        Ok(Transition::Noop)
    }
}

#[derive(Default)]
struct MoveObjectSubMode;

impl LevelEditorMode for MoveObjectSubMode {
    fn update(
        &mut self,
        ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<Transition> {
        if let Some(response) = ctx.canvas_response.as_ref() {
            if response.dragged() {
                let delta = response.drag_delta();
                let delta_vector = ctx
                    .camera
                    .borrow()
                    .screen_to_world_tx()
                    .transform_vector(&Vector2::new(delta.x, delta.y));
                let space = ctx.level.borrow().space.clone();
                let mut space_mut = space.borrow_mut();
                for &object in &ctx.selected_objects {
                    if let Ok(Position(pos)) = space_mut.query_one_mut::<&mut Position>(object) {
                        pos.translation.vector += delta_vector;
                    }
                }
            } else {
                return Ok(Transition::Pop);
            }
        }

        Ok(Transition::Noop)
    }
}

const POSITION_INTERACTION_RADIUS: f32 = 8.;

#[derive(Default)]
pub struct BaseSubMode;

impl LevelEditorMode for BaseSubMode {
    fn update(
        &mut self,
        ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<Transition> {
        if let Some(response) = ctx.canvas_response.as_ref() {
            let closest = ctx.get_closest_interactable_object_to_interact_pointer_pos(
                POSITION_INTERACTION_RADIUS,
            );

            if let Some(closest) = closest {
                log::trace!("closest: {:?}", closest);
            }

            if response.clicked_by(egui::PointerButton::Primary) {
                log::trace!("clicked (primary)");

                if !response.ctx.input().modifiers.shift {
                    ctx.selected_objects.clear();
                }

                if let Some(closest) = closest {
                    if !ctx.selected_objects.insert(closest) {
                        ctx.selected_objects.remove(&closest);
                    }
                }
            } else if response.clicked_by(egui::PointerButton::Secondary) {
                log::trace!("clicked (secondary)");

                if let Some(position) = response.interact_pointer_pos() {
                    return Ok(Transition::Push(Box::new(ContextMenuSubMode {
                        position,
                        object: closest,
                    })));
                }
            } else if response.dragged() {
                log::trace!("dragged");

                if let Some(closest) = closest {
                    if !ctx.selected_objects.contains(&closest) {
                        ctx.selected_objects.clear();
                        ctx.selected_objects.insert(closest);
                    }

                    return Ok(Transition::Push(Box::new(MoveObjectSubMode::default())));
                } else {
                    return Ok(Transition::Push(Box::new(PanSubMode::default())));
                }
            }
        }

        Ok(Transition::Noop)
    }
}

#[derive(Default)]
pub struct NormalMode {
    stack: ModeStack,
}

impl LevelEditorMode for NormalMode {
    fn enter(
        &mut self,
        ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<()> {
        self.stack.clear();
        self.stack.push(ctx, Box::new(BaseSubMode::default()))?;

        Ok(())
    }

    fn update(
        &mut self,
        ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<Transition> {
        self.stack.update(ctx)?;

        Ok(Transition::Noop)
    }

    fn show(
        &mut self,
        ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<Transition> {
        self.stack.show(ctx)?;

        Ok(Transition::Noop)
    }

    fn draw(
        &mut self,
        ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
    ) -> Result<()> {
        let mut gfx = ctx.graphics_lock.lock();
        let mut mesh = MeshBuilder::new(gfx.state.null_texture.clone())
            .circle(
                DrawMode::fill(),
                Point2::origin(),
                POSITION_INTERACTION_RADIUS,
                0.1,
                Color::WHITE,
            )
            .build(&mut gfx);

        let camera_inverse_scale = ctx.camera.borrow().world_to_screen_tx().scaling();
        gfx.modelview_mut().push(ctx.camera.borrow().view_tx());
        gfx.apply_modelview();

        let maybe_closest =
            ctx.get_closest_interactable_object_to_hover_pos(POSITION_INTERACTION_RADIUS);

        let space = ctx.level.borrow().space.clone();
        let mut space_mut = space.borrow_mut();

        if let Some(closest) = maybe_closest {
            if let Ok(Position(pos)) = space_mut.query_one_mut::<&Position>(closest) {
                mesh.draw_mut(
                    &mut gfx,
                    Instance::new()
                        .translate2(pos.translation.vector)
                        .scale2(Vector2::repeat(camera_inverse_scale))
                        .color(Color::new(1.0, 1.0, 1.0, 0.6)),
                );
            }
        }

        for &object in &ctx.selected_objects {
            if let Ok(Position(pos)) = space_mut.query_one_mut::<&Position>(object) {
                mesh.draw_mut(
                    &mut gfx,
                    Instance::new()
                        .translate2(pos.translation.vector)
                        .scale2(Vector2::repeat(camera_inverse_scale)),
                );
            }
        }

        gfx.modelview_mut().pop();

        Ok(())
    }

    fn event(
        &mut self,
        ctx: &mut LevelContext,
        _prev: &mut [Box<dyn LevelEditorMode>],
        event: EngineEvent,
    ) -> Result<Transition> {
        if let EngineEvent::MouseWheel { y, .. } = event {
            let mut camera = ctx.camera.borrow_mut();
            let scale = camera.scale();
            camera.set_scale(scale + y / 100.);

            log::trace!("mouse wheel y-vel: {}", y);
        }

        self.stack.event(ctx, event)?;

        Ok(Transition::Noop)
    }
}

pub fn modes() -> BTreeMap<String, Box<dyn LevelEditorMode>> {
    let mut modes = BTreeMap::<String, Box<dyn LevelEditorMode>>::new();

    modes.insert("Normal".to_string(), Box::new(NormalMode::default()));

    modes
}

pub fn default_mode() -> Option<String> {
    Some("Normal".to_string())
}
