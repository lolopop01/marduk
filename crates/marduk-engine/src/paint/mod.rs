//! Paint model shared between UI and renderers.
//!
//! Scope:
//! - color representation (linear premultiplied alpha)
//! - paint sources (solid, gradients)
//!
//! Geometry types remain in `coords`.

mod color;
mod paint;
pub mod gradient;

pub use color::Color;
