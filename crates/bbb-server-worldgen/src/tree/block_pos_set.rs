//! Insertion-ordered set of block positions standing in for Vanilla's
//! `HashSet<BlockPos>`; Java's hash iteration order is not reproduced.

use std::collections::VecDeque;

use bbb_protocol::packets::BlockPos;

use crate::fast_hash::FastHashSet;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockPosSet {
    order: VecDeque<BlockPos>,
    members: FastHashSet<(i32, i32, i32)>,
}

fn key(pos: BlockPos) -> (i32, i32, i32) {
    (pos.x, pos.y, pos.z)
}

impl BlockPosSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether `pos` was newly added.
    pub fn add(&mut self, pos: BlockPos) -> bool {
        let added = self.members.insert(key(pos));
        if added {
            self.order.push_back(pos);
        }
        added
    }

    pub fn contains(&self, pos: BlockPos) -> bool {
        self.members.contains(&key(pos))
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn first(&self) -> Option<BlockPos> {
        self.order.front().copied()
    }

    pub fn remove_first(&mut self) -> Option<BlockPos> {
        let pos = self.order.pop_front()?;
        self.members.remove(&key(pos));
        Some(pos)
    }

    pub fn iter(&self) -> impl Iterator<Item = BlockPos> + '_ {
        self.order.iter().copied()
    }

    pub fn to_vec(&self) -> Vec<BlockPos> {
        self.order.iter().copied().collect()
    }
}
