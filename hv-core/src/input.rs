//! Abstractions for handling input and creating input key/button/etc. bindings.
//!
//! Heavily based on the `ggez-goodies` crate's `input.rs` module; please see the source for the
//! license notification.
//!
//! An abstract input state object that gets fed user events and updates itself based on a set of
//! key bindings.
//!
//! The goals are:
//!
//! * Have a layer of abstract key bindings rather than looking at concrete event types
//! * Use this to be able to abstract away differences between keyboards, joysticks and game
//!   controllers (rather based on Unity3D),
//! * Do some tweening of input axes and stuff just for fun.
//! * Present event- or state-based API so you can do whichever you want.

/*
 * MIT License
 *
 * Copyright (c) 2016-2018 the ggez developers
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

// TODO: Handle mice, game pads, joysticks

use mlua::prelude::*;
use nalgebra::{Point2, Vector2};
use serde::*;
use std::{collections::HashMap, hash::Hash};

// Okay, but how does it actually work?
// Basically we have to bind input events to buttons and axes.
// Input events can be keys, mouse buttons/motion, or eventually
// joystick/controller inputs.  Mouse delta can be mapped to axes too.
//
// https://docs.unity3d.com/Manual/ConventionalGameInput.html has useful
// descriptions of the exact behavior of axes.
//
// So to think about this more clearly, here are the default bindings:
//
// W, ↑: +Y axis
// A, ←: -X axis
// S, ↓: -Y axis
// D, →: +X axis
// Enter, z, LMB: Button 1
// Shift, x, MMB: Button 2
// Ctrl,  c, RMB: Button 3
//
// Easy way?  Hash map of event -> axis/button bindings.

/// Supported key codes.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq, strum::EnumString, Serialize, Deserialize)]
#[strum(ascii_case_insensitive)]
#[repr(u32)]
pub enum KeyCode {
    Space,
    Apostrophe,
    Comma,
    Minus,
    Period,
    Slash,
    Key0,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Semicolon,
    Equal,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    LeftBracket,
    Backslash,
    RightBracket,
    GraveAccent,
    World1,
    World2,
    Escape,
    Enter,
    Tab,
    Backspace,
    Insert,
    Delete,
    Right,
    Left,
    Down,
    Up,
    PageUp,
    PageDown,
    Home,
    End,
    CapsLock,
    ScrollLock,
    NumLock,
    PrintScreen,
    Pause,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F25,
    Kp0,
    Kp1,
    Kp2,
    Kp3,
    Kp4,
    Kp5,
    Kp6,
    Kp7,
    Kp8,
    Kp9,
    KpDecimal,
    KpDivide,
    KpMultiply,
    KpSubtract,
    KpAdd,
    KpEnter,
    KpEqual,
    LeftShift,
    LeftControl,
    LeftAlt,
    LeftSuper,
    RightShift,
    RightControl,
    RightAlt,
    RightSuper,
    Menu,
    Unknown,
}

impl From<miniquad::KeyCode> for KeyCode {
    fn from(kc: miniquad::KeyCode) -> Self {
        use miniquad::KeyCode as MqKc;
        use KeyCode as HvKc;

        match kc {
            MqKc::Space => HvKc::Space,
            MqKc::Apostrophe => HvKc::Apostrophe,
            MqKc::Comma => HvKc::Comma,
            MqKc::Minus => HvKc::Minus,
            MqKc::Period => HvKc::Period,
            MqKc::Slash => HvKc::Slash,
            MqKc::Key0 => HvKc::Key0,
            MqKc::Key1 => HvKc::Key1,
            MqKc::Key2 => HvKc::Key2,
            MqKc::Key3 => HvKc::Key3,
            MqKc::Key4 => HvKc::Key4,
            MqKc::Key5 => HvKc::Key5,
            MqKc::Key6 => HvKc::Key6,
            MqKc::Key7 => HvKc::Key7,
            MqKc::Key8 => HvKc::Key8,
            MqKc::Key9 => HvKc::Key9,
            MqKc::Semicolon => HvKc::Semicolon,
            MqKc::Equal => HvKc::Equal,
            MqKc::A => HvKc::A,
            MqKc::B => HvKc::B,
            MqKc::C => HvKc::C,
            MqKc::D => HvKc::D,
            MqKc::E => HvKc::E,
            MqKc::F => HvKc::F,
            MqKc::G => HvKc::G,
            MqKc::H => HvKc::H,
            MqKc::I => HvKc::I,
            MqKc::J => HvKc::J,
            MqKc::K => HvKc::K,
            MqKc::L => HvKc::L,
            MqKc::M => HvKc::M,
            MqKc::N => HvKc::N,
            MqKc::O => HvKc::O,
            MqKc::P => HvKc::P,
            MqKc::Q => HvKc::Q,
            MqKc::R => HvKc::R,
            MqKc::S => HvKc::S,
            MqKc::T => HvKc::T,
            MqKc::U => HvKc::U,
            MqKc::V => HvKc::V,
            MqKc::W => HvKc::W,
            MqKc::X => HvKc::X,
            MqKc::Y => HvKc::Y,
            MqKc::Z => HvKc::Z,
            MqKc::LeftBracket => HvKc::LeftBracket,
            MqKc::Backslash => HvKc::Backslash,
            MqKc::RightBracket => HvKc::RightBracket,
            MqKc::GraveAccent => HvKc::GraveAccent,
            MqKc::World1 => HvKc::World1,
            MqKc::World2 => HvKc::World2,
            MqKc::Escape => HvKc::Escape,
            MqKc::Enter => HvKc::Enter,
            MqKc::Tab => HvKc::Tab,
            MqKc::Backspace => HvKc::Backspace,
            MqKc::Insert => HvKc::Insert,
            MqKc::Delete => HvKc::Delete,
            MqKc::Right => HvKc::Right,
            MqKc::Left => HvKc::Left,
            MqKc::Down => HvKc::Down,
            MqKc::Up => HvKc::Up,
            MqKc::PageUp => HvKc::PageUp,
            MqKc::PageDown => HvKc::PageDown,
            MqKc::Home => HvKc::Home,
            MqKc::End => HvKc::End,
            MqKc::CapsLock => HvKc::CapsLock,
            MqKc::ScrollLock => HvKc::ScrollLock,
            MqKc::NumLock => HvKc::NumLock,
            MqKc::PrintScreen => HvKc::PrintScreen,
            MqKc::Pause => HvKc::Pause,
            MqKc::F1 => HvKc::F1,
            MqKc::F2 => HvKc::F2,
            MqKc::F3 => HvKc::F3,
            MqKc::F4 => HvKc::F4,
            MqKc::F5 => HvKc::F5,
            MqKc::F6 => HvKc::F6,
            MqKc::F7 => HvKc::F7,
            MqKc::F8 => HvKc::F8,
            MqKc::F9 => HvKc::F9,
            MqKc::F10 => HvKc::F10,
            MqKc::F11 => HvKc::F11,
            MqKc::F12 => HvKc::F12,
            MqKc::F13 => HvKc::F13,
            MqKc::F14 => HvKc::F14,
            MqKc::F15 => HvKc::F15,
            MqKc::F16 => HvKc::F16,
            MqKc::F17 => HvKc::F17,
            MqKc::F18 => HvKc::F18,
            MqKc::F19 => HvKc::F19,
            MqKc::F20 => HvKc::F20,
            MqKc::F21 => HvKc::F21,
            MqKc::F22 => HvKc::F22,
            MqKc::F23 => HvKc::F23,
            MqKc::F24 => HvKc::F24,
            MqKc::F25 => HvKc::F25,
            MqKc::Kp0 => HvKc::Kp0,
            MqKc::Kp1 => HvKc::Kp1,
            MqKc::Kp2 => HvKc::Kp2,
            MqKc::Kp3 => HvKc::Kp3,
            MqKc::Kp4 => HvKc::Kp4,
            MqKc::Kp5 => HvKc::Kp5,
            MqKc::Kp6 => HvKc::Kp6,
            MqKc::Kp7 => HvKc::Kp7,
            MqKc::Kp8 => HvKc::Kp8,
            MqKc::Kp9 => HvKc::Kp9,
            MqKc::KpDecimal => HvKc::KpDecimal,
            MqKc::KpDivide => HvKc::KpDivide,
            MqKc::KpMultiply => HvKc::KpMultiply,
            MqKc::KpSubtract => HvKc::KpSubtract,
            MqKc::KpAdd => HvKc::KpAdd,
            MqKc::KpEnter => HvKc::KpEnter,
            MqKc::KpEqual => HvKc::KpEqual,
            MqKc::LeftShift => HvKc::LeftShift,
            MqKc::LeftControl => HvKc::LeftControl,
            MqKc::LeftAlt => HvKc::LeftAlt,
            MqKc::LeftSuper => HvKc::LeftSuper,
            MqKc::RightShift => HvKc::RightShift,
            MqKc::RightControl => HvKc::RightControl,
            MqKc::RightAlt => HvKc::RightAlt,
            MqKc::RightSuper => HvKc::RightSuper,
            MqKc::Menu => HvKc::Menu,
            MqKc::Unknown => HvKc::Unknown,
        }
    }
}

/// Key modifiers which could be active when a key is pressed.
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct KeyMods {
    /// The left/right "shift" keys.
    pub shift: bool,
    /// The "control" key.
    pub ctrl: bool,
    /// The "alt" key.
    pub alt: bool,
    /// The "command" or "clover"/Apple key on a mac.
    pub logo: bool,
}

impl From<miniquad::KeyMods> for KeyMods {
    fn from(km: miniquad::KeyMods) -> Self {
        Self {
            shift: km.shift,
            ctrl: km.ctrl,
            alt: km.alt,
            logo: km.logo,
        }
    }
}

/// Supported mouse buttons.
#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Unknown,
}

impl From<miniquad::MouseButton> for MouseButton {
    fn from(mq: miniquad::MouseButton) -> Self {
        use miniquad::MouseButton as MqMb;
        use MouseButton as HvMb;

        match mq {
            MqMb::Left => HvMb::Left,
            MqMb::Right => HvMb::Right,
            MqMb::Middle => HvMb::Middle,
            MqMb::Unknown => HvMb::Unknown,
        }
    }
}

/// Supported gamepad buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum GamepadButton {
    South,
    East,
    North,
    West,
    C,
    Z,
    LeftTrigger,
    LeftTrigger2,
    RightTrigger,
    RightTrigger2,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    Unknown,
}

impl From<gilrs::Button> for GamepadButton {
    fn from(gs: gilrs::Button) -> Self {
        use {gilrs::Button as Gb, GamepadButton::*};
        match gs {
            Gb::South => South,
            Gb::East => East,
            Gb::North => North,
            Gb::West => West,
            Gb::C => C,
            Gb::Z => Z,
            Gb::LeftTrigger => LeftTrigger,
            Gb::LeftTrigger2 => LeftTrigger2,
            Gb::RightTrigger => RightTrigger,
            Gb::RightTrigger2 => RightTrigger2,
            Gb::Select => Select,
            Gb::Start => Start,
            Gb::Mode => Mode,
            Gb::LeftThumb => LeftThumb,
            Gb::RightThumb => RightThumb,
            Gb::DPadUp => DPadUp,
            Gb::DPadDown => DPadDown,
            Gb::DPadLeft => DPadLeft,
            Gb::DPadRight => DPadRight,
            Gb::Unknown => Unknown,
        }
    }
}

/// Supported gamepad axes. The DPads of a gamepad can also be read as axes with this input module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum GamepadAxis {
    LeftStickX,
    LeftStickY,
    LeftZ,
    RightStickX,
    RightStickY,
    RightZ,
    DPadX,
    DPadY,
    Unknown,
}

impl From<gilrs::Axis> for GamepadAxis {
    fn from(axis: gilrs::Axis) -> Self {
        use {gilrs::Axis as Ga, GamepadAxis::*};
        match axis {
            Ga::LeftStickX => LeftStickX,
            Ga::LeftStickY => LeftStickY,
            Ga::LeftZ => LeftZ,
            Ga::RightStickX => RightStickX,
            Ga::RightStickY => RightStickY,
            Ga::RightZ => RightZ,
            Ga::DPadX => DPadX,
            Ga::DPadY => DPadY,
            Ga::Unknown => Unknown,
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Copy, Clone)]
enum InputType {
    Key(KeyCode),
    GamepadButton(GamepadButton),
    GamepadAxis(GamepadAxis),
    MouseButton(MouseButton),
}

/// An `InputEffect` represents a single input event acting on a parameterizable set of axes and
/// buttons.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputEffect<Axes, Buttons>
where
    Axes: Eq + Hash + Clone,
    Buttons: Eq + Hash + Clone,
{
    /// An event setting an axis's position.
    Axis(Axes, f32),
    /// An event indicating a button was pressed or released, along with a possible associated point
    /// (if it's a mouse click or such.)
    Button(Buttons, Option<Point2<f32>>),
    /// An event indicating the mouse was moved, and where it was moved to.
    Cursor(Point2<f32>),
}

impl<Axes, Buttons> InputEffect<Axes, Buttons>
where
    Axes: Eq + Hash + Clone,
    Buttons: Eq + Hash + Clone,
{
    /// Inject a mouse position into this effect.
    pub fn with_mouse_position(self, point: Point2<f32>) -> Self {
        match self {
            Self::Button(button, _) => Self::Button(button, Some(point)),
            _ => self,
        }
    }

    /// Inject a gamepad axis position into this effect.
    pub fn with_axis_position(self, position: f32) -> Self {
        match self {
            Self::Axis(axis, factor) => Self::Axis(axis, position * factor),
            _ => self,
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct CursorState {
    // Where the cursor currently is.
    position: Point2<f32>,
    // Where the cursor was last frame.
    last_position: Point2<f32>,
    // The difference between the current position and the position last update.
    delta: Vector2<f32>,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            position: Point2::origin(),
            last_position: Point2::origin(),
            delta: Vector2::zeros(),
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct AxisState {
    // Where the axis currently is, in [-1, 1]
    position: f32,
    // Where the axis is moving towards.  Possible values are -1, 0, +1 (or a continuous range for
    // analog devices I guess)
    direction: f32,
    // Speed in units per second that the axis moves towards the target value.
    acceleration: f32,
    // Speed in units per second that the axis will fall back toward 0 if the input stops.
    gravity: f32,
}

impl Default for AxisState {
    fn default() -> Self {
        AxisState {
            position: 0.0,
            direction: 0.0,
            acceleration: 16.0,
            gravity: 12.0,
        }
    }
}

#[derive(Debug, Copy, Clone, Default)]
struct ButtonState {
    pressed: bool,
    pressed_last_frame: bool,
    event_location: Option<Point2<f32>>,
}

/// A struct that contains a mapping from physical input events (currently just `KeyCode`s) to
/// whatever your logical Axis/Button types are.
pub struct InputBinding<Axes, Buttons>
where
    Axes: Hash + Eq + Clone,
    Buttons: Hash + Eq + Clone,
{
    // Once EnumSet is stable it should be used for these instead of BTreeMap. ♥? Binding of keys to
    // input values.
    bindings: HashMap<InputType, InputEffect<Axes, Buttons>>,
}

impl<Axes, Buttons> Default for InputBinding<Axes, Buttons>
where
    Axes: Hash + Eq + Clone,
    Buttons: Hash + Eq + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Axes, Buttons> InputBinding<Axes, Buttons>
where
    Axes: Hash + Eq + Clone,
    Buttons: Hash + Eq + Clone,
{
    /// Create an empty set of input bindings.
    pub fn new() -> Self {
        InputBinding {
            bindings: HashMap::new(),
        }
    }

    /// Adds a key binding connecting the given keycode to the given logical axis.
    pub fn bind_key_to_axis(mut self, keycode: KeyCode, axis: Axes, position: f32) -> Self {
        self.bindings
            .insert(InputType::Key(keycode), InputEffect::Axis(axis, position));
        self
    }

    /// Adds a key binding connecting the given keycode to the given logical button.
    pub fn bind_key_to_button(mut self, keycode: KeyCode, button: Buttons) -> Self {
        self.bindings
            .insert(InputType::Key(keycode), InputEffect::Button(button, None));
        self
    }

    /// Adds a gamepad button binding connecting the given gamepad button to the given logical
    /// button.
    pub fn bind_gamepad_button_to_button(
        mut self,
        gamepad_button: GamepadButton,
        button: Buttons,
    ) -> Self {
        self.bindings.insert(
            InputType::GamepadButton(gamepad_button),
            InputEffect::Button(button, None),
        );
        self
    }

    /// Adds a gamepad axis binding connecting the given gamepad axis to the given logical axis.
    pub fn bind_gamepad_axis_to_axis(mut self, gamepad_axis: GamepadAxis, axis: Axes) -> Self {
        self.bindings.insert(
            InputType::GamepadAxis(gamepad_axis),
            InputEffect::Axis(axis, 1.0),
        );
        self
    }

    /// Adds a mouse button binding connecting the given mouse button to the given logical button.
    pub fn bind_mouse_to_button(mut self, mouse_button: MouseButton, button: Buttons) -> Self {
        self.bindings.insert(
            InputType::MouseButton(mouse_button),
            InputEffect::Button(button, None),
        );
        self
    }

    /// Takes an physical input type and turns it into a logical input type (keycode ->
    /// axis/button).
    pub fn resolve_keycode(&self, keycode: KeyCode) -> Option<InputEffect<Axes, Buttons>> {
        self.bindings.get(&InputType::Key(keycode)).cloned()
    }

    /// Convert a physical gamepad input into a logical input.
    pub fn resolve_gamepad_button(
        &self,
        button: GamepadButton,
    ) -> Option<InputEffect<Axes, Buttons>> {
        self.bindings
            .get(&InputType::GamepadButton(button))
            .cloned()
    }

    /// Convert a physical mouse button input into a logical input.
    pub fn resolve_mouse_button(
        &self,
        mouse_button: MouseButton,
        point: Point2<f32>,
    ) -> Option<InputEffect<Axes, Buttons>> {
        self.bindings
            .get(&InputType::MouseButton(mouse_button))
            .cloned()
            .map(|eff| eff.with_mouse_position(point))
    }

    /// Convert a physical gamepad axis input into a logical input.
    pub fn resolve_gamepad_axis(
        &self,
        axis: GamepadAxis,
        position: f32,
    ) -> Option<InputEffect<Axes, Buttons>> {
        self.bindings
            .get(&InputType::GamepadAxis(axis))
            .cloned()
            .map(|eff| eff.with_axis_position(position))
    }
}

/// Represents an input state for a given set of logical axes and buttons.
#[derive(Debug)]
pub struct InputState<Axes, Buttons>
where
    Axes: Hash + Eq + Clone,
    Buttons: Hash + Eq + Clone,
{
    // Input state for axes
    axes: HashMap<Axes, AxisState>,
    // Input states for buttons
    buttons: HashMap<Buttons, ButtonState>,
    // Input state for the mouse cursor
    mouse: CursorState,
}

impl<Axes, Buttons> Default for InputState<Axes, Buttons>
where
    Axes: Eq + Hash + Clone,
    Buttons: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Axes, Buttons> InputState<Axes, Buttons>
where
    Axes: Eq + Hash + Clone,
    Buttons: Eq + Hash + Clone,
{
    /// Create a fresh [`InputState`].
    pub fn new() -> Self {
        InputState {
            axes: HashMap::new(),
            buttons: HashMap::new(),
            mouse: CursorState::default(),
        }
    }

    /// Updates the logical input state based on the actual physical input state.  Should be called
    /// in your update() handler. So, it will do things like move the axes and so on.
    pub fn update(&mut self, dt: f32) {
        for (_axis, axis_status) in self.axes.iter_mut() {
            if axis_status.direction != 0.0 {
                // Accelerate the axis towards the input'ed direction.
                let vel = axis_status.acceleration * dt;
                let pending_position = axis_status.position
                    + if axis_status.direction > 0.0 {
                        vel
                    } else {
                        -vel
                    };
                axis_status.position = if pending_position > 1.0 {
                    1.0
                } else if pending_position < -1.0 {
                    -1.0
                } else {
                    pending_position
                }
            } else {
                // Gravitate back towards 0.
                let abs_dx = f32::min(axis_status.gravity * dt, f32::abs(axis_status.position));
                let dx = if axis_status.position > 0.0 {
                    -abs_dx
                } else {
                    abs_dx
                };
                axis_status.position += dx;
            }
        }

        for (_button, button_status) in self.buttons.iter_mut() {
            button_status.pressed_last_frame = button_status.pressed;
        }

        self.mouse.delta = self.mouse.position - self.mouse.last_position;
        self.mouse.last_position = self.mouse.position;
    }

    /// This method should get called by your key_down_event handler.
    pub fn update_button_down(&mut self, button: Buttons) {
        self.update_effect(InputEffect::Button(button, None), true);
    }

    /// This method should get called by your key_up_event handler.
    pub fn update_button_up(&mut self, button: Buttons) {
        self.update_effect(InputEffect::Button(button, None), false);
    }

    /// This method should get called by your gamepad_axis_changed_event handler, or by your
    /// key_down_event handler if you're binding keypresses to logical axes.
    pub fn update_axis_start(&mut self, axis: Axes, position: f32) {
        self.update_effect(InputEffect::Axis(axis, position), true);
    }

    /// This method will probably not usually be used; however, if you're connecting logical axes to
    /// physical button or key presses, then you can call this in your key_up_event handler for the
    /// corresponding button/key releases.
    pub fn update_axis_stop(&mut self, axis: Axes, position: f32) {
        self.update_effect(InputEffect::Axis(axis, position), false);
    }

    /// This method should be called by your mouse_motion_event handler.
    pub fn update_mouse_position(&mut self, position: Point2<f32>) {
        self.update_effect(InputEffect::Cursor(position), false);
    }

    /// Takes an InputEffect and actually applies it.
    pub fn update_effect(&mut self, effect: InputEffect<Axes, Buttons>, started: bool) {
        match effect {
            InputEffect::Axis(axis, position) => {
                let f = || AxisState::default();
                let axis_status = self.axes.entry(axis).or_insert_with(f);
                if started {
                    axis_status.direction = position;
                } else if (position.is_sign_positive() && axis_status.direction > 0.0)
                    || (position.is_sign_negative() && axis_status.direction < 0.0)
                {
                    axis_status.direction = 0.0;
                }
            }
            InputEffect::Button(button, point) => {
                let button_status = self.buttons.entry(button).or_default();
                button_status.pressed = started;
                button_status.event_location = point;
            }
            InputEffect::Cursor(position) => {
                self.mouse.position = position;
            }
        }
    }

    /// Get the position of a logical axis.
    pub fn get_axis(&self, axis: Axes) -> f32 {
        let d = AxisState::default();
        let axis_status = self.axes.get(&axis).unwrap_or(&d);
        axis_status.position
    }

    /// Get the *actual* position of a logical axis. We actually smooth axes a bit; you usually
    /// don't want this, but this method will return the actual exact position value of the axis.
    pub fn get_axis_raw(&self, axis: Axes) -> f32 {
        let d = AxisState::default();
        let axis_status = self.axes.get(&axis).unwrap_or(&d);
        axis_status.direction
    }

    fn get_button(&self, button: Buttons) -> ButtonState {
        let d = ButtonState::default();
        let button_status = self.buttons.get(&button).unwrap_or(&d);
        *button_status
    }

    /// Check if a logical button is down.
    pub fn get_button_down(&self, button: Buttons) -> bool {
        self.get_button(button).pressed
    }

    /// Check if a logical button is up.
    pub fn get_button_up(&self, button: Buttons) -> bool {
        !self.get_button(button).pressed
    }

    /// Returns whether or not the button was pressed this frame, only returning true if the press
    /// happened this frame.
    ///
    /// Basically, `get_button_down()` and `get_button_up()` are level triggers, this and
    /// `get_button_released()` are edge triggered.
    pub fn get_button_pressed(&self, button: Buttons) -> bool {
        let b = self.get_button(button);
        b.pressed && !b.pressed_last_frame
    }

    /// Check whether or not a button was released on this frame.
    pub fn get_button_released(&self, button: Buttons) -> bool {
        let b = self.get_button(button);
        !b.pressed && b.pressed_last_frame
    }

    /// Get the location of a button event, if it has one. Generally speaking a button event will
    /// only have a location if it comes from a mouse click, in which case the location will be the
    /// position that the mouse clicked.
    pub fn get_button_event_location(&self, button: Buttons) -> Option<Point2<f32>> {
        let b = self.get_button(button);
        b.event_location
    }

    /// Get the current mouse position.
    pub fn mouse_position(&self) -> Point2<f32> {
        self.mouse.position
    }

    /// Get the change in the mouse position for this frame with respect to the previous frame.
    pub fn mouse_delta(&self) -> Vector2<f32> {
        self.mouse.delta
    }

    /// Reset the input state, all axes at zero, all buttons unpresseed, all positions and deltas
    /// zeroed out.
    pub fn reset_input_state(&mut self) {
        for (_axis, axis_status) in self.axes.iter_mut() {
            axis_status.position = 0.0;
            axis_status.direction = 0.0;
        }

        for (_button, button_status) in self.buttons.iter_mut() {
            button_status.pressed = false;
            button_status.pressed_last_frame = false;
        }

        self.mouse.position = Point2::origin();
        self.mouse.last_position = Point2::origin();
        self.mouse.delta = Vector2::zeros();
    }
}

/// Supported cursor icons.
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
#[allow(missing_docs)]
pub enum CursorIcon {
    Default,
    Help,
    Pointer,
    Wait,
    Crosshair,
    Text,
    Move,
    NotAllowed,
    EWResize,
    NSResize,
    NESWResize,
    NWSEResize,
}

impl From<mq::CursorIcon> for CursorIcon {
    fn from(mq: mq::CursorIcon) -> Self {
        match mq {
            mq::CursorIcon::Default => Self::Default,
            mq::CursorIcon::Help => Self::Help,
            mq::CursorIcon::Pointer => Self::Pointer,
            mq::CursorIcon::Wait => Self::Wait,
            mq::CursorIcon::Crosshair => Self::Crosshair,
            mq::CursorIcon::Text => Self::Text,
            mq::CursorIcon::Move => Self::Move,
            mq::CursorIcon::NotAllowed => Self::NotAllowed,
            mq::CursorIcon::EWResize => Self::EWResize,
            mq::CursorIcon::NSResize => Self::NSResize,
            mq::CursorIcon::NESWResize => Self::NESWResize,
            mq::CursorIcon::NWSEResize => Self::NWSEResize,
        }
    }
}

impl From<CursorIcon> for mq::CursorIcon {
    fn from(hv: CursorIcon) -> Self {
        match hv {
            CursorIcon::Default => Self::Default,
            CursorIcon::Help => Self::Help,
            CursorIcon::Pointer => Self::Pointer,
            CursorIcon::Wait => Self::Wait,
            CursorIcon::Crosshair => Self::Crosshair,
            CursorIcon::Text => Self::Text,
            CursorIcon::Move => Self::Move,
            CursorIcon::NotAllowed => Self::NotAllowed,
            CursorIcon::EWResize => Self::EWResize,
            CursorIcon::NSResize => Self::NSResize,
            CursorIcon::NESWResize => Self::NESWResize,
            CursorIcon::NWSEResize => Self::NWSEResize,
        }
    }
}

impl<Axes, Buttons> LuaUserData for InputState<Axes, Buttons>
where
    Axes: for<'lua> FromLua<'lua> + for<'lua> ToLua<'lua> + Eq + Hash + Clone,
    Buttons: for<'lua> FromLua<'lua> + for<'lua> ToLua<'lua> + Eq + Hash + Clone,
{
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get_button_down", |_, this, button| {
            Ok(this.get_button_down(button))
        });

        methods.add_method("get_button_up", |_, this, button| {
            Ok(this.get_button_up(button))
        });

        methods.add_method("get_button_pressed", |_, this, button| {
            Ok(this.get_button_pressed(button))
        });

        methods.add_method("get_button_released", |_, this, button| {
            Ok(this.get_button_released(button))
        });

        methods.add_method("get_axis", |_, this, axis| Ok(this.get_axis(axis)));

        methods.add_method("get_axis_raw", |_, this, axis| Ok(this.get_axis_raw(axis)));

        methods.add_method("mouse_position", |_, this, ()| {
            let pt = this.mouse_position();
            Ok((pt.x, pt.y))
        });

        methods.add_method("mouse_delta", |_, this, ()| {
            let v = this.mouse_delta();
            Ok((v.x, v.y))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
    enum Buttons {
        A,
        B,
        Select,
        Start,
    }

    #[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
    enum Axes {
        Horz,
        Vert,
    }

    fn make_input_binding() -> InputBinding<Axes, Buttons> {
        InputBinding::<Axes, Buttons>::new()
            .bind_key_to_button(KeyCode::Z, Buttons::A)
            .bind_key_to_button(KeyCode::X, Buttons::B)
            .bind_key_to_button(KeyCode::Enter, Buttons::Start)
            .bind_key_to_button(KeyCode::RightShift, Buttons::Select)
            .bind_key_to_button(KeyCode::LeftShift, Buttons::Select)
            .bind_key_to_axis(KeyCode::Up, Axes::Vert, 1.)
            .bind_key_to_axis(KeyCode::Down, Axes::Vert, -1.)
            .bind_key_to_axis(KeyCode::Left, Axes::Horz, -1.)
            .bind_key_to_axis(KeyCode::Right, Axes::Horz, 1.)
    }

    #[test]
    fn test_input_bindings() {
        let ib = make_input_binding();
        assert_eq!(
            ib.resolve_keycode(KeyCode::Z),
            Some(InputEffect::Button(Buttons::A, None))
        );
        assert_eq!(
            ib.resolve_keycode(KeyCode::X),
            Some(InputEffect::Button(Buttons::B, None))
        );
        assert_eq!(
            ib.resolve_keycode(KeyCode::Enter),
            Some(InputEffect::Button(Buttons::Start, None))
        );
        assert_eq!(
            ib.resolve_keycode(KeyCode::RightShift),
            Some(InputEffect::Button(Buttons::Select, None))
        );
        assert_eq!(
            ib.resolve_keycode(KeyCode::LeftShift),
            Some(InputEffect::Button(Buttons::Select, None))
        );

        assert_eq!(
            ib.resolve_keycode(KeyCode::Up),
            Some(InputEffect::Axis(Axes::Vert, 1.))
        );
        assert_eq!(
            ib.resolve_keycode(KeyCode::Down),
            Some(InputEffect::Axis(Axes::Vert, -1.))
        );
        assert_eq!(
            ib.resolve_keycode(KeyCode::Left),
            Some(InputEffect::Axis(Axes::Horz, -1.))
        );
        assert_eq!(
            ib.resolve_keycode(KeyCode::Right),
            Some(InputEffect::Axis(Axes::Horz, 1.))
        );

        assert_eq!(ib.resolve_keycode(KeyCode::Q), None);
        assert_eq!(ib.resolve_keycode(KeyCode::W), None);
    }

    #[allow(clippy::float_cmp)]
    #[test]
    fn test_input_events() {
        let mut im = InputState::new();
        im.update_button_down(Buttons::A);
        assert!(im.get_button_down(Buttons::A));
        im.update_button_up(Buttons::A);
        assert!(!im.get_button_down(Buttons::A));
        assert!(im.get_button_up(Buttons::A));

        // Push the 'up' button, watch the axis
        // increase to 1.0 but not beyond
        im.update_axis_start(Axes::Vert, 1.);
        assert!(im.get_axis_raw(Axes::Vert) > 0.0);
        while im.get_axis(Axes::Vert) < 0.99 {
            im.update(0.16);
            assert!(im.get_axis(Axes::Vert) >= 0.0);
            assert!(im.get_axis(Axes::Vert) <= 1.0);
        }
        // Release it, watch it wind down
        im.update_axis_stop(Axes::Vert, 1.);
        while im.get_axis(Axes::Vert) > 0.01 {
            im.update(0.16);
            assert!(im.get_axis(Axes::Vert) >= 0.0)
        }

        // Do the same with the 'down' button.
        im.update_axis_start(Axes::Vert, -1.);
        while im.get_axis(Axes::Vert) > -0.99 {
            im.update(0.16);
            assert!(im.get_axis(Axes::Vert) <= 0.0);
            assert!(im.get_axis(Axes::Vert) >= -1.0);
        }

        // Test the transition from 'up' to 'down'
        im.update_axis_start(Axes::Vert, 1.);
        while im.get_axis(Axes::Vert) < 1.0 {
            im.update(0.16);
        }
        im.update_axis_start(Axes::Vert, -1.);
        im.update(0.16);
        assert!(im.get_axis(Axes::Vert) < 1.0);
        im.update_axis_stop(Axes::Vert, 1.);
        assert!(im.get_axis_raw(Axes::Vert) < 0.0);
        im.update_axis_stop(Axes::Vert, -1.);
        assert_eq!(im.get_axis_raw(Axes::Vert), 0.0);
    }

    #[test]
    fn test_button_edge_transitions() {
        let mut im: InputState<Axes, Buttons> = InputState::new();

        // Push a key, confirm it's transitioned.
        assert!(!im.get_button_down(Buttons::A));
        im.update_button_down(Buttons::A);
        assert!(im.get_button_down(Buttons::A));
        assert!(im.get_button_pressed(Buttons::A));
        assert!(!im.get_button_released(Buttons::A));

        // Update, confirm it's still down but
        // wasn't pressed this frame
        im.update(0.1);
        assert!(im.get_button_down(Buttons::A));
        assert!(!im.get_button_pressed(Buttons::A));
        assert!(!im.get_button_released(Buttons::A));

        // Release it
        im.update_button_up(Buttons::A);
        assert!(im.get_button_up(Buttons::A));
        assert!(!im.get_button_pressed(Buttons::A));
        assert!(im.get_button_released(Buttons::A));
        im.update(0.1);
        assert!(im.get_button_up(Buttons::A));
        assert!(!im.get_button_pressed(Buttons::A));
        assert!(!im.get_button_released(Buttons::A));
    }
}
