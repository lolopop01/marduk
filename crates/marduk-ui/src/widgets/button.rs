use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;

use crate::constraints::{inset_rect, Constraints, Edges, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::{Element, Widget};

/// A clickable widget that wraps any child content.
///
/// Visual state (hover, press) is read from `Painter` during `paint` so no
/// retained state is needed — the button tree can be rebuilt each frame.
///
/// # Example
/// ```rust,ignore
/// Button::new(Text::new("Save", font, 14.0, black))
///     .background(Color::from_straight(0.2, 0.6, 1.0, 1.0))
///     .hover_background(Color::from_straight(0.3, 0.7, 1.0, 1.0))
///     .padding_all(10.0)
///     .corner_radius(6.0)
///     .on_click(|| println!("saved!"))
/// ```
pub struct Button {
    child: Element,
    on_click: Option<Box<dyn FnMut()>>,

    background: Color,
    hover_background: Color,
    press_background: Color,
    border: Option<Border>,
    corner_radius: f32,
    padding: Edges,
    min_width: f32,
    min_height: f32,
}

impl Button {
    pub fn new(child: impl Into<Element>) -> Self {
        Self {
            child: child.into(),
            on_click: None,
            background: Color::transparent(),
            hover_background: Color::transparent(),
            press_background: Color::transparent(),
            border: None,
            corner_radius: 0.0,
            padding: Edges::default(),
            min_width: 0.0,
            min_height: 0.0,
        }
    }

    /// Callback invoked when the button is clicked.
    pub fn on_click(mut self, f: impl FnMut() + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    /// Background color when the cursor is over the button.
    pub fn hover_background(mut self, color: Color) -> Self {
        self.hover_background = color;
        self
    }

    /// Background color while the primary button is held.
    pub fn press_background(mut self, color: Color) -> Self {
        self.press_background = color;
        self
    }

    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }

    pub fn corner_radius(mut self, r: f32) -> Self {
        self.corner_radius = r;
        self
    }

    pub fn padding(mut self, edges: Edges) -> Self {
        self.padding = edges;
        self
    }

    pub fn padding_all(mut self, v: f32) -> Self {
        self.padding = Edges::all(v);
        self
    }

    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }
}

impl Widget for Button {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        let inner = constraints.shrink(self.padding);
        let child_size = self.child.measure(inner, ctx);
        let w = (child_size.x + self.padding.h()).max(self.min_width);
        let h = (child_size.y + self.padding.v()).max(self.min_height);
        constraints.constrain(Vec2::new(w, h))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        // Pick visual state.
        let bg = if painter.is_pressed(rect) {
            self.press_background
        } else if painter.is_hovered(rect) {
            self.hover_background
        } else {
            self.background
        };

        painter.fill_rounded_rect(rect, self.corner_radius, Paint::Solid(bg), self.border.clone());
        self.child.paint(painter, inset_rect(rect, self.padding));
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect, _ctx: &LayoutCtx<'_>) -> EventResult {
        if let UiEvent::Click { pos } = event {
            if rect.contains(*pos) {
                if let Some(f) = &mut self.on_click {
                    f();
                }
                return EventResult::Consumed;
            }
        }
        EventResult::Ignored
    }
}
