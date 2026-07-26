use bbb_protocol::{
    block_state_types::vanilla_block_default_state_id,
    block_types::{
        BlockTypeCatalogEntry, VanillaGetStateForPlacementOverride, VANILLA_BLOCK_AIR_ID,
        VANILLA_BLOCK_TRIPWIRE_HOOK_ID, VANILLA_BLOCK_WATER_ID,
    },
    packets::Direction,
};

use super::{
    super::{LocalBlockUseContext, WorldStore},
    context::{
        clockwise, counterclockwise, direction_axis, direction_name, opposite, relative,
        DirectionAxis,
    },
    signal::placement_neighbor_signal_is_proven_absent,
    state::block_item_can_replace,
};
use crate::{client::block_collision_shape_is_full_block, BlockPos, BlockProbe, TerrainFluidKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DoorPlacement {
    pub(super) hinge: &'static str,
    pub(super) powered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TripWireHookClientPlacement {
    pub(super) states: Vec<(BlockPos, i32)>,
    pub(super) suppressed_state_sound_count: u8,
    pub(super) suppressed_detach_pitch: bool,
}

pub(super) fn double_high_above_can_replace(
    world: &WorldStore,
    target_pos: BlockPos,
    block_type: &BlockTypeCatalogEntry,
    context: LocalBlockUseContext,
) -> Option<bool> {
    let max_y = world
        .dimension()
        .min_y
        .saturating_add(world.dimension().height)
        .saturating_sub(1);
    if target_pos.y >= max_y {
        return Some(false);
    }
    block_item_can_replace(
        world,
        relative(target_pos, Direction::Up),
        block_type,
        context,
        false,
    )
}

pub(super) fn double_high_client_upper_state(
    world: &WorldStore,
    block_type: &BlockTypeCatalogEntry,
    target_pos: BlockPos,
    final_lower_state_id: i32,
) -> Option<i32> {
    let above_pos = relative(target_pos, Direction::Up);
    let above = world.probe_block(above_pos)?;
    let above_is_water = above
        .fluid
        .is_some_and(|fluid| fluid.kind == TerrainFluidKind::Water);
    match block_type.get_state_for_placement_override? {
        VanillaGetStateForPlacementOverride::DoublePlantBlock => {
            let default_state_id = vanilla_block_default_state_id(block_type.id)?;
            let default_state = world.registries().block_state(default_state_id)?;
            if default_state.name != block_type.resource_id {
                return None;
            }
            let mut properties = default_state.properties.clone();
            if !properties.contains_key("half") {
                return None;
            }
            properties.insert("half".to_string(), "upper".to_string());
            if properties.contains_key("waterlogged") {
                properties.insert("waterlogged".to_string(), above_is_water.to_string());
            }
            world
                .registries()
                .block_state_id_by_name_and_properties(block_type.resource_id, &properties)
        }
        VanillaGetStateForPlacementOverride::SmallDripleafBlock => {
            vanilla_block_default_state_id(if above_is_water {
                VANILLA_BLOCK_WATER_ID
            } else {
                VANILLA_BLOCK_AIR_ID
            })
        }
        VanillaGetStateForPlacementOverride::DoorBlock => {
            let lower_state = world.registries().block_state(final_lower_state_id)?;
            if lower_state.name != block_type.resource_id {
                return None;
            }
            let mut properties = lower_state.properties.clone();
            if !properties.contains_key("half") {
                return None;
            }
            properties.insert("half".to_string(), "upper".to_string());
            world
                .registries()
                .block_state_id_by_name_and_properties(block_type.resource_id, &properties)
        }
        _ => None,
    }
}

pub(super) fn trip_wire_hook_client_placement(
    world: &WorldStore,
    target_pos: BlockPos,
    final_state_id: i32,
) -> Option<TripWireHookClientPlacement> {
    let final_state = world.registries().block_state(final_state_id)?;
    if final_state.name != "minecraft:tripwire_hook" {
        return None;
    }
    let facing = direction_property(&final_state.properties, "facing")?;
    if direction_axis(facing) == DirectionAxis::Y {
        return None;
    }
    let was_attached = boolean_property(&final_state.properties, "attached")?;
    let was_powered = boolean_property(&final_state.properties, "powered")?;
    let mut attached = true;
    let mut powered = false;
    let mut receiver = None;
    let mut wires = Vec::new();

    for distance in 1..42 {
        let test_pos = relative_distance(target_pos, facing, distance)?;
        let state = world.probe_block(test_pos)?;
        if state.block_name.as_deref() == Some("minecraft:tripwire_hook") {
            if direction_property(&state.block_properties, "facing")? == opposite(facing) {
                receiver = Some((test_pos, state.block_state_id, distance));
            }
            break;
        }
        if state.block_name.as_deref() != Some("minecraft:tripwire") {
            attached = false;
            continue;
        }
        let disarmed = boolean_property(&state.block_properties, "disarmed")?;
        let wire_powered = boolean_property(&state.block_properties, "powered")?;
        powered |= !disarmed && wire_powered;
        wires.push((test_pos, state.block_state_id, distance));
    }

    attached &= receiver.is_some_and(|(_, _, distance)| distance > 1);
    powered &= attached;
    let target_state_id = trip_wire_hook_state(world, facing, attached, powered)?;
    let mut states = vec![(target_pos, target_state_id)];

    if let Some((receiver_pos, receiver_state_id, receiver_distance)) = receiver {
        let receiver_new_state_id =
            trip_wire_hook_state(world, opposite(facing), attached, powered)?;
        if receiver_new_state_id != receiver_state_id {
            states.push((receiver_pos, receiver_new_state_id));
        }
        if was_attached != attached {
            for (wire_pos, wire_state_id, distance) in wires {
                if distance >= receiver_distance {
                    continue;
                }
                let wire_state = world.registries().block_state(wire_state_id)?;
                let mut properties = wire_state.properties.clone();
                if !properties.contains_key("attached") {
                    return None;
                }
                properties.insert("attached".to_string(), attached.to_string());
                let new_state_id = world
                    .registries()
                    .block_state_id_by_name_and_properties(&wire_state.name, &properties)?;
                if new_state_id != wire_state_id {
                    states.push((wire_pos, new_state_id));
                }
            }
        }
    }

    let state_sound = powered && !was_powered
        || !powered && was_powered
        || attached && !was_attached
        || !attached && was_attached;
    let suppressed_state_sound_count = if state_sound {
        if receiver.is_some() {
            2
        } else {
            1
        }
    } else {
        0
    };
    let suppressed_detach_pitch =
        !attached && was_attached && !was_powered && suppressed_state_sound_count > 0;

    Some(TripWireHookClientPlacement {
        states,
        suppressed_state_sound_count,
        suppressed_detach_pitch,
    })
}

pub(super) fn door_placement(
    world: &WorldStore,
    target_pos: BlockPos,
    horizontal: Direction,
    context: LocalBlockUseContext,
) -> Option<DoorPlacement> {
    if direction_axis(horizontal) == DirectionAxis::Y {
        return None;
    }
    let above_pos = relative(target_pos, Direction::Up);
    placement_neighbor_signal_is_proven_absent(world, &[target_pos, above_pos])?;
    let powered = false;
    let left_direction = counterclockwise(horizontal)?;
    let right_direction = clockwise(horizontal)?;
    let left_pos = relative(target_pos, left_direction);
    let left_above_pos = relative(above_pos, left_direction);
    let right_pos = relative(target_pos, right_direction);
    let right_above_pos = relative(above_pos, right_direction);
    let left = world.probe_block(left_pos)?;
    let left_above = world.probe_block(left_above_pos)?;
    let right = world.probe_block(right_pos)?;
    let right_above = world.probe_block(right_above_pos)?;
    let solid_balance = i32::from(block_collision_shape_is_full_block(&right, right_pos))
        + i32::from(block_collision_shape_is_full_block(
            &right_above,
            right_above_pos,
        ))
        - i32::from(block_collision_shape_is_full_block(&left, left_pos))
        - i32::from(block_collision_shape_is_full_block(
            &left_above,
            left_above_pos,
        ));
    let door_left = is_lower_door(&left);
    let door_right = is_lower_door(&right);
    let hinge = if (!door_left || door_right) && solid_balance <= 0 {
        if (!door_right || door_left) && solid_balance >= 0 {
            let (step_x, step_z) = horizontal_steps(horizontal)?;
            let click_x = f64::from(context.pos.x) + context.cursor[0] - f64::from(target_pos.x);
            let click_z = f64::from(context.pos.z) + context.cursor[2] - f64::from(target_pos.z);
            if (step_x >= 0 || click_z >= 0.5)
                && (step_x <= 0 || click_z <= 0.5)
                && (step_z >= 0 || click_x <= 0.5)
                && (step_z <= 0 || click_x >= 0.5)
            {
                "left"
            } else {
                "right"
            }
        } else {
            "left"
        }
    } else {
        "right"
    };
    Some(DoorPlacement { hinge, powered })
}

fn is_lower_door(block: &BlockProbe) -> bool {
    block
        .block_name
        .as_deref()
        .and_then(bbb_protocol::block_types::vanilla_block_type_for_resource_id)
        .is_some_and(|block_type| {
            block_type.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::DoorBlock)
        })
        && block
            .block_properties
            .get("half")
            .is_some_and(|half| half == "lower")
}

fn horizontal_steps(direction: Direction) -> Option<(i32, i32)> {
    Some(match direction {
        Direction::North => (0, -1),
        Direction::South => (0, 1),
        Direction::West => (-1, 0),
        Direction::East => (1, 0),
        Direction::Down | Direction::Up => return None,
    })
}

fn trip_wire_hook_state(
    world: &WorldStore,
    facing: Direction,
    attached: bool,
    powered: bool,
) -> Option<i32> {
    let default_state_id = vanilla_block_default_state_id(VANILLA_BLOCK_TRIPWIRE_HOOK_ID)?;
    let default_state = world.registries().block_state(default_state_id)?;
    if default_state.name != "minecraft:tripwire_hook" {
        return None;
    }
    let mut properties = default_state.properties.clone();
    properties.insert("facing".to_string(), direction_name(facing).to_string());
    properties.insert("attached".to_string(), attached.to_string());
    properties.insert("powered".to_string(), powered.to_string());
    world
        .registries()
        .block_state_id_by_name_and_properties(&default_state.name, &properties)
}

fn direction_property(
    properties: &std::collections::BTreeMap<String, String>,
    property: &str,
) -> Option<Direction> {
    match properties.get(property)?.as_str() {
        "down" => Some(Direction::Down),
        "up" => Some(Direction::Up),
        "north" => Some(Direction::North),
        "south" => Some(Direction::South),
        "west" => Some(Direction::West),
        "east" => Some(Direction::East),
        _ => None,
    }
}

fn boolean_property(
    properties: &std::collections::BTreeMap<String, String>,
    property: &str,
) -> Option<bool> {
    match properties.get(property)?.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn relative_distance(pos: BlockPos, direction: Direction, distance: i32) -> Option<BlockPos> {
    Some(match direction {
        Direction::Down => BlockPos {
            y: pos.y.checked_sub(distance)?,
            ..pos
        },
        Direction::Up => BlockPos {
            y: pos.y.checked_add(distance)?,
            ..pos
        },
        Direction::North => BlockPos {
            z: pos.z.checked_sub(distance)?,
            ..pos
        },
        Direction::South => BlockPos {
            z: pos.z.checked_add(distance)?,
            ..pos
        },
        Direction::West => BlockPos {
            x: pos.x.checked_sub(distance)?,
            ..pos
        },
        Direction::East => BlockPos {
            x: pos.x.checked_add(distance)?,
            ..pos
        },
    })
}
