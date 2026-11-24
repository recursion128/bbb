use crate::fluid::FlowingFluid;
use bbb_protocol::block_types::{VanillaGetStateForPlacementOverride, VANILLA_BLOCK_TYPES};

use super::*;

const CHAIN_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn chain(resource_id: &str, waterlogged: bool) -> i32 {
    state_of(
        resource_id,
        &[
            ("axis", "y"),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

fn chain_entries() -> Vec<&'static bbb_protocol::block_types::BlockTypeCatalogEntry> {
    VANILLA_BLOCK_TYPES
        .iter()
        .filter(|entry| {
            entry.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::ChainBlock)
        })
        .collect()
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction) -> i32 {
    world.set(CHAIN_POS, state);
    world.set(relative(CHAIN_POS, direction), stone());
    world.shape_update(CHAIN_POS, direction);
    world.state(CHAIN_POS)
}

#[test]
fn waterlogged_chain_rearms_water_on_neighbour_update() {
    for entry in chain_entries().into_iter().take(1) {
        let state = chain(entry.resource_id, true);
        for direction in [Direction::Down, Direction::North] {
            let mut world = NativeWorld::new();
            assert_eq!(update(&mut world, state, direction), state);
            assert!(world
                .saved_fluid_ticks()
                .iter()
                .any(|tick| tick.id == "minecraft:water"
                    && (tick.x, tick.y, tick.z) == (CHAIN_POS.x, CHAIN_POS.y, CHAIN_POS.z)
                    && tick.delay == FlowingFluid::WATER_TICK_DELAY));
        }
    }
}

#[test]
fn dry_chain_does_not_schedule_water() {
    for entry in chain_entries().into_iter().take(1) {
        let state = chain(entry.resource_id, false);
        let mut world = NativeWorld::new();
        assert_eq!(update(&mut world, state, Direction::Down), state);
        assert!(
            world.saved_fluid_ticks().is_empty(),
            "{}",
            entry.resource_id
        );
    }
}
