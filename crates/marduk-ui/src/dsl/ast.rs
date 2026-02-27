use marduk_engine::paint::Color;

// ── Value ─────────────────────────────────────────────────────────────────

/// A literal value in a property.
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

// ── Prop ──────────────────────────────────────────────────────────────────

/// A single `key: value` property inside a widget block.
#[derive(Debug, Clone, PartialEq)]
pub struct Prop {
    pub key: String,
    pub value: Value,
}

// ── Node ──────────────────────────────────────────────────────────────────

/// A widget instantiation node in the tree.
///
/// ```mkml
/// Button "Save" {
///     on_click: save
///     bg: #4c6ef5ff
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Widget type name or component alias: `"Column"`, `"Text"`, `"Sidebar"`.
    pub widget: String,
    /// Optional inline string content (e.g. the text label for `Text` or `Button`).
    pub content: Option<String>,
    /// Properties inside the block (`key: value` lines).
    pub props: Vec<Prop>,
    /// Nested child widget nodes inside the block.
    pub children: Vec<Node>,
}

impl Node {
    /// Look up a property value by key.
    pub fn prop(&self, key: &str) -> Option<&Value> {
        self.props.iter().find(|p| p.key == key).map(|p| &p.value)
    }

    /// Get a property as `f32` if it is a `Number`.
    pub fn prop_f32(&self, key: &str) -> Option<f32> {
        match self.prop(key)? {
            Value::Number(v) => Some(*v),
            _ => None,
        }
    }

    /// Get a property as `&str` if it is a `Str` or `Ident`.
    pub fn prop_str(&self, key: &str) -> Option<&str> {
        match self.prop(key)? {
            Value::Str(s) | Value::Ident(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Get a property as `Color` if it is a `Color`.
    pub fn prop_color(&self, key: &str) -> Option<Color> {
        match self.prop(key)? {
            Value::Color(c) => Some(*c),
            _ => None,
        }
    }
}

// ── Import ────────────────────────────────────────────────────────────────

/// `import "path/to/file.mkml" as Alias`
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
