use marduk_engine::coords::Vec2;
use marduk_engine::input::Key;

/// Input events routed through the widget tree.
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// Primary mouse button pressed and released at `pos`.
    Click { pos: Vec2 },
    /// Mouse moved to `pos` (fired every frame).
    Hover { pos: Vec2 },
    /// Committed text input (one or more characters).
    TextInput { text: String },
    /// Named key pressed (Backspace, Enter, arrow keys, …).
    KeyPress { key: Key },
}

/// Result returned by [`Widget::on_event`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    /// Event was handled — stop routing to siblings / parents.
    Consumed,
    /// Event was not handled — keep routing.
    Ignored,
}

impl EventResult {
    #[inline]
    pub fn is_consumed(self) -> bool {
        self == EventResult::Consumed
    }
}
