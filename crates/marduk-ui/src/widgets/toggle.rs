use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::{Color, Paint};

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::Widget;

/// A pill-shaped on/off toggle switch.
///
/// # Example
/// ```rust,ignore
/// Toggle::new()
///     .checked(true)
///     .on_color(Color::from_straight(0.1, 0.7, 0.4, 1.0))
///     .on_change(|v| println!("toggle: {v}"))
/// ```
pub struct Toggle {
    checked: bool,
    width: f32,
    height: f32,
    on_color: Color,
    off_color: Color,
    thumb_color: Color,
    on_change: Option<Box<dyn FnMut(bool)>>,
}

impl Toggle {
    pub fn new() -> Self {
        Self {
            checked: false,
            width: 46.0,
            height: 24.0,
            on_color: Color::from_straight(0.1, 0.7, 0.45, 1.0),
            off_color: Color::from_straight(0.2, 0.25, 0.35, 1.0),
            thumb_color: Color::from_straight(1.0, 1.0, 1.0, 1.0),
            on_change: None,
        }
    }

    pub fn checked(mut self, v: bool) -> Self { self.checked = v; self }
    pub fn width(mut self, v: f32) -> Self { self.width = v; self }
    pub fn height(mut self, v: f32) -> Self { self.height = v; self }
    pub fn on_color(mut self, v: Color) -> Self { self.on_color = v; self }
    pub fn off_color(mut self, v: Color) -> Self { self.off_color = v; self }
    pub fn thumb_color(mut self, v: Color) -> Self { self.thumb_color = v; self }
    pub fn on_change(mut self, f: impl FnMut(bool) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }
}

impl Default for Toggle { fn default() -> Self { Self::new() } }

impl Widget for Toggle {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        constraints.constrain(Vec2::new(self.width, self.height))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let track_color = if self.checked { self.on_color } else { self.off_color };
        let radius = rect.size.y * 0.5;

        // Track (pill shape)
        painter.fill_rounded_rect(rect, radius, Paint::Solid(track_color), None);

        // Thumb
        let margin = rect.size.y * 0.13;
        let thumb_r = (rect.size.y * 0.5) - margin;
        let thumb_cx = if self.checked {
            rect.origin.x + rect.size.x - margin - thumb_r
        } else {
            rect.origin.x + margin + thumb_r
        };
        let thumb_cy = rect.origin.y + rect.size.y * 0.5;
        painter.fill_circle(
            Vec2::new(thumb_cx, thumb_cy),
            thumb_r,
            Paint::Solid(self.thumb_color),
            None,
        );
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _ctx: &LayoutCtx<'_>) -> EventResult {
        if let UiEvent::Click { pos } = event {
            if rect.contains(*pos) {
                self.checked = !self.checked;
                if let Some(f) = &mut self.on_change { f(self.checked); }
                return EventResult::Consumed;
            }
        }
        EventResult::Ignored
    }
}
