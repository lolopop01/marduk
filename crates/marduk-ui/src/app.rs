use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use winit::dpi::LogicalSize;
use winit::window::Fullscreen;

use marduk_engine::core::{App as EngineApp, AppControl, FrameCtx};
use marduk_engine::device::GpuInit;
use marduk_engine::input::{Key, MouseButton};
use marduk_engine::render::shapes::circle::CircleRenderer;
use marduk_engine::render::shapes::rect::RectRenderer;
use marduk_engine::render::shapes::rounded_rect::RoundedRectRenderer;
use marduk_engine::render::shapes::text::TextRenderer;
use marduk_engine::text::FontId;
use marduk_engine::window::{Runtime, RuntimeConfig, WindowMode};

use marduk_engine::coords::Vec2;

use crate::dsl::{DslBindings, DslDocument, DslLoader};
use crate::dsl::builder::WidgetStateValue;
use crate::scene::{UiInput, UiScene};
use crate::widget::Element;

// ── WidgetState ───────────────────────────────────────────────────────────

/// Handle passed to `on_event_state` callbacks for reading/writing DSL widget state.
///
/// Widget state is keyed by the `state_key` property set on stateful widgets
/// (TextBox, Checkbox, Toggle, Slider, RadioGroup). Mutations take effect on
/// the next frame rebuild.
pub struct WidgetState(Rc<RefCell<HashMap<String, WidgetStateValue>>>);

impl WidgetState {
    /// Remove a widget's state entry (e.g. clear a TextBox).
    pub fn clear(&mut self, key: &str) {
        self.0.borrow_mut().remove(key);
    }

    /// Overwrite a text-valued state entry.
    pub fn set_str(&mut self, key: &str, v: impl Into<String>) {
        self.0.borrow_mut().insert(key.to_string(), WidgetStateValue::Str(v.into()));
    }

    /// Overwrite a boolean state entry (Checkbox, Toggle).
    pub fn set_bool(&mut self, key: &str, v: bool) {
        self.0.borrow_mut().insert(key.to_string(), WidgetStateValue::Bool(v));
    }

    /// Overwrite a float state entry (Slider).
    pub fn set_float(&mut self, key: &str, v: f32) {
        self.0.borrow_mut().insert(key.to_string(), WidgetStateValue::Float(v));
    }
}

// ── FontMap ───────────────────────────────────────────────────────────────

/// A name-keyed map of loaded font handles.
///
/// Passed to the builder closure in [`Application::run_widget`] so the
/// application can retrieve [`FontId`] values by name without ever importing
/// engine internals.
///
/// ```rust,ignore
/// .run_widget(|fonts: &FontMap| {
///     let body = fonts.get("body");
///     MyApp::new(body).into()
/// })
/// ```
pub struct FontMap(pub(crate) HashMap<String, FontId>);

impl FontMap {
    /// Returns the [`FontId`] registered under `name`, or `None` if the name
    /// was not registered or the font failed to load.
    pub fn get(&self, name: &str) -> Option<FontId> {
        self.0.get(name).copied()
    }
}

// ── Application ───────────────────────────────────────────────────────────

/// Top-level UI application builder.
///
/// Follows a GTK-style builder pattern: configure fonts, components, and event
/// handlers, then start the event loop with either [`run`] (declarative DSL)
/// or [`run_widget`] (custom Rust widget tree).
///
/// # DSL-driven app
///
/// ```rust,ignore
/// Application::new()
///     .title("My App")
///     .font("body", include_bytes!("font.ttf").to_vec())
///     .component("Sidebar", include_str!("ui/sidebar.mkml"))
///     .on_event("quit", || std::process::exit(0))
///     .run(include_str!("ui/main.mkml"));
/// ```
///
/// # Custom widget app
///
/// ```rust,ignore
/// Application::new()
///     .title("Marduk Studio")
///     .font("body", load_font())
///     .run_widget(|fonts: &FontMap| MyApp::new(fonts.get("body")).into());
/// ```
pub struct Application {
    title:          String,
    width:          f64,
    height:         f64,
    zoom:           f32,
    window_mode:    WindowMode,
    fonts:          Vec<(String, Vec<u8>)>,
    components:     Vec<(String, String)>,
    event_handlers: HashMap<String, Box<dyn FnMut()>>,
    /// Shared widget state — created early so `on_event_state` closures can capture it.
    widget_state:   Rc<RefCell<HashMap<String, WidgetStateValue>>>,
}

impl Application {
    pub fn new() -> Self {
        Self {
            title:          "marduk".to_string(),
            width:          1280.0,
            height:         720.0,
            zoom:           1.0,
            window_mode:    WindowMode::Windowed,
            fonts:          Vec::new(),
            components:     Vec::new(),
            event_handlers: HashMap::new(),
            widget_state:   Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Set the window title.
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }

    /// Set the initial window size in logical pixels.
    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.width  = width;
        self.height = height;
        self
    }

    /// Set the initial zoom level (default `1.0`).
    ///
    /// The user can adjust zoom at runtime with **Ctrl + Scroll**.  Values
    /// below `0.25` or above `4.0` are clamped at render time.
    pub fn zoom(mut self, z: f32) -> Self {
        self.zoom = z;
        self
    }

    /// Set the initial window presentation mode (default: `Windowed`).
    ///
    /// - `WindowMode::Windowed`   — normal resizable window
    /// - `WindowMode::Maximized`  — fills the screen, OS title bar visible
    /// - `WindowMode::Fullscreen` — borderless fullscreen, no title bar
    pub fn window_mode(mut self, mode: WindowMode) -> Self {
        self.window_mode = mode;
        self
    }

    /// Register a named font. The name is used in `.mkml` `font=name` attrs
    /// and in [`FontMap::get`].
    ///
    /// If multiple fonts are registered, the first one whose bytes load
    /// successfully becomes the default.
    pub fn font(mut self, name: impl Into<String>, data: Vec<u8>) -> Self {
        self.fonts.push((name.into(), data));
        self
    }

    /// Register a `.mkml` source string under `alias` so other `.mkml`
    /// files can reference it with `use "..." as Alias`.
    pub fn component(mut self, alias: impl Into<String>, src: impl Into<String>) -> Self {
        self.components.push((alias.into(), src.into()));
        self
    }

    /// Register a callback for a named DSL event (e.g. `on_click=quit`).
    pub fn on_event(mut self, name: impl Into<String>, f: impl FnMut() + 'static) -> Self {
        self.event_handlers.insert(name.into(), Box::new(f));
        self
    }

    /// Register a callback that can read and write DSL widget state.
    ///
    /// Use this when an event needs to mutate widget state — e.g. clearing a
    /// TextBox when a "CLEAR" button is pressed.
    ///
    /// # Example
    /// ```rust,ignore
    /// .on_event_state("comms_clear", |state| {
    ///     state.clear("comms_message");
    /// })
    /// ```
    pub fn on_event_state<F>(mut self, name: impl Into<String>, mut f: F) -> Self
    where
        F: FnMut(&mut WidgetState) + 'static,
    {
        let shared = self.widget_state.clone();
        self.event_handlers.insert(name.into(), Box::new(move || {
            f(&mut WidgetState(shared.clone()));
        }));
        self
    }

    // ── Entry points ──────────────────────────────────────────────────────

    /// Start the event loop using a `.mkml` document as the root widget tree.
    ///
    /// This never returns.
    pub fn run(self, main_src: &str) -> ! {
        let doc = match DslLoader::new().parse(main_src) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("mkml parse error in main document: {e}");
                std::process::exit(1);
            }
        };
        let state = UiAppState::new_dsl(self, doc);
        Self::launch(state);
    }

    /// Start the event loop with a custom root widget.
    ///
    /// `build` is called once after fonts are loaded; the returned [`Element`]
    /// persists across frames and is mutated in place via `on_event`.
    ///
    /// This never returns.
    pub fn run_widget<F>(self, build: F) -> !
    where
        F: FnOnce(&FontMap) -> Element,
    {
        let state = UiAppState::new_widget(self, build);
        Self::launch(state);
    }

    fn launch(state: UiAppState) -> ! {
        let config = RuntimeConfig {
            title:        state.title.clone(),
            initial_size: LogicalSize::new(state.width, state.height),
            window_mode:  state.window_mode,
        };
        Runtime::run(config, GpuInit::default(), state)
            .unwrap_or_else(|e| {
                eprintln!("marduk runtime error: {e}");
                std::process::exit(1);
            });
        // Runtime::run only returns on fatal error (exit via AppControl::Exit
        // goes through the event loop exit path), but the compiler doesn't know
        // that, so we help it here.
        std::process::exit(0);
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

// ── UiAppState ────────────────────────────────────────────────────────────

/// Internal state that implements `marduk_engine::core::App`.
///
/// Everything engine-specific (renderers, FrameCtx) lives here.
/// User code never sees this type.
struct UiAppState {
    title:       String,
    width:       f64,
    height:      f64,
    window_mode: WindowMode,

    // Zoom — adjusted at runtime via Ctrl+Scroll.
    zoom: f32,

    // Rendering
    ui_scene:              UiScene,
    rect_renderer:         RectRenderer,
    rounded_rect_renderer: RoundedRectRenderer,
    circle_renderer:       CircleRenderer,
    text_renderer:         TextRenderer,

    // DSL mode
    loader:   DslLoader,
    doc:      Option<DslDocument>,
    bindings: DslBindings,

    // Widget mode (state persists across frames)
    root: Option<Element>,

    // Event dispatch
    event_handlers: HashMap<String, Box<dyn FnMut()>>,

    // Drag tracking — position where the current mouse drag started (None when not dragging).
    drag_origin: Option<Vec2>,
}

impl UiAppState {
    fn new_dsl(app: Application, doc: DslDocument) -> Self {
        let (ui_scene, loader, bindings) = Self::setup_dsl(&app);
        Self {
            title:                 app.title,
            width:                 app.width,
            height:                app.height,
            window_mode:           app.window_mode,
            zoom:                  app.zoom,
            ui_scene,
            rect_renderer:         RectRenderer::new(),
            rounded_rect_renderer: RoundedRectRenderer::new(),
            circle_renderer:       CircleRenderer::new(),
            text_renderer:         TextRenderer::new(),
            loader,
            doc:                   Some(doc),
            bindings,
            root:                  None,
            event_handlers:        app.event_handlers,
            drag_origin:           None,
        }
    }

    fn new_widget<F>(app: Application, build: F) -> Self
    where
        F: FnOnce(&FontMap) -> Element,
    {
        let (ui_scene, loader, bindings) = Self::setup_dsl(&app);
        let font_map = FontMap(bindings.fonts.clone());
        let root = build(&font_map);
        Self {
            title:                 app.title,
            width:                 app.width,
            height:                app.height,
            window_mode:           app.window_mode,
            zoom:                  app.zoom,
            ui_scene,
            rect_renderer:         RectRenderer::new(),
            rounded_rect_renderer: RoundedRectRenderer::new(),
            circle_renderer:       CircleRenderer::new(),
            text_renderer:         TextRenderer::new(),
            loader,
            doc:                   None,
            bindings,
            root:                  Some(root),
            event_handlers:        app.event_handlers,
            drag_origin:           None,
        }
    }

    /// Load fonts into a new `UiScene`, set up DSL loader, return both.
    fn setup_dsl(app: &Application) -> (UiScene, DslLoader, DslBindings) {
        let mut ui_scene = UiScene::new();
        // Re-use the widget_state Rc that event handlers may already have captured.
        let mut bindings = DslBindings::with_state(app.widget_state.clone());

        for (name, bytes) in &app.fonts {
            if let Ok(id) = ui_scene.load_font(bytes) {
                bindings.fonts.insert(name.clone(), id);
            } else {
                log::warn!("failed to load font '{name}'");
            }
        }

        let mut loader = DslLoader::new();
        for (alias, src) in &app.components {
            if let Err(e) = loader.parse_and_register(alias.as_str(), src.as_str()) {
                log::warn!("failed to parse component '{alias}': {e}");
            }
        }

        (ui_scene, loader, bindings)
    }
}

impl EngineApp for UiAppState {
    fn on_frame(&mut self, ctx: &mut FrameCtx<'_, '_>) -> AppControl {
        let (w, h) = ctx.window.logical_size();

        // ── F11 → toggle fullscreen ───────────────────────────────────────
        if ctx.input_frame.keys_pressed.contains(&Key::F11) {
            let is_fullscreen = ctx.window.window.fullscreen().is_some();
            if is_fullscreen {
                ctx.window.window.set_fullscreen(None);
            } else {
                ctx.window.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        }

        // ── Ctrl + Scroll → zoom ──────────────────────────────────────────
        let ctrl = ctx.input.modifiers.ctrl;
        let raw_scroll = ctx.input_frame.scroll_delta;
        if ctrl && raw_scroll != 0.0 {
            // Each scroll line zooms by ~10%. Negative delta = scroll down = zoom out.
            // Exponential scale so fast scrolls can't produce negative zoom.
            // positive raw_scroll = scroll down = zoom out → negative exponent.
            self.zoom *= f32::exp(-raw_scroll * 0.1);
            self.zoom = self.zoom.clamp(0.25, 4.0);
        }

        // Mouse position in *zoomed* logical space (divide by zoom).
        let (mx, my) = ctx.input.pointer_pos.unwrap_or((0.0, 0.0));
        let mouse_pos = Vec2::new(mx / self.zoom, my / self.zoom);

        // Track where the current drag started (in zoomed space).
        if ctx.input_frame.buttons_pressed.contains(&MouseButton::Left) {
            self.drag_origin = Some(mouse_pos);
        }
        let drag_end = if ctx.input_frame.buttons_released.contains(&MouseButton::Left) {
            self.drag_origin.take()
        } else {
            None
        };

        // Layout viewport = window size / zoom (widgets lay out in this space).
        let ui_viewport = Vec2::new(w / self.zoom, h / self.zoom);

        let ui_input = UiInput {
            mouse_pos,
            mouse_pressed: ctx.input.button_down(MouseButton::Left),
            mouse_clicked: ctx.input_frame.buttons_released.contains(&MouseButton::Left),
            text_input:    ctx.input_frame.text.iter().map(|t| t.text.clone()).collect(),
            keys_pressed:  ctx.input_frame.keys_pressed.iter().copied().collect(),
            // Swallow scroll delta when Ctrl is held (it was consumed for zoom).
            scroll_delta:  if ctrl { 0.0 } else { raw_scroll },
            drag_origin:   self.drag_origin,
            drag_end,
        };

        // ── Sync pixel_ratio for accurate text measurement ────────────────
        // Same quantisation the TextRenderer uses for raster_scale.
        let os_scale = ctx.window.window.scale_factor() as f32;
        let raster_scale = (os_scale * self.zoom * 4.0).round() / 4.0;
        self.ui_scene.pixel_ratio = raster_scale;

        // ── Layout + paint ────────────────────────────────────────────────
        match (&self.doc, &mut self.root) {
            (Some(doc), _) => {
                let root = self.loader.build(doc, &self.bindings);
                let _ = self.ui_scene.frame(root, ui_viewport, &ui_input);
            }
            (None, Some(root)) => {
                let _ = self.ui_scene.frame_ref(root, ui_viewport, &ui_input);
            }
            _ => {}
        }

        // ── Dispatch events ───────────────────────────────────────────────
        for event in self.bindings.take_events() {
            if let Some(handler) = self.event_handlers.get_mut(&event) {
                handler();
            }
        }

        // ── Render ────────────────────────────────────────────────────────
        let dl    = &mut self.ui_scene.draw_list;
        let fs    = &self.ui_scene.font_system;
        let r_r   = &mut self.rect_renderer;
        let r_rr  = &mut self.rounded_rect_renderer;
        let r_c   = &mut self.circle_renderer;
        let r_t   = &mut self.text_renderer;
        let zoom  = self.zoom;

        ctx.render_scaled(zoom, marduk_engine::paint::Color::from_straight(0.054, 0.051, 0.043, 1.0), |rctx, target| {
            r_r.render(rctx, target, dl);
            r_rr.render(rctx, target, dl);
            r_c.render(rctx, target, dl);
            r_t.render(rctx, target, dl, fs);
        })
    }
}

