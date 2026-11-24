use rustc_hash::FxHashSet;
use std::cell::Cell;

use bbb_protocol::packets::BlockPos;

use crate::chunk::ChunkStore;

const CACHE_SIZE: usize = 4096;
const CACHE_MASK: usize = CACHE_SIZE - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CacheEntry {
    key: i64,
    path_type: i32,
}

#[derive(Debug)]
pub(crate) struct PathNavigationState {
    cache: Box<[Cell<Option<CacheEntry>>]>,
    pending_updates: FxHashSet<(i32, i32, i32)>,
}

impl Default for PathNavigationState {
    fn default() -> Self {
        Self {
            cache: vec![Cell::new(None); CACHE_SIZE].into_boxed_slice(),
            pending_updates: FxHashSet::default(),
        }
    }
}

impl PathNavigationState {
    pub(super) fn unload_chunk(&mut self, chunk: (i32, i32)) {
        for slot in &self.cache {
            if let Some(entry) = slot.get() {
                if block_pos_chunk(entry.key) == chunk {
                    slot.set(None);
                }
            }
        }
        self.pending_updates
            .retain(|&(x, _, z)| (x.div_euclid(16), z.div_euclid(16)) != chunk);
    }

    // PathTypeCache.java:8-42 uses a 4096-slot direct-mapped cache and accepts
    // a hit only when the packed position key matches exactly.
    fn cache_index(pos: BlockPos) -> (usize, i64) {
        let key = packed_block_pos(pos);
        let mut hash = (key as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        hash ^= hash >> 32;
        hash ^= hash >> 16;
        ((hash as usize) & CACHE_MASK, key)
    }

    fn cached_path_type(&self, pos: BlockPos) -> Option<i32> {
        let (index, key) = Self::cache_index(pos);
        self.cache[index]
            .get()
            .filter(|entry| entry.key == key)
            .map(|entry| entry.path_type)
    }

    fn cache_path_type(&self, pos: BlockPos, path_type: i32) {
        let (index, key) = Self::cache_index(pos);
        self.cache[index].set(Some(CacheEntry { key, path_type }));
    }

    fn invalidate_path_type(&self, pos: BlockPos) {
        let (index, key) = Self::cache_index(pos);
        if self.cache[index]
            .get()
            .is_some_and(|entry| entry.key == key)
        {
            self.cache[index].set(None);
        }
    }

    fn clear_path_type_cache(&self) {
        for slot in &self.cache {
            slot.set(None);
        }
    }

    fn queue_update(&mut self, pos: BlockPos) {
        self.pending_updates.insert((pos.x, pos.y, pos.z));
    }

    fn drain_updates(&mut self) -> Vec<BlockPos> {
        std::mem::take(&mut self.pending_updates)
            .into_iter()
            .map(|(x, y, z)| BlockPos { x, y, z })
            .collect()
    }
}

impl ChunkStore {
    pub fn cached_path_type(&self, pos: BlockPos) -> Option<i32> {
        self.path_navigation.cached_path_type(pos)
    }

    pub fn cache_path_type(&self, pos: BlockPos, path_type: i32) {
        self.path_navigation.cache_path_type(pos, path_type);
    }

    pub fn invalidate_path_type(&self, pos: BlockPos) {
        self.path_navigation.invalidate_path_type(pos);
    }

    pub fn clear_path_type_cache(&self) {
        self.path_navigation.clear_path_type_cache();
    }

    pub fn queue_navigation_update(&mut self, pos: BlockPos) {
        self.path_navigation.queue_update(pos);
    }

    pub fn drain_navigation_updates(&mut self) -> Vec<BlockPos> {
        self.path_navigation.drain_updates()
    }
}

// BlockPos.java:107-115 packs signed X/Z into 26 bits and Y into 12 bits.
fn packed_block_pos(pos: BlockPos) -> i64 {
    const XZ_MASK: i64 = (1_i64 << 26) - 1;
    const Y_MASK: i64 = (1_i64 << 12) - 1;
    ((i64::from(pos.x) & XZ_MASK) << 38)
        | (i64::from(pos.y) & Y_MASK)
        | ((i64::from(pos.z) & XZ_MASK) << 12)
}

fn block_pos_chunk(key: i64) -> (i32, i32) {
    let x = (key >> 38) as i32;
    let z = ((key << 26) >> 38) as i32;
    (x.div_euclid(16), z.div_euclid(16))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    #[test]
    fn path_navigation_cache_requires_the_exact_position_key() {
        let state = PathNavigationState::default();
        let first = pos(-23, 71, 49);
        let first_index = PathNavigationState::cache_index(first).0;
        let collision = (0..100_000)
            .map(|x| pos(x, -17, x / 3))
            .find(|candidate| {
                *candidate != first && PathNavigationState::cache_index(*candidate).0 == first_index
            })
            .expect("a 4096-slot cache must collide in this search range");

        state.cache_path_type(first, 7);
        assert_eq!(state.cached_path_type(first), Some(7));
        assert_eq!(state.cached_path_type(collision), None);

        state.cache_path_type(collision, 11);
        assert_eq!(state.cached_path_type(first), None);
        assert_eq!(state.cached_path_type(collision), Some(11));

        state.invalidate_path_type(first);
        assert_eq!(state.cached_path_type(collision), Some(11));
        state.invalidate_path_type(collision);
        assert_eq!(state.cached_path_type(collision), None);
    }

    #[test]
    fn path_navigation_updates_keep_pending_positions_between_drains() {
        let mut state = PathNavigationState::default();
        state.queue_update(pos(4, 70, -2));
        state.queue_update(pos(-1, 64, 3));
        state.queue_update(pos(4, 70, -2));

        let positions = state.drain_updates();
        assert_eq!(positions.len(), 2);
        assert!(positions.contains(&pos(-1, 64, 3)));
        assert!(positions.contains(&pos(4, 70, -2)));

        state.queue_update(pos(8, 65, 8));
        assert_eq!(state.drain_updates(), vec![pos(8, 65, 8)]);
        assert!(state.drain_updates().is_empty());
    }

    #[test]
    fn clearing_the_path_type_cache_preserves_pending_updates() {
        let mut state = PathNavigationState::default();
        let cached = pos(1, 2, 3);
        let pending = pos(7, 8, 9);
        state.cache_path_type(cached, 5);
        state.queue_update(pending);

        state.clear_path_type_cache();

        assert_eq!(state.cached_path_type(cached), None);
        assert_eq!(state.drain_updates(), vec![pending]);
    }

    #[test]
    fn unloading_a_chunk_invalidates_only_that_chunks_cached_positions() {
        let mut state = PathNavigationState::default();
        let target = pos(-17, 64, -1);
        let target_other_block = pos(-32, 65, -16);
        let neighbor = pos(0, 64, 0);
        let far_negative = pos(-16, 64, -17);
        state.cache_path_type(target, 3);
        state.cache_path_type(target_other_block, 4);
        state.cache_path_type(neighbor, 5);
        state.cache_path_type(far_negative, 6);

        state.unload_chunk((-2, -1));

        assert_eq!(state.cached_path_type(target), None);
        assert_eq!(state.cached_path_type(target_other_block), None);
        assert_eq!(state.cached_path_type(neighbor), Some(5));
        assert_eq!(state.cached_path_type(far_negative), Some(6));
    }
}
