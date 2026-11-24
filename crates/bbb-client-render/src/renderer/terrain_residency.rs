//! Terrain GPU residency: paged vertex and index arenas per chunk layer, one slot per section
//! owner (its instance index into the fade buffer) and the per-layer draw lists the world passes
//! replay.

use super::*;
use crate::terrain::sequential_quad_indices;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Bound::{Excluded, Unbounded};
use wgpu::util::DrawIndexedIndirectArgs;

mod slots;
use slots::TerrainSlots;

const TERRAIN_LAYER_COUNT: usize = 3;
const TRANSLUCENT: usize = TerrainLayer::Translucent as usize;
pub(super) const TERRAIN_PAGE_BYTES: usize = if cfg!(target_arch = "wasm32") {
    16 << 20
} else {
    32 << 20
};
const VERTEX_ARENA_MIN_BYTES: usize = if cfg!(target_arch = "wasm32") {
    512 << 10
} else {
    2 << 20
};
const INDEX_ARENA_MIN_CAPACITY: u32 = if cfg!(target_arch = "wasm32") {
    1 << 15
} else {
    1 << 17
};
const FADE_BUFFER_MIN_CAPACITY: usize = if cfg!(target_arch = "wasm32") {
    256
} else {
    1 << 10
};
// In wgpu 30, MULTI_DRAW_INDIRECT_COUNT implies RenderPass::multi_draw_indexed_indirect is
// backed by hardware multi-draw indirect rather than emulated per-draw indirect loops.
pub(crate) const TERRAIN_MDI_FEATURES: wgpu::Features =
    wgpu::Features::INDIRECT_FIRST_INSTANCE.union(wgpu::Features::MULTI_DRAW_INDIRECT_COUNT);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerrainLayer {
    Solid,
    Cutout,
    Translucent,
}

/// A growable GPU buffer sub-allocated in whole elements, so a vertex offset is a valid
/// `base_vertex` and every byte offset stays 4-aligned for `write_buffer` and buffer copies.
struct GpuArena {
    label: &'static str,
    usage: wgpu::BufferUsages,
    element_size: u64,
    min_capacity: u32,
    /// Page capacity in elements.
    limit: u32,
    buffer: Option<wgpu::Buffer>,
    capacity: u32,
    end: u32,
    /// Coalesced free ranges `start -> len`, all below `end`, for O(log F) neighbor coalescing.
    free_by_start: BTreeMap<u32, u32>,
    /// Mirror index `(len, start)` for O(log F) best-fit lookup and `fits()` checks.
    free_by_len: BTreeSet<(u32, u32)>,
    /// Exact sum of all free-range lengths for O(1) `can_pack()`.
    free_total: u32,
}

impl GpuArena {
    fn new(
        label: &'static str,
        usage: wgpu::BufferUsages,
        element_size: usize,
        min_capacity: u32,
        limit: u32,
    ) -> Self {
        Self {
            label,
            usage,
            element_size: element_size as u64,
            min_capacity,
            limit,
            buffer: None,
            capacity: 0,
            end: 0,
            free_by_start: BTreeMap::new(),
            free_by_len: BTreeSet::new(),
            free_total: 0,
        }
    }

    fn fits(&self, count: u32) -> bool {
        count <= self.limit - self.end || self.free_by_len.range((count, 0)..).next().is_some()
    }

    fn can_pack(&self, count: u32) -> bool {
        count <= (self.limit - self.end).saturating_add(self.free_total)
    }

    fn allocate(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        retired: &mut Vec<wgpu::Buffer>,
        len: u32,
    ) -> Option<u32> {
        if let Some(&(free, start)) = self.free_by_len.range((len, 0)..).next() {
            self.free_by_len.remove(&(free, start));
            self.free_by_start.remove(&start);
            self.free_total = self.free_total.saturating_sub(len);
            if free > len {
                let rem_start = start + len;
                let rem_len = free - len;
                self.free_by_start.insert(rem_start, rem_len);
                self.free_by_len.insert((rem_len, rem_start));
            }
            return Some(start);
        }
        let start = self.end;
        let end = start.checked_add(len)?;
        if end > self.capacity {
            self.grow(device, encoder, retired, end)?;
        }
        self.end = end;
        Some(start)
    }

    fn release(&mut self, start: u32, len: u32, retired: &mut Vec<wgpu::Buffer>) {
        if len == 0 {
            return;
        }
        let (mut start, mut end) = (start, start.saturating_add(len));
        if let Some((&prev_start, &prev_len)) = self.free_by_start.range(..start).next_back() {
            if prev_start.saturating_add(prev_len) == start {
                self.free_by_start.remove(&prev_start);
                self.free_by_len.remove(&(prev_len, prev_start));
                self.free_total = self.free_total.saturating_sub(prev_len);
                start = prev_start;
            }
        }
        if let Some(&next_len) = self.free_by_start.get(&end) {
            self.free_by_start.remove(&end);
            self.free_by_len.remove(&(next_len, end));
            self.free_total = self.free_total.saturating_sub(next_len);
            end = end.saturating_add(next_len);
        }
        if end == self.end {
            self.end = start;
        } else {
            let merged_len = end - start;
            self.free_by_start.insert(start, merged_len);
            self.free_by_len.insert((merged_len, start));
            self.free_total = self.free_total.saturating_add(merged_len);
        }
        if self.end == 0 {
            retired.extend(self.buffer.take());
            self.capacity = 0;
            self.free_by_start.clear();
            self.free_by_len.clear();
            self.free_total = 0;
        }
    }

    fn grow(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        retired: &mut Vec<wgpu::Buffer>,
        needed: u32,
    ) -> Option<()> {
        if needed > self.limit {
            return None;
        }
        // Pages have byte limits. Doubling 16-byte vertex counts leaves a final partial growth.
        let bytes = (u64::from(needed) * self.element_size).next_power_of_two();
        let capacity = (bytes / self.element_size)
            .max(u64::from(self.min_capacity))
            .min(u64::from(self.limit)) as u32;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(self.label),
            size: u64::from(capacity) * self.element_size,
            usage: self.usage | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        if let Some(old) = self.buffer.as_ref().filter(|_| self.end > 0) {
            let mut live_start = 0;
            for (start, len) in self
                .free_by_start
                .iter()
                .map(|(&start, &len)| (start, len))
                .chain(std::iter::once((self.end, 0)))
            {
                let live = start - live_start;
                if live != 0 {
                    let byte_offset = u64::from(live_start) * self.element_size;
                    encoder.copy_buffer_to_buffer(
                        old,
                        byte_offset,
                        &buffer,
                        byte_offset,
                        u64::from(live) * self.element_size,
                    );
                }
                live_start = start + len;
            }
        }
        retired.extend(self.buffer.replace(buffer));
        self.capacity = capacity;
        Some(())
    }

    /// Packs the live ranges to the front of a fresh buffer of the same capacity and returns a
    /// prefix-sum remap table for O(M + N log M) resident offset updates.
    fn compact(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        retired: &mut Vec<wgpu::Buffer>,
    ) -> CompactionMap {
        let free = std::mem::take(&mut self.free_by_start);
        self.free_by_len.clear();
        self.free_total = 0;
        let map = CompactionMap::from_sorted_free_ranges(free.iter().map(|(&s, &l)| (s, l)));
        let Some(old) = self.buffer.take() else {
            return map;
        };
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(self.label),
            size: u64::from(self.capacity) * self.element_size,
            usage: self.usage | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let (mut live_start, mut packed) = (0, 0);
        for (start, len) in free
            .iter()
            .map(|(&start, &len)| (start, len))
            .chain(std::iter::once((self.end, 0)))
        {
            let live = start - live_start;
            if live != 0 {
                encoder.copy_buffer_to_buffer(
                    &old,
                    u64::from(live_start) * self.element_size,
                    &buffer,
                    u64::from(packed) * self.element_size,
                    u64::from(live) * self.element_size,
                );
            }
            packed += live;
            live_start = start + len;
        }
        retired.push(old);
        self.buffer = Some(buffer);
        self.end = packed;
        map
    }

    fn stage<T: bytemuck::Pod>(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        staging: &mut crate::frame_buffers::UploadBelt,
        start: u32,
        data: &[T],
    ) {
        let Some(buffer) = self.buffer.as_ref() else {
            debug_assert!(false, "GpuArena::stage called without an allocated buffer");
            return;
        };
        staging.write_bytes(
            encoder,
            buffer,
            u64::from(start) * self.element_size,
            bytemuck::cast_slice(data),
        );
    }

    fn used_slice(&self) -> Option<wgpu::BufferSlice<'_>> {
        // Binding the unused tail makes wgpu initialize it on the page's first draw.
        self.buffer
            .as_ref()
            .map(|buffer| buffer.slice(..u64::from(self.end) * self.element_size))
    }
}

/// Prefix-sum compaction index mapping pre-compaction element offsets to post-compaction offsets
/// in O(log F) per lookup (O(F + N log F) total instead of O(N * F)).
struct CompactionMap {
    /// `(free_range_start, cumulative_removed_elements)` sorted strictly ascending by start.
    prefix: Vec<(u32, u32)>,
}

impl CompactionMap {
    fn from_sorted_free_ranges(squeezed: impl IntoIterator<Item = (u32, u32)>) -> Self {
        let iter = squeezed.into_iter();
        let mut prefix = Vec::with_capacity(iter.size_hint().0);
        let mut removed = 0u32;
        for (start, len) in iter {
            removed = removed.saturating_add(len);
            prefix.push((start, removed));
        }
        Self { prefix }
    }

    fn remap(&self, offset: u32) -> u32 {
        let idx = self.prefix.partition_point(|&(start, _)| start < offset);
        if idx == 0 {
            offset
        } else {
            offset.saturating_sub(self.prefix[idx - 1].1)
        }
    }
}

/// Per-slot section origin and fade, bound as per-instance vertex attributes so every section draw
/// reads its exact integer block origin and `ChunkVisibility` without a per-section bind group.
#[derive(Default)]
struct TerrainFadeBuffer {
    buffer: Option<wgpu::Buffer>,
    values: Vec<crate::gpu::TerrainSectionInstance>,
    dirty: std::ops::Range<usize>,
}

impl TerrainFadeBuffer {
    fn ensure_slot(&mut self, index: usize) {
        if index >= self.values.len() {
            let capacity = (index + 1)
                .next_power_of_two()
                .max(FADE_BUFFER_MIN_CAPACITY);
            self.values.resize(
                capacity,
                crate::gpu::TerrainSectionInstance {
                    origin_blocks: [0; 3],
                    visibility: 1.0,
                },
            );
        }
    }

    fn mark_dirty(&mut self, index: usize) {
        self.dirty = if self.dirty.is_empty() {
            index..index + 1
        } else {
            self.dirty.start.min(index)..self.dirty.end.max(index + 1)
        };
    }

    fn set_slot(&mut self, slot: u32, origin_blocks: [i32; 3], visibility: f32) {
        let index = slot as usize;
        self.ensure_slot(index);
        let entry = &mut self.values[index];
        if entry.origin_blocks == origin_blocks
            && entry.visibility.to_bits() == visibility.to_bits()
        {
            return;
        }
        entry.origin_blocks = origin_blocks;
        entry.visibility = visibility;
        self.mark_dirty(index);
    }

    fn set(&mut self, slot: u32, value: f32) {
        let index = slot as usize;
        self.ensure_slot(index);
        if self.values[index].visibility.to_bits() == value.to_bits() {
            return;
        }
        self.values[index].visibility = value;
        self.mark_dirty(index);
    }

    fn flush(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging: &mut crate::frame_buffers::UploadBelt,
    ) -> bool {
        let dirty = std::mem::take(&mut self.dirty);
        let bytes = bytemuck::cast_slice(&self.values);
        if bytes.len() as u64 > self.buffer.as_ref().map_or(0, wgpu::Buffer::size) {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("bbb-terrain-section-fade-buffer"),
                size: (bytes.len() as u64).max(wgpu::COPY_BUFFER_ALIGNMENT),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            staging.write_bytes(encoder, &buffer, 0, bytes);
            if let Some(old) = self.buffer.replace(buffer) {
                staging.retire_buffer(old);
            }
            return true;
        }
        if !dirty.is_empty() {
            if let Some(buffer) = self.buffer.as_ref() {
                let slice: &[u8] = bytemuck::cast_slice(&self.values[dirty.clone()]);
                staging.write_bytes(
                    encoder,
                    buffer,
                    (dirty.start * size_of::<crate::gpu::TerrainSectionInstance>()) as u64,
                    slice,
                );
            }
        }
        false
    }
}

fn owner_origin_blocks(owner: TerrainSectionMeshOwner) -> [i32; 3] {
    [
        owner.chunk.chunk_x.saturating_mul(16),
        owner.section_y.saturating_mul(16),
        owner.chunk.chunk_z.saturating_mul(16),
    ]
}

fn local_camera_for_owner(camera: [f32; 3], owner: TerrainSectionMeshOwner) -> [f32; 3] {
    let origin = owner_origin_blocks(owner);
    std::array::from_fn(|axis| (f64::from(camera[axis]) - f64::from(origin[axis])) as f32)
}

#[derive(Clone, PartialEq, Eq)]
struct TerrainRunBufferKey {
    page: usize,
    range: std::ops::Range<usize>,
    index_u16: bool,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
}

/// Non-MDI fallback frames a terrain draw revision must remain unchanged before
/// compiling cached `wgpu::RenderBundle`s, avoiding per-frame bundle churn while turning.
const TERRAIN_BUNDLE_STABLE_FRAMES: u32 = 3;

struct CachedTerrainLayerBundle {
    draws_revision: u64,
    color_format: wgpu::TextureFormat,
    sample_count: u32,
    pipeline: wgpu::RenderPipeline,
    bind_groups: [wgpu::BindGroup; 2],
    fade_buffer: wgpu::Buffer,
    runs: Vec<TerrainRunBufferKey>,
    bundle: wgpu::RenderBundle,
}

#[derive(Default)]
struct TerrainBundleCacheState {
    observed_revision: Option<u64>,
    stable_frames: u32,
    bundle: Option<CachedTerrainLayerBundle>,
}

impl TerrainBundleCacheState {
    fn invalidate(&mut self) {
        self.bundle = None;
        self.stable_frames = 0;
    }

    fn advance(&mut self, draws_revision: u64) -> bool {
        if self.observed_revision != Some(draws_revision) {
            self.observed_revision = Some(draws_revision);
            self.stable_frames = 1;
            self.bundle = None;
        } else {
            self.stable_frames = self.stable_frames.saturating_add(1);
        }
        self.stable_frames >= TERRAIN_BUNDLE_STABLE_FRAMES
    }
}

struct TerrainPage {
    vertices: GpuArena,
    indices: GpuArena,
    shared_quads: bool,
    index_u16: bool,
    opaque_draws: Vec<DrawIndexedIndirectArgs>,
    last_draw: usize,
}

impl TerrainPage {
    fn index_format(&self) -> wgpu::IndexFormat {
        if self.index_u16 {
            wgpu::IndexFormat::Uint16
        } else {
            wgpu::IndexFormat::Uint32
        }
    }
}

struct TerrainLayerArena {
    pages: Vec<TerrainPage>,
    active_page: usize,
    page_size: (u32, u32),
    limits: (u32, u32),
    live: (u32, u32),
    vertex_label: &'static str,
    index_label: &'static str,
    draws: Vec<DrawIndexedIndirectArgs>,
    runs: Vec<(usize, std::ops::Range<usize>)>,
    draws_revision: u64,
    bundle_cache: std::cell::RefCell<TerrainBundleCacheState>,
    indirect_buffer: Option<wgpu::Buffer>,
    indirect_capacity: usize,
}

impl TerrainLayerArena {
    fn new(vertex_label: &'static str, index_label: &'static str) -> Self {
        let vertices =
            (TERRAIN_PAGE_BYTES / std::mem::size_of::<crate::gpu::TerrainGpuVertex>()) as u32;
        Self {
            pages: Vec::new(),
            active_page: 0,
            page_size: (vertices, vertices / 2 * 3),
            limits: (u32::MAX, u32::MAX),
            live: (0, 0),
            vertex_label,
            index_label,
            draws: Vec::new(),
            runs: Vec::new(),
            draws_revision: 0,
            bundle_cache: std::cell::RefCell::new(TerrainBundleCacheState::default()),
            indirect_buffer: None,
            indirect_capacity: 0,
        }
    }

    fn page_for(
        &mut self,
        device: &wgpu::Device,
        vertices: u32,
        indices: u32,
        shared_quads: bool,
        use_u16: bool,
    ) -> Option<usize> {
        let vertex_stride = std::mem::size_of::<crate::gpu::TerrainGpuVertex>();
        if self.pages.is_empty() {
            let max = device.limits().max_buffer_size;
            self.limits.0 = u64::from(self.limits.0).min(max / vertex_stride as u64) as u32;
            self.limits.1 =
                u64::from(self.limits.1).min(max / std::mem::size_of::<u32>() as u64) as u32;
        }
        let available_indices = if shared_quads {
            self.limits.1
        } else {
            self.limits.1 - self.live.1
        };
        if vertices > self.limits.0 - self.live.0 || indices > available_indices {
            return None;
        }
        let indices = if shared_quads { 0 } else { indices };
        let fits_mode = |p: &TerrainPage, u16_mode: bool| {
            p.vertices.end != 0
                && p.shared_quads == shared_quads
                && p.index_u16 == u16_mode
                && p.vertices.fits(vertices)
                && p.indices.fits(indices)
        };
        if self
            .pages
            .get(self.active_page)
            .is_some_and(|p| fits_mode(p, use_u16))
        {
            return Some(self.active_page);
        }
        if let Some(page) = self.pages.iter().position(|p| fits_mode(p, use_u16)) {
            self.active_page = page;
            return Some(page);
        }
        let empty = self.pages.iter().position(|p| p.vertices.end == 0);
        let page = empty.unwrap_or(self.pages.len());
        let vertex_limit = vertices.max(self.page_size.0).min(self.limits.0);
        let mut index_limit = indices.max(self.page_size.1).min(self.limits.1);
        if use_u16 {
            index_limit &= !1;
        }
        if empty.is_none()
            && (self.live.0 > self.limits.0.saturating_sub(vertex_limit)
                || self.live.1 > self.limits.1.saturating_sub(index_limit))
        {
            let packs_mode = |p: &TerrainPage, u16_mode: bool| {
                p.shared_quads == shared_quads
                    && p.index_u16 == u16_mode
                    && p.vertices.can_pack(vertices)
                    && p.indices.can_pack(indices)
            };
            if let Some(page) = self
                .pages
                .iter()
                .position(|p| packs_mode(p, use_u16))
                .or_else(|| {
                    use_u16
                        .then(|| self.pages.iter().position(|p| packs_mode(p, false)))
                        .flatten()
                })
            {
                self.active_page = page;
                return Some(page);
            }
        }
        let index_stride = if use_u16 {
            std::mem::size_of::<u16>()
        } else {
            std::mem::size_of::<u32>()
        };
        // Small views grow only the first page. Later pages keep their buffers through further uploads.
        let buffers = TerrainPage {
            shared_quads,
            index_u16: use_u16,
            opaque_draws: Vec::new(),
            last_draw: 0,
            vertices: GpuArena::new(
                self.vertex_label,
                wgpu::BufferUsages::VERTEX,
                vertex_stride,
                if page == 0 || cfg!(target_arch = "wasm32") {
                    (VERTEX_ARENA_MIN_BYTES / vertex_stride) as u32
                } else {
                    vertex_limit
                },
                vertex_limit,
            ),
            indices: GpuArena::new(
                self.index_label,
                wgpu::BufferUsages::INDEX,
                index_stride,
                if page == 0 || cfg!(target_arch = "wasm32") {
                    INDEX_ARENA_MIN_CAPACITY
                } else {
                    index_limit
                },
                index_limit,
            ),
        };
        if page == self.pages.len() {
            self.pages.push(buffers);
        } else {
            self.pages[page] = buffers;
        }
        self.active_page = page;
        Some(page)
    }
}

struct TerrainLayerMesh {
    page: usize,
    first_vertex: u32,
    vertex_count: u32,
    first_index: u32,
    index_count: u32,
    faces: usize,
    culled_faces: usize,
    bounds: Option<TerrainBounds>,
    translucent_sort: Option<TerrainTranslucentSortState>,
}

struct TerrainSlot {
    /// `None` after a failed upload, so the next upload of the owner retries it.
    revision: Option<u64>,
    meshes: [Option<TerrainLayerMesh>; TERRAIN_LAYER_COUNT],
}

#[derive(Default)]
struct TerrainTotals {
    meshes: usize,
    vertices: usize,
    indices: usize,
    faces: [usize; TERRAIN_LAYER_COUNT],
    culled_faces: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct VisibleTerrainSlot {
    owner: TerrainSectionMeshOwner,
    slot: u32,
}

pub(crate) struct TerrainResidency {
    layers: [TerrainLayerArena; TERRAIN_LAYER_COUNT],
    quad_indices_u16: Option<wgpu::Buffer>,
    quad_indices_u32: Option<wgpu::Buffer>,
    fades: TerrainFadeBuffer,
    slots: TerrainSlots,
    slot_of: TerrainSectionMap<u32>,
    pending_resorts: BTreeMap<TerrainSectionMeshOwner, [f32; 3]>,
    visibility: TerrainSectionMap<f32>,
    next_visibility: TerrainSectionMap<f32>,
    has_fading: bool,
    // Native near-to-far traversal. Vanilla reverses it for the TRANSLUCENT draw list
    // (MC 26.1). An owner is in `visible_order` iff it is in
    // `visibility`.
    visible_order: Vec<VisibleTerrainSlot>,
    next_visible_order: Vec<VisibleTerrainSlot>,
    nearby_order: Vec<VisibleTerrainSlot>,
    next_nearby_order: Vec<VisibleTerrainSlot>,
    orders_stale: bool,
    layout_revision: Option<u64>,
    states_revision: Option<u64>,
    draws_dirty: bool,
    last_sort_block_position: Option<[i32; 3]>,
    resort_iteration_index: usize,
    last_resort_owner: Option<TerrainSectionMeshOwner>,
    unsubmitted_resorts: Vec<(TerrainSectionMeshOwner, [f32; 3])>,
    resort_quad_distances: Vec<(u32, f32)>,
    upload_indices_u16: Vec<u16>,
    resort_indices: Vec<u32>,
    resort_indices_u16: Vec<u16>,
    source_sections: usize,
    totals: TerrainTotals,
    /// Owners whose last upload did not fit; retried once a release frees arena space.
    failed: TerrainSectionSet,
    retry_ready: bool,
    retired_buffers: Vec<wgpu::Buffer>,
}

impl Default for TerrainResidency {
    fn default() -> Self {
        Self {
            layers: [
                TerrainLayerArena::new("bbb-terrain-solid-vertices", "bbb-terrain-solid-indices"),
                TerrainLayerArena::new("bbb-terrain-cutout-vertices", "bbb-terrain-cutout-indices"),
                TerrainLayerArena::new(
                    "bbb-terrain-translucent-vertices",
                    "bbb-terrain-translucent-indices",
                ),
            ],
            fades: TerrainFadeBuffer::default(),
            quad_indices_u16: None,
            quad_indices_u32: None,
            slots: TerrainSlots::default(),
            slot_of: TerrainSectionMap::default(),
            pending_resorts: BTreeMap::new(),
            visibility: TerrainSectionMap::default(),
            next_visibility: TerrainSectionMap::default(),
            has_fading: false,
            visible_order: Vec::new(),
            next_visible_order: Vec::new(),
            nearby_order: Vec::new(),
            next_nearby_order: Vec::new(),
            orders_stale: false,
            layout_revision: None,
            states_revision: None,
            draws_dirty: false,
            last_sort_block_position: None,
            resort_iteration_index: 0,
            last_resort_owner: None,
            unsubmitted_resorts: Vec::new(),
            resort_quad_distances: Vec::new(),
            upload_indices_u16: Vec::new(),
            resort_indices: Vec::new(),
            resort_indices_u16: Vec::new(),
            source_sections: 0,
            totals: TerrainTotals::default(),
            failed: TerrainSectionSet::default(),
            retry_ready: false,
            retired_buffers: Vec::new(),
        }
    }
}

impl TerrainResidency {
    pub(super) fn revision(&self, owner: &TerrainSectionMeshOwner) -> Option<u64> {
        let slot = *self.slot_of.get(owner)?;
        self.slots[slot as usize].as_ref()?.revision
    }

    pub(super) fn set_source_sections(&mut self, source_sections: usize) {
        self.source_sections = source_sections;
    }

    pub(super) fn resident_owners(&self) -> impl Iterator<Item = TerrainSectionMeshOwner> + '_ {
        self.slot_of.keys().copied()
    }

    /// Drops an owner's slot and its culled/visible projection. Returns the removed layer
    /// meshes and whether one was translucent.
    pub(super) fn remove_owner(&mut self, owner: TerrainSectionMeshOwner) -> (usize, bool) {
        let Some(slot) = self.slot_of.remove(&owner) else {
            return (0, false);
        };
        self.layout_revision = None;
        self.orders_stale |= self.visibility.remove(&owner).is_some();
        self.failed.remove(&owner);
        let meshes = self
            .slots
            .remove(slot)
            .map(|slot| slot.meshes)
            .unwrap_or_default();
        self.release_meshes(owner, meshes)
    }

    /// Completes a batch of `remove_owner` calls with one pass over the visible orders.
    pub(super) fn finish_removals(&mut self) {
        if !self.orders_stale {
            return;
        }
        self.orders_stale = false;
        let visibility = &self.visibility;
        self.visible_order
            .retain(|section| visibility.contains_key(&section.owner));
        self.nearby_order
            .retain(|section| visibility.contains_key(&section.owner));
        self.has_fading = self.visibility.values().any(|visibility| *visibility < 1.0);
        self.draws_dirty = true;
    }

    fn release_meshes(
        &mut self,
        owner: TerrainSectionMeshOwner,
        meshes: [Option<TerrainLayerMesh>; TERRAIN_LAYER_COUNT],
    ) -> (usize, bool) {
        let mut removed = 0;
        let mut translucent = false;
        for (layer, mesh) in meshes.into_iter().enumerate() {
            let Some(mesh) = mesh else {
                continue;
            };
            let arena = &mut self.layers[layer];
            let page = &mut arena.pages[mesh.page];
            page.vertices.release(
                mesh.first_vertex,
                mesh.vertex_count,
                &mut self.retired_buffers,
            );
            if !page.shared_quads {
                page.indices.release(
                    mesh.first_index,
                    mesh.index_count,
                    &mut self.retired_buffers,
                );
                arena.live.1 -= mesh.index_count;
            }
            arena.live.0 -= mesh.vertex_count;
            if layer == TRANSLUCENT {
                self.pending_resorts.remove(&owner);
                translucent = true;
            }
            self.totals.meshes -= 1;
            self.totals.vertices -= mesh.vertex_count as usize;
            self.totals.indices -= mesh.index_count as usize;
            self.totals.faces[layer] -= mesh.faces;
            self.totals.culled_faces -= mesh.culled_faces;
            removed += 1;
        }
        self.retry_ready |= removed != 0 && !self.failed.is_empty();
        (removed, translucent)
    }

    /// Replaces one owner's meshes with its newly compiled layers. Returns whether the
    /// translucent mesh set changed.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn upsert(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut StagedCommands,
        counters: &mut RendererCounters,
        owner: TerrainSectionMeshOwner,
        revision: u64,
        layers: &terrain::TerrainSectionMeshLayers,
        mesh_camera_position: [f32; 3],
    ) -> bool {
        let slot = match self.slot_of.get(&owner) {
            Some(&slot) => slot,
            None => {
                let slot = self.slots.allocate();
                self.slot_of.insert(owner, slot);
                self.layout_revision = None;
                let fade = self.visibility.get(&owner).copied().unwrap_or(1.0);
                self.fades.set_slot(slot, owner_origin_blocks(owner), fade);
                slot
            }
        };
        let previous = self.slots[slot as usize].replace(TerrainSlot {
            revision: Some(revision),
            meshes: Default::default(),
        });
        let (removed, mut translucent_changed) =
            self.release_meshes(owner, previous.map(|slot| slot.meshes).unwrap_or_default());
        counters.terrain_removed_owners += removed;

        let sections = [&layers.opaque, &layers.cutout, &layers.translucent];
        let mut failed = false;
        for (layer, sections) in sections.into_iter().enumerate() {
            let Some(section) = sections
                .iter()
                .find(|section| section.section_y == owner.section_y)
            else {
                continue;
            };
            let mesh = match self.create_mesh(
                device,
                uploads,
                counters,
                layer,
                owner,
                &section.mesh,
                mesh_camera_position,
            ) {
                Ok(Some(mesh)) => mesh,
                Ok(None) => continue,
                Err(()) => {
                    failed = true;
                    continue;
                }
            };
            if let Some(slot) = self.slots[slot as usize].as_mut() {
                slot.meshes[layer] = Some(mesh);
            }
            counters.terrain_uploaded_owners += 1;
            translucent_changed |= layer == TRANSLUCENT;
        }
        if failed {
            if let Some(slot) = self.slots[slot as usize].as_mut() {
                slot.revision = None;
            }
            self.failed.insert(owner);
            self.retry_ready = false;
        } else {
            self.failed.remove(&owner);
        }
        self.draws_dirty |= self.visibility.contains_key(&owner);
        translucent_changed
    }

    #[allow(clippy::too_many_arguments)]
    fn create_mesh(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut StagedCommands,
        counters: &mut RendererCounters,
        layer: usize,
        owner: TerrainSectionMeshOwner,
        mesh: &terrain::TerrainMesh,
        mesh_camera_position: [f32; 3],
    ) -> Result<Option<TerrainLayerMesh>, ()> {
        let index_count = mesh.index_count();
        if mesh.vertices.is_empty() || index_count == 0 {
            return Ok(None);
        }
        let (Ok(vertex_count), Ok(index_count)) = (
            u32::try_from(mesh.vertices.len()),
            u32::try_from(index_count),
        ) else {
            return Err(());
        };
        let (encoder, staging) = uploads.record(device);
        let shared_quads = mesh.shared_quad_indices
            || (layer != TRANSLUCENT
                && vertex_count % 4 == 0
                && index_count % 6 == 0
                && index_count / 6 == vertex_count / 4
                && mesh
                    .indices
                    .as_chunks::<6>()
                    .0
                    .iter()
                    .enumerate()
                    .all(|(quad, &indices)| indices == sequential_quad_indices(quad as u32)));
        let use_u16 = vertex_count <= 65_536
            && (shared_quads
                || (index_count.is_multiple_of(2)
                    && !mesh
                        .indices
                        .iter()
                        .any(|&index| index > u32::from(u16::MAX))));
        let Some(page) =
            self.layers[layer].page_for(device, vertex_count, index_count, shared_quads, use_u16)
        else {
            tracing::error!(
                ?owner,
                vertex_count,
                index_count,
                "terrain layer exceeds the device limit"
            );
            return Err(());
        };
        let page_u16 = self.layers[layer].pages[page].index_u16;
        let Some(first_vertex) = self.allocate(device, encoder, layer, page, true, vertex_count)
        else {
            tracing::error!(
                ?owner,
                vertex_count,
                index_count,
                "terrain vertex arena allocation failed after compaction"
            );
            return Err(());
        };
        let first_index = if shared_quads {
            let bytes = self.ensure_quad_indices(device, layer, index_count / 6, page_u16);
            counters.upload_bytes += bytes;
            counters.terrain_index_upload_bytes += bytes;
            0
        } else {
            let Some(first_index) = self.allocate(device, encoder, layer, page, false, index_count)
            else {
                self.layers[layer].pages[page].vertices.release(
                    first_vertex,
                    vertex_count,
                    &mut self.retired_buffers,
                );
                tracing::error!(
                    ?owner,
                    vertex_count,
                    index_count,
                    "terrain index arena allocation failed after compaction"
                );
                return Err(());
            };
            self.layers[layer].live.1 += index_count;
            first_index
        };
        let layer_arena = &mut self.layers[layer];
        layer_arena.live.0 += vertex_count;
        let arena = &layer_arena.pages[page];
        arena
            .vertices
            .stage(encoder, staging, first_vertex, &mesh.vertices);
        let staged_index_bytes = if shared_quads {
            0
        } else if page_u16 {
            self.upload_indices_u16.clear();
            self.upload_indices_u16
                .extend(mesh.indices.iter().map(|&index| index as u16));
            arena
                .indices
                .stage(encoder, staging, first_index, &self.upload_indices_u16);
            std::mem::size_of_val(self.upload_indices_u16.as_slice()) as u64
        } else {
            arena
                .indices
                .stage(encoder, staging, first_index, &mesh.indices);
            std::mem::size_of_val(mesh.indices.as_slice()) as u64
        };
        if !self.retired_buffers.is_empty() {
            self.invalidate_bundles();
            for old in self.retired_buffers.drain(..) {
                staging.retire_buffer(old);
            }
        }

        let translucent_sort = (layer == TRANSLUCENT && mesh.translucent_faces > 0)
            .then(|| {
                TerrainTranslucentSortState::from_vertices(
                    &mesh.vertices,
                    &mesh.indices,
                    Some(translucency_point_of_view(mesh_camera_position, owner)),
                )
            })
            .flatten();
        let faces = [mesh.opaque_faces, mesh.cutout_faces, mesh.translucent_faces][layer];
        self.totals.meshes += 1;
        self.totals.vertices += mesh.vertices.len();
        self.totals.indices += index_count as usize;
        self.totals.faces[layer] += faces;
        self.totals.culled_faces += mesh.culled_faces;

        let vertex_bytes = std::mem::size_of_val(mesh.vertices.as_slice()) as u64;
        let index_bytes = staged_index_bytes;
        counters.upload_bytes += vertex_bytes + index_bytes;
        counters.terrain_vertex_upload_bytes = counters
            .terrain_vertex_upload_bytes
            .saturating_add(vertex_bytes);
        counters.terrain_index_upload_bytes = counters
            .terrain_index_upload_bytes
            .saturating_add(index_bytes);
        Ok(Some(TerrainLayerMesh {
            page,
            first_vertex,
            vertex_count,
            first_index,
            index_count,
            faces,
            culled_faces: mesh.culled_faces,
            bounds: TerrainBounds::from_section_vertices(
                owner_origin_blocks(owner),
                &mesh.vertices,
            ),
            translucent_sort,
        }))
    }

    fn quad_indices_buffer(&self, index_u16: bool) -> Option<&wgpu::Buffer> {
        if index_u16 {
            self.quad_indices_u16.as_ref()
        } else {
            self.quad_indices_u32.as_ref()
        }
    }

    fn ensure_quad_indices(
        &mut self,
        device: &wgpu::Device,
        layer: usize,
        quads: u32,
        index_u16: bool,
    ) -> u64 {
        let max_quads = if index_u16 {
            65_536 / 4
        } else {
            self.layers[layer].limits.1 / 6
        };
        if index_u16 {
            let stride = std::mem::size_of::<[u16; 6]>() as u64;
            let capacity =
                (self.quad_indices_u16.as_ref().map_or(0, wgpu::Buffer::size) / stride) as u32;
            if quads <= capacity {
                return 0;
            }
            let capacity = quads.max(capacity.saturating_mul(2)).min(max_quads);
            let mut indices = Vec::with_capacity((capacity as usize).saturating_mul(6));
            for quad in 0..capacity {
                indices.extend(sequential_quad_indices(quad).map(|index| index as u16));
            }
            self.retired_buffers.extend(self.quad_indices_u16.replace(
                crate::frame_buffers::create_initialized_buffer(
                    device,
                    Some("bbb-terrain-quad-indices-u16"),
                    bytemuck::cast_slice(&indices),
                    wgpu::BufferUsages::INDEX,
                ),
            ));
            return u64::from(capacity) * stride;
        }
        let stride = std::mem::size_of::<[u32; 6]>() as u64;
        let capacity =
            (self.quad_indices_u32.as_ref().map_or(0, wgpu::Buffer::size) / stride) as u32;
        if quads <= capacity {
            return 0;
        }
        let capacity = quads.max(capacity.saturating_mul(2)).min(max_quads);
        let mut indices = Vec::with_capacity((capacity as usize).saturating_mul(6));
        for quad in 0..capacity {
            indices.extend_from_slice(&sequential_quad_indices(quad));
        }
        self.retired_buffers.extend(self.quad_indices_u32.replace(
            crate::frame_buffers::create_initialized_buffer(
                device,
                Some("bbb-terrain-quad-indices"),
                bytemuck::cast_slice(&indices),
                wgpu::BufferUsages::INDEX,
            ),
        ));
        u64::from(capacity) * stride
    }

    /// Allocates from one arena; a failure at the size limit compacts that arena, moving the
    /// resident meshes' offsets, and retries once.
    fn allocate(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        layer: usize,
        page: usize,
        vertices: bool,
        len: u32,
    ) -> Option<u32> {
        let arenas = &mut self.layers[layer].pages[page];
        let arena = if vertices {
            &mut arenas.vertices
        } else {
            &mut arenas.indices
        };
        let old_capacity = arena.capacity;
        if let Some(start) = arena.allocate(device, encoder, &mut self.retired_buffers, len) {
            let grew = arena.capacity != old_capacity;
            if layer == TRANSLUCENT && !vertices && grew {
                self.requeue_unsubmitted_resorts();
            }
            return Some(start);
        }
        let compaction = arena.compact(device, encoder, &mut self.retired_buffers);
        for mesh in self
            .slots
            .occupied_mut()
            .filter_map(|slot| slot.meshes[layer].as_mut())
            .filter(|mesh| mesh.page == page)
        {
            let offset = if vertices {
                &mut mesh.first_vertex
            } else {
                &mut mesh.first_index
            };
            *offset = compaction.remap(*offset);
        }
        if layer == TRANSLUCENT && !vertices {
            self.requeue_unsubmitted_resorts();
        }
        self.draws_dirty = true;
        let arenas = &mut self.layers[layer].pages[page];
        let arena = if vertices {
            &mut arenas.vertices
        } else {
            &mut arenas.indices
        };
        arena.allocate(device, encoder, &mut self.retired_buffers, len)
    }

    /// Owners to upload again: those whose upload failed, once a release has freed space.
    pub(super) fn retry_owners(&self) -> impl Iterator<Item = TerrainSectionMeshOwner> + '_ {
        self.failed.iter().copied().filter(|_| self.retry_ready)
    }

    /// Updates only fade-in visibility values in `TerrainFadeBuffer` when the visible section
    /// membership and order (`layout_revision`) are unchanged, falling back to full section-state
    /// reconciliation when membership/order or resident slots changed.
    pub(super) fn set_section_states_with_fades(
        &mut self,
        layout_revision: u64,
        state_revision: u64,
        sections: &[TerrainSectionRenderState],
        fading_sections: &[(TerrainSectionMeshOwner, f32)],
    ) -> bool {
        if self.states_revision == Some(state_revision) {
            return false;
        }
        if self.layout_revision == Some(layout_revision) {
            self.states_revision = Some(state_revision);
            let mut changed = false;
            let mut has_fading = false;
            for &(owner, visibility) in fading_sections {
                let visibility = visibility.clamp(0.0, 1.0);
                has_fading |= visibility < 1.0;
                let Some(&slot) = self.slot_of.get(&owner) else {
                    continue;
                };
                if let Some(previous) = self.visibility.get_mut(&owner) {
                    if previous.to_bits() != visibility.to_bits() {
                        *previous = visibility;
                        self.fades.set(slot, visibility);
                        changed = true;
                    }
                }
            }
            self.has_fading = has_fading;
            return changed;
        }
        let changed = self.set_section_states(state_revision, sections);
        self.layout_revision = Some(layout_revision);
        changed
    }

    /// Replaces the native section-lifecycle projection. Returns whether it changed.
    pub(super) fn set_section_states(
        &mut self,
        revision: u64,
        sections: &[TerrainSectionRenderState],
    ) -> bool {
        if self.states_revision == Some(revision) {
            return false;
        }
        self.states_revision = Some(revision);
        let mut visible_index = 0;
        let mut nearby_index = 0;
        let mut values_changed = false;
        let mut has_fading = false;
        let same_layout = sections.iter().all(|section| {
            let Some(&slot) = self.slot_of.get(&section.owner) else {
                return true;
            };
            let visible = VisibleTerrainSlot {
                owner: section.owner,
                slot,
            };
            if self.visible_order.get(visible_index) != Some(&visible) {
                // Repeated owners keep the first occurrence's visibility and nearby flag.
                return self.visible_order[..visible_index].contains(&visible);
            }
            visible_index += 1;
            if section.nearby {
                if self.nearby_order.get(nearby_index) != Some(&visible) {
                    return false;
                }
                nearby_index += 1;
            }
            let visibility = section.visibility.clamp(0.0, 1.0);
            let Some(previous) = self.visibility.get_mut(&section.owner) else {
                return false;
            };
            if *previous != visibility {
                *previous = visibility;
                self.fades.set(slot, visibility);
                values_changed = true;
            }
            has_fading |= visibility < 1.0;
            true
        }) && visible_index == self.visible_order.len()
            && nearby_index == self.nearby_order.len();
        if same_layout {
            self.has_fading = has_fading;
            return values_changed;
        }
        let visible = &mut self.next_visibility;
        visible.clear();
        visible.reserve(sections.len());
        let order = &mut self.next_visible_order;
        order.clear();
        order.reserve(sections.len());
        let nearby_order = &mut self.next_nearby_order;
        nearby_order.clear();
        let mut has_fading = false;
        for section in sections {
            let Some(&slot) = self.slot_of.get(&section.owner) else {
                continue;
            };
            let visibility = section.visibility.clamp(0.0, 1.0);
            let Entry::Vacant(entry) = visible.entry(section.owner) else {
                continue;
            };
            entry.insert(visibility);
            has_fading |= visibility < 1.0;
            let section_slot = VisibleTerrainSlot {
                owner: section.owner,
                slot,
            };
            order.push(section_slot);
            if section.nearby {
                nearby_order.push(section_slot);
            }
        }
        let order_changed = self.visible_order != self.next_visible_order;
        let update_fades = self.has_fading || has_fading;
        std::mem::swap(&mut self.visibility, &mut self.next_visibility);
        std::mem::swap(&mut self.visible_order, &mut self.next_visible_order);
        std::mem::swap(&mut self.nearby_order, &mut self.next_nearby_order);
        self.has_fading = has_fading;
        if update_fades {
            // A slot's fade is its owner's visibility, or 1.0 while culled, so only owners in
            // the previous or the new projection can change.
            let previous = self.next_visibility.keys().map(|owner| (owner, 1.0));
            let current = self.visibility.iter().map(|(owner, value)| (owner, *value));
            for (owner, value) in previous.chain(current) {
                let value = self.visibility.get(owner).copied().unwrap_or(value);
                let Some(&slot) = self.slot_of.get(owner) else {
                    continue;
                };
                self.fades.set(slot, value);
            }
        }
        if order_changed {
            self.draws_dirty = true;
        }
        true
    }

    pub(super) fn visible_sections(&self) -> usize {
        self.visibility.len()
    }

    #[cfg(test)]
    pub(super) fn is_visible(&self, owner: &TerrainSectionMeshOwner) -> bool {
        self.visibility.contains_key(owner)
    }
}

const MIN_INDIRECT_DRAW_RUN: usize = if cfg!(test) { 1 } else { 128 };

impl TerrainResidency {
    fn invalidate_bundles(&mut self) {
        for arena in &mut self.layers {
            arena.bundle_cache.get_mut().invalidate();
        }
    }

    fn requeue_unsubmitted_resorts(&mut self) {
        for (owner, camera) in self.unsubmitted_resorts.drain(..) {
            let Some(&slot) = self.slot_of.get(&owner) else {
                continue;
            };
            if let Some(sort) = self.slots[slot as usize]
                .as_mut()
                .and_then(|s| s.meshes[TRANSLUCENT].as_mut())
                .and_then(|m| m.translucent_sort.as_mut())
            {
                sort.last_point_of_view = None;
                self.pending_resorts.insert(owner, camera);
            }
        }
    }

    pub(crate) fn mark_draws_submitted(&mut self) {
        self.unsubmitted_resorts.clear();
    }

    pub(crate) fn restore_unsubmitted_draws(&mut self) {
        self.draws_dirty = true;
        self.fades.dirty = 0..self.fades.values.len();
        self.requeue_unsubmitted_resorts();
        self.invalidate_bundles();
    }

    /// Rebuilds the per-layer draw lists after residency or the visible projection changed.
    pub(crate) fn prepare_draws(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        staging: &mut crate::frame_buffers::UploadBelt,
    ) {
        let mut invalidate = !self.retired_buffers.is_empty();
        for old in self.retired_buffers.drain(..) {
            staging.retire_buffer(old);
        }
        invalidate |= self.fades.flush(device, encoder, staging);
        if invalidate {
            self.invalidate_bundles();
        }
        if !self.draws_dirty {
            return;
        }
        self.draws_dirty = false;
        let slots = &self.slots;
        let mesh_draw = |section: &VisibleTerrainSlot, layer: usize| {
            let slot = section.slot;
            let mesh = slots[slot as usize].as_ref()?.meshes[layer].as_ref()?;
            Some((
                mesh.page,
                DrawIndexedIndirectArgs {
                    first_index: mesh.first_index,
                    index_count: mesh.index_count,
                    base_vertex: mesh.first_vertex as i32,
                    instance_count: 1,
                    first_instance: slot,
                },
            ))
        };
        for (layer, arena) in self.layers.iter_mut().enumerate() {
            arena.draws_revision = arena.draws_revision.wrapping_add(1);
            arena.bundle_cache.get_mut().invalidate();
            arena.draws.clear();
            arena.runs.clear();
            if layer == TRANSLUCENT {
                for (page, draw) in self
                    .visible_order
                    .iter()
                    .rev()
                    .filter_map(|section| mesh_draw(section, layer))
                {
                    let index = arena.draws.len();
                    arena.draws.push(draw);
                    if let Some((last, range)) = arena.runs.last_mut() {
                        if *last == page {
                            range.end += 1;
                            continue;
                        }
                    }
                    arena.runs.push((page, index..index + 1));
                }
            } else {
                // Opaque layers batch by buffer. Order within each buffer stays near-to-far.
                for (order, (page, draw)) in self
                    .visible_order
                    .iter()
                    .filter_map(|section| mesh_draw(section, layer))
                    .enumerate()
                {
                    let buffer = &mut arena.pages[page];
                    if buffer.opaque_draws.is_empty() {
                        arena.runs.push((page, 0..0));
                    }
                    buffer.opaque_draws.push(draw);
                    buffer.last_draw = order;
                }
                // Keep the last visible draw last when a batch spans other buffers.
                arena
                    .runs
                    .sort_unstable_by_key(|(page, _)| arena.pages[*page].last_draw);
                for (page, range) in &mut arena.runs {
                    let start = arena.draws.len();
                    arena.draws.append(&mut arena.pages[*page].opaque_draws);
                    *range = start..arena.draws.len();
                }
            }
        }

        if device.features().contains(TERRAIN_MDI_FEATURES) {
            for arena in &mut self.layers {
                if !arena
                    .runs
                    .iter()
                    .any(|(_, range)| range.len() >= MIN_INDIRECT_DRAW_RUN)
                {
                    continue;
                }
                if arena.draws.len() > arena.indirect_capacity {
                    arena.indirect_capacity = arena.draws.len().max(arena.indirect_capacity * 2);
                    if let Some(old) = arena.indirect_buffer.replace(device.create_buffer(
                        &wgpu::BufferDescriptor {
                            label: Some("bbb-terrain-indirect-draws"),
                            size: (arena.indirect_capacity
                                * std::mem::size_of::<DrawIndexedIndirectArgs>())
                                as u64,
                            usage: wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        },
                    )) {
                        staging.retire_buffer(old);
                    }
                }
                let bytes: &[u8] = bytemuck::cast_slice(&arena.draws);
                if let Some(indirect_buffer) = arena.indirect_buffer.as_ref() {
                    staging.write_bytes(encoder, indirect_buffer, 0, bytes);
                }
            }
        }
    }

    /// Records one layer's visible sections; returns the draw count.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw(
        &self,
        device: &wgpu::Device,
        pass: &mut wgpu::RenderPass<'_>,
        layer: TerrainLayer,
        pipeline: &wgpu::RenderPipeline,
        bind_groups: [&wgpu::BindGroup; 2],
        color_format: wgpu::TextureFormat,
        sample_count: u32,
    ) -> usize {
        let arena = &self.layers[layer as usize];
        if arena.draws.is_empty() {
            return 0;
        }
        let Some(fade_buffer) = self.fades.buffer.as_ref() else {
            return 0;
        };
        if !device.features().contains(TERRAIN_MDI_FEATURES) {
            let mut cache = arena.bundle_cache.borrow_mut();
            let compile_ready = cache.advance(arena.draws_revision);
            let valid = cache.bundle.as_ref().is_some_and(|cached| {
                cached.draws_revision == arena.draws_revision
                    && cached.color_format == color_format
                    && cached.sample_count == sample_count
                    && &cached.pipeline == pipeline
                    && &cached.bind_groups[0] == bind_groups[0]
                    && &cached.bind_groups[1] == bind_groups[1]
                    && &cached.fade_buffer == fade_buffer
                    && cached.runs.len() == arena.runs.len()
                    && cached
                        .runs
                        .iter()
                        .zip(&arena.runs)
                        .all(|(key, (page, range))| {
                            let buffers = &arena.pages[*page];
                            let Some(vertices) = buffers.vertices.buffer.as_ref() else {
                                return false;
                            };
                            let Some(indices) = (if buffers.shared_quads {
                                self.quad_indices_buffer(buffers.index_u16)
                            } else {
                                buffers.indices.buffer.as_ref()
                            }) else {
                                return false;
                            };
                            key.page == *page
                                && key.range == *range
                                && key.index_u16 == buffers.index_u16
                                && &key.vertices == vertices
                                && &key.indices == indices
                        })
            });
            if !valid && compile_ready {
                let mut run_keys = Vec::with_capacity(arena.runs.len());
                let mut bundle_encoder =
                    device.create_render_bundle_encoder(&wgpu::RenderBundleEncoderDescriptor {
                        label: Some("bbb-terrain-layer-bundle"),
                        color_formats: &[Some(color_format)],
                        depth_stencil: Some(wgpu::RenderBundleDepthStencil {
                            format: crate::gpu::DEPTH_FORMAT,
                            depth_read_only: false,
                            stencil_read_only: true,
                        }),
                        sample_count,
                        multiview: None,
                    });
                bundle_encoder.set_pipeline(pipeline);
                for (index, bind_group) in bind_groups.into_iter().enumerate() {
                    bundle_encoder.set_bind_group(index as u32, bind_group, &[]);
                }
                bundle_encoder.set_vertex_buffer(1, fade_buffer.slice(..));
                let mut bound_shared_u16 = None;
                for (page, range) in &arena.runs {
                    let buffers = &arena.pages[*page];
                    let Some(vertices_buf) = buffers.vertices.buffer.as_ref() else {
                        continue;
                    };
                    let Some(vertices_slice) = buffers.vertices.used_slice() else {
                        continue;
                    };
                    let (Some(indices_buf), Some(indices)) = (if buffers.shared_quads {
                        let buf = self.quad_indices_buffer(buffers.index_u16);
                        (buf, buf.map(|b| b.slice(..)))
                    } else {
                        (
                            buffers.indices.buffer.as_ref(),
                            buffers.indices.used_slice(),
                        )
                    }) else {
                        continue;
                    };
                    bundle_encoder.set_vertex_buffer(0, vertices_slice);
                    if !buffers.shared_quads || bound_shared_u16 != Some(buffers.index_u16) {
                        bundle_encoder.set_index_buffer(indices, buffers.index_format());
                    }
                    bound_shared_u16 = buffers.shared_quads.then_some(buffers.index_u16);
                    for draw in &arena.draws[range.clone()] {
                        bundle_encoder.draw_indexed(
                            draw.first_index..draw.first_index + draw.index_count,
                            draw.base_vertex,
                            draw.first_instance..draw.first_instance + draw.instance_count,
                        );
                    }
                    run_keys.push(TerrainRunBufferKey {
                        page: *page,
                        range: range.clone(),
                        index_u16: buffers.index_u16,
                        vertices: vertices_buf.clone(),
                        indices: indices_buf.clone(),
                    });
                }
                let bundle = bundle_encoder.finish(&wgpu::RenderBundleDescriptor {
                    label: Some("bbb-terrain-layer-bundle"),
                });
                cache.bundle = Some(CachedTerrainLayerBundle {
                    draws_revision: arena.draws_revision,
                    color_format,
                    sample_count,
                    pipeline: pipeline.clone(),
                    bind_groups: [bind_groups[0].clone(), bind_groups[1].clone()],
                    fade_buffer: fade_buffer.clone(),
                    runs: run_keys,
                    bundle,
                });
            }
            if valid || compile_ready {
                if let Some(cached) = cache.bundle.as_ref() {
                    pass.execute_bundles(std::iter::once(&cached.bundle));
                }
                return arena.draws.len();
            }
        }
        pass.set_pipeline(pipeline);
        for (index, bind_group) in bind_groups.into_iter().enumerate() {
            pass.set_bind_group(index as u32, bind_group, &[]);
        }
        pass.set_vertex_buffer(1, fade_buffer.slice(..));
        let mut bound_shared_u16 = None;
        for (page, range) in &arena.runs {
            let buffers = &arena.pages[*page];
            let Some(vertices_slice) = buffers.vertices.used_slice() else {
                continue;
            };
            pass.set_vertex_buffer(0, vertices_slice);
            if !buffers.shared_quads || bound_shared_u16 != Some(buffers.index_u16) {
                let Some(indices) = (if buffers.shared_quads {
                    self.quad_indices_buffer(buffers.index_u16)
                        .map(|buf| buf.slice(..))
                } else {
                    buffers.indices.used_slice()
                }) else {
                    continue;
                };
                pass.set_index_buffer(indices, buffers.index_format());
            }
            bound_shared_u16 = buffers.shared_quads.then_some(buffers.index_u16);
            if let Some(indirect) = arena
                .indirect_buffer
                .as_ref()
                .filter(|_| range.len() >= MIN_INDIRECT_DRAW_RUN)
            {
                let offset = range.start * std::mem::size_of::<DrawIndexedIndirectArgs>();
                pass.multi_draw_indexed_indirect(indirect, offset as u64, range.len() as u32);
            } else {
                for draw in &arena.draws[range.clone()] {
                    pass.draw_indexed(
                        draw.first_index..draw.first_index + draw.index_count,
                        draw.base_vertex,
                        draw.first_instance..draw.first_instance + draw.instance_count,
                    );
                }
            }
        }
        arena.draws.len()
    }

    pub(super) fn resort_translucent_for_camera(
        &mut self,
        device: &wgpu::Device,
        uploads: &mut StagedCommands,
        camera_position: Option<[f32; 3]>,
        counters: &mut RendererCounters,
    ) {
        counters.terrain_translucent_resort_owners = 0;
        counters.terrain_translucent_resort_index_bytes = 0;
        let Some(camera_position) = camera_position else {
            return;
        };
        let camera_block = camera_block_position(camera_position);
        let block_position_changed = self.last_sort_block_position != Some(camera_block);

        if !self.slot_of.is_empty() {
            // Vanilla 26.1 schedules direct references from the nearby list, then direct
            // references from a round-robin visible-section window.
            let visible = &self.visible_order;
            if visible.is_empty() {
                return;
            }
            for &section in &self.nearby_order {
                schedule_translucent_section(
                    &self.slots,
                    &mut self.pending_resorts,
                    section,
                    camera_position,
                    block_position_changed,
                    true,
                );
            }
            self.resort_iteration_index %= visible.len();
            let resorts = (visible.len() / 8).max(15);
            for _ in 0..resorts {
                let section = visible[self.resort_iteration_index % visible.len()];
                self.resort_iteration_index = self.resort_iteration_index.saturating_add(1);
                schedule_translucent_section(
                    &self.slots,
                    &mut self.pending_resorts,
                    section,
                    camera_position,
                    block_position_changed,
                    false,
                );
            }
        }

        // Drain one owner per update in circular section order. Recompiling or
        // removing a mesh cancels its pending request in release_meshes.
        let after = self.last_resort_owner.map_or(Unbounded, Excluded);
        if let Some((&owner, &camera)) = self
            .pending_resorts
            .range((after, Unbounded))
            .next()
            .or_else(|| self.pending_resorts.first_key_value())
        {
            self.pending_resorts.remove(&owner);
            self.last_resort_owner = Some(owner);
            if let Some((page, first_index)) = self
                .slot_of
                .get(&owner)
                .copied()
                .and_then(|slot| self.slots[slot as usize].as_mut())
                .and_then(|slot| slot.meshes[TRANSLUCENT].as_mut())
                .and_then(|mesh| {
                    let sort = mesh.translucent_sort.as_mut()?;
                    sort.write_indices_for_camera(
                        local_camera_for_owner(camera, owner),
                        translucency_point_of_view(camera, owner),
                        &mut self.resort_quad_distances,
                        &mut self.resort_indices,
                    );
                    Some((mesh.page, mesh.first_index))
                })
            {
                let (encoder, staging) = uploads.record(device);
                let page_arena = &self.layers[TRANSLUCENT].pages[page];
                let staged_bytes = if page_arena.index_u16 {
                    self.resort_indices_u16.clear();
                    self.resort_indices_u16
                        .extend(self.resort_indices.iter().map(|&index| index as u16));
                    page_arena.indices.stage(
                        encoder,
                        staging,
                        first_index,
                        &self.resort_indices_u16,
                    );
                    std::mem::size_of_val(self.resort_indices_u16.as_slice()) as u64
                } else {
                    page_arena
                        .indices
                        .stage(encoder, staging, first_index, &self.resort_indices);
                    std::mem::size_of_val(self.resort_indices.as_slice()) as u64
                };
                self.unsubmitted_resorts.push((owner, camera));
                counters.terrain_translucent_resort_owners = 1;
                counters.terrain_translucent_resort_index_bytes = staged_bytes;
            }
        }
        self.last_sort_block_position = Some(camera_block);
    }

    pub(super) fn finish_upload(&mut self, counters: &mut RendererCounters) {
        counters.uploaded_sections = if self.totals.meshes == 0 {
            0
        } else {
            self.source_sections
        };
        counters.visible_sections = self.visibility.len();
        let mut live_index_bytes = 0u64;
        let mut has_shared_u16 = false;
        let mut has_shared_u32 = false;
        for arena in &self.layers {
            for page in &arena.pages {
                if page.vertices.end == 0 {
                    continue;
                }
                if page.shared_quads {
                    if page.index_u16 {
                        has_shared_u16 = true;
                    } else {
                        has_shared_u32 = true;
                    }
                } else {
                    let live = page.indices.end.saturating_sub(page.indices.free_total);
                    live_index_bytes += u64::from(live) * page.indices.element_size;
                }
            }
        }
        let mut retired_shared = false;
        if !has_shared_u16 && self.quad_indices_u16.is_some() {
            self.retired_buffers.extend(self.quad_indices_u16.take());
            retired_shared = true;
        }
        if !has_shared_u32 && self.quad_indices_u32.is_some() {
            self.retired_buffers.extend(self.quad_indices_u32.take());
            retired_shared = true;
        }
        if retired_shared {
            self.invalidate_bundles();
        }
        counters.resident_bytes =
            (self.totals.vertices * std::mem::size_of::<crate::gpu::TerrainGpuVertex>()) as u64
                + live_index_bytes
                + self.quad_indices_u16.as_ref().map_or(0, wgpu::Buffer::size)
                + self.quad_indices_u32.as_ref().map_or(0, wgpu::Buffer::size);
        counters.terrain_peak_resident_bytes = counters
            .terrain_peak_resident_bytes
            .max(counters.resident_bytes);
        counters.terrain_vertices = self.totals.vertices;
        counters.terrain_indices = self.totals.indices;
        counters.opaque_faces = self.totals.faces[TerrainLayer::Solid as usize];
        counters.cutout_faces = self.totals.faces[TerrainLayer::Cutout as usize];
        counters.translucent_faces = self.totals.faces[TRANSLUCENT];
        counters.culled_faces = self.totals.culled_faces;
        counters.terrain_failed_owners = self.failed.len();
    }

    pub(super) fn bounds(&self) -> Option<TerrainBounds> {
        let mut bounds: Option<TerrainBounds> = None;
        for mesh_bounds in self
            .slots
            .occupied()
            .flat_map(|slot| slot.meshes.iter().flatten())
            .filter_map(|mesh| mesh.bounds)
        {
            match &mut bounds {
                Some(bounds) => bounds.include_bounds(mesh_bounds),
                None => bounds = Some(mesh_bounds),
            }
        }
        bounds
    }

    #[cfg(test)]
    pub(super) fn layer_draws(&self, layer: TerrainLayer) -> Vec<TerrainSectionMeshOwner> {
        self.layers[layer as usize]
            .draws
            .iter()
            .filter_map(|draw| {
                self.slot_of
                    .iter()
                    .find(|(_, &slot)| slot == draw.first_instance)
                    .map(|(owner, _)| *owner)
            })
            .collect()
    }

    #[cfg(test)]
    pub(super) fn layer_buffers(
        &self,
        layer: TerrainLayer,
        owner: TerrainSectionMeshOwner,
    ) -> Option<(wgpu::Buffer, wgpu::Buffer)> {
        let slot = self.slot_of[&owner] as usize;
        let mesh = self.slots[slot].as_ref()?.meshes[layer as usize].as_ref()?;
        let arena = &self.layers[layer as usize].pages[mesh.page];
        Some((
            arena.vertices.buffer.clone()?,
            if arena.shared_quads {
                self.quad_indices_buffer(arena.index_u16).cloned()?
            } else {
                arena.indices.buffer.clone()?
            },
        ))
    }

    #[cfg(test)]
    pub(super) fn mesh_range(
        &self,
        owner: TerrainSectionMeshOwner,
        layer: TerrainLayer,
    ) -> Option<(u32, u32)> {
        let slot = *self.slot_of.get(&owner)?;
        let mesh = self.slots[slot as usize].as_ref()?.meshes[layer as usize].as_ref()?;
        Some((mesh.first_vertex, mesh.first_index))
    }

    #[cfg(test)]
    pub(super) fn fade(&self, owner: TerrainSectionMeshOwner) -> Option<f32> {
        Some(self.fades.values[*self.slot_of.get(&owner)? as usize].visibility)
    }

    #[cfg(test)]
    pub(super) fn set_page_size(&mut self, vertices: u32, indices: u32) {
        for layer in &mut self.layers {
            layer.page_size = (vertices, indices);
        }
    }

    /// Caps every arena as if the device's `max_buffer_size` were this many elements.
    #[cfg(test)]
    pub(super) fn limit_arenas(&mut self, vertices: u32, indices: u32) {
        for arena in &mut self.layers {
            arena.limits = (vertices, indices);
        }
    }
}

fn schedule_translucent_section(
    slots: &TerrainSlots,
    pending: &mut BTreeMap<TerrainSectionMeshOwner, [f32; 3]>,
    section: VisibleTerrainSlot,
    camera_position: [f32; 3],
    block_position_changed: bool,
    nearby: bool,
) {
    let owner = section.owner;
    let Some(sort) = slots[section.slot as usize]
        .as_ref()
        .and_then(|slot| slot.meshes[TRANSLUCENT].as_ref())
        .and_then(|mesh| mesh.translucent_sort.as_ref())
    else {
        return;
    };
    let viewpoint = translucency_point_of_view(camera_position, owner);
    if sort.needs_resort(viewpoint, block_position_changed, nearby) || pending.contains_key(&owner)
    {
        pending.insert(owner, camera_position);
    }
}

#[derive(Debug, Clone)]
struct TerrainTranslucentSortState {
    centroids: Vec<[f32; 3]>,
    local_indices: Vec<[u8; 6]>,
    last_point_of_view: Option<[i8; 3]>,
}

impl TerrainTranslucentSortState {
    fn from_vertices(
        vertices: &[crate::gpu::TerrainGpuVertex],
        indices: &[u32],
        last_point_of_view: Option<[i8; 3]>,
    ) -> Option<Self> {
        if !vertices.len().is_multiple_of(4) || indices.len() != vertices.len() / 4 * 6 {
            return None;
        }

        let centroids = vertices
            .as_chunks::<4>()
            .0
            .iter()
            .map(|quad| {
                let p0 = quad[0].local_position();
                let p2 = quad[2].local_position();
                [
                    (p0[0] + p2[0]) * 0.5,
                    (p0[1] + p2[1]) * 0.5,
                    (p0[2] + p2[2]) * 0.5,
                ]
            })
            .collect::<Vec<_>>();
        // Six indices per quad and one entry per base guarantee coverage once duplicates are excluded.
        let mut local_indices = vec![[u8::MAX; 6]; centroids.len()];
        for quad in indices.as_chunks::<6>().0 {
            let quad_index = quad[0] / 4;
            let local = local_indices.get_mut(quad_index as usize)?;
            if local[0] != u8::MAX || quad.iter().any(|index| index / 4 != quad_index) {
                return None;
            }
            *local = std::array::from_fn(|i| (quad[i] % 4) as u8);
        }
        Some(Self {
            centroids,
            local_indices,
            last_point_of_view,
        })
    }

    fn needs_resort(
        &self,
        point_of_view: [i8; 3],
        block_position_changed: bool,
        nearby: bool,
    ) -> bool {
        self.last_point_of_view != Some(point_of_view)
            || (block_position_changed && (nearby || point_of_view.contains(&0)))
    }

    fn write_indices_for_camera(
        &mut self,
        camera_position: [f32; 3],
        point_of_view: [i8; 3],
        quad_distances: &mut Vec<(u32, f32)>,
        indices: &mut Vec<u32>,
    ) {
        self.last_point_of_view = Some(point_of_view);
        self.write_sorted_indices(camera_position, quad_distances, indices);
    }

    #[cfg(test)]
    fn indices_for_camera(
        &mut self,
        camera_position: [f32; 3],
        point_of_view: [i8; 3],
    ) -> Vec<u32> {
        let mut quad_distances = Vec::with_capacity(self.centroids.len());
        let mut indices = Vec::with_capacity(self.centroids.len() * 6);
        self.write_indices_for_camera(
            camera_position,
            point_of_view,
            &mut quad_distances,
            &mut indices,
        );
        indices
    }

    #[cfg(test)]
    fn indices_for_camera_if_needed(
        &mut self,
        camera_position: [f32; 3],
        point_of_view: [i8; 3],
        block_position_changed: bool,
        nearby: bool,
    ) -> Option<Vec<u32>> {
        self.needs_resort(point_of_view, block_position_changed, nearby)
            .then(|| self.indices_for_camera(camera_position, point_of_view))
    }

    fn write_sorted_indices(
        &self,
        camera_position: [f32; 3],
        quad_distances: &mut Vec<(u32, f32)>,
        indices: &mut Vec<u32>,
    ) {
        quad_distances.clear();
        quad_distances.reserve(self.centroids.len());
        quad_distances.extend(
            self.centroids
                .iter()
                .enumerate()
                .map(|(quad_index, centroid)| {
                    let dx = centroid[0] - camera_position[0];
                    let dy = centroid[1] - camera_position[1];
                    let dz = centroid[2] - camera_position[2];
                    (quad_index as u32, dx * dx + dy * dy + dz * dz)
                }),
        );
        quad_distances.sort_unstable_by(
            |(left_index, left_distance), (right_index, right_distance)| {
                right_distance
                    .total_cmp(left_distance)
                    .then_with(|| left_index.cmp(right_index))
            },
        );

        indices.clear();
        indices.reserve(quad_distances.len() * 6);
        for &(quad_index, _) in quad_distances.iter() {
            let base = quad_index * 4;
            indices.extend(
                self.local_indices[quad_index as usize]
                    .into_iter()
                    .map(|index| base + u32::from(index)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TerrainTranslucentSortState;
    use crate::gpu::TerrainGpuVertex;
    use crate::terrain::TerrainTint;

    fn terrain_vertex(position: [f32; 3]) -> TerrainGpuVertex {
        TerrainGpuVertex::pack(position, [0.0; 2], 0, TerrainTint::WHITE, 1.0, 1.0)
    }

    #[test]
    fn translucent_sort_state_rebuilds_vanilla_indices_after_camera_move() {
        let vertices = vec![
            terrain_vertex([0.0, 0.0, 0.0]),
            terrain_vertex([1.0, 0.0, 0.0]),
            terrain_vertex([1.0, 1.0, 0.0]),
            terrain_vertex([0.0, 1.0, 0.0]),
            terrain_vertex([0.0, 0.0, 1.0]),
            terrain_vertex([1.0, 0.0, 1.0]),
            terrain_vertex([1.0, 1.0, 1.0]),
            terrain_vertex([0.0, 1.0, 1.0]),
        ];
        let indices = vec![
            0, 2, 1, 0, 3, 2, // first quad uses BBB's reversed local winding
            4, 5, 6, 4, 6, 7,
        ];
        let mut sort =
            TerrainTranslucentSortState::from_vertices(&vertices, &indices, Some([0, 0, -1]))
                .expect("quad vertices and indices create sort state");

        assert_eq!(
            sort.indices_for_camera_if_needed([0.5, 0.5, -4.0], [0, 0, -1], false, true,),
            None
        );
        assert_eq!(
            sort.indices_for_camera_if_needed([0.5, 0.5, 4.0], [0, 0, 1], true, true),
            Some(vec![0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7])
        );
        assert_eq!(
            sort.indices_for_camera_if_needed([0.5, 0.5, -4.0], [0, 0, -1], true, true),
            Some(vec![4, 5, 6, 4, 6, 7, 0, 2, 1, 0, 3, 2])
        );
    }

    #[test]
    fn translucent_sort_state_uses_vanilla_block_and_point_of_view_triggers() {
        let vertices = vec![
            terrain_vertex([0.0, 0.0, 0.0]),
            terrain_vertex([1.0, 0.0, 0.0]),
            terrain_vertex([1.0, 1.0, 0.0]),
            terrain_vertex([0.0, 1.0, 0.0]),
        ];
        let mut sort = TerrainTranslucentSortState::from_vertices(
            &vertices,
            &[0, 1, 2, 2, 3, 0],
            Some([1, 1, 1]),
        )
        .expect("one translucent quad");

        assert_eq!(
            sort.indices_for_camera_if_needed([0.25, 0.5, 0.5], [1, 1, 1], false, true),
            None,
            "sub-block interpolation alone does not schedule a Vanilla resort"
        );
        assert_eq!(
            sort.indices_for_camera_if_needed([1.25, 0.5, 0.5], [1, 1, 1], true, false),
            None,
            "far, non-axis-aligned sections wait for their round-robin point-of-view change"
        );
        assert!(sort
            .indices_for_camera_if_needed([1.25, 0.5, 0.5], [0, 1, 1], true, false)
            .is_some());
        assert!(sort
            .indices_for_camera_if_needed([2.25, 0.5, 0.5], [0, 1, 1], true, false)
            .is_some());
    }

    #[test]
    fn write_indices_for_camera_reuses_scratch_buffers_and_preserves_tie_order() {
        let vertices = vec![
            terrain_vertex([0.0, 0.0, 0.0]),
            terrain_vertex([1.0, 0.0, 0.0]),
            terrain_vertex([1.0, 1.0, 0.0]),
            terrain_vertex([0.0, 1.0, 0.0]),
            terrain_vertex([0.0, 0.0, 0.0]),
            terrain_vertex([1.0, 0.0, 0.0]),
            terrain_vertex([1.0, 1.0, 0.0]),
            terrain_vertex([0.0, 1.0, 0.0]),
        ];
        let indices = vec![0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7];
        let mut sort = TerrainTranslucentSortState::from_vertices(&vertices, &indices, None)
            .expect("coincident quads");
        let mut scratch_distances = Vec::new();
        let mut scratch_indices = Vec::new();

        sort.write_indices_for_camera(
            [0.5, 0.5, 2.0],
            [0, 0, 1],
            &mut scratch_distances,
            &mut scratch_indices,
        );
        assert_eq!(scratch_indices, indices);
        let distances_ptr = scratch_distances.as_ptr();
        let indices_ptr = scratch_indices.as_ptr();

        sort.write_indices_for_camera(
            [0.5, 0.5, -2.0],
            [0, 0, -1],
            &mut scratch_distances,
            &mut scratch_indices,
        );
        assert_eq!(scratch_indices, indices);
        assert_eq!(scratch_distances.as_ptr(), distances_ptr);
        assert_eq!(scratch_indices.as_ptr(), indices_ptr);
    }

    #[test]
    fn compaction_map_remaps_offsets_with_prefix_sums() {
        let map = super::CompactionMap::from_sorted_free_ranges([(4, 2), (10, 5), (20, 3)]);
        assert_eq!(map.remap(0), 0);
        assert_eq!(map.remap(4), 4);
        assert_eq!(map.remap(6), 4);
        assert_eq!(map.remap(10), 8);
        assert_eq!(map.remap(15), 8);
        assert_eq!(map.remap(19), 12);
        assert_eq!(map.remap(23), 13);
        assert_eq!(map.remap(100), 90);
    }

    #[test]
    fn gpu_arena_uses_best_fit_coalescing_and_prefix_compaction() {
        let Some(renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        let mut arena = super::GpuArena::new("test-arena", wgpu::BufferUsages::VERTEX, 4, 16, 64);
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let mut retired = Vec::new();
        let a = arena
            .allocate(&renderer.device, &mut encoder, &mut retired, 10)
            .unwrap();
        let b = arena
            .allocate(&renderer.device, &mut encoder, &mut retired, 20)
            .unwrap();
        let c = arena
            .allocate(&renderer.device, &mut encoder, &mut retired, 10)
            .unwrap();
        let d = arena
            .allocate(&renderer.device, &mut encoder, &mut retired, 12)
            .unwrap();
        assert_eq!((a, b, c, d), (0, 10, 30, 40));

        // Free two holes: [10, 30) len 20 and [30, 40) len 10 -> coalesces to [10, 40) len 30.
        arena.release(b, 20, &mut retired);
        arena.release(c, 10, &mut retired);
        assert_eq!(arena.free_total, 30);
        assert!(arena.fits(30));
        assert!(arena.can_pack(42));

        // Split out [0, 10) len 10 as well, leaving holes len 10 at 0 and len 30 at 10?
        // Wait: [0, 10) is adjacent to [10, 40), so releasing a coalesces to [0, 40) len 40.
        arena.release(a, 10, &mut retired);
        assert_eq!(arena.free_total, 40);
        assert_eq!(arena.free_by_start.len(), 1);
        assert_eq!(arena.free_by_len.len(), 1);

        // Allocate 10 from [0, 40) -> [0, 10), remainder [10, 40) len 30.
        let e = arena
            .allocate(&renderer.device, &mut encoder, &mut retired, 10)
            .unwrap();
        assert_eq!(e, 0);
        // Allocate 20 -> best-fit takes [10, 30), remainder [30, 40) len 10.
        let f = arena
            .allocate(&renderer.device, &mut encoder, &mut retired, 20)
            .unwrap();
        assert_eq!(f, 10);
        // Compact squeezes out [30, 40), remapping d=40 -> 30.
        let compaction = arena.compact(&renderer.device, &mut encoder, &mut retired);
        assert_eq!(compaction.remap(e), 0);
        assert_eq!(compaction.remap(f), 10);
        assert_eq!(compaction.remap(d), 30);
        assert_eq!(arena.free_total, 0);
        assert_eq!(arena.end, 42);
    }

    #[test]
    fn render_bundle_cache_debounces_until_revision_stays_stable() {
        let mut cache = super::TerrainBundleCacheState::default();
        // Turning every frame keeps advancing draws_revision -> never triggers bundle build.
        for rev in 1..=5 {
            assert!(!cache.advance(rev));
        }
        // Stopping at revision 5 waits until TERRAIN_BUNDLE_STABLE_FRAMES total frames at rev 5.
        assert!(!cache.advance(5));
        assert!(cache.advance(5));
        assert!(cache.advance(5));

        cache.invalidate();
        assert!(!cache.advance(5));
        assert!(!cache.advance(5));
        assert!(cache.advance(5));
    }

    #[test]
    fn fade_only_updates_preserve_draws_and_bundle_cache() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        let owner = crate::TerrainSectionMeshOwner {
            chunk: crate::TerrainChunkMeshOwner {
                level_generation: 0,
                chunk_x: 0,
                chunk_z: 0,
            },
            section_y: 0,
        };
        let mesh = crate::terrain::TerrainMesh {
            vertices: vec![
                terrain_vertex([0.0, 0.0, 0.0]),
                terrain_vertex([1.0, 0.0, 0.0]),
                terrain_vertex([1.0, 1.0, 0.0]),
                terrain_vertex([0.0, 1.0, 0.0]),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            shared_quad_indices: true,
            opaque_faces: 1,
            cutout_faces: 0,
            translucent_faces: 0,
            culled_faces: 0,
            ..Default::default()
        };
        let layers = crate::terrain::TerrainSectionMeshLayers {
            opaque: vec![crate::terrain::TerrainSectionMesh { section_y: 0, mesh }],
            ..Default::default()
        };
        let mut uploads = super::StagedCommands::new(&renderer.device, "test-fade-uploads", 4096);
        let mut counters = crate::RendererCounters::default();
        renderer.terrain.upsert(
            &renderer.device,
            &mut uploads,
            &mut counters,
            owner,
            1,
            &layers,
            [0.0; 3],
        );
        uploads.submit(&renderer.queue);

        let initial_states = [crate::TerrainSectionRenderState {
            owner,
            visibility: 0.25,
            nearby: true,
        }];
        assert!(renderer.terrain.set_section_states_with_fades(
            1,
            1,
            &initial_states,
            &[(owner, 0.25)],
        ));
        assert!(renderer.terrain.draws_dirty);
        {
            let (encoder, staging) = uploads.record(&renderer.device);
            renderer
                .terrain
                .prepare_draws(&renderer.device, encoder, staging);
        }
        uploads.submit(&renderer.queue);
        assert!(!renderer.terrain.draws_dirty);
        let draws_revision_before = renderer.terrain.layers[0].draws_revision;
        for _ in 0..super::TERRAIN_BUNDLE_STABLE_FRAMES {
            renderer.terrain.layers[0]
                .bundle_cache
                .get_mut()
                .advance(draws_revision_before);
        }
        let stable_before = renderer.terrain.layers[0]
            .bundle_cache
            .get_mut()
            .stable_frames;
        assert_eq!(stable_before, super::TERRAIN_BUNDLE_STABLE_FRAMES);

        // Fade-only update on the same layout_revision updates TerrainFadeBuffer in place without
        // dirtying draw lists or resetting bundle_cache stability.
        let mid_states = [crate::TerrainSectionRenderState {
            owner,
            visibility: 0.75,
            nearby: true,
        }];
        assert!(renderer.terrain.set_section_states_with_fades(
            1,
            2,
            &mid_states,
            &[(owner, 0.75)],
        ));
        assert!(!renderer.terrain.draws_dirty);
        assert_eq!(renderer.terrain.fade(owner), Some(0.75));
        {
            let (encoder, staging) = uploads.record(&renderer.device);
            renderer
                .terrain
                .prepare_draws(&renderer.device, encoder, staging);
        }
        uploads.submit(&renderer.queue);
        assert_eq!(
            renderer.terrain.layers[0].draws_revision,
            draws_revision_before
        );
        assert_eq!(
            renderer.terrain.layers[0]
                .bundle_cache
                .get_mut()
                .stable_frames,
            stable_before
        );

        // Reaching full opacity (1.0) via fade-only update also preserves draws & bundle cache.
        let full_states = [crate::TerrainSectionRenderState {
            owner,
            visibility: 1.0,
            nearby: true,
        }];
        assert!(renderer.terrain.set_section_states_with_fades(
            1,
            3,
            &full_states,
            &[(owner, 1.0)],
        ));
        assert!(!renderer.terrain.draws_dirty);
        assert!(!renderer.terrain.has_fading);
        assert_eq!(renderer.terrain.fade(owner), Some(1.0));
        assert_eq!(
            renderer.terrain.layers[0].draws_revision,
            draws_revision_before
        );
    }
}
