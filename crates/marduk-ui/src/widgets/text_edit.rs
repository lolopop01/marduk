use marduk_engine::input::Key;
use marduk_engine::text::{FontId, FontSystem};
use marduk_engine::coords::Vec2;

use crate::event::UiEvent;

/// Reusable text-editing state (cursor, anchor, horizontal scroll).
///
/// Embed this in TextBox, ComboBox, or any widget that needs single-line text
/// editing. All methods maintain valid UTF-8 byte boundaries.
pub struct TextEditState {
    pub text:          String,
    /// Byte offset of the cursor (caret), always on a char boundary.
    pub cursor:        usize,
    /// Byte offset of the selection anchor. `cursor == anchor` means no selection.
    pub anchor:        usize,
    /// Horizontal pixel offset the text has been scrolled to the left.
    pub scroll_offset: f32,
}

impl TextEditState {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let end = text.len();
        Self { text, cursor: end, anchor: end, scroll_offset: 0.0 }
    }

    /// Returns `(lo, hi)` sorted byte range of the selection.
    #[inline]
    pub fn sel_range(&self) -> (usize, usize) {
        (self.cursor.min(self.anchor), self.cursor.max(self.anchor))
    }

    /// True when any text is selected (cursor != anchor).
    #[inline]
    pub fn has_selection(&self) -> bool {
        self.cursor != self.anchor
    }

    // ── cursor movement ───────────────────────────────────────────────────

    /// Move one character to the left.
    ///
    /// If there is a selection and `shift` is false, collapses to the left edge.
    pub fn move_left(&mut self, shift: bool) {
        if !shift && self.has_selection() {
            let left = self.sel_range().0;
            self.cursor = left;
            self.anchor = left;
        } else {
            self.cursor = prev_char(&self.text, self.cursor);
            if !shift { self.anchor = self.cursor; }
        }
    }

    /// Move one character to the right.
    pub fn move_right(&mut self, shift: bool) {
        if !shift && self.has_selection() {
            let right = self.sel_range().1;
            self.cursor = right;
            self.anchor = right;
        } else {
            self.cursor = next_char(&self.text, self.cursor);
            if !shift { self.anchor = self.cursor; }
        }
    }

    /// Move one word to the left (Ctrl+←).
    pub fn move_word_left(&mut self, shift: bool) {
        self.cursor = prev_word(&self.text, self.cursor);
        if !shift { self.anchor = self.cursor; }
    }

    /// Move one word to the right (Ctrl+→).
    pub fn move_word_right(&mut self, shift: bool) {
        self.cursor = next_word(&self.text, self.cursor);
        if !shift { self.anchor = self.cursor; }
    }

    /// Move to start of text.
    pub fn move_home(&mut self, shift: bool) {
        self.cursor = 0;
        if !shift { self.anchor = self.cursor; }
    }

    /// Move to end of text.
    pub fn move_end(&mut self, shift: bool) {
        self.cursor = self.text.len();
        if !shift { self.anchor = self.cursor; }
    }

    /// Select all text.
    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.cursor = self.text.len();
    }

    // ── editing ───────────────────────────────────────────────────────────

    /// Insert `s` at the cursor (replaces selection if any).
    pub fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.anchor = self.cursor;
    }

    /// Delete one character backward (Backspace). Deletes selection if any.
    pub fn delete_backward(&mut self) {
        if self.delete_selection() { return; }
        if self.cursor == 0 { return; }
        let prev = prev_char(&self.text, self.cursor);
        self.text.drain(prev..self.cursor);
        self.cursor = prev;
        self.anchor = prev;
    }

    /// Delete one character forward (Delete key). Deletes selection if any.
    pub fn delete_forward(&mut self) {
        if self.delete_selection() { return; }
        if self.cursor >= self.text.len() { return; }
        let next = next_char(&self.text, self.cursor);
        self.text.drain(self.cursor..next);
        // cursor stays in place
    }

    // ── clipboard ─────────────────────────────────────────────────────────

    /// Copy selection to the system clipboard. No-op if nothing is selected.
    pub fn copy(&self) {
        let (lo, hi) = self.sel_range();
        if lo == hi { return; }
        let selected = self.text[lo..hi].to_string();
        if let Ok(mut cb) = arboard::Clipboard::new() {
            let _ = cb.set_text(selected);
        }
    }

    /// Cut selection to clipboard; returns `true` if text changed.
    pub fn cut(&mut self) -> bool {
        if !self.has_selection() { return false; }
        self.copy();
        self.delete_selection();
        true
    }

    /// Paste from clipboard at cursor; returns `true` if text changed.
    pub fn paste(&mut self) -> bool {
        if let Ok(mut cb) = arboard::Clipboard::new() {
            if let Ok(text) = cb.get_text() {
                self.insert_str(&text);
                return true;
            }
        }
        false
    }

    // ── event handling ────────────────────────────────────────────────────

    /// Handle keyboard text editing events.
    ///
    /// Processes `TextInput` and `KeyPress` events (navigation, clipboard).
    /// Mouse/drag events are intentionally excluded — those need the widget rect
    /// and should be handled by the containing widget.
    ///
    /// Returns `(consumed, text_changed)`:
    /// - `consumed`: the event was handled; callers should return `EventResult::Consumed`.
    /// - `text_changed`: the text content changed; callers should fire their `on_change` callback.
    ///
    /// After a call where `consumed` is true, call [`ensure_cursor_visible`] with
    /// the available inner width to keep the cursor in view.
    pub fn on_event(
        &mut self,
        event: &UiEvent,
        _font: FontId,
        _size: f32,
        _fonts: &FontSystem,
        _scale: f32,
    ) -> (bool, bool) {
        match event {
            UiEvent::TextInput { text } => {
                self.insert_str(text);
                (true, true)
            }

            UiEvent::KeyPress { key, modifiers } => {
                let shift = modifiers.shift;
                let ctrl  = modifiers.ctrl;

                match key {
                    Key::Backspace => {
                        self.delete_backward();
                        (true, true)
                    }
                    Key::Delete => {
                        self.delete_forward();
                        (true, true)
                    }
                    Key::ArrowLeft => {
                        if ctrl { self.move_word_left(shift); }
                        else     { self.move_left(shift); }
                        (true, false)
                    }
                    Key::ArrowRight => {
                        if ctrl { self.move_word_right(shift); }
                        else     { self.move_right(shift); }
                        (true, false)
                    }
                    Key::Home => {
                        self.move_home(shift);
                        (true, false)
                    }
                    Key::End => {
                        self.move_end(shift);
                        (true, false)
                    }
                    Key::A if ctrl => {
                        self.select_all();
                        (true, false)
                    }
                    Key::C if ctrl => {
                        self.copy();
                        (true, false)
                    }
                    Key::X if ctrl => {
                        let changed = self.cut();
                        (true, changed)
                    }
                    Key::V if ctrl => {
                        let changed = self.paste();
                        (true, changed)
                    }
                    // Enter and Escape are intentionally NOT handled here — they have
                    // widget-specific semantics (submit, defocus) that TextEditState
                    // has no knowledge of.
                    _ => (false, false),
                }
            }

            _ => (false, false),
        }
    }

    // ── measurement helpers ───────────────────────────────────────────────

    /// X coordinate (pixels, relative to text origin) of the cursor.
    pub fn cursor_x(&self, font: FontId, size: f32, fs: &FontSystem, scale: f32) -> f32 {
        measure(fs, &self.text[..self.cursor], font, size, scale).x
    }

    /// X coordinate of the anchor (selection anchor).
    pub fn anchor_x(&self, font: FontId, size: f32, fs: &FontSystem, scale: f32) -> f32 {
        measure(fs, &self.text[..self.anchor], font, size, scale).x
    }

    /// Find the closest byte offset to a given x coordinate (relative to text origin).
    pub fn x_to_cursor(&self, x: f32, font: FontId, size: f32, fs: &FontSystem, scale: f32) -> usize {
        let text = &self.text;
        let mut best_pos = 0;
        let mut best_dist = f32::INFINITY;
        let mut i = 0;
        loop {
            let cx = measure(fs, &text[..i], font, size, scale).x;
            let dist = (cx - x).abs();
            if dist < best_dist {
                best_dist = dist;
                best_pos = i;
            }
            if i >= text.len() { break; }
            i = next_char(text, i);
        }
        best_pos
    }

    /// Adjust `scroll_offset` so the cursor stays within `[0, inner_width]`.
    pub fn ensure_cursor_visible(
        &mut self,
        inner_width: f32,
        font: FontId,
        size: f32,
        fs: &FontSystem,
        scale: f32,
    ) {
        let cx = self.cursor_x(font, size, fs, scale);
        if cx < self.scroll_offset {
            self.scroll_offset = cx;
        } else if cx > self.scroll_offset + inner_width {
            self.scroll_offset = cx - inner_width;
        }
        if self.scroll_offset < 0.0 { self.scroll_offset = 0.0; }
    }

    // ── private ───────────────────────────────────────────────────────────

    /// Delete selection; returns true if anything was deleted.
    fn delete_selection(&mut self) -> bool {
        if !self.has_selection() { return false; }
        let (lo, hi) = self.sel_range();
        self.text.drain(lo..hi);
        self.cursor = lo;
        self.anchor = lo;
        true
    }
}

// ── measurement shim ──────────────────────────────────────────────────────

#[inline]
fn measure(fs: &FontSystem, text: &str, font: FontId, size: f32, scale: f32) -> Vec2 {
    fs.measure_text_scaled(text, font, size, None, scale)
}

// ── UTF-8 helpers ─────────────────────────────────────────────────────────

/// Step one codepoint boundary backward from `from`.
fn prev_char(s: &str, from: usize) -> usize {
    if from == 0 { return 0; }
    let mut i = from - 1;
    while !s.is_char_boundary(i) { i -= 1; }
    i
}

/// Step one codepoint boundary forward from `from`.
fn next_char(s: &str, from: usize) -> usize {
    if from >= s.len() { return s.len(); }
    let mut i = from + 1;
    while i < s.len() && !s.is_char_boundary(i) { i += 1; }
    i
}

/// Jump backward over whitespace then word characters.
fn prev_word(s: &str, from: usize) -> usize {
    let before = &s[..from];
    // Strip trailing whitespace, then strip trailing non-whitespace (the word).
    let trimmed = before.trim_end();
    let word_start = trimmed.trim_end_matches(|c: char| !c.is_whitespace());
    word_start.len()
}

/// Jump forward over word characters then whitespace.
fn next_word(s: &str, from: usize) -> usize {
    let after = &s[from..];
    // Skip non-whitespace (current word tail), then skip whitespace.
    let after_word = after.trim_start_matches(|c: char| !c.is_whitespace());
    let after_ws   = after_word.trim_start();
    from + (after.len() - after_ws.len())
}
