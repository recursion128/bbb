use crate::chunk_sender::ChunkKey;
use bbb_server_persistence::Java26ChunkStatus;
use rustc_hash::FxHashMap;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

mod cold_columns;
pub(super) mod full_preparation;
mod generation_queue;
mod preparation;
#[cfg(test)]
mod saved_tick_test;
mod storage;
#[cfg(test)]
mod stored_tests;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum FullChunkStatus {
    #[default]
    Inaccessible,
    Full,
    BlockTicking,
    EntityTicking,
}

impl FullChunkStatus {
    pub(super) fn from_level(level: i32) -> Self {
        match level {
            ..=31 => Self::EntityTicking,
            32 => Self::BlockTicking,
            33 => Self::Full,
            _ => Self::Inaccessible,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EntityChunkLoadStatus {
    /// Saved and unloaded; the next access reads storage first.
    Stored,
    Pending,
    Loaded,
    /// The record did not read. Like vanilla's chunk left `PENDING` after a
    /// failed read (`PersistentEntitySectionManager.java:204-210`), it never
    /// loads and its record is never written over.
    Failed,
}

/// Equal-level ticket rectangle `(min, max, level)`; a point ticket is `(c, c, level)`.
pub(super) type LoadingSource = (ChunkKey, ChunkKey, i32);

#[derive(Default)]
pub(super) struct ChunkLifecycle {
    pub(super) loading_levels: Arc<FxHashMap<ChunkKey, i32>>,
    sources: Vec<LoadingSource>,
    sources_scratch: Vec<LoadingSource>,
    pub(super) confirmed: BTreeMap<ChunkKey, FullChunkStatus>,
    unknown: BTreeMap<ChunkKey, (i32, i64)>,
    pub(super) ender_pearl_tickets: BTreeMap<ChunkKey, i64>,
    pub(super) entity_loads: BTreeMap<ChunkKey, EntityChunkLoadStatus>,
    /// Entity records the `entities/` headers list that nothing asked for yet;
    /// each is `Stored` until its first request.
    pub(super) stored_entities: BTreeSet<ChunkKey>,
    pub(super) entity_unloads: BTreeSet<ChunkKey>,
    pub(super) column_unloads: BTreeSet<ChunkKey>,
    pub(super) evicted_columns: BTreeSet<ChunkKey>,
    /// The column the startup spawn search settled in. It and the columns its
    /// generation built stay in memory without a ticket until the first join
    /// reads them, instead of being saved and loaded back in between.
    pub(super) resident_spawn_column: Option<ChunkKey>,
    pub(super) storage_bound: bool,
    pub(super) generation: generation_queue::GenerationQueue,
    /// `(resident_spawn_column, loaded_chunk_count)` once every desired chunk
    /// has reached both its generation and confirmation targets and unloads are
    /// filed, allowing idle ticks with unchanged sources to skip re-scanning.
    pub(super) settled: Option<(Option<ChunkKey>, usize)>,
}

impl ChunkLifecycle {
    // ChunkLevel.java:10-32; ChunkPyramid.java:10-39; ChunkStep.java:108-129.
    pub(super) fn generation_status(level: i32) -> Option<Java26ChunkStatus> {
        match level - 33 {
            ..=0 => Some(Java26ChunkStatus::Full),
            1 => Some(Java26ChunkStatus::InitializeLight),
            2 => Some(Java26ChunkStatus::Carvers),
            3 => Some(Java26ChunkStatus::Biomes),
            4..=11 => Some(Java26ChunkStatus::StructureStarts),
            _ => None,
        }
    }

    pub(super) fn status(&self, chunk: ChunkKey) -> FullChunkStatus {
        self.confirmed.get(&chunk).copied().unwrap_or_default()
    }

    pub(super) fn entities_loaded(&self, chunk: ChunkKey) -> bool {
        self.entity_loads.get(&chunk) == Some(&EntityChunkLoadStatus::Loaded)
    }

    /// A chunk with no stored record has nothing to wait for.
    pub(super) fn request_entities(&mut self, chunk: ChunkKey) {
        let stored = &mut self.stored_entities;
        let status = self.entity_loads.entry(chunk).or_insert_with(|| {
            if stored.remove(&chunk) {
                EntityChunkLoadStatus::Stored
            } else {
                EntityChunkLoadStatus::Loaded
            }
        });
        if *status == EntityChunkLoadStatus::Stored {
            *status = EntityChunkLoadStatus::Pending;
        }
    }

    pub(super) fn entities_pending(&self, chunk: ChunkKey) -> bool {
        match self.entity_loads.get(&chunk) {
            Some(status) => *status != EntityChunkLoadStatus::Loaded,
            None => self.stored_entities.contains(&chunk),
        }
    }

    // ServerChunkCache.java:235-255; TicketType.java:25; Ticket.java:60-68.
    pub(super) fn add_unknown(&mut self, chunk: ChunkKey, level: i32) {
        let ticket = self.unknown.entry(chunk).or_insert((level, 1));
        ticket.0 = ticket.0.min(level);
        ticket.1 = 1;
    }

    pub(super) fn purge_unknown(&mut self) {
        self.unknown.retain(|_, (_, left)| {
            *left -= 1;
            *left >= 0
        });
    }

    pub(super) fn purge_transient_tickets(&mut self) {
        self.purge_unknown();
        self.ender_pearl_tickets.retain(|_, left| {
            *left -= 1;
            *left >= 0
        });
    }

    // ServerPlayer.java:2260-2263: radius two gives a level-31 source.
    pub(super) fn ender_pearl_sources(&self) -> impl Iterator<Item = (ChunkKey, i32)> + '_ {
        self.ender_pearl_tickets
            .keys()
            .map(|&chunk| (chunk, super::CHUNK_FULL_LEVEL - 2))
    }

    // ChunkTracker.java:17-32,64-67: min-plus propagation over king moves, which
    // for a rectangle of equal sources is its level plus the Chebyshev distance.
    pub(super) fn rebuild(&mut self, sources: impl IntoIterator<Item = LoadingSource>) -> bool {
        self.sources_scratch.clear();
        self.sources_scratch.extend(
            sources
                .into_iter()
                .chain(
                    self.unknown
                        .iter()
                        .map(|(&chunk, &(level, _))| (chunk, chunk, level)),
                )
                .filter(|&(_, _, level)| level <= 44),
        );
        self.sources_scratch.sort_unstable();
        self.sources_scratch.dedup();
        if self.sources_scratch == self.sources {
            return false;
        }
        self.settled = None;
        std::mem::swap(&mut self.sources, &mut self.sources_scratch);
        let levels = Arc::make_mut(&mut self.loading_levels);
        levels.clear();
        for &((min_x, min_z), (max_x, max_z), level) in &self.sources {
            let reach = 44 - level;
            let outside = |c: i32, min: i32, max: i32| {
                (i64::from(min) - i64::from(c))
                    .max(i64::from(c) - i64::from(max))
                    .max(0)
            };
            for cx in min_x.saturating_sub(reach)..=max_x.saturating_add(reach) {
                let dx = outside(cx, min_x, max_x);
                for cz in min_z.saturating_sub(reach)..=max_z.saturating_add(reach) {
                    let distance = dx.max(outside(cz, min_z, max_z)) as i32;
                    let entry = levels.entry((cx, cz)).or_insert(45);
                    *entry = (*entry).min(level + distance);
                }
            }
        }
        self.column_unloads
            .retain(|chunk| !self.loading_levels.contains_key(chunk));
        true
    }
}
