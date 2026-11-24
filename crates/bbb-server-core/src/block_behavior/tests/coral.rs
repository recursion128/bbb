//! The coral family: the cell each kind holds on to, and the countdown a
//! living one starts when `scanForWater` comes back empty.

use bbb_server_types::directions::opposite;

use super::*;

const CORAL: BlockPos = BlockPos { x: 2, y: 70, z: 3 };

#[derive(Clone, Copy, Debug)]
enum CoralShape {
    Solid,
    Plant,
    Fan,
    WallFan,
}

const CORAL_DEATHS: [(&str, &str, CoralShape); 4] = [
    (
        "minecraft:tube_coral_block",
        "minecraft:dead_tube_coral_block",
        CoralShape::Solid,
    ),
    (
        "minecraft:tube_coral",
        "minecraft:dead_tube_coral",
        CoralShape::Plant,
    ),
    (
        "minecraft:tube_coral_fan",
        "minecraft:dead_tube_coral_fan",
        CoralShape::Fan,
    ),
    (
        "minecraft:tube_coral_wall_fan",
        "minecraft:dead_tube_coral_wall_fan",
        CoralShape::WallFan,
    ),
];

fn coral_state(name: &str, shape: CoralShape, waterlogged: bool) -> i32 {
    match shape {
        CoralShape::Solid => default_state(name),
        CoralShape::Plant | CoralShape::Fan => coral_fan(name, waterlogged),
        CoralShape::WallFan => wall_fan(name, "west", waterlogged),
    }
}

fn coral_fan(name: &str, waterlogged: bool) -> i32 {
    state_of(
        name,
        &[("waterlogged", if waterlogged { "true" } else { "false" })],
    )
}

fn wall_fan(name: &str, facing: &str, waterlogged: bool) -> i32 {
    state_of(
        name,
        &[
            ("facing", facing),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn survives_over(state: i32, support_pos: BlockPos, support: i32) -> Option<bool> {
    can_survive(state, CORAL, vanilla_tags(), &mut |at| {
        if at == support_pos {
            support
        } else {
            air()
        }
    })
}

#[test]
fn a_coral_plant_or_fan_wants_a_full_face_under_it() {
    for name in ["minecraft:dead_tube_coral", "minecraft:tube_coral_fan"] {
        let state = coral_fan(name, true);
        assert_eq!(
            survives_over(state, below(CORAL), stone()),
            Some(true),
            "{name}"
        );
        assert_eq!(
            survives_over(state, below(CORAL), default_state("minecraft:oak_fence")),
            Some(false),
            "{name}"
        );
        assert_eq!(
            survives_over(state, below(CORAL), air()),
            Some(false),
            "{name}"
        );
    }
}

#[test]
fn a_coral_wall_fan_reads_the_cell_behind_its_facing() {
    for name in [
        "minecraft:dead_tube_coral_wall_fan",
        "minecraft:tube_coral_wall_fan",
    ] {
        for (facing, wall) in [("north", Direction::South), ("west", Direction::East)] {
            let state = wall_fan(name, facing, true);
            assert_eq!(
                survives_over(state, relative(CORAL, wall), stone()),
                Some(true),
                "{name} {facing}"
            );
            assert_eq!(
                survives_over(state, relative(CORAL, opposite(wall)), stone()),
                Some(false),
                "{name} {facing}"
            );
        }
    }
}

#[test]
fn an_unsupported_waterlogged_dead_coral_breaks_and_leaves_its_water() {
    let mut world = NativeWorld::new();
    world.set(CORAL, coral_fan("minecraft:dead_tube_coral", true));

    world.shape_update(CORAL, Direction::Down);

    assert_eq!(world.state(CORAL), water(0));
}

#[test]
fn a_dead_coral_stays_when_the_update_comes_from_anywhere_else() {
    let mut world = NativeWorld::new();
    world.set(CORAL, coral_fan("minecraft:dead_tube_coral_fan", false));

    world.shape_update(CORAL, Direction::North);

    assert_eq!(
        world.state(CORAL),
        coral_fan("minecraft:dead_tube_coral_fan", false)
    );
}

#[test]
fn every_living_coral_dies_to_its_exact_default_and_properties() {
    for (living_name, dead_name, shape) in CORAL_DEATHS {
        let living = coral_state(living_name, shape, false);
        let expected_dead = coral_state(dead_name, shape, false);
        let mut world = NativeWorld::new();
        world.set(CORAL, living);

        world.with_level(|level| tick_block(level, CORAL, living));

        assert_eq!(world.state(CORAL), expected_dead, "{living_name}");
    }
}

#[test]
fn adjacent_or_waterlogged_water_makes_every_death_tick_a_noop() {
    for (living_name, _, shape) in CORAL_DEATHS {
        let living = coral_state(living_name, shape, false);
        for direction in DIRECTION_VALUES {
            let mut world = NativeWorld::new();
            world.set(CORAL, living);
            world.set(relative(CORAL, direction), water(0));
            world.with_level(|level| tick_block(level, CORAL, living));
            assert_eq!(
                world.state(CORAL),
                living,
                "adjacent water: {living_name} {direction:?}"
            );
        }

        if !matches!(shape, CoralShape::Solid) {
            let waterlogged = coral_state(living_name, shape, true);
            let mut world = NativeWorld::new();
            world.set(CORAL, waterlogged);

            world.with_level(|level| tick_block(level, CORAL, waterlogged));

            assert_eq!(
                world.state(CORAL),
                waterlogged,
                "waterlogged: {living_name}"
            );
        }
    }
}

#[test]
fn dry_coral_dies_on_world_ticks_while_water_keeps_it_alive() {
    for (living_name, dead_name, shape) in CORAL_DEATHS {
        for wet in [false, true] {
            let mut world = NativeWorld::ticking(CORAL);
            world.set(below(CORAL), stone());
            world.set(relative(CORAL, Direction::East), stone());
            if wet {
                world.set(above(CORAL), water(0));
            }
            world.set(CORAL, coral_state(living_name, shape, false));
            world.shape_update(CORAL, Direction::North);
            for _ in 0..100 {
                world.run_tick();
            }
            assert_eq!(
                block_type_of(world.state(CORAL)),
                block_type_of(default_state(if wet { living_name } else { dead_name })),
                "{living_name} wet={wet}"
            );
        }
    }
}
