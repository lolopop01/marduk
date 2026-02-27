use marduk_engine::coords::{Rect, Vec2};

use crate::constraints::{inset_rect, Constraints, Edges, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::{Element, Widget};

// ── Align ─────────────────────────────────────────────────────────────────

/// Cross-axis alignment inside a flex container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// Children fill the full cross-axis extent (default).
    #[default]
    Stretch,
    /// Children are placed at the start of the cross axis.
    Start,
    /// Children are centered on the cross axis.
    Center,
    /// Children are placed at the end of the cross axis.
    End,
}

// ── Column ────────────────────────────────────────────────────────────────

/// Vertical flex container. Children are stacked top to bottom.
///
/// # Example
/// ```rust,ignore
/// Column::new()
///     .padding_all(16.0)
///     .spacing(8.0)
///     .child(Text::new("Title", font, 20.0, white))
///     .child(Text::new("Body",  font, 14.0, grey))
/// ```
pub struct Column {
    children: Vec<Element>,
    spacing: f32,
    padding: Edges,
    cross_align: Align,
}

impl Column {
    pub fn new() -> Self {
        Self { children: Vec::new(), spacing: 0.0, padding: Edges::default(), cross_align: Align::Stretch }
    }

    pub fn spacing(mut self, v: f32) -> Self {
        self.spacing = v;
        self
    }

    pub fn padding(mut self, edges: Edges) -> Self {
        self.padding = edges;
        self
    }

    pub fn padding_all(mut self, v: f32) -> Self {
        self.padding = Edges::all(v);
        self
    }

    pub fn cross_align(mut self, align: Align) -> Self {
        self.cross_align = align;
        self
    }

    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.children.push(child.into());
        self
    }

    pub fn children(mut self, iter: impl IntoIterator<Item = impl Into<Element>>) -> Self {
        self.children.extend(iter.into_iter().map(Into::into));
        self
    }

    // ── layout helpers ────────────────────────────────────────────────────

    fn inner_width(&self, available: f32) -> f32 {
        (available - self.padding.h()).max(0.0)
    }

    fn child_constraints(&self, inner_w: f32) -> Constraints {
        match self.cross_align {
            Align::Stretch => {
                // Only enforce the width when it is actually constrained.
                // When inner_w is INFINITY (e.g. Column inside an unconstrained Row)
                // children should size naturally, not to ∞.
                let min_x = if inner_w.is_finite() { inner_w } else { 0.0 };
                Constraints {
                    min: Vec2::new(min_x, 0.0),
                    max: Vec2::new(inner_w, f32::INFINITY),
                }
            }
            _ => Constraints::loose(Vec2::new(inner_w, f32::INFINITY)),
        }
    }

    fn child_x(&self, inner_origin_x: f32, inner_w: f32, child_w: f32) -> f32 {
        match self.cross_align {
            Align::Stretch | Align::Start => inner_origin_x,
            Align::Center => inner_origin_x + (inner_w - child_w) * 0.5,
            Align::End => inner_origin_x + (inner_w - child_w),
        }
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Column {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        let inner_w = self.inner_width(constraints.max.x);
        let child_c = self.child_constraints(inner_w);

        let mut total_h = self.padding.v();
        let mut max_child_w: f32 = 0.0;

        for (i, child) in self.children.iter().enumerate() {
            let s = child.measure(child_c, ctx);
            total_h += s.y;
            if i + 1 < self.children.len() {
                total_h += self.spacing;
            }
            max_child_w = max_child_w.max(s.x);
        }

        let w = match self.cross_align {
            // Symmetric to the Row height fix: only fill available width when
            // it is actually constrained (e.g. Column inside an unconstrained Row
            // should report content width, not ∞).
            Align::Stretch => {
                if constraints.max.x.is_finite() { constraints.max.x }
                else { (max_child_w + self.padding.h()).max(0.0) }
            }
            _ => (max_child_w + self.padding.h()).max(0.0),
        };

        constraints.constrain(Vec2::new(w, total_h))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        // Copy the font_system reference out of painter first.
        // `&FontSystem` is Copy so this ends the borrow on `painter`,
        // letting us pass `painter` mutably to child.paint() in the loop.
        let fonts = painter.font_system;
        let ctx = LayoutCtx { fonts };

        let inner = inset_rect(rect, self.padding);
        let child_c = self.child_constraints(inner.size.x);

        let mut y = inner.origin.y;
        for (i, child) in self.children.iter().enumerate() {
            let s = child.measure(child_c, &ctx);
            let x = self.child_x(inner.origin.x, inner.size.x, s.x);
            child.paint(painter, Rect::new(x, y, s.x, s.y));
            y += s.y;
            if i + 1 < self.children.len() {
                y += self.spacing;
            }
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        let inner       = inset_rect(rect, self.padding);
        let child_c     = self.child_constraints(inner.size.x);
        let cross_align = self.cross_align;
        let spacing     = self.spacing;
        let n           = self.children.len();

        let mut y = inner.origin.y;
        for (i, child) in self.children.iter_mut().enumerate() {
            let s = child.measure(child_c, ctx);
            let x = match cross_align {
                Align::Stretch | Align::Start => inner.origin.x,
                Align::Center => inner.origin.x + (inner.size.x - s.x) * 0.5,
                Align::End    => inner.origin.x + (inner.size.x - s.x),
            };
            let child_rect = Rect::new(x, y, s.x, s.y);
            if child.on_event(event, child_rect, ctx).is_consumed() {
                return EventResult::Consumed;
            }
            y += s.y;
            if i + 1 < n {
                y += spacing;
            }
        }
        EventResult::Ignored
    }
}

// ── Row ───────────────────────────────────────────────────────────────────

/// Horizontal flex container. Children are placed left to right.
///
/// # Example
/// ```rust,ignore
/// Row::new()
///     .spacing(8.0)
///     .child(icon_widget)
///     .child(Text::new("Label", font, 14.0, white))
/// ```
pub struct Row {
    children: Vec<Element>,
    spacing: f32,
    padding: Edges,
    cross_align: Align,
}

impl Row {
    pub fn new() -> Self {
        Self { children: Vec::new(), spacing: 0.0, padding: Edges::default(), cross_align: Align::Stretch }
    }

    pub fn spacing(mut self, v: f32) -> Self {
        self.spacing = v;
        self
    }

    pub fn padding(mut self, edges: Edges) -> Self {
        self.padding = edges;
        self
    }

    pub fn padding_all(mut self, v: f32) -> Self {
        self.padding = Edges::all(v);
        self
    }

    pub fn cross_align(mut self, align: Align) -> Self {
        self.cross_align = align;
        self
    }

    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.children.push(child.into());
        self
    }

    pub fn children(mut self, iter: impl IntoIterator<Item = impl Into<Element>>) -> Self {
        self.children.extend(iter.into_iter().map(Into::into));
        self
    }

    // ── layout helpers ────────────────────────────────────────────────────

    fn inner_height(&self, available: f32) -> f32 {
        (available - self.padding.v()).max(0.0)
    }

    fn child_constraints(&self, inner_h: f32) -> Constraints {
        match self.cross_align {
            Align::Stretch => {
                // Only enforce the height when it is finite (the Row has a known
                // height to fill).  When inner_h is INFINITY the row is inside an
                // unconstrained column — children should size naturally, not to ∞.
                let min_h = if inner_h.is_finite() { inner_h } else { 0.0 };
                Constraints {
                    min: Vec2::new(0.0, min_h),
                    max: Vec2::new(f32::INFINITY, inner_h),
                }
            }
            _ => Constraints::loose(Vec2::new(f32::INFINITY, inner_h)),
        }
    }

    fn child_y(&self, inner_origin_y: f32, inner_h: f32, child_h: f32) -> f32 {
        match self.cross_align {
            Align::Stretch | Align::Start => inner_origin_y,
            Align::Center => inner_origin_y + (inner_h - child_h) * 0.5,
            Align::End => inner_origin_y + (inner_h - child_h),
        }
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Row {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        let inner_h = self.inner_height(constraints.max.y);
        let child_c = self.child_constraints(inner_h);

        let sizes: Vec<Vec2> = self.children.iter().map(|c| c.measure(child_c, ctx)).collect();
        let spacer_count = sizes.iter().filter(|s| s.x == 0.0 && s.y == 0.0).count();

        let fixed_w: f32     = sizes.iter().map(|s| s.x).sum();
        let max_child_h: f32 = sizes.iter().map(|s| s.y).fold(0.0f32, f32::max);
        let spacing_total    = (self.children.len().saturating_sub(1)) as f32 * self.spacing;

        // When spacers are present and width is bounded, they fill remaining space.
        let total_w = if spacer_count > 0 && constraints.max.x.is_finite() {
            constraints.max.x
        } else {
            fixed_w + spacing_total + self.padding.h()
        };

        let h = match self.cross_align {
            // Only fill the available height when it is actually constrained.
            // When max.y is INFINITY (e.g. Row inside an unconstrained Column)
            // fall back to content height so we don't report a giant size.
            Align::Stretch => {
                if constraints.max.y.is_finite() { constraints.max.y }
                else { (max_child_h + self.padding.v()).max(0.0) }
            }
            _ => (max_child_h + self.padding.v()).max(0.0),
        };

        constraints.constrain(Vec2::new(total_w, h))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let fonts = painter.font_system;
        let ctx = LayoutCtx { fonts };

        let inner   = inset_rect(rect, self.padding);
        let child_c = self.child_constraints(inner.size.y);

        // First pass: natural sizes.
        let mut sizes: Vec<Vec2> = self.children.iter().map(|c| c.measure(child_c, &ctx)).collect();

        // Distribute remaining width to zero-sized spacer children.
        let spacer_count = sizes.iter().filter(|s| s.x == 0.0 && s.y == 0.0).count();
        if spacer_count > 0 {
            let fixed_w: f32  = sizes.iter().map(|s| s.x).sum();
            let spacing_total = (self.children.len().saturating_sub(1)) as f32 * self.spacing;
            let remaining     = (inner.size.x - fixed_w - spacing_total).max(0.0);
            let spacer_w      = remaining / spacer_count as f32;
            for s in sizes.iter_mut() {
                if s.x == 0.0 && s.y == 0.0 {
                    s.x = spacer_w;
                }
            }
        }

        let mut x = inner.origin.x;
        for (i, (child, s)) in self.children.iter().zip(sizes.iter()).enumerate() {
            let y = self.child_y(inner.origin.y, inner.size.y, s.y);
            child.paint(painter, Rect::new(x, y, s.x, s.y));
            x += s.x;
            if i + 1 < self.children.len() {
                x += self.spacing;
            }
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        let inner       = inset_rect(rect, self.padding);
        let child_c     = self.child_constraints(inner.size.y);
        let cross_align = self.cross_align;
        let spacing     = self.spacing;
        let n           = self.children.len();

        // Measure pass (immutable) to get each child's size.
        let mut sizes: Vec<Vec2> = self.children.iter().map(|c| c.measure(child_c, ctx)).collect();

        // Distribute remaining width to spacer (zero-sized) children.
        let spacer_count = sizes.iter().filter(|s| s.x == 0.0 && s.y == 0.0).count();
        if spacer_count > 0 {
            let fixed_w: f32  = sizes.iter().map(|s| s.x).sum();
            let spacing_total = (n.saturating_sub(1)) as f32 * spacing;
            let remaining     = (inner.size.x - fixed_w - spacing_total).max(0.0);
            let spacer_w      = remaining / spacer_count as f32;
            for s in sizes.iter_mut() {
                if s.x == 0.0 && s.y == 0.0 {
                    s.x = spacer_w;
                }
            }
        }

        // Event-routing pass (mutable).
        let mut x = inner.origin.x;
        for (i, (child, s)) in self.children.iter_mut().zip(sizes.iter()).enumerate() {
            let y = match cross_align {
                Align::Stretch | Align::Start => inner.origin.y,
                Align::Center => inner.origin.y + (inner.size.y - s.y) * 0.5,
                Align::End    => inner.origin.y + (inner.size.y - s.y),
            };
            let child_rect = Rect::new(x, y, s.x, s.y);
            if child.on_event(event, child_rect, ctx).is_consumed() {
                return EventResult::Consumed;
            }
            x += s.x;
            if i + 1 < n {
                x += spacing;
            }
        }
        EventResult::Ignored
    }
}
