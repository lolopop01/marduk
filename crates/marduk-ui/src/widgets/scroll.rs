use std::cell::Cell;

use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::input::Key;
use marduk_engine::paint::{Color, Paint};

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::{Element, Widget};

/// A scrollable single-child container.
///
/// Clips its child to the visible viewport and translates it by the current
/// scroll offset. Optionally draws a thin scrollbar on the right edge.
///
/// The child is measured at unbounded height so it can report its natural size;
/// the `ScrollView` itself takes all available height from its parent.
///
/// # Example
/// ```rust,ignore
/// ScrollView::new(
///     Column::new()
///         .child(Text::new("Item 1", font, 14.0, white))
///         .child(Text::new("Item 2", font, 14.0, white))
/// )
/// .line_height(28.0)
/// ```
pub struct ScrollView {
    child: Element,
    /// Current scroll offset in logical pixels (>=0, content shifted up by this amount).
    pub scroll_offset: f32,
    /// Pixels scrolled per line-delta unit.
    line_height: f32,
    /// Whether to draw the scrollbar thumb.
    show_scrollbar: bool,
    /// Cached content height from the most recent measure/paint pass.
    cached_content_height: Cell<f32>,
    /// Called when the scroll offset changes (for DSL state persistence).
    on_scroll: Option<Box<dyn FnMut(f32)>>,
}

impl ScrollView {
    pub fn new(child: impl Into<Element>) -> Self {
        Self {
            child: child.into(),
            scroll_offset: 0.0,
            line_height: 24.0,
            show_scrollbar: true,
            cached_content_height: Cell::new(0.0),
            on_scroll: None,
        }
    }

    pub fn line_height(mut self, v: f32) -> Self {
        self.line_height = v;
        self
    }

    pub fn show_scrollbar(mut self, v: bool) -> Self {
        self.show_scrollbar = v;
        self
    }

    pub fn on_scroll(mut self, f: impl FnMut(f32) + 'static) -> Self {
        self.on_scroll = Some(Box::new(f));
        self
    }

    pub fn scroll_to(mut self, offset: f32) -> Self {
        self.scroll_offset = offset.max(0.0);
        self
    }

    // ── helpers ───────────────────────────────────────────────────────────

    fn measure_content(&self, viewport_w: f32, ctx: &LayoutCtx) -> Vec2 {
        let c = Constraints::loose(Vec2::new(viewport_w, f32::INFINITY));
        self.child.measure(c, ctx)
    }

    fn clamped_offset(&self, content_h: f32, viewport_h: f32) -> f32 {
        let max = (content_h - viewport_h).max(0.0);
        self.scroll_offset.clamp(0.0, max)
    }

    fn content_rect(&self, rect: Rect, content_h: f32) -> Rect {
        let offset = self.clamped_offset(content_h, rect.size.y);
        Rect::new(rect.origin.x, rect.origin.y - offset, rect.size.x, content_h)
    }

    fn scrollbar_rects(&self, rect: Rect, content_h: f32) -> Option<(Rect, Rect)> {
        if !self.show_scrollbar || content_h <= rect.size.y {
            return None;
        }
        let bar_w: f32 = 6.0;
        let bar_x = rect.origin.x + rect.size.x - bar_w;

        // Track: full height of the viewport
        let track = Rect::new(bar_x, rect.origin.y, bar_w, rect.size.y);

        // Thumb: proportional to viewport / content ratio
        let ratio = rect.size.y / content_h;
        let thumb_h = (rect.size.y * ratio).max(24.0);
        let offset = self.clamped_offset(content_h, rect.size.y);
        let scroll_range = content_h - rect.size.y;
        let thumb_y = rect.origin.y + (offset / scroll_range) * (rect.size.y - thumb_h);

        let thumb = Rect::new(bar_x, thumb_y, bar_w, thumb_h);
        Some((track, thumb))
    }
}

impl Widget for ScrollView {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        let max_w = if constraints.max.x.is_finite() { constraints.max.x } else { 0.0 };
        let content = self.measure_content(max_w, ctx);
        self.cached_content_height.set(content.y);
        let h = if constraints.max.y.is_finite() {
            content.y.min(constraints.max.y)
        } else {
            content.y
        };
        Vec2::new(content.x.min(constraints.max.x.max(0.0)), h)
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        // Remeasure inside paint so layout is always fresh.
        let ctx = painter.layout_ctx();
        let content_h = self.measure_content(rect.size.x, &ctx).y;
        self.cached_content_height.set(content_h);

        let content_rect = self.content_rect(rect, content_h);

        // Clip child to viewport.
        painter.push_clip(rect);
        self.child.paint(painter, content_rect);
        painter.pop_clip();

        // Draw scrollbar on top of clip (outside clip so it's always visible).
        if let Some((track, thumb)) = self.scrollbar_rects(rect, content_h) {
            let track_color = Paint::Solid(Color::from_straight(0.15, 0.15, 0.15, 0.8));
            let thumb_color = Paint::Solid(Color::from_straight(0.55, 0.55, 0.55, 0.9));
            painter.fill_rounded_rect(track, 3.0, track_color, None);
            painter.fill_rounded_rect(thumb, 3.0, thumb_color, None);
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        let content_h = self.cached_content_height.get();

        match event {
            UiEvent::ScrollWheel { delta } => {
                // Positive delta = scroll down (increase offset to reveal content below).
                self.apply_scroll(*delta * self.line_height, content_h, rect.size.y);
                EventResult::Consumed
            }

            UiEvent::KeyPress { key, .. } => {
                // Route to child first — a focused TextBox should eat arrow keys.
                let content_rect = self.content_rect(rect, content_h);
                if self.child.on_event(event, content_rect, ctx) == EventResult::Consumed {
                    return EventResult::Consumed;
                }
                // Child didn't consume it — try keyboard scrolling.
                let page = rect.size.y * 0.9;
                match key {
                    Key::ArrowDown  => self.apply_scroll( self.line_height, content_h, rect.size.y),
                    Key::ArrowUp    => self.apply_scroll(-self.line_height, content_h, rect.size.y),
                    Key::PageDown   => self.apply_scroll( page,             content_h, rect.size.y),
                    Key::PageUp     => self.apply_scroll(-page,             content_h, rect.size.y),
                    Key::Home       => self.apply_scroll(f32::NEG_INFINITY, content_h, rect.size.y),
                    Key::End        => self.apply_scroll(f32::INFINITY,     content_h, rect.size.y),
                    _ => return EventResult::Ignored,
                }
                EventResult::Consumed
            }

            other => {
                // Route all other events to the child with an offset rect so
                // hit-testing uses content-space coordinates.
                let content_rect = self.content_rect(rect, content_h);
                self.child.on_event(other, content_rect, ctx)
            }
        }
    }

}

impl ScrollView {
    fn apply_scroll(&mut self, delta: f32, content_h: f32, viewport_h: f32) {
        let max = (content_h - viewport_h).max(0.0);
        let prev = self.scroll_offset;
        self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max);
        if self.scroll_offset != prev {
            if let Some(f) = &mut self.on_scroll {
                f(self.scroll_offset);
            }
        }
    }
}
