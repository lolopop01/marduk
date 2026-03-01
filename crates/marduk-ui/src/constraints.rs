use std::cell::RefCell;
use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::text::FontSystem;

use crate::focus::{FocusId, FocusManager};

// ── Edges ─────────────────────────────────────────────────────────────────

/// Insets on all four sides (padding, margin, border).
#[derive(Debug, Clone, Copy, Default)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    #[inline]
    pub fn all(v: f32) -> Self {
        Self { top: v, right: v, bottom: v, left: v }
    }

    #[inline]
    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self { top: vertical, bottom: vertical, left: horizontal, right: horizontal }
    }

    #[inline]
    pub fn horizontal(v: f32) -> Self {
        Self { left: v, right: v, ..Self::default() }
    }

    #[inline]
    pub fn vertical(v: f32) -> Self {
        Self { top: v, bottom: v, ..Self::default() }
    }

    /// Total inset on the horizontal axis.
    #[inline]
    pub fn h(self) -> f32 {
        self.left + self.right
    }

    /// Total inset on the vertical axis.
    #[inline]
    pub fn v(self) -> f32 {
        self.top + self.bottom
    }
}

// ── Constraints ───────────────────────────────────────────────────────────

/// Layout constraints passed down from parent to child during measure.
///
/// A child may return any size in `[min, max]`. Parents enforce their own
/// policy by calling [`Constraints::constrain`] on the returned size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Constraints {
    pub min: Vec2,
    pub max: Vec2,
}

impl Constraints {
    /// Tight: child must be exactly `size`.
    #[inline]
    pub fn tight(size: Vec2) -> Self {
        Self { min: size, max: size }
    }

    /// Loose: child can be anywhere from zero up to `max`.
    #[inline]
    pub fn loose(max: Vec2) -> Self {
        Self { min: Vec2::zero(), max }
    }

    /// No constraint: child can take any positive size.
    #[inline]
    pub fn unbounded() -> Self {
        Self { min: Vec2::zero(), max: Vec2::new(f32::INFINITY, f32::INFINITY) }
    }

    /// Clamp a size into `[min, max]`.
    #[inline]
    #[must_use]
    pub fn constrain(self, size: Vec2) -> Vec2 {
        Vec2::new(
            size.x.max(self.min.x).min(self.max.x),
            size.y.max(self.min.y).min(self.max.y),
        )
    }

    /// Shrink max inward by `edges` (for padding). Min becomes zero.
    #[inline]
    #[must_use]
    pub fn shrink(self, edges: Edges) -> Self {
        Self {
            min: Vec2::zero(),
            max: Vec2::new(
                (self.max.x - edges.h()).max(0.0),
                (self.max.y - edges.v()).max(0.0),
            ),
        }
    }

    /// Exact-size constraint: child must be exactly `size` (alias for [`tight`](Self::tight)).
    #[inline]
    pub fn fixed(size: Vec2) -> Self {
        Self::tight(size)
    }

    /// Min-only constraint: child must be at least `min`; no upper bound.
    #[inline]
    pub fn at_least(min: Vec2) -> Self {
        Self { min, max: Vec2::new(f32::INFINITY, f32::INFINITY) }
    }

    /// Explicit range: child can be any size in `[min, max]`.
    #[inline]
    pub fn between(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Replace the height constraint with `f32::INFINITY` (used by flex containers).
    #[inline]
    pub fn with_infinite_height(self) -> Self {
        Self { max: Vec2::new(self.max.x, f32::INFINITY), ..self }
    }

    /// Replace the width constraint with `f32::INFINITY`.
    #[inline]
    pub fn with_infinite_width(self) -> Self {
        Self { max: Vec2::new(f32::INFINITY, self.max.y), ..self }
    }
}

// ── LayoutCtx ────────────────────────────────────────────────────────────

/// Resources made available to [`Widget::measure`] and [`Widget::on_event`].
///
/// Passed down through the widget tree so any widget can measure text and
/// interact with the focus system without owning those resources.
pub struct LayoutCtx<'a> {
    pub fonts: &'a FontSystem,
    /// Physical-to-logical pixel ratio (os_scale × zoom), matching the text
    /// renderer's `raster_scale`.  Pass this to `fonts.measure_text_scaled` so
    /// that measured widths exactly match what the renderer will draw.
    pub scale: f32,
    /// Focus manager, available during event routing so widgets can request focus.
    ///
    /// `None` in contexts that do not support focus (custom renderers, tests).
    pub focus: Option<&'a RefCell<FocusManager>>,
}

impl<'a> LayoutCtx<'a> {
    /// Request that `id` becomes the focused widget.
    ///
    /// Call this from [`Widget::on_event`] when the widget receives a `Click`
    /// or other activation event. The focus change takes effect at end of frame.
    #[inline]
    pub fn request_focus(&self, id: FocusId) {
        if let Some(fm) = self.focus {
            fm.borrow_mut().request_focus(id);
        }
    }

    /// Returns `true` if `id` is currently focused.
    #[inline]
    pub fn is_focused(&self, id: FocusId) -> bool {
        self.focus.is_some_and(|fm| fm.borrow().is_focused(id))
    }
}

// ── rect helper ──────────────────────────────────────────────────────────

/// Shrink a rect by `edges` (padding/inset).
#[inline]
pub fn inset_rect(rect: Rect, edges: Edges) -> Rect {
    Rect::new(
        rect.origin.x + edges.left,
        rect.origin.y + edges.top,
        (rect.size.x - edges.h()).max(0.0),
        (rect.size.y - edges.v()).max(0.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use marduk_engine::coords::Rect;

    // ── Constraints::constrain ────────────────────────────────────────────

    #[test]
    fn constrain_clamps_below_min() {
        let c = Constraints { min: Vec2::new(10.0, 10.0), max: Vec2::new(100.0, 100.0) };
        let out = c.constrain(Vec2::new(5.0, 3.0));
        assert_eq!(out.x, 10.0);
        assert_eq!(out.y, 10.0);
    }

    #[test]
    fn constrain_clamps_above_max() {
        let c = Constraints::loose(Vec2::new(50.0, 50.0));
        let out = c.constrain(Vec2::new(200.0, 200.0));
        assert_eq!(out.x, 50.0);
        assert_eq!(out.y, 50.0);
    }

    #[test]
    fn constrain_inside_range_unchanged() {
        let c = Constraints { min: Vec2::new(5.0, 5.0), max: Vec2::new(50.0, 50.0) };
        let v = Vec2::new(20.0, 30.0);
        assert_eq!(c.constrain(v), v);
    }

    // ── Constraints::shrink ───────────────────────────────────────────────

    #[test]
    fn shrink_reduces_max() {
        let c = Constraints::loose(Vec2::new(100.0, 80.0));
        let s = c.shrink(Edges::all(10.0));
        assert_eq!(s.max.x, 80.0);
        assert_eq!(s.max.y, 60.0);
    }

    #[test]
    fn shrink_clamps_to_zero() {
        let c = Constraints::loose(Vec2::new(5.0, 5.0));
        let s = c.shrink(Edges::all(20.0));
        assert_eq!(s.max.x, 0.0);
        assert_eq!(s.max.y, 0.0);
    }

    // ── inset_rect ────────────────────────────────────────────────────────

    #[test]
    fn inset_rect_uniform_padding() {
        let rect = Rect::new(0.0, 0.0, 100.0, 80.0);
        let inner = inset_rect(rect, Edges::all(10.0));
        assert_eq!(inner.origin.x, 10.0);
        assert_eq!(inner.origin.y, 10.0);
        assert_eq!(inner.size.x, 80.0);
        assert_eq!(inner.size.y, 60.0);
    }

    #[test]
    fn inset_rect_asymmetric_padding() {
        let rect = Rect::new(5.0, 5.0, 100.0, 60.0);
        let edges = Edges { top: 4.0, bottom: 8.0, left: 6.0, right: 10.0 };
        let inner = inset_rect(rect, edges);
        assert_eq!(inner.origin.x, 11.0); // 5 + 6
        assert_eq!(inner.origin.y, 9.0);  // 5 + 4
        assert_eq!(inner.size.x, 84.0);   // 100 - 6 - 10
        assert_eq!(inner.size.y, 48.0);   // 60 - 4 - 8
    }

    #[test]
    fn inset_rect_clamps_to_zero() {
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let inner = inset_rect(rect, Edges::all(20.0));
        assert_eq!(inner.size.x, 0.0);
        assert_eq!(inner.size.y, 0.0);
    }

    // ── Edges helpers ─────────────────────────────────────────────────────

    #[test]
    fn edges_h_and_v() {
        let e = Edges::symmetric(4.0, 8.0);
        assert_eq!(e.h(), 16.0); // left + right
        assert_eq!(e.v(), 8.0);  // top + bottom
    }
}
