use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::Color;
use marduk_engine::text::FontId;

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::Widget;

/// A single-run text widget.
///
/// Text is measured using the engine's `FontSystem` so layout is pixel-accurate.
/// Wrapping is controlled by the width constraint from the parent.
///
/// # Example
/// ```rust,ignore
/// Text::new("Hello, world!", font, 16.0, Color::from_straight(1.0, 1.0, 1.0, 1.0))
/// ```
pub struct Text {
    pub text: String,
    pub font: FontId,
    pub size: f32,
    pub color: Color,
}

impl Text {
    pub fn new(text: impl Into<String>, font: FontId, size: f32, color: Color) -> Self {
        Self { text: text.into(), font, size, color }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl Widget for Text {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        let max_w = if constraints.max.x.is_finite() { Some(constraints.max.x) } else { None };
        let size = ctx.fonts.measure_text(&self.text, self.font, self.size, max_w);
        constraints.constrain(size)
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let max_w = if rect.size.x > 0.0 { Some(rect.size.x) } else { None };
        painter.text(&self.text, self.font, self.size, self.color, rect.origin, max_w);
    }

    fn on_event(&mut self, _event: &UiEvent, _rect: Rect, _ctx: &LayoutCtx<'_>) -> EventResult {
        EventResult::Ignored
    }
}
