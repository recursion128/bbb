use std::time::{Duration, Instant};

#[cfg(target_os = "macos")]
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Receiver, SyncSender},
    Arc,
};

use anyhow::{anyhow, Result};

use crate::counters::RendererCounters;

#[cfg(target_os = "macos")]
const COMPLETION_OBSERVATION_CAPACITY: usize = 1;

/// The explicit in-flight budget retained by the behavior-preserving first
/// FrameFlightController slice. Only uncapped CAMetalLayer presentation uses
/// it; FIFO already supplies presentation backpressure.
const MACOS_IMMEDIATE_IN_FLIGHT_BUDGET: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrameFlightPlatform {
    MacOs,
    Other,
}

impl FrameFlightPlatform {
    const CURRENT: Self = if cfg!(target_os = "macos") {
        Self::MacOs
    } else {
        Self::Other
    };
}

fn in_flight_budget(
    platform: FrameFlightPlatform,
    present_mode: wgpu::PresentMode,
) -> Option<usize> {
    (platform == FrameFlightPlatform::MacOs && present_mode == wgpu::PresentMode::Immediate)
        .then_some(MACOS_IMMEDIATE_IN_FLIGHT_BUDGET)
}

fn in_flight_depth_after_submit(
    platform: FrameFlightPlatform,
    present_mode: wgpu::PresentMode,
) -> u32 {
    u32::from(in_flight_budget(platform, present_mode).is_some())
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
                #[cfg(test)]
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

enum InjectedOrAcquiredSurface {
    Frame(FrameTarget),
    Status(wgpu::SurfaceStatus),
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

    fn present(self, queue: &wgpu::Queue) {
        match self {
            Self::Surface(frame) => queue.present(frame),
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
    /// Age of the retired submission when its wgpu callback was observed.
    /// This is callback-delivery time, not the exact GPU completion instant.
    pub(super) retired_submission_render_to_callback: Duration,
    pub(super) retired_submission_submit_to_callback: Duration,
    pub(super) retired_submission_callback_to_wait_return: Duration,
    pub(super) completion_observation_expected: bool,
    pub(super) completion_observation_missing: bool,
    pub(super) completion_observations_dropped: u64,
}

pub(super) struct FrameFlightSubmission {
    pub(super) queue_submit: Duration,
    pub(super) swap_buffers_present: Duration,
    pub(super) in_flight_depth_at_submit: u32,
}

#[derive(Default)]
struct PreviousSubmissionSynchronization {
    poll: Duration,
    blocking_wait: Duration,
    total: Duration,
    render_to_callback: Duration,
    submit_to_callback: Duration,
    callback_to_wait_return: Duration,
    observation_expected: bool,
    observation_missing: bool,
    observations_dropped: u64,
}

#[cfg(target_os = "macos")]
struct SubmissionCompletionObservation {
    id: u64,
    render_to_callback: Duration,
    submit_to_callback: Duration,
    callback_at: Instant,
}

#[cfg(target_os = "macos")]
fn send_completion_observation(
    sender: &SyncSender<SubmissionCompletionObservation>,
    dropped: &AtomicU64,
    observation: SubmissionCompletionObservation,
) {
    if sender.try_send(observation).is_err() {
        dropped.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(target_os = "macos")]
struct PendingSubmission {
    index: wgpu::SubmissionIndex,
    completion_observation_id: Option<u64>,
}

/// Owns the backend frame-flight lifecycle without owning visible frame work.
/// macOS Immediate retains one in-flight submission and waits on the exact
/// prior fence before drawable acquisition.
pub(super) struct FrameFlightController {
    #[cfg(target_os = "macos")]
    previous_submission: Option<PendingSubmission>,
    #[cfg(target_os = "macos")]
    completion_sender: SyncSender<SubmissionCompletionObservation>,
    #[cfg(target_os = "macos")]
    completion_receiver: Receiver<SubmissionCompletionObservation>,
    #[cfg(target_os = "macos")]
    completion_observations_dropped: Arc<AtomicU64>,
    #[cfg(target_os = "macos")]
    next_completion_observation_id: u64,
}

impl FrameFlightController {
    pub(super) fn new() -> Self {
        #[cfg(target_os = "macos")]
        let (completion_sender, completion_receiver) =
            mpsc::sync_channel(COMPLETION_OBSERVATION_CAPACITY);
        Self {
            #[cfg(target_os = "macos")]
            previous_submission: None,
            #[cfg(target_os = "macos")]
            completion_sender,
            #[cfg(target_os = "macos")]
            completion_receiver,
            #[cfg(target_os = "macos")]
            completion_observations_dropped: Arc::new(AtomicU64::new(0)),
            #[cfg(target_os = "macos")]
            next_completion_observation_id: 1,
        }
    }

    pub(super) fn acquire_frame(
        &mut self,
        surface: &mut RenderSurface,
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        counters: &mut RendererCounters,
        injected_fault: Option<wgpu::SurfaceStatus>,
        profile_wait: bool,
        profile_completion: bool,
        render_encoding_started_at: Instant,
    ) -> FrameFlightAcquisition {
        let previous_submission_result = self.wait_for_previous_submission(
            device,
            config.present_mode,
            profile_wait,
            profile_completion,
        );
        let render_duration_before_surface = render_encoding_started_at.elapsed();
        let (previous_submission, frame, surface_acquire) = match previous_submission_result {
            Ok(previous_submission) => {
                let surface_acquire_started_at = Instant::now();
                let frame = surface.acquire_frame(device, config, counters, injected_fault);
                (
                    previous_submission,
                    frame,
                    surface_acquire_started_at.elapsed(),
                )
            }
            Err(error) => (
                PreviousSubmissionSynchronization::default(),
                Err(error),
                Duration::ZERO,
            ),
        };
        FrameFlightAcquisition {
            frame,
            previous_submission_poll: previous_submission.poll,
            previous_submission_blocking_wait: previous_submission.blocking_wait,
            previous_submission_wait: previous_submission.total,
            render_duration_before_surface,
            surface_acquire,
            retired_submission_render_to_callback: previous_submission.render_to_callback,
            retired_submission_submit_to_callback: previous_submission.submit_to_callback,
            retired_submission_callback_to_wait_return: previous_submission.callback_to_wait_return,
            completion_observation_expected: previous_submission.observation_expected,
            completion_observation_missing: previous_submission.observation_missing,
            completion_observations_dropped: previous_submission.observations_dropped,
        }
    }

    fn wait_for_previous_submission(
        &mut self,
        device: &wgpu::Device,
        present_mode: wgpu::PresentMode,
        profile_wait: bool,
        profile_completion: bool,
    ) -> Result<PreviousSubmissionSynchronization> {
        #[cfg(target_os = "macos")]
        {
            let budget = in_flight_budget(FrameFlightPlatform::CURRENT, present_mode);
            debug_assert!(budget.is_none() || budget == Some(1));
            if budget == Some(1) {
                if let Some(submission) = self.previous_submission.take() {
                    let synchronization_started_at = profile_wait.then(Instant::now);
                    let wait_started_at = profile_wait.then(Instant::now);
                    let poll_result = device.poll(wgpu::PollType::Wait {
                        submission_index: Some(submission.index.clone()),
                        timeout: None,
                    });
                    let blocking_wait = wait_started_at
                        .map(|started_at| started_at.elapsed())
                        .unwrap_or_default();
                    if let Err(error) = poll_result {
                        self.previous_submission = Some(submission);
                        return Err(error.into());
                    }
                    let callback = profile_completion
                        .then(|| {
                            self.take_completion_observation(submission.completion_observation_id)
                        })
                        .flatten();
                    if !profile_completion {
                        self.drain_completion_observations();
                    }
                    let observations_dropped = if profile_completion {
                        self.completion_observations_dropped
                            .swap(0, Ordering::Relaxed)
                    } else {
                        self.completion_observations_dropped
                            .store(0, Ordering::Relaxed);
                        0
                    };
                    return Ok(PreviousSubmissionSynchronization {
                        // Schema v2 retains the rejected completion-poll phase
                        // as an explicit zero rather than changing old columns.
                        poll: Duration::ZERO,
                        blocking_wait,
                        total: synchronization_started_at
                            .map(|started_at| started_at.elapsed())
                            .unwrap_or_default(),
                        render_to_callback: callback
                            .as_ref()
                            .map(|value| value.render_to_callback)
                            .unwrap_or_default(),
                        submit_to_callback: callback
                            .as_ref()
                            .map(|value| value.submit_to_callback)
                            .unwrap_or_default(),
                        callback_to_wait_return: callback
                            .as_ref()
                            .map(|value| value.callback_at.elapsed())
                            .unwrap_or_default(),
                        observation_expected: profile_completion
                            && submission.completion_observation_id.is_some(),
                        observation_missing: profile_completion
                            && submission.completion_observation_id.is_some()
                            && callback.is_none(),
                        observations_dropped,
                    });
                }
            }
            Ok(PreviousSubmissionSynchronization::default())
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (device, present_mode, profile_wait, profile_completion);
            Ok(PreviousSubmissionSynchronization::default())
        }
    }

    #[cfg(target_os = "macos")]
    fn take_completion_observation(
        &self,
        expected_id: Option<u64>,
    ) -> Option<SubmissionCompletionObservation> {
        let Some(expected_id) = expected_id else {
            self.drain_completion_observations();
            return None;
        };
        while let Ok(observation) = self.completion_receiver.try_recv() {
            if observation.id == expected_id {
                return Some(observation);
            }
        }
        None
    }

    #[cfg(target_os = "macos")]
    fn drain_completion_observations(&self) {
        while self.completion_receiver.try_recv().is_ok() {}
    }

    pub(super) fn submit_and_present(
        &mut self,
        queue: &wgpu::Queue,
        encoder: wgpu::CommandEncoder,
        frame: FrameTarget,
        present_mode: wgpu::PresentMode,
        profile_completion: bool,
        render_started_at: Instant,
    ) -> FrameFlightSubmission {
        let submit_started_at = Instant::now();
        let submission = queue.submit(Some(encoder.finish()));
        #[cfg(target_os = "macos")]
        let in_flight_depth_at_submit = {
            let completion_observation_id =
                if profile_completion && present_mode == wgpu::PresentMode::Immediate {
                    let id = self.next_completion_observation_id;
                    self.next_completion_observation_id =
                        self.next_completion_observation_id.wrapping_add(1).max(1);
                    let sender = self.completion_sender.clone();
                    let dropped = self.completion_observations_dropped.clone();
                    queue.on_submitted_work_done(move || {
                        let observation = SubmissionCompletionObservation {
                            id,
                            render_to_callback: render_started_at.elapsed(),
                            submit_to_callback: submit_started_at.elapsed(),
                            callback_at: Instant::now(),
                        };
                        send_completion_observation(&sender, &dropped, observation);
                    });
                    Some(id)
                } else {
                    None
                };
            self.previous_submission = Some(PendingSubmission {
                index: submission,
                completion_observation_id,
            });
            let in_flight_depth_at_submit = u32::from(self.previous_submission.is_some());
            debug_assert_eq!(
                in_flight_depth_at_submit,
                in_flight_depth_after_submit(FrameFlightPlatform::CURRENT, present_mode)
            );
            in_flight_depth_at_submit
        };
        #[cfg(not(target_os = "macos"))]
        let in_flight_depth_at_submit = {
            let _ = (submission, profile_completion, render_started_at);
            in_flight_depth_after_submit(FrameFlightPlatform::CURRENT, present_mode)
        };
        let queue_submit = submit_started_at.elapsed();

        let present_started_at = Instant::now();
        frame.present(queue);
        FrameFlightSubmission {
            queue_submit,
            swap_buffers_present: present_started_at.elapsed(),
            in_flight_depth_at_submit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        in_flight_budget, in_flight_depth_after_submit, FrameFlightPlatform,
        MACOS_IMMEDIATE_IN_FLIGHT_BUDGET,
    };

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
        assert_eq!(
            in_flight_depth_after_submit(FrameFlightPlatform::MacOs, wgpu::PresentMode::Immediate,),
            1
        );
        assert_eq!(
            in_flight_depth_after_submit(FrameFlightPlatform::Other, wgpu::PresentMode::Immediate,),
            0
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
            .find("self.previous_submission = Some(PendingSubmission")
            .map(|offset| submit + offset)
            .unwrap();
        let present = source[record_submission..]
            .find("frame.present(queue)")
            .map(|offset| record_submission + offset)
            .unwrap();
        assert!(submit < record_submission && record_submission < present);
        assert!(source.contains("Self::Surface(frame) => queue.present(frame)"));
    }

    #[test]
    fn fixed_budget_wait_does_not_restore_the_rejected_completion_poll() {
        let source = include_str!("frame_flight.rs");
        let wait = source.find("fn wait_for_previous_submission(").unwrap();
        let submit = source.find("pub(super) fn submit_and_present(").unwrap();
        let body = &source[wait..submit];
        let blocking_wait = body.find("wgpu::PollType::Wait").unwrap();
        assert!(blocking_wait > 0);
        assert!(!body.contains("wgpu::PollType::Poll"));
        assert!(!body.contains("is_queue_empty"));
        assert!(body.contains("self.previous_submission.take()"));
        assert!(body.contains("submission_index: Some(submission.index.clone())"));
        assert!(body.contains("timeout: None"));
        assert!(body.contains("self.previous_submission = Some(submission)"));
        assert!(body.contains("poll: Duration::ZERO"));
    }

    #[test]
    fn wgpu30_surface_statuses_are_all_handled_fail_closed() {
        let source = include_str!("frame_flight.rs");
        for status in [
            "Success(frame)",
            "Suboptimal(frame)",
            "Timeout",
            "Occluded",
            "Outdated",
            "Lost",
            "Validation",
        ] {
            assert!(
                source.contains(&format!("wgpu::CurrentSurfaceTexture::{status}")),
                "missing CurrentSurfaceTexture::{status}"
            );
        }
        assert!(source.contains("SurfaceStatus::Suboptimal"));
        assert!(source.contains("SurfaceStatus::Occluded"));
        assert!(source.contains("surface acquisition validation error"));
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

    #[test]
    fn completion_callback_is_guarded_by_profiler_and_macos_immediate() {
        let source = include_str!("frame_flight.rs");
        let submit = source.find("pub(super) fn submit_and_present(").unwrap();
        let tests = source[submit..]
            .find("#[cfg(test)]")
            .map(|offset| submit + offset)
            .unwrap();
        let body = &source[submit..tests];
        let guard = body
            .find("if profile_completion")
            .expect("profiler completion guard");
        let immediate = body[guard..]
            .find("present_mode == wgpu::PresentMode::Immediate")
            .map(|offset| guard + offset)
            .expect("Immediate-only guard");
        let callback = body[immediate..]
            .find("queue.on_submitted_work_done")
            .map(|offset| immediate + offset)
            .expect("wgpu completion observation");
        let record = body[callback..]
            .find("self.previous_submission = Some(PendingSubmission")
            .map(|offset| callback + offset)
            .expect("submission ownership");
        let present = body[record..]
            .find("frame.present(queue)")
            .map(|offset| record + offset)
            .expect("drawable present");
        assert!(guard < immediate && immediate < callback && callback < record && record < present);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn completion_observations_match_exact_submission_and_report_channel_overflow() {
        use std::{
            sync::atomic::Ordering,
            time::{Duration, Instant},
        };

        use super::{
            send_completion_observation, FrameFlightController, SubmissionCompletionObservation,
        };

        let controller = FrameFlightController::new();
        controller
            .completion_sender
            .try_send(SubmissionCompletionObservation {
                id: 1,
                render_to_callback: Duration::from_millis(3),
                submit_to_callback: Duration::from_millis(1),
                callback_at: Instant::now(),
            })
            .unwrap();
        assert!(controller.take_completion_observation(Some(2)).is_none());
        controller
            .completion_sender
            .try_send(SubmissionCompletionObservation {
                id: 2,
                render_to_callback: Duration::from_millis(4),
                submit_to_callback: Duration::from_millis(2),
                callback_at: Instant::now(),
            })
            .unwrap();
        let matched = controller
            .take_completion_observation(Some(2))
            .expect("matching observation");
        assert_eq!(matched.id, 2);
        assert!(matched.render_to_callback >= matched.submit_to_callback);

        controller
            .completion_sender
            .try_send(SubmissionCompletionObservation {
                id: 99,
                render_to_callback: Duration::from_millis(2),
                submit_to_callback: Duration::from_millis(1),
                callback_at: Instant::now(),
            })
            .unwrap();
        assert!(controller.take_completion_observation(None).is_none());
        assert!(controller.completion_receiver.try_recv().is_err());

        for id in 3..=4 {
            send_completion_observation(
                &controller.completion_sender,
                &controller.completion_observations_dropped,
                SubmissionCompletionObservation {
                    id,
                    render_to_callback: Duration::from_millis(2),
                    submit_to_callback: Duration::from_millis(1),
                    callback_at: Instant::now(),
                },
            );
        }
        assert_eq!(
            controller
                .completion_observations_dropped
                .load(Ordering::Relaxed),
            1
        );
    }
}
