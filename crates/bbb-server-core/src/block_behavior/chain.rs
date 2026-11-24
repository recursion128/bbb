use crate::fluid::with_property;
use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::{block_type_of, default_block_state_id, BlockStateAt};

const AXIS: &str = "axis";
const WATERLOGGED: &str = "waterlogged";

#[must_use]
pub(super) fn is_chain(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::ChainBlock)
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    clicked_face: Direction,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let replaced_fluid = block_state_fluid_state(replaced);

    let state = default_block_state_id(block_type_id);
    if !is_chain(state) {
        return None;
    }
    let axis = match clicked_face {
        Direction::West | Direction::East => "x",
        Direction::Down | Direction::Up => "y",
        Direction::North | Direction::South => "z",
    };
    let state = with_property(state, AXIS, axis)?;
    with_property(
        state,
        WATERLOGGED,
        if replaced_fluid.is_source() && replaced_fluid.is_same_family(FluidFamily::Water) {
            "true"
        } else {
            "false"
        },
    )
}
