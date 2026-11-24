//! `RedStoneOreBlock` (`RedStoneOreBlock.java`): the lit ore that goes dark.

use bbb_protocol::block_types::{VanillaRandomTickOverride, VANILLA_BLOCK_TYPES};

use super::*;

use crate::block_behavior::redstone_ore;

const ORE: BlockPos = BlockPos { x: 3, y: 70, z: -6 };

fn ore(name: &str, lit: bool) -> i32 {
    state_of(name, &[("lit", if lit { "true" } else { "false" })])
}

/// Both 26.1 registrations of `RedStoneOreBlock`, read off the generated
/// `random_tick_override` class tag rather than listed by name.
fn redstone_ores() -> Vec<&'static str> {
    VANILLA_BLOCK_TYPES
        .iter()
        .filter(|entry| {
            entry.random_tick_override == Some(VanillaRandomTickOverride::RedStoneOreBlock)
        })
        .map(|entry| entry.resource_id)
        .collect()
}

#[test]
fn a_lit_ore_goes_dark_on_a_random_tick() {
    for name in redstone_ores() {
        let mut world = NativeWorld::new();
        world.set(ORE, ore(name, true));
        world.random_tick(ORE, 41);

        assert_eq!(world.state(ORE), ore(name, false), "{name}");
    }
}

#[test]
fn interacting_lights_the_ore() {
    for name in redstone_ores() {
        let mut world = NativeWorld::new();
        let state = ore(name, false);
        world.set(ORE, state);
        let dimension = bbb_server_types::save::Java26BuiltinDimension::Overworld;
        assert_eq!(world.world.level_light_facts_for_test(dimension, ORE).0, 0);
        assert_eq!(
            world.with_level(|level| redstone_ore::interact(level, ORE, state)),
            Some(())
        );
        assert_eq!(world.state(ORE), ore(name, true), "{name}");
        world.run_tick();
        assert!(world.world.level_light_facts_for_test(dimension, ORE).0 > 0);
    }
}
