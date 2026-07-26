use bbb_protocol::{
    block_state_types::vanilla_block_default_state_id,
    block_types::{
        BlockTypeCatalogEntry, VanillaGetStateForPlacementOverride, VANILLA_BLOCK_AIR_ID,
        VANILLA_BLOCK_WATER_ID,
    },
    packets::Direction,
};

use super::{
    super::{LocalBlockUseContext, WorldStore},
    context::{clockwise, counterclockwise, direction_axis, relative, DirectionAxis},
    state::block_item_can_replace,
};
use crate::{client::block_collision_shape_is_full_block, BlockPos, BlockProbe, TerrainFluidKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DoorPlacement {
    pub(super) hinge: &'static str,
    pub(super) powered: bool,
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

pub(super) fn door_placement(
    world: &WorldStore,
    target_pos: BlockPos,
    horizontal: Direction,
    context: LocalBlockUseContext,
) -> Option<DoorPlacement> {
    if direction_axis(horizontal) == DirectionAxis::Y {
        return None;
    }
    if !door_has_no_possible_nearby_signal_source(world, target_pos)? {
        return None;
    }
    let powered = false;
    let above_pos = relative(target_pos, Direction::Up);
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

fn door_has_no_possible_nearby_signal_source(
    world: &WorldStore,
    target_pos: BlockPos,
) -> Option<bool> {
    // Vanilla 26.1 `SignalGetter.getSignal` can also relay a direct source
    // through a conductor. The radius-two audit below covers both door halves,
    // their six queried neighbours, and every direct-source neighbour. Any
    // source owner found in the 26.1 getSignal/getDirectSignal override audit
    // keeps this branch server-authoritative until shared signal evaluation
    // can resolve its direction and strength.
    let upper_pos = relative(target_pos, Direction::Up);
    for x in target_pos.x.saturating_sub(2)..=target_pos.x.saturating_add(2) {
        for y in target_pos.y.saturating_sub(2)..=upper_pos.y.saturating_add(2) {
            for z in target_pos.z.saturating_sub(2)..=target_pos.z.saturating_add(2) {
                let pos = BlockPos { x, y, z };
                let distance_to_lower =
                    x.abs_diff(target_pos.x) + y.abs_diff(target_pos.y) + z.abs_diff(target_pos.z);
                let distance_to_upper =
                    x.abs_diff(upper_pos.x) + y.abs_diff(upper_pos.y) + z.abs_diff(upper_pos.z);
                if distance_to_lower > 2 && distance_to_upper > 2 {
                    continue;
                }
                let block = world.probe_block(pos)?;
                if block
                    .block_name
                    .as_deref()
                    .is_some_and(possible_redstone_signal_source)
                {
                    return None;
                }
            }
        }
    }
    Some(true)
}

fn possible_redstone_signal_source(name: &str) -> bool {
    name.ends_with("_button")
        || name.ends_with("_pressure_plate")
        || matches!(
            name,
            "minecraft:redstone_wire"
                | "minecraft:lightning_rod"
                | "minecraft:lever"
                | "minecraft:redstone_torch"
                | "minecraft:redstone_wall_torch"
                | "minecraft:repeater"
                | "minecraft:comparator"
                | "minecraft:redstone_block"
                | "minecraft:jukebox"
                | "minecraft:daylight_detector"
                | "minecraft:sculk_sensor"
                | "minecraft:calibrated_sculk_sensor"
                | "minecraft:detector_rail"
                | "minecraft:trapped_chest"
                | "minecraft:tripwire_hook"
                | "minecraft:target"
                | "minecraft:lectern"
                | "minecraft:observer"
                | "minecraft:test_block"
        )
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
