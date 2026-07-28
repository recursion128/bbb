//! Persistent, grow-only GPU buffers for geometry rebuilt every frame.
//!
//! Per-frame vertex/index data (particles, weather, HUD quads, ...) used to
//! allocate a fresh `create_buffer_init` buffer each frame. `FrameDataBuffer`
//! keeps one buffer alive across frames and re-uploads with
//! `Queue::write_buffer`, reallocating only when the data outgrows the
//! current capacity.

use std::ops::Range;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FrameDataUpload {
    pub(crate) bytes: u64,
    pub(crate) writes: u64,
    pub(crate) buffer_replaced: bool,
}

pub(crate) struct FrameDataBuffer {
    label: &'static str,
    usage: wgpu::BufferUsages,
    buffer: Option<wgpu::Buffer>,
    capacity: u64,
}

impl FrameDataBuffer {
    pub(crate) fn vertex(label: &'static str) -> Self {
        Self::new(label, wgpu::BufferUsages::VERTEX)
    }

    pub(crate) fn index(label: &'static str) -> Self {
        Self::new(label, wgpu::BufferUsages::INDEX)
    }

    fn new(label: &'static str, usage: wgpu::BufferUsages) -> Self {
        Self {
            label,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            buffer: None,
            capacity: 0,
        }
    }

    /// Uploads `bytes` into the persistent buffer, growing it when needed.
    /// Returns false (and leaves any previous buffer untouched) for empty
    /// input; callers must gate their draw on this result rather than on
    /// `buffer()` alone, which still exposes last frame's buffer.
    pub(crate) fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> bool {
        if bytes.is_empty() {
            return false;
        }
        // `Queue::write_buffer` requires the data length to be a multiple of
        // COPY_BUFFER_ALIGNMENT; all frame geometry is f32/u32 based.
        debug_assert_eq!(bytes.len() as u64 % wgpu::COPY_BUFFER_ALIGNMENT, 0);
        let size = bytes.len() as u64;
        self.ensure_capacity(device, size);
        let buffer = self
            .buffer
            .as_ref()
            .expect("frame buffer allocated just above");
        queue.write_buffer(buffer, 0, bytes);
        true
    }

    /// Uploads only the ordered, non-overlapping changed byte ranges when the
    /// persistent buffer already fits `bytes`. A growth replaces the buffer
    /// and uploads the complete payload so untouched ranges remain defined.
    pub(crate) fn upload_changed_ranges(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        changed_ranges: &[Range<usize>],
    ) -> FrameDataUpload {
        if bytes.is_empty() {
            return FrameDataUpload::default();
        }
        debug_assert_eq!(bytes.len() as u64 % wgpu::COPY_BUFFER_ALIGNMENT, 0);
        let buffer_replaced = self.ensure_capacity(device, bytes.len() as u64);
        let buffer = self
            .buffer
            .as_ref()
            .expect("frame buffer allocated just above");
        if buffer_replaced {
            queue.write_buffer(buffer, 0, bytes);
            return FrameDataUpload {
                bytes: bytes.len() as u64,
                writes: 1,
                buffer_replaced: true,
            };
        }

        let mut uploaded = FrameDataUpload::default();
        let mut previous_end = 0;
        for range in changed_ranges {
            debug_assert!(range.start < range.end);
            debug_assert!(range.start >= previous_end);
            debug_assert!(range.end <= bytes.len());
            debug_assert_eq!(
                range.start as u64 % wgpu::COPY_BUFFER_ALIGNMENT,
                0,
                "changed upload offset must satisfy wgpu alignment"
            );
            debug_assert_eq!(
                range.len() as u64 % wgpu::COPY_BUFFER_ALIGNMENT,
                0,
                "changed upload size must satisfy wgpu alignment"
            );
            queue.write_buffer(buffer, range.start as u64, &bytes[range.start..range.end]);
            uploaded.bytes = uploaded.bytes.saturating_add(range.len() as u64);
            uploaded.writes = uploaded.writes.saturating_add(1);
            previous_end = range.end;
        }
        uploaded
    }

    fn ensure_capacity(&mut self, device: &wgpu::Device, size: u64) -> bool {
        if self.buffer.is_some() && self.capacity >= size {
            return false;
        }
        let capacity = size.next_power_of_two().max(1024);
        self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(self.label),
            size: capacity,
            usage: self.usage,
            mapped_at_creation: false,
        }));
        self.capacity = capacity;
        true
    }

    pub(crate) fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub(crate) fn capacity(&self) -> u64 {
        self.capacity
    }
}
