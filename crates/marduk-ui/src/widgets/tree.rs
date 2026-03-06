use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::input::Key;
use marduk_engine::paint::Color;

use crate::constraints::{Constraints, LayoutCtx};
use crate::cursor::CursorIcon;
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::Widget;

/// A node in a [`TreeView`].
#[derive(Clone)]
pub struct TreeNode {
    /// Display label.
    pub label: String,
    /// Optional icon character shown before the label.
    pub icon: Option<char>,
    /// Stable caller-assigned identifier.
    pub data_id: usize,
    /// Child nodes (empty = leaf).
    pub children: Vec<TreeNode>,
    /// Whether this node is currently expanded.
    pub expanded: bool,
}

impl TreeNode {
    pub fn new(label: impl Into<String>, data_id: usize) -> Self {
        Self {
            label: label.into(),
            icon: None,
            data_id,
            children: Vec::new(),
            expanded: false,
        }
    }

    pub fn icon(mut self, c: char) -> Self {
        self.icon = Some(c);
        self
    }

    pub fn expanded(mut self, v: bool) -> Self {
        self.expanded = v;
        self
    }

    pub fn child(mut self, node: TreeNode) -> Self {
        self.children.push(node);
        self
    }
}

struct VisibleRow {
    depth: usize,
    data_id: usize,
    label: String,
    icon: Option<char>,
    has_children: bool,
    expanded: bool,
}

/// A collapsible, selectable tree widget.
///
/// The caller owns the `TreeNode` tree and provides it via [`roots`].
/// Expansion state is stored inside the `TreeNode`s themselves, which means
/// the caller should persist the roots (or handle the `on_toggle` callback
/// to update their own copy) across frame rebuilds.
///
/// # Example
/// ```rust,ignore
/// let mut tree = TreeView::new(font)
///     .roots(vec![
///         TreeNode::new("src", 0).icon('📁').child(
///             TreeNode::new("main.rs", 1).icon('📄')
///         ),
///     ])
///     .on_select(|id| println!("selected {id}"));
/// ```
pub struct TreeView {
    roots: Vec<TreeNode>,
    selected: Option<usize>,
    /// Visible-row index under the cursor, updated by Hover events.
    hover_row: Option<usize>,
    row_height: f32,
    /// Horizontal pixels per depth level.
    indent: f32,
    font: marduk_engine::text::FontId,
    font_size: f32,
    bg: Color,
    hover_bg: Color,
    select_bg: Color,
    text_color: Color,
    on_select: Option<Box<dyn FnMut(usize)>>,
    on_toggle: Option<Box<dyn FnMut(usize, bool)>>,
}

impl TreeView {
    pub fn new(font: marduk_engine::text::FontId) -> Self {
        Self {
            roots: Vec::new(),
            selected: None,
            hover_row: None,
            row_height: 24.0,
            indent: 16.0,
            font,
            font_size: 13.0,
            bg: Color::from_straight(0.0, 0.0, 0.0, 0.0),
            hover_bg: Color::from_straight(0.18, 0.18, 0.22, 1.0),
            select_bg: Color::from_straight(0.22, 0.35, 0.55, 1.0),
            text_color: Color::from_straight(0.85, 0.82, 0.75, 1.0),
            on_select: None,
            on_toggle: None,
        }
    }

    pub fn roots(mut self, roots: Vec<TreeNode>) -> Self {
        self.roots = roots;
        self
    }

    pub fn selected(mut self, id: Option<usize>) -> Self {
        self.selected = id;
        self
    }

    pub fn row_height(mut self, h: f32) -> Self {
        self.row_height = h;
        self
    }

    pub fn indent(mut self, i: f32) -> Self {
        self.indent = i;
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

    pub fn hover_bg(mut self, c: Color) -> Self {
        self.hover_bg = c;
        self
    }

    pub fn select_bg(mut self, c: Color) -> Self {
        self.select_bg = c;
        self
    }

    pub fn text_color(mut self, c: Color) -> Self {
        self.text_color = c;
        self
    }

    pub fn on_select(mut self, f: impl FnMut(usize) + 'static) -> Self {
        self.on_select = Some(Box::new(f));
        self
    }

    pub fn on_toggle(mut self, f: impl FnMut(usize, bool) + 'static) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
    }

    // ── helpers ───────────────────────────────────────────────────────────

    fn visible_rows(&self) -> Vec<VisibleRow> {
        let mut rows = Vec::new();
        for root in &self.roots {
            Self::collect_visible(root, 0, &mut rows);
        }
        rows
    }

    fn collect_visible(node: &TreeNode, depth: usize, rows: &mut Vec<VisibleRow>) {
        rows.push(VisibleRow {
            depth,
            data_id: node.data_id,
            label: node.label.clone(),
            icon: node.icon,
            has_children: !node.children.is_empty(),
            expanded: node.expanded,
        });
        if node.expanded {
            for child in &node.children {
                Self::collect_visible(child, depth + 1, rows);
            }
        }
    }

    fn count_visible(node: &TreeNode) -> usize {
        let mut count = 1;
        if node.expanded {
            for child in &node.children {
                count += Self::count_visible(child);
            }
        }
        count
    }

    fn find_node_mut(roots: &mut Vec<TreeNode>, data_id: usize) -> Option<&mut TreeNode> {
        for root in roots.iter_mut() {
            if let Some(n) = Self::find_in_node_mut(root, data_id) {
                return Some(n);
            }
        }
        None
    }

    fn find_in_node_mut(node: &mut TreeNode, data_id: usize) -> Option<&mut TreeNode> {
        if node.data_id == data_id {
            return Some(node);
        }
        for child in node.children.iter_mut() {
            if let Some(n) = Self::find_in_node_mut(child, data_id) {
                return Some(n);
            }
        }
        None
    }

    fn find_selected_row(rows: &[VisibleRow], selected: Option<usize>) -> Option<usize> {
        let sel = selected?;
        rows.iter().position(|r| r.data_id == sel)
    }
}

impl Widget for TreeView {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        let count: usize = self.roots.iter().map(|r| Self::count_visible(r)).sum();
        let w = if constraints.max.x.is_finite() { constraints.max.x } else { 200.0 };
        let natural_h = count as f32 * self.row_height;
        let h = if constraints.max.y.is_finite() {
            natural_h.min(constraints.max.y).max(0.0)
        } else {
            natural_h
        };
        Vec2::new(w, h)
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let rows = self.visible_rows();

        for (i, row) in rows.iter().enumerate() {
            let y = rect.origin.y + i as f32 * self.row_height;
            if y + self.row_height < rect.origin.y || y > rect.origin.y + rect.size.y {
                continue; // outside visible region (parent may not clip)
            }
            let row_rect = Rect::new(rect.origin.x, y, rect.size.x, self.row_height);

            // Background
            let bg = if self.selected == Some(row.data_id) {
                self.select_bg
            } else if self.hover_row == Some(i) {
                self.hover_bg
            } else {
                self.bg
            };
            if bg.a > 0.001 {
                painter.fill_rect(row_rect, bg);
            }

            // Set cursor when hovered
            if self.hover_row == Some(i) {
                painter.set_cursor(CursorIcon::Pointer);
            }

            let x_base = rect.origin.x + row.depth as f32 * self.indent;
            let chevron_x = x_base;
            let label_x = x_base + self.indent;
            let text_y = y + (self.row_height - self.font_size) * 0.5;

            // Chevron
            if row.has_children {
                let chevron = if row.expanded { "▼" } else { "▶" };
                painter.text(
                    chevron,
                    self.font,
                    10.0,
                    Color::from_straight(0.6, 0.6, 0.65, 1.0),
                    Vec2::new(chevron_x + 3.0, y + (self.row_height - 10.0) * 0.5),
                    Some(self.indent - 3.0),
                );
            }

            // Icon + label
            let mut cursor_x = label_x;
            if let Some(icon) = row.icon {
                painter.text(
                    icon.to_string(),
                    self.font,
                    self.font_size,
                    self.text_color,
                    Vec2::new(cursor_x, text_y),
                    Some(self.font_size + 4.0),
                );
                cursor_x += self.font_size + 6.0;
            }
            let max_w = (rect.origin.x + rect.size.x - cursor_x).max(0.0);
            painter.text(
                row.label.as_str(),
                self.font,
                self.font_size,
                self.text_color,
                Vec2::new(cursor_x, text_y),
                Some(max_w),
            );
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _ctx: &LayoutCtx<'_>) -> EventResult {
        match event {
            UiEvent::Hover { pos } => {
                if rect.contains(*pos) {
                    let rel_y = pos.y - rect.origin.y;
                    let row_i = (rel_y / self.row_height) as usize;
                    let rows = self.visible_rows();
                    self.hover_row = if row_i < rows.len() { Some(row_i) } else { None };
                } else {
                    self.hover_row = None;
                }
                EventResult::Ignored // Hover never consumes
            }

            UiEvent::Click { pos } => {
                if !rect.contains(*pos) {
                    return EventResult::Ignored;
                }
                let rel_y = pos.y - rect.origin.y;
                let row_i = (rel_y / self.row_height) as usize;
                let rows = self.visible_rows();
                let Some(row) = rows.get(row_i) else {
                    return EventResult::Ignored;
                };

                let rel_x = pos.x - rect.origin.x;
                let chevron_start = row.depth as f32 * self.indent;
                let chevron_end = chevron_start + self.indent;

                if row.has_children && rel_x >= chevron_start && rel_x < chevron_end {
                    // Toggle expansion
                    let data_id = row.data_id;
                    let new_expanded = !row.expanded;
                    if let Some(node) = Self::find_node_mut(&mut self.roots, data_id) {
                        node.expanded = new_expanded;
                    }
                    if let Some(f) = &mut self.on_toggle {
                        f(data_id, new_expanded);
                    }
                } else {
                    // Select
                    let data_id = row.data_id;
                    self.selected = Some(data_id);
                    if let Some(f) = &mut self.on_select {
                        f(data_id);
                    }
                }
                EventResult::Consumed
            }

            UiEvent::KeyPress { key, .. } => {
                let rows = self.visible_rows();
                let sel_row = Self::find_selected_row(&rows, self.selected);

                match key {
                    Key::ArrowDown => {
                        let next = sel_row.map(|i| (i + 1).min(rows.len().saturating_sub(1)))
                            .unwrap_or(0);
                        if let Some(r) = rows.get(next) {
                            self.selected = Some(r.data_id);
                            if let Some(f) = &mut self.on_select { f(r.data_id); }
                        }
                        EventResult::Consumed
                    }
                    Key::ArrowUp => {
                        let prev = sel_row.map(|i| i.saturating_sub(1)).unwrap_or(0);
                        if let Some(r) = rows.get(prev) {
                            self.selected = Some(r.data_id);
                            if let Some(f) = &mut self.on_select { f(r.data_id); }
                        }
                        EventResult::Consumed
                    }
                    Key::ArrowRight => {
                        if let Some(row_i) = sel_row {
                            if let Some(row) = rows.get(row_i) {
                                if row.has_children && !row.expanded {
                                    let data_id = row.data_id;
                                    if let Some(node) = Self::find_node_mut(&mut self.roots, data_id) {
                                        node.expanded = true;
                                    }
                                    if let Some(f) = &mut self.on_toggle { f(data_id, true); }
                                }
                            }
                        }
                        EventResult::Consumed
                    }
                    Key::ArrowLeft => {
                        if let Some(row_i) = sel_row {
                            if let Some(row) = rows.get(row_i) {
                                if row.has_children && row.expanded {
                                    let data_id = row.data_id;
                                    if let Some(node) = Self::find_node_mut(&mut self.roots, data_id) {
                                        node.expanded = false;
                                    }
                                    if let Some(f) = &mut self.on_toggle { f(data_id, false); }
                                }
                            }
                        }
                        EventResult::Consumed
                    }
                    _ => EventResult::Ignored,
                }
            }

            _ => EventResult::Ignored,
        }
    }
}
