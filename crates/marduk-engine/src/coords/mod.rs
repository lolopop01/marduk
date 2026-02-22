//! Coordinate and geometry types shared across engine renderers and UI.
//!
//! Canonical CPU space:
//! - Logical pixels (DPI-aware)
//! - Origin top-left
//! - +X right, +Y down
//!
//! Renderers convert to NDC in shaders using a viewport uniform.

mod color;
mod rect;
mod vec2;
mod viewport;

pub use color::ColorRgba;
pub use rect::Rect;
pub use vec2::Vec2;
pub use viewport::Viewport;
