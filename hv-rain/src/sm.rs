use hv_core::{
    engine::LuaResource,
    mlua::{prelude::*, Variadic as LuaVariadic},
};
use hv_friends::math::*;
use smallbox::{smallbox, space::S4, SmallBox};
use std::{any::Any, sync::Arc};
use thunderdome::{Arena, Index};

use crate::{graphics::ProjectileSprite, ProjectileState};

pub type DynExtraSmState = SmallBox<dyn ExtraSmState, S4>;

pub trait ExtraSmState: Any + Send + Sync {
    #[doc(hidden)]
    fn as_any_ref(&self) -> &(dyn Any + Send + Sync);

    #[doc(hidden)]
    fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync);

    #[doc(hidden)]
    fn small_box_clone(&self) -> DynExtraSmState;
}

impl<T: Any + Send + Sync + Clone> ExtraSmState for T {
    fn as_any_ref(&self) -> &(dyn Any + Send + Sync) {
        self
    }

    fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync) {
        self
    }

    fn small_box_clone(&self) -> DynExtraSmState {
        smallbox!(self.clone())
    }
}

impl dyn ExtraSmState {
    pub fn downcast_ref<T: ExtraSmState>(&self) -> Option<&T> {
        self.as_any_ref().downcast_ref()
    }

    pub fn downcast_mut<T: ExtraSmState>(&mut self) -> Option<&mut T> {
        self.as_mut_any().downcast_mut()
    }
}

pub struct StateMachine {
    pub index: StateIndex,
    pub time: f32,

    pub linear_velocity: Velocity2<f32>,
    pub polar_velocity: Velocity2<f32>,

    pub extra: Option<SmallBox<dyn ExtraSmState, S4>>,
}

impl Clone for StateMachine {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            time: self.time,
            linear_velocity: self.linear_velocity,
            polar_velocity: self.polar_velocity,
            extra: self.extra.as_deref().map(ExtraSmState::small_box_clone),
        }
    }
}

impl StateMachine {
    pub fn new(initial_state: StateIndex) -> Self {
        Self {
            index: initial_state,
            time: 0.,
            linear_velocity: Velocity2::zero(),
            polar_velocity: Velocity2::zero(),
            extra: None,
        }
    }
}

pub enum Transition {
    To(StateIndex),
    Done,
    None,
}

pub trait State: Send + Sync + 'static {
    fn enter(
        &self,
        _lua: &Lua,
        _machine: &StateRegistry,
        _state: &mut ProjectileState,
        _fsm: &mut StateMachine,
    ) {
    }

    fn update(
        &self,
        _lua: &Lua,
        _machine: &StateRegistry,
        _dt: f32,
        _projectile_state: &mut ProjectileState,
        _fsm: &mut StateMachine,
    ) -> Transition {
        Transition::None
    }

    fn cleanup(
        &self,
        _lua: &Lua,
        _machine: &StateRegistry,
        _state: &mut ProjectileState,
        _fsm: &mut StateMachine,
    ) {
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateIndex(Index);

impl<'lua> ToLua<'lua> for StateIndex {
    fn to_lua(self, _lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        Ok(LuaValue::LightUserData(LuaLightUserData(
            self.0.to_bits() as *mut _
        )))
    }
}

impl<'lua> FromLua<'lua> for StateIndex {
    fn from_lua(lua_value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self(Index::from_bits(
            LuaLightUserData::from_lua(lua_value, lua)?.0 as u64,
        )))
    }
}

pub struct StateRegistry {
    terminal: StateIndex,
    behaviors: Arena<Box<dyn State>>,
}

impl Default for StateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl StateRegistry {
    pub fn new() -> Self {
        let mut behaviors = Arena::<Box<dyn State>>::new();
        let terminal = StateIndex(behaviors.insert(Box::new(())));

        Self {
            terminal,
            behaviors,
        }
    }

    pub fn insert(&mut self, state: impl State) -> StateIndex {
        StateIndex(self.behaviors.insert(Box::new(state)))
    }

    pub fn enter(
        &self,
        lua: &Lua,
        projectile_state: &mut ProjectileState,
        fsm_state: &mut StateMachine,
    ) {
        let machine_state = fsm_state.index;
        self.behaviors[machine_state.0].enter(lua, self, projectile_state, fsm_state);
    }

    pub fn update(
        &self,
        lua: &Lua,
        dt: f32,
        projectile_state: &mut ProjectileState,
        fsm_state: &mut StateMachine,
    ) -> bool {
        loop {
            let machine_state = fsm_state.index;
            match self.behaviors[machine_state.0].update(lua, self, dt, projectile_state, fsm_state)
            {
                Transition::To(new_state) => {
                    fsm_state.index = new_state;
                    self.behaviors[machine_state.0].cleanup(lua, self, projectile_state, fsm_state);
                    self.behaviors[new_state.0].enter(lua, self, projectile_state, fsm_state);
                    continue;
                }
                Transition::Done => {
                    self.behaviors[machine_state.0].cleanup(lua, self, projectile_state, fsm_state);
                    fsm_state.index = self.terminal;
                    return true;
                }
                Transition::None => return false,
            }
        }
    }
}

impl LuaUserData for StateRegistry {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut(
            "sprite",
            |_lua, this, (sprite, wait): (Option<ProjectileSprite>, Option<bool>)| {
                Ok(this.insert(Sprite {
                    sprite,
                    wait: wait.unwrap_or(true),
                    next: None,
                }))
            },
        );

        methods.add_method_mut(
            "sprite_sequence",
            |_lua, this, sprites: LuaVariadic<ProjectileSprite>| {
                Ok(sprites
                    .into_iter()
                    .rev()
                    .fold(None, |acc, sprite| {
                        Some(this.insert(Sprite {
                            sprite: Some(sprite),
                            wait: true,
                            next: acc,
                        }))
                    })
                    .unwrap_or(this.terminal))
            },
        );

        methods.add_method_mut("sequence", |_lua, this, states: LuaVariadic<StateIndex>| {
            Ok(this.insert(Sequence {
                sequential_states: states.into_iter().collect(),
            }))
        });

        methods.add_method_mut("parallel", |_lua, this, states: LuaVariadic<StateIndex>| {
            Ok(this.insert(Parallel {
                parallel_states: states.into_iter().collect(),
            }))
        });

        methods.add_method_mut("wait", |_lua, this, t| Ok(this.insert(Wait { t })));

        methods.add_method_mut("lerp_linear_speed", |_lua, this, (from, to, duration)| {
            Ok(this.insert(LerpLinearSpeed { from, to, duration }))
        });

        methods.add_method_mut(
            "lerp_polar_angular_speed",
            |_lua, this, (from, to, duration)| {
                Ok(this.insert(LerpPolarAngularSpeed { from, to, duration }))
            },
        );

        methods.add_method_mut(
            "lerp_polar_linear_speed",
            |_lua, this, (from, to, duration)| {
                Ok(this.insert(LerpPolarLinearSpeed { from, to, duration }))
            },
        );

        methods.add_method_mut("kill", |_lua, this, ()| Ok(this.insert(Kill)));

        methods.add_method_mut("halt", |_lua, this, ()| Ok(this.terminal));
    }
}

impl LuaResource for StateRegistry {
    const REGISTRY_KEY: &'static str = "HV_DANMAKU_STATE_REGISTRY";
}

impl State for () {}

#[derive(Debug, Clone, Copy)]
pub struct LerpLinearSpeed {
    pub from: f32,
    pub to: f32,
    pub duration: f32,
}

impl State for LerpLinearSpeed {
    fn enter(
        &self,
        _: &Lua,
        _: &StateRegistry,
        projectile_state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) {
        fsm.time = 0.;
        fsm.linear_velocity = projectile_state.linear_vel;
    }

    fn update(
        &self,
        _: &Lua,
        _: &StateRegistry,
        dt: f32,
        projectile_state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) -> Transition {
        if fsm.time >= self.duration {
            Transition::Done
        } else {
            let lerp_factor =
                self.from + (self.to - self.from) * (fsm.time / self.duration).min(1.0);
            projectile_state.linear_vel = fsm.linear_velocity * lerp_factor;
            fsm.time += dt;
            Transition::None
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LerpPolarLinearSpeed {
    pub from: f32,
    pub to: f32,
    pub duration: f32,
}

impl State for LerpPolarLinearSpeed {
    fn enter(
        &self,
        _: &Lua,
        _: &StateRegistry,
        projectile_state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) {
        fsm.time = 0.;
        fsm.polar_velocity.linear = projectile_state.polar_vel.linear;
    }

    fn update(
        &self,
        _: &Lua,
        _: &StateRegistry,
        dt: f32,
        projectile_state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) -> Transition {
        if fsm.time >= self.duration {
            Transition::Done
        } else {
            let lerp_factor =
                self.from + (self.to - self.from) * (fsm.time / self.duration).min(1.0);
            projectile_state.polar_vel.linear = fsm.polar_velocity.linear * lerp_factor;
            fsm.time += dt;
            Transition::None
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LerpPolarAngularSpeed {
    pub from: f32,
    pub to: f32,
    pub duration: f32,
}

impl State for LerpPolarAngularSpeed {
    fn enter(
        &self,
        _: &Lua,
        _: &StateRegistry,
        projectile_state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) {
        fsm.time = 0.;
        fsm.polar_velocity.angular = projectile_state.polar_vel.angular;
    }

    fn update(
        &self,
        _: &Lua,
        _: &StateRegistry,
        dt: f32,
        projectile_state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) -> Transition {
        if fsm.time >= self.duration {
            Transition::Done
        } else {
            let lerp_factor =
                self.from + (self.to - self.from) * (fsm.time / self.duration).min(1.0);
            projectile_state.polar_vel.angular = fsm.polar_velocity.angular * lerp_factor;
            fsm.time += dt;
            Transition::None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sequence {
    pub sequential_states: Arc<[StateIndex]>,
}

#[derive(Clone)]
struct SequenceState {
    fsm: StateMachine,
    i: usize,
}

impl State for Sequence {
    fn enter(
        &self,
        lua: &Lua,
        machine: &StateRegistry,
        projectile_state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) {
        fsm.extra = self.sequential_states.first().map(|&index| {
            let mut sub_fsm = StateMachine::new(index);
            machine.enter(lua, projectile_state, &mut sub_fsm);
            smallbox!(SequenceState { fsm: sub_fsm, i: 0 })
        });
    }

    fn update(
        &self,
        lua: &Lua,
        machine: &StateRegistry,
        dt: f32,
        projectile_state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) -> Transition {
        let extra_mut = match fsm.extra.as_mut() {
            Some(ex) => ex,
            None => return Transition::Done,
        };

        let seq_state = extra_mut.downcast_mut::<SequenceState>().unwrap();
        loop {
            if machine.update(lua, dt, projectile_state, &mut seq_state.fsm) {
                seq_state.i += 1;
                if let Some(&new_index) = self.sequential_states.get(seq_state.i) {
                    seq_state.fsm.index = new_index;
                    machine.enter(lua, projectile_state, &mut seq_state.fsm);
                    continue;
                } else {
                    fsm.extra = None;
                    return Transition::Done;
                }
            } else {
                return Transition::None;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Parallel {
    pub parallel_states: Arc<[StateIndex]>,
}

impl State for Parallel {
    fn enter(
        &self,
        lua: &Lua,
        machine: &StateRegistry,
        state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) {
        let sub_fsms = self
            .parallel_states
            .iter()
            .copied()
            .map(|index| {
                let mut sub_fsm = StateMachine::new(index);
                machine.enter(lua, state, &mut sub_fsm);
                sub_fsm
            })
            .collect::<Vec<StateMachine>>();

        fsm.extra = Some(smallbox!(sub_fsms));
    }

    fn update(
        &self,
        lua: &Lua,
        machine: &StateRegistry,
        dt: f32,
        projectile_state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) -> Transition {
        let extra_mut = fsm.extra.as_mut().unwrap();
        let sub_fsms = extra_mut.downcast_mut::<Vec<StateMachine>>().unwrap();
        sub_fsms.drain_filter(|sub_fsm| machine.update(lua, dt, projectile_state, sub_fsm));

        if sub_fsms.is_empty() {
            Transition::Done
        } else {
            Transition::None
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Wait {
    pub t: f32,
}

impl State for Wait {
    fn enter(
        &self,
        _: &Lua,
        _machine: &StateRegistry,
        _state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) {
        fsm.time = 0.;
    }

    fn update(
        &self,
        _: &Lua,
        _machine: &StateRegistry,
        dt: f32,
        _projectile_state: &mut ProjectileState,
        fsm: &mut StateMachine,
    ) -> Transition {
        if fsm.time >= self.t {
            Transition::Done
        } else {
            fsm.time += dt;
            Transition::None
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Kill;

impl State for Kill {
    fn enter(
        &self,
        _: &Lua,
        _machine: &StateRegistry,
        state: &mut ProjectileState,
        _fsm: &mut StateMachine,
    ) {
        state.kill = true;
    }

    fn update(
        &self,
        _: &Lua,
        _machine: &StateRegistry,
        _dt: f32,
        _projectile_state: &mut ProjectileState,
        _fsm: &mut StateMachine,
    ) -> Transition {
        Transition::Done
    }
}

/// Swaps to a specific animation and then waits for it to end (if it ever does).
#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub sprite: Option<ProjectileSprite>,
    pub wait: bool,
    pub next: Option<StateIndex>,
}

impl State for Sprite {
    fn enter(
        &self,
        _lua: &Lua,
        _machine: &StateRegistry,
        state: &mut ProjectileState,
        _fsm: &mut StateMachine,
    ) {
        state.sprite = self.sprite;
    }

    fn update(
        &self,
        _lua: &Lua,
        _machine: &StateRegistry,
        _dt: f32,
        projectile_state: &mut ProjectileState,
        _fsm: &mut StateMachine,
    ) -> Transition {
        if self.wait
            && matches!(projectile_state.sprite.as_ref(), Some(sprite) if !sprite.tag.is_paused)
        {
            Transition::None
        } else {
            match self.next {
                Some(state) => Transition::To(state),
                None => Transition::Done,
            }
        }
    }
}
