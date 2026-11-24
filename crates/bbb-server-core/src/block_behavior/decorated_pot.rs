use super::{BlockShapeCtx, LevelCtx};
use crate::fluid::{with_property, FlowingFluid};
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseWithoutItemOverride, VanillaGetStateForPlacementOverride,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{
    block_type_of, bool_property, default_block_state_id, direction_serialized_name, BlockStateAt,
};

const CRACKED: &str = "cracked";
const FACING: &str = "facing";
const WATERLOGGED: &str = "waterlogged";

pub const INSERT_SOUND: &str = "minecraft:block.decorated_pot.insert";
pub const INSERT_FAIL_SOUND: &str = "minecraft:block.decorated_pot.insert_fail";

/// `DecoratedPotBlock.useWithoutItem` is unique to that class.
#[must_use]
pub fn is_decorated_pot(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::DecoratedPotBlock)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    horizontal_direction: Direction,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if vanilla_block_type(block_type_id)?.get_state_for_placement_override
        != Some(VanillaGetStateForPlacementOverride::DecoratedPotBlock)
    {
        return None;
    }
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let fluid = block_state_fluid_state(replaced);
    if matches!(horizontal_direction, Direction::Down | Direction::Up) {
        return None;
    }
    let state = with_property(
        default_block_state_id(block_type_id),
        FACING,
        direction_serialized_name(horizontal_direction),
    )?;
    let state = with_property(
        state,
        WATERLOGGED,
        if fluid.is_source() && fluid.is_same_family(FluidFamily::Water) {
            "true"
        } else {
            "false"
        },
    )?;
    with_property(state, CRACKED, "false")
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

/// `DecoratedPotBlock.affectNeighborsAfterRemoval` (`:176-178`).
pub(super) fn affect_neighbors_after_removal(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
) {
    let Some(block_type_id) = block_type_of(block_state_id) else {
        return;
    };
    level.update_neighbour_for_output_signal(pos, block_type_id);
}
