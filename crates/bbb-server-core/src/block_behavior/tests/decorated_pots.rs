//! `DecoratedPotBlock` identity and water re-arm.

use crate::fluid::FlowingFluid;

use super::*;

const POT_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn pot(waterlogged: bool) -> i32 {
    state_of(
        "minecraft:decorated_pot",
        &[
            ("cracked", "false"),
            ("facing", "north"),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction) -> i32 {
    world.set(POT_POS, state);
    world.set(relative(POT_POS, direction), stone());
    world.shape_update(POT_POS, direction);
    world.state(POT_POS)
}

#[test]
fn waterlogged_decorated_pot_rearms_water_on_neighbour_update() {
    let state = pot(true);
    {
        let direction = Direction::North;
        let mut world = NativeWorld::new();
        assert_eq!(update(&mut world, state, direction), state);
        assert!(world.saved_fluid_ticks().iter().any(|tick| {
            tick.id == "minecraft:water"
                && (tick.x, tick.y, tick.z) == (POT_POS.x, POT_POS.y, POT_POS.z)
                && tick.delay == FlowingFluid::WATER_TICK_DELAY
        }));
    }
}
