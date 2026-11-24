//! `ShulkerBoxBlock` identity and `canOpen`.

use super::{default_state, state_of, stone};

fn shulker(facing: &str) -> i32 {
    state_of("minecraft:shulker_box", &[("facing", facing)])
}

#[test]
fn coloured_shulker_boxes_are_shulker_boxes() {
    use bbb_server_types::block_queries::is_shulker_box;
    let names = ["minecraft:shulker_box", "minecraft:red_shulker_box"];
    for name in names {
        assert!(is_shulker_box(default_state(name)), "{name}");
    }
    assert!(!is_shulker_box(stone()));
    assert!(!is_shulker_box(default_state("minecraft:chest")));
}

/// `ShulkerBoxBlock.FACING` (`ShulkerBoxBlock.java:48`), default up.
#[test]
fn the_default_facing_is_up() {
    use crate::block_behavior::shulker_box::facing;
    use bbb_protocol::packets::Direction;
    assert_eq!(
        facing(default_state("minecraft:shulker_box")),
        Direction::Up
    );
    assert_eq!(facing(shulker("north")), Direction::North);
}
