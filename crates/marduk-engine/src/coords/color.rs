/// Linear RGBA color.
///
/// Values are expected in linear space. sRGB conversion is handled by render targets
/// and/or shaders depending on pipeline policy.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct ColorRgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRgba {
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    #[inline]
    pub const fn black() -> Self {
        Self::new(0.0, 0.0, 0.0, 1.0)
    }

    #[inline]
    pub const fn white() -> Self {
        Self::new(1.0, 1.0, 1.0, 1.0)
    }

    #[inline]
    pub fn is_finite(self) -> bool {
        self.r.is_finite() && self.g.is_finite() && self.b.is_finite() && self.a.is_finite()
    }
}