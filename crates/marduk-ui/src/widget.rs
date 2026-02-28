use marduk_engine::coords::{Rect, Vec2};

use crate::constraints::{Constraints, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;

// (LayoutCtx is re-exported here for on_event impls in other modules)

// ── Widget trait ──────────────────────────────────────────────────────────

/// The core trait every UI component implements.
///
/// # Implementing a custom widget
///
/// ```rust,ignore
/// use marduk_ui::prelude::*;
///
/// pub struct MyBadge { color: Color, size: f32 }
///
/// impl Widget for MyBadge {
///     fn measure(&self, _constraints: Constraints, _ctx: &LayoutCtx) -> Vec2 {
///         Vec2::new(self.size, self.size)
///     }
///     fn paint(&self, painter: &mut Painter, rect: Rect) {
///         painter.fill_rounded_rect(rect, rect.size.x / 2.0, self.color, None);
///     }
/// }
///
/// // Use it anywhere an Element is accepted:
/// Column::new().child(MyBadge { color: Color::from_straight(1.0, 0.0, 0.0, 1.0), size: 12.0 })
/// ```
pub trait Widget: 'static {
    /// Compute the size this widget wants given the available space.
    ///
    /// Must be deterministic — calling `measure` twice with the same arguments
    /// must return the same result. The parent may call `measure` multiple times.
    #[must_use]
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2;

    /// Draw this widget into `painter` within the bounds of `rect`.
    ///
    /// `rect` is the space allocated by the parent — the widget draws inside it.
    /// Children are painted by calling their own `paint` recursively.
    fn paint(&self, painter: &mut Painter, rect: Rect);

    /// Route an input event. Return [`EventResult::Consumed`] to stop propagation.
    ///
    /// `ctx` provides the font system so container widgets can re-run layout
    /// and pass each child its actual screen rect (enabling correct hit-testing).
    ///
    /// The default implementation does nothing and returns `Ignored`, so leaf
    /// widgets only need to override this if they handle events.
    fn on_event(&mut self, _event: &UiEvent, _rect: Rect, _ctx: &LayoutCtx<'_>) -> EventResult {
        EventResult::Ignored
    }
}

// ── Element ───────────────────────────────────────────────────────────────

/// A type-erased widget — the universal child type for container widgets.
///
/// Any `Widget` converts to `Element` via `From` / `Into`:
///
/// ```rust,ignore
/// Column::new()
///     .child(Text::new("hello", font, 14.0, white))  // Text: Widget → Element
///     .child(my_custom_widget)                        // anything: Widget → Element
/// ```
pub struct Element(Box<dyn Widget>);

impl Element {
    pub fn new<W: Widget>(w: W) -> Self {
        Self(Box::new(w))
    }

    #[inline]
    #[must_use]
    pub fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        self.0.measure(constraints, ctx)
    }

    #[inline]
    pub fn paint(&self, painter: &mut Painter, rect: Rect) {
        self.0.paint(painter, rect)
    }

    #[inline]
    pub fn on_event(&mut self, event: &UiEvent, rect: Rect, ctx: &LayoutCtx<'_>) -> EventResult {
        self.0.on_event(event, rect, ctx)
    }
}

impl<W: Widget> From<W> for Element {
    fn from(w: W) -> Self {
        Self::new(w)
    }
}

