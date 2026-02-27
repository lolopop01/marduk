use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::text::FontSystem;

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
#[derive(Debug, Clone, Copy)]
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
    pub fn constrain(self, size: Vec2) -> Vec2 {
        Vec2::new(
            size.x.max(self.min.x).min(self.max.x),
            size.y.max(self.min.y).min(self.max.y),
        )
    }

    /// Shrink max inward by `edges` (for padding). Min becomes zero.
    #[inline]
    pub fn shrink(self, edges: Edges) -> Self {
        Self {
            min: Vec2::zero(),
            max: Vec2::new(
                (self.max.x - edges.h()).max(0.0),
                (self.max.y - edges.v()).max(0.0),
            ),
        }
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

/// Resources made available to [`Widget::measure`].
///
/// Passed down through the widget tree so any widget can measure text without
/// owning the `FontSystem`.
pub struct LayoutCtx<'a> {
    pub fonts: &'a FontSystem,
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
