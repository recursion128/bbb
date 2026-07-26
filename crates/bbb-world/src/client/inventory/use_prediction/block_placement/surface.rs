use std::collections::BTreeMap;

use bbb_protocol::{block_types::BlockTypeCatalogEntry, packets::Direction};

use super::{
    context::{direction_axis, direction_name, opposite, relative, DirectionAxis},
    state::{state_with_properties, BlockPlacementState},
};
use crate::{BlockPos, WorldStore};

pub(super) fn multiface_state_for_placement(
    world: &WorldStore,
    block_type: &BlockTypeCatalogEntry,
    target_pos: BlockPos,
    directions: [Direction; 6],
    default_state_id: i32,
) -> BlockPlacementState {
    let Some(old_state) = world.probe_block(target_pos) else {
        return BlockPlacementState::Unsupported;
    };
    let old_is_same = old_state.block_name.as_deref() == Some(block_type.resource_id);

    for direction in directions {
        let property = direction_name(direction);
        if old_is_same
            && old_state
                .block_properties
                .get(property)
                .is_some_and(|value| value == "true")
        {
            continue;
        }
        let Some(can_attach) = can_attach_to(world, target_pos, direction) else {
            return BlockPlacementState::Unsupported;
        };
        if !can_attach {
            continue;
        }
        if old_is_same {
            return state_with_properties(
                world,
                block_type,
                old_state.block_state_id,
                [(property, "true")],
            );
        }
        let waterlogged = old_state.fluid.is_some_and(|fluid| {
            fluid.kind == crate::TerrainFluidKind::Water && fluid.amount == 8 && !fluid.falling
        });
        return state_with_properties(
            world,
            block_type,
            default_state_id,
            [
                (property, "true"),
                ("waterlogged", boolean_name(waterlogged)),
            ],
        );
    }

    BlockPlacementState::Fail
}

pub(super) fn vine_state_for_placement(
    world: &WorldStore,
    block_type: &BlockTypeCatalogEntry,
    target_pos: BlockPos,
    directions: [Direction; 6],
    default_state_id: i32,
) -> BlockPlacementState {
    let Some(old_state) = world.probe_block(target_pos) else {
        return BlockPlacementState::Unsupported;
    };
    let old_is_vine = old_state.block_name.as_deref() == Some(block_type.resource_id);
    let base_state_id = if old_is_vine {
        old_state.block_state_id
    } else {
        default_state_id
    };

    for direction in directions {
        if direction == Direction::Down {
            continue;
        }
        let property = direction_name(direction);
        if old_is_vine
            && old_state
                .block_properties
                .get(property)
                .is_some_and(|value| value == "true")
        {
            continue;
        }
        let Some(can_support) = vine_can_support_at_face(world, target_pos, direction) else {
            return BlockPlacementState::Unsupported;
        };
        if can_support {
            return state_with_properties(world, block_type, base_state_id, [(property, "true")]);
        }
    }

    if old_is_vine {
        BlockPlacementState::State(old_state.block_state_id)
    } else {
        BlockPlacementState::Fail
    }
}

pub(super) fn vine_can_survive(
    world: &WorldStore,
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
) -> Option<bool> {
    if property_is_true(properties, "up")? && can_attach_to(world, pos, Direction::Up)? {
        return Some(true);
    }
    for direction in [
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ] {
        if property_is_true(properties, direction_name(direction))?
            && vine_can_support_at_face(world, pos, direction)?
        {
            return Some(true);
        }
    }
    Some(false)
}

fn vine_can_support_at_face(
    world: &WorldStore,
    pos: BlockPos,
    direction: Direction,
) -> Option<bool> {
    if direction == Direction::Down {
        return Some(false);
    }
    if can_attach_to(world, pos, direction)? {
        return Some(true);
    }
    if direction_axis(direction) == DirectionAxis::Y {
        return Some(false);
    }
    let above = world.probe_block(relative(pos, Direction::Up))?;
    Some(
        above.block_name.as_deref() == Some("minecraft:vine")
            && above
                .block_properties
                .get(direction_name(direction))
                .is_some_and(|value| value == "true"),
    )
}

fn can_attach_to(world: &WorldStore, pos: BlockPos, direction: Direction) -> Option<bool> {
    let support = world.probe_block(relative(pos, direction))?;
    world.placement_full_support_or_collision_face(&support, opposite(direction))
}

fn property_is_true(properties: &BTreeMap<String, String>, property: &str) -> Option<bool> {
    match properties.get(property)?.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

const fn boolean_name(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}
