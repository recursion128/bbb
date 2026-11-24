use super::*;

#[test]
fn the_random_tick_speed_rule_defaults_to_three_and_bottoms_out_at_zero() {
    assert_eq!(DEFAULT_RANDOM_TICK_SPEED, 3);
    assert_eq!(MIN_RANDOM_TICK_SPEED, 0);
}

#[test]
fn a_sampled_position_always_lands_inside_the_section_it_was_drawn_for() {
    let random = crate::random::GameRandom::with_seed(1);
    for _ in 0..4096 {
        let drawn = random_block_pos(&random, 64, -32, -16, RANDOM_TICK_Y_MASK);
        assert!((64..80).contains(&drawn.x), "x out of section: {drawn:?}");
        assert!((-32..-16).contains(&drawn.y), "y out of section: {drawn:?}");
        assert!((-16..0).contains(&drawn.z), "z out of section: {drawn:?}");
    }
}
