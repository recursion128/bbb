use std::{
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    time::Duration,
};

const SLOT_COUNT: usize = 3;
const QUERIES_PER_FRAME: u32 = 12;
const QUERY_BYTES: u64 = QUERIES_PER_FRAME as u64 * std::mem::size_of::<u64>() as u64;
const RESOLVE_SLOT_BYTES: u64 = wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT;
const STAGE_COUNT: usize = QUERIES_PER_FRAME as usize - 1;
const STAGE_LOG_SAMPLE_COUNT: u64 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotState {
    Idle,
    Encoding,
    Mapping { cpu_duration_nanos: u64 },
}

#[derive(Debug)]
struct TimingSlot {
    readback: wgpu::Buffer,
    state: SlotState,
}

#[derive(Debug)]
struct MapCompletion {
    slot: usize,
    result: Result<(), wgpu::BufferAsyncError>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct GpuFrameToken {
    slot: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GpuFrameStage {
    TerrainAtlasAnimationEnd = 1,
    OpaqueWorldEnd = 2,
    OpaqueParticlesEnd = 3,
    FeatureDepthCopiesEnd = 4,
    EntityTranslucentFeaturesEnd = 5,
    FeatureTargetsEnd = 6,
    CloudsEnd = 7,
    WeatherEnd = 8,
    TransparencyCombineEnd = 9,
    PresentationTailEnd = 10,
}

#[derive(Debug)]
pub(super) struct GpuFrameTiming {
    query_set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: Vec<TimingSlot>,
    completion_sender: Sender<MapCompletion>,
    completions: Receiver<MapCompletion>,
    timestamp_period_nanos: f64,
    next_slot: usize,
    enabled: bool,
    latest_utilization: Option<f64>,
    stage_sample_count: u64,
    stage_nanos_sum: [u128; STAGE_COUNT],
    stage_nanos_samples: [Vec<u64>; STAGE_COUNT],
}

impl GpuFrameTiming {
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        device
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY)
            .then(|| {
                let query_count = SLOT_COUNT as u32 * QUERIES_PER_FRAME;
                let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
                    label: Some("bbb-frame-timestamp-queries"),
                    ty: wgpu::QueryType::Timestamp,
                    count: query_count,
                });
                let resolve = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("bbb-frame-timestamp-resolve"),
                    size: SLOT_COUNT as u64 * RESOLVE_SLOT_BYTES,
                    usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });
                let slots = (0..SLOT_COUNT)
                    .map(|_| TimingSlot {
                        readback: device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("bbb-frame-timestamp-readback"),
                            size: QUERY_BYTES,
                            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                            mapped_at_creation: false,
                        }),
                        state: SlotState::Idle,
                    })
                    .collect();
                let (completion_sender, completions) = mpsc::channel();
                Self {
                    query_set,
                    resolve,
                    slots,
                    completion_sender,
                    completions,
                    timestamp_period_nanos: f64::from(queue.get_timestamp_period()),
                    next_slot: 0,
                    enabled: false,
                    latest_utilization: None,
                    stage_sample_count: 0,
                    stage_nanos_sum: [0; STAGE_COUNT],
                    stage_nanos_samples: std::array::from_fn(|_| {
                        Vec::with_capacity(STAGE_LOG_SAMPLE_COUNT as usize)
                    }),
                }
            })
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.latest_utilization = None;
            self.stage_sample_count = 0;
            self.stage_nanos_sum = [0; STAGE_COUNT];
            for samples in &mut self.stage_nanos_samples {
                samples.clear();
            }
        }
    }

    pub(super) fn poll(&mut self, device: &wgpu::Device) -> Duration {
        if !self.enabled
            && self
                .slots
                .iter()
                .all(|slot| !matches!(slot.state, SlotState::Mapping { .. }))
        {
            return Duration::ZERO;
        }
        let started_at = std::time::Instant::now();
        device.poll(wgpu::Maintain::Poll);
        loop {
            match self.completions.try_recv() {
                Ok(completion) => self.finish_mapping(completion),
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }
        started_at.elapsed()
    }

    pub(super) fn begin_frame(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Option<GpuFrameToken> {
        if !self.enabled {
            return None;
        }
        let slot = (0..self.slots.len())
            .map(|offset| (self.next_slot + offset) % self.slots.len())
            .find(|slot| self.slots[*slot].state == SlotState::Idle)?;
        self.next_slot = (slot + 1) % self.slots.len();
        self.slots[slot].state = SlotState::Encoding;
        encoder.write_timestamp(&self.query_set, query_index(slot));
        Some(GpuFrameToken { slot })
    }

    pub(super) fn mark_stage(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        token: GpuFrameToken,
        stage: GpuFrameStage,
    ) {
        debug_assert_eq!(self.slots[token.slot].state, SlotState::Encoding);
        encoder.write_timestamp(&self.query_set, query_index(token.slot) + stage as u32);
    }

    pub(super) fn finish_encoding(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        token: GpuFrameToken,
    ) {
        debug_assert_eq!(self.slots[token.slot].state, SlotState::Encoding);
        let start = query_index(token.slot);
        encoder.write_timestamp(&self.query_set, start + QUERIES_PER_FRAME - 1);
        let resolve_offset = resolve_offset(token.slot);
        encoder.resolve_query_set(
            &self.query_set,
            start..start + QUERIES_PER_FRAME,
            &self.resolve,
            resolve_offset,
        );
        encoder.copy_buffer_to_buffer(
            &self.resolve,
            resolve_offset,
            &self.slots[token.slot].readback,
            0,
            QUERY_BYTES,
        );
    }

    pub(super) fn after_submit(&mut self, token: GpuFrameToken, cpu_duration: Duration) {
        debug_assert_eq!(self.slots[token.slot].state, SlotState::Encoding);
        self.slots[token.slot].state = SlotState::Mapping {
            cpu_duration_nanos: duration_nanos(cpu_duration),
        };
        let sender = self.completion_sender.clone();
        let slot = token.slot;
        self.slots[slot]
            .readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(MapCompletion { slot, result });
            });
    }

    pub(super) fn cancel(&mut self, token: GpuFrameToken) {
        if self.slots[token.slot].state == SlotState::Encoding {
            self.slots[token.slot].state = SlotState::Idle;
        }
    }

    pub(super) fn latest_utilization(&self) -> Option<f64> {
        self.latest_utilization
    }

    fn finish_mapping(&mut self, completion: MapCompletion) {
        let mut stage_sample = None;
        let slot = &mut self.slots[completion.slot];
        let SlotState::Mapping { cpu_duration_nanos } = slot.state else {
            return;
        };
        if completion.result.is_ok() {
            let mapped = slot.readback.slice(..).get_mapped_range();
            let timestamps: Option<[u64; QUERIES_PER_FRAME as usize]> = mapped
                .get(..QUERY_BYTES as usize)
                .and_then(|bytes| bytemuck::try_cast_slice::<u8, u64>(bytes).ok())
                .and_then(|values| values.get(..QUERIES_PER_FRAME as usize))
                .and_then(|values| values.try_into().ok());
            if self.enabled {
                self.latest_utilization = timestamps.and_then(|timestamps| {
                    gpu_utilization_from_timestamps(
                        [timestamps[0], timestamps[QUERIES_PER_FRAME as usize - 1]],
                        self.timestamp_period_nanos,
                        cpu_duration_nanos,
                    )
                });
                stage_sample = timestamps.and_then(|timestamps| {
                    gpu_stage_nanos_from_timestamps(timestamps, self.timestamp_period_nanos)
                });
            }
            drop(mapped);
            slot.readback.unmap();
        } else {
            self.latest_utilization = None;
        }
        slot.state = SlotState::Idle;
        if let Some(stage_nanos) = stage_sample {
            self.record_stage_sample(stage_nanos);
        }
    }

    fn record_stage_sample(&mut self, stage_nanos: [u64; STAGE_COUNT]) {
        self.stage_sample_count = self.stage_sample_count.saturating_add(1);
        for ((sum, samples), nanos) in self
            .stage_nanos_sum
            .iter_mut()
            .zip(&mut self.stage_nanos_samples)
            .zip(stage_nanos)
        {
            *sum = sum.saturating_add(u128::from(nanos));
            samples.push(nanos);
        }
        if self.stage_sample_count < STAGE_LOG_SAMPLE_COUNT {
            return;
        }
        let averages = self
            .stage_nanos_sum
            .map(|sum| (sum / u128::from(self.stage_sample_count)) as u64);
        for samples in &mut self.stage_nanos_samples {
            samples.sort_unstable();
        }
        let p95: [u64; STAGE_COUNT] = std::array::from_fn(|index| {
            nearest_rank_percentile(&self.stage_nanos_samples[index], 95)
        });
        let p99: [u64; STAGE_COUNT] = std::array::from_fn(|index| {
            nearest_rank_percentile(&self.stage_nanos_samples[index], 99)
        });
        tracing::debug!(
            samples = self.stage_sample_count,
            terrain_atlas_animation_us = averages[0] as f64 / 1_000.0,
            opaque_world_us = averages[1] as f64 / 1_000.0,
            opaque_particles_us = averages[2] as f64 / 1_000.0,
            feature_depth_copies_us = averages[3] as f64 / 1_000.0,
            entity_translucent_features_us = averages[4] as f64 / 1_000.0,
            remaining_feature_targets_us = averages[5] as f64 / 1_000.0,
            clouds_us = averages[6] as f64 / 1_000.0,
            weather_us = averages[7] as f64 / 1_000.0,
            transparency_combine_us = averages[8] as f64 / 1_000.0,
            presentation_tail_us = averages[9] as f64 / 1_000.0,
            encoder_tail_us = averages[10] as f64 / 1_000.0,
            terrain_atlas_animation_p95_us = p95[0] as f64 / 1_000.0,
            terrain_atlas_animation_p99_us = p99[0] as f64 / 1_000.0,
            opaque_world_p95_us = p95[1] as f64 / 1_000.0,
            opaque_world_p99_us = p99[1] as f64 / 1_000.0,
            feature_depth_copies_p95_us = p95[3] as f64 / 1_000.0,
            feature_depth_copies_p99_us = p99[3] as f64 / 1_000.0,
            remaining_feature_targets_p95_us = p95[5] as f64 / 1_000.0,
            remaining_feature_targets_p99_us = p99[5] as f64 / 1_000.0,
            clouds_p95_us = p95[6] as f64 / 1_000.0,
            clouds_p99_us = p99[6] as f64 / 1_000.0,
            transparency_combine_p95_us = p95[8] as f64 / 1_000.0,
            transparency_combine_p99_us = p99[8] as f64 / 1_000.0,
            presentation_tail_p95_us = p95[9] as f64 / 1_000.0,
            presentation_tail_p99_us = p99[9] as f64 / 1_000.0,
            encoder_tail_p95_us = p95[10] as f64 / 1_000.0,
            encoder_tail_p99_us = p99[10] as f64 / 1_000.0,
            "averaged renderer GPU stage timings"
        );
        self.stage_sample_count = 0;
        self.stage_nanos_sum = [0; STAGE_COUNT];
        for samples in &mut self.stage_nanos_samples {
            samples.clear();
        }
    }
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

pub(super) fn optional_gpu_timing_features(adapter_features: wgpu::Features) -> wgpu::Features {
    adapter_features & wgpu::Features::TIMESTAMP_QUERY
}

fn gpu_utilization_from_timestamps(
    timestamps: [u64; 2],
    timestamp_period_nanos: f64,
    cpu_duration_nanos: u64,
) -> Option<f64> {
    let ticks = timestamps[1].checked_sub(timestamps[0])?;
    if ticks == 0
        || cpu_duration_nanos == 0
        || !timestamp_period_nanos.is_finite()
        || timestamp_period_nanos <= 0.0
    {
        return None;
    }
    let utilization = ticks as f64 * timestamp_period_nanos * 100.0 / cpu_duration_nanos as f64;
    utilization.is_finite().then_some(utilization)
}

fn gpu_stage_nanos_from_timestamps(
    timestamps: [u64; QUERIES_PER_FRAME as usize],
    timestamp_period_nanos: f64,
) -> Option<[u64; STAGE_COUNT]> {
    if !timestamp_period_nanos.is_finite() || timestamp_period_nanos <= 0.0 {
        return None;
    }
    let mut stage_nanos = [0; STAGE_COUNT];
    for (index, pair) in timestamps.windows(2).enumerate() {
        let ticks = pair[1].checked_sub(pair[0])?;
        let nanos = ticks as f64 * timestamp_period_nanos;
        if !nanos.is_finite() || nanos < 0.0 || nanos > u64::MAX as f64 {
            return None;
        }
        stage_nanos[index] = nanos.round() as u64;
    }
    Some(stage_nanos)
}

fn query_index(slot: usize) -> u32 {
    slot as u32 * QUERIES_PER_FRAME
}

fn resolve_offset(slot: usize) -> u64 {
    slot as u64 * RESOLVE_SLOT_BYTES
}

fn duration_nanos(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_timestamp_test_device() -> Option<(wgpu::Device, wgpu::Queue)> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;
        let required_features = optional_gpu_timing_features(adapter.features());
        if !required_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("bbb-gpu-timing-test-device"),
                required_features,
                required_limits:
                    wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            },
            None,
        ))
        .ok()
    }

    #[test]
    fn gpu_timing_requests_only_supported_timestamp_queries() {
        assert_eq!(
            optional_gpu_timing_features(wgpu::Features::empty()),
            wgpu::Features::empty()
        );
        assert_eq!(
            optional_gpu_timing_features(
                wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TEXTURE_COMPRESSION_BC
            ),
            wgpu::Features::TIMESTAMP_QUERY
        );
    }

    #[test]
    fn gpu_timing_utilization_uses_real_query_delta_and_cpu_frame_duration() {
        assert_eq!(
            gpu_utilization_from_timestamps([100, 300], 2.0, 1_000),
            Some(40.0)
        );
        assert_eq!(
            gpu_utilization_from_timestamps([300, 100], 2.0, 1_000),
            None
        );
        assert_eq!(gpu_utilization_from_timestamps([100, 300], 2.0, 0), None);
        assert_eq!(
            gpu_utilization_from_timestamps([100, 300], f64::NAN, 1_000),
            None
        );
    }

    #[test]
    fn gpu_stage_timing_uses_ordered_boundaries_and_rejects_invalid_input() {
        assert_eq!(
            gpu_stage_nanos_from_timestamps(
                [10, 20, 40, 70, 110, 160, 220, 290, 370, 460, 560, 670],
                2.0
            ),
            Some([20, 40, 60, 80, 100, 120, 140, 160, 180, 200, 220])
        );
        assert_eq!(
            gpu_stage_nanos_from_timestamps(
                [10, 20, 19, 70, 110, 160, 220, 290, 370, 460, 560, 670],
                2.0
            ),
            None
        );
        assert_eq!(
            gpu_stage_nanos_from_timestamps(
                [10, 20, 40, 70, 110, 160, 220, 290, 370, 460, 560, 670],
                f64::NAN
            ),
            None
        );
    }

    #[test]
    fn gpu_timing_resolve_slots_satisfy_wgpu_alignment() {
        for slot in 0..SLOT_COUNT {
            assert_eq!(
                resolve_offset(slot) % wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT,
                0
            );
            assert!(QUERY_BYTES <= RESOLVE_SLOT_BYTES);
        }
    }

    #[test]
    fn gpu_stage_percentiles_use_nearest_rank_without_averaging_tails() {
        let samples = (1..=100).collect::<Vec<_>>();
        assert_eq!(nearest_rank_percentile(&samples, 50), 50);
        assert_eq!(nearest_rank_percentile(&samples, 95), 95);
        assert_eq!(nearest_rank_percentile(&samples, 99), 99);
        assert_eq!(nearest_rank_percentile(&[], 99), 0);
    }

    #[test]
    fn gpu_timing_resolves_all_in_flight_slots_on_wgpu_device() {
        let Some((device, queue)) = request_timestamp_test_device() else {
            eprintln!("skipping GPU timestamp test without a compatible adapter");
            return;
        };
        let mut timing = GpuFrameTiming::new(&device, &queue).expect("timestamp timing");
        timing.set_enabled(true);

        for _ in 0..SLOT_COUNT {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bbb-gpu-timing-slot-test"),
            });
            let token = timing.begin_frame(&mut encoder).expect("free timing slot");
            timing.mark_stage(&mut encoder, token, GpuFrameStage::TerrainAtlasAnimationEnd);
            timing.mark_stage(&mut encoder, token, GpuFrameStage::OpaqueWorldEnd);
            timing.mark_stage(&mut encoder, token, GpuFrameStage::OpaqueParticlesEnd);
            timing.mark_stage(&mut encoder, token, GpuFrameStage::FeatureDepthCopiesEnd);
            timing.mark_stage(
                &mut encoder,
                token,
                GpuFrameStage::EntityTranslucentFeaturesEnd,
            );
            timing.mark_stage(&mut encoder, token, GpuFrameStage::FeatureTargetsEnd);
            timing.mark_stage(&mut encoder, token, GpuFrameStage::CloudsEnd);
            timing.mark_stage(&mut encoder, token, GpuFrameStage::WeatherEnd);
            timing.mark_stage(&mut encoder, token, GpuFrameStage::TransparencyCombineEnd);
            timing.mark_stage(&mut encoder, token, GpuFrameStage::PresentationTailEnd);
            timing.finish_encoding(&mut encoder, token);
            queue.submit(Some(encoder.finish()));
            timing.after_submit(token, Duration::from_millis(16));
        }

        device.poll(wgpu::Maintain::Wait);
        let _ = timing.poll(&device);
        assert!(timing
            .slots
            .iter()
            .all(|slot| slot.state == SlotState::Idle));
    }
}
