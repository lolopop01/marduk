/// Linear premultiplied RGBA color.
///
/// Invariant:
/// - `rgb` components are expected to be multiplied by `a` (premultiplied alpha).
///
/// Rationale:
/// - Correct blending with linear filtering (avoids fringes).
/// - Matches typical GPU blending configurations for UI compositing.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct Color {
    pub r: f32, // premultiplied
    pub g: f32, // premultiplied
    pub b: f32, // premultiplied
    pub a: f32,
}

impl Color {
    #[inline]
    pub const fn transparent() -> Self {
        Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }
    }

    /// Creates a premultiplied color from straight sRGB bytes (`0`–`255`).
    ///
    /// This is the preferred constructor for colors coming from hex literals or
    /// the `.mkml` DSL parser, which produce `[u8; 4]` straight-alpha RGBA.
    #[inline]
    pub fn from_srgb_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::from_srgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0)
    }

    /// Creates a premultiplied color from straight sRGB `f32` components in `[0, 1]`.
    ///
    /// Clearer alternative to [`from_straight`](Self::from_straight), which is kept
    /// for backwards compatibility.
    #[inline]
    pub fn from_srgb(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::from_straight(r, g, b, a)
    }

    /// Debug-only validation: asserts that RGB channels do not exceed alpha,
    /// which would indicate a straight-alpha color was passed where premul was expected.
    ///
    /// No-op in release builds.
    #[inline]
    pub fn debug_assert_premul(self) {
        debug_assert!(
            self.r <= self.a + f32::EPSILON,
            "Color::debug_assert_premul: r ({}) > a ({}), looks like straight-alpha was passed as premul",
            self.r, self.a
        );
        debug_assert!(
            self.g <= self.a + f32::EPSILON,
            "Color::debug_assert_premul: g ({}) > a ({}), looks like straight-alpha was passed as premul",
            self.g, self.a
        );
        debug_assert!(
            self.b <= self.a + f32::EPSILON,
            "Color::debug_assert_premul: b ({}) > a ({}), looks like straight-alpha was passed as premul",
            self.b, self.a
        );
    }

    /// Creates a premultiplied color from premultiplied components.
    #[inline]
    pub const fn from_premul(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates a premultiplied color from straight alpha components.
    #[inline]
    pub fn from_straight(r: f32, g: f32, b: f32, a: f32) -> Self {
        let a = a.clamp(0.0, 1.0);
        Self {
            r: (r.clamp(0.0, 1.0)) * a,
            g: (g.clamp(0.0, 1.0)) * a,
            b: (b.clamp(0.0, 1.0)) * a,
            a,
        }
    }

    /// Returns a straight-alpha representation.
    ///
    /// For `a == 0`, RGB is returned as 0.
    #[inline]
    pub fn to_straight(self) -> (f32, f32, f32, f32) {
        if self.a <= 0.0 {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            let inv = 1.0 / self.a;
            (self.r * inv, self.g * inv, self.b * inv, self.a)
        }
    }

    #[inline]
    pub fn is_finite(self) -> bool {
        self.r.is_finite() && self.g.is_finite() && self.b.is_finite() && self.a.is_finite()
    }

    /// Clamps all channels to [0, 1] and enforces premultiplication.
    ///
    /// This is intended for debug validation and user-provided inputs.
    #[inline]
    pub fn clamped(self) -> Self {
        let a = self.a.clamp(0.0, 1.0);

        // Clamp premultiplied rgb so it cannot exceed alpha.
        let r = self.r.clamp(0.0, a);
        let g = self.g.clamp(0.0, a);
        let b = self.b.clamp(0.0, a);

        Self { r, g, b, a }
    }
}