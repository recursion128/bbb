use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::{
    sync::mpsc::{self, Receiver, SyncSender},
    thread::{self, JoinHandle},
};

use anyhow::{anyhow, Result};
use bbb_client_clock::Instant;

use crate::render_core::counters::RendererCounters;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct TerrainCommandFinisher {
    requests: Option<SyncSender<wgpu::CommandEncoder>>,
    results: Receiver<(wgpu::CommandBuffer, Duration)>,
    worker: Option<JoinHandle<()>>,
    pending: bool,
    inline_pending: Option<(wgpu::CommandBuffer, Duration)>,
    warned_worker_closed: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl TerrainCommandFinisher {
    pub(crate) fn new() -> std::io::Result<Self> {
        let (requests, inputs) = mpsc::sync_channel::<wgpu::CommandEncoder>(1);
        let (outputs, results) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("bbb-finish-terrain".into())
            .spawn(move || {
                while let Ok(encoder) = inputs.recv() {
                    let started = Instant::now();
                    let commands = encoder.finish();
                    if outputs.send((commands, started.elapsed())).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            requests: Some(requests),
            results,
            worker: Some(worker),
            pending: false,
            inline_pending: None,
            warned_worker_closed: false,
        })
    }

    pub(crate) fn start(&mut self, encoder: wgpu::CommandEncoder) {
        debug_assert!(!self.pending && self.inline_pending.is_none());
        let sent = match self.requests.as_ref() {
            Some(requests) => requests.send(encoder),
            None => Err(mpsc::SendError(encoder)),
        };
        match sent {
            Ok(()) => {
                self.pending = true;
            }
            Err(mpsc::SendError(encoder)) => {
                self.requests = None;
                if !self.warned_worker_closed {
                    self.warned_worker_closed = true;
                    tracing::warn!(
                        "terrain finish worker channel closed; finishing terrain commands inline"
                    );
                }
                let started = Instant::now();
                let commands = encoder.finish();
                self.inline_pending = Some((commands, started.elapsed()));
            }
        }
    }

    pub(crate) fn complete(&mut self) -> Option<(wgpu::CommandBuffer, Duration)> {
        if let Some(inline) = self.inline_pending.take() {
            return Some(inline);
        }
        if !std::mem::take(&mut self.pending) {
            return None;
        }
        match self.results.recv() {
            Ok(finished) => Some(finished),
            Err(_) => {
                self.requests = None;
                if !self.warned_worker_closed {
                    self.warned_worker_closed = true;
                    tracing::warn!("terrain finish worker stopped before returning command buffer");
                }
                None
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for TerrainCommandFinisher {
    fn drop(&mut self) {
        drop(self.requests.take());
        if let Some(worker) = self.worker.take() {
            if let Err(panic) = worker.join() {
                if !thread::panicking() {
                    std::panic::resume_unwind(panic);
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct TerrainCommandFinisher {
    pending: Option<(wgpu::CommandBuffer, Duration)>,
}

#[cfg(target_arch = "wasm32")]
impl TerrainCommandFinisher {
    pub(crate) fn new() -> std::io::Result<Self> {
        Ok(Self { pending: None })
    }

    pub(crate) fn start(&mut self, encoder: wgpu::CommandEncoder) {
        debug_assert!(self.pending.is_none());
        let started = Instant::now();
        let commands = encoder.finish();
        self.pending = Some((commands, started.elapsed()));
    }

    pub(crate) fn complete(&mut self) -> Option<(wgpu::CommandBuffer, Duration)> {
        self.pending.take()
    }
}

/// Where `render()` sends the frame's color output. Production always draws
/// into the window swapchain; whole-frame readback tests swap in a plain
/// offscreen texture so the full frame-step sequence runs without a window.
pub enum RenderSurface {
    Window(wgpu::Surface<'static>),
    Offscreen(std::sync::Arc<wgpu::Texture>),
}

impl RenderSurface {
    fn acquire_frame(
        &mut self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        counters: &mut RendererCounters,
        injected_fault: Option<wgpu::SurfaceStatus>,
    ) -> Result<Option<FrameTarget>> {
        let acquired = match injected_fault {
            Some(status) => InjectedOrAcquiredSurface::Status(status),
            None => match self {
                Self::Window(surface) => match surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame) => {
                        InjectedOrAcquiredSurface::Frame(FrameTarget::Surface(frame))
                    }
                    wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                        drop(frame);
                        InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Suboptimal)
                    }
                    wgpu::CurrentSurfaceTexture::Timeout => {
                        InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Timeout)
                    }
                    wgpu::CurrentSurfaceTexture::Occluded => {
                        InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Occluded)
                    }
                    wgpu::CurrentSurfaceTexture::Outdated => {
                        InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Outdated)
                    }
                    wgpu::CurrentSurfaceTexture::Lost => {
                        InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Lost)
                    }
                    wgpu::CurrentSurfaceTexture::Validation => {
                        InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Validation)
                    }
                },
                Self::Offscreen(texture) => {
                    InjectedOrAcquiredSurface::Frame(FrameTarget::Offscreen(texture.clone()))
                }
            },
        };
        match acquired {
            InjectedOrAcquiredSurface::Frame(frame) => Ok(Some(frame)),
            InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Lost) => {
                counters.surface_lost = counters.surface_lost.saturating_add(1);
                counters.surface_reconfigurations =
                    counters.surface_reconfigurations.saturating_add(1);
                counters.surface_frames_skipped = counters.surface_frames_skipped.saturating_add(1);
                self.configure(device, config);
                Ok(None)
            }
            InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Outdated) => {
                counters.surface_outdated = counters.surface_outdated.saturating_add(1);
                counters.surface_reconfigurations =
                    counters.surface_reconfigurations.saturating_add(1);
                counters.surface_frames_skipped = counters.surface_frames_skipped.saturating_add(1);
                self.configure(device, config);
                Ok(None)
            }
            InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Timeout) => {
                counters.surface_timeouts = counters.surface_timeouts.saturating_add(1);
                counters.surface_frames_skipped = counters.surface_frames_skipped.saturating_add(1);
                Ok(None)
            }
            InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Occluded) => {
                counters.surface_frames_skipped = counters.surface_frames_skipped.saturating_add(1);
                Ok(None)
            }
            InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Suboptimal) => {
                counters.surface_reconfigurations =
                    counters.surface_reconfigurations.saturating_add(1);
                counters.surface_frames_skipped = counters.surface_frames_skipped.saturating_add(1);
                self.configure(device, config);
                Ok(None)
            }
            InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Validation) => {
                Err(anyhow!("surface acquisition validation error"))
            }
            InjectedOrAcquiredSurface::Status(wgpu::SurfaceStatus::Good) => Err(anyhow!(
                "surface acquisition reported success without a texture"
            )),
        }
    }

    pub fn configure(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        match self {
            Self::Window(surface) => surface.configure(device, config),
            Self::Offscreen(texture) => {
                *texture = std::sync::Arc::new(device.create_texture(&wgpu::TextureDescriptor {
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
                }));
            }
        }
    }
}

enum InjectedOrAcquiredSurface {
    Frame(FrameTarget),
    Status(wgpu::SurfaceStatus),
}

/// One frame's acquired color target: a swapchain frame to present, or the
/// offscreen texture (readback harness) where present is a no-op.
pub enum FrameTarget {
    Surface(wgpu::SurfaceTexture),
    Offscreen(std::sync::Arc<wgpu::Texture>),
}

impl FrameTarget {
    pub fn texture(&self) -> &wgpu::Texture {
        match self {
            Self::Surface(frame) => &frame.texture,
            Self::Offscreen(texture) => texture.as_ref(),
        }
    }

    fn present(self, queue: &wgpu::Queue) {
        match self {
            Self::Surface(frame) => queue.present(frame),
            Self::Offscreen(_) => {}
        }
    }
}

pub struct FrameFlightAcquisition {
    pub frame: Result<Option<FrameTarget>>,
    pub previous_submission_poll: Duration,
    pub previous_submission_blocking_wait: Duration,
    pub previous_submission_wait: Duration,
    /// Legacy render duration boundary: CPU encoding plus the previous
    /// submission wait, stopping immediately before drawable acquisition.
    pub render_duration_before_surface: Duration,
    pub surface_acquire: Duration,
    /// Age of the retired submission when its wgpu callback was observed.
    /// This is callback-delivery time, not the exact GPU completion instant.
    pub retired_submission_render_to_callback: Duration,
    pub retired_submission_submit_to_callback: Duration,
    pub retired_submission_callback_to_wait_return: Duration,
    pub completion_observation_expected: bool,
    pub completion_observation_missing: bool,
    pub completion_observations_dropped: u64,
}

pub struct FrameFlightSubmission {
    pub queue_submit: Duration,
    pub swap_buffers_present: Duration,
    pub queue_submit_api: Duration,
    pub post_present_fence_submit: Duration,
    pub in_flight_depth_at_submit: u32,
    pub queue_submitted_at: Instant,
    pub present_call_returned_at: Instant,
}

pub fn acquire_frame(
    surface: &mut RenderSurface,
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    counters: &mut RendererCounters,
    injected_fault: Option<wgpu::SurfaceStatus>,
    render_encoding_started_at: Instant,
) -> FrameFlightAcquisition {
    let render_duration_before_surface = render_encoding_started_at.elapsed();
    let surface_acquire_started_at = Instant::now();
    let frame = surface.acquire_frame(device, config, counters, injected_fault);
    FrameFlightAcquisition {
        frame,
        render_duration_before_surface,
        surface_acquire: surface_acquire_started_at.elapsed(),
        previous_submission_poll: Duration::ZERO,
        previous_submission_blocking_wait: Duration::ZERO,
        previous_submission_wait: Duration::ZERO,
        retired_submission_render_to_callback: Duration::ZERO,
        retired_submission_submit_to_callback: Duration::ZERO,
        retired_submission_callback_to_wait_return: Duration::ZERO,
        completion_observation_expected: false,
        completion_observation_missing: false,
        completion_observations_dropped: 0,
    }
}

pub fn submit_and_present(
    queue: &wgpu::Queue,
    commands: impl IntoIterator<Item = wgpu::CommandBuffer>,
    finish_duration: Duration,
    frame: FrameTarget,
    profile_submission: bool,
) -> FrameFlightSubmission {
    let submit_started_at = Instant::now();
    queue.submit(commands);
    let queue_submit_returned_at = profile_submission.then(Instant::now);
    let queue_submitted_at = bbb_client_clock::now();
    let queue_submit = finish_duration + submit_started_at.elapsed();

    let present_started_at = Instant::now();
    frame.present(queue);
    let present_call_returned_at = bbb_client_clock::now();
    FrameFlightSubmission {
        queue_submit,
        swap_buffers_present: present_started_at.elapsed(),
        queue_submit_api: queue_submit_returned_at
            .map(|finished_at| finished_at.duration_since(submit_started_at))
            .unwrap_or_default(),
        post_present_fence_submit: Duration::ZERO,
        // No explicit fence observations; zero does not describe GPU queue depth.
        in_flight_depth_at_submit: 0,
        queue_submitted_at,
        present_call_returned_at,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn terrain_command_finisher_idle_and_closed_worker_do_not_panic() {
        let mut finisher = TerrainCommandFinisher::new().expect("spawn terrain finish worker");
        assert!(finisher.complete().is_none());
        drop(finisher);

        let (requests, inputs) = mpsc::sync_channel::<wgpu::CommandEncoder>(1);
        let (outputs, results) = mpsc::sync_channel::<(wgpu::CommandBuffer, Duration)>(1);
        drop(inputs);
        drop(outputs);
        let mut disconnected = TerrainCommandFinisher {
            requests: Some(requests),
            results,
            worker: None,
            pending: true,
            inline_pending: None,
            warned_worker_closed: false,
        };
        assert!(disconnected.complete().is_none());
        assert!(disconnected.requests.is_none());
        assert!(disconnected.warned_worker_closed);
        assert!(disconnected.complete().is_none());
    }
}
