# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build everything
cargo build

# Run the studio demo
cargo run -p marduk-studio

# Run tests
cargo test

# Run tests for a specific crate
cargo test -p marduk-engine

# Lint
cargo clippy

# Format
cargo fmt

# Run the language server (stdio, for editor integration)
cargo run -p marduk-lsp
```

## Workspace layout

Five crates:

| Crate | Role |
|---|---|
| `marduk-engine` | Platform + GPU runtime; no app logic |
| `marduk-mkml` | `.mkml` lexer / parser / AST — zero dependencies |
| `marduk-ui` | Widget system + DSL builder on top of the engine |
| `marduk-studio` | Demo binary ("Mission Control" UI) |
| `marduk-lsp` | Language server for `.mkml` files (hover, completion, diagnostics) |

Dependency graph: `marduk-engine` ← `marduk-ui` ← `marduk-studio`; `marduk-mkml` ← `marduk-ui`, `marduk-lsp`.

---

## marduk-engine (`crates/marduk-engine/src/`)

### `core/`
- `App` trait: implement `on_frame(&mut FrameCtx)` (required) and `on_window_event(...)` (optional)
- `AppControl::Continue / Exit` returned from callbacks
- `FrameCtx`: per-frame access to `gpu`, `input`, `input_frame`, `time`, `window`, `runtime`

### `window/`
- `Runtime::run(config, gpu_init, app)` — starts the winit event loop
- Supports Wayland + Vulkan (and Metal/DX12/WebGPU via `Backends::all()`)
- `WindowEntry` uses `ouroboros` self-referential struct to keep `Gpu<'w>` borrowing its window
- **Drop order**: `CloseRequested` must NOT call `destroy_window_entry` eagerly — `AppState`'s field order (`app` before `windows`) ensures wgpu resources drop before the device

### `device/`
- `Gpu<'w>`: wraps wgpu instance/adapter/device/queue/surface
- `begin_frame()` → `GpuFrame` (encoder + view); `submit(frame)` presents
- Field order in `Gpu`: `instance` declared **last** — Vulkan requires instance to outlive surface/device/queue

### `scene/`
- `DrawList`: records `DrawCmd` items with a `ZIndex`; iterates back-to-front via stable sort on `SortKey`
- `DrawCmd` variants: `Rect`, `RoundedRect`, `Circle`, `Text`
- To add a primitive: `scene/shapes/<name>.rs` + `DrawCmd` variant + `render/shapes/<name>.rs`

### `render/`
- `RenderCtx`: device/queue + surface format + viewport
- Shape renderers: `RectRenderer`, `RoundedRectRenderer`, `CircleRenderer`, `TextRenderer`
- All lazily initialize pipeline/buffers on first call; reinitialize when surface format changes
- `render/shapes/common.rs`: shared `ViewportUniform`, `QuadVertex`/`QUAD_VERTICES`/`QUAD_INDICES`, `premul_alpha_blend()`, `resolve_paint()`

### `coords/`
`Vec2`, `Rect`, `Viewport`, `Color`, `CornerRadii` — CPU geometry in **logical pixels**, top-left origin, +Y down

### `paint/`
- `Paint::Solid(Color)` or `Paint::LinearGradient(LinearGradient { start, end, stops, spread })`
- Colors are **premultiplied linear RGBA**; `Color::from_straight(r,g,b,a)` converts from straight alpha

### `input/`
- `InputState` (persistent across frames), `InputFrame` (per-frame delta: `keys_pressed`, `buttons_pressed/released`, `scroll_delta`)
- Key repeat: `InputEvent::Key { repeat: true }` flows through to `frame.keys_pressed` (so holding Backspace works)
- `Drag` origin tracked in `UiAppState::drag_origin`: set on left-button press, cleared on release; passed through `UiInput`

### `text/`
- `FontSystem`: loads TTF fonts; `load_font(data) -> FontId`; `measure_text(text, id, size, max_width) -> Vec2`

### Shader conventions
- Logical pixels; viewport UBO: `vec2<f32>` at group 0 binding 0 (16-byte padded struct)
- AA via `smoothstep(0.5, -0.5, sdf_dist)`; premultiplied RGBA throughout
- Shaders live at `render/shapes/shaders/<name>.wgsl`, included via `include_str!`

---

## marduk-mkml (`crates/marduk-mkml/src/`)

Zero-dependency crate — safe to use from the LSP without pulling in GPU code.

- `parse_str(src) -> Result<DslDocument, ParseError>`
- `DslDocument { imports: Vec<Import>, root: Node }`
- `Node { widget: String, content: Option<String>, props: Vec<Prop>, children: Vec<Node> }`
- `Value::Str | Number(f32) | Color([u8; 4]) | Ident` — `Color` is straight-alpha RGBA bytes as written in source

### `.mkml` syntax
```
// comment
import "sidebar.mkml" as Sidebar

Column {
    gap: 8
    bg: #1a1a2eff

    Text "Hello world" { color: #ffffffff  size: 16 }
    Button "Click me" { on_click: my_event  corner_radius: 6 }

    Sidebar { }
}
```
- No commas, semicolons, or brackets
- Color literals: `#rrggbbaa` (8 hex digits, straight alpha)
- Children are nested widget blocks; properties are `key: value` lines
- `import "path" as Alias` at top; reference `Alias { }` anywhere below

---

## marduk-ui (`crates/marduk-ui/src/`)

### Core traits
- `Widget`: `measure(&self, Constraints, &LayoutCtx) -> Vec2` + `paint(&self, &mut Painter, Rect)` + `on_event(&mut self, &UiEvent, Rect, &LayoutCtx) -> EventResult`
- `Element`: `Box<dyn Widget>` newtype; any `Widget` → `Element` via `From`
- `Painter<'a>`: `fill_rect`, `fill_rounded_rect`, `fill_circle`, `text`, `push_clip`/`pop_clip`, `is_hovered(rect)`, `is_pressed(rect)`

### Events (`event.rs`)
```rust
pub enum UiEvent {
    Click { pos: Vec2 },           // mouse released
    Hover { pos: Vec2 },           // every frame
    Drag { pos: Vec2, start: Vec2 },// mouse held + moved; start = where drag began
    TextInput { text: String },
    KeyPress { key: Key },
    ScrollWheel { delta: f32 },    // positive = scroll down
}
```

### Built-in widgets (all in `marduk_ui::prelude::*`)
| Widget | Key API |
|---|---|
| `Text` | `new(text, font, size, color)` |
| `Container` | `.child()`, `.padding_all()`, `.background()`, `.border()`, `.corner_radius()` |
| `Column` / `Row` | `.child()`, `.spacing()`, `.padding_all()`, `.cross_align(Align::*)` |
| `Button` | `.on_click(|| ...)`, `.background()`, `.hover_background()`, `.press_background()` |
| `Checkbox` | `.checked(bool)`, `.on_change(\|b\| ...)` |
| `Toggle` | `.checked(bool)`, `.on_change(\|b\| ...)` |
| `Slider` | `.min().max().value()`, `.on_drag(\|v\| ...)` (visual), `.on_change(\|v\| ...)` (release) |
| `RadioGroup` | `.option(label, value)`, `.selected(val)`, `.on_change(\|s\| ...)` |
| `ProgressBar` | `.value(0.0..=1.0)`, `.fill_color()` |
| `TextBox` | `.text()`, `.placeholder()`, `.on_change(\|s\| ...)`, `.on_submit(\|s\| ...)` |
| `ScrollView` | `.line_height()`, `.show_scrollbar()`, `.on_scroll(\|offset\| ...)` |
| `Stack` | `.item(StackItem)` with `.left/.top/.right/.bottom/.width/.height` anchors |

**Slider note**: `on_drag` fires on every mouse-move (updates `widget_state` for real-time visual); `on_change` fires only on mouse release (the public event). Both must be wired in `build_slider` for DSL sliders to work correctly.

### DSL layer (`dsl/`)
- `DslLoader`: parse/cache `.mkml`; `build(doc, bindings) -> Element` (called every frame)
- `DslBindings`: `fonts`, `event_queue` (Rc<RefCell<Vec<String>>>), `widget_state` (Rc<RefCell<HashMap<String, WidgetStateValue>>>)
- `WidgetStateValue::Bool | Float | Str` — persists across frame rebuilds (keyed by `state_key` or `on_change` name)
- `DslBindings::take_events() -> Vec<String>` — drain the event queue each frame

### Application entry point
```rust
use marduk_ui::Application;

Application::new()
    .title("My App")
    .size(1280.0, 720.0)
    .font("body", load_font())
    .component("Sidebar", include_str!("ui/sidebar.mkml"))
    .on_event("quit", || std::process::exit(0))
    .on_event_state("clear_input", |state| state.clear("my_textbox"))
    .run(include_str!("ui/main.mkml"))    // never returns
```

`run_widget(|fonts: &FontMap| my_widget.into())` for pure-Rust widget trees.

---

## marduk-lsp (`crates/marduk-lsp/src/`)

stdio LSP server using `tower-lsp 0.20`.

- `knowledge.rs` — static `WIDGETS: &[WidgetInfo]` + `PropInfo`/`PropKind` (both `Copy`)
- `analysis.rs` — `word_at`, `find_enclosing_widget`, `completion_context -> Context`
- `backend.rs` — `Backend { client, docs: Arc<RwLock<HashMap<Url, String>>> }` — hover, completion, diagnostics
- `main.rs` — `LspService::new(Backend::new)` on stdin/stdout

Capabilities: full-sync document store, parse diagnostics on every change, hover docs, context-aware completion (widget names, property keys, enum/bool/color values).

---

## Key patterns

### Borrow trick in flex paint
`painter.font_system` is `&'a FontSystem` (Copy) — copy it out before the loop so `painter` is free for mutable child `.paint()` calls:
```rust
let fonts = painter.font_system; // copies the shared reference
let ctx = LayoutCtx { fonts };
child.paint(painter, rect);      // painter freely usable again
```

### DSL widget state persistence
The DSL rebuilds the widget tree every frame. Per-widget state (slider value, checkbox state, text) survives only via `widget_state` map. Any widget that needs to persist a value across frames must:
1. Read from `widget_state` at build time
2. Write to `widget_state` in its callback (before touching `event_queue`)
