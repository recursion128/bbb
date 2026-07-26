use bbb_protocol::{
    block_state_types::VanillaBlockSupportType,
    block_types::{vanilla_block_type_for_resource_id, VanillaCanSurviveOverride},
    packets::Direction,
};

use super::{
    super::LocalBlockUseContext,
    context::{direction_axis, horizontal_direction, opposite, relative, DirectionAxis},
};
use crate::{BlockPos, TerrainFluidKind, WorldStore};

#[derive(Debug, Clone, Copy)]
pub(super) struct ScaffoldingPlacementContext {
    pub(super) context: LocalBlockUseContext,
    pub(super) target_pos: BlockPos,
    pub(super) replace_clicked: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ScaffoldingContextUpdate {
    Context(ScaffoldingPlacementContext),
    Fail,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ScaffoldingPlacement {
    pub(super) distance: i32,
    pub(super) bottom: bool,
    pub(super) waterlogged: bool,
}

pub(super) fn update_placement_context(
    world: &WorldStore,
    context: LocalBlockUseContext,
    target_pos: BlockPos,
    replace_clicked: bool,
) -> ScaffoldingContextUpdate {
    let Some(target) = world.probe_block(target_pos) else {
        return ScaffoldingContextUpdate::Unsupported;
    };
    if target.block_name.as_deref() != Some("minecraft:scaffolding") {
        return match distance(world, target_pos) {
            Some(7) => ScaffoldingContextUpdate::Fail,
            Some(_) => ScaffoldingContextUpdate::Context(ScaffoldingPlacementContext {
                context,
                target_pos,
                replace_clicked,
            }),
            None => ScaffoldingContextUpdate::Unsupported,
        };
    }

    let Some(pose) = world.local_player().pose else {
        return ScaffoldingContextUpdate::Unsupported;
    };
    let Some(player_horizontal) = horizontal_direction(pose.y_rot) else {
        return ScaffoldingContextUpdate::Unsupported;
    };
    let direction = if context.secondary_use_active {
        if context.inside {
            opposite(context.face)
        } else {
            context.face
        }
    } else if context.face == Direction::Up {
        player_horizontal
    } else {
        Direction::Up
    };

    let mut placement_pos = relative(target_pos, direction);
    let mut horizontal_distance = 0;
    while horizontal_distance < 7 {
        let Some(candidate) = world.probe_block(placement_pos) else {
            return ScaffoldingContextUpdate::Unsupported;
        };
        if candidate.block_name.as_deref() != Some("minecraft:scaffolding") {
            let Some(candidate_type) = candidate
                .block_name
                .as_deref()
                .and_then(vanilla_block_type_for_resource_id)
            else {
                return ScaffoldingContextUpdate::Unsupported;
            };
            // `ScaffoldingBlockItem` passes its original context. A vine
            // candidate sees the clicked scaffolding, not itself, and thus
            // falls through to VineBlock's static replaceable property.
            let replaceable = if candidate_type.can_survive_override
                == Some(VanillaCanSurviveOverride::VineBlock)
            {
                candidate_type.replaceable
            } else {
                let Some(replaceable) =
                    world.local_block_is_replaceable_by_different_item(placement_pos)
                else {
                    return ScaffoldingContextUpdate::Unsupported;
                };
                replaceable
            };
            return if replaceable {
                ScaffoldingContextUpdate::Context(ScaffoldingPlacementContext {
                    context: context_at(context, placement_pos, direction),
                    target_pos: placement_pos,
                    replace_clicked: true,
                })
            } else {
                ScaffoldingContextUpdate::Fail
            };
        }
        placement_pos = relative(placement_pos, direction);
        if direction_axis(direction) != DirectionAxis::Y {
            horizontal_distance += 1;
        }
    }

    ScaffoldingContextUpdate::Fail
}

pub(super) fn placement(world: &WorldStore, pos: BlockPos) -> Option<ScaffoldingPlacement> {
    let distance = distance(world, pos)?;
    let below = world.probe_block(relative(pos, Direction::Down))?;
    Some(ScaffoldingPlacement {
        distance,
        bottom: distance > 0 && below.block_name.as_deref() != Some("minecraft:scaffolding"),
        waterlogged: world.probe_block(pos)?.fluid.is_some_and(|fluid| {
            fluid.kind == TerrainFluidKind::Water && fluid.amount == 8 && !fluid.falling
        }),
    })
}

pub(super) fn distance(world: &WorldStore, pos: BlockPos) -> Option<i32> {
    let below = world.probe_block(relative(pos, Direction::Down))?;
    let mut distance = 7;
    if below.block_name.as_deref() == Some("minecraft:scaffolding") {
        distance = integer_property(&below.block_properties, "distance")?;
    } else if world.placement_face_sturdy(&below, Direction::Up, VanillaBlockSupportType::Full)? {
        return Some(0);
    }

    for direction in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        let neighbour = world.probe_block(relative(pos, direction))?;
        if neighbour.block_name.as_deref() == Some("minecraft:scaffolding") {
            distance = distance.min(integer_property(&neighbour.block_properties, "distance")? + 1);
            if distance == 1 {
                break;
            }
        }
    }
    Some(distance)
}

fn context_at(
    context: LocalBlockUseContext,
    pos: BlockPos,
    direction: Direction,
) -> LocalBlockUseContext {
    let cursor = match direction {
        Direction::Down => [0.5, 0.0, 0.5],
        Direction::Up => [0.5, 1.0, 0.5],
        Direction::North => [0.5, 0.5, 0.0],
        Direction::South => [0.5, 0.5, 1.0],
        Direction::West => [0.0, 0.5, 0.5],
        Direction::East => [1.0, 0.5, 0.5],
    };
    LocalBlockUseContext {
        pos,
        face: direction,
        cursor,
        inside: false,
        ..context
    }
}

fn integer_property(
    properties: &std::collections::BTreeMap<String, String>,
    property: &str,
) -> Option<i32> {
    properties.get(property)?.parse().ok()
}
