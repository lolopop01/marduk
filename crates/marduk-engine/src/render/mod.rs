//! GPU rendering subsystem.
//!
//! Renderers consume `scene` draw streams and issue GPU commands via wgpu.
//! Each renderer is responsible for its own GPU resources (pipelines, buffers).
//!
//! Convention:
//! - CPU geometry is in logical pixels (top-left origin, +Y down).
//! - Vertex shader converts to NDC using a viewport uniform.

mod ctx;
pub mod shapes;

pub use ctx::{RenderCtx, RenderTarget};