use marduk_engine::coords::{Rect, Vec2};
use marduk_engine::paint::{Color, Paint};
use marduk_engine::scene::Border;

use crate::constraints::{inset_rect, Constraints, Edges, LayoutCtx};
use crate::event::{EventResult, UiEvent};
use crate::painter::Painter;
use crate::widget::{Element, Widget};

/// A single-child widget that applies padding, background, border, and/or corner rounding.
///
/// All properties are optional — an empty `Container` is a no-op.
///
/// # Example
/// ```rust,ignore
/// Container::new()
///     .padding_all(12.0)
///     .background(Color::from_straight(0.1, 0.1, 0.15, 1.0))
///     .corner_radius(8.0)
///     .border(Border::new(1.0, Color::from_straight(0.3, 0.3, 0.35, 1.0)))
///     .child(Text::new("hello", font, 14.0, white))
/// ```
pub struct Container {
    child: Option<Element>,
    padding: Edges,
    background: Option<Paint>,
    border: Option<Border>,
    corner_radius: f32,
    min_width: f32,
    min_height: f32,
}

impl Container {
    pub fn new() -> Self {
        Self {
            child: None,
            padding: Edges::default(),
            background: None,
            border: None,
            corner_radius: 0.0,
            min_width: 0.0,
            min_height: 0.0,
        }
    }

    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.child = Some(child.into());
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

    pub fn background(mut self, paint: impl Into<Paint>) -> Self {
        self.background = Some(paint.into());
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

    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.min_width = width;
        self.min_height = height;
        self
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Container {
    fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
        let inner = constraints.shrink(self.padding);

        let child_size = self
            .child
            .as_ref()
            .map(|c| c.measure(inner, ctx))
            .unwrap_or(Vec2::zero());

        let w = (child_size.x + self.padding.h()).max(self.min_width);
        let h = (child_size.y + self.padding.v()).max(self.min_height);
        constraints.constrain(Vec2::new(w, h))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        // Draw background + optional border.
        if self.background.is_some() || self.border.is_some() {
            let paint = self
                .background
                .clone()
                .unwrap_or_else(|| Paint::Solid(Color::transparent()));
            painter.fill_rounded_rect(rect, self.corner_radius, paint, self.border.clone());
        }

        // Paint child inside the padded inner rect.
        if let Some(child) = &self.child {
            child.paint(painter, inset_rect(rect, self.padding));
        }
    }

    fn on_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        if let Some(child) = &mut self.child {
            child.on_event(event, inset_rect(rect, self.padding))
        } else {
            EventResult::Ignored
        }
    }
}
