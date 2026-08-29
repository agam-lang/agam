//! GPU abstraction, device-loss recovery, and surface lifecycle management.
//!
//! Confines all `wgpu` and adapter-level primitives behind Agam-owned capability
//! tiers and Nyāya diagnostics.

use std::sync::Arc;

use crate::diagnostic::{GuiError, GuiResult};
use crate::platform::GuiWindow;

/// Hardware rendering capability tiers defined in RFC-gui-engine §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HardwareTier {
    /// Safe recovery mode: solid surfaces, opaque layers, no blur/shaders.
    Safe,
    /// Integrated GPU baseline: rounded rects, gradients, text, 60 FPS target.
    Integrated,
    /// Balanced discrete baseline: cached shadows, bounded blur, 60/120 Hz.
    Balanced,
    /// Discrete enthusiast: full material stack, backdrop blur, 120+ Hz.
    Discrete,
}

/// Abstract representation of GPU adapter capabilities without exposing backend names.
#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    /// Selected hardware rendering tier.
    pub tier: HardwareTier,
    /// Maximum 2D texture dimension in pixels.
    pub max_texture_dimension_2d: u32,
    /// Whether compute shaders are available on the active device.
    pub supports_compute: bool,
}

impl Default for GpuCapabilities {
    fn default() -> Self {
        Self {
            tier: HardwareTier::Integrated,
            max_texture_dimension_2d: 8192,
            supports_compute: true,
        }
    }
}

/// Evaluate hardware rendering tier and limits from an adapter.
pub fn evaluate_hardware_tier(adapter: &wgpu::Adapter) -> (HardwareTier, GpuCapabilities) {
    let info = adapter.get_info();
    let limits = adapter.limits();

    let supports_compute = limits.max_compute_workgroup_size_x > 0;

    let tier = match info.device_type {
        wgpu::DeviceType::Cpu => HardwareTier::Safe,
        wgpu::DeviceType::IntegratedGpu => {
            if limits.max_texture_dimension_2d >= 8192 {
                HardwareTier::Integrated
            } else {
                HardwareTier::Safe
            }
        }
        wgpu::DeviceType::DiscreteGpu => {
            if limits.max_texture_dimension_2d >= 16384 && limits.max_buffer_size >= (1 << 30) {
                HardwareTier::Discrete
            } else {
                HardwareTier::Balanced
            }
        }
        _ => HardwareTier::Integrated,
    };

    let capabilities = GpuCapabilities {
        tier,
        max_texture_dimension_2d: limits.max_texture_dimension_2d,
        supports_compute,
    };

    (tier, capabilities)
}

/// An acquired GPU presentation frame ready for rendering and presentation.
pub struct GpuFrame {
    surface_texture: wgpu::SurfaceTexture,
}

impl GpuFrame {
    /// Internal constructor.
    pub(crate) fn new(surface_texture: wgpu::SurfaceTexture) -> Self {
        Self { surface_texture }
    }

    /// Present the rendered frame to the physical display surface.
    pub fn present(self) {
        self.surface_texture.present();
    }

    /// Internal access to create a texture view for Vello rendering.
    #[allow(dead_code)]
    pub(crate) fn create_view(&self) -> wgpu::TextureView {
        self.surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Get frame dimensions `(width, height)` in physical pixels.
    pub fn dimensions(&self) -> (u32, u32) {
        (
            self.surface_texture.texture.width(),
            self.surface_texture.texture.height(),
        )
    }
}

/// Represents a window-backed GPU presentation surface.
pub struct GpuSurface {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    device: Arc<wgpu::Device>,
    width: u32,
    height: u32,
}

impl GpuSurface {
    /// Reconfigure the surface for new physical window dimensions.
    pub fn resize(&mut self, width: u32, height: u32) -> GuiResult<()> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        self.width = width;
        self.height = height;
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        Ok(())
    }

    /// Acquire the next presentation texture from the swapchain.
    pub fn acquire_frame(&mut self) -> GuiResult<GpuFrame> {
        let current = self.surface.get_current_texture();
        match current {
            wgpu::CurrentSurfaceTexture::Success(tex)
            | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => Ok(GpuFrame::new(tex)),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                match self.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(tex)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(tex) => Ok(GpuFrame::new(tex)),
                    other => Err(map_surface_status(&other)),
                }
            }
            other => Err(map_surface_status(&other)),
        }
    }

    /// Return current surface physical dimensions `(width, height)`.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Return surface texture format.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }
}

/// Central GPU context encapsulating instance, adapter, device, and queue.
#[derive(Clone)]
pub struct GpuContext {
    instance: Arc<wgpu::Instance>,
    adapter: Arc<wgpu::Adapter>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    capabilities: GpuCapabilities,
}

impl GpuContext {
    /// Initialize the GPU context synchronously using available hardware.
    pub fn new() -> GuiResult<Self> {
        block_on(Self::new_async())
    }

    /// Initialize the GPU context asynchronously.
    pub async fn new_async() -> GuiResult<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::empty(),
            backend_options: Default::default(),
            display: None,
            memory_budget_thresholds: Default::default(),
        });

        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
        {
            Ok(a) => a,
            Err(_) => {
                // Fallback to low-power or software adapter if high performance fails
                instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::LowPower,
                        compatible_surface: None,
                        force_fallback_adapter: true,
                    })
                    .await
                    .map_err(|e| map_device_error(&e))?
            }
        };

        let (_tier, capabilities) = evaluate_hardware_tier(&adapter);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .map_err(|e| map_device_error(&e))?;

        Ok(Self {
            instance: Arc::new(instance),
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
            capabilities,
        })
    }

    /// Create a presentation surface bound to a native window.
    pub fn create_surface(&self, window: &GuiWindow) -> GuiResult<GpuSurface> {
        let (width, height) = window.inner_size();
        let target = window.raw_window().clone();
        let surface = self
            .instance
            .create_surface(target)
            .map_err(|e| map_create_surface_error(&e))?;

        let caps = surface.get_capabilities(&self.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == wgpu::TextureFormat::Rgba8Unorm)
            .or_else(|| {
                caps.formats
                    .iter()
                    .copied()
                    .find(|f| *f == wgpu::TextureFormat::Bgra8Unorm)
            })
            .or_else(|| caps.formats.iter().copied().find(|f| !f.is_srgb()))
            .unwrap_or_else(|| {
                caps.formats
                    .first()
                    .copied()
                    .unwrap_or(wgpu::TextureFormat::Rgba8Unorm)
            });

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::STORAGE_BINDING,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
        };

        surface.configure(&self.device, &config);

        Ok(GpuSurface {
            surface,
            config,
            device: self.device.clone(),
            width: width.max(1),
            height: height.max(1),
        })
    }

    /// Return evaluated GPU capabilities.
    pub fn capabilities(&self) -> &GpuCapabilities {
        &self.capabilities
    }

    /// Crate-internal accessor for device.
    #[allow(dead_code)]
    pub(crate) fn device(&self) -> &Arc<wgpu::Device> {
        &self.device
    }

    /// Crate-internal accessor for queue.
    #[allow(dead_code)]
    pub(crate) fn queue(&self) -> &Arc<wgpu::Queue> {
        &self.queue
    }

    /// Crate-internal accessor for adapter.
    #[allow(dead_code)]
    pub(crate) fn adapter(&self) -> &Arc<wgpu::Adapter> {
        &self.adapter
    }
}

struct ThreadWaker(std::thread::Thread);

impl std::task::Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.0.unpark();
    }
}

/// Synchronous block_on helper using thread parking waker.
pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let waker = std::task::Waker::from(Arc::new(ThreadWaker(std::thread::current())));
    let mut cx = std::task::Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(output) => return output,
            std::task::Poll::Pending => std::thread::park(),
        }
    }
}

/// Convert a `wgpu::CurrentSurfaceTexture` error variant into an Agam-owned Nyāya `GuiError`.
pub fn map_surface_status(status: &wgpu::CurrentSurfaceTexture) -> GuiError {
    match status {
        wgpu::CurrentSurfaceTexture::Timeout => GuiError::new(
            "GPU surface texture acquisition timed out",
            "Presentation engine was busy or locked by host display compositor",
            Some("Skip current frame or retry on next vsync cycle"),
            "RFC-gui-engine §1: Surface timeouts must not panic and must skip frame gracefully",
        ),
        wgpu::CurrentSurfaceTexture::Outdated => GuiError::new(
            "GPU surface configuration is outdated relative to window dimensions",
            "Window was resized or scale factor changed during presentation lifecycle",
            Some("Reconfigure surface with updated logical/physical dimensions before next frame"),
            "RFC-gui-engine §1: Outdated surfaces must trigger automatic reconfiguration",
        ),
        wgpu::CurrentSurfaceTexture::Lost => GuiError::new(
            "GPU surface connection was lost",
            "Physical display disconnected or GPU driver reset surface handle",
            Some("Re-create surface from native window handle and resume render loop"),
            "RFC-gui-engine §1: Lost surfaces must be recreated automatically without crash",
        ),
        wgpu::CurrentSurfaceTexture::Occluded => GuiError::new(
            "GPU surface is occluded or obscured by another window",
            "Window is minimized or completely covered by other top-level windows",
            Some("Pause rendering frames until window becomes visible again"),
            "RFC-gui-engine §1: Occluded surfaces should pause active render loops",
        ),
        wgpu::CurrentSurfaceTexture::Validation => GuiError::new(
            "GPU surface validation error",
            "Surface usage or texture view violated graphics device invariants",
            Some("Verify surface configuration parameters and swapchain limits"),
            "RFC-gui-engine §1: Validation errors must return structured Nyāya diagnostics",
        ),
        _ => GuiError::new(
            "Unknown GPU surface acquisition error",
            "Underlying graphics driver reported non-standard surface condition",
            Some("Verify GPU drivers and recreate presentation surface"),
            "RFC-gui-engine §1: All surface failures must yield structured Nyāya diagnostics",
        ),
    }
}

/// Convert missing adapter condition into a structured Nyāya `GuiError`.
pub fn map_adapter_error() -> GuiError {
    GuiError::new(
        "No compatible GPU adapter found on host system",
        "Host graphics hardware does not satisfy Vulkan, DX12, Metal, or GLES baseline limits",
        Some(
            "Ensure GPU drivers are installed or enable software rendering fallback (e.g. WARP/llvmpipe)",
        ),
        "RFC-gui-engine §4: Safe fallback tier must be selected when hardware acceleration is unavailable",
    )
}

/// Convert logical device creation failure into a structured Nyāya `GuiError`.
pub fn map_device_error(err: &impl std::fmt::Display) -> GuiError {
    GuiError::new(
        format!("Failed to create logical GPU device and queue: {err}"),
        "Device creation failed due to unsupported limits, missing features, or device loss",
        Some("Demote to integrated/safe hardware tier with reduced feature requirements"),
        "RFC-gui-engine §1: Device loss must trigger automatic recreation attempt and tier demotion",
    )
}

/// Convert surface creation failure into a structured Nyāya `GuiError`.
pub fn map_create_surface_error(err: &impl std::fmt::Display) -> GuiError {
    GuiError::new(
        format!("Failed to create GPU presentation surface: {err}"),
        "Windowing system surface handle could not be bound to GPU instance",
        Some("Verify window handle validity and display server permissions"),
        "RFC-gui-engine §1: Presentation surface creation errors must return structured diagnostics",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_error_mappings() {
        let timeout_err = map_surface_status(&wgpu::CurrentSurfaceTexture::Timeout);
        assert!(timeout_err.fact.contains("timed out"));
        assert!(timeout_err.fix.is_some());

        let lost_err = map_surface_status(&wgpu::CurrentSurfaceTexture::Lost);
        assert!(lost_err.fact.contains("lost"));

        let outdated_err = map_surface_status(&wgpu::CurrentSurfaceTexture::Outdated);
        assert!(outdated_err.fact.contains("outdated"));
    }

    #[test]
    fn test_adapter_error_mapping() {
        let err = map_adapter_error();
        assert!(err.fact.contains("No compatible GPU adapter"));
        let proof = err.to_proof();
        assert_eq!(proof.fact, err.fact);
    }

    #[test]
    fn test_gpu_context_creation() {
        let ctx_res = GpuContext::new();
        match ctx_res {
            Ok(ctx) => {
                let caps = ctx.capabilities();
                assert!(caps.max_texture_dimension_2d >= 2048);
                assert!(matches!(
                    caps.tier,
                    HardwareTier::Safe
                        | HardwareTier::Integrated
                        | HardwareTier::Balanced
                        | HardwareTier::Discrete
                ));
            }
            Err(err) => {
                // In headless environments without GPU, must return structured Nyāya error, not panic
                assert!(
                    err.fact.contains("GPU")
                        || err.fact.contains("adapter")
                        || err.fact.contains("device")
                );
            }
        }
    }
}
