use std::{
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    time::Duration,
};

const SLOT_COUNT: usize = 3;
const QUERIES_PER_FRAME: u32 = 2;
const QUERY_BYTES: u64 = 16;

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
                    size: u64::from(query_count) * u64::from(std::mem::size_of::<u64>() as u32),
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
                }
            })
    }

    pub(super) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.latest_utilization = None;
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

    pub(super) fn finish_encoding(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        token: GpuFrameToken,
    ) {
        debug_assert_eq!(self.slots[token.slot].state, SlotState::Encoding);
        let start = query_index(token.slot);
        encoder.write_timestamp(&self.query_set, start + 1);
        let resolve_offset = u64::from(start) * std::mem::size_of::<u64>() as u64;
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
        let slot = &mut self.slots[completion.slot];
        let SlotState::Mapping { cpu_duration_nanos } = slot.state else {
            return;
        };
        if completion.result.is_ok() {
            let mapped = slot.readback.slice(..).get_mapped_range();
            let timestamps = mapped
                .get(..QUERY_BYTES as usize)
                .and_then(|bytes| bytemuck::try_cast_slice::<u8, u64>(bytes).ok())
                .and_then(|values| values.get(..2))
                .map(|values| [values[0], values[1]]);
            if self.enabled {
                self.latest_utilization = timestamps.and_then(|timestamps| {
                    gpu_utilization_from_timestamps(
                        timestamps,
                        self.timestamp_period_nanos,
                        cpu_duration_nanos,
                    )
                });
            }
            drop(mapped);
            slot.readback.unmap();
        } else {
            self.latest_utilization = None;
        }
        slot.state = SlotState::Idle;
    }
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

fn query_index(slot: usize) -> u32 {
    slot as u32 * QUERIES_PER_FRAME
}

fn duration_nanos(duration: Duration) -> u64 {
    duration.as_nanos().min(u128::from(u64::MAX)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
