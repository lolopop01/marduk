use anyhow::{Context, Result};
use ouroboros::self_referencing;
use std::collections::HashMap;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::core::{App as CoreApp, AppControl, FrameCtx, WindowCtx};
use crate::device::{Gpu, GpuInit};
use crate::input::{InputFrame, InputState};
use crate::input::platform::winit as input_winit;
use crate::time::FrameClock;

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

/// Runtime context passed to the application.
///
/// Commands are buffered and applied after the current callback returns.
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
    pub fn run<A>(initial: RuntimeConfig, gpu_init: GpuInit, app: A) -> Result<()>
    where
        A: 'static + CoreApp,
    {
        let event_loop = EventLoop::new().context("failed to create winit EventLoop")?;
        let mut state = AppState::new(initial, gpu_init, app);

        event_loop
            .run_app(&mut state)
            .context("winit event loop terminated with error")?;

        Ok(())
    }
}

#[self_referencing]
struct WindowEntry {
    input_state: InputState,
    input_frame: InputFrame,
    clock: FrameClock,

    window: Window,

    #[borrows(window)]
    #[covariant]
    gpu: Gpu<'this>,
}

struct AppState<A>
where
    A: CoreApp + 'static,
{
    initial: RuntimeConfig,
    gpu_init: GpuInit,
    app: A,

    windows: HashMap<WindowId, WindowEntry>,
    exit_requested: bool,
}

impl<A> AppState<A>
where
    A: CoreApp + 'static,
{
    fn new(initial: RuntimeConfig, gpu_init: GpuInit, app: A) -> Self {
        Self {
            initial,
            gpu_init,
            app,
            windows: HashMap::new(),
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

        // Note: `ouroboros` builder closures are not fallible. GPU init failure is treated
        // as fatal during bootstrap. If non-panicking behavior is required, migrate the
        // device layer to an owning-surface model (Fix B) to remove self-referential storage.
        let entry = WindowEntryBuilder {
            input_state: InputState::default(),
            input_frame: InputFrame::default(),
            clock: FrameClock::default(),
            window,
            gpu_builder: |w| {
                match pollster::block_on(Gpu::new(w, gpu_init)) {
                    Ok(g) => g,
                    Err(e) => {
                        log::error!("GPU initialization failed for window: {e:#}");
                        panic!("GPU initialization failed");
                    }
                }
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
                    if let Err(e) = self.create_window_entry(event_loop, cfg) {
                        log::error!("failed to create window: {e:#}");
                        self.request_exit();
                    }
                }
                Command::CloseWindow(id) => self.destroy_window_entry(id),
                Command::Exit => self.request_exit(),
            }
        }

        if self.windows.is_empty() {
            self.request_exit();
        }

        if self.exit_requested {
            event_loop.exit();
        }
    }
}

impl<A> ApplicationHandler for AppState<A>
where
    A: CoreApp + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if !self.windows.is_empty() {
            return;
        }

        if let Err(e) = self.create_window_entry(event_loop, self.initial.clone()) {
            log::error!("failed to create initial window: {e:#}");
            self.request_exit();
            event_loop.exit();
            return;
        }

        for entry in self.windows.values() {
            entry.with_window(|w| w.request_redraw());
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_requested {
            event_loop.exit();
            return;
        }

        event_loop.set_control_flow(ControlFlow::Wait);

        // Bootstrap behavior: continuous redraw.
        // Later: invalidation-based redraw from UI.
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

        // Split borrows to avoid capturing `self` inside `ouroboros` closures.
        let (app, windows) = (&mut self.app, &mut self.windows);

        let Some(entry) = windows.get_mut(&window_id) else {
            return;
        };

        // Exit intent is latched locally, then applied outside the closure.
        let mut exit_from_app_event = false;

        entry.with_mut(|fields| {
            if let Some(ev) =
                input_winit::translate_window_event(fields.window, fields.input_state, &event)
            {
                fields
                    .input_state
                    .apply_event(fields.input_frame, ev);
            }

            if app.on_window_event(window_id, &event) == AppControl::Exit {
                exit_from_app_event = true;
            }
        });

        if exit_from_app_event {
            self.request_exit();
            event_loop.exit();
            return;
        }

        match &event {
            WindowEvent::CloseRequested => {
                self.destroy_window_entry(window_id);
                if self.windows.is_empty() {
                    self.request_exit();
                    event_loop.exit();
                }
            }

            WindowEvent::Resized(new_size) => {
                if let Some(entry) = self.windows.get_mut(&window_id) {
                    entry.with_gpu_mut(|gpu| gpu.resize(*new_size));
                    entry.with_window(|w| w.request_redraw());
                }
            }

            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(entry) = self.windows.get_mut(&window_id) {
                    let new_size = entry.with_window(|w| w.inner_size());
                    entry.with_gpu_mut(|gpu| gpu.resize(new_size));
                    entry.with_window(|w| w.request_redraw());
                }
            }

            WindowEvent::RedrawRequested => {
                let mut runtime_ctx = RuntimeCtx::default();
                let mut app_control = AppControl::Continue;

                if let Some(entry) = self.windows.get_mut(&window_id) {
                    entry.with_mut(|fields| {
                        let ft = fields.clock.tick();

                        // Ensure `FrameCtx` is dropped before mutating frame-local state.
                        {
                            let mut ctx = FrameCtx {
                                window: WindowCtx {
                                    id: window_id,
                                    window: fields.window,
                                },
                                gpu: fields.gpu,
                                input: fields.input_state,
                                input_frame: fields.input_frame,
                                time: ft,
                                runtime: &mut runtime_ctx,
                            };

                            app_control = self.app.on_frame(&mut ctx);
                        }

                        fields.input_frame.clear();
                    });
                }

                if app_control == AppControl::Exit {
                    runtime_ctx.exit();
                }

                self.apply_commands(event_loop, runtime_ctx);
            }

            _ => {}
        }

        if self.exit_requested {
            event_loop.exit();
        }
    }
}