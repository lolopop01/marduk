use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;
use marduk_engine::text::FontId;

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::{Element, Widget};

/// A wrapper that shows a tooltip label after the cursor hovers for a delay.
///
/// The tooltip is drawn in the overlay layer (above all other content) near the
/// widget rect when the hover duration exceeds `delay_ms`.
///
/// # Example
/// ```rust,ignore
/// Tooltip::new(button_widget)
///     .text("Save the current file (Ctrl+S)")
///     .delay_ms(400)
/// ```
pub struct Tooltip {
    child: Element,
    text: String,
    delay_ms: u64,
    font: Option<FontId>,
    font_size: f32,
    text_color: Color,
    bg: Color,
    corner_radius: f32,
    padding: f32,

    /// Time (in app ms) when the cursor first entered the child's rect.
    /// `None` means not hovering.
    hover_since_ms: Option<u64>,

    /// Called when hover state changes (`true` = entered with timestamp; `false` = exited).
    /// Used in DSL mode to persist the hover-start timestamp across frame rebuilds.
    on_hover_change: Option<Box<dyn FnMut(bool, u64)>>,
}

impl Tooltip {
    pub fn new(child: impl Into<Element>) -> Self {
        Self {
            child: child.into(),
            text: String::new(),
            delay_ms: 400,
            font: None,
            font_size: 12.0,
            text_color: Color::from_srgb(0.95, 0.95, 0.95, 1.0),
            bg:         Color::from_srgb(0.12, 0.12, 0.15, 0.97),
            corner_radius: 4.0,
            padding: 6.0,

            hover_since_ms: None,
            on_hover_change: None,
        }
    }

    pub fn text(mut self, t: impl Into<String>) -> Self {
        self.text = t.into();
        self
    }

    pub fn delay_ms(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }

    pub fn font(mut self, id: FontId) -> Self {
        self.font = Some(id);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn text_color(mut self, c: Color) -> Self {
        self.text_color = c;
        self
    }

    pub fn bg(mut self, c: Color) -> Self {
        self.bg = c;
        self
    }

    /// Called when hover state changes; used in DSL mode to persist state across rebuilds.
    pub fn on_hover_change(mut self, f: impl FnMut(bool, u64) + 'static) -> Self {
        self.on_hover_change = Some(Box::new(f));
        self
    }

    /// Restore a previously-persisted hover start time (DSL mode).
    pub fn hover_since_ms(mut self, ms: Option<u64>) -> Self {
        self.hover_since_ms = ms;
        self
    }

    fn is_ready(&self, time_ms: u64) -> bool {
        self.hover_since_ms
            .is_some_and(|start| time_ms.saturating_sub(start) >= self.delay_ms)
    }
}

impl Widget for Tooltip {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        self.child.measure(constraints, ctx)
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        self.child.paint(painter, rect);

        if self.text.is_empty() || !self.is_ready(painter.time_ms) {
            return;
        }
        let Some(font) = self.font else { return; };

        let pad = self.padding;
        let text_sz = painter.measure_text(&self.text, font, self.font_size, Some(320.0));
        let tip_w = text_sz.x + pad * 2.0;
        let tip_h = text_sz.y + pad * 2.0;

        // Position below the widget, left-aligned.
        let tip_x = rect.origin.x;
        let tip_y = rect.origin.y + rect.size.y + 4.0;
        let tip_rect = Rect::new(tip_x, tip_y, tip_w, tip_h);

        let text_color  = self.text_color;
        let bg          = self.bg;
        let corner      = self.corner_radius;
        let font_size   = self.font_size;
        let text        = self.text.clone();

        painter.register_overlay(tip_rect);
        painter.overlay_scope(|p| {
            p.fill_rounded_rect(
                tip_rect,
                corner,
                Paint::Solid(bg),
                Some(Border::new(1.0, Color::from_srgb(0.3, 0.3, 0.35, 1.0))),
            );
            p.text(
                text,
                font,
                font_size,
                text_color,
                Vec2::new(tip_x + pad, tip_y + pad),
                Some(320.0),
            );
        });
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        if let UiEvent::Hover { pos } = event {
            let hovered = rect.contains(*pos);
            match (hovered, self.hover_since_ms) {
                (true, None) => {
                    self.hover_since_ms = Some(ctx.time_ms);
                    if let Some(f) = &mut self.on_hover_change {
                        f(true, ctx.time_ms);
                    }
                }
                (false, Some(_)) => {
                    self.hover_since_ms = None;
                    if let Some(f) = &mut self.on_hover_change {
                        f(false, ctx.time_ms);
                    }
                }
                _ => {}
            }
        }

        self.child.on_event(event, rect, ctx)
    }
}
