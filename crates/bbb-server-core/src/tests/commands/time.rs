//! `/time` against the live clock manager.

use super::*;

fn overworld_state(world: &ServerWorld) -> bbb_server_persistence::Java26WorldClockState {
    world.world_clock_states_for_test()[0]
}

/// `ServerClockManager.setTotalTicks` (`ServerClockManager.java:80-85`) writes
/// the clock the command resolved and drops the fractional tick with it.
#[test]
fn set_writes_the_live_clock_and_clears_the_partial_tick() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();
    world.set_world_clock_state_for_test(
        0,
        bbb_server_persistence::Java26WorldClockState {
            total_ticks: 40,
            partial_tick: 0.75,
            rate: 1.0,
            paused: false,
        },
    );

    let tick = block_command(&sender, &receiver, &mut world, 1, "time set 1000");
    assert_eq!(
        management_report(&tick),
        (
            Some(1000),
            vec!["commands.time.set.absolute(minecraft:overworld, 1000)".to_string()]
        )
    );
    // The command runs where `processQueuedPackets` does, ahead of
    // `tickChildren`'s clock arm (`MinecraftServer.java:1000-1010`,
    // `:1089-1092`), so the tick that carried it also advances the write once.
    let state = overworld_state(&world);
    assert_eq!(state.total_ticks, 1001);
    // The write is what the save carries out (`ServerClockManager.TYPE`,
    // `ServerClockManager.java:25-30`).
    let saved = world.world_clock_snapshot_for_test();
    assert_eq!(saved.entries()[0].resource_id, "minecraft:overworld");
    assert_eq!(saved.entries()[0].state.total_ticks, 1001);
}

/// `setRate` and `setPaused` (`ServerClockManager.java:104-110`) change what
/// `ClockInstance.tick` (`:161-167`) does on the ticks that follow.
#[test]
fn rate_and_pause_change_how_the_clock_advances() {
    let (sender, receiver) = mailbox();
    let mut world = block_command_world();

    block_command(&sender, &receiver, &mut world, 1, "time rate 3");
    let before = overworld_state(&world).total_ticks;
    world.tick(receiver.drain());
    assert_eq!(overworld_state(&world).total_ticks, before + 3);

    block_command(&sender, &receiver, &mut world, 2, "time pause");
    let paused_at = overworld_state(&world).total_ticks;
    world.tick(receiver.drain());
    assert_eq!(overworld_state(&world).total_ticks, paused_at);

    block_command(&sender, &receiver, &mut world, 3, "time resume");
    let resumed_at = overworld_state(&world).total_ticks;
    world.tick(receiver.drain());
    assert_eq!(overworld_state(&world).total_ticks, resumed_at + 3);
}

/// `modifyClock` broadcasts `Map.of(clock, ...)`
/// (`ServerClockManager.java:115-116`): the clock that moved, and not the
/// whole registry the join sync carries.
#[test]
fn a_clock_write_broadcasts_only_the_clock_that_moved() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    let tick = block_command(&sender, &receiver, &mut world, 1, "time set 1000");
    let updates = tick
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetTime(time),
                ..
            } => Some(time.clock_updates.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(updates.len(), 1);
    assert_eq!(updates[0].len(), 1);
    assert_eq!(updates[0][0].clock_id, 0);
    assert_eq!(updates[0][0].total_ticks, 1000);
}
