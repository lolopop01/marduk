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

    /// Accumulated scroll wheel delta this frame.
    ///
    /// Positive = wheel moved in the "scroll down" direction (reveal content below).
    /// Negative = scroll up. Line deltas are passed through as-is; pixel deltas
    /// are normalised by dividing by 20 so both are in comparable "line" units.
    pub scroll_delta: f32,
}

impl InputFrame {
    pub fn clear(&mut self) {
        self.events.clear();
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.buttons_pressed.clear();
        self.buttons_released.clear();
        self.text.clear();
        self.scroll_delta = 0.0;
    }

    pub fn push_event(&mut self, ev: InputEvent) {
        self.events.push(ev);
    }
}