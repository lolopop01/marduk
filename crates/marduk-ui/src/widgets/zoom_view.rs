use std::cell::Cell;

use marduk_engine::coords::{Rect, Vec2};

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::{Element, Widget};

/// A zoomable, pannable container.
///
/// The child widget is laid out in an unbounded "content space". The ZoomView
/// displays a windowed view of that space:
///
/// - **Ctrl + scroll wheel**: zoom in / out, centred on the current cursor position.
/// - **Scroll wheel** (no modifier): pan vertically.
/// - **Shift + scroll wheel**: pan horizontally.
///
/// Pan and zoom state are stored on the widget. In DSL mode, wire `on_change`
/// so the values survive frame rebuilds.
///
/// # Example
/// ```rust,ignore
/// ZoomView::new(my_canvas_widget)
///     .zoom(1.0)
///     .min_zoom(0.1)
///     .max_zoom(8.0)
///     .on_change(|zoom, pan| { state.set("zoom", zoom); state.set_pan(pan); })
/// ```
pub struct ZoomView {
    child: Element,
    /// Current zoom factor (1.0 = 100 %).
    pub zoom: f32,
    /// Pan offset: top-left corner of the visible window, in content-space pixels.
    pub pan: Vec2,
    min_zoom: f32,
    max_zoom: f32,
    /// Fractional zoom change per scroll notch (default 0.15 → 15 %).
    zoom_step: f32,
    /// Pixels of content-space scroll per wheel notch (default 60 / zoom).
    pan_speed: f32,
    /// Fired whenever zoom or pan changes.
    on_change: Option<Box<dyn FnMut(f32, Vec2)>>,
    /// Last known cursor position in screen space (updated by `Hover` each frame).
    /// Used to zoom centred on the cursor rather than the viewport centre.
    hover_pos: Cell<Vec2>,
    /// Cached natural content size from the most recent paint call.
    cached_content_size: Cell<Vec2>,
}

impl ZoomView {
    pub fn new(child: impl Into<Element>) -> Self {
        Self {
            child: child.into(),
            zoom: 1.0,
            pan: Vec2::new(0.0, 0.0),
            min_zoom: 0.1,
            max_zoom: 10.0,
            zoom_step: 0.15,
            pan_speed: 60.0,
            cached_content_size: Cell::new(Vec2::new(1.0, 1.0)),
            hover_pos: Cell::new(Vec2::new(0.0, 0.0)),
            on_change: None,
        }
    }

    pub fn zoom(mut self, z: f32) -> Self {
        self.zoom = z;
        self
    }

    pub fn pan(mut self, p: Vec2) -> Self {
        self.pan = p;
        self
    }

    pub fn min_zoom(mut self, v: f32) -> Self {
        self.min_zoom = v.max(0.001);
        self
    }

    pub fn max_zoom(mut self, v: f32) -> Self {
        self.max_zoom = v;
        self
    }

    pub fn zoom_step(mut self, v: f32) -> Self {
        self.zoom_step = v;
        self
    }

    /// Base pan speed in content pixels per scroll notch (scaled by 1/zoom automatically).
    pub fn pan_speed(mut self, v: f32) -> Self {
        self.pan_speed = v;
        self
    }

    /// Called with `(new_zoom, new_pan)` whenever either changes.
    pub fn on_change(mut self, f: impl FnMut(f32, Vec2) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    // ── helpers ───────────────────────────────────────────────────────────

    fn to_content(&self, screen_pos: Vec2, view_origin: Vec2) -> Vec2 {
        Vec2::new(
            (screen_pos.x - view_origin.x) / self.zoom + self.pan.x,
            (screen_pos.y - view_origin.y) / self.zoom + self.pan.y,
        )
    }

    fn clamp_pan(&self, pan: Vec2, view_size: Vec2) -> Vec2 {
        let content = self.cached_content_size.get();
        let max_x = (content.x - view_size.x / self.zoom).max(0.0);
        let max_y = (content.y - view_size.y / self.zoom).max(0.0);
        Vec2::new(pan.x.clamp(0.0, max_x), pan.y.clamp(0.0, max_y))
    }

    fn notify(&mut self) {
        let zoom = self.zoom;
        let pan  = self.pan;
        if let Some(f) = &mut self.on_change { f(zoom, pan); }
    }

    /// Transform positional event fields from screen space to content space.
    fn xform_event(&self, event: &UiEvent, view_origin: Vec2) -> UiEvent {
        match *event {
            UiEvent::Hover { pos } =>
                UiEvent::Hover { pos: self.to_content(pos, view_origin) },
            UiEvent::Click { pos } =>
                UiEvent::Click { pos: self.to_content(pos, view_origin) },
            UiEvent::Drag { pos, start } => UiEvent::Drag {
                pos:   self.to_content(pos,   view_origin),
                start: self.to_content(start, view_origin),
            },
            UiEvent::DragEnd { pos, start } => UiEvent::DragEnd {
                pos:   self.to_content(pos,   view_origin),
                start: self.to_content(start, view_origin),
            },
            ref other => other.clone(),
        }
    }
}

impl Widget for ZoomView {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        let w = if constraints.max.x.is_finite() { constraints.max.x } else { 200.0 };
        let h = if constraints.max.y.is_finite() { constraints.max.y } else { 200.0 };
        Vec2::new(w, h)
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let ctx = painter.layout_ctx();
        let content_size = self.child.measure(
            Constraints::loose(Vec2::new(f32::INFINITY, f32::INFINITY)),
            &ctx,
        );
        self.cached_content_size.set(content_size);

        painter.push_clip(rect);

        // Transform: screen = content * zoom + (rect.origin - pan * zoom)
        // → content at `pan` maps to `rect.origin` on screen.
        let offset = Vec2::new(
            rect.origin.x - self.pan.x * self.zoom,
            rect.origin.y - self.pan.y * self.zoom,
        );
        painter.push_transform(self.zoom, offset);

        self.child.paint(painter, Rect::new(0.0, 0.0, content_size.x, content_size.y));

        painter.pop_transform();
        painter.pop_clip();

        // ── scrollbars (drawn outside the clip in screen space) ───────────
        self.paint_scrollbars(painter, rect, content_size);
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        let content_rect = Rect::new(
            0.0, 0.0,
            self.cached_content_size.get().x,
            self.cached_content_size.get().y,
        );

        match event {
            // ── track cursor for zoom-centring ─────────────────────────────
            UiEvent::Hover { pos } => {
                if rect.contains(*pos) {
                    self.hover_pos.set(*pos);
                }
                let transformed = self.xform_event(event, rect.origin);
                self.child.on_event(&transformed, content_rect, ctx);
                return EventResult::Ignored; // Hover never consumes
            }

            // ── Ctrl+scroll: zoom centred on cursor ────────────────────────
            UiEvent::ScrollWheel { delta, modifiers } if modifiers.ctrl => {
                if !rect.contains(self.hover_pos.get()) { return EventResult::Ignored; }

                let pivot = self.to_content(self.hover_pos.get(), rect.origin);

                // delta < 0 = scroll up = zoom in
                let factor = if *delta < 0.0 {
                    1.0 + self.zoom_step
                } else {
                    1.0 / (1.0 + self.zoom_step)
                };
                let new_zoom = (self.zoom * factor).clamp(self.min_zoom, self.max_zoom);

                // Keep the pivot fixed: pan_new = pivot - (pivot - pan_old) * (old/new)
                let ratio = self.zoom / new_zoom;
                let new_pan = Vec2::new(
                    pivot.x - (pivot.x - self.pan.x) * ratio,
                    pivot.y - (pivot.y - self.pan.y) * ratio,
                );
                self.zoom = new_zoom;
                self.pan  = self.clamp_pan(new_pan, rect.size);
                self.notify();
                return EventResult::Consumed;
            }

            // ── Shift+scroll: horizontal pan ───────────────────────────────
            UiEvent::ScrollWheel { delta, modifiers } if modifiers.shift => {
                if !rect.contains(self.hover_pos.get()) { return EventResult::Ignored; }
                let speed = self.pan_speed / self.zoom;
                let new_pan = Vec2::new(self.pan.x + delta * speed, self.pan.y);
                self.pan = self.clamp_pan(new_pan, rect.size);
                self.notify();
                return EventResult::Consumed;
            }

            // ── plain scroll: child first, then vertical pan ───────────────
            UiEvent::ScrollWheel { delta, .. } => {
                if !rect.contains(self.hover_pos.get()) { return EventResult::Ignored; }
                let transformed = self.xform_event(event, rect.origin);
                if self.child.on_event(&transformed, content_rect, ctx).is_consumed() {
                    return EventResult::Consumed;
                }
                let speed = self.pan_speed / self.zoom;
                let new_pan = Vec2::new(self.pan.x, self.pan.y + delta * speed);
                self.pan = self.clamp_pan(new_pan, rect.size);
                self.notify();
                return EventResult::Consumed;
            }

            // ── Click / Drag / DragEnd: gate on viewport containment ────────
            UiEvent::Click { pos } if !rect.contains(*pos) =>
                return EventResult::Ignored,

            UiEvent::Drag { start, .. } | UiEvent::DragEnd { start, .. }
                if !rect.contains(*start) =>
                    return EventResult::Ignored,

            // ── all other events: transform and route to child ─────────────
            _ => {
                let transformed = self.xform_event(event, rect.origin);
                return self.child.on_event(&transformed, content_rect, ctx);
            }
        }
    }
}

impl ZoomView {
    fn paint_scrollbars(&self, painter: &mut Painter, rect: Rect, content: Vec2) {
        use marduk_engine::paint::{Color, Paint};

        const BAR_W: f32 = 6.0;
        const THUMB_MIN: f32 = 24.0;

        let track_c = Color::from_straight(0.12, 0.12, 0.12, 0.8);
        let thumb_c = Color::from_straight(0.50, 0.50, 0.50, 0.85);

        let vis_w = rect.size.x / self.zoom;
        let vis_h = rect.size.y / self.zoom;

        if content.y > vis_h {
            let track = Rect::new(rect.origin.x + rect.size.x - BAR_W, rect.origin.y, BAR_W, rect.size.y);
            let ratio_h  = vis_h / content.y;
            let thumb_h  = (rect.size.y * ratio_h).max(THUMB_MIN);
            let scroll_r = content.y - vis_h;
            let thumb_y  = rect.origin.y + (self.pan.y / scroll_r) * (rect.size.y - thumb_h);
            let thumb = Rect::new(track.origin.x, thumb_y, BAR_W, thumb_h);
            painter.fill_rounded_rect(track, 3.0, Paint::Solid(track_c), None);
            painter.fill_rounded_rect(thumb, 3.0, Paint::Solid(thumb_c), None);
        }

        if content.x > vis_w {
            let track = Rect::new(rect.origin.x, rect.origin.y + rect.size.y - BAR_W, rect.size.x, BAR_W);
            let ratio_w  = vis_w / content.x;
            let thumb_w  = (rect.size.x * ratio_w).max(THUMB_MIN);
            let scroll_r = content.x - vis_w;
            let thumb_x  = rect.origin.x + (self.pan.x / scroll_r) * (rect.size.x - thumb_w);
            let thumb = Rect::new(thumb_x, track.origin.y, thumb_w, BAR_W);
            painter.fill_rounded_rect(track, 3.0, Paint::Solid(track_c), None);
            painter.fill_rounded_rect(thumb, 3.0, Paint::Solid(thumb_c), None);
        }
    }
}
