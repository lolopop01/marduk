use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;
use marduk_engine::text::FontId;

use crate::constraints::Edges;
use crate::dsl::ast::{DslDocument, Node};
use crate::dsl::error::ParseError;
use crate::dsl::parser::parse_str;
use crate::widget::Element;
use crate::widgets::{
    button::Button,
    container::Container,
    flex::{Align, Column, Row},
    text::Text,
};

// ── DslBindings ───────────────────────────────────────────────────────────

/// Runtime bindings supplied by the application when building a widget tree.
pub struct DslBindings {
    /// Named fonts available to DSL nodes (e.g. `"body"` → `FontId`).
    pub fonts: HashMap<String, FontId>,
    /// Shared event queue. Button `on_click: name` pushes `name` here.
    pub event_queue: Rc<RefCell<Vec<String>>>,
}

impl DslBindings {
    pub fn new() -> Self {
        Self { fonts: HashMap::new(), event_queue: Rc::new(RefCell::new(Vec::new())) }
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
            "Text"      => self.build_text(node, bindings),
            "Container" => self.build_container(node, bindings),
            "Column"    => self.build_column(node, bindings),
            "Row"       => self.build_row(node, bindings),
            "Button"    => self.build_button(node, bindings),
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

    // ── Helpers ───────────────────────────────────────────────────────────

    /// If a node has a `bg` property, wrap `elem` in a Container with that background.
    /// Used for Column/Row which don't have built-in background support.
    fn maybe_wrap_bg(&self, elem: Element, node: &Node) -> Element {
        if let Some(bg) = node.prop_color("bg") {
            Container::new()
                .background(Paint::Solid(bg))
                .child(elem)
                .into()
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
