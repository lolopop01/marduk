use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::input::Key;
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;
use marduk_engine::text::FontId;

use crate::constraints::{Constraints, Edges, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::focus::FocusId;
use crate::painter::Painter;
use crate::widget::Widget;
use crate::widgets::text_edit::TextEditState;

/// A single-line text input field with cursor, selection, and clipboard support.
///
/// Click to place cursor, drag to select, Shift+Arrows to extend selection,
/// Ctrl+A/C/X/V for select-all/copy/cut/paste. Long text scrolls horizontally
/// to keep the cursor visible.
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
    edit:             TextEditState,
    /// Suppresses a `Click` cursor-set on the frame a drag ends.
    drag_was_active:  bool,
    focused:          bool,
    /// Stable identity for the focus manager (Tab-key cycling, FocusGained/FocusLost).
    ///
    /// Allocated once in `new()`. The `focused` bool above remains the primary
    /// source of truth for visual state; `focus_id` bridges to `FocusManager`.
    focus_id:         FocusId,
    font:             Option<FontId>,
    font_size:        f32,
    text_color:       Color,
    placeholder:      String,
    placeholder_color: Color,
    bg:               Color,
    focused_bg:       Color,
    border_color:     Color,
    focused_border_color: Color,
    corner_radius:    f32,
    padding:          Edges,
    selection_color:  Color,
    on_change:        Option<Box<dyn FnMut(String)>>,
    on_submit:        Option<Box<dyn FnMut(String)>>,
    /// Fired when this box gains focus.
    on_focus:         Option<Box<dyn FnMut()>>,
    /// Fired after any cursor / selection / scroll change.
    /// Args: (cursor_byte, anchor_byte, scroll_offset)
    on_cursor_change: Option<Box<dyn FnMut(usize, usize, f32)>>,
}

impl TextBox {
    pub fn new() -> Self {
        Self {
            edit:                 TextEditState::new(""),
            drag_was_active:      false,
            focused:              false,
            focus_id:             FocusId::new(),
            font:                 None,
            font_size:            13.0,
            text_color:           Color::from_straight(0.9, 0.92, 0.95, 1.0),
            placeholder:          String::new(),
            placeholder_color:    Color::from_straight(0.35, 0.45, 0.55, 1.0),
            bg:                   Color::from_straight(0.06, 0.1, 0.16, 1.0),
            focused_bg:           Color::from_straight(0.07, 0.14, 0.22, 1.0),
            border_color:         Color::from_straight(0.18, 0.28, 0.42, 1.0),
            focused_border_color: Color::from_straight(0.0, 0.67, 1.0, 1.0),
            corner_radius:        4.0,
            padding:              Edges::symmetric(6.0, 10.0),
            selection_color:      Color::from_straight(0.1, 0.4, 0.9, 0.4),
            on_change:            None,
            on_submit:            None,
            on_focus:             None,
            on_cursor_change:     None,
        }
    }

    // ── content / state ───────────────────────────────────────────────────

    pub fn text(mut self, v: impl Into<String>) -> Self {
        let s: String = v.into();
        let end = s.len();
        self.edit.text   = s;
        self.edit.cursor = end;
        self.edit.anchor = end;
        self
    }

    pub fn cursor(mut self, v: usize) -> Self {
        self.edit.cursor = v.min(self.edit.text.len());
        self
    }

    pub fn anchor(mut self, v: usize) -> Self {
        self.edit.anchor = v.min(self.edit.text.len());
        self
    }

    pub fn scroll_offset(mut self, v: f32) -> Self {
        self.edit.scroll_offset = v;
        self
    }

    pub fn focused(mut self, v: bool) -> Self { self.focused = v; self }

    // ── style ─────────────────────────────────────────────────────────────

    pub fn placeholder(mut self, v: impl Into<String>) -> Self { self.placeholder = v.into(); self }
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
    pub fn selection_color(mut self, v: Color) -> Self { self.selection_color = v; self }

    // ── callbacks ─────────────────────────────────────────────────────────

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
    pub fn on_cursor_change(mut self, f: impl FnMut(usize, usize, f32) + 'static) -> Self {
        self.on_cursor_change = Some(Box::new(f));
        self
    }

    // ── helpers ───────────────────────────────────────────────────────────

    fn notify_cursor_change(&mut self) {
        if let Some(f) = &mut self.on_cursor_change {
            f(self.edit.cursor, self.edit.anchor, self.edit.scroll_offset);
        }
    }

    fn fire_change(&mut self) {
        if let Some(f) = &mut self.on_change {
            f(self.edit.text.clone());
        }
    }

    /// Compute the inner content rect from a widget rect.
    fn inner_rect(&self, rect: Rect) -> Rect {
        Rect::new(
            rect.origin.x + self.padding.left,
            rect.origin.y + self.padding.top,
            (rect.size.x - self.padding.h()).max(0.0),
            (rect.size.y - self.padding.v()).max(0.0),
        )
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
        // Register as focusable so Tab-key cycling can reach this widget.
        painter.register_focusable(self.focus_id);

        // Focused state: either explicitly set (DSL / click) or via FocusManager (Tab cycling).
        let is_focused = self.focused || painter.is_focused(self.focus_id);

        let bg = if is_focused { self.focused_bg } else { self.bg };
        let border_color = if is_focused { self.focused_border_color } else { self.border_color };
        let border_width = if is_focused { 2.0 } else { 1.0 };

        painter.fill_rounded_rect(
            rect, self.corner_radius, Paint::Solid(bg),
            Some(Border::new(border_width, border_color)),
        );

        let inner = self.inner_rect(rect);
        let text_y = inner.origin.y + (inner.size.y - self.font_size) * 0.5;

        let Some(font) = self.font else { return };

        painter.push_clip(inner);

        if self.edit.text.is_empty() && !self.placeholder.is_empty() {
            painter.text(
                &self.placeholder, font, self.font_size,
                self.placeholder_color,
                Vec2::new(inner.origin.x, text_y),
                Some(inner.size.x),
            );
        } else {
            let scroll = self.edit.scroll_offset;
            let scale  = painter.scale;
            let fs     = painter.font_system;

            // Selection highlight
            if is_focused && self.edit.has_selection() {
                let (lo, hi) = self.edit.sel_range();
                let x0 = fs.measure_text_scaled(&self.edit.text[..lo], font, self.font_size, None, scale).x
                    - scroll;
                let x1 = fs.measure_text_scaled(&self.edit.text[..hi], font, self.font_size, None, scale).x
                    - scroll;
                let sel_x  = inner.origin.x + x0;
                let sel_w  = (x1 - x0).max(0.0);
                painter.fill_rounded_rect(
                    Rect::new(sel_x, inner.origin.y, sel_w, inner.size.y),
                    0.0,
                    Paint::Solid(self.selection_color),
                    None,
                );
            }

            // Text — shifted left by scroll_offset; allow overshooting to the right
            painter.text(
                &self.edit.text, font, self.font_size, self.text_color,
                Vec2::new(inner.origin.x - scroll, text_y),
                Some(inner.size.x + scroll),
            );

            // Cursor bar — only when nothing is selected
            if is_focused && !self.edit.has_selection() {
                let scale = painter.scale;
                let fs    = painter.font_system;
                let cx = fs.measure_text_scaled(
                    &self.edit.text[..self.edit.cursor],
                    font, self.font_size, None, scale,
                ).x;
                let bar_x = (inner.origin.x + cx - self.edit.scroll_offset + 1.0)
                    .min(inner.origin.x + inner.size.x - 2.0);
                painter.fill_rounded_rect(
                    Rect::new(bar_x, inner.origin.y, 2.0, inner.size.y),
                    1.0,
                    Paint::Solid(self.focused_border_color),
                    None,
                );
            }
        }

        painter.pop_clip();
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        // Focused state: either explicitly set (DSL / click) or via FocusManager (Tab cycling).
        let fm_focused = ctx.is_focused(self.focus_id);

        match event {
            // ── FocusGained: sync self.focused when Tab cycles to this widget ─
            UiEvent::FocusGained => {
                if fm_focused {
                    self.focused = true;
                    if let Some(f) = &mut self.on_focus { f(); }
                }
                EventResult::Ignored
            }

            // ── FocusLost: sync self.focused when Tab cycles away ─────────
            UiEvent::FocusLost => {
                if !fm_focused {
                    self.focused = false;
                }
                EventResult::Ignored
            }

            // ── Click: place cursor ────────────────────────────────────────
            UiEvent::Click { pos } => {
                if rect.contains(*pos) {
                    // Suppress cursor placement if a drag just ended here.
                    if self.drag_was_active {
                        self.drag_was_active = false;
                        return EventResult::Consumed;
                    }
                    if !self.focused {
                        self.focused = true;
                        if let Some(f) = &mut self.on_focus { f(); }
                    }
                    // Also register with the focus manager so Tab cycling stays in sync.
                    ctx.request_focus(self.focus_id);
                    if let Some(font) = self.font {
                        let inner = self.inner_rect(rect);
                        let rel_x = pos.x - inner.origin.x + self.edit.scroll_offset;
                        let c = self.edit.x_to_cursor(
                            rel_x, font, self.font_size, ctx.fonts, ctx.scale,
                        );
                        self.edit.cursor = c;
                        self.edit.anchor = c;
                        self.notify_cursor_change();
                    }
                    EventResult::Consumed
                } else {
                    self.focused = false;
                    EventResult::Ignored
                }
            }

            // ── Drag: extend selection ─────────────────────────────────────
            //
            // NOTE: Drag fires on the button-press frame, BEFORE Click fires
            // (Click only fires on release). So we must gain focus here on the
            // first drag frame, not wait for Click.
            UiEvent::Drag { pos, start } => {
                if rect.contains(*start) {
                    // Gain focus on the first drag frame if not already focused.
                    if !self.focused && !fm_focused {
                        self.focused = true;
                        ctx.request_focus(self.focus_id);
                        if let Some(f) = &mut self.on_focus { f(); }
                    }
                    if let Some(font) = self.font {
                        self.drag_was_active = true;
                        let inner = self.inner_rect(rect);
                        let scroll = self.edit.scroll_offset;

                        let anchor_rel = start.x - inner.origin.x + scroll;
                        self.edit.anchor = self.edit.x_to_cursor(
                            anchor_rel, font, self.font_size, ctx.fonts, ctx.scale,
                        );

                        let cursor_rel = pos.x - inner.origin.x + scroll;
                        self.edit.cursor = self.edit.x_to_cursor(
                            cursor_rel, font, self.font_size, ctx.fonts, ctx.scale,
                        );

                        self.edit.ensure_cursor_visible(
                            inner.size.x, font, self.font_size, ctx.fonts, ctx.scale,
                        );
                        self.notify_cursor_change();
                    }
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            // ── Text input ─────────────────────────────────────────────────
            UiEvent::TextInput { text } => {
                if !self.focused && !fm_focused { return EventResult::Ignored; }
                self.edit.insert_str(text);
                if let Some(font) = self.font {
                    let inner = self.inner_rect(rect);
                    self.edit.ensure_cursor_visible(
                        inner.size.x, font, self.font_size, ctx.fonts, ctx.scale,
                    );
                }
                self.fire_change();
                self.notify_cursor_change();
                EventResult::Consumed
            }

            // ── Key presses ────────────────────────────────────────────────
            UiEvent::KeyPress { key, modifiers } => {
                if !self.focused && !fm_focused { return EventResult::Ignored; }

                let shift = modifiers.shift;
                let ctrl  = modifiers.ctrl;

                match key {
                    Key::Backspace => {
                        self.edit.delete_backward();
                        self.ensure_visible_and_notify(rect, ctx);
                        self.fire_change();
                        EventResult::Consumed
                    }
                    Key::Delete => {
                        self.edit.delete_forward();
                        self.ensure_visible_and_notify(rect, ctx);
                        self.fire_change();
                        EventResult::Consumed
                    }
                    Key::ArrowLeft => {
                        if ctrl { self.edit.move_word_left(shift); }
                        else     { self.edit.move_left(shift); }
                        self.ensure_visible_and_notify(rect, ctx);
                        EventResult::Consumed
                    }
                    Key::ArrowRight => {
                        if ctrl { self.edit.move_word_right(shift); }
                        else     { self.edit.move_right(shift); }
                        self.ensure_visible_and_notify(rect, ctx);
                        EventResult::Consumed
                    }
                    Key::Home => {
                        self.edit.move_home(shift);
                        self.ensure_visible_and_notify(rect, ctx);
                        EventResult::Consumed
                    }
                    Key::End => {
                        self.edit.move_end(shift);
                        self.ensure_visible_and_notify(rect, ctx);
                        EventResult::Consumed
                    }
                    Key::A if ctrl => {
                        self.edit.select_all();
                        self.notify_cursor_change();
                        EventResult::Consumed
                    }
                    Key::C if ctrl => {
                        self.edit.copy();
                        EventResult::Consumed
                    }
                    Key::X if ctrl => {
                        if self.edit.cut() {
                            self.ensure_visible_and_notify(rect, ctx);
                            self.fire_change();
                        }
                        EventResult::Consumed
                    }
                    Key::V if ctrl => {
                        if self.edit.paste() {
                            self.ensure_visible_and_notify(rect, ctx);
                            self.fire_change();
                        }
                        EventResult::Consumed
                    }
                    Key::Enter => {
                        if let Some(f) = &mut self.on_submit { f(self.edit.text.clone()); }
                        EventResult::Consumed
                    }
                    Key::Escape => {
                        self.focused = false;
                        EventResult::Consumed
                    }
                    _ => EventResult::Ignored,
                }
            }

            // ── DragEnd: arm the Click suppression ────────────────────────
            //
            // In DSL mode the widget is rebuilt fresh every frame, so
            // `drag_was_active` is always false when Click fires — unless we
            // set it here.  DragEnd is dispatched *before* Click on the
            // release frame, so setting the flag here lets Click see it.
            UiEvent::DragEnd { start, .. } => {
                if rect.contains(*start) {
                    self.drag_was_active = true;
                }
                EventResult::Ignored
            }

            _ => EventResult::Ignored,
        }
    }
}

impl TextBox {
    fn ensure_visible_and_notify(&mut self, rect: Rect, ctx: &LayoutCtx<'_>) {
        if let Some(font) = self.font {
            let inner = self.inner_rect(rect);
            self.edit.ensure_cursor_visible(
                inner.size.x, font, self.font_size, ctx.fonts, ctx.scale,
            );
        }
        self.notify_cursor_change();
    }
}
