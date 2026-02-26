use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::paint::Paint;
use crate::render::{RenderCtx, RenderTarget};
use crate::scene::{DrawCmd, DrawList};

/// Renderer for `DrawCmd::RoundedRect`.
///
/// Supported paints:
/// - `Paint::Solid`
/// - `Paint::LinearGradient` (2-stop; uses first and last stop for gradients with more stops)
///
/// Borders are rendered as an AA ring on the outer edge of the shape.
pub struct RoundedRectRenderer {
    pipeline_format: Option<wgpu::TextureFormat>,
    pipeline: Option<wgpu::RenderPipeline>,

    bind_group_layout: Option<wgpu::BindGroupLayout>,
    bind_group: Option<wgpu::BindGroup>,
    viewport_ubo: Option<wgpu::Buffer>,

    quad_vbo: Option<wgpu::Buffer>,
    quad_ibo: Option<wgpu::Buffer>,

    instance_vbo: Option<wgpu::Buffer>,
    instance_capacity: usize,

    warned_multi_stop: bool,
}

impl Default for RoundedRectRenderer {
    fn default() -> Self {
        Self {
            pipeline_format: None,
            pipeline: None,
            bind_group_layout: None,
            bind_group: None,
            viewport_ubo: None,
            quad_vbo: None,
            quad_ibo: None,
            instance_vbo: None,
            instance_capacity: 0,
            warned_multi_stop: false,
        }
    }
}

impl RoundedRectRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(
        &mut self,
        ctx: &RenderCtx<'_>,
        target: &mut RenderTarget<'_>,
        draw_list: &mut DrawList,
    ) {
        self.ensure_pipeline(ctx);
        self.ensure_static_buffers(ctx);
        self.ensure_bindings(ctx);

        let mut instances: Vec<RoundedRectInstance> = Vec::new();

        for item in draw_list.iter_in_paint_order() {
            let DrawCmd::RoundedRect(cmd) = &item.cmd else { continue; };

            let r = cmd.rect.normalized();
            if r.is_empty() {
                continue;
            }

            let (color0, color1, grad_p0, grad_p1) =
                resolve_paint(&cmd.paint, &mut self.warned_multi_stop);

            let rd = cmd.radii;
            let (border_width, border_color) = match &cmd.border {
                Some(b) => (b.width.max(0.0), [b.color.r, b.color.g, b.color.b, b.color.a]),
                None => (0.0, [0.0f32; 4]),
            };

            instances.push(RoundedRectInstance {
                origin: [r.origin.x, r.origin.y],
                size: [r.size.x, r.size.y],
                radii: [rd.top_left, rd.top_right, rd.bottom_right, rd.bottom_left],
                color0,
                color1,
                grad_p0,
                grad_p1,
                border_width_pad: [border_width, 0.0],
                border_color,
            });
        }

        if instances.is_empty() {
            return;
        }

        self.write_viewport_uniform(ctx);
        self.ensure_instance_capacity(ctx, instances.len());

        let Some(instance_vbo) = self.instance_vbo.as_ref() else { return; };
        ctx.queue.write_buffer(instance_vbo, 0, bytemuck::cast_slice(&instances));

        let Some(pipeline)   = self.pipeline.as_ref()   else { return; };
        let Some(bind_group) = self.bind_group.as_ref() else { return; };
        let Some(quad_vbo)   = self.quad_vbo.as_ref()   else { return; };
        let Some(quad_ibo)   = self.quad_ibo.as_ref()   else { return; };

        let mut rpass = target.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("marduk rounded_rect pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        rpass.set_pipeline(pipeline);
        rpass.set_bind_group(0, bind_group, &[]);
        rpass.set_vertex_buffer(0, quad_vbo.slice(..));
        rpass.set_vertex_buffer(1, instance_vbo.slice(..));
        rpass.set_index_buffer(quad_ibo.slice(..), wgpu::IndexFormat::Uint16);
        rpass.draw_indexed(0..6, 0, 0..instances.len() as u32);
    }

    // ── private helpers ────────────────────────────────────────────────────

    fn ensure_pipeline(&mut self, ctx: &RenderCtx<'_>) {
        if self.pipeline_format == Some(ctx.surface_format) && self.pipeline.is_some() {
            return;
        }

        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("marduk rounded_rect shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/rounded_rect.wgsl").into(),
            ),
        });

        let bind_group_layout =
            ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("marduk rounded_rect bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            std::num::NonZeroU64::new(
                                std::mem::size_of::<ViewportUniform>() as u64,
                            )
                            .unwrap(),
                        ),
                    },
                    count: None,
                }],
            });

        let pipeline_layout =
            ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("marduk rounded_rect pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                immediate_size: 0,
            });

        let pipeline = ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("marduk rounded_rect pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[QuadVertex::layout(), RoundedRectInstance::layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: ctx.surface_format,
                    blend: Some(premul_alpha_blend()),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        self.pipeline_format = Some(ctx.surface_format);
        self.pipeline = Some(pipeline);
        self.bind_group_layout = Some(bind_group_layout);
        self.bind_group = None;
        self.viewport_ubo = None;
    }

    fn ensure_bindings(&mut self, ctx: &RenderCtx<'_>) {
        if self.bind_group.is_some() && self.viewport_ubo.is_some() {
            return;
        }
        let Some(bgl) = self.bind_group_layout.as_ref() else { return; };

        let viewport_ubo = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("marduk rounded_rect viewport ubo"),
            size: std::mem::size_of::<ViewportUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("marduk rounded_rect bind group"),
            layout: bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_ubo.as_entire_binding(),
            }],
        });

        self.viewport_ubo = Some(viewport_ubo);
        self.bind_group = Some(bind_group);
    }

    fn ensure_static_buffers(&mut self, ctx: &RenderCtx<'_>) {
        if self.quad_vbo.is_some() && self.quad_ibo.is_some() {
            return;
        }

        let quad = [
            QuadVertex { pos: [0.0, 0.0] },
            QuadVertex { pos: [1.0, 0.0] },
            QuadVertex { pos: [1.0, 1.0] },
            QuadVertex { pos: [0.0, 1.0] },
        ];
        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        self.quad_vbo = Some(ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("marduk rounded_rect quad vbo"),
            contents: bytemuck::cast_slice(&quad),
            usage: wgpu::BufferUsages::VERTEX,
        }));
        self.quad_ibo = Some(ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("marduk rounded_rect quad ibo"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        }));
    }

    fn write_viewport_uniform(&mut self, ctx: &RenderCtx<'_>) {
        let Some(ubo) = self.viewport_ubo.as_ref() else { return; };
        let w = ctx.viewport.width.max(1.0);
        let h = ctx.viewport.height.max(1.0);
        ctx.queue.write_buffer(
            ubo,
            0,
            bytemuck::bytes_of(&ViewportUniform { viewport: [w, h], _pad: [0.0; 2] }),
        );
    }

    fn ensure_instance_capacity(&mut self, ctx: &RenderCtx<'_>, required: usize) {
        if required <= self.instance_capacity && self.instance_vbo.is_some() {
            return;
        }
        let new_cap = required.next_power_of_two().max(64);
        let new_size = (new_cap * std::mem::size_of::<RoundedRectInstance>()) as u64;
        self.instance_vbo = Some(ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("marduk rounded_rect instance vbo"),
            size: new_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.instance_capacity = new_cap;
    }
}

// ── paint helpers ──────────────────────────────────────────────────────────

/// Converts a `Paint` to the (color0, color1, grad_p0, grad_p1) tuple used by the shader.
///
/// For solid fills, both colors are identical and the gradient points are degenerate
/// (zero-length), so the shader falls back to color0.
fn resolve_paint(
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
                log::debug!(
                    "RoundedRectRenderer: only 2-stop gradients supported; \
                     using first and last stop"
                );
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

// ── blend state ───────────────────────────────────────────────────────────

fn premul_alpha_blend() -> wgpu::BlendState {
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

// ── GPU types ─────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct ViewportUniform {
    viewport: [f32; 2],
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct QuadVertex {
    pos: [f32; 2],
}

impl QuadVertex {
    const ATTRS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRS,
        }
    }
}

/// Instance data layout (104 bytes):
///
///  offset  0  origin          [f32; 2]   loc 1
///  offset  8  size            [f32; 2]   loc 2
///  offset 16  radii           [f32; 4]   loc 3  (tl, tr, br, bl)
///  offset 32  color0          [f32; 4]   loc 4
///  offset 48  color1          [f32; 4]   loc 5
///  offset 64  grad_p0         [f32; 2]   loc 6
///  offset 72  grad_p1         [f32; 2]   loc 7
///  offset 80  border_width_pad[f32; 2]   loc 8  (.x = border_width)
///  offset 88  border_color    [f32; 4]   loc 9
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct RoundedRectInstance {
    origin: [f32; 2],
    size: [f32; 2],
    radii: [f32; 4],
    color0: [f32; 4],
    color1: [f32; 4],
    grad_p0: [f32; 2],
    grad_p1: [f32; 2],
    border_width_pad: [f32; 2],
    border_color: [f32; 4],
}

impl RoundedRectInstance {
    const ATTRS: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
        1 => Float32x2, // origin
        2 => Float32x2, // size
        3 => Float32x4, // radii
        4 => Float32x4, // color0
        5 => Float32x4, // color1
        6 => Float32x2, // grad_p0
        7 => Float32x2, // grad_p1
        8 => Float32x2, // border_width_pad
        9 => Float32x4  // border_color
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<RoundedRectInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRS,
        }
    }
}
