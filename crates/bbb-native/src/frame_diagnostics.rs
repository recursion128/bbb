//! Fixed-window CPU phase diagnostics for renderer tail-latency investigation.
//!
//! This sampler is enabled only with renderer GPU timing (the debug GPU entry or
//! an active Vanilla-style profiler recording). It does not participate in the
//! normal frame path and deliberately reports CPU wall-clock phases separately
//! from GPU timestamp stages and MangoHud present intervals.

use std::time::Duration;

const SAMPLE_COUNT: usize = 600;
const PHASE_COUNT: usize = 14;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FramePhaseSample {
    pub(crate) frame_interval: Duration,
    pub(crate) frame_work: Duration,
    pub(crate) update: Duration,
    pub(crate) extract: Duration,
    pub(crate) render_call_total: Duration,
    pub(crate) cpu_encode_before_wait: Duration,
    pub(crate) previous_submission_wait: Duration,
    pub(crate) surface_acquire: Duration,
    pub(crate) presentation_tail_encode: Duration,
    pub(crate) queue_submit: Duration,
    pub(crate) swap_buffers_present: Duration,
    pub(crate) gpu_async_poll: Duration,
    pub(crate) legacy_render_encode: Duration,
    pub(crate) publish: Duration,
}

impl FramePhaseSample {
    fn nanos(self) -> [u64; PHASE_COUNT] {
        [
            duration_nanos(self.frame_interval),
            duration_nanos(self.frame_work),
            duration_nanos(self.update),
            duration_nanos(self.extract),
            duration_nanos(self.render_call_total),
            duration_nanos(self.cpu_encode_before_wait),
            duration_nanos(self.previous_submission_wait),
            duration_nanos(self.surface_acquire),
            duration_nanos(self.presentation_tail_encode),
            duration_nanos(self.queue_submit),
            duration_nanos(self.swap_buffers_present),
            duration_nanos(self.gpu_async_poll),
            duration_nanos(self.legacy_render_encode),
            duration_nanos(self.publish),
        ]
    }
}

#[derive(Debug)]
pub(crate) struct FramePhaseDiagnostics {
    enabled: bool,
    samples: [Vec<u64>; PHASE_COUNT],
}

/// Match Vanilla's metrics-recorder tick window on an event-loop renderer.
///
/// Vanilla starts the tick sampler before `pollEvents` and ends it after
/// `runTick`, so consecutive samples partition wall-clock time. BBB's
/// `frame_work` starts later, inside `RedrawRequested`; using the larger of it
/// and the render-start interval double-counts work across adjacent frames.
/// The render-start interval is therefore the application-frame sample. Only
/// an explicit client cap may own the residual `frameLimiter` section.
pub(crate) fn profiler_frame_timing(
    frame_interval: Duration,
    frame_work: Duration,
    limiter_enabled: bool,
) -> (Duration, Duration) {
    let limiter = limiter_enabled
        .then(|| frame_interval.saturating_sub(frame_work))
        .unwrap_or(Duration::ZERO);
    (frame_interval, limiter)
}

impl Default for FramePhaseDiagnostics {
    fn default() -> Self {
        Self {
            enabled: false,
            samples: std::array::from_fn(|_| Vec::with_capacity(SAMPLE_COUNT)),
        }
    }
}

impl FramePhaseDiagnostics {
    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        self.clear();
    }

    pub(crate) fn record(&mut self, sample: FramePhaseSample) {
        if !self.enabled {
            return;
        }
        for (samples, nanos) in self.samples.iter_mut().zip(sample.nanos()) {
            samples.push(nanos);
        }
        if self.samples[0].len() < SAMPLE_COUNT {
            return;
        }
        let summaries: [PhaseSummary; PHASE_COUNT] =
            std::array::from_fn(|index| phase_summary(&self.samples[index], &self.samples[0]));
        tracing::debug!(
            samples = SAMPLE_COUNT,
            frame_interval_p50_us = micros(summaries[0].p50),
            frame_interval_p95_us = micros(summaries[0].p95),
            frame_interval_p99_us = micros(summaries[0].p99),
            frame_interval_stddev_us = nanos_to_micros(summaries[0].stddev),
            render_call_total_p50_us = micros(summaries[4].p50),
            render_call_total_p95_us = micros(summaries[4].p95),
            render_call_total_p99_us = micros(summaries[4].p99),
            render_call_total_stddev_us = nanos_to_micros(summaries[4].stddev),
            render_call_total_interval_correlation = summaries[4].interval_correlation,
            cpu_encode_before_wait_p50_us = micros(summaries[5].p50),
            cpu_encode_before_wait_p95_us = micros(summaries[5].p95),
            cpu_encode_before_wait_p99_us = micros(summaries[5].p99),
            cpu_encode_before_wait_stddev_us = nanos_to_micros(summaries[5].stddev),
            cpu_encode_before_wait_interval_correlation = summaries[5].interval_correlation,
            previous_submission_wait_p50_us = micros(summaries[6].p50),
            previous_submission_wait_p95_us = micros(summaries[6].p95),
            previous_submission_wait_p99_us = micros(summaries[6].p99),
            previous_submission_wait_stddev_us = nanos_to_micros(summaries[6].stddev),
            previous_submission_wait_interval_correlation = summaries[6].interval_correlation,
            surface_acquire_p50_us = micros(summaries[7].p50),
            surface_acquire_p95_us = micros(summaries[7].p95),
            surface_acquire_p99_us = micros(summaries[7].p99),
            surface_acquire_stddev_us = nanos_to_micros(summaries[7].stddev),
            surface_acquire_interval_correlation = summaries[7].interval_correlation,
            presentation_tail_encode_p50_us = micros(summaries[8].p50),
            presentation_tail_encode_p95_us = micros(summaries[8].p95),
            presentation_tail_encode_p99_us = micros(summaries[8].p99),
            presentation_tail_encode_stddev_us = nanos_to_micros(summaries[8].stddev),
            presentation_tail_encode_interval_correlation = summaries[8].interval_correlation,
            queue_submit_p50_us = micros(summaries[9].p50),
            queue_submit_p95_us = micros(summaries[9].p95),
            queue_submit_p99_us = micros(summaries[9].p99),
            queue_submit_stddev_us = nanos_to_micros(summaries[9].stddev),
            queue_submit_interval_correlation = summaries[9].interval_correlation,
            swap_buffers_present_p50_us = micros(summaries[10].p50),
            swap_buffers_present_p95_us = micros(summaries[10].p95),
            swap_buffers_present_p99_us = micros(summaries[10].p99),
            swap_buffers_present_stddev_us = nanos_to_micros(summaries[10].stddev),
            swap_buffers_present_interval_correlation = summaries[10].interval_correlation,
            gpu_async_poll_p50_us = micros(summaries[11].p50),
            gpu_async_poll_p95_us = micros(summaries[11].p95),
            gpu_async_poll_p99_us = micros(summaries[11].p99),
            gpu_async_poll_stddev_us = nanos_to_micros(summaries[11].stddev),
            gpu_async_poll_interval_correlation = summaries[11].interval_correlation,
            "renderer CPU phase tail diagnostics"
        );
        self.clear();
    }

    fn clear(&mut self) {
        for samples in &mut self.samples {
            samples.clear();
        }
    }
}

fn duration_nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn nearest_rank_percentile(sorted_samples: &[u64], percentile: usize) -> u64 {
    if sorted_samples.is_empty() {
        return 0;
    }
    let rank = sorted_samples
        .len()
        .saturating_mul(percentile)
        .saturating_add(99)
        / 100;
    sorted_samples[rank.saturating_sub(1).min(sorted_samples.len() - 1)]
}

#[derive(Debug, Clone, Copy, Default)]
struct PhaseSummary {
    p50: u64,
    p95: u64,
    p99: u64,
    stddev: f64,
    interval_correlation: f64,
}

fn phase_summary(samples: &[u64], interval: &[u64]) -> PhaseSummary {
    if samples.is_empty() {
        return PhaseSummary::default();
    }
    let mut ordered = samples.to_vec();
    ordered.sort_unstable();
    PhaseSummary {
        p50: nearest_rank_percentile(&ordered, 50),
        p95: nearest_rank_percentile(&ordered, 95),
        p99: nearest_rank_percentile(&ordered, 99),
        stddev: population_stddev(samples),
        interval_correlation: pearson_correlation(samples, interval),
    }
}

fn population_stddev(samples: &[u64]) -> f64 {
    if samples.len() < 2 {
        return 0.0;
    }
    let mean = samples.iter().map(|value| *value as f64).sum::<f64>() / samples.len() as f64;
    let variance = samples
        .iter()
        .map(|value| (*value as f64 - mean).powi(2))
        .sum::<f64>()
        / samples.len() as f64;
    variance.sqrt()
}

fn pearson_correlation(left: &[u64], right: &[u64]) -> f64 {
    if left.len() != right.len() || left.len() < 2 {
        return 0.0;
    }
    let left_mean = left.iter().map(|value| *value as f64).sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().map(|value| *value as f64).sum::<f64>() / right.len() as f64;
    let mut covariance = 0.0;
    let mut left_square_sum = 0.0;
    let mut right_square_sum = 0.0;
    for (left, right) in left.iter().zip(right) {
        let left_delta = *left as f64 - left_mean;
        let right_delta = *right as f64 - right_mean;
        covariance += left_delta * right_delta;
        left_square_sum += left_delta * left_delta;
        right_square_sum += right_delta * right_delta;
    }
    let divisor = (left_square_sum * right_square_sum).sqrt();
    if divisor == 0.0 {
        0.0
    } else {
        covariance / divisor
    }
}

fn micros(nanos: u64) -> f64 {
    nanos as f64 / 1_000.0
}

fn nanos_to_micros(nanos: f64) -> f64 {
    nanos / 1_000.0
}

#[cfg(test)]
mod tests {
    use super::{
        nearest_rank_percentile, pearson_correlation, phase_summary, profiler_frame_timing,
        FramePhaseDiagnostics, FramePhaseSample, SAMPLE_COUNT,
    };
    use std::time::Duration;

    #[test]
    fn profiler_ticktime_partitions_render_starts_without_double_counting_work() {
        assert_eq!(
            profiler_frame_timing(Duration::from_millis(6), Duration::from_millis(11), false,),
            (Duration::from_millis(6), Duration::ZERO)
        );
    }

    #[test]
    fn profiler_frame_limiter_is_owned_only_by_an_explicit_cap() {
        assert_eq!(
            profiler_frame_timing(Duration::from_millis(16), Duration::from_millis(5), true,),
            (Duration::from_millis(16), Duration::from_millis(11))
        );
        assert_eq!(
            profiler_frame_timing(Duration::from_millis(16), Duration::from_millis(5), false,),
            (Duration::from_millis(16), Duration::ZERO)
        );
    }

    #[test]
    fn disabled_frame_diagnostics_retain_no_samples() {
        let mut diagnostics = FramePhaseDiagnostics::default();
        diagnostics.record(FramePhaseSample {
            frame_interval: Duration::from_millis(1),
            ..FramePhaseSample::default()
        });
        assert!(diagnostics.samples.iter().all(Vec::is_empty));
    }

    #[test]
    fn fixed_window_clears_after_reporting() {
        let mut diagnostics = FramePhaseDiagnostics::default();
        diagnostics.set_enabled(true);
        for millis in 1..=SAMPLE_COUNT {
            diagnostics.record(FramePhaseSample {
                frame_interval: Duration::from_millis(millis as u64),
                ..FramePhaseSample::default()
            });
        }
        assert!(diagnostics.samples.iter().all(Vec::is_empty));
    }

    #[test]
    fn frame_phase_percentiles_use_nearest_rank() {
        let samples = (1..=100).collect::<Vec<_>>();
        assert_eq!(nearest_rank_percentile(&samples, 95), 95);
        assert_eq!(nearest_rank_percentile(&samples, 99), 99);
        assert_eq!(nearest_rank_percentile(&[], 99), 0);
    }

    #[test]
    fn frame_phase_summary_preserves_pairing_for_interval_correlation() {
        let interval = [10, 20, 30, 40];
        let summary = phase_summary(&[1, 2, 3, 4], &interval);
        assert_eq!((summary.p50, summary.p95, summary.p99), (2, 4, 4));
        assert!((summary.interval_correlation - 1.0).abs() < f64::EPSILON);
        assert!((pearson_correlation(&[4, 3, 2, 1], &interval) + 1.0).abs() < f64::EPSILON);
    }
}
