use crate::fluid::FlowingFluid;
use bbb_protocol::block_types::{VanillaGetStateForPlacementOverride, VANILLA_BLOCK_TYPES};
use bbb_server_types::directions::relative;

use super::*;

const STAIR_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn stair(resource_id: &str, facing: &str, half: &str, shape: &str, waterlogged: bool) -> i32 {
    state_of(
        resource_id,
        &[
            ("facing", facing),
            ("half", half),
            ("shape", shape),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn stair_entries() -> Vec<&'static bbb_protocol::block_types::BlockTypeCatalogEntry> {
    VANILLA_BLOCK_TYPES
        .iter()
        .filter(|entry| {
            entry.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::StairBlock)
        })
        .collect()
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction, neighbour_state: i32) -> i32 {
    world.set(STAIR_POS, state);
    world.set(relative(STAIR_POS, direction), neighbour_state);
    world.shape_update(STAIR_POS, direction);
    world.state(STAIR_POS)
}

#[test]
fn perpendicular_stairs_form_all_four_outer_and_inner_turns() {
    let own = stair("minecraft:oak_stairs", "north", "bottom", "straight", false);
    for (direction, neighbour_facing, expected_shape) in [
        (Direction::North, "west", "outer_left"),
        (Direction::North, "east", "outer_right"),
        (Direction::South, "west", "inner_left"),
        (Direction::South, "east", "inner_right"),
    ] {
        let mut world = NativeWorld::new();
        let neighbour = stair(
            "minecraft:quartz_stairs",
            neighbour_facing,
            "bottom",
            "straight",
            false,
        );
        assert_eq!(
            update(&mut world, own, direction, neighbour),
            stair(
                "minecraft:oak_stairs",
                "north",
                "bottom",
                expected_shape,
                false,
            )
        );
    }
}

#[test]
fn outer_shape_wins_before_an_inner_candidate_is_considered() {
    let own = stair("minecraft:oak_stairs", "north", "bottom", "straight", false);
    let mut world = NativeWorld::new();
    world.set(
        relative(STAIR_POS, Direction::South),
        stair(
            "minecraft:quartz_stairs",
            "east",
            "bottom",
            "straight",
            false,
        ),
    );
    assert_eq!(
        update(
            &mut world,
            own,
            Direction::North,
            stair(
                "minecraft:quartz_stairs",
                "west",
                "bottom",
                "straight",
                false,
            ),
        ),
        stair(
            "minecraft:oak_stairs",
            "north",
            "bottom",
            "outer_left",
            false,
        )
    );
}

#[test]
fn matching_side_stairs_block_outer_and_inner_shapes() {
    let own = stair("minecraft:oak_stairs", "north", "bottom", "straight", false);
    for (candidate_direction, candidate_facing, blocker_direction) in [
        (Direction::North, "west", Direction::East),
        (Direction::South, "west", Direction::West),
    ] {
        let mut world = NativeWorld::new();
        world.set(relative(STAIR_POS, blocker_direction), own);
        let candidate = stair(
            "minecraft:quartz_stairs",
            candidate_facing,
            "bottom",
            "straight",
            false,
        );
        assert_eq!(update(&mut world, own, candidate_direction, candidate), own);
    }

    let mut different_facing = NativeWorld::new();
    different_facing.set(
        relative(STAIR_POS, Direction::East),
        stair("minecraft:oak_stairs", "east", "bottom", "straight", false),
    );
    assert_eq!(
        update(
            &mut different_facing,
            own,
            Direction::North,
            stair(
                "minecraft:quartz_stairs",
                "west",
                "bottom",
                "straight",
                false,
            ),
        ),
        stair(
            "minecraft:oak_stairs",
            "north",
            "bottom",
            "outer_left",
            false,
        )
    );
}

#[test]
fn same_axis_or_opposite_half_candidates_leave_stairs_straight() {
    let own = stair("minecraft:oak_stairs", "north", "bottom", "straight", false);
    for neighbour in [
        stair(
            "minecraft:quartz_stairs",
            "south",
            "bottom",
            "straight",
            false,
        ),
        stair("minecraft:quartz_stairs", "west", "top", "straight", false),
    ] {
        assert_eq!(
            update(&mut NativeWorld::new(), own, Direction::North, neighbour),
            own
        );
    }
}

#[test]
fn waterlogged_stair_rearms_water_and_vertical_updates_keep_shape() {
    for entry in stair_entries().into_iter().take(1) {
        let state = stair(entry.resource_id, "north", "bottom", "straight", true);
        for direction in [Direction::Up, Direction::North] {
            let mut world = NativeWorld::new();
            assert_eq!(update(&mut world, state, direction, stone()), state);
            assert!(world
                .saved_fluid_ticks()
                .iter()
                .any(|tick| tick.id == "minecraft:water"
                    && (tick.x, tick.y, tick.z) == (STAIR_POS.x, STAIR_POS.y, STAIR_POS.z)
                    && tick.delay == FlowingFluid::WATER_TICK_DELAY));
        }
    }

    let state = stair(
        "minecraft:oak_stairs",
        "north",
        "bottom",
        "outer_right",
        false,
    );
    let mut world = NativeWorld::new();
    world.set(
        relative(STAIR_POS, Direction::North),
        stair(
            "minecraft:quartz_stairs",
            "west",
            "bottom",
            "straight",
            false,
        ),
    );
    assert_eq!(update(&mut world, state, Direction::Up, stone()), state);
    assert!(world.saved_fluid_ticks().is_empty());
}
