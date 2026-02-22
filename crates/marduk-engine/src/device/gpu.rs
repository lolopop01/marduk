use anyhow::{Context, Result};
use wgpu::SurfaceError;
use winit::dpi::PhysicalSize;
use winit::window::Window;

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

/// Owns wgpu core objects and the surface configuration.
///
/// This type is the low-level rendering context:
/// - creates and stores Instance/Adapter/Device/Queue
/// - creates and configures the Surface (swapchain)
/// - acquires frames and provides an encoder + view for rendering
pub struct Gpu<'w> {
    /// wgpu instance used to create the adapter and surface.
    instance: wgpu::Instance,

    /// Surface bound to the window.
    ///
    /// Surface lifetime is tied to the window; architecture must ensure the window
    /// outlives the `Gpu` instance.
    surface: wgpu::Surface<'w>,

    /// Selected adapter.
    adapter: wgpu::Adapter,

    /// Logical device.
    device: wgpu::Device,

    /// Command queue.
    queue: wgpu::Queue,

    /// Active surface configuration.
    config: wgpu::SurfaceConfiguration,

    /// Current drawable size in physical pixels.
    size: PhysicalSize<u32>,
}

/// Represents a single acquired frame.
///
/// This object is short-lived and must be finalized promptly. Holding the surface
/// texture prevents acquisition of subsequent frames.
pub struct GpuFrame {
    pub surface_texture: wgpu::SurfaceTexture,
    pub view: wgpu::TextureView,
    pub encoder: wgpu::CommandEncoder,
}

/// High-level response after a surface error.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum SurfaceErrorAction {
    /// Surface was reconfigured; rendering may resume next frame.
    Reconfigured,
    /// Transient error; skip the current frame.
    SkipFrame,
    /// Fatal error (commonly OOM); terminate gracefully.
    Fatal,
}

impl<'w> Gpu<'w> {
    /// Creates a GPU context bound to a window.
    ///
    /// Adapter/device acquisition is asynchronous under wgpu.
    pub async fn new(window: &'w Window, init: GpuInit) -> Result<Self> {
        let size = window.inner_size();
        anyhow::ensure!(size.width > 0 && size.height > 0, "window has zero size");

        // Use all backends to allow wgpu to select the optimal platform backend.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Surface lifetime is tied to `window` via `'w`.
        let surface = instance
            .create_surface(window)
            .context("failed to create wgpu surface")?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("failed to find a suitable GPU adapter")?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("marduk-engine device"),
                required_features: init.required_features,
                required_limits: init.required_limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .context("failed to create wgpu device/queue")?;

        let surface_caps = surface.get_capabilities(&adapter);
        let format = choose_surface_format(&surface_caps, init.prefer_srgb)
            .context("no supported surface formats")?;

        let alpha_mode = init
            .alpha_mode
            .filter(|m| surface_caps.alpha_modes.contains(m))
            .unwrap_or_else(|| {
                surface_caps
                    .alpha_modes
                    .first()
                    .copied()
                    .unwrap_or(wgpu::CompositeAlphaMode::Auto)
            });

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: init.present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: init.desired_maximum_frame_latency,
        };

        surface.configure(&device, &config);

        Ok(Gpu {
            instance,
            surface,
            adapter,
            device,
            queue,
            config,
            size,
        })
    }

    /// Returns the active surface format.
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    /// Returns the current drawable size (physical pixels).
    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    /// Returns a reference to the logical device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Returns a reference to the command queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Reconfigures the surface after a resize.
    ///
    /// wgpu does not support configuring a surface with a 0x0 size; in that case,
    /// only internal state is updated and configuration is deferred.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            self.size = new_size;
            return;
        }

        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Acquires the next surface texture and creates an encoder.
    ///
    /// The returned frame owns the surface texture. Releasing it (after submission)
    /// presents the frame.
    pub fn begin_frame(&self) -> std::result::Result<GpuFrame, SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("marduk frame encoder"),
            });

        Ok(GpuFrame {
            surface_texture,
            view,
            encoder,
        })
    }

    /// Submits the recorded commands for the given frame.
    ///
    /// Presentation occurs when `surface_texture` is dropped after submission.
    pub fn submit(&self, frame: GpuFrame) {
        self.queue.submit(std::iter::once(frame.encoder.finish()));
        drop(frame.view);
        drop(frame.surface_texture);
    }

    /// Converts a `SurfaceError` into a higher-level action.
    pub fn handle_surface_error(&mut self, err: SurfaceError) -> SurfaceErrorAction {
        match err {
            SurfaceError::Lost | SurfaceError::Outdated => {
                if self.size.width > 0 && self.size.height > 0 {
                    self.surface.configure(&self.device, &self.config);
                }
                SurfaceErrorAction::Reconfigured
            }
            SurfaceError::OutOfMemory => SurfaceErrorAction::Fatal,
            SurfaceError::Timeout => SurfaceErrorAction::SkipFrame,
            SurfaceError::Other => SurfaceErrorAction::SkipFrame,
        }
    }
}

fn choose_surface_format(
    caps: &wgpu::SurfaceCapabilities,
    prefer_srgb: bool,
) -> Option<wgpu::TextureFormat> {
    if caps.formats.is_empty() {
        return None;
    }

    if prefer_srgb {
        let preferred = [
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ];
        for f in preferred {
            if caps.formats.contains(&f) {
                return Some(f);
            }
        }
    }

    Some(caps.formats[0])
}