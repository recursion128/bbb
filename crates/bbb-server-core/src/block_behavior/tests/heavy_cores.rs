use crate::fluid::FlowingFluid;

use super::*;

const CORE_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn core(waterlogged: bool) -> i32 {
    state_of(
        "minecraft:heavy_core",
        &[("waterlogged", if waterlogged { "true" } else { "false" })],
    )
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction) -> i32 {
    world.set(CORE_POS, state);
    world.set(relative(CORE_POS, direction), stone());
    world.shape_update(CORE_POS, direction);
    world.state(CORE_POS)
}

#[test]
fn waterlogged_heavy_core_rearms_water_on_neighbour_update() {
    let state = core(true);
    {
        let direction = Direction::North;
        let mut world = NativeWorld::new();
        assert_eq!(update(&mut world, state, direction), state);
        assert!(world.saved_fluid_ticks().iter().any(|tick| {
            tick.id == "minecraft:water"
                && (tick.x, tick.y, tick.z) == (CORE_POS.x, CORE_POS.y, CORE_POS.z)
                && tick.delay == FlowingFluid::WATER_TICK_DELAY
        }));
        assert_eq!(world.state(CORE_POS), state);
    }
}
