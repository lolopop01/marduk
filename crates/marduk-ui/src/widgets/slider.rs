use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::Widget;

/// A horizontal slider for selecting a value in a range.
///
/// Click anywhere on the track to set the value at that position.
///
/// # Example
/// ```rust,ignore
/// Slider::new()
///     .min(0.0).max(100.0).value(50.0)
///     .fill_color(Color::from_straight(0.2, 0.6, 1.0, 1.0))
///     .on_change(|v| println!("slider: {v}"))
/// ```
pub struct Slider {
    /// Current value (clamped to [min, max]).
    value: f32,
    min: f32,
    max: f32,
    track_height: f32,
    thumb_radius: f32,
    track_color: Color,
    fill_color: Color,
    thumb_color: Color,
    thumb_border_color: Color,
    corner_radius: f32,
    /// Called on every drag move — used to persist visual state without firing an event.
    on_drag: Option<Box<dyn FnMut(f32)>>,
    /// Called once on mouse release — the public "value committed" event.
    on_change: Option<Box<dyn FnMut(f32)>>,
}

impl Slider {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 1.0,
            track_height: 4.0,
            thumb_radius: 8.0,
            track_color: Color::from_straight(0.15, 0.2, 0.3, 1.0),
            fill_color: Color::from_straight(0.2, 0.6, 1.0, 1.0),
            thumb_color: Color::from_straight(1.0, 1.0, 1.0, 1.0),
            thumb_border_color: Color::from_straight(0.4, 0.6, 0.9, 1.0),
            corner_radius: 2.0,
            on_drag: None,
            on_change: None,
        }
    }

    pub fn value(mut self, v: f32) -> Self {
        self.value = v.clamp(self.min, self.max);
        self
    }
    pub fn min(mut self, v: f32) -> Self { self.min = v; self }
    pub fn max(mut self, v: f32) -> Self { self.max = v; self }
    pub fn track_height(mut self, v: f32) -> Self { self.track_height = v; self }
    pub fn thumb_radius(mut self, v: f32) -> Self { self.thumb_radius = v; self }
    pub fn track_color(mut self, v: Color) -> Self { self.track_color = v; self }
    pub fn fill_color(mut self, v: Color) -> Self { self.fill_color = v; self }
    pub fn thumb_color(mut self, v: Color) -> Self { self.thumb_color = v; self }
    pub fn corner_radius(mut self, v: f32) -> Self { self.corner_radius = v; self }
    pub fn on_drag(mut self, f: impl FnMut(f32) + 'static) -> Self {
        self.on_drag = Some(Box::new(f));
        self
    }
    pub fn on_change(mut self, f: impl FnMut(f32) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    /// Normalized value in [0, 1].
    fn normalized(&self) -> f32 {
        if (self.max - self.min).abs() < f32::EPSILON {
            0.0
        } else {
            ((self.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
        }
    }

    fn thumb_cx(&self, track_rect: Rect) -> f32 {
        track_rect.origin.x + self.normalized() * track_rect.size.x
    }
}

impl Default for Slider { fn default() -> Self { Self::new() } }

impl Widget for Slider {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        let h = self.thumb_radius * 2.0;
        let w = if constraints.max.x.is_finite() { constraints.max.x } else { 200.0 };
        constraints.constrain(Vec2::new(w, h))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let cy = rect.origin.y + rect.size.y * 0.5;

        // Track
        let track = Rect::new(
            rect.origin.x,
            cy - self.track_height * 0.5,
            rect.size.x,
            self.track_height,
        );
        painter.fill_rounded_rect(track, self.corner_radius, Paint::Solid(self.track_color), None);

        // Fill (left of thumb)
        let thumb_cx = self.thumb_cx(track);
        let fill_w = (thumb_cx - track.origin.x).max(0.0);
        if fill_w > 0.0 {
            let fill = Rect::new(track.origin.x, track.origin.y, fill_w, track.size.y);
            painter.fill_rounded_rect(fill, self.corner_radius, Paint::Solid(self.fill_color), None);
        }

        // Thumb
        let hovered  = painter.is_hovered(rect);
        let r_outer  = self.thumb_radius;
        let r_draw   = if hovered { r_outer + 1.5 } else { r_outer };
        let border   = Some(Border::new(2.0, self.thumb_border_color));
        painter.fill_circle(Vec2::new(thumb_cx, cy), r_draw, Paint::Solid(self.thumb_color), border);
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _ctx: &LayoutCtx<'_>) -> EventResult {
        let value_at = |x: f32| -> f32 {
            let t = ((x - rect.origin.x) / rect.size.x).clamp(0.0, 1.0);
            self.min + t * (self.max - self.min)
        };
        match event {
            // Drag: update value + state (visual), no public event.
            UiEvent::Drag { pos, start } if rect.contains(*start) => {
                self.value = value_at(pos.x);
                if let Some(f) = &mut self.on_drag { f(self.value); }
                EventResult::Consumed
            }
            // DragEnd: fires when the button is released regardless of cursor position.
            // Guards on `start` so only the slider that owns the drag commits.
            UiEvent::DragEnd { pos, start } if rect.contains(*start) => {
                self.value = value_at(pos.x);
                if let Some(f) = &mut self.on_drag   { f(self.value); }
                if let Some(f) = &mut self.on_change { f(self.value); }
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }
}
