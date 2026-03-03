# Marduk UI — Production Roadmap

This document is a living spec for everything that needs to happen before marduk-ui
is suitable for real applications. Items within each section are loosely ordered by
dependency: fix the foundations before adding surface-level features.

---

## Section 1 — API Stabilization

This section covers things that are broken, inconsistent, or likely to cause hard-to-
debug problems for someone building an app. Nothing here is a new feature; it is all
making existing things solid.

---

### 1.1 Naming consistency pass

**Problem.** The DSL and Rust API disagree on property names, and a user who reads
the Rust docs and then writes `.mkml` files will be constantly surprised.

| Thing | Rust API | DSL today | Target DSL |
|---|---|---|---|
| Flex gap | `.spacing(f32)` | `gap:` / `spacing:` (both work) | `gap:` only |
| Cross-axis align | `.cross_align(Align::*)` | `align:` | `align:` |
| Background | `.background(Color)` | `bg:` | `bg:` |
| Corner radius | `.corner_radius(f32)` | `corner_radius:` | `radius:` |
| Padding | `.padding_all(f32)` | `padding:` | `padding:` |

**Tasks.**
- [ ] Audit every property name in `dsl/builder.rs` against the Rust builder.
- [ ] Pick one canonical name per property; add aliases for the other as deprecated
      (warn in debug builds, remove in v0.2).
- [ ] Document canonical names in a single table in `CLAUDE.md`.

---

### 1.2 Error handling audit — eliminate panics

**Problem.** Several code paths unconditionally panic in production conditions.

| Location | Issue |
|---|---|
| `DrawList::pop_clip()` | `expect("clip stack underflow")` — panics on unbalanced calls |
| `DslLoader::build_widget()` | Unknown widget name silently dropped or panics |
| `TextRenderer` glyph upload | Various `unwrap()` on atlas allocation |
| `FontSystem::load_font()` | Error returned as `Err(&'static str)` — callers ignore it |
| `arboard::Clipboard::new()` | Silent failure; clipboard ops become no-ops without notice |

**Tasks.**
- [ ] Replace `pop_clip` panic with a debug-mode assertion; no-op in release.
- [ ] `DslLoader::build_widget` unknown name → render a red placeholder box labeled
      with the widget name (visible in dev, configurable).
- [ ] Wrap `FontSystem::load_font` return in a real error type
      (`FontLoadError { path, reason }`).
- [ ] Add `Application::on_error(impl Fn(MardukError))` hook for runtime errors.
- [ ] Audit every `.unwrap()` and `.expect()` in `marduk-ui` and `marduk-engine`;
      replace with `?`-propagation or logged fallback.

---

### 1.3 `ParseError` — add source location

**Problem.** When a `.mkml` file has a syntax error, the parser returns a
`ParseError` with no line or column number. Users have no idea where the error is.
The LSP can show errors in the editor but the runtime error message is useless.

**Tasks.**
- [ ] Add `line: usize, col: usize` fields to `ParseError`.
- [ ] Lexer tracks line/col in `Lexer::pos`; propagate into `ParseError` on failure.
- [ ] Update LSP `backend.rs` diagnostic range to use the new fields (currently
      hardcodes range to `(0,0)–(0,1)` for all errors).
- [ ] Add a `Display` impl that prints `file.mkml:12:5: unexpected token '{'`.

---

### 1.4 Color space — single source of truth

**Problem.** Two color representations exist simultaneously:

- DSL/AST: `[u8; 4]` straight-alpha RGBA bytes (`#rrggbbaa`)
- Engine: `Color { r, g, b, a: f32 }` premultiplied linear

The conversion happens ad-hoc in `builder.rs::engine_color()`. This is invisible to
users writing Rust widgets and creates a footgun (passing a straight-alpha color
directly to the engine gives wrong results silently).

**Tasks.**
- [ ] Add `Color::from_srgb_u8(r, g, b, a) -> Color` (straight sRGB bytes → premul
      linear) and deprecate `from_straight` (keep as alias).
- [ ] Rename the confusing `from_straight` to `from_srgb` for clarity.
- [ ] In debug builds, add a `Color::debug_assert_premul()` that warns if any
      channel exceeds alpha (a sign that straight-alpha was passed by mistake).
- [ ] Document the color pipeline in `CLAUDE.md` once and link to it everywhere.

---

### 1.5 Layout — eliminate triple-measure per frame

**Problem.** `Column` and `Row` measure their children in `measure()`, `paint()`,
and `on_event()` — three separate passes per frame. For a column of 50 widgets this
is 150 measure calls per frame.

**Tasks.**
- [ ] Introduce a `LayoutResult` struct: `Vec<(Rect, usize)>` — child rect and index.
- [ ] `Column::layout(constraints, &LayoutCtx) -> LayoutResult` — call this once.
- [ ] Store `LayoutResult` in a `Cell<Option<LayoutResult>>` (interior mutability)
      so `paint` and `on_event` can reuse the last compute without borrowing.
- [ ] Mark the cache as stale at frame start (a boolean flag on `UiScene`).
- [ ] Same treatment for `Row` and `Stack`.

---

### 1.6 Focus management

**Problem.** There is no concept of keyboard focus. Tab key navigation doesn't exist.
All widgets receive all `KeyPress` events simultaneously. `TextBox` hacks around this
with an internal `focused: bool` field, but there's no global focus registry.

**Tasks.**
- [ ] Add `FocusId: usize` type (auto-increment per widget that opts in).
- [ ] `UiScene` owns `focused: Option<FocusId>`.
- [ ] `Painter` exposes `is_focused(id: FocusId) -> bool`.
- [ ] `UiEvent::FocusGained / FocusLost` dispatched to the relevant widget.
- [ ] Tab key cycles through focusable widgets in paint order.
- [ ] DSL: `focusable: true` property opts a widget into the tab cycle.
- [ ] `Application::focus("state_key")` lets the app programmatically set focus.

---

### 1.7 `EventResult` — proper stop-propagation

**Problem.** `EventResult::Consumed` exists but most containers ignore it. A
`TextBox` that consumes a `KeyPress` still delivers it to sibling widgets below.

**Tasks.**
- [ ] `Column::on_event` and `Row::on_event`: break out of the child loop on
      `EventResult::Consumed` for key/text events.
- [ ] `Stack::on_event`: already iterates children in reverse; stop on first Consumed.
- [ ] Document the rule: `Click` and `KeyPress` propagate bottom-up; `Hover` always
      visits all widgets.

---

### 1.8 `TextEditState` — extract to public API

**Problem.** `TextEditState` is the best piece of reusable logic in the codebase but
it is buried in `widgets/text_edit.rs` and not re-exported. Anyone building a
custom multi-line editor or code editor has to reinvent it.

**Tasks.**
- [ ] Move `TextEditState` to `marduk_ui::text_edit::TextEditState` (public module).
- [ ] Add `TextEditState::new(text: impl Into<String>) -> Self`.
- [ ] Add `TextEditState::on_event(&mut self, event: &UiEvent, ...)` — handles all
      keyboard and mouse events in one call; callers just render from state.
- [ ] Re-export via `marduk_ui::prelude::*`.

---

### 1.9 Constraint helpers — fill in missing constructors

**Problem.** Users frequently want "fill available width, fixed height" or "min width
100px, expand to fill" but these require constructing `Constraints` manually.

**Tasks.**
- [ ] `Constraints::fixed(size)` — both min and max are the same (force exact size).
- [ ] `Constraints::at_least(min)` — min set, max = ∞.
- [ ] `Constraints::between(min, max)` — explicit range.
- [ ] `Constraints::fill(available)` — min = max = available (alias for `tight`).
- [ ] Add `SizeHint::FillFraction(f32)` to the DSL stack system so items can take a
      proportional share of remaining space.

---

### 1.10 DSL — consistent widget state key semantics

**Problem.** State keys in the DSL are ad-hoc: some widgets use `state_key:`, some
infer from `on_change:`, and `TextBox` has special cursor persistence via a separate
map. A user can't predict how state will be keyed without reading the source.

**Tasks.**
- [ ] Define a single rule: every stateful widget uses `id:` as its persistence key
      (short, memorable). `state_key:` and `on_change:`-based inference become
      aliases with a deprecation warning.
- [ ] `TextBox`, `Slider`, `Checkbox`, `Toggle`, `RadioGroup`, `ScrollView` all
      honor `id:`.
- [ ] `TextBox` cursor/anchor/scroll persisted under `{id}::cursor` automatically.
- [ ] Document the keying rule in `CLAUDE.md`.

---

## Section 2 — New Rendering Capabilities

These are engine-level additions that unlock new widget types.

---

### 2.1 Image rendering

Without image support, any UI that shows icons, avatars, thumbnails, or illustrations
is impossible. This is the single most blocking missing feature.

**Engine tasks.**
- [ ] `ImageHandle`: opaque GPU texture handle returned by `Scene::load_image(data)`.
- [ ] Supported formats: PNG and JPEG via the `image` crate (feature-gated).
- [ ] `DrawCmd::Image { rect, handle, tint: Color, corner_radii: CornerRadii }`.
- [ ] `ImageRenderer` — renders a GPU texture on a quad; tint multiplied in shader.
- [ ] Texture cache keyed by `ImageHandle` id; LRU eviction when budget exceeded.
- [ ] HiDPI: images uploaded at logical size; sampler uses `FilterMode::Linear`.

**UI tasks.**
- [ ] `Image::new(handle)` widget builder; `.tint(Color)`, `.corner_radius(f32)`,
      `.fit(ImageFit::Contain|Cover|Fill|None)`.
- [ ] `Application::image(name, bytes)` — pre-load at startup, reference by name.
- [ ] DSL: `Image { src: "name"  fit: contain  corner_radius: 8 }`.

---

### 2.2 Shadows

Shadows are required for elevated surfaces (cards, modals, dropdowns).

**Engine tasks.**
- [ ] `Shadow { offset: Vec2, blur: f32, spread: f32, color: Color }`.
- [ ] Add `shadow: Option<Shadow>` field to `RoundedRectCmd` and `RectCmd`.
- [ ] Shader: SDF-based shadow rendered as a separate back-layer quad; Gaussian
      approximation via `exp(-dist² / blur²)`.

**UI tasks.**
- [ ] `Container::shadow(Shadow)` builder.
- [ ] DSL: `shadow: "dx dy blur spread #color"` shorthand string property.

---

### 2.3 Line / stroke rendering

Required for dividers, charts, sparklines, custom underlines.

**Engine tasks.**
- [ ] `LineCmd { start, end, width, color, cap: LineCap }`.
- [ ] `LineCap::Butt | Round | Square`.
- [ ] SDF shader for anti-aliased thick lines.
- [ ] `DrawList::push_line(z, start, end, width, color)`.

**UI tasks.**
- [ ] `Painter::stroke_line(start, end, width, color)`.
- [ ] `Divider::horizontal() / vertical()` widget (thin 1px line).

---

### 2.4 Opacity & transform (2D)

Required for animations and overlay effects.

**Engine tasks.**
- [ ] `DrawCmd::Layer { children: DrawList, opacity: f32, transform: Affine2 }`.
- [ ] `Affine2 { translate, rotate, scale, skew }` — 3×2 matrix.
- [ ] Render layers to an offscreen texture; composite with opacity.
- [ ] `DrawList::push_layer(z, opacity, transform) -> LayerGuard` — RAII layer.

**UI tasks.**
- [ ] `Painter::with_opacity(f32, |painter| { ... })`.
- [ ] `Container::opacity(f32)`, `Container::rotate(radians)`.

---

## Section 3 — New Widgets

Ordered by how often a production app needs them.

---

### 3.1 `Combobox` (dropdown select)

**What it is.** A text field that opens a list of options on click. Supports keyboard
navigation. Optionally supports typed filtering.

**Rust API.**
```rust
Combobox::new()
    .options(vec!["Alpha", "Beta", "Gamma"])
    .selected("Beta")
    .placeholder("Pick one…")
    .on_change(|val| { … })
    .searchable(true)        // shows a filter input in the popup
    .max_height(200.0)       // pixel height of the dropdown list
```

**Behavior.**
- Click → popup opens as an overlay above the rest of the widget tree.
- Popup is drawn in a higher Z layer (via `DrawList::push_layer`).
- Arrow keys navigate options; Enter confirms; Escape cancels.
- Scroll wheel works inside the popup.
- Clicking outside the popup closes it.

**DSL.**
```mkml
Combobox {
    id: my_select
    options: "Alpha,Beta,Gamma"   // comma-separated string
    placeholder: "Pick one…"
    on_change: my_event
    searchable: true
}
```

**Implementation notes.**
- Popup state (open/closed, filter text, highlighted index) stored in `widget_state`.
- Popup rect rendered at a fixed Z above all other widgets.
- Requires the overlay/popup infrastructure (see 3.5).

---

### 3.2 `Tabs`

**What it is.** A tab bar + content area; clicking a tab reveals its panel.

**Rust API.**
```rust
Tabs::new()
    .tab("Files",    file_panel.into())
    .tab("Settings", settings_panel.into())
    .selected(0)
    .on_change(|idx| { … })
    .tab_height(36.0)
    .active_color(Color::from_srgb_u8(100, 150, 255, 255))
```

**DSL.**
```mkml
Tabs {
    id: main_tabs
    tab_height: 36

    Tab "Files" { Column { … } }
    Tab "Settings" { Column { … } }
}
```

**Implementation notes.**
- `Tab` is a DSL-only virtual node; `DslLoader` collapses all `Tab` children into
  a single `Tabs` widget.
- Active tab index in `widget_state` keyed by widget `id`.
- Tab bar is a `Row` of `Button`-like tab headers.
- Tab panel is wrapped in a `Stack` at the same position; only active panel visible.

---

### 3.3 `Tooltip`

**What it is.** A small popup label that appears when the cursor hovers over a widget
for a fixed duration.

**Rust API.**
```rust
Container::new()
    .child(some_widget)
    .tooltip("Saves the current file (Ctrl+S)")
    .tooltip_delay(500)   // milliseconds, default 400
```

**DSL.**
```mkml
Button "Save" {
    on_click: save
    tooltip: "Save file (Ctrl+S)"
}
```

**Implementation notes.**
- Requires time tracking: `UiInput` gains `elapsed_ms: u64` (already available
  via `FrameCtx::time`).
- Tooltip state: `hovered_widget: Option<(FocusId, Instant)>`.
- After `tooltip_delay` ms of hover, render a small rounded rect + text at cursor
  position, in a very high Z layer.
- Any cursor movement or click dismisses the tooltip.

---

### 3.4 `Menu` and `ContextMenu`

**What it is.** A popup list of labeled actions, optionally nested (submenus).

**Rust API.**
```rust
// App menu bar
MenuBar::new()
    .menu("File", |m| m
        .item("New",    "Ctrl+N", || { … })
        .item("Open…",  "Ctrl+O", || { … })
        .separator()
        .item("Quit",   "Ctrl+Q", || std::process::exit(0))
    )
    .menu("Edit", |m| m
        .item("Undo",   "Ctrl+Z", || { … })
    )

// Context menu (right-click)
widget.context_menu(|m| m
    .item("Copy",  || { … })
    .item("Paste", || { … })
)
```

**DSL.**
```mkml
MenuBar {
    Menu "File" {
        MenuItem "New"  { shortcut: "Ctrl+N"  on_click: new_file }
        MenuItem "Quit" { shortcut: "Ctrl+Q"  on_click: quit }
    }
}
```

**Implementation notes.**
- Menus are rendered in the highest Z layer (above everything).
- Right-click on a widget with `context_menu` opens the menu at cursor pos.
- Menu items are keyboard navigable (arrows, Enter, Escape).
- Nested submenus open to the right (or left if near screen edge).
- Requires overlay infrastructure.

---

### 3.5 Overlay/Popup infrastructure

This is a prerequisite for Combobox, Menu, Tooltip, and Modal. Currently there is no
mechanism to render widgets outside the normal layout tree.

**Tasks.**
- [ ] `UiScene` gains an `overlay_list: DrawList` — drawn after the main list with
      `ZIndex::OVERLAY` (a reserved very-high value).
- [ ] `Painter::overlay_painter() -> Painter` — returns a painter that writes to
      `overlay_list`. Any widget can call this.
- [ ] `UiScene::register_overlay(rect, widget_id)` — tells the event system that
      clicks on this rect should route to `widget_id`, not the underlying widget.
- [ ] `UiScene::clear_overlays()` — called at the start of each frame.
- [ ] Global Escape key dismisses the topmost overlay (handled in `UiScene::frame`).

---

### 3.6 `Modal` / `Dialog`

**What it is.** A centered panel that blocks all input behind it.

**Rust API.**
```rust
// Shown when `open` is true
Modal::new(open)
    .title("Confirm Delete")
    .child(
        Column::new()
            .child(Text::new("Are you sure?", …))
            .child(Row::new()
                .child(Button::new(…).on_click(|| { *open = false; on_confirm(); }))
                .child(Button::new(…).on_click(|| { *open = false; }))
            )
    )
    .on_dismiss(|| { *open = false; })
    .max_width(400.0)
```

**DSL.**
```mkml
Modal {
    id: confirm_dialog
    title: "Confirm Delete"
    max_width: 400

    Column {
        Text "Are you sure?" { size: 16 }
        Row {
            Button "Delete" { on_click: do_delete  bg: #e74c3cff }
            Button "Cancel" { on_click: close_dialog }
        }
    }
}
```

**Implementation notes.**
- `open` state in `widget_state` under the widget's `id`.
- When open: dim backdrop rendered at high Z; panel rendered above backdrop.
- Mouse events on the backdrop trigger `on_dismiss`.
- Tab focus trapped inside the modal while open.
- Escape key triggers `on_dismiss`.

---

### 3.7 `Splitter`

**What it is.** A draggable divider between two panes.

**Rust API.**
```rust
Splitter::horizontal(left_panel, right_panel)
    .initial_ratio(0.3)     // 30% left / 70% right
    .min_left(120.0)
    .min_right(200.0)
    .on_change(|ratio| { … })

Splitter::vertical(top_panel, bottom_panel)
    .initial_ratio(0.6)
```

**DSL.**
```mkml
Splitter {
    id: main_split
    direction: horizontal
    ratio: 0.3
    min_left: 120
    min_right: 200

    Column { /* left panel */ }
    Column { /* right panel */ }
}
```

**Implementation notes.**
- Ratio stored in `widget_state` under `id`.
- Resize handle is a 4px wide rect that shows a resize cursor on hover
  (cursor feedback via `UiOutput` — see 4.1).
- Dragging the handle updates the ratio.

---

### 3.8 `Tree`

**What it is.** A hierarchical list with collapsible nodes. Used for file trees,
outlines, property inspectors.

**Rust API.**
```rust
Tree::new()
    .item("src/", true, |t| t     // label, initially_expanded
        .item("main.rs", false, |_| {})
        .item("lib.rs", false, |_| {})
    )
    .item("tests/", false, |t| t
        .item("integration.rs", false, |_| {})
    )
    .on_select(|path| { … })      // path is a Vec<usize> of indices
    .row_height(24.0)
    .indent(16.0)
```

**DSL.**
This widget is likely more useful from Rust code than DSL, but minimal DSL support:
```mkml
Tree {
    id: file_tree
    row_height: 24
    indent: 16
    on_select: file_selected
}
```
Tree data loaded dynamically from `Application::on_event_state`.

**Implementation notes.**
- Expand/collapse state per node stored in `widget_state` as a bit-set string.
- Visible rows computed from expanded state; virtualized (only visible rows measured).
- Arrow keys navigate; Space/Enter expand or select.

---

### 3.9 `Table`

**What it is.** A data table with sortable columns, fixed header, row selection,
and optional virtualization.

**Rust API.**
```rust
Table::new()
    .column("Name",  200.0, |row: &MyData| Text::new(&row.name, …).into())
    .column("Size",   80.0, |row| Text::new(&row.size_str, …).into())
    .column("Modified", 120.0, |row| Text::new(&row.modified, …).into())
    .rows(my_data_slice)
    .row_height(28.0)
    .on_sort(|col, dir| { … })
    .on_select(|idx| { … })
    .selected(Some(3))
```

**Implementation notes.**
- Virtualized: only rows in the visible scroll region are measured and painted.
- `ScrollView` internally powers the vertical scroll.
- Column headers are `Button`-like; clicking cycles sort direction.
- Row selection stored as `Option<usize>` in the widget.

---

### 3.10 `NumberInput`

**What it is.** A text field specialized for numeric input with increment/decrement
buttons.

**Rust API.**
```rust
NumberInput::new()
    .value(42.0)
    .min(0.0)
    .max(100.0)
    .step(1.0)
    .decimals(0)
    .on_change(|v| { … })
```

**DSL.**
```mkml
NumberInput {
    id: port_input
    min: 1
    max: 65535
    step: 1
    decimals: 0
    on_change: port_changed
}
```

**Implementation notes.**
- Wraps `TextBox` + two small arrow buttons (up/down).
- Input validation: only allow digits, minus sign, decimal point.
- Scroll wheel increments/decrements.
- Clamps to `[min, max]` on commit.

---

## Section 4 — Platform & Integration

---

### 4.1 Cursor shape feedback

**Problem.** When hovering over a resize handle, text input, or link, the OS cursor
should change shape. Currently the cursor is always the default arrow.

**Tasks.**
- [ ] Add `UiOutput` struct (analogous to `UiInput`):
  ```rust
  pub struct UiOutput {
      pub cursor: CursorIcon,
  }
  ```
- [ ] `Painter::set_cursor(CursorIcon)` — widgets call this during paint when hovered.
- [ ] `UiScene::frame` returns `(DrawList, UiOutput)`.
- [ ] `UiAppState` applies `UiOutput::cursor` via `window.set_cursor(…)`.
- [ ] `CursorIcon` mirrors `winit::window::CursorIcon`: `Default, Text, Crosshair,
      Hand, ResizeEw, ResizeNs, ResizeNwSe, ResizeNeSw, NotAllowed, …`.

---

### 4.2 System file dialog

**Tasks.**
- [ ] Add `marduk_ui::dialog::open_file(filter) -> Option<PathBuf>` — blocking (runs
      in a thread, result sent via channel). Uses `rfd` crate (cross-platform).
- [ ] `dialog::save_file(filter, default_name) -> Option<PathBuf>`.
- [ ] `Application::on_event_state` callback receives the path.
- [ ] DSL: `Button "Open…" { on_click: pick_file }` — no special DSL syntax needed.

---

### 4.3 Clipboard (app-level, not just TextBox)

**Problem.** `arboard` is wired only to `TextEditState`. The app has no way to
copy/paste arbitrary text from event callbacks.

**Tasks.**
- [ ] `Application::on_event` closure receives `ClipboardCtx`.
- [ ] `ClipboardCtx::get_text() -> Option<String>`.
- [ ] `ClipboardCtx::set_text(text: &str)`.
- [ ] Re-use the single `arboard::Clipboard` instance (re-creating it per-operation
      causes issues on some platforms).

---

### 4.4 IME composition support

**Problem.** On Linux/macOS, typing CJK characters requires an Input Method Editor
that shows a composition string before committing. The current `Text` event only
fires on committed characters, so CJK input is broken.

**Tasks.**
- [ ] Wire `winit::event::WindowEvent::Ime` → new `InputEvent::ImeCompose { text }`
      and `InputEvent::ImeCommit { text }`.
- [ ] `UiEvent::ImeCompose { text: String }` dispatched to focused widget.
- [ ] `TextBox` renders compose string with an underline (not yet committed).
- [ ] `Application::set_ime_cursor_area(rect)` — tells OS where the composition
      popup should appear.

---

### 4.5 Window management

**Tasks.**
- [ ] `Application::min_size(w, h)` / `.max_size(w, h)` — enforce OS window bounds.
- [ ] `Application::resizable(bool)` — lock window size.
- [ ] `Application::icon(bytes)` — set taskbar/dock icon from PNG bytes.
- [ ] `FrameCtx::set_title(str)` — update window title at runtime.
- [ ] Multi-window support (stretch goal; requires significant engine changes).

---

## Section 5 — Animation

All widgets currently have instant visual state changes. Animation is needed for
hover effects, transitions, loading states, and polished interactions.

---

### 5.1 `Animated<T>` value type

**What it is.** A value that interpolates from its current value toward a target over
time. Used inside widget state.

```rust
pub struct Animated<T: Lerp> {
    current: T,
    target: T,
    duration: f32,   // seconds
    easing: Easing,
    elapsed: f32,
}

impl<T: Lerp> Animated<T> {
    pub fn set_target(&mut self, target: T);
    pub fn tick(&mut self, dt: f32) -> T;  // returns current interpolated value
    pub fn is_done(&self) -> bool;
}
```

**Lerp implementations.**
- `f32`, `Color`, `Vec2`, `Rect` — all trivially `Lerp`.

**Easing functions.**
- `Linear`, `EaseIn`, `EaseOut`, `EaseInOut`, `Spring(damping, stiffness)`.

---

### 5.2 Widget animation integration

**Tasks.**
- [ ] `Button` hover background: animate color transition on hover enter/leave.
- [ ] `Toggle` thumb: animate position on state change.
- [ ] `Checkbox` checkmark: animate draw-in.
- [ ] `Slider` thumb: animate to new value on programmatic change.
- [ ] Widgets call `ctx.time.dt` (available from `FrameCtx`) for delta.
- [ ] `Animated<T>` stored as widget-level state (mutable field).

---

### 5.3 `Transition` widget

**What it is.** Wraps a child and cross-fades when its content changes.

```rust
Transition::new(child)
    .duration(0.2)
    .easing(Easing::EaseOut)
```

**Implementation notes.**
- When `child` changes identity (via a key), capture current frame to texture,
  fade out old, fade in new.
- Requires opacity infrastructure (Section 2.4).

---

## Section 6 — Text & Typography

---

### 6.1 Multi-line text rendering

**Problem.** `Text` widget renders a single line. There is no paragraph widget that
wraps text at word boundaries.

**Tasks.**
- [ ] `FontSystem::layout_paragraph(text, font, size, max_width) -> Vec<TextLine>` —
      returns line rects and byte ranges.
- [ ] `Text::multiline(true)` — measures full paragraph height; paints all lines.
- [ ] `DrawCmd::Text` gains a `line_height: f32` field (defaults to `size * 1.2`).
- [ ] DSL: `Text { multiline: true  line_height: 1.4  color: #fff }`.

---

### 6.2 Text decorations

**Tasks.**
- [ ] `TextCmd` gains `underline: bool, strikethrough: bool`.
- [ ] Rendered as thin colored rects at the correct baseline offset.
- [ ] `Painter::text_underlined(…)` / `Painter::text_strikethrough(…)`.
- [ ] Builder: `Text::underline(true)`, `Text::strikethrough(true)`.

---

### 6.3 Selectable text (read-only)

**Problem.** `Text` widgets cannot be selected/copied. Only `TextBox` supports
selection.

**Tasks.**
- [ ] `Text::selectable(true)` — makes the text widget respond to drag events.
- [ ] Uses `TextEditState` internally (read-only: no insert/delete).
- [ ] Right-click context menu with "Copy".

---

## Section 7 — DSL Language Features

---

### 7.1 Variables / constants

**What it is.** Named values that can be reused across properties.

```mkml
$primary: #6c8ebfff
$radius: 8
$gap: 12

Column {
    gap: $gap
    Button "Save" { bg: $primary  corner_radius: $radius }
    Button "Cancel" { bg: #aaaaaaff  corner_radius: $radius }
}
```

**Parser tasks.**
- [ ] `Token::Var(name)` — `$identifier` syntax.
- [ ] Top-level `$name: value` declarations (before `import` lines).
- [ ] `Node::Prop::value` can be `Value::Var(name)`.
- [ ] `DslDocument::vars: HashMap<String, Value>`.
- [ ] `DslLoader::build` resolves vars before passing to builders; error on
      undefined variable.

---

### 7.2 Conditional rendering

**What it is.** Show or hide subtrees based on widget state.

```mkml
Column {
    if: $show_sidebar
    gap: 8
    Text "Visible when show_sidebar is true" { size: 14 }
}
```

Or as a dedicated node:

```mkml
If {
    condition: $logged_in
    Text "Welcome back!" { size: 16 }
}
```

**Parser tasks.**
- [ ] `Node::Prop` with key `"if"` → stored as `Node::condition: Option<Value>`.
- [ ] `DslLoader::build` skips child if condition evaluates to falsy.
- [ ] Conditions: `bool` values from `widget_state`; `Value::Ident(name)` resolved
      against `widget_state` map.

---

### 7.3 Iteration / list rendering

**What it is.** Render a widget template for each item in a list.

```mkml
ForEach {
    items: $file_list     // bound from Application::bind_list("file_list", vec![...])
    Row {
        Text { text: $item  size: 14 }
        Button "Delete" { on_click: delete_item  data: $item }
    }
}
```

**Implementation notes.**
- `Application::bind_list(name, Vec<String>)` — stores a list in `DslBindings`.
- `ForEach` node expands to N copies of its child, each with `$item` bound.
- Only string lists initially; typed lists (structs) later.

---

### 7.4 Style inheritance / themes

**What it is.** A way to define a set of default property values and apply them to
all widgets of a type.

```mkml
Theme {
    Button {
        bg: #444444ff
        corner_radius: 6
        padding: 10
    }
    Text {
        color: #ffffffff
        size: 14
    }
}

Column {
    Button "Uses theme defaults" { }
    Button "Overrides bg" { bg: #e74c3cff }
}
```

**Implementation notes.**
- `DslDocument::theme: HashMap<String, Vec<Prop>>` — parsed from optional `Theme { }`
  block at top of file.
- `DslLoader::build_widget` merges theme props (lower priority) before widget-
  specific props (higher priority).

---

## Section 8 — Developer Experience

---

### 8.1 Widget inspector

**What it is.** An overlay that shows widget rects, names, and computed sizes when
a debug key is held. Essential for layout debugging.

**Tasks.**
- [ ] `Application::debug_overlay(true)` or `--debug` CLI flag.
- [ ] When active: pressing F2 toggles "inspect mode"; hovering a widget shows its
      name, rect, and constraints in a tooltip.
- [ ] Overlay drawn by `UiScene` at max Z above everything.

---

### 8.2 Hot reload of `.mkml` files

**What it is.** When a `.mkml` file changes on disk, the UI updates immediately
without restarting the process.

**Tasks.**
- [ ] `DslLoader::watch(path)` — spawns a background thread using `notify` crate.
- [ ] Change events sent via channel to main thread.
- [ ] `Application::watch_dir(path)` — watches `ui/` directory.
- [ ] On change: re-parse affected files, `DslLoader` evicts cache entries,
      next frame rebuilds the tree.

---

### 8.3 Improved LSP

**Current state.** Parse diagnostics, hover, completion — all working.

**Tasks.**
- [ ] Completion for `$variable` names (defined in the current file).
- [ ] "Go to definition" for `import "path" as Alias` → open file.
- [ ] Color preview in hover (show a colored square next to `#rrggbbaa` values).
- [ ] Code action: "Extract to component" — move selected block to a new `.mkml` file
      and replace with an import.
- [ ] Formatter: normalize indentation, align colons in property blocks.

---

### 8.4 Example gallery

**Tasks.**
- [ ] `marduk-studio` becomes a gallery of all widgets.
- [ ] Each widget shown with default appearance + interactive controls.
- [ ] Source for each example shown as a code panel (syntax highlighted `Text` widgets).
- [ ] Accessible from a side navigation using `Tree` widget (after it's built).

---

## Appendix: Priority order

**Do first (API stabilization — blocks everything).**
1.2 Eliminate panics
1.3 ParseError source location
1.5 Triple-measure elimination
1.6 Focus management
1.7 Stop-propagation fix

**Do next (unlock new apps).**
2.1 Image rendering
3.5 Overlay/popup infrastructure
3.1 Combobox
3.2 Tabs
3.6 Modal

**Do alongside (DX improvements).**
1.1 Naming pass
1.10 DSL state key rule
4.1 Cursor feedback
8.1 Widget inspector

**Do later (language features).**
7.1 Variables
7.2 Conditionals
5.x Animation system
2.2 Shadows
3.4 Menu/MenuBar

**Do last (advanced).**
6.x Typography improvements
3.8 Tree
3.9 Table
7.3 Iteration
8.2 Hot reload
