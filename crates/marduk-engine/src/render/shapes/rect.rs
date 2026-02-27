use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::paint::Paint;
use crate::render::{RenderCtx, RenderTarget};
use crate::scene::{DrawCmd, DrawList};

use super::common::{
    logical_clip_to_scissor, premul_alpha_blend, QuadVertex, ViewportUniform, QUAD_INDICES,
    QUAD_VERTICES,
};

/// Rectangle renderer (solid fill only for v0).
///
/// Geometry is provided as logical pixels, converted to NDC in the vertex shader using viewport.
/// Color is expected to be linear premultiplied RGBA (`paint::Color`).
#[derive(Default)]
pub struct RectRenderer {
    pipeline_format: Option<wgpu::TextureFormat>,
    pipeline: Option<wgpu::RenderPipeline>,

    bind_group_layout: Option<wgpu::BindGroupLayout>,
    bind_group: Option<wgpu::BindGroup>,
    viewport_ubo: Option<wgpu::Buffer>,

    quad_vbo: Option<wgpu::Buffer>,
    quad_ibo: Option<wgpu::Buffer>,

    instance_vbo: Option<wgpu::Buffer>,
    instance_capacity: usize,

    warned_non_solid: bool,
}

impl RectRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders rectangles contained in `draw_list` into `target`.
    ///
    /// Supported:
    /// - `DrawCmd::Rect` with `Paint::Solid`
    ///
    /// Unsupported paints are ignored (one-time debug message).
    pub fn render(
        &mut self,
        ctx: &RenderCtx<'_>,
        target: &mut RenderTarget<'_>,
        draw_list: &mut DrawList,
    ) {
        self.ensure_pipeline(ctx);
        self.ensure_static_buffers(ctx);
        self.ensure_bindings(ctx);

        // Build instance data from draw list in paint order.
        // Each entry pairs the GPU instance with its clip rect.
        let mut instances: Vec<(RectInstance, Option<crate::coords::Rect>)> = Vec::new();

        for item in draw_list.iter_in_paint_order() {
            let DrawCmd::Rect(cmd) = &item.cmd else { continue };
            match &cmd.paint {
                Paint::Solid(c) => {
                    let r = cmd.rect.normalized();
                    if r.is_empty() {
                        continue;
                    }
                    instances.push((
                        RectInstance {
                            origin: [r.origin.x, r.origin.y],
                            size: [r.size.x, r.size.y],
                            color: [c.r, c.g, c.b, c.a],
                        },
                        item.clip_rect,
                    ));
                }
                _ => {
                    if !self.warned_non_solid {
                        log::debug!("RectRenderer: non-solid paint encountered; ignored in v0");
                        self.warned_non_solid = true;
                    }
                }
            }
        }

        if instances.is_empty() {
            return;
        }

        // Mutating methods must happen before borrowing pipeline/buffers immutably.
        self.write_viewport_uniform(ctx);
        self.ensure_instance_capacity(ctx, instances.len());

        let Some(instance_vbo) = self.instance_vbo.as_ref() else { return };

        // Upload raw instance data (strip clip rects).
        let raw: Vec<RectInstance> = instances.iter().map(|(inst, _)| *inst).collect();
        ctx.queue.write_buffer(instance_vbo, 0, bytemuck::cast_slice(&raw));

        // Now take immutable borrows.
        let Some(pipeline) = self.pipeline.as_ref() else { return };
        let Some(bind_group) = self.bind_group.as_ref() else { return };
        let Some(quad_vbo) = self.quad_vbo.as_ref() else { return };
        let Some(quad_ibo) = self.quad_ibo.as_ref() else { return };

        let mut rpass = target.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("marduk rect pass"),
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

        // Draw one instanced call per consecutive clip-rect group.
        let mut i = 0u32;
        while i < instances.len() as u32 {
            let clip = instances[i as usize].1;
            let mut j = i + 1;
            while j < instances.len() as u32 && instances[j as usize].1 == clip {
                j += 1;
            }
            if let Some((sx, sy, sw, sh)) =
                logical_clip_to_scissor(clip, ctx.viewport, ctx.scale_factor)
            {
                rpass.set_scissor_rect(sx, sy, sw, sh);
                rpass.draw_indexed(0..6, 0, i..j);
            }
            i = j;
        }
    }

    fn ensure_pipeline(&mut self, ctx: &RenderCtx<'_>) {
        if self.pipeline_format == Some(ctx.surface_format) && self.pipeline.is_some() {
            return;
        }

        let shader_src = include_str!("shaders/rect.wgsl");
        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("marduk rect shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        let bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("marduk rect bgl"),
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
            ctx.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("marduk rect pipeline layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    // Newer wgpu uses immediate constants; keep disabled for now.
                    immediate_size: 0,
                });

        let pipeline = ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("marduk rect pipeline"),
            layout: Some(&pipeline_layout),

            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[QuadVertex::layout(), RectInstance::layout()],
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

            // Newer wgpu field names:
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
        let Some(bgl) = self.bind_group_layout.as_ref() else { return };

        let viewport_ubo = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("marduk rect viewport ubo"),
            size: std::mem::size_of::<ViewportUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("marduk rect bind group"),
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

        self.quad_vbo = Some(ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("marduk rect quad vbo"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        }));

        self.quad_ibo = Some(ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("marduk rect quad ibo"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        }));
    }

    fn write_viewport_uniform(&mut self, ctx: &RenderCtx<'_>) {
        let Some(ubo) = self.viewport_ubo.as_ref() else { return };
        let u = ViewportUniform {
            viewport: [ctx.viewport.width.max(1.0), ctx.viewport.height.max(1.0)],
            _pad: [0.0; 2],
        };
        ctx.queue.write_buffer(ubo, 0, bytemuck::bytes_of(&u));
    }

    fn ensure_instance_capacity(&mut self, ctx: &RenderCtx<'_>, required_instances: usize) {
        if required_instances <= self.instance_capacity && self.instance_vbo.is_some() {
            return;
        }

        let new_cap = required_instances.next_power_of_two().max(64);
        let new_size = (new_cap * std::mem::size_of::<RectInstance>()) as u64;

        self.instance_vbo = Some(ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("marduk rect instance vbo"),
            size: new_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.instance_capacity = new_cap;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct RectInstance {
    origin: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
}

impl RectInstance {
    const ATTRS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        1 => Float32x2, // origin
        2 => Float32x2, // size
        3 => Float32x4  // color
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<RectInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRS,
        }
    }
}
