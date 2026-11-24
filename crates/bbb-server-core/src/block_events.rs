//! Pending block events and their wire and recipient values.

use rustc_hash::FxHashSet;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};

use bbb_protocol::packets::{BlockEvent, BlockPos};

/// The `range` `ServerLevel` passes to `PlayerList.broadcast` for a block
/// event (`ServerLevel.java:1255`).
pub const BLOCK_EVENT_BROADCAST_RADIUS: f64 = 64.0;

/// A pending event and the block type it was queued against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockEventData {
    pub pos: BlockPos,
    /// Registry id of the block type the event was queued against.
    pub block_type_id: i32,
    /// `paramA`, the block-defined first argument.
    pub param_a: i32,
    /// `paramB`, the block-defined second argument.
    pub param_b: i32,
}

// BlockPos has no Hash implementation.
impl Hash for BlockEventData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pos.x.hash(state);
        self.pos.y.hash(state);
        self.pos.z.hash(state);
        self.block_type_id.hash(state);
        self.param_a.hash(state);
        self.param_b.hash(state);
    }
}

impl BlockEventData {
    #[must_use]
    pub const fn new(pos: BlockPos, block_type_id: i32, param_a: i32, param_b: i32) -> Self {
        Self {
            pos,
            block_type_id,
            param_a,
            param_b,
        }
    }

    /// ClientboundBlockEventPacket.java:30-38: parameters carry their low eight bits.
    #[must_use]
    pub fn packet(self) -> BlockEvent {
        BlockEvent {
            pos: self.pos,
            b0: self.param_a as u8,
            b1: self.param_b as u8,
            block_id: self.block_type_id,
        }
    }

    /// PlayerList.java:606-615: distance is measured from the raw block coordinates.
    #[must_use]
    pub fn reaches(self, same_dimension: bool, x: f64, y: f64, z: f64) -> bool {
        if !same_dimension {
            return false;
        }
        let dx = f64::from(self.pos.x) - x;
        let dy = f64::from(self.pos.y) - y;
        let dz = f64::from(self.pos.z) - z;
        dx * dx + dy * dy + dz * dz < BLOCK_EVENT_BROADCAST_RADIUS * BLOCK_EVENT_BROADCAST_RADIUS
    }
}

/// Pending events, deduplicated by position, block and parameters.
#[derive(Clone, Debug, Default)]
pub struct BlockEventQueue {
    pending: VecDeque<BlockEventData>,
    queued: FxHashSet<BlockEventData>,
    /// `ServerLevel.handlingTick` (`ServerLevel.java:342,393,628-629`).
    handling_tick: bool,
}

impl BlockEventQueue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `ServerLevel.isHandlingTick` (`ServerLevel.java:628-630`).
    #[must_use]
    pub const fn is_handling_tick(&self) -> bool {
        self.handling_tick
    }

    pub fn set_handling_tick(&mut self, handling: bool) -> bool {
        std::mem::replace(&mut self.handling_tick, handling)
    }

    pub fn block_event(&mut self, event: BlockEventData) -> bool {
        if !self.queued.insert(event) {
            return false;
        }
        self.pending.push_back(event);
        true
    }

    /// Whether an identical event is already pending.
    #[cfg(test)]
    #[must_use]
    pub fn contains(&self, event: BlockEventData) -> bool {
        self.queued.contains(&event)
    }

    #[cfg(test)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.pending.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    #[cfg(test)]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = BlockEventData> + '_ {
        self.pending.iter().copied()
    }

    pub(crate) fn pop(&mut self) -> Option<BlockEventData> {
        let event = self.pending.pop_front()?;
        self.queued.remove(&event);
        Some(event)
    }
}

#[cfg(test)]
mod tests;
