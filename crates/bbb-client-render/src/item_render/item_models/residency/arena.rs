use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

use crate::item_render::frame_buffers::UploadBelt;

use super::{ItemModelAssetKey, ResidentItemModelGpu, MAX_CHANGED_INSTANCE_UPLOAD_RANGES};

pub(super) const LRU_NONE: u32 = u32::MAX;
pub(super) const MIN_ARENA_ELEMS: u32 = 1024;

pub(super) struct PendingArenaWrite {
    pub(super) first_elem: u32,
    pub(super) bytes: Vec<u8>,
}

impl PendingArenaWrite {
    pub(super) fn elem_count(&self, elem_size: usize) -> u32 {
        (self.bytes.len() / elem_size) as u32
    }

    pub(super) fn end_elem(&self, elem_size: usize) -> u32 {
        self.first_elem + self.elem_count(elem_size)
    }
}

pub(super) struct ItemArenaBuffer {
    pub(super) label: &'static str,
    pub(super) usage: wgpu::BufferUsages,
    pub(super) elem_size: usize,
    pub(super) max_elem: u32,
    pub(super) buffer: Option<wgpu::Buffer>,
    pub(super) capacity: u32,
    pub(super) used_end: u32,
    pub(super) preserved_elems: u32,
    pub(super) free_by_start: BTreeMap<u32, u32>,
    pub(super) free_by_len: BTreeSet<(u32, u32)>,
    pub(super) pending_writes: Vec<PendingArenaWrite>,
    pub(super) unsubmitted_writes: Vec<PendingArenaWrite>,
    pub(super) unsubmitted_old_buffer: Option<(wgpu::Buffer, u32)>,
    pub(super) retired_buffers: Vec<wgpu::Buffer>,
}

impl ItemArenaBuffer {
    pub(super) fn new(
        label: &'static str,
        usage: wgpu::BufferUsages,
        elem_size: usize,
        max_elem: u32,
    ) -> Self {
        Self {
            label,
            usage: usage | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            elem_size,
            max_elem,
            buffer: None,
            capacity: 0,
            used_end: 0,
            preserved_elems: 0,
            free_by_start: BTreeMap::new(),
            free_by_len: BTreeSet::new(),
            pending_writes: Vec::new(),
            unsubmitted_writes: Vec::new(),
            unsubmitted_old_buffer: None,
            retired_buffers: Vec::new(),
        }
    }

    pub(super) fn buffer(&self) -> Option<&wgpu::Buffer> {
        self.buffer.as_ref()
    }

    pub(super) fn allocate(&mut self, len: u32) -> Option<u32> {
        if len == 0 {
            return Some(0);
        }
        if let Some(&(block_len, start)) = self.free_by_len.range((len, 0)..).next() {
            self.free_by_len.remove(&(block_len, start));
            self.free_by_start.remove(&start);
            let rem = block_len - len;
            if rem > 0 {
                let rem_start = start + len;
                self.free_by_start.insert(rem_start, rem);
                self.free_by_len.insert((rem, rem_start));
            }
            return Some(start);
        }
        let end = self.used_end.checked_add(len)?;
        if end > self.max_elem {
            return None;
        }
        let start = self.used_end;
        self.used_end = end;
        Some(start)
    }

    pub(super) fn free(&mut self, mut start: u32, mut len: u32) {
        if len == 0 {
            return;
        }
        if let Some((&prev_start, &prev_len)) = self.free_by_start.range(..start).next_back() {
            if prev_start + prev_len == start {
                self.free_by_start.remove(&prev_start);
                self.free_by_len.remove(&(prev_len, prev_start));
                start = prev_start;
                len += prev_len;
            }
        }
        if let Some((&next_start, &next_len)) = self.free_by_start.range(start..).next() {
            if start + len == next_start {
                self.free_by_start.remove(&next_start);
                self.free_by_len.remove(&(next_len, next_start));
                len += next_len;
            }
        }
        if start + len == self.used_end {
            self.used_end = start;
            self.preserved_elems = self.preserved_elems.min(self.used_end);
        } else {
            self.free_by_start.insert(start, len);
            self.free_by_len.insert((len, start));
        }
    }

    pub(super) fn queue_write(&mut self, first_elem: u32, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let elem_size = self.elem_size;
        if let Some(last) = self.pending_writes.last_mut() {
            if last.end_elem(elem_size) == first_elem {
                last.bytes.extend_from_slice(bytes);
                return;
            }
        }
        self.pending_writes.push(PendingArenaWrite {
            first_elem,
            bytes: bytes.to_vec(),
        });
    }

    pub(super) fn stage(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        belt: &mut UploadBelt,
    ) {
        self.drain_retired_into(belt);
        if self.used_end == 0 {
            self.pending_writes.clear();
            return;
        }
        let elem_size = self.elem_size;
        if self.buffer.is_none() || self.capacity < self.used_end {
            let new_capacity = self
                .used_end
                .next_power_of_two()
                .max(MIN_ARENA_ELEMS)
                .min(self.max_elem);
            let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: (new_capacity as u64) * (elem_size as u64),
                usage: self.usage,
                mapped_at_creation: false,
            });
            if self.unsubmitted_old_buffer.is_some() {
                if let Some(intermediate) = self.buffer.take() {
                    belt.retire_buffer(intermediate);
                }
            } else if let Some(old_buffer) = self.buffer.take() {
                self.unsubmitted_old_buffer = Some((old_buffer, self.preserved_elems));
            }
            if let Some((committed_buf, committed_elems)) = &self.unsubmitted_old_buffer {
                let copy_elems = (*committed_elems).min(self.used_end);
                if copy_elems > 0 {
                    encoder.copy_buffer_to_buffer(
                        committed_buf,
                        0,
                        &new_buffer,
                        0,
                        (copy_elems as u64) * (elem_size as u64),
                    );
                }
            }
            for write in &self.unsubmitted_writes {
                if write.first_elem >= self.used_end {
                    continue;
                }
                let max_bytes = ((self.used_end - write.first_elem) as usize) * elem_size;
                let slice = &write.bytes[..write.bytes.len().min(max_bytes)];
                belt.write_bytes(
                    encoder,
                    &new_buffer,
                    (write.first_elem as u64) * (elem_size as u64),
                    slice,
                );
            }
            self.buffer = Some(new_buffer);
            self.capacity = new_capacity;
        }
        if self.pending_writes.is_empty() {
            return;
        }
        let buffer = self.buffer.as_ref().expect("item arena allocated above");
        for mut write in self.pending_writes.drain(..) {
            if write.first_elem >= self.used_end {
                continue;
            }
            let max_bytes = ((self.used_end - write.first_elem) as usize) * elem_size;
            if write.bytes.len() > max_bytes {
                write.bytes.truncate(max_bytes);
            }
            if write.bytes.is_empty() {
                continue;
            }
            belt.write_bytes(
                encoder,
                buffer,
                (write.first_elem as u64) * (elem_size as u64),
                &write.bytes,
            );
            if let Some(last) = self.unsubmitted_writes.last_mut() {
                if last.end_elem(elem_size) == write.first_elem {
                    last.bytes.extend_from_slice(&write.bytes);
                    continue;
                }
            }
            self.unsubmitted_writes.push(write);
        }
    }

    pub(super) fn mark_submitted(&mut self) {
        if let Some((old_buffer, _)) = self.unsubmitted_old_buffer.take() {
            self.retired_buffers.push(old_buffer);
        }
        self.unsubmitted_writes.clear();
        self.preserved_elems = self.used_end;
    }

    pub(super) fn restore_unsubmitted(&mut self) {
        if let Some((old_buffer, old_preserved)) = self.unsubmitted_old_buffer.take() {
            self.capacity = (old_buffer.size() / self.elem_size as u64) as u32;
            self.preserved_elems = old_preserved.min(self.used_end);
            if let Some(discarded) = self.buffer.replace(old_buffer) {
                self.retired_buffers.push(discarded);
            }
        }
        if !self.unsubmitted_writes.is_empty() {
            let mut restored = std::mem::take(&mut self.unsubmitted_writes);
            restored.append(&mut self.pending_writes);
            self.pending_writes = restored;
        }
    }

    pub(super) fn drain_retired_into(&mut self, belt: &mut UploadBelt) {
        for old in self.retired_buffers.drain(..) {
            belt.retire_buffer(old);
        }
    }

    pub(super) fn clear(&mut self) {
        self.free_by_start.clear();
        self.free_by_len.clear();
        self.used_end = 0;
        self.preserved_elems = 0;
        self.pending_writes.clear();
        self.unsubmitted_writes.clear();
        if let Some((old_buffer, _)) = self.unsubmitted_old_buffer.take() {
            self.retired_buffers.push(old_buffer);
        }
    }
}

pub(super) struct OwnerSlot {
    pub(super) key: ItemModelAssetKey,
    pub(super) gpu: ResidentItemModelGpu,
    pub(super) pinned_epoch: u64,
    pub(super) lru_prev: u32,
    pub(super) lru_next: u32,
}

pub(super) fn merge_byte_ranges(
    dst: &mut Vec<Range<usize>>,
    incoming: &[Range<usize>],
    max_len: usize,
) {
    dst.retain_mut(|range| {
        range.end = range.end.min(max_len);
        range.start < range.end
    });
    if incoming.is_empty() {
        return;
    }
    if dst.is_empty() {
        dst.extend_from_slice(incoming);
        return;
    }
    dst.extend_from_slice(incoming);
    dst.sort_unstable_by_key(|range| range.start);
    let mut write = 0usize;
    for read in 1..dst.len() {
        if dst[read].start <= dst[write].end {
            dst[write].end = dst[write].end.max(dst[read].end);
        } else {
            write += 1;
            dst[write] = dst[read].clone();
        }
    }
    dst.truncate(write + 1);
    if dst.len() > MAX_CHANGED_INSTANCE_UPLOAD_RANGES {
        let span = dst.first().unwrap().start..dst.last().unwrap().end;
        dst.clear();
        dst.push(span);
    }
}
