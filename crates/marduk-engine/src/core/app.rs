use winit::event::WindowEvent;
use winit::window::WindowId;

use super::ctx::FrameCtx;

/// Control directive returned by app callbacks.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AppControl {
    Continue,
    Exit,
}

/// Application contract implemented by higher layers.
pub trait App {
    /// Called for window events.
    fn on_window_event(&mut self, window_id: WindowId, event: &WindowEvent) -> AppControl {
        let _ = (window_id, event);
        AppControl::Continue
    }

    /// Called once per rendered frame per window.
    fn on_frame(&mut self, ctx: &mut FrameCtx<'_, '_>) -> AppControl;
}