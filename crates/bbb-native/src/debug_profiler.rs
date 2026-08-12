use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};

use bbb_control::RendererCounters;
use bbb_renderer::{
    HudDebugProfilerChart, HudDebugProfilerSlice, RendererFrameProfile, RendererGpuTimingProfile,
};

use self::report::{
    ProfilingReport, ProfilingReportCompletion, ProfilingReportWriter, RecordedDeviation,
    RecordedMetricsFrame,
};

mod report;

const ROOT_PATH: &str = "root";
const UNSPECIFIED: &str = "unspecified";
const PATH_SEPARATOR: char = '\u{001e}';
const MAX_PATH_DEPTH: usize = 16;
const MAX_SECTIONS_PER_FRAME: usize = 256;
const MAX_RECORDING_FRAMES: usize = 2_800;
const TICK_TIME_DEVIATION_INCREASE: f64 = 2.0;
const PROFILING_MAX_DURATION: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy)]
pub(crate) struct DebugProfilerSectionSample {
    /// Path below `root`, in parent-to-child order.
    pub(crate) path: &'static [&'static str],
    pub(crate) duration: Duration,
    /// False keeps a last-published parallel-owner section visible in F3+1
    /// without counting it again in a higher-rate render recording.
    pub(crate) record_in_report: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DebugProfilerFrameMetrics {
    pub(crate) tick_time_nanos: f64,
    pub(crate) process_memory_mib: Option<f64>,
    pub(crate) cpu_load_percent: Vec<f64>,
    pub(crate) total_chunks: Option<f64>,
    pub(crate) rendered_chunks: Option<f64>,
    pub(crate) last_view_distance: Option<f64>,
    pub(crate) compile_queue_size: Option<f64>,
    pub(crate) gpu_utilization: Option<f64>,
    pub(crate) gpu_duration_nanos: Option<f64>,
    pub(crate) gpu_stages: Option<DebugProfilerGpuStages>,
    pub(crate) frame_phases: Option<DebugProfilerFramePhases>,
    pub(crate) frame_flight: Option<DebugProfilerFrameFlight>,
    pub(crate) input_latency: Option<DebugProfilerInputLatency>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DebugProfilerInputLatency {
    pub(crate) sequence: f64,
    pub(crate) enqueue_to_simulation_apply: f64,
    pub(crate) simulation_apply_to_renderer_extract: f64,
    pub(crate) renderer_extract_to_queue_submit: f64,
    pub(crate) queue_submit_to_present_call_return: f64,
    pub(crate) enqueue_to_present_call_return: f64,
    pub(crate) superseded_events: f64,
    pub(crate) out_of_order_events: f64,
    pub(crate) present_call_return_software_boundary: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DebugProfilerFramePhases {
    pub(crate) application_frame_interval: f64,
    pub(crate) cpu_encode_before_wait: f64,
    pub(crate) previous_submission_wait: f64,
    pub(crate) previous_submission_poll: f64,
    pub(crate) previous_submission_blocking_wait: f64,
    pub(crate) surface_acquire: f64,
    pub(crate) presentation_tail_encode: f64,
    pub(crate) queue_submit: f64,
    pub(crate) swap_buffers_present: f64,
    pub(crate) gpu_async_poll: f64,
    pub(crate) render_call_total: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DebugProfilerFrameFlight {
    pub(crate) render_to_callback_observed: f64,
    pub(crate) submit_to_callback_observed: f64,
    pub(crate) callback_to_wait_return: f64,
    pub(crate) in_flight_depth_at_submit: f64,
    pub(crate) completion_observation_expected: f64,
    pub(crate) completion_observation_missing: f64,
    pub(crate) completion_observations_dropped: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DebugProfilerGpuStages {
    pub(crate) application_frame_interval: f64,
    pub(crate) total_gpu_duration: f64,
    pub(crate) terrain_atlas_animation: f64,
    pub(crate) opaque_world: f64,
    pub(crate) opaque_particles: f64,
    pub(crate) feature_depth_copies: f64,
    pub(crate) entity_translucent_features: f64,
    pub(crate) remaining_feature_targets: f64,
    pub(crate) clouds: f64,
    pub(crate) weather: f64,
    pub(crate) transparency_combine: f64,
    pub(crate) presentation_tail: f64,
    pub(crate) encoder_tail: f64,
}

impl DebugProfilerGpuStages {
    fn from_profile(timing: RendererGpuTimingProfile) -> Option<Self> {
        let stages = timing.stages?;
        Some(Self {
            application_frame_interval: duration_nanos(timing.application_frame_interval) as f64,
            total_gpu_duration: duration_nanos(timing.duration) as f64,
            terrain_atlas_animation: duration_nanos(stages.terrain_atlas_animation) as f64,
            opaque_world: duration_nanos(stages.opaque_world) as f64,
            opaque_particles: duration_nanos(stages.opaque_particles) as f64,
            feature_depth_copies: duration_nanos(stages.feature_depth_copies) as f64,
            entity_translucent_features: duration_nanos(stages.entity_translucent_features) as f64,
            remaining_feature_targets: duration_nanos(stages.remaining_feature_targets) as f64,
            clouds: duration_nanos(stages.clouds) as f64,
            weather: duration_nanos(stages.weather) as f64,
            transparency_combine: duration_nanos(stages.transparency_combine) as f64,
            presentation_tail: duration_nanos(stages.presentation_tail) as f64,
            encoder_tail: duration_nanos(stages.encoder_tail) as f64,
        })
    }
}

impl From<RendererFrameProfile> for DebugProfilerFramePhases {
    fn from(profile: RendererFrameProfile) -> Self {
        Self {
            application_frame_interval: duration_nanos(profile.frame_interval) as f64,
            cpu_encode_before_wait: duration_nanos(profile.cpu_encode_before_wait) as f64,
            previous_submission_wait: duration_nanos(profile.previous_submission_wait) as f64,
            previous_submission_poll: duration_nanos(profile.previous_submission_poll) as f64,
            previous_submission_blocking_wait: duration_nanos(
                profile.previous_submission_blocking_wait,
            ) as f64,
            surface_acquire: duration_nanos(profile.surface_acquire) as f64,
            presentation_tail_encode: duration_nanos(profile.presentation_tail_encode) as f64,
            queue_submit: duration_nanos(profile.queue_submit) as f64,
            swap_buffers_present: duration_nanos(profile.swap_buffers_present) as f64,
            gpu_async_poll: duration_nanos(profile.gpu_async_poll) as f64,
            render_call_total: duration_nanos(profile.render_call_total) as f64,
        }
    }
}

impl From<RendererFrameProfile> for DebugProfilerFrameFlight {
    fn from(profile: RendererFrameProfile) -> Self {
        Self {
            render_to_callback_observed: duration_nanos(
                profile.retired_submission_render_to_callback,
            ) as f64,
            submit_to_callback_observed: duration_nanos(
                profile.retired_submission_submit_to_callback,
            ) as f64,
            callback_to_wait_return: duration_nanos(
                profile.retired_submission_callback_to_wait_return,
            ) as f64,
            in_flight_depth_at_submit: f64::from(profile.in_flight_depth_at_submit),
            completion_observation_expected: f64::from(
                profile.completion_observation_expected as u8,
            ),
            completion_observation_missing: f64::from(profile.completion_observation_missing as u8),
            completion_observations_dropped: profile.completion_observations_dropped as f64,
        }
    }
}

impl DebugProfilerFrameMetrics {
    pub(crate) fn from_renderer(
        tick_time: Duration,
        counters: &RendererCounters,
        render_distance_chunks: u32,
        gpu_utilization: Option<f64>,
        gpu_timing: Option<RendererGpuTimingProfile>,
        frame_profile: RendererFrameProfile,
    ) -> Self {
        let gpu_stages = gpu_timing.and_then(DebugProfilerGpuStages::from_profile);
        let input_latency = frame_profile
            .input_latency
            .map(|latency| DebugProfilerInputLatency {
                sequence: latency.sequence as f64,
                enqueue_to_simulation_apply: duration_nanos(latency.enqueue_to_simulation_apply)
                    as f64,
                simulation_apply_to_renderer_extract: duration_nanos(
                    latency.simulation_apply_to_renderer_extract,
                ) as f64,
                renderer_extract_to_queue_submit: duration_nanos(
                    latency.renderer_extract_to_queue_submit,
                ) as f64,
                queue_submit_to_present_call_return: duration_nanos(
                    latency.queue_submit_to_present_call_return,
                ) as f64,
                enqueue_to_present_call_return: duration_nanos(
                    latency.enqueue_to_present_call_return,
                ) as f64,
                superseded_events: latency.superseded_events as f64,
                out_of_order_events: latency.out_of_order_events as f64,
                present_call_return_software_boundary: 1.0,
            });
        Self {
            tick_time_nanos: duration_nanos(tick_time) as f64,
            process_memory_mib: process_resident_memory_mib(),
            total_chunks: Some(counters.uploaded_sections as f64),
            rendered_chunks: Some(counters.visible_sections as f64),
            last_view_distance: Some(render_distance_chunks as f64),
            compile_queue_size: Some(counters.terrain_mesh_queue_depth as f64),
            gpu_utilization: finite_metric(gpu_utilization),
            gpu_duration_nanos: gpu_timing.map(|timing| duration_nanos(timing.duration) as f64),
            gpu_stages,
            frame_phases: Some(frame_profile.into()),
            frame_flight: Some(frame_profile.into()),
            input_latency,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DebugProfilerEvent {
    RecordingStarted,
    RecordingStopped {
        duration: Duration,
        ticks: usize,
        ticks_per_second: f64,
    },
    ReportSaved {
        path: PathBuf,
    },
    ReportFailed {
        error: String,
    },
}

#[derive(Debug)]
pub(crate) struct DebugProfiler {
    latest: ProfileTree,
    path: String,
    recording: Option<ActiveRecording>,
    report_writer: Option<ProfilingReportWriter>,
    report_identity: String,
    events: VecDeque<DebugProfilerEvent>,
    cpu_sampler: ProcCpuSampler,
}

impl Default for DebugProfiler {
    fn default() -> Self {
        Self {
            latest: ProfileTree::default(),
            path: String::new(),
            recording: None,
            report_writer: None,
            report_identity: "remote".to_string(),
            events: VecDeque::new(),
            cpu_sampler: ProcCpuSampler::default(),
        }
    }
}

impl DebugProfiler {
    pub(crate) fn with_report_writer(
        report_root: impl Into<PathBuf>,
        report_identity: impl Into<String>,
    ) -> Self {
        let mut profiler = Self {
            report_identity: report_identity.into(),
            ..Self::default()
        };
        match ProfilingReportWriter::spawn(report_root.into()) {
            Ok(writer) => profiler.report_writer = Some(writer),
            Err(error) => profiler.events.push_back(DebugProfilerEvent::ReportFailed {
                error: format!("failed to start profiling report writer: {error}"),
            }),
        }
        profiler
    }

    pub(crate) fn start_recording(&mut self, now: Instant) -> bool {
        if self.recording.is_some() {
            return false;
        }
        self.recording = Some(ActiveRecording::new(now));
        self.events.push_back(DebugProfilerEvent::RecordingStarted);
        true
    }

    pub(crate) fn stop_recording(&mut self) -> bool {
        let Some(recording) = self.recording.as_mut() else {
            return false;
        };
        recording.stop_requested = true;
        true
    }

    pub(crate) fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    pub(crate) fn record_frame(
        &mut self,
        now: Instant,
        total: Duration,
        sections: &[DebugProfilerSectionSample],
        mut metrics: DebugProfilerFrameMetrics,
    ) {
        self.latest = ProfileTree::from_frame(total, sections);
        if self.path.is_empty() || !self.latest.entries.contains_key(&self.path) {
            self.path = ROOT_PATH.to_string();
        }

        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        metrics.cpu_load_percent = self.cpu_sampler.sample().unwrap_or_default();
        let recording_profile = sections
            .iter()
            .any(|section| !section.record_in_report)
            .then(|| ProfileTree::from_recorded_frame(total, sections));
        recording.record_frame(
            now,
            recording_profile.as_ref().unwrap_or(&self.latest),
            metrics,
        );
        if recording.should_finish(now) {
            self.finish_recording(now);
        }
    }

    pub(crate) fn take_events(&mut self) -> Vec<DebugProfilerEvent> {
        if let Some(writer) = self.report_writer.as_mut() {
            for completion in writer.take_completions() {
                self.events.push_back(match completion {
                    ProfilingReportCompletion::Saved(path) => {
                        DebugProfilerEvent::ReportSaved { path }
                    }
                    ProfilingReportCompletion::Failed(error) => {
                        DebugProfilerEvent::ReportFailed { error }
                    }
                });
            }
        }
        self.events.drain(..).collect()
    }

    pub(crate) fn chart(&self) -> Option<HudDebugProfilerChart> {
        if self.latest.entries.is_empty() {
            return None;
        }
        let mut times = self.latest.get_times(&self.path);
        if times.is_empty() {
            return None;
        }
        let current = times.remove(0);
        Some(HudDebugProfilerChart {
            current_node_name: current.name,
            current_global_percentage: current.global_percentage,
            slices: times
                .into_iter()
                .map(|field| HudDebugProfilerSlice {
                    name: field.name,
                    percentage: field.percentage,
                    global_percentage: field.global_percentage,
                })
                .collect(),
        })
    }

    pub(crate) fn navigate(&mut self, digit: u8) {
        if self.latest.entries.is_empty() {
            return;
        }
        let mut times = self.latest.get_times(&self.path);
        if times.is_empty() {
            return;
        }
        let current = times.remove(0);
        if digit == 0 {
            if !current.name.is_empty() {
                if let Some(pos) = self.path.rfind(PATH_SEPARATOR) {
                    self.path.truncate(pos);
                }
            }
            return;
        }

        let child_index = usize::from(digit.saturating_sub(1));
        let Some(child) = times.get(child_index) else {
            return;
        };
        if child.name == UNSPECIFIED {
            return;
        }
        if !self.path.is_empty() {
            self.path.push(PATH_SEPARATOR);
        }
        self.path.push_str(&child.name);
    }

    pub(crate) fn current_path(&self) -> &str {
        if self.path.is_empty() {
            ROOT_PATH
        } else {
            &self.path
        }
    }

    fn finish_recording(&mut self, now: Instant) {
        let Some(recording) = self.recording.take() else {
            return;
        };
        let report = recording.finish(now, self.report_identity.clone());
        self.events.push_back(DebugProfilerEvent::RecordingStopped {
            duration: report.duration(),
            ticks: report.tick_count(),
            ticks_per_second: report.ticks_per_second(),
        });
        let Some(writer) = self.report_writer.as_ref() else {
            self.events.push_back(DebugProfilerEvent::ReportFailed {
                error: "profiling report writer is unavailable".to_string(),
            });
            return;
        };
        if let Err(error) = writer.submit(report) {
            self.events
                .push_back(DebugProfilerEvent::ReportFailed { error });
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct DebugProfilerEntry {
    duration_nanos: u64,
    count: u64,
    max_duration_nanos: u64,
}

impl DebugProfilerEntry {
    fn record(&mut self, duration_nanos: u64, count: u64, max_duration_nanos: u64) {
        self.duration_nanos = self.duration_nanos.saturating_add(duration_nanos);
        self.count = self.count.saturating_add(count);
        self.max_duration_nanos = self.max_duration_nanos.max(max_duration_nanos);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ProfileTree {
    entries: BTreeMap<String, DebugProfilerEntry>,
}

impl ProfileTree {
    fn from_frame(total: Duration, sections: &[DebugProfilerSectionSample]) -> Self {
        Self::from_frame_filter(total, sections, |_| true)
    }

    fn from_recorded_frame(total: Duration, sections: &[DebugProfilerSectionSample]) -> Self {
        Self::from_frame_filter(total, sections, |section| section.record_in_report)
    }

    fn from_frame_filter(
        total: Duration,
        sections: &[DebugProfilerSectionSample],
        include: impl Fn(&DebugProfilerSectionSample) -> bool,
    ) -> Self {
        let mut tree = Self::default();
        let total_nanos = duration_nanos(total);
        tree.entries.insert(
            ROOT_PATH.to_string(),
            DebugProfilerEntry {
                duration_nanos: total_nanos,
                count: 1,
                max_duration_nanos: total_nanos,
            },
        );
        for section in sections
            .iter()
            .filter(|section| include(section))
            .take(MAX_SECTIONS_PER_FRAME)
        {
            let Some(path) = profiler_path(section.path) else {
                continue;
            };
            let duration_nanos = duration_nanos(section.duration);
            tree.entries
                .entry(path)
                .or_default()
                .record(duration_nanos, 1, duration_nanos);
        }
        tree
    }

    fn merge(&mut self, frame: &Self) {
        for (path, entry) in &frame.entries {
            self.entries.entry(path.clone()).or_default().record(
                entry.duration_nanos,
                entry.count,
                entry.max_duration_nanos,
            );
        }
    }

    fn get_times(&self, raw_path: &str) -> Vec<DebugProfilerResultField> {
        let root_entry = self.entry(ROOT_PATH);
        let current_entry = self.entry(raw_path);
        let mut global_time = root_entry.duration_nanos.max(1);
        let self_time = current_entry.duration_nanos;
        let prefix = profiler_child_prefix(raw_path);
        let mut total_time = self
            .entries
            .keys()
            .filter(|key| is_direct_child(&prefix, key))
            .map(|key| self.entry(key).duration_nanos)
            .sum::<u64>();
        let old_time = total_time;
        total_time = total_time.max(self_time).max(1);
        global_time = global_time.max(total_time);

        let mut result = self
            .entries
            .iter()
            .filter(|(key, _)| is_direct_child(&prefix, key))
            .map(|(key, entry)| {
                let time = entry.duration_nanos;
                DebugProfilerResultField {
                    name: key[prefix.len()..].to_string(),
                    percentage: time as f64 * 100.0 / total_time as f64,
                    global_percentage: time as f64 * 100.0 / global_time as f64,
                    count: entry.count,
                }
            })
            .collect::<Vec<_>>();

        if total_time > old_time {
            let unspecified = total_time - old_time;
            result.push(DebugProfilerResultField {
                name: UNSPECIFIED.to_string(),
                percentage: unspecified as f64 * 100.0 / total_time as f64,
                global_percentage: unspecified as f64 * 100.0 / global_time as f64,
                count: current_entry.count,
            });
        }

        result.sort_by(|left, right| {
            right
                .percentage
                .partial_cmp(&left.percentage)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.name.cmp(&left.name))
        });
        result.insert(
            0,
            DebugProfilerResultField {
                name: raw_path.to_string(),
                percentage: 100.0,
                global_percentage: total_time as f64 * 100.0 / global_time as f64,
                count: current_entry.count,
            },
        );
        result
    }

    fn entry(&self, path: &str) -> DebugProfilerEntry {
        self.entries.get(path).copied().unwrap_or_default()
    }

    fn profiling_text(&self, duration: Duration, tick_count: usize) -> String {
        let duration_nanos = duration_nanos(duration);
        let duration_seconds = duration.as_secs_f64();
        let tick_rate = if duration_seconds > 0.0 {
            tick_count as f64 / duration_seconds
        } else {
            0.0
        };
        let mut output = String::new();
        output.push_str("---- bbb Native Profile Results ----\n");
        output.push_str(&format!("Version: {}\n", bbb_protocol::MC_VERSION));
        output.push_str(&format!("Time span: {} ms\n", duration_nanos / 1_000_000));
        output.push_str(&format!("Tick span: {tick_count} ticks\n"));
        output.push_str(&format!(
            "// This is approximately {tick_rate:.2} ticks per second. It should be 20 ticks per second\n\n"
        ));
        output.push_str("--- BEGIN PROFILE DUMP ---\n\n");
        self.append_profile_results(0, ROOT_PATH, tick_count.max(1), &mut output);
        output.push_str("--- END PROFILE DUMP ---\n");
        output
    }

    fn append_profile_results(
        &self,
        depth: usize,
        path: &str,
        tick_count: usize,
        output: &mut String,
    ) {
        let results = self.get_times(path);
        // Vanilla omits a branch unless getTimes contains the current node
        // plus at least two rows (normally a child and `unspecified`).
        if results.len() < 3 {
            return;
        }
        for result in results.iter().skip(1) {
            let calls_per_tick = (result.count as f64 / tick_count as f64).round() as u64;
            append_profile_indent(output, depth);
            output.push_str(&format!(
                "{}({}/{}) - {:.2}%/{:.2}%\n",
                result.name,
                result.count,
                calls_per_tick,
                result.percentage,
                result.global_percentage
            ));
            if result.name != UNSPECIFIED {
                self.append_profile_results(
                    depth + 1,
                    &format!("{path}{PATH_SEPARATOR}{}", result.name),
                    tick_count,
                    output,
                );
            }
        }
    }

    fn duration(&self) -> Duration {
        Duration::from_nanos(self.entry(ROOT_PATH).duration_nanos)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct DebugProfilerResultField {
    name: String,
    percentage: f64,
    global_percentage: f64,
    count: u64,
}

#[derive(Debug)]
struct ActiveRecording {
    started_at: Instant,
    deadline: Instant,
    stop_requested: bool,
    aggregate: ProfileTree,
    frames: Vec<RecordedMetricsFrame>,
    deviations: Vec<RecordedDeviation>,
    previous_tick_time_nanos: Option<f64>,
}

impl ActiveRecording {
    fn new(started_at: Instant) -> Self {
        Self {
            started_at,
            deadline: started_at + PROFILING_MAX_DURATION,
            stop_requested: false,
            aggregate: ProfileTree::default(),
            frames: Vec::new(),
            deviations: Vec::new(),
            previous_tick_time_nanos: None,
        }
    }

    fn record_frame(
        &mut self,
        recorded_at: Instant,
        profile: &ProfileTree,
        metrics: DebugProfilerFrameMetrics,
    ) {
        if self.frames.len() >= MAX_RECORDING_FRAMES {
            self.stop_requested = true;
            return;
        }
        self.aggregate.merge(profile);
        let tick = self.frames.len() + 1;
        if self
            .previous_tick_time_nanos
            .is_some_and(|previous| tick_time_deviated(previous, metrics.tick_time_nanos))
        {
            self.deviations
                .push(RecordedDeviation::new(tick, recorded_at, profile.clone()));
        }
        self.previous_tick_time_nanos = Some(metrics.tick_time_nanos);
        self.frames.push(RecordedMetricsFrame::new(tick, metrics));
    }

    fn should_finish(&self, now: Instant) -> bool {
        self.stop_requested || now > self.deadline || self.frames.len() >= MAX_RECORDING_FRAMES
    }

    fn finish(self, now: Instant, identity: String) -> ProfilingReport {
        ProfilingReport::new(
            identity,
            SystemTime::now(),
            now.saturating_duration_since(self.started_at),
            self.aggregate,
            self.frames,
            self.deviations,
        )
    }
}

fn profiler_path(segments: &[&str]) -> Option<String> {
    if segments.is_empty()
        || segments.len() > MAX_PATH_DEPTH
        || segments
            .iter()
            .any(|segment| segment.is_empty() || segment.contains(PATH_SEPARATOR))
    {
        return None;
    }
    let mut path = ROOT_PATH.to_string();
    for segment in segments {
        path.push(PATH_SEPARATOR);
        path.push_str(segment);
    }
    Some(path)
}

fn profiler_child_prefix(path: &str) -> String {
    if path.is_empty() {
        String::new()
    } else {
        format!("{path}{PATH_SEPARATOR}")
    }
}

fn is_direct_child(prefix: &str, path: &str) -> bool {
    path.len() > prefix.len()
        && path.starts_with(prefix)
        && !path[prefix.len()..].contains(PATH_SEPARATOR)
}

fn append_profile_indent(output: &mut String, depth: usize) {
    output.push_str(&format!("[{depth:02}] "));
    for _ in 0..depth {
        output.push_str("|   ");
    }
}

fn tick_time_deviated(previous: f64, current: f64) -> bool {
    previous.is_finite()
        && current.is_finite()
        && previous > 0.0
        && current > previous
        && (current - previous) / previous >= TICK_TIME_DEVIATION_INCREASE
}

fn process_resident_memory_mib() -> Option<f64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let kib = status.lines().find_map(|line| {
        let value = line.strip_prefix("VmRSS:")?.trim();
        value.split_ascii_whitespace().next()?.parse::<f64>().ok()
    })?;
    Some(kib / 1024.0)
}

#[derive(Debug, Default)]
struct ProcCpuSampler {
    previous: Option<Vec<(u64, u64)>>,
}

impl ProcCpuSampler {
    fn sample(&mut self) -> Option<Vec<f64>> {
        let contents = fs::read_to_string("/proc/stat").ok()?;
        let current = contents
            .lines()
            .skip(1)
            .take_while(|line| {
                line.strip_prefix("cpu")
                    .and_then(|rest| rest.chars().next())
                    .is_some_and(|first| first.is_ascii_digit())
            })
            .filter_map(parse_proc_cpu_line)
            .collect::<Vec<_>>();
        if current.is_empty() {
            return None;
        }
        let previous = self.previous.replace(current.clone())?;
        Some(
            current
                .into_iter()
                .zip(previous)
                .map(|((total, idle), (previous_total, previous_idle))| {
                    let total_delta = total.saturating_sub(previous_total);
                    let idle_delta = idle.saturating_sub(previous_idle);
                    if total_delta == 0 {
                        0.0
                    } else {
                        total_delta.saturating_sub(idle_delta) as f64 * 100.0 / total_delta as f64
                    }
                })
                .collect(),
        )
    }
}

fn parse_proc_cpu_line(line: &str) -> Option<(u64, u64)> {
    let mut values = line.split_ascii_whitespace();
    let name = values.next()?;
    if !name
        .strip_prefix("cpu")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()))
    {
        return None;
    }
    let values = values
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if values.len() < 4 {
        return None;
    }
    // Linux guest/guest_nice are already included in user/nice, so summing
    // fields past steal would double-count guest CPU time.
    let total = values
        .iter()
        .take(8)
        .copied()
        .fold(0_u64, u64::saturating_add);
    let idle = values[3].saturating_add(values.get(4).copied().unwrap_or(0));
    Some((total, idle))
}

fn finite_metric(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite())
}

fn duration_nanos(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(path: &'static [&'static str], nanos: u64) -> DebugProfilerSectionSample {
        DebugProfilerSectionSample {
            path,
            duration: Duration::from_nanos(nanos),
            record_in_report: true,
        }
    }

    #[test]
    fn chart_uses_nested_vanilla_percentages_counts_and_unspecified_slice() {
        let mut profiler = DebugProfiler::default();
        profiler.record_frame(
            Instant::now(),
            Duration::from_nanos(100),
            &[
                sample(&["tick"], 30),
                sample(&["render"], 40),
                sample(&["render"], 10),
                sample(&["publish"], 10),
                sample(&["tick", "entities"], 20),
            ],
            DebugProfilerFrameMetrics::default(),
        );

        let chart = profiler.chart().expect("profiler chart");
        assert_eq!(chart.current_node_name, ROOT_PATH);
        assert_eq!(chart.current_global_percentage, 100.0);
        assert_eq!(
            chart
                .slices
                .iter()
                .map(|slice| (
                    slice.name.as_str(),
                    slice.percentage.round() as i32,
                    slice.global_percentage.round() as i32
                ))
                .collect::<Vec<_>>(),
            vec![
                ("render", 50, 50),
                ("tick", 30, 30),
                ("unspecified", 10, 10),
                ("publish", 10, 10),
            ]
        );
        assert_eq!(
            profiler.latest.entry("root\u{001e}render"),
            DebugProfilerEntry {
                duration_nanos: 50,
                count: 2,
                max_duration_nanos: 40,
            }
        );
    }

    #[test]
    fn navigation_matches_vanilla_zero_parent_and_ignores_unspecified() {
        let mut profiler = DebugProfiler::default();
        profiler.record_frame(
            Instant::now(),
            Duration::from_nanos(100),
            &[
                sample(&["tick"], 30),
                sample(&["tick", "entities"], 20),
                sample(&["render"], 50),
            ],
            DebugProfilerFrameMetrics::default(),
        );

        profiler.navigate(1);
        assert_eq!(profiler.current_path(), "root\u{001e}render");
        profiler.navigate(1);
        assert_eq!(profiler.current_path(), "root\u{001e}render");
        profiler.navigate(0);
        assert_eq!(profiler.current_path(), ROOT_PATH);

        profiler.navigate(2);
        assert_eq!(profiler.current_path(), "root\u{001e}tick");
        let chart = profiler.chart().expect("tick chart");
        assert_eq!(chart.slices[0].name, "entities");
        profiler.navigate(1);
        assert_eq!(profiler.current_path(), "root\u{001e}tick\u{001e}entities");
    }

    #[test]
    fn stale_navigation_path_resets_and_invalid_paths_fail_closed() {
        let mut profiler = DebugProfiler::default();
        profiler.record_frame(
            Instant::now(),
            Duration::from_nanos(50),
            &[
                sample(&["render"], 50),
                sample(&[], 20),
                sample(&["bad\u{001e}path"], 20),
            ],
            DebugProfilerFrameMetrics::default(),
        );
        profiler.navigate(1);
        assert_eq!(profiler.current_path(), "root\u{001e}render");

        profiler.record_frame(
            Instant::now(),
            Duration::from_nanos(50),
            &[sample(&["update"], 50)],
            DebugProfilerFrameMetrics::default(),
        );
        assert_eq!(profiler.current_path(), ROOT_PATH);
        assert_eq!(profiler.latest.entries.len(), 2);
    }

    #[test]
    fn recording_stops_manually_or_after_vanilla_deadline_and_captures_deviation() {
        let start = Instant::now();
        let mut manual = DebugProfiler::default();
        assert!(manual.start_recording(start));
        manual.record_frame(
            start,
            Duration::from_nanos(10),
            &[sample(&["tick"], 10)],
            DebugProfilerFrameMetrics {
                tick_time_nanos: 10.0,
                ..DebugProfilerFrameMetrics::default()
            },
        );
        manual.record_frame(
            start + Duration::from_millis(50),
            Duration::from_nanos(40),
            &[sample(&["tick"], 40)],
            DebugProfilerFrameMetrics {
                tick_time_nanos: 40.0,
                ..DebugProfilerFrameMetrics::default()
            },
        );
        assert_eq!(
            manual
                .recording
                .as_ref()
                .expect("active recording")
                .deviations
                .len(),
            1
        );
        assert!(manual.stop_recording());
        manual.record_frame(
            start + Duration::from_millis(100),
            Duration::from_nanos(10),
            &[sample(&["tick"], 10)],
            DebugProfilerFrameMetrics {
                tick_time_nanos: 10.0,
                ..DebugProfilerFrameMetrics::default()
            },
        );
        assert!(!manual.is_recording());
        assert!(manual
            .take_events()
            .iter()
            .any(|event| matches!(event, DebugProfilerEvent::RecordingStopped { ticks: 3, .. })));

        let mut automatic = DebugProfiler::default();
        assert!(automatic.start_recording(start));
        automatic.record_frame(
            start + PROFILING_MAX_DURATION,
            Duration::from_nanos(10),
            &[sample(&["tick"], 10)],
            DebugProfilerFrameMetrics {
                tick_time_nanos: 10.0,
                ..DebugProfilerFrameMetrics::default()
            },
        );
        assert!(automatic.is_recording());
        automatic.record_frame(
            start + PROFILING_MAX_DURATION + Duration::from_nanos(1),
            Duration::from_nanos(10),
            &[sample(&["tick"], 10)],
            DebugProfilerFrameMetrics {
                tick_time_nanos: 10.0,
                ..DebugProfilerFrameMetrics::default()
            },
        );
        assert!(!automatic.is_recording());
    }

    #[test]
    fn profile_text_uses_vanilla_half_up_calls_per_tick_rounding() {
        let tree = ProfileTree::from_frame(
            Duration::from_nanos(100),
            &[sample(&["tick"], 40), sample(&["render"], 50)],
        );
        assert!(tree
            .profiling_text(Duration::from_nanos(100), 2)
            .contains("tick(1/1) - 40.00%/40.00%"));
    }

    #[test]
    fn parallel_simulation_sections_stay_visible_without_repeating_in_report() {
        let start = Instant::now();
        let mut profiler = DebugProfiler::default();
        assert!(profiler.start_recording(start));
        profiler.record_frame(
            start,
            Duration::from_nanos(100),
            &[sample(&["tick"], 30), sample(&["frame"], 60)],
            DebugProfilerFrameMetrics::default(),
        );
        profiler.record_frame(
            start + Duration::from_millis(10),
            Duration::from_nanos(100),
            &[
                DebugProfilerSectionSample {
                    path: &["tick"],
                    duration: Duration::from_nanos(30),
                    record_in_report: false,
                },
                sample(&["frame"], 60),
            ],
            DebugProfilerFrameMetrics::default(),
        );

        assert_eq!(profiler.latest.entry("root\u{001e}tick").duration_nanos, 30);
        let aggregate = &profiler.recording.as_ref().expect("recording").aggregate;
        assert_eq!(
            aggregate.entry("root\u{001e}tick"),
            DebugProfilerEntry {
                duration_nanos: 30,
                count: 1,
                max_duration_nanos: 30,
            }
        );
        assert_eq!(aggregate.entry("root\u{001e}frame").count, 2);
    }

    #[test]
    fn proc_cpu_parser_and_deviation_threshold_match_vanilla_shapes() {
        assert_eq!(
            parse_proc_cpu_line("cpu3 100 10 20 30 5 0 0 0 40 50"),
            Some((165, 35))
        );
        assert_eq!(parse_proc_cpu_line("cpu 100 10 20 30"), None);
        assert!(!tick_time_deviated(10.0, 29.9));
        assert!(tick_time_deviated(10.0, 30.0));
    }
}
