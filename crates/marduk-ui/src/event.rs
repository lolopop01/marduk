use marduk_engine::coords::Vec2;
use marduk_engine::input::Key;

pub use marduk_engine::input::Modifiers;

/// Input events routed through the widget tree.
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// Primary mouse button pressed and released at `pos`.
    Click { pos: Vec2 },
    /// Mouse moved to `pos` (fired every frame).
    Hover { pos: Vec2 },
    /// Mouse moved while the primary button is held.
    /// `pos` is the current cursor position; `start` is where the drag began.
    Drag { pos: Vec2, start: Vec2 },
    /// Committed text input (one or more characters).
    TextInput { text: String },
    /// Named key pressed (Backspace, Enter, arrow keys, …).
    KeyPress { key: Key, modifiers: Modifiers },
    /// Mouse wheel / trackpad scroll.
    ///
    /// `delta` > 0 → scroll down (reveal content below); < 0 → scroll up.
    ScrollWheel { delta: f32 },
    /// Primary button released after a drag.
    ///
    /// `pos` is where the button was released; `start` is where the drag began.
    /// Unlike `Click`, this fires even when `pos` is outside the widget — use
    /// `rect.contains(start)` to check ownership of the drag.
    DragEnd { pos: Vec2, start: Vec2 },
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
