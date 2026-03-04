use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::input::Key;
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;
use marduk_engine::text::FontId;

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::Widget;

/// A drop-down selection widget.
///
/// Clicking the trigger opens a list of options; clicking an option selects it
/// and closes the dropdown.  `OverlayDismiss` (click outside) and `Escape` also
/// close the dropdown.
///
/// # Example
/// ```rust,ignore
/// Combobox::new()
///     .option("Red",   "red")
///     .option("Green", "green")
///     .option("Blue",  "blue")
///     .selected("green")
///     .on_change(|v| println!("selected: {v}"))
/// ```
pub struct Combobox {
    options: Vec<(String, String)>,
    selected: Option<String>,
    placeholder: String,
    on_change: Option<Box<dyn FnMut(String)>>,

    open: bool,
    /// Called when the dropdown opens/closes; used in DSL mode to persist
    /// the `open` state across frame rebuilds.
    on_open_change: Option<Box<dyn FnMut(bool)>>,

    font: Option<FontId>,
    font_size: f32,
    text_color: Color,
    bg: Color,
    border_color: Color,
    focused_border_color: Color,
    item_bg: Color,
    item_hover_bg: Color,
    item_selected_bg: Color,
    corner_radius: f32,
    height: f32,
    width: f32,
    max_dropdown_height: f32,
}

impl Combobox {
    pub fn new() -> Self {
        Self {
            options: Vec::new(),
            selected: None,
            placeholder: String::from("Select…"),
            on_change: None,

            open: false,
            on_open_change: None,

            font: None,
            font_size: 14.0,
            text_color:           Color::from_srgb(1.0, 1.0, 1.0, 1.0),
            bg:                   Color::from_srgb(0.15, 0.15, 0.18, 1.0),
            border_color:         Color::from_srgb(0.30, 0.30, 0.35, 1.0),
            focused_border_color: Color::from_srgb(0.39, 0.55, 1.0, 1.0),
            item_bg:              Color::from_srgb(0.13, 0.13, 0.16, 1.0),
            item_hover_bg:        Color::from_srgb(0.22, 0.22, 0.27, 1.0),
            item_selected_bg:     Color::from_srgb(0.25, 0.38, 0.70, 1.0),
            corner_radius: 4.0,
            height: 32.0,
            width: 160.0,
            max_dropdown_height: 200.0,
        }
    }

    pub fn option(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.push((label.into(), value.into()));
        self
    }

    pub fn selected(mut self, value: impl Into<String>) -> Self {
        self.selected = Some(value.into());
        self
    }

    pub fn placeholder(mut self, s: impl Into<String>) -> Self {
        self.placeholder = s.into();
        self
    }

    pub fn on_change(mut self, f: impl FnMut(String) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    pub fn open(mut self, v: bool) -> Self {
        self.open = v;
        self
    }

    /// Called when the dropdown opens or closes.  Use in DSL mode to persist
    /// the open/close state across frame rebuilds.
    pub fn on_open_change(mut self, f: impl FnMut(bool) + 'static) -> Self {
        self.on_open_change = Some(Box::new(f));
        self
    }

    pub fn font(mut self, id: FontId) -> Self { self.font = Some(id); self }
    pub fn font_size(mut self, s: f32) -> Self { self.font_size = s; self }
    pub fn text_color(mut self, c: Color) -> Self { self.text_color = c; self }
    pub fn bg(mut self, c: Color) -> Self { self.bg = c; self }
    pub fn border_color(mut self, c: Color) -> Self { self.border_color = c; self }
    pub fn focused_border_color(mut self, c: Color) -> Self { self.focused_border_color = c; self }
    pub fn item_bg(mut self, c: Color) -> Self { self.item_bg = c; self }
    pub fn item_hover_bg(mut self, c: Color) -> Self { self.item_hover_bg = c; self }
    pub fn corner_radius(mut self, r: f32) -> Self { self.corner_radius = r; self }
    pub fn height(mut self, h: f32) -> Self { self.height = h; self }
    pub fn width(mut self, w: f32) -> Self { self.width = w; self }
    pub fn max_dropdown_height(mut self, h: f32) -> Self { self.max_dropdown_height = h; self }

    // ── helpers ───────────────────────────────────────────────────────────

    fn dropdown_rect(&self, rect: Rect) -> Rect {
        let n = self.options.len();
        let h = (n as f32 * self.height).min(self.max_dropdown_height);
        Rect::new(rect.origin.x, rect.origin.y + rect.size.y, rect.size.x, h)
    }

    fn selected_label(&self) -> &str {
        self.selected
            .as_deref()
            .and_then(|v| {
                self.options
                    .iter()
                    .find(|(_, val)| val == v)
                    .map(|(l, _)| l.as_str())
            })
            .unwrap_or(&self.placeholder)
    }
}

impl Default for Combobox {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Combobox {
    fn measure(&self, _constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        Vec2::new(self.width, self.height)
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let border_col = if self.open {
            self.focused_border_color
        } else {
            self.border_color
        };
        let bg = if painter.is_pressed(rect) {
            Color::from_srgb(0.22, 0.22, 0.26, 1.0)
        } else if painter.is_hovered(rect) || self.open {
            Color::from_srgb(0.20, 0.20, 0.24, 1.0)
        } else {
            self.bg
        };

        painter.fill_rounded_rect(rect, self.corner_radius, Paint::Solid(bg), Some(Border::new(1.0, border_col)));

        if let Some(font) = self.font {
            let pad = 8.0;
            let arrow_w = self.height * 0.7;
            let text_clip = Rect::new(
                rect.origin.x + pad,
                rect.origin.y,
                rect.size.x - arrow_w - pad * 2.0,
                rect.size.y,
            );
            let text_y = rect.origin.y + (rect.size.y - self.font_size * 1.2) * 0.5;
            painter.push_clip(text_clip);
            painter.text(
                self.selected_label().to_string(),
                font,
                self.font_size,
                self.text_color,
                Vec2::new(rect.origin.x + pad, text_y),
                None,
            );
            painter.pop_clip();

            let arrow = if self.open { "▲" } else { "▼" };
            let arrow_sz = self.font_size * 0.75;
            let arrow_m = painter.measure_text(arrow, font, arrow_sz, None);
            let arrow_x = rect.origin.x + rect.size.x - arrow_w * 0.5 - arrow_m.x * 0.5;
            let arrow_y = rect.origin.y + (rect.size.y - arrow_m.y) * 0.5;
            painter.text(arrow.to_string(), font, arrow_sz, self.text_color, Vec2::new(arrow_x, arrow_y), None);
        }

        if self.open {
            let dropdown = self.dropdown_rect(rect);

            // Register both trigger and dropdown so clicks on them don't fire
            // OverlayDismiss while the dropdown is open.
            painter.register_overlay(rect);
            painter.register_overlay(dropdown);

            let item_bg         = self.item_bg;
            let item_hover_bg   = self.item_hover_bg;
            let item_selected_bg = self.item_selected_bg;
            let border_col      = self.border_color;
            let corner          = self.corner_radius;
            let font            = self.font;
            let font_size       = self.font_size;
            let text_color      = self.text_color;
            let height          = self.height;
            let options         = self.options.clone();
            let selected        = self.selected.clone();

            painter.overlay_scope(|p| {
                p.fill_rounded_rect(
                    dropdown,
                    corner,
                    Paint::Solid(item_bg),
                    Some(Border::new(1.0, border_col)),
                );
                p.push_clip(dropdown);

                for (i, (label, value)) in options.iter().enumerate() {
                    let item = Rect::new(
                        dropdown.origin.x,
                        dropdown.origin.y + i as f32 * height,
                        dropdown.size.x,
                        height,
                    );
                    if item.origin.y >= dropdown.origin.y + dropdown.size.y {
                        break;
                    }

                    let is_selected = selected.as_deref() == Some(value.as_str());
                    let row_bg = if is_selected {
                        item_selected_bg
                    } else if p.is_hovered(item) {
                        item_hover_bg
                    } else {
                        item_bg
                    };
                    // Use rounded_rect (radius=0) so this lands in RoundedRectRenderer
                    // alongside the dropdown background.  Both are then sorted by z within
                    // the same renderer, ensuring the hover highlight renders on top of the
                    // dropdown background rather than being covered by it.
                    p.fill_rounded_rect(item, 0.0, Paint::Solid(row_bg), None);

                    if let Some(f) = font {
                        let pad = 8.0;
                        let text_y = item.origin.y + (height - font_size * 1.2) * 0.5;
                        p.text(
                            label.clone(),
                            f,
                            font_size,
                            text_color,
                            Vec2::new(item.origin.x + pad, text_y),
                            None,
                        );
                    }
                }

                p.pop_clip();
            });
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _ctx: &LayoutCtx<'_>) -> EventResult {
        let dropdown = self.dropdown_rect(rect);

        match event {
            UiEvent::Click { pos } => {
                if rect.contains(*pos) {
                    self.open = !self.open;
                    if let Some(f) = &mut self.on_open_change {
                        f(self.open);
                    }
                    return EventResult::Consumed;
                }
                if self.open && dropdown.contains(*pos) {
                    let idx = ((pos.y - dropdown.origin.y) / self.height) as usize;
                    let max_visible = (dropdown.size.y / self.height).floor() as usize;
                    let idx = idx.min(self.options.len().saturating_sub(1));
                    if idx < self.options.len() && idx < max_visible {
                        let value = self.options[idx].1.clone();
                        self.selected = Some(value.clone());
                        self.open = false;
                        if let Some(f) = &mut self.on_open_change {
                            f(false);
                        }
                        if let Some(f) = &mut self.on_change {
                            f(value);
                        }
                    }
                    return EventResult::Consumed;
                }
            }

            UiEvent::OverlayDismiss => {
                if self.open {
                    self.open = false;
                    if let Some(f) = &mut self.on_open_change {
                        f(false);
                    }
                    return EventResult::Consumed;
                }
            }

            UiEvent::KeyPress { key, .. } => {
                if self.open {
                    match key {
                        Key::Escape => {
                            self.open = false;
                            if let Some(f) = &mut self.on_open_change {
                                f(false);
                            }
                            return EventResult::Consumed;
                        }
                        Key::Enter => {
                            self.open = false;
                            if let Some(f) = &mut self.on_open_change {
                                f(false);
                            }
                            return EventResult::Consumed;
                        }
                        Key::ArrowUp => {
                            let len = self.options.len();
                            if len > 0 {
                                let new_idx = if let Some(cur) = &self.selected {
                                    let i = self.options.iter().position(|(_, v)| v == cur).unwrap_or(0);
                                    i.saturating_sub(1)
                                } else {
                                    0
                                };
                                let value = self.options[new_idx].1.clone();
                                self.selected = Some(value.clone());
                                if let Some(f) = &mut self.on_change {
                                    f(value);
                                }
                            }
                            return EventResult::Consumed;
                        }
                        Key::ArrowDown => {
                            let len = self.options.len();
                            if len > 0 {
                                let new_idx = if let Some(cur) = &self.selected {
                                    let i = self.options.iter().position(|(_, v)| v == cur).unwrap_or(0);
                                    (i + 1).min(len - 1)
                                } else {
                                    0
                                };
                                let value = self.options[new_idx].1.clone();
                                self.selected = Some(value.clone());
                                if let Some(f) = &mut self.on_change {
                                    f(value);
                                }
                            }
                            return EventResult::Consumed;
                        }
                        _ => {}
                    }
                }
            }

            _ => {}
        }

        EventResult::Ignored
    }
}
