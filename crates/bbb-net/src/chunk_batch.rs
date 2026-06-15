use std::time::{Duration, Instant};

const INITIAL_NANOS_PER_CHUNK: f64 = 2_000_000.0;
const TARGET_NANOS_PER_TICK: f64 = 7_000_000.0;
const MAX_OLD_SAMPLES_WEIGHT: u32 = 49;
const CLAMP_COEFFICIENT: f64 = 3.0;

#[derive(Debug, Clone)]
pub(crate) struct ChunkBatchSizeCalculator {
    aggregated_nanos_per_chunk: f64,
    old_samples_weight: u32,
    chunk_batch_start_time: Instant,
}

impl Default for ChunkBatchSizeCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkBatchSizeCalculator {
    pub(crate) fn new() -> Self {
        Self {
            aggregated_nanos_per_chunk: INITIAL_NANOS_PER_CHUNK,
            old_samples_weight: 1,
            chunk_batch_start_time: Instant::now(),
        }
    }

    pub(crate) fn on_batch_start(&mut self) {
        self.chunk_batch_start_time = Instant::now();
    }

    pub(crate) fn on_batch_finished(&mut self, batch_size: i32) -> f32 {
        self.on_batch_finished_duration(
            batch_size,
            Instant::now().saturating_duration_since(self.chunk_batch_start_time),
        )
    }

    pub(crate) fn on_batch_finished_duration(
        &mut self,
        batch_size: i32,
        batch_duration: Duration,
    ) -> f32 {
        if batch_size > 0 {
            let nanos_per_chunk = batch_duration.as_nanos() as f64 / f64::from(batch_size);
            let clamped_nanos_per_chunk = nanos_per_chunk.clamp(
                self.aggregated_nanos_per_chunk / CLAMP_COEFFICIENT,
                self.aggregated_nanos_per_chunk * CLAMP_COEFFICIENT,
            );
            self.aggregated_nanos_per_chunk = (self.aggregated_nanos_per_chunk
                * f64::from(self.old_samples_weight)
                + clamped_nanos_per_chunk)
                / f64::from(self.old_samples_weight + 1);
            self.old_samples_weight = (self.old_samples_weight + 1).min(MAX_OLD_SAMPLES_WEIGHT);
        }

        self.desired_chunks_per_tick()
    }

    pub(crate) fn desired_chunks_per_tick(&self) -> f32 {
        (TARGET_NANOS_PER_TICK / self.aggregated_nanos_per_chunk) as f32
    }

    #[cfg(test)]
    fn old_samples_weight(&self) -> u32 {
        self.old_samples_weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_batch_calculator_starts_with_vanilla_default_rate() {
        let calculator = ChunkBatchSizeCalculator::new();

        assert_near(calculator.desired_chunks_per_tick(), 3.5);
    }

    #[test]
    fn chunk_batch_calculator_uses_batch_duration_per_chunk() {
        let mut calculator = ChunkBatchSizeCalculator::new();

        let desired = calculator.on_batch_finished_duration(9, Duration::from_millis(18));

        assert_near(desired, 3.5);
        assert_eq!(calculator.old_samples_weight(), 2);
    }

    #[test]
    fn chunk_batch_calculator_clamps_fast_and_slow_samples() {
        let mut fast = ChunkBatchSizeCalculator::new();
        let fast_desired = fast.on_batch_finished_duration(9, Duration::from_millis(3));

        assert_near(fast_desired, 5.25);

        let mut slow = ChunkBatchSizeCalculator::new();
        let slow_desired = slow.on_batch_finished_duration(1, Duration::from_millis(20));

        assert_near(slow_desired, 1.75);
    }

    #[test]
    fn chunk_batch_calculator_ignores_non_positive_batch_sizes() {
        let mut calculator = ChunkBatchSizeCalculator::new();

        assert_near(
            calculator.on_batch_finished_duration(0, Duration::from_millis(18)),
            3.5,
        );
        assert_near(
            calculator.on_batch_finished_duration(-1, Duration::from_millis(18)),
            3.5,
        );
        assert_eq!(calculator.old_samples_weight(), 1);
    }

    #[test]
    fn chunk_batch_calculator_caps_old_sample_weight() {
        let mut calculator = ChunkBatchSizeCalculator::new();

        for _ in 0..100 {
            calculator.on_batch_finished_duration(1, Duration::from_millis(2));
        }

        assert_eq!(calculator.old_samples_weight(), 49);
    }

    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {actual} near {expected}"
        );
    }
}
