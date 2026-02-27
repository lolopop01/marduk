use crate::coords::Viewport;

/// Renderer-facing context (device/queue + surface format + viewport + scale factor).
///
/// This is intentionally small and stable.
pub struct RenderCtx<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub surface_format: wgpu::TextureFormat,
    /// Viewport in logical pixels.
    pub viewport: Viewport,
    /// Physical-pixel scale factor (e.g. 2.0 on a HiDPI display).
    /// Used by renderers to convert logical clip rects to physical scissor rects.
    pub scale_factor: f32,
}

impl<'a> RenderCtx<'a> {
    #[inline]
    pub fn new(
        device: &'a wgpu::Device,
        queue: &'a wgpu::Queue,
        surface_format: wgpu::TextureFormat,
        viewport: Viewport,
        scale_factor: f32,
    ) -> Self {
        Self {
            device,
            queue,
            surface_format,
            viewport,
            scale_factor,
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
