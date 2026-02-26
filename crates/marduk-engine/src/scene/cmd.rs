use crate::scene::shapes::circle::CircleCmd;
use crate::scene::shapes::rect::RectCmd;
use crate::scene::shapes::rounded_rect::RoundedRectCmd;
use crate::scene::shapes::text::TextCmd;

/// Renderer-agnostic draw command stream.
///
/// Extending the scene:
/// - add a new shape module under `scene::shapes::*`
/// - add a new variant here
/// - implement push helpers inside that shape module
/// - add a matching renderer under `render::shapes::*`
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCmd {
    Rect(RectCmd),
    RoundedRect(RoundedRectCmd),
    Circle(CircleCmd),
    Text(TextCmd),
}
