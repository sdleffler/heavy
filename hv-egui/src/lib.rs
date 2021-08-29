pub extern crate egui;

mod input;
mod painter;

use egui::CursorIcon;
use hv_core::{
    engine::{Engine, LuaExt, LuaResource},
    input::{KeyCode, KeyMods, MouseButton},
    mq,
    prelude::*,
};

#[cfg(target_os = "macos")] // https://github.com/not-fl3/miniquad/issues/172
use copypasta::ClipboardProvider;

pub struct Egui {
    egui_ctx: egui::CtxRef,
    egui_ctx_resource: Shared<egui::CtxRef>,
    egui_input: egui::RawInput,
    painter: painter::Painter,
    #[cfg(target_os = "macos")]
    clipboard: Option<copypasta::ClipboardContext>,
    shapes: Option<Vec<egui::epaint::ClippedShape>>,
}

impl Egui {
    pub fn new(engine: &Engine) -> Result<Shared<Egui>> {
        if let Some(this) = engine.try_get::<Self>() {
            return Ok(this);
        }

        let egui_ctx = egui::CtxRef::default();
        let egui_ctx_resource = engine.insert(egui_ctx.clone());

        let this = Self {
            egui_ctx,
            egui_ctx_resource,
            painter: painter::Painter::new(engine),
            egui_input: Default::default(),
            #[cfg(target_os = "macos")]
            clipboard: init_clipboard(),
            shapes: None,
        };

        let resource = engine.insert(this);
        engine.lua().register(resource.clone())?;

        Ok(resource)
    }

    /// Use this to open egui windows, panels etc.
    /// Can only be used between [`Self::begin_frame`] and [`Self::end_frame`].
    pub fn egui_ctx(&self) -> &egui::CtxRef {
        &self.egui_ctx
    }

    /// Call this at the start of each `draw` call.
    pub fn begin_frame(&mut self, engine: &Engine) {
        let mq = &engine.mq();
        input::on_frame_start(&mut self.egui_input, mq);
        self.egui_ctx.begin_frame(self.egui_input.take());
        *self.egui_ctx_resource.borrow_mut() = self.egui_ctx.clone();
    }

    /// Call this at the end of each `draw` call.
    /// This will draw the `egui` interface.
    pub fn end_frame(&mut self, engine: &Engine) {
        let mq = &mut engine.mq();

        let (output, shapes) = self.egui_ctx.end_frame();
        if self.shapes.is_some() {
            eprintln!(
                "Egui contents not drawed. You need to call `draw` after calling `end_frame`"
            );
        }
        self.shapes = Some(shapes);

        let egui::Output {
            cursor_icon,
            open_url,
            copied_text,
            needs_repaint: _,   // miniquad always runs at full framerate
            events: _,          // no screen reader
            text_cursor_pos: _, // no IME
        } = output;

        if let Some(url) = open_url {
            quad_url::link_open(&url.url, url.new_tab);
        }

        if cursor_icon == egui::CursorIcon::None {
            mq.show_mouse(false);
        } else {
            mq.show_mouse(true);

            let mq_cursor_icon = to_mq_cursor_icon(cursor_icon);
            let mq_cursor_icon = mq_cursor_icon.unwrap_or(mq::CursorIcon::Default);
            mq.set_mouse_cursor(mq_cursor_icon);
        }

        if !copied_text.is_empty() {
            self.set_clipboard(mq, copied_text);
        }
    }

    /// Call this when you need to draw egui.
    /// Must be called after `end_frame`.
    pub fn draw(&mut self, engine: &Engine) {
        if let Some(shapes) = self.shapes.take() {
            let paint_jobs = self.egui_ctx.tessellate(shapes);
            self.painter
                .paint(engine, paint_jobs, &self.egui_ctx.texture());
        } else {
            eprintln!("Failed to draw egui. You need to call `end_frame` before calling `draw`");
        }
    }

    pub fn mouse_motion_event(&mut self, engine: &Engine, x: f32, y: f32) {
        let mq = &engine.mq();
        let pos = egui::pos2(x as f32 / mq.dpi_scale(), y as f32 / mq.dpi_scale());
        self.egui_input.events.push(egui::Event::PointerMoved(pos))
    }

    pub fn mouse_wheel_event(&mut self, _engine: &Engine, dx: f32, dy: f32) {
        let delta = egui::vec2(dx, dy); // Correct for web, but too slow for mac native :/

        if self.egui_input.modifiers.ctrl {
            // Treat as zoom instead:
            self.egui_input.zoom_delta *= (delta.y / 200.0).exp();
        } else {
            self.egui_input.scroll_delta += delta;
        }
    }

    pub fn mouse_button_down_event(&mut self, engine: &Engine, mb: MouseButton, x: f32, y: f32) {
        let mq = &engine.mq();
        let pos = egui::pos2(x as f32 / mq.dpi_scale(), y as f32 / mq.dpi_scale());
        let button = to_egui_button(mb);
        self.egui_input.events.push(egui::Event::PointerButton {
            pos,
            button,
            pressed: true,
            modifiers: self.egui_input.modifiers,
        })
    }

    pub fn mouse_button_up_event(&mut self, engine: &Engine, mb: MouseButton, x: f32, y: f32) {
        let mq = &engine.mq();
        let pos = egui::pos2(x as f32 / mq.dpi_scale(), y as f32 / mq.dpi_scale());
        let button = to_egui_button(mb);

        self.egui_input.events.push(egui::Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: self.egui_input.modifiers,
        })
    }

    pub fn char_event(&mut self, chr: char) {
        if input::is_printable_char(chr)
            && !self.egui_input.modifiers.ctrl
            && !self.egui_input.modifiers.mac_cmd
        {
            self.egui_input
                .events
                .push(egui::Event::Text(chr.to_string()));
        }
    }

    pub fn key_down_event(&mut self, engine: &Engine, keycode: KeyCode, keymods: KeyMods) {
        let modifiers = input::egui_modifiers_from_hv_modifiers(keymods);
        self.egui_input.modifiers = modifiers;

        if modifiers.command && keycode == KeyCode::X {
            self.egui_input.events.push(egui::Event::Cut);
        } else if modifiers.command && keycode == KeyCode::C {
            self.egui_input.events.push(egui::Event::Copy);
        } else if modifiers.command && keycode == KeyCode::V {
            let mq = &mut engine.mq();
            if let Some(text) = self.get_clipboard(mq) {
                self.egui_input.events.push(egui::Event::Text(text));
            }
        } else if let Some(key) = input::egui_key_from_hv_key(keycode) {
            self.egui_input.events.push(egui::Event::Key {
                key,
                pressed: true,
                modifiers,
            })
        }
    }

    pub fn key_up_event(&mut self, keycode: KeyCode, keymods: KeyMods) {
        let modifiers = input::egui_modifiers_from_hv_modifiers(keymods);
        self.egui_input.modifiers = modifiers;
        if let Some(key) = input::egui_key_from_hv_key(keycode) {
            self.egui_input.events.push(egui::Event::Key {
                key,
                pressed: false,
                modifiers,
            })
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn set_clipboard(&mut self, mq: &mut mq::Context, text: String) {
        mq::clipboard::set(mq, text.as_str());
    }

    #[cfg(not(target_os = "macos"))]
    fn get_clipboard(&mut self, mq: &mut mq::Context) -> Option<String> {
        mq::clipboard::get(mq)
    }

    #[cfg(target_os = "macos")]
    fn set_clipboard(&mut self, _mq: &mut mq::Context, text: String) {
        if let Some(clipboard) = &mut self.clipboard {
            if let Err(err) = clipboard.set_contents(text) {
                eprintln!("Copy/Cut error: {}", err);
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn get_clipboard(&mut self, _mq: &mut mq::Context) -> Option<String> {
        if let Some(clipboard) = &mut self.clipboard {
            match clipboard.get_contents() {
                Ok(contents) => Some(contents),
                Err(err) => {
                    eprintln!("Paste error: {}", err);
                    None
                }
            }
        } else {
            None
        }
    }
}

impl LuaResource for Egui {
    const REGISTRY_KEY: &'static str = "HV_EGUI";
}

impl LuaUserData for Egui {}

#[cfg(target_os = "macos")]
fn init_clipboard() -> Option<copypasta::ClipboardContext> {
    match copypasta::ClipboardContext::new() {
        Ok(clipboard) => Some(clipboard),
        Err(err) => {
            eprintln!("Failed to initialize clipboard: {}", err);
            None
        }
    }
}

fn to_egui_button(mb: MouseButton) -> egui::PointerButton {
    match mb {
        MouseButton::Left => egui::PointerButton::Primary,
        MouseButton::Right => egui::PointerButton::Secondary,
        MouseButton::Middle => egui::PointerButton::Middle,
        // MouseButton::Unknown => egui::PointerButton::Primary,
    }
}

fn to_mq_cursor_icon(cursor_icon: egui::CursorIcon) -> Option<mq::CursorIcon> {
    match cursor_icon {
        // Handled outside this function
        CursorIcon::None => None,

        egui::CursorIcon::Default => Some(mq::CursorIcon::Default),
        egui::CursorIcon::PointingHand => Some(mq::CursorIcon::Pointer),
        egui::CursorIcon::Text => Some(mq::CursorIcon::Text),
        egui::CursorIcon::ResizeHorizontal => Some(mq::CursorIcon::EWResize),
        egui::CursorIcon::ResizeVertical => Some(mq::CursorIcon::NSResize),
        egui::CursorIcon::ResizeNeSw => Some(mq::CursorIcon::NESWResize),
        egui::CursorIcon::ResizeNwSe => Some(mq::CursorIcon::NWSEResize),
        egui::CursorIcon::Help => Some(mq::CursorIcon::Help),
        egui::CursorIcon::Wait => Some(mq::CursorIcon::Wait),
        egui::CursorIcon::Crosshair => Some(mq::CursorIcon::Crosshair),
        egui::CursorIcon::Move => Some(mq::CursorIcon::Move),
        egui::CursorIcon::NotAllowed => Some(mq::CursorIcon::NotAllowed),

        // Similar enough
        egui::CursorIcon::AllScroll => Some(mq::CursorIcon::Move),
        egui::CursorIcon::Progress => Some(mq::CursorIcon::Wait),

        // Not implemented, see https://github.com/not-fl3/miniquad/pull/173 and https://github.com/not-fl3/miniquad/issues/171
        egui::CursorIcon::Grab | egui::CursorIcon::Grabbing => None,

        // Also not implemented:
        egui::CursorIcon::Alias
        | egui::CursorIcon::Cell
        | egui::CursorIcon::ContextMenu
        | egui::CursorIcon::Copy
        | egui::CursorIcon::NoDrop
        | egui::CursorIcon::VerticalText
        | egui::CursorIcon::ZoomIn
        | egui::CursorIcon::ZoomOut => None,
    }
}
