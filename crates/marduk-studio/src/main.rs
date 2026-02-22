use anyhow::Result;

use marduk_engine::coords::{Rect, Vec2, Viewport};
use marduk_engine::core::{App, AppControl, FrameCtx};
use marduk_engine::device::{GpuInit, SurfaceErrorAction};
use marduk_engine::logging::{init_logging, LoggingConfig};
use marduk_engine::paint::Color;
use marduk_engine::render::{RenderCtx, RenderTarget};
use marduk_engine::render::shapes::rect::RectRenderer;
use marduk_engine::scene::{DrawList, ZIndex};
use marduk_engine::window::{Runtime, RuntimeConfig};

struct StudioApp {
    rect_renderer: RectRenderer,
    draw_list: DrawList,
}

impl Default for StudioApp {
    fn default() -> Self {
        Self {
            rect_renderer: RectRenderer::new(),
            draw_list: DrawList::new(),
        }
    }
}

impl App for StudioApp {
    fn on_frame(&mut self, ctx: &mut FrameCtx<'_, '_>) -> AppControl {
        // Build a flashing square draw list.
        self.draw_list.clear();

        let flash_on = (ctx.time.frame_index / 30) % 2 == 0; // ~0.5s at 60fps
        let color = if flash_on {
            // White, fully opaque (premultiplied)
            Color::from_straight(1.0, 1.0, 1.0, 1.0)
        } else {
            // Transparent (no-op visually)
            Color::transparent()
        };

        // 100x100 square at (100,100)
        let rect = Rect::from_origin_size(Vec2::new(100.0, 100.0), Vec2::new(100.0, 100.0));
        self.draw_list.push_solid_rect(ZIndex::new(0), rect, color);

        // Acquire frame.
        let mut frame = match ctx.gpu.begin_frame() {
            Ok(f) => f,
            Err(err) => {
                let action = ctx.gpu.handle_surface_error(err);
                if action == SurfaceErrorAction::Fatal {
                    return AppControl::Exit;
                }
                return AppControl::Continue;
            }
        };

        // Clear background (black).
        {
            let _rpass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("marduk-studio clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        // Compute viewport in logical pixels.
        let phys = ctx.window.window.inner_size();
        let scale = ctx.window.window.scale_factor();
        let logical: winit::dpi::LogicalSize<f64> = phys.to_logical(scale);
        let viewport = Viewport::new(logical.width as f32, logical.height as f32);

        let rctx = RenderCtx::new(
            ctx.gpu.device(),
            ctx.gpu.queue(),
            ctx.gpu.surface_format(),
            viewport,
        );

        let mut target = RenderTarget::new(&mut frame.encoder, &frame.view);

        // Draw rectangles (solid only in v0).
        self.rect_renderer.render(&rctx, &mut target, &mut self.draw_list);

        // Present (Wayland requires pre-present notification to properly drive frame callbacks).
        ctx.window.window.pre_present_notify();
        ctx.gpu.submit(frame);

        AppControl::Continue
    }
}

fn main() -> Result<()> {
    init_logging(LoggingConfig::default());

    let initial = RuntimeConfig::default();
    let gpu_init = GpuInit::default();

    Runtime::run(initial, gpu_init, StudioApp::default())
}