use super::LevelCtx;
use crate::fluid::with_property;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseWithoutItemOverride, VanillaGetStateForPlacementOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::opposite;

use super::container_openers::OpenersOwner;
use super::{block_type_of, default_block_state_id, direction_serialized_name};

const FACING: &str = "facing";

#[must_use]
pub fn is_barrel(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::BarrelBlock)
}

pub(super) fn state_for_placement(block_type_id: i32, nearest: Direction) -> Option<i32> {
    if vanilla_block_type(block_type_id)?.get_state_for_placement_override
        != Some(VanillaGetStateForPlacementOverride::BarrelBlock)
    {
        return None;
    }
    with_property(
        default_block_state_id(block_type_id),
        FACING,
        direction_serialized_name(opposite(nearest)),
    )
}

/// `SoundEvents.BARREL_OPEN` / `BARREL_CLOSE` as the codec names them.
pub const BARREL_OPEN_SOUND: &str = "minecraft:block.barrel.open";
pub const BARREL_CLOSE_SOUND: &str = "minecraft:block.barrel.close";

pub(super) fn tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    super::container_openers::recheck_open(level, pos, block_state_id, OpenersOwner::Barrel);
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
        OpenersOwner::Barrel,
        source_entity_id,
        range,
        opening,
    )
}
