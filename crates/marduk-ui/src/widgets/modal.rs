use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;
use marduk_engine::text::FontId;

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::{Element, Widget};

/// A centered overlay panel that blocks input behind it.
///
/// Renders a dim backdrop when `open` is `true` and a centered dialog box on
/// top.  Clicks on the backdrop call `on_dismiss`.
///
/// # Example
/// ```rust,ignore
/// Modal::new(true)
///     .title("Confirm Delete")
///     .child(
///         Column::new()
///             .child(Text::new("Are you sure?", font, 16.0, white))
///             .child(Row::new()
///                 .child(Button::new(…).on_click(on_confirm))
///                 .child(Button::new(…).on_click(on_cancel))
///             )
///     )
///     .on_dismiss(|| { /* close */ })
/// ```
pub struct Modal {
    child: Element,
    open: bool,
    on_dismiss: Option<Box<dyn FnMut()>>,
    title: Option<String>,
    max_width: f32,

    font: Option<FontId>,
    font_size: f32,
    title_color: Color,
    backdrop_color: Color,
    bg: Color,
    border_color: Color,
    corner_radius: f32,
}

impl Modal {
    pub fn new(open: bool) -> Self {
        Self {
            child: crate::widgets::container::Container::new().into(),
            open,
            on_dismiss: None,
            title: None,
            max_width: 480.0,

            font: None,
            font_size: 16.0,
            title_color:    Color::from_srgb(1.0, 1.0, 1.0, 1.0),
            backdrop_color: Color::from_srgb(0.0, 0.0, 0.0, 0.6),
            bg:             Color::from_srgb(0.16, 0.16, 0.20, 1.0),
            border_color:   Color::from_srgb(0.3, 0.3, 0.36, 1.0),
            corner_radius: 8.0,
        }
    }

    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.child = child.into();
        self
    }

    pub fn open(mut self, v: bool) -> Self {
        self.open = v;
        self
    }

    pub fn on_dismiss(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_dismiss = Some(Box::new(f));
        self
    }

    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }

    pub fn max_width(mut self, w: f32) -> Self {
        self.max_width = w;
        self
    }

    pub fn font(mut self, id: FontId) -> Self {
        self.font = Some(id);
        self
    }

    pub fn font_size(mut self, s: f32) -> Self {
        self.font_size = s;
        self
    }

    pub fn bg(mut self, c: Color) -> Self {
        self.bg = c;
        self
    }

    pub fn backdrop_color(mut self, c: Color) -> Self {
        self.backdrop_color = c;
        self
    }

    pub fn corner_radius(mut self, r: f32) -> Self {
        self.corner_radius = r;
        self
    }

    // ── helpers ───────────────────────────────────────────────────────────

    fn dialog_rect(&self, viewport: Rect, content_h: f32) -> Rect {
        let title_h = if self.title.is_some() { self.font_size * 1.6 + 16.0 } else { 0.0 };
        let pad = 24.0;
        let w = self.max_width.min(viewport.size.x - 48.0);
        let h = (title_h + content_h + pad * 2.0).min(viewport.size.y - 48.0);
        let x = viewport.origin.x + (viewport.size.x - w) * 0.5;
        let y = viewport.origin.y + (viewport.size.y - h) * 0.5;
        Rect::new(x, y, w, h)
    }

    fn content_rect(&self, dialog: Rect) -> Rect {
        let title_h = if self.title.is_some() { self.font_size * 1.6 + 16.0 } else { 0.0 };
        let pad = 24.0;
        Rect::new(
            dialog.origin.x + pad,
            dialog.origin.y + title_h + pad,
            dialog.size.x - pad * 2.0,
            (dialog.size.y - title_h - pad * 2.0).max(0.0),
        )
    }
}

impl Widget for Modal {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        // Modal takes the full available space.
        let w = if constraints.max.x.is_finite() { constraints.max.x } else { 0.0 };
        let h = if constraints.max.y.is_finite() { constraints.max.y } else { 0.0 };
        Vec2::new(w, h)
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        if !self.open { return; }

        // Compute dialog size based on child's natural height.
        let ctx = painter.layout_ctx();
        let content_c = Constraints::loose(Vec2::new(self.max_width - 48.0, f32::INFINITY));
        let content_sz = self.child.measure(content_c, &ctx);
        let dialog = self.dialog_rect(rect, content_sz.y);
        let content = self.content_rect(dialog);

        let backdrop_color = self.backdrop_color;
        let bg             = self.bg;
        let border_color   = self.border_color;
        let corner         = self.corner_radius;
        let title          = self.title.clone();
        let font           = self.font;
        let font_size      = self.font_size;
        let title_color    = self.title_color;

        painter.register_overlay(dialog);
        painter.overlay_scope(|p| {
            // Dim backdrop.
            p.fill_rect(rect, backdrop_color);

            // Dialog background.
            p.fill_rounded_rect(
                dialog,
                corner,
                Paint::Solid(bg),
                Some(Border::new(1.0, border_color)),
            );

            // Title bar.
            if let (Some(title_str), Some(f)) = (&title, font) {
                let title_h = font_size * 1.6 + 16.0;
                let title_bar = Rect::new(dialog.origin.x, dialog.origin.y, dialog.size.x, title_h);
                let sep = Rect::new(dialog.origin.x, dialog.origin.y + title_h - 1.0, dialog.size.x, 1.0);
                p.fill_rect(sep, border_color);
                let m = p.measure_text(title_str, f, font_size, None);
                p.text(
                    title_str.clone(),
                    f,
                    font_size,
                    title_color,
                    Vec2::new(
                        title_bar.origin.x + 16.0,
                        title_bar.origin.y + (title_h - m.y) * 0.5,
                    ),
                    None,
                );
            }

            // Child content.
            p.push_clip(content);
        });

        // Paint child outside the overlay_scope so it uses the normal z range,
        // but it's clipped to the content area which is inside the dialog.
        // Since overlay draws happen at z=100_000 and child draws happen after
        // at z < 100_000, the overlay background will be on top unless we also
        // draw the child in the overlay scope.  So we include child paint inside.
        painter.overlay_scope(|p| {
            p.push_clip(content);
            self.child.paint(p, content);
            p.pop_clip();
        });
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        if !self.open { return EventResult::Ignored; }

        let content_c = Constraints::loose(Vec2::new(self.max_width - 48.0, f32::INFINITY));
        let content_sz = self.child.measure(content_c, ctx);
        let dialog = self.dialog_rect(rect, content_sz.y);
        let content = self.content_rect(dialog);

        match event {
            UiEvent::Click { pos } => {
                if dialog.contains(*pos) {
                    // Route to child.
                    let result = self.child.on_event(event, content, ctx);
                    // Always consume to block underlying widgets.
                    let _ = result;
                    return EventResult::Consumed;
                } else {
                    // Click outside dialog → dismiss.
                    if let Some(f) = &mut self.on_dismiss {
                        f();
                    }
                    return EventResult::Consumed;
                }
            }
            UiEvent::OverlayDismiss => {
                if let Some(f) = &mut self.on_dismiss {
                    f();
                }
                return EventResult::Consumed;
            }
            UiEvent::KeyPress { key: marduk_engine::input::Key::Escape, .. } => {
                if let Some(f) = &mut self.on_dismiss {
                    f();
                }
                return EventResult::Consumed;
            }
            UiEvent::Hover { .. } => {
                // Always visit child for hover.
                self.child.on_event(event, content, ctx);
            }
            _ => {
                return self.child.on_event(event, content, ctx);
            }
        }

        EventResult::Ignored
    }
}
