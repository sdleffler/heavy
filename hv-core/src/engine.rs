//! Event handling, window/graphics context creation and the main game loop.
//!
//! The core type, representing an instance of the Heavy framework with a Lua context,
//! window/graphics context, and more. I can't imagine a case where you'd need more than one of
//! these. Use [`Engine::run`] to start your event loop.

use {
    miniquad as mq,
    send_wrapper::SendWrapper,
    std::{
        any::{Any, TypeId},
        collections::HashMap,
        marker::PhantomData,
        sync::{Arc as StdArc, Mutex, MutexGuard, Weak as StdWeak},
    },
};

use gilrs::Gilrs;

use crate::{
    conf::Conf,
    error::*,
    filesystem::Filesystem,
    input::{CursorIcon, GamepadAxis, GamepadButton, KeyCode, KeyMods, MouseButton},
    mlua::prelude::*,
    shared::{Shared, Weak},
};

/// Currently miniquad's update rate is fixed to 60 frames per second.
pub const MINIQUAD_DT: f32 = 1. / 60.;

/// A [`LuaResource`] can be fetched from the Lua context, similarly to how a regular resource can
/// be fetched from [`Engine::get`]. Most of the time a resource can be fetched from both locations.
pub trait LuaResource: LuaUserData + Send + Sync + 'static {
    /// The registry key for this resource to be stored with. It should not collide with any other
    /// string registry keys, or you'll get panics and dynamic type mismatches.
    const REGISTRY_KEY: &'static str;
}

/// An extension trait implemented on the [`Lua`] context type, allowing for easily registering and
/// retrieving shared resources.
pub trait LuaExt {
    /// Retrieve a resource implementing [`LuaResource`] from its entry in the Lua registry.
    fn get_resource<T: LuaResource>(&self) -> LuaResult<Shared<T>>;

    /// Insert a resource implementing [`LuaResource`] into the Lua registry.
    fn insert_resource<T: LuaResource>(&self, resource: Shared<T>) -> LuaResult<()>;
}

impl LuaExt for Lua {
    #[inline]
    fn get_resource<T: LuaResource>(&self) -> LuaResult<Shared<T>> {
        self.named_registry_value(T::REGISTRY_KEY)
    }

    #[inline]
    fn insert_resource<T: LuaResource>(&self, resource: Shared<T>) -> LuaResult<()> {
        self.set_named_registry_value(T::REGISTRY_KEY, resource)
    }
}

struct EngineInner {
    handler: Mutex<Box<dyn EventHandler>>,
    lua: Mutex<Lua>,
    mq: Mutex<mq::Context>,
    fs: Mutex<Filesystem>,
    gilrs: Mutex<SendWrapper<Gilrs>>,
    resources: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

/// A "weak" shared reference to the [`Engine`].
///
/// It's unlikely that you care about leaking an [`Engine`], but if you do, weak references should
/// be used to break cycles. In addition, they can be initialized as "empty", unlike strong
/// references, and so you can use them when you can't immediately get an [`Engine`] in some
/// context. The [`EngineRefCache`] also exists to help with this, mostly in the context of Rust
/// code which has nothing to interact with but the Lua context.
///
/// Can be upgraded to a strong reference temporarily with [`EngineRef::upgrade`].
#[derive(Clone)]
pub struct EngineRef {
    weak: StdWeak<EngineInner>,
}

/// A "strong" shared reference to the running "engine" and all its resources.
pub struct Engine<'a> {
    _restrictor: PhantomData<&'a ()>,
    inner: StdArc<EngineInner>,
}

impl Engine<'static> {
    /// Create a new engine from its parts.
    ///
    /// ***Normally you will never call this yourself!*** You will almost always want to use
    /// [`Engine::run`] instead!!
    pub fn new(fs: Filesystem, mq: mq::Context, handler: impl EventHandler) -> Result<Self> {
        use mlua::StdLib;
        let lua = Lua::new_with(
            /* /* if using Lua 5.2 or above and *not* 5.1 or LuaJIT: */ StdLib::COROUTINE | */
            StdLib::TABLE | StdLib::STRING | StdLib::MATH,
            LuaOptions::new(),
        )?;

        let this = Engine {
            _restrictor: PhantomData,
            inner: StdArc::new(EngineInner {
                handler: Mutex::new(Box::new(handler)),
                lua: Mutex::new(lua),
                mq: Mutex::new(mq),
                fs: Mutex::new(fs),
                gilrs: Mutex::new(send_wrapper::SendWrapper::new(
                    Gilrs::new().expect("unrecoverable error initializing gilrs"),
                )),
                resources: Default::default(),
            }),
        };

        {
            let lua = this.lua();
            lua.insert_resource(Shared::new(this.downgrade()))?;

            let hv = lua.create_table()?;
            lua.globals().set("hv", hv.clone())?;

            for module in crate::plugins::registered_modules() {
                hv.set(module.name(), module.open(&lua, &this)?)?;
            }

            let chunk = mlua::chunk! {
                function hv.load() end
                function hv.update() end
                function hv.draw() end

                std = require("std")
            };
            lua.load(chunk).exec()?;

            for module in crate::plugins::registered_modules() {
                module.load(&lua, &this)?;
            }
        }

        this.handler().init(&this)?;

        Ok(this)
    }

    /// Construct an [`Engine`], initialize an [`EventHandler`] from it, and start the main event
    /// loop.
    ///
    /// 99.9% of the time this is your entrypoint to Heavy, and the last thing in your `main`
    /// function.
    pub fn run<H: EventHandler>(
        conf: Conf,
        handler_constructor: impl FnOnce(&Engine) -> Result<H> + Send + Sync + 'static,
    ) {
        let handler = LazyHandler::new(handler_constructor);
        mq::start(
            mq::conf::Conf {
                window_title: conf.window_title.clone(),
                window_width: conf.window_width as i32,
                window_height: conf.window_height as i32,
                ..mq::conf::Conf::default()
            },
            move |ctx| mq::UserData::free(Self::new(conf.filesystem, ctx, handler).unwrap()),
        );
    }
}

impl<'a> Engine<'a> {
    /// Get a weak reference from this strong reference.
    pub fn downgrade(&self) -> EngineRef {
        EngineRef {
            weak: StdArc::downgrade(&self.inner),
        }
    }

    /// Acquire a lock on the event handler.
    pub fn handler(&self) -> MutexGuard<Box<dyn EventHandler>> {
        self.inner.handler.try_lock().unwrap()
    }

    /// Acquire a lock on the Lua context.
    pub fn lua(&self) -> MutexGuard<Lua> {
        self.inner.lua.try_lock().unwrap()
    }

    /// Acquire a lock on the miniquad context.
    pub fn mq(&self) -> MutexGuard<mq::Context> {
        self.inner.mq.try_lock().unwrap()
    }

    /// Acquire a lock on the GilRs context.
    pub fn gilrs(&self) -> MutexGuard<SendWrapper<Gilrs>> {
        self.inner.gilrs.try_lock().unwrap()
    }

    /// Acquire a lock on the [`Filesystem`].
    pub fn fs(&self) -> MutexGuard<Filesystem> {
        self.inner.fs.try_lock().unwrap()
    }

    /// Insert a resource already wrapped in a [`Shared`].
    pub fn insert_wrapped<T: Send + Sync + 'static>(&self, resource: Shared<T>) {
        self.inner
            .resources
            .lock()
            .unwrap()
            .insert(TypeId::of::<T>(), Box::new(resource));
    }

    /// Insert a resource into the internal type-to-value resource map.
    pub fn insert<T: Send + Sync + 'static>(&self, resource: T) -> Shared<T> {
        let res = Shared::new(resource);
        self.insert_wrapped(res.clone());
        res
    }

    /// Get a resource from the internal resource map. Will panic if the resource is not present.
    pub fn get<T: Send + Sync + 'static>(&self) -> Shared<T> {
        self.inner.resources.lock().unwrap()[&TypeId::of::<T>()]
            .downcast_ref::<Shared<T>>()
            .unwrap()
            .clone()
    }

    /// Get a resource from the internal resource map if present, returning `None` if it is missing.
    pub fn try_get<T: Send + Sync + 'static>(&self) -> Option<Shared<T>> {
        self.inner
            .resources
            .lock()
            .unwrap()
            .get(&TypeId::of::<T>())
            .map(|entry| entry.downcast_ref::<Shared<T>>().unwrap().clone())
    }

    /// Set whether the mouse is shown on-screen.
    pub fn show_mouse(&self, show: bool) {
        self.mq().show_mouse(show);
    }

    /// Set whether the mouse is "grabbed" (locked in place and hidden.)
    pub fn set_mouse_grabbed(&self, grabbed: bool) {
        self.mq().set_cursor_grab(grabbed);
    }

    /// Set the mouse cursor icon.
    pub fn set_mouse_cursor(&self, icon: CursorIcon) {
        self.mq().set_mouse_cursor(icon.into());
    }
}

impl Default for EngineRef {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineRef {
    /// Create a new empty [`EngineRef`] that points to nothing.
    pub fn new() -> Self {
        Self {
            weak: StdWeak::new(),
        }
    }

    /// Try to upgrade to a strong reference.
    pub fn try_upgrade(&self) -> Option<Engine> {
        self.weak.upgrade().map(|inner| Engine {
            _restrictor: PhantomData,
            inner,
        })
    }

    /// Upgrade to a strong reference and panic if we can't.
    pub fn upgrade(&self) -> Engine {
        self.try_upgrade()
            .expect("failed to upgrade weak reference!")
    }
}

impl LuaUserData for EngineRef {}

impl LuaResource for EngineRef {
    const REGISTRY_KEY: &'static str = "HV_ENGINE";
}

/// A cache for reducing the need to access the Lua registry to get an [`EngineRef`].
pub struct EngineRefCache {
    weak: StdWeak<EngineInner>,
}

impl Default for EngineRefCache {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineRefCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self {
            weak: StdWeak::new(),
        }
    }

    /// If the cache is empty, retrieve a shared reference to the engine from the Lua context.
    /// Otherwise, return the cached reference.
    pub fn get<'lua>(&mut self, lua: &'lua Lua) -> Engine {
        let inner = match self.weak.upgrade() {
            Some(strong) => strong,
            None => {
                self.weak = lua
                    .get_resource::<EngineRef>()
                    .unwrap()
                    .borrow()
                    .weak
                    .clone();
                self.weak
                    .upgrade()
                    .expect("failed to upgrade weak reference!")
            }
        };

        Engine {
            _restrictor: PhantomData,
            inner,
        }
    }
}

/// A simple cache for holding resources that may or may not be initialized when the cache is
/// created.
#[derive(Debug)]
pub struct WeakResourceCache<T> {
    weak: Weak<T>,
}

impl<T> Clone for WeakResourceCache<T> {
    fn clone(&self) -> Self {
        Self {
            weak: self.weak.clone(),
        }
    }
}

impl<T> Default for WeakResourceCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> WeakResourceCache<T> {
    /// Create a new empty `WeakResourceCache`.
    pub fn new() -> Self {
        Self { weak: Weak::new() }
    }

    /// If the cache's internal [`Weak`] is valid, upgrade it to a strong reference and return it;
    /// otherwise, call the provided `init` closure, create a downgraded copy of the returned strong
    /// value and store it in the cache, and then return the strong value.
    pub fn get<F: FnOnce() -> Result<Shared<T>, E>, E>(&mut self, init: F) -> Result<Shared<T>, E> {
        match self.weak.try_upgrade() {
            Some(resource) => Ok(resource),
            None => {
                let strong = init()?;
                self.weak = strong.downgrade();
                Ok(strong)
            }
        }
    }
}

/// The main event handler interface for Heavy. This is how your main game loop gets driven.
pub trait EventHandler: Send + Sync + 'static {
    /// Called once per frame; use this to run your update code.
    fn update(&mut self, engine: &Engine, dt: f32) -> Result<()>;

    /// Called once per frame; use this to run your rendering/drawing code.
    fn draw(&mut self, engine: &Engine) -> Result<()>;

    /// Called when a key is pressed.
    ///
    /// If a key is held down, then this event may be repeatedly generated. If the event is a
    /// "repeat" from a held-down key (the first type the key is pressed will not be a "repeat")
    /// then `repeat` will be true.
    fn key_down_event(
        &mut self,
        _engine: &Engine,
        keycode: KeyCode,
        keymods: KeyMods,
        repeat: bool,
    ) {
        log::trace!(
            "unhandled key_down_event({:?}, {:?}, {})",
            keycode,
            keymods,
            repeat
        );
    }

    /// Called when a key is released.
    fn key_up_event(&mut self, _engine: &Engine, keycode: KeyCode, keymods: KeyMods) {
        log::trace!("unhandled key_up_event({:?}, {:?})", keycode, keymods);
    }

    /// Called when any of keypresses which would type a character are completed.
    ///
    /// This is useful for text entry.
    fn char_event(&mut self, _engine: &Engine, character: char, keymods: KeyMods, repeat: bool) {
        log::trace!(
            "unhandled char_event({:?}, {:?}, {})",
            character,
            keymods,
            repeat
        );
    }

    /// Called when the mouse is moved.
    ///
    /// Arguments are the delta/change in mouse position, not the new position. Will still be
    /// generated if the mouse is grabbed.
    fn mouse_motion_event(&mut self, _engine: &Engine, x: f32, y: f32) {
        log::trace!("unhandled mouse_motion_event({}, {})", x, y);
    }

    /// Called when the mouse wheel is moved.
    ///
    /// Arguments are the delta/change in mouse wheel position, not the new position. `x` is
    /// horizontal mouse wheel movement, `y` is vertical mouse wheel movement.
    fn mouse_wheel_event(&mut self, _engine: &Engine, x: f32, y: f32) {
        log::trace!("unhandled mouse_wheel_event({}, {})", x, y);
    }

    /// Called when a mouse button is pressed.
    ///
    /// Receives the location at which the mouse was when the button was pressed.
    fn mouse_button_down_event(&mut self, _engine: &Engine, button: MouseButton, x: f32, y: f32) {
        log::trace!(
            "unhandled mouse_button_down_event({:?}, {}, {})",
            button,
            x,
            y
        );
    }

    /// Called when a mouse button is released.
    ///
    /// Receives the location at which the mouse was when the button was released.
    fn mouse_button_up_event(&mut self, _engine: &Engine, button: MouseButton, x: f32, y: f32) {
        log::trace!(
            "unhandled mouse_button_up_event({:?}, {}, {})",
            button,
            x,
            y
        );
    }

    /// Called when a gamepad button is pressed.
    ///
    /// Similarly to [`EventHandler::key_down_event`], if the press is a repeat, the `repeat`
    /// parameter will be `true`.
    fn gamepad_button_down_event(&mut self, _engine: &Engine, button: GamepadButton, repeat: bool) {
        log::trace!(
            "unhandled gamepad_button_down_event({:?}, {})",
            button,
            repeat
        );
    }

    /// Called when a gamepad button is released.
    fn gamepad_button_up_event(&mut self, _engine: &Engine, button: GamepadButton) {
        log::trace!("unhandled gamepad_button_up_event({:?})", button);
    }

    /// Called when a gamepad axis changes in value.
    fn gamepad_axis_changed_event(&mut self, _engine: &Engine, axis: GamepadAxis, position: f32) {
        log::trace!(
            "unhandled gamepad_axis_changed_event({:?}, {})",
            axis,
            position
        );
    }

    /// Called when a gamepad is connected.
    fn gamepad_connected_event(&mut self, _engine: &Engine) {
        log::trace!("unhandled gamepad_connected_event()");
    }

    /// Called when a gamepad is disconnected.
    fn gamepad_disconnected_event(&mut self, _engine: &Engine) {
        log::trace!("unhandled gamepad_disconnected_event()");
    }

    /// Called when the window size changes.
    fn resize_event(&mut self, _engine: &Engine, width: f32, height: f32) {
        log::trace!("unhandled resize_event({}, {})", width, height);
    }

    /// Called after the [`Engine`] is created. Normally you won't need this, as [`Engine::run`]
    /// lets you construct an [`EventHandler`] directly from a strong reference to the engine.
    /// Internally however that call uses a [`LazyHandler`], which runs your constructor closure in
    /// its own `init` method.
    fn init(&mut self, _engine: &Engine) -> Result<()> {
        Ok(())
    }
}

impl mq::EventHandlerFree for Engine<'static> {
    fn update(&mut self) {
        use gilrs::EventType;

        let mut handler = self.handler();

        while let Some(event) = self.gilrs().next_event() {
            log::trace!("gilrs: {:?}", event);

            match event.event {
                EventType::ButtonPressed(button, _) => {
                    handler.gamepad_button_down_event(self, GamepadButton::from(button), false)
                }
                EventType::ButtonRepeated(button, _) => {
                    handler.gamepad_button_down_event(self, GamepadButton::from(button), true)
                }
                EventType::ButtonReleased(button, _) => {
                    handler.gamepad_button_up_event(self, GamepadButton::from(button))
                }
                EventType::AxisChanged(axis, position, _) => {
                    handler.gamepad_axis_changed_event(self, GamepadAxis::from(axis), position)
                }
                EventType::Connected => handler.gamepad_connected_event(self),
                EventType::Disconnected => handler.gamepad_disconnected_event(self),
                ev => {
                    log::trace!("unhandled gamepad event: {:?}", ev);
                }
            }
        }

        handler.update(self, MINIQUAD_DT).unwrap();
    }

    fn draw(&mut self) {
        self.handler().draw(self).unwrap();
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.handler().resize_event(self, width, height);
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.handler().mouse_motion_event(self, x, y);
    }

    fn mouse_wheel_event(&mut self, x: f32, y: f32) {
        self.handler().mouse_wheel_event(self, x, y);
    }

    fn mouse_button_down_event(&mut self, button: mq::MouseButton, x: f32, y: f32) {
        self.handler()
            .mouse_button_down_event(self, MouseButton::from(button), x, y)
    }

    fn mouse_button_up_event(&mut self, button: mq::MouseButton, x: f32, y: f32) {
        self.handler()
            .mouse_button_up_event(self, MouseButton::from(button), x, y);
    }

    fn char_event(&mut self, character: char, keymods: mq::KeyMods, repeat: bool) {
        self.handler()
            .char_event(self, character, KeyMods::from(keymods), repeat);
    }

    fn key_down_event(&mut self, keycode: mq::KeyCode, keymods: mq::KeyMods, repeat: bool) {
        self.handler()
            .key_down_event(self, KeyCode::from(keycode), KeyMods::from(keymods), repeat);
    }

    fn key_up_event(&mut self, keycode: mq::KeyCode, keymods: mq::KeyMods) {
        self.handler()
            .key_up_event(self, KeyCode::from(keycode), KeyMods::from(keymods));
    }

    /// Default implementation emulates mouse clicks
    fn touch_event(&mut self, phase: mq::TouchPhase, _id: u64, x: f32, y: f32) {
        if phase == mq::TouchPhase::Started {
            self.mouse_button_down_event(mq::MouseButton::Left, x, y);
        }

        if phase == mq::TouchPhase::Ended {
            self.mouse_button_up_event(mq::MouseButton::Left, x, y);
        }

        if phase == mq::TouchPhase::Moved {
            self.mouse_motion_event(x, y);
        }
    }

    /// Represents raw hardware mouse motion event
    /// Note that these events are delivered regardless of input focus and not in pixels, but in
    /// hardware units instead. And those units may be different from pixels depending on the target platform
    fn raw_mouse_motion(&mut self, _dx: f32, _dy: f32) {}

    /// This event is sent when the userclicks the window's close button
    /// or application code calls the ctx.request_quit() function. The event
    /// handler callback code can handle this event by calling
    /// ctx.cancel_quit() to cancel the quit.
    /// If the event is ignored, the application will quit as usual.
    fn quit_requested_event(&mut self) {}
}

impl<T: EventHandler> EventHandler for Shared<T> {
    fn init(&mut self, engine: &Engine) -> Result<()> {
        self.borrow_mut().init(engine)
    }

    fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        self.borrow_mut().update(engine, dt)
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        self.borrow_mut().draw(engine)
    }

    fn key_down_event(
        &mut self,
        engine: &Engine,
        keycode: KeyCode,
        keymods: KeyMods,
        repeat: bool,
    ) {
        self.borrow_mut()
            .key_down_event(engine, keycode, keymods, repeat)
    }

    fn key_up_event(&mut self, engine: &Engine, keycode: KeyCode, keymods: KeyMods) {
        self.borrow_mut().key_up_event(engine, keycode, keymods)
    }

    fn char_event(&mut self, engine: &Engine, character: char, keymods: KeyMods, repeat: bool) {
        self.borrow_mut()
            .char_event(engine, character, keymods, repeat)
    }

    fn mouse_motion_event(&mut self, engine: &Engine, x: f32, y: f32) {
        self.borrow_mut().mouse_motion_event(engine, x, y)
    }

    fn mouse_wheel_event(&mut self, engine: &Engine, x: f32, y: f32) {
        self.borrow_mut().mouse_wheel_event(engine, x, y)
    }

    fn mouse_button_down_event(&mut self, engine: &Engine, button: MouseButton, x: f32, y: f32) {
        self.borrow_mut()
            .mouse_button_down_event(engine, button, x, y)
    }

    fn mouse_button_up_event(&mut self, engine: &Engine, button: MouseButton, x: f32, y: f32) {
        self.borrow_mut()
            .mouse_button_up_event(engine, button, x, y)
    }

    fn gamepad_button_down_event(&mut self, engine: &Engine, button: GamepadButton, repeat: bool) {
        self.borrow_mut()
            .gamepad_button_down_event(engine, button, repeat)
    }

    fn gamepad_button_up_event(&mut self, engine: &Engine, button: GamepadButton) {
        self.borrow_mut().gamepad_button_up_event(engine, button)
    }

    fn gamepad_axis_changed_event(&mut self, engine: &Engine, axis: GamepadAxis, position: f32) {
        self.borrow_mut()
            .gamepad_axis_changed_event(engine, axis, position)
    }

    fn resize_event(&mut self, engine: &Engine, width: f32, height: f32) {
        self.borrow_mut().resize_event(engine, width, height)
    }
}

enum LazyHandlerState {
    Uninitialized(Box<dyn FnOnce(&Engine) -> Result<Box<dyn EventHandler>> + Send + Sync>),
    Initialized(Box<dyn EventHandler>),
    Empty,
}

/// An event handler which lazily initializes another handler inside it and then delegates all
/// events to it.
pub struct LazyHandler(LazyHandlerState);

impl LazyHandler {
    /// Create a new lazy handler from a closure used to initialize the internal handler.
    pub fn new<H: EventHandler>(
        f: impl FnOnce(&Engine) -> Result<H> + Send + Sync + 'static,
    ) -> Self {
        Self(LazyHandlerState::Uninitialized(Box::new(move |engine| {
            Ok(Box::new(f(engine)?))
        })))
    }

    fn get_mut(&mut self) -> &mut dyn EventHandler {
        match &mut self.0 {
            LazyHandlerState::Initialized(handler) => &mut **handler,
            _ => unreachable!("attempted to access uninitialized `LazyHandler`"),
        }
    }
}

impl EventHandler for LazyHandler {
    fn init(&mut self, engine: &Engine) -> Result<()> {
        match std::mem::replace(&mut self.0, LazyHandlerState::Empty) {
            LazyHandlerState::Uninitialized(thunk) => {
                let mut handler = thunk(engine)?;
                handler.init(engine)?;
                self.0 = LazyHandlerState::Initialized(handler);
            }
            _ => unreachable!("`LazyHandler` must not be initialized multiple times!"),
        }

        Ok(())
    }

    fn update(&mut self, engine: &Engine, dt: f32) -> Result<()> {
        self.get_mut().update(engine, dt)
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        self.get_mut().draw(engine)
    }

    fn key_down_event(
        &mut self,
        engine: &Engine,
        keycode: KeyCode,
        keymods: KeyMods,
        repeat: bool,
    ) {
        self.get_mut()
            .key_down_event(engine, keycode, keymods, repeat)
    }

    fn key_up_event(&mut self, engine: &Engine, keycode: KeyCode, keymods: KeyMods) {
        self.get_mut().key_up_event(engine, keycode, keymods)
    }

    fn char_event(&mut self, engine: &Engine, character: char, keymods: KeyMods, repeat: bool) {
        self.get_mut()
            .char_event(engine, character, keymods, repeat)
    }

    fn mouse_motion_event(&mut self, engine: &Engine, x: f32, y: f32) {
        self.get_mut().mouse_motion_event(engine, x, y)
    }

    fn mouse_wheel_event(&mut self, engine: &Engine, x: f32, y: f32) {
        self.get_mut().mouse_wheel_event(engine, x, y)
    }

    fn mouse_button_down_event(&mut self, engine: &Engine, button: MouseButton, x: f32, y: f32) {
        self.get_mut().mouse_button_down_event(engine, button, x, y)
    }

    fn mouse_button_up_event(&mut self, engine: &Engine, button: MouseButton, x: f32, y: f32) {
        self.get_mut().mouse_button_up_event(engine, button, x, y)
    }

    fn gamepad_button_down_event(&mut self, engine: &Engine, button: GamepadButton, repeat: bool) {
        self.get_mut()
            .gamepad_button_down_event(engine, button, repeat)
    }

    fn gamepad_button_up_event(&mut self, engine: &Engine, button: GamepadButton) {
        self.get_mut().gamepad_button_up_event(engine, button)
    }

    fn gamepad_axis_changed_event(&mut self, engine: &Engine, axis: GamepadAxis, position: f32) {
        self.get_mut()
            .gamepad_axis_changed_event(engine, axis, position)
    }

    fn gamepad_connected_event(&mut self, engine: &Engine) {
        self.get_mut().gamepad_connected_event(engine)
    }

    fn gamepad_disconnected_event(&mut self, engine: &Engine) {
        self.get_mut().gamepad_disconnected_event(engine)
    }

    fn resize_event(&mut self, engine: &Engine, width: f32, height: f32) {
        self.get_mut().resize_event(engine, width, height)
    }
}
