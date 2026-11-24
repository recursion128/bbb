//! `ChorusFlowerBlock` / `ChorusPlantBlock` production paths.

use crate::block_behavior::chorus::{LEVEL_EVENT_CHORUS_DEATH, LEVEL_EVENT_CHORUS_GROW};

use super::*;

fn end_stone() -> i32 {
    default_state("minecraft:end_stone")
}

fn chorus_flower(age: i32) -> i32 {
    state_of("minecraft:chorus_flower", &[("age", &age.to_string())])
}

fn chorus_plant() -> i32 {
    default_state("minecraft:chorus_plant")
}

#[test]
fn chorus_flower_survives_on_end_stone_and_dies_in_mid_air() {
    let pos = at(0, 70, 0);
    assert_eq!(
        can_survive(chorus_flower(0), pos, vanilla_tags(), &mut |at| {
            if at == super::at(0, 69, 0) {
                end_stone()
            } else {
                air()
            }
        }),
        Some(true)
    );
    assert_eq!(
        can_survive(chorus_flower(0), pos, vanilla_tags(), &mut |_| air()),
        Some(false)
    );
}

#[test]
fn chorus_flower_tick_destroys_an_unsupported_flower() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), chorus_flower(0));
    world.with_level(|level| tick_block(level, at(0, 70, 0), chorus_flower(0)));
    assert_eq!(world.state(at(0, 70, 0)), air());
}

#[test]
fn chorus_flower_breaks_a_tick_after_the_floor_is_gone() {
    let mut world = NativeWorld::ticking(at(0, 70, 0));
    world.set(at(0, 70, 0), chorus_flower(0));
    world.shape_update(at(0, 70, 0), Direction::Down);
    world.run_tick();
    assert_eq!(world.state(at(0, 70, 0)), air());
}

#[test]
fn chorus_flower_grows_upward_from_end_stone() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), end_stone());
    world.set(at(0, 70, 0), chorus_flower(0));
    let seed = 1;
    world.random_tick(at(0, 70, 0), seed);
    assert_eq!(int_property(world.state(at(0, 71, 0)), "age"), Some(0));
    assert!(world
        .level_events()
        .iter()
        .any(|event| event.event_type == LEVEL_EVENT_CHORUS_GROW
            && event.pos == at(0, 71, 0)
            && event.data == 0));
    assert_eq!(
        block_type_of(world.state(at(0, 70, 0))),
        block_type_of(chorus_plant())
    );
}

#[test]
fn chorus_flower_on_a_one_block_pillar_grows_upward() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), end_stone());
    world.set(at(0, 70, 0), chorus_plant());
    world.set(at(0, 71, 0), chorus_flower(0));
    let seed = 1;
    world.random_tick(at(0, 71, 0), seed);
    assert_eq!(
        block_type_of(world.state(at(0, 71, 0))),
        block_type_of(chorus_plant())
    );
    assert_eq!(world.state(at(0, 72, 0)), chorus_flower(0));
}

#[test]
fn chorus_flower_age_four_dies_when_it_cannot_grow_up() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), end_stone());
    world.set(at(0, 70, 0), chorus_flower(4));
    world.set(at(0, 72, 0), stone());
    let seed = 1;
    world.random_tick(at(0, 70, 0), seed);
    assert_eq!(int_property(world.state(at(0, 70, 0)), "age"), Some(5));
    assert!(world
        .level_events()
        .iter()
        .any(|event| event.event_type == LEVEL_EVENT_CHORUS_DEATH));
}

#[test]
fn chorus_flower_random_tick_is_a_no_op_when_above_is_blocked() {
    let mut world = NativeWorld::new();
    world.set(at(0, 69, 0), end_stone());
    world.set(at(0, 70, 0), chorus_flower(0));
    world.set(at(0, 71, 0), stone());
    let seed = 1;
    world.random_tick(at(0, 70, 0), seed);
    assert_eq!(world.state(at(0, 70, 0)), chorus_flower(0));
    assert!(world.level_events().is_empty());
}

#[test]
fn chorus_flower_branch_draw_can_place_a_side_flower() {
    let mut found = false;
    for seed in 0..64 {
        let mut trial = NativeWorld::new();
        trial.set(at(0, 69, 0), end_stone());
        trial.set(at(0, 70, 0), chorus_plant());
        trial.set(at(0, 71, 0), chorus_plant());
        trial.set(at(0, 72, 0), chorus_plant());
        trial.set(at(0, 73, 0), chorus_flower(1));
        trial.random_tick(at(0, 73, 0), seed);
        let side = [at(1, 73, 0), at(-1, 73, 0), at(0, 73, 1), at(0, 73, -1)]
            .into_iter()
            .any(|pos| block_type_of(trial.state(pos)) == block_type_of(chorus_flower(0)));
        if side {
            found = true;
            assert!(trial
                .level_events()
                .iter()
                .any(|event| event.event_type == LEVEL_EVENT_CHORUS_GROW));
            break;
        }
    }
    assert!(found, "no seed in 0..64 produced a chorus branch");
}

#[test]
fn chorus_plant_survives_on_end_stone() {
    assert_eq!(
        can_survive(chorus_plant(), at(0, 70, 0), vanilla_tags(), &mut |at| {
            if at == super::at(0, 69, 0) {
                end_stone()
            } else {
                air()
            }
        }),
        Some(true)
    );
}
