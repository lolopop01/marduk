# marduk

A Rust UI framework built on wgpu, with a declarative markup language (`.mkml`), a retained widget system, and a language server.

## Workspace

| Crate | Description |
|---|---|
| `marduk-engine` | Platform + GPU runtime (wgpu, winit). Handles windowing, rendering, input, and text. |
| `marduk-mkml` | Zero-dependency lexer/parser/AST for the `.mkml` markup language. |
| `marduk-ui` | Widget system and DSL builder on top of the engine. |
| `marduk-studio` | Demo binary — a "Mission Control" UI that exercises all widgets. |
| `marduk-lsp` | Language server for `.mkml` files (hover, completion, diagnostics). |

## Running

```bash
# Demo app
cargo run -p marduk-studio

# Language server (stdio — point your editor at this)
cargo run -p marduk-lsp
```

## The `.mkml` markup language

UI layouts are described in `.mkml` files — a whitespace-sensitive format inspired by CSS and YAML:

```
// main.mkml
import "sidebar.mkml" as Sidebar

Column {
    gap: 8
    bg: #1a1a2eff

    Text "Hello, world!" { color: #ffffffff  size: 20 }

    Row {
        gap: 12
        Button "Launch" { on_click: launch  corner_radius: 6  bg: #4c6ef5ff }
        Button "Abort"  { on_click: abort   corner_radius: 6  bg: #e03131ff }
    }

    Sidebar { }
}
```

- **No commas, semicolons, or angle brackets**
- Colors: `#rrggbbaa` (8 hex digits, straight alpha)
- `import "file.mkml" as Alias` at top; use `Alias { }` anywhere below
- Comments: `// ...`

### Supported widgets

| Widget | Purpose |
|---|---|
| `Text` | Single line of text |
| `Container` | Box with one child; supports padding, background, border, corner radius |
| `Column` / `Row` | Vertical / horizontal flex layout |
| `Button` | Pressable button with hover + press states |
| `Checkbox` | Labelled checkbox |
| `Toggle` | On/off switch |
| `Slider` | Horizontal range slider |
| `RadioGroup` + `RadioOption` | Mutually-exclusive radio buttons |
| `ProgressBar` | Non-interactive fill bar |
| `TextBox` | Single-line text input |
| `ScrollView` | Scrollable container with optional scrollbar |
| `Stack` | Overlay layout with per-child anchor positioning |

### Wiring events in Rust

```rust
use marduk_ui::Application;

Application::new()
    .title("My App")
    .size(1280.0, 720.0)
    .font("body", load_font())
    .component("Sidebar", include_str!("ui/sidebar.mkml"))
    .on_event("launch", || println!("Launching!"))
    .on_event("abort",  || println!("Aborted."))
    .on_event_state("clear_log", |state| state.clear("log_text"))
    .run(include_str!("ui/main.mkml"))
```

`on_event` names match `on_click: launch` (and similar) in `.mkml`. `on_event_state` gives access to widget state (read/write TextBox content, Slider values, etc.).

## Custom widgets

Implement `Widget` and drop it anywhere:

```rust
use marduk_ui::prelude::*;

pub struct MyWidget;

impl Widget for MyWidget {
    fn measure(&self, c: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        c.constrain(Vec2::new(200.0, 40.0))
    }
    fn paint(&self, painter: &mut Painter, rect: Rect) {
        painter.fill_rounded_rect(
            rect, 8.0,
            Paint::Solid(Color::from_straight(0.2, 0.5, 1.0, 1.0)),
            None,
        );
    }
}

// Use it in a pure-Rust tree:
Application::new()
    .run_widget(|_fonts| MyWidget.into());
```

## Language server

`marduk-lsp` implements the Language Server Protocol over stdio.

Features:
- **Diagnostics** — parse errors shown inline as you type
- **Hover** — widget and property documentation on mouse-over
- **Completion** — widget names, property keys, enum values, and color snippets

Point your editor at `cargo run -p marduk-lsp` with `.mkml` file association.

## Architecture notes

- GPU backend: wgpu (Vulkan / Metal / DX12 / WebGPU)
- Coordinates: logical pixels, top-left origin, +Y down; shaders convert to NDC via a viewport uniform
- Colors: premultiplied linear RGBA internally; `Color::from_straight(r,g,b,a)` for straight-alpha input
- Widget tree is **rebuilt every frame** from the `.mkml` document; stateful widget values (slider position, checkbox state, text) are persisted across rebuilds in `DslBindings::widget_state`
- Drag tracking lives in `UiAppState` (not in widgets) so it survives the per-frame rebuild

## Building

```bash
cargo build          # debug
cargo build --release

cargo test           # all crates
cargo test -p marduk-engine

cargo clippy
cargo fmt
```

Requires a Rust toolchain (edition 2024). The engine links wgpu — on Linux, Vulkan drivers or Mesa are needed.
