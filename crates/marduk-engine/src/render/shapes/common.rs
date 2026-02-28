//! Shared GPU types and utilities used by all shape renderers.

use bytemuck::{Pod, Zeroable};

use crate::coords::{Rect, Viewport};
use crate::paint::Paint;

// ── blend ─────────────────────────────────────────────────────────────────

pub(super) fn premul_alpha_blend() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

// ── viewport uniform ──────────────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub(super) struct ViewportUniform {
    pub viewport: [f32; 2],
    pub _pad: [f32; 2], // 16-byte alignment
}

// ── quad vertex ───────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub(super) struct QuadVertex {
    pub pos: [f32; 2], // 0..1
}

impl QuadVertex {
    const ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];

    pub(super) fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

pub(super) const QUAD_VERTICES: [QuadVertex; 4] = [
    QuadVertex { pos: [0.0, 0.0] },
    QuadVertex { pos: [1.0, 0.0] },
    QuadVertex { pos: [1.0, 1.0] },
    QuadVertex { pos: [0.0, 1.0] },
];

pub(super) const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

// ── scissor rect ──────────────────────────────────────────────────────────

/// Converts a logical-pixel clip rect to physical scissor rect arguments for wgpu.
///
/// Returns `None` if the clip rect is zero-area (renderer should skip the draw call).
/// Returns `Some((x, y, w, h))` in physical pixels, clamped to the viewport.
///
/// `clip = None` means "no scissor" → returns the full viewport rect.
pub(super) fn logical_clip_to_scissor(
    clip: Option<Rect>,
    viewport: Viewport,
    scale: f32,
) -> Option<(u32, u32, u32, u32)> {
    let phys_vw = (viewport.width * scale).max(1.0) as u32;
    let phys_vh = (viewport.height * scale).max(1.0) as u32;

    let (x, y, w, h) = match clip {
        None => (0, 0, phys_vw, phys_vh),
        Some(r) => {
            let x  = ((r.origin.x * scale).max(0.0) as u32).min(phys_vw);
            let y  = ((r.origin.y * scale).max(0.0) as u32).min(phys_vh);
            let x2 = (((r.origin.x + r.size.x) * scale).max(0.0) as u32).min(phys_vw);
            let y2 = (((r.origin.y + r.size.y) * scale).max(0.0) as u32).min(phys_vh);
            (x, y, x2.saturating_sub(x), y2.saturating_sub(y))
        }
    };

    if w == 0 || h == 0 { None } else { Some((x, y, w, h)) }
}

// ── viewport UBO binding size ─────────────────────────────────────────────

/// Returns the `wgpu` minimum binding size for the viewport uniform buffer.
///
/// `ViewportUniform` contains two `[f32; 2]` fields (16 bytes total) so its
/// size is always non-zero. Centralising this avoids `.unwrap()` at each
/// renderer's pipeline-creation site.
pub(super) fn viewport_ubo_min_binding_size() -> std::num::NonZeroU64 {
    std::num::NonZeroU64::new(std::mem::size_of::<ViewportUniform>() as u64)
        .expect("ViewportUniform has non-zero size by construction")
}

// ── paint resolution ──────────────────────────────────────────────────────

/// Converts a `Paint` to `(color0, color1, grad_p0, grad_p1)` for gradient-capable shaders.
///
/// Solid fills produce identical colors and a degenerate (zero-length) gradient
/// axis, so the shader falls back to `color0` as a uniform fill.
///
/// Linear gradients are clamped to 2 stops (first and last); more stops are
/// unsupported and emit a one-time debug message.
pub(super) fn resolve_paint(
    paint: &Paint,
    warned_multi_stop: &mut bool,
) -> ([f32; 4], [f32; 4], [f32; 2], [f32; 2]) {
    match paint {
        Paint::Solid(c) => {
            let col = [c.r, c.g, c.b, c.a];
            (col, col, [0.0, 0.0], [0.0, 0.0])
        }
        Paint::LinearGradient(g) => {
            // Degenerate gradient (< 2 stops): treat as solid using the first stop.
            if g.stops.len() < 2 {
                let col = g
                    .stops
                    .first()
                    .map_or([0.0f32; 4], |s| [s.color.r, s.color.g, s.color.b, s.color.a]);
                return (col, col, [0.0, 0.0], [0.0, 0.0]);
            }
            if g.stops.len() > 2 && !*warned_multi_stop {
                log::debug!("only 2-stop gradients supported; using first and last stop");
                *warned_multi_stop = true;
            }
            // Safety: len >= 2 is checked above, so first/last always exist.
            let c0 = g.stops[0].color;
            let c1 = g.stops[g.stops.len() - 1].color;
            (
                [c0.r, c0.g, c0.b, c0.a],
                [c1.r, c1.g, c1.b, c1.a],
                [g.start.x, g.start.y],
                [g.end.x, g.end.y],
            )
        }
    }
}
