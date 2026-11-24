use super::*;

use crate::entity::{BoatData, EntityData};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;

fn oak_boat(world: &mut ServerWorld, position: Vec3d) -> i32 {
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            position,
            EntityData::Boat(BoatData::new()),
        )
        .0
}

fn interact(boat: i32, secondary: bool) -> PlayServerbound {
    PlayServerbound::Interact(InteractEntity {
        entity_id: boat,
        hand: InteractionHand::MainHand,
        location: Vec3d::default(),
        using_secondary_action: secondary,
    })
}

#[test]
fn external_player_vehicle_control_mounts_in_packet_order_and_drives_after_listener_tick() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());

    let mounted = play(&sender, &receiver, &mut world, 1, interact(boat, false));
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(boat),
        "mount events: {:#?}",
        mounted.events
    );
    let position_index = mounted
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::PlayerPosition(_)
                }
            )
        })
        .expect("mount sends the rider correction");
    let passengers_index = mounted
        .events
        .iter()
        .position(|event| matches!(event, WorldEvent::Send { packet: ClientboundPlayPacket::SetPassengers(update), .. } if update.vehicle_id == boat))
        .expect("mount publishes the relation");
    assert!(position_index < passengers_index);
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(boat)
    );

    let mount_teleport = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: mount_teleport },
    );
    let (last_vehicle, _, baseline, _) =
        world.external_player_vehicle_listener_for_test(ConnectionId(1));
    assert_eq!(last_vehicle, Some(boat));
    let target = Vec3d {
        x: baseline.x + 0.25,
        ..baseline
    };
    let moved = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: target,
            y_rot: 30.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(!moved.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::MoveVehicle(_),
            ..
        }
    )));
    let (position, y_rot, _, _) =
        world.boat_control_state_for_test(Java26BuiltinDimension::Overworld, boat);
    assert_eq!(position, target);
    assert_eq!(y_rot, 30.0);

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PaddleBoat(PaddleBoat {
            left: true,
            right: false,
        }),
    );
    assert_eq!(
        world
            .boat_control_state_for_test(Java26BuiltinDimension::Overworld, boat)
            .3,
        (true, false)
    );
}

#[test]
fn external_player_vehicle_control_just_mounted_move_is_ignored_and_nan_disconnects() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    let before = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
        .unwrap()
        .position();
    for packet in [
        interact(boat, false),
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: before.x + 1.0,
                ..before
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    ] {
        enqueue(
            &sender,
            WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet,
            },
        )
        .unwrap();
    }
    world.tick(receiver.drain());
    assert_ne!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
            .unwrap()
            .position()
            .x,
        before.x + 1.0
    );

    let invalid = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: f64::NAN,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(invalid.events.iter().any(|event| matches!(
        event,
        WorldEvent::Close {
            connection_id: ConnectionId(1),
            reason: WorldCloseReason::InvalidVehicleMovement,
        }
    )));
}

#[test]
fn external_player_vehicle_control_refuses_secondary_cooldown_and_spectator() {
    for refusal in [
        "secondary",
        "cooldown",
        "spectator",
        "range",
        "water",
        "border",
    ] {
        let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let player_position = world.publication().players[0].position;
        let boat = oak_boat(
            &mut world,
            Vec3d {
                y: player_position.y + 2.0,
                ..player_position
            },
        );
        match refusal {
            "cooldown" => world.set_player_boarding_cooldown_for_test(ConnectionId(1), 20),
            "spectator" => world.set_player_mode_and_abilities_for_test(
                ConnectionId(1),
                GameType::Spectator,
                PlayerAbilities::for_game_type(GameType::Spectator),
            ),
            "border" => world.set_world_border_for_test(
                Java26BuiltinDimension::Overworld,
                bbb_server_types::interaction::WorldBorder::from_settings(
                    100.0, 100.0, 0.2, 5.0, 5, 15, 10.0, 0, 10.0, 0,
                ),
            ),
            "range" => world.set_stored_entity_position_for_test(
                Java26BuiltinDimension::Overworld,
                boat,
                Vec3d {
                    x: player_position.x + 20.0,
                    ..player_position
                },
            ),
            "water" => {
                let state =
                    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
                        "minecraft:water",
                        &BTreeMap::from([("level".to_string(), "0".to_string())]),
                    )
                    .unwrap();
                let eye = bbb_server_types::interaction::player_eye_position(player_position)
                    .expect("the joined player has finite placement");
                assert!(world.set_block(
                    Java26BuiltinDimension::Overworld,
                    BlockPos {
                        x: eye.x.floor() as i32,
                        y: eye.y.floor() as i32,
                        z: eye.z.floor() as i32,
                    },
                    state,
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                ));
            }
            _ => {}
        }
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact(boat, refusal == "secondary"),
        );
        assert_eq!(
            world
                .external_player_vehicle_state_for_test(ConnectionId(1))
                .2,
            None,
            "{refusal}"
        );
    }
}

#[test]
fn external_player_vehicle_control_enforces_capacity_and_corrects_too_quick_motion() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    for (connection_id, name) in [(1, "alice"), (2, "bob"), (3, "carol")] {
        join_loaded(
            &sender,
            &receiver,
            &mut world,
            connection_id,
            name,
            connection_id as u128,
        );
    }
    let player_position = world.publication().players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    let mut first_mount_teleport = None;
    for connection_id in [1, 2, 3] {
        let tick = play(
            &sender,
            &receiver,
            &mut world,
            connection_id,
            interact(boat, false),
        );
        if connection_id == 1 {
            first_mount_teleport = tick.events.iter().find_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::PlayerPosition(update),
                } => Some(update.id),
                _ => None,
            });
        }
    }
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(boat)
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(2))
            .2,
        Some(boat)
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(3))
            .2,
        None
    );

    let controlled_vehicle = world
        .external_player_vehicle_listener_for_test(ConnectionId(1))
        .0
        .expect("first rider controls the Boat");
    assert_eq!(controlled_vehicle, boat);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation {
            id: first_mount_teleport.expect("first mount sends a position correction"),
        },
    );
    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    let corrected = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: baseline.x + 11.0,
                ..baseline
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(corrected.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::MoveVehicle(_),
        }
    )));
}

#[test]
fn external_player_vehicle_control_keeps_carrier_tick_and_rider_placement_while_frozen() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: player_position.y + 4.0,
            ..player_position
        },
    );
    let mounted = play(&sender, &receiver, &mut world, 1, interact(boat, false));
    let teleport_id = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    world.tick_rate_mut().set_frozen(true);
    let before_tick_count = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
        .unwrap()
        .tick_count();
    let mut frozen = world.tick(receiver.drain());
    for _ in 0..3 {
        frozen = world.tick(receiver.drain());
    }
    let after_tick_count = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
        .unwrap()
        .tick_count();
    assert_eq!(
        after_tick_count - before_tick_count,
        4,
        "the Player carrier is exempt from entity freeze"
    );
    assert_eq!(
        frozen.snapshot.players[0].position,
        world
            .external_player_vehicle_placement_for_test(ConnectionId(1))
            .unwrap()
            .position
    );
}

#[test]
fn external_player_vehicle_control_corrects_new_block_entity_player_and_border_collisions() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    let mounted = play(&sender, &receiver, &mut world, 1, interact(boat, false));
    let teleport_id = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );

    let correction_sent = |tick: &TickResult| {
        tick.events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::MoveVehicle(_),
                }
            )
        })
    };
    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    let block = BlockPos {
        x: (baseline.x + 1.5).floor() as i32,
        y: baseline.y.floor() as i32,
        z: baseline.z.floor() as i32,
    };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        block,
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let blocked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: baseline.x + 1.5,
                ..baseline
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(correction_sent(&blocked));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        block,
        block_state_id("minecraft:air"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    let blocker = oak_boat(
        &mut world,
        Vec3d {
            x: baseline.x + 1.5,
            ..baseline
        },
    );
    let entity_blocked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: baseline.x + 0.5,
                ..baseline
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(correction_sent(&entity_blocked));
    assert!(world.remove_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        blocker,
        crate::entity::RemovalReason::Discarded,
    ));

    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: baseline.x + 1.1,
                ..baseline
            }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    let player_blocked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: baseline.x + 0.3,
                ..baseline
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(correction_sent(&player_blocked));

    play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: baseline.x + 20.0,
                ..baseline
            }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    world.set_world_border_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_server_types::interaction::WorldBorder::from_settings(
            baseline.x, baseline.z, 0.2, 5.0, 5, 15, 3.0, 0, 3.0, 0,
        ),
    );
    let border_blocked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: baseline.x + 0.9,
                ..baseline
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(correction_sent(&border_blocked));
}

#[test]
fn external_player_vehicle_control_non_air_without_collision_prevents_floating() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    let mounted = play(&sender, &receiver, &mut world, 1, interact(boat, false));
    let teleport_id = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    let target = Vec3d {
        x: baseline.x + 0.25,
        ..baseline
    };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: target.x.floor() as i32,
            y: (target.y - 0.1).floor() as i32,
            z: target.z.floor() as i32,
        },
        block_state_id("minecraft:short_grass"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    let moved = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: target,
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(!moved.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::MoveVehicle(_),
            ..
        }
    )));
    assert!(
        !world
            .external_player_vehicle_listener_for_test(ConnectionId(1))
            .3,
        "short grass is non-air even though it has no collision box"
    );
}

#[test]
fn external_player_vehicle_control_disconnects_after_gravity_scaled_floating_limit() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    let mounted = play(&sender, &receiver, &mut world, 1, interact(boat, false));
    let teleport_id = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: baseline.x + 0.1,
                ..baseline
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(
        world
            .external_player_vehicle_listener_for_test(ConnectionId(1))
            .3,
        "unsupported Boat movement establishes the production floating fact"
    );

    // Boat gravity is 0.04, so ceil(80 * max(0.08 / 0.04, 1)) is 160.
    world.set_external_player_vehicle_floating_ticks_for_test(ConnectionId(1), 160);
    let timed_out = world.tick(receiver.drain());
    let reason = timed_out.events.iter().find_map(|event| match event {
        WorldEvent::Close {
            connection_id: ConnectionId(1),
            reason,
        } => Some(reason),
        _ => None,
    });
    assert_eq!(reason, Some(&WorldCloseReason::Floating));
    assert_eq!(
        reason
            .and_then(WorldCloseReason::reason_component)
            .map(|component| component.text().to_owned())
            .as_deref(),
        Some("multiplayer.disconnect.flying")
    );
    assert!(timed_out.snapshot.players.is_empty());
}

#[test]
fn external_player_vehicle_control_same_vehicle_passenger_is_not_a_collision() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let (boat, passenger) = world
        .spawn_boat_with_stored_passenger_for_test(
            Java26BuiltinDimension::Overworld,
            Vec3d {
                y: player_position.y + 2.0,
                ..player_position
            },
        )
        .expect("the saved-tree loader publishes a Boat passenger tree");
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, passenger)
        .is_some());
    let mounted = play(&sender, &receiver, &mut world, 1, interact(boat, false));
    let teleport_id = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    let target = Vec3d {
        x: baseline.x + 0.25,
        ..baseline
    };
    let moved = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: target,
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(!moved.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::MoveVehicle(_),
            ..
        }
    )));
    assert_eq!(
        world
            .boat_control_state_for_test(Java26BuiltinDimension::Overworld, boat)
            .0,
        target
    );
    assert_eq!(
        world.external_player_boat_distance_stat_for_test(ConnectionId(1)),
        25
    );
}

#[test]
fn external_player_vehicle_control_gates_unloaded_awaiting_and_wrong_identity_but_clamps_infinity()
{
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
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
    let joined = world.tick(receiver.drain());
    let player_position = joined.snapshot.players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    play(&sender, &receiver, &mut world, 1, interact(boat, false));
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        None,
        "an unloaded play session cannot mount"
    );

    let initial_teleport = joined
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation {
            id: initial_teleport,
        },
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerLoaded,
    );
    let mounted = play(&sender, &receiver, &mut world, 1, interact(boat, false));
    let mount_teleport = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    let before = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
        .unwrap()
        .position();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: before.x + 0.25,
                ..before
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert_eq!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
            .unwrap()
            .position(),
        before,
        "an outstanding mount teleport gates vehicle movement"
    );

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: mount_teleport },
    );
    let other_boat = oak_boat(
        &mut world,
        Vec3d {
            x: before.x + 4.0,
            ..before
        },
    );
    world.set_external_player_last_vehicle_for_test(ConnectionId(1), Some(other_boat));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: before.x + 0.25,
                ..before
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert_eq!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
            .unwrap()
            .position(),
        before,
        "the typed kernel rejects a stale expected vehicle identity"
    );

    world.set_external_player_last_vehicle_for_test(ConnectionId(1), Some(boat));
    let infinity = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: f64::INFINITY,
                ..before
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    assert!(!infinity.events.iter().any(|event| matches!(
        event,
        WorldEvent::Close {
            reason: WorldCloseReason::InvalidVehicleMovement,
            ..
        }
    )));
    assert!(infinity.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::MoveVehicle(_),
        }
    )));
}

#[test]
fn external_player_vehicle_control_refuses_live_out_of_control_boat() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let player_position = world.publication().players[0].position;
    let boat_position = Vec3d {
        y: player_position.y + 3.0,
        ..player_position
    };
    let boat = oak_boat(&mut world, boat_position);
    let mounted = play(&sender, &receiver, &mut world, 1, interact(boat, false));
    let teleport_id = mounted
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(update),
                ..
            } => Some(update.id),
            _ => None,
        })
        .unwrap();
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    let water = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &BTreeMap::from([("level".to_string(), "0".to_string())]),
    )
    .unwrap();
    let center = BlockPos {
        x: boat_position.x.floor() as i32,
        y: boat_position.y.floor() as i32,
        z: boat_position.z.floor() as i32,
    };
    for x in center.x - 1..=center.x + 1 {
        for y in center.y..=center.y + 4 {
            for z in center.z - 1..=center.z + 1 {
                assert!(world.set_block(
                    Java26BuiltinDimension::Overworld,
                    BlockPos { x, y, z },
                    water,
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                ));
            }
        }
    }
    let mut ejected = Vec::new();
    let mut ejection_start = None;
    for _ in 0..80 {
        let before = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
            .unwrap();
        let before_top = before.bounding_box().max_y;
        let before_block = before.block_position();
        ejected.extend(world.tick(receiver.drain()).events);
        if world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2
            .is_none()
        {
            ejection_start = Some((before_top, before_block));
            break;
        }
    }
    assert!(
        world.boat_out_of_control_ticks_for_test(Java26BuiltinDimension::Overworld, boat) >= 60.0
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        None,
        "the underwater timeout ejects the existing controller"
    );

    // `AbstractBoat.ejectPassengers` reaches `LivingEntity.stopRiding`
    // (`AbstractBoat.java:218`, `Entity.java:2397-2400`), so the rider is moved
    // to `getDismountLocationForPassenger`'s answer. Water under the escape
    // point takes the `super` arm (`AbstractBoat.java:636`, `:658`,
    // `Entity.java:3532-3534`).
    let (boat_position, boat_bounds, boat_block) = {
        let entity = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
            .expect("the boat");
        (
            entity.position(),
            entity.bounding_box(),
            entity.block_position(),
        )
    };
    let landing = world.publication().players[0].position;
    assert_eq!([landing.x, landing.z], [boat_position.x, boat_position.z]);
    let (before_top, before_block) = ejection_start.expect("the boat ejected its rider");
    assert!(
        (before_top.min(boat_bounds.max_y)..=before_top.max(boat_bounds.max_y))
            .contains(&landing.y),
        "the timeout eject lands the rider above the boat's movement during this tick"
    );
    let player_entity_id = world.player_entity_id_for_test(ConnectionId(1));
    let (raised, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(
        raised.iter().any(|event| {
            event.game_event == crate::level_event::VanillaGameEvent::ENTITY_DISMOUNT
                && [before_block, boat_block].contains(&event.pos)
                && event.source_entity_id == Some(player_entity_id)
        }),
        "the timeout eject raises `ENTITY_DISMOUNT` at the boat"
    );
    assert!(
        ejected.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(packet),
                ..
            } if packet.position == landing
        )),
        "the timeout eject teleports the rider to the landing"
    );

    play(&sender, &receiver, &mut world, 2, interact(boat, false));
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(2))
            .2,
        None,
        "the live out-of-control fact rejects a new controller"
    );
}

#[test]
fn an_unanswered_mount_teleport_is_resent_from_the_vehicle_move_path() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    play(&sender, &receiver, &mut world, 1, interact(boat, false));
    let move_vehicle = || {
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: player_position,
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        })
    };
    let resent = |events: &[WorldEvent]| {
        events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::PlayerPosition(_),
                }
            )
        })
    };
    let early = play(&sender, &receiver, &mut world, 1, move_vehicle());
    assert!(!resent(&early.events));
    for _ in 0..21 {
        world.tick(receiver.drain());
    }
    let late = play(&sender, &receiver, &mut world, 1, move_vehicle());
    assert!(resent(&late.events));
}

/// LivingEntity.java:3226-3228: every ride tick clears the rider's fall
/// distance, so a fall started before mounting never lands as damage.
#[test]
fn riding_a_boat_resets_the_riders_fall_distance() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    play(&sender, &receiver, &mut world, 1, interact(boat, false));
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        Some(boat)
    );
    world.set_player_fall_distance_for_test(ConnectionId(1), 9.0);
    world.tick(receiver.drain());
    assert_eq!(world.player_fall_distance_for_test(ConnectionId(1)), 0.0);
}
