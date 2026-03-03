use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::Color;
use marduk_engine::text::FontId;

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::{Element, Widget};

/// A tab bar + content area; clicking a tab reveals its panel.
///
/// # Example
/// ```rust,ignore
/// Tabs::new()
///     .tab("Files",    file_panel.into())
///     .tab("Settings", settings_panel.into())
///     .selected(0)
///     .on_change(|idx| println!("switched to tab {idx}"))
/// ```
pub struct Tabs {
    tabs: Vec<(String, Element)>,
    selected: usize,
    on_change: Option<Box<dyn FnMut(usize)>>,
    tab_height: f32,
    font: Option<FontId>,
    font_size: f32,
    active_color: Color,
    inactive_color: Color,
    active_text_color: Color,
    inactive_text_color: Color,
    indicator_color: Color,
}

impl Tabs {
    pub fn new() -> Self {
        Self {
            tabs: Vec::new(),
            selected: 0,
            on_change: None,
            tab_height: 36.0,
            font: None,
            font_size: 14.0,
            active_color:         Color::from_srgb(0.18, 0.18, 0.22, 1.0),
            inactive_color:       Color::from_srgb(0.12, 0.12, 0.15, 1.0),
            active_text_color:    Color::from_srgb(1.0, 1.0, 1.0, 1.0),
            inactive_text_color:  Color::from_srgb(0.65, 0.65, 0.65, 1.0),
            indicator_color:      Color::from_srgb(0.39, 0.55, 1.0, 1.0),
        }
    }

    pub fn tab(mut self, label: impl Into<String>, content: impl Into<Element>) -> Self {
        self.tabs.push((label.into(), content.into()));
        self
    }

    pub fn selected(mut self, idx: usize) -> Self {
        self.selected = idx;
        self
    }

    pub fn on_change(mut self, f: impl FnMut(usize) + 'static) -> Self {
        self.on_change = Some(Box::new(f));
        self
    }

    pub fn tab_height(mut self, h: f32) -> Self {
        self.tab_height = h;
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

    pub fn active_color(mut self, c: Color) -> Self {
        self.active_color = c;
        self
    }

    pub fn inactive_color(mut self, c: Color) -> Self {
        self.inactive_color = c;
        self
    }

    pub fn indicator_color(mut self, c: Color) -> Self {
        self.indicator_color = c;
        self
    }

    // ── helpers ───────────────────────────────────────────────────────────

    fn tab_rect(&self, rect: Rect, i: usize) -> Rect {
        let n = self.tabs.len().max(1);
        let tab_w = rect.size.x / n as f32;
        Rect::new(rect.origin.x + i as f32 * tab_w, rect.origin.y, tab_w, self.tab_height)
    }

    fn content_rect(&self, rect: Rect) -> Rect {
        Rect::new(
            rect.origin.x,
            rect.origin.y + self.tab_height,
            rect.size.x,
            (rect.size.y - self.tab_height).max(0.0),
        )
    }
}

impl Default for Tabs {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Tabs {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        let w = if constraints.max.x.is_finite() { constraints.max.x } else { 0.0 };
        let content_h = if let Some((_, content)) = self.tabs.get(self.selected) {
            let c = Constraints::loose(Vec2::new(w, (constraints.max.y - self.tab_height).max(0.0)));
            content.measure(c, ctx).y
        } else {
            0.0
        };
        let h = if constraints.max.y.is_finite() {
            constraints.max.y
        } else {
            self.tab_height + content_h
        };
        Vec2::new(w, h)
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let bar = Rect::new(rect.origin.x, rect.origin.y, rect.size.x, self.tab_height);
        painter.fill_rect(bar, self.inactive_color);

        for (i, (label, _)) in self.tabs.iter().enumerate() {
            let tab_rect = self.tab_rect(rect, i);
            let active = i == self.selected;
            let bg = if active { self.active_color } else {
                if painter.is_hovered(tab_rect) {
                    Color::from_srgb(0.16, 0.16, 0.20, 1.0)
                } else {
                    self.inactive_color
                }
            };
            painter.fill_rect(tab_rect, bg);

            if let Some(font) = self.font {
                let text_sz = painter.measure_text(label, font, self.font_size, None);
                let tx = tab_rect.origin.x + (tab_rect.size.x - text_sz.x) * 0.5;
                let ty = tab_rect.origin.y + (self.tab_height - text_sz.y) * 0.5;
                let tc = if active { self.active_text_color } else { self.inactive_text_color };
                painter.text(label.clone(), font, self.font_size, tc, Vec2::new(tx, ty), None);
            }

            if active {
                let indicator = Rect::new(
                    tab_rect.origin.x,
                    tab_rect.origin.y + self.tab_height - 2.0,
                    tab_rect.size.x,
                    2.0,
                );
                painter.fill_rect(indicator, self.indicator_color);
            }
        }

        let content_rect = self.content_rect(rect);
        if let Some((_, content)) = self.tabs.get(self.selected) {
            content.paint(painter, content_rect);
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        let bar = Rect::new(rect.origin.x, rect.origin.y, rect.size.x, self.tab_height);

        if let UiEvent::Click { pos } = event {
            if bar.contains(*pos) {
                let n = self.tabs.len().max(1);
                let tab_w = rect.size.x / n as f32;
                let i = ((pos.x - rect.origin.x) / tab_w) as usize;
                let i = i.min(self.tabs.len().saturating_sub(1));
                if i != self.selected {
                    self.selected = i;
                    if let Some(f) = &mut self.on_change {
                        f(i);
                    }
                }
                return EventResult::Consumed;
            }
        }

        // Route keyboard events to the active tab content
        if let UiEvent::KeyPress { key: marduk_engine::input::Key::ArrowLeft, .. } = event {
            if self.selected > 0 {
                self.selected -= 1;
                if let Some(f) = &mut self.on_change { f(self.selected); }
                return EventResult::Consumed;
            }
        }
        if let UiEvent::KeyPress { key: marduk_engine::input::Key::ArrowRight, .. } = event {
            if self.selected + 1 < self.tabs.len() {
                self.selected += 1;
                if let Some(f) = &mut self.on_change { f(self.selected); }
                return EventResult::Consumed;
            }
        }

        let content_rect = self.content_rect(rect);
        if let Some((_, content)) = self.tabs.get_mut(self.selected) {
            content.on_event(event, content_rect, ctx)
        } else {
            EventResult::Ignored
        }
    }
}
