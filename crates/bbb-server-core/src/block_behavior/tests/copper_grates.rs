use crate::fluid::FlowingFluid;
use bbb_protocol::block_types::{VanillaGetStateForPlacementOverride, VANILLA_BLOCK_TYPES};

use super::*;

const GRATE_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn grate(resource_id: &str, waterlogged: bool) -> i32 {
    state_of(
        resource_id,
        &[("waterlogged", if waterlogged { "true" } else { "false" })],
    )
}

fn grate_entries() -> Vec<&'static bbb_protocol::block_types::BlockTypeCatalogEntry> {
    VANILLA_BLOCK_TYPES
        .iter()
        .filter(|entry| {
            entry.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::WaterloggedTransparentBlock)
        })
        .collect()
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction) -> i32 {
    world.set(GRATE_POS, state);
    world.set(relative(GRATE_POS, direction), stone());
    world.shape_update(GRATE_POS, direction);
    world.state(GRATE_POS)
}

#[test]
fn waterlogged_grate_rearms_water_on_neighbour_update() {
    for entry in grate_entries().into_iter().take(1) {
        let state = grate(entry.resource_id, true);
        {
            let direction = Direction::North;
            let mut world = NativeWorld::new();
            assert_eq!(update(&mut world, state, direction), state);
            assert!(world.saved_fluid_ticks().iter().any(|tick| {
                tick.id == "minecraft:water"
                    && (tick.x, tick.y, tick.z) == (GRATE_POS.x, GRATE_POS.y, GRATE_POS.z)
                    && tick.delay == FlowingFluid::WATER_TICK_DELAY
            }));
            assert_eq!(world.state(GRATE_POS), state);
        }
    }
}
