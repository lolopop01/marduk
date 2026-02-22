//! Window + runtime loop.
//!
//! Owns the `winit` EventLoop and Window, and wires them to the GPU layer.

mod runtime;

pub use runtime::{RunControl, Runtime, RuntimeConfig, RuntimeCtx};