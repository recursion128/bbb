//! Hopper enable state follows live neighbor power.

use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::packets::BlockPos;

use super::{state_of, stone, NativeWorld};

fn enabled(state: i32) -> Option<String> {
    vanilla_block_state_identity(state)?
        .properties
        .get("enabled")
        .cloned()
}

const HOPPER: BlockPos = BlockPos { x: 0, y: 71, z: 0 };
const FLOOR: BlockPos = BlockPos { x: 0, y: 70, z: 0 };
const SIDE: BlockPos = BlockPos { x: 1, y: 71, z: 0 };

fn hopper_state(enabled: bool) -> i32 {
    state_of(
        "minecraft:hopper",
        &[
            ("facing", "down"),
            ("enabled", if enabled { "true" } else { "false" }),
        ],
    )
}

fn redstone_block() -> i32 {
    super::default_state("minecraft:redstone_block")
}

/// A neighbour signal turns `ENABLED` off; removing it turns `ENABLED` back on.
#[test]
fn a_hopper_locks_when_it_has_a_neighbour_signal() {
    let mut world = NativeWorld::new();
    world.set(FLOOR, stone());
    world.set(HOPPER, hopper_state(true));
    world.set(SIDE, redstone_block());
    world.neighbor_changed(
        HOPPER,
        bbb_server_types::block_queries::block_type_of(redstone_block()).unwrap(),
    );
    assert_eq!(enabled(world.state(HOPPER)).as_deref(), Some("false"));

    world.set(SIDE, super::air());
    world.neighbor_changed(
        HOPPER,
        bbb_server_types::block_queries::block_type_of(redstone_block()).unwrap(),
    );
    assert_eq!(enabled(world.state(HOPPER)).as_deref(), Some("true"));
}
