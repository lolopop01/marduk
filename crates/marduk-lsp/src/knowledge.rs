//! Static knowledge base: every widget the builder knows, and every property
//! each widget accepts.  This drives hover documentation and completion.

#![allow(dead_code)] // fields / statics reserved for future LSP features

// ── Property kinds ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    /// Numeric literal (`16`, `1.5`, …).
    Number,
    /// Color literal (`#rrggbbaa`).
    Color,
    /// Boolean expressed as `0` or `1`.
    Bool,
    /// One of a fixed set of identifier strings.
    Enum(&'static [&'static str]),
    /// Any identifier or string: names an application event.
    Event,
    /// Any identifier: names a font registered in the bindings.
    Font,
}

// ── Property info ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct PropInfo {
    pub name: &'static str,
    pub kind: PropKind,
    pub doc:  &'static str,
}

// ── Widget info ───────────────────────────────────────────────────────────

pub struct WidgetInfo {
    pub name:         &'static str,
    pub doc:          &'static str,
    /// Whether the widget accepts an inline string: `Text "Hello"`.
    pub has_content:  bool,
    /// Whether the widget accepts child widget blocks.
    pub has_children: bool,
    pub props:        &'static [PropInfo],
}

// ── Shared prop sets (referenced by multiple widgets) ─────────────────────

const PADDING_PROPS: &[PropInfo] = &[
    PropInfo { name: "padding",        kind: PropKind::Number, doc: "Uniform padding on all sides (logical px)." },
    PropInfo { name: "padding_top",    kind: PropKind::Number, doc: "Top padding (logical px)." },
    PropInfo { name: "padding_right",  kind: PropKind::Number, doc: "Right padding (logical px)." },
    PropInfo { name: "padding_bottom", kind: PropKind::Number, doc: "Bottom padding (logical px)." },
    PropInfo { name: "padding_left",   kind: PropKind::Number, doc: "Left padding (logical px)." },
];

const BORDER_PROPS: &[PropInfo] = &[
    PropInfo { name: "border_width", kind: PropKind::Number, doc: "Border stroke width (logical px)." },
    PropInfo { name: "border_color", kind: PropKind::Color,  doc: "Border stroke color (`#rrggbbaa`)." },
];

const STATE_PROPS: &[PropInfo] = &[
    PropInfo { name: "state_key", kind: PropKind::Event, doc: "Key used to persist this widget's value across frame rebuilds." },
    PropInfo { name: "on_change", kind: PropKind::Event, doc: "Event fired when the value changes. Also used as the state key if `state_key` is absent." },
];

const FONT_PROPS: &[PropInfo] = &[
    PropInfo { name: "font",      kind: PropKind::Font,   doc: "Font name (must be registered in `DslBindings`)." },
    PropInfo { name: "font_size", kind: PropKind::Number, doc: "Font size in logical pixels." },
];

// ── Widget registry ───────────────────────────────────────────────────────

pub static WIDGETS: &[WidgetInfo] = &[
    // ── Text ──────────────────────────────────────────────────────────────
    WidgetInfo {
        name: "Text",
        doc: "Renders a single line of text.\n\nInline content is the displayed string: `Text \"Hello\"`.",
        has_content: true,
        has_children: false,
        props: &[
            PropInfo { name: "font",  kind: PropKind::Font,   doc: "Font name." },
            PropInfo { name: "size",  kind: PropKind::Number, doc: "Font size in logical pixels." },
            PropInfo { name: "color", kind: PropKind::Color,  doc: "Text color (`#rrggbbaa`)." },
        ],
    },

    // ── Container ─────────────────────────────────────────────────────────
    WidgetInfo {
        name: "Container",
        doc: "A box that holds one child widget.\n\nSupports padding, background, border, and rounded corners.",
        has_content: false,
        has_children: true,
        props: &[
            PropInfo { name: "bg",            kind: PropKind::Color,  doc: "Background color (`#rrggbbaa`)." },
            PropInfo { name: "corner_radius",  kind: PropKind::Number, doc: "Corner radius in logical pixels." },
            PADDING_PROPS[0], PADDING_PROPS[1], PADDING_PROPS[2], PADDING_PROPS[3], PADDING_PROPS[4],
            BORDER_PROPS[0],  BORDER_PROPS[1],
        ],
    },

    // ── Column ────────────────────────────────────────────────────────────
    WidgetInfo {
        name: "Column",
        doc: "Arranges children vertically.\n\nWrap in a `Container` (or use `bg`) for backgrounds and borders.",
        has_content: false,
        has_children: true,
        props: &[
            PropInfo { name: "gap",     kind: PropKind::Number, doc: "Space between children (logical px). Alias: `spacing`." },
            PropInfo { name: "spacing", kind: PropKind::Number, doc: "Space between children (logical px). Prefer `gap`." },
            PropInfo { name: "align",   kind: PropKind::Enum(&["start", "center", "end", "stretch"]), doc: "Cross-axis alignment of children." },
            PropInfo { name: "bg",      kind: PropKind::Color,  doc: "Background color — wraps column in a Container automatically." },
            PropInfo { name: "corner_radius", kind: PropKind::Number, doc: "Corner radius (requires `bg`)." },
            PADDING_PROPS[0], PADDING_PROPS[1], PADDING_PROPS[2], PADDING_PROPS[3], PADDING_PROPS[4],
            BORDER_PROPS[0],  BORDER_PROPS[1],
        ],
    },

    // ── Row ───────────────────────────────────────────────────────────────
    WidgetInfo {
        name: "Row",
        doc: "Arranges children horizontally.",
        has_content: false,
        has_children: true,
        props: &[
            PropInfo { name: "gap",     kind: PropKind::Number, doc: "Space between children (logical px). Alias: `spacing`." },
            PropInfo { name: "spacing", kind: PropKind::Number, doc: "Space between children (logical px). Prefer `gap`." },
            PropInfo { name: "align",   kind: PropKind::Enum(&["start", "center", "end", "stretch"]), doc: "Cross-axis alignment of children." },
            PropInfo { name: "bg",      kind: PropKind::Color,  doc: "Background color — wraps row in a Container automatically." },
            PropInfo { name: "corner_radius", kind: PropKind::Number, doc: "Corner radius (requires `bg`)." },
            PADDING_PROPS[0], PADDING_PROPS[1], PADDING_PROPS[2], PADDING_PROPS[3], PADDING_PROPS[4],
            BORDER_PROPS[0],  BORDER_PROPS[1],
        ],
    },

    // ── Button ────────────────────────────────────────────────────────────
    WidgetInfo {
        name: "Button",
        doc: "A pressable button.\n\nInline content is the label text. Or nest a child widget for a custom layout.",
        has_content: true,
        has_children: true,
        props: &[
            PropInfo { name: "on_click",    kind: PropKind::Event,  doc: "Event fired on click." },
            PropInfo { name: "bg",          kind: PropKind::Color,  doc: "Default background color." },
            PropInfo { name: "hover_bg",    kind: PropKind::Color,  doc: "Background color while hovered." },
            PropInfo { name: "press_bg",    kind: PropKind::Color,  doc: "Background color while pressed." },
            PropInfo { name: "text_color",  kind: PropKind::Color,  doc: "Label text color." },
            PropInfo { name: "corner_radius", kind: PropKind::Number, doc: "Corner radius in logical pixels." },
            FONT_PROPS[0], FONT_PROPS[1],
            PADDING_PROPS[0], PADDING_PROPS[1], PADDING_PROPS[2], PADDING_PROPS[3], PADDING_PROPS[4],
            BORDER_PROPS[0], BORDER_PROPS[1],
        ],
    },

    // ── Checkbox ──────────────────────────────────────────────────────────
    WidgetInfo {
        name: "Checkbox",
        doc: "A labelled checkbox.\n\nInline content is the label text.",
        has_content: true,
        has_children: false,
        props: &[
            PropInfo { name: "checked",       kind: PropKind::Bool,  doc: "Initial checked state (`0` or `1`)." },
            PropInfo { name: "label_color",   kind: PropKind::Color, doc: "Label text color. Alias: `color`." },
            PropInfo { name: "color",         kind: PropKind::Color, doc: "Label text color. Prefer `label_color`." },
            PropInfo { name: "checked_color", kind: PropKind::Color, doc: "Fill color when checked. Alias: `accent`." },
            PropInfo { name: "accent",        kind: PropKind::Color, doc: "Accent color (checked fill). Prefer `checked_color`." },
            PropInfo { name: "box_size",      kind: PropKind::Number, doc: "Size of the checkbox square (logical px)." },
            PropInfo { name: "corner_radius", kind: PropKind::Number, doc: "Corner radius of the checkbox box." },
            PropInfo { name: "border_color",  kind: PropKind::Color,  doc: "Border color of the checkbox box." },
            FONT_PROPS[0], FONT_PROPS[1],
            STATE_PROPS[0], STATE_PROPS[1],
        ],
    },

    // ── Toggle ────────────────────────────────────────────────────────────
    WidgetInfo {
        name: "Toggle",
        doc: "An on/off toggle switch.",
        has_content: false,
        has_children: false,
        props: &[
            PropInfo { name: "checked",     kind: PropKind::Bool,   doc: "Initial state (`0` or `1`)." },
            PropInfo { name: "width",       kind: PropKind::Number, doc: "Toggle track width (logical px)." },
            PropInfo { name: "height",      kind: PropKind::Number, doc: "Toggle track height (logical px)." },
            PropInfo { name: "on_color",    kind: PropKind::Color,  doc: "Track color when on." },
            PropInfo { name: "off_color",   kind: PropKind::Color,  doc: "Track color when off." },
            PropInfo { name: "thumb_color", kind: PropKind::Color,  doc: "Thumb (knob) color." },
            STATE_PROPS[0], STATE_PROPS[1],
        ],
    },

    // ── Slider ────────────────────────────────────────────────────────────
    WidgetInfo {
        name: "Slider",
        doc: "A horizontal range slider.",
        has_content: false,
        has_children: false,
        props: &[
            PropInfo { name: "min",          kind: PropKind::Number, doc: "Minimum value." },
            PropInfo { name: "max",          kind: PropKind::Number, doc: "Maximum value." },
            PropInfo { name: "value",        kind: PropKind::Number, doc: "Initial value (clamped to [min, max])." },
            PropInfo { name: "track_height", kind: PropKind::Number, doc: "Track bar height (logical px)." },
            PropInfo { name: "thumb_radius", kind: PropKind::Number, doc: "Thumb circle radius (logical px)." },
            PropInfo { name: "track_color",  kind: PropKind::Color,  doc: "Track background color." },
            PropInfo { name: "fill_color",   kind: PropKind::Color,  doc: "Fill color left of the thumb. Alias: `accent`." },
            PropInfo { name: "accent",       kind: PropKind::Color,  doc: "Accent color (fill). Prefer `fill_color`." },
            PropInfo { name: "thumb_color",  kind: PropKind::Color,  doc: "Thumb color." },
            PropInfo { name: "corner_radius",kind: PropKind::Number, doc: "Track corner radius." },
            STATE_PROPS[0], STATE_PROPS[1],
        ],
    },

    // ── RadioGroup ────────────────────────────────────────────────────────
    WidgetInfo {
        name: "RadioGroup",
        doc: "A group of mutually-exclusive radio buttons.\n\nChildren must be `RadioOption` nodes.",
        has_content: false,
        has_children: true,
        props: &[
            PropInfo { name: "default",      kind: PropKind::Event,  doc: "Initially selected option value." },
            PropInfo { name: "selected",     kind: PropKind::Event,  doc: "Initially selected option value. Prefer `default`." },
            PropInfo { name: "label_color",  kind: PropKind::Color,  doc: "Option label color. Alias: `color`." },
            PropInfo { name: "color",        kind: PropKind::Color,  doc: "Option label color. Prefer `label_color`." },
            PropInfo { name: "accent",       kind: PropKind::Color,  doc: "Selected option fill color." },
            PropInfo { name: "border_color", kind: PropKind::Color,  doc: "Radio button border color." },
            PropInfo { name: "dot_radius",   kind: PropKind::Number, doc: "Inner dot radius (logical px)." },
            PropInfo { name: "item_gap",     kind: PropKind::Number, doc: "Vertical gap between options (logical px)." },
            FONT_PROPS[0], FONT_PROPS[1],
            STATE_PROPS[0], STATE_PROPS[1],
        ],
    },

    // ── RadioOption ───────────────────────────────────────────────────────
    WidgetInfo {
        name: "RadioOption",
        doc: "A single option inside a `RadioGroup`.\n\nInline content is the display label.",
        has_content: true,
        has_children: false,
        props: &[
            PropInfo { name: "value", kind: PropKind::Event, doc: "The string value this option represents." },
        ],
    },

    // ── ProgressBar ───────────────────────────────────────────────────────
    WidgetInfo {
        name: "ProgressBar",
        doc: "A non-interactive horizontal progress bar.",
        has_content: false,
        has_children: false,
        props: &[
            PropInfo { name: "value",        kind: PropKind::Number, doc: "Fill fraction in [0, 1]." },
            PropInfo { name: "height",       kind: PropKind::Number, doc: "Bar height (logical px)." },
            PropInfo { name: "track_color",  kind: PropKind::Color,  doc: "Track background color." },
            PropInfo { name: "fill_color",   kind: PropKind::Color,  doc: "Fill color. Alias: `accent`." },
            PropInfo { name: "accent",       kind: PropKind::Color,  doc: "Fill color. Prefer `fill_color`." },
            PropInfo { name: "corner_radius",kind: PropKind::Number, doc: "Track corner radius." },
        ],
    },

    // ── TextBox ───────────────────────────────────────────────────────────
    WidgetInfo {
        name: "TextBox",
        doc: "A single-line text input field.\n\nClick to focus, type to edit, Backspace to delete.",
        has_content: true,
        has_children: false,
        props: &[
            PropInfo { name: "placeholder",          kind: PropKind::Event,  doc: "Placeholder text shown when the field is empty." },
            PropInfo { name: "text",                 kind: PropKind::Event,  doc: "Initial text content." },
            PropInfo { name: "text_color",           kind: PropKind::Color,  doc: "Input text color. Alias: `color`." },
            PropInfo { name: "color",                kind: PropKind::Color,  doc: "Input text color. Prefer `text_color`." },
            PropInfo { name: "placeholder_color",    kind: PropKind::Color,  doc: "Placeholder text color." },
            PropInfo { name: "bg",                   kind: PropKind::Color,  doc: "Background color when unfocused." },
            PropInfo { name: "focused_bg",           kind: PropKind::Color,  doc: "Background color when focused." },
            PropInfo { name: "border_color",         kind: PropKind::Color,  doc: "Border color when unfocused." },
            PropInfo { name: "focused_border_color", kind: PropKind::Color,  doc: "Border color when focused. Alias: `accent`." },
            PropInfo { name: "accent",               kind: PropKind::Color,  doc: "Focused border color. Prefer `focused_border_color`." },
            PropInfo { name: "corner_radius",        kind: PropKind::Number, doc: "Corner radius." },
            PropInfo { name: "on_submit",            kind: PropKind::Event,  doc: "Event fired when the user presses Enter." },
            FONT_PROPS[0], FONT_PROPS[1],
            PADDING_PROPS[0],
            STATE_PROPS[0], STATE_PROPS[1],
        ],
    },

    // ── ScrollView ────────────────────────────────────────────────────────
    WidgetInfo {
        name: "ScrollView",
        doc: "A scrollable container with a single child.\n\nSupports mouse wheel and a thin scrollbar.",
        has_content: false,
        has_children: true,
        props: &[
            PropInfo { name: "line_height",   kind: PropKind::Number, doc: "Scroll distance per mouse-wheel tick (logical px)." },
            PropInfo { name: "show_scrollbar",kind: PropKind::Bool,   doc: "Whether to draw the scrollbar (`0` or `1`)." },
            PropInfo { name: "offset",        kind: PropKind::Number, doc: "Initial scroll offset (logical px from top)." },
            PropInfo { name: "on_scroll",     kind: PropKind::Event,  doc: "Event fired when the scroll offset changes." },
            STATE_PROPS[0],
        ],
    },

    // ── Stack ─────────────────────────────────────────────────────────────
    WidgetInfo {
        name: "Stack",
        doc: "Overlays children on top of each other.\n\nEach child can be anchored with `left`, `top`, `right`, `bottom`, `width`, `height`.",
        has_content: false,
        has_children: true,
        props: &[
            PropInfo { name: "width",  kind: PropKind::Enum(&["fill", "natural"]), doc: "Stack width hint: `fill` (default) or `natural`." },
            PropInfo { name: "height", kind: PropKind::Enum(&["fill", "natural"]), doc: "Stack height hint: `fill` (default) or `natural`." },
            PropInfo { name: "bg",     kind: PropKind::Color, doc: "Background color." },
        ],
    },
];

// ── Lookup helpers ────────────────────────────────────────────────────────

pub fn widget_by_name(name: &str) -> Option<&'static WidgetInfo> {
    WIDGETS.iter().find(|w| w.name == name)
}

pub fn prop_in_widget(widget: &str, prop: &str) -> Option<&'static PropInfo> {
    widget_by_name(widget)?.props.iter().find(|p| p.name == prop)
}

/// Properties valid on Stack child items (anchor/size positioning).
pub static STACK_CHILD_PROPS: &[PropInfo] = &[
    PropInfo { name: "left",   kind: PropKind::Number, doc: "Distance from the Stack's left edge (logical px)." },
    PropInfo { name: "top",    kind: PropKind::Number, doc: "Distance from the Stack's top edge (logical px)." },
    PropInfo { name: "right",  kind: PropKind::Number, doc: "Distance from the Stack's right edge (logical px)." },
    PropInfo { name: "bottom", kind: PropKind::Number, doc: "Distance from the Stack's bottom edge (logical px)." },
    PropInfo { name: "width",  kind: PropKind::Enum(&["fill", "natural"]), doc: "Child width hint." },
    PropInfo { name: "height", kind: PropKind::Enum(&["fill", "natural"]), doc: "Child height hint." },
];
