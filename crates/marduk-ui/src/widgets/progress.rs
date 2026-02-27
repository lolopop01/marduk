use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::{Color, Paint};

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::Widget;

/// A non-interactive horizontal progress bar.
///
/// # Example
/// ```rust,ignore
/// ProgressBar::new()
///     .value(0.7)
///     .fill_color(Color::from_straight(0.1, 0.8, 0.5, 1.0))
///     .height(6.0)
/// ```
pub struct ProgressBar {
    /// Current progress in [0, 1].
    value: f32,
    height: f32,
    track_color: Color,
    fill_color: Color,
    corner_radius: f32,
}

impl ProgressBar {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            height: 6.0,
            track_color: Color::from_straight(0.15, 0.2, 0.3, 1.0),
            fill_color: Color::from_straight(0.2, 0.65, 1.0, 1.0),
            corner_radius: 3.0,
        }
    }

    pub fn value(mut self, v: f32) -> Self { self.value = v.clamp(0.0, 1.0); self }
    pub fn height(mut self, v: f32) -> Self { self.height = v; self }
    pub fn track_color(mut self, v: Color) -> Self { self.track_color = v; self }
    pub fn fill_color(mut self, v: Color) -> Self { self.fill_color = v; self }
    pub fn corner_radius(mut self, v: f32) -> Self { self.corner_radius = v; self }
}

impl Default for ProgressBar { fn default() -> Self { Self::new() } }

impl Widget for ProgressBar {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        let w = if constraints.max.x.is_finite() { constraints.max.x } else { 200.0 };
        constraints.constrain(Vec2::new(w, self.height))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        // Track
        painter.fill_rounded_rect(rect, self.corner_radius, Paint::Solid(self.track_color), None);

        // Fill
        let fill_w = rect.size.x * self.value;
        if fill_w > 0.0 {
            painter.fill_rounded_rect(
                Rect::new(rect.origin.x, rect.origin.y, fill_w, rect.size.y),
                self.corner_radius,
                Paint::Solid(self.fill_color),
                None,
            );
        }
    }

    fn on_event(&mut self, _event: &UiEvent, _rect: Rect, _ctx: &LayoutCtx<'_>) -> EventResult {
        EventResult::Ignored
    }
}
