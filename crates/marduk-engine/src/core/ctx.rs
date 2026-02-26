use winit::window::{Window, WindowId};

use crate::coords::Viewport;
use crate::device::{Gpu, SurfaceErrorAction};
use crate::input::{InputFrame, InputState};
use crate::paint::Color;
use crate::render::{RenderCtx, RenderTarget};
use crate::time::FrameTime;
use crate::window::RuntimeCtx;

use super::app::AppControl;

/// Per-window handles and immutable window metadata.
pub struct WindowCtx<'a> {
    pub id:     WindowId,
    pub window: &'a Window,
}

impl<'a> WindowCtx<'a> {
    /// Returns the logical window size as `(width, height)` in logical pixels.
    pub fn logical_size(&self) -> (f32, f32) {
        let phys  = self.window.inner_size();
        let scale = self.window.scale_factor();
        let logi: winit::dpi::LogicalSize<f64> = phys.to_logical(scale);
        (logi.width as f32, logi.height as f32)
    }
}

/// Per-frame context passed to `core::App::on_frame`.
///
/// Lifetimes:
/// - `'a` is the duration of the callback invocation
/// - `'w` is the window-borrow lifetime carried by `Gpu<'w>`
pub struct FrameCtx<'a, 'w> {
    pub window:       WindowCtx<'a>,
    pub gpu:          &'a mut Gpu<'w>,
    pub input:        &'a InputState,
    pub input_frame:  &'a InputFrame,
    pub time:         FrameTime,
    pub runtime:      &'a mut RuntimeCtx,
}

impl<'a, 'w> FrameCtx<'a, 'w> {
    /// Clears the surface with `clear`, calls `draw` with a ready [`RenderCtx`] and
    /// [`RenderTarget`], then presents the frame.
    ///
    /// This is the primary entry point for rendering: it handles frame acquisition,
    /// the clear pass, viewport construction, and presentation. The caller only
    /// needs to issue draw calls inside the closure.
    ///
    /// Returns [`AppControl::Continue`] on success or [`AppControl::Exit`] on a
    /// fatal surface error.
    pub fn render<F>(&mut self, clear: Color, draw: F) -> AppControl
    where
        F: FnOnce(&RenderCtx<'_>, &mut RenderTarget<'_>),
    {
        let (w, h) = self.window.logical_size();

        let mut frame = match self.gpu.begin_frame() {
            Ok(f) => f,
            Err(err) => {
                let action = self.gpu.handle_surface_error(err);
                if action == SurfaceErrorAction::Fatal {
                    return AppControl::Exit;
                }
                return AppControl::Continue;
            }
        };

        // Clear pass — dropped before the encoder is moved into submit().
        {
            let _rpass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("marduk clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view:           &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load:  wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear.r as f64,
                            g: clear.g as f64,
                            b: clear.b as f64,
                            a: clear.a as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes:         None,
                occlusion_query_set:      None,
                multiview_mask:           None,
            });
        }

        let rctx = RenderCtx::new(
            self.gpu.device(),
            self.gpu.queue(),
            self.gpu.surface_format(),
            Viewport::new(w, h),
        );

        // RenderTarget borrows frame.encoder; dropped before submit() takes frame.
        {
            let mut target = RenderTarget::new(&mut frame.encoder, &frame.view);
            draw(&rctx, &mut target);
        }

        self.window.window.pre_present_notify();
        self.gpu.submit(frame);

        AppControl::Continue
    }
}
