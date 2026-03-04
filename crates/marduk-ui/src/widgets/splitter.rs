use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::Color;

use crate::constraints::{Constraints, LayoutCtx};
use crate::cursor::CursorIcon;
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::{Element, Widget};

/// Direction of the split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    /// First pane on the left, second on the right.
    Horizontal,
    /// First pane on top, second on the bottom.
    Vertical,
}

/// A draggable divider between two panes.
///
/// The `ratio` is the fraction of available space given to the first pane
/// (0.0 = first pane collapsed, 1.0 = second pane collapsed).
///
/// # Example
/// ```rust,ignore
/// Splitter::horizontal(left_panel, right_panel)
///     .initial_ratio(0.3)
///     .min_first(120.0)
///     .min_second(200.0)
///     .on_change(|ratio| println!("split at {ratio:.2}"))
/// ```
pub struct Splitter {
    first: Element,
    second: Element,
    direction: SplitDirection,
    ratio: f32,
    min_first: f32,
    min_second: f32,
    handle_size: f32,
    on_change: Option<Box<dyn FnMut(f32)>>,
    /// True while a drag that started on the handle is in progress.
    /// In pure-Rust usage this persists on the struct; for DSL usage it must
    /// be restored each frame from widget_state via `initial_dragging`.
    dragging: bool,
    /// Called when the drag-lock changes (true = drag started, false = ended).
    /// The DSL builder uses this to persist `dragging` across frame rebuilds.
    on_drag_change: Option<Box<dyn FnMut(bool)>>,
}

impl Splitter {
    pub fn horizontal(first: impl Into<Element>, second: impl Into<Element>) -> Self {
        Self::new(first, second, SplitDirection::Horizontal)
    }

    pub fn vertical(first: impl Into<Element>, second: impl Into<Element>) -> Self {
        Self::new(first, second, SplitDirection::Vertical)
    }

    fn new(
        first: impl Into<Element>,
        second: impl Into<Element>,
        direction: SplitDirection,
    ) -> Self {
        Self {
            first: first.into(),
            second: second.into(),
            direction,
            ratio: 0.5,
            min_first: 0.0,
            min_second: 0.0,
            handle_size: 4.0,
            on_change: None,
            dragging: false,
            on_drag_change: None,
        }
    }

    pub fn initial_ratio(mut self, r: f32) -> Self {
        self.ratio = r.clamp(0.0, 1.0);
        self
    }

    /// Restore the drag-lock state (used by the DSL builder each frame).
    pub fn initial_dragging(mut self, dragging: bool) -> Self {
        self.dragging = dragging;
        self
    }

    pub fn min_first(mut self, px: f32) -> Self {
        self.min_first = px;
        self
    }

    pub fn min_second(mut self, px: f32) -> Self {
        self.min_second = px;
        self
    }

    pub fn handle_size(mut self, px: f32) -> Self {
        self.handle_size = px;
        self
    }

    pub fn on_change(mut self, f: impl FnMut(f32) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    /// Called with `true` when a handle drag begins, `false` when it ends.
    pub fn on_drag_change(mut self, f: impl FnMut(bool) + 'static) -> Self {
        self.on_drag_change = Some(Box::new(f));
        self
    }

    // ── layout helpers ────────────────────────────────────────────────────

    fn compute_rects(&self, rect: Rect) -> (Rect, Rect, Rect) {
        let hs = self.handle_size;
        match self.direction {
            SplitDirection::Horizontal => {
                let total_w = rect.size.x;
                let usable = (total_w - hs).max(0.0);
                let raw = total_w * self.ratio - hs * 0.5;
                let first_w = if usable > self.min_first + self.min_second {
                    raw.clamp(self.min_first, usable - self.min_second)
                } else {
                    raw.max(0.0)
                };
                let second_w = (total_w - first_w - hs).max(0.0);
                let first_rect  = Rect::new(rect.origin.x, rect.origin.y, first_w, rect.size.y);
                let handle_rect = Rect::new(rect.origin.x + first_w, rect.origin.y, hs, rect.size.y);
                let second_rect = Rect::new(rect.origin.x + first_w + hs, rect.origin.y, second_w, rect.size.y);
                (first_rect, handle_rect, second_rect)
            }
            SplitDirection::Vertical => {
                let total_h = rect.size.y;
                let usable = (total_h - hs).max(0.0);
                let raw = total_h * self.ratio - hs * 0.5;
                let first_h = if usable > self.min_first + self.min_second {
                    raw.clamp(self.min_first, usable - self.min_second)
                } else {
                    raw.max(0.0)
                };
                let second_h = (total_h - first_h - hs).max(0.0);
                let first_rect  = Rect::new(rect.origin.x, rect.origin.y, rect.size.x, first_h);
                let handle_rect = Rect::new(rect.origin.x, rect.origin.y + first_h, rect.size.x, hs);
                let second_rect = Rect::new(rect.origin.x, rect.origin.y + first_h + hs, rect.size.x, second_h);
                (first_rect, handle_rect, second_rect)
            }
        }
    }

    /// Returns a wider rect for drag/hover detection around the visual handle.
    ///
    /// The visual bar stays at `handle_size` pixels, but the interactive region
    /// extends by `GRAB_EXTRA` pixels on each perpendicular side so the divider
    /// is easy to grab without pixel-precise aiming.
    fn grab_rect(&self, handle_rect: Rect) -> Rect {
        const GRAB_EXTRA: f32 = 6.0;
        match self.direction {
            SplitDirection::Horizontal => Rect::new(
                handle_rect.origin.x - GRAB_EXTRA,
                handle_rect.origin.y,
                handle_rect.size.x + GRAB_EXTRA * 2.0,
                handle_rect.size.y,
            ),
            SplitDirection::Vertical => Rect::new(
                handle_rect.origin.x,
                handle_rect.origin.y - GRAB_EXTRA,
                handle_rect.size.x,
                handle_rect.size.y + GRAB_EXTRA * 2.0,
            ),
        }
    }

    fn ratio_from_drag(&self, rect: Rect, pos: Vec2) -> f32 {
        match self.direction {
            SplitDirection::Horizontal => {
                let usable = rect.size.x - self.handle_size;
                if usable <= 0.0 { return self.ratio; }
                let px = (pos.x - rect.origin.x).clamp(self.min_first, usable - self.min_second);
                (px + self.handle_size * 0.5) / rect.size.x
            }
            SplitDirection::Vertical => {
                let usable = rect.size.y - self.handle_size;
                if usable <= 0.0 { return self.ratio; }
                let py = (pos.y - rect.origin.y).clamp(self.min_first, usable - self.min_second);
                (py + self.handle_size * 0.5) / rect.size.y
            }
        }
    }
}

impl Widget for Splitter {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        let w = if constraints.max.x.is_finite() { constraints.max.x } else { 200.0 };
        let h = if constraints.max.y.is_finite() { constraints.max.y } else { 200.0 };
        Vec2::new(w, h)
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let (first_rect, handle_rect, second_rect) = self.compute_rects(rect);

        self.first.paint(painter, first_rect);
        self.second.paint(painter, second_rect);

        // Draw the handle on top of the children.
        // Hover detection uses the wider grab_rect so the highlight activates
        // before the cursor is precisely over the 4 px bar.
        let grab = self.grab_rect(handle_rect);
        let hovered = self.dragging || painter.is_hovered(grab);
        let handle_color = if hovered {
            Color::from_srgb(0.45, 0.45, 0.50, 1.0)
        } else {
            Color::from_srgb(0.25, 0.25, 0.28, 1.0)
        };
        painter.fill_rect(handle_rect, handle_color);
        if hovered {
            let cursor = match self.direction {
                SplitDirection::Horizontal => CursorIcon::EwResize,
                SplitDirection::Vertical   => CursorIcon::NsResize,
            };
            painter.set_cursor(cursor);
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        let (first_rect, handle_rect, second_rect) = self.compute_rects(rect);

        // The grab rect is computed from the CURRENT handle position. Once a
        // drag starts we latch `dragging = true` so we keep owning it even as
        // the handle moves away from the original click point. In DSL mode
        // `dragging` is restored each frame via `initial_dragging` + the
        // `on_drag_change` callback which persists it through widget_state.
        let grab = self.grab_rect(handle_rect);
        match event {
            UiEvent::Drag { pos, start } => {
                if self.dragging || grab.contains(*start) {
                    if !self.dragging {
                        self.dragging = true;
                        if let Some(f) = &mut self.on_drag_change { f(true); }
                    }
                    self.ratio = self.ratio_from_drag(rect, *pos).clamp(0.01, 0.99);
                    if let Some(f) = &mut self.on_change { f(self.ratio); }
                    return EventResult::Consumed;
                }
            }
            UiEvent::DragEnd { pos, start } => {
                if self.dragging || grab.contains(*start) {
                    self.dragging = false;
                    if let Some(f) = &mut self.on_drag_change { f(false); }
                    self.ratio = self.ratio_from_drag(rect, *pos).clamp(0.01, 0.99);
                    if let Some(f) = &mut self.on_change { f(self.ratio); }
                    return EventResult::Consumed;
                }
            }
            _ => {}
        }

        // Route other events to children based on mouse/event position.
        if self.first.on_event(event, first_rect, ctx).is_consumed() {
            return EventResult::Consumed;
        }
        self.second.on_event(event, second_rect, ctx)
    }
}
