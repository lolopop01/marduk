//! `.mkml` widget builder for `marduk-ui`.
//!
//! Parsing (lexer / parser / AST) lives in the `marduk-mkml` crate so it can
//! be consumed by language-server tooling without engine dependencies.
//! This module re-exports those types at their original paths to keep the
//! public API unchanged.
//!
//! The only piece that stays here is [`builder`], which converts a parsed
//! [`DslDocument`] into a live widget tree using `marduk-ui` widget types.

// ── Re-exports from marduk-mkml ───────────────────────────────────────────

pub use marduk_mkml::ast;
pub use marduk_mkml::error;
pub use marduk_mkml::lexer;
pub use marduk_mkml::parser;

pub use marduk_mkml::DslDocument;
pub use marduk_mkml::ParseError;
pub use marduk_mkml::parse_str;

// ── Widget builder (marduk-ui–specific) ───────────────────────────────────

pub mod builder;
pub use builder::{DslBindings, DslLoader, WidgetStateValue};
