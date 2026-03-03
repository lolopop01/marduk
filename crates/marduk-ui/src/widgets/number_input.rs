use marduk_engine::coords::{CornerRadii, Rect, Vec2};
use marduk_engine::input::Key;
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;
use marduk_engine::text::FontId;

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::focus::FocusId;
use crate::painter::Painter;
use crate::widget::Widget;

/// A text field specialized for numeric input with increment/decrement buttons.
///
/// The widget shows the current value as text. The user can type to edit, use
/// the up/down arrow buttons, or scroll the mouse wheel to change the value.
///
/// # Example
/// ```rust,ignore
/// NumberInput::new()
///     .value(42.0)
///     .min(0.0)
///     .max(100.0)
///     .step(1.0)
///     .decimals(0)
///     .on_change(|v| println!("value: {v}"))
/// ```
pub struct NumberInput {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    decimals: usize,
    on_change: Option<Box<dyn FnMut(f64)>>,

    font: Option<FontId>,
    font_size: f32,
    text_color: Color,
    bg: Color,
    focused_bg: Color,
    border_color: Color,
    focused_border_color: Color,
    corner_radius: f32,
    height: f32,
    width: f32,

    // ── per-frame state ────────────────────────────────────────────────────
    /// Text being typed while editing; `None` when not in edit mode.
    edit_text: Option<String>,
    focus_id: FocusId,
}

impl NumberInput {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
            step: 1.0,
            decimals: 0,
            on_change: None,

            font: None,
            font_size: 14.0,
            text_color:           Color::from_srgb(1.0, 1.0, 1.0, 1.0),
            bg:                   Color::from_srgb(0.15, 0.15, 0.18, 1.0),
            focused_bg:           Color::from_srgb(0.18, 0.18, 0.22, 1.0),
            border_color:         Color::from_srgb(0.3, 0.3, 0.35, 1.0),
            focused_border_color: Color::from_srgb(0.39, 0.55, 1.0, 1.0),
            corner_radius: 4.0,
            height: 32.0,
            width: 120.0,

            edit_text: None,
            focus_id: FocusId::new(),
        }
    }

    pub fn value(mut self, v: f64) -> Self { self.value = v; self }
    pub fn min(mut self, v: f64) -> Self { self.min = v; self }
    pub fn max(mut self, v: f64) -> Self { self.max = v; self }
    pub fn step(mut self, v: f64) -> Self { self.step = v; self }
    pub fn decimals(mut self, d: usize) -> Self { self.decimals = d; self }
    pub fn on_change(mut self, f: impl FnMut(f64) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }
    pub fn font(mut self, id: FontId) -> Self { self.font = Some(id); self }
    pub fn font_size(mut self, s: f32) -> Self { self.font_size = s; self }
    pub fn text_color(mut self, c: Color) -> Self { self.text_color = c; self }
    pub fn bg(mut self, c: Color) -> Self { self.bg = c; self }
    pub fn border_color(mut self, c: Color) -> Self { self.border_color = c; self }
    pub fn focused_border_color(mut self, c: Color) -> Self { self.focused_border_color = c; self }
    pub fn corner_radius(mut self, r: f32) -> Self { self.corner_radius = r; self }
    pub fn height(mut self, h: f32) -> Self { self.height = h; self }
    pub fn width(mut self, w: f32) -> Self { self.width = w; self }

    // ── helpers ───────────────────────────────────────────────────────────

    fn format_value(&self) -> String {
        format!("{:.prec$}", self.value, prec = self.decimals)
    }

    fn btn_width(&self) -> f32 {
        self.height * 0.7
    }

    fn field_rect(&self, rect: Rect) -> Rect {
        let bw = self.btn_width();
        Rect::new(rect.origin.x, rect.origin.y, rect.size.x - bw * 2.0 - 2.0, rect.size.y)
    }

    fn up_btn_rect(&self, rect: Rect) -> Rect {
        let bw = self.btn_width();
        let bh = rect.size.y * 0.5;
        Rect::new(rect.origin.x + rect.size.x - bw, rect.origin.y, bw, bh)
    }

    fn down_btn_rect(&self, rect: Rect) -> Rect {
        let bw = self.btn_width();
        let bh = rect.size.y * 0.5;
        Rect::new(rect.origin.x + rect.size.x - bw, rect.origin.y + bh, bw, bh)
    }

    fn clamp_and_commit(&mut self, raw: f64) {
        let clamped = raw.clamp(self.min, self.max);
        if (clamped - self.value).abs() > 1e-12 || raw != clamped {
            self.value = clamped;
            if let Some(f) = &mut self.on_change {
                f(clamped);
            }
        }
    }

    fn step_value(&mut self, dir: f64) {
        let new_val = self.value + dir * self.step;
        self.clamp_and_commit(new_val);
        self.edit_text = None;
    }

    fn commit_edit(&mut self) {
        if let Some(text) = self.edit_text.take() {
            if let Ok(v) = text.trim().parse::<f64>() {
                self.clamp_and_commit(v);
            }
            // If parse fails, revert to current value (edit_text already None).
        }
    }
}

impl Default for NumberInput {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for NumberInput {
    fn measure(&self, _constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        Vec2::new(self.width, self.height)
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let focused = painter.is_focused(self.focus_id);
        painter.register_focusable(self.focus_id);

        let field = self.field_rect(rect);
        let up    = self.up_btn_rect(rect);
        let down  = self.down_btn_rect(rect);

        // ── field background ──────────────────────────────────────────────
        let bg = if focused { self.focused_bg } else { self.bg };
        let border_col = if focused { self.focused_border_color } else { self.border_color };
        let radii = CornerRadii {
            top_left:     self.corner_radius,
            top_right:    0.0,
            bottom_left:  self.corner_radius,
            bottom_right: 0.0,
        };
        painter.fill_rounded_rect_corners(field, radii, Paint::Solid(bg), Some(Border::new(1.0, border_col)));

        // ── displayed text ────────────────────────────────────────────────
        if let Some(font) = self.font {
            let formatted = self.format_value();
            let display = self.edit_text.as_deref().unwrap_or(&formatted);
            let pad = 6.0;
            let text_y = field.origin.y + (field.size.y - self.font_size * 1.2) * 0.5;
            painter.push_clip(field);
            painter.text(
                display.to_string(),
                font,
                self.font_size,
                self.text_color,
                Vec2::new(field.origin.x + pad, text_y),
                Some(field.size.x - pad * 2.0),
            );
            painter.pop_clip();
        }

        // ── arrow buttons ─────────────────────────────────────────────────
        let up_bg = if painter.is_pressed(up) {
            Color::from_srgb(0.35, 0.35, 0.4, 1.0)
        } else if painter.is_hovered(up) {
            Color::from_srgb(0.28, 0.28, 0.33, 1.0)
        } else {
            self.bg
        };
        let down_bg = if painter.is_pressed(down) {
            Color::from_srgb(0.35, 0.35, 0.4, 1.0)
        } else if painter.is_hovered(down) {
            Color::from_srgb(0.28, 0.28, 0.33, 1.0)
        } else {
            self.bg
        };

        let btn_radii_up = CornerRadii { top_left: 0.0, top_right: self.corner_radius, bottom_left: 0.0, bottom_right: 0.0 };
        let btn_radii_dn = CornerRadii { top_left: 0.0, top_right: 0.0, bottom_left: 0.0, bottom_right: self.corner_radius };
        painter.fill_rounded_rect_corners(up,   btn_radii_up, Paint::Solid(up_bg),   Some(Border::new(1.0, border_col)));
        painter.fill_rounded_rect_corners(down, btn_radii_dn, Paint::Solid(down_bg), Some(Border::new(1.0, border_col)));

        // Arrow symbols
        if let Some(font) = self.font {
            let arrow_sz = self.font_size * 0.8;
            let up_m   = painter.measure_text("▲", font, arrow_sz, None);
            let down_m = painter.measure_text("▼", font, arrow_sz, None);
            painter.text("▲", font, arrow_sz, self.text_color,
                Vec2::new(up.origin.x + (up.size.x - up_m.x) * 0.5, up.origin.y + (up.size.y - up_m.y) * 0.5), None);
            painter.text("▼", font, arrow_sz, self.text_color,
                Vec2::new(down.origin.x + (down.size.x - down_m.x) * 0.5, down.origin.y + (down.size.y - down_m.y) * 0.5), None);
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        let field = self.field_rect(rect);
        let up    = self.up_btn_rect(rect);
        let down  = self.down_btn_rect(rect);
        let focused = ctx.is_focused(self.focus_id);

        match event {
            UiEvent::Click { pos } => {
                if up.contains(*pos) {
                    self.step_value(1.0);
                    return EventResult::Consumed;
                }
                if down.contains(*pos) {
                    self.step_value(-1.0);
                    return EventResult::Consumed;
                }
                if field.contains(*pos) {
                    ctx.request_focus(self.focus_id);
                    if self.edit_text.is_none() {
                        self.edit_text = Some(self.format_value());
                    }
                    return EventResult::Consumed;
                }
                if focused {
                    self.commit_edit();
                }
            }

            UiEvent::TextInput { text } => {
                if focused {
                    if self.edit_text.is_none() {
                        self.edit_text = Some(self.format_value());
                    }
                    let current = self.edit_text.as_mut().unwrap();
                    for ch in text.chars() {
                        if ch.is_ascii_digit() || ch == '-' || ch == '.' {
                            current.push(ch);
                        }
                    }
                    return EventResult::Consumed;
                }
            }

            UiEvent::KeyPress { key, .. } => {
                if focused {
                    match key {
                        Key::Backspace => {
                            if self.edit_text.is_none() {
                                self.edit_text = Some(self.format_value());
                            }
                            let current = self.edit_text.as_mut().unwrap();
                            current.pop();
                            return EventResult::Consumed;
                        }
                        Key::Enter => {
                            self.commit_edit();
                            return EventResult::Consumed;
                        }
                        Key::Escape => {
                            self.edit_text = None;
                            return EventResult::Consumed;
                        }
                        Key::ArrowUp => {
                            self.step_value(1.0);
                            return EventResult::Consumed;
                        }
                        Key::ArrowDown => {
                            self.step_value(-1.0);
                            return EventResult::Consumed;
                        }
                        _ => {}
                    }
                }
            }

            UiEvent::ScrollWheel { delta } => {
                if focused {
                    // Positive delta = scroll down = decrease value.
                    self.step_value(-(*delta as f64).signum());
                    return EventResult::Consumed;
                }
            }

            UiEvent::FocusLost => {
                self.commit_edit();
            }

            _ => {}
        }

        EventResult::Ignored
    }
}
