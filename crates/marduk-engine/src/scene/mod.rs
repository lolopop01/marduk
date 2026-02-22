//! Scene (draw stream) types.
//!
//! Responsibilities:
//! - store renderer-agnostic draw commands
//! - provide deterministic ordering (z-index + insertion order)
//! - keep shape-specific helpers isolated per shape file under `scene::shapes`

mod cmd;
mod key;
mod list;
mod z_index;

pub mod shapes;

pub use cmd::DrawCmd;
pub use key::SortKey;
pub use list::{DrawItem, DrawList};
pub use z_index::ZIndex;