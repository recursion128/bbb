use super::*;

const CATALYST: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn catalyst(bloom: bool) -> i32 {
    state_of(
        "minecraft:sculk_catalyst",
        &[("bloom", if bloom { "true" } else { "false" })],
    )
}

#[test]
fn scheduled_tick_clears_an_active_bloom() {
    let active = catalyst(true);
    let dormant = catalyst(false);
    let mut world = NativeWorld::new();
    world.set(CATALYST, active);

    world.with_level(|level| tick_block(level, CATALYST, active));

    assert_eq!(world.state(CATALYST), dormant);
}

#[test]
fn dormant_tick_preserves_the_dormant_block() {
    let dormant = catalyst(false);
    let mut world = NativeWorld::new();

    world.set(CATALYST, dormant);
    world.with_level(|level| tick_block(level, CATALYST, dormant));

    assert_eq!(world.state(CATALYST), dormant);
}
