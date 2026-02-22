use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::Window;

use crate::input::{
    InputEvent, InputState, Key, KeyState, Modifiers, MouseButton, MouseButtonState,
    MouseWheelDelta, PointerButtonEvent, PointerMoveEvent, TextEvent,
};

/// Translates a winit `WindowEvent` into an engine `InputEvent`.
///
/// Returns `None` for events not represented by the input subsystem.
pub fn translate_window_event(
    window: &Window,
    state: &InputState,
    event: &WindowEvent,
) -> Option<InputEvent> {
    match event {
        WindowEvent::ModifiersChanged(m) => {
            // winit 0.30: ModifiersChanged carries a wrapper with `.state()`.
            let ms: ModifiersState = m.state();
            Some(InputEvent::ModifiersChanged(map_modifiers(ms)))
        }

        WindowEvent::Focused(f) => Some(InputEvent::Focused(*f)),

        WindowEvent::CursorLeft { .. } => Some(InputEvent::PointerLeft),

        WindowEvent::CursorMoved { position, .. } => {
            let (x, y) = to_logical_f32(window, *position);
            Some(InputEvent::PointerMoved(PointerMoveEvent { x, y }))
        }

        WindowEvent::MouseInput { state: st, button, .. } => {
            let st = match st {
                ElementState::Pressed => MouseButtonState::Pressed,
                ElementState::Released => MouseButtonState::Released,
            };

            let button = map_mouse_button(*button);

            // winit 0.30 does not expose `Window::modifiers()`; use the tracked state.
            let modifiers = state.modifiers;

            // winit 0.30 does not expose cursor query; use tracked pointer position.
            let (x, y) = state.pointer_pos.unwrap_or((0.0, 0.0));

            Some(InputEvent::PointerButton(PointerButtonEvent {
                button,
                state: st,
                x,
                y,
                modifiers,
            }))
        }

        WindowEvent::MouseWheel { delta, .. } => {
            let modifiers = state.modifiers;
            let delta = match delta {
                MouseScrollDelta::LineDelta(x, y) => MouseWheelDelta::Line { x: *x, y: *y },
                MouseScrollDelta::PixelDelta(p) => {
                    let (x, y) = to_logical_f32(window, *p);
                    MouseWheelDelta::Pixel { x, y }
                }
            };
            Some(InputEvent::MouseWheel { delta, modifiers })
        }

        WindowEvent::KeyboardInput { event, .. } => {
            let modifiers = state.modifiers;
            let st = match event.state {
                ElementState::Pressed => KeyState::Pressed,
                ElementState::Released => KeyState::Released,
            };

            let (key, code) = map_key(event.physical_key);

            Some(InputEvent::Key {
                key,
                state: st,
                modifiers,
                code,
                repeat: event.repeat,
            })
        }

        WindowEvent::Ime(ime) => match ime {
            winit::event::Ime::Commit(text) if !text.is_empty() => Some(InputEvent::Text(TextEvent {
                text: text.clone(),
            })),
            _ => None,
        },

        _ => None,
    }
}

fn to_logical_f32(window: &Window, pos: PhysicalPosition<f64>) -> (f32, f32) {
    let scale = window.scale_factor();
    let logical = pos.to_logical::<f64>(scale);
    (logical.x as f32, logical.y as f32)
}

fn map_modifiers(m: ModifiersState) -> Modifiers {
    Modifiers {
        shift: m.shift_key(),
        ctrl: m.control_key(),
        alt: m.alt_key(),
        meta: m.super_key(),
    }
}

fn map_mouse_button(b: WinitMouseButton) -> MouseButton {
    match b {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Back => MouseButton::Back,
        WinitMouseButton::Forward => MouseButton::Forward,
        WinitMouseButton::Other(v) => MouseButton::Other(v),
    }
}

fn map_key(pk: PhysicalKey) -> (Key, u32) {
    match pk {
        PhysicalKey::Code(code) => {
            let key = match code {
                KeyCode::Escape => Key::Escape,
                KeyCode::Enter => Key::Enter,
                KeyCode::Tab => Key::Tab,
                KeyCode::Backspace => Key::Backspace,
                KeyCode::Space => Key::Space,

                KeyCode::Insert => Key::Insert,
                KeyCode::Delete => Key::Delete,
                KeyCode::Home => Key::Home,
                KeyCode::End => Key::End,
                KeyCode::PageUp => Key::PageUp,
                KeyCode::PageDown => Key::PageDown,

                KeyCode::ArrowUp => Key::ArrowUp,
                KeyCode::ArrowDown => Key::ArrowDown,
                KeyCode::ArrowLeft => Key::ArrowLeft,
                KeyCode::ArrowRight => Key::ArrowRight,

                KeyCode::ShiftLeft | KeyCode::ShiftRight => Key::Shift,
                KeyCode::ControlLeft | KeyCode::ControlRight => Key::Control,
                KeyCode::AltLeft | KeyCode::AltRight => Key::Alt,
                KeyCode::SuperLeft | KeyCode::SuperRight => Key::Meta,

                KeyCode::KeyA => Key::A,
                KeyCode::KeyB => Key::B,
                KeyCode::KeyC => Key::C,
                KeyCode::KeyD => Key::D,
                KeyCode::KeyE => Key::E,
                KeyCode::KeyF => Key::F,
                KeyCode::KeyG => Key::G,
                KeyCode::KeyH => Key::H,
                KeyCode::KeyI => Key::I,
                KeyCode::KeyJ => Key::J,
                KeyCode::KeyK => Key::K,
                KeyCode::KeyL => Key::L,
                KeyCode::KeyM => Key::M,
                KeyCode::KeyN => Key::N,
                KeyCode::KeyO => Key::O,
                KeyCode::KeyP => Key::P,
                KeyCode::KeyQ => Key::Q,
                KeyCode::KeyR => Key::R,
                KeyCode::KeyS => Key::S,
                KeyCode::KeyT => Key::T,
                KeyCode::KeyU => Key::U,
                KeyCode::KeyV => Key::V,
                KeyCode::KeyW => Key::W,
                KeyCode::KeyX => Key::X,
                KeyCode::KeyY => Key::Y,
                KeyCode::KeyZ => Key::Z,

                KeyCode::Digit0 => Key::Digit0,
                KeyCode::Digit1 => Key::Digit1,
                KeyCode::Digit2 => Key::Digit2,
                KeyCode::Digit3 => Key::Digit3,
                KeyCode::Digit4 => Key::Digit4,
                KeyCode::Digit5 => Key::Digit5,
                KeyCode::Digit6 => Key::Digit6,
                KeyCode::Digit7 => Key::Digit7,
                KeyCode::Digit8 => Key::Digit8,
                KeyCode::Digit9 => Key::Digit9,

                KeyCode::F1 => Key::F1,
                KeyCode::F2 => Key::F2,
                KeyCode::F3 => Key::F3,
                KeyCode::F4 => Key::F4,
                KeyCode::F5 => Key::F5,
                KeyCode::F6 => Key::F6,
                KeyCode::F7 => Key::F7,
                KeyCode::F8 => Key::F8,
                KeyCode::F9 => Key::F9,
                KeyCode::F10 => Key::F10,
                KeyCode::F11 => Key::F11,
                KeyCode::F12 => Key::F12,

                other => Key::Unknown(other as u32),
            };

            (key, code as u32)
        }

        // winit 0.30 uses NativeKeyCode; no stable numeric is guaranteed here.
        PhysicalKey::Unidentified(_) => (Key::Unknown(0), 0),
    }
}