use anyhow::{Context, Result};
use ouroboros::self_referencing;
use std::collections::HashMap;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::device::{Gpu, GpuFrame, GpuInit, SurfaceErrorAction};

/// Window/runtime configuration.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub title: String,
    pub initial_size: LogicalSize<f64>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            title: "marduk".to_string(),
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

/// Runtime context passed to the tick callback.
///
/// This object records commands which are applied after the callback returns.
#[derive(Default)]
pub struct RuntimeCtx {
    commands: Vec<Command>,
}

impl RuntimeCtx {
    pub fn create_window(&mut self, config: RuntimeConfig) {
        self.commands.push(Command::CreateWindow(config));
    }

    pub fn close_window(&mut self, id: WindowId) {
        self.commands.push(Command::CloseWindow(id));
    }

    pub fn exit(&mut self) {
        self.commands.push(Command::Exit);
    }
}

enum Command {
    CreateWindow(RuntimeConfig),
    CloseWindow(WindowId),
    Exit,
}

/// Entry point for the runtime.
pub struct Runtime;

impl Runtime {
    /// Runs the application with multi-window support.
    ///
    /// The callback is invoked once per redraw for the window being redrawn.
    /// The callback may enqueue commands in `RuntimeCtx` to create/close windows or exit.
    pub fn run<F>(initial: RuntimeConfig, gpu_init: GpuInit, tick: F) -> Result<()>
    where
        F: 'static + FnMut(&mut RuntimeCtx, WindowId, &Window, &mut Gpu, f32) -> RunControl,
    {
        let event_loop = EventLoop::new().context("failed to create winit EventLoop")?;
        let mut app = App::new(initial, gpu_init, tick);

        event_loop
            .run_app(&mut app)
            .context("winit event loop terminated with error")?;

        Ok(())
    }
}

#[self_referencing]
struct WindowEntry {
    window: Window,

    #[borrows(window)]
    #[covariant]
    gpu: Gpu<'this>,
}

struct App<F>
where
    F: FnMut(&mut RuntimeCtx, WindowId, &Window, &mut Gpu, f32) -> RunControl + 'static,
{
    initial: RuntimeConfig,
    gpu_init: GpuInit,
    tick: F,

    windows: HashMap<WindowId, WindowEntry>,
    last_instant: Instant,
    exit_requested: bool,
}

impl<F> App<F>
where
    F: FnMut(&mut RuntimeCtx, WindowId, &Window, &mut Gpu, f32) -> RunControl + 'static,
{
    fn new(initial: RuntimeConfig, gpu_init: GpuInit, tick: F) -> Self {
        Self {
            initial,
            gpu_init,
            tick,
            windows: HashMap::new(),
            last_instant: Instant::now(),
            exit_requested: false,
        }
    }

    fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    fn create_window_entry(
        &mut self,
        event_loop: &ActiveEventLoop,
        config: RuntimeConfig,
    ) -> Result<WindowId> {
        let attrs = Window::default_attributes()
            .with_title(config.title)
            .with_inner_size(config.initial_size);

        let window = event_loop
            .create_window(attrs)
            .context("failed to create window")?;

        let id = window.id();
        let gpu_init = self.gpu_init.clone();

        let entry = WindowEntryBuilder {
            window,
            gpu_builder: |w| {
                pollster::block_on(Gpu::new(w, gpu_init))
                    .expect("failed to initialize GPU for window")
            },
        }
            .build();

        self.windows.insert(id, entry);

        Ok(id)
    }

    fn destroy_window_entry(&mut self, id: WindowId) {
        self.windows.remove(&id);
    }

    fn apply_commands(&mut self, event_loop: &ActiveEventLoop, mut ctx: RuntimeCtx) {
        for cmd in ctx.commands.drain(..) {
            match cmd {
                Command::CreateWindow(cfg) => {
                    if self.create_window_entry(event_loop, cfg).is_err() {
                        self.request_exit();
                    }
                }
                Command::CloseWindow(id) => {
                    self.destroy_window_entry(id);
                }
                Command::Exit => {
                    self.request_exit();
                }
            }
        }

        if self.windows.is_empty() {
            self.request_exit();
        }

        if self.exit_requested {
            event_loop.exit();
        }
    }

    fn clear_pass(frame: &mut GpuFrame) {
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
        drop(_rpass);
    }
}

impl<F> ApplicationHandler for App<F>
where
    F: FnMut(&mut RuntimeCtx, WindowId, &Window, &mut Gpu, f32) -> RunControl + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }

        match self.create_window_entry(event_loop, self.initial.clone()) {
            Ok(id) => {
                if let Some(entry) = self.windows.get(&id) {
                    entry.with_window(|w| w.request_redraw());
                }
            }
            Err(e) => {
                eprintln!("failed to create initial window: {e:?}");
                self.request_exit();
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_requested {
            event_loop.exit();
            return;
        }

        event_loop.set_control_flow(ControlFlow::Wait);

        // For now, redraw continuously. Later, switch to invalidation-based redraw.
        for entry in self.windows.values() {
            entry.with_window(|w| w.request_redraw());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.exit_requested {
            event_loop.exit();
            return;
        }

        let Some(entry) = self.windows.get_mut(&window_id) else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                self.destroy_window_entry(window_id);
                if self.windows.is_empty() {
                    self.request_exit();
                    event_loop.exit();
                }
            }

            WindowEvent::Resized(new_size) => {
                entry.with_gpu_mut(|gpu| gpu.resize(new_size));
                entry.with_window(|w| w.request_redraw());
            }

            WindowEvent::ScaleFactorChanged { .. } => {
                let new_size = entry.with_window(|w| w.inner_size());
                entry.with_gpu_mut(|gpu| gpu.resize(new_size));
                entry.with_window(|w| w.request_redraw());
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_instant).as_secs_f32();
                self.last_instant = now;

                let mut ctx = RuntimeCtx::default();
                let mut exit_from_tick = false;

                entry.with_mut(|fields| {
                    let window = fields.window;
                    let gpu = fields.gpu;

                    match gpu.begin_frame() {
                        Ok(mut frame) => {
                            Self::clear_pass(&mut frame);
                            gpu.submit(frame);

                            let rc = (self.tick)(&mut ctx, window_id, window, gpu, dt);
                            if rc == RunControl::Exit {
                                exit_from_tick = true;
                            }
                        }
                        Err(err) => match gpu.handle_surface_error(err) {
                            SurfaceErrorAction::Reconfigured => {}
                            SurfaceErrorAction::SkipFrame => {}
                            SurfaceErrorAction::Fatal => {
                                exit_from_tick = true;
                            }
                        },
                    }
                });

                if exit_from_tick {
                    ctx.exit();
                }

                self.apply_commands(event_loop, ctx);
            }

            _ => {}
        }
    }
}