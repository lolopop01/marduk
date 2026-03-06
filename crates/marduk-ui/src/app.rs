use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use winit::dpi::LogicalSize;
use winit::window::{Fullscreen, WindowId};

use marduk_engine::core::{App as EngineApp, AppControl, FrameCtx};
use marduk_engine::device::GpuInit;
use marduk_engine::image::ImageId;
use marduk_engine::input::{Key, MouseButton};
use marduk_engine::render::shapes::circle::CircleRenderer;
use marduk_engine::render::shapes::image::ImageRenderer;
use marduk_engine::render::shapes::rect::RectRenderer;
use marduk_engine::render::shapes::rounded_rect::RoundedRectRenderer;
use marduk_engine::render::shapes::text::TextRenderer;
use marduk_engine::text::FontId;
use marduk_engine::window::{Runtime, RuntimeConfig, WindowMode};

use marduk_engine::coords::Vec2;

use crate::dsl::{DslBindings, DslDocument, DslLoader};
use crate::dsl::builder::WidgetStateValue;
use crate::image_loader::{decode_image, decode_svg, is_svg};
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

/// A name-keyed map of loaded font and image handles.
///
/// Passed to the builder closure in [`Application::run_widget`] so the
/// application can retrieve [`FontId`] / [`ImageId`] values by name without
/// ever importing engine internals.
///
/// ```rust,ignore
/// .run_widget(|fonts: &FontMap| {
///     let body  = fonts.get("body");
///     let logo  = fonts.image("logo").unwrap();
///     MyApp::new(body, logo).into()
/// })
/// ```
pub struct FontMap {
    pub(crate) fonts: HashMap<String, FontId>,
    pub(crate) images: HashMap<String, ImageId>,
}

impl FontMap {
    /// Returns the [`FontId`] registered under `name`, or `None` if the name
    /// was not registered or the font failed to load.
    pub fn get(&self, name: &str) -> Option<FontId> {
        self.fonts.get(name).copied()
    }

    /// Returns the [`ImageId`] registered under `name`, or `None` if the name
    /// was not registered or the image failed to decode.
    pub fn image(&self, name: &str) -> Option<ImageId> {
        self.images.get(name).copied()
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
    /// Images: `(name, bytes, svg_scale)`. `svg_scale` is 1.0 for raster formats.
    images:         Vec<(String, Vec<u8>, f32)>,
    components:     Vec<(String, String)>,
    event_handlers: HashMap<String, Box<dyn FnMut()>>,
    /// Shared widget state — created early so `on_event_state` closures can capture it.
    widget_state:   Rc<RefCell<HashMap<String, WidgetStateValue>>>,
    /// Named Rust widget factories for `Native { id: "name" }` DSL nodes.
    native_slots:   Vec<(String, Rc<dyn Fn(&FontMap) -> Element>)>,
    /// Secondary OS windows to open alongside the primary window.
    pending_windows: Vec<(RuntimeConfig, Box<dyn Fn(&FontMap) -> Box<dyn Fn() -> Element>>)>,
}

impl Application {
    pub fn new() -> Self {
        Self {
            title:           "marduk".to_string(),
            width:           1280.0,
            height:          720.0,
            zoom:            1.0,
            window_mode:     WindowMode::Windowed,
            fonts:           Vec::new(),
            images:          Vec::new(),
            components:      Vec::new(),
            event_handlers:  HashMap::new(),
            widget_state:    Rc::new(RefCell::new(HashMap::new())),
            native_slots:    Vec::new(),
            pending_windows: Vec::new(),
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

    /// Register an image (PNG, JPEG, BMP, GIF, WebP, ICO, TIFF, or SVG) under `name`.
    ///
    /// The image is decoded once at startup. Raster formats are premultiplied;
    /// SVG is rasterized at its natural size. Use [`svg`] for explicit scale control.
    ///
    /// In `.mkml` files reference the image with `src: "name"`.
    /// In [`run_widget`] closures retrieve the [`ImageId`] via [`FontMap::image`].
    pub fn image(mut self, name: impl Into<String>, bytes: Vec<u8>) -> Self {
        self.images.push((name.into(), bytes, 1.0));
        self
    }

    /// Register an SVG image under `name`, rasterized at `scale × natural size`.
    ///
    /// Use `scale = 2.0` for HiDPI icons.
    pub fn svg(mut self, name: impl Into<String>, bytes: Vec<u8>, scale: f32) -> Self {
        self.images.push((name.into(), bytes, scale));
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

    /// Register a native Rust widget factory under `id`.
    ///
    /// The closure receives the [`FontMap`] on every frame so it can look up
    /// font IDs by name.  In `.mkml` embed it with `Native { id: "name" }`.
    ///
    /// ```rust,ignore
    /// let pane: Rc<RefCell<Option<FilePane>>> = Rc::new(RefCell::new(None));
    /// let pane2 = Rc::clone(&pane);
    /// .native_slot("files", move |fonts| {
    ///     let mut opt = pane2.borrow_mut();
    ///     if opt.is_none() {
    ///         let font = fonts.get("body").unwrap();
    ///         *opt = Some(FilePane::new(dir.clone(), font, ...));
    ///     }
    ///     // build element from pane state ...
    /// })
    /// ```
    pub fn native_slot(
        mut self,
        id: impl Into<String>,
        f: impl Fn(&FontMap) -> Element + 'static,
    ) -> Self {
        self.native_slots.push((id.into(), Rc::new(f) as Rc<dyn Fn(&FontMap) -> Element>));
        self
    }

    /// Open a secondary OS window alongside the primary window.
    ///
    /// `init` is called once per window, receiving the window's [`FontMap`].
    /// It should return a `Box<dyn Fn() -> Element>` — a cheap closure that
    /// rebuilds the element tree each frame.  All persistent state (e.g. a
    /// `FilePane`) should be captured in `init`'s closure and shared with the
    /// inner frame-builder via `Rc<RefCell<>>`.
    ///
    /// ```rust,ignore
    /// .open_window("Explorer", 800.0, 600.0, |fonts| {
    ///     let font = fonts.get("body").unwrap();
    ///     let pane = Rc::new(RefCell::new(FilePane::new(dir, font, ...)));
    ///     Box::new(move || FilePane::as_element(Rc::clone(&pane)))
    /// })
    /// ```
    pub fn open_window(
        mut self,
        title: impl Into<String>,
        width: f64,
        height: f64,
        init: impl Fn(&FontMap) -> Box<dyn Fn() -> Element> + 'static,
    ) -> Self {
        let config = RuntimeConfig {
            title: title.into(),
            initial_size: LogicalSize::new(width, height),
            window_mode: WindowMode::Windowed,
        };
        self.pending_windows.push((config, Box::new(init)));
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
/// Per-window rendering state for secondary OS windows.
struct SecondaryWindowState {
    scene:                 UiScene,
    rect_renderer:         RectRenderer,
    rounded_rect_renderer: RoundedRectRenderer,
    circle_renderer:       CircleRenderer,
    text_renderer:         TextRenderer,
    image_renderer:        ImageRenderer,
    /// Called every frame to produce the root element.
    root_fn:               Box<dyn Fn() -> Element>,
    drag_origin:           Option<Vec2>,
    svg_sources:           Vec<(ImageId, Vec<u8>)>,
    last_raster_scale:     f32,
    start_time:            std::time::Instant,
}

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
    image_renderer:        ImageRenderer,

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

    // SVG re-rasterization: raw bytes stored so we can re-decode at new scale.
    /// `(id, raw_svg_bytes)` for every SVG registered via `.image()` / `.svg()`.
    svg_sources: Vec<(ImageId, Vec<u8>)>,
    /// The `raster_scale` at which SVGs were last rasterized.
    last_raster_scale: f32,

    /// App start time for monotonic `time_ms` in [`UiInput`].
    start_time: std::time::Instant,

    // ── Multi-window ──────────────────────────────────────────────────────
    /// The WindowId of the first window created (the primary window).
    primary_window_id: Option<WindowId>,
    /// State for secondary OS windows, keyed by their WindowId.
    secondary_windows: HashMap<WindowId, SecondaryWindowState>,
    /// Builders waiting to be assigned to new secondary windows (FIFO).
    pending_builders: VecDeque<Box<dyn Fn(&FontMap) -> Box<dyn Fn() -> Element>>>,
    /// Window configs waiting to be submitted to the runtime (FIFO).
    pending_configs: VecDeque<RuntimeConfig>,
    /// Font bytes retained for secondary window initialization.
    font_bytes: Vec<(String, Vec<u8>)>,
    /// Image bytes retained for secondary window initialization.
    image_bytes: Vec<(String, Vec<u8>, f32)>,
}

impl UiAppState {
    fn new_dsl(app: Application, doc: DslDocument) -> Self {
        let (ui_scene, loader, bindings, svg_sources) = Self::setup_dsl(&app);
        let (pending_configs, pending_builders) = Self::split_pending(app.pending_windows);
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
            image_renderer:        ImageRenderer::new(),
            loader,
            doc:                   Some(doc),
            bindings,
            root:                  None,
            event_handlers:        app.event_handlers,
            drag_origin:           None,
            svg_sources,
            last_raster_scale:     0.0,
            start_time:            std::time::Instant::now(),
            primary_window_id:     None,
            secondary_windows:     HashMap::new(),
            pending_builders,
            pending_configs,
            font_bytes:            app.fonts,
            image_bytes:           app.images,
        }
    }

    fn new_widget<F>(app: Application, build: F) -> Self
    where
        F: FnOnce(&FontMap) -> Element,
    {
        let (ui_scene, loader, bindings, svg_sources) = Self::setup_dsl(&app);
        let font_map = FontMap {
            fonts: bindings.fonts.clone(),
            images: bindings.images.clone(),
        };
        let root = build(&font_map);
        let (pending_configs, pending_builders) = Self::split_pending(app.pending_windows);
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
            image_renderer:        ImageRenderer::new(),
            loader,
            doc:                   None,
            bindings,
            root:                  Some(root),
            event_handlers:        app.event_handlers,
            drag_origin:           None,
            svg_sources,
            last_raster_scale:     0.0,
            start_time:            std::time::Instant::now(),
            primary_window_id:     None,
            secondary_windows:     HashMap::new(),
            pending_builders,
            pending_configs,
            font_bytes:            app.fonts,
            image_bytes:           app.images,
        }
    }

    fn split_pending(
        pending: Vec<(RuntimeConfig, Box<dyn Fn(&FontMap) -> Box<dyn Fn() -> Element>>)>,
    ) -> (
        VecDeque<RuntimeConfig>,
        VecDeque<Box<dyn Fn(&FontMap) -> Box<dyn Fn() -> Element>>>,
    ) {
        let mut configs = VecDeque::new();
        let mut builders = VecDeque::new();
        for (cfg, b) in pending {
            configs.push_back(cfg);
            builders.push_back(b);
        }
        (configs, builders)
    }

    /// Load fonts + images into a new `UiScene`, set up DSL loader, return all.
    ///
    /// SVG images are decoded at scale 1.0 initially; `UiAppState::on_frame`
    /// re-rasterizes them at the actual `raster_scale` on the first frame and
    /// whenever the scale changes thereafter.
    fn setup_dsl(
        app: &Application,
    ) -> (UiScene, DslLoader, DslBindings, Vec<(ImageId, Vec<u8>)>) {
        let mut ui_scene = UiScene::new();
        // Re-use the widget_state Rc that event handlers may already have captured.
        let mut bindings = DslBindings::with_state(app.widget_state.clone());
        let mut svg_sources: Vec<(ImageId, Vec<u8>)> = Vec::new();

        for (name, bytes) in &app.fonts {
            if let Ok(id) = ui_scene.load_font(bytes) {
                bindings.fonts.insert(name.clone(), id);
            } else {
                log::warn!("failed to load font '{name}'");
            }
        }

        for (name, bytes, scale) in &app.images {
            match decode_image(bytes, *scale) {
                Ok(img) => {
                    let id = ui_scene.load_image_scaled(
                        img.pixels, img.width, img.height,
                        img.logical_width, img.logical_height,
                    );
                    bindings.images.insert(name.clone(), id);
                    // Retain SVG source bytes so we can re-rasterize on scale change.
                    if is_svg(bytes) {
                        svg_sources.push((id, bytes.clone()));
                    }
                }
                Err(e) => log::warn!("failed to load image '{name}': {e}"),
            }
        }

        let mut loader = DslLoader::new();
        for (alias, src) in &app.components {
            if let Err(e) = loader.parse_and_register(alias.as_str(), src.as_str()) {
                log::warn!("failed to parse component '{alias}': {e}");
            }
        }

        // Wrap native slots: the stored Fn(&FontMap)->Element is called with a
        // captured FontMap snapshot so DslBindings sees Fn()->Element.
        let font_map = FontMap { fonts: bindings.fonts.clone(), images: bindings.images.clone() };
        for (id, fn_with_map) in &app.native_slots {
            let fm = FontMap { fonts: font_map.fonts.clone(), images: font_map.images.clone() };
            let f = Rc::clone(fn_with_map);
            let slot: Rc<dyn Fn() -> Element> = Rc::new(move || f(&fm));
            bindings.native_slots.insert(id.clone(), slot);
        }

        (ui_scene, loader, bindings, svg_sources)
    }

    /// Initialize a new `SecondaryWindowState` using stored font/image bytes.
    fn init_secondary(
        &self,
        builder: &dyn Fn(&FontMap) -> Box<dyn Fn() -> Element>,
    ) -> SecondaryWindowState {
        let mut scene = UiScene::new();
        let mut font_map = FontMap { fonts: HashMap::new(), images: HashMap::new() };
        let mut svg_sources: Vec<(ImageId, Vec<u8>)> = Vec::new();

        for (name, bytes) in &self.font_bytes {
            match scene.load_font(bytes) {
                Ok(id) => { font_map.fonts.insert(name.clone(), id); }
                Err(e) => log::warn!("secondary window font '{name}': {e}"),
            }
        }
        for (name, bytes, scale) in &self.image_bytes {
            match decode_image(bytes, *scale) {
                Ok(img) => {
                    let id = scene.load_image_scaled(
                        img.pixels, img.width, img.height,
                        img.logical_width, img.logical_height,
                    );
                    font_map.images.insert(name.clone(), id);
                    if is_svg(bytes) {
                        svg_sources.push((id, bytes.clone()));
                    }
                }
                Err(e) => log::warn!("secondary window image '{name}': {e}"),
            }
        }

        // Call the init closure once to set up persistent state and get the
        // per-frame frame-builder closure back.
        let root_fn = builder(&font_map);
        SecondaryWindowState {
            scene,
            rect_renderer:         RectRenderer::new(),
            rounded_rect_renderer: RoundedRectRenderer::new(),
            circle_renderer:       CircleRenderer::new(),
            text_renderer:         TextRenderer::new(),
            image_renderer:        ImageRenderer::new(),
            root_fn,
            drag_origin:           None,
            svg_sources,
            last_raster_scale:     0.0,
            start_time:            std::time::Instant::now(),
        }
    }

    /// Re-rasterize all SVG images at `scale` and update the image store.
    fn rerasterize_svgs(&mut self, scale: f32) {
        for (id, bytes) in &self.svg_sources {
            match decode_svg(bytes, scale) {
                Ok(img) => {
                    self.ui_scene.image_store.update(*id, img.pixels, img.width, img.height);
                }
                Err(e) => log::warn!("SVG re-rasterization failed: {e}"),
            }
        }
    }
}

impl EngineApp for UiAppState {
    fn on_frame(&mut self, ctx: &mut FrameCtx<'_, '_>) -> AppControl {
        let window_id = ctx.window.id;

        // ── Primary window detection ──────────────────────────────────────
        let is_primary = match self.primary_window_id {
            None => {
                // First frame ever — claim this window as primary.
                self.primary_window_id = Some(window_id);
                // Drain pending secondary window configs.
                while let Some(config) = self.pending_configs.pop_front() {
                    ctx.runtime.create_window(config);
                }
                true
            }
            Some(id) => id == window_id,
        };

        if !is_primary {
            return self.on_frame_secondary(ctx, window_id);
        }

        // ─────────────────────────────────────────────────────────────────
        // Primary window frame
        // ─────────────────────────────────────────────────────────────────

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

        let raw_scroll = ctx.input_frame.scroll_delta;

        // Mouse position in logical space.
        let (mx, my) = ctx.input.pointer_pos.unwrap_or((0.0, 0.0));
        let mouse_pos = Vec2::new(mx / self.zoom, my / self.zoom);

        // Track where the current drag started.
        if ctx.input_frame.buttons_pressed.contains(&MouseButton::Left) {
            self.drag_origin = Some(mouse_pos);
        }
        let drag_end = if ctx.input_frame.buttons_released.contains(&MouseButton::Left) {
            self.drag_origin.take()
        } else {
            None
        };

        // Layout viewport = window size / zoom.
        let ui_viewport = Vec2::new(w / self.zoom, h / self.zoom);

        let ui_input = UiInput {
            mouse_pos,
            mouse_pressed:  ctx.input.button_down(MouseButton::Left),
            mouse_clicked:  ctx.input_frame.buttons_released.contains(&MouseButton::Left),
            right_clicked:  ctx.input_frame.buttons_released.contains(&MouseButton::Right),
            text_input:     ctx.input_frame.text.iter().map(|t| t.text.clone()).collect(),
            keys_pressed:   ctx.input_frame.keys_pressed.iter().copied().collect(),
            scroll_delta:   raw_scroll,
            drag_origin:    self.drag_origin,
            drag_end,
            modifiers:      ctx.input.modifiers,
            time_ms:        self.start_time.elapsed().as_millis() as u64,
        };

        // ── Sync pixel_ratio for accurate text measurement ────────────────
        let os_scale = ctx.window.window.scale_factor() as f32;
        let raster_scale = (os_scale * self.zoom * 4.0).round() / 4.0;
        self.ui_scene.pixel_ratio = raster_scale;

        // ── Re-rasterize SVGs at current physical scale if it changed ─────
        if raster_scale != self.last_raster_scale && !self.svg_sources.is_empty() {
            self.rerasterize_svgs(raster_scale);
            self.last_raster_scale = raster_scale;
        }

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

        // Apply the cursor shape requested by widgets this frame.
        ctx.window.set_cursor(self.ui_scene.current_cursor().into());

        // ── Dispatch events ───────────────────────────────────────────────
        for event in self.bindings.take_events() {
            if let Some(handler) = self.event_handlers.get_mut(&event) {
                handler();
            }
        }

        // ── Render ────────────────────────────────────────────────────────
        let dl    = &mut self.ui_scene.draw_list;
        let fs    = &self.ui_scene.font_system;
        let imgs  = &self.ui_scene.image_store;
        let r_r   = &mut self.rect_renderer;
        let r_rr  = &mut self.rounded_rect_renderer;
        let r_c   = &mut self.circle_renderer;
        let r_t   = &mut self.text_renderer;
        let r_img = &mut self.image_renderer;
        let zoom  = self.zoom;

        ctx.render_scaled(zoom, marduk_engine::paint::Color::from_straight(0.054, 0.051, 0.043, 1.0), |rctx, target| {
            dl.set_z_range(i32::MIN, 99_999);
            r_r.render(rctx, target, dl);
            r_rr.render(rctx, target, dl);
            r_c.render(rctx, target, dl);
            r_img.render(rctx, target, dl, imgs);
            r_t.render(rctx, target, dl, fs);

            dl.set_z_range(100_000, i32::MAX);
            r_r.render(rctx, target, dl);
            r_rr.render(rctx, target, dl);
            r_c.render(rctx, target, dl);
            r_img.render(rctx, target, dl, imgs);
            r_t.render(rctx, target, dl, fs);

            dl.reset_z_range();
        })
    }
}

impl UiAppState {
    /// Per-frame logic for secondary OS windows.
    fn on_frame_secondary(
        &mut self,
        ctx: &mut FrameCtx<'_, '_>,
        window_id: WindowId,
    ) -> AppControl {
        // Lazily initialize the secondary window on first frame.
        if !self.secondary_windows.contains_key(&window_id) {
            if let Some(builder) = self.pending_builders.pop_front() {
                let state = self.init_secondary(builder.as_ref());
                self.secondary_windows.insert(window_id, state);
            } else {
                // No builder available — render nothing.
                return AppControl::Continue;
            }
        }

        let Some(sec) = self.secondary_windows.get_mut(&window_id) else {
            return AppControl::Continue;
        };

        let (w, h) = ctx.window.logical_size();
        let (mx, my) = ctx.input.pointer_pos.unwrap_or((0.0, 0.0));
        let mouse_pos = Vec2::new(mx, my);

        if ctx.input_frame.buttons_pressed.contains(&MouseButton::Left) {
            sec.drag_origin = Some(mouse_pos);
        }
        let drag_end = if ctx.input_frame.buttons_released.contains(&MouseButton::Left) {
            sec.drag_origin.take()
        } else {
            None
        };

        let ui_input = UiInput {
            mouse_pos,
            mouse_pressed:  ctx.input.button_down(MouseButton::Left),
            mouse_clicked:  ctx.input_frame.buttons_released.contains(&MouseButton::Left),
            right_clicked:  ctx.input_frame.buttons_released.contains(&MouseButton::Right),
            text_input:     ctx.input_frame.text.iter().map(|t| t.text.clone()).collect(),
            keys_pressed:   ctx.input_frame.keys_pressed.iter().copied().collect(),
            scroll_delta:   ctx.input_frame.scroll_delta,
            drag_origin:    sec.drag_origin,
            drag_end,
            modifiers:      ctx.input.modifiers,
            time_ms:        sec.start_time.elapsed().as_millis() as u64,
        };

        let os_scale = ctx.window.window.scale_factor() as f32;
        let raster_scale = (os_scale * 4.0).round() / 4.0;
        sec.scene.pixel_ratio = raster_scale;

        if raster_scale != sec.last_raster_scale && !sec.svg_sources.is_empty() {
            for (id, bytes) in &sec.svg_sources {
                if let Ok(img) = decode_svg(bytes, raster_scale) {
                    sec.scene.image_store.update(*id, img.pixels, img.width, img.height);
                }
            }
            sec.last_raster_scale = raster_scale;
        }

        let ui_viewport = Vec2::new(w, h);
        // Build a fresh element tree each frame (state lives in closures' Rc<RefCell<>>).
        let root = (sec.root_fn)();
        let _ = sec.scene.frame(root, ui_viewport, &ui_input);

        ctx.window.set_cursor(sec.scene.current_cursor().into());

        let dl    = &mut sec.scene.draw_list;
        let fs    = &sec.scene.font_system;
        let imgs  = &sec.scene.image_store;
        let r_r   = &mut sec.rect_renderer;
        let r_rr  = &mut sec.rounded_rect_renderer;
        let r_c   = &mut sec.circle_renderer;
        let r_t   = &mut sec.text_renderer;
        let r_img = &mut sec.image_renderer;

        ctx.render_scaled(1.0, marduk_engine::paint::Color::from_straight(0.054, 0.051, 0.043, 1.0), |rctx, target| {
            dl.set_z_range(i32::MIN, 99_999);
            r_r.render(rctx, target, dl);
            r_rr.render(rctx, target, dl);
            r_c.render(rctx, target, dl);
            r_img.render(rctx, target, dl, imgs);
            r_t.render(rctx, target, dl, fs);

            dl.set_z_range(100_000, i32::MAX);
            r_r.render(rctx, target, dl);
            r_rr.render(rctx, target, dl);
            r_c.render(rctx, target, dl);
            r_img.render(rctx, target, dl, imgs);
            r_t.render(rctx, target, dl, fs);

            dl.reset_z_range();
        })
    }
}

