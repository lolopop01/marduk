struct Viewport {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> u_view: Viewport;

@group(1) @binding(0)
var t_image: texture_2d<f32>;

@group(1) @binding(1)
var s_image: sampler;

// ── Instance layout ───────────────────────────────────────────────────────
// loc 1  origin         [f32; 2]
// loc 2  size           [f32; 2]
// loc 3  uv_min         [f32; 2]
// loc 4  uv_max         [f32; 2]
// loc 5  tint           [f32; 4]  straight RGBA
// loc 6  radii          [f32; 4]  tl, tr, br, bl

struct VsIn {
    @location(0) quad_pos: vec2<f32>,

    @location(1) origin:   vec2<f32>,
    @location(2) size:     vec2<f32>,
    @location(3) uv_min:   vec2<f32>,
    @location(4) uv_max:   vec2<f32>,
    @location(5) tint:     vec4<f32>,
    @location(6) radii:    vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv:         vec2<f32>,
    @location(1) tint:       vec4<f32>,
    @location(2) local_pos:  vec2<f32>,  // position within [0, size], for SDF
    @location(3) size:       vec2<f32>,
    @location(4) radii:      vec4<f32>,
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

    // Expand quad by 1 px for AA fringe.
    let exp_origin = input.origin - vec2<f32>(1.0);
    let exp_size   = input.size   + vec2<f32>(2.0);

    let pos_px  = exp_origin + exp_size * input.quad_pos;
    let ndc     = px_to_ndc(pos_px, viewport);

    // Map expanded quad_pos to UV space.
    let t       = (pos_px - input.origin) / input.size;
    let uv      = input.uv_min + (input.uv_max - input.uv_min) * t;

    // local_pos for SDF: position within the un-expanded quad in [0, size].
    let local_pos = (pos_px - input.origin);

    out.position  = vec4<f32>(ndc, 0.0, 1.0);
    out.uv        = uv;
    out.tint      = input.tint;
    out.local_pos = local_pos;
    out.size      = input.size;
    out.radii     = input.radii;
    return out;
}

// Signed distance to a rounded box.
// p  : position relative to box center (+Y down)
// b  : half-size
// r  : per-corner radii  (tl=r.x, tr=r.y, br=r.z, bl=r.w)
fn sd_rounded_box(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    var corner_r: f32;
    if (p.x > 0.0) {
        corner_r = select(r.y, r.z, p.y > 0.0);
    } else {
        corner_r = select(r.x, r.w, p.y > 0.0);
    }
    let q = abs(p) - b + corner_r;
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - corner_r;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Sample the texture (premultiplied RGBA8).
    var color = textureSample(t_image, s_image, in.uv);

    // Apply straight tint: multiply RGB by tint.rgb, alpha by tint.a.
    // The texture is already premultiplied, so we scale the premul channels.
    color = vec4<f32>(color.rgb * in.tint.rgb, color.a * in.tint.a);

    // Corner SDF clip.
    let half_size  = in.size * 0.5;
    let max_r      = min(half_size.x, half_size.y);
    let radii      = min(in.radii, vec4<f32>(max(max_r, 0.0)));
    let p          = in.local_pos - half_size;
    let dist       = sd_rounded_box(p, half_size, radii);
    let sdf_alpha  = smoothstep(0.5, -0.5, dist);

    if (sdf_alpha <= 0.0) { discard; }

    return color * sdf_alpha;
}
