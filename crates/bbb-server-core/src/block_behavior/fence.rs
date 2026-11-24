use super::{block_tag_matches, BlockShapeCtx};

use crate::fluid::FlowingFluid;
use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockUseWithoutItemOverride, VANILLA_BLOCK_BARRIER_ID,
    VANILLA_BLOCK_CARVED_PUMPKIN_ID, VANILLA_BLOCK_JACK_O_LANTERN_ID, VANILLA_BLOCK_MELON_ID,
    VANILLA_BLOCK_PUMPKIN_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::FluidFamily;

use super::{
    block_type_of, bool_property, direction_property, direction_serialized_name, face_sturdy_at,
    with_property,
};

const WATERLOGGED: &str = "waterlogged";
const FACING: &str = "facing";
const FENCES_TAG: &str = "minecraft:fences";
const WOODEN_FENCES_TAG: &str = "minecraft:wooden_fences";
const SHULKER_BOXES_TAG: &str = "minecraft:shulker_boxes";

#[must_use]
pub(super) fn is_fence(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.use_without_item_override)
        == Some(VanillaBlockUseWithoutItemOverride::FenceBlock)
}

pub(super) fn is_exception_for_connection_at(
    block_state_id: i32,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    bbb_server_types::block_queries::is_leaves(block_state_id)
        || block_type_of(block_state_id).is_some_and(|block_type_id| {
            matches!(
                block_type_id,
                VANILLA_BLOCK_BARRIER_ID
                    | VANILLA_BLOCK_CARVED_PUMPKIN_ID
                    | VANILLA_BLOCK_JACK_O_LANTERN_ID
                    | VANILLA_BLOCK_MELON_ID
                    | VANILLA_BLOCK_PUMPKIN_ID
            )
        })
        || block_tag_matches(block_state_id, SHULKER_BOXES_TAG, membership)
}

fn horizontal_axis(direction: Direction) -> Option<char> {
    match direction {
        Direction::North | Direction::South => Some('z'),
        Direction::West | Direction::East => Some('x'),
        Direction::Down | Direction::Up => None,
    }
}

fn gate_connects(neighbour_state: i32, direction: Direction) -> bool {
    if !super::footing::is_fence_gate(neighbour_state) {
        return false;
    }
    direction_property(neighbour_state, FACING)
        .and_then(horizontal_axis)
        .zip(horizontal_axis(direction))
        .is_some_and(|(gate_axis, direction_axis)| gate_axis != direction_axis)
}

pub(super) fn connects_to_at(
    block_state_id: i32,
    neighbour_state: i32,
    neighbour_pos: BlockPos,
    direction: Direction,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    let same_fence = block_tag_matches(neighbour_state, FENCES_TAG, membership)
        && block_tag_matches(block_state_id, WOODEN_FENCES_TAG, membership)
            == block_tag_matches(neighbour_state, WOODEN_FENCES_TAG, membership);
    gate_connects(neighbour_state, direction)
        || same_fence
        || !is_exception_for_connection_at(neighbour_state, membership)
            && face_sturdy_at(
                neighbour_state,
                neighbour_pos,
                opposite(direction),
                SupportType::Full,
            )
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    let next = if horizontal_axis(direction_to_neighbour).is_none() {
        block_state_id
    } else {
        let neighbour_pos = relative(pos, direction_to_neighbour);
        let connects = connects_to_at(
            block_state_id,
            neighbour_state,
            neighbour_pos,
            direction_to_neighbour,
            &|registry, tag, id| level.registry_tag_membership(registry, tag, id),
        );
        with_property(
            block_state_id,
            direction_serialized_name(direction_to_neighbour),
            if connects { "true" } else { "false" },
        )
        .unwrap_or(block_state_id)
    };
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    next
}
