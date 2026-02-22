use crate::paint::Paint;
use crate::render::{RenderCtx, RenderTarget};
use crate::scene::{DrawCmd, DrawList};

use bytemuck::{Pod, Zeroable};

/// Rectangle renderer (solid fill only for v0).
///
/// Geometry is provided as logical pixels, converted to NDC in the vertex shader using viewport.
/// Color is expected to be linear premultiplied RGBA (`paint::Color`).
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

    // Set to true once a non-solid paint is encountered (avoid log spam).
    warned_non_solid: bool,
}

impl Default for RectRenderer {
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
            warned_non_solid: false,
        }
    }
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
    /// Unsupported paints are ignored (with one-time warning hook).
    pub fn render(&mut self, ctx: &RenderCtx<'_>, target: &mut RenderTarget<'_>, draw_list: &mut DrawList) {
        self.ensure_pipeline(ctx);
        self.ensure_static_buffers(ctx);
        self.ensure_bindings(ctx);

        let Some(pipeline) = self.pipeline.as_ref() else { return; };
        let Some(bind_group) = self.bind_group.as_ref() else { return; };
        let Some(quad_vbo) = self.quad_vbo.as_ref() else { return; };
        let Some(quad_ibo) = self.quad_ibo.as_ref() else { return; };

        // Build instance data from draw list in paint order.
        // No heap allocation besides the Vec itself; callers can reuse a Vec in the future if needed.
        let mut instances: Vec<RectInstance> = Vec::new();

        for item in draw_list.iter_in_paint_order() {
            match &item.cmd {
                DrawCmd::Rect(cmd) => match &cmd.paint {
                    Paint::Solid(c) => {
                        // Skip empty rects early.
                        let r = cmd.rect.normalized();
                        if r.is_empty() {
                            continue;
                        }

                        instances.push(RectInstance {
                            origin: [r.origin.x, r.origin.y],
                            size: [r.size.x, r.size.y],
                            color: [c.r, c.g, c.b, c.a],
                        });
                    }
                    _ => {
                        if !self.warned_non_solid {
                            log::debug!("RectRenderer: non-solid paint encountered; ignored in v0");
                            self.warned_non_solid = true;
                        }
                    }
                },
                _ => {}
            }
        }

        if instances.is_empty() {
            return;
        }

        // Upload viewport uniform (logical px).
        self.write_viewport_uniform(ctx);

        // Upload instance buffer (resized if needed).
        self.ensure_instance_capacity(ctx, instances.len());
        let Some(instance_vbo) = self.instance_vbo.as_ref() else { return; };

        ctx.queue
            .write_buffer(instance_vbo, 0, bytemuck::cast_slice(&instances));

        // Draw pass.
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

        // 6 indices per quad, instanced per rect.
        rpass.draw_indexed(0..6, 0, 0..instances.len() as u32);
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

        let bind_group_layout = ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("marduk rect bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            std::num::NonZeroU64::new(std::mem::size_of::<ViewportUniform>() as u64)
                                .unwrap(),
                        ),
                    },
                    count: None,
                }],
            });

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("marduk rect pipeline layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("marduk rect pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    QuadVertex::layout(),
                    RectInstance::layout(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
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
            multiview: None,
        });

        self.pipeline_format = Some(ctx.surface_format);
        self.pipeline = Some(pipeline);
        self.bind_group_layout = Some(bind_group_layout);

        // Force re-bind creation on next call.
        self.bind_group = None;
        self.viewport_ubo = None;
    }

    fn ensure_bindings(&mut self, ctx: &RenderCtx<'_>) {
        if self.bind_group.is_some() && self.viewport_ubo.is_some() {
            return;
        }
        let Some(bgl) = self.bind_group_layout.as_ref() else { return; };

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

        // Unit quad vertices (0..1) in local rect space.
        let quad = [
            QuadVertex { pos: [0.0, 0.0] },
            QuadVertex { pos: [1.0, 0.0] },
            QuadVertex { pos: [1.0, 1.0] },
            QuadVertex { pos: [0.0, 1.0] },
        ];

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

        let quad_vbo = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("marduk rect quad vbo"),
            contents: bytemuck::cast_slice(&quad),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let quad_ibo = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("marduk rect quad ibo"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        self.quad_vbo = Some(quad_vbo);
        self.quad_ibo = Some(quad_ibo);
    }

    fn write_viewport_uniform(&mut self, ctx: &RenderCtx<'_>) {
        let Some(ubo) = self.viewport_ubo.as_ref() else { return; };

        // Guard against invalid viewport.
        let w = ctx.viewport.width.max(1.0);
        let h = ctx.viewport.height.max(1.0);

        let u = ViewportUniform {
            viewport: [w, h],
            _pad: [0.0, 0.0],
        };

        ctx.queue.write_buffer(ubo, 0, bytemuck::bytes_of(&u));
    }

    fn ensure_instance_capacity(&mut self, ctx: &RenderCtx<'_>, required_instances: usize) {
        if required_instances <= self.instance_capacity && self.instance_vbo.is_some() {
            return;
        }

        // Growth policy: next power of two to reduce realloc churn.
        let new_cap = required_instances.next_power_of_two().max(64);
        let new_size = (new_cap * std::mem::size_of::<RectInstance>()) as u64;

        let buf = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("marduk rect instance vbo"),
            size: new_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.instance_vbo = Some(buf);
        self.instance_capacity = new_cap;
    }
}

fn premul_alpha_blend() -> wgpu::BlendState {
    // Premultiplied alpha:
    // out.rgb = src.rgb + dst.rgb*(1-src.a)
    // out.a   = src.a   + dst.a*(1-src.a)
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

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct ViewportUniform {
    viewport: [f32; 2],
    // Pad to 16 bytes to satisfy uniform alignment on most backends.
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct QuadVertex {
    pos: [f32; 2], // 0..1
}

impl QuadVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2],
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct RectInstance {
    origin: [f32; 2], // logical px
    size: [f32; 2],   // logical px
    color: [f32; 4],  // premultiplied RGBA
}

impl RectInstance {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<RectInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![
                1 => Float32x2, // origin
                2 => Float32x2, // size
                3 => Float32x4  // color
            ],
        }
    }
}

// wgpu util import for create_buffer_init
use wgpu::util::DeviceExt;