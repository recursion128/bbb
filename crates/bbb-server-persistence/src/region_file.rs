//! Exact-version Anvil region media.
//!
//! The region file itself is deliberately kept as a small, synchronous media
//! primitive. Higher-level save orchestration decides when chunks are dirty;
//! this type validates the on-disk allocation table and reuses a bounded LRU
//! cache of open `.mca` handles and validated 8 KiB headers so repeated chunk
//! reads and writes in the same region avoid reopening files and re-parsing the
//! allocation table on every chunk operation.

use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, PoisonError, Weak,
    },
    time::SystemTime,
};

use bbb_protocol::decode_file_saved_nbt_summary;
#[cfg(test)]
use bbb_protocol::encode_named_nbt_summary;
use flate2::{
    read::{GzDecoder, ZlibDecoder},
    write::{GzEncoder, ZlibEncoder},
    Compression,
};
use rayon::prelude::*;

use super::atomic_file::{open_file_matches_path, sync_parent_dir};
use super::lz4_block::{compress_java_block, decompress_java_block};
use bbb_server_types::save::{NamedCompound, SaveError, SaveResult, WorldSaveLimits};

const SECTOR_BYTES: usize = 4096;
const HEADER_BYTES: usize = SECTOR_BYTES * 2;
const HEADER_ENTRIES: usize = 1024;
const EXTERNAL_STREAM_FLAG: u8 = 0x80;
const MAX_SECTOR_COUNT: usize = u8::MAX as usize;

/// Default bounded `.mca` handle and 8 KiB header cache size per world storage.
pub const DEFAULT_REGION_CACHE_CAPACITY: usize = 64;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RegionCompression {
    Gzip,
    #[default]
    Zlib,
    None,
    Lz4,
}

impl RegionCompression {
    fn id(self) -> u8 {
        match self {
            Self::Gzip => 1,
            Self::Zlib => 2,
            Self::None => 3,
            Self::Lz4 => 4,
        }
    }

    fn from_id(path: &Path, id: u8) -> SaveResult<Self> {
        match id {
            1 => Ok(Self::Gzip),
            2 => Ok(Self::Zlib),
            3 => Ok(Self::None),
            4 => Ok(Self::Lz4),
            _ => Err(SaveError::UnsupportedCompression {
                path: path.to_path_buf(),
                id,
            }),
        }
    }
}

struct OpenRegion {
    file: File,
    writable: bool,
    dirty: bool,
    /// `None` represents an existing 0-byte region file.
    header: Option<Header>,
    max_region_bytes: u64,
}

impl OpenRegion {
    fn is_coherent(&self, limits: WorldSaveLimits) -> bool {
        self.max_region_bytes == limits.max_region_bytes
    }

    /// Forces dirty `.mca` data and metadata to storage (`RegionFile.java:354-364`).
    fn sync_if_dirty(&mut self, path: &Path) -> SaveResult<bool> {
        if self.writable && self.dirty {
            self.file
                .sync_all()
                .map_err(|source| io_error("syncing region file", path, source))?;
            self.dirty = false;
            return Ok(true);
        }
        Ok(false)
    }

    fn sync_if_dirty_best_effort(&mut self) -> bool {
        if self.writable && self.dirty && self.file.sync_all().is_ok() {
            self.dirty = false;
            return true;
        }
        false
    }
}

type RegionSlotHandle = Arc<Mutex<Option<OpenRegion>>>;

struct CacheNode {
    path: PathBuf,
    slot: RegionSlotHandle,
    prev: Option<usize>,
    next: Option<usize>,
}

struct RegionCacheInner {
    capacity: usize,
    nodes: Vec<Option<CacheNode>>,
    free: Vec<usize>,
    index: HashMap<PathBuf, usize>,
    /// Weak registry of every currently live per-path slot so concurrent
    /// callers for the same `.mca` path always serialize on one mutex even
    /// under LRU eviction or explicit `evict_*`.
    live: HashMap<PathBuf, Weak<Mutex<Option<OpenRegion>>>>,
    head: Option<usize>,
    tail: Option<usize>,
    #[cfg(test)]
    hits: u64,
    #[cfg(test)]
    misses: u64,
    #[cfg(test)]
    syncs: Arc<AtomicU64>,
}

impl RegionCacheInner {
    fn find_oldest_idle(&self) -> Option<usize> {
        let mut cursor = self.head;
        while let Some(idx) = cursor {
            let node = self.nodes[idx].as_ref().expect("linked cache node");
            if Arc::strong_count(&node.slot) == 1 {
                return Some(idx);
            }
            cursor = node.next;
        }
        None
    }

    fn unlink(&mut self, idx: usize) {
        let node = self.nodes[idx].as_mut().expect("linked cache node");
        let prev = node.prev.take();
        let next = node.next.take();
        match prev {
            Some(prev_idx) => {
                self.nodes[prev_idx].as_mut().expect("prev node").next = next;
            }
            None => self.head = next,
        }
        match next {
            Some(next_idx) => {
                self.nodes[next_idx].as_mut().expect("next node").prev = prev;
            }
            None => self.tail = prev,
        }
    }

    fn push_tail(&mut self, idx: usize) {
        let old_tail = self.tail.replace(idx);
        let node = self.nodes[idx].as_mut().expect("cache node");
        node.prev = old_tail;
        node.next = None;
        match old_tail {
            Some(tail_idx) => {
                self.nodes[tail_idx].as_mut().expect("tail node").next = Some(idx);
            }
            None => self.head = Some(idx),
        }
    }

    fn remove_index(&mut self, idx: usize) -> Option<CacheNode> {
        self.nodes[idx].as_ref()?;
        self.unlink(idx);
        let node = self.nodes[idx].take().expect("checked occupied node");
        self.index.remove(&node.path);
        self.free.push(idx);
        Some(node)
    }

    fn insert(&mut self, path: PathBuf, slot: RegionSlotHandle) {
        let idx = if let Some(free_idx) = self.free.pop() {
            self.nodes[free_idx] = Some(CacheNode {
                path: path.clone(),
                slot,
                prev: None,
                next: None,
            });
            free_idx
        } else {
            let idx = self.nodes.len();
            self.nodes.push(Some(CacheNode {
                path: path.clone(),
                slot,
                prev: None,
                next: None,
            }));
            idx
        };
        self.index.insert(path, idx);
        self.push_tail(idx);
    }
}

impl Drop for RegionCacheInner {
    fn drop(&mut self) {
        self.index.clear();
        self.live.clear();
        self.head = None;
        self.tail = None;
        self.free.clear();
        for node in self.nodes.drain(..).flatten() {
            if close_slot(&node.slot) {
                #[cfg(test)]
                self.syncs.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

/// Bounded O(1) LRU cache of open `.mca` [`File`] handles and validated 8 KiB
/// allocation/timestamp headers.
#[derive(Clone)]
pub struct RegionFileCache {
    inner: Arc<Mutex<RegionCacheInner>>,
}

impl std::fmt::Debug for RegionFileCache {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        formatter
            .debug_struct("RegionFileCache")
            .field("capacity", &guard.capacity)
            .field("len", &guard.index.len())
            .finish()
    }
}

impl Default for RegionFileCache {
    fn default() -> Self {
        Self::new(DEFAULT_REGION_CACHE_CAPACITY)
    }
}

impl RegionFileCache {
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.clamp(1, DEFAULT_REGION_CACHE_CAPACITY);
        Self {
            inner: Arc::new(Mutex::new(RegionCacheInner {
                capacity: capacity.max(1),
                nodes: Vec::with_capacity(cap),
                free: Vec::new(),
                index: HashMap::with_capacity(cap),
                live: HashMap::with_capacity(cap),
                head: None,
                tail: None,
                #[cfg(test)]
                hits: 0,
                #[cfg(test)]
                misses: 0,
                #[cfg(test)]
                syncs: Arc::new(AtomicU64::new(0)),
            })),
        }
    }

    fn slot_for(&self, path: &Path) -> RegionSlotHandle {
        let (slot, evicted) = {
            let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            if let Some(&idx) = guard.index.get(path) {
                guard.unlink(idx);
                guard.push_tail(idx);
                let slot = Arc::clone(&guard.nodes[idx].as_ref().expect("indexed node").slot);
                (slot, Vec::new())
            } else {
                let mut evicted = Vec::new();
                while guard.index.len() >= guard.capacity {
                    let Some(victim) = guard.find_oldest_idle() else {
                        break;
                    };
                    if let Some(node) = guard.remove_index(victim) {
                        evicted.push(node);
                    }
                }
                let slot = if let Some(existing) = guard.live.get(path).and_then(Weak::upgrade) {
                    existing
                } else {
                    let slot = Arc::new(Mutex::new(None));
                    guard.live.insert(path.to_path_buf(), Arc::downgrade(&slot));
                    slot
                };
                guard.insert(path.to_path_buf(), Arc::clone(&slot));
                if guard.live.len() > guard.capacity.saturating_mul(2) {
                    guard.live.retain(|_, weak| weak.strong_count() > 0);
                }
                (slot, evicted)
            }
        };
        for node in evicted {
            if close_slot(&node.slot) {
                #[cfg(test)]
                self.record_sync();
            }
        }
        slot
    }

    #[cfg(test)]
    fn record_hit(&self) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .hits += 1;
    }

    #[cfg(test)]
    fn record_miss(&self) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .misses += 1;
    }

    #[cfg(test)]
    fn record_sync(&self) {
        self.inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .syncs
            .fetch_add(1, Ordering::Relaxed);
    }

    #[cfg(test)]
    pub(crate) fn stats(&self) -> (usize, u64, u64) {
        let guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        (guard.index.len(), guard.hits, guard.misses)
    }

    #[cfg(test)]
    pub(crate) fn sync_counter(&self) -> Arc<AtomicU64> {
        Arc::clone(
            &self
                .inner
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .syncs,
        )
    }

    /// Closes and removes a cached `.mca` handle for `path`, if open.
    pub fn evict_path(&self, path: &Path) {
        let removed = {
            let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            guard
                .index
                .get(path)
                .copied()
                .and_then(|idx| guard.remove_index(idx))
        };
        if let Some(node) = removed {
            if close_slot(&node.slot) {
                #[cfg(test)]
                self.record_sync();
            }
        }
    }

    fn take_matching_under(&self, root: &Path) -> Vec<CacheNode> {
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let indices: Vec<usize> = guard
            .index
            .iter()
            .filter(|(path, _)| path.starts_with(root))
            .map(|(_, &idx)| idx)
            .collect();
        indices
            .into_iter()
            .filter_map(|idx| guard.remove_index(idx))
            .collect()
    }

    /// Closes and removes every cached `.mca` handle under `root` (including
    /// `root` itself) before directory rename, swap, or delete operations.
    pub fn evict_under(&self, root: &Path) {
        for node in self.take_matching_under(root) {
            if close_slot(&node.slot) {
                #[cfg(test)]
                self.record_sync();
            }
        }
    }

    /// Closes and removes every cached `.mca` handle in this cache.
    pub fn evict_all(&self) {
        let removed = {
            let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
            guard.index.clear();
            guard.head = None;
            guard.tail = None;
            guard.free.clear();
            std::mem::take(&mut guard.nodes)
        };
        for node in removed.into_iter().flatten() {
            if close_slot(&node.slot) {
                #[cfg(test)]
                self.record_sync();
            }
        }
    }

    /// Syncs and closes every cached `.mca` handle under `root`.
    pub fn flush_and_evict_under(&self, root: &Path) -> SaveResult<()> {
        let removed = self.take_matching_under(root);
        let mut first_error = None;
        for node in removed {
            match sync_and_close_slot(&node.path, &node.slot) {
                Ok(synced) => {
                    if synced {
                        #[cfg(test)]
                        self.record_sync();
                    }
                }
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

fn close_slot(slot: &RegionSlotHandle) -> bool {
    let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    guard
        .take()
        .is_some_and(|mut open| open.sync_if_dirty_best_effort())
}

fn sync_and_close_slot(path: &Path, slot: &RegionSlotHandle) -> SaveResult<bool> {
    let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
    match guard.take() {
        Some(mut open) => open.sync_if_dirty(path),
        None => Ok(false),
    }
}

#[derive(Debug, Clone)]
pub struct RegionFileStore {
    path: PathBuf,
    external_dir: PathBuf,
    limits: WorldSaveLimits,
    operation_lock: Arc<Mutex<()>>,
    cache: RegionFileCache,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionOccupiedLocalSlot {
    pub local_x: u8,
    pub local_z: u8,
    pub timestamp: u32,
}

impl RegionFileStore {
    pub fn new(
        path: impl Into<PathBuf>,
        external_dir: impl Into<PathBuf>,
        limits: WorldSaveLimits,
    ) -> SaveResult<Self> {
        Self::with_cache(path, external_dir, limits, RegionFileCache::new(1))
    }

    pub fn with_cache(
        path: impl Into<PathBuf>,
        external_dir: impl Into<PathBuf>,
        limits: WorldSaveLimits,
        cache: RegionFileCache,
    ) -> SaveResult<Self> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return Err(SaveError::InvalidPath {
                path,
                detail: "region file path must not be empty",
            });
        }
        let external_dir = external_dir.into();
        if external_dir.as_os_str().is_empty() {
            return Err(SaveError::InvalidPath {
                path: external_dir,
                detail: "external chunk directory must not be empty",
            });
        }
        let operation_lock = Arc::new(Mutex::new(()));
        Ok(Self {
            path,
            external_dir,
            limits: limits.validate()?,
            operation_lock,
            cache,
        })
    }

    pub fn close_cached(&self) {
        self.cache.evict_path(&self.path);
    }

    /// Validates that an existing `.mca` file at this path is not a symbolic
    /// link and has a well-formed 8 KiB allocation header within save budgets.
    /// Missing or 0-byte region files are accepted as empty.
    pub fn validate_existing_header(&self) -> SaveResult<()> {
        let _operation = self
            .operation_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.with_cached_existing_mode(false, |_| Ok(()))
    }

    /// Validates an existing region file header while opening the handle
    /// read-write so an immediately following write reuses the cached descriptor.
    pub(crate) fn validate_existing_header_for_write(&self) -> SaveResult<()> {
        let _operation = self
            .operation_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.with_cached_existing_mode(true, |_| Ok(()))
    }

    pub fn read_chunk(&self, global_x: i32, global_z: i32) -> SaveResult<Option<NamedCompound>> {
        let _operation = self
            .operation_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.read_chunk_locked(global_x, global_z)
            .map(|loaded| loaded.map(|(document, _, _)| document))
    }

    pub fn read_chunk_with_decompressed_size(
        &self,
        global_x: i32,
        global_z: i32,
    ) -> SaveResult<Option<(NamedCompound, usize)>> {
        let _operation = self
            .operation_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.read_chunk_locked(global_x, global_z)
            .map(|loaded| loaded.map(|(document, decompressed, _)| (document, decompressed)))
    }

    pub fn read_chunk_with_timestamp(
        &self,
        global_x: i32,
        global_z: i32,
    ) -> SaveResult<Option<(NamedCompound, u32)>> {
        let _operation = self
            .operation_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.read_chunk_locked(global_x, global_z)
            .map(|loaded| loaded.map(|(document, _, timestamp)| (document, timestamp)))
    }

    /// Reads one chunk while keeping `.mca` open/header errors in the outer
    /// `Result` and per-slot record decode errors in the inner `Result`.
    pub(crate) fn read_chunk_with_timestamp_distinguishing_header(
        &self,
        global_x: i32,
        global_z: i32,
    ) -> SaveResult<SaveResult<Option<(NamedCompound, u32)>>> {
        let _operation = self
            .operation_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.read_chunk_locked_split(global_x, global_z)
            .map(|inner| inner.map(|loaded| loaded.map(|(document, _, ts)| (document, ts))))
    }

    /// Reads occupied `(local_x, local_z)` chunk coordinates directly from a
    /// region file header without placing the file handle into [`RegionFileCache`].
    pub(crate) fn occupied_local_coords_uncached(
        path: &Path,
        limits: WorldSaveLimits,
    ) -> SaveResult<Vec<(u8, u8)>> {
        let mut file = match open_nofollow(path, true, false, false) {
            Ok(file) if open_file_matches_path(&file, path) => file,
            Ok(_) => {
                return Err(invalid_path(
                    path,
                    "region file changed or became a symbolic link while opening",
                ))
            }
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(io_error("opening region file", path, source)),
        };
        let len = file
            .metadata()
            .map_err(|source| io_error("reading region metadata", path, source))?
            .len();
        if len == 0 {
            return Ok(Vec::new());
        }
        let header = Header::read_with_len(&mut file, path, limits, len)?;
        Ok(header
            .locations
            .iter()
            .enumerate()
            .filter_map(|(index, location)| {
                location.map(|_| ((index % 32) as u8, (index / 32) as u8))
            })
            .collect())
    }

    pub fn occupied_local_slots(&self) -> SaveResult<Vec<RegionOccupiedLocalSlot>> {
        let _operation = self
            .operation_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        self.with_cached_existing(|open| {
            let Some(open) = open else {
                return Ok(Vec::new());
            };
            let Some(header) = open.header.as_ref() else {
                return Ok(Vec::new());
            };
            Ok(header
                .locations
                .iter()
                .enumerate()
                .filter_map(|(index, location)| {
                    location.map(|_| RegionOccupiedLocalSlot {
                        local_x: (index % 32) as u8,
                        local_z: (index / 32) as u8,
                        timestamp: header.timestamps[index],
                    })
                })
                .collect())
        })
    }

    fn with_cached_existing<T>(
        &self,
        op: impl FnOnce(Option<&mut OpenRegion>) -> SaveResult<T>,
    ) -> SaveResult<T> {
        self.with_cached_existing_mode(false, op)
    }

    fn with_cached_existing_mode<T>(
        &self,
        require_writable: bool,
        op: impl FnOnce(Option<&mut OpenRegion>) -> SaveResult<T>,
    ) -> SaveResult<T> {
        let slot = self.cache.slot_for(&self.path);
        let mut guard = slot.lock().unwrap_or_else(PoisonError::into_inner);
        let hit = guard.as_ref().is_some_and(|open| {
            open.is_coherent(self.limits) && (!require_writable || open.writable)
        });
        if hit {
            #[cfg(test)]
            self.cache.record_hit();
        } else {
            #[cfg(test)]
            self.cache.record_miss();
            if let Some(mut stale) = guard.take() {
                if stale.sync_if_dirty(&self.path)? {
                    #[cfg(test)]
                    self.cache.record_sync();
                }
            }
            let opened = if require_writable {
                self.open_existing_writable_uncached()?
            } else {
                self.open_existing_uncached()?
            };
            if let Some(mut file) = opened {
                let len = file
                    .metadata()
                    .map_err(|source| io_error("reading region metadata", &self.path, source))?
                    .len();
                let header = if len == 0 {
                    None
                } else {
                    Some(Header::read_with_len(
                        &mut file,
                        &self.path,
                        self.limits,
                        len,
                    )?)
                };
                *guard = Some(OpenRegion {
                    file,
                    writable: require_writable,
                    dirty: false,
                    header,
                    max_region_bytes: self.limits.max_region_bytes,
                });
            }
        }
        op(guard.as_mut())
    }

    fn read_chunk_locked(
        &self,
        global_x: i32,
        global_z: i32,
    ) -> SaveResult<Option<(NamedCompound, usize, u32)>> {
        self.read_chunk_locked_split(global_x, global_z)?
    }

    fn read_chunk_locked_split(
        &self,
        global_x: i32,
        global_z: i32,
    ) -> SaveResult<SaveResult<Option<(NamedCompound, usize, u32)>>> {
        enum CompressedPayload {
            Inline(Slot),
            External(Vec<u8>),
        }
        impl CompressedPayload {
            fn as_slice(&self) -> &[u8] {
                match self {
                    Self::Inline(slot) => &slot.buf[5..4 + slot.payload_len],
                    Self::External(bytes) => bytes.as_slice(),
                }
            }
        }

        // Hold the slot mutex across reading both the `.mca` slot and any
        // external `.mcc` sidecar so a concurrent writer cannot replace or
        // delete the sidecar before it is read.
        let loaded_slot = self.with_cached_existing(|open| {
            let Some(open) = open else {
                return Ok(Ok(None));
            };
            let Some(header) = open.header.as_mut() else {
                return Ok(Ok(None));
            };
            let index = chunk_index(global_x, global_z);
            let Some(location) = header.locations[index] else {
                return Ok(Ok(None));
            };
            let timestamp = header.timestamps[index];
            let slot_result: SaveResult<_> = (|| {
                let slot = read_slot(&mut open.file, &self.path, location)?;
                let external = slot.compression & EXTERNAL_STREAM_FLAG != 0;
                header.external[index] = Some(external);
                let compression = RegionCompression::from_id(
                    &self.path,
                    slot.compression & !EXTERNAL_STREAM_FLAG,
                )?;
                let payload = if external {
                    if slot.payload_len != 1 {
                        return Err(invalid_region(
                            &self.path,
                            "external chunk slot must contain only its compression byte",
                        ));
                    }
                    CompressedPayload::External(self.read_external(global_x, global_z)?)
                } else {
                    CompressedPayload::Inline(slot)
                };
                Ok(Some((compression, payload, timestamp)))
            })();
            Ok(slot_result)
        })?;
        let (compression, payload, timestamp) = match loaded_slot {
            Ok(Some(loaded)) => loaded,
            Ok(None) => return Ok(Ok(None)),
            Err(error) => return Ok(Err(error)),
        };
        let decode_result: SaveResult<_> = (|| {
            let compressed = payload.as_slice();
            check_budget(
                "compressed region chunk",
                compressed.len(),
                self.limits.max_chunk_compressed_bytes,
            )?;
            let bytes = decompress(
                &self.path,
                compression,
                compressed,
                self.limits.max_chunk_decompressed_bytes,
            )?;
            let decompressed_bytes = bytes.len();
            decode_document(&self.path, &bytes)
                .map(|document| Some((document, decompressed_bytes, timestamp)))
        })();
        Ok(decode_result)
    }
}

/// A serialized, compressed region chunk payload ready to write to a `.mca`
/// (or sidecar `.mcc`) without touching the filesystem during compression.
#[derive(Debug, Clone)]
pub struct PreparedRegionChunk {
    compressed: Vec<u8>,
    external: bool,
    required_sectors: usize,
}

/// Encodes and compresses a single region chunk document without touching disk.
pub fn prepare_chunk_payload(
    path: &Path,
    limits: WorldSaveLimits,
    document: &NamedCompound,
    compression: RegionCompression,
) -> SaveResult<PreparedRegionChunk> {
    let raw = super::level_file::encode_saved_compound_nbt(
        path,
        document,
        limits.max_chunk_decompressed_bytes,
    )?;
    check_budget(
        "decompressed region chunk",
        raw.len(),
        limits.max_chunk_decompressed_bytes,
    )?;
    let compressed = compress(path, compression, &raw)?;
    check_budget(
        "compressed region chunk",
        compressed.len(),
        limits.max_chunk_compressed_bytes,
    )?;

    // The on-region payload is length + compression byte + stream. A
    // stream requiring 256 sectors is represented by a one-sector stub;
    // Vanilla stores its bytes beside the region as an .mcc file.
    let inline_bytes = compressed
        .len()
        .checked_add(5)
        .ok_or_else(|| invalid_region(path, "chunk payload length overflow"))?;
    let external = sectors_for(inline_bytes) > MAX_SECTOR_COUNT;
    let required_sectors = if external {
        1
    } else {
        sectors_for(inline_bytes)
    };
    Ok(PreparedRegionChunk {
        compressed,
        external,
        required_sectors,
    })
}

/// Encodes and compresses a batch of region chunk documents in parallel using
/// `rayon` when more than one document is present.
pub fn prepare_chunks_parallel(
    items: &[(&Path, WorldSaveLimits, &NamedCompound, RegionCompression)],
) -> SaveResult<Vec<PreparedRegionChunk>> {
    if items.len() <= 1 {
        return items
            .iter()
            .map(|(path, limits, document, compression)| {
                prepare_chunk_payload(path, *limits, document, *compression)
            })
            .collect();
    }
    items
        .par_iter()
        .map(|(path, limits, document, compression)| {
            prepare_chunk_payload(path, *limits, document, *compression)
        })
        .collect()
}

impl RegionFileStore {
    pub fn write_chunk(
        &self,
        global_x: i32,
        global_z: i32,
        document: &NamedCompound,
        compression: RegionCompression,
        timestamp: u32,
    ) -> SaveResult<()> {
        let prepared = prepare_chunk_payload(&self.path, self.limits, document, compression)?;
        self.write_prepared_chunk(global_x, global_z, prepared, compression, timestamp)
    }

    pub fn write_prepared_chunk(
        &self,
        global_x: i32,
        global_z: i32,
        prepared: PreparedRegionChunk,
        compression: RegionCompression,
        timestamp: u32,
    ) -> SaveResult<()> {
        let _operation = self
            .operation_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let PreparedRegionChunk {
            compressed,
            external,
            required_sectors,
        } = prepared;
        check_budget(
            "compressed region chunk",
            compressed.len(),
            self.limits.max_chunk_compressed_bytes,
        )?;

        let slot_handle = self.cache.slot_for(&self.path);
        let mut guard = slot_handle.lock().unwrap_or_else(PoisonError::into_inner);
        let hit = guard.as_ref().is_some_and(|open| {
            open.writable && open.header.is_some() && open.is_coherent(self.limits)
        });
        if hit {
            #[cfg(test)]
            self.cache.record_hit();
        } else {
            #[cfg(test)]
            self.cache.record_miss();
            if let Some(mut stale) = guard.take() {
                if stale.sync_if_dirty(&self.path)? {
                    #[cfg(test)]
                    self.cache.record_sync();
                }
            }
            let (mut file, initialized) = self.open_or_create_uncached()?;
            let mut dirty = initialized;
            let mut len = file
                .metadata()
                .map_err(|source| io_error("reading region metadata", &self.path, source))?
                .len();
            if len >= HEADER_BYTES as u64 && !len.is_multiple_of(SECTOR_BYTES as u64) {
                let padded_len = len
                    .div_ceil(SECTOR_BYTES as u64)
                    .checked_mul(SECTOR_BYTES as u64)
                    .ok_or_else(|| invalid_region(&self.path, "region length overflow"))?;
                if padded_len > self.limits.max_region_bytes {
                    return Err(SaveError::BudgetExceeded {
                        resource: "region file",
                        actual: padded_len,
                        limit: self.limits.max_region_bytes,
                    });
                }
                file.set_len(padded_len).map_err(|source| {
                    io_error("padding trailing region sector", &self.path, source)
                })?;
                dirty = true;
                len = padded_len;
            }
            let header = Header::read_with_len(&mut file, &self.path, self.limits, len)?;
            *guard = Some(OpenRegion {
                file,
                writable: true,
                dirty,
                header: Some(header),
                max_region_bytes: self.limits.max_region_bytes,
            });
        }

        let write_result = (|| {
            let open = guard.as_mut().expect("writable region initialized");
            let header = open.header.as_mut().expect("writable region has header");
            let index = chunk_index(global_x, global_z);
            let previous_location = header.locations[index];
            let previous_timestamp = header.timestamps[index];
            let previous_sector_count = header.sector_count;
            let was_external = match previous_location {
                Some(prev) if !external => {
                    header.slot_is_external(&mut open.file, &self.path, index, prev)?
                }
                _ => false,
            };
            let location = header.allocate(required_sectors, self.limits, &self.path)?;
            let staged_external = match if external {
                check_budget(
                    "external region chunk",
                    compressed.len(),
                    self.limits.max_external_chunk_bytes,
                )?;
                Some(self.stage_external(global_x, global_z, &compressed))
            } else {
                None
            } {
                Some(Ok(staged)) => Some(staged),
                Some(Err(error)) => {
                    header.sector_count = previous_sector_count;
                    return Err(error);
                }
                None => None,
            };

            let mut slot = vec![0_u8; required_sectors * SECTOR_BYTES];
            let payload_len = if external { 1 } else { compressed.len() + 1 };
            slot[..4].copy_from_slice(&(payload_len as u32).to_be_bytes());
            slot[4] = compression.id() | if external { EXTERNAL_STREAM_FLAG } else { 0 };
            if !external {
                slot[5..5 + compressed.len()].copy_from_slice(&compressed);
            }
            open.file
                .seek(SeekFrom::Start(
                    location.offset as u64 * SECTOR_BYTES as u64,
                ))
                .map_err(|source| io_error("seeking region chunk", &self.path, source))?;
            open.file
                .write_all(&slot)
                .map_err(|source| io_error("writing region chunk", &self.path, source))?;
            open.dirty = true;

            // Publish staged external `.mcc` before the `.mca` header points to it
            // so a crash between steps never leaves a dangling external reference.
            if let Some(staged_external) = staged_external {
                if let Err(error) = staged_external.publish() {
                    header.sector_count = previous_sector_count;
                    return Err(error);
                }
            }

            header.locations[index] = Some(location);
            header.timestamps[index] = timestamp;
            header.external[index] = Some(external);
            if let Err(error) = header.write_entry(&mut open.file, &self.path, index) {
                header.locations[index] = previous_location;
                header.timestamps[index] = previous_timestamp;
                header.sector_count = previous_sector_count;
                let _ = header.write_entry(&mut open.file, &self.path, index);
                return Err(error);
            }
            header.truncate_trailing_free(&mut open.file, &self.path)?;
            if !external && was_external {
                self.remove_external_if_exists(global_x, global_z)?;
            }
            Ok(())
        })();

        if write_result.is_err() {
            *guard = None;
        }
        write_result
    }

    pub fn clear_chunk(&self, global_x: i32, global_z: i32) -> SaveResult<()> {
        let _operation = self
            .operation_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let slot_handle = self.cache.slot_for(&self.path);
        let mut guard = slot_handle.lock().unwrap_or_else(PoisonError::into_inner);
        let hit = guard
            .as_ref()
            .is_some_and(|open| open.writable && open.is_coherent(self.limits));
        if hit {
            #[cfg(test)]
            self.cache.record_hit();
        } else {
            #[cfg(test)]
            self.cache.record_miss();
            if let Some(mut stale) = guard.take() {
                if stale.sync_if_dirty(&self.path)? {
                    #[cfg(test)]
                    self.cache.record_sync();
                }
            }
            let Some(mut file) = self.open_existing_writable_uncached()? else {
                return Ok(());
            };
            let len = file
                .metadata()
                .map_err(|source| io_error("reading region metadata", &self.path, source))?
                .len();
            if len == 0 {
                return Ok(());
            }
            let header = Header::read_with_len(&mut file, &self.path, self.limits, len)?;
            *guard = Some(OpenRegion {
                file,
                writable: true,
                dirty: false,
                header: Some(header),
                max_region_bytes: self.limits.max_region_bytes,
            });
        }

        let clear_result = (|| {
            let Some(open) = guard.as_mut() else {
                return Ok(());
            };
            let Some(header) = open.header.as_mut() else {
                return Ok(());
            };
            let index = chunk_index(global_x, global_z);
            let Some(previous_location) = header.locations[index] else {
                return Ok(());
            };
            let was_external =
                header.slot_is_external(&mut open.file, &self.path, index, previous_location)?;
            header.locations[index] = None;
            header.timestamps[index] = current_timestamp();
            header.external[index] = None;
            header.write_entry(&mut open.file, &self.path, index)?;
            open.dirty = true;
            header.truncate_trailing_free(&mut open.file, &self.path)?;
            if was_external {
                self.remove_external_if_exists(global_x, global_z)?;
            }
            Ok(())
        })();

        if clear_result.is_err() {
            *guard = None;
        }
        clear_result
    }

    /// Region writes are not synced; Vanilla forces the file only on an explicit
    /// flush or close (`RegionFile.java:269-271`, `RegionFileStorage.java:96-100`).
    pub fn flush(&self) -> SaveResult<()> {
        let _operation = self
            .operation_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let slot_handle = self.cache.slot_for(&self.path);
        let mut guard = slot_handle.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(open) = guard.as_mut() {
            if open.is_coherent(self.limits) {
                #[cfg(test)]
                self.cache.record_hit();
                if open.sync_if_dirty(&self.path)? {
                    #[cfg(test)]
                    self.cache.record_sync();
                }
                return Ok(());
            }
            if let Some(mut stale) = guard.take() {
                if stale.sync_if_dirty(&self.path)? {
                    #[cfg(test)]
                    self.cache.record_sync();
                }
            }
        }
        #[cfg(test)]
        self.cache.record_miss();
        match self.open_existing_writable_uncached()? {
            Some(file) => {
                file.sync_all()
                    .map_err(|source| io_error("syncing region file", &self.path, source))?;
                #[cfg(test)]
                self.cache.record_sync();
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn open_existing_uncached(&self) -> SaveResult<Option<File>> {
        match open_nofollow(&self.path, true, false, false) {
            Ok(file) if open_file_matches_path(&file, &self.path) => Ok(Some(file)),
            Ok(_) => Err(invalid_path(
                &self.path,
                "region file changed or became a symbolic link while opening",
            )),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(io_error("opening region file", &self.path, source)),
        }
    }

    fn open_or_create_uncached(&self) -> SaveResult<(File, bool)> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|source| io_error("creating region directory", parent, source))?;
        }
        reject_existing_symlink(&self.path)?;
        let mut file = open_nofollow(&self.path, true, true, true)
            .map_err(|source| io_error("opening region file", &self.path, source))?;
        if !open_file_matches_path(&file, &self.path) {
            return Err(invalid_path(
                &self.path,
                "region file changed or became a symbolic link while opening",
            ));
        }
        let mut initialized = false;
        if file
            .metadata()
            .map_err(|source| io_error("reading region metadata", &self.path, source))?
            .len()
            == 0
        {
            file.write_all(&[0; HEADER_BYTES])
                .map_err(|source| io_error("initializing region header", &self.path, source))?;
            initialized = true;
        }
        Ok((file, initialized))
    }

    fn open_existing_writable_uncached(&self) -> SaveResult<Option<File>> {
        match open_nofollow(&self.path, true, true, false) {
            Ok(file) if open_file_matches_path(&file, &self.path) => Ok(Some(file)),
            Ok(_) => Err(invalid_path(
                &self.path,
                "region file changed or became a symbolic link while opening",
            )),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(io_error("opening writable region file", &self.path, source)),
        }
    }

    fn external_path(&self, global_x: i32, global_z: i32) -> PathBuf {
        self.external_dir
            .join(format!("c.{global_x}.{global_z}.mcc"))
    }

    fn read_external(&self, global_x: i32, global_z: i32) -> SaveResult<Vec<u8>> {
        let path = self.external_path(global_x, global_z);
        reject_existing_symlink(&path)?;
        let file = open_nofollow(&path, true, false, false)
            .map_err(|source| io_error("opening external chunk", &path, source))?;
        if !open_file_matches_path(&file, &path) {
            return Err(invalid_path(
                &path,
                "external chunk changed or became a symbolic link while opening",
            ));
        }
        let metadata = file
            .metadata()
            .map_err(|source| io_error("reading external chunk metadata", &path, source))?;
        if metadata.len() > self.limits.max_external_chunk_bytes as u64 {
            return Err(SaveError::BudgetExceeded {
                resource: "external region chunk",
                actual: metadata.len(),
                limit: self.limits.max_external_chunk_bytes as u64,
            });
        }
        if metadata.len() > self.limits.max_chunk_compressed_bytes as u64 {
            return Err(SaveError::BudgetExceeded {
                resource: "compressed region chunk",
                actual: metadata.len(),
                limit: self.limits.max_chunk_compressed_bytes as u64,
            });
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        let read_limit = (self.limits.max_external_chunk_bytes as u64).saturating_add(1);
        file.take(read_limit)
            .read_to_end(&mut bytes)
            .map_err(|source| io_error("reading external chunk", &path, source))?;
        check_budget(
            "external region chunk",
            bytes.len(),
            self.limits.max_external_chunk_bytes,
        )?;
        Ok(bytes)
    }

    fn stage_external(
        &self,
        global_x: i32,
        global_z: i32,
        bytes: &[u8],
    ) -> SaveResult<StagedExternal> {
        fs::create_dir_all(&self.external_dir).map_err(|source| {
            io_error(
                "creating external chunk directory",
                &self.external_dir,
                source,
            )
        })?;
        let path = self.external_path(global_x, global_z);
        let temp = self.external_dir.join(format!(
            ".{}.{}.{}.tmp",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("chunk"),
            std::process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let write_result: SaveResult<StagedExternal> = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|source| io_error("creating staged external chunk", &temp, source))?;
            file.write_all(bytes)
                .map_err(|source| io_error("writing staged external chunk", &temp, source))?;
            file.sync_all()
                .map_err(|source| io_error("syncing staged external chunk", &temp, source))?;
            Ok(StagedExternal {
                temp: temp.clone(),
                target: path,
                published: false,
            })
        })();
        if write_result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        write_result
    }

    fn remove_external_if_exists(&self, global_x: i32, global_z: i32) -> SaveResult<()> {
        let path = self.external_path(global_x, global_z);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(io_error("removing external chunk", &path, source)),
        }
    }
}

fn open_nofollow(path: &Path, read: bool, write: bool, create: bool) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(read).write(write).create(create);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    options.open(path)
}

fn reject_existing_symlink(path: &Path) -> SaveResult<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(invalid_path(
            path,
            "managed region path must not be a symbolic link",
        )),
        Ok(_) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error("reading managed region metadata", path, source)),
    }
}

fn invalid_path(path: &Path, detail: &'static str) -> SaveError {
    SaveError::InvalidPath {
        path: path.to_path_buf(),
        detail,
    }
}

struct StagedExternal {
    temp: PathBuf,
    target: PathBuf,
    published: bool,
}

impl StagedExternal {
    fn publish(mut self) -> SaveResult<()> {
        fs::rename(&self.temp, &self.target)
            .map_err(|source| io_error("publishing external chunk", &self.target, source))?;
        self.published = true;
        sync_parent_dir(&self.target);
        Ok(())
    }
}

impl Drop for StagedExternal {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_file(&self.temp);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Location {
    offset: u32,
    count: u8,
}

#[derive(Debug)]
struct Header {
    locations: [Option<Location>; HEADER_ENTRIES],
    timestamps: [u32; HEADER_ENTRIES],
    external: [Option<bool>; HEADER_ENTRIES],
    sector_count: usize,
    occupied_scratch: Vec<bool>,
}

impl Header {
    #[cfg(test)]
    fn read(file: &mut File, path: &Path, limits: WorldSaveLimits) -> SaveResult<Self> {
        let length = file
            .metadata()
            .map_err(|source| io_error("reading region metadata", path, source))?
            .len();
        Self::read_with_len(file, path, limits, length)
    }

    fn read_with_len(
        file: &mut File,
        path: &Path,
        limits: WorldSaveLimits,
        length: u64,
    ) -> SaveResult<Self> {
        if length > limits.max_region_bytes {
            return Err(SaveError::BudgetExceeded {
                resource: "region file",
                actual: length,
                limit: limits.max_region_bytes,
            });
        }
        if length < HEADER_BYTES as u64 {
            return Err(invalid_region(path, "truncated 8192-byte header"));
        }
        let padded_len = length
            .div_ceil(SECTOR_BYTES as u64)
            .checked_mul(SECTOR_BYTES as u64)
            .ok_or_else(|| invalid_region(path, "region length overflow"))?;
        if padded_len > limits.max_region_bytes {
            return Err(SaveError::BudgetExceeded {
                resource: "region file",
                actual: padded_len,
                limit: limits.max_region_bytes,
            });
        }
        let sector_count = usize::try_from(padded_len / SECTOR_BYTES as u64)
            .map_err(|_| invalid_region(path, "region file is too large for this platform"))?;
        let mut raw = [0_u8; HEADER_BYTES];
        file.seek(SeekFrom::Start(0))
            .map_err(|source| io_error("seeking region header", path, source))?;
        file.read_exact(&mut raw)
            .map_err(|source| io_error("reading region header", path, source))?;
        let mut locations = [None; HEADER_ENTRIES];
        let mut timestamps = [0; HEADER_ENTRIES];
        let mut occupied = vec![false; sector_count];
        occupied[0] = true;
        occupied[1] = true;
        for index in 0..HEADER_ENTRIES {
            let encoded = read_be_u32(&raw, index * 4);
            if encoded != 0 {
                let offset = encoded >> 8;
                let count = encoded as u8;
                if offset < 2 || count == 0 {
                    return Err(invalid_region(
                        path,
                        "location has an invalid sector offset or count",
                    ));
                }
                let end = (offset as usize)
                    .checked_add(count as usize)
                    .ok_or_else(|| invalid_region(path, "location sector range overflow"))?;
                if end > sector_count {
                    return Err(invalid_region(path, "location extends beyond region file"));
                }
                for slot in &mut occupied[offset as usize..end] {
                    if *slot {
                        return Err(invalid_region(path, "location sectors overlap"));
                    }
                    *slot = true;
                }
                locations[index] = Some(Location { offset, count });
            }
            timestamps[index] = read_be_u32(&raw, SECTOR_BYTES + index * 4);
        }
        Ok(Self {
            locations,
            timestamps,
            external: [None; HEADER_ENTRIES],
            sector_count,
            occupied_scratch: occupied,
        })
    }

    fn slot_is_external(
        &mut self,
        file: &mut File,
        path: &Path,
        index: usize,
        location: Location,
    ) -> SaveResult<bool> {
        if let Some(external) = self.external[index] {
            return Ok(external);
        }
        if location.count != 1 {
            self.external[index] = Some(false);
            return Ok(false);
        }
        let flag_offset = location.offset as u64 * SECTOR_BYTES as u64 + 4;
        file.seek(SeekFrom::Start(flag_offset))
            .map_err(|source| io_error("seeking region chunk compression byte", path, source))?;
        let mut compression = [0_u8; 1];
        file.read_exact(&mut compression)
            .map_err(|source| io_error("reading region chunk compression byte", path, source))?;
        let external = (compression[0] & EXTERNAL_STREAM_FLAG) != 0;
        self.external[index] = Some(external);
        Ok(external)
    }

    fn used_sector_count(&self) -> usize {
        self.locations
            .iter()
            .flatten()
            .map(|location| location.offset as usize + location.count as usize)
            .max()
            .unwrap_or(2)
            .max(2)
    }

    fn truncate_trailing_free(&mut self, file: &mut File, path: &Path) -> SaveResult<()> {
        let used = self.used_sector_count();
        if used < self.sector_count {
            let new_len = (used as u64)
                .checked_mul(SECTOR_BYTES as u64)
                .ok_or_else(|| invalid_region(path, "region length overflow"))?;
            file.set_len(new_len)
                .map_err(|source| io_error("truncating trailing free sectors", path, source))?;
            self.sector_count = used;
        }
        Ok(())
    }

    fn allocate(
        &mut self,
        needed: usize,
        limits: WorldSaveLimits,
        path: &Path,
    ) -> SaveResult<Location> {
        if needed == 0 || needed > MAX_SECTOR_COUNT {
            return Err(invalid_region(
                path,
                "chunk allocation exceeds Anvil sector-count limit",
            ));
        }
        self.occupied_scratch.clear();
        self.occupied_scratch.resize(self.sector_count, false);
        self.occupied_scratch[0] = true;
        self.occupied_scratch[1] = true;
        for location in self.locations.iter().flatten() {
            let start = location.offset as usize;
            let end = start + location.count as usize;
            self.occupied_scratch[start..end].fill(true);
        }
        if let Some(start) = find_run(&self.occupied_scratch, needed) {
            return Ok(Location {
                offset: start as u32,
                count: needed as u8,
            });
        }
        let trailing_free = self
            .occupied_scratch
            .iter()
            .rev()
            .take_while(|occupied| !**occupied)
            .count();
        let offset = self.sector_count - trailing_free;
        let end = offset
            .checked_add(needed)
            .ok_or_else(|| invalid_region(path, "region allocation overflow"))?;
        let length = (end as u64)
            .checked_mul(SECTOR_BYTES as u64)
            .ok_or_else(|| invalid_region(path, "region length overflow"))?;
        if length > limits.max_region_bytes {
            return Err(SaveError::BudgetExceeded {
                resource: "region file",
                actual: length,
                limit: limits.max_region_bytes,
            });
        }
        if offset > 0x00ff_ffff {
            return Err(invalid_region(
                path,
                "region sector offset exceeds Anvil location encoding",
            ));
        }
        self.sector_count = end;
        Ok(Location {
            offset: offset as u32,
            count: needed as u8,
        })
    }

    fn write_entry(&self, file: &mut File, path: &Path, index: usize) -> SaveResult<()> {
        let location = self.locations[index]
            .map(|location| (location.offset << 8) | u32::from(location.count))
            .unwrap_or(0);
        file.seek(SeekFrom::Start((index * 4) as u64))
            .map_err(|source| io_error("seeking region location", path, source))?;
        file.write_all(&location.to_be_bytes())
            .map_err(|source| io_error("writing region location", path, source))?;
        file.seek(SeekFrom::Start((SECTOR_BYTES + index * 4) as u64))
            .map_err(|source| io_error("seeking region timestamp", path, source))?;
        file.write_all(&self.timestamps[index].to_be_bytes())
            .map_err(|source| io_error("writing region timestamp", path, source))
    }
}

struct Slot {
    compression: u8,
    payload_len: usize,
    buf: Vec<u8>,
}

fn read_slot(file: &mut File, path: &Path, location: Location) -> SaveResult<Slot> {
    let capacity = location.count as usize * SECTOR_BYTES;
    let mut slot = vec![0_u8; capacity];
    file.seek(SeekFrom::Start(
        location.offset as u64 * SECTOR_BYTES as u64,
    ))
    .map_err(|source| io_error("seeking region chunk", path, source))?;
    let mut read = 0;
    while read < capacity {
        match file.read(&mut slot[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(source) if source.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(source) => return Err(io_error("reading region chunk", path, source)),
        }
    }
    if read < 5 {
        return Err(invalid_region(path, "truncated chunk slot header"));
    }
    let length = read_be_u32(&slot, 0) as usize;
    if length == 0 {
        return Err(invalid_region(path, "chunk payload has zero length"));
    }
    if length > read - 4 {
        return Err(invalid_region(
            path,
            "chunk payload length exceeds allocated sectors",
        ));
    }
    Ok(Slot {
        compression: slot[4],
        payload_len: length,
        buf: slot,
    })
}

fn sectors_for(bytes: usize) -> usize {
    bytes.div_ceil(SECTOR_BYTES)
}

fn read_be_u32(bytes: &[u8], offset: usize) -> u32 {
    // All callers use statically bounded header/sector offsets after their
    // enclosing buffer has been fully read, so this cannot be influenced into
    // an out-of-bounds conversion by on-disk contents.
    u32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn find_run(occupied: &[bool], needed: usize) -> Option<usize> {
    let mut run = 0;
    for (index, used) in occupied.iter().enumerate().skip(2) {
        if *used {
            run = 0;
        } else {
            run += 1;
            if run == needed {
                return Some(index + 1 - needed);
            }
        }
    }
    None
}

fn chunk_index(global_x: i32, global_z: i32) -> usize {
    ((global_x & 31) as usize) + (((global_z & 31) as usize) * 32)
}

fn compress(path: &Path, compression: RegionCompression, bytes: &[u8]) -> SaveResult<Vec<u8>> {
    match compression {
        RegionCompression::None => Ok(bytes.to_vec()),
        RegionCompression::Gzip => finish_encoder(
            GzEncoder::new(Vec::new(), Compression::default()),
            path,
            bytes,
        ),
        RegionCompression::Zlib => finish_encoder(
            ZlibEncoder::new(Vec::new(), Compression::default()),
            path,
            bytes,
        ),
        RegionCompression::Lz4 => {
            compress_java_block(bytes).map_err(|detail| SaveError::InvalidRegion {
                path: path.to_path_buf(),
                detail,
            })
        }
    }
}

fn finish_encoder<W: FinishEncoder>(
    mut encoder: W,
    path: &Path,
    bytes: &[u8],
) -> SaveResult<Vec<u8>> {
    encoder
        .write_all(bytes)
        .map_err(|source| io_error("compressing region chunk", path, source))?;
    encoder
        .finish_encoder()
        .map_err(|source| io_error("finishing region compression", path, source))
}

trait FinishEncoder: Write {
    fn finish_encoder(self) -> std::io::Result<Vec<u8>>;
}

impl FinishEncoder for GzEncoder<Vec<u8>> {
    fn finish_encoder(self) -> std::io::Result<Vec<u8>> {
        self.finish()
    }
}

impl FinishEncoder for ZlibEncoder<Vec<u8>> {
    fn finish_encoder(self) -> std::io::Result<Vec<u8>> {
        self.finish()
    }
}

/// The operation a decompression failure reports, which is a malformed record
/// rather than a failed read.
pub(crate) const DECOMPRESSING_CHUNK: &str = "decompressing region chunk";

fn decompress(
    path: &Path,
    compression: RegionCompression,
    compressed: &[u8],
    limit: usize,
) -> SaveResult<Vec<u8>> {
    let mut bytes = Vec::new();
    match compression {
        RegionCompression::None => {
            check_budget("decompressed region chunk", compressed.len(), limit)?;
            bytes.extend_from_slice(compressed);
        }
        RegionCompression::Gzip => {
            GzDecoder::new(compressed)
                .take((limit as u64).saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(|source| io_error(DECOMPRESSING_CHUNK, path, source))?;
        }
        RegionCompression::Zlib => {
            ZlibDecoder::new(compressed)
                .take((limit as u64).saturating_add(1))
                .read_to_end(&mut bytes)
                .map_err(|source| io_error(DECOMPRESSING_CHUNK, path, source))?;
        }
        RegionCompression::Lz4 => {
            bytes = decompress_java_block(compressed, limit).map_err(|detail| {
                SaveError::InvalidRegion {
                    path: path.to_path_buf(),
                    detail,
                }
            })?;
        }
    };
    check_budget("decompressed region chunk", bytes.len(), limit)?;
    Ok(bytes)
}

fn decode_document(path: &Path, bytes: &[u8]) -> SaveResult<NamedCompound> {
    let root = decode_file_saved_nbt_summary(bytes)
        .map_err(|error| SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: error.to_string(),
        })?
        .ok_or_else(|| SaveError::InvalidNbt {
            path: Some(path.to_path_buf()),
            detail: "region chunk root must not be TAG_End".to_string(),
        })?;
    NamedCompound::try_from_summary("", root).map_err(|error| SaveError::InvalidNbt {
        path: Some(path.to_path_buf()),
        detail: error.to_string(),
    })
}

fn check_budget(resource: &'static str, actual: usize, limit: usize) -> SaveResult<()> {
    if actual > limit {
        return Err(SaveError::BudgetExceeded {
            resource,
            actual: actual as u64,
            limit: limit as u64,
        });
    }
    Ok(())
}

fn current_timestamp() -> u32 {
    let seconds = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    seconds.min(u64::from(u32::MAX)) as u32
}

fn invalid_region(path: &Path, detail: impl Into<String>) -> SaveError {
    SaveError::InvalidRegion {
        path: path.to_path_buf(),
        detail: detail.into(),
    }
}

fn io_error(operation: &'static str, path: &Path, source: std::io::Error) -> SaveError {
    SaveError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use super::*;

    static NEXT_DIR: AtomicU64 = AtomicU64::new(1);

    fn paths(name: &str) -> (PathBuf, PathBuf, PathBuf) {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "bbb-region-file-{name}-{}-{id}",
            std::process::id()
        ));
        (root.clone(), root.join("r.0.0.mca"), root.join("external"))
    }

    fn store(name: &str) -> (PathBuf, RegionFileStore) {
        let (root, region, external) = paths(name);
        let store = RegionFileStore::new(region, external, WorldSaveLimits::default()).unwrap();
        (root, store)
    }

    fn document(value: i32) -> NamedCompound {
        NamedCompound::new(
            "",
            vec![NbtSummaryEntry {
                name: "value".into(),
                value: NbtSummaryValue::Int(value),
            }],
        )
    }

    fn large_document(bytes: usize) -> NamedCompound {
        NamedCompound::new(
            "",
            vec![NbtSummaryEntry {
                name: "payload".into(),
                value: NbtSummaryValue::ByteArray(vec![7; bytes]),
            }],
        )
    }

    fn external_document() -> NamedCompound {
        NamedCompound::new(
            "",
            vec![NbtSummaryEntry {
                name: "payload".into(),
                value: NbtSummaryValue::List(vec![NbtSummaryValue::string("x".repeat(256)); 4096]),
            }],
        )
    }

    fn read_header(path: &Path) -> Vec<u8> {
        fs::read(path).unwrap()[..HEADER_BYTES].to_vec()
    }

    #[test]
    fn missing_region_reads_empty_and_new_region_has_big_endian_header_entries() {
        let (root, store) = store("new");
        assert_eq!(store.read_chunk(-1, -1).unwrap(), None);
        store
            .write_chunk(-1, -1, &document(7), RegionCompression::Zlib, 0x0102_0304)
            .unwrap();
        assert_eq!(store.read_chunk(-1, -1).unwrap(), Some(document(7)));

        let header = read_header(&store.path);
        let index = chunk_index(-1, -1);
        assert_eq!(&header[index * 4..index * 4 + 4], &[0, 0, 2, 1]);
        let timestamp = SECTOR_BYTES + index * 4;
        assert_eq!(&header[timestamp..timestamp + 4], &[1, 2, 3, 4]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn chunk_reader_skips_malformed_root_name_bytes() {
        let (root, store) = store("raw-root");
        let document = document(7);
        store
            .write_chunk(0, 0, &document, RegionCompression::None, 1)
            .unwrap();
        let mut raw = encode_named_nbt_summary("abc", &document.to_summary(), 4096).unwrap();
        raw[3..6].copy_from_slice(&[0xff, 0xff, 0xff]);
        let mut file = OpenOptions::new().write(true).open(&store.path).unwrap();
        file.seek(SeekFrom::Start(HEADER_BYTES as u64)).unwrap();
        file.write_all(&((raw.len() + 1) as u32).to_be_bytes())
            .unwrap();
        file.write_all(&[RegionCompression::None.id()]).unwrap();
        file.write_all(&raw).unwrap();
        drop(file);
        assert_eq!(store.read_chunk(0, 0).unwrap(), Some(document));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn existing_empty_region_is_empty_but_remains_writable() {
        let (root, region, external) = paths("existing-empty");
        fs::create_dir_all(&root).unwrap();
        fs::write(&region, []).unwrap();
        let store = RegionFileStore::new(&region, &external, WorldSaveLimits::default()).unwrap();

        assert_eq!(store.read_chunk(0, 0).unwrap(), None);
        store.clear_chunk(0, 0).unwrap();
        assert_eq!(fs::metadata(&region).unwrap().len(), 0);

        store
            .write_chunk(0, 0, &document(11), RegionCompression::Zlib, 5)
            .unwrap();
        assert_eq!(store.read_chunk(0, 0).unwrap(), Some(document(11)));
        assert!(fs::metadata(&region).unwrap().len() >= HEADER_BYTES as u64);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn occupied_slot_discovery_treats_missing_and_empty_regions_as_empty_without_creating() {
        let (root, region, external) = paths("discover-empty");
        let store = RegionFileStore::new(&region, &external, WorldSaveLimits::default()).unwrap();

        assert_eq!(store.occupied_local_slots().unwrap(), Vec::new());
        assert!(!region.exists());

        fs::create_dir_all(&root).unwrap();
        fs::write(&region, []).unwrap();
        assert_eq!(store.occupied_local_slots().unwrap(), Vec::new());
        assert_eq!(fs::metadata(&region).unwrap().len(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn occupied_slot_discovery_is_row_major_and_preserves_raw_timestamps() {
        let (root, store) = store("discover-order");
        for (x, z, timestamp) in [
            (31, 31, 0xfeed_beef),
            (4, 2, 0),
            (1, 0, 17),
            (0, 0, 0x0102_0304),
        ] {
            store
                .write_chunk(x, z, &document(x + z), RegionCompression::None, timestamp)
                .unwrap();
        }

        assert_eq!(
            store.occupied_local_slots().unwrap(),
            vec![
                RegionOccupiedLocalSlot {
                    local_x: 0,
                    local_z: 0,
                    timestamp: 0x0102_0304,
                },
                RegionOccupiedLocalSlot {
                    local_x: 1,
                    local_z: 0,
                    timestamp: 17,
                },
                RegionOccupiedLocalSlot {
                    local_x: 4,
                    local_z: 2,
                    timestamp: 0,
                },
                RegionOccupiedLocalSlot {
                    local_x: 31,
                    local_z: 31,
                    timestamp: 0xfeed_beef,
                },
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn occupied_slot_discovery_does_not_validate_the_chunk_payload() {
        let (root, region, external) = paths("discover-zero-payload");
        fs::create_dir_all(&root).unwrap();
        let mut bytes = vec![0; HEADER_BYTES + SECTOR_BYTES];
        let index = chunk_index(3, 5);
        bytes[index * 4..index * 4 + 4].copy_from_slice(&(2_u32 << 8 | 1).to_be_bytes());
        let timestamp_offset = SECTOR_BYTES + index * 4;
        bytes[timestamp_offset..timestamp_offset + 4].copy_from_slice(&29_u32.to_be_bytes());
        fs::write(&region, bytes).unwrap();
        let store = RegionFileStore::new(&region, &external, WorldSaveLimits::default()).unwrap();

        assert_eq!(
            store.occupied_local_slots().unwrap(),
            vec![RegionOccupiedLocalSlot {
                local_x: 3,
                local_z: 5,
                timestamp: 29,
            }]
        );
        assert!(matches!(
            store.read_chunk(3, 5),
            Err(SaveError::InvalidRegion { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn all_vanilla_region_compressions_round_trip() {
        for compression in [
            RegionCompression::Gzip,
            RegionCompression::Zlib,
            RegionCompression::None,
            RegionCompression::Lz4,
        ] {
            let (root, store) = store("compression");
            store
                .write_chunk(4, -3, &document(42), compression, 1)
                .unwrap();
            assert_eq!(store.read_chunk(4, -3).unwrap(), Some(document(42)));
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn allocation_relocates_and_reuses_cleared_sectors() {
        let (root, store) = store("allocation");
        store
            .write_chunk(0, 0, &large_document(3000), RegionCompression::None, 1)
            .unwrap();
        store
            .write_chunk(1, 0, &large_document(5000), RegionCompression::None, 1)
            .unwrap();
        store
            .write_chunk(0, 0, &large_document(5000), RegionCompression::None, 2)
            .unwrap();
        let mut file = File::open(&store.path).unwrap();
        let header = Header::read(&mut file, &store.path, store.limits).unwrap();
        assert_eq!(header.locations[chunk_index(0, 0)].unwrap().offset, 5);
        store.clear_chunk(1, 0).unwrap();
        store
            .write_chunk(2, 0, &document(3), RegionCompression::None, 3)
            .unwrap();
        let mut file = File::open(&store.path).unwrap();
        let header = Header::read(&mut file, &store.path, store.limits).unwrap();
        assert_eq!(header.locations[chunk_index(2, 0)].unwrap().offset, 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn external_stub_uses_global_coordinates_and_mcc_bytes() {
        let (root, store) = store("external");
        let document = external_document();
        store
            .write_chunk(-33, 64, &document, RegionCompression::None, 9)
            .unwrap();
        let external = store.external_path(-33, 64);
        assert!(external.ends_with("c.-33.64.mcc"));
        assert!(external.exists());
        assert_eq!(store.read_chunk(-33, 64).unwrap(), Some(document));
        let header = read_header(&store.path);
        let offset = HEADER_BYTES + 4;
        assert_eq!(header[chunk_index(-33, 64) * 4 + 3], 1);
        assert_eq!(
            fs::read(&store.path).unwrap()[offset],
            RegionCompression::None.id() | EXTERNAL_STREAM_FLAG
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cloned_store_serializes_concurrent_writes_to_the_same_region() {
        let (root, store) = store("concurrent");
        let first = store.clone();
        let second = store.clone();
        let first_thread = std::thread::spawn(move || {
            for value in 0..16 {
                first
                    .write_chunk(
                        0,
                        0,
                        &document(value),
                        RegionCompression::None,
                        value as u32,
                    )
                    .unwrap();
            }
        });
        let second_thread = std::thread::spawn(move || {
            for value in 0..16 {
                second
                    .write_chunk(
                        1,
                        0,
                        &document(value),
                        RegionCompression::None,
                        value as u32,
                    )
                    .unwrap();
            }
        });
        first_thread.join().unwrap();
        second_thread.join().unwrap();

        assert_eq!(store.read_chunk(0, 0).unwrap(), Some(document(15)));
        assert_eq!(store.read_chunk(1, 0).unwrap(), Some(document(15)));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn clear_removes_header_entry_and_external_file() {
        let (root, store) = store("clear");
        let document = external_document();
        store
            .write_chunk(0, 0, &document, RegionCompression::None, 8)
            .unwrap();
        assert!(store.external_path(0, 0).exists());
        store.clear_chunk(0, 0).unwrap();
        assert_eq!(store.read_chunk(0, 0).unwrap(), None);
        assert!(!store.external_path(0, 0).exists());
        let header = read_header(&store.path);
        assert_eq!(&header[..4], &[0; 4]);
        let timestamp = read_be_u32(&header, SECTOR_BYTES);
        assert!(timestamp > 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bounded_lru_cache_reuses_open_handles_and_evicts_cleanly_before_rename_or_delete() {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "bbb-region-file-lru-cache-{}-{id}",
            std::process::id()
        ));
        let renamed_root = std::env::temp_dir().join(format!(
            "bbb-region-file-lru-cache-renamed-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&renamed_root);

        let cache = RegionFileCache::new(2);
        let r0 = RegionFileStore::with_cache(
            root.join("r.0.0.mca"),
            root.join("region"),
            WorldSaveLimits::default(),
            cache.clone(),
        )
        .unwrap();
        r0.write_chunk(0, 0, &document(10), RegionCompression::Zlib, 111)
            .unwrap();
        for x in 1..8 {
            r0.write_chunk(
                x,
                0,
                &document(10 + x),
                RegionCompression::Zlib,
                111 + x as u32,
            )
            .unwrap();
        }
        for x in 0..8 {
            assert_eq!(
                r0.read_chunk_with_timestamp(x, 0).unwrap(),
                Some((document(10 + x), 111 + x as u32))
            );
        }
        r0.flush().unwrap();
        let (len_single, hits_single, misses_single) = cache.stats();
        assert_eq!(len_single, 1);
        assert_eq!(misses_single, 1);
        assert_eq!(hits_single, 16);

        // Touch two more regions so the capacity-2 LRU evicts r.0.0.mca cleanly.
        for rx in [1, 2] {
            let store = RegionFileStore::with_cache(
                root.join(format!("r.{rx}.0.mca")),
                root.join("region"),
                WorldSaveLimits::default(),
                cache.clone(),
            )
            .unwrap();
            store
                .write_chunk(rx * 32, 0, &document(rx), RegionCompression::None, 1)
                .unwrap();
        }
        assert_eq!(cache.stats().0, 2);

        // Flush and close cached handles under `root` before atomic directory rename/delete.
        cache.flush_and_evict_under(&root).unwrap();
        assert_eq!(cache.stats().0, 0);
        fs::rename(&root, &renamed_root).unwrap();
        let reopened = RegionFileStore::new(
            renamed_root.join("r.0.0.mca"),
            renamed_root.join("region"),
            WorldSaveLimits::default(),
        )
        .unwrap();
        assert_eq!(
            reopened.read_chunk_with_timestamp(7, 0).unwrap(),
            Some((document(17), 118))
        );
        drop(reopened);
        fs::remove_dir_all(&renamed_root).unwrap();
    }

    #[test]
    #[ignore = "requires BBB_TEST_JAVA_26_1_WORLD from an authorized official-server run"]
    fn verified_official_java_26_1_region_decodes_chunk_nbt() {
        let root = std::env::var_os("BBB_TEST_JAVA_26_1_WORLD")
            .expect("BBB_TEST_JAVA_26_1_WORLD must name an official 26.1 world");
        let paths = bbb_server_types::save::WorldSavePaths::new(root).unwrap();
        let region_dir = paths.region_dir();
        let store = RegionFileStore::new(
            region_dir.join("r.-1.0.mca"),
            &region_dir,
            WorldSaveLimits::default(),
        )
        .unwrap();

        let chunk = store.read_chunk(-1, 0).unwrap().unwrap();

        assert_eq!(chunk.root_name(), "");
        assert!(chunk
            .entries()
            .iter()
            .any(|entry| entry.name == "DataVersion"));
    }

    #[test]
    fn write_chunk_supports_saved_file_wide_compounds_and_deep_nesting() {
        let (root, store) = store("saved-file-limits");
        let mut entries: Vec<bbb_protocol::NbtSummaryEntry> = (0..5000)
            .map(|i| bbb_protocol::NbtSummaryEntry {
                name: format!("k{i}").into(),
                value: bbb_protocol::NbtSummaryValue::Int(i),
            })
            .collect();
        let mut nested = bbb_protocol::NbtSummaryValue::Int(9);
        for _ in 0..78 {
            nested = bbb_protocol::NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                name: "next".into(),
                value: nested,
            }]);
        }
        entries.push(bbb_protocol::NbtSummaryEntry {
            name: "deep".into(),
            value: nested,
        });
        let doc = NamedCompound::new("", entries);
        store
            .write_chunk(0, 0, &doc, RegionCompression::Zlib, 12)
            .unwrap();
        let loaded = store.read_chunk(0, 0).unwrap().unwrap();
        assert_eq!(loaded.entries().len(), 5001);
        drop(store);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn overwrite_and_clear_truncate_trailing_free_sectors() {
        let (root, store) = store("truncate-trailing");
        // Write a chunk, overwrite it twice so the second overwrite reuses sector 2 and frees tail sectors.
        let large = NamedCompound::new(
            "",
            vec![bbb_protocol::NbtSummaryEntry {
                name: "payload".into(),
                value: bbb_protocol::NbtSummaryValue::ByteArray(vec![7_i8; SECTOR_BYTES * 3]),
            }],
        );
        store
            .write_chunk(0, 0, &large, RegionCompression::None, 1)
            .unwrap();
        let initial_len = fs::metadata(&store.path).unwrap().len();
        assert_eq!(initial_len, (2 + 4) as u64 * SECTOR_BYTES as u64);

        // First overwrite with a tiny chunk allocates after sector 6 (since old sectors stay live until header commit).
        store
            .write_chunk(0, 0, &document(1), RegionCompression::None, 2)
            .unwrap();
        let expanded_len = fs::metadata(&store.path).unwrap().len();
        assert_eq!(expanded_len, (2 + 4 + 1) as u64 * SECTOR_BYTES as u64);

        // Second overwrite reuses sector 2 and truncates the freed tail back down to 3 sectors.
        store
            .write_chunk(0, 0, &document(2), RegionCompression::None, 3)
            .unwrap();
        let compacted_len = fs::metadata(&store.path).unwrap().len();
        assert_eq!(compacted_len, 3 * SECTOR_BYTES as u64);
        assert_eq!(store.read_chunk(0, 0).unwrap(), Some(document(2)));

        // Clearing the final chunk truncates the file back down to the 2-sector header.
        store.clear_chunk(0, 0).unwrap();
        assert_eq!(
            fs::metadata(&store.path).unwrap().len(),
            HEADER_BYTES as u64
        );
        assert_eq!(store.read_chunk(0, 0).unwrap(), None);

        drop(store);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lru_cache_uses_o1_eviction_and_preserves_order() {
        let cache = RegionFileCache::new(3);
        let p1 = PathBuf::from("r.0.0.mca");
        let p2 = PathBuf::from("r.1.0.mca");
        let p3 = PathBuf::from("r.2.0.mca");
        let p4 = PathBuf::from("r.3.0.mca");

        let s1 = cache.slot_for(&p1);
        let s2_ptr = Arc::as_ptr(&cache.slot_for(&p2));
        let s3 = cache.slot_for(&p3);
        // Touch p1 so idle p2 becomes the least recently used slot.
        assert!(Arc::ptr_eq(&cache.slot_for(&p1), &s1));

        let s4 = cache.slot_for(&p4);
        assert_eq!(cache.stats().0, 3);
        assert!(Arc::ptr_eq(&cache.slot_for(&p1), &s1));
        assert!(Arc::ptr_eq(&cache.slot_for(&p3), &s3));
        assert!(Arc::ptr_eq(&cache.slot_for(&p4), &s4));
        // Drop s1 so p1 is idle and can be evicted when re-inserting p2.
        let s1_ptr = Arc::as_ptr(&s1);
        drop(s1);
        let s2_new = cache.slot_for(&p2);
        assert_ne!(Arc::as_ptr(&s2_new), s2_ptr);
        assert_eq!(cache.stats().0, 3);

        // Even if p2 is explicitly evicted while a caller still holds its handle,
        // a concurrent lookup for p2 must reuse the same live Arc mutex.
        cache.evict_path(&p2);
        let s2_concurrent = cache.slot_for(&p2);
        assert!(Arc::ptr_eq(&s2_concurrent, &s2_new));
        // And s1 (evicted while idle) was not kept alive.
        let s1_new = cache.slot_for(&p1);
        assert_ne!(Arc::as_ptr(&s1_new), s1_ptr);
    }

    #[test]
    fn external_mcc_removal_checks_previous_external_bit() {
        let (root, store) = store("external-bit-gate");
        let inline_doc = document(1);
        store
            .write_chunk(0, 0, &inline_doc, RegionCompression::None, 1)
            .unwrap();
        let mcc = store.external_path(0, 0);
        fs::create_dir_all(mcc.parent().unwrap()).unwrap();
        fs::write(&mcc, b"sentinel").unwrap();

        // Overwriting or clearing an inline chunk must not touch an unrelated `.mcc`.
        store
            .write_chunk(0, 0, &document(2), RegionCompression::None, 2)
            .unwrap();
        assert!(mcc.exists());
        store.clear_chunk(0, 0).unwrap();
        assert!(mcc.exists());
        fs::remove_file(&mcc).unwrap();

        // When the previous sector actually carried the external-file bit, overwriting
        // inline after reopening must remove the stale `.mcc`.
        store
            .write_chunk(0, 0, &external_document(), RegionCompression::None, 3)
            .unwrap();
        assert!(mcc.exists());
        drop(store);

        let reopened = RegionFileStore::new(
            root.join("r.0.0.mca"),
            root.join("external"),
            WorldSaveLimits::default(),
        )
        .unwrap();
        reopened
            .write_chunk(0, 0, &document(4), RegionCompression::None, 4)
            .unwrap();
        assert!(!mcc.exists());
        assert_eq!(reopened.read_chunk(0, 0).unwrap(), Some(document(4)));
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parallel_chunk_preparation_round_trips_all_compressions() {
        let (root, path, external) = paths("parallel-compression");
        let docs: Vec<_> = (0..12).map(document).collect();

        for compression in [
            RegionCompression::None,
            RegionCompression::Zlib,
            RegionCompression::Gzip,
            RegionCompression::Lz4,
        ] {
            let items: Vec<_> = docs
                .iter()
                .map(|doc| (path.as_path(), WorldSaveLimits::default(), doc, compression))
                .collect();
            let prepared = prepare_chunks_parallel(&items).unwrap();
            assert_eq!(prepared.len(), docs.len());

            let store = RegionFileStore::new(&path, &external, WorldSaveLimits::default()).unwrap();
            for (idx, payload) in prepared.into_iter().enumerate() {
                store
                    .write_prepared_chunk(idx as i32, 0, payload, compression, 42)
                    .unwrap();
            }
            for (idx, expected) in docs.iter().enumerate() {
                assert_eq!(
                    store.read_chunk(idx as i32, 0).unwrap(),
                    Some(expected.clone())
                );
            }
            drop(store);
            let _ = fs::remove_file(&path);
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn opens_and_repairs_region_with_unaligned_trailing_partial_sector() {
        let (root, path, external) = paths("unaligned-tail");
        let store = RegionFileStore::new(&path, &external, WorldSaveLimits::default()).unwrap();
        store
            .write_chunk(0, 0, &document(7), RegionCompression::Zlib, 10)
            .unwrap();
        drop(store);

        // Simulate power loss mid-append leaving a non-4KiB-aligned trailing sector.
        let mut bytes = fs::read(&path).unwrap();
        bytes.extend_from_slice(&[0x55; 123]);
        fs::write(&path, &bytes).unwrap();

        let reopened = RegionFileStore::new(&path, &external, WorldSaveLimits::default()).unwrap();
        assert_eq!(reopened.read_chunk(0, 0).unwrap(), Some(document(7)));
        reopened
            .write_chunk(1, 0, &document(8), RegionCompression::Zlib, 11)
            .unwrap();
        assert_eq!(reopened.read_chunk(1, 0).unwrap(), Some(document(8)));
        assert_eq!(fs::metadata(&path).unwrap().len() % SECTOR_BYTES as u64, 0);
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn dirty_flag_syncs_only_modified_handles_on_flush_evict_and_drop() {
        let (root, path, external) = paths("dirty-sync");
        let cache = RegionFileCache::new(2);
        let syncs = cache.sync_counter();
        let store = RegionFileStore::with_cache(
            &path,
            &external,
            WorldSaveLimits::default(),
            cache.clone(),
        )
        .unwrap();

        // Missing/clean operations must never trigger fsync.
        assert_eq!(store.read_chunk(0, 0).unwrap(), None);
        store.clear_chunk(0, 0).unwrap();
        cache.evict_all();
        assert_eq!(syncs.load(Ordering::Relaxed), 0);

        // Write -> flush syncs once and clears dirty so repeated flush/evict is free.
        store
            .write_chunk(0, 0, &document(1), RegionCompression::None, 1)
            .unwrap();
        assert_eq!(syncs.load(Ordering::Relaxed), 0);
        store.flush().unwrap();
        assert_eq!(syncs.load(Ordering::Relaxed), 1);
        store.flush().unwrap();
        cache.evict_path(&path);
        assert_eq!(syncs.load(Ordering::Relaxed), 1);

        // Read-only and header-only writable opens remain clean on eviction.
        assert_eq!(store.read_chunk(0, 0).unwrap(), Some(document(1)));
        cache.evict_path(&path);
        store.validate_existing_header_for_write().unwrap();
        cache.evict_path(&path);
        assert_eq!(syncs.load(Ordering::Relaxed), 1);

        // Clearing an absent chunk stays clean; writing again syncs on eviction.
        store.clear_chunk(1, 0).unwrap();
        cache.evict_path(&path);
        assert_eq!(syncs.load(Ordering::Relaxed), 1);

        store
            .write_chunk(1, 0, &document(2), RegionCompression::None, 2)
            .unwrap();
        cache.evict_path(&path);
        assert_eq!(syncs.load(Ordering::Relaxed), 2);

        // Clearing an existing chunk marks dirty and syncs when the cache drops.
        store.clear_chunk(1, 0).unwrap();
        drop(store);
        drop(cache);
        assert_eq!(syncs.load(Ordering::Relaxed), 3);

        let reopened = RegionFileStore::new(&path, &external, WorldSaveLimits::default()).unwrap();
        assert_eq!(reopened.read_chunk(0, 0).unwrap(), Some(document(1)));
        assert_eq!(reopened.read_chunk(1, 0).unwrap(), None);
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }
}
