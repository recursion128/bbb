//! `/setidletimeout` and the kick it arms: which packets count as activity and
//! what the tick does once none has arrived for long enough.

use super::*;
use crate::world::idle_timeout::{advance_clock_for_test, set_clock_for_test};

/// The allowance one armed minute buys, with enough slack either side that the
/// real clock advancing under the test cannot decide an outcome.
const WELL_PAST_A_MINUTE: i64 = 90_000;
const WELL_INSIDE_A_MINUTE: i64 = 30_000;

fn idle_close(tick: &TickResult, connection_id: u64) -> bool {
    tick.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Close {
                connection_id: id,
                reason: WorldCloseReason::IdleTimeout,
            } if *id == ConnectionId(connection_id)
        )
    })
}

/// Reply to any `KeepAlive` challenges emitted during `tick`, matching a live
/// idle client (`KeepAlive` has `PlayerActivity::Never`, so acknowledging it
/// keeps the connection alive without resetting the idle timer).
fn ack_keep_alives(sender: &WorldCommandSender, tick: &TickResult) {
    for event in &tick.events {
        if let WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::KeepAlive { id },
        } = event
        {
            enqueue(
                sender,
                WorldCommand::Play {
                    connection_id: *connection_id,
                    packet: PlayServerbound::KeepAlive { id: *id },
                },
            )
            .unwrap();
        }
    }
}

/// `setIdleTimeout` (SetPlayerIdleTimeoutCommand.java:22) writes the value the
/// kick reads, and until it does the kick is off: `playerIdleTimeout() > 0` is
/// the second conjunct of `tick`'s idle arm
/// (ServerGamePacketListenerImpl.java:301).
#[test]
fn setidletimeout_arms_a_kick_that_is_off_by_default() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    // `DedicatedServerProperties.java:121` defaults `player-idle-timeout` to 0.
    assert_eq!(world.player_idle_timeout_for_test(), 0);

    advance_clock_for_test(10 * WELL_PAST_A_MINUTE);
    let unarmed = world.tick(receiver.drain());
    assert!(!idle_close(&unarmed, 1));
    assert_eq!(unarmed.snapshot.players.len(), 1);
    ack_keep_alives(&sender, &unarmed);

    // The command is chat, and `tryHandleChat` (`:1613-1620`) resets the
    // last action time on its way through -- so the idle window starts again
    // from here.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ChatCommand {
            command: "setidletimeout 1".into(),
        },
    );
    assert_eq!(world.player_idle_timeout_for_test(), 1);
    let still_here = world.tick(receiver.drain());
    assert!(!idle_close(&still_here, 1));
    ack_keep_alives(&sender, &still_here);

    advance_clock_for_test(WELL_PAST_A_MINUTE);
    let kicked = world.tick(receiver.drain());
    assert!(idle_close(&kicked, 1));
    assert!(kicked.snapshot.players.is_empty());
}

/// Zero is the disabled value the command's own wording names, and it disarms
/// a kick that was armed.
#[test]
fn zero_disarms_the_kick_again() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    for minutes in ["1", "0"] {
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ChatCommand {
                command: format!("setidletimeout {minutes}"),
            },
        );
    }
    assert_eq!(world.player_idle_timeout_for_test(), 0);

    advance_clock_for_test(10 * WELL_PAST_A_MINUTE);
    let spared = world.tick(receiver.drain());
    assert!(!idle_close(&spared, 1));
    assert_eq!(spared.snapshot.players.len(), 1);
}

/// The allowance is per session, and one packet inside the window is all it
/// takes: `resetLastActionTime` (ServerPlayer.java:1890-1892) is a per-player
/// field, not a server-wide one.
#[test]
fn the_kick_spares_the_session_that_did_something() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ChatCommand {
            command: "setidletimeout 1".into(),
        },
    );
    advance_clock_for_test(WELL_PAST_A_MINUTE);

    // Enqueued rather than played: `apply_command` runs ahead of the tick's
    // keep-alive arm, so the swing has to land in the same tick the kick is
    // decided in for this to be about the reset rather than about ordering.
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::Swing {
                hand: InteractionHand::MainHand,
            },
        },
    )
    .unwrap();
    let ticked = world.tick(receiver.drain());
    assert!(!idle_close(&ticked, 1));
    assert!(idle_close(&ticked, 2));
    assert_eq!(ticked.snapshot.players.len(), 1);
}

/// `handlePlayerKnownMovement` (`:2179-2182`) resets only when the move was
/// larger than `1.0E-5F` squared, so a client that only looks around -- which
/// it does whether or not anyone is at the keyboard -- stays idle.
#[test]
fn a_move_counts_only_when_the_player_actually_moved() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    // One step of a walking client, which is inside the tick budget
    // `handleMovePlayer` enforces (`:1103`).
    let ground = Vec3d {
        x: start.x + 0.25,
        ..start
    };
    let moved = |world: &mut ServerWorld, packet: PlayServerbound| {
        let before = world.last_action_millis_for_test(ConnectionId(1));
        advance_clock_for_test(WELL_INSIDE_A_MINUTE);
        let tick = play(&sender, &receiver, world, 1, packet);
        ack_keep_alives(&sender, &tick);
        world.last_action_millis_for_test(ConnectionId(1)) != before
    };

    // The first update carries a position, so it is a move: the session is
    // still at the spawn point it joined at.
    assert!(moved(
        &mut world,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(ground),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        })
    ));
    // The same position again is a zero-length move.
    assert!(!moved(
        &mut world,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(ground),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        })
    ));
    // And a look-only update carries no position at all.
    assert!(!moved(
        &mut world,
        PlayServerbound::MovePlayer(MovePlayer {
            position: None,
            rotation: Some(PlayerRotation {
                y_rot: 90.0,
                x_rot: 0.0,
            }),
            on_ground: true,
            horizontal_collision: false,
        })
    ));
}

/// The handlers vanilla wraps in `hasClientLoaded()` -- `handlePlayerInput`
/// (`:417-421`) among them -- do not reset for a session that has not sent
/// `ServerboundPlayerLoadedPacket` yet, because the handler never reaches the
/// call.
#[test]
fn a_session_that_has_not_loaded_does_not_reset_on_a_gated_handler() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    enqueue(
        &sender,
        WorldCommand::Join {
            client_information: Default::default(),
            context: JoinContext::default(),
            connection_id: ConnectionId(1),
            profile: profile("alice", 1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert!(!world.publication().players[0].client_loaded);

    let before = world.last_action_millis_for_test(ConnectionId(1));
    advance_clock_for_test(WELL_INSIDE_A_MINUTE);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput::default()),
    );
    assert_eq!(world.last_action_millis_for_test(ConnectionId(1)), before);

    // The ungated shape resets on the same session.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Swing {
            hand: InteractionHand::MainHand,
        },
    );
    assert!(world.last_action_millis_for_test(ConnectionId(1)) > before);
}

/// `Util.getMillis() - getLastActionTime() > TimeUnit.MINUTES.toMillis(..)`
/// (`:302`) is exceeded rather than reached: a session that has been still for
/// exactly the allowance keeps its place.
#[test]
fn the_allowance_is_exceeded_rather_than_reached() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ChatCommand {
            command: "setidletimeout 1".into(),
        },
    );

    advance_clock_for_test(60_000);
    let exactly = world.tick(receiver.drain());
    assert!(!idle_close(&exactly, 1));
    advance_clock_for_test(1);
    let over = world.tick(receiver.drain());
    assert!(idle_close(&over, 1));
}

/// `getLastActionTime() > 0` (`:300`) is the first conjunct, and it is not
/// spare: `lastActionTime` is only ever `Util.getMillis()`, so a platform whose
/// clock has not passed zero leaves the session with a last action the kick
/// refuses to measure against.
#[test]
fn a_session_whose_last_action_predates_the_clock_is_never_kicked() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("op", 2)])).unwrap();
    set_clock_for_test(0);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    assert_eq!(world.last_action_millis_for_test(ConnectionId(1)), 0);

    // The operator arms the kick from a session the clock did reach, so the
    // difference between the two is the guard and nothing else.
    set_clock_for_test(1);
    join_loaded(&sender, &receiver, &mut world, 2, "op", 2);
    play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::ChatCommand {
            command: "setidletimeout 1".into(),
        },
    );

    advance_clock_for_test(WELL_PAST_A_MINUTE);
    let ticked = world.tick(receiver.drain());
    assert!(!idle_close(&ticked, 1));
    assert!(idle_close(&ticked, 2));
}

/// A session watching the End credits is neither kicked for idling
/// (`ServerGamePacketListenerImpl.java:300-305`) nor moved by its packets
/// (`:1059`).
#[test]
fn a_player_watching_the_credits_is_not_idle_and_does_not_move() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    chat_command(&sender, &receiver, &mut world, 1, "setidletimeout 1");
    world.set_won_game_for_test(ConnectionId(1));
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: start.x + 0.25,
                ..start
            }),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    assert_eq!(
        world.player_movement_validation_for_test(ConnectionId(1)).0,
        start
    );

    advance_clock_for_test(WELL_PAST_A_MINUTE);
    let spared = world.tick(receiver.drain());
    assert!(!idle_close(&spared, 1));
}
