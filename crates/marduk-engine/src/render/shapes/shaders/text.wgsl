struct Viewport {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u_view: Viewport;
@group(0) @binding(1) var t_atlas: texture_2d<f32>;
@group(0) @binding(2) var s_atlas: sampler;

struct VsIn {
    // Unit quad vertex [0, 1].
    @location(0) quad_pos: vec2<f32>,

    // Per-instance.
    @location(1) dst_min: vec2<f32>,   // glyph top-left in logical pixels
    @location(2) dst_max: vec2<f32>,   // glyph bottom-right in logical pixels
    @location(3) uv_min: vec2<f32>,    // atlas UV top-left  [0, 1]
    @location(4) uv_max: vec2<f32>,    // atlas UV bottom-right [0, 1]
    @location(5) color: vec4<f32>,     // premultiplied RGBA
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

fn px_to_ndc(p: vec2<f32>, viewport: vec2<f32>) -> vec2<f32> {
    let x = (p.x / viewport.x) * 2.0 - 1.0;
    let y = 1.0 - (p.y / viewport.y) * 2.0;
    return vec2<f32>(x, y);
}

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    let viewport = max(u_view.viewport, vec2<f32>(1.0, 1.0));
    let pos_px = mix(input.dst_min, input.dst_max, input.quad_pos);
    out.position = vec4<f32>(px_to_ndc(pos_px, viewport), 0.0, 1.0);
    out.uv    = mix(input.uv_min, input.uv_max, input.quad_pos);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Atlas stores per-pixel coverage in the R channel (R8Unorm).
    // Color is premultiplied; scale by coverage to produce premultiplied output.
    let coverage = textureSample(t_atlas, s_atlas, in.uv).r;
    if (coverage <= 0.0) { discard; }
    return in.color * coverage;
}
