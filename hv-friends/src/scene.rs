//! This is a modified version of the `scene` module from `ggez-goodies`. Please
//! see the licensing information below in the source file.
//!
//! The Scene system is basically for transitioning between
//! *completely* different states that have entirely different game
//! loops and but which all share a state.  It operates as a stack, with new
//! scenes getting pushed to the stack (while the old ones stay in
//! memory unchanged).  Apparently this is basically a push-down automata.
//!
//! Also there's no reason you can't have a Scene contain its own
//! Scene subsystem to do its own indirection.  With a different state
//! type, as well!  What fun!  Though whether you want to go that deep
//! down the rabbit-hole is up to you.  I haven't found it necessary
//! yet.
//!
//! This is basically identical in concept to the Amethyst engine's scene
//! system, the only difference is the details of how the pieces are put
//! together.

use hv_core::{
    engine::{Engine, EngineRef, EventHandler},
    input::{KeyCode, KeyMods, MouseButton},
    util::RwLockExt,
};

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

use {
    anyhow::*,
    std::{
        borrow::Cow,
        fmt,
        sync::{Arc, RwLock},
    },
};

pub struct DynamicScene<C, Ev>(Arc<RwLock<dyn Scene<C, Ev>>>);

impl<C, Ev> Clone for DynamicScene<C, Ev> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<C: 'static, Ev: 'static> fmt::Debug for DynamicScene<C, Ev> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let borrowed = self.0.borrow();
        let name = borrowed.name();
        f.debug_tuple("DynamicScene").field(&name).finish()
    }
}

impl<C, Ev> DynamicScene<C, Ev> {
    pub fn new<T>(scene: T) -> Self
    where
        T: Scene<C, Ev> + 'static,
    {
        Self(Arc::new(RwLock::new(scene)))
    }

    fn map_mut_inner<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut dyn Scene<C, Ev>) -> R,
    {
        match Arc::get_mut(&mut self.0) {
            Some(m) => f(m.get_mut().unwrap()),
            None => f(&mut *self.0.borrow_mut()),
        }
    }
}

impl<C: 'static, Ev: 'static> Scene<C, Ev> for DynamicScene<C, Ev> {
    fn update(&mut self, scene_stack: &mut SceneStack<C, Ev>, ctx: &mut C) -> Result<()> {
        self.map_mut_inner(|s| s.update(scene_stack, ctx))
    }

    fn draw(&mut self, ctx: &mut C) -> Result<()> {
        self.map_mut_inner(|s| s.draw(ctx))
    }

    fn event(&mut self, ctx: &mut C, event: Ev) -> Result<()> {
        self.map_mut_inner(|s| s.event(ctx, event))
    }

    fn name(&self) -> Option<Cow<'_, str>> {
        self.0.borrow().name().map(|cow| cow.into_owned().into())
    }

    fn draw_previous(&self) -> bool {
        self.0.borrow().draw_previous()
    }
}

/// A trait for you to implement on a scene.
/// Defines the callbacks the scene uses:
/// a common context type `C`, and an input event type `Ev`.
pub trait Scene<C, Ev>: Send + Sync + 'static {
    fn update(&mut self, scene_stack: &mut SceneStack<C, Ev>, ctx: &mut C) -> Result<()>;
    fn draw(&mut self, ctx: &mut C) -> Result<()>;
    fn event(&mut self, ctx: &mut C, event: Ev) -> Result<()>;
    /// Only used for human-readable convenience (or not at all, tbh)
    fn name(&self) -> Option<Cow<'_, str>> {
        None
    }
    /// This returns whether or not to draw the next scene down on the
    /// stack as well; this is useful for layers or GUI stuff that
    /// only partially covers the screen.
    fn draw_previous(&self) -> bool {
        false
    }
}

/// A stack of `Scene`'s, together with a context object.
pub struct SceneStack<C, Ev> {
    scenes: Vec<DynamicScene<C, Ev>>,
}

impl<C, Ev> Default for SceneStack<C, Ev>
where
    C: 'static,
    Ev: 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<C, Ev> SceneStack<C, Ev>
where
    C: 'static,
    Ev: 'static,
{
    pub fn new() -> Self {
        Self { scenes: Vec::new() }
    }

    /// Add a new scene to the top of the stack.
    pub fn push(&mut self, scene: DynamicScene<C, Ev>) {
        self.scenes.push(scene)
    }

    /// Remove the top scene from the stack and returns it;
    /// panics if there is none.
    pub fn pop(&mut self) -> DynamicScene<C, Ev> {
        self.scenes
            .pop()
            .expect("ERROR: Popped an empty scene stack.")
    }

    /// Replace the top scene on the stack by popping and then
    /// pushing a new scene. Will panic if the stack is empty.
    /// Returns the replaced scene.
    pub fn replace(&mut self, scene: DynamicScene<C, Ev>) -> DynamicScene<C, Ev> {
        let replaced = self.pop();
        self.push(scene);
        replaced
    }

    /// Returns the current scene; panics if there is none.
    pub fn current(&self) -> &DynamicScene<C, Ev> {
        self.scenes
            .last()
            .expect("ERROR: Tried to get current scene of an empty scene stack.")
    }

    // These functions must be on the SceneStack because otherwise
    // if you try to get the current scene and the world to call
    // update() on the current scene it causes a double-borrow.  :/
    pub fn update(&mut self, ctx: &mut C) -> Result<()> {
        if let Some(mut current_scene) = self.scenes.last().cloned() {
            current_scene.update(self, ctx)?;
        }

        Ok(())
    }

    /// We walk down the scene stack until we find a scene where we aren't
    /// supposed to draw the previous one, then draw them from the bottom up.
    ///
    /// This allows for layering GUI's and such.
    fn draw_scenes(scenes: &mut [DynamicScene<C, Ev>], ctx: &mut C) -> Result<()> {
        if let Some((current, rest)) = scenes.split_last_mut() {
            if current.draw_previous() {
                SceneStack::draw_scenes(rest, ctx)?;
            }
            current.draw(ctx)
        } else {
            Ok(())
        }
    }

    /// Draw the current scene.
    pub fn draw(&mut self, ctx: &mut C) -> Result<()> {
        SceneStack::draw_scenes(&mut self.scenes, ctx)
    }

    /// Feeds the given event to the current scene.
    pub fn event(&mut self, ctx: &mut C, event: Ev) -> Result<()> {
        if let Some(current_scene) = self.scenes.last_mut() {
            current_scene.event(ctx, event)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EngineEvent {
    KeyDown {
        keycode: KeyCode,
        keymods: KeyMods,
        repeat: bool,
    },
    KeyUp {
        keycode: KeyCode,
        keymods: KeyMods,
    },
    Char {
        character: char,
        keymods: KeyMods,
        repeat: bool,
    },
    MouseMotion {
        x: f32,
        y: f32,
    },
    MouseWheel {
        x: f32,
        y: f32,
    },
    MouseButtonDown {
        button: MouseButton,
        x: f32,
        y: f32,
    },
    MouseButtonUp {
        button: MouseButton,
        x: f32,
        y: f32,
    },
}

impl EngineEvent {
    pub fn dispatch<E: EventHandler>(self, handler: &mut E, engine: &Engine) {
        match self {
            Self::KeyDown {
                keycode,
                keymods,
                repeat,
            } => handler.key_down_event(engine, keycode, keymods, repeat),
            Self::KeyUp { keycode, keymods } => handler.key_up_event(engine, keycode, keymods),
            Self::Char {
                character,
                keymods,
                repeat,
            } => handler.char_event(engine, character, keymods, repeat),
            Self::MouseMotion { x, y } => handler.mouse_motion_event(engine, x, y),
            Self::MouseWheel { x, y } => handler.mouse_wheel_event(engine, x, y),
            Self::MouseButtonDown { button, x, y } => {
                handler.mouse_button_down_event(engine, button, x, y)
            }
            Self::MouseButtonUp { button, x, y } => {
                handler.mouse_button_up_event(engine, button, x, y)
            }
        }
    }
}

pub struct InitScene<F>(Option<F>)
where
    F: FnOnce(&mut SceneStack<EngineRef, EngineEvent>, &Engine) -> Result<()>
        + Send
        + Sync
        + 'static;

impl<F> Scene<EngineRef, EngineEvent> for InitScene<F>
where
    F: FnOnce(&mut SceneStack<EngineRef, EngineEvent>, &Engine) -> Result<()>
        + Send
        + Sync
        + 'static,
{
    fn update(
        &mut self,
        scene_stack: &mut SceneStack<EngineRef, EngineEvent>,
        ctx: &mut EngineRef,
    ) -> Result<()> {
        // Get ourselves off the stack.
        scene_stack.pop();
        (self.0.take().expect("init scene must be called only once"))(
            scene_stack,
            &mut ctx.upgrade(),
        )
    }

    fn draw(&mut self, _ctx: &mut EngineRef) -> Result<()> {
        Ok(())
    }

    fn event(&mut self, _ctx: &mut EngineRef, _event: EngineEvent) -> Result<()> {
        Ok(())
    }
}

impl<C, E, T: Scene<C, E>> Scene<C, E> for Arc<RwLock<T>> {
    fn update(&mut self, scene_stack: &mut SceneStack<C, E>, ctx: &mut C) -> Result<()> {
        self.borrow_mut().update(scene_stack, ctx)
    }

    fn draw(&mut self, ctx: &mut C) -> Result<()> {
        self.borrow_mut().draw(ctx)
    }

    fn event(&mut self, ctx: &mut C, event: E) -> Result<()> {
        self.borrow_mut().event(ctx, event)
    }
}

impl EventHandler for SceneStack<EngineRef, EngineEvent> {
    fn init(&mut self, _engine: &Engine) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, engine: &Engine) -> Result<()> {
        self.update(&mut engine.downgrade())
    }

    fn draw(&mut self, engine: &Engine) -> Result<()> {
        self.draw(&mut engine.downgrade())
    }

    fn key_down_event(
        &mut self,
        engine: &Engine,
        keycode: KeyCode,
        keymods: KeyMods,
        repeat: bool,
    ) {
        // FIXME(sleffy): error handling
        let _ = self.event(
            &mut engine.downgrade(),
            EngineEvent::KeyDown {
                keycode,
                keymods,
                repeat,
            },
        );
    }

    fn key_up_event(&mut self, engine: &Engine, keycode: KeyCode, keymods: KeyMods) {
        // FIXME(sleffy): error handling
        let _ = self.event(
            &mut engine.downgrade(),
            EngineEvent::KeyUp { keycode, keymods },
        );
    }

    fn char_event(&mut self, engine: &Engine, character: char, keymods: KeyMods, repeat: bool) {
        // FIXME(sleffy): error handling
        let _ = self.event(
            &mut engine.downgrade(),
            EngineEvent::Char {
                character,
                keymods,
                repeat,
            },
        );
    }

    fn mouse_motion_event(&mut self, engine: &Engine, x: f32, y: f32) {
        // FIXME(sleffy): error handling
        let _ = self.event(&mut engine.downgrade(), EngineEvent::MouseMotion { x, y });
    }

    fn mouse_wheel_event(&mut self, engine: &Engine, x: f32, y: f32) {
        // FIXME(sleffy): error handling
        let _ = self.event(&mut engine.downgrade(), EngineEvent::MouseWheel { x, y });
    }

    fn mouse_button_down_event(&mut self, engine: &Engine, button: MouseButton, x: f32, y: f32) {
        // FIXME(sleffy): error handling
        let _ = self.event(
            &mut engine.downgrade(),
            EngineEvent::MouseButtonDown { button, x, y },
        );
    }

    fn mouse_button_up_event(&mut self, engine: &Engine, button: MouseButton, x: f32, y: f32) {
        // FIXME(sleffy): error handling
        let _ = self.event(
            &mut engine.downgrade(),
            EngineEvent::MouseButtonUp { button, x, y },
        );
    }
}

impl SceneStack<EngineRef, EngineEvent> {
    pub fn with_init<F>(func: F) -> Self
    where
        F: FnOnce(&mut Self, &Engine) -> Result<()> + Send + Sync + 'static,
    {
        let mut this = Self::new();
        this.push(DynamicScene::new(InitScene(Some(func))));
        this
    }
}
