use crate::fluid::FlowingFluid;

use super::*;

const ROOTS_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn roots(waterlogged: bool) -> i32 {
    state_of(
        "minecraft:mangrove_roots",
        &[("waterlogged", if waterlogged { "true" } else { "false" })],
    )
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction) -> i32 {
    world.set(ROOTS_POS, state);
    world.set(relative(ROOTS_POS, direction), stone());
    world.shape_update(ROOTS_POS, direction);
    world.state(ROOTS_POS)
}

#[test]
fn waterlogged_roots_rearm_water_for_every_neighbour_direction() {
    let state = roots(true);
    for direction in DIRECTION_VALUES {
        let mut world = NativeWorld::new();
        assert_eq!(update(&mut world, state, direction), state);
        assert!(world.saved_fluid_ticks().iter().any(|tick| {
            tick.id == "minecraft:water"
                && (tick.x, tick.y, tick.z) == (ROOTS_POS.x, ROOTS_POS.y, ROOTS_POS.z)
                && tick.delay == FlowingFluid::WATER_TICK_DELAY
        }));
        assert_eq!(world.state(ROOTS_POS), state);
    }
}
