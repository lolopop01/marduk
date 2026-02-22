//! Time subsystem.
//!
//! Provides stable, testable frame timing utilities without coupling to the runtime.
//! Intended usage:
//! - one `FrameClock` per window (or per render loop)
//! - call `tick()` once per presented frame to obtain `FrameTime`

mod frame_clock;

pub use frame_clock::{FrameClock, FrameTime};