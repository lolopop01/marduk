use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::Color;

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::{Element, Widget};

// ── AnchorVal ─────────────────────────────────────────────────────────────

/// A distance from a parent edge, used to anchor a [`Stack`] child.
#[derive(Debug, Clone, Copy)]
pub enum AnchorVal {
    /// Fixed pixel distance from this edge.
    Px(f32),
    /// Fraction of the parent's dimension on this axis (0.0 = 0 %, 1.0 = 100 %).
    Pct(f32),
}

impl AnchorVal {
    #[inline]
    pub fn resolve(self, parent_dim: f32) -> f32 {
        match self {
            AnchorVal::Px(v) => v,
            AnchorVal::Pct(p) => parent_dim * p,
        }
    }
}

// ── SizeHint ──────────────────────────────────────────────────────────────

/// Controls how a [`Stack`] child's width or height is determined.
#[derive(Debug, Clone, Copy, Default)]
pub enum SizeHint {
    /// Use the child's measured natural size (default).
    #[default]
    Natural,
    /// Fixed pixel size.
    Px(f32),
    /// Fraction of the parent's dimension (0.0 = 0 %, 1.0 = 100 %).
    Pct(f32),
    /// Equal to the parent's dimension (equivalent to `Pct(1.0)`).
    Fill,
}

impl SizeHint {
    #[inline]
    pub fn resolve(self, parent_dim: f32, natural: f32) -> f32 {
        match self {
            SizeHint::Natural => natural,
            SizeHint::Px(v)   => v,
            SizeHint::Pct(p)  => parent_dim * p,
            SizeHint::Fill    => parent_dim,
        }
    }
}

// ── StackItem ─────────────────────────────────────────────────────────────

/// A child inside a [`Stack`] together with its anchor constraints.
///
/// Leave an anchor as `None` to let that axis be positioned at origin with
/// the child's natural or explicitly-given size.
///
/// | left  | right | outcome                                               |
/// |-------|-------|-------------------------------------------------------|
/// | Some  | Some  | width = parent_w – left – right  (stretches)         |
/// | Some  | None  | pinned to left edge; width from `SizeHint`           |
/// | None  | Some  | pinned to right edge; width from `SizeHint`          |
/// | None  | None  | x = 0; width from `SizeHint`                         |
///
/// (same table applies vertically with top / bottom / height)
pub struct StackItem {
    pub element: Element,
    pub left:    Option<AnchorVal>,
    pub top:     Option<AnchorVal>,
    pub right:   Option<AnchorVal>,
    pub bottom:  Option<AnchorVal>,
    pub width:   SizeHint,
    pub height:  SizeHint,
}

impl StackItem {
    pub fn new(element: impl Into<Element>) -> Self {
        Self {
            element: element.into(),
            left:    None,
            top:     None,
            right:   None,
            bottom:  None,
            width:   SizeHint::Natural,
            height:  SizeHint::Natural,
        }
    }

    pub fn left(mut self, v: AnchorVal) -> Self   { self.left   = Some(v); self }
    pub fn top(mut self, v: AnchorVal) -> Self    { self.top    = Some(v); self }
    pub fn right(mut self, v: AnchorVal) -> Self  { self.right  = Some(v); self }
    pub fn bottom(mut self, v: AnchorVal) -> Self { self.bottom = Some(v); self }
    pub fn width(mut self, v: SizeHint) -> Self   { self.width  = v;       self }
    pub fn height(mut self, v: SizeHint) -> Self  { self.height = v;       self }

    /// Compute this item's rect within `parent` using its anchor + size rules.
    pub fn compute_rect(&self, parent: Rect, ctx: &LayoutCtx) -> Rect {
        let pw = parent.size.x;
        let ph = parent.size.y;

        let natural = self.element.measure(Constraints::loose(Vec2::new(pw, ph)), ctx);

        // Width: stretch between both edges, or use SizeHint
        let w = match (self.left, self.right) {
            (Some(l), Some(r)) => (pw - l.resolve(pw) - r.resolve(pw)).max(0.0),
            _ => self.width.resolve(pw, natural.x),
        };

        // Height: same logic
        let h = match (self.top, self.bottom) {
            (Some(t), Some(b)) => (ph - t.resolve(ph) - b.resolve(ph)).max(0.0),
            _ => self.height.resolve(ph, natural.y),
        };

        // X position
        let x = match (self.left, self.right) {
            (Some(l), _)    => parent.origin.x + l.resolve(pw),
            (None, Some(r)) => parent.origin.x + pw - r.resolve(pw) - w,
            (None, None)    => parent.origin.x,
        };

        // Y position
        let y = match (self.top, self.bottom) {
            (Some(t), _)    => parent.origin.y + t.resolve(ph),
            (None, Some(b)) => parent.origin.y + ph - b.resolve(ph) - h,
            (None, None)    => parent.origin.y,
        };

        Rect::new(x, y, w, h)
    }
}

// ── Stack ─────────────────────────────────────────────────────────────────

/// An overlay container that positions each child using anchor constraints.
///
/// Children are painted in insertion order (first = bottom, last = top).
/// Events are routed in reverse order (topmost child gets first hit-test).
///
/// # Example
/// ```rust,ignore
/// Stack::new()
///     // Toolbar pinned to top, stretches full width
///     .item(StackItem::new(toolbar)
///         .top(AnchorVal::Px(0.0))
///         .left(AnchorVal::Px(0.0))
///         .right(AnchorVal::Px(0.0))
///         .height(SizeHint::Px(40.0)))
///     // Content fills the remaining area below the toolbar
///     .item(StackItem::new(content)
///         .top(AnchorVal::Px(40.0))
///         .left(AnchorVal::Px(0.0))
///         .right(AnchorVal::Px(0.0))
///         .bottom(AnchorVal::Px(0.0)))
/// ```
pub struct Stack {
    children: Vec<StackItem>,
    width:    SizeHint,
    height:   SizeHint,
    bg:       Option<Color>,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            width:    SizeHint::Fill,
            height:   SizeHint::Fill,
            bg:       None,
        }
    }

    pub fn item(mut self, item: StackItem) -> Self {
        self.children.push(item);
        self
    }

    pub fn width(mut self, v: SizeHint) -> Self  { self.width  = v; self }
    pub fn height(mut self, v: SizeHint) -> Self { self.height = v; self }
    pub fn bg(mut self, color: Color) -> Self    { self.bg = Some(color); self }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Stack {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        let pw = if constraints.max.x.is_finite() { constraints.max.x } else { 0.0 };
        let ph = if constraints.max.y.is_finite() { constraints.max.y } else { 0.0 };
        let child_c = Constraints::loose(Vec2::new(pw, ph));

        let natural_w = match self.width {
            SizeHint::Natural => self.children.iter()
                .map(|item| item.element.measure(child_c, ctx).x)
                .fold(0.0f32, f32::max),
            _ => 0.0,
        };

        let natural_h = match self.height {
            SizeHint::Natural => self.children.iter()
                .map(|item| item.element.measure(child_c, ctx).y)
                .fold(0.0f32, f32::max),
            _ => 0.0,
        };

        let w = self.width.resolve(pw, natural_w);
        let h = self.height.resolve(ph, natural_h);
        constraints.constrain(Vec2::new(w, h))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        if let Some(color) = self.bg {
            painter.fill_rect(rect, color);
        }

        // Copy the font reference and scale out so `painter` is free for child.paint() calls.
        let fonts = painter.font_system;
        let scale = painter.scale;
        let ctx = LayoutCtx { fonts, scale };

        for item in &self.children {
            let child_rect = item.compute_rect(rect, &ctx);
            item.element.paint(painter, child_rect);
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        // Topmost child (last in vec) gets first chance.
        for item in self.children.iter_mut().rev() {
            let child_rect = item.compute_rect(rect, ctx);
            if item.element.on_event(event, child_rect, ctx) == EventResult::Consumed {
                return EventResult::Consumed;
            }
        }
        EventResult::Ignored
    }
}
