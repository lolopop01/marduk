use crate::coords::Vec2;

use super::Color;

/// Gradient spread behavior outside [0, 1] range.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SpreadMode {
    /// Clamp to edge stops.
    Pad,
    /// Repeat the gradient pattern.
    Repeat,
    /// Mirror-repeat the gradient pattern.
    Reflect,
}

/// A single gradient stop.
///
/// `t` is expected in [0, 1] in typical usage, but is not strictly enforced.
/// Renderers may clamp/sort stops at build time.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ColorStop {
    pub t: f32,
    pub color: Color,
}

impl ColorStop {
    #[inline]
    pub const fn new(t: f32, color: Color) -> Self {
        Self { t, color }
    }
}

/// Linear gradient definition in logical pixel space.
///
/// Semantics:
/// - `start` and `end` are positions in the same coordinate space as geometry.
/// - Stops define premultiplied linear colors.
/// - `spread` defines out-of-range behavior.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearGradient {
    pub start: Vec2,
    pub end: Vec2,
    pub stops: Vec<ColorStop>,
    pub spread: SpreadMode,
}

impl LinearGradient {
    pub fn new(start: Vec2, end: Vec2, stops: Vec<ColorStop>, spread: SpreadMode) -> Self {
        Self {
            start,
            end,
            stops,
            spread,
        }
    }

    /// Returns true when the gradient definition is structurally usable.
    ///
    /// Renderers may still impose additional constraints (minimum number of stops, sorting, etc.).
    pub fn is_valid(&self) -> bool {
        self.start.is_finite()
            && self.end.is_finite()
            && self.stops.iter().all(|s| s.t.is_finite() && s.color.is_finite())
            && self.stops.len() >= 2
            && (self.end.x != self.start.x || self.end.y != self.start.y)
    }
}