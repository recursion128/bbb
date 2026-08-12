use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime},
};

use chrono::{DateTime, Utc};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

use super::{finite_metric, DebugProfilerFrameMetrics, ProfileTree};

const REPORT_QUEUE_CAPACITY: usize = 2;
static REPORT_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub(super) struct RecordedMetricsFrame {
    tick: usize,
    metrics: DebugProfilerFrameMetrics,
}

impl RecordedMetricsFrame {
    pub(super) fn new(tick: usize, metrics: DebugProfilerFrameMetrics) -> Self {
        Self { tick, metrics }
    }
}

#[derive(Debug)]
pub(super) struct RecordedDeviation {
    tick: usize,
    recorded_at: Instant,
    profile: ProfileTree,
}

impl RecordedDeviation {
    pub(super) fn new(tick: usize, recorded_at: Instant, profile: ProfileTree) -> Self {
        Self {
            tick,
            recorded_at,
            profile,
        }
    }
}

#[derive(Debug)]
pub(super) struct ProfilingReport {
    identity: String,
    finished_at: SystemTime,
    duration: Duration,
    aggregate: ProfileTree,
    frames: Vec<RecordedMetricsFrame>,
    deviations: Vec<RecordedDeviation>,
}

impl ProfilingReport {
    pub(super) fn new(
        identity: String,
        finished_at: SystemTime,
        duration: Duration,
        aggregate: ProfileTree,
        frames: Vec<RecordedMetricsFrame>,
        deviations: Vec<RecordedDeviation>,
    ) -> Self {
        Self {
            identity,
            finished_at,
            duration,
            aggregate,
            frames,
            deviations,
        }
    }

    pub(super) fn duration(&self) -> Duration {
        self.duration
    }

    pub(super) fn tick_count(&self) -> usize {
        self.frames.len()
    }

    pub(super) fn ticks_per_second(&self) -> f64 {
        let seconds = self.duration.as_secs_f64();
        if seconds > 0.0 {
            self.frames.len() as f64 / seconds
        } else {
            0.0
        }
    }
}

#[derive(Debug)]
pub(super) enum ProfilingReportCompletion {
    Saved(PathBuf),
    Failed(String),
}

#[derive(Debug)]
pub(super) struct ProfilingReportWriter {
    sender: Option<SyncSender<ProfilingReport>>,
    completions: Receiver<ProfilingReportCompletion>,
    thread: Option<JoinHandle<()>>,
}

impl ProfilingReportWriter {
    pub(super) fn spawn(report_root: PathBuf) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(REPORT_QUEUE_CAPACITY);
        let (completion_sender, completions) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("bbb-profiler-writer".to_string())
            .spawn(move || {
                while let Ok(report) = receiver.recv() {
                    let completion = match write_profiling_report(&report_root, &report) {
                        Ok(path) => ProfilingReportCompletion::Saved(path),
                        Err(error) => ProfilingReportCompletion::Failed(error.to_string()),
                    };
                    if completion_sender.send(completion).is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            sender: Some(sender),
            completions,
            thread: Some(thread),
        })
    }

    pub(super) fn submit(&self, report: ProfilingReport) -> Result<(), String> {
        let Some(sender) = self.sender.as_ref() else {
            return Err("profiling report writer has shut down".to_string());
        };
        sender.try_send(report).map_err(|error| match error {
            TrySendError::Full(_) => "profiling report queue is full".to_string(),
            TrySendError::Disconnected(_) => "profiling report writer disconnected".to_string(),
        })
    }

    pub(super) fn take_completions(&mut self) -> Vec<ProfilingReportCompletion> {
        let mut output = Vec::new();
        loop {
            match self.completions.try_recv() {
                Ok(completion) => output.push(completion),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        output
    }
}

impl Drop for ProfilingReportWriter {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                tracing::warn!("profiling report writer panicked during shutdown");
            }
        }
    }
}

impl DebugProfilerFrameMetrics {
    fn categories(&self) -> BTreeMap<String, BTreeMap<String, f64>> {
        let mut categories = BTreeMap::new();
        insert_metric(&mut categories, "ticking", "ticktime", self.tick_time_nanos);
        if let Some(memory) = finite_metric(self.process_memory_mib) {
            insert_metric(&mut categories, "jvm", "process resident MiB", memory);
        }
        for (index, load) in self.cpu_load_percent.iter().copied().enumerate() {
            if load.is_finite() {
                insert_metric(&mut categories, "cpu", &format!("cpu#{index}"), load);
            }
        }
        for (name, value) in [
            ("totalChunks", self.total_chunks),
            ("renderedChunks", self.rendered_chunks),
            ("lastViewDistance", self.last_view_distance),
        ] {
            if let Some(value) = finite_metric(value) {
                insert_metric(&mut categories, "chunk rendering", name, value);
            }
        }
        if let Some(value) = finite_metric(self.compile_queue_size) {
            insert_metric(
                &mut categories,
                "chunk rendering dispatching",
                "compileQueueSize",
                value,
            );
        }
        if let Some(value) = finite_metric(self.gpu_utilization) {
            insert_metric(&mut categories, "gpu", "gpuUtilization", value);
        }
        if let Some(value) = finite_metric(self.gpu_duration_nanos) {
            insert_metric(&mut categories, "gpu", "gpuDuration", value);
        }
        if let Some(stages) = &self.gpu_stages {
            for (name, value) in [
                (
                    "application_frame_interval",
                    stages.application_frame_interval,
                ),
                ("total_gpu_duration", stages.total_gpu_duration),
                ("terrain_atlas_animation", stages.terrain_atlas_animation),
                ("opaque_world", stages.opaque_world),
                ("opaque_particles", stages.opaque_particles),
                ("feature_depth_copies", stages.feature_depth_copies),
                (
                    "entity_translucent_features",
                    stages.entity_translucent_features,
                ),
                (
                    "remaining_feature_targets",
                    stages.remaining_feature_targets,
                ),
                ("clouds", stages.clouds),
                ("weather", stages.weather),
                ("transparency_combine", stages.transparency_combine),
                ("presentation_tail", stages.presentation_tail),
                ("encoder_tail", stages.encoder_tail),
            ] {
                insert_metric(&mut categories, "gpu stages", name, value);
            }
        }
        if let Some(phases) = &self.frame_phases {
            for (name, value) in [
                (
                    "application_frame_interval",
                    phases.application_frame_interval,
                ),
                ("cpu_encode_before_wait", phases.cpu_encode_before_wait),
                ("previous_submission_wait", phases.previous_submission_wait),
                ("previous_submission_poll", phases.previous_submission_poll),
                (
                    "previous_submission_blocking_wait",
                    phases.previous_submission_blocking_wait,
                ),
                ("surface_acquire", phases.surface_acquire),
                ("presentation_tail_encode", phases.presentation_tail_encode),
                ("queue_submit", phases.queue_submit),
                ("swap_buffers_present", phases.swap_buffers_present),
                (
                    "post_present_fence_submit",
                    phases.post_present_fence_submit,
                ),
                ("gpu_async_poll", phases.gpu_async_poll),
                ("render_call_total", phases.render_call_total),
            ] {
                insert_metric(&mut categories, "frame phases", name, value);
            }
        }
        if let Some(flight) = &self.frame_flight {
            for (name, value) in [
                (
                    "render_to_callback_observed",
                    flight.render_to_callback_observed,
                ),
                (
                    "submit_to_callback_observed",
                    flight.submit_to_callback_observed,
                ),
                ("callback_to_wait_return", flight.callback_to_wait_return),
                (
                    "in_flight_depth_at_submit",
                    flight.in_flight_depth_at_submit,
                ),
                (
                    "completion_observation_expected",
                    flight.completion_observation_expected,
                ),
                (
                    "completion_observation_missing",
                    flight.completion_observation_missing,
                ),
                (
                    "completion_observations_dropped",
                    flight.completion_observations_dropped,
                ),
            ] {
                insert_metric(&mut categories, "frame flight", name, value);
            }
        }
        if let Some(latency) = &self.input_latency {
            for (name, value) in [
                ("sequence", latency.sequence),
                (
                    "enqueue_to_simulation_apply",
                    latency.enqueue_to_simulation_apply,
                ),
                (
                    "simulation_apply_to_renderer_extract",
                    latency.simulation_apply_to_renderer_extract,
                ),
                (
                    "renderer_extract_to_queue_submit",
                    latency.renderer_extract_to_queue_submit,
                ),
                (
                    "queue_submit_to_present_call_return",
                    latency.queue_submit_to_present_call_return,
                ),
                (
                    "enqueue_to_present_call_return",
                    latency.enqueue_to_present_call_return,
                ),
                ("superseded_events", latency.superseded_events),
                ("out_of_order_events", latency.out_of_order_events),
                (
                    "present_call_return_software_boundary",
                    latency.present_call_return_software_boundary,
                ),
            ] {
                insert_metric(&mut categories, "input latency", name, value);
            }
        }
        categories
    }
}

fn write_profiling_report(report_root: &Path, report: &ProfilingReport) -> io::Result<PathBuf> {
    fs::create_dir_all(report_root)?;
    let timestamp: DateTime<Utc> = report.finished_at.into();
    let stem = format!(
        "{}-{}-{}",
        timestamp.format("%Y-%m-%d_%H.%M.%S"),
        sanitize_file_name(&report.identity),
        bbb_protocol::PROTOCOL_VERSION
    );
    let temporary_dir = create_report_temp_dir(report_root)?;
    let temporary_path = temporary_dir.join("profiling.zip");
    let result = write_profiling_zip(&temporary_path, report)
        .and_then(|()| install_archive_without_clobber(&temporary_path, report_root, &stem));
    let _ = fs::remove_file(&temporary_path);
    let _ = fs::remove_dir(&temporary_dir);
    result
}

fn write_profiling_zip(path: &Path, report: &ProfilingReport) -> io::Result<()> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    write_zip_text(
        &mut zip,
        "system.txt",
        &format!(
            "bbb-native: {}\nMinecraft: {}\nProtocol: {}\nOS: {} {}\nCPU threads: {}\n",
            env!("CARGO_PKG_VERSION"),
            bbb_protocol::MC_VERSION,
            bbb_protocol::PROTOCOL_VERSION,
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::thread::available_parallelism()
                .map(|threads| threads.get())
                .unwrap_or(1)
        ),
        options,
    )?;
    write_zip_text(
        &mut zip,
        "client/profiling.txt",
        &report
            .aggregate
            .profiling_text(report.duration, report.frames.len()),
        options,
    )?;

    for (category, csv) in metrics_csv_files(&report.frames) {
        write_zip_text(
            &mut zip,
            &format!("client/metrics/{}.csv", sanitize_file_name(&category)),
            &csv,
            options,
        )?;
    }
    for deviation in &report.deviations {
        let offset = deviation
            .recorded_at
            .saturating_duration_since(
                report
                    .deviations
                    .first()
                    .map_or(deviation.recorded_at, |first| first.recorded_at),
            )
            .as_millis();
        write_zip_text(
            &mut zip,
            &format!(
                "client/deviations/ticktime/{}@{:06}.txt",
                deviation.tick, offset
            ),
            &deviation
                .profile
                .profiling_text(deviation.profile.duration(), 1),
            options,
        )?;
    }
    zip.finish()?;
    Ok(())
}

fn write_zip_text(
    zip: &mut ZipWriter<std::fs::File>,
    path: &str,
    text: &str,
    options: SimpleFileOptions,
) -> io::Result<()> {
    zip.start_file(path, options)?;
    zip.write_all(text.as_bytes())
}

fn metrics_csv_files(frames: &[RecordedMetricsFrame]) -> BTreeMap<String, String> {
    let mut columns_by_category = BTreeMap::<String, Vec<String>>::new();
    let categorized = frames
        .iter()
        .map(|frame| (frame.tick, frame.metrics.categories()))
        .collect::<Vec<_>>();
    for (_, categories) in &categorized {
        for (category, values) in categories {
            let columns = columns_by_category.entry(category.clone()).or_default();
            for name in values.keys() {
                if !columns.contains(name) {
                    columns.push(name.clone());
                }
            }
        }
    }
    for columns in columns_by_category.values_mut() {
        columns.sort();
    }

    columns_by_category
        .into_iter()
        .map(|(category, columns)| {
            let mut csv = String::from("@tick");
            for column in &columns {
                csv.push(',');
                csv.push_str(&csv_field(column));
            }
            csv.push('\n');
            for (tick, categories) in &categorized {
                csv.push_str(&tick.to_string());
                let values = categories.get(&category);
                for column in &columns {
                    csv.push(',');
                    if let Some(value) = values.and_then(|values| values.get(column)) {
                        csv.push_str(&value.to_string());
                    } else {
                        csv.push('0');
                    }
                }
                csv.push('\n');
            }
            (category, csv)
        })
        .collect()
}

fn create_report_temp_dir(report_root: &Path) -> io::Result<PathBuf> {
    let parent = report_root.parent().unwrap_or_else(|| Path::new("."));
    for _ in 0..=10_000_u32 {
        let sequence = REPORT_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(
            ".bbb-profiling-tmp-{}-{sequence}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "profiling temporary directory collision limit reached",
    ))
}

fn install_archive_without_clobber(
    temporary_path: &Path,
    root: &Path,
    stem: &str,
) -> io::Result<PathBuf> {
    for suffix in 0..=10_000_u32 {
        let file_name = if suffix == 0 {
            format!("{stem}.zip")
        } else {
            format!("{stem}_{suffix}.zip")
        };
        let path = root.join(file_name);
        match fs::hard_link(temporary_path, &path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "profiling archive collision limit reached",
    ))
}

fn insert_metric(
    categories: &mut BTreeMap<String, BTreeMap<String, f64>>,
    category: &str,
    name: &str,
    value: f64,
) {
    categories
        .entry(category.to_string())
        .or_default()
        .insert(name.to_string(), value);
}

fn sanitize_file_name(value: &str) -> String {
    let mut sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    while sanitized.contains("__") {
        sanitized = sanitized.replace("__", "_");
    }
    let sanitized = sanitized.trim_matches(['.', '_', '-']);
    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized.chars().take(128).collect()
    }
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::Read,
        sync::atomic::{AtomicU64, Ordering},
    };

    use zip::ZipArchive;

    use super::*;
    use crate::debug_profiler::{
        DebugProfilerFrameFlight, DebugProfilerFramePhases, DebugProfilerGpuStages,
        DebugProfilerSectionSample,
    };

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    fn sample(path: &'static [&'static str], nanos: u64) -> DebugProfilerSectionSample {
        DebugProfilerSectionSample {
            path,
            duration: Duration::from_nanos(nanos),
            record_in_report: true,
        }
    }

    fn test_report(finished_at: SystemTime) -> ProfilingReport {
        let frames = vec![
            RecordedMetricsFrame::new(
                1,
                DebugProfilerFrameMetrics {
                    tick_time_nanos: 10.0,
                    process_memory_mib: Some(25.0),
                    total_chunks: Some(8.0),
                    rendered_chunks: Some(5.0),
                    last_view_distance: Some(12.0),
                    compile_queue_size: Some(2.0),
                    frame_phases: Some(DebugProfilerFramePhases {
                        application_frame_interval: 100.0,
                        cpu_encode_before_wait: 50.0,
                        previous_submission_wait: 20.0,
                        previous_submission_poll: 8.0,
                        previous_submission_blocking_wait: 12.0,
                        surface_acquire: 3.0,
                        presentation_tail_encode: 4.0,
                        queue_submit: 5.0,
                        swap_buffers_present: 6.0,
                        post_present_fence_submit: 9.0,
                        gpu_async_poll: 7.0,
                        render_call_total: 95.0,
                    }),
                    frame_flight: Some(DebugProfilerFrameFlight {
                        render_to_callback_observed: 80.0,
                        submit_to_callback_observed: 30.0,
                        callback_to_wait_return: 2.0,
                        in_flight_depth_at_submit: 1.0,
                        completion_observation_expected: 1.0,
                        completion_observation_missing: 0.0,
                        completion_observations_dropped: 0.0,
                    }),
                    input_latency: Some(crate::debug_profiler::DebugProfilerInputLatency {
                        sequence: 17.0,
                        enqueue_to_simulation_apply: 12.0,
                        simulation_apply_to_renderer_extract: 13.0,
                        renderer_extract_to_queue_submit: 14.0,
                        queue_submit_to_present_call_return: 15.0,
                        enqueue_to_present_call_return: 54.0,
                        superseded_events: 2.0,
                        out_of_order_events: 0.0,
                        present_call_return_software_boundary: 1.0,
                    }),
                    gpu_stages: Some(DebugProfilerGpuStages {
                        application_frame_interval: 100.0,
                        total_gpu_duration: 66.0,
                        terrain_atlas_animation: 1.0,
                        opaque_world: 2.0,
                        opaque_particles: 3.0,
                        feature_depth_copies: 4.0,
                        entity_translucent_features: 5.0,
                        remaining_feature_targets: 6.0,
                        clouds: 7.0,
                        weather: 8.0,
                        transparency_combine: 9.0,
                        presentation_tail: 10.0,
                        encoder_tail: 11.0,
                    }),
                    ..DebugProfilerFrameMetrics::default()
                },
            ),
            RecordedMetricsFrame::new(
                2,
                DebugProfilerFrameMetrics {
                    tick_time_nanos: 40.0,
                    process_memory_mib: Some(26.0),
                    total_chunks: Some(9.0),
                    rendered_chunks: Some(6.0),
                    last_view_distance: Some(12.0),
                    compile_queue_size: Some(1.0),
                    gpu_utilization: Some(50.0),
                    gpu_duration_nanos: Some(20.0),
                    ..DebugProfilerFrameMetrics::default()
                },
            ),
        ];
        let frame_profile = ProfileTree::from_frame(
            Duration::from_nanos(100),
            &[
                sample(&["tick"], 40),
                sample(&["tick", "entities"], 25),
                sample(&["render"], 50),
            ],
        );
        let mut aggregate = frame_profile.clone();
        aggregate.merge(&frame_profile);
        ProfilingReport::new(
            "example.invalid:25565".to_string(),
            finished_at,
            Duration::from_secs(1),
            aggregate.clone(),
            frames,
            vec![RecordedDeviation::new(2, Instant::now(), aggregate)],
        )
    }

    fn temp_dir(name: &str) -> PathBuf {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bbb-profiler-test-{name}-{}-{sequence}",
            std::process::id()
        ))
    }

    #[test]
    fn metrics_csv_groups_union_columns_and_fills_absent_values_with_zero() {
        let report = test_report(SystemTime::UNIX_EPOCH);
        let files = metrics_csv_files(&report.frames);
        assert_eq!(files["ticking"], "@tick,ticktime\n1,10\n2,40\n");
        assert_eq!(
            files["gpu"],
            "@tick,gpuDuration,gpuUtilization\n1,0,0\n2,20,50\n"
        );
        assert!(files["chunk rendering"]
            .contains("@tick,lastViewDistance,renderedChunks,totalChunks\n1,12,5,8\n2,12,6,9\n"));
        assert!(files["frame phases"].contains(
            "@tick,application_frame_interval,cpu_encode_before_wait,gpu_async_poll,post_present_fence_submit,presentation_tail_encode,previous_submission_blocking_wait,previous_submission_poll,previous_submission_wait,queue_submit,render_call_total,surface_acquire,swap_buffers_present\n1,100,50,7,9,4,12,8,20,5,95,3,6\n"
        ));
        assert!(files["frame flight"].contains(
            "@tick,callback_to_wait_return,completion_observation_expected,completion_observation_missing,completion_observations_dropped,in_flight_depth_at_submit,render_to_callback_observed,submit_to_callback_observed\n1,2,1,0,0,1,80,30\n"
        ));
        assert!(files["gpu stages"].contains(
            "@tick,application_frame_interval,clouds,encoder_tail,entity_translucent_features,feature_depth_copies,opaque_particles,opaque_world,presentation_tail,remaining_feature_targets,terrain_atlas_animation,total_gpu_duration,transparency_combine,weather\n1,100,7,11,5,4,3,2,10,6,1,66,9,8\n"
        ));
        assert!(files["input latency"].contains(
            "@tick,enqueue_to_present_call_return,enqueue_to_simulation_apply,out_of_order_events,present_call_return_software_boundary,queue_submit_to_present_call_return,renderer_extract_to_queue_submit,sequence,simulation_apply_to_renderer_extract,superseded_events\n1,54,12,0,1,15,14,17,13,2\n"
        ));
    }

    #[test]
    fn report_writer_creates_real_collision_safe_zip_with_profile_metrics_and_deviation() {
        let root = temp_dir("zip");
        let report = test_report(SystemTime::UNIX_EPOCH);
        let first = write_profiling_report(&root, &report).expect("first report");
        let second = write_profiling_report(&root, &report).expect("collision report");
        assert_ne!(first, second);
        assert_eq!(
            first.extension().and_then(|value| value.to_str()),
            Some("zip")
        );

        let file = std::fs::File::open(first).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        for name in [
            "system.txt",
            "client/profiling.txt",
            "client/metrics/ticking.csv",
            "client/metrics/chunk_rendering.csv",
            "client/metrics/frame_phases.csv",
            "client/metrics/frame_flight.csv",
            "client/metrics/gpu_stages.csv",
            "client/metrics/input_latency.csv",
            "client/deviations/ticktime/2@000000.txt",
        ] {
            assert!(archive.by_name(name).is_ok(), "missing {name}");
        }
        let mut profile = String::new();
        archive
            .by_name("client/profiling.txt")
            .unwrap()
            .read_to_string(&mut profile)
            .unwrap();
        assert!(profile.contains("tick(2/1) - 40.00%/40.00%"));
        assert!(profile.contains("entities(2/1) - 62.50%/25.00%"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn report_writer_surfaces_output_root_failure_without_leaving_temporary_file() {
        let root = temp_dir("failure");
        fs::write(&root, b"not a directory").unwrap();
        let error = write_profiling_report(&root, &test_report(SystemTime::UNIX_EPOCH))
            .expect_err("file root must fail");
        assert!(matches!(
            error.kind(),
            io::ErrorKind::AlreadyExists | io::ErrorKind::NotADirectory
        ));
        fs::remove_file(root).unwrap();
    }
}
