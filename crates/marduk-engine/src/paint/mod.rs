//! Paint model shared between UI and renderers.
//!
//! Scope:
//! - color representation (linear premultiplied alpha)
//! - paint sources (solid, gradients)
//!
//! Geometry types remain in `coords`.

pub mod color;
pub mod gradient;

pub use color::Color;
pub use gradient::{ColorStop, LinearGradient, SpreadMode};


/// Paint source for filling geometry.
///
/// This is intentionally a small enum in v0. Extend by adding variants:
/// - `RadialGradient`
/// - `Image`
/// - `Pattern`
///
/// while keeping the enum stable for renderer dispatch.
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    Solid(Color),
    LinearGradient(LinearGradient),
}

impl Paint {
    #[inline]
    pub fn solid(color: Color) -> Self {
        Paint::Solid(color)
    }

    #[inline]
    pub fn is_opaque(&self) -> bool {
        match self {
            Paint::Solid(c) => c.a >= 1.0 && c.r <= 1.0 && c.g <= 1.0 && c.b <= 1.0,
            Paint::LinearGradient(g) => {
                // Conservative: treat gradients as potentially translucent unless proven otherwise.
                // Renderers can optimize this once gradient compilation exists.
                g.stops.iter().all(|s| s.color.a >= 1.0)
            }
        }
    }
}
