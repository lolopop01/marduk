struct Viewport {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> u_view: Viewport;

struct VsIn {
    // Unit quad vertex position (0..1).
    @location(0) quad_pos: vec2<f32>,

    // Per-instance data.
    @location(1) origin: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn px_to_ndc(p: vec2<f32>, viewport: vec2<f32>) -> vec2<f32> {
    // CPU space: origin top-left, +Y down, logical pixels.
    // NDC: origin center, +Y up.
    let x = (p.x / viewport.x) * 2.0 - 1.0;
    let y = 1.0 - (p.y / viewport.y) * 2.0;
    return vec2<f32>(x, y);
}

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;

    let viewport = max(u_view.viewport, vec2<f32>(1.0, 1.0));
    let pos_px = input.origin + input.size * input.quad_pos;
    let ndc = px_to_ndc(pos_px, viewport);

    out.position = vec4<f32>(ndc, 0.0, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    // Premultiplied linear RGBA; blend state expects premultiplied alpha.
    return input.color;
}