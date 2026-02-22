use crate::coords::Rect;
use crate::paint::{Color, Paint};
use crate::scene::{DrawCmd, DrawList, ZIndex};

/// Rectangle draw payload.
#[derive(Debug, Clone, PartialEq)]
pub struct RectCmd {
    pub rect: Rect,
    pub paint: Paint,
}

impl RectCmd {
    #[inline]
    pub fn new(rect: Rect, paint: Paint) -> Self {
        Self { rect, paint }
    }
}

impl DrawList {
    /// Records a rectangle draw command.
    #[inline]
    pub fn push_rect(&mut self, z: ZIndex, rect: Rect, paint: Paint) {
        self.push(z, DrawCmd::Rect(RectCmd::new(rect, paint)));
    }

    /// Records a solid rectangle draw command.
    #[inline]
    pub fn push_solid_rect(&mut self, z: ZIndex, rect: Rect, color: Color) {
        self.push_rect(z, rect, Paint::Solid(color));
    }
}