# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build

# Run the studio demo application
cargo run -p marduk-studio

# Run tests
cargo test

# Run tests for a specific crate
cargo test -p marduk-engine

# Lint
cargo clippy

# Format
cargo fmt
```

## Architecture

Rust workspace with three crates:

- **`marduk-engine`** — platform + GPU runtime; no app logic
- **`marduk-studio`** — binary that uses the engine (demo/test app)
- **`marduk-ui`** — placeholder for future UI layer

### Engine subsystems (`marduk-engine/src/`)

**`core/`** — Application contract
- `App` trait: implement `on_frame(&mut FrameCtx)` (required) and `on_window_event(...)` (optional)
- `AppControl::Continue / Exit` returned from callbacks
- `FrameCtx` provides per-frame access to `gpu`, `input`, `input_frame`, `time`, `window`, `runtime`

**`window/`** — Event loop and window management
- `Runtime::run(config, gpu_init, app)` starts the winit event loop
- `RuntimeCtx` (in `FrameCtx`) lets the app queue commands: `create_window`, `close_window`, `exit`
- Currently Wayland + Vulkan; window/surface creation uses `ouroboros` self-referential struct (`WindowEntry`) to keep `Gpu<'w>` borrowing its window

**`device/`** — GPU device abstraction (`Gpu<'w>`)
- Wraps wgpu instance/adapter/device/queue/surface
- `begin_frame()` → `GpuFrame` (encoder + view); `submit(frame)` presents
- `resize()` reconfigures the swapchain on window resize

**`scene/`** — Renderer-agnostic draw stream
- `DrawList`: records `DrawCmd` items with a `ZIndex`; iterates in paint order (back-to-front) using a stable sort on `SortKey` (z-index + insertion order)
- `DrawCmd` enum currently has one variant: `Rect(RectCmd)`
- To add a new primitive: add `scene::shapes::<name>`, a new `DrawCmd` variant, and a matching renderer under `render::shapes::<name>`

**`render/`** — wgpu renderers consuming `DrawList`
- `RenderCtx`: device/queue + surface format + viewport (logical px)
- `RenderTarget`: encoder + color view
- `RectRenderer`: instanced draw of solid-fill rectangles; lazily initializes pipeline/buffers on first call; re-initializes pipeline when surface format changes
- Shader at `render/shapes/shaders/rect.wgsl` — included via `include_str!` at compile time

**`coords/`** — `Vec2`, `Rect`, `Viewport`, `Color` primitives

**`paint/`** — `Paint` (currently `Solid(Color)`, gradient variant planned)
- Colors are linear premultiplied RGBA (`Color::from_straight` converts from straight alpha)

**`input/`** — `InputState` (persistent), `InputFrame` (per-frame delta); winit events translated in `input::platform::winit`

**`time/`** — `FrameClock` ticks each frame; provides `FrameTime` with `frame_index` and elapsed

### Coordinate conventions
- CPU geometry in **logical pixels**, top-left origin, +Y down
- Vertex shaders convert to NDC using a viewport uniform
