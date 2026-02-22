use anyhow::{Context, Result};
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window};

use crate::device::{Gpu, GpuInit, SurfaceErrorAction};

/// Window/runtime configuration.
///
/// This struct is deliberately small; expand only when necessary.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub title: String,
    pub initial_size: LogicalSize<f64>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            title: "marduk-studio".to_string(),
            initial_size: LogicalSize::new(1280.0, 720.0),
        }
    }
}

/// Callback return used to control the runtime loop.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum RunControl {
    Continue,
    Exit,
}

/// Runtime loop entry.
///
/// Uses winit `run_app` (modern API). Application state is managed by an
/// `ApplicationHandler` implementation.
///
/// Note: this runtime assumes a single main window for now.
pub struct Runtime;

impl Runtime {
    pub fn run<F>(config: RuntimeConfig, gpu_init: GpuInit, tick: F) -> Result<()>
    where
        F: 'static + FnMut(&Window, &mut Gpu<'static>, f32) -> RunControl,
    {
        let event_loop = EventLoop::new().context("failed to create winit EventLoop")?;

        let mut app = App::new(config, gpu_init, tick);

        event_loop
            .run_app(&mut app)
            .context("winit event loop terminated with error")?;

        Ok(())
    }
}

/// Application state for winit `run_app`.
///
/// This type intentionally stores:
/// - a leaked `&'static Window`
/// - a `Gpu<'static>` borrowing that window
///
/// This avoids self-referential structs while preserving the device layer’s
/// borrowing model (Fix A).
struct App<F>
where
    F: FnMut(&Window, &mut Gpu<'static>, f32) -> RunControl + 'static,
{
    config: RuntimeConfig,
    gpu_init: GpuInit,
    tick: F,

    window: Option<&'static Window>,
    gpu: Option<Gpu<'static>>,

    last_instant: Instant,
    exit_requested: bool,
}

impl<F> App<F>
where
    F: FnMut(&Window, &mut Gpu<'static>, f32) -> RunControl + 'static,
{
    fn new(config: RuntimeConfig, gpu_init: GpuInit, tick: F) -> Self {
        Self {
            config,
            gpu_init,
            tick,
            window: None,
            gpu: None,
            last_instant: Instant::now(),
            exit_requested: false,
        }
    }

    fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    fn clear_pass(gpu: &Gpu<'static>, frame: &mut crate::device::GpuFrame) {
        // Minimal clear used as a validation step.
        // Rendering infrastructure (pipelines, batching) belongs in a later module.
        let _rpass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("marduk clear pass"),
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

        // Explicit drop keeps intent clear; encoder owns the pass.
        drop(_rpass);

        // `gpu` is currently unused in the clear pass; kept for symmetry and future expansion.
        let _ = gpu;
    }
}

impl<F> ApplicationHandler for App<F>
where
    F: FnMut(&Window, &mut Gpu<'static>, f32) -> RunControl + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        // winit 0.30+ uses window attributes instead of WindowBuilder.
        let attrs = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(self.config.initial_size);

        let window_owned = match event_loop.create_window(attrs) {
            Ok(w) => w,
            Err(e) => {
                // winit requires error handling without panicking in handler callbacks.
                // Store an exit request; the loop will terminate on the next cycle.
                eprintln!("failed to create window: {e}");
                self.request_exit();
                return;
            }
        };

        // Leak the window for process lifetime to obtain a stable `'static` reference.
        // This enables storing `Gpu<'static>` without self-referential state.
        let window: &'static Window = Box::leak(Box::new(window_owned));

        let gpu = match pollster::block_on(Gpu::new(window, self.gpu_init.clone())) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("failed to initialize GPU: {e:?}");
                self.request_exit();
                return;
            }
        };

        self.window = Some(window);
        self.gpu = Some(gpu);

        event_loop.set_control_flow(ControlFlow::Wait);
        window.request_redraw();
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_requested {
            event_loop.exit();
            return;
        }

        if let Some(window) = self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if self.exit_requested {
            event_loop.exit();
            return;
        }

        let Some(window) = self.window else { return; };
        let Some(gpu) = self.gpu.as_mut() else { return; };

        match event {
            WindowEvent::CloseRequested => {
                self.request_exit();
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                gpu.resize(new_size);
                window.request_redraw();
            }

            WindowEvent::ScaleFactorChanged { .. } => {
                gpu.resize(window.inner_size());
                window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_instant).as_secs_f32();
                self.last_instant = now;

                match gpu.begin_frame() {
                    Ok(mut frame) => {
                        Self::clear_pass(gpu, &mut frame);
                        gpu.submit(frame);

                        if (self.tick)(window, gpu, dt) == RunControl::Exit {
                            self.request_exit();
                            event_loop.exit();
                        }
                    }

                    Err(err) => match gpu.handle_surface_error(err) {
                        SurfaceErrorAction::Reconfigured => {}
                        SurfaceErrorAction::SkipFrame => {}
                        SurfaceErrorAction::Fatal => {
                            self.request_exit();
                            event_loop.exit();
                        }
                    },
                }
            }

            _ => {}
        }
    }
}