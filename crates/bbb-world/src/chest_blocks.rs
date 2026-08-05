//! `ChestBlock.combine` — the single owner of the vanilla double-chest
//! `DoubleBlockCombiner` result every chest-behaviour consumer folds over.
//!
//! `DoubleBlockCombiner.combineWithNeigbour` answers `acceptNone` when the
//! position has no chest block entity or when the blocked predicate holds
//! there, `acceptDouble` when the neighbour is a joined, unblocked half, and
//! `acceptSingle` otherwise. `ChestBlock.combine` passes
//! `ChestBlock::isChestBlockedAt` as that predicate unless the caller asks to
//! ignore blocking, so a solid redstone conductor or a sitting cat above the
//! chest collapses the result to `acceptNone`.

use bbb_protocol::{
    block_state_types::vanilla_block_state_support_facts,
    block_types::vanilla_block_type_for_resource_id,
};

use crate::{
    chest_lids::{ChestModelFacing, ChestModelHalf},
    BlockPos, BlockProbe, WorldStore,
};

/// The `DoubleBlockCombiner.NeighborCombineResult` a chest position folds to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChestCombineResult {
    None,
    Single,
    Double(BlockPos),
}

impl WorldStore {
    pub(crate) fn chest_is_blocked(&self, pos: BlockPos) -> Option<bool> {
        let above = self.probe_block(BlockPos {
            x: pos.x,
            y: pos.y + 1,
            z: pos.z,
        })?;
        Some(
            vanilla_block_state_support_facts(above.block_state_id)?.redstone_conductor()
                || self.entities.sitting_cat_intersects_chest_top(pos)?,
        )
    }

    pub(crate) fn chest_combine(
        &self,
        pos: BlockPos,
        block: &BlockProbe,
        ignore_being_blocked: bool,
    ) -> Option<ChestCombineResult> {
        if !self.chest_block_entity_present_at(pos, block) {
            return Some(ChestCombineResult::None);
        }
        if !ignore_being_blocked && self.chest_is_blocked(pos)? {
            return Some(ChestCombineResult::None);
        }
        let facing = block
            .block_properties
            .get("facing")
            .and_then(|value| ChestModelFacing::parse(value));
        let half = match block.block_properties.get("type").map(String::as_str) {
            Some("left") => ChestModelHalf::Left,
            Some("right") => ChestModelHalf::Right,
            _ => ChestModelHalf::Single,
        };
        let Some(facing) = facing else {
            return Some(ChestCombineResult::Single);
        };
        let Some(partner_pos) =
            self.joined_chest_partner_pos(pos, block.block_name.as_deref()?, facing, half)
        else {
            return Some(ChestCombineResult::Single);
        };
        if !ignore_being_blocked && self.chest_is_blocked(partner_pos)? {
            return Some(ChestCombineResult::None);
        }
        let partner = self.probe_block(partner_pos)?;
        if !self.chest_block_entity_present_at(partner_pos, &partner) {
            return Some(ChestCombineResult::Single);
        }
        Some(ChestCombineResult::Double(partner_pos))
    }

    fn chest_block_entity_present_at(&self, pos: BlockPos, block: &BlockProbe) -> bool {
        let expected = block
            .block_name
            .as_deref()
            .and_then(vanilla_block_type_for_resource_id)
            .and_then(|entry| entry.block_entity_type_id);
        expected.is_some() && self.block_entity_type_id_at(pos) == expected
    }
}
