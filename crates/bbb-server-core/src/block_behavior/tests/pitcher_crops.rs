use super::native_world::NativeWorld;
use super::*;

const PLANT: BlockPos = BlockPos { x: 0, y: 71, z: 0 };

fn pitcher_crop(age: i32, half: &str) -> i32 {
    state_of(
        "minecraft:pitcher_crop",
        &[("age", &age.to_string()), ("half", half)],
    )
}

fn on_moist_farmland(age: i32) -> NativeWorld {
    let mut world = NativeWorld::new();
    for x in -2..=2 {
        for z in -2..=2 {
            world.set(at(x, 70, z), dirt());
        }
    }
    world.set(at(0, 70, 0), farmland_state(farmland::MAX_MOISTURE));
    world.set(PLANT, pitcher_crop(age, "lower"));
    world
}

fn shape_update(
    world: &mut NativeWorld,
    state: i32,
    direction: Direction,
    neighbour_state: i32,
) -> i32 {
    world.set(PLANT, state);
    world.set(relative(PLANT, direction), neighbour_state);
    world.shape_update(PLANT, direction);
    world.state(PLANT)
}

#[test]
fn a_pitcher_bulb_ages_on_a_winning_growth_roll() {
    let mut world = on_moist_farmland(0);

    world.random_tick(PLANT, 30);

    assert_eq!(world.state(PLANT), pitcher_crop(1, "lower"));
}

#[test]
fn reaching_the_double_plant_age_places_the_upper_half() {
    let mut world = on_moist_farmland(2);

    world.random_tick(PLANT, 30);

    assert_eq!(world.state(PLANT), pitcher_crop(3, "lower"));
    assert_eq!(world.state(above(PLANT)), pitcher_crop(3, "upper"));
}

#[test]
fn a_blocked_cell_above_refuses_the_whole_growth() {
    let mut world = on_moist_farmland(2);
    world.set(above(PLANT), stone());

    world.random_tick(PLANT, 30);

    assert_eq!(world.state(PLANT), pitcher_crop(2, "lower"));
    assert_eq!(world.state(above(PLANT)), stone());
}

#[test]
fn a_pitcher_crop_of_the_same_block_above_is_a_cell_it_may_grow_into() {
    let mut world = on_moist_farmland(3);
    world.set(above(PLANT), pitcher_crop(3, "upper"));

    world.random_tick(PLANT, 30);

    assert_eq!(world.state(PLANT), pitcher_crop(4, "lower"));
    assert_eq!(world.state(above(PLANT)), pitcher_crop(4, "upper"));
}

#[test]
fn a_fully_grown_pitcher_crop_does_not_grow() {
    let mut world = on_moist_farmland(4);

    world.random_tick(PLANT, 30);

    assert_eq!(world.state(PLANT), pitcher_crop(4, "lower"));
}

#[test]
fn growth_needs_the_crop_survival_light_level() {
    let mut world = on_moist_farmland(0);
    world.illuminate(PLANT, crop::SURVIVE_LIGHT_THRESHOLD - 1);
    world.random_tick(PLANT, 30);
    assert_eq!(world.state(PLANT), pitcher_crop(0, "lower"));

    let mut world = on_moist_farmland(0);
    world.illuminate(PLANT, crop::SURVIVE_LIGHT_THRESHOLD);
    world.random_tick(PLANT, 30);
    assert_eq!(world.state(PLANT), pitcher_crop(1, "lower"));
}

#[test]
fn a_bulb_ignores_the_partner_rule_and_the_grown_plant_does_not() {
    let mut world = on_moist_farmland(0);
    for age in 0..pitcher_crop::DOUBLE_PLANT_AGE_INTERSECTION {
        let lower = pitcher_crop(age, "lower");
        assert_eq!(
            shape_update(&mut world, lower, Direction::Up, air()),
            lower,
            "age {age}"
        );
    }
    for age in pitcher_crop::DOUBLE_PLANT_AGE_INTERSECTION..=pitcher_crop::MAX_AGE {
        let lower = pitcher_crop(age, "lower");
        assert_eq!(
            shape_update(&mut world, lower, Direction::Up, air()),
            air(),
            "age {age}"
        );
        assert_eq!(
            shape_update(&mut world, lower, Direction::Up, pitcher_crop(age, "upper")),
            lower,
            "age {age}"
        );
    }
}

#[test]
fn a_pitcher_crop_that_loses_its_farmland_becomes_air_at_every_age() {
    let mut world = on_moist_farmland(0);
    world.set(below(PLANT), stone());
    for age in 0..=pitcher_crop::MAX_AGE {
        let lower = pitcher_crop(age, "lower");
        assert_eq!(
            shape_update(&mut world, lower, Direction::Down, stone()),
            air(),
            "age {age}"
        );
    }
}

#[test]
fn a_pitcher_crop_below_the_survival_light_cannot_stand() {
    let mut world = on_moist_farmland(0);
    world.illuminate(PLANT, crop::SURVIVE_LIGHT_THRESHOLD - 1);
    let lower = pitcher_crop(0, "lower");
    assert_eq!(
        shape_update(&mut world, lower, Direction::North, air()),
        air()
    );
    world.illuminate(PLANT, crop::SURVIVE_LIGHT_THRESHOLD);
    assert_eq!(
        shape_update(&mut world, lower, Direction::North, air()),
        lower
    );
}
