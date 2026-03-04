use std::cell::{Cell, RefCell};
use std::rc::Rc;

use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::image::{ImageId, ImageStore};
use marduk_engine::input::{Key, Modifiers};
use marduk_engine::scene::DrawList;
use marduk_engine::text::{FontId, FontSystem};

use crate::constraints::{Constraints, LayoutCtx};
use crate::cursor::CursorIcon;
use crate::event::UiEvent;
use crate::focus::FocusManager;
use crate::painter::Painter;
use crate::widget::{Element, Widget};

// ── UiInput ───────────────────────────────────────────────────────────────

/// Snapshot of input state for one UI frame.
///
/// Construct this from your engine's `InputState` / `InputFrame` each frame.
#[derive(Debug, Clone, Default)]
pub struct UiInput {
    /// Current cursor position in logical pixels.
    pub mouse_pos: Vec2,
    /// `true` while the primary button is held down.
    pub mouse_pressed: bool,
    /// `true` for exactly one frame when the primary button is released.
    pub mouse_clicked: bool,
    /// Committed text characters typed this frame (for `TextBox`).
    pub text_input: Vec<String>,
    /// Named keys pressed this frame (Backspace, Enter, …).
    pub keys_pressed: Vec<Key>,
    /// Accumulated scroll wheel delta this frame (positive = scroll down).
    pub scroll_delta: f32,
    /// Modifier keys currently held (Shift, Ctrl, Alt, Meta).
    pub modifiers: Modifiers,
    /// Where the current mouse drag started (`None` when no drag is in progress).
    pub drag_origin: Option<Vec2>,
    /// Set to `Some(start)` for the single frame in which a drag ends (button released).
    /// Used to dispatch `DragEnd` so widgets can commit regardless of release position.
    pub drag_end: Option<Vec2>,
    /// Monotonic application time in milliseconds (from app startup).
    ///
    /// Used by time-dependent widgets (e.g. `Tooltip`) to measure hover duration.
    pub time_ms: u64,
}

// ── UiScene ───────────────────────────────────────────────────────────────

/// Top-level coordinator that owns shared resources across frames.
///
/// Owns the `FontSystem` (and therefore all loaded fonts and the glyph atlas
/// indirectly through the `TextRenderer`) and the `DrawList` that is populated
/// each frame by [`frame`].
///
/// The GPU renderers (`RectRenderer`, `TextRenderer`, …) still live in the
/// application and receive `&mut DrawList` returned by `frame`.
///
/// # Example
///
/// ```rust,ignore
/// let mut ui = UiScene::new();
/// let font  = ui.load_font(include_bytes!("my_font.ttf")).unwrap();
///
/// // In your on_frame callback:
/// let draw_list = ui.frame(
///     Column::new().child(Text::new("Hello", font, 16.0, white)),
///     viewport,
///     &UiInput { mouse_pos, ..Default::default() },
/// );
/// rect_renderer.render(rctx, target, draw_list);
/// text_renderer.render(rctx, target, draw_list, &ui.font_system);
/// ```
pub struct UiScene {
    /// Fonts are public so the application can pass `&ui.font_system` to the
    /// engine's `TextRenderer::render`.
    pub font_system: FontSystem,
    /// Images are public so the application can pass `&ui.image_store` to the
    /// engine's `ImageRenderer::render`.
    pub image_store: ImageStore,
    /// Draw list populated by the most recent [`frame`] call.
    ///
    /// Public so callers can split-borrow it alongside `font_system` when
    /// passing both to engine renderers.
    pub draw_list: DrawList,
    /// Physical-to-logical pixel ratio for text measurement.
    ///
    /// Set this each frame to `os_scale_factor × zoom` (quantised to 0.25
    /// steps, matching the text renderer's `raster_scale`) **before** calling
    /// [`frame`] or [`frame_ref`].  The [`Painter`] carries this value so
    /// widgets can call [`Painter::measure_text`] and get widths that exactly
    /// match what the renderer will draw.
    pub pixel_ratio: f32,
    /// Keyboard focus manager. Tracks which widget is focused and provides
    /// Tab-key cycling. Wrapped in `RefCell` so widgets can request focus
    /// through the shared `&LayoutCtx` during event routing.
    pub focus: RefCell<FocusManager>,
    /// Overlay rect registry populated by widgets during each paint pass.
    ///
    /// Cleared at the start of every frame.  Widgets with open popups call
    /// `painter.register_overlay(rect)` to push their popup rect here.
    /// After paint, the event-routing phase uses this list to dispatch
    /// `OverlayDismiss` when a click misses all registered overlay rects.
    overlay_rects: Rc<RefCell<Vec<Rect>>>,
    /// Cursor shape requested by widgets this frame.
    ///
    /// Reset to `CursorIcon::Default` at the start of each frame.
    /// Read via [`current_cursor`] after the frame to apply to the OS window.
    cursor: Rc<Cell<CursorIcon>>,
}

impl UiScene {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            image_store: ImageStore::new(),
            draw_list: DrawList::new(),
            pixel_ratio: 1.0,
            focus: RefCell::new(FocusManager::new()),
            overlay_rects: Rc::new(RefCell::new(Vec::new())),
            cursor: Rc::new(Cell::new(CursorIcon::Default)),
        }
    }

    /// Returns the cursor shape requested by widgets during the last frame.
    ///
    /// Call this after [`frame`] / [`frame_ref`] and apply the result to the
    /// OS window cursor (e.g. via `window.set_cursor(icon.into())`).
    pub fn current_cursor(&self) -> CursorIcon {
        self.cursor.get()
    }

    /// Load a TrueType / OpenType font from raw bytes.
    pub fn load_font(&mut self, data: &[u8]) -> Result<FontId, marduk_engine::text::FontLoadError> {
        self.font_system.load_font(data)
    }

    /// Store premultiplied RGBA8 pixel data where logical size == physical size.
    ///
    /// Use this for raster images. For SVGs use [`load_image_scaled`].
    pub fn load_image(&mut self, pixels: Vec<u8>, width: u32, height: u32) -> ImageId {
        self.image_store.insert(pixels, width, height)
    }

    /// Store premultiplied RGBA8 pixel data where the physical texture differs
    /// from the logical layout size (e.g. SVG rasterized at a scale factor).
    pub fn load_image_scaled(
        &mut self,
        pixels: Vec<u8>,
        width: u32,
        height: u32,
        logical_width: u32,
        logical_height: u32,
    ) -> ImageId {
        self.image_store.insert_scaled(pixels, width, height, logical_width, logical_height)
    }

    /// Like [`frame`] but borrows the root widget instead of consuming it.
    ///
    /// Use this when the root widget holds state that must persist across frames
    /// (e.g. selection, scroll position). The widget is kept alive in the caller
    /// and updated via `on_event` each frame.
    #[must_use]
    pub fn frame_ref(
        &mut self,
        root: &mut Element,
        viewport: Vec2,
        input: &UiInput,
    ) -> &mut DrawList {
        self.draw_list.clear();
        self.overlay_rects.borrow_mut().clear();
        self.cursor.set(CursorIcon::Default);

        // ── Tab / Escape: advance or clear focus before the paint pass ────
        // This ensures the paint pass sees the correct focus state.
        for key in &input.keys_pressed {
            match key {
                Key::Tab => self.focus.borrow_mut().advance(input.modifiers.shift),
                Key::Escape => self.focus.borrow_mut().clear(),
                _ => {}
            }
        }

        // ── measure ───────────────────────────────────────────────────────
        let ctx = LayoutCtx {
            fonts: &self.font_system,
            images: &self.image_store,
            scale: self.pixel_ratio,
            focus: None,
            time_ms: input.time_ms,
        };
        let _ = root.measure(Constraints::loose(viewport), &ctx);
        let rect = Rect::new(0.0, 0.0, viewport.x, viewport.y);

        // ── paint ─────────────────────────────────────────────────────────
        {
            let mut painter = Painter::new(
                &mut self.draw_list,
                &self.font_system,
                &self.image_store,
                input.mouse_pos,
                input.mouse_pressed,
                self.pixel_ratio,
                input.time_ms,
            ).with_focus(&self.focus)
             .with_overlays(Rc::clone(&self.overlay_rects))
             .with_cursor(Rc::clone(&self.cursor));
            root.paint(&mut painter, rect);
        }

        // ── events ────────────────────────────────────────────────────────
        {
            let ctx = LayoutCtx {
                fonts: &self.font_system,
                images: &self.image_store,
                scale: self.pixel_ratio,
                focus: Some(&self.focus),
                time_ms: input.time_ms,
            };
            root.on_event(&UiEvent::Hover { pos: input.mouse_pos }, rect, &ctx);
            if let Some(start) = input.drag_origin {
                root.on_event(&UiEvent::Drag { pos: input.mouse_pos, start }, rect, &ctx);
            }
            if let Some(start) = input.drag_end {
                root.on_event(&UiEvent::DragEnd { pos: input.mouse_pos, start }, rect, &ctx);
            }
            if input.mouse_clicked {
                let overlays = self.overlay_rects.borrow();
                if !overlays.is_empty() && !overlays.iter().any(|r| r.contains(input.mouse_pos)) {
                    // Click outside all overlays → dismiss them instead.
                    drop(overlays);
                    root.on_event(&UiEvent::OverlayDismiss, rect, &ctx);
                } else {
                    drop(overlays);
                    root.on_event(&UiEvent::Click { pos: input.mouse_pos }, rect, &ctx);
                }
            }
            for text in &input.text_input {
                root.on_event(&UiEvent::TextInput { text: text.clone() }, rect, &ctx);
            }
            for key in &input.keys_pressed {
                root.on_event(&UiEvent::KeyPress { key: *key, modifiers: input.modifiers }, rect, &ctx);
            }
            if input.scroll_delta != 0.0 {
                root.on_event(&UiEvent::ScrollWheel { delta: input.scroll_delta }, rect, &ctx);
            }
        }

        // ── end of frame: apply focus requests, dispatch focus events ─────
        self.focus.borrow_mut().end_frame();
        {
            let ctx = LayoutCtx {
                fonts: &self.font_system,
                images: &self.image_store,
                scale: self.pixel_ratio,
                focus: Some(&self.focus),
                time_ms: input.time_ms,
            };
            if self.focus.borrow().just_gained().is_some() {
                root.on_event(&UiEvent::FocusGained, rect, &ctx);
            }
            if self.focus.borrow().just_lost().is_some() {
                root.on_event(&UiEvent::FocusLost, rect, &ctx);
            }
        }

        &mut self.draw_list
    }

    /// Build, layout, and paint a widget tree for this frame.
    ///
    /// The root widget is consumed (it is freshly constructed each call).
    /// The returned `&mut DrawList` is owned by the `UiScene` and valid
    /// until the next call to `frame`.
    ///
    /// Pass the returned list to each engine renderer in your render closure.
    /// Convenience: wrap any [`Widget`] in an [`Element`] and call [`frame`].
    pub fn frame_widget<W: Widget>(
        &mut self,
        root: W,
        viewport: Vec2,
        input: &UiInput,
    ) -> &mut DrawList {
        self.frame(root.into(), viewport, input)
    }

    /// Build, layout, and paint a widget tree for this frame.
    ///
    /// The root widget is consumed (freshly constructed each call). The returned
    /// `&mut DrawList` is valid until the next call to `frame`.
    ///
    /// # Event routing
    ///
    /// - **`Hover`** visits every widget unconditionally every frame.
    /// - All other events (`Click`, `KeyPress`, `TextInput`, `ScrollWheel`,
    ///   `Drag`, `DragEnd`) stop at the first widget returning
    ///   [`crate::event::EventResult::Consumed`].
    ///
    /// A nested `ScrollView` inside a `Column` correctly captures scroll events
    /// before the outer column sees them.
    #[must_use]
    pub fn frame(
        &mut self,
        mut root: Element,
        viewport: Vec2,
        input: &UiInput,
    ) -> &mut DrawList {
        self.draw_list.clear();
        self.overlay_rects.borrow_mut().clear();
        self.cursor.set(CursorIcon::Default);

        // ── Tab / Escape: advance or clear focus before the paint pass ────
        for key in &input.keys_pressed {
            match key {
                Key::Tab => self.focus.borrow_mut().advance(input.modifiers.shift),
                Key::Escape => self.focus.borrow_mut().clear(),
                _ => {}
            }
        }

        // ── measure ───────────────────────────────────────────────────────
        let ctx = LayoutCtx {
            fonts: &self.font_system,
            images: &self.image_store,
            scale: self.pixel_ratio,
            focus: None,
            time_ms: input.time_ms,
        };
        let _ = root.measure(Constraints::loose(viewport), &ctx);
        let rect = Rect::new(0.0, 0.0, viewport.x, viewport.y);

        // ── paint ─────────────────────────────────────────────────────────
        {
            let mut painter = Painter::new(
                &mut self.draw_list,
                &self.font_system,
                &self.image_store,
                input.mouse_pos,
                input.mouse_pressed,
                self.pixel_ratio,
                input.time_ms,
            ).with_focus(&self.focus)
             .with_overlays(Rc::clone(&self.overlay_rects))
             .with_cursor(Rc::clone(&self.cursor));
            root.paint(&mut painter, rect);
        }

        // ── events ────────────────────────────────────────────────────────
        {
            let ctx = LayoutCtx {
                fonts: &self.font_system,
                images: &self.image_store,
                scale: self.pixel_ratio,
                focus: Some(&self.focus),
                time_ms: input.time_ms,
            };
            root.on_event(&UiEvent::Hover { pos: input.mouse_pos }, rect, &ctx);
            if let Some(start) = input.drag_origin {
                root.on_event(&UiEvent::Drag { pos: input.mouse_pos, start }, rect, &ctx);
            }
            if let Some(start) = input.drag_end {
                root.on_event(&UiEvent::DragEnd { pos: input.mouse_pos, start }, rect, &ctx);
            }
            if input.mouse_clicked {
                let overlays = self.overlay_rects.borrow();
                if !overlays.is_empty() && !overlays.iter().any(|r| r.contains(input.mouse_pos)) {
                    drop(overlays);
                    root.on_event(&UiEvent::OverlayDismiss, rect, &ctx);
                } else {
                    drop(overlays);
                    root.on_event(&UiEvent::Click { pos: input.mouse_pos }, rect, &ctx);
                }
            }
            for text in &input.text_input {
                root.on_event(&UiEvent::TextInput { text: text.clone() }, rect, &ctx);
            }
            for key in &input.keys_pressed {
                root.on_event(&UiEvent::KeyPress { key: *key, modifiers: input.modifiers }, rect, &ctx);
            }
            if input.scroll_delta != 0.0 {
                root.on_event(&UiEvent::ScrollWheel { delta: input.scroll_delta }, rect, &ctx);
            }
        }

        // ── end of frame: apply focus requests, dispatch focus events ─────
        self.focus.borrow_mut().end_frame();
        {
            let ctx = LayoutCtx {
                fonts: &self.font_system,
                images: &self.image_store,
                scale: self.pixel_ratio,
                focus: Some(&self.focus),
                time_ms: input.time_ms,
            };
            if self.focus.borrow().just_gained().is_some() {
                root.on_event(&UiEvent::FocusGained, rect, &ctx);
            }
            if self.focus.borrow().just_lost().is_some() {
                root.on_event(&UiEvent::FocusLost, rect, &ctx);
            }
        }

        &mut self.draw_list
    }
}

impl Default for UiScene {
    fn default() -> Self {
        Self::new()
    }
}
