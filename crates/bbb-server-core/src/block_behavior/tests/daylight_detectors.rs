use bbb_protocol::packets::{BlockPos, Direction};

use crate::block_behavior::daylight_detector::signal_strength;
use crate::block_behavior::use_without_item;
use bbb_server_types::block_queries::bool_property;

use super::{interaction_player, state_of, NativeWorld};

const DETECTOR: BlockPos = BlockPos { x: 0, y: 70, z: 0 };

fn detector(inverted: bool, power: i32) -> i32 {
    state_of(
        "minecraft:daylight_detector",
        &[
            ("inverted", if inverted { "true" } else { "false" }),
            ("power", &power.to_string()),
        ],
    )
}

#[test]
fn may_build_false_is_pass() {
    let mut world = NativeWorld::new();
    world.set(DETECTOR, detector(false, 0));
    let state = world.state(DETECTOR);
    assert!(!world.with_level(|level| use_without_item(
        level,
        DETECTOR,
        state,
        interaction_player(false, Direction::North)
    )));
    assert_eq!(world.state(DETECTOR), state);
}

#[test]
fn invert_flips_the_mode() {
    let mut world = NativeWorld::new();
    world.set(DETECTOR, detector(false, 0));
    let state = world.state(DETECTOR);
    assert!(world.with_level(|level| use_without_item(
        level,
        DETECTOR,
        state,
        interaction_player(true, Direction::North)
    )));
    assert_eq!(bool_property(world.state(DETECTOR), "inverted"), Some(true));
}

#[test]
fn inverted_power_is_fifteen_minus_effective_sky() {
    assert_eq!(signal_strength(true, 8, 0.0), 7);
}

#[test]
fn day_arm_at_zero_sun_angle_keeps_full_sky() {
    assert_eq!(signal_strength(false, 15, 0.0), 15);
}

#[test]
fn day_arm_scales_by_cos_after_the_point_two_lerp() {
    assert_eq!(signal_strength(false, 15, 90.0), 5);
}

#[test]
fn day_arm_past_pi_lerps_toward_two_pi() {
    assert_eq!(signal_strength(false, 15, 180.0), 0);
}

#[test]
fn signal_strength_clamps_to_the_signal_range() {
    assert_eq!(signal_strength(true, -5, 0.0), 15);
    assert_eq!(signal_strength(false, 0, 0.0), 0);
}
