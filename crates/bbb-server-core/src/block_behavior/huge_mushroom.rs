use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::directions::relative;

use super::{
    block_type_of, default_block_state_id, direction_serialized_name, with_property, BlockStateAt,
};

#[must_use]
pub(super) fn is_huge_mushroom(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::HugeMushroomBlock)
}

/// `HugeMushroomBlock.getStateForPlacement`
/// (`HugeMushroomBlock.java:47-57`).
pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    block_state_at: BlockStateAt<'_>,
) -> Option<i32> {
    let mut state = default_block_state_id(block_type_id);
    if !is_huge_mushroom(state) {
        return None;
    }
    for direction in [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        let neighbor_type = block_type_of(block_state_at(relative(pos, direction)))?;
        state = with_property(
            state,
            direction_serialized_name(direction),
            if neighbor_type == block_type_id {
                "false"
            } else {
                "true"
            },
        )?;
    }
    Some(state)
}

/// `HugeMushroomBlock.updateShape` (`HugeMushroomBlock.java:60-72`).
#[must_use]
pub(super) fn update_shape(
    block_state_id: i32,
    direction: Direction,
    neighbour_state_id: i32,
) -> i32 {
    let Some(block_type_id) =
        block_type_of(block_state_id).filter(|_| is_huge_mushroom(block_state_id))
    else {
        return block_state_id;
    };
    if block_type_of(neighbour_state_id) != Some(block_type_id) {
        return block_state_id;
    }
    with_property(
        block_state_id,
        direction_serialized_name(direction),
        "false",
    )
    .unwrap_or(block_state_id)
}
