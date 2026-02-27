use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;
use marduk_engine::text::FontId;

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::Widget;

/// An option within a [`RadioGroup`].
#[derive(Clone)]
pub struct RadioOption {
    /// Display label.
    pub label: String,
    /// Logical value identifying this option.
    pub value: String,
}

impl RadioOption {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self { label: label.into(), value: value.into() }
    }
}

/// A vertical list of mutually-exclusive radio-button options.
///
/// # Example
/// ```rust,ignore
/// RadioGroup::new()
///     .option("Solid",     "solid")
///     .option("Wireframe", "wire")
///     .option("Points",    "pts")
///     .selected("solid")
///     .font(body_font)
///     .on_change(|v| println!("selected: {v}"))
/// ```
pub struct RadioGroup {
    options: Vec<RadioOption>,
    selected: Option<String>,
    font: Option<FontId>,
    font_size: f32,
    label_color: Color,
    selected_color: Color,
    border_color: Color,
    dot_radius: f32,
    gap: f32,       // between dot and label
    item_gap: f32,  // between rows
    on_change: Option<Box<dyn FnMut(String)>>,
}

impl RadioGroup {
    pub fn new() -> Self {
        Self {
            options: Vec::new(),
            selected: None,
            font: None,
            font_size: 13.0,
            label_color: Color::from_straight(0.85, 0.85, 0.9, 1.0),
            selected_color: Color::from_straight(0.2, 0.65, 1.0, 1.0),
            border_color: Color::from_straight(0.35, 0.45, 0.6, 1.0),
            dot_radius: 8.0,
            gap: 8.0,
            item_gap: 10.0,
            on_change: None,
        }
    }

    pub fn option(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.push(RadioOption::new(label, value));
        self
    }
    pub fn options(mut self, opts: impl IntoIterator<Item = RadioOption>) -> Self {
        self.options.extend(opts);
        self
    }
    pub fn selected(mut self, value: impl Into<String>) -> Self {
        self.selected = Some(value.into());
        self
    }
    pub fn font(mut self, v: FontId) -> Self { self.font = Some(v); self }
    pub fn font_size(mut self, v: f32) -> Self { self.font_size = v; self }
    pub fn label_color(mut self, v: Color) -> Self { self.label_color = v; self }
    pub fn selected_color(mut self, v: Color) -> Self { self.selected_color = v; self }
    pub fn border_color(mut self, v: Color) -> Self { self.border_color = v; self }
    pub fn dot_radius(mut self, v: f32) -> Self { self.dot_radius = v; self }
    pub fn item_gap(mut self, v: f32) -> Self { self.item_gap = v; self }
    pub fn on_change(mut self, f: impl FnMut(String) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    fn row_height(&self) -> f32 {
        (self.dot_radius * 2.0).max(self.font_size * 1.2)
    }

    fn total_height(&self) -> f32 {
        let n = self.options.len() as f32;
        if n == 0.0 { return 0.0; }
        n * self.row_height() + (n - 1.0) * self.item_gap
    }
}

impl Default for RadioGroup { fn default() -> Self { Self::new() } }

impl Widget for RadioGroup {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        let max_label_w = self.options.iter().fold(0.0f32, |acc, opt| {
            let w = self.font.map(|f| ctx.fonts.measure_text(&opt.label, f, self.font_size, None).x)
                .unwrap_or(0.0);
            acc.max(w)
        });
        let w = self.dot_radius * 2.0 + self.gap + max_label_w;
        let h = self.total_height();
        constraints.constrain(Vec2::new(w, h))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let row_h    = self.row_height();
        let dot_d    = self.dot_radius * 2.0;

        let mut y = rect.origin.y;
        for opt in &self.options {
            let is_selected = self.selected.as_deref() == Some(&opt.value);
            let row_cy = y + row_h * 0.5;

            // Outer ring
            let ring_cx = rect.origin.x + self.dot_radius;
            painter.fill_circle(
                Vec2::new(ring_cx, row_cy),
                self.dot_radius,
                Paint::Solid(Color::from_straight(0.1, 0.12, 0.18, 1.0)),
                Some(Border::new(1.5, if is_selected { self.selected_color } else { self.border_color })),
            );

            // Inner dot (if selected)
            if is_selected {
                painter.fill_circle(
                    Vec2::new(ring_cx, row_cy),
                    self.dot_radius * 0.45,
                    Paint::Solid(self.selected_color),
                    None,
                );
            }

            // Label
            if let Some(font) = self.font {
                let text_x = rect.origin.x + dot_d + self.gap;
                let text_y = row_cy - self.font_size * 0.5;
                painter.text(
                    &opt.label, font, self.font_size, self.label_color,
                    Vec2::new(text_x, text_y), None,
                );
            }

            y += row_h + self.item_gap;
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        if let UiEvent::Click { pos } = event {
            if !rect.contains(*pos) { return EventResult::Ignored; }

            let row_h = self.row_height();
            let mut y = rect.origin.y;
            for opt in &self.options {
                let row_rect = Rect::new(rect.origin.x, y, rect.size.x, row_h);
                if row_rect.contains(*pos) {
                    let value = opt.value.clone();
                    self.selected = Some(value.clone());
                    if let Some(f) = &mut self.on_change { f(value); }
                    return EventResult::Consumed;
                }
                y += row_h + self.item_gap;
            }
        }
        let _ = ctx; // not needed for this widget
        EventResult::Ignored
    }
}
