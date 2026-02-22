use winit::window::{Window, WindowId};

use crate::device::Gpu;
use crate::input::{InputFrame, InputState};
use crate::time::FrameTime;
use crate::window::RuntimeCtx;

/// Per-window handles and immutable window metadata.
pub struct WindowCtx<'a> {
    pub id: WindowId,
    pub window: &'a Window,
}

/// Per-frame context passed to `core::App::on_frame`.
///
/// Lifetimes:
/// - `'a` is the duration of the callback invocation
/// - `'w` is the window-borrow lifetime carried by `Gpu<'w>`
pub struct FrameCtx<'a, 'w> {
    pub window: WindowCtx<'a>,
    pub gpu: &'a mut Gpu<'w>,
    pub input: &'a InputState,
    pub input_frame: &'a InputFrame,
    pub time: FrameTime,
    pub runtime: &'a mut RuntimeCtx,
}