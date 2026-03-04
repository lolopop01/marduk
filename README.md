# Marduk

**Marduk** is a lightweight Rust UI framework built on **wgpu**, featuring:

* a declarative markup language (`.mkml`)
* a retained widget system
* a language server for editor integration

Marduk aims to solve a common problem in UI frameworks:

> Most UI systems are either **huge**, **too simple**, or **painful to use**.

Marduk is built around a simple philosophy:

```
            Capability
               ▲
              / \
             /   \
            /     \
           /       \
          /         \
         /           \
        /             \
       /               \
      /                 \
     /                   \
    ▼                     ▼
Simplicity ----------- Performance
```

This is the **UI Trinity**.

Most frameworks choose two:

| Framework        | Capability | Simplicity | Performance |
| ---------------- | ---------- | ---------- | ----------- |
| Qt / GTK         | ✅          | ✅          | ❌           |
| Electron / React | ✅          | ✅          | ❌           |
| Raw GPU UI       | ✅          | ❌          | ✅           |
| Minimal toolkits | ❌          | ✅          | ✅           |

**Marduk aims for the center of the triangle**:

* powerful enough for real applications
* simple enough to learn quickly
* lightweight enough to stay fast and dependency-minimal

The core philosophy is:

> **Small engine. Powerful primitives. Composable UI.**

---

# Workspace

| Crate           | Description                                                                          |
| --------------- | ------------------------------------------------------------------------------------ |
| `marduk-engine` | Platform + GPU runtime (wgpu, winit). Handles windowing, rendering, input, and text. |
| `marduk-mkml`   | Zero-dependency lexer/parser/AST for the `.mkml` markup language.                    |
| `marduk-ui`     | Widget system and DSL builder on top of the engine.                                  |
| `marduk-studio` | Demo binary — a "Mission Control" UI that exercises all widgets.                     |
| `marduk-lsp`    | Language server for `.mkml` files (hover, completion, diagnostics).                  |

---

# Running

```bash
# Demo application
cargo run -p marduk-studio

# Language server (stdio — point your editor at this)
cargo run -p marduk-lsp
```

---

# The `.mkml` markup language

UI layouts are written in **`.mkml`**, a brace-based markup format inspired by **CSS, YAML, and UI DSLs**.

Example:

```mkml
// main.mkml
import "sidebar.mkml" as Sidebar

Column {
    gap: 8
    bg: #1a1a2eff

    Text "Hello, world!" { color: #ffffffff size: 20 }

    Row {
        gap: 12

        Button "Launch" {
            on_click: launch
            corner_radius: 6
            bg: #4c6ef5ff
        }

        Button "Abort" {
            on_click: abort
            corner_radius: 6
            bg: #e03131ff
        }
    }

    Sidebar { }
}
```

Design goals of `.mkml`:

* **No commas, semicolons, or angle brackets**
* minimal punctuation
* readable UI structure
* easy to parse and lint

### Syntax notes

* Colors: `#rrggbb` or `#rrggbbaa` (6 or 8 hex digits, straight alpha)
* Imports: `import "file.mkml" as Alias`
* Comments: `// comment`

---

# Supported widgets

Widgets currently supported by the `.mkml` builder:

| Widget | Purpose |
| --- | --- |
| `Text` | Single-line text |
| `Container` | One-child box with padding/background/border/radius |
| `Column` / `Row` | Vertical / horizontal layout |
| `Button` | Clickable button |
| `Checkbox` | Labelled checkbox |
| `Toggle` | On/off switch |
| `Slider` | Horizontal range slider |
| `RadioGroup` + `RadioOption` | Mutually-exclusive options |
| `ProgressBar` | Non-interactive fill bar |
| `TextBox` | Single-line text input |
| `ScrollView` | Scrollable container |
| `Stack` | Overlay layout |
| `Image` | Displays registered image assets (`src`) |
| `Tabs` | Tab strip + active tab body |
| `Splitter` | Resizable two-pane layout |
| `NumberInput` | Numeric stepper/input |
| `Tooltip` | Hover tooltip wrapper |
| `Modal` | Overlay dialog container |
| `Combobox` | Select dropdown |

These primitives are designed to be composed into larger UI patterns.

---

# Wiring events in Rust

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

Event names correspond directly to `.mkml` properties such as:

```
on_click: launch
```

`on_event_state` gives access to widget state:

* TextBox content
* Slider values
* Checkbox state
* etc.

---

# Custom widgets

Marduk supports **custom widgets implemented directly in Rust**.

```rust
use marduk_ui::prelude::*;

pub struct MyWidget;

impl Widget for MyWidget {
    fn measure(&self, c: Constraints, _ctx: &LayoutCtx) -> Vec2 {
        c.constrain(Vec2::new(200.0, 40.0))
    }

    fn paint(&self, painter: &mut Painter, rect: Rect) {
        painter.fill_rounded_rect(
            rect,
            8.0,
            Paint::Solid(Color::from_straight(0.2, 0.5, 1.0, 1.0)),
            None,
        );
    }
}
```

Use it in a pure Rust UI tree:

```rust
Application::new()
    .run_widget(|_fonts| MyWidget.into());
```

This allows advanced users to build:

* graph editors
* canvas tools
* custom controls
* visualization widgets

while still using the Marduk engine.

---

# Language server

`marduk-lsp` implements the **Language Server Protocol** for `.mkml`.

Features:

* **Diagnostics** — parse errors shown inline
* **Hover** — widget/property documentation
* **Completion** — widgets, properties, enums, colors

Point your editor to:

```
cargo run -p marduk-lsp
```

with `.mkml` file association.

---

# Architecture notes

**Renderer**

* GPU backend: `wgpu`
* Supports Vulkan / Metal / DX12 / WebGPU

**Coordinates**

* logical pixels
* origin: top-left
* +Y downward
* shaders convert to NDC using a viewport uniform

**Colors**

* stored internally as **premultiplied linear RGBA**
* input via `Color::from_straight(r,g,b,a)`

**Widget lifecycle**

The widget tree is **rebuilt every frame** from the `.mkml` document.

Stateful values are preserved via:

```
DslBindings::widget_state
```

Examples:

* slider position
* checkbox state
* textbox text

Drag tracking is stored in `UiAppState`, allowing it to persist across rebuilds.

---

# Building

```bash
cargo build
cargo build --release

cargo test
cargo test -p marduk-engine

cargo clippy
cargo fmt
```

Requirements:

* Rust toolchain (edition **2024**)
* `wgpu` compatible GPU drivers

On Linux, ensure **Vulkan drivers or Mesa** are installed.
