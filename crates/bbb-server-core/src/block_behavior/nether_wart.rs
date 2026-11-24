//! `NetherWartBlock` (`net/minecraft/world/level/block/NetherWartBlock.java`).
//!
//! The smallest random tick in the family, and the only growth in it that
//! reads nothing off the level: a flat one-in-ten draw moves `AGE` up by one
//! until it reaches 3 (`:52-58`). No light, no soil quality, no neighbours.
//!
//! `NetherWartBlock` is a `VegetationBlock` that does not replace
//! `canSurvive`, so its survival is [`super::vegetation`]'s
//! `VegetationBlock.canSurvive` reaching `NetherWartBlock.mayPlaceOn`
//! (`:42-44`), and this module deliberately carries only the growth. The
//! dispatch in [`super::block_behavior_owner`] hands the survival questions
//! straight back to that module rather than restating them.
//!
//! # What is not here
//!
//! * `getCloneItemStack` (`:61-63`) and `getShape` (`:37-39`), a pick-block
//!   and a collision surface.

use super::LevelCtx;
use bbb_protocol::block_types::VanillaRandomTickOverride;
use bbb_protocol::packets::BlockPos;

use super::{
    block_type_of, int_property, random_tick_override, with_property, SET_BLOCK_UPDATE_CLIENTS,
};

/// The property `NetherWartBlock.AGE` names (`NetherWartBlock.java:23`,
/// `BlockStateProperties.AGE_3` = `IntegerProperty.create("age", 0, 3)`,
/// `BlockStateProperties.java:93`).
const AGE: &str = "age";

/// `NetherWartBlock.MAX_AGE` (`NetherWartBlock.java:22`).
///
/// It is also the whole of `isRandomlyTicking` (`:47-49`): a wart at 3 is not
/// visited at all, which is why the `age < 3` test inside `randomTick`
/// (`:54`) can never fail when the scheduler is the caller.
pub const MAX_AGE: i32 = 3;

/// The bound of the growth draw (`NetherWartBlock.java:54`).
pub const GROWTH_DRAW_BOUND: i32 = 10;

/// Whether a stored state's behaviour belongs to `NetherWartBlock`.
///
/// `NetherWartBlock` has no subclass in 26.1, so the generated
/// `random_tick_override` class tag names the class outright.
#[must_use]
pub fn is_nether_wart(block_state_id: i32) -> bool {
    block_type_of(block_state_id).is_some_and(|block_type_id| {
        random_tick_override(block_type_id) == Some(VanillaRandomTickOverride::NetherWartBlock)
    })
}

///
/// The draw comes *after* the age test, so a wart at 3 consumes no randomness;
/// the write is flag 2 (`:56`), the same client-only update
/// [`super::crop`] uses, because nothing observes `AGE` from a neighbour.
pub(super) fn random_tick(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let Some(age) = int_property(block_state_id, AGE) else {
        return;
    };
    if age >= MAX_AGE || level.level_random().next_int(GROWTH_DRAW_BOUND) != 0 {
        return;
    }
    if let Some(grown) = with_property(block_state_id, AGE, &(age + 1).to_string()) {
        level.set_block(
            pos,
            grown,
            SET_BLOCK_UPDATE_CLIENTS,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
    }
}
