//! Persistent, grow-only GPU buffers for geometry rebuilt every frame.
//!
//! Per-frame vertex/index data (particles, weather, HUD quads, ...) used to
//! allocate a fresh `create_buffer_init` buffer each frame. `FrameDataBuffer`
//! keeps one buffer alive across frames and re-uploads with
//! `Queue::write_buffer`, reallocating only when the data outgrows the
//! current capacity.

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
        if self.buffer.is_none() || self.capacity < size {
            let capacity = size.next_power_of_two().max(1024);
            self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: capacity,
                usage: self.usage,
                mapped_at_creation: false,
            }));
            self.capacity = capacity;
        }
        let buffer = self
            .buffer
            .as_ref()
            .expect("frame buffer allocated just above");
        queue.write_buffer(buffer, 0, bytes);
        true
    }

    pub(crate) fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }
}
