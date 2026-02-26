use crate::coords::Vec2;
use crate::paint::Color;
use crate::scene::{DrawCmd, DrawList, ZIndex};
use crate::text::FontId;

/// Text draw payload.
#[derive(Debug, Clone, PartialEq)]
pub struct TextCmd {
    pub text: String,
    pub font: FontId,
    /// Font size in logical pixels.
    pub size: f32,
    pub color: Color,
    /// Top-left of the text block in logical pixels.
    pub origin: Vec2,
    /// Wrapping width in logical pixels. `None` = no wrapping.
    pub max_width: Option<f32>,
}

impl DrawList {
    /// Records a text draw command.
    pub fn push_text(
        &mut self,
        z: ZIndex,
        text: impl Into<String>,
        font: FontId,
        size: f32,
        color: Color,
        origin: Vec2,
        max_width: Option<f32>,
    ) {
        self.push(z, DrawCmd::Text(TextCmd {
            text: text.into(),
            font,
            size,
            color,
            origin,
            max_width,
        }));
    }
}
