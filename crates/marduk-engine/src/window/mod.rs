//! Window + runtime loop.
//!
//! Owns the `winit` EventLoop and Window, and wires them to the GPU layer.

mod runtime;

pub use runtime::{Runtime, RuntimeConfig, RuntimeCtx, WindowMode};
pub use winit::window::CursorIcon;