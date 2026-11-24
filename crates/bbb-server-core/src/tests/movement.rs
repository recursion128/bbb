//! Player movement: the relative and absolute moves a session broadcasts, the
//! rotation it stores, and the camera a spectator rides.

use super::*;

#[test]
fn a_player_outside_the_border_can_crouch_and_stand() {
    use bbb_protocol::entity_poses::{
        VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
    };

    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let position = world.publication().players[0].position;
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("worldborder center {} {}", position.x - 10.0, position.z),
    );
    for diameter in [1000, 2] {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("worldborder set {diameter}"),
        );
        for (shift, expected) in [
            (true, VANILLA_ENTITY_POSE_CROUCHING_ID),
            (false, VANILLA_ENTITY_POSE_STANDING_ID),
        ] {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::PlayerInput(PlayerInput {
                    shift,
                    ..PlayerInput::default()
                }),
            );
            assert_eq!(
                world.player_pose_id_for_test(ConnectionId(1)),
                expected,
                "border {diameter}, shift {shift}",
            );
        }
    }
}

fn is_move_entity(event: &WorldEvent) -> bool {
    matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::MoveEntity(_),
            ..
        }
    )
}

fn admit_players(world: &mut ServerWorld, connection_ids: &[u64]) {
    for &connection_id in connection_ids {
        world.reset_pending_chunks_for_test(ConnectionId(connection_id), &[]);
    }
    for &connection_id in connection_ids {
        world.reconcile_player_tracking_for_test(ConnectionId(connection_id));
    }
}

/// `ServerPlayer.checkMovementStatistics` (`ServerPlayer.java:1420-1468`) picks
/// exactly one counter per move, and the four branches above the ones this
/// server can decide are cut for want of fluid contact and a climbable scan.
#[test]
fn one_move_feeds_exactly_one_movement_counter() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let alice = entity_id_of(&world, 1);
    let stat = |world: &ServerWorld, name: &str| {
        world.player_stat_for_test(ConnectionId(1), bbb_server_types::stats::custom_stat(name))
    };
    // `handleMovePlayer` validates every step against the position the tick
    // began at, so the fixture's own spawn is the only legal starting point.
    let ground = world.player_movement_validation_for_test(ConnectionId(1)).0;
    let step = |world: &mut ServerWorld, from: Vec3d, dx: f64, dy: f64, on_ground: bool| {
        let to = Vec3d {
            x: from.x + dx,
            y: from.y + dy,
            z: from.z,
        };
        play(
            &sender,
            &receiver,
            world,
            1,
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(to),
                rotation: None,
                on_ground,
                horizontal_collision: false,
            }),
        );
        to
    };
    let at = step(&mut world, ground, 0.0, 0.0, true);

    // `:1454`: on the ground, neither sprinting nor crouching.
    let walked = stat(&world, "minecraft:walk_one_cm");
    let at = step(&mut world, at, 1.0, 0.0, true);
    assert_eq!(stat(&world, "minecraft:walk_one_cm") - walked, 100);
    assert_eq!(stat(&world, "minecraft:sprint_one_cm"), 0);
    assert_eq!(stat(&world, "minecraft:crouch_one_cm"), 0);

    // `:1447-1449`: sprinting outranks crouching.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput {
            shift: true,
            ..PlayerInput::default()
        }),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: alice,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        }),
    );
    let walked = stat(&world, "minecraft:walk_one_cm");
    let at = step(&mut world, at, 0.5, 0.0, true);
    assert_eq!(stat(&world, "minecraft:sprint_one_cm"), 50);
    assert_eq!(stat(&world, "minecraft:walk_one_cm"), walked);
    assert_eq!(stat(&world, "minecraft:crouch_one_cm"), 0);

    // `:1450-1452` once the sprint stops, the shift key still down.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerCommand(PlayerCommand {
            entity_id: alice,
            action: PlayerCommandAction::StopSprinting,
            data: 0,
        }),
    );
    let at = step(&mut world, at, 0.25, 0.0, true);
    assert_eq!(stat(&world, "minecraft:crouch_one_cm"), 25);
    assert_eq!(stat(&world, "minecraft:sprint_one_cm"), 50);

    // `:1461-1465`: off the ground and not gliding, and only past 25cm.
    let at = step(&mut world, at, 0.2, 0.0, false);
    assert_eq!(stat(&world, "minecraft:fly_one_cm"), 0);
    let at = step(&mut world, at, 0.3, 0.0, false);
    assert_eq!(stat(&world, "minecraft:fly_one_cm"), 30);

    // `:1458-1460`: gliding outranks the fall-through, and awards without a
    // floor under the distance.
    world.set_player_fall_flying_for_test(ConnectionId(1), true);
    let at = step(&mut world, at, 0.2, 0.0, false);
    assert_eq!(stat(&world, "minecraft:aviate_one_cm"), 20);
    assert_eq!(stat(&world, "minecraft:fly_one_cm"), 30);

    // `:1421` through `didNotMove` (`:1492-1494`): a move of nothing counts
    // nothing anywhere. That the entry is not even created is pinned on the
    // written document by `walked_centimetres_count_into_the_statistics_file`.
    let aviated = stat(&world, "minecraft:aviate_one_cm");
    let at = step(&mut world, at, 0.0, 0.0, false);
    assert_eq!(stat(&world, "minecraft:aviate_one_cm"), aviated);

    world.set_player_fall_flying_for_test(ConnectionId(1), false);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput::default()),
    );
    // `:1445`: the on-ground branch measures the horizontal leg only, so a
    // metre travelled diagonally counts the sixty centimetres of it that were
    // horizontal.
    let walked = stat(&world, "minecraft:walk_one_cm");
    let at = step(&mut world, at, 0.6, 0.8, true);
    assert_eq!(stat(&world, "minecraft:walk_one_cm") - walked, 60);

    // `Math.round` (`:1445`) rounds the half up rather than dropping it.
    let walked = stat(&world, "minecraft:walk_one_cm");
    let at = step(&mut world, at, 0.125, 0.0, true);
    assert_eq!(stat(&world, "minecraft:walk_one_cm") - walked, 13);

    // `Player.getDesiredPose` (`Player.java:368`) wants CROUCHING only while
    // the player is not flying, so a flying session on the ground walks even
    // with the shift key held.
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput {
            shift: true,
            ..PlayerInput::default()
        }),
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAbilities { flying: true },
    );
    let crouched = stat(&world, "minecraft:crouch_one_cm");
    let walked = stat(&world, "minecraft:walk_one_cm");
    let at = step(&mut world, at, 0.4, 0.0, true);
    assert_eq!(stat(&world, "minecraft:crouch_one_cm"), crouched);
    assert_eq!(stat(&world, "minecraft:walk_one_cm") - walked, 40);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerAbilities { flying: false },
    );

    // `:1421`: a passenger's distance belongs to `checkRidingStatistics`
    // (`:1470-1490`), and its `MovePlayer` never reaches this far anyway
    // (`ServerGamePacketListenerImpl.java:1073-1075`).
    let counters = [
        "minecraft:walk_one_cm",
        "minecraft:sprint_one_cm",
        "minecraft:crouch_one_cm",
        "minecraft:fly_one_cm",
        "minecraft:aviate_one_cm",
    ];
    let before: Vec<i32> = counters.iter().map(|name| stat(&world, name)).collect();
    world.set_external_player_last_vehicle_for_test(ConnectionId(1), Some(4242));
    step(&mut world, at, 1.0, 0.0, true);
    let after: Vec<i32> = counters.iter().map(|name| stat(&world, name)).collect();
    assert_eq!(after, before);
}

#[test]
fn movement_broadcasts_relative_moves_to_other_players_only() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    // Alice moves +2/+3 horizontally along the floor and rotates: within the
    // i16 delta range, so bob gets a PosRot relative move
    // (ServerEntity.java:158-170). The ground the tick's travel found under
    // her stays, so the move does not flip it into an absolute resync.
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    x: 2.5,
                    y: 64.0,
                    z: 3.5,
                }),
                rotation: Some(PlayerRotation {
                    y_rot: 90.0,
                    x_rot: -30.0,
                }),
                on_ground: true,
                horizontal_collision: false,
            }),
        },
    )
    .unwrap();
    let moved = world.tick(receiver.drain());
    let movement = moved
        .events
        .iter()
        .filter(|event| is_move_entity(event))
        .collect::<Vec<_>>();
    assert_eq!(movement.len(), 1, "alice herself gets no movement echo");
    assert!(matches!(
        movement[0],
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::MoveEntity(update),
        } if update.id == 1
            && update.has_position
            && (update.delta_x, update.delta_y, update.delta_z) == (8192, 0, 12288)
            && update.y_rot == Some(90.0)
            && update.x_rot == Some(-30.0)
            && update.on_ground
    ));

    // Bob's next call is off his PLAYER interval, so the rotation waits for
    // his following `ChunkMap.tick` and then reaches Alice as a Rot packet.
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(2),
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: None,
                rotation: Some(PlayerRotation {
                    y_rot: 45.0,
                    x_rot: 0.0,
                }),
                on_ground: true,
                horizontal_collision: false,
            }),
        },
    )
    .unwrap();
    let deferred = world.tick(receiver.drain());
    assert!(!deferred.events.iter().any(is_move_entity));
    let rotated = world.tick(Vec::new());
    assert_eq!(rotated.events.len(), 2);
    assert!(matches!(
        &rotated.events[0],
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::MoveEntity(update),
        } if update.id == 2
            && !update.has_position
            && update.y_rot == Some(45.0)
            && update.x_rot == Some(0.0)
    ));
    assert!(matches!(
        &rotated.events[1],
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::RotateHead(update),
        } if update.id == 2 && update.y_head_rot == 45.0
    ));
}

#[test]
fn same_level_teleport_preserves_the_observer_sync_baseline() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    admit_players(&mut world, &[1, 2]);
    let alice = entity_id_of(&world, 1);
    let mut direct = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Overworld.into(),
            position: Vec3d {
                x: 20.5,
                y: 64.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut direct,
    ));
    assert!(!direct.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::MoveEntity(_)
                | ClientboundPlayPacket::EntityPositionSync(_),
        }
    )));
    let tick = world.tick(Vec::new());
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::EntityPositionSync(update),
        } if update.id == alice && update.position.x == 20.5
    )));
}

#[test]
fn cross_level_teleport_rebuilds_the_player_server_entity() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let alice = entity_id_of(&world, 1);
    for (connection_id, x) in [(ConnectionId(2), 50.5), (ConnectionId(1), 60.5)] {
        let mut direct = Vec::new();
        assert!(world.teleport_player(
            connection_id,
            crate::world::TeleportTransition {
                new_level: Java26BuiltinDimension::Nether.into(),
                position: Vec3d { x, y: 70.0, z: 0.5 },
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: 0,
            },
            &mut direct,
        ));
        if connection_id == ConnectionId(2) {
            world.tick(Vec::new());
        }
    }
    admit_players(&mut world, &[1, 2]);
    assert_eq!(
        world.player_tracking_viewers_for_test(Java26BuiltinDimension::Nether, alice),
        [ConnectionId(2)]
    );
    let tick = world.tick(Vec::new());
    assert!(tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::MoveEntity(update),
        } if update.id == alice
            && update.has_position
            && (update.delta_x, update.delta_y, update.delta_z) == (0, 0, 0)
    )));
    assert!(!tick.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::EntityPositionSync(update),
        } if update.id == alice
    )));
}

#[test]
fn death_respawn_rebuilds_the_player_server_entity() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    admit_players(&mut world, &[1, 2]);
    let alice = entity_id_of(&world, 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: 20.5,
                y: 64.0,
                z: 0.5,
            }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    world.set_player_dead_for_test(ConnectionId(1));
    let respawned = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    assert!(respawned.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::MoveEntity(update),
        } if update.id == alice
            && update.has_position
            && (update.delta_x, update.delta_y, update.delta_z) == (0, 0, 0)
    )));
    assert!(!respawned.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::EntityPositionSync(update),
        } if update.id == alice
    )));
}

#[test]
fn movement_resyncs_absolute_on_large_delta_and_ground_flip() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);

    // A 10-block step exceeds the i16 delta range (32767/4096 ~= 8 blocks):
    // ServerEntity.java:147-157 falls back to an absolute EntityPositionSync.
    // It is also exactly the `handleMovePlayer` budget (`:1103`), so it is the
    // largest step this session may take in one packet.
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    let target = Vec3d {
        x: start.x + 10.0,
        y: start.y,
        z: start.z,
    };
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: Some(target),
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            }),
        },
    )
    .unwrap();
    let teleported = world.tick(receiver.drain());
    // Bob's own sync rides along: his body found the floor the tick before
    // (see `settle_joined_bodies`).
    let position_syncs = teleported
        .events
        .iter()
        .filter(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::EntityPositionSync(update),
                    ..
                } if update.id == 1
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(position_syncs.len(), 1);
    assert!(matches!(
        position_syncs[0],
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::EntityPositionSync(update),
        } if update.id == 1
            && update.position.x == target.x
            && update.on_ground
    ));

    // An on-ground flip forces another absolute sync on Alice's next due call
    // (ServerEntity.java:120-152). The floor has to go for the flip to hold:
    // the tick's own travel puts a body that stands on one back on it.
    for dx in -1..=1 {
        for dz in -1..=1 {
            world.set_block(
                Java26BuiltinDimension::Overworld,
                BlockPos {
                    x: target.x.floor() as i32 + dx,
                    y: target.y.floor() as i32 - 1,
                    z: target.z.floor() as i32 + dz,
                },
                block_state_id("minecraft:air"),
                crate::fluid::SET_BLOCK_UPDATE_ALL,
            );
        }
    }
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: None,
                rotation: None,
                on_ground: false,
                horizontal_collision: false,
            }),
        },
    )
    .unwrap();
    let deferred = world.tick(receiver.drain());
    assert!(!deferred.events.iter().any(is_move_entity));
    let flipped = world.tick(Vec::new());
    assert_eq!(flipped.events.len(), 1);
    assert!(matches!(
        &flipped.events[0],
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::EntityPositionSync(update),
        } if update.id == 1 && !update.on_ground
    ));

    // The delta base reset with the sync: a later small step is relative again.
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    x: target.x + 0.5,
                    y: target.y,
                    z: target.z,
                }),
                rotation: None,
                on_ground: false,
                horizontal_collision: false,
            }),
        },
    )
    .unwrap();
    let deferred = world.tick(receiver.drain());
    assert!(!deferred.events.iter().any(is_move_entity));
    let relative = world.tick(Vec::new());
    let movement = relative
        .events
        .iter()
        .filter(|event| is_move_entity(event))
        .collect::<Vec<_>>();
    assert_eq!(movement.len(), 1);
    assert!(matches!(
        movement[0],
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet: ClientboundPlayPacket::MoveEntity(update),
        } if update.id == 1
            && update.has_position
            && (update.delta_x, update.delta_y, update.delta_z) == (2048, 0, 0)
    ));
}

#[test]
fn movement_forces_a_zero_delta_position_every_sixty_ticks() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    admit_players(&mut world, &[1, 2]);

    // Each ServerEntity counts its own calls from construction, so Alice's
    // 60th tickCount value lands at world tick 61 and Bob's at tick 63.
    for expected_tick in 5..61 {
        let idle = world.tick(Vec::new());
        assert_eq!(idle.tick, expected_tick);
        assert!(
            !idle.events.iter().any(is_move_entity),
            "tick {expected_tick} must not broadcast movement"
        );
    }
    // ServerEntity.java:141: tickCount % 60 == 0 forces each avatar's own
    // zero-delta position re-send, independent of the world tick.
    for (tick, expected) in [
        (61, Some((1, ConnectionId(2)))),
        (62, None),
        (63, Some((2, ConnectionId(1)))),
    ] {
        let result = world.tick(Vec::new());
        assert_eq!(result.tick, tick);
        let moves = result
            .events
            .iter()
            .filter(|event| is_move_entity(event))
            .collect::<Vec<_>>();
        let Some((entity_id, viewer)) = expected else {
            assert!(moves.is_empty(), "tick {tick}: {:?}", result.events);
            continue;
        };
        assert_eq!(moves.len(), 1, "tick {tick}: {:?}", result.events);
        let event = moves[0];
        assert!(matches!(
            event,
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::MoveEntity(update),
            } if *connection_id == viewer
                && update.id == entity_id
                && update.has_position
                && (update.delta_x, update.delta_y, update.delta_z) == (0, 0, 0)
        ));
    }
}

#[test]
fn teleport_load_and_movement_apply_in_command_sequence() {
    let (sender, receiver) = mailbox();
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
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    let joined = world.tick(receiver.drain());
    let teleport_id = match &joined.events[12] {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::PlayerPosition(position),
            ..
        } => position.id,
        other => panic!("expected player position, got {other:?}"),
    };

    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    x: 5.0,
                    y: 70.0,
                    z: 5.0,
                }),
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            }),
        },
    )
    .unwrap();
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::AcceptTeleportation { id: teleport_id },
        },
    )
    .unwrap();
    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::PlayerLoaded,
        },
    )
    .unwrap();
    let loaded = world.tick(receiver.drain());
    assert_eq!(loaded.snapshot.players[0].position.y, 64.0);
    assert!(loaded.snapshot.players[0].client_loaded);

    // A loaded session's own step applies.
    let start = loaded.snapshot.players[0].position;
    let stepped = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: start.x + 1.0,
                ..start
            }),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    assert_eq!(stepped.snapshot.players[0].position.x, start.x + 1.0);

    enqueue(
        &sender,
        WorldCommand::Play {
            connection_id: ConnectionId(1),
            packet: PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    x: f64::INFINITY,
                    y: 70.0,
                    z: -5.0,
                }),
                rotation: Some(PlayerRotation {
                    y_rot: 450.0,
                    x_rot: -370.0,
                }),
                on_ground: true,
                horizontal_collision: false,
            }),
        },
    )
    .unwrap();
    let moved = world.tick(receiver.drain());
    // The clamp turns the packet into a target tens of millions of blocks away,
    // which the tick-budget arm refuses (`:1103`): the position is the one the
    // session already had and the correction carries the rotation it already
    // had, because the packet's own rotation is only applied by an accepted
    // move (`:1146`).
    let correction = moved
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(*update),
            _ => None,
        })
        .expect("the far move is corrected");
    let player = &moved.snapshot.players[0];
    assert_eq!(player.position.x, start.x + 1.0);
    assert_eq!(correction.position, player.position);
    assert_eq!((correction.y_rot, correction.x_rot), (0.0, 0.0));
}

/// `ServerPlayer.forceSetRotation` (ServerPlayer.java:1138-1141): the server
/// stores the absolute rotation `Entity.forceSetRotation` computed
/// (Entity.java:3091-3100) and the client is sent the *unresolved* four values,
/// so it applies the same arithmetic to the rotation it is holding.
#[test]
fn rotate_stores_the_absolute_angle_and_sends_the_client_the_relative_one() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    let absolute = chat_command(&sender, &receiver, &mut world, 1, "rotate op 90 -20");
    assert_eq!(
        rotations(&absolute),
        vec![(90.0, false, -20.0, false)],
        "an absolute rotation travels as it was typed"
    );

    // `~10 ~` resolves against the *source* (yaw 90 now) and is handed over as
    // the difference from the *entity*'s own angle -- which for a player
    // rotating itself is a hundred minus ninety.
    let relative = chat_command(&sender, &receiver, &mut world, 1, "rotate op ~10 ~");
    assert_eq!(rotations(&relative), vec![(10.0, true, 0.0, true)]);
}

/// The four fields of every `ClientboundPlayerRotationPacket` a tick sent.
fn rotations(tick: &TickResult) -> Vec<(f32, bool, f32, bool)> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerRotation(update),
                ..
            } => Some((
                update.y_rot,
                update.relative_y,
                update.x_rot,
                update.relative_x,
            )),
            _ => None,
        })
        .collect()
}

/// `ServerPlayer.setCamera` (ServerPlayer.java:1916-1931): a camera that did
/// not move does nothing at all (`:1919`), and one that did teleports the
/// player onto it (`:1921`) before the packet naming it goes out (`:1928`).
#[test]
fn spectate_moves_the_camera_once_and_stopping_puts_it_back_on_the_player() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode spectator op");

    let started = chat_command(&sender, &receiver, &mut world, 1, "spectate bob");
    assert_eq!(cameras(&started).len(), 1, "{:?}", started.events);
    let camera = cameras(&started)[0];

    // `:1919`: the same target again is not a move, so nothing at all is sent.
    let again = chat_command(&sender, &receiver, &mut world, 1, "spectate bob");
    assert!(cameras(&again).is_empty(), "{:?}", again.events);

    // Stopping names the player's own entity, which is what `getCamera`
    // answers with when the field holds the player itself (`:1918`).
    let stopped = chat_command(&sender, &receiver, &mut world, 1, "spectate");
    assert_eq!(cameras(&stopped).len(), 1);
    assert_ne!(cameras(&stopped)[0], camera);
}

/// `ServerPlayer.doTick`'s camera half (ServerPlayer.java:586-597): a session
/// following a camera is snapped onto it every tick, and a camera that is no
/// longer alive puts the camera back on the player.
#[test]
fn a_spectating_session_follows_its_camera_and_is_released_when_it_leaves() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode spectator op");
    chat_command(&sender, &receiver, &mut world, 1, "spectate bob");
    // `setCamera` already put the spectator on the camera once (`:1921`), so
    // the camera has to move afterwards for the follow to be what is measured.
    chat_command(&sender, &receiver, &mut world, 1, "teleport bob 40 70 -12");

    let followed = world.tick(receiver.drain());
    let position = |name: &str| {
        followed
            .snapshot
            .players
            .iter()
            .find(|player| player.name == name)
            .map(|player| (player.position.x, player.position.y, player.position.z))
            .unwrap_or_else(|| panic!("{name} is online"))
    };
    assert_eq!(
        position("op"),
        position("bob"),
        "the spectator is snapped onto the camera every tick"
    );

    // `:594-596`: a camera that is no longer alive releases the session, which
    // is the same `setCamera(null)` `/spectate` with no target makes -- so the
    // packet naming the spectator's own entity goes out.
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(2),
        },
    )
    .unwrap();
    let released = world.tick(receiver.drain());
    assert_eq!(cameras(&released).len(), 1, "{:?}", released.events);
}

/// Every `ClientboundSetCameraPacket` entity id a tick sent.
fn cameras(tick: &TickResult) -> Vec<i32> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetCamera(update),
                ..
            } => Some(update.camera_id),
            _ => None,
        })
        .collect()
}

/// `ServerGamePacketListenerImpl.handleMovePlayer`'s moved-too-quickly arm
/// (`:1103-1106`): a step past `metersPerTick * deltaPackets` from the tick's
/// baseline leaves the position where the server had it and sends the client
/// back to it.
#[test]
fn a_move_further_than_the_tick_budget_is_sent_back() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (start, awaiting, first_good, last_good, received, known) =
        world.player_movement_validation_for_test(ConnectionId(1));
    assert_eq!(awaiting, None);
    assert_eq!(first_good, start, "`resetPosition` (`:372-377`)");
    assert_eq!(last_good, start);
    assert_eq!((received, known), (0, 0));

    // Two hundred blocks is 40_000 blocks squared against a budget of 100. The
    // packet also turns, which the refusal must leave unapplied: vanilla reads
    // the rotation the session already had (`:1105`).
    let jumped = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: start.x,
                y: start.y,
                z: start.z + 200.0,
            }),
            rotation: Some(PlayerRotation {
                y_rot: 90.0,
                x_rot: 12.0,
            }),
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    let correction = jumped
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(position),
                ..
            } => Some(*position),
            _ => None,
        })
        .expect("the too-quick move is corrected");
    assert_eq!(correction.position, start, "`this.teleport(getX(), ...)`");
    assert_eq!((correction.y_rot, correction.x_rot), (0.0, 0.0));
    let (after, awaiting, _, last_good, received, _) =
        world.player_movement_validation_for_test(ConnectionId(1));
    assert_eq!(after, start, "the position never moved");
    assert_eq!(awaiting, Some(correction.id), "the client owes an ack");
    assert_eq!(last_good, start);
    assert_eq!(received, 1, "the packet counted toward the rate");
}

/// `shouldCheckPlayerMovement` (`:1193-1204`) is gated on
/// `GameRules.PLAYER_MOVEMENT_CHECK`: with the rule off the same packet is
/// accepted.
#[test]
fn the_player_movement_check_rule_turns_the_refusal_off() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    assert!(world.set_game_rule_for_test(
        "minecraft:player_movement_check",
        bbb_protocol::game_rules::GameRuleValue::Boolean(false)
    ));
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    let jumped = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: start.x,
                y: start.y,
                z: start.z + 200.0,
            }),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    assert!(
        !jumped.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(_),
                ..
            }
        )),
        "the rule turns the arm off: {:?}",
        jumped.events
    );
    assert_eq!(
        world
            .player_movement_validation_for_test(ConnectionId(1))
            .0
            .z,
        start.z + 200.0
    );
}

/// The same arm is inside `level.tickRateManager().runsNormally()`
/// (`:1091-1109`), so a frozen tick rate accepts the packet without counting
/// it.
#[test]
fn a_frozen_tick_rate_skips_the_move_packet_check() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_tick_frozen(true, &mut Vec::new());
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    let jumped = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: start.x,
                y: start.y,
                z: start.z + 200.0,
            }),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    assert!(
        !jumped.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(_),
                ..
            }
        )),
        "{:?}",
        jumped.events
    );
    assert_eq!(
        world
            .player_movement_validation_for_test(ConnectionId(1))
            .0
            .z,
        start.z + 200.0
    );
}

/// `handleMovePlayer`'s passenger arm (`:1073-1075`): a session riding
/// something has its own position report ignored -- the vehicle owns where it
/// is -- and only the rotation lands.
#[test]
fn a_riders_position_report_is_ignored_and_only_its_rotation_lands() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (boat, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        world.player_movement_validation_for_test(ConnectionId(1)).0,
        crate::entity::EntityData::Boat(crate::entity::BoatData::new()),
    );
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    assert!(world.attach_external_player_for_test(ConnectionId(1), boat));
    world.tick(receiver.drain());
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    let ridden = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: start.x + 40.0,
                ..start
            }),
            rotation: Some(PlayerRotation {
                y_rot: 45.0,
                x_rot: -10.0,
            }),
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    assert!(
        !ridden.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(_),
                ..
            }
        )),
        "the rider is owed no correction: {:?}",
        ridden.events
    );
    assert_eq!(
        world.player_movement_validation_for_test(ConnectionId(1)).0,
        start,
        "`absSnapTo(getX(), ...)`"
    );
    let published = snapshot_player(&ridden, 1);
    assert_eq!(
        (published.rotation.y_rot, published.rotation.x_rot),
        (45.0, -10.0)
    );
    // The arm skips the packet's ground, and a passenger is never on it
    // (`Player.java:233-235`).
    assert!(!published.on_ground);
}

/// `handleMovePlayer` counts every packet the non-passenger arm reaches, and a
/// rotation-only one is among them: `packet.getX(player.getX())` (`:1070-1072`)
/// falls back to where the session already is, so the rate arm sees it.
#[test]
fn a_rotation_only_packet_counts_toward_the_move_packet_rate() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (start, _, _, _, received, _) = world.player_movement_validation_for_test(ConnectionId(1));
    let looked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: None,
            rotation: Some(PlayerRotation {
                y_rot: 90.0,
                x_rot: 0.0,
            }),
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    let (after, _, _, _, received_after, _) =
        world.player_movement_validation_for_test(ConnectionId(1));
    assert_eq!(after, start, "the position is the fallback, not a new one");
    assert_eq!(received_after, received + 1);
    assert_eq!(snapshot_player(&looked, 1).rotation.y_rot, 90.0);
}

/// `:1131-1140`: a target the server's own collision solve refuses -- the wall
/// between the session and where the client says it is -- is corrected back to
/// the position the packet started from, with the rotation the packet asked
/// for (`:1180`).
#[test]
fn a_move_through_a_wall_is_corrected_to_where_the_packet_started() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    // Creative and spectator are exempt from moved-wrongly (`:1135-1136`).
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    let wall = BlockPos {
        x: start.x.floor() as i32 + 1,
        y: start.y.floor() as i32,
        z: start.z.floor() as i32,
    };
    for y in [wall.y, wall.y + 1] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { y, ..wall },
            block_state_id("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }

    // The client claims a position on the far side of the wall.
    let walked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: start.x + 3.0,
                ..start
            }),
            rotation: Some(PlayerRotation {
                y_rot: 45.0,
                x_rot: 0.0,
            }),
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    let correction = walked
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(position),
                ..
            } => Some(*position),
            _ => None,
        })
        .expect("the wall refuses the move");
    assert_eq!(correction.position, start);
    assert_eq!((correction.y_rot, correction.x_rot), (45.0, 0.0));
    assert_eq!(
        world.player_movement_validation_for_test(ConnectionId(1)).0,
        start
    );
    assert_eq!(
        world.player_movement_validation_for_test(ConnectionId(1)).1,
        Some(correction.id),
        "the client owes an ack"
    );
}

/// The same target is accepted in creative: `:1135-1136` exempts the two game
/// modes from the moved-wrongly flag.
#[test]
fn a_creative_client_may_report_a_position_the_solve_refuses() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    let wall = BlockPos {
        x: start.x.floor() as i32 + 1,
        y: start.y.floor() as i32,
        z: start.z.floor() as i32,
    };
    for y in [wall.y, wall.y + 1] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { y, ..wall },
            block_state_id("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }
    let target = Vec3d {
        x: start.x + 3.0,
        ..start
    };
    let walked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(target),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    assert!(
        !walked.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(_),
                ..
            }
        )),
        "{:?}",
        walked.events
    );
    assert_eq!(
        world.player_movement_validation_for_test(ConnectionId(1)).0,
        target
    );
}

/// `:1142-1145`: a session whose own box is already inside geometry may still
/// move, but not into geometry its old box did not touch.
#[test]
fn a_stuck_session_may_leave_but_not_into_new_geometry() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    let feet = BlockPos {
        x: start.x.floor() as i32,
        y: start.y.floor() as i32,
        z: start.z.floor() as i32,
    };
    for y in [feet.y, feet.y + 1] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { y, ..feet },
            block_state_id("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }

    // The target is clear of the block the session is stuck in.
    let clear = Vec3d {
        x: start.x + 3.0,
        ..start
    };
    let walked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(clear),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    assert!(
        !walked.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(_),
                ..
            }
        )),
        "`!fail || !level.noCollision` forgives the stuck session: {:?}",
        walked.events
    );
    assert_eq!(
        world.player_movement_validation_for_test(ConnectionId(1)).0,
        clear
    );
}

/// `isEntityCollidingWithAnythingNew` (`:1216-1233`) is asked whatever the
/// moved-wrongly flag says, so even a stuck session cannot move into a block
/// its old box never touched.
#[test]
fn a_stuck_session_cannot_move_into_new_geometry() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    let feet = BlockPos {
        x: start.x.floor() as i32,
        y: start.y.floor() as i32,
        z: start.z.floor() as i32,
    };
    let ahead = BlockPos {
        x: feet.x + 3,
        ..feet
    };
    for pos in [
        feet,
        BlockPos {
            y: feet.y + 1,
            ..feet
        },
        ahead,
        BlockPos {
            y: ahead.y + 1,
            ..ahead
        },
    ] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            block_state_id("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }
    let target = Vec3d {
        x: start.x + 3.0,
        ..start
    };
    let walked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(target),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    let correction = walked
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(position),
                ..
            } => Some(*position),
            _ => None,
        })
        .expect("the block ahead is new geometry");
    assert_eq!(correction.position, start);
    assert_eq!(
        world.player_movement_validation_for_test(ConnectionId(1)).0,
        start
    );
}

/// `:318-327`: a session that keeps reporting a position with nothing under it
/// is disconnected once its gravity-scaled allowance runs out
/// (`getMaximumFlyingTicks`, `:356-364`).
#[test]
fn a_hovering_client_is_kicked_after_the_gravity_allowance() {
    // `:319` kicks on the 81st consecutive floating tick, and the tick the
    // declaring packet arrived in is the first of them.
    assert_eq!(hovering_kick_tick(creative_fixture_config()), Some(79));
}

/// `allow-flight` removes `!server.allowFlight()` from the floating conjuncts
/// (`:1151`), so a hovering survival session is never kicked.
#[test]
fn allow_flight_keeps_a_hovering_client_connected() {
    let config = ServerWorldConfig {
        allow_flight: true,
        ..creative_fixture_config()
    };
    assert_eq!(hovering_kick_tick(config), None);
}

/// `:1147,1155`: a riptide spin is not floating, so the hover never counts.
#[test]
fn a_spinning_client_is_not_floating() {
    assert_eq!(
        hovering_kick_tick_with(creative_fixture_config(), true),
        None
    );
}

/// `:1073` snaps any passenger to its own position and `:318` skips the
/// floating kick for every passenger, steering or not: a player riding a
/// hovering mob it does not control is never kicked.
#[test]
fn a_non_steering_passenger_of_a_hovering_mob_is_not_kicked() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    let (x, y, z) = (start.x, start.y + 40.0, start.z);
    for (id, command) in [
        format!("summon minecraft:zombie {x} {y} {z} {{NoGravity:1b,NoAI:1b,Tags:[\"seat\"]}}"),
        "ride alice mount @e[tag=seat,limit=1]".to_string(),
    ]
    .into_iter()
    .enumerate()
    {
        let tick =
            crate::tests::console_command(&sender, &receiver, &mut world, 10 + id as u64, &command);
        assert!(
            crate::tests::console_command_answer(&tick).0.is_some(),
            "{command}: {:?}",
            crate::tests::console_command_answer(&tick)
        );
        // The mount's rider correction must be answered before moves count.
        if let Some(teleport_id) = tick.events.iter().find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        }) {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::AcceptTeleportation { id: teleport_id },
            );
        }
    }
    let mut kicked = false;
    for tick in 0..120 {
        // A seat a hair above where the vehicle holds the rider.
        let at = world.player_movement_validation_for_test(ConnectionId(1)).0;
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    y: at.y + 0.01,
                    ..at
                }),
                rotation: Some(bbb_protocol::packets::PlayerRotation {
                    y_rot: tick as f32,
                    x_rot: 0.0,
                }),
                on_ground: false,
                horizontal_collision: false,
            }),
        );
        let ticked = world.tick(receiver.drain());
        kicked |= ticked.events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::Close {
                    reason: WorldCloseReason::Floating,
                    ..
                }
            )
        });
    }
    assert!(!kicked);
}

fn hovering_kick_tick(config: ServerWorldConfig) -> Option<usize> {
    hovering_kick_tick_with(config, false)
}

fn hovering_kick_tick_with(config: ServerWorldConfig, spinning: bool) -> Option<usize> {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    // A creative session may fly, and `!getAbilities().mayfly` is one of the
    // floating conjuncts (`:1152`).
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
    // A fixture puts the session in the air; the move below is what declares it
    // floating (`:1148-1156`).
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: start.x,
            y: start.y + 40.0,
            z: start.z,
        },
    );
    // The tick's own travel finds nothing under the raised body, which is what
    // `playerStandsOnSomething` reads next (`Entity.java:757-761`).
    world.tick(Vec::new());
    if spinning {
        world.start_player_auto_spin_attack_for_test(ConnectionId(1), 20);
    }
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
            on_ground: false,
            horizontal_collision: false,
        }),
    );

    let mut kicked_at = None;
    for tick in 0..100 {
        let ticked = world.tick(receiver.drain());
        if ticked.events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::Close {
                    reason: WorldCloseReason::Floating,
                    ..
                }
            )
        }) {
            kicked_at = Some(tick);
            break;
        }
    }
    kicked_at
}

mod piston_cursor;

/// `ServerPlayerGameMode.changeGameModeForPlayer` (`ServerPlayerGameMode.java:61-63`):
/// leaving spectator within a block of the ground lands the flight; higher up it is kept.
#[test]
fn leaving_spectator_for_creative_near_the_ground_stops_flying() {
    let mut world = ServerWorld::new(operator_config(&[("op", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "op", 1);

    chat_command(&sender, &receiver, &mut world, 1, "gamemode spectator op");
    assert!(world.player_abilities_for_test(ConnectionId(1)).flying);
    chat_command(&sender, &receiver, &mut world, 1, "gamemode creative op");
    assert!(!world.player_abilities_for_test(ConnectionId(1)).flying);

    chat_command(&sender, &receiver, &mut world, 1, "gamemode spectator op");
    chat_command(&sender, &receiver, &mut world, 1, "teleport op ~ ~10 ~");
    chat_command(&sender, &receiver, &mut world, 1, "gamemode creative op");
    assert!(world.player_abilities_for_test(ConnectionId(1)).flying);
}
