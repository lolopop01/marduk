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