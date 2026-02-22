//! Core engine-facing contracts.
//!
//! This module defines the stable interface between the runtime (platform loop)
//! and higher layers (studio, UI, tools). It avoids leaking runtime internals
//! into user code and provides a consistent per-frame context.

mod app;
mod ctx;

pub use app::{App, AppControl};
pub use ctx::{FrameCtx, WindowCtx};