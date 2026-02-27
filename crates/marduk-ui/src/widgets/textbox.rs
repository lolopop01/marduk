use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::input::Key;
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;
use marduk_engine::text::FontId;

use crate::constraints::{Constraints, Edges, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::Widget;

/// A single-line text input field.
///
/// Click to focus, then type to edit. Backspace deletes. Enter fires `on_submit`.
///
/// # Example
/// ```rust,ignore
/// TextBox::new()
///     .font(body_font)
///     .placeholder("Type here…")
///     .on_change(|v| println!("text: {v}"))
///     .on_submit(|v| println!("submitted: {v}"))
/// ```
pub struct TextBox {
    text: String,
    placeholder: String,
    focused: bool,
    font: Option<FontId>,
    font_size: f32,
    text_color: Color,
    placeholder_color: Color,
    bg: Color,
    focused_bg: Color,
    border_color: Color,
    focused_border_color: Color,
    corner_radius: f32,
    padding: Edges,
    on_change: Option<Box<dyn FnMut(String)>>,
    on_submit: Option<Box<dyn FnMut(String)>>,
    /// Fired when this box gains focus (for DSL focus-tracking).
    on_focus: Option<Box<dyn FnMut()>>,
}

impl TextBox {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            placeholder: String::new(),
            focused: false,
            font: None,
            font_size: 13.0,
            text_color: Color::from_straight(0.9, 0.92, 0.95, 1.0),
            placeholder_color: Color::from_straight(0.35, 0.45, 0.55, 1.0),
            bg: Color::from_straight(0.06, 0.1, 0.16, 1.0),
            focused_bg: Color::from_straight(0.07, 0.14, 0.22, 1.0),
            border_color: Color::from_straight(0.18, 0.28, 0.42, 1.0),
            focused_border_color: Color::from_straight(0.0, 0.67, 1.0, 1.0),
            corner_radius: 4.0,
            padding: Edges::symmetric(6.0, 10.0),
            on_change: None,
            on_submit: None,
            on_focus: None,
        }
    }

    pub fn text(mut self, v: impl Into<String>) -> Self { self.text = v.into(); self }
    pub fn placeholder(mut self, v: impl Into<String>) -> Self { self.placeholder = v.into(); self }
    pub fn focused(mut self, v: bool) -> Self { self.focused = v; self }
    pub fn font(mut self, v: FontId) -> Self { self.font = Some(v); self }
    pub fn font_size(mut self, v: f32) -> Self { self.font_size = v; self }
    pub fn text_color(mut self, v: Color) -> Self { self.text_color = v; self }
    pub fn placeholder_color(mut self, v: Color) -> Self { self.placeholder_color = v; self }
    pub fn bg(mut self, v: Color) -> Self { self.bg = v; self }
    pub fn focused_bg(mut self, v: Color) -> Self { self.focused_bg = v; self }
    pub fn border_color(mut self, v: Color) -> Self { self.border_color = v; self }
    pub fn focused_border_color(mut self, v: Color) -> Self { self.focused_border_color = v; self }
    pub fn corner_radius(mut self, v: f32) -> Self { self.corner_radius = v; self }
    pub fn padding(mut self, v: Edges) -> Self { self.padding = v; self }
    pub fn padding_all(mut self, v: f32) -> Self { self.padding = Edges::all(v); self }

    pub fn on_change(mut self, f: impl FnMut(String) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }
    pub fn on_submit(mut self, f: impl FnMut(String) + 'static) -> Self {
        self.on_submit = Some(Box::new(f));
        self
    }
    pub fn on_focus(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_focus = Some(Box::new(f));
        self
    }
}

impl Default for TextBox { fn default() -> Self { Self::new() } }

impl Widget for TextBox {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        let min_h = self.font_size + self.padding.v() + 2.0;
        let w = if constraints.max.x.is_finite() { constraints.max.x } else { 200.0 };
        constraints.constrain(Vec2::new(w, min_h))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let bg = if self.focused { self.focused_bg } else { self.bg };
        let border_color = if self.focused { self.focused_border_color } else { self.border_color };
        let border_width = if self.focused { 2.0 } else { 1.0 };

        painter.fill_rounded_rect(
            rect, self.corner_radius, Paint::Solid(bg),
            Some(Border::new(border_width, border_color)),
        );

        let inner = Rect::new(
            rect.origin.x + self.padding.left,
            rect.origin.y + self.padding.top,
            (rect.size.x - self.padding.h()).max(0.0),
            (rect.size.y - self.padding.v()).max(0.0),
        );

        let text_y = inner.origin.y + (inner.size.y - self.font_size) * 0.5;

        if let Some(font) = self.font {
            if self.text.is_empty() && !self.placeholder.is_empty() {
                // Placeholder
                painter.text(
                    &self.placeholder, font, self.font_size,
                    self.placeholder_color,
                    Vec2::new(inner.origin.x, text_y),
                    Some(inner.size.x),
                );
            } else {
                // Content
                painter.text(
                    &self.text, font, self.font_size, self.text_color,
                    Vec2::new(inner.origin.x, text_y),
                    Some(inner.size.x),
                );

                // Cursor — a thin vertical bar after the last character
                if self.focused {
                    let text_w = painter.font_system
                        .measure_text(&self.text, font, self.font_size, None).x;
                    let cx = (inner.origin.x + text_w + 1.0).min(inner.origin.x + inner.size.x - 2.0);
                    painter.fill_rounded_rect(
                        Rect::new(cx, inner.origin.y, 2.0, inner.size.y),
                        1.0,
                        Paint::Solid(self.focused_border_color),
                        None,
                    );
                }
            }
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _ctx: &LayoutCtx<'_>) -> EventResult {
        match event {
            UiEvent::Click { pos } => {
                if rect.contains(*pos) {
                    if !self.focused {
                        self.focused = true;
                        if let Some(f) = &mut self.on_focus { f(); }
                    }
                    EventResult::Consumed
                } else {
                    // Click outside — lose focus
                    self.focused = false;
                    EventResult::Ignored
                }
            }
            UiEvent::TextInput { text } => {
                if self.focused {
                    self.text.push_str(text);
                    if let Some(f) = &mut self.on_change { f(self.text.clone()); }
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }
            UiEvent::KeyPress { key } => {
                if !self.focused { return EventResult::Ignored; }
                match key {
                    Key::Backspace => {
                        // Remove last UTF-8 character
                        let mut chars = self.text.chars();
                        chars.next_back();
                        self.text = chars.as_str().to_string();
                        if let Some(f) = &mut self.on_change { f(self.text.clone()); }
                        EventResult::Consumed
                    }
                    Key::Enter => {
                        if let Some(f) = &mut self.on_submit { f(self.text.clone()); }
                        EventResult::Consumed
                    }
                    Key::Escape => {
                        self.focused = false;
                        EventResult::Consumed
                    }
                    _ => EventResult::Ignored,
                }
            }
            _ => EventResult::Ignored,
        }
    }
}
