use std::collections::HashSet;

use super::frame::InputFrame;
use super::types::{
    InputEvent,
    Key,
    KeyState,
    Modifiers,
    MouseButton,
    MouseButtonState,
    PointerButtonEvent,
    PointerMoveEvent,
    TextEvent,
};

/// Current input state for a single window.
///
/// Holds "is down" information and current pointer position.
/// Per-frame transitions are recorded into an `InputFrame`.
#[derive(Debug, Default)]
pub struct InputState {
    /// Current modifier state.
    pub modifiers: Modifiers,

    /// Whether the window is focused.
    pub focused: bool,

    /// Pointer position in logical pixels.
    pub pointer_pos: Option<(f32, f32)>,

    /// Set of currently held keys.
    pub keys_down: HashSet<Key>,

    /// Set of currently held mouse buttons.
    pub buttons_down: HashSet<MouseButton>,
}

impl InputState {
    /// Applies a platform-agnostic input event to the current state and writes deltas to `frame`.
    pub fn apply_event(&mut self, frame: &mut InputFrame, ev: InputEvent) {
        match &ev {
            InputEvent::ModifiersChanged(m) => {
                self.modifiers = *m;
            }

            InputEvent::Focused(f) => {
                self.focused = *f;
                if !*f {
                    // Conservative behavior: on focus loss, clear "down" sets.
                    // Avoids stuck keys/buttons when focus changes mid-press.
                    self.keys_down.clear();
                    self.buttons_down.clear();
                }
            }

            InputEvent::PointerMoved(PointerMoveEvent { x, y }) => {
                self.pointer_pos = Some((*x, *y));
            }

            InputEvent::PointerLeft => {
                self.pointer_pos = None;
            }

            InputEvent::Key {
                key,
                state,
                modifiers,
                ..
            } => {
                self.modifiers = *modifiers;

                match state {
                    KeyState::Pressed => {
                        let inserted = self.keys_down.insert(*key);
                        if inserted {
                            frame.keys_pressed.insert(*key);
                        }
                    }
                    KeyState::Released => {
                        let removed = self.keys_down.remove(key);
                        if removed {
                            frame.keys_released.insert(*key);
                        }
                    }
                }
            }

            InputEvent::PointerButton(PointerButtonEvent {
                                          button,
                                          state,
                                          x,
                                          y,
                                          modifiers,
                                      }) => {
                self.pointer_pos = Some((*x, *y));
                self.modifiers = *modifiers;

                match state {
                    MouseButtonState::Pressed => {
                        let inserted = self.buttons_down.insert(*button);
                        if inserted {
                            frame.buttons_pressed.insert(*button);
                        }
                    }
                    MouseButtonState::Released => {
                        let removed = self.buttons_down.remove(button);
                        if removed {
                            frame.buttons_released.insert(*button);
                        }
                    }
                }
            }

            InputEvent::MouseWheel { modifiers, .. } => {
                self.modifiers = *modifiers;
            }

            InputEvent::Text(TextEvent { text: _ }) => {
                // No persistent state update; text is consumed as a per-frame stream.
            }
        }

        // Update frame with raw event + text streams.
        match &ev {
            InputEvent::Text(t) => frame.text.push(t.clone()),
            _ => {}
        }

        frame.push_event(ev);
    }

    /// Helper queries
    pub fn key_down(&self, key: Key) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn button_down(&self, btn: MouseButton) -> bool {
        self.buttons_down.contains(&btn)
    }
}