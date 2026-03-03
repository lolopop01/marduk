use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::image::{ImageId, ImageStore};
use crate::render::{RenderCtx, RenderTarget};
use crate::scene::{DrawCmd, DrawList};

use super::common::{
    logical_clip_to_scissor, premul_alpha_blend, viewport_ubo_min_binding_size,
    QuadVertex, ViewportUniform, QUAD_INDICES, QUAD_VERTICES,
};

// ── Per-GPU-image state ───────────────────────────────────────────────────

struct GpuImage {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    #[allow(dead_code)]
    view: wgpu::TextureView,
    bind_group: wgpu::BindGroup,
    /// The `CpuImage::version` that was current when this texture was uploaded.
    /// If the CPU version advances (SVG re-rasterized at new scale), we re-upload.
    version: u64,
}

// ── ImageRenderer ─────────────────────────────────────────────────────────

/// Renderer for `DrawCmd::Image`.
///
/// Textures are uploaded lazily: the first time a given [`ImageId`] appears in
/// the draw list the renderer uploads the CPU image from [`ImageStore`] to the
/// GPU and caches the resulting texture + bind group.
#[derive(Default)]
pub struct ImageRenderer {
    pipeline_format: Option<wgpu::TextureFormat>,
    pipeline: Option<wgpu::RenderPipeline>,

    bgl_viewport: Option<wgpu::BindGroupLayout>,
    bgl_image: Option<wgpu::BindGroupLayout>,

    bg_viewport: Option<wgpu::BindGroup>,
    viewport_ubo: Option<wgpu::Buffer>,

    sampler: Option<wgpu::Sampler>,

    quad_vbo: Option<wgpu::Buffer>,
    quad_ibo: Option<wgpu::Buffer>,

    instance_vbo: Option<wgpu::Buffer>,
    instance_capacity: usize,

    gpu_images: HashMap<ImageId, GpuImage>,
}

impl ImageRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(
        &mut self,
        ctx: &RenderCtx<'_>,
        target: &mut RenderTarget<'_>,
        draw_list: &mut DrawList,
        store: &ImageStore,
    ) {
        self.ensure_pipeline(ctx);
        self.ensure_static_buffers(ctx);
        self.ensure_viewport_binding(ctx);

        // Collect instances, grouping by ImageId so we can batch by texture.
        // Each entry: (instance_data, image_id, clip_rect)
        let mut instances: Vec<(ImageInstance, ImageId, Option<crate::coords::Rect>)> = Vec::new();

        for item in draw_list.iter_in_paint_order() {
            let DrawCmd::Image(cmd) = &item.cmd else { continue };

            let r = cmd.rect.normalized();
            if r.is_empty() {
                continue;
            }

            // Ensure the GPU image exists for this id.
            self.ensure_gpu_image(ctx, cmd.image_id, store);

            let rd = cmd.corner_radii;
            instances.push((
                ImageInstance {
                    origin: [r.origin.x, r.origin.y],
                    size: [r.size.x, r.size.y],
                    uv_min: cmd.uv_min,
                    uv_max: cmd.uv_max,
                    tint: cmd.tint,
                    radii: [rd.top_left, rd.top_right, rd.bottom_right, rd.bottom_left],
                },
                cmd.image_id,
                item.clip_rect,
            ));
        }

        if instances.is_empty() {
            return;
        }

        self.write_viewport_uniform(ctx);
        self.ensure_instance_capacity(ctx, instances.len());

        let Some(pipeline) = self.pipeline.as_ref() else { return };
        let Some(bg_viewport) = self.bg_viewport.as_ref() else { return };
        let Some(quad_vbo) = self.quad_vbo.as_ref() else { return };
        let Some(quad_ibo) = self.quad_ibo.as_ref() else { return };
        let Some(instance_vbo) = self.instance_vbo.as_ref() else { return };

        // Upload all instances.
        let raw: Vec<ImageInstance> = instances.iter().map(|(inst, _, _)| *inst).collect();
        ctx.queue.write_buffer(instance_vbo, 0, bytemuck::cast_slice(&raw));

        let mut rpass = target.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("marduk image pass"),
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
        rpass.set_bind_group(0, bg_viewport, &[]);
        rpass.set_vertex_buffer(0, quad_vbo.slice(..));
        rpass.set_vertex_buffer(1, instance_vbo.slice(..));
        rpass.set_index_buffer(quad_ibo.slice(..), wgpu::IndexFormat::Uint16);

        // Draw each instance individually (each may have a different texture).
        // Group consecutive instances with the same image_id AND same clip.
        let mut i = 0u32;
        while i < instances.len() as u32 {
            let (_, img_id, clip) = &instances[i as usize];
            let img_id = *img_id;
            let clip = *clip;

            // Extend run only if same image_id and same clip.
            let mut j = i + 1;
            while j < instances.len() as u32 {
                let (_, jid, jclip) = &instances[j as usize];
                if *jid != img_id || *jclip != clip { break; }
                j += 1;
            }

            if let (Some(gpu_img), Some((sx, sy, sw, sh))) = (
                self.gpu_images.get(&img_id),
                logical_clip_to_scissor(clip, ctx.viewport, ctx.scale_factor),
            ) {
                rpass.set_scissor_rect(sx, sy, sw, sh);
                rpass.set_bind_group(1, &gpu_img.bind_group, &[]);
                rpass.draw_indexed(0..6, 0, i..j);
            }
            i = j;
        }
    }

    // ── private helpers ────────────────────────────────────────────────────

    fn ensure_pipeline(&mut self, ctx: &RenderCtx<'_>) {
        if self.pipeline_format == Some(ctx.surface_format) && self.pipeline.is_some() {
            return;
        }

        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("marduk image shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/image.wgsl").into()),
        });

        let bgl_viewport =
            ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("marduk image bgl viewport"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(viewport_ubo_min_binding_size()),
                    },
                    count: None,
                }],
            });

        let bgl_image =
            ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("marduk image bgl texture"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let pipeline_layout =
            ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("marduk image pipeline layout"),
                bind_group_layouts: &[&bgl_viewport, &bgl_image],
                immediate_size: 0,
            });

        let pipeline = ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("marduk image pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[QuadVertex::layout(), ImageInstance::layout()],
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

        // Invalidate derived state.
        self.pipeline_format = Some(ctx.surface_format);
        self.pipeline = Some(pipeline);
        self.bgl_viewport = Some(bgl_viewport);
        self.bgl_image = Some(bgl_image);
        self.bg_viewport = None;
        self.viewport_ubo = None;
        self.sampler = None;
        self.gpu_images.clear();
    }

    fn ensure_static_buffers(&mut self, ctx: &RenderCtx<'_>) {
        if self.quad_vbo.is_some() && self.quad_ibo.is_some() {
            return;
        }
        self.quad_vbo = Some(ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("marduk image quad vbo"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        }));
        self.quad_ibo = Some(ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("marduk image quad ibo"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        }));
    }

    fn ensure_viewport_binding(&mut self, ctx: &RenderCtx<'_>) {
        if self.bg_viewport.is_some() && self.viewport_ubo.is_some() {
            return;
        }
        let Some(bgl) = self.bgl_viewport.as_ref() else { return };

        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("marduk image sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        self.sampler = Some(sampler);

        let viewport_ubo = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("marduk image viewport ubo"),
            size: std::mem::size_of::<ViewportUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bg_viewport = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("marduk image viewport bind group"),
            layout: bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: viewport_ubo.as_entire_binding(),
            }],
        });
        self.viewport_ubo = Some(viewport_ubo);
        self.bg_viewport = Some(bg_viewport);
    }

    fn write_viewport_uniform(&mut self, ctx: &RenderCtx<'_>) {
        let Some(ubo) = self.viewport_ubo.as_ref() else { return };
        ctx.queue.write_buffer(
            ubo,
            0,
            bytemuck::bytes_of(&ViewportUniform {
                viewport: [ctx.viewport.width.max(1.0), ctx.viewport.height.max(1.0)],
                _pad: [0.0; 2],
            }),
        );
    }

    fn ensure_instance_capacity(&mut self, ctx: &RenderCtx<'_>, required: usize) {
        if required <= self.instance_capacity && self.instance_vbo.is_some() {
            return;
        }
        let new_cap = required.next_power_of_two().max(16);
        let new_size = (new_cap * std::mem::size_of::<ImageInstance>()) as u64;
        self.instance_vbo = Some(ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("marduk image instance vbo"),
            size: new_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.instance_capacity = new_cap;
    }

    fn ensure_gpu_image(&mut self, ctx: &RenderCtx<'_>, id: ImageId, store: &ImageStore) {
        let Some(cpu) = store.get(id) else { return };
        // Check if already up-to-date.
        if let Some(gpu) = self.gpu_images.get(&id) {
            if gpu.version == cpu.version {
                return;
            }
            // CPU image was updated (e.g. SVG re-rasterized at new scale) — re-upload.
            self.gpu_images.remove(&id);
        }
        let Some(bgl_image) = self.bgl_image.as_ref() else { return };
        let Some(sampler) = self.sampler.as_ref() else { return };

        let size = wgpu::Extent3d {
            width: cpu.width.max(1),
            height: cpu.height.max(1),
            depth_or_array_layers: 1,
        };
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("marduk image texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &cpu.pixels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(cpu.width.max(1) * 4),
                rows_per_image: None,
            },
            size,
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("marduk image bind group"),
            layout: bgl_image,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        self.gpu_images.insert(id, GpuImage { texture, view, bind_group, version: cpu.version });
    }
}

// ── GPU instance layout ───────────────────────────────────────────────────

/// Instance data layout (80 bytes):
///
///  offset  0  origin   [f32; 2]   loc 1
///  offset  8  size     [f32; 2]   loc 2
///  offset 16  uv_min   [f32; 2]   loc 3
///  offset 24  uv_max   [f32; 2]   loc 4
///  offset 32  tint     [f32; 4]   loc 5
///  offset 48  radii    [f32; 4]   loc 6  (tl, tr, br, bl)
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct ImageInstance {
    origin: [f32; 2],
    size: [f32; 2],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    tint: [f32; 4],
    radii: [f32; 4],
}

impl ImageInstance {
    const ATTRS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        1 => Float32x2, // origin
        2 => Float32x2, // size
        3 => Float32x2, // uv_min
        4 => Float32x2, // uv_max
        5 => Float32x4, // tint
        6 => Float32x4  // radii
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ImageInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRS,
        }
    }
}
