//! The scheduled-tick containers a loaded chunk holds once the level scheduler
//! registered it.

use bbb_protocol::packets::BlockPos;
use bbb_server_types::chunk_ticks::{ChunkTicks, Java26SavedTick, ScheduledTick};

use super::ChunkStore;
use crate::level_ticks::{
    chunk_of_block, ChunkKey, LevelTickType, ScheduleOutcome, TickContainers,
};

impl ChunkStore {
    /// Hands a column's parked containers to the level scheduler, loading the
    /// column first when nothing has yet.
    pub fn register_ticks(&mut self, chunk: ChunkKey, now: i64) {
        if !self.is_loaded(chunk.0, chunk.1) {
            self.load_column(chunk.0, chunk.1);
        }
        let loaded = self.loaded_mut(chunk);
        if loaded.ticks.is_none() {
            loaded.ticks = Some(TickContainers::register(&mut loaded.column, now));
        }
    }

    pub fn ticks_registered(&self, chunk: ChunkKey) -> bool {
        self.chunk(chunk)
            .is_some_and(|loaded| loaded.ticks.is_some())
    }

    pub fn tick_container<T: LevelTickType>(&self, chunk: ChunkKey) -> Option<&ChunkTicks<T>> {
        self.chunk(chunk)?.ticks.as_ref().map(T::registered)
    }

    pub(crate) fn tick_container_mut<T: LevelTickType>(
        &mut self,
        chunk: ChunkKey,
    ) -> Option<&mut ChunkTicks<T>> {
        self.chunk_mut(chunk)?.ticks.as_mut().map(T::registered_mut)
    }

    pub fn schedule_tick<T: LevelTickType>(&mut self, tick: ScheduledTick<T>) -> ScheduleOutcome {
        match self
            .tick_container_mut(chunk_of_block(tick.pos))
            .map(|container| container.schedule(tick))
        {
            None => ScheduleOutcome::ChunkNotLoaded,
            Some(true) => ScheduleOutcome::Scheduled,
            Some(false) => ScheduleOutcome::AlreadyScheduled,
        }
    }

    /// Whether a tick for this `(pos, type)` is still waiting in its
    /// container; a tick collected for this game tick no longer is.
    pub fn has_scheduled_tick<T: LevelTickType>(&self, pos: BlockPos, tick_type: T) -> bool {
        self.tick_container::<T>(chunk_of_block(pos))
            .is_some_and(|container| container.has_scheduled_tick(pos, tick_type))
    }

    pub fn scheduled_tick_keys<T: LevelTickType>(
        &self,
    ) -> impl Iterator<Item = (BlockPos, T)> + '_ {
        self.chunks
            .values()
            .filter_map(|loaded| loaded.ticks.as_ref())
            .flat_map(|ticks| T::registered(ticks).scheduled_tick_keys())
    }

    /// Registered chunks that hold a pending tick of either kind.
    pub fn chunks_with_scheduled_ticks(&self) -> impl Iterator<Item = ChunkKey> + '_ {
        self.chunks.iter().filter_map(|(&key, loaded)| {
            let ticks = loaded.ticks.as_ref()?;
            (!ticks.is_empty()).then_some(key)
        })
    }

    /// `LevelChunkTicks.pack(gameTime)` of a registered chunk, or the ticks
    /// the column itself still parks when nothing registered it.
    pub fn saved_ticks<T: LevelTickType>(&self, chunk: ChunkKey, now: i64) -> Vec<Java26SavedTick> {
        let Some(loaded) = self.chunk(chunk) else {
            return Vec::new();
        };
        match &loaded.ticks {
            Some(ticks) => T::registered(ticks).pack_saved(now),
            None => T::parked(&loaded.column).pack_saved(now),
        }
    }
}
