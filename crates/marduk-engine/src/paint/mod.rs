//! Paint model shared between UI and renderers.
//!
//! Scope:
//! - color representation (linear premultiplied alpha)
//! - paint sources (solid, gradients)
//!
//! Geometry types remain in `coords`.

pub mod color;
pub mod paint;
pub mod gradient;

pub use color::Color;

pub use paint::Paint;
