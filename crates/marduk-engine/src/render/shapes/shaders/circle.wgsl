struct Viewport {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> u_view: Viewport;

struct VsIn {
    // Unit quad vertex [0, 1].
    @location(0) quad_pos: vec2<f32>,

    // Per-instance.
    @location(1) center: vec2<f32>,
    @location(2) radius_bw: vec2<f32>,    // .x = radius, .y = border_width
    @location(3) color0: vec4<f32>,
    @location(4) color1: vec4<f32>,
    @location(5) grad_p0: vec2<f32>,
    @location(6) grad_p1: vec2<f32>,
    @location(7) border_color: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) pixel_pos: vec2<f32>,
    @location(1) center: vec2<f32>,
    @location(2) radius: f32,
    @location(3) border_width: f32,
    @location(4) color0: vec4<f32>,
    @location(5) color1: vec4<f32>,
    @location(6) grad_p0: vec2<f32>,
    @location(7) grad_p1: vec2<f32>,
    @location(8) border_color: vec4<f32>,
};

fn px_to_ndc(p: vec2<f32>, viewport: vec2<f32>) -> vec2<f32> {
    let x = (p.x / viewport.x) * 2.0 - 1.0;
    let y = 1.0 - (p.y / viewport.y) * 2.0;
    return vec2<f32>(x, y);
}

fn sample_fill(
    pixel_pos: vec2<f32>,
    color0: vec4<f32>,
    color1: vec4<f32>,
    grad_p0: vec2<f32>,
    grad_p1: vec2<f32>,
) -> vec4<f32> {
    let dir    = grad_p1 - grad_p0;
    let len_sq = dot(dir, dir);
    if (len_sq < 0.0001) {
        return color0;
    }
    let t = clamp(dot(pixel_pos - grad_p0, dir) / len_sq, 0.0, 1.0);
    return mix(color0, color1, t);
}

@vertex
fn vs_main(input: VsIn) -> VsOut {
    var out: VsOut;
    let viewport = max(u_view.viewport, vec2<f32>(1.0, 1.0));

    let radius       = max(input.radius_bw.x, 0.0);
    let border_width = max(input.radius_bw.y, 0.0);
    // Expand bounding quad by border + 1 px for AA fringe.
    let expansion  = border_width + 1.0;
    let half_extent = radius + expansion;
    let origin = input.center - vec2<f32>(half_extent);
    let size   = vec2<f32>(half_extent * 2.0);

    let pos_px = origin + size * input.quad_pos;
    let ndc    = px_to_ndc(pos_px, viewport);

    out.position    = vec4<f32>(ndc, 0.0, 1.0);
    out.pixel_pos   = pos_px;
    out.center      = input.center;
    out.radius      = radius;
    out.border_width = border_width;
    out.color0      = input.color0;
    out.color1      = input.color1;
    out.grad_p0     = input.grad_p0;
    out.grad_p1     = input.grad_p1;
    out.border_color = input.border_color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let dist = length(in.pixel_pos - in.center) - in.radius;

    let shape_mask = smoothstep(0.5, -0.5, dist);
    if (shape_mask <= 0.0) { discard; }

    let fill_mask   = smoothstep(0.5, -0.5, dist + in.border_width);
    let border_mask = shape_mask - fill_mask;

    let fill_color = sample_fill(in.pixel_pos, in.color0, in.color1, in.grad_p0, in.grad_p1);

    return fill_color * fill_mask + in.border_color * border_mask;
}
