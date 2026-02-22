//! Marduk engine crate.
//!
//! This crate owns the platform + GPU runtime pieces used by higher layers.

pub mod device;
pub mod window;
pub mod input;
pub mod time;
pub mod core;

pub mod logging;
pub mod coords;
pub mod render;
pub mod paint;
mod scene;