/// Per-corner radii for a rounded rectangle (logical pixels).
///
/// Corners follow CSS convention: top-left, top-right, bottom-right, bottom-left.
/// Negative values are treated as zero by renderers.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    #[inline]
    pub const fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self { top_left, top_right, bottom_right, bottom_left }
    }

    /// Uniform radius on all four corners.
    #[inline]
    pub const fn all(r: f32) -> Self {
        Self { top_left: r, top_right: r, bottom_right: r, bottom_left: r }
    }

    /// No rounding.
    #[inline]
    pub const fn zero() -> Self {
        Self::all(0.0)
    }
}
