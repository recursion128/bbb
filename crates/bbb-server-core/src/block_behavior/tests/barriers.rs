use crate::fluid::FlowingFluid;

use super::*;

const BARRIER_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn barrier(waterlogged: bool) -> i32 {
    state_of(
        "minecraft:barrier",
        &[("waterlogged", if waterlogged { "true" } else { "false" })],
    )
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction) -> i32 {
    world.set(BARRIER_POS, state);
    world.set(relative(BARRIER_POS, direction), stone());
    world.shape_update(BARRIER_POS, direction);
    world.state(BARRIER_POS)
}

#[test]
fn waterlogged_barrier_rearms_water_for_every_neighbour_direction() {
    let state = barrier(true);
    for direction in DIRECTION_VALUES {
        let mut world = NativeWorld::new();
        assert_eq!(update(&mut world, state, direction), state);
        assert!(world.saved_fluid_ticks().iter().any(|tick| {
            tick.id == "minecraft:water"
                && (tick.x, tick.y, tick.z) == (BARRIER_POS.x, BARRIER_POS.y, BARRIER_POS.z)
                && tick.delay == FlowingFluid::WATER_TICK_DELAY
        }));
        assert_eq!(world.state(BARRIER_POS), state);
    }
}
