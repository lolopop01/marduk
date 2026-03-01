//! Marduk UI — retained widget tree on top of `marduk-engine`.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use marduk_ui::prelude::*;
//!
//! let mut scene = UiScene::new();
//! let font = scene.load_font(include_bytes!("my_font.ttf")).unwrap();
//!
//! // In your frame callback:
//! let input = UiInput { mouse_pos, mouse_clicked, mouse_pressed };
//! let draw_list = scene.frame(
//!     Column::new()
//!         .child(Text::new("Hello!", font, 18.0, Color::from_straight(1.0, 1.0, 1.0, 1.0)))
//!         .child(Button::new(Text::new("Click me", font, 14.0, Color::from_straight(0.0, 0.0, 0.0, 1.0)))
//!             .on_click(|| println!("clicked!"))),
//!     viewport,
//!     &input,
//! );
//! // Pass draw_list to your renderers.
//! ```
//!
//! # Extending with custom widgets
//!
//! Implement [`Widget`] for any type, then use it anywhere an [`Element`] is accepted:
//!
//! ```rust,ignore
//! use marduk_ui::prelude::*;
//!
//! pub struct MyWidget { /* your fields */ }
//!
//! impl Widget for MyWidget {
//!     fn measure(&self, constraints: Constraints, ctx: &LayoutCtx) -> Vec2 {
//!         Vec2::new(100.0, 40.0)
//!     }
//!     fn paint(&self, painter: &mut Painter, rect: Rect) {
//!         painter.fill_rounded_rect(rect, 4.0, Paint::Solid(Color::from_straight(0.2, 0.5, 1.0, 1.0)), None);
//!     }
//! }
//! ```

pub mod app;
pub mod constraints;

// Top-level re-exports for the common entry point — `use marduk_ui::Application`
pub use app::Application;
pub use marduk_engine::window::WindowMode;
pub mod dsl;
pub mod event;
pub mod painter;
pub mod scene;
pub mod widget;
pub mod widgets;

/// Everything you need to build and extend UI — import this in your component files.
pub mod prelude {
    pub use crate::constraints::{Constraints, Edges, LayoutCtx};
    pub use crate::event::{EventResult, UiEvent};
    pub use crate::painter::Painter;
    pub use crate::scene::{UiInput, UiScene};
    pub use crate::widget::{Element, Widget};
    pub use crate::widgets::{
        button::Button,
        checkbox::Checkbox,
        container::Container,
        flex::{Align, Column, Row},
        progress::ProgressBar,
        radio::{RadioGroup, RadioOption},
        scroll::ScrollView,
        slider::Slider,
        stack::{AnchorVal, SizeHint, Stack, StackItem},
        text::Text,
        textbox::TextBox,
        toggle::Toggle,
    };

    // Re-export the engine primitives everyone needs.
    pub use marduk_engine::coords::{CornerRadii, Rect, Vec2};
    pub use marduk_engine::paint::{Color, ColorStop, LinearGradient, Paint, SpreadMode};
    pub use marduk_engine::scene::Border;
    pub use marduk_engine::text::FontId;

    // DSL
    pub use crate::dsl::{DslBindings, DslDocument, DslLoader, ParseError};

    // Application (entry point for end-user apps)
    pub use crate::app::{Application, FontMap, WidgetState};
}
