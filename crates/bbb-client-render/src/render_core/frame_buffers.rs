//! Persistent, grow-only GPU buffers for geometry rebuilt every frame.
//!
//! Per-frame vertex/index data (particles, weather, HUD quads, ...) used to
//! allocate a fresh `create_buffer_init` buffer each frame. `FrameDataBuffer`
//! keeps one buffer alive across frames and re-uploads with
//! `Queue::write_buffer`, reallocating only when the data outgrows the
//! current capacity.

use std::ops::Range;

/// Staging upload belt.
///
/// Desktop uses `wgpu::util::StagingBelt` (`MAP_WRITE` host-mapped chunks).
/// WebGPU (`wasm32`) avoids `MAP_WRITE` / `getMappedRange` / `mapAsync` completely:
/// in `wgpu`'s WebGPU backend, `StagingBelt` allocates cross-process Dawn IPC shared memory,
/// copies into temporary Rust `Vec<u8>`s on unmap, and ignores rejected `mapAsync` callbacks
/// (pushing unmapped `GPUBuffer`s back into `free_chunks`, which then panic in `getMappedRange`).
/// Instead, `wasm32` packs frame uploads into persistent device-local `COPY_DST | COPY_SRC`
/// scratch buffers flushed via `Queue::write_buffer` immediately before `Queue::submit`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct StagedWriteSlice<'a>(wgpu::BufferViewMut, std::marker::PhantomData<&'a mut [u8]>);

#[cfg(not(target_arch = "wasm32"))]
impl StagedWriteSlice<'_> {
    pub(crate) fn write_at(&mut self, offset: usize, bytes: &[u8]) {
        self.0
            .slice(offset..offset + bytes.len())
            .copy_from_slice(bytes);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct UploadBelt {
    inner: wgpu::util::StagingBelt,
}

#[cfg(not(target_arch = "wasm32"))]
impl UploadBelt {
    pub(crate) fn new(device: &wgpu::Device, chunk_size: u64) -> Self {
        Self {
            inner: wgpu::util::StagingBelt::new(device.clone(), chunk_size),
        }
    }

    pub(crate) fn allocate_write(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::Buffer,
        offset: u64,
        size: usize,
    ) -> Option<StagedWriteSlice<'_>> {
        let buf_size = wgpu::BufferSize::new(size as u64)?;
        Some(StagedWriteSlice(
            self.inner.write_buffer(encoder, target, offset, buf_size),
            std::marker::PhantomData,
        ))
    }

    pub(crate) fn write_bytes(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::Buffer,
        offset: u64,
        bytes: &[u8],
    ) {
        if let Some(mut slice) = self.allocate_write(encoder, target, offset, bytes.len()) {
            slice.write_at(0, bytes);
        }
    }

    pub(crate) fn finish(&mut self) {
        self.inner.finish();
    }

    pub(crate) fn recall(&mut self) {
        self.inner.recall();
    }

    pub(crate) fn finish_and_recall_on_submit(&mut self, encoder: &wgpu::CommandEncoder) {
        self.inner.finish_and_recall_on_submit(encoder);
    }

    pub(crate) fn flush_to_queue(&mut self, _queue: &wgpu::Queue) {}

    pub(crate) fn retire_submitted_buffers(&mut self, _queue: &wgpu::Queue) {}

    pub(crate) fn retire_buffer(&mut self, _buffer: wgpu::Buffer) {}

    pub(crate) fn discard(&mut self) {
        self.inner.finish();
        self.inner.recall();
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct StagedWriteSlice<'a>(&'a mut [u8]);

#[cfg(target_arch = "wasm32")]
impl StagedWriteSlice<'_> {
    pub(crate) fn write_at(&mut self, offset: usize, bytes: &[u8]) {
        self.0[offset..offset + bytes.len()].copy_from_slice(bytes);
    }
}

#[cfg(target_arch = "wasm32")]
const WASM_UPLOAD_BELT_RETAINED_CHUNKS: usize = 2;

#[cfg(target_arch = "wasm32")]
struct ScratchChunk {
    buffer: wgpu::Buffer,
    capacity: u64,
    data: Vec<u8>,
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct UploadBelt {
    device: wgpu::Device,
    chunk_size: u64,
    chunks: Vec<ScratchChunk>,
    retired_buffers: Vec<wgpu::Buffer>,
    retired_textures: Vec<wgpu::Texture>,
}

#[cfg(target_arch = "wasm32")]
impl UploadBelt {
    pub(crate) fn new(device: &wgpu::Device, chunk_size: u64) -> Self {
        Self {
            device: device.clone(),
            chunk_size: chunk_size.max(wgpu::COPY_BUFFER_ALIGNMENT),
            chunks: Vec::new(),
            retired_buffers: Vec::new(),
            retired_textures: Vec::new(),
        }
    }

    pub(crate) fn allocate_write(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::Buffer,
        offset: u64,
        size: usize,
    ) -> Option<StagedWriteSlice<'_>> {
        if size == 0 {
            return None;
        }
        debug_assert_eq!(offset % wgpu::COPY_BUFFER_ALIGNMENT, 0);
        debug_assert_eq!(size as u64 % wgpu::COPY_BUFFER_ALIGNMENT, 0);
        let copy_size = (size as u64).next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT);
        let align = wgpu::COPY_BUFFER_ALIGNMENT as usize;
        let chunk_index = self
            .chunks
            .iter()
            .position(|chunk| {
                let aligned_len = chunk.data.len().next_multiple_of(align) as u64;
                aligned_len + copy_size <= chunk.capacity
            })
            .unwrap_or_else(|| {
                let capacity = copy_size.max(self.chunk_size).next_power_of_two();
                let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("bbb-web-upload-belt"),
                    size: capacity,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });
                self.chunks.push(ScratchChunk {
                    buffer,
                    capacity,
                    data: Vec::with_capacity(copy_size.min(64 * 1024) as usize),
                });
                self.chunks.len() - 1
            });
        let chunk = &mut self.chunks[chunk_index];
        let src_offset = chunk.data.len().next_multiple_of(align);
        chunk.data.resize(src_offset + copy_size as usize, 0);
        encoder.copy_buffer_to_buffer(&chunk.buffer, src_offset as u64, target, offset, copy_size);
        Some(StagedWriteSlice(
            &mut chunk.data[src_offset..src_offset + size],
        ))
    }

    pub(crate) fn write_bytes(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::Buffer,
        offset: u64,
        bytes: &[u8],
    ) {
        if let Some(mut slice) = self.allocate_write(encoder, target, offset, bytes.len()) {
            slice.write_at(0, bytes);
        }
    }

    pub(crate) fn finish(&mut self) {}

    pub(crate) fn recall(&mut self) {}

    pub(crate) fn finish_and_recall_on_submit(&mut self, _encoder: &wgpu::CommandEncoder) {}

    pub(crate) fn flush_to_queue(&mut self, queue: &wgpu::Queue) {
        for chunk in &mut self.chunks {
            if !chunk.data.is_empty() {
                queue.write_buffer(&chunk.buffer, 0, &chunk.data);
                chunk.data.clear();
            }
        }
        self.trim_retained_chunks();
    }

    pub(crate) fn retire_submitted_buffers(&mut self, queue: &wgpu::Queue) {
        if !self.retired_buffers.is_empty() || !self.retired_textures.is_empty() {
            let retired_buffers = std::mem::take(&mut self.retired_buffers);
            let retired_textures = std::mem::take(&mut self.retired_textures);
            schedule_webgpu_retire(queue, retired_buffers, retired_textures);
        }
    }

    pub(crate) fn retire_buffer(&mut self, buffer: wgpu::Buffer) {
        self.retired_buffers.push(buffer);
    }

    pub(crate) fn retire_texture(&mut self, texture: wgpu::Texture) {
        self.retired_textures.push(texture);
    }

    pub(crate) fn discard(&mut self) {
        for chunk in &mut self.chunks {
            chunk.data.clear();
        }
        self.trim_retained_chunks();
    }

    fn trim_retained_chunks(&mut self) {
        if self.chunks.len() > WASM_UPLOAD_BELT_RETAINED_CHUNKS {
            for overflow in self.chunks.drain(WASM_UPLOAD_BELT_RETAINED_CHUNKS..) {
                self.retired_buffers.push(overflow.buffer);
            }
        }
        let max_retained_bytes = (self.chunk_size as usize).saturating_mul(2);
        for chunk in &mut self.chunks {
            if chunk.data.capacity() > max_retained_bytes {
                chunk.data.shrink_to(self.chunk_size as usize);
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn schedule_webgpu_retire(
    queue: &wgpu::Queue,
    buffers: Vec<wgpu::Buffer>,
    textures: Vec<wgpu::Texture>,
) {
    use std::{
        cell::RefCell,
        collections::BTreeMap,
        sync::atomic::{AtomicU64, Ordering},
    };

    struct RetiredBatch {
        buffers: Vec<wgpu::Buffer>,
        textures: Vec<wgpu::Texture>,
    }

    static NEXT_BATCH_ID: AtomicU64 = AtomicU64::new(1);
    thread_local! {
        static RETIRED_BATCHES: RefCell<BTreeMap<u64, RetiredBatch>> =
            const { RefCell::new(BTreeMap::new()) };
    }

    if buffers.is_empty() && textures.is_empty() {
        return;
    }
    let batch_id = NEXT_BATCH_ID.fetch_add(1, Ordering::Relaxed);
    RETIRED_BATCHES.with(|batches| {
        batches
            .borrow_mut()
            .insert(batch_id, RetiredBatch { buffers, textures });
    });
    queue.on_submitted_work_done(move || {
        let _ = RETIRED_BATCHES.try_with(|batches| {
            if let Some(batch) = batches.borrow_mut().remove(&batch_id) {
                for buffer in batch.buffers {
                    buffer.destroy();
                }
                for texture in batch.textures {
                    texture.destroy();
                }
            }
        });
    });
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static WASM_RENDER_QUEUE: std::cell::RefCell<Option<wgpu::Queue>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn set_wasm_queue(queue: &wgpu::Queue) {
    WASM_RENDER_QUEUE.with(|slot| {
        *slot.borrow_mut() = Some(queue.clone());
    });
}

/// Allocates and populates a GPU buffer without `mappedAtCreation` on `wasm32`.
///
/// Chromium's WebGPU implementation backs `mappedAtCreation: true` buffers with
/// bounded renderer-to-GPU shared memory that is not reclaimed synchronously
/// within a JS turn, throwing `RangeError` during atlas/item/cloud bursts.
pub(crate) fn create_initialized_buffer(
    device: &wgpu::Device,
    label: Option<&str>,
    contents: &[u8],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    #[cfg(target_arch = "wasm32")]
    {
        if contents.is_empty() {
            return device.create_buffer(&wgpu::BufferDescriptor {
                label,
                size: 0,
                usage,
                mapped_at_creation: false,
            });
        }
        if let Some(queue) = WASM_RENDER_QUEUE.with(|slot| slot.borrow().clone()) {
            let align = wgpu::COPY_BUFFER_ALIGNMENT as usize;
            let padded_size = (contents.len() as u64)
                .next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT)
                .max(wgpu::COPY_BUFFER_ALIGNMENT);
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label,
                size: padded_size,
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let aligned_len = (contents.len() / align) * align;
            if aligned_len > 0 {
                queue.write_buffer(&buffer, 0, &contents[..aligned_len]);
            }
            if aligned_len < contents.len() {
                let mut tail = [0u8; wgpu::COPY_BUFFER_ALIGNMENT as usize];
                tail[..contents.len() - aligned_len].copy_from_slice(&contents[aligned_len..]);
                queue.write_buffer(&buffer, aligned_len as u64, &tail);
            }
            return buffer;
        }
    }
    wgpu::util::DeviceExt::create_buffer_init(
        device,
        &wgpu::util::BufferInitDescriptor {
            label,
            contents,
            usage,
        },
    )
}

/// Defers `GPUBuffer.destroy()` on WebGPU until all already-submitted queue work completes,
/// since `wgpu`'s `Drop for WebBuffer` is a no-op and otherwise retains VRAM until JS GC.
pub(crate) fn retire_gpu_buffer(queue: &wgpu::Queue, buffer: wgpu::Buffer) {
    #[cfg(target_arch = "wasm32")]
    schedule_webgpu_retire(queue, vec![buffer], Vec::new());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (queue, buffer);
}

/// Defers `GPUTexture.destroy()` on WebGPU until all already-submitted queue work completes.
pub(crate) fn retire_gpu_textures<I>(queue: &wgpu::Queue, textures: I)
where
    I: IntoIterator<Item = wgpu::Texture>,
{
    #[cfg(target_arch = "wasm32")]
    schedule_webgpu_retire(queue, Vec::new(), textures.into_iter().collect());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (queue, textures);
}

/// Defers `GPUBuffer.destroy()` and `GPUTexture.destroy()` together on WebGPU until all
/// already-submitted queue work completes.
pub(crate) fn retire_gpu_resources(
    queue: &wgpu::Queue,
    buffers: Vec<wgpu::Buffer>,
    textures: Vec<wgpu::Texture>,
) {
    #[cfg(target_arch = "wasm32")]
    schedule_webgpu_retire(queue, buffers, textures);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (queue, buffers, textures);
}

pub(crate) struct StagedCommands {
    label: &'static str,
    staging: Option<UploadBelt>,
    encoder: Option<wgpu::CommandEncoder>,
    // Held separately from `staging` so mid-frame passes that temporarily `take_belt()`
    // can still queue replaced textures for post-submit retirement without panicking.
    #[cfg(target_arch = "wasm32")]
    retired_textures: Vec<wgpu::Texture>,
}

impl StagedCommands {
    pub(crate) fn new(device: &wgpu::Device, label: &'static str, chunk_size: u64) -> Self {
        Self {
            label,
            staging: Some(UploadBelt::new(device, chunk_size)),
            encoder: None,
            #[cfg(target_arch = "wasm32")]
            retired_textures: Vec::new(),
        }
    }

    pub(crate) fn record(
        &mut self,
        device: &wgpu::Device,
    ) -> (&mut wgpu::CommandEncoder, &mut UploadBelt) {
        let encoder = self.encoder.get_or_insert_with(|| {
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(self.label),
            })
        });
        (encoder, self.staging.as_mut().unwrap())
    }

    pub(crate) fn write(
        &mut self,
        device: &wgpu::Device,
        buffer: &wgpu::Buffer,
        offset: u64,
        bytes: &[u8],
    ) {
        if bytes.is_empty() {
            return;
        }
        let (encoder, staging) = self.record(device);
        staging.write_bytes(encoder, buffer, offset, bytes);
    }

    pub(crate) fn take_encoder(&mut self, device: &wgpu::Device) -> wgpu::CommandEncoder {
        self.encoder.take().unwrap_or_else(|| {
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(self.label),
            })
        })
    }

    pub(crate) fn has_pending_encoder(&self) -> bool {
        self.encoder.is_some()
    }

    pub(crate) fn belt(&mut self) -> &mut UploadBelt {
        self.staging.as_mut().unwrap()
    }

    pub(crate) fn take_belt(&mut self) -> UploadBelt {
        self.staging.take().unwrap()
    }

    pub(crate) fn restore_belt(&mut self, belt: UploadBelt) {
        self.staging = Some(belt);
    }

    pub(crate) fn finish_belt(&mut self) {
        self.staging.as_mut().unwrap().finish();
    }

    pub(crate) fn recall_belt(&mut self) {
        self.staging.as_mut().unwrap().recall();
    }

    pub(crate) fn flush_before_external_submit(&mut self, queue: &wgpu::Queue) {
        self.staging.as_mut().unwrap().flush_to_queue(queue);
    }

    pub(crate) fn retire_texture(&mut self, texture: wgpu::Texture) {
        #[cfg(target_arch = "wasm32")]
        self.retired_textures.push(texture);
        #[cfg(not(target_arch = "wasm32"))]
        let _ = texture;
    }

    pub(crate) fn retire_after_external_submit(&mut self, queue: &wgpu::Queue) {
        let staging = self.staging.as_mut().unwrap();
        #[cfg(target_arch = "wasm32")]
        for texture in self.retired_textures.drain(..) {
            staging.retire_texture(texture);
        }
        staging.retire_submitted_buffers(queue);
    }

    pub(crate) fn discard(&mut self) {
        self.encoder = None;
        self.staging.as_mut().unwrap().discard();
    }

    pub(crate) fn submit(&mut self, queue: &wgpu::Queue) {
        let staging = self.staging.as_mut().unwrap();
        if let Some(encoder) = self.encoder.take() {
            staging.flush_to_queue(queue);
            staging.finish_and_recall_on_submit(&encoder);
            queue.submit(Some(encoder.finish()));
        }
        #[cfg(target_arch = "wasm32")]
        for texture in self.retired_textures.drain(..) {
            staging.retire_texture(texture);
        }
        staging.retire_submitted_buffers(queue);
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameDataUpload {
    pub bytes: u64,
    pub writes: u64,
    pub buffer_replaced: bool,
}

pub struct FrameDataBuffer {
    label: &'static str,
    usage: wgpu::BufferUsages,
    buffer: Option<wgpu::Buffer>,
    capacity: u64,
}

impl FrameDataBuffer {
    pub fn vertex(label: &'static str) -> Self {
        Self::new(label, wgpu::BufferUsages::VERTEX)
    }

    pub(crate) fn take_buffer(&mut self) -> Option<wgpu::Buffer> {
        self.capacity = 0;
        self.buffer.take()
    }

    pub fn index(label: &'static str) -> Self {
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
    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, bytes: &[u8]) -> bool {
        if bytes.is_empty() {
            return false;
        }
        // `Queue::write_buffer` requires the data length to be a multiple of
        // COPY_BUFFER_ALIGNMENT; all frame geometry is f32/u32 based.
        debug_assert_eq!(bytes.len() as u64 % wgpu::COPY_BUFFER_ALIGNMENT, 0);
        let size = bytes.len() as u64;
        let (_, retired) = self.ensure_capacity(device, size);
        if let Some(old) = retired {
            retire_gpu_buffer(queue, old);
        }
        let buffer = self
            .buffer
            .as_ref()
            .expect("frame buffer allocated just above");
        queue.write_buffer(buffer, 0, bytes);
        true
    }

    pub(crate) fn upload_staged(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging: &mut UploadBelt,
        bytes: &[u8],
    ) -> bool {
        if bytes.is_empty() {
            return false;
        }
        let size = bytes.len() as u64;
        debug_assert_eq!(size % wgpu::COPY_BUFFER_ALIGNMENT, 0);
        let (_, retired) = self.ensure_capacity(device, size);
        if let Some(old) = retired {
            staging.retire_buffer(old);
        }
        let buffer = self
            .buffer
            .as_ref()
            .expect("frame buffer allocated just above");
        staging.write_bytes(encoder, buffer, 0, bytes);
        true
    }

    pub(crate) fn upload_staged_commands(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut StagedCommands,
        bytes: &[u8],
    ) -> bool {
        if bytes.is_empty() {
            return false;
        }
        let (encoder, staging) = uploads.record(device);
        self.upload_staged(device, encoder, staging, bytes)
    }

    pub(crate) fn upload_changed(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        previous: &[u8],
    ) {
        if bytes.is_empty() {
            return;
        }
        let (replaced, retired) = self.ensure_capacity(device, bytes.len() as u64);
        if let Some(old) = retired {
            retire_gpu_buffer(queue, old);
        }
        write_changed(
            queue,
            self.buffer.as_ref().expect("allocated changed-data buffer"),
            0,
            bytes,
            if replaced { &[] } else { previous },
        );
    }

    pub(crate) fn upload_changed_staged_with_encoder(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging: &mut UploadBelt,
        bytes: &[u8],
        previous: &[u8],
    ) {
        if bytes.is_empty() {
            return;
        }
        let (replaced, retired) = self.ensure_capacity(device, bytes.len() as u64);
        if let Some(old) = retired {
            staging.retire_buffer(old);
        }
        let buffer = self.buffer.as_ref().expect("allocated changed-data buffer");
        let prev = if replaced { &[] } else { previous };
        for_each_changed_range(bytes, prev, |range| {
            staging.write_bytes(encoder, buffer, range.start as u64, &bytes[range]);
        });
    }

    pub(crate) fn upload_changed_staged(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut StagedCommands,
        bytes: &[u8],
        previous: &[u8],
    ) {
        if bytes.is_empty() {
            return;
        }
        let (replaced, retired) = self.ensure_capacity(device, bytes.len() as u64);
        if let Some(old) = retired {
            uploads.belt().retire_buffer(old);
        }
        write_changed_staged(
            device,
            uploads,
            self.buffer.as_ref().expect("allocated changed-data buffer"),
            0,
            bytes,
            if replaced { &[] } else { previous },
        );
    }

    /// Uploads only the ordered, non-overlapping changed byte ranges when the
    /// persistent buffer already fits `bytes`. A growth replaces the buffer
    /// and uploads the complete payload so untouched ranges remain defined.
    pub fn upload_changed_ranges(
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
        let (buffer_replaced, retired) = self.ensure_capacity(device, bytes.len() as u64);
        if let Some(old) = retired {
            retire_gpu_buffer(queue, old);
        }
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

    pub(crate) fn prepare_changed_ranges(
        &mut self,
        device: &wgpu::Device,
        bytes_len: usize,
        changed_ranges: &[Range<usize>],
    ) -> (FrameDataUpload, Option<wgpu::Buffer>) {
        if bytes_len == 0 {
            return (FrameDataUpload::default(), None);
        }
        debug_assert_eq!(bytes_len as u64 % wgpu::COPY_BUFFER_ALIGNMENT, 0);
        let (buffer_replaced, retired) = self.ensure_capacity(device, bytes_len as u64);
        if buffer_replaced {
            return (
                FrameDataUpload {
                    bytes: bytes_len as u64,
                    writes: 1,
                    buffer_replaced: true,
                },
                retired,
            );
        }
        let mut uploaded = FrameDataUpload::default();
        let mut previous_end = 0;
        for range in changed_ranges {
            debug_assert!(range.start < range.end);
            debug_assert!(range.start >= previous_end);
            debug_assert!(range.end <= bytes_len);
            debug_assert_eq!(range.start as u64 % wgpu::COPY_BUFFER_ALIGNMENT, 0);
            debug_assert_eq!(range.len() as u64 % wgpu::COPY_BUFFER_ALIGNMENT, 0);
            uploaded.bytes = uploaded.bytes.saturating_add(range.len() as u64);
            uploaded.writes = uploaded.writes.saturating_add(1);
            previous_end = range.end;
        }
        (uploaded, None)
    }

    fn ensure_capacity(
        &mut self,
        device: &wgpu::Device,
        size: u64,
    ) -> (bool, Option<wgpu::Buffer>) {
        if self.buffer.is_some() && self.capacity >= size {
            return (false, None);
        }
        let capacity = size.next_power_of_two().max(1024);
        let old = self
            .buffer
            .replace(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: capacity,
                usage: self.usage,
                mapped_at_creation: false,
            }));
        self.capacity = capacity;
        (true, old)
    }

    pub fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub fn capacity(&self) -> u64 {
        self.capacity
    }
}

fn for_each_changed_range(next: &[u8], previous: &[u8], mut emit: impl FnMut(Range<usize>)) {
    const BLOCK: usize = 256;
    const COALESCE_GAP: usize = 2048;
    let mut start = 0;
    while start < next.len() {
        let block_end = (start + BLOCK).min(next.len());
        if previous.get(start..block_end) == Some(&next[start..block_end]) {
            start = block_end;
            continue;
        }
        let mut dirty_end = block_end;
        let mut cursor = block_end;
        while cursor < next.len() {
            let next_end = (cursor + BLOCK).min(next.len());
            if previous.get(cursor..next_end) != Some(&next[cursor..next_end]) {
                dirty_end = next_end;
                cursor = next_end;
            } else if cursor + BLOCK - dirty_end <= COALESCE_GAP {
                cursor = next_end;
            } else {
                break;
            }
        }
        emit(start..dirty_end);
        start = cursor;
    }
}

pub(crate) fn write_changed(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    offset: u64,
    next: &[u8],
    previous: &[u8],
) {
    for_each_changed_range(next, previous, |range| {
        queue.write_buffer(buffer, offset + range.start as u64, &next[range]);
    });
}

pub(crate) fn write_changed_staged(
    device: &wgpu::Device,
    uploads: &mut StagedCommands,
    buffer: &wgpu::Buffer,
    offset: u64,
    next: &[u8],
    previous: &[u8],
) {
    for_each_changed_range(next, previous, |range| {
        uploads.write(device, buffer, offset + range.start as u64, &next[range]);
    });
}
