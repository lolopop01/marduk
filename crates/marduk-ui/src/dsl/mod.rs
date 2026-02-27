//! `.mkml` DSL parser and widget builder for `marduk-ui`.
//!
//! # Overview
//!
//! The DSL lets you author widget trees in separate `.mkml` files and wire
//! named events to Rust callbacks via a shared queue — no closures need to
//! cross the file boundary.
//!
//! ## Format
//!
//! ```mkml
//! // Import a reusable component
//! use "toolbar.mkml" as Toolbar;
//!
//! Column [padding=16, spacing=8] {
//!     Text "Hello!" [font=body, size=20, color=#ffffffff]
//!     Toolbar
//!     Button "Save" [font=body, on_click=save, bg=#4c6ef5ff, corner_radius=6,
//!                    hover_bg=#748ffcff, press_bg=#5c7cfaff, padding=10]
//! }
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use std::rc::Rc;
//! use marduk_ui::dsl::{DslBindings, DslLoader};
//!
//! // Set up loader and register component docs.
//! let mut loader = DslLoader::new();
//! loader.parse_and_register("Toolbar", include_str!("../ui/toolbar.mkml")).unwrap();
//!
//! // Set up bindings once; reuse every frame.
//! let bindings = DslBindings::new().with_font("body", font_id);
//!
//! // Parse the root document.
//! let doc = loader.parse(include_str!("../ui/main.mkml")).unwrap();
//!
//! // Each frame: build the widget tree and drain events.
//! let root = loader.build(&doc, &bindings);
//! let draw_list = ui_scene.frame(root, viewport, &input);
//! for event in bindings.take_events() {
//!     match event.as_str() {
//!         "save" => save_file(),
//!         _ => {}
//!     }
//! }
//! ```

pub mod ast;
pub mod builder;
pub mod error;
pub mod lexer;
pub mod parser;

pub use ast::DslDocument;
pub use builder::{DslBindings, DslLoader};
pub use error::ParseError;
pub use parser::parse_str;
