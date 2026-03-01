use marduk_engine::coords::{CornerRadii, Rect, Vec2};
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::{Border, DrawList, ZIndex};
use marduk_engine::text::{FontId, FontSystem};

use crate::constraints::LayoutCtx;

/// Drawing surface passed to [`Widget::paint`].
///
/// Wraps the engine's `DrawList` with a high-level API and exposes
/// per-frame input state so widgets can express hover / pressed visuals
/// directly in their paint implementations.
pub struct Painter<'a> {
    pub(crate) draw_list: &'a mut DrawList,
    pub(crate) font_system: &'a FontSystem,
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
}

impl<'a> Painter<'a> {
    pub(crate) fn new(
        draw_list: &'a mut DrawList,
        font_system: &'a FontSystem,
        mouse_pos: Vec2,
        mouse_pressed: bool,
        scale: f32,
    ) -> Self {
        Self { draw_list, font_system, scale, z: 0, mouse_pos, mouse_pressed }
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

    // ── layout context ────────────────────────────────────────────────────

    /// Returns a [`LayoutCtx`] borrowing this painter's font system.
    ///
    /// Useful inside [`Widget::paint`] when a container needs to re-measure
    /// its children to compute their layout positions.
    #[inline]
    pub fn layout_ctx(&self) -> LayoutCtx<'_> {
        LayoutCtx { fonts: self.font_system, scale: self.scale }
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
