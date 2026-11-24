use super::native_world::NativeWorld;
use super::*;

fn farmland_world(moisture: i32) -> NativeWorld {
    let mut world = NativeWorld::ticking(at(0, 70, 0));
    world.set(at(0, 70, 0), farmland_state(moisture));
    world
}

#[test]
fn farmland_within_the_water_scan_jumps_straight_to_full_moisture() {
    let mut world = farmland_world(0);
    world.set(at(4, 70, 0), water(0));

    world.random_tick(at(0, 70, 0), 0);

    assert_eq!(
        world.state(at(0, 70, 0)),
        farmland_state(farmland::MAX_MOISTURE)
    );
}

#[test]
fn the_water_scan_reaches_four_blocks_out_and_one_up_but_never_down() {
    let mut world = farmland_world(0);
    world.set(at(5, 70, 0), water(0));
    assert!(!world.with_level(|level| farmland::is_near_water(level, at(0, 70, 0))));

    let mut world = farmland_world(0);
    world.set(at(0, 69, 0), water(0));
    assert!(!world.with_level(|level| farmland::is_near_water(level, at(0, 70, 0))));

    let mut world = farmland_world(0);
    world.set(at(-4, 71, 4), water(0));
    assert!(world.with_level(|level| farmland::is_near_water(level, at(0, 70, 0))));
}

#[test]
fn flowing_water_counts_as_much_as_a_source() {
    let mut world = farmland_world(0);
    world.set(at(1, 70, 1), water(3));
    assert!(world.with_level(|level| farmland::is_near_water(level, at(0, 70, 0))));
}

#[test]
fn rain_on_the_block_above_wets_farmland_that_has_no_water_near_it() {
    let mut world = farmland_world(2);
    world.command("weather rain 1000t");
    for _ in 0..30 {
        world.run_tick();
    }
    world.set(at(0, 70, 0), farmland_state(2));

    world.random_tick(at(0, 70, 0), 0);

    assert_eq!(
        world.state(at(0, 70, 0)),
        farmland_state(farmland::MAX_MOISTURE)
    );
}

#[test]
fn dry_farmland_loses_exactly_one_moisture_step_per_random_tick() {
    let mut world = farmland_world(farmland::MAX_MOISTURE);
    for expected in (0..farmland::MAX_MOISTURE).rev() {
        world.random_tick(at(0, 70, 0), 0);
        assert_eq!(world.state(at(0, 70, 0)), farmland_state(expected));
    }
}

#[test]
fn farmland_at_zero_moisture_with_nothing_growing_on_it_turns_to_dirt() {
    let mut world = farmland_world(0);

    world.random_tick(at(0, 70, 0), 0);

    assert_eq!(world.state(at(0, 70, 0)), dirt());
}

#[test]
fn farmland_at_zero_moisture_survives_while_a_crop_stands_on_it() {
    let mut world = farmland_world(0);
    world.set(at(0, 71, 0), wheat(2));

    world.random_tick(at(0, 70, 0), 0);

    assert_eq!(world.state(at(0, 70, 0)), farmland_state(0));
}

#[test]
fn a_solid_block_landing_on_farmland_reverts_it_a_tick_later() {
    let mut world = farmland_world(farmland::MAX_MOISTURE);
    world.set(at(0, 71, 0), stone());

    world.shape_update(at(0, 70, 0), Direction::Up);

    assert_eq!(
        world.state(at(0, 70, 0)),
        farmland_state(farmland::MAX_MOISTURE)
    );

    world.run_tick();
    assert_eq!(world.state(at(0, 70, 0)), dirt());
}

#[test]
fn a_non_solid_block_on_top_leaves_farmland_alone() {
    let mut world = farmland_world(farmland::MAX_MOISTURE);
    world.set(at(0, 71, 0), wheat(0));

    world.shape_update(at(0, 70, 0), Direction::Up);
    world.run_tick();

    assert_eq!(
        world.state(at(0, 70, 0)),
        farmland_state(farmland::MAX_MOISTURE)
    );
    assert_eq!(
        world.with_level(|level| farmland::can_survive(
            level,
            at(0, 70, 0),
            &|registry, tag, element| level.registry_tag_membership(registry, tag, element),
        )),
        Some(true)
    );
}

#[test]
fn a_scheduled_tick_on_farmland_that_can_survive_does_nothing() {
    let mut world = farmland_world(3);
    world.set(at(0, 71, 0), stone());
    world.shape_update(at(0, 70, 0), Direction::Up);
    world.set(at(0, 71, 0), air());

    world.run_tick();

    assert_eq!(world.state(at(0, 70, 0)), farmland_state(3));
}
