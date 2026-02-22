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
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }
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