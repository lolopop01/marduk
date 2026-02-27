//! Shared GPU types and utilities used by all shape renderers.

use bytemuck::{Pod, Zeroable};

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

// ── paint resolution ──────────────────────────────────────────────────────

/// Converts a `Paint` to `(color0, color1, grad_p0, grad_p1)` for gradient-capable shaders.
///
/// Solid fills produce identical colors and degenerate gradient points (zero-length),
/// so the shader falls back to `color0` as the uniform fill.
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
            let c0 = g.stops.first().unwrap().color;
            let c1 = g.stops.last().unwrap().color;
            (
                [c0.r, c0.g, c0.b, c0.a],
                [c1.r, c1.g, c1.b, c1.a],
                [g.start.x, g.start.y],
                [g.end.x, g.end.y],
            )
        }
    }
}
