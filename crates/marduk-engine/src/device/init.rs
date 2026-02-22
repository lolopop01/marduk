/// Initialization parameters for the GPU layer.
///
/// Keep this structure stable and minimal. Add configuration flags only when a
/// concrete platform or backend requirement exists.
#[derive(Debug, Clone)]
pub struct GpuInit {
    /// Prefer an sRGB surface format when available.
    ///
    /// sRGB is typically required for correct UI color output.
    pub prefer_srgb: bool,

    /// Present mode (swap behavior).
    ///
    /// FIFO is broadly supported and generally appropriate for UI workloads.
    pub present_mode: wgpu::PresentMode,

    /// Optional alpha mode preference for the surface.
    ///
    /// If provided but unsupported on the current surface, a supported mode is selected.
    pub alpha_mode: Option<wgpu::CompositeAlphaMode>,

    /// Required wgpu features.
    ///
    /// Favor an empty set for portability unless a feature is strictly necessary.
    pub required_features: wgpu::Features,

    /// Limits requested from the adapter/device.
    pub required_limits: wgpu::Limits,

    /// Desired maximum frame latency for the surface.
    ///
    /// This value is a hint; support depends on platform/backend.
    pub desired_maximum_frame_latency: u32,
}

impl Default for GpuInit {
    fn default() -> Self {
        Self {
            prefer_srgb: true,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            desired_maximum_frame_latency: 2,
        }
    }
}