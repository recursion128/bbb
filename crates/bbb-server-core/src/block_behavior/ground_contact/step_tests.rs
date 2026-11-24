use super::tests::{body, random_where};
use super::*;
use crate::fluid::with_property;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;

fn state(block: i32) -> i32 {
    vanilla_block_default_state_id(block).unwrap()
}

fn step(state: i32, body: GroundBody) -> StepOn {
    step_on(
        state,
        body,
        Vec3d {
            x: 2.0,
            y: -0.05,
            z: -4.0,
        },
        &GameRandom::with_seed(0),
    )
    .unwrap()
}

#[test]
fn step_on_magma_hurts_living_bodies_that_are_not_careful() {
    for (careful, living, hurts) in [
        (false, true, true),
        (true, true, false),
        (false, false, false),
    ] {
        assert_eq!(
            step(
                state(VANILLA_BLOCK_MAGMA_BLOCK_ID),
                GroundBody {
                    careful,
                    living,
                    ..body()
                }
            ),
            if hurts {
                StepOn::Hurt("minecraft:hot_floor", 1.0)
            } else {
                StepOn::None
            }
        );
    }
}

#[test]
fn step_on_slime_slows_horizontal_motion_and_preserves_vertical() {
    for y in [-0.1, 0.0, 0.05] {
        for careful in [false, true] {
            let initial = Vec3d { x: 2.0, y, z: -4.0 };
            let expected = if y.abs() < 0.1 && !careful {
                StepOn::Movement(if y == 0.0 {
                    Vec3d { x: 0.8, y, z: -1.6 }
                } else {
                    Vec3d {
                        x: 0.820_000_000_000_000_1,
                        y,
                        z: -1.640_000_000_000_000_1,
                    }
                })
            } else {
                StepOn::None
            };
            assert_eq!(
                step_on(
                    state(VANILLA_BLOCK_SLIME_BLOCK_ID),
                    GroundBody {
                        living: false,
                        careful,
                        ..body()
                    },
                    initial,
                    &GameRandom::with_seed(0),
                ),
                Some(expected)
            );
        }
    }
}

#[test]
fn step_on_turtle_egg_breaks_for_eligible_bodies_on_one_in_hundred_roll() {
    let egg = state(VANILLA_BLOCK_TURTLE_EGG_ID);
    for (careful, turtle_or_bat, living, player, griefing, hit, breaks) in [
        (true, false, true, false, Some(true), true, false),
        (false, true, true, false, Some(true), true, false),
        (false, false, false, false, Some(true), true, false),
        (false, false, true, false, Some(false), true, false),
        (false, false, true, true, Some(false), true, true),
        (false, false, true, false, Some(true), true, true),
        (false, false, true, false, Some(true), false, false),
    ] {
        let body = GroundBody {
            careful,
            turtle_or_bat,
            living,
            player,
            mob_griefing: griefing,
            ..body()
        };
        let random = random_where(|random| (random.next_int(100) == 0) == hit);
        assert_eq!(
            step_on(egg, body, Vec3d::default(), &random) == Some(StepOn::DecreaseTurtleEggs),
            breaks
        );
    }
}

#[test]
fn step_on_redstone_ore_interacts_unless_careful_even_when_lit() {
    for lit in ["false", "true"] {
        let ore = with_property(state(VANILLA_BLOCK_REDSTONE_ORE_ID), "lit", lit).unwrap();
        for careful in [false, true] {
            assert_eq!(
                step(ore, GroundBody { careful, ..body() }),
                if careful {
                    StepOn::None
                } else {
                    StepOn::InteractRedstoneOre
                }
            );
        }
    }
}

#[test]
fn step_on_sculk_sensor_and_shrieker_trigger_even_when_careful() {
    let careful = GroundBody {
        careful: true,
        ..body()
    };
    assert_eq!(
        step(state(VANILLA_BLOCK_SCULK_SENSOR_ID), careful),
        StepOn::SculkSensor
    );
    assert_eq!(
        step(state(VANILLA_BLOCK_SCULK_SHRIEKER_ID), careful),
        StepOn::SculkShrieker
    );
}
