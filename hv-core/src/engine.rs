use {
    anyhow::*,
    miniquad as mq,
    mlua::prelude::*,
    std::sync::Mutex,
    std::{
        any::{Any, TypeId},
        collections::HashMap,
        marker::PhantomData,
        sync::{Arc, MutexGuard, RwLock, Weak},
    },
};

use crate::{
    conf::Conf,
    filesystem::Filesystem,
    input::{CursorIcon, KeyCode, KeyMods, MouseButton},
    util::RwLockExt,
};

pub type Resource<T> = Arc<RwLock<T>>;

pub trait LuaResource: LuaUserData + Send + Sync + 'static {
    const REGISTRY_KEY: &'static str;
}

pub trait LuaExt {
    fn resource<T: LuaResource>(&self) -> LuaResult<Resource<T>>;
    fn register<T: LuaResource>(&self, resource: Resource<T>) -> LuaResult<()>;
}

impl LuaExt for Lua {
    #[inline]
    fn resource<T: LuaResource>(&self) -> LuaResult<Resource<T>> {
        self.named_registry_value(T::REGISTRY_KEY)
    }

    #[inline]
    fn register<T: LuaResource>(&self, resource: Resource<T>) -> LuaResult<()> {
        self.set_named_registry_value(T::REGISTRY_KEY, resource)
    }
}

struct EngineInner {
    handler: Mutex<Box<dyn EventHandler>>,
    lua: Mutex<Lua>,
    mq: Mutex<mq::Context>,
    fs: Mutex<Filesystem>,
    resources: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

#[derive(Clone)]
pub struct EngineRef {
    weak: Weak<EngineInner>,
}

pub struct Engine<'a> {
    _restrictor: PhantomData<&'a ()>,
    inner: Arc<EngineInner>,
}

impl Engine<'static> {
    pub fn new(fs: Filesystem, mq: mq::Context, handler: impl EventHandler) -> Result<Self> {
        use mlua::StdLib;
        let lua = Lua::new_with(
            /* /* if using Lua 5.2 or above and *not* 5.1 or LuaJIT: */ StdLib::COROUTINE | */
            StdLib::TABLE | StdLib::STRING | StdLib::MATH,
            LuaOptions::new(),
        )?;

        let this = Engine {
            _restrictor: PhantomData,
            inner: Arc::new(EngineInner {
                handler: Mutex::new(Box::new(handler)),
                lua: Mutex::new(lua),
                mq: Mutex::new(mq),
                fs: Mutex::new(fs),
                resources: Default::default(),
            }),
        };

        {
            let lua = this.lua();
            lua.register(Arc::new(RwLock::new(this.downgrade())))?;

            let hv = lua.create_table()?;
            lua.globals().set("hv", hv.clone())?;

            for module in crate::plugins::registered_modules() {
                hv.set(module.name(), module.open(&lua, &this)?)?;
            }

            lua.load(mlua::chunk! {
                function hv.load() end
                function hv.update() end
                function hv.draw() end
            })
            .exec()?;
        }

        this.handler().init(&this)?;

        Ok(this)
    }

    pub fn run(conf: Conf, handler: impl EventHandler) {
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
    pub fn downgrade(&self) -> EngineRef {
        EngineRef {
            weak: Arc::downgrade(&self.inner),
        }
    }

    pub fn handler(&self) -> MutexGuard<Box<dyn EventHandler>> {
        self.inner.handler.try_lock().unwrap()
    }

    pub fn lua(&self) -> MutexGuard<Lua> {
        self.inner.lua.try_lock().unwrap()
    }

    pub fn mq(&self) -> MutexGuard<mq::Context> {
        self.inner.mq.try_lock().unwrap()
    }

    pub fn fs(&self) -> MutexGuard<Filesystem> {
        self.inner.fs.try_lock().unwrap()
    }

    pub fn insert_wrapped<T: Send + Sync + 'static>(&self, resource: Resource<T>) {
        self.inner
            .resources
            .lock()
            .unwrap()
            .insert(TypeId::of::<T>(), Box::new(resource));
    }

    pub fn insert<T: Send + Sync + 'static>(&self, resource: T) -> Resource<T> {
        let res = Arc::new(RwLock::new(resource));
        self.insert_wrapped(res.clone());
        res
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Resource<T> {
        self.inner.resources.lock().unwrap()[&TypeId::of::<T>()]
            .downcast_ref::<Resource<T>>()
            .unwrap()
            .clone()
    }

    pub fn try_get<T: Send + Sync + 'static>(&self) -> Option<Resource<T>> {
        self.inner
            .resources
            .lock()
            .unwrap()
            .get(&TypeId::of::<T>())
            .map(|entry| entry.downcast_ref::<Resource<T>>().unwrap().clone())
    }

    pub fn show_mouse(&self, show: bool) {
        self.mq().show_mouse(show);
    }

    pub fn set_mouse_grabbed(&self, grabbed: bool) {
        self.mq().set_cursor_grab(grabbed);
    }

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
    pub fn new() -> Self {
        Self { weak: Weak::new() }
    }

    pub fn try_upgrade(&self) -> Option<Engine> {
        self.weak.upgrade().map(|inner| Engine {
            _restrictor: PhantomData,
            inner,
        })
    }

    pub fn upgrade(&self) -> Engine {
        self.try_upgrade()
            .expect("failed to upgrade weak reference!")
    }
}

impl LuaUserData for EngineRef {}

impl LuaResource for EngineRef {
    const REGISTRY_KEY: &'static str = "HV_ENGINE";
}

pub struct EngineRefCache {
    weak: Weak<EngineInner>,
}

impl Default for EngineRefCache {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineRefCache {
    pub fn new() -> Self {
        Self { weak: Weak::new() }
    }

    pub fn get<'lua>(&mut self, lua: &'lua Lua) -> Engine {
        let inner = match self.weak.upgrade() {
            Some(strong) => strong,
            None => {
                self.weak = lua.resource::<EngineRef>().unwrap().borrow().weak.clone();
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

#[derive(Debug)]
pub struct WeakResourceCache<T> {
    weak: Weak<RwLock<T>>,
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
    pub fn new() -> Self {
        Self { weak: Weak::new() }
    }

    pub fn get<F: FnOnce() -> Result<Resource<T>, E>, E>(
        &mut self,
        init: F,
    ) -> Result<Resource<T>, E> {
        match self.weak.upgrade() {
            Some(resource) => Ok(resource),
            None => {
                let strong = init()?;
                self.weak = Resource::downgrade(&strong);
                Ok(strong)
            }
        }
    }
}

pub trait EventHandler: Send + Sync + 'static {
    fn init(&mut self, engine: &Engine) -> Result<()>;

    fn update(&mut self, engine: &Engine) -> Result<()>;
    fn draw(&mut self, engine: &Engine) -> Result<()>;

    fn key_down_event(
        &mut self,
        _engine: &Engine,
        _keycode: KeyCode,
        _keymods: KeyMods,
        _repeat: bool,
    ) {
    }
    fn key_up_event(&mut self, _engine: &Engine, _keycode: KeyCode, _keymods: KeyMods) {}
    fn char_event(&mut self, _engine: &Engine, _character: char, _keymods: KeyMods, _repeat: bool) {
    }
    fn mouse_motion_event(&mut self, _engine: &Engine, _x: f32, _y: f32) {}
    fn mouse_wheel_event(&mut self, _engine: &Engine, _x: f32, _y: f32) {}
    fn mouse_button_down_event(
        &mut self,
        _engine: &Engine,
        _button: MouseButton,
        _x: f32,
        _y: f32,
    ) {
    }
    fn mouse_button_up_event(&mut self, _engine: &Engine, _button: MouseButton, _x: f32, _y: f32) {}
}

impl mq::EventHandlerFree for Engine<'static> {
    fn update(&mut self) {
        self.handler().update(self).unwrap();
    }

    fn draw(&mut self) {
        self.handler().draw(self).unwrap();
    }

    fn resize_event(&mut self, _width: f32, _height: f32) {}

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

impl<T: EventHandler> EventHandler for Arc<RwLock<T>> {
    fn init(&mut self, engine: &Engine) -> Result<()> {
        self.borrow_mut().init(engine)
    }

    fn update(&mut self, engine: &Engine) -> Result<()> {
        self.borrow_mut().update(engine)
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
}
