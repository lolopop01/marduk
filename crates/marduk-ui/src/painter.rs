use std::cell::RefCell;
use std::rc::Rc;

use marduk_engine::coords::{CornerRadii, Rect, Vec2};
use marduk_engine::image::{ImageId, ImageStore};
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::{Border, DrawList, ZIndex};
use marduk_engine::text::{FontId, FontSystem};

use crate::constraints::LayoutCtx;
use crate::focus::{FocusId, FocusManager};

/// Drawing surface passed to [`Widget::paint`].
///
/// Wraps the engine's `DrawList` with a high-level API and exposes
/// per-frame input state so widgets can express hover / pressed visuals
/// directly in their paint implementations.
pub struct Painter<'a> {
    pub(crate) draw_list: &'a mut DrawList,
    pub(crate) font_system: &'a FontSystem,
    pub(crate) image_store: &'a ImageStore,
    /// Physical-to-logical pixel ratio for this frame (os_scale × zoom).
    ///
    /// Use [`measure_text`] rather than `font_system.measure_text` so that
    /// text width measurements match the renderer's physical-scale layout.
    pub scale: f32,
    z: i32,
    /// Current mouse position in logical pixels.
    pub mouse_pos: Vec2,
    /// True while the primary button is held down.
    pub mouse_pressed: bool,
    /// Focus manager for this frame. May be `None` in contexts without focus support.
    ///
    /// Stored as a `RefCell` reference so focus state can be mutated through the
    /// shared `&Painter` reference that widget paint implementations receive.
    focus: Option<&'a RefCell<FocusManager>>,
    /// Overlay rect registry shared with `UiScene`.
    ///
    /// Widgets with open popups call [`register_overlay`] to declare the popup
    /// rect.  After paint, the scene uses this list for click-routing decisions.
    overlays: Option<Rc<RefCell<Vec<Rect>>>>,
    /// Monotonic application time in milliseconds. Matches [`UiInput::time_ms`].
    pub time_ms: u64,
}

impl<'a> Painter<'a> {
    pub(crate) fn new(
        draw_list: &'a mut DrawList,
        font_system: &'a FontSystem,
        image_store: &'a ImageStore,
        mouse_pos: Vec2,
        mouse_pressed: bool,
        scale: f32,
        time_ms: u64,
    ) -> Self {
        Self {
            draw_list,
            font_system,
            image_store,
            scale,
            z: 0,
            mouse_pos,
            mouse_pressed,
            focus: None,
            overlays: None,
            time_ms,
        }
    }

    pub(crate) fn with_focus(mut self, focus: &'a RefCell<FocusManager>) -> Self {
        self.focus = Some(focus);
        self
    }

    pub(crate) fn with_overlays(mut self, overlays: Rc<RefCell<Vec<Rect>>>) -> Self {
        self.overlays = Some(overlays);
        self
    }

    // ── focus ─────────────────────────────────────────────────────────────

    /// Returns `true` if `id` is the currently focused widget.
    ///
    /// Call this during paint to drive focused-state visuals (e.g. border color).
    #[inline]
    pub fn is_focused(&self, id: FocusId) -> bool {
        self.focus.as_ref().is_some_and(|f| f.borrow().is_focused(id))
    }

    /// Request keyboard focus for `id`.
    ///
    /// Call this during paint when the widget is clicked or activated.
    /// Focus takes effect at end of frame.
    #[inline]
    pub fn request_focus(&mut self, id: FocusId) {
        if let Some(f) = &self.focus {
            f.borrow_mut().request_focus(id);
        }
    }

    /// Register `id` as a focusable widget in Tab-cycle order.
    ///
    /// Call this from [`Widget::paint`] for every widget that can receive keyboard
    /// focus. The order of calls determines the Tab-cycle order.
    #[inline]
    pub fn register_focusable(&mut self, id: FocusId) {
        if let Some(f) = &self.focus {
            f.borrow_mut().register(id);
        }
    }

    // ── input queries ─────────────────────────────────────────────────────

    /// Returns `true` if the mouse cursor is inside `rect`.
    #[inline]
    pub fn is_hovered(&self, rect: Rect) -> bool {
        rect.contains(self.mouse_pos)
    }

    /// Returns `true` if the primary button is held and the cursor is over `rect`.
    #[inline]
    pub fn is_pressed(&self, rect: Rect) -> bool {
        self.mouse_pressed && rect.contains(self.mouse_pos)
    }

    // ── text measurement ──────────────────────────────────────────────────

    /// Measures `text` at the renderer's current physical scale.
    ///
    /// Prefer this over `font_system.measure_text` inside widget `paint`
    /// implementations: it lays out at `size × scale` and divides back, so
    /// the returned width matches the positions the text renderer actually
    /// places glyphs at, eliminating cursor-drift at HiDPI or non-1× zoom.
    pub fn measure_text(
        &self,
        text: &str,
        font: FontId,
        size: f32,
        max_width: Option<f32>,
    ) -> Vec2 {
        self.font_system.measure_text_scaled(text, font, size, max_width, self.scale)
    }

    // ── overlay ───────────────────────────────────────────────────────────

    /// Register `rect` as an overlay region for this frame.
    ///
    /// When overlays are registered, a click that falls **outside** all overlay
    /// rects dispatches [`UiEvent::OverlayDismiss`] rather than
    /// [`UiEvent::Click`].  Widgets that own open popups should consume
    /// `OverlayDismiss` to close themselves.
    pub fn register_overlay(&mut self, rect: Rect) {
        if let Some(overlays) = &self.overlays {
            overlays.borrow_mut().push(rect);
        }
    }

    /// Execute `f` with the Z-index boosted to the overlay layer.
    ///
    /// All draw calls inside `f` will appear above all regular widget content.
    /// The clip stack is also cleared so overlay content (e.g. combobox dropdowns)
    /// can render outside their parent container's scissor region.
    /// Both the Z-index and the clip stack are restored after `f` returns.
    pub fn overlay_scope(&mut self, f: impl FnOnce(&mut Painter)) {
        let old_z = self.z;
        let saved_clips = self.draw_list.take_clips();
        self.z = 100_000;
        f(self);
        self.z = old_z;
        self.draw_list.restore_clips(saved_clips);
    }

    // ── layout context ────────────────────────────────────────────────────

    /// Returns a [`LayoutCtx`] borrowing this painter's font and image stores.
    ///
    /// Useful inside [`Widget::paint`] when a container needs to re-measure
    /// its children to compute their layout positions.
    #[inline]
    pub fn layout_ctx(&self) -> LayoutCtx<'_> {
        LayoutCtx {
            fonts: self.font_system,
            images: self.image_store,
            scale: self.scale,
            focus: None,
            time_ms: self.time_ms,
        }
    }

    // ── drawing ───────────────────────────────────────────────────────────

    /// Solid axis-aligned rectangle.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        let z = self.next_z();
        self.draw_list.push_solid_rect(z, rect, color);
    }

    /// Rounded rectangle with optional border.
    ///
    /// Pass `radius = 0.0` for sharp corners. Pass `border = None` for no stroke.
    pub fn fill_rounded_rect(
        &mut self,
        rect: Rect,
        radius: f32,
        paint: impl Into<Paint>,
        border: Option<Border>,
    ) {
        let z = self.next_z();
        self.draw_list.push_rounded_rect(z, rect, CornerRadii::all(radius), paint.into(), border);
    }

    /// Circle with optional border.
    pub fn fill_circle(
        &mut self,
        center: Vec2,
        radius: f32,
        paint: impl Into<Paint>,
        border: Option<Border>,
    ) {
        let z = self.next_z();
        self.draw_list.push_circle(z, center, radius, paint.into(), border);
    }

    /// Rounded rectangle with per-corner radii and optional border.
    pub fn fill_rounded_rect_corners(
        &mut self,
        rect: Rect,
        radii: CornerRadii,
        paint: impl Into<Paint>,
        border: Option<Border>,
    ) {
        let z = self.next_z();
        self.draw_list.push_rounded_rect(z, rect, radii, paint.into(), border);
    }

    /// Text at `origin` (top-left of the first line), clipped to `max_width`.
    pub fn text(
        &mut self,
        text: impl Into<String>,
        font: FontId,
        size: f32,
        color: Color,
        origin: Vec2,
        max_width: Option<f32>,
    ) {
        let z = self.next_z();
        self.draw_list.push_text(z, text, font, size, color, origin, max_width);
    }

    /// Draw an image in `dest_rect`.
    ///
    /// - `uv_min` / `uv_max`: texture coordinate range (use `[0,0]`/`[1,1]` for full image).
    /// - `tint_straight`: straight-alpha RGBA multiplier; pass `[1,1,1,1]` for no tint.
    /// - `corner_radii`: rounds the corners with an SDF clip.
    pub fn draw_image(
        &mut self,
        dest_rect: Rect,
        id: ImageId,
        uv_min: [f32; 2],
        uv_max: [f32; 2],
        tint_straight: [f32; 4],
        corner_radii: CornerRadii,
    ) {
        let z = self.next_z();
        self.draw_list.push_image(z, dest_rect, id, uv_min, uv_max, tint_straight, corner_radii);
    }

    // ── clipping ──────────────────────────────────────────────────────────

    /// Begin a scissor region. Must be paired with [`pop_clip`].
    pub fn push_clip(&mut self, rect: Rect) {
        self.draw_list.push_clip(rect);
    }

    /// End the most recent scissor region.
    pub fn pop_clip(&mut self) {
        self.draw_list.pop_clip();
    }

    // ── internal ──────────────────────────────────────────────────────────

    #[inline]
    fn next_z(&mut self) -> ZIndex {
        let z = ZIndex::new(self.z);
        self.z += 1;
        z
    }
}
