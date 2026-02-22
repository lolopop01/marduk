/// Viewport size in logical pixels.
///
/// Renderers should treat this as the coordinate basis for converting logical px
/// positions to NDC in shaders.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
}

impl Viewport {
    #[inline]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self.width > 0.0 && self.height > 0.0 && self.width.is_finite() && self.height.is_finite()
    }
}