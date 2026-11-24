use super::BlockShapeCtx;
use crate::fluid::FlowingFluid;
use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::fluid::FluidFamily;

use super::{block_type_of, bool_property};

const WATERLOGGED: &str = "waterlogged";

#[must_use]
pub(super) fn is_slab(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::SlabBlock)
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    block_state_id
}
