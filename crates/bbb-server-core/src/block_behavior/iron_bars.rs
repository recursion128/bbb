use super::{block_tag_matches, BlockShapeCtx};

use crate::fluid::FlowingFluid;
use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::FluidFamily;

use super::{
    block_type_of, bool_property, direction_serialized_name, face_sturdy_at, with_property,
};

const WATERLOGGED: &str = "waterlogged";
const WALLS_TAG: &str = "minecraft:walls";

#[must_use]
pub(super) fn is_iron_bars(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|entry| entry.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::IronBarsBlock)
}

fn is_horizontal(direction: Direction) -> bool {
    !matches!(direction, Direction::Down | Direction::Up)
}

pub(super) fn attaches_to_at(
    neighbour_state: i32,
    neighbour_pos: BlockPos,
    direction: Direction,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    is_iron_bars(neighbour_state)
        || block_tag_matches(neighbour_state, WALLS_TAG, membership)
        || !super::fence::is_exception_for_connection_at(neighbour_state, membership)
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
    let next = if !is_horizontal(direction_to_neighbour) {
        block_state_id
    } else {
        let neighbour_pos = relative(pos, direction_to_neighbour);
        let attaches = attaches_to_at(
            neighbour_state,
            neighbour_pos,
            direction_to_neighbour,
            &|registry, tag, id| level.registry_tag_membership(registry, tag, id),
        );
        with_property(
            block_state_id,
            direction_serialized_name(direction_to_neighbour),
            if attaches { "true" } else { "false" },
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
