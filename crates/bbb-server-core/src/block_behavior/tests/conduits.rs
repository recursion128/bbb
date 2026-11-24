use crate::fluid::FlowingFluid;

use super::*;

const CONDUIT_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn conduit(waterlogged: bool) -> i32 {
    state_of(
        "minecraft:conduit",
        &[("waterlogged", if waterlogged { "true" } else { "false" })],
    )
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction) -> i32 {
    world.set(CONDUIT_POS, state);
    world.set(relative(CONDUIT_POS, direction), stone());
    world.shape_update(CONDUIT_POS, direction);
    world.state(CONDUIT_POS)
}

#[test]
fn waterlogged_conduit_rearms_water_on_neighbour_update() {
    let state = conduit(true);
    for direction in [Direction::Down, Direction::North] {
        let mut world = NativeWorld::new();
        assert_eq!(update(&mut world, state, direction), state);
        assert!(world.saved_fluid_ticks().iter().any(|tick| {
            tick.id == "minecraft:water"
                && (tick.x, tick.y, tick.z) == (CONDUIT_POS.x, CONDUIT_POS.y, CONDUIT_POS.z)
                && tick.delay == FlowingFluid::WATER_TICK_DELAY
        }));
    }
}

#[test]
fn dry_conduit_remains_unchanged_without_a_tick() {
    let state = conduit(false);
    let mut world = NativeWorld::new();
    assert_eq!(update(&mut world, state, Direction::Down), state);
    assert!(world.saved_fluid_ticks().is_empty());
}
