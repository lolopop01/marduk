use anyhow::{Context, Result};
use ouroboros::self_referencing;
use std::collections::HashMap;

use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::core::{App as CoreApp, AppControl, FrameCtx, WindowCtx};
use crate::device::{Gpu, GpuInit};
use crate::input::{
    InputEvent, InputFrame, InputState, Key, KeyState, Modifiers, MouseButton, MouseButtonState,
    MouseWheelDelta, PointerButtonEvent, PointerMoveEvent, TextEvent,
};
use crate::time::{FrameClock, FrameTime};

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

        let entry = WindowEntryBuilder {
            input_state: InputState::default(),
            input_frame: InputFrame::default(),
            clock: FrameClock::default(),
            window,
            gpu_builder: |w| {
                pollster::block_on(Gpu::new(w, gpu_init))
                    .expect("GPU initialization failed for window")
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

        // Split borrows to avoid `self` capture inside `ouroboros` closures.
        let (app, windows) = (&mut self.app, &mut self.windows);

        let Some(entry) = windows.get_mut(&window_id) else {
            return;
        };

        // Track exit request from callbacks without mutating `self` in the closure.
        let mut exit_from_app_event = false;

        entry.with_mut(|fields| {
            if let Some(ev) = translate_input_event(fields.window, fields.input_state, &event) {
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

        // Runtime-managed window lifecycle / resize / redraw handling.
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

                // Drive one frame for this window.
                if let Some(entry) = self.windows.get_mut(&window_id) {
                    entry.with_mut(|fields| {
                        let ft: FrameTime = fields.clock.tick();

                        // Scope to ensure `ctx` is dropped before mutating frame state.
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

                        // Clear per-frame deltas after the frame is consumed.
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

fn translate_input_event(
    window: &Window,
    state: &InputState,
    event: &WindowEvent,
) -> Option<InputEvent> {
    match event {
        // winit 0.30: ModifiersChanged carries a struct with a state() accessor on some platforms.
        // Treat it as `ModifiersState` source via deref-like access patterns.
        WindowEvent::ModifiersChanged(m) => {
            // `*m` in 0.30 is not `ModifiersState`; use `m.state()` if available via pattern:
            // The type is compatible with `ModifiersState` through `.state()`.
            let ms: ModifiersState = m.state();
            Some(InputEvent::ModifiersChanged(map_modifiers(ms)))
        }

        WindowEvent::Focused(f) => Some(InputEvent::Focused(*f)),

        WindowEvent::CursorLeft { .. } => Some(InputEvent::PointerLeft),

        WindowEvent::CursorMoved { position, .. } => {
            let (x, y) = to_logical_f32(window, *position);
            Some(InputEvent::PointerMoved(PointerMoveEvent { x, y }))
        }

        WindowEvent::MouseInput { state: st, button, .. } => {
            let st = match st {
                ElementState::Pressed => MouseButtonState::Pressed,
                ElementState::Released => MouseButtonState::Released,
            };

            let button = map_mouse_button(*button);
            let modifiers = state.modifiers;

            let (x, y) = state.pointer_pos.unwrap_or((0.0, 0.0));

            Some(InputEvent::PointerButton(PointerButtonEvent {
                button,
                state: st,
                x,
                y,
                modifiers,
            }))
        }

        WindowEvent::MouseWheel { delta, .. } => {
            let modifiers = state.modifiers;
            let delta = match delta {
                MouseScrollDelta::LineDelta(x, y) => MouseWheelDelta::Line { x: *x, y: *y },
                MouseScrollDelta::PixelDelta(p) => {
                    let (x, y) = to_logical_f32(window, *p);
                    MouseWheelDelta::Pixel { x, y }
                }
            };
            Some(InputEvent::MouseWheel { delta, modifiers })
        }

        WindowEvent::KeyboardInput { event, .. } => {
            let modifiers = state.modifiers;
            let st = match event.state {
                ElementState::Pressed => KeyState::Pressed,
                ElementState::Released => KeyState::Released,
            };

            let (key, code) = map_key(event.physical_key);

            Some(InputEvent::Key {
                key,
                state: st,
                modifiers,
                code,
                repeat: event.repeat,
            })
        }

        WindowEvent::Ime(ime) => match ime {
            winit::event::Ime::Commit(text) if !text.is_empty() => Some(InputEvent::Text(TextEvent {
                text: text.clone(),
            })),
            _ => None,
        },

        _ => None,
    }
}

fn to_logical_f32(window: &Window, pos: PhysicalPosition<f64>) -> (f32, f32) {
    let scale = window.scale_factor();
    let logical = pos.to_logical::<f64>(scale);
    (logical.x as f32, logical.y as f32)
}

fn map_modifiers(m: ModifiersState) -> Modifiers {
    Modifiers {
        shift: m.shift_key(),
        ctrl: m.control_key(),
        alt: m.alt_key(),
        meta: m.super_key(),
    }
}

fn map_mouse_button(b: WinitMouseButton) -> MouseButton {
    match b {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Back => MouseButton::Back,
        WinitMouseButton::Forward => MouseButton::Forward,
        WinitMouseButton::Other(v) => MouseButton::Other(v),
    }
}

fn map_key(pk: PhysicalKey) -> (Key, u32) {
    match pk {
        PhysicalKey::Code(code) => {
            let key = match code {
                KeyCode::Escape => Key::Escape,
                KeyCode::Enter => Key::Enter,
                KeyCode::Tab => Key::Tab,
                KeyCode::Backspace => Key::Backspace,
                KeyCode::Space => Key::Space,

                KeyCode::Insert => Key::Insert,
                KeyCode::Delete => Key::Delete,
                KeyCode::Home => Key::Home,
                KeyCode::End => Key::End,
                KeyCode::PageUp => Key::PageUp,
                KeyCode::PageDown => Key::PageDown,

                KeyCode::ArrowUp => Key::ArrowUp,
                KeyCode::ArrowDown => Key::ArrowDown,
                KeyCode::ArrowLeft => Key::ArrowLeft,
                KeyCode::ArrowRight => Key::ArrowRight,

                KeyCode::ShiftLeft | KeyCode::ShiftRight => Key::Shift,
                KeyCode::ControlLeft | KeyCode::ControlRight => Key::Control,
                KeyCode::AltLeft | KeyCode::AltRight => Key::Alt,
                KeyCode::SuperLeft | KeyCode::SuperRight => Key::Meta,

                KeyCode::KeyA => Key::A,
                KeyCode::KeyB => Key::B,
                KeyCode::KeyC => Key::C,
                KeyCode::KeyD => Key::D,
                KeyCode::KeyE => Key::E,
                KeyCode::KeyF => Key::F,
                KeyCode::KeyG => Key::G,
                KeyCode::KeyH => Key::H,
                KeyCode::KeyI => Key::I,
                KeyCode::KeyJ => Key::J,
                KeyCode::KeyK => Key::K,
                KeyCode::KeyL => Key::L,
                KeyCode::KeyM => Key::M,
                KeyCode::KeyN => Key::N,
                KeyCode::KeyO => Key::O,
                KeyCode::KeyP => Key::P,
                KeyCode::KeyQ => Key::Q,
                KeyCode::KeyR => Key::R,
                KeyCode::KeyS => Key::S,
                KeyCode::KeyT => Key::T,
                KeyCode::KeyU => Key::U,
                KeyCode::KeyV => Key::V,
                KeyCode::KeyW => Key::W,
                KeyCode::KeyX => Key::X,
                KeyCode::KeyY => Key::Y,
                KeyCode::KeyZ => Key::Z,

                KeyCode::Digit0 => Key::Digit0,
                KeyCode::Digit1 => Key::Digit1,
                KeyCode::Digit2 => Key::Digit2,
                KeyCode::Digit3 => Key::Digit3,
                KeyCode::Digit4 => Key::Digit4,
                KeyCode::Digit5 => Key::Digit5,
                KeyCode::Digit6 => Key::Digit6,
                KeyCode::Digit7 => Key::Digit7,
                KeyCode::Digit8 => Key::Digit8,
                KeyCode::Digit9 => Key::Digit9,

                KeyCode::F1 => Key::F1,
                KeyCode::F2 => Key::F2,
                KeyCode::F3 => Key::F3,
                KeyCode::F4 => Key::F4,
                KeyCode::F5 => Key::F5,
                KeyCode::F6 => Key::F6,
                KeyCode::F7 => Key::F7,
                KeyCode::F8 => Key::F8,
                KeyCode::F9 => Key::F9,
                KeyCode::F10 => Key::F10,
                KeyCode::F11 => Key::F11,
                KeyCode::F12 => Key::F12,

                other => Key::Unknown(other as u32),
            };

            (key, code as u32)
        }

        // NativeKeyCode is not a u32 in winit 0.30; preserve "unknown" without a stable numeric.
        PhysicalKey::Unidentified(_) => (Key::Unknown(0), 0),
    }
}