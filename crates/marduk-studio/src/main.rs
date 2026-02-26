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
        self.draw_list.clear();

        let t = ctx.time.frame_index as f32 * 0.012;
        let phys = ctx.window.window.inner_size();
        let scale = ctx.window.window.scale_factor();
        let logical: winit::dpi::LogicalSize<f64> = phys.to_logical(scale);

        let cx = logical.width as f32 / 2.0;
        let cy = logical.height as f32 / 2.0;
        let orbit = 110.0_f32;
        let size = Vec2::new(220.0, 220.0);
        let half = size * 0.5;

        // Three blobs orbiting the center, 120° apart, 60% opacity.
        let blobs: [(f32, Color); 3] = [
            (0.0,                               Color::from_straight(1.0, 0.15, 0.15, 0.6)),
            (std::f32::consts::TAU / 3.0,       Color::from_straight(0.15, 1.0, 0.25, 0.6)),
            (2.0 * std::f32::consts::TAU / 3.0, Color::from_straight(0.2, 0.5, 1.0, 0.6)),
        ];

        for (phase, color) in blobs {
            let angle = t + phase;
            let origin = Vec2::new(
                cx + orbit * angle.cos() - half.x,
                cy + orbit * angle.sin() - half.y,
            );
            self.draw_list.push_solid_rect(ZIndex::new(0), Rect::from_origin_size(origin, size), color);
        }

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

        // Clear to near-black.
        {
            let _rpass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("marduk-studio clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.04, g: 0.04, b: 0.04, a: 1.0 }),
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
        let viewport = Viewport::new(logical.width as f32, logical.height as f32);
        let rctx = RenderCtx::new(ctx.gpu.device(), ctx.gpu.queue(), ctx.gpu.surface_format(), viewport);
        let mut target = RenderTarget::new(&mut frame.encoder, &frame.view);
        self.rect_renderer.render(&rctx, &mut target, &mut self.draw_list);

        ctx.window.window.pre_present_notify();
        ctx.gpu.submit(frame);

        AppControl::Continue
    }
}

fn main() -> Result<()> {
    init_logging(LoggingConfig::default());
    Runtime::run(RuntimeConfig::default(), GpuInit::default(), StudioApp::default())
}
