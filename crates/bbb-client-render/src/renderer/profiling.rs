use super::*;

pub const WGPU_VERSION: &str = "30.0.0";

pub const WGPU_CORE_VERSION: &str = "30.0.0";

pub const WGPU_HAL_VERSION: &str = "30.0.0";

pub const WGPU_TYPES_VERSION: &str = "30.0.0";

pub const NAGA_VERSION: &str = "30.0.0";

pub const WGPU_METAL_FENCE_UPSTREAM_COMMIT: &str = "e0a6065c65b5b6c2e92ee7052ed084cd8910d486";

pub const WGPU_METAL_FENCE_STRATEGY: &str = "completion-handler-condition-variable";

#[cfg(feature = "lifecycle-soak")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleSoakSurfaceError {
    Lost,
    Outdated,
    Timeout,
    Occluded,
    Suboptimal,
    Validation,
}

#[cfg(feature = "lifecycle-soak")]
impl LifecycleSoakSurfaceError {
    fn into_surface_status(self) -> wgpu::SurfaceStatus {
        match self {
            Self::Lost => wgpu::SurfaceStatus::Lost,
            Self::Outdated => wgpu::SurfaceStatus::Outdated,
            Self::Timeout => wgpu::SurfaceStatus::Timeout,
            Self::Occluded => wgpu::SurfaceStatus::Occluded,
            Self::Suboptimal => wgpu::SurfaceStatus::Suboptimal,
            Self::Validation => wgpu::SurfaceStatus::Validation,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RendererFrameProfile {
    pub frame_interval: Duration,
    pub gpu_async: Duration,
    pub surface_acquire: Duration,
    pub render: Duration,
    pub present: Duration,
    pub swap_buffers: Duration,
    /// CPU command encoding before surface acquisition.
    pub cpu_encode_before_wait: Duration,
    /// No application-level submission fences are used; synchronization fields stay zero.
    pub previous_submission_wait: Duration,
    pub previous_submission_poll: Duration,
    /// Blocking fence-wait subset of `previous_submission_wait`.
    pub previous_submission_blocking_wait: Duration,
    /// Wait for asynchronous world command completion (zero for one frame encoder).
    pub world_command_finish_wait: Duration,
    /// Final swapchain blit and timestamp encoding after surface acquisition.
    pub presentation_tail_encode: Duration,
    /// Main-thread time spent finishing, waiting for terrain commands, and submitting.
    pub queue_submit: Duration,
    /// Total time spent inside command encoder finish calls, including work on the terrain worker.
    pub command_encoder_finish: Duration,
    /// Public `Queue::submit` call after command-buffer finalization.
    pub queue_submit_api: Duration,
    /// Surface drawable presentation (`SurfaceTexture::present`).
    pub swap_buffers_present: Duration,
    /// No post-present submission is issued.
    pub post_present_fence_submit: Duration,
    /// Async GPU timestamp completion polling at render entry.
    pub gpu_async_poll: Duration,
    /// Complete successful `Renderer::render` call, including its publication tail.
    pub render_call_total: Duration,
    /// Render-start age when wgpu delivered the retired submission callback.
    /// This is an observation timestamp, not the exact GPU completion instant.
    pub retired_submission_render_to_callback: Duration,
    /// Queue-submit age when wgpu delivered the retired submission callback.
    pub retired_submission_submit_to_callback: Duration,
    /// Time from callback delivery until the exact prior-submission wait returned.
    pub retired_submission_callback_to_wait_return: Duration,
    /// Unobserved submission depth; zero is not an empty-queue measurement.
    pub in_flight_depth_at_submit: u32,
    /// The retired submission registered a completion observation callback.
    pub completion_observation_expected: bool,
    /// The exact prior submission expected an observation but none was received.
    pub completion_observation_missing: bool,
    /// Completion observations rejected because the bounded channel was full.
    pub completion_observations_dropped: u64,
    /// One profiler-only local-input receipt completed by this successful
    /// present call. This is a software boundary, not display/photon latency.
    pub input_latency: Option<RendererInputLatencyProfile>,
}

/// Input receipt after simulation application and RendererFrame extraction,
/// waiting for association with one successful submit/present operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RendererInputLatencyStamp {
    pub sequence: u64,
    pub enqueued_at: Instant,
    pub simulation_applied_at: Instant,
    pub renderer_extracted_at: Instant,
    pub superseded_events: u64,
}

/// Profiler-only software input latency stages for one represented input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RendererInputLatencyProfile {
    pub sequence: u64,
    pub enqueue_to_simulation_apply: Duration,
    pub simulation_apply_to_renderer_extract: Duration,
    pub renderer_extract_to_queue_submit: Duration,
    pub queue_submit_to_present_call_return: Duration,
    pub enqueue_to_present_call_return: Duration,
    pub superseded_events: u64,
    pub out_of_order_events: u64,
}

#[derive(Debug, Default)]
pub(crate) struct RendererInputLatencyState {
    pending: Option<RendererInputLatencyStamp>,
    last_sequence_seen: u64,
    out_of_order_events: u64,
}

impl RendererInputLatencyState {
    pub(super) fn push(&mut self, stamp: RendererInputLatencyStamp) {
        if stamp.sequence == self.last_sequence_seen {
            return;
        }
        if stamp.sequence < self.last_sequence_seen {
            self.out_of_order_events = self.out_of_order_events.saturating_add(1);
            return;
        }
        self.last_sequence_seen = stamp.sequence;
        self.pending = Some(stamp);
    }

    pub(crate) fn complete(
        &mut self,
        queue_submitted_at: Instant,
        present_call_returned_at: Instant,
    ) -> Option<RendererInputLatencyProfile> {
        let stamp = self.pending.take()?;
        Some(RendererInputLatencyProfile {
            sequence: stamp.sequence,
            enqueue_to_simulation_apply: stamp
                .simulation_applied_at
                .saturating_duration_since(stamp.enqueued_at),
            simulation_apply_to_renderer_extract: stamp
                .renderer_extracted_at
                .saturating_duration_since(stamp.simulation_applied_at),
            renderer_extract_to_queue_submit: queue_submitted_at
                .saturating_duration_since(stamp.renderer_extracted_at),
            queue_submit_to_present_call_return: present_call_returned_at
                .saturating_duration_since(queue_submitted_at),
            enqueue_to_present_call_return: present_call_returned_at
                .saturating_duration_since(stamp.enqueued_at),
            superseded_events: stamp.superseded_events,
            out_of_order_events: std::mem::take(&mut self.out_of_order_events),
        })
    }
}

impl Renderer {
    pub fn counters(&self) -> RendererCounters {
        self.counters.clone()
    }

    pub fn counters_ref(&self) -> &RendererCounters {
        &self.counters
    }

    /// Submits queued resource writes and waits for that upload-only
    /// submission to finish. Startup/resource-generation callers use this at
    /// logical batch boundaries so native `Queue::write_*` staging allocations
    /// do not all remain live until the first presented frame.
    ///
    /// This submission does not acquire or present a surface texture.
    pub fn finish_upload_batch(&self) -> Result<()> {
        let submission = self.queue.submit([]);
        #[cfg(not(target_arch = "wasm32"))]
        self.device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        })?;
        #[cfg(target_arch = "wasm32")]
        let _ = submission;
        Ok(())
    }

    #[cfg(feature = "lifecycle-soak")]
    pub fn inject_lifecycle_soak_surface_error(&mut self, error: LifecycleSoakSurfaceError) {
        self.lifecycle_soak_surface_faults
            .push_back(error.into_surface_status());
    }

    #[cfg(feature = "lifecycle-soak")]
    pub fn record_lifecycle_soak_window_resize_request(&mut self) {
        self.counters.window_resize_requests =
            self.counters.window_resize_requests.saturating_add(1);
    }

    #[cfg(feature = "lifecycle-soak")]
    pub fn record_lifecycle_soak_window_minimize_request(&mut self) {
        self.counters.window_minimize_requests =
            self.counters.window_minimize_requests.saturating_add(1);
    }

    #[cfg(feature = "lifecycle-soak")]
    pub fn record_lifecycle_soak_window_restore_request(&mut self) {
        self.counters.window_restore_requests =
            self.counters.window_restore_requests.saturating_add(1);
    }

    pub fn set_gpu_timing_enabled(&mut self, enabled: bool) {
        if let Some(timing) = self.gpu_timing.as_mut() {
            timing.set_enabled(enabled);
        }
        self.frame_phase_profiling_enabled = enabled;
    }

    /// Enables profiler-only command finalization and queue-submit timestamps.
    pub fn set_submission_profiling_enabled(&mut self, enabled: bool) {
        self.submission_profiling_enabled = enabled;
    }

    /// Associates the newest extracted platform input with the next
    /// successful surface submission. Repeated immutable publications are
    /// ignored; the simulation publication owner has already counted any
    /// unpresented receipt superseded by a newer publication.
    pub fn set_input_latency_stamp(&mut self, stamp: RendererInputLatencyStamp) {
        self.input_latency.push(stamp);
    }

    pub fn set_level_frame_route(&mut self, route: LevelFrameRoute) {
        self.level_frame_route = route;
    }

    pub fn gpu_utilization(&self) -> Option<f64> {
        self.gpu_timing
            .as_ref()
            .and_then(GpuFrameTiming::latest_utilization)
    }

    pub fn take_gpu_timing_profile(
        &mut self,
    ) -> Option<crate::gpu_timing::RendererGpuTimingProfile> {
        self.gpu_timing
            .as_mut()
            .and_then(GpuFrameTiming::take_completed_profile)
    }

    pub fn last_frame_profile(&self) -> RendererFrameProfile {
        self.last_frame_profile
    }
}
