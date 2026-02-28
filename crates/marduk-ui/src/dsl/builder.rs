use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;
use marduk_engine::text::FontId;

use crate::constraints::Edges;
use crate::dsl::ast::{DslDocument, Node, Value};
use crate::dsl::error::ParseError;
use crate::dsl::parser::parse_str;
use crate::widget::Element;
use crate::widgets::{
    button::Button,
    checkbox::Checkbox,
    container::Container,
    flex::{Align, Column, Row},
    progress::ProgressBar,
    radio::RadioGroup,
    scroll::ScrollView,
    slider::Slider,
    stack::{AnchorVal, SizeHint, Stack, StackItem},
    text::Text,
    textbox::TextBox,
    toggle::Toggle,
};

// ── WidgetStateValue ──────────────────────────────────────────────────────

/// Persistent state for stateful DSL widgets (Checkbox, Toggle, Slider, RadioGroup).
///
/// Keyed by the widget's `state_key` prop (falls back to `on_change` event name).
/// Lives in [`DslBindings::widget_state`] so it survives widget-tree rebuilds.
#[derive(Debug, Clone)]
pub enum WidgetStateValue {
    Bool(bool),
    Float(f32),
    Str(String),
}

// ── DslBindings ───────────────────────────────────────────────────────────

/// Runtime bindings supplied by the application when building a widget tree.
pub struct DslBindings {
    /// Named fonts available to DSL nodes (e.g. `"body"` → `FontId`).
    pub fonts: HashMap<String, FontId>,
    /// Shared event queue. Button `on_click: name` pushes `name` here.
    pub event_queue: Rc<RefCell<Vec<String>>>,
    /// Persistent state for stateful widgets (Checkbox, Toggle, Slider, RadioGroup, TextBox).
    pub widget_state: Rc<RefCell<HashMap<String, WidgetStateValue>>>,
    /// State key of the currently focused TextBox, if any.
    pub focused_widget: Rc<RefCell<Option<String>>>,
}

impl DslBindings {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            event_queue: Rc::new(RefCell::new(Vec::new())),
            widget_state: Rc::new(RefCell::new(HashMap::new())),
            focused_widget: Rc::new(RefCell::new(None)),
        }
    }

    pub fn with_font(mut self, name: impl Into<String>, id: FontId) -> Self {
        self.fonts.insert(name.into(), id);
        self
    }

    /// Drain all pending events from the queue.
    pub fn take_events(&self) -> Vec<String> {
        self.event_queue.borrow_mut().drain(..).collect()
    }
}

impl Default for DslBindings {
    fn default() -> Self {
        Self::new()
    }
}

// ── DslLoader ─────────────────────────────────────────────────────────────

/// Parses and caches `.mkml` documents, resolving component imports.
pub struct DslLoader {
    registry: HashMap<String, DslDocument>,
}

impl DslLoader {
    pub fn new() -> Self {
        Self { registry: HashMap::new() }
    }

    /// Parse a `.mkml` source string into a [`DslDocument`].
    pub fn parse(&self, src: &str) -> Result<DslDocument, ParseError> {
        parse_str(src)
    }

    /// Register a pre-parsed document under an alias so other documents can
    /// reference it with `import "..." as Alias`.
    pub fn register(&mut self, alias: impl Into<String>, doc: DslDocument) {
        self.registry.insert(alias.into(), doc);
    }

    /// Parse and immediately register a source under `alias`.
    pub fn parse_and_register(
        &mut self,
        alias: impl Into<String>,
        src: &str,
    ) -> Result<(), ParseError> {
        let doc = parse_str(src)?;
        self.registry.insert(alias.into(), doc);
        Ok(())
    }

    /// Build an [`Element`] from a previously parsed document.
    ///
    /// All component aliases referenced in the document must already be
    /// registered via [`register`] / [`parse_and_register`].
    pub fn build(&self, doc: &DslDocument, bindings: &DslBindings) -> Element {
        self.build_node(&doc.root, bindings)
    }

    // ── internal ──────────────────────────────────────────────────────────

    fn build_node(&self, node: &Node, bindings: &DslBindings) -> Element {
        match node.widget.as_str() {
            "Text"        => self.build_text(node, bindings),
            "Container"   => self.build_container(node, bindings),
            "Column"      => self.build_column(node, bindings),
            "Row"         => self.build_row(node, bindings),
            "Button"      => self.build_button(node, bindings),
            "Checkbox"    => self.build_checkbox(node, bindings),
            "Toggle"      => self.build_toggle(node, bindings),
            "Slider"      => self.build_slider(node, bindings),
            "RadioGroup"  => self.build_radio_group(node, bindings),
            "ProgressBar" => self.build_progress_bar(node, bindings),
            "TextBox"     => self.build_textbox(node, bindings),
            "ScrollView"  => self.build_scroll_view(node, bindings),
            "Stack"       => self.build_stack(node, bindings),
            alias => {
                if let Some(component) = self.registry.get(alias) {
                    self.build_node(&component.root, bindings)
                } else {
                    Container::new().into()
                }
            }
        }
    }

    // ── Text ──────────────────────────────────────────────────────────────

    fn build_text(&self, node: &Node, bindings: &DslBindings) -> Element {
        let Some(font) = self.resolve_font(node, bindings) else {
            return Container::new().into();
        };
        let text  = node.content.clone().unwrap_or_default();
        let size  = node.prop_f32("size").unwrap_or(14.0);
        let color = node.prop_color("color")
            .unwrap_or_else(|| Color::from_straight(1.0, 1.0, 1.0, 1.0));
        Text::new(text, font, size, color).into()
    }

    // ── Container ─────────────────────────────────────────────────────────

    fn build_container(&self, node: &Node, bindings: &DslBindings) -> Element {
        let mut c = Container::new();
        if let Some(v) = node.prop_f32("padding") {
            c = c.padding_all(v);
        }
        if let Some(edges) = self.parse_edges(node) {
            c = c.padding(edges);
        }
        if let Some(col) = node.prop_color("bg") {
            c = c.background(Paint::Solid(col));
        }
        if let Some(r) = node.prop_f32("corner_radius") {
            c = c.corner_radius(r);
        }
        c = self.apply_border(c, node);
        if let Some(child_node) = node.children.first() {
            c = c.child(self.build_node(child_node, bindings));
        }
        c.into()
    }

    // ── Column ────────────────────────────────────────────────────────────

    fn build_column(&self, node: &Node, bindings: &DslBindings) -> Element {
        let mut col = Column::new();
        // Accept `gap` (preferred) or `spacing` (compat)
        if let Some(v) = node.prop_f32("gap").or_else(|| node.prop_f32("spacing")) {
            col = col.spacing(v);
        }
        if let Some(v) = node.prop_f32("padding") {
            col = col.padding_all(v);
        }
        if let Some(edges) = self.parse_edges(node) {
            col = col.padding(edges);
        }
        col = col.cross_align(self.parse_align(node));
        for child in &node.children {
            col = col.child(self.build_node(child, bindings));
        }
        let elem: Element = col.into();
        self.maybe_wrap_bg(elem, node)
    }

    // ── Row ───────────────────────────────────────────────────────────────

    fn build_row(&self, node: &Node, bindings: &DslBindings) -> Element {
        let mut row = Row::new();
        // Accept `gap` (preferred) or `spacing` (compat)
        if let Some(v) = node.prop_f32("gap").or_else(|| node.prop_f32("spacing")) {
            row = row.spacing(v);
        }
        if let Some(v) = node.prop_f32("padding") {
            row = row.padding_all(v);
        }
        if let Some(edges) = self.parse_edges(node) {
            row = row.padding(edges);
        }
        row = row.cross_align(self.parse_align(node));
        for child in &node.children {
            row = row.child(self.build_node(child, bindings));
        }
        let elem: Element = row.into();
        self.maybe_wrap_bg(elem, node)
    }

    // ── Button ────────────────────────────────────────────────────────────

    fn build_button(&self, node: &Node, bindings: &DslBindings) -> Element {
        let inner: Element = if !node.children.is_empty() {
            if node.children.len() == 1 {
                self.build_node(&node.children[0], bindings)
            } else {
                let mut col = Column::new();
                for c in &node.children {
                    col = col.child(self.build_node(c, bindings));
                }
                col.into()
            }
        } else if let Some(font) = self.resolve_font(node, bindings) {
            let label = node.content.clone().unwrap_or_default();
            let size  = node.prop_f32("font_size").unwrap_or(14.0);
            let color = node.prop_color("text_color")
                .unwrap_or_else(|| Color::from_straight(1.0, 1.0, 1.0, 1.0));
            Text::new(label, font, size, color).into()
        } else {
            Container::new().into()
        };

        let mut btn = Button::new(inner);

        if let Some(col) = node.prop_color("bg") {
            btn = btn.background(col);
        }
        if let Some(col) = node.prop_color("hover_bg") {
            btn = btn.hover_background(col);
        }
        if let Some(col) = node.prop_color("press_bg") {
            btn = btn.press_background(col);
        }
        if let Some(r) = node.prop_f32("corner_radius") {
            btn = btn.corner_radius(r);
        }
        if let Some(v) = node.prop_f32("padding") {
            btn = btn.padding_all(v);
        }
        if let Some(edges) = self.parse_edges(node) {
            btn = btn.padding(edges);
        }
        if let Some(bw) = node.prop_f32("border_width") {
            let bc = node.prop_color("border_color")
                .unwrap_or_else(|| Color::from_straight(1.0, 1.0, 1.0, 0.3));
            btn = btn.border(Border::new(bw, bc));
        }
        if let Some(event_name) = node.prop_str("on_click") {
            let queue = Rc::clone(&bindings.event_queue);
            let name  = event_name.to_string();
            btn = btn.on_click(move || queue.borrow_mut().push(name.clone()));
        }

        btn.into()
    }

    // ── Checkbox ──────────────────────────────────────────────────────────

    fn build_checkbox(&self, node: &Node, bindings: &DslBindings) -> Element {
        let state_key = node.prop_str("state_key")
            .or_else(|| node.prop_str("on_change"))
            .map(|s| s.to_string());

        let default_checked = node.prop_f32("checked").map(|v| v != 0.0).unwrap_or(false);
        let checked = if let Some(key) = &state_key {
            match bindings.widget_state.borrow().get(key.as_str()) {
                Some(WidgetStateValue::Bool(b)) => *b,
                _ => default_checked,
            }
        } else {
            default_checked
        };

        let mut cb = Checkbox::new().checked(checked);

        if let Some(font) = self.resolve_font(node, bindings) { cb = cb.font(font); }
        if let Some(v) = node.prop_f32("font_size") { cb = cb.font_size(v); }
        if let Some(v) = node.prop_color("label_color").or_else(|| node.prop_color("color")) {
            cb = cb.label_color(v);
        }
        if let Some(v) = node.prop_f32("box_size") { cb = cb.box_size(v); }
        if let Some(v) = node.prop_color("checked_color").or_else(|| node.prop_color("accent")) {
            cb = cb.checked_color(v);
        }
        if let Some(v) = node.prop_color("border_color") { cb = cb.border_color(v); }
        if let Some(v) = node.prop_f32("corner_radius") { cb = cb.corner_radius(v); }

        let label = node.content.clone()
            .or_else(|| node.prop_str("label").map(|s| s.to_string()))
            .unwrap_or_default();
        cb = cb.label(label);

        if let Some(event_name) = node.prop_str("on_change") {
            let queue = Rc::clone(&bindings.event_queue);
            let state = Rc::clone(&bindings.widget_state);
            let key   = state_key.unwrap_or_else(|| event_name.to_string());
            let name  = event_name.to_string();
            cb = cb.on_change(move |v| {
                state.borrow_mut().insert(key.clone(), WidgetStateValue::Bool(v));
                queue.borrow_mut().push(name.clone());
            });
        }

        cb.into()
    }

    // ── Toggle ────────────────────────────────────────────────────────────

    fn build_toggle(&self, node: &Node, bindings: &DslBindings) -> Element {
        let state_key = node.prop_str("state_key")
            .or_else(|| node.prop_str("on_change"))
            .map(|s| s.to_string());

        let default_checked = node.prop_f32("checked").map(|v| v != 0.0).unwrap_or(false);
        let checked = if let Some(key) = &state_key {
            match bindings.widget_state.borrow().get(key.as_str()) {
                Some(WidgetStateValue::Bool(b)) => *b,
                _ => default_checked,
            }
        } else {
            default_checked
        };

        let mut tg = Toggle::new().checked(checked);

        if let Some(v) = node.prop_f32("width")  { tg = tg.width(v); }
        if let Some(v) = node.prop_f32("height") { tg = tg.height(v); }
        if let Some(v) = node.prop_color("on_color")    { tg = tg.on_color(v); }
        if let Some(v) = node.prop_color("off_color")   { tg = tg.off_color(v); }
        if let Some(v) = node.prop_color("thumb_color") { tg = tg.thumb_color(v); }

        if let Some(event_name) = node.prop_str("on_change") {
            let queue = Rc::clone(&bindings.event_queue);
            let state = Rc::clone(&bindings.widget_state);
            let key   = state_key.unwrap_or_else(|| event_name.to_string());
            let name  = event_name.to_string();
            tg = tg.on_change(move |v| {
                state.borrow_mut().insert(key.clone(), WidgetStateValue::Bool(v));
                queue.borrow_mut().push(name.clone());
            });
        }

        tg.into()
    }

    // ── Slider ────────────────────────────────────────────────────────────

    fn build_slider(&self, node: &Node, bindings: &DslBindings) -> Element {
        let state_key = node.prop_str("state_key")
            .or_else(|| node.prop_str("on_change"))
            .map(|s| s.to_string());

        let min = node.prop_f32("min").unwrap_or(0.0);
        let max = node.prop_f32("max").unwrap_or(1.0);
        let default_val = node.prop_f32("value").unwrap_or(min);

        let value = if let Some(key) = &state_key {
            match bindings.widget_state.borrow().get(key.as_str()) {
                Some(WidgetStateValue::Float(v)) => *v,
                _ => default_val,
            }
        } else {
            default_val
        };

        let mut sl = Slider::new().min(min).max(max).value(value);

        if let Some(v) = node.prop_f32("track_height")  { sl = sl.track_height(v); }
        if let Some(v) = node.prop_f32("thumb_radius")  { sl = sl.thumb_radius(v); }
        if let Some(v) = node.prop_color("track_color") { sl = sl.track_color(v); }
        if let Some(v) = node.prop_color("fill_color").or_else(|| node.prop_color("accent")) {
            sl = sl.fill_color(v);
        }
        if let Some(v) = node.prop_color("thumb_color") { sl = sl.thumb_color(v); }
        if let Some(v) = node.prop_f32("corner_radius") { sl = sl.corner_radius(v); }

        if let Some(event_name) = node.prop_str("on_change") {
            let queue = Rc::clone(&bindings.event_queue);
            let state = Rc::clone(&bindings.widget_state);
            let key   = state_key.unwrap_or_else(|| event_name.to_string());
            let name  = event_name.to_string();
            sl = sl.on_change(move |v| {
                state.borrow_mut().insert(key.clone(), WidgetStateValue::Float(v));
                queue.borrow_mut().push(name.clone());
            });
        }

        sl.into()
    }

    // ── RadioGroup ────────────────────────────────────────────────────────

    fn build_radio_group(&self, node: &Node, bindings: &DslBindings) -> Element {
        let state_key = node.prop_str("state_key")
            .or_else(|| node.prop_str("on_change"))
            .map(|s| s.to_string());

        let default_sel = node.prop_str("default")
            .or_else(|| node.prop_str("selected"))
            .map(|s| s.to_string());

        let selected = if let Some(key) = &state_key {
            match bindings.widget_state.borrow().get(key.as_str()) {
                Some(WidgetStateValue::Str(s)) => Some(s.clone()),
                _ => default_sel,
            }
        } else {
            default_sel
        };

        let mut rg = RadioGroup::new();

        if let Some(font) = self.resolve_font(node, bindings) { rg = rg.font(font); }
        if let Some(v) = node.prop_f32("font_size")            { rg = rg.font_size(v); }
        if let Some(v) = node.prop_color("label_color").or_else(|| node.prop_color("color")) {
            rg = rg.label_color(v);
        }
        if let Some(v) = node.prop_color("accent")         { rg = rg.selected_color(v); }
        if let Some(v) = node.prop_color("border_color")   { rg = rg.border_color(v); }
        if let Some(v) = node.prop_f32("dot_radius")       { rg = rg.dot_radius(v); }
        if let Some(v) = node.prop_f32("item_gap")         { rg = rg.item_gap(v); }
        if let Some(sel) = selected                         { rg = rg.selected(sel); }

        // Each RadioOption child node: RadioOption "Label" { value: some_value }
        for child in &node.children {
            if child.widget == "RadioOption" {
                let label = child.content.clone().unwrap_or_default();
                let value = child.prop_str("value").unwrap_or(&label).to_string();
                rg = rg.option(label, value);
            }
        }

        if let Some(event_name) = node.prop_str("on_change") {
            let queue = Rc::clone(&bindings.event_queue);
            let state = Rc::clone(&bindings.widget_state);
            let key   = state_key.unwrap_or_else(|| event_name.to_string());
            let name  = event_name.to_string();
            rg = rg.on_change(move |v| {
                state.borrow_mut().insert(key.clone(), WidgetStateValue::Str(v));
                queue.borrow_mut().push(name.clone());
            });
        }

        rg.into()
    }

    // ── TextBox ───────────────────────────────────────────────────────────

    fn build_textbox(&self, node: &Node, bindings: &DslBindings) -> Element {
        let state_key = node.prop_str("state_key")
            .or_else(|| node.prop_str("on_change"))
            .or_else(|| node.prop_str("on_submit"))
            .map(|s| s.to_string());

        let default_text = node.prop_str("text")
            .or_else(|| node.content.as_deref())
            .map(|s| s.to_string())
            .unwrap_or_default();

        let text = if let Some(key) = &state_key {
            match bindings.widget_state.borrow().get(key.as_str()) {
                Some(WidgetStateValue::Str(s)) => s.clone(),
                _ => default_text,
            }
        } else {
            default_text
        };

        let focused = state_key.as_deref()
            .map(|k| bindings.focused_widget.borrow().as_deref() == Some(k))
            .unwrap_or(false);

        let mut tb = TextBox::new().text(text).focused(focused);

        if let Some(font) = self.resolve_font(node, bindings) { tb = tb.font(font); }
        if let Some(v) = node.prop_f32("font_size")               { tb = tb.font_size(v); }
        if let Some(v) = node.prop_color("text_color").or_else(|| node.prop_color("color")) {
            tb = tb.text_color(v);
        }
        if let Some(v) = node.prop_color("placeholder_color")     { tb = tb.placeholder_color(v); }
        if let Some(v) = node.prop_color("bg")                    { tb = tb.bg(v); }
        if let Some(v) = node.prop_color("focused_bg")            { tb = tb.focused_bg(v); }
        if let Some(v) = node.prop_color("border_color")          { tb = tb.border_color(v); }
        if let Some(v) = node.prop_color("focused_border_color").or_else(|| node.prop_color("accent")) {
            tb = tb.focused_border_color(v);
        }
        if let Some(v) = node.prop_f32("corner_radius")           { tb = tb.corner_radius(v); }
        if let Some(v) = node.prop_f32("padding")                 { tb = tb.padding_all(v); }
        if let Some(placeholder) = node.prop_str("placeholder")   {
            tb = tb.placeholder(placeholder.to_string());
        }

        // on_focus: mark this widget as focused in bindings
        if let Some(key) = state_key.clone() {
            let focused_slot = Rc::clone(&bindings.focused_widget);
            let k = key.clone();
            tb = tb.on_focus(move || { *focused_slot.borrow_mut() = Some(k.clone()); });
        }

        // on_change: update text state + fire event
        if let Some(event_name) = node.prop_str("on_change") {
            let queue = Rc::clone(&bindings.event_queue);
            let state = Rc::clone(&bindings.widget_state);
            let key   = state_key.clone().unwrap_or_else(|| event_name.to_string());
            let name  = event_name.to_string();
            tb = tb.on_change(move |v| {
                state.borrow_mut().insert(key.clone(), WidgetStateValue::Str(v));
                queue.borrow_mut().push(name.clone());
            });
        }

        // on_submit: fire event
        if let Some(event_name) = node.prop_str("on_submit") {
            let queue = Rc::clone(&bindings.event_queue);
            let name  = event_name.to_string();
            tb = tb.on_submit(move |_v| { queue.borrow_mut().push(name.clone()); });
        }

        tb.into()
    }

    // ── ProgressBar ───────────────────────────────────────────────────────

    fn build_progress_bar(&self, node: &Node, _bindings: &DslBindings) -> Element {
        let mut pb = ProgressBar::new();
        if let Some(v) = node.prop_f32("value")          { pb = pb.value(v); }
        if let Some(v) = node.prop_f32("height")         { pb = pb.height(v); }
        if let Some(v) = node.prop_color("track_color")  { pb = pb.track_color(v); }
        if let Some(v) = node.prop_color("fill_color").or_else(|| node.prop_color("accent")) {
            pb = pb.fill_color(v);
        }
        if let Some(v) = node.prop_f32("corner_radius")  { pb = pb.corner_radius(v); }
        pb.into()
    }

    // ── ScrollView ────────────────────────────────────────────────────────

    fn build_scroll_view(&self, node: &Node, bindings: &DslBindings) -> Element {
        let state_key = node.prop_str("state_key")
            .or_else(|| node.prop_str("on_scroll"))
            .map(|s| s.to_string());

        let default_offset = node.prop_f32("offset").unwrap_or(0.0);
        let offset = if let Some(key) = &state_key {
            match bindings.widget_state.borrow().get(key.as_str()) {
                Some(WidgetStateValue::Float(v)) => *v,
                _ => default_offset,
            }
        } else {
            default_offset
        };

        let child: Element = if let Some(child_node) = node.children.first() {
            self.build_node(child_node, bindings)
        } else {
            Container::new().into()
        };

        let mut sv = ScrollView::new(child).scroll_to(offset);

        if let Some(v) = node.prop_f32("line_height")      { sv = sv.line_height(v); }
        if let Some(v) = node.prop_f32("show_scrollbar")   { sv = sv.show_scrollbar(v != 0.0); }

        if let Some(event_name) = node.prop_str("on_scroll") {
            let queue = Rc::clone(&bindings.event_queue);
            let state = Rc::clone(&bindings.widget_state);
            let key   = state_key.unwrap_or_else(|| event_name.to_string());
            let name  = event_name.to_string();
            sv = sv.on_scroll(move |v| {
                state.borrow_mut().insert(key.clone(), WidgetStateValue::Float(v));
                queue.borrow_mut().push(name.clone());
            });
        }

        sv.into()
    }

    // ── Stack ─────────────────────────────────────────────────────────────

    fn build_stack(&self, node: &Node, bindings: &DslBindings) -> Element {
        let mut stack = Stack::new();

        if let Some(v) = parse_size_hint(node, "width")  { stack = stack.width(v); }
        if let Some(v) = parse_size_hint(node, "height") { stack = stack.height(v); }
        if let Some(v) = node.prop_color("bg")           { stack = stack.bg(v); }

        for child_node in &node.children {
            let element = self.build_node(child_node, bindings);
            let item = StackItem {
                element,
                left:   parse_anchor_val(child_node, "left"),
                top:    parse_anchor_val(child_node, "top"),
                right:  parse_anchor_val(child_node, "right"),
                bottom: parse_anchor_val(child_node, "bottom"),
                width:  parse_size_hint(child_node, "width").unwrap_or(SizeHint::Natural),
                height: parse_size_hint(child_node, "height").unwrap_or(SizeHint::Natural),
            };
            stack = stack.item(item);
        }

        stack.into()
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    /// Wrap `elem` in a Container when the node carries visual decoration
    /// properties (bg, border, corner_radius) that Column/Row don't support
    /// natively.
    fn maybe_wrap_bg(&self, elem: Element, node: &Node) -> Element {
        let bg     = node.prop_color("bg");
        let radius = node.prop_f32("corner_radius");
        let has_border = node.prop_f32("border_width").is_some();

        if bg.is_some() || has_border || radius.is_some() {
            let mut c = Container::new().child(elem);
            if let Some(color) = bg {
                c = c.background(Paint::Solid(color));
            }
            c = self.apply_border(c, node);
            if let Some(r) = radius {
                c = c.corner_radius(r);
            }
            c.into()
        } else {
            elem
        }
    }

    fn resolve_font(&self, node: &Node, bindings: &DslBindings) -> Option<FontId> {
        node.prop_str("font")
            .and_then(|name| bindings.fonts.get(name).copied())
            .or_else(|| bindings.fonts.values().next().copied())
    }

    /// Parse `align` (preferred) or `cross_align` (compat) property.
    fn parse_align(&self, node: &Node) -> Align {
        let val = node.prop_str("align")
            .or_else(|| node.prop_str("cross_align"))
            .unwrap_or("stretch");
        match val {
            "start"   => Align::Start,
            "center"  => Align::Center,
            "end"     => Align::End,
            _         => Align::Stretch,
        }
    }

    fn parse_edges(&self, node: &Node) -> Option<Edges> {
        let top    = node.prop_f32("padding_top");
        let right  = node.prop_f32("padding_right");
        let bottom = node.prop_f32("padding_bottom");
        let left   = node.prop_f32("padding_left");

        if top.or(right).or(bottom).or(left).is_some() {
            Some(Edges {
                top:    top.unwrap_or(0.0),
                right:  right.unwrap_or(0.0),
                bottom: bottom.unwrap_or(0.0),
                left:   left.unwrap_or(0.0),
            })
        } else {
            None
        }
    }

    fn apply_border(&self, c: Container, node: &Node) -> Container {
        if let Some(bw) = node.prop_f32("border_width") {
            let bc = node.prop_color("border_color")
                .unwrap_or_else(|| Color::from_straight(1.0, 1.0, 1.0, 0.3));
            c.border(Border::new(bw, bc))
        } else {
            c
        }
    }
}

impl Default for DslLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ── DSL property helpers for anchor/size types ────────────────────────────

/// Parse a `SizeHint` from a node property.
///
/// Accepts:
/// - `Number(v)` → `SizeHint::Px(v)`
/// - `Ident("fill")` / `Str("fill")` → `SizeHint::Fill`
fn parse_size_hint(node: &Node, key: &str) -> Option<SizeHint> {
    match node.prop(key)? {
        Value::Number(v) => Some(SizeHint::Px(*v)),
        Value::Ident(s) | Value::Str(s) => match s.as_str() {
            "fill"    => Some(SizeHint::Fill),
            "natural" => Some(SizeHint::Natural),
            _ => None,
        },
        _ => None,
    }
}

/// Parse an `AnchorVal` from a node property.
///
/// Accepts:
/// - `Number(v)` → `AnchorVal::Px(v)`
fn parse_anchor_val(node: &Node, key: &str) -> Option<AnchorVal> {
    match node.prop(key)? {
        Value::Number(v) => Some(AnchorVal::Px(*v)),
        _ => None,
    }
}
