//! Offscreen whole-frame readback harness.
//!
//! Runs the complete `render()` frame — every `FRAME_STEPS` pass, in
//! production order — against an injected offscreen color target instead of a
//! window swapchain, then reads the presented pixels back through the shared
//! screenshot copy path (`prepare_screenshot_copy` + `read_screenshot_pixels`,
//! the single home of padded-row and BGRA handling). Pixel-proof tests for
//! HUD / screen slices build on these two entry points instead of hand-rolling
//! per-test sub-passes.

use std::sync::Arc;

use anyhow::{bail, Result};
use winit::dpi::PhysicalSize;

use crate::renderer::RenderSurface;
use crate::screenshot::ScreenshotPixels;
use crate::Renderer;

/// The production surface format family `choose_format` prefers (sRGB), so
/// offscreen frames exercise the same color pipeline as the swapchain,
/// including the BGRA byte-order swap on readback.
const OFFSCREEN_SURFACE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

impl Renderer {
    /// Builds a windowless `Renderer` whose frame target is a `width`x`height`
    /// offscreen texture in the production-preferred surface format. Returns
    /// `None` when the machine has no usable GPU adapter or device (callers
    /// skip rather than fail, matching the existing readback-test pattern).
    pub(crate) fn new_offscreen(width: u32, height: u32) -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;
        eprintln!(
            "offscreen frame harness adapter: {}",
            adapter.get_info().name
        );
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("bbb-offscreen-frame-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        ))
        .ok()?;

        // Mirrors the swapchain configuration `Renderer::new` builds, minus the
        // window-dependent present/alpha negotiation (both irrelevant offscreen).
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: OFFSCREEN_SURFACE_FORMAT,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        let target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bbb-offscreen-frame-target"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: config.usage,
            view_formats: &[],
        });
        let size = PhysicalSize::new(config.width, config.height);
        Some(
            Renderer::with_gpu(
                RenderSurface::Offscreen(Arc::new(target)),
                device,
                queue,
                config,
                size,
            )
            .expect("offscreen renderer construction"),
        )
    }

    /// Renders one full frame (all `FRAME_STEPS`) into the offscreen target
    /// and returns its pixels, read back through the shared screenshot path.
    pub(crate) fn render_offscreen_frame(&mut self) -> Result<ScreenshotPixels> {
        let RenderSurface::Offscreen(target) = &self.surface else {
            bail!("render_offscreen_frame requires a Renderer built by new_offscreen");
        };
        let target = target.clone();
        self.render(None)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bbb-offscreen-frame-readback"),
            });
        let pending = self.prepare_screenshot_copy(&mut encoder, &target)?;
        self.queue.submit(Some(encoder.finish()));
        self.read_screenshot_pixels(pending)
    }
}

mod tests {
    use crate::{ClearColor, Renderer};

    const WIDTH: u32 = 320;
    const HEIGHT: u32 = 240;

    /// Harness acceptance: a whole `render()` frame runs headless and the
    /// readback shows a known HUD element (centered crosshair) over the clear
    /// color, with the frame counters proving the step pipeline executed.
    #[test]
    fn offscreen_frame_renders_hud_sentinel_over_clear_color() {
        let Some(mut renderer) = Renderer::new_offscreen(WIDTH, HEIGHT) else {
            // No GPU / software adapter on this machine — skip rather than fail.
            return;
        };
        // Pure blue background: survives the sRGB surface encoding as byte 255.
        renderer.set_clear_color(ClearColor {
            r: 0.0,
            g: 0.0,
            b: 1.0,
            a: 1.0,
        });
        // A 4x4 solid-red crosshair; `collect_hud_draws` centers it, so the
        // framebuffer center pixel must come out red.
        renderer
            .upload_hud_crosshair(4, 4, &[255, 0, 0, 255].repeat(16))
            .expect("upload crosshair");

        let pixels = renderer.render_offscreen_frame().expect("offscreen frame");

        assert_eq!((pixels.width, pixels.height), (WIDTH, HEIGHT));
        let center = pixels.pixel(WIDTH / 2, HEIGHT / 2);
        let corner = pixels.pixel(8, 8);
        assert!(
            center[0] > 128 && center[2] < 128,
            "center should show the red crosshair, got {center:?}"
        );
        assert!(
            corner[2] > 128 && corner[0] < 128,
            "corner should stay clear-color blue, got {corner:?}"
        );

        // The frame went through finish_frame's counter fold: one frame, a HUD
        // draw for the crosshair, and at least the always-on lightmap +
        // transparency combine + blit draws alongside it.
        assert_eq!(renderer.counters.frame_index, 1);
        assert!(
            renderer.counters.hud_draw_calls >= 1,
            "crosshair HUD draw is tallied, got {}",
            renderer.counters.hud_draw_calls
        );
        assert!(
            renderer.counters.draw_calls >= 4,
            "lightmap/combine/blit/HUD draws are tallied, got {}",
            renderer.counters.draw_calls
        );
        assert_eq!(renderer.counters.screenshots_written, 0);
    }
}
