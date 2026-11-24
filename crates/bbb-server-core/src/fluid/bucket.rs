//! `BucketItem.emptyContents` (`BucketItem.java:106-150`): where a bucket's
//! fluid goes.

use bbb_protocol::block_state_types::vanilla_block_state_fluid_replaceable;
use bbb_protocol::block_types::VanillaLiquidBlockContainerOverride;
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::{
    block_state_fluid_state, block_state_is_air, FluidFamily, FluidState,
};

use super::{
    can_hold_specific_fluid, liquid_block_container, liquid_block_family, place_liquid,
    UPDATE_LIMIT,
};
use crate::world::simulation_level::LevelCtx;

/// `UPDATE_ALL_IMMEDIATE` (`Block.java:105`), the flags `:142` writes with.
const UPDATE_ALL_IMMEDIATE: i32 = 11;

/// The two things `emptyContents` reads off its user.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BucketUser {
    pub(crate) shift_key_down: bool,
    pub(crate) creative: bool,
}

/// How a successful `emptyContents` ended, at the cell it ended in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BucketEmptied {
    /// `:121-132`: water where it evaporates.
    Evaporated,
    /// `:133-147`: a container filled or a source placed; the empty sound
    /// plays.
    Placed,
}

/// `BucketItem.emptyContents` for `fluid`, with the clicked cell and face
/// when the use came from a hit (`hitResult != null`).
pub(crate) fn empty_bucket(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    fluid: FluidFamily,
    user: BucketUser,
    hit: Option<(BlockPos, Direction)>,
) -> Option<(BlockPos, BucketEmptied)> {
    let state = level.block_state(pos);
    let container = liquid_block_container(state);
    let may_replace = vanilla_block_state_fluid_replaceable(state) == Some(true);
    // `LiquidBlockContainer.canPlaceLiquid(user, ...)`.
    let container_takes = fluid == FluidFamily::Water
        && match container {
            None => false,
            Some(VanillaLiquidBlockContainerOverride::BarrierBlock) => user.creative,
            Some(_) => can_hold_specific_fluid(state, fluid.source_identity()),
        };
    let place_liquid_here = may_replace || container_takes;
    if !(block_state_is_air(state) || place_liquid_here && (!user.shift_key_down || hit.is_none()))
    {
        let (clicked, face) = hit?;
        return empty_bucket(level, relative(clicked, face), fluid, user, None);
    }
    if fluid == FluidFamily::Water && level.water_evaporates(pos) {
        return Some((pos, BucketEmptied::Evaporated));
    }
    let source = FluidState::source(fluid, false);
    if container.is_some() && fluid == FluidFamily::Water {
        place_liquid(level, pos, state, source);
        return Some((pos, BucketEmptied::Placed));
    }
    if may_replace && liquid_block_family(state).is_none() {
        level.destroy_block(pos, true);
    }
    if !level.set_block(
        pos,
        source.create_legacy_block(),
        UPDATE_ALL_IMMEDIATE,
        UPDATE_LIMIT,
    ) && !block_state_fluid_state(state).is_source()
    {
        return None;
    }
    Some((pos, BucketEmptied::Placed))
}
