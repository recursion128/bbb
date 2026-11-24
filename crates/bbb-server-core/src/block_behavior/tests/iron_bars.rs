use crate::fluid::FlowingFluid;
use bbb_protocol::block_types::{VanillaGetStateForPlacementOverride, VANILLA_BLOCK_TYPES};

use super::*;

const BARS_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn bars(
    resource_id: &str,
    north: bool,
    east: bool,
    south: bool,
    west: bool,
    waterlogged: bool,
) -> i32 {
    state_of(
        resource_id,
        &[
            ("east", if east { "true" } else { "false" }),
            ("north", if north { "true" } else { "false" }),
            ("south", if south { "true" } else { "false" }),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
            ("west", if west { "true" } else { "false" }),
        ],
    )
}

fn plain_bars(resource_id: &str) -> i32 {
    bars(resource_id, false, false, false, false, false)
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction, neighbour_state: i32) -> i32 {
    world.set(BARS_POS, state);
    world.set(relative(BARS_POS, direction), neighbour_state);
    world.shape_update(BARS_POS, direction);
    world.state(BARS_POS)
}

fn iron_bars_entries() -> Vec<&'static bbb_protocol::block_types::BlockTypeCatalogEntry> {
    VANILLA_BLOCK_TYPES
        .iter()
        .filter(|entry| {
            entry.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::IronBarsBlock)
        })
        .collect()
}

#[test]
fn every_bars_subclass_and_walls_connect() {
    let current = plain_bars("minecraft:iron_bars");
    let connected = bars("minecraft:iron_bars", false, true, false, false, false);
    for neighbour in [
        plain_bars("minecraft:glass_pane"),
        plain_bars("minecraft:white_stained_glass_pane"),
        plain_bars("minecraft:copper_bars"),
        default_state("minecraft:cobblestone_wall"),
    ] {
        assert_eq!(
            update(&mut NativeWorld::new(), current, Direction::East, neighbour),
            connected
        );
    }
}

#[test]
fn sturdy_faces_connect_except_for_vanillas_exception_classes() {
    let current = plain_bars("minecraft:iron_bars");
    let connected = bars("minecraft:iron_bars", false, true, false, false, false);
    assert_eq!(
        update(&mut NativeWorld::new(), current, Direction::East, stone()),
        connected
    );
    for resource_id in [
        "minecraft:oak_leaves",
        "minecraft:barrier",
        "minecraft:melon",
    ] {
        assert_eq!(
            update(
                &mut NativeWorld::new(),
                connected,
                Direction::East,
                default_state(resource_id),
            ),
            current,
            "{resource_id}"
        );
    }

    assert_eq!(
        update(
            &mut NativeWorld::new(),
            connected,
            Direction::East,
            default_state("minecraft:shulker_box"),
        ),
        current
    );
}

#[test]
fn every_horizontal_direction_updates_only_its_property() {
    for (direction, expected) in [
        (
            Direction::North,
            bars("minecraft:iron_bars", true, false, false, false, false),
        ),
        (
            Direction::East,
            bars("minecraft:iron_bars", false, true, false, false, false),
        ),
        (
            Direction::South,
            bars("minecraft:iron_bars", false, false, true, false, false),
        ),
        (
            Direction::West,
            bars("minecraft:iron_bars", false, false, false, true, false),
        ),
    ] {
        assert_eq!(
            update(
                &mut NativeWorld::new(),
                plain_bars("minecraft:iron_bars"),
                direction,
                stone(),
            ),
            expected
        );
    }
}

#[test]
fn waterlogged_bars_rearm_water_and_do_not_join_dripstone() {
    for entry in iron_bars_entries().into_iter().take(1) {
        let waterlogged = bars(entry.resource_id, false, false, false, false, true);
        for direction in [Direction::Up, Direction::East] {
            let mut world = NativeWorld::new();
            update(&mut world, waterlogged, direction, stone());
            assert!(world
                .saved_fluid_ticks()
                .iter()
                .any(|tick| tick.id == "minecraft:water"
                    && (tick.x, tick.y, tick.z) == (BARS_POS.x, BARS_POS.y, BARS_POS.z)
                    && tick.delay == FlowingFluid::WATER_TICK_DELAY));
        }

        let mut dry = NativeWorld::new();
        update(
            &mut dry,
            plain_bars(entry.resource_id),
            Direction::Down,
            stone(),
        );
        assert!(dry.saved_fluid_ticks().is_empty(), "{}", entry.resource_id);
    }

    assert_eq!(
        update(
            &mut NativeWorld::new(),
            plain_bars("minecraft:iron_bars"),
            Direction::East,
            default_state("minecraft:pointed_dripstone"),
        ),
        plain_bars("minecraft:iron_bars")
    );
}
