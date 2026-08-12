use std::{
    collections::VecDeque,
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    time::Duration,
};

// Give every asynchronous slot its own zero-based query set. Eight slots keep
// the uncapped renderer from dropping timestamp samples while the backend
// finishes asynchronous readback; the query sets and readback buffers remain
// tiny and do not alter render work.
const SLOT_COUNT: usize = 8;
const QUERIES_PER_FRAME: u32 = 12;
const TOTAL_ONLY_QUERIES_PER_FRAME: u32 = 2;
const RESOLVE_SLOT_BYTES: u64 = wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT;
const STAGE_COUNT: usize = QUERIES_PER_FRAME as usize - 1;
// Detailed stage samples are archived per frame by the profiler. This bounded
// buffer only feeds the human-readable tracing summary, so sorting it while a
// profile is recording would perturb the very tail being measured. Flush it
// when GPU timing is disabled instead.
const STAGE_LOG_SAMPLE_CAPACITY: u64 = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotState {
    Idle,
    Encoding {
        detailed_stages: bool,
    },
    Mapping {
        cpu_duration_nanos: u64,
        detailed_stages: bool,
    },
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
    detailed_stages: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimestampWriteStrategy {
    Encoder,
    PassBoundary,
}

#[derive(Debug)]
struct TimestampMarker {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkerSamplePoint {
    AfterMarker,
    BeforeMarker,
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

/// GPU durations between the existing ordered frame-stage timestamp boundaries.
///
/// These values are populated only when detailed GPU timing is explicitly
/// enabled. The normal total-duration query path remains two timestamps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RendererGpuStageProfile {
    pub terrain_atlas_animation: Duration,
    pub opaque_world: Duration,
    pub opaque_particles: Duration,
    pub feature_depth_copies: Duration,
    pub entity_translucent_features: Duration,
    pub remaining_feature_targets: Duration,
    pub clouds: Duration,
    pub weather: Duration,
    pub transparency_combine: Duration,
    pub presentation_tail: Duration,
    pub encoder_tail: Duration,
}

impl RendererGpuStageProfile {
    fn from_nanos(values: [u64; STAGE_COUNT]) -> Self {
        let [terrain_atlas_animation, opaque_world, opaque_particles, feature_depth_copies, entity_translucent_features, remaining_feature_targets, clouds, weather, transparency_combine, presentation_tail, encoder_tail] =
            values.map(Duration::from_nanos);
        Self {
            terrain_atlas_animation,
            opaque_world,
            opaque_particles,
            feature_depth_copies,
            entity_translucent_features,
            remaining_feature_targets,
            clouds,
            weather,
            transparency_combine,
            presentation_tail,
            encoder_tail,
        }
    }
}

/// One completed GPU timestamp sample and the CPU frame interval captured for
/// the same submitted frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RendererGpuTimingProfile {
    pub duration: Duration,
    pub application_frame_interval: Duration,
    pub stages: Option<RendererGpuStageProfile>,
}

#[derive(Debug)]
pub(super) struct GpuFrameTiming {
    query_sets: Vec<wgpu::QuerySet>,
    resolve: wgpu::Buffer,
    slots: Vec<TimingSlot>,
    completion_sender: Sender<MapCompletion>,
    completions: Receiver<MapCompletion>,
    timestamp_period_nanos: f64,
    timestamp_write_strategy: TimestampWriteStrategy,
    timestamp_marker: Option<TimestampMarker>,
    queries_per_frame: u32,
    detailed_stages: bool,
    next_slot: usize,
    enabled: bool,
    latest_utilization: Option<f64>,
    completed_profiles: VecDeque<RendererGpuTimingProfile>,
    stage_sample_count: u64,
    stage_nanos_sum: [u128; STAGE_COUNT],
    stage_nanos_samples: [Vec<u64>; STAGE_COUNT],
}

impl GpuFrameTiming {
    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        let detailed_stages =
            tracing::enabled!(target: "bbb_renderer::gpu_timing", tracing::Level::DEBUG);
        Self::new_with_detailed_stages(device, queue, detailed_stages)
    }

    fn new_with_detailed_stages(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        detailed_stages: bool,
    ) -> Option<Self> {
        timestamp_write_strategy(device.features()).map(|timestamp_write_strategy| {
            tracing::info!(
                strategy = ?timestamp_write_strategy,
                "renderer GPU timestamp strategy selected"
            );
            let queries_per_frame = queries_per_frame(detailed_stages);
            let query_bytes = u64::from(queries_per_frame) * std::mem::size_of::<u64>() as u64;
            let query_sets = (0..SLOT_COUNT)
                .map(|_| {
                    device.create_query_set(&wgpu::QuerySetDescriptor {
                        label: Some("bbb-frame-timestamp-queries"),
                        ty: wgpu::QueryType::Timestamp,
                        count: queries_per_frame,
                    })
                })
                .collect();
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
                        size: query_bytes,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    }),
                    state: SlotState::Idle,
                })
                .collect();
            let (completion_sender, completions) = mpsc::channel();
            let timestamp_marker =
                (timestamp_write_strategy == TimestampWriteStrategy::PassBoundary).then(|| {
                    let texture = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("bbb-frame-timestamp-marker"),
                        size: wgpu::Extent3d {
                            width: 1,
                            height: 1,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                        view_formats: &[],
                    });
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    TimestampMarker {
                        _texture: texture,
                        view,
                    }
                });
            Self {
                query_sets,
                resolve,
                slots,
                completion_sender,
                completions,
                timestamp_period_nanos: f64::from(queue.get_timestamp_period()),
                timestamp_write_strategy,
                timestamp_marker,
                queries_per_frame,
                detailed_stages,
                next_slot: 0,
                enabled: false,
                latest_utilization: None,
                completed_profiles: VecDeque::with_capacity(SLOT_COUNT),
                stage_sample_count: 0,
                stage_nanos_sum: [0; STAGE_COUNT],
                stage_nanos_samples: std::array::from_fn(|_| {
                    Vec::with_capacity(STAGE_LOG_SAMPLE_CAPACITY as usize)
                }),
            }
        })
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) {
        if self.enabled && !enabled {
            self.log_stage_summary();
        }
        self.enabled = enabled;
        if !enabled {
            self.latest_utilization = None;
            self.completed_profiles.clear();
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
        if let Err(error) = device.poll(wgpu::PollType::Poll) {
            tracing::warn!(%error, "GPU timing poll failed");
        }
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
        let detailed_stages = self.detailed_stages;
        self.slots[slot].state = SlotState::Encoding { detailed_stages };
        // Apple tile GPUs can overlap an earlier pass's fragment tail with a
        // later pass's vertex head. Start at the first marker's beginning and
        // finish at the last marker's end so the pair remains ordered.
        self.write_timestamp(encoder, slot, 0, MarkerSamplePoint::BeforeMarker);
        Some(GpuFrameToken {
            slot,
            detailed_stages,
        })
    }

    pub(super) fn mark_stage(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        token: GpuFrameToken,
        stage: GpuFrameStage,
    ) {
        debug_assert_eq!(
            self.slots[token.slot].state,
            SlotState::Encoding {
                detailed_stages: token.detailed_stages
            }
        );
        if token.detailed_stages {
            self.write_timestamp(
                encoder,
                token.slot,
                stage as u32,
                MarkerSamplePoint::AfterMarker,
            );
        }
    }

    pub(super) fn finish_encoding(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        token: GpuFrameToken,
    ) {
        debug_assert_eq!(
            self.slots[token.slot].state,
            SlotState::Encoding {
                detailed_stages: token.detailed_stages
            }
        );
        let query_count = self.queries_per_frame;
        self.write_timestamp(
            encoder,
            token.slot,
            query_count - 1,
            MarkerSamplePoint::AfterMarker,
        );
        let resolve_offset = resolve_offset(token.slot);
        encoder.resolve_query_set(
            &self.query_sets[token.slot],
            zero_based_query_range(query_count),
            &self.resolve,
            resolve_offset,
        );
        encoder.copy_buffer_to_buffer(
            &self.resolve,
            resolve_offset,
            &self.slots[token.slot].readback,
            0,
            u64::from(query_count) * std::mem::size_of::<u64>() as u64,
        );
    }

    pub(super) fn after_submit(&mut self, token: GpuFrameToken, cpu_duration: Duration) {
        debug_assert_eq!(
            self.slots[token.slot].state,
            SlotState::Encoding {
                detailed_stages: token.detailed_stages
            }
        );
        self.slots[token.slot].state = SlotState::Mapping {
            cpu_duration_nanos: duration_nanos(cpu_duration),
            detailed_stages: token.detailed_stages,
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
        if matches!(self.slots[token.slot].state, SlotState::Encoding { .. }) {
            self.slots[token.slot].state = SlotState::Idle;
        }
    }

    fn write_timestamp(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        slot: usize,
        query_index: u32,
        marker_sample_point: MarkerSamplePoint,
    ) {
        match self.timestamp_write_strategy {
            TimestampWriteStrategy::Encoder => {
                encoder.write_timestamp(&self.query_sets[slot], query_index);
            }
            TimestampWriteStrategy::PassBoundary => {
                let marker = self
                    .timestamp_marker
                    .as_ref()
                    .expect("pass-boundary timestamps own a marker attachment");
                let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                    view: &marker.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Discard,
                    },
                })];
                let (beginning_of_pass_write_index, end_of_pass_write_index) =
                    match marker_sample_point {
                        MarkerSamplePoint::AfterMarker => (None, Some(query_index)),
                        MarkerSamplePoint::BeforeMarker => (Some(query_index), None),
                    };
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("bbb-frame-timestamp-boundary"),
                    color_attachments: &color_attachments,
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: Some(wgpu::RenderPassTimestampWrites {
                        query_set: &self.query_sets[slot],
                        beginning_of_pass_write_index,
                        end_of_pass_write_index,
                    }),
                    multiview_mask: None,
                });
            }
        }
    }

    pub(super) fn latest_utilization(&self) -> Option<f64> {
        self.latest_utilization
    }

    pub(super) fn take_completed_profile(&mut self) -> Option<RendererGpuTimingProfile> {
        self.completed_profiles.pop_front()
    }

    fn finish_mapping(&mut self, completion: MapCompletion) {
        let mut completed_profile = None;
        let mut stage_sample_for_log = None;
        let slot = &mut self.slots[completion.slot];
        let SlotState::Mapping {
            cpu_duration_nanos,
            detailed_stages,
        } = slot.state
        else {
            return;
        };
        if completion.result.is_ok() {
            if let Ok(mapped) = slot.readback.slice(..).get_mapped_range() {
                let query_bytes = self.queries_per_frame as usize * std::mem::size_of::<u64>();
                let timestamps = mapped
                    .get(..query_bytes)
                    .and_then(|bytes| bytemuck::try_cast_slice::<u8, u64>(bytes).ok());
                if self.enabled {
                    let end_index = if detailed_stages {
                        QUERIES_PER_FRAME as usize - 1
                    } else {
                        TOTAL_ONLY_QUERIES_PER_FRAME as usize - 1
                    };
                    let duration = timestamps.and_then(|timestamps| {
                        gpu_duration_from_timestamps(
                            [timestamps[0], timestamps[end_index]],
                            self.timestamp_period_nanos,
                        )
                    });
                    self.latest_utilization = duration.and_then(|duration| {
                        gpu_utilization_from_duration(duration, cpu_duration_nanos)
                    });
                    let stage_sample = detailed_stages
                        .then(|| {
                            timestamps
                                .and_then(|timestamps| timestamps.try_into().ok())
                                .and_then(|timestamps| {
                                    gpu_stage_nanos_from_timestamps(
                                        timestamps,
                                        self.timestamp_period_nanos,
                                    )
                                })
                        })
                        .flatten();
                    if let Some(duration) = duration {
                        completed_profile = Some(RendererGpuTimingProfile {
                            duration,
                            application_frame_interval: Duration::from_nanos(cpu_duration_nanos),
                            stages: stage_sample.map(RendererGpuStageProfile::from_nanos),
                        });
                    }
                    stage_sample_for_log = stage_sample;
                }
                drop(mapped);
            } else {
                self.latest_utilization = None;
            }
            slot.readback.unmap();
        } else {
            self.latest_utilization = None;
        }
        slot.state = SlotState::Idle;
        if let Some(stage_sample) = stage_sample_for_log {
            self.record_stage_sample(stage_sample);
        }
        if let Some(profile) = completed_profile {
            if self.completed_profiles.len() == SLOT_COUNT {
                self.completed_profiles.pop_front();
            }
            self.completed_profiles.push_back(profile);
        }
    }

    fn record_stage_sample(&mut self, stage_nanos: [u64; STAGE_COUNT]) {
        if self.stage_sample_count == STAGE_LOG_SAMPLE_CAPACITY {
            return;
        }
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
    }

    fn log_stage_summary(&mut self) {
        if self.stage_sample_count == 0 {
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
    if !adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
        return wgpu::Features::empty();
    }
    wgpu::Features::TIMESTAMP_QUERY
        | (adapter_features & wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
}

fn timestamp_write_strategy(device_features: wgpu::Features) -> Option<TimestampWriteStrategy> {
    device_features
        .contains(wgpu::Features::TIMESTAMP_QUERY)
        .then(|| {
            if device_features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS) {
                TimestampWriteStrategy::Encoder
            } else {
                TimestampWriteStrategy::PassBoundary
            }
        })
}

fn gpu_duration_from_timestamps(
    timestamps: [u64; 2],
    timestamp_period_nanos: f64,
) -> Option<Duration> {
    let ticks = timestamps[1].checked_sub(timestamps[0])?;
    if ticks == 0 || !timestamp_period_nanos.is_finite() || timestamp_period_nanos <= 0.0 {
        return None;
    }
    let nanos = ticks as f64 * timestamp_period_nanos;
    (nanos.is_finite() && nanos > 0.0 && nanos <= u64::MAX as f64)
        .then(|| Duration::from_nanos(nanos.round() as u64))
}

fn gpu_utilization_from_duration(duration: Duration, cpu_duration_nanos: u64) -> Option<f64> {
    if cpu_duration_nanos == 0 {
        return None;
    }
    let utilization = duration_nanos(duration) as f64 * 100.0 / cpu_duration_nanos as f64;
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

fn queries_per_frame(detailed_stages: bool) -> u32 {
    if detailed_stages {
        QUERIES_PER_FRAME
    } else {
        TOTAL_ONLY_QUERIES_PER_FRAME
    }
}

fn zero_based_query_range(query_count: u32) -> std::ops::Range<u32> {
    0..query_count
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
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .ok()?;
        let required_features = optional_gpu_timing_features(adapter.features());
        if !required_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("bbb-gpu-timing-test-device"),
            required_features,
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        }))
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
        assert_eq!(
            optional_gpu_timing_features(
                wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
                    | wgpu::Features::TEXTURE_COMPRESSION_BC
            ),
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
        );
        assert_eq!(timestamp_write_strategy(wgpu::Features::empty()), None);
        assert_eq!(
            timestamp_write_strategy(wgpu::Features::TIMESTAMP_QUERY),
            Some(TimestampWriteStrategy::PassBoundary)
        );
        assert_eq!(
            timestamp_write_strategy(
                wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
            ),
            Some(TimestampWriteStrategy::Encoder)
        );
    }

    #[test]
    fn gpu_timing_utilization_uses_real_query_delta_and_cpu_frame_duration() {
        assert_eq!(
            gpu_utilization_from_duration(Duration::from_nanos(400), 1_000),
            Some(40.0)
        );
        assert_eq!(
            gpu_utilization_from_duration(Duration::from_nanos(400), 0),
            None
        );
    }

    #[test]
    fn gpu_timing_duration_preserves_the_query_pair_without_cpu_frame_reconstruction() {
        assert_eq!(
            gpu_duration_from_timestamps([100, 300], 2.0),
            Some(Duration::from_nanos(400))
        );
        assert_eq!(gpu_duration_from_timestamps([300, 100], 2.0), None);
        assert_eq!(gpu_duration_from_timestamps([100, 300], f64::NAN), None);
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
    fn gpu_stage_profile_preserves_every_ordered_interval() {
        let profile = RendererGpuStageProfile::from_nanos([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        assert_eq!(profile.terrain_atlas_animation, Duration::from_nanos(1));
        assert_eq!(profile.opaque_world, Duration::from_nanos(2));
        assert_eq!(profile.opaque_particles, Duration::from_nanos(3));
        assert_eq!(profile.feature_depth_copies, Duration::from_nanos(4));
        assert_eq!(profile.entity_translucent_features, Duration::from_nanos(5));
        assert_eq!(profile.remaining_feature_targets, Duration::from_nanos(6));
        assert_eq!(profile.clouds, Duration::from_nanos(7));
        assert_eq!(profile.weather, Duration::from_nanos(8));
        assert_eq!(profile.transparency_combine, Duration::from_nanos(9));
        assert_eq!(profile.presentation_tail, Duration::from_nanos(10));
        assert_eq!(profile.encoder_tail, Duration::from_nanos(11));
    }

    #[test]
    fn gpu_timing_resolve_slots_satisfy_wgpu_alignment() {
        for slot in 0..SLOT_COUNT {
            assert_eq!(
                resolve_offset(slot) % wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT,
                0
            );
            assert!(
                u64::from(queries_per_frame(false)) * std::mem::size_of::<u64>() as u64
                    <= RESOLVE_SLOT_BYTES
            );
            assert!(
                u64::from(queries_per_frame(true)) * std::mem::size_of::<u64>() as u64
                    <= RESOLVE_SLOT_BYTES
            );
        }
    }

    #[test]
    fn gpu_timing_query_layout_is_two_for_formal_and_full_only_for_diagnostics() {
        assert_eq!(queries_per_frame(false), 2);
        assert_eq!(queries_per_frame(true), QUERIES_PER_FRAME);
    }

    #[test]
    fn gpu_timing_keeps_enough_zero_based_query_sets_for_async_readback() {
        assert_eq!(SLOT_COUNT, 8);
        assert_eq!(zero_based_query_range(queries_per_frame(false)), 0..2);
        assert_eq!(
            zero_based_query_range(queries_per_frame(true)),
            0..QUERIES_PER_FRAME
        );
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
        let mut timing = GpuFrameTiming::new_with_detailed_stages(&device, &queue, true)
            .expect("timestamp timing");
        assert_eq!(timing.query_sets.len(), SLOT_COUNT);
        assert_eq!(timing.queries_per_frame, QUERIES_PER_FRAME);
        assert_eq!(
            timing.timestamp_write_strategy,
            timestamp_write_strategy(device.features()).expect("device timestamp strategy")
        );
        timing.set_enabled(true);
        let timestamp_work = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bbb-gpu-timing-test-work"),
            // A tiny clear can begin and end within one Metal counter tick on
            // Apple GPUs. Keep the focused test workload large enough that all
            // eight independently resolved query pairs must be non-zero.
            size: 16 * 1024 * 1024,
            usage: wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
            encoder.clear_buffer(&timestamp_work, 0, None);
            timing.finish_encoding(&mut encoder, token);
            queue.submit(Some(encoder.finish()));
            timing.after_submit(token, Duration::from_millis(16));
        }

        let _ = device.poll(wgpu::PollType::wait_indefinitely());
        let _ = timing.poll(&device);
        assert!(timing
            .slots
            .iter()
            .all(|slot| slot.state == SlotState::Idle));
        let profiles = (0..SLOT_COUNT)
            .filter_map(|_| timing.take_completed_profile())
            .collect::<Vec<_>>();
        assert_eq!(profiles.len(), SLOT_COUNT);
        assert!(profiles
            .iter()
            .all(|profile| profile.duration > Duration::ZERO));
        assert!(profiles
            .iter()
            .all(|profile| profile.application_frame_interval == Duration::from_millis(16)));
        for profile in profiles {
            let stages = profile.stages.expect("detailed stage profile");
            let stage_total = [
                stages.terrain_atlas_animation,
                stages.opaque_world,
                stages.opaque_particles,
                stages.feature_depth_copies,
                stages.entity_translucent_features,
                stages.remaining_feature_targets,
                stages.clouds,
                stages.weather,
                stages.transparency_combine,
                stages.presentation_tail,
                stages.encoder_tail,
            ]
            .into_iter()
            .map(|duration| duration.as_nanos())
            .sum::<u128>();
            assert!(stage_total.abs_diff(profile.duration.as_nanos()) <= STAGE_COUNT as u128);
        }
    }
}
