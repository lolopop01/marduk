//! Input subsystem.
//!
//! Public API is platform-agnostic and does not expose winit types.
//! Runtime code is responsible for translating platform events into `InputEvent`s.

mod frame;
mod state;
mod types;

pub use frame::InputFrame;
pub use state::InputState;
pub use types::{
    Axis,
    InputEvent,
    Key,
    KeyState,
    Modifiers,
    MouseButton,
    MouseButtonState,
    MouseWheelDelta,
    PointerButtonEvent,
    PointerMoveEvent,
    TextEvent,
};