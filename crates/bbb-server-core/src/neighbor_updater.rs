//! Bounded neighbor work executed by the live level.

use crate::block_behavior::redstone_wire::experimental::RedstoneOrientation;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::{opposite, relative};

pub const UPDATE_ORDER: [Direction; 6] = [
    Direction::West,
    Direction::East,
    Direction::Down,
    Direction::Up,
    Direction::North,
    Direction::South,
];

pub const UPDATE_SHAPE_ORDER: [Direction; 6] = [
    Direction::West,
    Direction::East,
    Direction::North,
    Direction::South,
    Direction::Down,
    Direction::Up,
];

/// Block.UPDATE_LIMIT (Block.java:111).
pub const UPDATE_LIMIT: i32 = 512;

/// SharedConstants.MAX_CHAINED_NEIGHBOR_UPDATES (SharedConstants.java:132).
const MAX_CHAINED_NEIGHBOR_UPDATES: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockNeighborUpdate {
    pub pos: BlockPos,
    pub state: Option<i32>,
    pub source_block: i32,
    pub orientation: Option<RedstoneOrientation>,
    pub moved_by_piston: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeNeighborUpdate {
    pub direction: Direction,
    pub pos: BlockPos,
    pub neighbor_pos: BlockPos,
    pub neighbor_state: i32,
    pub update_flags: i32,
    pub update_limit: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborUpdate {
    Block(BlockNeighborUpdate),
    Shape(ShapeNeighborUpdate),
}

/// `CollectingNeighborUpdater` (`CollectingNeighborUpdater.java:68-110`):
/// depth-first. Updates queued while one runs are collected in `added` and run,
/// in queue order, before the rest of the updates already on the stack.
#[derive(Debug, Clone, Default)]
pub(crate) struct NeighborUpdates {
    stack: Vec<NeighborUpdate>,
    added: Vec<NeighborUpdate>,
    draining: bool,
    count: usize,
}

impl NeighborUpdates {
    fn push(&mut self, update: NeighborUpdate) {
        if self.count < MAX_CHAINED_NEIGHBOR_UPDATES {
            self.added.push(update);
        }
        self.count = self.count.saturating_add(1);
    }

    pub(crate) fn begin(&mut self) -> bool {
        if self.draining {
            return false;
        }
        self.draining = true;
        true
    }

    pub(crate) fn pop(&mut self) -> Option<NeighborUpdate> {
        self.stack.extend(self.added.drain(..).rev());
        self.stack.pop()
    }

    pub(crate) fn finish(&mut self) {
        self.draining = false;
        self.count = 0;
    }

    pub(crate) fn neighbor_changed(&mut self, pos: BlockPos, source_block: i32) {
        self.neighbor_changed_with_orientation(pos, source_block, None);
    }

    pub(crate) fn neighbor_changed_with_orientation(
        &mut self,
        pos: BlockPos,
        source_block: i32,
        orientation: Option<RedstoneOrientation>,
    ) {
        self.push(NeighborUpdate::Block(BlockNeighborUpdate {
            pos,
            state: None,
            source_block,
            orientation,
            moved_by_piston: false,
        }));
    }

    pub(crate) fn neighbor_changed_with_state_and_orientation(
        &mut self,
        state: i32,
        pos: BlockPos,
        source_block: i32,
        orientation: Option<RedstoneOrientation>,
    ) {
        self.push(NeighborUpdate::Block(BlockNeighborUpdate {
            pos,
            state: Some(state),
            source_block,
            orientation,
            moved_by_piston: false,
        }));
    }

    pub(crate) fn update_neighbors_at(&mut self, pos: BlockPos, source_block: i32) {
        self.update_neighbors_at_with_orientation(pos, source_block, None, None);
    }

    pub(crate) fn update_neighbors_at_with_orientation(
        &mut self,
        pos: BlockPos,
        source_block: i32,
        skip_direction: Option<Direction>,
        orientation: Option<RedstoneOrientation>,
    ) {
        for direction in UPDATE_ORDER {
            if Some(direction) != skip_direction {
                self.neighbor_changed_with_orientation(
                    relative(pos, direction),
                    source_block,
                    orientation.map(|base| base.with_front(direction)),
                );
            }
        }
    }

    pub(crate) fn shape_update(&mut self, update: ShapeNeighborUpdate) {
        self.push(NeighborUpdate::Shape(update));
    }

    pub(crate) fn update_neighbor_shapes(
        &mut self,
        pos: BlockPos,
        state: i32,
        update_flags: i32,
        update_limit: i32,
    ) {
        for direction in UPDATE_SHAPE_ORDER {
            self.shape_update(ShapeNeighborUpdate {
                direction: opposite(direction),
                pos: relative(pos, direction),
                neighbor_pos: pos,
                neighbor_state: state,
                update_flags,
                update_limit,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn popped(updates: &mut NeighborUpdates) -> Option<BlockPos> {
        match updates.pop()? {
            NeighborUpdate::Block(update) => Some(update.pos),
            NeighborUpdate::Shape(update) => Some(update.pos),
        }
    }

    #[test]
    fn updates_queued_by_a_running_update_run_before_its_siblings() {
        let origin = BlockPos { x: 0, y: 0, z: 0 };
        let mut updates = NeighborUpdates::default();
        updates.update_neighbors_at(origin, 1);
        assert!(updates.begin());
        let west = relative(origin, Direction::West);
        assert_eq!(popped(&mut updates), Some(west));
        updates.update_neighbors_at(west, 1);
        for direction in UPDATE_ORDER {
            assert_eq!(popped(&mut updates), Some(relative(west, direction)));
        }
        for direction in &UPDATE_ORDER[1..] {
            assert_eq!(popped(&mut updates), Some(relative(origin, *direction)));
        }
        assert_eq!(popped(&mut updates), None);
        updates.finish();
    }
}
