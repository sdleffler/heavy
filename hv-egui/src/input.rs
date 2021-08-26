use hv_core::{input as hvi, mq};

pub fn on_frame_start(egui_input: &mut egui::RawInput, mq: &mq::Context) {
    let screen_size_in_pixels = mq.screen_size();
    let pixels_per_point = mq.dpi_scale();
    let screen_size_in_points =
        egui::vec2(screen_size_in_pixels.0, screen_size_in_pixels.1) / pixels_per_point;
    egui_input.screen_rect = Some(egui::Rect::from_min_size(
        Default::default(),
        screen_size_in_points,
    ));
    egui_input.pixels_per_point = Some(pixels_per_point);

    // mq::date::now() lies on web
    #[cfg(not(target_arch = "wasm32"))]
    {
        egui_input.time = Some(mq::date::now());
    }
}

/// miniquad sends special keys (backspace, delete, F1, ...) as characters.
/// Ignore those.
/// We also ignore '\r', '\n', '\t'.
/// Newlines are handled by the `Key::Enter` event.
pub fn is_printable_char(chr: char) -> bool {
    #![allow(clippy::manual_range_contains)]

    let is_in_private_use_area = '\u{e000}' <= chr && chr <= '\u{f8ff}'
        || '\u{f0000}' <= chr && chr <= '\u{ffffd}'
        || '\u{100000}' <= chr && chr <= '\u{10fffd}';

    !is_in_private_use_area && !chr.is_ascii_control()
}

pub fn egui_modifiers_from_hv_modifiers(keymods: hvi::KeyMods) -> egui::Modifiers {
    egui::Modifiers {
        alt: keymods.alt,
        ctrl: keymods.ctrl,
        shift: keymods.shift,
        mac_cmd: keymods.logo && cfg!(target_os = "macos"),
        command: if cfg!(target_os = "macos") {
            keymods.logo
        } else {
            keymods.ctrl
        },
    }
}

pub fn egui_key_from_hv_key(key: hvi::KeyCode) -> Option<egui::Key> {
    Some(match key {
        hvi::KeyCode::Down => egui::Key::ArrowDown,
        hvi::KeyCode::Left => egui::Key::ArrowLeft,
        hvi::KeyCode::Right => egui::Key::ArrowRight,
        hvi::KeyCode::Up => egui::Key::ArrowUp,

        hvi::KeyCode::Escape => egui::Key::Escape,
        hvi::KeyCode::Tab => egui::Key::Tab,
        hvi::KeyCode::Backspace => egui::Key::Backspace,
        hvi::KeyCode::Enter => egui::Key::Enter,
        hvi::KeyCode::Space => egui::Key::Space,

        hvi::KeyCode::Insert => egui::Key::Insert,
        hvi::KeyCode::Delete => egui::Key::Delete,
        hvi::KeyCode::Home => egui::Key::Home,
        hvi::KeyCode::End => egui::Key::End,
        hvi::KeyCode::PageUp => egui::Key::PageUp,
        hvi::KeyCode::PageDown => egui::Key::PageDown,

        hvi::KeyCode::Key0 => egui::Key::Num0,
        hvi::KeyCode::Key1 => egui::Key::Num1,
        hvi::KeyCode::Key2 => egui::Key::Num2,
        hvi::KeyCode::Key3 => egui::Key::Num3,
        hvi::KeyCode::Key4 => egui::Key::Num4,
        hvi::KeyCode::Key5 => egui::Key::Num5,
        hvi::KeyCode::Key6 => egui::Key::Num6,
        hvi::KeyCode::Key7 => egui::Key::Num7,
        hvi::KeyCode::Key8 => egui::Key::Num8,
        hvi::KeyCode::Key9 => egui::Key::Num9,

        hvi::KeyCode::A => egui::Key::A,
        hvi::KeyCode::B => egui::Key::B,
        hvi::KeyCode::C => egui::Key::C,
        hvi::KeyCode::D => egui::Key::D,
        hvi::KeyCode::E => egui::Key::E,
        hvi::KeyCode::F => egui::Key::F,
        hvi::KeyCode::G => egui::Key::G,
        hvi::KeyCode::H => egui::Key::H,
        hvi::KeyCode::I => egui::Key::I,
        hvi::KeyCode::J => egui::Key::J,
        hvi::KeyCode::K => egui::Key::K,
        hvi::KeyCode::L => egui::Key::L,
        hvi::KeyCode::M => egui::Key::M,
        hvi::KeyCode::N => egui::Key::N,
        hvi::KeyCode::O => egui::Key::O,
        hvi::KeyCode::P => egui::Key::P,
        hvi::KeyCode::Q => egui::Key::Q,
        hvi::KeyCode::R => egui::Key::R,
        hvi::KeyCode::S => egui::Key::S,
        hvi::KeyCode::T => egui::Key::T,
        hvi::KeyCode::U => egui::Key::U,
        hvi::KeyCode::V => egui::Key::V,
        hvi::KeyCode::W => egui::Key::W,
        hvi::KeyCode::X => egui::Key::X,
        hvi::KeyCode::Y => egui::Key::Y,
        hvi::KeyCode::Z => egui::Key::Z,

        _ => return None,
    })
}
