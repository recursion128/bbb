use super::native_world::NativeWorld;
use super::*;

#[test]
fn a_snow_layer_melts_only_above_block_light_eleven() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), snow_layer(3));
    world.illuminate(at(0, 70, 0), 11);
    world.random_tick(at(0, 70, 0), 0);
    assert_eq!(world.state(at(0, 70, 0)), snow_layer(3));

    world.illuminate(at(0, 70, 0), 12);
    world.random_tick(at(0, 70, 0), 0);
    assert_eq!(world.state(at(0, 70, 0)), air());
}

#[test]
fn a_snow_layer_ignores_sky_light() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), snow_layer(1));
    assert_eq!(
        world.with_level(|level| level.brightness(LightLayer::Sky, at(0, 70, 0))),
        15
    );

    world.random_tick(at(0, 70, 0), 0);

    assert_eq!(world.state(at(0, 70, 0)), snow_layer(1));
}

#[test]
fn ice_melts_at_a_threshold_lowered_by_its_own_light_dampening() {
    let mut world = NativeWorld::new();
    world.set(at(0, 70, 0), ice());
    world.illuminate(at(0, 70, 0), 10);
    world.random_tick(at(0, 70, 0), 0);
    assert_eq!(world.state(at(0, 70, 0)), ice());

    world.illuminate(at(0, 70, 0), 11);
    world.random_tick(at(0, 70, 0), 0);
    assert_eq!(world.state(at(0, 70, 0)), water(0));
}

const FROST: BlockPos = at(0, 70, 0);

fn frosted_ice(age: i32) -> i32 {
    state_of("minecraft:frosted_ice", &[("age", &age.to_string())])
}

fn frost_tick(world: &mut NativeWorld, pos: BlockPos) {
    let state = world.state(pos);
    world.with_level(|level| tick_block(level, pos, state));
}

#[test]
fn frosted_ice_stays_unaged_in_the_dark() {
    let mut world = NativeWorld::new();
    world.set(FROST, frosted_ice(0));
    world.illuminate(FROST, 0);

    frost_tick(&mut world, FROST);

    assert_eq!(world.state(FROST), frosted_ice(0));
}

#[test]
fn frosted_ice_ages_one_step_at_a_time_and_only_then_goes() {
    let mut world = NativeWorld::new();
    world.set(FROST, frosted_ice(0));
    frost_tick(&mut world, FROST);
    assert_eq!(world.state(FROST), frosted_ice(1));

    let mut world = NativeWorld::new();
    world.set(FROST, frosted_ice(3));
    frost_tick(&mut world, FROST);
    assert_eq!(world.state(FROST), water(0));
}

#[test]
fn ice_that_goes_takes_its_neighbours_one_step_with_it() {
    let neighbour = at(1, 70, 0);
    let mut world = NativeWorld::new();
    world.set(FROST, frosted_ice(3));
    world.set(neighbour, frosted_ice(0));

    frost_tick(&mut world, FROST);

    assert_eq!(world.state(FROST), water(0));
    assert_eq!(world.state(neighbour), water(0));

    let mut world = NativeWorld::new();
    world.set(FROST, frosted_ice(3));
    world.set(neighbour, frosted_ice(0));
    world.set(at(2, 70, 0), frosted_ice(0));
    world.set(at(1, 70, 1), frosted_ice(0));
    frost_tick(&mut world, FROST);
    assert_eq!(world.state(neighbour), frosted_ice(1));
}

#[test]
fn frosted_ice_left_with_fewer_than_two_of_its_own_kind_melts_on_the_spot() {
    let mut world = NativeWorld::new();
    world.set(FROST, frosted_ice(0));
    world.set(at(1, 70, 0), frosted_ice(0));
    world.neighbor_changed(FROST, block_type_of(frosted_ice(0)).unwrap());
    assert_eq!(world.state(FROST), water(0));

    let mut world = NativeWorld::new();
    world.set(FROST, frosted_ice(0));
    world.set(at(1, 70, 0), frosted_ice(0));
    world.set(at(-1, 70, 0), frosted_ice(0));
    world.neighbor_changed(FROST, block_type_of(frosted_ice(0)).unwrap());
    assert_eq!(world.state(FROST), frosted_ice(0));

    let mut world = NativeWorld::new();
    world.set(FROST, frosted_ice(0));
    world.neighbor_changed(FROST, block_type_of(stone()).unwrap());
    assert_eq!(world.state(FROST), frosted_ice(0));
}
