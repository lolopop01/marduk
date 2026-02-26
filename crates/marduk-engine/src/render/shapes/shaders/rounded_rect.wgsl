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
    @location(1) origin: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) radii: vec4<f32>,          // tl, tr, br, bl
    @location(4) color0: vec4<f32>,
    @location(5) color1: vec4<f32>,
    @location(6) grad_p0: vec2<f32>,
    @location(7) grad_p1: vec2<f32>,
    @location(8) border_width_pad: vec2<f32>, // .x = border_width
    @location(9) border_color: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) pixel_pos: vec2<f32>,
    @location(1) center: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) radii: vec4<f32>,
    @location(4) color0: vec4<f32>,
    @location(5) color1: vec4<f32>,
    @location(6) grad_p0: vec2<f32>,
    @location(7) grad_p1: vec2<f32>,
    @location(8) border_width: f32,
    @location(9) border_color: vec4<f32>,
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

    let border_width = input.border_width_pad.x;
    // Expand bounding quad by border + 1 px for AA fringe.
    let expansion = max(border_width, 0.0) + 1.0;
    let exp_origin = input.origin - vec2<f32>(expansion);
    let exp_size   = input.size   + vec2<f32>(expansion * 2.0);

    let pos_px = exp_origin + exp_size * input.quad_pos;
    let ndc    = px_to_ndc(pos_px, viewport);

    out.position    = vec4<f32>(ndc, 0.0, 1.0);
    out.pixel_pos   = pos_px;
    out.center      = input.origin + input.size * 0.5;
    out.half_size   = input.size * 0.5;
    out.radii       = input.radii;
    out.color0      = input.color0;
    out.color1      = input.color1;
    out.grad_p0     = input.grad_p0;
    out.grad_p1     = input.grad_p1;
    out.border_width = border_width;
    out.border_color = input.border_color;
    return out;
}

// Signed distance to a rounded box.
// p  : position relative to box center (+Y down)
// b  : half-size
// r  : per-corner radii  (tl=r.x, tr=r.y, br=r.z, bl=r.w)
fn sd_rounded_box(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    // Pick the corner radius for the quadrant the fragment is in.
    var corner_r: f32;
    if (p.x > 0.0) {
        corner_r = select(r.y, r.z, p.y > 0.0); // right: tr or br
    } else {
        corner_r = select(r.x, r.w, p.y > 0.0); // left:  tl or bl
    }
    let q = abs(p) - b + corner_r;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - corner_r;
}

// Samples the fill paint.  For solid color, grad_p0 == grad_p1 is degenerate
// and the function returns color0 unchanged.
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

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Clamp radii so they never exceed the smallest half-dimension.
    let max_r  = min(in.half_size.x, in.half_size.y);
    let radii  = min(in.radii, vec4<f32>(max(max_r, 0.0)));

    let p    = in.pixel_pos - in.center;
    let dist = sd_rounded_box(p, in.half_size, radii);

    // Outer shape mask (fill + border region, AA at the outer edge).
    let shape_mask = smoothstep(0.5, -0.5, dist);
    if (shape_mask <= 0.0) { discard; }

    // Fill mask (interior only, AA at the inner border edge).
    let fill_mask   = smoothstep(0.5, -0.5, dist + in.border_width);
    let border_mask = shape_mask - fill_mask;

    let fill_color = sample_fill(in.pixel_pos, in.color0, in.color1, in.grad_p0, in.grad_p1);

    // Both colors are premultiplied; contributions add correctly.
    return fill_color * fill_mask + in.border_color * border_mask;
}
