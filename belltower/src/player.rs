use std::collections::HashMap;

use anyhow::*;
use hv_core::{
    components::DynamicComponentConstructor, engine::Engine, input::InputState, inventory,
    mlua::prelude::*, plugins::Plugin,
};
use hv_friends::{math::*, nc::shape::Cuboid, Position, Velocity};

use crate::{
    box_geometry::{BoxCollider, BoxIndex},
    combat_geometry::CombatGeometry,
    Axes, Buttons,
};

const MAGNITUDE_EPSILON: f32 = 0.5;
const MOVE_SPEED: f32 = 160.;
const FOCUS_COEFF: f32 = 0.4;
const DASH_SPEED: f32 = 1000.;
const DASH_FRAMES: u32 = 7;
const SKID_COEFF: f32 = 0.5;
const SKID_FRAMES: u32 = 11;

pub struct PlayerInput {
    pub stick_direction: Vector2<f32>,
    pub frames_since_last_press: HashMap<Buttons, u32>,
    pub is_focused: bool,
}

impl PlayerInput {
    fn new() -> Self {
        Self {
            stick_direction: Vector2::zeros(),
            frames_since_last_press: [
                (Buttons::Dash, u32::MAX),
                (Buttons::Focus, u32::MAX),
                (Buttons::Swing, u32::MAX),
                (Buttons::Crush, u32::MAX),
            ]
            .iter()
            .copied()
            .collect(),
            is_focused: false,
        }
    }

    fn is_stick_centered(&self) -> bool {
        self.stick_direction.magnitude() <= MAGNITUDE_EPSILON
    }

    fn update(&mut self, input_state: &InputState<Axes, Buttons>) {
        let horz_axis_state = input_state.get_axis(Axes::Horz);
        let vert_axis_state = input_state.get_axis(Axes::Vert);
        self.stick_direction = Vector2::new(horz_axis_state, vert_axis_state);

        for (&button, frames_since_last_press) in self.frames_since_last_press.iter_mut() {
            if input_state.get_button_pressed(button) {
                *frames_since_last_press = 0;
            } else if input_state.get_button_down(button) {
                *frames_since_last_press = frames_since_last_press.saturating_add(1);
            } else {
                *frames_since_last_press = u32::MAX;
            }
        }

        self.is_focused = input_state.get_button_down(Buttons::Focus);
    }
}

pub enum StateTransition {
    None,
    Push(Box<dyn PlayerState>),
    Pop,
    To(Box<dyn PlayerState>),
}

pub trait PlayerState: Send + Sync {
    fn name(&self) -> &'static str;

    fn update(
        &mut self,
        position: &mut Position,
        velocity: &mut Velocity,
        combat_geometry: &mut CombatGeometry,
        input_state: &PlayerInput,
    ) -> Result<StateTransition>;
}

pub struct NormalState {
    direction: Vector2<f32>,
}

impl NormalState {
    pub fn new() -> Self {
        Self {
            direction: Vector2::zeros(),
        }
    }
}

impl PlayerState for NormalState {
    fn name(&self) -> &'static str {
        "run"
    }

    fn update(
        &mut self,
        position: &mut Position,
        velocity: &mut Velocity,
        combat_geometry: &mut CombatGeometry,
        input_state: &PlayerInput,
    ) -> Result<StateTransition> {
        if !input_state.is_stick_centered() {
            position.isometry.rotation =
                UnitComplex::rotation_between(&Vector2::x(), &input_state.stick_direction);
        }

        if input_state.frames_since_last_press[&Buttons::Dash] < 8
            && !input_state.is_stick_centered()
        {
            Ok(StateTransition::To(Box::new(DashState::new(
                input_state.stick_direction.normalize() * DASH_SPEED,
                DASH_FRAMES,
                SKID_COEFF,
                SKID_FRAMES,
            ))))
        } else if input_state.frames_since_last_press[&Buttons::Swing] < 8 {
            Ok(StateTransition::To(Box::new(SwingState::new(
                position.isometry.transform_vector(&Vector2::x()) * 160.,
            ))))
        } else {
            if !input_state.is_stick_centered() {
                let speed = if input_state.is_focused {
                    MOVE_SPEED * FOCUS_COEFF
                } else {
                    MOVE_SPEED
                };

                self.direction = input_state.stick_direction.normalize() * speed;
            } else {
                self.direction = Vector2::zeros();
            }

            velocity.velocity = Velocity2::new(self.direction, 0.);

            combat_geometry.clear();
            combat_geometry.hitbox(Cuboid::new(Vector2::repeat(4.)), Isometry2::identity());

            Ok(StateTransition::None)
        }
    }
}

pub struct DashState {
    linear_velocity: Vector2<f32>,
    dash_frame: u32,
    max_dash_frames: u32,
    skid_coeff: f32,
    skid_frames: u32,
}

impl DashState {
    pub fn new(
        linear_velocity: Vector2<f32>,
        dash_frames: u32,
        skid_coeff: f32,
        skid_frames: u32,
    ) -> Self {
        Self {
            linear_velocity,
            dash_frame: 0,
            max_dash_frames: dash_frames,
            skid_coeff,
            skid_frames,
        }
    }
}

impl PlayerState for DashState {
    fn name(&self) -> &'static str {
        "dash"
    }

    fn update(
        &mut self,
        position: &mut Position,
        velocity: &mut Velocity,
        _combat_geometry: &mut CombatGeometry,
        _input_state: &PlayerInput,
    ) -> Result<StateTransition> {
        position.isometry.rotation =
            UnitComplex::rotation_between(&Vector2::x(), &self.linear_velocity);

        if self.dash_frame < self.max_dash_frames {
            self.dash_frame += 1;
            velocity.velocity = Velocity2::new(self.linear_velocity, 0.);
            Ok(StateTransition::None)
        } else {
            Ok(StateTransition::To(Box::new(SkidState::new(
                self.linear_velocity * self.skid_coeff,
                self.skid_frames,
            ))))
        }
    }
}

pub struct SkidState {
    linear_velocity: Vector2<f32>,
    frame: u32,
    max_frames: u32,
}

impl SkidState {
    pub fn new(linear_velocity: Vector2<f32>, frames: u32) -> Self {
        Self {
            linear_velocity,
            frame: 0,
            max_frames: frames,
        }
    }
}

impl PlayerState for SkidState {
    fn name(&self) -> &'static str {
        "skid"
    }

    fn update(
        &mut self,
        _position: &mut Position,
        velocity: &mut Velocity,
        _combat_geometry: &mut CombatGeometry,
        input_state: &PlayerInput,
    ) -> Result<StateTransition> {
        if input_state.frames_since_last_press[&Buttons::Dash] < 4
            && !input_state.is_stick_centered()
        {
            Ok(StateTransition::To(Box::new(DashState::new(
                input_state.stick_direction.normalize() * DASH_SPEED,
                DASH_FRAMES,
                SKID_COEFF,
                SKID_FRAMES,
            ))))
        } else if self.frame < self.max_frames {
            self.frame += 1;
            let decay_factor = (self.max_frames - self.frame) as f32 / self.max_frames as f32;
            velocity.velocity = Velocity2::new(self.linear_velocity * decay_factor, 0.);
            Ok(StateTransition::None)
        } else {
            Ok(StateTransition::To(Box::new(NormalState::new())))
        }
    }
}

pub struct SwingState {
    hurtbox_begin: u32,
    hurtbox_end: u32,
    hurtbox: Option<BoxIndex>,

    parrybox_begin: u32,
    parrybox_end: u32,
    parrybox: Option<BoxIndex>,

    slide_end: u32,
    recovery_begin: u32,

    frame: u32,
    max_frames: u32,

    linear_velocity: Vector2<f32>,
}

impl SwingState {
    pub fn new(linear_velocity: Vector2<f32>) -> Self {
        Self {
            hurtbox_begin: 7,
            hurtbox_end: 14,
            hurtbox: None,

            parrybox_begin: 5,
            parrybox_end: 16,
            parrybox: None,

            slide_end: 11,
            recovery_begin: 16,

            frame: 0,
            max_frames: 21,

            linear_velocity,
        }
    }
}

impl PlayerState for SwingState {
    fn name(&self) -> &'static str {
        "swing"
    }

    fn update(
        &mut self,
        position: &mut Position,
        velocity: &mut Velocity,
        combat_geometry: &mut CombatGeometry,
        input_state: &PlayerInput,
    ) -> Result<StateTransition> {
        if self.frame < self.max_frames {
            if self.frame <= self.slide_end {
                let decay_factor = (self.slide_end - self.frame) as f32 / self.slide_end as f32;
                velocity.velocity = Velocity2::new(self.linear_velocity * decay_factor, 0.);
            }

            if self.frame == self.hurtbox_begin {
                self.hurtbox = Some(combat_geometry.geometry_mut().insert(BoxCollider::hurt(
                    Cuboid::new(Vector2::new(4., 12.)),
                    Isometry2::new(Vector2::new(16., 0.), 0.),
                )));
            } else if self.frame == self.hurtbox_end {
                combat_geometry.geometry_mut().remove(self.hurtbox.unwrap());
            }

            if self.frame == self.parrybox_begin {
                self.parrybox = Some(combat_geometry.geometry_mut().insert(BoxCollider::parry(
                    Cuboid::new(Vector2::new(6., 12.)),
                    Isometry2::new(Vector2::new(12., 0.), 0.),
                )));
            } else if self.frame == self.parrybox_end {
                combat_geometry
                    .geometry_mut()
                    .remove(self.parrybox.unwrap());
            }

            if input_state.frames_since_last_press[&Buttons::Swing] < 8
                && self.frame >= self.recovery_begin
            {
                if let Some(hurtbox) = self.hurtbox {
                    combat_geometry.geometry_mut().remove(hurtbox);
                }

                if let Some(parrybox) = self.parrybox {
                    combat_geometry.geometry_mut().remove(parrybox);
                }

                Ok(StateTransition::To(Box::new(SwingState::new(
                    position.isometry.transform_vector(&Vector2::x()) * 70.,
                ))))
            } else {
                self.frame += 1;
                Ok(StateTransition::None)
            }
        } else {
            Ok(StateTransition::To(Box::new(NormalState::new())))
        }
    }
}

pub struct PlayerController {
    stack: Vec<Box<dyn PlayerState>>,
    input: PlayerInput,
}

impl PlayerController {
    pub fn new() -> Self {
        Self {
            stack: vec![Box::new(NormalState::new())],
            input: PlayerInput::new(),
        }
    }

    pub fn update(
        &mut self,
        position: &mut Position,
        velocity: &mut Velocity,
        combat_geometry: &mut CombatGeometry,
        input_state: &InputState<Axes, Buttons>,
    ) -> Result<()> {
        self.input.update(input_state);

        loop {
            let transition = self.stack.last_mut().expect("empty state stack!").update(
                position,
                velocity,
                combat_geometry,
                &self.input,
            )?;

            match transition {
                StateTransition::None => break,
                StateTransition::Push(state) => self.stack.push(state),
                StateTransition::To(state) => *self.stack.last_mut().unwrap() = state,
                StateTransition::Pop => drop(self.stack.pop()),
            }
        }

        Ok(())
    }
}

struct PlayerControllerComponentPlugin;

impl Plugin for PlayerControllerComponentPlugin {
    fn name(&self) -> &'static str {
        "game.PlayerController"
    }

    fn open<'lua>(&self, lua: &'lua Lua, _engine: &Engine) -> Result<LuaTable<'lua>> {
        let new_pc = lua.create_function(|_, ()| {
            Ok(DynamicComponentConstructor::new(|_: &Lua, _| {
                Ok(PlayerController::new())
            }))
        })?;

        Ok(lua
            .load(mlua::chunk! {
                local PlayerController = {}

                return setmetatable(PlayerController, { __call = $new_pc })
            })
            .eval()?)
    }
}

hv_core::component!(PlayerControllerComponentPlugin);
