use anyhow::Result;

use marduk_engine::core::{App, AppControl, FrameCtx};
use marduk_engine::device::GpuInit;
use marduk_engine::logging::{init_logging, LoggingConfig};
use marduk_engine::window::{Runtime, RuntimeConfig};

struct StudioApp {
    spawned_extra: bool,
}

impl Default for StudioApp {
    fn default() -> Self {
        Self {
            spawned_extra: false,
        }
    }
}

impl App for StudioApp {
    fn on_frame(&mut self, ctx: &mut FrameCtx<'_, '_>) -> AppControl {
        // Minimal: clear/submit a frame to verify end-to-end rendering.
        // For now, this duplicates the bootstrap clear that used to live in the runtime.
        // Later, this will be replaced by marduk-ui rendering.

        let mut frame = match ctx.gpu.begin_frame() {
            Ok(f) => f,
            Err(err) => {
                let action = ctx.gpu.handle_surface_error(err);
                if matches!(action, marduk_engine::device::SurfaceErrorAction::Fatal) {
                    return AppControl::Exit;
                }
                return AppControl::Continue;
            }
        };

        // Clear pass.
        {
            let mut rpass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            drop(rpass);
        }

        ctx.gpu.submit(frame);

        // Demo multi-window command (spawn once).
        if !self.spawned_extra && ctx.time.frame_index == 60 {
            ctx.runtime.create_window(RuntimeConfig {
                title: "marduk-studio (second window)".to_string(),
                initial_size: winit::dpi::LogicalSize::new(900.0, 600.0),
            });
            self.spawned_extra = true;
        }

        AppControl::Continue
    }
}

fn main() -> Result<()> {
    init_logging(LoggingConfig::default());

    let initial = RuntimeConfig::default();
    let gpu_init = GpuInit::default();

    Runtime::run(initial, gpu_init, StudioApp::default())
}