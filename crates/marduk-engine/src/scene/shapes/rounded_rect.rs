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
        let scale  = self.current_transform().1;
        let rect   = self.tx_rect(rect);
        let radii  = CornerRadii {
            top_left:     radii.top_left     * scale,
            top_right:    radii.top_right    * scale,
            bottom_right: radii.bottom_right * scale,
            bottom_left:  radii.bottom_left  * scale,
        };
        let paint  = self.tx_paint(paint);
        let border = self.tx_border(border);
        self.push(z, DrawCmd::RoundedRect(RoundedRectCmd::new(rect, radii, paint, border)));
    }

    /// Records a solid rounded rectangle with uniform corner radius.
    #[inline]
    pub fn push_solid_rounded_rect(&mut self, z: ZIndex, rect: Rect, radius: f32, color: Color) {
        self.push_rounded_rect(z, rect, CornerRadii::all(radius), Paint::Solid(color), None);
    }
}
