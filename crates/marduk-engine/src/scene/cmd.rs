use crate::scene::shapes::rect::RectCmd;

/// Renderer-agnostic draw command stream.
///
/// Extending the scene:
/// - add a new shape module under `scene::shapes::*`
/// - add a new variant here
/// - implement push helpers inside that shape module
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCmd {
    Rect(RectCmd),

    // Future:
    // RoundedRect(RoundedRectCmd),
    // Circle(CircleCmd),
    // TextRun(TextRunCmd),
    // Image(ImageCmd),
    // ClipPush(ClipCmd),
    // ClipPop,
    // OpacityPush(OpacityCmd),
    // OpacityPop,
}