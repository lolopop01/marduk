use anyhow::{Context, Result};
use wgpu::SurfaceError;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use super::{GpuFrame, GpuInit, SurfaceErrorAction};
use super::surface;

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

impl<'w> Gpu<'w> {
    /// Creates a GPU context bound to a window.
    ///
    /// Adapter/device acquisition is asynchronous under wgpu.
    pub async fn new(window: &'w Window, init: GpuInit) -> Result<Self> {
        let size = window.inner_size();
        anyhow::ensure!(size.width > 0 && size.height > 0, "window has zero size");

        let GpuInit {
            prefer_srgb,
            present_mode,
            alpha_mode,
            required_features,
            required_limits,
            desired_maximum_frame_latency,
        } = init;

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

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
                required_features,
                required_limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .context("failed to create wgpu device/queue")?;

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface::choose_surface_format(&surface_caps, prefer_srgb)
            .context("no supported surface formats")?;

        let alpha_mode = surface::choose_alpha_mode(&surface_caps, alpha_mode);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency,
        };

        surface.configure(&device, &config);

        Ok(Self {
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
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        surface::apply_resize(
            &self.surface,
            &self.device,
            &mut self.config,
            &mut self.size,
            new_size,
        );
    }

    /// Acquires the next surface texture and creates an encoder.
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
    pub fn submit(&self, frame: GpuFrame) {
        self.queue.submit(std::iter::once(frame.encoder.finish()));
        drop(frame.view);
        drop(frame.surface_texture);
    }

    /// Converts a `SurfaceError` into a higher-level action.
    pub fn handle_surface_error(&mut self, err: SurfaceError) -> SurfaceErrorAction {
        surface::map_surface_error(&self.surface, &self.device, &self.config, self.size, err)
    }
}