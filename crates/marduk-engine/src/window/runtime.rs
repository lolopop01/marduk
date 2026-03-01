use anyhow::{Context, Result};
use ouroboros::self_referencing;
use std::collections::HashMap;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window, WindowId};

use crate::core::{App as CoreApp, AppControl, FrameCtx, WindowCtx};
use crate::device::{Gpu, GpuInit};
use crate::input::{InputEvent, InputFrame, InputState};
use crate::input::platform::winit as input_winit;
use crate::time::FrameClock;

/// How the initial window should be presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowMode {
    /// Normal resizable window. `initial_size` sets the starting dimensions.
    #[default]
    Windowed,
    /// OS-maximized window — fills the screen but keeps the title bar.
    Maximized,
    /// Borderless fullscreen — covers the entire screen, no title bar.
    Fullscreen,
}

/// Window/runtime configuration.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub title: String,
    pub initial_size: LogicalSize<f64>,
    pub window_mode: WindowMode,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            title: "marduk".to_string(),
            initial_size: LogicalSize::new(1280.0, 720.0),
            window_mode: WindowMode::Windowed,
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
        let mut attrs = Window::default_attributes()
            .with_visible(true)
            .with_title(config.title.clone())
            .with_inner_size(config.initial_size);

        match config.window_mode {
            WindowMode::Windowed   => {}
            WindowMode::Maximized  => attrs = attrs.with_maximized(true),
            WindowMode::Fullscreen => attrs = attrs.with_fullscreen(
                Some(Fullscreen::Borderless(None))
            ),
        }


        let window = event_loop
            .create_window(attrs)
            .context("failed to create window")?;

        window.set_visible(true);
        window.set_ime_allowed(true);
        window.request_redraw();
        // Do not focus or request attention here; defer to resumed

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
            entry.with_window(|w| {
                w.request_redraw();
            });
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_requested {
            event_loop.exit();
            return;
        }

        // Sleep until the next OS event (mouse move, key press, resize, …).
        // Redraws are requested explicitly: once on window creation, and again
        // on Resized / ScaleFactorChanged. This keeps CPU/GPU usage near zero
        // while the window is idle.
        event_loop.set_control_flow(ControlFlow::Wait);
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
        // True when input events that could cause a visual change were received.
        // ModifiersChanged and Focused update internal state but don't affect
        // the widget tree's appearance, so they are excluded.
        let mut had_visual_input = false;

        entry.with_mut(|fields| {
            let events = input_winit::translate_window_event(fields.window, fields.input_state, &event);
            for ev in &events {
                had_visual_input |= matches!(ev,
                    InputEvent::PointerMoved(_)
                    | InputEvent::PointerLeft
                    | InputEvent::PointerButton(_)
                    | InputEvent::MouseWheel { .. }
                    | InputEvent::Key { .. }
                    | InputEvent::Text(_)
                );
            }
            for ev in events {
                fields.input_state.apply_event(fields.input_frame, ev);
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
                // Do NOT call destroy_window_entry here. AppState's field drop
                // order (app before windows) ensures wgpu resources are freed
                // before the Device. Eagerly dropping the Gpu here would destroy
                // VkDevice while the app's wgpu Buffers/Textures are still alive,
                // causing a use-after-free (SIGSEGV) when they later drop.
                self.request_exit();
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                if let Some(entry) = self.windows.get_mut(&window_id) {
                    entry.with_gpu_mut(|gpu| gpu.resize(*new_size));
                    entry.with_window(|w| {
                        w.request_redraw();
                    });
                }
            }

            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(entry) = self.windows.get_mut(&window_id) {
                    let new_size = entry.with_window(|w| w.inner_size());
                    entry.with_gpu_mut(|gpu| gpu.resize(new_size));
                    entry.with_window(|w| {
                        w.request_redraw();
                    });
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

                        // Keep the compositor frame-callback chain alive while the user
                        // is actively interacting (input arrived this frame, or a button/
                        // key is still held). This avoids Wayland latency: without it, the
                        // compositor stops sending frame callbacks when the window is idle,
                        // and the next `request_redraw()` must wait up to one vsync period
                        // before `RedrawRequested` is delivered. PresentMode::Fifo caps the
                        // actual render rate at the monitor refresh rate.
                        let had_activity = !fields.input_frame.events.is_empty()
                            || !fields.input_state.buttons_down.is_empty()
                            || !fields.input_state.keys_down.is_empty();

                        fields.input_frame.clear();

                        if had_activity {
                            fields.window.request_redraw();
                        }
                    });
                }

                if app_control == AppControl::Exit {
                    runtime_ctx.exit();
                }

                self.apply_commands(event_loop, runtime_ctx);
            }

            _ => {
                // Any event that visually changes the UI (cursor move, click,
                // scroll, key, text) needs a repaint. Modifier/focus changes
                // are filtered out by the `had_visual_input` flag above.
                if had_visual_input {
                    if let Some(entry) = self.windows.get(&window_id) {
                        entry.with_window(|w| w.request_redraw());
                    }
                }
            }
        }

        if self.exit_requested {
            event_loop.exit();
        }
    }
}