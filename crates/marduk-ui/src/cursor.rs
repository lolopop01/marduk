/// The shape of the OS mouse cursor.
///
/// Pass to [`Painter::set_cursor`] during a widget's `paint()` call to
/// change the cursor while the mouse is over that widget.
///
/// The default value is [`Default`] (the standard arrow pointer).
///
/// # Example
/// ```rust,ignore
/// fn paint(&self, painter: &mut Painter, rect: Rect) {
///     if painter.is_hovered(rect) {
///         painter.set_cursor(CursorIcon::Pointer);
///     }
///     // ...
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorIcon {
    /// Standard arrow pointer (default).
    #[default]
    Default,
    /// Text insertion bar (I-beam). Use over text input areas.
    Text,
    /// Hand pointer. Use over clickable elements (links, buttons).
    Pointer,
    /// Horizontal resize arrow (↔). Use on vertical splitter handles.
    EwResize,
    /// Vertical resize arrow (↕). Use on horizontal splitter handles.
    NsResize,
    /// Crosshair. Use for precision selection or drawing tools.
    Crosshair,
    /// Prohibited indicator. Use when an action is not allowed.
    NotAllowed,
    /// Open hand. Use to indicate drag-to-scroll is available.
    Grab,
    /// Closed hand. Use while drag-scrolling is active.
    Grabbing,
}

impl From<CursorIcon> for winit::window::CursorIcon {
    fn from(c: CursorIcon) -> Self {
        match c {
            CursorIcon::Default    => winit::window::CursorIcon::Default,
            CursorIcon::Text       => winit::window::CursorIcon::Text,
            CursorIcon::Pointer    => winit::window::CursorIcon::Pointer,
            CursorIcon::EwResize   => winit::window::CursorIcon::EwResize,
            CursorIcon::NsResize   => winit::window::CursorIcon::NsResize,
            CursorIcon::Crosshair  => winit::window::CursorIcon::Crosshair,
            CursorIcon::NotAllowed => winit::window::CursorIcon::NotAllowed,
            CursorIcon::Grab       => winit::window::CursorIcon::Grab,
            CursorIcon::Grabbing   => winit::window::CursorIcon::Grabbing,
        }
    }
}
