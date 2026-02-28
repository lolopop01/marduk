use winit::dpi::PhysicalSize;

use super::{GpuInit, SurfaceErrorAction};

pub(crate) fn choose_surface_format(
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

pub(crate) fn choose_alpha_mode(
    caps: &wgpu::SurfaceCapabilities,
    requested: Option<wgpu::CompositeAlphaMode>,
) -> wgpu::CompositeAlphaMode {
    requested
        .filter(|m| caps.alpha_modes.contains(m))
        .or_else(|| caps.alpha_modes.first().copied())
        .unwrap_or(wgpu::CompositeAlphaMode::Auto)
}

pub(crate) fn apply_resize(
    surface: &wgpu::Surface,
    device: &wgpu::Device,
    config: &mut wgpu::SurfaceConfiguration,
    size: &mut PhysicalSize<u32>,
    new_size: PhysicalSize<u32>,
) {
    if new_size.width == 0 || new_size.height == 0 {
        *size = new_size;
        return;
    }

    *size = new_size;
    config.width = new_size.width;
    config.height = new_size.height;

    surface.configure(device, config);
}

pub(crate) fn map_surface_error(
    surface: &wgpu::Surface,
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    err: wgpu::SurfaceError,
) -> SurfaceErrorAction {
    match err {
        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
            if size.width > 0 && size.height > 0 {
                surface.configure(device, config);
            }
            SurfaceErrorAction::Reconfigured
        }
        wgpu::SurfaceError::OutOfMemory => SurfaceErrorAction::Fatal,
        wgpu::SurfaceError::Timeout => SurfaceErrorAction::SkipFrame,
        wgpu::SurfaceError::Other => SurfaceErrorAction::SkipFrame,
    }
}