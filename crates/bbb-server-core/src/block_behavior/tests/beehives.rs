//! `BeehiveBlock` identity, honey analog and harvest gates.

use super::state_of;

fn hive(name: &str, honey: i32) -> i32 {
    state_of(
        name,
        &[("facing", "north"), ("honey_level", &honey.to_string())],
    )
}

/// `BeehiveBlock.getAnalogOutputSignal` (`BeehiveBlock.java:85-87`).
#[test]
fn analog_follows_honey_level() {
    use crate::block_behavior::beehive::{analog_output, honey_level, reset_honey_state};
    let full = hive("minecraft:bee_nest", 5);
    assert_eq!(honey_level(full), 5);
    assert_eq!(analog_output(full), 5);
    let empty = reset_honey_state(full).expect("honey_level is a property");
    assert_eq!(honey_level(empty), 0);
    assert_eq!(analog_output(empty), 0);
    assert_eq!(analog_output(hive("minecraft:beehive", 3)), 3);
}

#[test]
fn shears_and_glass_bottles_are_the_two_harvest_items() {
    use crate::block_behavior::beehive::{item_is_glass_bottle, item_is_shears};
    assert!(item_is_shears(Some(
        bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID
    )));
    assert!(item_is_glass_bottle(Some(
        bbb_protocol::item_types::VANILLA_ITEM_GLASS_BOTTLE_ID
    )));
    assert!(!item_is_shears(Some(
        bbb_protocol::item_types::VANILLA_ITEM_GLASS_BOTTLE_ID
    )));
    assert!(!item_is_glass_bottle(None));
}
