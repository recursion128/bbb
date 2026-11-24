use super::native_world::NativeWorld;
use super::*;

fn dirt_plate(block_state_id: i32) -> NativeWorld {
    let mut world = NativeWorld::new();
    for x in -2..=2 {
        for z in -2..=2 {
            world.set(at(x, 70, z), dirt());
        }
    }
    world.set(at(0, 70, 0), block_state_id);
    world
}

#[test]
fn grass_turns_to_dirt_under_a_block_that_blocks_all_light() {
    let mut world = dirt_plate(grass_block(false));
    world.set(at(0, 71, 0), stone());

    world.random_tick(at(0, 70, 0), 11);

    assert_eq!(world.state(at(0, 70, 0)), dirt());
}

#[test]
fn grass_survives_under_a_block_that_only_dampens_light() {
    let mut world = dirt_plate(grass_block(false));
    world.set(at(0, 71, 0), oak_leaves(1, true, false));

    world.random_tick(at(0, 70, 0), 11);

    assert_eq!(world.state(at(0, 70, 0)), grass_block(false));
}

#[test]
fn grass_turns_to_dirt_under_a_full_fluid_whatever_its_opacity() {
    let mut world = dirt_plate(grass_block(false));
    world.set(at(0, 71, 0), water(0));

    world.random_tick(at(0, 70, 0), 11);

    assert_eq!(world.state(at(0, 70, 0)), dirt());
}

#[test]
fn grass_survives_under_exactly_one_snow_layer() {
    let mut world = dirt_plate(grass_block(false));
    world.set(at(0, 71, 0), snow_layer(1));
    world.random_tick(at(0, 70, 0), 11);
    assert_eq!(world.state(at(0, 70, 0)), grass_block(false));

    let mut world = dirt_plate(grass_block(false));
    world.set(at(0, 71, 0), snow_layer(8));
    world.random_tick(at(0, 70, 0), 11);
    assert_eq!(world.state(at(0, 70, 0)), dirt());
}

#[test]
fn grass_spreads_to_survivable_dirt_nearby() {
    let mut world = dirt_plate(grass_block(false));
    let candidates: Vec<_> = (-2..=2)
        .flat_map(|x| (-2..=2).map(move |z| at(x, 70, z)))
        .filter(|pos| world.state(*pos) == dirt())
        .collect();
    for seed in 0..32 {
        world.random_tick(at(0, 70, 0), seed);
    }
    assert!(candidates
        .iter()
        .any(|pos| world.state(*pos) == grass_block(false)));
    assert!(candidates.iter().all(
        |pos| matches!(world.state(*pos), state if state == dirt() || state == grass_block(false))
    ));
}

#[test]
fn grass_does_not_spread_when_the_block_above_it_is_below_light_nine() {
    let mut world = dirt_plate(grass_block(false));
    world.illuminate(at(0, 71, 0), 8);
    let candidates: Vec<_> = (-2..=2)
        .flat_map(|x| (-2..=2).map(move |z| at(x, 70, z)))
        .filter(|pos| world.state(*pos) == dirt())
        .collect();
    for seed in 0..32 {
        world.random_tick(at(0, 70, 0), seed);
    }
    assert_eq!(world.state(at(0, 70, 0)), grass_block(false));
    assert!(candidates.iter().all(|pos| world.state(*pos) == dirt()));
}

#[test]
fn grass_spreads_at_exactly_light_nine() {
    let mut world = dirt_plate(grass_block(false));
    world.illuminate(at(0, 71, 0), 9);
    let candidates: Vec<_> = (-2..=2)
        .flat_map(|x| (-2..=2).map(move |z| at(x, 70, z)))
        .filter(|pos| world.state(*pos) == dirt())
        .collect();
    for seed in 0..32 {
        world.random_tick(at(0, 70, 0), seed);
    }
    assert!(candidates
        .iter()
        .any(|pos| world.state(*pos) == grass_block(false)));
}

#[test]
fn grass_does_not_spread_onto_dirt_with_water_directly_above_it() {
    let mut world = dirt_plate(grass_block(false));
    let candidates: Vec<_> = (-2..=2)
        .flat_map(|x| (-2..=2).map(move |z| at(x, 70, z)))
        .filter(|pos| world.state(*pos) == dirt())
        .collect();
    for pos in &candidates {
        world.set(above(*pos), water(1));
    }
    for seed in 0..32 {
        world.random_tick(at(0, 70, 0), seed);
    }
    assert_eq!(world.state(at(0, 70, 0)), grass_block(false));
    assert!(candidates.iter().all(|pos| world.state(*pos) == dirt()));
}

#[test]
fn a_spread_block_takes_its_snowy_flag_from_the_block_above_the_target() {
    let mut world = dirt_plate(grass_block(false));
    let candidates: Vec<_> = (-2..=2)
        .flat_map(|x| (-2..=2).map(move |z| at(x, 70, z)))
        .filter(|pos| world.state(*pos) == dirt())
        .collect();
    for pos in &candidates {
        world.set(above(*pos), snow_layer(1));
    }
    for seed in 0..32 {
        world.random_tick(at(0, 70, 0), seed);
    }
    assert!(candidates
        .iter()
        .any(|pos| world.state(*pos) == grass_block(true)));
    assert!(candidates.iter().all(
        |pos| matches!(world.state(*pos), state if state == dirt() || state == grass_block(true))
    ));
}

#[test]
fn mycelium_runs_the_same_body_over_the_same_dirt_base() {
    let mut world = dirt_plate(mycelium(false));
    let candidates: Vec<_> = (-2..=2)
        .flat_map(|x| (-2..=2).map(move |z| at(x, 70, z)))
        .filter(|pos| world.state(*pos) == dirt())
        .collect();
    for seed in 0..32 {
        world.random_tick(at(0, 70, 0), seed);
    }
    assert!(candidates
        .iter()
        .any(|pos| world.state(*pos) == mycelium(false)));
}

#[test]
fn the_snowy_shape_update_only_listens_to_the_block_directly_above() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), grass_block(false));
    world.set(at(0, 71, 0), snow_layer(1));
    world.set(at(1, 70, 0), snow_layer(1));

    world.shape_update(at(0, 70, 0), Direction::East);
    assert_eq!(world.state(at(0, 70, 0)), grass_block(false));

    world.shape_update(at(0, 70, 0), Direction::Up);
    assert_eq!(world.state(at(0, 70, 0)), grass_block(true));
}

#[test]
fn the_snowy_shape_update_clears_the_flag_when_the_snow_goes_away() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), grass_block(true));

    world.shape_update(at(0, 70, 0), Direction::Up);

    assert_eq!(world.state(at(0, 70, 0)), grass_block(false));
}
