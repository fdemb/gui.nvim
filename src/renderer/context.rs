use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::config::VsyncMode;

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
}

impl GpuContext {
    pub async fn new(window: Arc<Window>, vsync: VsyncMode) -> Result<Self, GpuContextError> {
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Err(GpuContextError::InvalidSize);
        }

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        log::info!("Using GPU adapter: {:?}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = Self::select_present_mode(&surface_caps, vsync);

        // With VSync, use 1 frame in flight for lower latency (vs 2 which adds ~16-33ms)
        let uses_vsync = matches!(
            present_mode,
            wgpu::PresentMode::AutoVsync | wgpu::PresentMode::Fifo | wgpu::PresentMode::Mailbox
        );
        let desired_maximum_frame_latency = if uses_vsync { 1 } else { 2 };

        log::info!(
            "Present mode: {:?}, frame latency: {}",
            present_mode,
            desired_maximum_frame_latency
        );

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency,
        };
        surface.configure(&device, &surface_config);

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.surface_config.width, self.surface_config.height)
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    pub fn get_current_texture(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }

    fn select_present_mode(
        surface_caps: &wgpu::SurfaceCapabilities,
        vsync: VsyncMode,
    ) -> wgpu::PresentMode {
        match vsync {
            VsyncMode::Enabled => wgpu::PresentMode::AutoVsync,
            VsyncMode::Disabled => wgpu::PresentMode::AutoNoVsync,
            VsyncMode::MailboxIfAvailable => {
                if surface_caps
                    .present_modes
                    .contains(&wgpu::PresentMode::Mailbox)
                {
                    wgpu::PresentMode::Mailbox
                } else {
                    wgpu::PresentMode::AutoVsync
                }
            }
            // DisplayLink: CADisplayLink handles timing, no need for wgpu vsync
            // On non-macOS, fall back to AutoVsync
            VsyncMode::DisplayLink => {
                #[cfg(target_os = "macos")]
                {
                    wgpu::PresentMode::AutoNoVsync
                }
                #[cfg(not(target_os = "macos"))]
                {
                    wgpu::PresentMode::AutoVsync
                }
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GpuContextError {
    #[error("Invalid window size (zero width or height)")]
    InvalidSize,

    #[error("Failed to create surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),

    #[error("No compatible GPU adapter found: {0}")]
    NoAdapter(#[from] wgpu::RequestAdapterError),

    #[error("Failed to request device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physical_size_zero() {
        let size = PhysicalSize::new(0u32, 0u32);
        assert_eq!(size.width, 0);
        assert_eq!(size.height, 0);
    }
}
