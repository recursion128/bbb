//! Fixed-window CPU phase diagnostics for renderer tail-latency investigation.
//!
//! This sampler is enabled only with renderer GPU timing (the debug GPU entry or
//! an active Vanilla-style profiler recording). It does not participate in the
//! normal frame path and deliberately reports CPU wall-clock phases separately
//! from GPU timestamp stages and MangoHud present intervals.

use std::time::Duration;

const SAMPLE_COUNT: usize = 600;
const PHASE_COUNT: usize = 11;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FramePhaseSample {
    pub(crate) frame_interval: Duration,
    pub(crate) frame_work: Duration,
    pub(crate) update: Duration,
    pub(crate) extract: Duration,
    pub(crate) render_call: Duration,
    pub(crate) surface_acquire: Duration,
    pub(crate) render_encode: Duration,
    pub(crate) queue_submit: Duration,
    pub(crate) swap_buffers: Duration,
    pub(crate) gpu_poll: Duration,
    pub(crate) publish: Duration,
}

impl FramePhaseSample {
    fn nanos(self) -> [u64; PHASE_COUNT] {
        [
            duration_nanos(self.frame_interval),
            duration_nanos(self.frame_work),
            duration_nanos(self.update),
            duration_nanos(self.extract),
            duration_nanos(self.render_call),
            duration_nanos(self.surface_acquire),
            duration_nanos(self.render_encode),
            duration_nanos(self.queue_submit),
            duration_nanos(self.swap_buffers),
            duration_nanos(self.gpu_poll),
            duration_nanos(self.publish),
        ]
    }
}

#[derive(Debug)]
pub(crate) struct FramePhaseDiagnostics {
    enabled: bool,
    samples: [Vec<u64>; PHASE_COUNT],
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
        for samples in &mut self.samples {
            samples.sort_unstable();
        }
        let p95: [u64; PHASE_COUNT] =
            std::array::from_fn(|index| nearest_rank_percentile(&self.samples[index], 95));
        let p99: [u64; PHASE_COUNT] =
            std::array::from_fn(|index| nearest_rank_percentile(&self.samples[index], 99));
        tracing::debug!(
            samples = SAMPLE_COUNT,
            frame_interval_p95_us = micros(p95[0]),
            frame_interval_p99_us = micros(p99[0]),
            frame_work_p95_us = micros(p95[1]),
            frame_work_p99_us = micros(p99[1]),
            update_p95_us = micros(p95[2]),
            update_p99_us = micros(p99[2]),
            extract_p95_us = micros(p95[3]),
            extract_p99_us = micros(p99[3]),
            render_call_p95_us = micros(p95[4]),
            render_call_p99_us = micros(p99[4]),
            surface_acquire_p95_us = micros(p95[5]),
            surface_acquire_p99_us = micros(p99[5]),
            render_encode_p95_us = micros(p95[6]),
            render_encode_p99_us = micros(p99[6]),
            queue_submit_p95_us = micros(p95[7]),
            queue_submit_p99_us = micros(p99[7]),
            swap_buffers_p95_us = micros(p95[8]),
            swap_buffers_p99_us = micros(p99[8]),
            gpu_poll_p95_us = micros(p95[9]),
            gpu_poll_p99_us = micros(p99[9]),
            publish_p95_us = micros(p95[10]),
            publish_p99_us = micros(p99[10]),
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

fn micros(nanos: u64) -> f64 {
    nanos as f64 / 1_000.0
}

#[cfg(test)]
mod tests {
    use super::{nearest_rank_percentile, FramePhaseDiagnostics, FramePhaseSample, SAMPLE_COUNT};
    use std::time::Duration;

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
}
