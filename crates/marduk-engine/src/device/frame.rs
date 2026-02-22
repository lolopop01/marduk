/// Represents a single acquired frame.
///
/// This object is short-lived and must be finalized promptly. Holding the surface
/// texture prevents acquisition of subsequent frames.
pub struct GpuFrame {
    pub surface_texture: wgpu::SurfaceTexture,
    pub view: wgpu::TextureView,
    pub encoder: wgpu::CommandEncoder,
}