use std::collections::HashSet;

use super::types::{InputEvent, Key, MouseButton, TextEvent};

/// Per-frame input deltas.
///
/// `InputState` provides the current state (held keys/buttons, pointer position).
/// `InputFrame` provides events and transition sets for the current frame.
#[derive(Debug, Default)]
pub struct InputFrame {
    /// Raw events in arrival order.
    pub events: Vec<InputEvent>,

    /// Keys pressed this frame.
    pub keys_pressed: HashSet<Key>,

    /// Keys released this frame.
    pub keys_released: HashSet<Key>,

    /// Mouse buttons pressed this frame.
    pub buttons_pressed: HashSet<MouseButton>,

    /// Mouse buttons released this frame.
    pub buttons_released: HashSet<MouseButton>,

    /// Text committed this frame.
    pub text: Vec<TextEvent>,
}

impl InputFrame {
    pub fn clear(&mut self) {
        self.events.clear();
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.buttons_pressed.clear();
        self.buttons_released.clear();
        self.text.clear();
    }

    pub fn push_event(&mut self, ev: InputEvent) {
        self.events.push(ev);
    }
}