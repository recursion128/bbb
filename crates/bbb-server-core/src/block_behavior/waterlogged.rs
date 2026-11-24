//! Source-water placement and shape updates that only rearm contained water.

use super::{block_type_of, bool_property, default_block_state_id, BlockShapeCtx, BlockStateAt};
use crate::fluid::{with_property, FlowingFluid};
use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

/// `EnderChestBlock.java:148-162`, `ShelfBlock.java:296-311`,
/// `LightBlock.java:86-100`, `SculkShriekerBlock.java:99-113`.
pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
) -> i32 {
    if bool_property(block_state_id, "waterlogged") == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    block_state_id
}

pub(super) fn is_source_water(block_state_id: i32) -> bool {
    let fluid = block_state_fluid_state(block_state_id);
    fluid.is_source() && fluid.is_same_family(FluidFamily::Water)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    with_property(
        default_block_state_id(block_type_id),
        "waterlogged",
        if is_source_water(replaced) {
            "true"
        } else {
            "false"
        },
    )
}

pub(super) fn is_waterlogged_only(state: i32) -> bool {
    matches!(
        block_type_of(state)
            .and_then(vanilla_block_type)
            .and_then(|block| block.get_state_for_placement_override),
        Some(
            VanillaGetStateForPlacementOverride::WaterloggedTransparentBlock
                | VanillaGetStateForPlacementOverride::MangroveRootsBlock
                | VanillaGetStateForPlacementOverride::BarrierBlock
                | VanillaGetStateForPlacementOverride::HeavyCoreBlock
        )
    )
}
