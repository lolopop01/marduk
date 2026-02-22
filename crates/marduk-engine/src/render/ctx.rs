use crate::coords::Viewport;

/// Renderer-facing context (device/queue + surface format + viewport).
///
/// This is intentionally small and stable.
pub struct RenderCtx<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub surface_format: wgpu::TextureFormat,
    pub viewport: Viewport, // logical px
}

impl<'a> RenderCtx<'a> {
    #[inline]
    pub fn new(
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        viewport: Viewport,
    ) -> Self {
        Self {
            device,
            queue,
            surface_format,
            viewport,
        }
    }
}

/// Target for drawing (encoder + color view).
pub struct RenderTarget<'a> {
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub color_view: &'a wgpu::TextureView,
}

impl<'a> RenderTarget<'a> {
    #[inline]
    pub fn new(encoder: &'a mut wgpu::CommandEncoder, color_view: &'a wgpu::TextureView) -> Self {
        Self { encoder, color_view }
    }
}