use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::scene::DrawList;
use marduk_engine::text::{FontId, FontSystem};

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::UiEvent;
use crate::painter::Painter;
use crate::widget::Widget;

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
    draw_list: DrawList,
}

impl UiScene {
    pub fn new() -> Self {
        Self { font_system: FontSystem::new(), draw_list: DrawList::new() }
    }

    /// Load a TrueType / OpenType font from raw bytes.
    pub fn load_font(&mut self, data: &[u8]) -> Result<FontId, &'static str> {
        self.font_system.load_font(data)
    }

    /// Build, layout, and paint a widget tree for this frame.
    ///
    /// The root widget is consumed (it is freshly constructed each call).
    /// The returned `&mut DrawList` is owned by the `UiScene` and valid
    /// until the next call to `frame`.
    ///
    /// Pass the returned list to each engine renderer in your render closure.
    pub fn frame<W: Widget>(
        &mut self,
        mut root: W,
        viewport: Vec2,
        input: &UiInput,
    ) -> &mut DrawList {
        self.draw_list.clear();

        // ── measure ───────────────────────────────────────────────────────
        let ctx = LayoutCtx { fonts: &self.font_system };
        let constraints = Constraints::loose(viewport);
        let size = root.measure(constraints, &ctx);
        let rect = Rect::new(0.0, 0.0, size.x.min(viewport.x), size.y.min(viewport.y));

        // ── paint ─────────────────────────────────────────────────────────
        {
            let mut painter = Painter::new(
                &mut self.draw_list,
                &self.font_system,
                input.mouse_pos,
                input.mouse_pressed,
            );
            root.paint(&mut painter, rect);
        }

        // ── events ────────────────────────────────────────────────────────
        if input.mouse_clicked {
            root.on_event(&UiEvent::Click { pos: input.mouse_pos }, rect);
        }

        &mut self.draw_list
    }
}

impl Default for UiScene {
    fn default() -> Self {
        Self::new()
    }
}
