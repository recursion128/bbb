use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::packets::BlockPos;

use super::{state_of, stone, NativeWorld};

const HEAD: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const SIDE: BlockPos = BlockPos { x: 1, y: 64, z: 0 };

fn player_head(powered: bool) -> i32 {
    state_of(
        "minecraft:player_head",
        &[
            ("powered", if powered { "true" } else { "false" }),
            ("rotation", "0"),
        ],
    )
}

fn powered(state: i32) -> Option<String> {
    vanilla_block_state_identity(state)?
        .properties
        .get("powered")
        .cloned()
}

fn redstone_block() -> i32 {
    super::default_state("minecraft:redstone_block")
}

#[test]
fn a_skull_powers_when_it_has_a_neighbour_signal() {
    let mut world = NativeWorld::new();
    world.set(HEAD, player_head(false));
    world.set(SIDE, redstone_block());
    world.neighbor_changed(
        HEAD,
        bbb_server_types::block_queries::block_type_of(redstone_block()).unwrap(),
    );
    assert_eq!(powered(world.state(HEAD)).as_deref(), Some("true"));
}

#[test]
fn a_skull_unpowers_when_the_neighbour_signal_goes() {
    let mut world = NativeWorld::new();
    world.set(HEAD, player_head(true));
    world.neighbor_changed(
        HEAD,
        bbb_server_types::block_queries::block_type_of(stone()).unwrap(),
    );
    assert_eq!(powered(world.state(HEAD)).as_deref(), Some("false"));
}

#[test]
fn a_wall_skull_powers_when_it_has_a_neighbour_signal() {
    let mut world = NativeWorld::new();
    world.set(HEAD, super::default_state("minecraft:skeleton_wall_skull"));
    world.set(SIDE, redstone_block());
    world.neighbor_changed(
        HEAD,
        bbb_server_types::block_queries::block_type_of(redstone_block()).unwrap(),
    );
    assert_eq!(powered(world.state(HEAD)).as_deref(), Some("true"));
}
