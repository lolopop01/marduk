package com.marduk.lsp

/** Embedded widget/property registry — mirrors knowledge.rs in the LSP crate. */
object MardukKnowledge {

    val WIDGETS = listOf(
        "Text", "Container", "Column", "Row", "Button",
        "Checkbox", "Toggle", "Slider", "RadioGroup", "RadioOption",
        "ProgressBar", "TextBox", "ScrollView", "Stack",
    )

    // Properties available on every widget when placed inside a Stack.
    private val ANCHOR_PROPS = listOf("top", "bottom", "left", "right", "width", "height")

    private val PADDING_PROPS = listOf(
        "padding", "padding_top", "padding_right", "padding_bottom", "padding_left",
    )

    private val BORDER_PROPS = listOf("border_width", "border_color")

    val PROPS: Map<String, List<String>> = mapOf(
        "Text" to listOf("font", "size", "color") + ANCHOR_PROPS,
        "Container" to listOf("bg", "corner_radius") + BORDER_PROPS + PADDING_PROPS + ANCHOR_PROPS,
        "Column" to listOf("gap", "align", "bg") + PADDING_PROPS + ANCHOR_PROPS,
        "Row" to listOf("gap", "align", "bg") + PADDING_PROPS + ANCHOR_PROPS,
        "Button" to listOf(
            "bg", "hover_bg", "press_bg", "text_color",
            "font", "font_size", "corner_radius", "on_click",
        ) + BORDER_PROPS + PADDING_PROPS + ANCHOR_PROPS,
        "Checkbox" to listOf("value", "on_change") + ANCHOR_PROPS,
        "Toggle" to listOf("value", "on_change") + ANCHOR_PROPS,
        "Slider" to listOf("min", "max", "value", "on_change", "on_drag") + ANCHOR_PROPS,
        "RadioGroup" to listOf("value", "on_change") + ANCHOR_PROPS,
        "RadioOption" to listOf("value") + ANCHOR_PROPS,
        "ProgressBar" to listOf("value", "min", "max", "fg", "bg") + ANCHOR_PROPS,
        "TextBox" to listOf("placeholder", "value", "on_change", "font", "size", "color") + ANCHOR_PROPS,
        "ScrollView" to listOf("line_height", "on_scroll") + ANCHOR_PROPS,
        "Stack" to listOf("bg") + ANCHOR_PROPS,
    )

    fun propsFor(widget: String): List<String> = PROPS[widget] ?: ANCHOR_PROPS
}
