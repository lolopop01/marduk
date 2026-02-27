use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;
use marduk_engine::text::FontId;

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::Widget;

/// A checkbox toggle widget with an optional text label.
///
/// # Example
/// ```rust,ignore
/// Checkbox::new()
///     .label("Enable shadows")
///     .font(body_font)
///     .checked(true)
///     .checked_color(Color::from_straight(0.2, 0.6, 1.0, 1.0))
///     .on_change(|v| println!("checkbox: {v}"))
/// ```
pub struct Checkbox {
    checked: bool,
    label: String,
    font: Option<FontId>,
    font_size: f32,
    label_color: Color,
    box_size: f32,
    bg: Color,
    checked_color: Color,
    border_color: Color,
    border_width: f32,
    corner_radius: f32,
    gap: f32,
    on_change: Option<Box<dyn FnMut(bool)>>,
}

impl Checkbox {
    pub fn new() -> Self {
        Self {
            checked: false,
            label: String::new(),
            font: None,
            font_size: 13.0,
            label_color: Color::from_straight(0.85, 0.85, 0.9, 1.0),
            box_size: 16.0,
            bg: Color::from_straight(0.1, 0.12, 0.18, 1.0),
            checked_color: Color::from_straight(0.2, 0.65, 1.0, 1.0),
            border_color: Color::from_straight(0.35, 0.45, 0.6, 1.0),
            border_width: 1.5,
            corner_radius: 3.0,
            gap: 8.0,
            on_change: None,
        }
    }

    pub fn checked(mut self, v: bool) -> Self { self.checked = v; self }
    pub fn label(mut self, v: impl Into<String>) -> Self { self.label = v.into(); self }
    pub fn font(mut self, v: FontId) -> Self { self.font = Some(v); self }
    pub fn font_size(mut self, v: f32) -> Self { self.font_size = v; self }
    pub fn label_color(mut self, v: Color) -> Self { self.label_color = v; self }
    pub fn box_size(mut self, v: f32) -> Self { self.box_size = v; self }
    pub fn checked_color(mut self, v: Color) -> Self { self.checked_color = v; self }
    pub fn border_color(mut self, v: Color) -> Self { self.border_color = v; self }
    pub fn corner_radius(mut self, v: f32) -> Self { self.corner_radius = v; self }
    pub fn on_change(mut self, f: impl FnMut(bool) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }
}

impl Default for Checkbox { fn default() -> Self { Self::new() } }

impl Widget for Checkbox {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        let text_size = if !self.label.is_empty() {
            self.font.map(|f| ctx.fonts.measure_text(&self.label, f, self.font_size, None))
                .unwrap_or(Vec2::zero())
        } else {
            Vec2::zero()
        };

        let w = if text_size.x > 0.0 {
            self.box_size + self.gap + text_size.x
        } else {
            self.box_size
        };
        let h = self.box_size.max(text_size.y);
        constraints.constrain(Vec2::new(w, h))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let hovered = painter.is_hovered(rect);

        let box_y = rect.origin.y + (rect.size.y - self.box_size) * 0.5;
        let box_rect = Rect::new(rect.origin.x, box_y, self.box_size, self.box_size);

        // Background: slightly brighter on hover
        let bg = if self.checked {
            if hovered {
                Color::from_straight(
                    (self.checked_color.r / self.checked_color.a + 0.1).min(1.0),
                    (self.checked_color.g / self.checked_color.a + 0.1).min(1.0),
                    (self.checked_color.b / self.checked_color.a + 0.1).min(1.0),
                    self.checked_color.a,
                )
            } else {
                self.checked_color
            }
        } else {
            self.bg
        };

        let border_col = if hovered && !self.checked {
            Color::from_straight(0.6, 0.7, 0.9, 1.0)
        } else {
            self.border_color
        };

        painter.fill_rounded_rect(
            box_rect,
            self.corner_radius,
            Paint::Solid(bg),
            Some(Border::new(self.border_width, border_col)),
        );

        // Checkmark — use ✓ text if a font is loaded, else a small filled square
        if self.checked {
            if let Some(font) = self.font {
                let mark_size = self.box_size * 0.72;
                painter.text(
                    "✓",
                    font,
                    mark_size,
                    Color::from_straight(1.0, 1.0, 1.0, 1.0),
                    Vec2::new(
                        box_rect.origin.x + self.box_size * 0.08,
                        box_rect.origin.y + (self.box_size - mark_size) * 0.5,
                    ),
                    None,
                );
            } else {
                let m = self.box_size * 0.22;
                painter.fill_rounded_rect(
                    Rect::new(box_rect.origin.x + m, box_rect.origin.y + m,
                              self.box_size - m * 2.0, self.box_size - m * 2.0),
                    self.corner_radius * 0.4,
                    Paint::Solid(Color::from_straight(1.0, 1.0, 1.0, 0.9)),
                    None,
                );
            }
        }

        // Label
        if !self.label.is_empty() {
            if let Some(font) = self.font {
                let text_x = rect.origin.x + self.box_size + self.gap;
                let text_h  = self.font_size;
                let text_y  = rect.origin.y + (rect.size.y - text_h) * 0.5;
                painter.text(
                    &self.label, font, self.font_size, self.label_color,
                    Vec2::new(text_x, text_y),
                    Some(rect.size.x - self.box_size - self.gap),
                );
            }
        }
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
