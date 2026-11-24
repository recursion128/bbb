use crate::fluid::FlowingFluid;
use bbb_protocol::block_types::{VanillaGetStateForPlacementOverride, VANILLA_BLOCK_TYPES};

use super::*;

const SLAB_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn slab(resource_id: &str, slab_type: &str, waterlogged: bool) -> i32 {
    state_of(
        resource_id,
        &[
            ("type", slab_type),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn slab_entries() -> Vec<&'static bbb_protocol::block_types::BlockTypeCatalogEntry> {
    VANILLA_BLOCK_TYPES
        .iter()
        .filter(|entry| {
            entry.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::SlabBlock)
        })
        .collect()
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction) -> i32 {
    world.set(SLAB_POS, state);
    world.set(relative(SLAB_POS, direction), stone());
    world.shape_update(SLAB_POS, direction);
    world.state(SLAB_POS)
}

#[test]
fn waterlogged_slab_rearms_water_on_neighbour_update() {
    for entry in slab_entries().into_iter().take(1) {
        {
            let slab_type = "bottom";
            let state = slab(entry.resource_id, slab_type, true);
            {
                let direction = Direction::North;
                let mut world = NativeWorld::new();
                assert_eq!(update(&mut world, state, direction), state);
                assert!(world.saved_fluid_ticks().iter().any(|tick| {
                    tick.id == "minecraft:water"
                        && (tick.x, tick.y, tick.z) == (SLAB_POS.x, SLAB_POS.y, SLAB_POS.z)
                        && tick.delay == FlowingFluid::WATER_TICK_DELAY
                }));
                assert_eq!(world.state(SLAB_POS), state);
            }
        }
    }
}
