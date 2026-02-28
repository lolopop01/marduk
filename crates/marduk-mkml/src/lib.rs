//! Lexer, parser, and AST for the **Marduk Markup Language** (`.mkml`).
//!
//! This crate is intentionally dependency-free so it can be consumed by
//! language-server tooling, editors, and linters without pulling in any
//! engine or GPU code.
//!
//! # Structure
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`ast`] | `DslDocument`, `Node`, `Prop`, `Value`, `Import` |
//! | [`error`] | `ParseError` |
//! | [`lexer`] | `Lexer`, `Token` |
//! | [`parser`] | `parse_str` entry point |
//!
//! # Quick start
//!
//! ```rust
//! use marduk_mkml::parse_str;
//!
//! let src = r#"
//!     Column {
//!         gap: 8
//!         Text "Hello" { color: #ffffffff }
//!     }
//! "#;
//!
//! let doc = parse_str(src).unwrap();
//! assert_eq!(doc.root.widget, "Column");
//! ```

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;

pub use ast::DslDocument;
pub use error::ParseError;
pub use parser::parse_str;

#[cfg(test)]
mod parse_tests {
    use super::*;

    fn ok(src: &str) { parse_str(src).unwrap(); }
    fn err(src: &str) { parse_str(src).unwrap_err(); }

    #[test] fn empty_widget() { ok("Container { }"); }
    #[test] fn widget_with_props() {
        ok(r#"Column { gap: 8  Text "hello" { size: 14  color: #ffffffff } }"#);
    }
    #[test] fn nested_widgets() {
        ok("Column { gap: 10  Row { gap: 8  Container { bg: #ff000088 } } }");
    }
    #[test] fn block_comment() {
        ok("/* header */ Column { /* body */ gap: 8 /* tail */ }");
    }
    #[test] fn line_comment() {
        ok("// top\nColumn {\n    // inside\n    gap: 8\n}");
    }
    #[test] fn color_6digit() { ok("Container { bg: #aabbcc }"); }
    #[test] fn color_8digit() { ok("Container { bg: #aabbccdd }"); }
    #[test] fn negative_number() { ok("Slider { min: -10  max: 10  value: -5 }"); }
    #[test] fn float_number() { ok("ProgressBar { value: 0.75 }"); }
    #[test] fn string_content() { ok(r#"Text "hello world" { size: 12 }"#); }
    #[test] fn string_escape() { ok(r#"Text "say \"hi\"" { size: 12 }"#); }
    #[test] fn import_as() {
        ok(r#"import "sidebar.mkml" as Sidebar  Column { gap: 0 }"#);
    }
    #[test] fn stack_anchors() {
        ok("Stack { bg: #000000ff  Column { top: 0  left: 0  right: 0  height: 48 } }");
    }
    #[test] fn scroll_view() {
        ok("ScrollView { on_scroll: scroll  Column { gap: 8 } }");
    }
    #[test] fn full_button() {
        ok(r#"Button "OK" {
            font: body  font_size: 14  text_color: #ffffffff
            bg: #1a1a2aff  hover_bg: #2a2a3aff  press_bg: #3a3a4aff
            corner_radius: 4  padding: 8  on_click: submit
        }"#);
    }
    #[test] fn radio_group() {
        ok(r#"RadioGroup {
            state_key: mode  default: auto  accent: #00aaffff
            RadioOption "Auto"   { value: auto }
            RadioOption "Manual" { value: manual }
        }"#);
    }
    #[test] fn textbox() {
        ok(r#"TextBox {
            state_key: msg  placeholder: "type here..."
            bg: #111111ff  accent: #00e5a0ff  corner_radius: 3  padding: 8
            on_change: msg_changed  on_submit: msg_submit
        }"#);
    }
    #[test] fn toggle_and_checkbox() {
        ok(r#"Column {
            Toggle { state_key: active  checked: true  on_color: #00ff00ff  on_change: toggled }
            Checkbox "Enabled" { state_key: en  checked: false  on_change: checked }
        }"#);
    }
    #[test] fn err_bad_color() { err("Container { bg: #xyz }"); }
    #[test] fn err_unclosed_string() { err(r#"Text "oops { }"#); }
    #[test] fn err_double_colon() { err("Column { gap: : 8 }"); }
}