use super::*;

const TARGET: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn target(power: i32) -> i32 {
    state_of("minecraft:target", &[("power", &power.to_string())])
}

#[test]
fn placement_recovery_clears_target_power_only_without_a_pending_tick() {
    let powered = target(13);
    let unpowered = target(0);
    let tick_type = BlockTickType::of_block_state(powered).unwrap();
    let mut recovered = NativeWorld::ticking(TARGET);
    load_target(&mut recovered, powered);
    recovered.with_level(|level| on_place(level, TARGET, powered, stone(), false));
    assert_eq!(recovered.state(TARGET), unpowered);

    let mut pending = NativeWorld::ticking(TARGET);
    load_target(&mut pending, powered);
    pending
        .with_level(|level| level.schedule_block_tick(TARGET, tick_type, 4, TickPriority::Normal));
    pending.with_level(|level| on_place(level, TARGET, powered, stone(), false));
    assert_eq!(pending.state(TARGET), powered);
}

#[test]
fn real_level_ticks_deliver_the_reset_once() {
    let powered = target(15);
    let unpowered = target(0);
    let mut world = NativeWorld::ticking(TARGET);
    load_target(&mut world, powered);
    world.with_level(|level| {
        level.schedule_block_tick(
            TARGET,
            BlockTickType::of_block_state(powered).expect("target tick type"),
            1,
            TickPriority::Normal,
        )
    });

    world.run_tick();

    assert_eq!(world.state(TARGET), unpowered);
}

fn load_target(world: &mut NativeWorld, state: i32) {
    use bbb_server_types::save::Java26BuiltinDimension;
    world.set(TARGET, target(0));
    // Recover legal saved target power rather than replay its placement.
    world
        .world
        .set_raw_block_state_for_test(Java26BuiltinDimension::Overworld, TARGET, state);
}
