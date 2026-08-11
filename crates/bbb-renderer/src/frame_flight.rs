use std::time::{Duration, Instant};

use anyhow::Result;

use crate::counters::RendererCounters;

/// The explicit in-flight budget retained by the behavior-preserving first
/// FrameFlightController slice. Only uncapped CAMetalLayer presentation uses
/// it; FIFO already supplies presentation backpressure.
#[cfg(any(target_os = "macos", test))]
const MACOS_IMMEDIATE_IN_FLIGHT_BUDGET: usize = 1;

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrameFlightPlatform {
    MacOs,
    Other,
}

#[cfg(any(target_os = "macos", test))]
impl FrameFlightPlatform {
    const CURRENT: Self = if cfg!(target_os = "macos") {
        Self::MacOs
    } else {
        Self::Other
    };
}

#[cfg(any(target_os = "macos", test))]
fn in_flight_budget(
    platform: FrameFlightPlatform,
    present_mode: wgpu::PresentMode,
) -> Option<usize> {
    (platform == FrameFlightPlatform::MacOs && present_mode == wgpu::PresentMode::Immediate)
        .then_some(MACOS_IMMEDIATE_IN_FLIGHT_BUDGET)
}

/// Where `render()` sends the frame's color output. Production always draws
/// into the window swapchain; whole-frame readback tests swap in a plain
/// offscreen texture so the full frame-step sequence runs without a window.
pub(super) enum RenderSurface {
    Window(wgpu::Surface<'static>),
    #[cfg(test)]
    Offscreen(std::sync::Arc<wgpu::Texture>),
}

impl RenderSurface {
    fn acquire_frame(
        &mut self,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        counters: &mut RendererCounters,
        injected_fault: Option<wgpu::SurfaceError>,
    ) -> Result<Option<FrameTarget>> {
        let acquired = match injected_fault {
            Some(error) => Err(error),
            None => match self {
                Self::Window(surface) => surface.get_current_texture().map(FrameTarget::Surface),
                #[cfg(test)]
                Self::Offscreen(texture) => Ok(FrameTarget::Offscreen(texture.clone())),
            },
        };
        match acquired {
            Ok(frame) => Ok(Some(frame)),
            Err(wgpu::SurfaceError::Lost) => {
                counters.surface_lost = counters.surface_lost.saturating_add(1);
                counters.surface_reconfigurations =
                    counters.surface_reconfigurations.saturating_add(1);
                counters.surface_frames_skipped = counters.surface_frames_skipped.saturating_add(1);
                self.configure(device, config);
                Ok(None)
            }
            Err(wgpu::SurfaceError::Outdated) => {
                counters.surface_outdated = counters.surface_outdated.saturating_add(1);
                counters.surface_reconfigurations =
                    counters.surface_reconfigurations.saturating_add(1);
                counters.surface_frames_skipped = counters.surface_frames_skipped.saturating_add(1);
                self.configure(device, config);
                Ok(None)
            }
            Err(wgpu::SurfaceError::Timeout) => {
                counters.surface_timeouts = counters.surface_timeouts.saturating_add(1);
                counters.surface_frames_skipped = counters.surface_frames_skipped.saturating_add(1);
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn configure(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
        match self {
            Self::Window(surface) => surface.configure(device, config),
            #[cfg(test)]
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

/// One frame's acquired color target: a swapchain frame to present, or the
/// offscreen texture (readback harness) where present is a no-op.
pub(super) enum FrameTarget {
    Surface(wgpu::SurfaceTexture),
    #[cfg(test)]
    Offscreen(std::sync::Arc<wgpu::Texture>),
}

impl FrameTarget {
    pub(super) fn texture(&self) -> &wgpu::Texture {
        match self {
            Self::Surface(frame) => &frame.texture,
            #[cfg(test)]
            Self::Offscreen(texture) => texture.as_ref(),
        }
    }

    fn present(self) {
        match self {
            Self::Surface(frame) => frame.present(),
            #[cfg(test)]
            Self::Offscreen(_) => {}
        }
    }
}

pub(super) struct FrameFlightAcquisition {
    pub(super) frame: Result<Option<FrameTarget>>,
    pub(super) previous_submission_poll: Duration,
    pub(super) previous_submission_blocking_wait: Duration,
    pub(super) previous_submission_wait: Duration,
    /// Legacy render duration boundary: CPU encoding plus the previous
    /// submission wait, stopping immediately before drawable acquisition.
    pub(super) render_duration_before_surface: Duration,
    pub(super) surface_acquire: Duration,
}

pub(super) struct FrameFlightSubmission {
    pub(super) queue_submit: Duration,
    pub(super) swap_buffers_present: Duration,
}

#[derive(Default)]
struct PreviousSubmissionSynchronization {
    poll: Duration,
    blocking_wait: Duration,
    total: Duration,
}

/// Owns the backend frame-flight lifecycle without owning visible frame work.
/// macOS Immediate retains one in-flight submission. The current bounded policy
/// first polls completion without blocking and waits on the exact prior fence
/// only while that submission remains active.
pub(super) struct FrameFlightController {
    #[cfg(target_os = "macos")]
    previous_submission: Option<wgpu::SubmissionIndex>,
}

impl FrameFlightController {
    pub(super) fn new() -> Self {
        Self {
            #[cfg(target_os = "macos")]
            previous_submission: None,
        }
    }

    pub(super) fn acquire_frame(
        &mut self,
        surface: &mut RenderSurface,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        counters: &mut RendererCounters,
        injected_fault: Option<wgpu::SurfaceError>,
        profile_wait: bool,
        render_encoding_started_at: Instant,
    ) -> FrameFlightAcquisition {
        let previous_submission =
            self.wait_for_previous_submission(device, config.present_mode, profile_wait);
        let render_duration_before_surface = render_encoding_started_at.elapsed();
        let surface_acquire_started_at = Instant::now();
        let frame = surface.acquire_frame(device, config, counters, injected_fault);
        FrameFlightAcquisition {
            frame,
            previous_submission_poll: previous_submission.poll,
            previous_submission_blocking_wait: previous_submission.blocking_wait,
            previous_submission_wait: previous_submission.total,
            render_duration_before_surface,
            surface_acquire: surface_acquire_started_at.elapsed(),
        }
    }

    fn wait_for_previous_submission(
        &mut self,
        device: &wgpu::Device,
        present_mode: wgpu::PresentMode,
        profile_wait: bool,
    ) -> PreviousSubmissionSynchronization {
        #[cfg(target_os = "macos")]
        {
            let budget = in_flight_budget(FrameFlightPlatform::CURRENT, present_mode);
            debug_assert!(budget.is_none() || budget == Some(1));
            if budget == Some(1) {
                if let Some(submission) = self.previous_submission.take() {
                    let synchronization_started_at = profile_wait.then(Instant::now);
                    let poll_started_at = profile_wait.then(Instant::now);
                    let queue_empty = device.poll(wgpu::Maintain::Poll).is_queue_empty();
                    let poll = poll_started_at
                        .map(|started_at| started_at.elapsed())
                        .unwrap_or_default();
                    let blocking_wait = if queue_empty {
                        Duration::ZERO
                    } else {
                        let wait_started_at = profile_wait.then(Instant::now);
                        device.poll(wgpu::Maintain::WaitForSubmissionIndex(submission));
                        wait_started_at
                            .map(|started_at| started_at.elapsed())
                            .unwrap_or_default()
                    };
                    return PreviousSubmissionSynchronization {
                        poll,
                        blocking_wait,
                        total: synchronization_started_at
                            .map(|started_at| started_at.elapsed())
                            .unwrap_or_default(),
                    };
                }
            }
            PreviousSubmissionSynchronization::default()
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (device, present_mode, profile_wait);
            PreviousSubmissionSynchronization::default()
        }
    }

    pub(super) fn submit_and_present(
        &mut self,
        queue: &wgpu::Queue,
        encoder: wgpu::CommandEncoder,
        frame: FrameTarget,
    ) -> FrameFlightSubmission {
        let submit_started_at = Instant::now();
        let submission = queue.submit(Some(encoder.finish()));
        #[cfg(target_os = "macos")]
        {
            self.previous_submission = Some(submission);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = submission;
        let queue_submit = submit_started_at.elapsed();

        let present_started_at = Instant::now();
        frame.present();
        FrameFlightSubmission {
            queue_submit,
            swap_buffers_present: present_started_at.elapsed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{in_flight_budget, FrameFlightPlatform, MACOS_IMMEDIATE_IN_FLIGHT_BUDGET};

    #[test]
    fn only_macos_immediate_has_the_existing_single_submission_budget() {
        assert_eq!(MACOS_IMMEDIATE_IN_FLIGHT_BUDGET, 1);
        assert_eq!(
            in_flight_budget(FrameFlightPlatform::MacOs, wgpu::PresentMode::Immediate),
            Some(1)
        );
        assert_eq!(
            in_flight_budget(FrameFlightPlatform::Other, wgpu::PresentMode::Immediate),
            None
        );
        for mode in [
            wgpu::PresentMode::Fifo,
            wgpu::PresentMode::FifoRelaxed,
            wgpu::PresentMode::Mailbox,
            wgpu::PresentMode::AutoVsync,
            wgpu::PresentMode::AutoNoVsync,
        ] {
            assert_eq!(
                in_flight_budget(FrameFlightPlatform::MacOs, mode),
                None,
                "{mode:?} keeps its prior non-serializing behavior"
            );
            assert_eq!(in_flight_budget(FrameFlightPlatform::Other, mode), None);
        }
    }

    #[test]
    fn controller_source_pins_wait_acquire_submit_present_order() {
        let source = include_str!("frame_flight.rs");
        let controller = source.find("impl FrameFlightController").unwrap();
        let acquire_start = source[controller..]
            .find("pub(super) fn acquire_frame(")
            .map(|offset| controller + offset)
            .unwrap();
        let wait_start = source[acquire_start..]
            .find("self.wait_for_previous_submission")
            .map(|offset| acquire_start + offset)
            .unwrap();
        let legacy_render_boundary = source[wait_start..]
            .find("render_encoding_started_at.elapsed()")
            .map(|offset| wait_start + offset)
            .unwrap();
        let surface_acquire = source[wait_start..]
            .find("surface.acquire_frame")
            .map(|offset| wait_start + offset)
            .unwrap();
        assert!(wait_start < legacy_render_boundary && legacy_render_boundary < surface_acquire);

        let submit_start = source.find("pub(super) fn submit_and_present(").unwrap();
        let submit = source[submit_start..]
            .find("queue.submit")
            .map(|offset| submit_start + offset)
            .unwrap();
        let record_submission = source[submit..]
            .find("self.previous_submission = Some(submission)")
            .map(|offset| submit + offset)
            .unwrap();
        let present = source[record_submission..]
            .find("frame.present()")
            .map(|offset| record_submission + offset)
            .unwrap();
        assert!(submit < record_submission && record_submission < present);
    }

    #[test]
    fn completion_poll_only_skips_the_blocking_wait_when_the_queue_is_empty() {
        let source = include_str!("frame_flight.rs");
        let wait = source.find("fn wait_for_previous_submission(").unwrap();
        let submit = source.find("pub(super) fn submit_and_present(").unwrap();
        let body = &source[wait..submit];
        let poll = body.find("wgpu::Maintain::Poll").unwrap();
        let queue_empty = body.find("if queue_empty").unwrap();
        let blocking_wait = body
            .find("wgpu::Maintain::WaitForSubmissionIndex(submission)")
            .unwrap();
        assert!(poll < queue_empty && queue_empty < blocking_wait);
        assert!(body.contains("self.previous_submission.take()"));
    }

    #[test]
    fn wait_timestamp_remains_disabled_without_frame_phase_profiling() {
        let source = include_str!("frame_flight.rs");
        let wait = source.find("fn wait_for_previous_submission(").unwrap();
        let submit = source.find("pub(super) fn submit_and_present(").unwrap();
        let body = &source[wait..submit];
        assert!(body.contains("profile_wait.then(Instant::now)"));
        assert!(!body.contains("Instant::now();"));
    }
}
