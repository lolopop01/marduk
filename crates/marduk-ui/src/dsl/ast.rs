use marduk_engine::paint::Color;

// ── Value ─────────────────────────────────────────────────────────────────

/// A literal value in an attribute list.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Quoted string: `"hello"`
    Str(String),
    /// Floating-point literal: `16.0` or `16`
    Number(f32),
    /// Color literal: `#rrggbbaa` (8 hex digits, straight alpha)
    Color(Color),
    /// Unquoted identifier: used for font names, event names, enum variants
    Ident(String),
}

// ── Attr ──────────────────────────────────────────────────────────────────

/// A single `key=value` pair inside `[...]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Attr {
    pub key: String,
    pub value: Value,
}

// ── Node ──────────────────────────────────────────────────────────────────

/// A widget instantiation node in the tree.
///
/// ```mkml
/// Button "Save" [on_click=save, bg=#4c6ef5ff] { ... }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Widget type name or component alias: `"Column"`, `"Text"`, `"Sidebar"`.
    pub widget: String,
    /// Optional inline string content (e.g. the text label for `Text` or `Button`).
    pub content: Option<String>,
    /// Attribute list inside `[...]`.
    pub attrs: Vec<Attr>,
    /// Nested child nodes inside `{...}`.
    pub children: Vec<Node>,
}

impl Node {
    /// Look up an attribute value by key.
    pub fn attr(&self, key: &str) -> Option<&Value> {
        self.attrs.iter().find(|a| a.key == key).map(|a| &a.value)
    }

    /// Get an attribute as `f32` if it is a `Number`.
    pub fn attr_f32(&self, key: &str) -> Option<f32> {
        match self.attr(key)? {
            Value::Number(v) => Some(*v),
            _ => None,
        }
    }

    /// Get an attribute as `&str` if it is a `Str` or `Ident`.
    pub fn attr_str(&self, key: &str) -> Option<&str> {
        match self.attr(key)? {
            Value::Str(s) | Value::Ident(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Get an attribute as `Color` if it is a `Color`.
    pub fn attr_color(&self, key: &str) -> Option<Color> {
        match self.attr(key)? {
            Value::Color(c) => Some(*c),
            _ => None,
        }
    }
}

// ── Import ────────────────────────────────────────────────────────────────

/// `use "path/to/file.mkml" as Alias;`
#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub path: String,
    pub alias: String,
}

// ── DslDocument ───────────────────────────────────────────────────────────

/// The top-level parse result for a `.mkml` source file.
#[derive(Debug, Clone, PartialEq)]
pub struct DslDocument {
    pub imports: Vec<Import>,
    pub root: Node,
}
