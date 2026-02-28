use winit::window::{CursorIcon, Window, WindowId};

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

    /// Sets the mouse cursor shape for this window.
    ///
    /// Call each frame to keep the cursor updated (e.g. `Arrow` normally,
    /// `Pointer` over a button, `Text` over a text field, etc.).
    pub fn set_cursor(&self, cursor: CursorIcon) {
        self.window.set_cursor(cursor);
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
    /// Equivalent to [`render_scaled`] with `zoom = 1.0`.
    pub fn render<F>(&mut self, clear: Color, draw: F) -> AppControl
    where
        F: FnOnce(&RenderCtx<'_>, &mut RenderTarget<'_>),
    {
        self.render_scaled(1.0, clear, draw)
    }

    /// Like [`render`] but applies a zoom/scale factor.
    ///
    /// The `zoom` factor shrinks the logical viewport seen by renderers
    /// (`viewport = window_size / zoom`) while scaling up the physical pixel
    /// mapping (`scale_factor *= zoom`).  The net effect is that all draw-list
    /// coordinates — which live in the *zoomed* logical space — appear
    /// `zoom × larger` on screen without any changes to the renderers or
    /// shaders.
    ///
    /// Mouse coordinates should be divided by the same `zoom` factor before
    /// being compared against widget rects.
    ///
    /// `zoom` is clamped to `[0.05, 32.0]`.
    pub fn render_scaled<F>(&mut self, zoom: f32, clear: Color, draw: F) -> AppControl
    where
        F: FnOnce(&RenderCtx<'_>, &mut RenderTarget<'_>),
    {
        let zoom = zoom.clamp(0.05, 32.0);
        let (w, h) = self.window.logical_size();
        let scale_factor = self.window.window.scale_factor() as f32;

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

        // Zoom is applied by shrinking the logical viewport and scaling up
        // scale_factor by the same amount.  Renderers upload `viewport` to
        // their viewport UBO (used in NDC conversion) and use `scale_factor`
        // for scissor rect conversion — both adjustments cancel out perfectly.
        let rctx = RenderCtx::new(
            self.gpu.device(),
            self.gpu.queue(),
            self.gpu.surface_format(),
            Viewport::new(w / zoom, h / zoom),
            scale_factor * zoom,
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
