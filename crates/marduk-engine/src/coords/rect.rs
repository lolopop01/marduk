use super::Vec2;

/// Axis-aligned rectangle in logical pixels (top-left origin).
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct Rect {
    pub origin: Vec2,
    pub size: Vec2,
}

impl Rect {
    #[inline]
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            origin: Vec2::new(x, y),
            size: Vec2::new(w, h),
        }
    }

    #[inline]
    pub const fn from_origin_size(origin: Vec2, size: Vec2) -> Self {
        Self { origin, size }
    }

    #[inline]
    pub fn min(self) -> Vec2 {
        self.origin
    }

    #[inline]
    pub fn max(self) -> Vec2 {
        Vec2::new(self.origin.x + self.size.x, self.origin.y + self.size.y)
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.size.x <= 0.0 || self.size.y <= 0.0
    }

    #[inline]
    pub fn is_finite(self) -> bool {
        self.origin.is_finite() && self.size.is_finite()
    }

    /// Normalizes the rectangle so width/height are non-negative.
    #[inline]
    pub fn normalized(self) -> Self {
        let mut x = self.origin.x;
        let mut y = self.origin.y;
        let mut w = self.size.x;
        let mut h = self.size.y;

        if w < 0.0 {
            x += w;
            w = -w;
        }
        if h < 0.0 {
            y += h;
            h = -h;
        }

        Rect::new(x, y, w, h)
    }

    /// Half-open containment: [min, max).
    #[inline]
    pub fn contains(self, p: Vec2) -> bool {
        let r = self.normalized();
        p.x >= r.origin.x
            && p.y >= r.origin.y
            && p.x < (r.origin.x + r.size.x)
            && p.y < (r.origin.y + r.size.y)
    }

    #[inline]
    pub fn intersect(self, other: Rect) -> Option<Rect> {
        let a = self.normalized();
        let b = other.normalized();

        let x0 = a.origin.x.max(b.origin.x);
        let y0 = a.origin.y.max(b.origin.y);
        let x1 = (a.origin.x + a.size.x).min(b.origin.x + b.size.x);
        let y1 = (a.origin.y + a.size.y).min(b.origin.y + b.size.y);

        let w = x1 - x0;
        let h = y1 - y0;

        if w <= 0.0 || h <= 0.0 {
            None
        } else {
            Some(Rect::new(x0, y0, w, h))
        }
    }
}