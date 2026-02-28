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

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect { Rect::new(x, y, w, h) }

    // ── normalized ────────────────────────────────────────────────────────

    #[test]
    fn normalized_positive_is_identity() {
        let rect = r(1.0, 2.0, 10.0, 20.0);
        assert_eq!(rect.normalized(), rect);
    }

    #[test]
    fn normalized_negative_width() {
        let rect = r(10.0, 0.0, -4.0, 5.0);
        let n = rect.normalized();
        assert_eq!(n.origin.x, 6.0);
        assert_eq!(n.size.x, 4.0);
    }

    #[test]
    fn normalized_negative_height() {
        let rect = r(0.0, 10.0, 5.0, -3.0);
        let n = rect.normalized();
        assert_eq!(n.origin.y, 7.0);
        assert_eq!(n.size.y, 3.0);
    }

    // ── contains ──────────────────────────────────────────────────────────

    #[test]
    fn contains_interior_point() {
        assert!(r(0.0, 0.0, 10.0, 10.0).contains(Vec2::new(5.0, 5.0)));
    }

    #[test]
    fn contains_top_left_inclusive() {
        assert!(r(0.0, 0.0, 10.0, 10.0).contains(Vec2::new(0.0, 0.0)));
    }

    #[test]
    fn contains_bottom_right_exclusive() {
        // Half-open [min, max) — the max edge is not contained.
        assert!(!r(0.0, 0.0, 10.0, 10.0).contains(Vec2::new(10.0, 10.0)));
    }

    #[test]
    fn contains_outside() {
        assert!(!r(0.0, 0.0, 10.0, 10.0).contains(Vec2::new(-1.0, 5.0)));
        assert!(!r(0.0, 0.0, 10.0, 10.0).contains(Vec2::new(5.0, -1.0)));
    }

    // ── intersect ─────────────────────────────────────────────────────────

    #[test]
    fn intersect_overlapping() {
        let a = r(0.0, 0.0, 10.0, 10.0);
        let b = r(5.0, 5.0, 10.0, 10.0);
        let i = a.intersect(b).unwrap();
        assert_eq!(i, r(5.0, 5.0, 5.0, 5.0));
    }

    #[test]
    fn intersect_contained() {
        let outer = r(0.0, 0.0, 100.0, 100.0);
        let inner = r(10.0, 10.0, 20.0, 20.0);
        assert_eq!(outer.intersect(inner).unwrap(), inner);
    }

    #[test]
    fn intersect_touching_edge_returns_none() {
        // Rects share an edge — zero-width overlap is not a valid intersection.
        let a = r(0.0, 0.0, 10.0, 10.0);
        let b = r(10.0, 0.0, 10.0, 10.0);
        assert!(a.intersect(b).is_none());
    }

    #[test]
    fn intersect_disjoint_returns_none() {
        let a = r(0.0, 0.0, 5.0, 5.0);
        let b = r(20.0, 20.0, 5.0, 5.0);
        assert!(a.intersect(b).is_none());
    }

    // ── is_empty ──────────────────────────────────────────────────────────

    #[test]
    fn is_empty_zero_size() {
        assert!(r(0.0, 0.0, 0.0, 5.0).is_empty());
        assert!(r(0.0, 0.0, 5.0, 0.0).is_empty());
    }

    #[test]
    fn is_empty_positive_size() {
        assert!(!r(0.0, 0.0, 1.0, 1.0).is_empty());
    }
}