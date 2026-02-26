use crate::coords::{CornerRadii, Rect};
use crate::paint::{Color, Paint};
use crate::scene::{DrawCmd, DrawList, ZIndex};

use super::Border;

/// Rounded rectangle draw payload.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundedRectCmd {
    pub rect: Rect,
    pub radii: CornerRadii,
    pub paint: Paint,
    pub border: Option<Border>,
}

impl RoundedRectCmd {
    #[inline]
    pub fn new(rect: Rect, radii: CornerRadii, paint: Paint, border: Option<Border>) -> Self {
        Self { rect, radii, paint, border }
    }
}

impl DrawList {
    /// Records a rounded rectangle draw command.
    #[inline]
    pub fn push_rounded_rect(
        &mut self,
        z: ZIndex,
        rect: Rect,
        radii: CornerRadii,
        paint: Paint,
        border: Option<Border>,
    ) {
        self.push(z, DrawCmd::RoundedRect(RoundedRectCmd::new(rect, radii, paint, border)));
    }

    /// Records a solid rounded rectangle with uniform corner radius.
    #[inline]
    pub fn push_solid_rounded_rect(&mut self, z: ZIndex, rect: Rect, radius: f32, color: Color) {
        self.push_rounded_rect(z, rect, CornerRadii::all(radius), Paint::Solid(color), None);
    }
}
