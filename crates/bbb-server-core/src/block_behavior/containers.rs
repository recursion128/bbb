//!
//! Ten classes over thirty-eight blocks — furnaces, barrels, brewing stands,
//! chests, crafters, creaking hearts, dispensers, hoppers, jukeboxes and
//! shulker boxes — whose whole `affectNeighborsAfterRemoval` is this one call.
//! A comparator reading a container has to be told the moment it goes, and
//! nothing else about the removal is the block's business.

use super::LevelCtx;
use bbb_protocol::block_types::{vanilla_block_type, VanillaAffectNeighborsAfterRemovalOverride};
use bbb_protocol::packets::BlockPos;

use super::block_type_of;

#[must_use]
pub fn is_container(block_state_id: i32) -> bool {
    matches!(
        block_type_of(block_state_id)
            .and_then(vanilla_block_type)
            .and_then(|entry| entry.affect_neighbors_after_removal_override),
        Some(
            VanillaAffectNeighborsAfterRemovalOverride::AbstractFurnaceBlock
                | VanillaAffectNeighborsAfterRemovalOverride::BarrelBlock
                | VanillaAffectNeighborsAfterRemovalOverride::BrewingStandBlock
                | VanillaAffectNeighborsAfterRemovalOverride::ChestBlock
                | VanillaAffectNeighborsAfterRemovalOverride::CrafterBlock
                | VanillaAffectNeighborsAfterRemovalOverride::CreakingHeartBlock
                | VanillaAffectNeighborsAfterRemovalOverride::DispenserBlock
                | VanillaAffectNeighborsAfterRemovalOverride::HopperBlock
                | VanillaAffectNeighborsAfterRemovalOverride::JukeboxBlock
                | VanillaAffectNeighborsAfterRemovalOverride::ShulkerBoxBlock
        )
    )
}

pub(crate) fn update_neighbours_after_destroy(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.update_neighbour_for_output_signal(pos, block_type_id);
}
