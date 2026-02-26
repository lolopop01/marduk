pub(crate) mod circle;
pub(crate) mod rect;
pub(crate) mod rounded_rect;
pub(crate) mod text;

use crate::paint::Color;

/// Stroke drawn along the outer edge of a shape.
#[derive(Debug, Clone, PartialEq)]
pub struct Border {
    pub width: f32,
    pub color: Color,
}

impl Border {
    #[inline]
    pub fn new(width: f32, color: Color) -> Self {
        Self { width, color }
    }
}
