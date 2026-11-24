//! Ender chest opener changes and scheduled rechecks share one counter owner.

use super::LevelCtx;
use bbb_protocol::block_types::{vanilla_block_type, VanillaTickOverride};
use bbb_protocol::packets::BlockPos;

use super::block_type_of;
use super::container_openers::OpenersOwner;

#[must_use]
pub fn is_ender_chest(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.tick_override)
        == Some(VanillaTickOverride::EnderChestBlock)
}

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    super::container_openers::recheck_open(level, pos, block_state_id, OpenersOwner::EnderChest);
}

pub(crate) fn change_openers(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
    source_entity_id: i32,
    range: f64,
    opening: bool,
) -> bool {
    super::container_openers::change_openers(
        level,
        pos,
        state,
        OpenersOwner::EnderChest,
        source_entity_id,
        range,
        opening,
    )
}
