//! Bounded fatal-runtime diagnostics and immutable crash-report publication.
//!
//! Vanilla's `CrashReport` owns report formatting and save-once publication,
//! while `Minecraft` relays fatal failures to the main thread before shutdown.
//! BBB keeps the same semantic split: producers create `RuntimeFailure` values;
//! only `CrashReportRuntime` renders and publishes them after runtime teardown.

use std::{
    any::Any,
    backtrace::Backtrace,
    error::Error,
    fmt::{self, Write as _},
    fs::{self, File, OpenOptions},
    io::{self, Write as _},
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use anyhow::Result;
use chrono::{DateTime, Utc};

const MAX_FAILURE_PHASE_BYTES: usize = 256;
const MAX_FAILURE_TITLE_BYTES: usize = 4 * 1024;
const MAX_FAILURE_MESSAGE_BYTES: usize = 64 * 1024;
const MAX_FAILURE_THREAD_BYTES: usize = 1024;
const MAX_FAILURE_BACKTRACE_BYTES: usize = 256 * 1024;
const MAX_SUPPRESSED_FAILURES: usize = 8;
const MAX_CATEGORIES: usize = 32;
const MAX_CATEGORY_ENTRIES: usize = 256;
const MAX_CATEGORY_TITLE_BYTES: usize = 1024;
const MAX_CATEGORY_KEY_BYTES: usize = 1024;
const MAX_CATEGORY_VALUE_BYTES: usize = 16 * 1024;
const MAX_SYSTEM_ENTRIES: usize = 64;
const MAX_RETAINED_REPORT_BYTES: usize = 512 * 1024;
const MAX_RENDERED_REPORT_BYTES: usize = 1024 * 1024;
const EMERGENCY_RESERVE_BYTES: usize = 64 * 1024;
const MAX_TEMP_FILE_ATTEMPTS: usize = 128;
const MAX_FINAL_FILE_COLLISIONS: usize = 1024;
const TRUNCATION_MARKER: &str = "\n...[truncated by BBB crash-report budget]";

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeFailure {
    phase: String,
    title: String,
    message: String,
    thread: String,
    backtrace: String,
}

impl RuntimeFailure {
    pub(crate) fn new(
        phase: impl fmt::Display,
        title: impl fmt::Display,
        message: impl fmt::Display,
    ) -> Self {
        Self {
            phase: format_bounded(format_args!("{phase}"), MAX_FAILURE_PHASE_BYTES).0,
            title: format_bounded(format_args!("{title}"), MAX_FAILURE_TITLE_BYTES).0,
            message: format_bounded(format_args!("{message}"), MAX_FAILURE_MESSAGE_BYTES).0,
            thread: format_bounded(
                format_args!("{}", std::thread::current().name().unwrap_or("unnamed")),
                MAX_FAILURE_THREAD_BYTES,
            )
            .0,
            backtrace: format_bounded(
                format_args!("{}", Backtrace::force_capture()),
                MAX_FAILURE_BACKTRACE_BYTES,
            )
            .0,
        }
    }

    fn from_panic(phase: &'static str, payload: &(dyn Any + Send)) -> Self {
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic payload");
        Self::new(phase, "Unexpected panic", message)
    }

    #[cfg(test)]
    fn retained_bytes(&self) -> usize {
        self.phase.len()
            + self.title.len()
            + self.message.len()
            + self.thread.len()
            + self.backtrace.len()
    }
}

impl fmt::Display for RuntimeFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {}: {}",
            self.phase, self.title, self.message
        )
    }
}

impl Error for RuntimeFailure {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeFailureSet {
    primary: RuntimeFailure,
    suppressed: Vec<RuntimeFailure>,
    dropped_suppressed: usize,
}

impl RuntimeFailureSet {
    fn new(primary: RuntimeFailure) -> Self {
        Self {
            primary,
            suppressed: Vec::new(),
            dropped_suppressed: 0,
        }
    }

    fn record(&mut self, failure: RuntimeFailure) {
        if self.suppressed.len() < MAX_SUPPRESSED_FAILURES {
            self.suppressed.push(failure);
        } else {
            self.dropped_suppressed = self.dropped_suppressed.saturating_add(1);
        }
    }
}

impl fmt::Display for RuntimeFailureSet {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.primary.fmt(formatter)
    }
}

impl Error for RuntimeFailureSet {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.primary)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RuntimeFailureAccumulator {
    failures: Option<RuntimeFailureSet>,
}

impl RuntimeFailureAccumulator {
    /// Records one failure. The first failure remains canonical; later
    /// failures are bounded suppressed diagnostics.
    pub(crate) fn record(&mut self, failure: RuntimeFailure) -> bool {
        if let Some(failures) = &mut self.failures {
            failures.record(failure);
            false
        } else {
            self.failures = Some(RuntimeFailureSet::new(failure));
            true
        }
    }

    pub(crate) fn take(&mut self) -> Option<RuntimeFailureSet> {
        self.failures.take()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CrashReportCategory {
    title: String,
    entries: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CrashSaveResult {
    Saved(PathBuf),
    AlreadySaved(PathBuf),
}

#[derive(Debug)]
struct CrashReport {
    failures: RuntimeFailureSet,
    categories: Vec<CrashReportCategory>,
    system_details: Vec<(String, String)>,
    retained_budget: RetainedBudget,
    saved_file: Option<PathBuf>,
}

impl CrashReport {
    fn new(failures: RuntimeFailureSet, uptime: Duration) -> Self {
        let mut retained_budget = RetainedBudget::new();
        let failures = retain_failure_set(&mut retained_budget, failures);
        let mut report = Self {
            failures,
            categories: Vec::new(),
            system_details: Vec::new(),
            retained_budget,
            saved_file: None,
        };
        let uptime_category = report.add_category("Uptime");
        report.add_category_detail(
            uptime_category,
            "Process uptime",
            format_args!("{:.3}s", uptime.as_secs_f64()),
        );
        report.add_system_detail("Client", "bbb-native");
        report.add_system_detail("Client version", env!("CARGO_PKG_VERSION"));
        report.add_system_detail("Minecraft version", bbb_protocol::MC_VERSION);
        report.add_system_detail("Protocol version", bbb_protocol::PROTOCOL_VERSION);
        report.add_system_detail("Operating system", std::env::consts::OS);
        report.add_system_detail("Architecture", std::env::consts::ARCH);
        report
    }

    fn add_category(&mut self, title: impl fmt::Display) -> Option<usize> {
        if self.categories.len() >= MAX_CATEGORIES {
            self.retained_budget.dropped_entries =
                self.retained_budget.dropped_entries.saturating_add(1);
            return None;
        }
        let title = self
            .retained_budget
            .retain(format_args!("{title}"), MAX_CATEGORY_TITLE_BYTES);
        let index = self.categories.len();
        self.categories.push(CrashReportCategory {
            title,
            entries: Vec::new(),
        });
        Some(index)
    }

    fn add_category_detail(
        &mut self,
        category: Option<usize>,
        key: impl fmt::Display,
        value: fmt::Arguments<'_>,
    ) {
        let Some(category) = category else {
            return;
        };
        let entry_count: usize = self
            .categories
            .iter()
            .map(|entry| entry.entries.len())
            .sum();
        if entry_count >= MAX_CATEGORY_ENTRIES {
            self.retained_budget.dropped_entries =
                self.retained_budget.dropped_entries.saturating_add(1);
            return;
        }
        let key = self
            .retained_budget
            .retain(format_args!("{key}"), MAX_CATEGORY_KEY_BYTES);
        let value = self.retained_budget.retain(value, MAX_CATEGORY_VALUE_BYTES);
        self.categories[category].entries.push((key, value));
    }

    fn add_system_detail(&mut self, key: impl fmt::Display, value: impl fmt::Display) {
        if self.system_details.len() >= MAX_SYSTEM_ENTRIES {
            self.retained_budget.dropped_entries =
                self.retained_budget.dropped_entries.saturating_add(1);
            return;
        }
        let key = self
            .retained_budget
            .retain(format_args!("{key}"), MAX_CATEGORY_KEY_BYTES);
        let value = self
            .retained_budget
            .retain(format_args!("{value}"), MAX_CATEGORY_VALUE_BYTES);
        self.system_details.push((key, value));
    }

    fn friendly_report_at(&self, now: DateTime<Utc>) -> String {
        let mut output = BoundedWriter::new(MAX_RENDERED_REPORT_BYTES);
        let _ = writeln!(output, "---- Minecraft Crash Report ----");
        let _ = writeln!(output, "// BBB native client crash diagnostics\n");
        let _ = writeln!(output, "Time: {}", now.format("%Y-%m-%d %H:%M:%S UTC"));
        let _ = writeln!(output, "Description: {}\n", self.failures.primary.title);
        let _ = writeln!(output, "{}", self.failures.primary.message);
        let _ = writeln!(
            output,
            "\nA detailed walkthrough of the error, its code path and all known details is as follows:"
        );
        let _ = writeln!(output, "{}\n", "-".repeat(87));
        let _ = writeln!(output, "-- Head --");
        let _ = writeln!(output, "Thread: {}", self.failures.primary.thread);
        let _ = writeln!(output, "Phase: {}", self.failures.primary.phase);
        let _ = writeln!(output, "Stacktrace:\n{}\n", self.failures.primary.backtrace);

        for category in &self.categories {
            let _ = writeln!(output, "-- {} --", category.title);
            let _ = writeln!(output, "Details:");
            for (key, value) in &category.entries {
                let _ = writeln!(output, "\t{key}: {value}");
            }
            let _ = writeln!(output);
        }

        if !self.failures.suppressed.is_empty() || self.failures.dropped_suppressed > 0 {
            let _ = writeln!(output, "-- Suppressed failures --");
            for (index, failure) in self.failures.suppressed.iter().enumerate() {
                let _ = writeln!(
                    output,
                    "\t{}: [{} / {}] {}",
                    index + 1,
                    failure.phase,
                    failure.thread,
                    failure.message
                );
            }
            if self.failures.dropped_suppressed > 0 {
                let _ = writeln!(
                    output,
                    "\tDropped by suppressed-failure budget: {}",
                    self.failures.dropped_suppressed
                );
            }
            let _ = writeln!(output);
        }

        let _ = writeln!(output, "-- System Details --");
        let _ = writeln!(output, "Details:");
        for (key, value) in &self.system_details {
            let _ = writeln!(output, "\t{key}: {value}");
        }
        let _ = writeln!(
            output,
            "\tRetained report bytes: {} / {}",
            self.retained_budget.retained_bytes, MAX_RETAINED_REPORT_BYTES
        );
        let _ = writeln!(
            output,
            "\tTruncated fields: {}",
            self.retained_budget.truncated_fields
        );
        let _ = writeln!(
            output,
            "\tDropped entries: {}",
            self.retained_budget.dropped_entries
        );
        output.finish()
    }

    fn save_to_directory_at(
        &mut self,
        report_dir: &Path,
        now: DateTime<Utc>,
    ) -> io::Result<CrashSaveResult> {
        if let Some(path) = &self.saved_file {
            return Ok(CrashSaveResult::AlreadySaved(path.clone()));
        }
        fs::create_dir_all(report_dir)?;
        let report = self.friendly_report_at(now);
        let (temporary_path, mut temporary_file) = create_temporary_report(report_dir)?;
        let write_result = temporary_file
            .write_all(report.as_bytes())
            .and_then(|()| temporary_file.sync_all());
        drop(temporary_file);
        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary_path);
            return Err(error);
        }
        let write_result = install_report_without_clobber(&temporary_path, report_dir, now);
        let _ = fs::remove_file(&temporary_path);
        let final_path = write_result?;
        self.saved_file = Some(final_path.clone());
        Ok(CrashSaveResult::Saved(final_path))
    }

    #[cfg(test)]
    fn retained_bytes(&self) -> usize {
        self.retained_budget.retained_bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CrashDisposition {
    exit_code: i32,
    saved_path: Option<PathBuf>,
}

#[derive(Debug)]
pub(crate) struct CrashReportRuntime {
    report_dir: PathBuf,
    started_at: Instant,
    emergency_reserve: Vec<u8>,
}

impl CrashReportRuntime {
    pub(crate) fn new(report_dir: PathBuf) -> Self {
        Self {
            report_dir,
            started_at: Instant::now(),
            emergency_reserve: vec![0; EMERGENCY_RESERVE_BYTES],
        }
    }

    pub(crate) fn guard(
        &self,
        run: impl FnOnce() -> Result<()>,
    ) -> std::result::Result<(), RuntimeFailureSet> {
        match panic::catch_unwind(AssertUnwindSafe(run)) {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                if let Some(failures) = error.downcast_ref::<RuntimeFailureSet>() {
                    Err(failures.clone())
                } else if let Some(failure) = error.downcast_ref::<RuntimeFailure>() {
                    Err(RuntimeFailureSet::new(failure.clone()))
                } else {
                    Err(RuntimeFailureSet::new(RuntimeFailure::new(
                        "startup/runtime",
                        "Unexpected error",
                        format_args!("{error:#}"),
                    )))
                }
            }
            Err(payload) => Err(RuntimeFailureSet::new(RuntimeFailure::from_panic(
                "main thread",
                payload.as_ref(),
            ))),
        }
    }

    pub(crate) fn finish_failure(&mut self, failures: RuntimeFailureSet) -> CrashDisposition {
        self.emergency_reserve = Vec::new();
        let now = Utc::now();
        let mut report = CrashReport::new(failures, self.started_at.elapsed());
        eprintln!("{}", report.friendly_report_at(now));
        match report.save_to_directory_at(&self.report_dir, now) {
            Ok(CrashSaveResult::Saved(path)) | Ok(CrashSaveResult::AlreadySaved(path)) => {
                eprintln!("BBB crashed; crash report saved to {}", path.display());
                CrashDisposition {
                    exit_code: 1,
                    saved_path: Some(path),
                }
            }
            Err(error) => {
                eprintln!(
                    "BBB crashed; crash report could not be saved to {}: {}",
                    self.report_dir.display(),
                    format_bounded(format_args!("{error}"), MAX_FAILURE_MESSAGE_BYTES).0
                );
                CrashDisposition {
                    exit_code: 2,
                    saved_path: None,
                }
            }
        }
    }
}

pub(crate) fn default_crash_report_dir() -> PathBuf {
    PathBuf::from("crash-reports")
}

pub(crate) fn exit_after_failure(disposition: CrashDisposition) -> ! {
    process::exit(disposition.exit_code)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RetainedBudget {
    retained_bytes: usize,
    truncated_fields: usize,
    dropped_entries: usize,
}

impl RetainedBudget {
    fn new() -> Self {
        Self {
            retained_bytes: 0,
            truncated_fields: 0,
            dropped_entries: 0,
        }
    }

    fn retain(&mut self, value: fmt::Arguments<'_>, field_limit: usize) -> String {
        let remaining = MAX_RETAINED_REPORT_BYTES.saturating_sub(self.retained_bytes);
        let (value, truncated) = format_bounded(value, field_limit.min(remaining));
        self.retained_bytes = self.retained_bytes.saturating_add(value.len());
        if truncated {
            self.truncated_fields = self.truncated_fields.saturating_add(1);
        }
        value
    }
}

fn retain_failure_set(
    budget: &mut RetainedBudget,
    failures: RuntimeFailureSet,
) -> RuntimeFailureSet {
    let primary = retain_failure(budget, failures.primary);
    let suppressed = failures
        .suppressed
        .into_iter()
        .map(|failure| retain_failure(budget, failure))
        .collect();
    RuntimeFailureSet {
        primary,
        suppressed,
        dropped_suppressed: failures.dropped_suppressed,
    }
}

fn retain_failure(budget: &mut RetainedBudget, failure: RuntimeFailure) -> RuntimeFailure {
    RuntimeFailure {
        phase: budget.retain(format_args!("{}", failure.phase), MAX_FAILURE_PHASE_BYTES),
        title: budget.retain(format_args!("{}", failure.title), MAX_FAILURE_TITLE_BYTES),
        message: budget.retain(
            format_args!("{}", failure.message),
            MAX_FAILURE_MESSAGE_BYTES,
        ),
        thread: budget.retain(format_args!("{}", failure.thread), MAX_FAILURE_THREAD_BYTES),
        backtrace: budget.retain(
            format_args!("{}", failure.backtrace),
            MAX_FAILURE_BACKTRACE_BYTES,
        ),
    }
}

fn format_bounded(value: fmt::Arguments<'_>, limit: usize) -> (String, bool) {
    let mut output = BoundedWriter::new(limit);
    let _ = output.write_fmt(value);
    let truncated = output.truncated;
    (output.finish(), truncated)
}

#[derive(Debug)]
struct BoundedWriter {
    output: String,
    content_limit: usize,
    total_limit: usize,
    truncated: bool,
}

impl BoundedWriter {
    fn new(total_limit: usize) -> Self {
        Self {
            output: String::with_capacity(total_limit.min(16 * 1024)),
            content_limit: total_limit.saturating_sub(TRUNCATION_MARKER.len()),
            total_limit,
            truncated: false,
        }
    }

    fn finish(mut self) -> String {
        if self.truncated {
            let marker_limit = self.total_limit.saturating_sub(self.output.len());
            let marker_end = utf8_prefix_len(TRUNCATION_MARKER, marker_limit);
            self.output.push_str(&TRUNCATION_MARKER[..marker_end]);
        }
        self.output
    }
}

impl fmt::Write for BoundedWriter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if self.output.len() >= self.content_limit {
            self.truncated |= !value.is_empty();
            return Ok(());
        }
        let remaining = self.content_limit - self.output.len();
        let end = utf8_prefix_len(value, remaining);
        self.output.push_str(&value[..end]);
        self.truncated |= end < value.len();
        Ok(())
    }
}

fn utf8_prefix_len(value: &str, limit: usize) -> usize {
    if value.len() <= limit {
        return value.len();
    }
    let mut end = limit;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    end
}

fn create_temporary_report(report_dir: &Path) -> io::Result<(PathBuf, File)> {
    for _ in 0..MAX_TEMP_FILE_ATTEMPTS {
        let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let path = report_dir.join(format!(".bbb-crash-{}-{sequence}.tmp", process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "crash-report temporary file collision limit reached",
    ))
}

fn install_report_without_clobber(
    temporary_path: &Path,
    report_dir: &Path,
    now: DateTime<Utc>,
) -> io::Result<PathBuf> {
    let timestamp = now.format("%Y-%m-%d_%H.%M.%S");
    for collision in 0..MAX_FINAL_FILE_COLLISIONS {
        let suffix = if collision == 0 {
            String::new()
        } else {
            format!("-{collision}")
        };
        let path = report_dir.join(format!("crash-{timestamp}-client{suffix}.txt"));
        match fs::hard_link(temporary_path, &path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "crash-report final file collision limit reached",
    ))
}

#[cfg(test)]
mod tests {
    use std::{fs, panic::panic_any};

    use chrono::TimeZone;

    use super::*;

    fn failure(phase: &str, title: &str, message: impl fmt::Display) -> RuntimeFailure {
        RuntimeFailure::new(phase, title, message)
    }

    fn fixed_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 2, 12, 34, 56)
            .single()
            .unwrap()
    }

    fn test_directory(name: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "bbb-crash-report-test-{}-{sequence}-{name}",
            process::id()
        ))
    }

    #[test]
    fn bounded_formatter_keeps_utf8_and_marks_truncation() {
        let (formatted, truncated) = format_bounded(format_args!("中日한{}", "x".repeat(80)), 48);
        assert!(truncated);
        assert!(formatted.len() <= 48);
        assert!(formatted.is_char_boundary(formatted.len()));
        assert!(formatted.contains("truncated"));
    }

    #[test]
    fn first_failure_wins_and_suppressed_failures_are_bounded() {
        let mut failures = RuntimeFailureAccumulator::default();
        assert!(failures.record(failure("simulation", "first", "root")));
        for index in 0..(MAX_SUPPRESSED_FAILURES + 3) {
            assert!(!failures.record(failure("shutdown", "later", index)));
        }
        let failures = failures.take().unwrap();
        assert_eq!(failures.primary.title, "first");
        assert_eq!(failures.suppressed.len(), MAX_SUPPRESSED_FAILURES);
        assert_eq!(failures.dropped_suppressed, 3);
    }

    #[test]
    fn friendly_report_has_vanilla_sections_and_bounded_accounting() {
        let mut failures = RuntimeFailureSet::new(failure(
            "renderer",
            "Render failure",
            "a detailed renderer error",
        ));
        failures.record(failure("shutdown", "Join failure", "worker stopped"));
        let report = CrashReport::new(failures, Duration::from_millis(1250));
        let rendered = report.friendly_report_at(fixed_time());
        assert!(rendered.starts_with("---- Minecraft Crash Report ----"));
        assert!(rendered.contains("Time: 2026-08-02 12:34:56 UTC"));
        assert!(rendered.contains("Description: Render failure"));
        assert!(rendered.contains("-- Head --"));
        assert!(rendered.contains("-- Suppressed failures --"));
        assert!(rendered.contains("Process uptime: 1.250s"));
        assert!(rendered.contains("Minecraft version: 26.1"));
        assert!(rendered.len() <= MAX_RENDERED_REPORT_BYTES);
        assert!(report.retained_bytes() <= MAX_RETAINED_REPORT_BYTES);
    }

    #[test]
    fn huge_failure_is_truncated_before_retention_and_render() {
        let huge = "中".repeat(MAX_RENDERED_REPORT_BYTES);
        let failure = failure(&huge, &huge, &huge);
        assert!(
            failure.retained_bytes()
                <= MAX_FAILURE_PHASE_BYTES
                    + MAX_FAILURE_TITLE_BYTES
                    + MAX_FAILURE_MESSAGE_BYTES
                    + MAX_FAILURE_THREAD_BYTES
                    + MAX_FAILURE_BACKTRACE_BYTES
        );
        let report = CrashReport::new(RuntimeFailureSet::new(failure), Duration::ZERO);
        let rendered = report.friendly_report_at(fixed_time());
        assert!(rendered.len() <= MAX_RENDERED_REPORT_BYTES);
        assert!(rendered.contains("truncated"));
    }

    #[test]
    fn category_count_entry_count_and_retained_bytes_are_bounded() {
        let mut report = CrashReport::new(
            RuntimeFailureSet::new(failure("main", "bounded", "root")),
            Duration::ZERO,
        );
        for category in 0..(MAX_CATEGORIES + 20) {
            let category = report.add_category(format_args!("category-{category}"));
            for entry in 0..20 {
                report.add_category_detail(
                    category,
                    format_args!("entry-{entry}"),
                    format_args!("{}", "x".repeat(MAX_CATEGORY_VALUE_BYTES * 2)),
                );
            }
        }
        assert_eq!(report.categories.len(), MAX_CATEGORIES);
        assert_eq!(
            report
                .categories
                .iter()
                .map(|category| category.entries.len())
                .sum::<usize>(),
            MAX_CATEGORY_ENTRIES
        );
        assert!(report.retained_bytes() <= MAX_RETAINED_REPORT_BYTES);
        assert!(report.retained_budget.dropped_entries > 0);
        assert!(report.retained_budget.truncated_fields > 0);
    }

    #[test]
    fn save_is_atomic_no_clobber_and_once_per_report() {
        let directory = test_directory("atomic");
        let mut first = CrashReport::new(
            RuntimeFailureSet::new(failure("main", "first", "one")),
            Duration::ZERO,
        );
        let first_path = match first
            .save_to_directory_at(&directory, fixed_time())
            .unwrap()
        {
            CrashSaveResult::Saved(path) => path,
            other => panic!("unexpected save result: {other:?}"),
        };
        let first_bytes = fs::read(&first_path).unwrap();
        assert_eq!(
            first
                .save_to_directory_at(&directory, fixed_time())
                .unwrap(),
            CrashSaveResult::AlreadySaved(first_path.clone())
        );

        let mut second = CrashReport::new(
            RuntimeFailureSet::new(failure("main", "second", "two")),
            Duration::ZERO,
        );
        let second_path = match second
            .save_to_directory_at(&directory, fixed_time())
            .unwrap()
        {
            CrashSaveResult::Saved(path) => path,
            other => panic!("unexpected save result: {other:?}"),
        };
        assert_ne!(first_path, second_path);
        assert_eq!(fs::read(first_path).unwrap(), first_bytes);
        assert!(fs::read_to_string(second_path).unwrap().contains("second"));
    }

    #[test]
    fn writer_failure_leaves_no_report_and_uses_distinct_exit_status() {
        let parent = test_directory("failure");
        fs::create_dir_all(&parent).unwrap();
        let root_file = parent.join("not-a-directory");
        fs::write(&root_file, b"occupied").unwrap();
        let mut runtime = CrashReportRuntime::new(root_file);
        let disposition = runtime.finish_failure(RuntimeFailureSet::new(failure(
            "startup",
            "failure",
            "cannot publish",
        )));
        assert_eq!(disposition.exit_code, 2);
        assert_eq!(disposition.saved_path, None);
    }

    #[test]
    fn guarded_error_and_panic_become_typed_failures_but_success_stays_clean() {
        let runtime = CrashReportRuntime::new(test_directory("guard"));
        assert_eq!(runtime.guard(|| Ok(())), Ok(()));

        let error = runtime
            .guard(|| anyhow::bail!("startup exploded"))
            .unwrap_err();
        assert_eq!(error.primary.phase, "startup/runtime");
        assert!(error.primary.message.contains("startup exploded"));

        let panic = runtime
            .guard(|| {
                panic_any(42_u32);
                #[allow(unreachable_code)]
                Ok(())
            })
            .unwrap_err();
        assert_eq!(panic.primary.phase, "main thread");
        assert!(panic.primary.message.contains("non-string panic payload"));
    }

    #[test]
    fn persisted_failure_does_not_poison_the_next_successful_run() {
        let directory = test_directory("recovery");
        let mut runtime = CrashReportRuntime::new(directory.clone());
        let failures = runtime.guard(|| anyhow::bail!("first run")).unwrap_err();
        let disposition = runtime.finish_failure(failures);
        assert_eq!(disposition.exit_code, 1);
        let saved = disposition.saved_path.unwrap();
        let before = fs::read(&saved).unwrap();

        let next = CrashReportRuntime::new(directory);
        assert_eq!(next.guard(|| Ok(())), Ok(()));
        assert_eq!(fs::read(saved).unwrap(), before);
    }

    #[test]
    fn process_failure_child() {
        let Some(directory) = std::env::var_os("BBB_CRASH_REPORT_TEST_CHILD_DIR") else {
            return;
        };
        let mut runtime = CrashReportRuntime::new(PathBuf::from(directory));
        let disposition = runtime.finish_failure(RuntimeFailureSet::new(failure(
            "test child",
            "Process failure acceptance",
            "intentional child failure",
        )));
        exit_after_failure(disposition);
    }

    #[test]
    fn process_failure_writes_report_exits_nonzero_and_next_process_recovers() {
        let directory = test_directory("process");
        let current_exe = std::env::current_exe().unwrap();
        let output = std::process::Command::new(&current_exe)
            .args([
                "--exact",
                "crash_report::tests::process_failure_child",
                "--nocapture",
            ])
            .env("BBB_CRASH_REPORT_TEST_CHILD_DIR", &directory)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&output.stderr).contains("crash report saved"));
        let reports = fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(reports.len(), 1);
        assert!(fs::read_to_string(&reports[0])
            .unwrap()
            .contains("Process failure acceptance"));

        let recovered = std::process::Command::new(current_exe)
            .args(["--exact", "crash_report::tests::process_failure_child"])
            .output()
            .unwrap();
        assert!(recovered.status.success());
        assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
    }

    #[test]
    fn production_fatal_paths_cannot_restore_log_only_exit_or_string_worker_errors() {
        let main = include_str!("main.rs")
            .split("\n#[cfg(test)]\nmod tests")
            .next()
            .unwrap();
        let simulation = include_str!("simulation/thread.rs")
            .split("\n#[cfg(test)]\nmod tests")
            .next()
            .unwrap();
        let sources = format!("{main}\n{simulation}");
        for forbidden in [
            "failed to join simulation thread during shutdown",
            "fatal simulation render-effect sequence gap",
            "failed to resize renderer resources",
            "\"render failed\"",
            "JoinHandle<Result<(), String>>",
            "simulation thread panicked: {}",
        ] {
            assert!(
                !sources.contains(forbidden),
                "restored log-only fatal path: {forbidden}"
            );
        }
        assert!(main.contains("record_runtime_failure(&event_failures"));
        assert!(main.contains("simulation_thread.shutdown_and_join()"));
        assert!(main.contains("fatal_failures.borrow_mut().take()"));
        assert!(simulation.contains("JoinHandle<Result<(), RuntimeFailure>>"));
    }
}
