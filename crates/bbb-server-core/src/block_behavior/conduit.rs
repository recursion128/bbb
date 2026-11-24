use crate::fluid::with_property;
use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::BlockPos;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{block_type_of, default_block_state_id, BlockStateAt};

const WATERLOGGED: &str = "waterlogged";

#[must_use]
pub(super) fn is_conduit(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::ConduitBlock)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    if vanilla_block_type(block_type_id)?.get_state_for_placement_override
        != Some(VanillaGetStateForPlacementOverride::ConduitBlock)
    {
        return None;
    }
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let fluid = block_state_fluid_state(replaced);
    with_property(
        default_block_state_id(block_type_id),
        WATERLOGGED,
        if fluid.is_same_family(FluidFamily::Water) && fluid.is_full() {
            "true"
        } else {
            "false"
        },
    )
}
