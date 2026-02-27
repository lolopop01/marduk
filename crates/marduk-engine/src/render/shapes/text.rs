use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};
use fontdue::layout::{CoordinateSystem, GlyphRasterConfig, Layout, LayoutSettings, TextStyle};
use wgpu::util::DeviceExt;

use crate::render::{RenderCtx, RenderTarget};
use crate::scene::{DrawCmd, DrawList};
use crate::text::FontSystem;

use super::common::{premul_alpha_blend, QuadVertex, ViewportUniform, QUAD_INDICES, QUAD_VERTICES};

// ── atlas constants ────────────────────────────────────────────────────────

const ATLAS_SIZE: u32 = 2048;
const GLYPH_PADDING: u32 = 1; // pixels between glyphs in the atlas

// ── cached glyph ──────────────────────────────────────────────────────────

struct CachedGlyph {
    uv_min: [f32; 2],
    uv_max: [f32; 2],
}

// ── renderer ──────────────────────────────────────────────────────────────

/// Renderer for `DrawCmd::Text`.
///
/// Maintains a 2048 × 2048 R8Unorm glyph atlas.  Glyphs are rasterized on
/// first use via fontdue and cached for the renderer's lifetime.
///
/// The cache key is `fontdue::layout::GlyphRasterConfig`, which encodes font
/// identity, glyph index, and pixel size — so the same glyph at the same size
/// across multiple text commands is rasterized only once.
pub struct TextRenderer {
    // pipeline
    pipeline_format: Option<wgpu::TextureFormat>,
    pipeline: Option<wgpu::RenderPipeline>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,

    // bindings (rebuilt when atlas changes)
    bind_group: Option<wgpu::BindGroup>,
    viewport_ubo: Option<wgpu::Buffer>,
    sampler: Option<wgpu::Sampler>,

    // atlas
    atlas_texture: Option<wgpu::Texture>,
    atlas_view: Option<wgpu::TextureView>,
    atlas_cursor_x: u32,
    atlas_cursor_y: u32,
    atlas_row_height: u32,
    atlas_generation: u64,
    bind_group_generation: u64,
    atlas_full: bool,

    // glyph cache
    glyph_cache: HashMap<GlyphRasterConfig, CachedGlyph>,

    // geometry
    quad_vbo: Option<wgpu::Buffer>,
    quad_ibo: Option<wgpu::Buffer>,
    instance_vbo: Option<wgpu::Buffer>,
    instance_capacity: usize,

    // reusable fontdue layout
    layout: Layout<()>,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self {
            pipeline_format: None,
            pipeline: None,
            bind_group_layout: None,
            bind_group: None,
            viewport_ubo: None,
            sampler: None,
            atlas_texture: None,
            atlas_view: None,
            atlas_cursor_x: GLYPH_PADDING,
            atlas_cursor_y: GLYPH_PADDING,
            atlas_row_height: 0,
            atlas_generation: 0,
            bind_group_generation: u64::MAX,
            atlas_full: false,
            glyph_cache: HashMap::new(),
            quad_vbo: None,
            quad_ibo: None,
            instance_vbo: None,
            instance_capacity: 0,
            layout: Layout::new(CoordinateSystem::PositiveYDown),
        }
    }
}

impl TextRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders all `DrawCmd::Text` entries in `draw_list`.
    pub fn render(
        &mut self,
        ctx: &RenderCtx<'_>,
        target: &mut RenderTarget<'_>,
        draw_list: &mut DrawList,
        font_system: &FontSystem,
    ) {
        self.ensure_pipeline(ctx);
        self.ensure_atlas(ctx);
        self.ensure_sampler(ctx);
        self.ensure_static_buffers(ctx);

        // ── collect text commands ──────────────────────────────────────────
        let text_cmds: Vec<_> = draw_list
            .iter_in_paint_order()
            .filter_map(|item| {
                if let DrawCmd::Text(cmd) = &item.cmd { Some(cmd.clone()) } else { None }
            })
            .collect();

        // ── build glyph instance list ──────────────────────────────────────
        let mut instances: Vec<GlyphInstance> = Vec::new();

        for cmd in &text_cmds {
            let Some(font) = font_system.get(cmd.font) else {
                log::warn!("TextRenderer: unknown FontId {:?}, skipping", cmd.font);
                continue;
            };

            let color = [cmd.color.r, cmd.color.g, cmd.color.b, cmd.color.a];

            self.layout.reset(&LayoutSettings {
                x: cmd.origin.x,
                y: cmd.origin.y,
                max_width: cmd.max_width,
                ..LayoutSettings::default()
            });
            self.layout.append(&[font], &TextStyle::new(&cmd.text, cmd.size, 0));

            // Snapshot glyph positions into a plain Vec so that the borrow on
            // `self.layout` ends before we call `self.try_place_glyph` (which
            // needs `&mut self`).
            let glyph_snap: Vec<(GlyphRasterConfig, f32, f32, usize, usize)> = self
                .layout
                .glyphs()
                .iter()
                .filter(|g| g.char_data.rasterize() && g.width > 0 && g.height > 0)
                .map(|g| (g.key, g.x, g.y, g.width, g.height))
                .collect();

            for (key, x, y, w, h) in glyph_snap {
                // Rasterize + upload on first encounter.
                if !self.glyph_cache.contains_key(&key) {
                    let (metrics, bitmap) = font.rasterize_config(key);
                    if metrics.width == 0 || metrics.height == 0 {
                        continue;
                    }
                    if let Some(entry) = self.try_place_glyph(
                        ctx,
                        &bitmap,
                        metrics.width as u32,
                        metrics.height as u32,
                    ) {
                        self.glyph_cache.insert(key, entry);
                    }
                }

                let Some(cached) = self.glyph_cache.get(&key) else { continue; };

                instances.push(GlyphInstance {
                    dst_min: [x, y],
                    dst_max: [x + w as f32, y + h as f32],
                    uv_min:  cached.uv_min,
                    uv_max:  cached.uv_max,
                    color,
                });
            }
        }

        if instances.is_empty() {
            return;
        }

        // ── mutable operations before any immutable borrows ────────────────
        self.ensure_bindings(ctx);
        self.write_viewport_uniform(ctx);
        self.ensure_instance_capacity(ctx, instances.len());

        let Some(instance_vbo) = self.instance_vbo.as_ref() else { return; };
        ctx.queue.write_buffer(instance_vbo, 0, bytemuck::cast_slice(&instances));

        // ── immutable borrows ──────────────────────────────────────────────
        let Some(pipeline)   = self.pipeline.as_ref()   else { return; };
        let Some(bind_group) = self.bind_group.as_ref() else { return; };
        let Some(quad_vbo)   = self.quad_vbo.as_ref()   else { return; };
        let Some(quad_ibo)   = self.quad_ibo.as_ref()   else { return; };

        let mut rpass = target.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("marduk text pass"),
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

    // ── atlas helpers ──────────────────────────────────────────────────────

    fn try_place_glyph(
        &mut self,
        ctx: &RenderCtx<'_>,
        bitmap: &[u8],
        w: u32,
        h: u32,
    ) -> Option<CachedGlyph> {
        if self.atlas_full {
            return None;
        }

        // Advance to a new shelf row when the glyph doesn't fit horizontally.
        if self.atlas_cursor_x + w + GLYPH_PADDING > ATLAS_SIZE {
            self.atlas_cursor_y += self.atlas_row_height + GLYPH_PADDING;
            self.atlas_cursor_x = GLYPH_PADDING;
            self.atlas_row_height = 0;
        }

        if self.atlas_cursor_y + h + GLYPH_PADDING > ATLAS_SIZE {
            log::warn!(
                "TextRenderer: glyph atlas is full ({ATLAS_SIZE}×{ATLAS_SIZE}); \
                 some glyphs will not be rendered"
            );
            self.atlas_full = true;
            return None;
        }

        let gx = self.atlas_cursor_x;
        let gy = self.atlas_cursor_y;

        let atlas = self.atlas_texture.as_ref()?;

        ctx.queue.write_texture(
            // wgpu 28: ImageCopyTexture → TexelCopyTextureInfo
            wgpu::TexelCopyTextureInfo {
                texture: atlas,
                mip_level: 0,
                origin: wgpu::Origin3d { x: gx, y: gy, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            bitmap,
            // wgpu 28: ImageDataLayout → TexelCopyBufferLayout
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );

        self.atlas_cursor_x += w + GLYPH_PADDING;
        self.atlas_row_height = self.atlas_row_height.max(h);

        let atlas_f = ATLAS_SIZE as f32;
        Some(CachedGlyph {
            uv_min: [gx as f32 / atlas_f,       gy as f32 / atlas_f],
            uv_max: [(gx + w) as f32 / atlas_f, (gy + h) as f32 / atlas_f],
        })
    }

    // ── lazy-init helpers ──────────────────────────────────────────────────

    fn ensure_pipeline(&mut self, ctx: &RenderCtx<'_>) {
        if self.pipeline_format == Some(ctx.surface_format) && self.pipeline.is_some() {
            return;
        }

        let shader = ctx.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("marduk text shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/text.wgsl").into()),
        });

        let bgl = ctx.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("marduk text bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
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
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = ctx.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("marduk text pipeline layout"),
            bind_group_layouts: &[&bgl],
            immediate_size: 0,
        });

        let pipeline = ctx.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("marduk text pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[QuadVertex::layout(), GlyphInstance::layout()],
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
        self.bind_group_layout = Some(bgl);
        self.bind_group = None;
        self.viewport_ubo = None;
        self.bind_group_generation = u64::MAX;
    }

    fn ensure_atlas(&mut self, ctx: &RenderCtx<'_>) {
        if self.atlas_texture.is_some() {
            return;
        }

        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("marduk text atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE,
                height: ATLAS_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.atlas_view     = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
        self.atlas_texture  = Some(texture);
        self.atlas_generation += 1;
        self.atlas_cursor_x = GLYPH_PADDING;
        self.atlas_cursor_y = GLYPH_PADDING;
        self.atlas_row_height = 0;
        self.atlas_full = false;
    }

    fn ensure_sampler(&mut self, ctx: &RenderCtx<'_>) {
        if self.sampler.is_some() {
            return;
        }
        self.sampler = Some(ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("marduk text sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest, // fix: MipmapFilterMode
            ..Default::default()
        }));
    }

    fn ensure_bindings(&mut self, ctx: &RenderCtx<'_>) {
        if self.bind_group_generation == self.atlas_generation
            && self.bind_group.is_some()
            && self.viewport_ubo.is_some()
        {
            return;
        }

        let Some(bgl)        = self.bind_group_layout.as_ref() else { return; };
        let Some(atlas_view) = self.atlas_view.as_ref()        else { return; };
        let Some(sampler)    = self.sampler.as_ref()            else { return; };

        let viewport_ubo = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("marduk text viewport ubo"),
            size: std::mem::size_of::<ViewportUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("marduk text bind group"),
            layout: bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: viewport_ubo.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });

        self.viewport_ubo = Some(viewport_ubo);
        self.bind_group   = Some(bind_group);
        self.bind_group_generation = self.atlas_generation;
    }

    fn ensure_static_buffers(&mut self, ctx: &RenderCtx<'_>) {
        if self.quad_vbo.is_some() && self.quad_ibo.is_some() {
            return;
        }
        self.quad_vbo = Some(ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("marduk text quad vbo"),
            contents: bytemuck::cast_slice(&QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        }));
        self.quad_ibo = Some(ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("marduk text quad ibo"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
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
        let new_cap  = required.next_power_of_two().max(64);
        let new_size = (new_cap * std::mem::size_of::<GlyphInstance>()) as u64;
        self.instance_vbo = Some(ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("marduk text instance vbo"),
            size: new_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.instance_capacity = new_cap;
    }
}

// ── GPU types ─────────────────────────────────────────────────────────────

/// Instance data layout (48 bytes):
///
///  offset  0  dst_min  [f32; 2]   loc 1
///  offset  8  dst_max  [f32; 2]   loc 2
///  offset 16  uv_min   [f32; 2]   loc 3
///  offset 24  uv_max   [f32; 2]   loc 4
///  offset 32  color    [f32; 4]   loc 5
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct GlyphInstance {
    dst_min: [f32; 2],
    dst_max: [f32; 2],
    uv_min:  [f32; 2],
    uv_max:  [f32; 2],
    color:   [f32; 4],
}

impl GlyphInstance {
    const ATTRS: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        1 => Float32x2, // dst_min
        2 => Float32x2, // dst_max
        3 => Float32x2, // uv_min
        4 => Float32x2, // uv_max
        5 => Float32x4  // color
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GlyphInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRS,
        }
    }
}
