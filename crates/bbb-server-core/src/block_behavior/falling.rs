use super::{BlockShapeCtx, LevelCtx};
use bbb_protocol::block_state_types::vanilla_block_state_support_facts;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaOnPlaceOverride, VANILLA_BLOCK_DRAGON_EGG_ID,
};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::fluid::block_state_is_air;

use super::{block_type_of, state_liquid};

const FIRE_TAG: &str = "minecraft:fire";

/// `FallingBlock.getDelayAfterPlace` (`FallingBlock.java:59-61`).
const DELAY_AFTER_PLACE: i32 = 2;

/// `DragonEggBlock.getDelayAfterPlace` (`DragonEggBlock.java:83-85`), the only
/// 26.1 override of it.
const DRAGON_EGG_DELAY_AFTER_PLACE: i32 = 5;

/// Generated `on_place_override` `FallingBlock`: the twenty-three
/// registrations that inherit both bodies here.
#[must_use]
pub fn is_falling(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.on_place_override)
        == Some(VanillaOnPlaceOverride::FallingBlock)
}

/// `FallingBlock.isFree(BlockState)` (`FallingBlock.java`): the four ways
/// a block below a falling block counts as nothing to stand on.
pub fn is_free_with_tags(tags: &dyn crate::block_getter::BlockTags, block_state_id: i32) -> bool {
    block_state_is_air(block_state_id)
        || state_liquid(block_state_id)
        || vanilla_block_state_support_facts(block_state_id)
            .is_some_and(|facts| facts.replaceable())
        || tags.block_in_tag(block_state_id, FIRE_TAG)
}

pub(crate) fn is_free(level: &LevelCtx<'_, '_>, block_state_id: i32) -> Option<bool> {
    Some(is_free_with_tags(level.tags(), block_state_id))
}

fn delay_after_place(block_state_id: i32) -> i32 {
    if block_type_of(block_state_id) == Some(VANILLA_BLOCK_DRAGON_EGG_ID) {
        DRAGON_EGG_DELAY_AFTER_PLACE
    } else {
        DELAY_AFTER_PLACE
    }
}

/// The `scheduleTick(pos, this, getDelayAfterPlace())` both bodies below are.
pub(super) fn schedule_self(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(tick_type) = BlockTickType::of_block_state(block_state_id) else {
        return;
    };
    level.schedule_block_tick(
        pos,
        tick_type,
        delay_after_place(block_state_id),
        TickPriority::Normal,
    );
}

pub(super) fn on_place(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    schedule_self(&mut BlockShapeCtx::Live(level), pos, block_state_id);
}

#[must_use]
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    schedule_self(level, pos, block_state_id);
    block_state_id
}
