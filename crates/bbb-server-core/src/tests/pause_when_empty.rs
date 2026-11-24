//! `pause-when-empty-seconds` (`MinecraftServer.java:954-972`).

use super::*;

fn autosaves(tick: &TickResult) -> usize {
    tick.events
        .iter()
        .filter(|event| {
            matches!(
                event,
                WorldEvent::SaveRequested {
                    trigger: SaveTrigger::Autosave,
                    ..
                }
            )
        })
        .count()
}

/// Twenty empty ticks per configured second run as usual; the tick that
/// reaches the threshold saves once and every later one leaves game time
/// alone until a player is online again.
#[test]
fn an_empty_server_pauses_after_the_configured_time_and_resumes_on_join() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        pause_when_empty_seconds: 1,
        ..ServerWorldConfig::default()
    })
    .unwrap();
    let start = world.game_time();
    for _ in 0..19 {
        assert_eq!(autosaves(&world.tick(Vec::new())), 0);
    }
    let running = world.game_time();
    assert!(running > start, "the first 19 empty ticks still run");

    assert_eq!(autosaves(&world.tick(Vec::new())), 1, "pausing saves once");
    for _ in 0..40 {
        assert_eq!(autosaves(&world.tick(Vec::new())), 0);
    }
    assert_eq!(world.game_time(), running, "a paused server keeps its time");

    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let joined = world.game_time();
    world.tick(Vec::new());
    world.tick(Vec::new());
    assert!(
        world.game_time() > joined,
        "a joined player resumes the tick"
    );
}

/// Zero, the value every other test world runs with, never pauses.
#[test]
fn a_nonpositive_pause_never_stops_the_tick() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    for _ in 0..100 {
        world.tick(Vec::new());
    }
    let before = world.game_time();
    world.tick(Vec::new());
    assert!(world.game_time() > before);
}
