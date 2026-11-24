//! Block-entity storage: each object lives in its [`Chunk`] and loads, saves
//! and unloads with it; the level keeps only the ticker list, the game-event
//! listener sections and the objects lent out to their own bodies.

use rustc_hash::{FxHashMap, FxHashSet};

use bbb_protocol::packets::BlockPos;

use super::{Chunk, ChunkStore};
use crate::block_entity::game_event::{GameEventListenerEntry, LevelGameEventListeners};
use crate::block_entity::{has_ticker, BlockEntity, BlockEntityKey};

type ChunkKey = (i32, i32);
type Chunks = FxHashMap<ChunkKey, Chunk>;

fn chunk_of(pos: BlockPos) -> ChunkKey {
    (pos.x >> 4, pos.z >> 4)
}

fn key_of(pos: BlockPos) -> BlockEntityKey {
    (pos.x, pos.y, pos.z)
}

/// The level-wide half of block-entity state.
#[derive(Debug, Default)]
pub struct LevelBlockEntities {
    /// `Level.blockEntityTickers`, in registration order; an entry whose cell
    /// no longer holds a ticking object drops out at the next drain.
    tickers: Vec<BlockEntityKey>,
    /// Membership mirror of [`Self::tickers`] preventing duplicate tickers when
    /// a block entity is removed and re-added before the next prune (`LevelChunk.java:502-507`).
    tickers_set: FxHashSet<BlockEntityKey>,
    /// Reusable scratch buffer handed out to `tick_block_entities` across ticks
    /// to avoid reallocating the ticker walk snapshot on every tick.
    tickers_scratch: Vec<BlockEntityKey>,
    game_event_listeners: LevelGameEventListeners,
    /// Objects lent to a body that needs the whole level, with their type: a
    /// write the body makes to its own cell must still find the object there.
    on_loan: FxHashMap<BlockEntityKey, i32>,
}

pub(crate) struct ChunkBlockEntitySnapshot {
    listeners: FxHashMap<BlockEntityKey, GameEventListenerEntry>,
    on_loan: Vec<(BlockEntityKey, i32)>,
}

impl LevelBlockEntities {
    pub(super) fn unload_chunk(&mut self, chunk: ChunkKey) {
        let in_chunk = |key: &BlockEntityKey| (key.0 >> 4, key.2 >> 4) == chunk;
        self.tickers.retain(|key| !in_chunk(key));
        self.tickers_set.retain(|key| !in_chunk(key));
        self.on_loan.retain(|key, _| !in_chunk(key));
        self.game_event_listeners.remove_chunk(chunk);
    }

    pub(crate) fn snapshot_for_chunks(
        &self,
        chunks: &Chunks,
        keys: &[ChunkKey],
    ) -> ChunkBlockEntitySnapshot {
        let mut listeners = FxHashMap::default();
        for &chunk_key in keys {
            if let Some(chunk) = chunks.get(&chunk_key) {
                for (&pos_key, entity) in &chunk.block_entities {
                    if let Some(entry) = entity.game_event_listener_entry() {
                        listeners.insert(pos_key, entry);
                    }
                }
            }
        }
        let on_loan = self
            .on_loan
            .iter()
            .filter(|&(pos_key, _)| keys.contains(&(pos_key.0 >> 4, pos_key.2 >> 4)))
            .map(|(&pos_key, &type_id)| (pos_key, type_id))
            .collect();
        ChunkBlockEntitySnapshot { listeners, on_loan }
    }

    pub(crate) fn from_snapshot_on_loan(snapshot: &ChunkBlockEntitySnapshot) -> Self {
        let mut level = Self::default();
        for &(pos_key, type_id) in &snapshot.on_loan {
            level.on_loan.insert(pos_key, type_id);
        }
        level
    }

    pub(crate) fn reconcile_isolated(
        &mut self,
        chunks: &Chunks,
        keys: &[ChunkKey],
        snapshot: ChunkBlockEntitySnapshot,
        isolated: Self,
    ) {
        for (pos_key, old_entry) in &snapshot.listeners {
            let current_entry = chunks
                .get(&(pos_key.0 >> 4, pos_key.2 >> 4))
                .and_then(|chunk| chunk.block_entities.get(pos_key))
                .and_then(BlockEntity::game_event_listener_entry);
            if current_entry != Some(*old_entry) {
                self.game_event_listeners.unregister(*old_entry);
            }
        }
        for &chunk_key in keys {
            if let Some(chunk) = chunks.get(&chunk_key) {
                for (&pos_key, entity) in &chunk.block_entities {
                    if let Some(new_entry) = entity.game_event_listener_entry() {
                        if snapshot.listeners.get(&pos_key) != Some(&new_entry) {
                            self.game_event_listeners.register(new_entry);
                        }
                    }
                }
            }
        }
        for key in isolated.tickers {
            if self.tickers_set.insert(key) {
                self.tickers.push(key);
            }
        }
    }
}

/// Read access to one level's block entities.
#[derive(Clone, Copy)]
pub struct BlockEntities<'a> {
    chunks: &'a Chunks,
    level: &'a LevelBlockEntities,
}

/// Write access to one level's block entities.
pub struct BlockEntitiesMut<'a> {
    chunks: &'a mut Chunks,
    level: &'a mut LevelBlockEntities,
}

impl ChunkStore {
    pub fn block_entities(&self) -> BlockEntities<'_> {
        BlockEntities {
            chunks: &self.chunks,
            level: &self.block_entities,
        }
    }

    pub fn block_entities_mut(&mut self) -> BlockEntitiesMut<'_> {
        BlockEntitiesMut::new(&mut self.chunks, &mut self.block_entities)
    }

    /// The block entities beside the level's tags, which a block change reads
    /// while it edits them.
    pub(crate) fn block_entities_mut_and_tags(
        &mut self,
    ) -> (
        BlockEntitiesMut<'_>,
        &crate::registry_tags::RegistryTagSnapshot,
    ) {
        let tags = self
            .registry_tags
            .as_deref()
            .unwrap_or_else(|| crate::registry_tags::RegistryTagSnapshot::builtin());
        (
            BlockEntitiesMut::new(&mut self.chunks, &mut self.block_entities),
            tags,
        )
    }
}

impl<'a> BlockEntities<'a> {
    /// `LevelChunk.getBlockEntity(pos)` with `EntityCreationType.CHECK`.
    #[must_use]
    pub fn get(self, pos: BlockPos) -> Option<&'a BlockEntity> {
        self.chunks
            .get(&chunk_of(pos))?
            .block_entities
            .get(&key_of(pos))
    }

    /// Every block entity of one column, in position order.
    pub fn chunk(self, chunk: ChunkKey) -> impl Iterator<Item = &'a BlockEntity> {
        self.chunks
            .get(&chunk)
            .into_iter()
            .flat_map(|chunk| chunk.block_entities.values())
    }

    /// The type of the object lent out at `pos`.
    #[must_use]
    pub fn loaned_type(self, pos: BlockPos) -> Option<i32> {
        self.level.on_loan.get(&key_of(pos)).copied()
    }

    #[must_use]
    pub fn game_event_listeners(self) -> &'a LevelGameEventListeners {
        &self.level.game_event_listeners
    }
}

impl<'a> BlockEntitiesMut<'a> {
    pub(crate) fn new(chunks: &'a mut Chunks, level: &'a mut LevelBlockEntities) -> Self {
        Self { chunks, level }
    }

    pub fn reborrow(&mut self) -> BlockEntitiesMut<'_> {
        BlockEntitiesMut::new(self.chunks, self.level)
    }

    #[must_use]
    pub fn read(&self) -> BlockEntities<'_> {
        BlockEntities {
            chunks: self.chunks,
            level: self.level,
        }
    }

    #[must_use]
    pub fn get(&self, pos: BlockPos) -> Option<&BlockEntity> {
        self.read().get(pos)
    }

    pub fn get_mut(self, pos: BlockPos) -> Option<&'a mut BlockEntity> {
        let chunk = self.chunks.get_mut(&chunk_of(pos))?;
        chunk.block_entities.get_mut(&key_of(pos))
    }

    /// Lends the object at `pos` to a body that needs the whole level; its
    /// listener and ticker stay registered until [`Self::put_back`].
    pub fn take(&mut self, pos: BlockPos) -> Option<BlockEntity> {
        let chunk = self.chunks.get_mut(&chunk_of(pos))?;
        let taken = chunk.block_entities.remove(&key_of(pos))?;
        self.level.on_loan.insert(key_of(pos), taken.type_id());
        Some(taken)
    }

    /// Returns a lent object, its cached state following any write its body
    /// made to its own cell. A different type or invalid block the level placed
    /// in the cell meanwhile wins (`LevelChunk.java:307-316,330-347`).
    pub fn put_back(&mut self, mut block_entity: BlockEntity) {
        let pos = block_entity.block_pos();
        let key = key_of(pos);
        self.level.on_loan.remove(&key);
        let Some(chunk) = self.chunks.get_mut(&chunk_of(pos)) else {
            if let Some(listener) = block_entity.game_event_listener_entry() {
                self.level.game_event_listeners.unregister(listener);
            }
            return;
        };
        if chunk
            .block_entities
            .get(&key)
            .is_some_and(|existing| existing.type_id() != block_entity.type_id())
        {
            if let Some(listener) = block_entity.game_event_listener_entry() {
                let kept_same_listener = chunk
                    .block_entities
                    .get(&key)
                    .and_then(BlockEntity::game_event_listener_entry)
                    == Some(listener);
                if !kept_same_listener {
                    self.level.game_event_listeners.unregister(listener);
                }
            }
            return;
        }
        let Some(state) = chunk
            .column
            .get_block_state(pos.x & 15, pos.y, pos.z & 15)
            .filter(|&state| block_entity.is_valid_block_state(state))
        else {
            if let Some(listener) = block_entity.game_event_listener_entry() {
                self.level.game_event_listeners.unregister(listener);
            }
            return;
        };
        block_entity.set_block_state(state);
        chunk.block_entities.insert(key, block_entity);
    }

    /// `LevelChunk.addAndRegisterBlockEntity`: stores the object, moves the
    /// cell's game-event listener to it and files its ticker.
    pub fn add_and_register(&mut self, block_entity: BlockEntity) {
        let key = key_of(block_entity.block_pos());
        let Some(chunk) = self.chunks.get_mut(&chunk_of(block_entity.block_pos())) else {
            return;
        };
        chunk.cached_packet.take();
        chunk.packed_block_entities.remove(&key);
        let listener = block_entity.game_event_listener_entry();
        let ticks = has_ticker(&block_entity);
        let displaced = chunk.block_entities.insert(key, block_entity);
        let listeners = &mut self.level.game_event_listeners;
        if let Some(old) = displaced
            .as_ref()
            .and_then(BlockEntity::game_event_listener_entry)
        {
            listeners.unregister(old);
        }
        if let Some(listener) = listener {
            listeners.register(listener);
        }
        // A displaced or lent ticking object already holds the cell's entry
        // (every lent type ticks).
        let registered =
            displaced.as_ref().is_some_and(has_ticker) || self.level.on_loan.contains_key(&key);
        if ticks && !registered && self.level.tickers_set.insert(key) {
            self.level.tickers.push(key);
        }
    }

    /// `LevelChunk.setBlockState`'s keep arm (`LevelChunk.java:343-345`): the
    /// cached state follows a state this type still accepts.
    pub fn rebind_block_state(&mut self, pos: BlockPos, block_state_id: i32) {
        if let Some(block_entity) = self.reborrow().get_mut(pos) {
            if block_entity.is_valid_block_state(block_state_id) {
                block_entity.set_block_state(block_state_id);
            }
        }
    }

    /// `LevelChunk.removeBlockEntity(pos)`; answers whether an object left.
    pub fn remove(&mut self, pos: BlockPos) -> bool {
        let key = key_of(pos);
        let Some(removed) = self.chunks.get_mut(&chunk_of(pos)).and_then(|chunk| {
            chunk.packed_block_entities.remove(&key);
            let removed = chunk.block_entities.remove(&key)?;
            chunk.cached_packet.take();
            Some(removed)
        }) else {
            return false;
        };
        if let Some(listener) = removed.game_event_listener_entry() {
            self.level.game_event_listeners.unregister(listener);
        }
        true
    }

    /// Drops ticker entries whose cell no longer holds a ticking object.
    pub fn prune_tickers(&mut self) {
        let chunks = &*self.chunks;
        let tickers_set = &mut self.level.tickers_set;
        self.level.tickers.retain(|&key| {
            let live = chunks
                .get(&(key.0 >> 4, key.2 >> 4))
                .and_then(|chunk| chunk.block_entities.get(&key))
                .is_some_and(has_ticker);
            if !live {
                tickers_set.remove(&key);
            }
            live
        });
    }

    /// `Level.tickBlockEntities`'s walk list, with entries whose cell lost
    /// its ticking object dropped.
    pub fn drain_tickers(&mut self) -> Vec<BlockEntityKey> {
        self.prune_tickers();
        self.level.tickers.clone()
    }

    /// Prunes stale tickers and copies the live walk list into the level's
    /// reusable scratch buffer; return it via [`Self::return_tickers_scratch`].
    pub fn take_tickers(&mut self) -> Vec<BlockEntityKey> {
        self.prune_tickers();
        let mut scratch = std::mem::take(&mut self.level.tickers_scratch);
        scratch.clear();
        scratch.extend_from_slice(&self.level.tickers);
        scratch
    }

    /// Returns a ticker scratch buffer taken via [`Self::take_tickers`] so its
    /// allocation is reused on subsequent ticks.
    pub fn return_tickers_scratch(&mut self, mut scratch: Vec<BlockEntityKey>) {
        scratch.clear();
        if scratch.capacity() >= self.level.tickers_scratch.capacity() {
            self.level.tickers_scratch = scratch;
        }
    }

    /// `BlockEntity.setChanged` reaching `Level.blockEntityChanged`: marks the
    /// owning column dirty for the next whole-column save.
    pub fn set_changed(&mut self, pos: BlockPos) {
        if let Some(chunk) = self.chunks.get_mut(&chunk_of(pos)) {
            chunk.cached_packet.take();
            chunk.mark_save_dirty();
        }
    }

    pub fn game_event_listeners_mut(self) -> &'a mut LevelGameEventListeners {
        &mut self.level.game_event_listeners
    }
}
