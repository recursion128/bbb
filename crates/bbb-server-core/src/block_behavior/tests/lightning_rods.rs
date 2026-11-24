use super::*;

const ROD: BlockPos = BlockPos { x: 3, y: 70, z: 4 };

fn rod(name: &str, facing: &str, powered: bool, waterlogged: bool) -> i32 {
    state_of(
        name,
        &[
            ("facing", facing),
            ("powered", if powered { "true" } else { "false" }),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

#[test]
fn every_shape_update_returns_the_rod_and_rearms_only_waterlogged_states() {
    for direction in DIRECTION_VALUES {
        let state = rod("minecraft:lightning_rod", "up", false, true);
        let mut world = NativeWorld::new();
        world.set(ROD, state);
        world.set(relative(ROD, direction), stone());
        world.shape_update(ROD, direction);
        assert_eq!(world.state(ROD), state);
        assert!(world.saved_fluid_ticks().iter().any(|tick| {
            tick.id == "minecraft:water"
                && (tick.x, tick.y, tick.z) == (ROD.x, ROD.y, ROD.z)
                && tick.delay == 5
        }));
    }
    let state = rod("minecraft:waxed_lightning_rod", "up", false, false);
    let mut world = NativeWorld::new();
    world.set(ROD, state);
    world.set(relative(ROD, Direction::North), water(0));
    world.shape_update(ROD, Direction::North);
    assert_eq!(world.state(ROD), state);
    assert!(world
        .saved_fluid_ticks()
        .iter()
        .all(|tick| (tick.x, tick.y, tick.z) != (ROD.x, ROD.y, ROD.z)));
}

#[test]
fn powered_rod_placement_recovers_to_an_unpowered_state() {
    for (name, facing) in [
        ("minecraft:lightning_rod", "up"),
        ("minecraft:waxed_exposed_lightning_rod", "south"),
    ] {
        let state = rod(name, facing, true, false);
        let expected = rod(name, facing, false, false);
        let mut world = NativeWorld::ticking(ROD);
        world.set(ROD, state);
        for _ in 0..9 {
            world.run_tick();
        }
        assert_eq!(world.state(ROD), expected);
    }
}

#[test]
fn unwaxed_rod_still_uses_the_weathering_random_tick() {
    let mut world = NativeWorld::new();
    world.set(ROD, rod("minecraft:lightning_rod", "north", true, true));
    world.random_tick(ROD, 4610);

    assert_eq!(
        world.state(ROD),
        rod("minecraft:exposed_lightning_rod", "north", true, true)
    );
}
