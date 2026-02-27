//! Window + runtime loop.
//!
//! Owns the `winit` EventLoop and Window, and wires them to the GPU layer.

mod runtime;

pub use runtime::{Runtime, RuntimeConfig, RuntimeCtx};
pub use winit::window::CursorIcon;