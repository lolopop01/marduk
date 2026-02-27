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
            Align::Stretch => Constraints {
                min: Vec2::new(inner_w, 0.0),
                max: Vec2::new(inner_w, f32::INFINITY),
            },
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
            Align::Stretch => constraints.max.x,
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

    fn on_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        // Event routing without layout context: pass the full inner rect to
        // each child. Children that care about their exact rect (e.g. Button)
        // will still hit-test correctly when the tree is shallow and
        // non-overlapping. A future revision can thread LayoutCtx through here.
        let inner = inset_rect(rect, self.padding);
        for child in self.children.iter_mut() {
            if child.on_event(event, inner).is_consumed() {
                return EventResult::Consumed;
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
            Align::Stretch => Constraints {
                min: Vec2::new(0.0, inner_h),
                max: Vec2::new(f32::INFINITY, inner_h),
            },
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

        let mut total_w = self.padding.h();
        let mut max_child_h: f32 = 0.0;

        for (i, child) in self.children.iter().enumerate() {
            let s = child.measure(child_c, ctx);
            total_w += s.x;
            if i + 1 < self.children.len() {
                total_w += self.spacing;
            }
            max_child_h = max_child_h.max(s.y);
        }

        let h = match self.cross_align {
            Align::Stretch => constraints.max.y.min(f32::MAX),
            _ => (max_child_h + self.padding.v()).max(0.0),
        };

        // For Row, width is naturally sized (children side by side).
        let w = total_w;
        constraints.constrain(Vec2::new(w, h))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let fonts = painter.font_system;
        let ctx = LayoutCtx { fonts };

        let inner = inset_rect(rect, self.padding);
        let child_c = self.child_constraints(inner.size.y);

        let mut x = inner.origin.x;
        for (i, child) in self.children.iter().enumerate() {
            let s = child.measure(child_c, &ctx);
            let y = self.child_y(inner.origin.y, inner.size.y, s.y);
            child.paint(painter, Rect::new(x, y, s.x, s.y));
            x += s.x;
            if i + 1 < self.children.len() {
                x += self.spacing;
            }
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        let inner = inset_rect(rect, self.padding);
        for child in self.children.iter_mut() {
            if child.on_event(event, inner).is_consumed() {
                return EventResult::Consumed;
            }
        }
        EventResult::Ignored
    }
}
