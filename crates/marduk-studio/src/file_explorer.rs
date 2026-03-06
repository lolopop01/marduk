//! Production file explorer — Adwaita dark (GNOME Files inspired).
//!
//! Layout (Stack filling the viewport):
//!
//!  ┌──────────────────────────────────────────────────────┐
//!  │  ‹ Home  ›  Documents  ›  Project            [48px] │  ← Headerbar / pathbar
//!  ├────────────────┬─────────────────────────────────────┤
//!  │ Sidebar (tree) │ Name                    Size  [36px]│
//!  │  ▶ Documents   │ » src/                              │
//!  │  ▶ Downloads   │ » target/                           │
//!  │  ▶ Projects    │   Cargo.lock            42 KB       │
//!  │                │   Cargo.toml              512 B     │
//!  ├────────────────┴─────────────────────────────────────┤
//!  │  /home/user/Projects  ·  4 items             [22px] │  ← Status bar
//!  └──────────────────────────────────────────────────────┘
//!
//! Right-click on any row opens a floating context menu (overlay).
//! Double-click navigates into directories or opens files with xdg-open.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use marduk_ui::prelude::*;

// ── Layout / sizing ────────────────────────────────────────────────────────

const HEADER_H: f32 = 48.0;   // pathbar
const STATUS_H: f32 = 24.0;   // status bar
const ROW_H:    f32 = 36.0;   // file/tree row
const FONT_SZ:  f32 = 14.0;   // primary text
const SMALL_SZ: f32 = 12.0;   // secondary / size column
const LPAD:     f32 = 12.0;   // left padding in rows
const SIZE_COL: f32 = 72.0;   // width reserved for size text (right side)

const MENU_W:      f32 = 220.0;
const MENU_ITEM_H: f32 = 36.0;
const MENU_SEP_H:  f32 = 9.0;
const MENU_R:      f32 = 8.0;  // context menu corner radius

// ── Adwaita dark palette ───────────────────────────────────────────────────

fn c_window()   -> Color { Color::from_straight(0.110, 0.110, 0.110, 1.0) } // #1c1c1c
fn c_content()  -> Color { Color::from_straight(0.141, 0.141, 0.141, 1.0) } // #242424
fn c_sidebar()  -> Color { Color::from_straight(0.110, 0.110, 0.110, 1.0) } // #1c1c1c
fn c_header()   -> Color { Color::from_straight(0.188, 0.188, 0.188, 1.0) } // #303030
fn c_sel()      -> Color { Color::from_straight(0.110, 0.443, 0.847, 1.0) } // #1c71d8
fn c_hover()    -> Color { Color::from_straight(1.0, 1.0, 1.0, 0.07)      } // subtle white tint
fn c_text()     -> Color { Color::from_straight(1.0, 1.0, 1.0, 1.0)       } // #ffffff
fn c_dim()      -> Color { Color::from_straight(1.0, 1.0, 1.0, 0.55)      } // dim white
fn c_folder()   -> Color { Color::from_straight(1.0, 0.839, 0.200, 1.0)   } // #ffd633 Adwaita folder
fn c_divider()  -> Color { Color::from_straight(1.0, 1.0, 1.0, 0.08)      }
fn c_menu_bg()  -> Color { Color::from_straight(0.216, 0.216, 0.216, 1.0) } // #373737
fn c_menu_bdr() -> Color { Color::from_straight(1.0, 1.0, 1.0, 0.15)      }
fn c_menu_sep() -> Color { Color::from_straight(1.0, 1.0, 1.0, 0.10)      }
fn c_destructive() -> Color { Color::from_straight(0.937, 0.329, 0.298, 1.0) } // #ef544c

// ── MenuItem (context menu) ────────────────────────────────────────────────

enum MenuItem {
    Action { label: &'static str, destructive: bool, action: Box<dyn FnMut()> },
    Separator,
}

// ── ContextMenu widget ─────────────────────────────────────────────────────
//
// Floating popup rendered in overlay space.  The widget itself is given
// the full viewport rect via the Stack; it computes the actual menu rect
// from `anchor_pos`, clamped to fit the screen.

struct ContextMenu {
    anchor:     Vec2,
    items:      Vec<MenuItem>,
    hover_item: Option<usize>,  // index into items (separators don't hover)
    on_close:   Box<dyn FnMut()>,
    font:       FontId,
}

impl ContextMenu {
    fn menu_height(items: &[MenuItem]) -> f32 {
        items.iter().map(|it| match it {
            MenuItem::Action { .. } => MENU_ITEM_H,
            MenuItem::Separator    => MENU_SEP_H,
        }).sum::<f32>() + 8.0 // top+bottom padding
    }

    fn menu_rect(&self, viewport: Rect) -> Rect {
        let h = Self::menu_height(&self.items);
        let x = (self.anchor.x).min(viewport.size.x - MENU_W - 4.0).max(4.0);
        let y = if self.anchor.y + h + 4.0 > viewport.size.y {
            (self.anchor.y - h).max(4.0)
        } else {
            self.anchor.y
        };
        Rect::new(x, y, MENU_W, h)
    }
}

impl Widget for ContextMenu {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        // Fill the whole viewport so we can draw anywhere inside overlay_scope.
        Vec2::new(
            if constraints.max.x.is_finite() { constraints.max.x } else { 800.0 },
            if constraints.max.y.is_finite() { constraints.max.y } else { 600.0 },
        )
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let mr = self.menu_rect(rect);
        painter.register_overlay(mr);

        painter.overlay_scope(|p| {
            // Drop shadow (translucent black shifted down-right by 4px).
            let shadow = Rect::new(mr.origin.x + 4.0, mr.origin.y + 4.0, mr.size.x, mr.size.y);
            p.fill_rounded_rect(shadow, MENU_R, Paint::Solid(Color::from_straight(0.0,0.0,0.0,0.35)), None);

            // Menu background.
            p.fill_rounded_rect(mr, MENU_R, Paint::Solid(c_menu_bg()),
                Some(Border::new(1.0, c_menu_bdr())));

            // Items.
            let mut y = mr.origin.y + 4.0;
            for (i, item) in self.items.iter().enumerate() {
                match item {
                    MenuItem::Separator => {
                        let sy = y + MENU_SEP_H * 0.5;
                        p.fill_rect(Rect::new(mr.origin.x + 8.0, sy, mr.size.x - 16.0, 1.0), c_menu_sep());
                        y += MENU_SEP_H;
                    }
                    MenuItem::Action { label, destructive, .. } => {
                        let item_rect = Rect::new(mr.origin.x + 4.0, y, mr.size.x - 8.0, MENU_ITEM_H);
                        if self.hover_item == Some(i) {
                            p.fill_rounded_rect(item_rect, 4.0,
                                Paint::Solid(c_hover()), None);
                        }
                        let tc = if *destructive { c_destructive() } else { c_text() };
                        let ty = y + (MENU_ITEM_H - FONT_SZ) * 0.5;
                        p.text(*label, self.font, FONT_SZ, tc,
                            Vec2::new(mr.origin.x + 14.0, ty), Some(mr.size.x - 20.0));
                        y += MENU_ITEM_H;
                    }
                }
            }
        });
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _ctx: &LayoutCtx) -> EventResult {
        let mr = self.menu_rect(rect);
        match event {
            UiEvent::Hover { pos } => {
                if mr.contains(*pos) {
                    let mut y = mr.origin.y + 4.0;
                    self.hover_item = None;
                    for (i, item) in self.items.iter().enumerate() {
                        match item {
                            MenuItem::Separator  => { y += MENU_SEP_H; }
                            MenuItem::Action { .. } => {
                                if pos.y >= y && pos.y < y + MENU_ITEM_H {
                                    self.hover_item = Some(i);
                                }
                                y += MENU_ITEM_H;
                            }
                        }
                    }
                    // Cursor set in paint; on_event has no painter.
                } else {
                    self.hover_item = None;
                }
                EventResult::Ignored
            }
            UiEvent::Click { pos } => {
                if !mr.contains(*pos) { return EventResult::Ignored; }
                if let Some(hi) = self.hover_item {
                    if let Some(MenuItem::Action { action, .. }) = self.items.get_mut(hi) {
                        action();
                    }
                }
                (self.on_close)();
                EventResult::Consumed
            }
            UiEvent::RightClick { .. } | UiEvent::OverlayDismiss => {
                (self.on_close)();
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }
}

// ── FileRow widget ─────────────────────────────────────────────────────────

struct FileRow {
    name:        String,
    is_dir:      bool,
    size_str:    String,
    is_selected: bool,
    hovered:     bool,
    font:        FontId,
    on_click:        Option<Box<dyn FnMut()>>,
    on_right_click:  Option<Box<dyn FnMut(Vec2)>>,
    // Double-click detection lives in FilePane state; single-click closure does the check.
}

impl FileRow {
    fn new(name: String, is_dir: bool, size_str: String, is_selected: bool, font: FontId) -> Self {
        Self {
            name, is_dir, size_str, is_selected, hovered: false, font,
            on_click: None, on_right_click: None,
        }
    }
    fn on_click(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_click = Some(Box::new(f)); self
    }
    fn on_right_click(mut self, f: impl FnMut(Vec2) + 'static) -> Self {
        self.on_right_click = Some(Box::new(f)); self
    }
}

impl Widget for FileRow {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        let w = if constraints.max.x.is_finite() { constraints.max.x } else { 300.0 };
        Vec2::new(w, ROW_H)
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        let bg = if self.is_selected { c_sel() }
                 else if self.hovered { c_hover() }
                 else { Color::from_straight(0.0, 0.0, 0.0, 0.0) };

        if self.is_selected || self.hovered {
            // Adwaita rows use slightly inset rounded highlights.
            let hr = Rect::new(
                rect.origin.x + 2.0, rect.origin.y + 1.0,
                rect.size.x - 4.0, rect.size.y - 2.0,
            );
            painter.fill_rounded_rect(hr, 5.0, Paint::Solid(bg), None);
        }

        let text_y  = rect.origin.y + (ROW_H - FONT_SZ).max(0.0) * 0.5;
        let small_y = rect.origin.y + (ROW_H - SMALL_SZ).max(0.0) * 0.5 + 0.5;

        // Type badge: small colored square (Adwaita-style icon placeholder).
        let badge_x = rect.origin.x + LPAD;
        let badge_y = rect.origin.y + (ROW_H - 16.0) * 0.5;
        let badge_color = if self.is_dir { c_folder() }
                          else { Color::from_straight(0.55, 0.75, 0.95, 1.0) }; // light blue for files
        painter.fill_rounded_rect(
            Rect::new(badge_x, badge_y, 16.0, 16.0),
            3.0,
            Paint::Solid(badge_color),
            None,
        );

        // Name.
        let name_x   = badge_x + 16.0 + 10.0;
        let max_name = (rect.origin.x + rect.size.x - name_x - SIZE_COL - LPAD).max(0.0);
        let name_color = if self.is_selected { c_text() } else { c_text() };
        painter.text(
            &self.name, self.font, FONT_SZ, name_color,
            Vec2::new(name_x, text_y), Some(max_name),
        );

        // Size (right-aligned).
        if !self.size_str.is_empty() {
            let sw = painter.measure_text(&self.size_str, self.font, SMALL_SZ, None).x;
            let dim = if self.is_selected {
                Color::from_straight(1.0, 1.0, 1.0, 0.75)
            } else { c_dim() };
            painter.text(
                &self.size_str, self.font, SMALL_SZ, dim,
                Vec2::new(rect.origin.x + rect.size.x - sw - LPAD, small_y),
                None,
            );
        }

        if self.hovered || self.is_selected {
            painter.set_cursor(CursorIcon::Pointer);
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _ctx: &LayoutCtx) -> EventResult {
        match event {
            UiEvent::Hover { pos } => {
                self.hovered = rect.contains(*pos);
                EventResult::Ignored
            }
            UiEvent::Click { pos } => {
                if !rect.contains(*pos) { return EventResult::Ignored; }
                if let Some(f) = &mut self.on_click { f(); }
                EventResult::Consumed
            }
            UiEvent::RightClick { pos } => {
                if !rect.contains(*pos) { return EventResult::Ignored; }
                if let Some(f) = &mut self.on_right_click { f(*pos); }
                EventResult::Consumed
            }
            _ => EventResult::Ignored,
        }
    }
}

// ── StatusBar widget ───────────────────────────────────────────────────────

struct StatusBar { text: String, font: FontId }

impl Widget for StatusBar {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        Vec2::new(if constraints.max.x.is_finite() { constraints.max.x } else { 200.0 }, STATUS_H)
    }
    fn paint(&self, painter: &mut Painter, rect: Rect) {
        painter.fill_rect(rect, c_header());
        painter.fill_rect(Rect::new(rect.origin.x, rect.origin.y, rect.size.x, 1.0), c_divider());
        let ty = rect.origin.y + (STATUS_H - SMALL_SZ) * 0.5;
        painter.text(&self.text, self.font, SMALL_SZ, c_dim(),
            Vec2::new(rect.origin.x + LPAD, ty), Some(rect.size.x - LPAD * 2.0));
    }
    fn on_event(&mut self, _e: &UiEvent, _r: Rect, _c: &LayoutCtx) -> EventResult { EventResult::Ignored }
}

// ── ColumnHeader widget ────────────────────────────────────────────────────

struct ColumnHeader { font: FontId }

impl Widget for ColumnHeader {
    fn measure(&self, constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        Vec2::new(if constraints.max.x.is_finite() { constraints.max.x } else { 200.0 }, ROW_H)
    }
    fn paint(&self, painter: &mut Painter, rect: Rect) {
        painter.fill_rect(rect, c_header());
        painter.fill_rect(Rect::new(rect.origin.x, rect.origin.y + rect.size.y - 1.0, rect.size.x, 1.0), c_divider());
        let ty = rect.origin.y + (ROW_H - SMALL_SZ) * 0.5;
        let lx = rect.origin.x + LPAD + 16.0 + 10.0; // align with name text
        painter.text("Name", self.font, SMALL_SZ, c_dim(), Vec2::new(lx, ty), None);
        let sw = painter.measure_text("Size", self.font, SMALL_SZ, None).x;
        painter.text("Size", self.font, SMALL_SZ, c_dim(),
            Vec2::new(rect.origin.x + rect.size.x - sw - LPAD, ty), None);
    }
    fn on_event(&mut self, _e: &UiEvent, _r: Rect, _c: &LayoutCtx) -> EventResult { EventResult::Ignored }
}

// ── FileEntry ──────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct FileEntry {
    pub name:   String,
    pub path:   PathBuf,
    pub is_dir: bool,
    pub size:   Option<u64>,
}

impl FileEntry {
    fn size_str(&self) -> String {
        if self.is_dir { return String::new(); }
        match self.size {
            Some(b) if b < 1_024             => format!("{b} B"),
            Some(b) if b < 1_024 * 1_024     => format!("{:.1} KB", b as f64 / 1_024.0),
            Some(b) if b < 1_024u64.pow(3)   => format!("{:.1} MB", b as f64 / (1_024.0 * 1_024.0)),
            Some(b)                           => format!("{:.1} GB", b as f64 / (1_024.0f64.powi(3))),
            None                             => String::new(),
        }
    }
}

// ── ContextMenuState ───────────────────────────────────────────────────────

struct ContextMenuState {
    anchor: Vec2,
    path:   Option<PathBuf>,
    is_dir: bool,
}

// ── FilePane ──────────────────────────────────────────────────────────────

pub struct FilePane {
    current_dir:      PathBuf,
    entries:          Vec<FileEntry>,
    tree_roots:       Vec<TreeNode>,
    tree_paths:       Vec<PathBuf>,
    tree_selected:    Option<usize>,
    selected_entry:   Option<usize>,
    breadcrumb_parts: Vec<(String, PathBuf)>,
    split_ratio:      f32,
    split_dragging:   bool,
    context_menu:     Option<ContextMenuState>,
    // Double-click detection.
    last_click:       Option<(Instant, usize)>,
    pub font:         FontId,
    event_queue:      Rc<RefCell<Vec<String>>>,
}

impl FilePane {
    pub fn new(
        dir: PathBuf,
        font: FontId,
        event_queue: Rc<RefCell<Vec<String>>>,
    ) -> Self {
        let mut pane = Self {
            current_dir:      dir.clone(),
            entries:          Vec::new(),
            tree_roots:       Vec::new(),
            tree_paths:       Vec::new(),
            tree_selected:    None,
            selected_entry:   None,
            breadcrumb_parts: Vec::new(),
            split_ratio:      0.26,
            split_dragging:   false,
            context_menu:     None,
            last_click:       None,
            font,
            event_queue,
        };
        pane.scan(&dir.clone());
        pane.build_tree(&dir);
        pane.build_breadcrumb();
        pane
    }

    // ── fs helpers ─────────────────────────────────────────────────────────

    pub fn scan(&mut self, path: &Path) {
        self.current_dir = path.to_path_buf();
        self.entries.clear();
        self.selected_entry = None;
        self.context_menu   = None;

        let Ok(rd) = std::fs::read_dir(path) else { return };
        let (mut dirs, mut files) = (Vec::new(), Vec::new());
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') { continue; }
            let meta  = entry.metadata().ok();
            let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
            let size   = if is_dir { None } else { meta.as_ref().map(|m| m.len()) };
            let fe = FileEntry { name, path: entry.path(), is_dir, size };
            if is_dir { dirs.push(fe) } else { files.push(fe) }
        }
        dirs.sort_by(|a,b|  a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        files.sort_by(|a,b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        self.entries.extend(dirs);
        self.entries.extend(files);
    }

    pub fn build_tree(&mut self, root: &Path) {
        self.tree_roots.clear();
        self.tree_paths.clear();
        let label = root.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string_lossy().into_owned());
        let root_id = self.alloc_id(root.to_path_buf());
        let mut rn  = TreeNode::new(label, root_id).expanded(true);
        for sub in Self::list_dirs(root) {
            let lbl    = sub.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let sub_id = self.alloc_id(sub.clone());
            let mut dn  = TreeNode::new(lbl, sub_id);
            // One deep level so chevron appears if folder has children.
            for s2 in Self::list_dirs(&sub) {
                let l2  = s2.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let s2i = self.alloc_id(s2.clone());
                let mut s2n = TreeNode::new(l2, s2i);
                if Self::list_dirs(&s2).next().is_some() {
                    // Placeholder so chevron shows for the grandchild level.
                    s2n = s2n.child(TreeNode::new(String::new(), self.alloc_id(s2.join(".ph"))));
                }
                dn = dn.child(s2n);
            }
            rn = rn.child(dn);
        }
        self.tree_roots = vec![rn];
    }

    fn list_dirs(path: &Path) -> impl Iterator<Item = PathBuf> {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(path)
            .into_iter().flatten().flatten()
            .filter_map(|e| {
                let p = e.path();
                let n = e.file_name();
                if p.is_dir() && !n.to_string_lossy().starts_with('.') { Some(p) } else { None }
            })
            .collect();
        dirs.sort_by(|a,b| a.file_name().unwrap_or_default().cmp(b.file_name().unwrap_or_default()));
        dirs.into_iter()
    }

    fn alloc_id(&mut self, path: PathBuf) -> usize {
        let id = self.tree_paths.len();
        self.tree_paths.push(path);
        id
    }

    fn build_breadcrumb(&mut self) {
        self.breadcrumb_parts.clear();
        let mut parts: VecDeque<(String, PathBuf)> = VecDeque::new();
        let mut p = self.current_dir.clone();
        loop {
            let label = p.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.to_string_lossy().into_owned());
            parts.push_front((label, p.clone()));
            match p.parent() {
                Some(par) if par != p => p = par.to_path_buf(),
                _ => break,
            }
        }
        self.breadcrumb_parts = parts.into();
        if self.breadcrumb_parts.len() > 5 {
            let skip = self.breadcrumb_parts.len() - 5;
            self.breadcrumb_parts = self.breadcrumb_parts[skip..].to_vec();
        }
    }

    // ── open helpers ───────────────────────────────────────────────────────

    fn open_path(path: &Path) {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }

    fn navigate_to(this: &Rc<RefCell<Self>>, path: PathBuf) {
        let mut p = this.borrow_mut();
        p.scan(&path.clone());
        p.build_tree(&path);
        p.build_breadcrumb();
    }

    // ── element builders ───────────────────────────────────────────────────

    pub fn as_element(this: Rc<RefCell<Self>>) -> Element {
        let font         = this.borrow().font;
        let split_ratio  = this.borrow().split_ratio;
        let split_drag   = this.borrow().split_dragging;
        let has_menu     = this.borrow().context_menu.is_some();

        let header  = Self::build_header(Rc::clone(&this), font);
        let sidebar = Self::build_sidebar(Rc::clone(&this), font);
        let content = Self::build_content(Rc::clone(&this), font);
        let status  = Self::build_status(Rc::clone(&this), font);

        let this2 = Rc::clone(&this);
        let this3 = Rc::clone(&this);
        let splitter = Splitter::horizontal(
            ScrollView::new(sidebar).line_height(ROW_H),
            Column::new()
                .child(ColumnHeader { font })
                .child(ScrollView::new(content).line_height(ROW_H)),
        )
        .initial_ratio(split_ratio)
        .initial_dragging(split_drag)
        .on_change(move |r| { this2.borrow_mut().split_ratio = r; })
        .on_drag_change(move |d| { this3.borrow_mut().split_dragging = d; });

        let mut stack = Stack::new()
            .bg(c_window())
            .item(StackItem::new(header)
                .left(AnchorVal::Px(0.0)).top(AnchorVal::Px(0.0))
                .right(AnchorVal::Px(0.0)).height(SizeHint::Px(HEADER_H)))
            .item(StackItem::new(splitter)
                .left(AnchorVal::Px(0.0)).top(AnchorVal::Px(HEADER_H))
                .right(AnchorVal::Px(0.0)).bottom(AnchorVal::Px(STATUS_H)))
            .item(StackItem::new(status)
                .left(AnchorVal::Px(0.0)).right(AnchorVal::Px(0.0))
                .bottom(AnchorVal::Px(0.0)).height(SizeHint::Px(STATUS_H)));

        // Overlay: context menu (drawn above everything else).
        if has_menu {
            let menu = Self::build_context_menu(Rc::clone(&this), font);
            stack = stack.item(StackItem::new(menu)
                .left(AnchorVal::Px(0.0)).top(AnchorVal::Px(0.0))
                .right(AnchorVal::Px(0.0)).bottom(AnchorVal::Px(0.0)));
        }

        stack.into()
    }

    fn build_header(this: Rc<RefCell<Self>>, font: FontId) -> Element {
        let parts = this.borrow().breadcrumb_parts.clone();
        let mut row = Row::new().spacing(0.0).padding_all(8.0).cross_align(Align::Center);

        for (i, (label, path)) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            let color   = if is_last { c_text() } else { c_dim() };
            let path2   = path.clone();
            let this2   = Rc::clone(&this);

            let btn = Button::new(Text::new(label.clone(), font, FONT_SZ, color))
                .background(Color::from_straight(0.0, 0.0, 0.0, 0.0))
                .hover_background(Color::from_straight(1.0, 1.0, 1.0, 0.10))
                .padding_all(6.0)
                .corner_radius(5.0)
                .on_click(move || {
                    let mut p = this2.borrow_mut();
                    p.scan(&path2);
                    p.build_breadcrumb();
                });
            row = row.child(btn);

            if !is_last {
                row = row.child(
                    Container::new()
                        .child(Text::new("/", font, FONT_SZ, c_dim()))
                        .padding_all(2.0)
                );
            }
        }

        Container::new()
            .child(row)
            .background(Paint::Solid(c_header()))
            .into()
    }

    fn build_sidebar(this: Rc<RefCell<Self>>, font: FontId) -> Element {
        let pane     = this.borrow();
        let roots    = pane.tree_roots.clone();
        let selected = pane.tree_selected;
        drop(pane);

        let this2 = Rc::clone(&this);
        let this3 = Rc::clone(&this);

        Container::new()
            .background(Paint::Solid(c_sidebar()))
            .child(
                TreeView::new(font)
                    .roots(roots)
                    .selected(selected)
                    .row_height(ROW_H)
                    .indent(16.0)
                    .font_size(FONT_SZ)
                    .bg(Color::from_straight(0.0,0.0,0.0,0.0))
                    .hover_bg(c_hover())
                    .select_bg(c_sel())
                    .text_color(c_text())
                    .on_select(move |id| {
                        let path = this2.borrow().tree_paths.get(id).cloned();
                        if let Some(dir) = path {
                            if dir.is_dir() {
                                let mut p = this2.borrow_mut();
                                p.tree_selected = Some(id);
                                p.scan(&dir.clone());
                                p.build_breadcrumb();
                                set_expanded_in(&mut p.tree_roots, id, true);
                            }
                        }
                    })
                    .on_toggle(move |id, exp| {
                        set_expanded_in(&mut this3.borrow_mut().tree_roots, id, exp);
                    })
            )
            .into()
    }

    fn build_content(this: Rc<RefCell<Self>>, font: FontId) -> Element {
        let entries  = this.borrow().entries.clone();
        let selected = this.borrow().selected_entry;
        let eq       = Rc::clone(&this.borrow().event_queue);

        let mut col = Column::new();

        if entries.is_empty() {
            col = col.child(Container::new()
                .child(Text::new("(empty directory)", font, SMALL_SZ, c_dim()))
                .padding_all(LPAD));
        }

        for (i, entry) in entries.iter().enumerate() {
            let is_sel   = selected == Some(i);
            let size_str = entry.size_str();
            let path     = entry.path.clone();
            let path3    = entry.path.clone();
            let is_dir   = entry.is_dir;
            let this_c   = Rc::clone(&this);
            let this_r   = Rc::clone(&this);
            let eq_c     = Rc::clone(&eq);

            let row = FileRow::new(entry.name.clone(), is_dir, size_str, is_sel, font)
                .on_click(move || {
                    let now = Instant::now();
                    let mut p = this_c.borrow_mut();
                    // Double-click detection in single-click handler.
                    let is_double = p.last_click.as_ref()
                        .map(|(t, idx)| *idx == i && t.elapsed().as_millis() < 400)
                        .unwrap_or(false);
                    if is_double {
                        p.last_click = None;
                        drop(p);
                        if is_dir {
                            Self::navigate_to(&this_c, path.clone());
                        } else {
                            Self::open_path(&path);
                        }
                    } else {
                        p.last_click      = Some((now, i));
                        p.selected_entry  = Some(i);
                        if !is_dir {
                            eq_c.borrow_mut().push(
                                format!("file_select:{}", path.to_string_lossy())
                            );
                        }
                    }
                })
                .on_right_click(move |pos| {
                    let mut p = this_r.borrow_mut();
                    p.selected_entry = Some(i);
                    p.context_menu = Some(ContextMenuState {
                        anchor: pos,
                        path: Some(path3.clone()),
                        is_dir,
                    });
                });
            col = col.child(row);
        }

        Container::new()
            .child(col)
            .background(Paint::Solid(c_content()))
            .into()
    }

    fn build_context_menu(this: Rc<RefCell<Self>>, font: FontId) -> Element {
        let state = this.borrow().context_menu.as_ref().and_then(|m| {
            m.path.clone().map(|p| (m.anchor, p, m.is_dir))
        });
        let Some((anchor, path, is_dir)) = state else {
            return Container::new().into();
        };

        let this_open  = Rc::clone(&this);
        let this_trash = Rc::clone(&this);
        let this_close = Rc::clone(&this);

        let path_open  = path.clone();
        let path_copy  = path.clone();
        let path_trash = path.clone();

        let mut items: Vec<MenuItem> = Vec::new();

        // "Open" — navigate into dir or xdg-open file.
        items.push(MenuItem::Action {
            label: if is_dir { "Open" } else { "Open" },
            destructive: false,
            action: Box::new(move || {
                if is_dir {
                    Self::navigate_to(&this_open, path_open.clone());
                } else {
                    Self::open_path(&path_open);
                }
            }),
        });

        // "Open in Terminal" for directories.
        if is_dir {
            let path_term = path.clone();
            items.push(MenuItem::Action {
                label: "Open in Terminal",
                destructive: false,
                action: Box::new(move || {
                    let _ = std::process::Command::new("xdg-open")
                        .arg(&path_term)
                        .spawn();
                }),
            });
        }

        items.push(MenuItem::Separator);

        items.push(MenuItem::Action {
            label: "Copy Path",
            destructive: false,
            action: Box::new(move || {
                let s = path_copy.to_string_lossy().to_string();
                // Try wl-copy (Wayland), then xclip (X11).
                let _ = std::process::Command::new("wl-copy").arg(&s).spawn()
                    .or_else(|_| std::process::Command::new("xclip")
                        .args(["-selection", "clipboard"])
                        .stdin(std::process::Stdio::piped())
                        .spawn()
                        .map(|mut ch| {
                            use std::io::Write;
                            if let Some(stdin) = ch.stdin.as_mut() {
                                let _ = stdin.write_all(s.as_bytes());
                            }
                            ch
                        }));
            }),
        });

        items.push(MenuItem::Separator);

        items.push(MenuItem::Action {
            label: "Move to Trash",
            destructive: true,
            action: Box::new(move || {
                // Use gio trash on GNOME systems; fall back to rm.
                let status = std::process::Command::new("gio")
                    .args(["trash", &path_trash.to_string_lossy()])
                    .status();
                if status.map(|s| !s.success()).unwrap_or(true) {
                    let _ = std::process::Command::new("rm")
                        .arg("-rf").arg(&path_trash).status();
                }
                // Refresh the current directory.
                {
                    let mut p = this_trash.borrow_mut();
                    let dir = p.current_dir.clone();
                    p.scan(&dir);
                }
            }),
        });

        ContextMenu {
            anchor,
            items,
            hover_item: None,
            font,
            on_close: Box::new(move || {
                this_close.borrow_mut().context_menu = None;
            }),
        }.into()
    }

    fn build_status(this: Rc<RefCell<Self>>, font: FontId) -> Element {
        let pane  = this.borrow();
        let count = pane.entries.len();
        let sel   = pane.selected_entry.and_then(|i| pane.entries.get(i));
        let sel_s = sel.map(|e| {
            if e.is_dir { format!("  —  {}", e.name) }
            else        { format!("  —  {}  ({})", e.name, e.size_str()) }
        }).unwrap_or_default();
        let text = format!("{path}  ·  {count} items{sel_s}",
            path = pane.current_dir.to_string_lossy());
        StatusBar { text, font }.into()
    }
}

// ── helpers ───────────────────────────────────────────────────────────────

fn set_expanded_in(roots: &mut Vec<TreeNode>, id: usize, exp: bool) {
    for r in roots.iter_mut() {
        if set_expanded_node(r, id, exp) { return; }
    }
}
fn set_expanded_node(node: &mut TreeNode, id: usize, exp: bool) -> bool {
    if node.data_id == id { node.expanded = exp; return true; }
    for c in node.children.iter_mut() {
        if set_expanded_node(c, id, exp) { return true; }
    }
    false
}
