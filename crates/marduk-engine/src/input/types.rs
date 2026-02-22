use std::fmt;

/// Keyboard key identifier.
///
/// This is intentionally minimal for v0.
/// The runtime should map platform scancodes/keycodes into these variants where possible.
/// For unsupported keys, use `Key::Unknown(u32)` with a stable platform code.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Key {
    // Common control keys
    Escape,
    Enter,
    Tab,
    Backspace,
    Space,

    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,

    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,

    // Modifiers as keys (useful for focus/navigation policies)
    Shift,
    Control,
    Alt,
    Meta,

    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    // Digits
    Digit0, Digit1, Digit2, Digit3, Digit4,
    Digit5, Digit6, Digit7, Digit8, Digit9,

    // Function keys
    F1, F2, F3, F4, F5, F6,
    F7, F8, F9, F10, F11, F12,

    /// Platform-dependent key not yet represented here.
    Unknown(u32),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum KeyState {
    Pressed,
    Released,
}

/// Mouse button identifier.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum MouseButtonState {
    Pressed,
    Released,
}

/// Modifier keys state.
///
/// This is stored as booleans rather than bitflags to keep it explicit and stable.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    pub fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt || self.meta
    }
}

/// Axis identifier for wheel/scroll style inputs.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Axis {
    X,
    Y,
}

/// Mouse wheel delta.
///
/// `Line` corresponds to "scroll lines" style input; `Pixel` is high precision.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MouseWheelDelta {
    Line { x: f32, y: f32 },
    Pixel { x: f32, y: f32 },
}

/// Pointer move event in logical pixels.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PointerMoveEvent {
    pub x: f32,
    pub y: f32,
}

/// Pointer button event.
///
/// Coordinates are included to make event processing independent from an external
/// "current pointer position" if desired.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct PointerButtonEvent {
    pub button: MouseButton,
    pub state: MouseButtonState,
    pub x: f32,
    pub y: f32,
    pub modifiers: Modifiers,
}

/// Text input event.
///
/// Represents committed text (not IME composition). IME can be added later
/// by introducing additional event types without changing this.
#[derive(Debug, Clone, PartialEq)]
pub struct TextEvent {
    pub text: String,
}

/// Platform-agnostic input events emitted by the runtime.
///
/// Runtime translates window system events into these.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    ModifiersChanged(Modifiers),

    Key {
        key: Key,
        state: KeyState,
        modifiers: Modifiers,
        /// Stable platform code when available (e.g. scancode).
        code: u32,
        /// True when event is a key-repeat.
        repeat: bool,
    },

    PointerMoved(PointerMoveEvent),
    PointerButton(PointerButtonEvent),

    MouseWheel {
        delta: MouseWheelDelta,
        modifiers: Modifiers,
    },

    Text(TextEvent),

    /// Pointer left the window surface.
    PointerLeft,

    /// Window focus change.
    Focused(bool),
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}