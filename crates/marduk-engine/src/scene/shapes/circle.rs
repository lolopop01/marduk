use crate::coords::Vec2;
use crate::paint::{Color, Paint};
use crate::scene::{DrawCmd, DrawList, ZIndex};

use super::Border;

/// Circle draw payload.
#[derive(Debug, Clone, PartialEq)]
pub struct CircleCmd {
    pub center: Vec2,
    pub radius: f32,
    pub paint: Paint,
    pub border: Option<Border>,
}

impl CircleCmd {
    #[inline]
    pub fn new(center: Vec2, radius: f32, paint: Paint, border: Option<Border>) -> Self {
        Self { center, radius, paint, border }
    }
}

impl DrawList {
    /// Records a circle draw command.
    #[inline]
    pub fn push_circle(
        &mut self,
        z: ZIndex,
        center: Vec2,
        radius: f32,
        paint: Paint,
        border: Option<Border>,
    ) {
        self.push(z, DrawCmd::Circle(CircleCmd::new(center, radius, paint, border)));
    }

    /// Records a solid circle.
    #[inline]
    pub fn push_solid_circle(&mut self, z: ZIndex, center: Vec2, radius: f32, color: Color) {
        self.push_circle(z, center, radius, Paint::Solid(color), None);
    }
}
