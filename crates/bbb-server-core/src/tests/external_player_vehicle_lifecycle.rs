use super::*;

use crate::entity::{BoatData, EntityData, ExternalPlayerCanRide};
use crate::tests::entities::console_command_answer;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;
use bbb_protocol::packets::PlayerInput;

use crate::world::{SPAWN_X, SPAWN_Y, SPAWN_Z};

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

#[test]
fn external_player_vehicle_lifecycle_cooldown_is_player_owned_and_keeps_tracker_events() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 2);

    let boat = oak_boat(
        &mut world,
        Vec3d {
            x: SPAWN_X,
            y: SPAWN_Y,
            z: SPAWN_Z,
        },
    );
    let mut events = Vec::new();
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut events);
    assert!(world.attach_external_player_for_test(ConnectionId(1), boat));
    events.clear();
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut events);
    assert_eq!(
        world.detach_external_player_for_test(ConnectionId(1)),
        Some(boat)
    );
    events.clear();
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut events);

    let (cooldown, admission, vehicle, root) =
        world.external_player_vehicle_state_for_test(ConnectionId(1));
    assert_eq!(cooldown, 0);
    assert_eq!(admission, ExternalPlayerCanRide::Allowed);
    assert_eq!((vehicle, root), (None, None));
    assert!(events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetPassengers(packet),
            ..
        } if packet.vehicle_id == boat && packet.passenger_ids.is_empty()
    )));

    world.set_player_boarding_cooldown_for_test(ConnectionId(1), 2);
    world.tick_rate_mut().set_frozen(true);
    world.tick(receiver.drain());
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .0,
        1
    );
    let mut transition_events = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 0.5,
                y: 70.0,
                z: 0.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut transition_events,
    ));
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .0,
        0
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .1,
        ExternalPlayerCanRide::Allowed
    );
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
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .1,
        ExternalPlayerCanRide::Denied
    );
}

#[test]
fn external_player_vehicle_lifecycle_disconnect_freezes_and_rejoins_full_snapshot() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let unmounted = world
        .save_request(1_786_000_000_000, 0, RegionCompression::Zlib)
        .unwrap();
    assert!(unmounted.player_patches[0].root_vehicle.is_none());
    let latest_position = Vec3d {
        x: 4.5,
        y: 70.0,
        z: 5.5,
    };
    place_player(&sender, &receiver, &mut world, 1, latest_position, None);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCarriedItem { slot: 3 },
    );
    world.set_player_tags_for_test(ConnectionId(1), vec!["latest".into()]);
    let boat = oak_boat(&mut world, latest_position);
    let player_entity_id = world.player_entity_id_for_test(ConnectionId(1));
    assert!(world.attach_external_player_for_test(ConnectionId(1), boat));

    let online = world
        .save_request(1_786_000_000_001, 1, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(online.player_patches.len(), 1);
    assert!(online.player_patches[0].root_vehicle.is_some());

    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    let (raised, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(
        raised
            .iter()
            .filter(|event| {
                event.game_event == VanillaGameEvent::ENTITY_DISMOUNT
                    && event.source_entity_id == Some(player_entity_id)
            })
            .count(),
        1,
        "disconnect's unRide raises one dismount before unloading the boat"
    );
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
        .is_none());
    let departed = world
        .save_request(1_786_000_000_002, 2, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(departed.player_patches.len(), 1);
    assert_eq!(departed.player_patches[0].position, [4.5, 70.0, 5.5]);
    assert_eq!(departed.player_patches[0].tags, vec!["latest"]);
    assert_eq!(departed.player_patches[0].created.selected_item_slot, 3);
    assert!(departed.player_patches[0].root_vehicle.is_some());
    assert!(departed.departed_player_stats.is_empty());

    join_loaded(&sender, &receiver, &mut world, 2, "alice", 1);
    let snapshot = world.tick(receiver.drain()).snapshot;
    assert_eq!(
        snapshot.players[0].position,
        world
            .external_player_vehicle_placement_for_test(ConnectionId(2))
            .expect("the restored rider has live placement")
            .position
    );
    assert_eq!(
        world.player_tags_for_test(ConnectionId(2)),
        &["latest".to_string()]
    );
    let (_, _, vehicle, root) = world.external_player_vehicle_state_for_test(ConnectionId(2));
    assert!(vehicle.is_some());
    assert!(root.is_some());
    let rejoined = world
        .save_request(1_786_000_000_003, 3, RegionCompression::Zlib)
        .unwrap();
    assert_eq!(rejoined.player_patches.len(), 1);
    assert_eq!(rejoined.player_patches[0].created.selected_item_slot, 3);
    assert!(rejoined.player_patches[0].root_vehicle.is_some());
    assert!(rejoined.departed_player_stats.is_empty());
}

#[test]
fn external_player_vehicle_lifecycle_shared_root_detaches_then_sole_root_unloads() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let boat = oak_boat(
        &mut world,
        Vec3d {
            x: SPAWN_X,
            y: SPAWN_Y,
            z: SPAWN_Z,
        },
    );
    assert!(world.attach_external_player_for_test(ConnectionId(1), boat));
    assert!(world.attach_external_player_for_test(ConnectionId(2), boat));

    for connection_id in [ConnectionId(1), ConnectionId(2)] {
        enqueue(&sender, WorldCommand::Disconnect { connection_id }).unwrap();
        world.tick(receiver.drain());
        if connection_id == ConnectionId(1) {
            assert!(world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
                .is_some());
            assert_eq!(
                world
                    .external_player_vehicle_state_for_test(ConnectionId(2))
                    .2,
                Some(boat)
            );
        }
    }
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
        .is_none());
}

#[test]
fn external_player_vehicle_lifecycle_teleport_and_death_detach_before_body_moves() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let position = Vec3d {
        x: SPAWN_X,
        y: SPAWN_Y,
        z: SPAWN_Z,
    };
    let first_boat = oak_boat(&mut world, position);
    let first_player = world.player_entity_id_for_test(ConnectionId(1));
    assert!(world.attach_external_player_for_test(ConnectionId(1), first_boat));
    let mut teleport_events = Vec::new();
    assert!(world.teleport_player(
        ConnectionId(1),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position,
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut teleport_events,
    ));
    let (old_level_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(
        old_level_events
            .iter()
            .filter(|event| {
                event.game_event == VanillaGameEvent::ENTITY_DISMOUNT
                    && event.source_entity_id == Some(first_player)
            })
            .count(),
        1,
        "cross-level teleport raises one dismount in the old level"
    );
    assert_eq!(
        world.external_player_relation_in_dimension_for_test(
            Java26BuiltinDimension::Overworld,
            first_player,
        ),
        (None, None)
    );
    assert_eq!(
        world.external_player_vehicle_state_for_test(ConnectionId(1)),
        (0, ExternalPlayerCanRide::Allowed, None, None)
    );
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, first_boat)
        .is_some());

    world.reset_pending_chunks_for_test(ConnectionId(1), &[]);
    let second_boat = world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Nether,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            position,
            EntityData::Boat(BoatData::new()),
        )
        .0;
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Nether, &mut Vec::new());
    assert!(world.attach_external_player_for_test(ConnectionId(1), second_boat));
    world.tick(receiver.drain());
    world.prime_player_body_removal_for_test(ConnectionId(1));
    let died = world.tick(receiver.drain());
    let (death_level_events, _) = world.game_events(Java26BuiltinDimension::Nether);
    assert_eq!(
        death_level_events
            .iter()
            .filter(|event| {
                event.game_event == VanillaGameEvent::ENTITY_DISMOUNT
                    && event.source_entity_id == Some(first_player)
            })
            .count(),
        1,
        "body removal raises one dismount at the vehicle"
    );
    let poof = died
        .events
        .iter()
        .position(|event| {
            matches!(event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityEvent(update),
                ..
            } if update.entity_id == first_player && update.event_id == 60)
        })
        .expect("the mounted body poofs");
    let detached = died
        .events
        .iter()
        .position(|event| {
            matches!(event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetPassengers(update),
                ..
            } if update.vehicle_id == second_boat && update.passenger_ids.is_empty())
        })
        .expect("the vehicle publishes its empty passenger list");
    assert!(poof < detached, "{:#?}", died.events);
    assert_eq!(
        world.external_player_vehicle_state_for_test(ConnectionId(1)),
        (0, ExternalPlayerCanRide::Allowed, None, None)
    );
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Nether, second_boat)
        .is_some());
    world.set_player_boarding_cooldown_for_test(ConnectionId(1), 7);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    assert_eq!(
        world.external_player_vehicle_state_for_test(ConnectionId(1)),
        (0, ExternalPlayerCanRide::Allowed, None, None)
    );
}

#[test]
fn external_player_vehicle_lifecycle_shift_input_dismounts_on_the_real_player_tick() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let boat = oak_boat(
        &mut world,
        Vec3d {
            x: SPAWN_X,
            y: SPAWN_Y,
            z: SPAWN_Z,
        },
    );
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    assert!(world.attach_external_player_for_test(ConnectionId(1), boat));
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    let player_entity_id = world.player_entity_id_for_test(ConnectionId(1));
    world.tick_rate_mut().set_frozen(true);

    let stopped = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput {
            shift: true,
            ..PlayerInput::default()
        }),
    );
    assert_eq!(
        world.external_player_vehicle_state_for_test(ConnectionId(1)),
        (0, ExternalPlayerCanRide::Allowed, None, None)
    );
    assert!(!world.player_shift_key_down_for_test(ConnectionId(1)));
    let relation_index = stopped
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SetPassengers(packet),
                    ..
                } if packet.vehicle_id == boat && packet.passenger_ids.is_empty()
            )
        })
        .expect("rideTick publishes the relation edge");
    let shift_set_index = stopped
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SetEntityData(update),
                    ..
                } if update.id == player_entity_id
                    && update.values.iter().any(|value| {
                        value.data_id == VANILLA_ENTITY_SHARED_FLAGS_DATA_ID
                            && value.serializer_id == VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID
                            && matches!(
                                &value.value,
                                EntityDataValueKind::Byte(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN)
                            )
                    })
            )
        })
        .expect("the level tracker publishes the input flag");
    assert!(!stopped.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetEntityData(update),
                ..
            } if update.id == player_entity_id
                && update.values.iter().any(|value| {
                    value.data_id == VANILLA_ENTITY_SHARED_FLAGS_DATA_ID
                        && value.serializer_id == VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID
                        && matches!(&value.value, EntityDataValueKind::Byte(0))
                })
        )
    }));
    assert!(shift_set_index < relation_index);
    // The level tracker ran before this connection tick. The dismount cleared
    // the flag after that sweep, so Vanilla publishes it on the next one.
    let next = world.tick(Vec::new());
    assert!(next.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::SetEntityData(update),
                ..
            } if update.id == player_entity_id
                && update.values.iter().any(|value| {
                    value.data_id == VANILLA_ENTITY_SHARED_FLAGS_DATA_ID
                        && value.serializer_id == VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID
                        && matches!(&value.value, EntityDataValueKind::Byte(0))
                })
        )
    }));
}

#[test]
fn external_player_vehicle_lifecycle_restore_applies_game_type_to_saved_abilities() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities {
            invulnerable: true,
            flying: true,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.07,
            walking_speed: 0.2,
        },
    );
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    join_loaded(&sender, &receiver, &mut world, 2, "alice", 1);
    assert_eq!(
        world.player_abilities_for_test(ConnectionId(2)),
        PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: false,
            flying_speed: 0.07,
            walking_speed: 0.2,
        }
    );

    world.set_player_mode_and_abilities_for_test(
        ConnectionId(2),
        GameType::Creative,
        PlayerAbilities {
            invulnerable: false,
            flying: true,
            can_fly: false,
            instabuild: false,
            flying_speed: 0.09,
            walking_speed: 0.25,
        },
    );
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(2),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    join_loaded(&sender, &receiver, &mut world, 3, "alice", 1);
    assert_eq!(
        world.player_abilities_for_test(ConnectionId(3)),
        PlayerAbilities {
            invulnerable: true,
            flying: true,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.09,
            walking_speed: 0.25,
        }
    );
}

#[test]
fn external_player_vehicle_lifecycle_death_respawn_rebuilds_new_player_abilities() {
    for game_type in [GameType::Creative, GameType::Spectator] {
        let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        world.set_player_mode_and_abilities_for_test(
            ConnectionId(1),
            game_type,
            PlayerAbilities {
                invulnerable: false,
                flying: true,
                can_fly: false,
                instabuild: false,
                flying_speed: 0.075,
                walking_speed: 0.25,
            },
        );
        world.prime_player_body_removal_for_test(ConnectionId(1));
        world.tick(receiver.drain());
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
        );

        assert_eq!(
            world.player_abilities_for_test(ConnectionId(1)),
            PlayerAbilities::for_game_type(game_type),
            "{game_type:?} respawn constructs a fresh Player ability state"
        );
    }
}

#[test]
fn external_player_vehicle_lifecycle_moved_packet_pose_survives_real_storage_restart() {
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-vehicle-control-restart-{}",
        std::process::id()
    ));
    let storage = created_fixture_save(&root, "bbb-vehicle-control-restart");
    let paths = storage.paths().clone();
    let candidate = storage.load_candidate().unwrap();
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &candidate).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: player_position.y + 2.0,
            ..player_position
        },
    );
    let mounted = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: boat,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
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
        .expect("normal Boat admission positions the rider");
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
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: target,
            y_rot: 35.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );
    let live_vehicle = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
        .expect("the moved Boat remains live")
        .position();
    let live_player = world.publication().players[0].position;
    let request = world
        .save_request(1_786_000_020_000, 400, RegionCompression::Zlib)
        .unwrap();
    storage.save_snapshot(&candidate, &request).unwrap();
    storage.close().unwrap();

    let storage = Java26WorldStorage::open(paths, WorldSaveLimits::default()).unwrap();
    let reloaded = storage.load_candidate().unwrap();
    let (_, player_data) = reloaded
        .load_player(profile("alice", 1).uuid)
        .unwrap()
        .expect("saved playerdata should load lazily");
    assert_eq!(
        player_data.position(),
        [live_player.x, live_player.y, live_player.z]
    );
    assert_eq!(
        player_data
            .root_vehicle()
            .expect("storage retained RootVehicle")
            .entity()
            .position(),
        [live_vehicle.x, live_vehicle.y, live_vehicle.z]
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::restore(ServerWorldConfig::default(), &reloaded).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let restarted = world.tick(receiver.drain());
    assert!(world
        .external_player_vehicle_state_for_test(ConnectionId(1))
        .2
        .is_some());
    assert_eq!(
        restarted.snapshot.players[0].position,
        world
            .external_player_vehicle_placement_for_test(ConnectionId(1))
            .unwrap()
            .position
    );
    storage.close().unwrap();
}

/// `AbstractHorse.mobInteract`'s ride arm and `RunAroundLikeCrazyGoal`
/// (`AbstractHorse.java:643-668`, `RunAroundLikeCrazyGoal.java:51-73`) on the
/// production interaction and mob-tick path.
///
/// Every draw either branch takes is the horse's own random, none the level's:
/// `canUse`'s `DefaultRandomPos.getPos` (`:26`), `tick`'s
/// `nextInt(adjustedTickDelay(50))` (`:52`) and, once that hits with a Player
/// rider, `nextInt(getMaxTemper())` (`:61`). The temper the second draw is
/// weighed against is what makes each branch deterministic here: a temper of 0
/// can never satisfy `nextInt(100) < 0`, and the throw that follows raises it
/// by five (`:66`) until one roll has to take.
fn horse_ride_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(startup_generation());
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    (sender, receiver, world)
}

/// An untamed adult horse inside `Player.interactOn`'s reach of the session.
fn horse_beside_alice(world: &mut ServerWorld) -> i32 {
    let horse_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID;
    let beside = {
        let position = world.publication().players[0].position;
        Vec3d {
            x: position.x + 1.5,
            ..position
        }
    };
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            horse_type,
            beside,
            EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(horse_type).expect("a horse lives"),
            ),
        )
        .0
}

fn horse_record_int(world: &ServerWorld, horse: i32, field: &str) -> i32 {
    let record = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, horse)
        .expect("the horse answers its record");
    let NbtSummaryValue::Compound(entries) = record else {
        panic!("an entity record is a compound");
    };
    match entries
        .iter()
        .find(|entry| entry.name == field)
        .map(|entry| &entry.value)
    {
        Some(NbtSummaryValue::Int(value)) => *value,
        Some(NbtSummaryValue::Byte(value)) => i32::from(*value),
        other => panic!("{field} is {other:?}"),
    }
}

fn horse_owner(world: &ServerWorld, horse: i32) -> Option<Uuid> {
    let record = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, horse)
        .expect("the horse answers its record");
    let NbtSummaryValue::Compound(entries) = record else {
        panic!("an entity record is a compound");
    };
    entries
        .iter()
        .find(|entry| entry.name == "Owner")
        .map(|entry| match &entry.value {
            NbtSummaryValue::IntArray(values) if values.len() == 4 => {
                let mut bytes = [0_u8; 16];
                for (index, word) in values.iter().enumerate() {
                    bytes[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
                }
                Uuid::from_bytes(bytes)
            }
            other => panic!("Owner is {other:?}"),
        })
}

fn right_click_horse(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    horse: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: horse,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    )
}

fn rides(world: &ServerWorld, horse: i32) -> bool {
    let (vehicle, _, _, _) = world.external_player_vehicle_state_for_test(ConnectionId(1));
    let _ = vehicle;
    world
        .external_player_relation_in_dimension_for_test(
            Java26BuiltinDimension::Overworld,
            world.player_entity_id_for_test(ConnectionId(1)),
        )
        .0
        == Some(horse)
}

/// Ticks until the rider is off the horse again, returning the events of every
/// tick it took.
fn tick_until_thrown(
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    horse: i32,
    limit: usize,
) -> Vec<WorldEvent> {
    let mut events = Vec::new();
    for _ in 0..limit {
        let result = world.tick(receiver.drain());
        events.extend(result.events);
        if !rides(world, horse) {
            return events;
        }
    }
    panic!("the goal threw nobody in {limit} ticks");
}

#[test]
fn an_untamed_horse_seats_the_clicking_session_and_the_goal_throws_it_for_five_temper() {
    let (sender, receiver, mut world) = horse_ride_world();
    let horse = horse_beside_alice(&mut world);
    world.tick(receiver.drain());
    assert_eq!(horse_record_int(&world, horse, "Temper"), 0);
    assert_eq!(horse_record_int(&world, horse, "Tame"), 0);

    // `Horse.mobInteract` with an empty hand reaches
    // `AbstractHorse.mobInteract`'s `doPlayerRide` (`Horse.java:157-170`,
    // `AbstractHorse.java:666`).
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, horse)
        .expect("the horse")
        .set_rot(41.0, -12.0);
    let seated = right_click_horse(&sender, &receiver, &mut world, horse);
    assert!(rides(&world, horse), "the click seated the session");
    // `player.setYRot(this.getYRot())` and `setXRot` run before `startRiding`
    // (`AbstractHorse.java:498-500`), so the teleport carries them absolutely.
    assert!(
        seated.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(packet),
                ..
            } if packet.relatives_mask == 0 && packet.y_rot == 41.0 && packet.x_rot == -12.0
        )),
        "the rider takes the horse's rotation"
    );

    // `nextInt(100) < 0` is never true, so the first firing of `tick` must take
    // the throw arm (`RunAroundLikeCrazyGoal.java:61-71`).
    let events = tick_until_thrown(&receiver, &mut world, horse, 600);
    assert_eq!(horse_record_int(&world, horse, "Temper"), 5);
    assert_eq!(horse_record_int(&world, horse, "Tame"), 0);
    assert_eq!(horse_owner(&world, horse), None);
    // `broadcastEntityEvent(this.horse, (byte)6)` (`:71`).
    assert!(
        events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityEvent(packet),
                ..
            } if packet.entity_id == horse && packet.event_id == 6
        )),
        "the throw broadcasts the smoke event"
    );
    // `makeMad`'s `makeSound(getAngrySound())` (`AbstractHorse.java:705`,
    // `Horse.java:150-152`).
    assert!(
        played_horse_sound(&events, "minecraft:entity.horse.angry"),
        "the throw plays the angry sound"
    );
}

fn played_horse_sound(events: &[WorldEvent], resource_id: &str) -> bool {
    events.iter().any(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Sound(sound),
            ..
        } => match sound.sound {
            SoundEventHolder::Reference { registry_id } => {
                vanilla_sound_event_resource_id(registry_id) == Some(resource_id)
            }
            SoundEventHolder::Direct { .. } => false,
        },
        _ => false,
    })
}

#[test]
fn riding_an_untamed_horse_until_a_roll_takes_tames_it_and_grants_the_advancement() {
    const TAME_AN_ANIMAL: &str = "minecraft:husbandry/tame_an_animal";
    let (sender, receiver, mut world) = horse_ride_world();
    let horse = horse_beside_alice(&mut world);
    world.tick(receiver.drain());
    assert!(!world.player_advancement_is_done(ConnectionId(1), TAME_AN_ANIMAL));

    // Each throw adds five temper (`RunAroundLikeCrazyGoal.java:66`), so by the
    // twenty-first ride `nextInt(100) < temper` cannot refuse any more.
    let mut rides_taken = 0;
    while horse_record_int(&world, horse, "Tame") == 0 {
        assert!(rides_taken < 24, "the roll takes inside 24 rides");
        rides_taken += 1;
        let _ = right_click_horse(&sender, &receiver, &mut world, horse);
        assert!(
            rides(&world, horse),
            "ride {rides_taken} seated the session"
        );
        for _ in 0..600 {
            world.tick(receiver.drain());
            if !rides(&world, horse) || horse_record_int(&world, horse, "Tame") != 0 {
                break;
            }
        }
    }

    // `tameWithName` (`AbstractHorse.java:709-718`): owner, tame flag and the
    // `minecraft:tame_animal` trigger, with the rider left seated.
    assert_eq!(
        horse_owner(&world, horse),
        Some(world.publication().players[0].uuid)
    );
    assert!(rides(&world, horse), "taming keeps the rider on");
    assert!(world.player_advancement_is_done(ConnectionId(1), TAME_AN_ANIMAL));
}

/// `Entity.startRiding`'s `sendEventAndTriggers` block
/// (`Entity.java:2380-2385`) on the production horse click.
///
/// `husbandry/ride_a_boat_with_a_goat` is the only built-in consumer of
/// `started_riding` and its player predicate names a vehicle and a passenger
/// (`VanillaHusbandryAdvancements.java:479-489`), which riding a horse does not
/// satisfy; the criterion this pack adds carries an empty player predicate,
/// so the fire is observable.
#[test]
fn riding_a_horse_raises_the_mount_event_and_fires_the_started_riding_criterion() {
    const RIDING: &str = "test:started_riding";
    let (sender, receiver, mut world) = horse_ride_world_with(&[(
        RIDING,
        serde_json::json!({"criteria":{"ride":{"trigger":"minecraft:started_riding"}}}),
    )]);
    let horse = horse_beside_alice(&mut world);
    world.tick(receiver.drain());
    assert!(!world.player_advancement_is_done(ConnectionId(1), RIDING));

    let horse_before_click = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, horse)
        .expect("the horse")
        .position();

    let seated = right_click_horse(&sender, &receiver, &mut world, horse);
    assert!(rides(&world, horse), "the click seated the session");
    assert!(world.player_advancement_is_done(ConnectionId(1), RIDING));
    assert!(
        seated.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::UpdateAdvancements(_),
                ..
            }
        )),
        "the riding criterion publishes its advancement"
    );
    assert!(
        !world.player_advancement_is_done(
            ConnectionId(1),
            "minecraft:husbandry/ride_a_boat_with_a_goat"
        ),
        "the built-in consumer names a vehicle and a passenger, so it refuses"
    );

    let horse_pos = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, horse)
        .expect("the horse")
        .block_position();
    let player_entity_id = world.player_entity_id_for_test(ConnectionId(1));
    let interaction = world
        .positioned_game_events_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|event| {
            event.recorded.game_event == VanillaGameEvent::ENTITY_INTERACT
                && event.recorded.source_entity_id == Some(player_entity_id)
        })
        .expect("the horse interaction raised its event");
    assert_eq!(interaction.position, horse_before_click);
    let (raised, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(
        raised
            .iter()
            .filter(|event| {
                event.game_event == VanillaGameEvent::ENTITY_MOUNT
                    && event.pos == horse_pos
                    && event.source_entity_id == Some(player_entity_id)
            })
            .count(),
        1,
        "the mount raises `ENTITY_MOUNT` at the horse for the rider"
    );
}

/// `RunAroundLikeCrazyGoal`'s throw reaching `LivingEntity.stopRiding`
/// (`RunAroundLikeCrazyGoal.java:69`, `LivingEntity.java:3217-3223`): the rider
/// is moved to `AbstractHorse.getDismountLocationForPassenger`'s answer
/// (`AbstractHorse.java:998-1012`) instead of being left where it sat.
#[test]
fn the_goal_throw_moves_the_thrown_rider_to_the_horses_main_arm_dismount_location() {
    let (sender, receiver, mut world) = horse_ride_world();
    let ground = flatten_around_spawn(&mut world, 12);
    let horse = horse_beside_alice(&mut world);
    world.tick(receiver.drain());

    let right = throw_and_read_landing(&sender, &receiver, &mut world, horse, ground);
    assert!(
        right.cross_with_horse_facing > 0.0,
        "a right-handed session lands on the horse's right"
    );

    // `ServerPlayer.updateOptions` is the only writer of `DATA_PLAYER_MAIN_HAND`
    // (`ServerPlayer.java:1843`), and the off-hand direction is the mirrored one
    // (`AbstractHorse.java:1007-1009`).
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientInformation(ClientInformation {
            main_hand: bbb_facts::client_information_options::ClientMainHand::Left,
            ..ClientInformation::default()
        }),
    );
    let left = throw_and_read_landing(&sender, &receiver, &mut world, horse, ground);
    assert!(
        left.cross_with_horse_facing < 0.0,
        "a left-handed session lands on the horse's left"
    );
}

struct ThrowLanding {
    cross_with_horse_facing: f64,
}

/// Rides the horse until the goal throws the session, then checks every
/// landing fact the throw owes and reports which side of the horse it took.
fn throw_and_read_landing(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    horse: i32,
    ground: i32,
) -> ThrowLanding {
    let _ = right_click_horse(sender, receiver, world, horse);
    assert!(rides(world, horse), "the click seated the session");
    let (raised_before, before) = world.game_events(Java26BuiltinDimension::Overworld);
    let dismounts_before = raised_before
        .iter()
        .filter(|event| event.game_event == VanillaGameEvent::ENTITY_DISMOUNT)
        .count();
    // The horse is read before the throwing tick: it keeps travelling after
    // the goal throws its rider within that tick.
    let mut events = Vec::new();
    let mut thrown_from = None;
    for _ in 0..600 {
        let entity = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, horse)
            .expect("the horse");
        let state = (
            entity.position(),
            entity.bounding_box(),
            entity.y_rot(),
            entity.block_position(),
        );
        events.extend(world.tick(receiver.drain()).events);
        if !rides(world, horse) {
            thrown_from = Some(state);
            break;
        }
    }
    let (horse_position, horse_bounds, horse_y_rot, horse_block) =
        thrown_from.expect("the goal threw nobody in 600 ticks");
    assert!(
        horse_bounds.min_y.floor() as i32 == ground,
        "the horse stayed on the flattened platform"
    );
    let landing = world.publication().players[0].position;

    // `dismountTo` reaches `ServerPlayer.teleportTo(double, double, double)`
    // (`ServerPlayer.java:1669-1671`), whose relative set is delta and rotation.
    assert!(
        events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(packet),
                ..
            } if packet.position == landing
                && packet.relatives_mask
                    == PLAYER_RELATIVE_DELTA_X
                        | PLAYER_RELATIVE_DELTA_Y
                        | PLAYER_RELATIVE_DELTA_Z
                        | PLAYER_RELATIVE_Y_ROT
                        | PLAYER_RELATIVE_X_ROT
        )),
        "the throw teleports the rider to the landing"
    );
    let player_entity_id = world.player_entity_id_for_test(ConnectionId(1));
    let (raised, after) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(after > before, "the throw raised something");
    assert!(raised.iter().any(|event| {
        event.game_event == VanillaGameEvent::ENTITY_DISMOUNT
            && event.pos == horse_block
            && event.source_entity_id == Some(player_entity_id)
    }));
    assert_eq!(
        raised
            .iter()
            .filter(|event| {
                event.game_event == VanillaGameEvent::ENTITY_DISMOUNT
                    && event.source_entity_id == Some(player_entity_id)
            })
            .count(),
        dismounts_before + 1,
        "`removeVehicle` raises `ENTITY_DISMOUNT` at the horse (`Entity.java:2410`)"
    );

    // The landing is a floor the standing box fits on, which is what
    // `getBlockFloorHeight` plus `DismountHelper.canDismountTo` buy.
    assert_eq!(
        landing.y,
        f64::from(ground),
        "the rider came down to the platform"
    );
    let player_box = bbb_server_types::geometry::Aabb::of_entity(landing, 0.6, 1.8);
    assert!(
        !player_box.intersects(horse_bounds),
        "the landing clears the horse"
    );
    let radians = f64::from(horse_y_rot).to_radians();
    let forward = (-radians.sin(), radians.cos());
    let offset = (landing.x - horse_position.x, landing.z - horse_position.z);
    ThrowLanding {
        cross_with_horse_facing: forward.0 * offset.1 - forward.1 * offset.0,
    }
}

/// A stone platform with three clear blocks over it, so the dismount search
/// answers from a floor the test states rather than from generated terrain.
/// Returns the y the platform's top surface sits at.
fn flatten_around_spawn(world: &mut ServerWorld, radius: i32) -> i32 {
    let spawn = world.publication().players[0].position;
    let ground = spawn.y.floor() as i32;
    let stone = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:stone",
        &BTreeMap::new(),
    )
    .expect("stone");
    let air = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:air",
        &BTreeMap::new(),
    )
    .expect("air");
    // `Level.setBlock` answers false for a write that changes nothing, so the
    // column is checked back rather than the write.
    for x in spawn.x.floor() as i32 - radius..=spawn.x.floor() as i32 + radius {
        for z in spawn.z.floor() as i32 - radius..=spawn.z.floor() as i32 + radius {
            let floor = BlockPos {
                x,
                y: ground - 1,
                z,
            };
            world.set_block(
                Java26BuiltinDimension::Overworld,
                floor,
                stone,
                crate::fluid::SET_BLOCK_UPDATE_ALL,
            );
            assert_eq!(
                world.block_state(Java26BuiltinDimension::Overworld, floor),
                Some(stone)
            );
            for y in ground..=ground + 3 {
                let clear = BlockPos { x, y, z };
                world.set_block(
                    Java26BuiltinDimension::Overworld,
                    clear,
                    air,
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                );
                assert_eq!(
                    world.block_state(Java26BuiltinDimension::Overworld, clear),
                    Some(air)
                );
            }
        }
    }
    ground
}

fn horse_ride_world_with(
    advancements: &[(&str, serde_json::Value)],
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let mut startup = (*startup_generation()).clone();
    let mut declared = startup.advancements.advancements.clone();
    for (id, body) in advancements {
        declared.push(bbb_server_datapack::advancement::AdvancementElement {
            id: (*id).to_string(),
            source_pack: "test".to_string(),
            advancement: crate::tests::parse_advancement(body)
                .expect("the test advancement parses"),
        });
    }
    declared.sort_by(|left, right| left.id.cmp(&right.id));
    startup.advancements = bbb_server_datapack::advancement::AdvancementSet {
        advancements: declared,
        warnings: startup.advancements.warnings.clone(),
    };
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(Arc::new(startup));
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    (sender, receiver, world)
}

/// `AbstractBoat.getDismountLocationForPassenger` (`AbstractBoat.java:630-658`)
/// on the shift dismount, which is `Player.rideTick`'s `stopRiding`
/// (`Player.java:432-438`).
///
/// The expected point is computed here from real trigonometry rather than from
/// the production sine table, so the two agree only if the escape vector, the
/// floor candidate and the pose box are all the ones vanilla names.
#[test]
fn a_shift_dismount_from_a_beached_boat_lands_at_the_boats_escape_vector() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    let ground = flatten_around_spawn(&mut world, 4);
    let spawn = world.publication().players[0].position;
    let boat = oak_boat(
        &mut world,
        Vec3d {
            x: spawn.x,
            y: f64::from(ground),
            z: spawn.z,
        },
    );
    let mounted_boat_position = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
        .expect("the boat")
        .position();
    let mounted = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: boat,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    let player_entity_id = world.player_entity_id_for_test(ConnectionId(1));
    let (mount_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(
        mount_events
            .iter()
            .filter(|event| {
                event.game_event == VanillaGameEvent::ENTITY_MOUNT
                    && event.source_entity_id == Some(player_entity_id)
            })
            .count(),
        1
    );
    let mounted_event = world
        .positioned_game_events_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|event| {
            event.recorded.game_event == VanillaGameEvent::ENTITY_MOUNT
                && event.recorded.source_entity_id == Some(player_entity_id)
        })
        .expect("the mount's exact event");
    assert_eq!(mounted_event.position, mounted_boat_position);
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
        .expect("the mount teleports the session");
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    // `play` applies the packet and ticks, so the shift dismount lands in this
    // result rather than in a later tick.
    let dismounted = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput {
            shift: true,
            ..PlayerInput::default()
        }),
    );
    assert_eq!(
        world
            .external_player_vehicle_state_for_test(ConnectionId(1))
            .2,
        None,
        "the shift input ends the ride"
    );

    let (boat_position, boat_width, boat_block) = {
        let entity = world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, boat)
            .expect("the boat");
        (
            entity.position(),
            entity.bb_width(),
            entity.block_position(),
        )
    };
    let player = &world.publication().players[0];
    // `getCollisionHorizontalEscapeVector(bbWidth * SQRT_OF_TWO, 0.6, yRot)`
    // (`AbstractBoat.java:631`, `Entity.java:3524-3530`).
    let distance = (f64::from(boat_width * 2.0_f32.sqrt()) + 0.6 + f64::from(1.0e-5_f32)) / 2.0;
    let radians = f64::from(player.rotation.y_rot).to_radians();
    let direction_x = -radians.sin();
    let direction_z = radians.cos();
    let scale = direction_x.abs().max(direction_z.abs());
    let expected = Vec3d {
        x: boat_position.x + direction_x * distance / scale,
        y: f64::from(ground),
        z: boat_position.z + direction_z * distance / scale,
    };
    assert!(
        (player.position.x - expected.x).abs() < 1.0e-3
            && (player.position.z - expected.z).abs() < 1.0e-3,
        "landed at {:?}, expected {expected:?}",
        player.position
    );
    assert_eq!(player.position.y, expected.y);

    let (raised, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert_eq!(
        raised
            .iter()
            .filter(|event| {
                event.game_event == VanillaGameEvent::ENTITY_DISMOUNT
                    && event.pos == boat_block
                    && event.source_entity_id == Some(player_entity_id)
            })
            .count(),
        1,
        "the shift dismount raises `ENTITY_DISMOUNT` at the boat"
    );
    let dismounted_event = world
        .positioned_game_events_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find(|event| {
            event.recorded.game_event == VanillaGameEvent::ENTITY_DISMOUNT
                && event.recorded.source_entity_id == Some(player_entity_id)
        })
        .expect("the dismount's exact event");
    assert_eq!(dismounted_event.position, boat_position);
    assert_eq!(dismounted_event.recorded.pos, boat_block);
    let landing = player.position;
    assert!(
        dismounted.events.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                packet: ClientboundPlayPacket::PlayerPosition(packet),
                ..
            } if packet.position == landing
        )),
        "the dismount teleports the session to the landing"
    );
}

#[test]
fn boat_dismount_selects_crouching_then_swimming_under_low_roofs() {
    use bbb_protocol::entity_poses::{
        VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
        VANILLA_ENTITY_POSE_SWIMMING_ID,
    };
    use bbb_protocol::entity_synched_data::VANILLA_ENTITY_POSE_DATA_ID;

    for (slab_type, expected_pose) in [
        ("top", VANILLA_ENTITY_POSE_CROUCHING_ID),
        ("bottom", VANILLA_ENTITY_POSE_SWIMMING_ID),
    ] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(ServerWorldConfig::default());
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
        let ground = flatten_around_spawn(&mut world, 5);
        let spawn = world.publication().players[0].position;
        let boat = oak_boat(
            &mut world,
            Vec3d {
                y: f64::from(ground),
                ..spawn
            },
        );
        let mounted = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::Interact(InteractEntity {
                entity_id: boat,
                hand: InteractionHand::MainHand,
                location: Vec3d::default(),
                using_secondary_action: false,
            }),
        );
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
            .expect("mount teleport");
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::AcceptTeleportation { id: teleport_id },
        );
        let slab = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:oak_slab",
            &BTreeMap::from([
                ("type".to_string(), slab_type.to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]),
        )
        .expect("slab state");
        let air = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:air",
            &BTreeMap::new(),
        )
        .expect("air");
        let center = BlockPos {
            x: spawn.x.floor() as i32,
            y: ground + 1,
            z: spawn.z.floor() as i32,
        };
        for x in center.x - 4..=center.x + 4 {
            for z in center.z - 4..=center.z + 4 {
                world.set_block(
                    Java26BuiltinDimension::Overworld,
                    BlockPos { x, y: center.y, z },
                    slab,
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                );
            }
        }
        let dismounted = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::PlayerInput(PlayerInput {
                shift: true,
                ..PlayerInput::default()
            }),
        );
        assert_eq!(
            world
                .external_player_vehicle_state_for_test(ConnectionId(1))
                .2,
            None
        );
        assert_eq!(
            world.player_pose_id_for_test(ConnectionId(1)),
            expected_pose,
            "{slab_type} roof chooses the first fitting pose"
        );
        let landing_teleport = dismounted
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::PlayerPosition(update),
                    ..
                } => Some(update.id),
                _ => None,
            })
            .expect("dismount teleport");
        let published = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::AcceptTeleportation {
                id: landing_teleport,
            },
        );
        let pose_values: Vec<_> = published
            .events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::SetEntityData(update),
                    ..
                } if update.id == world.player_entity_id_for_test(ConnectionId(1)) => update
                    .values
                    .iter()
                    .find(|value| value.data_id == VANILLA_ENTITY_POSE_DATA_ID),
                _ => None,
            })
            .collect();
        assert_eq!(pose_values.len(), 1);
        assert_eq!(
            pose_values[0].value,
            EntityDataValueKind::Pose(expected_pose)
        );
        let position = world.publication().players[0].position;
        let moved = play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(Vec3d {
                    x: position.x + 0.1,
                    ..position
                }),
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            }),
        );
        assert!(
            (world.publication().players[0].position.x - position.x - 0.1).abs() < 1.0e-4,
            "pose-sized walking box must fit beneath the slab"
        );
        assert_eq!(
            world.player_pose_id_for_test(ConnectionId(1)),
            expected_pose
        );
        assert!(moved.events.iter().all(|event| !matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(update), .. }
                if update.values.iter().any(|value| value.data_id == VANILLA_ENTITY_POSE_DATA_ID)
        )));
        for x in center.x - 4..=center.x + 4 {
            for z in center.z - 4..=center.z + 4 {
                world.set_block(
                    Java26BuiltinDimension::Overworld,
                    BlockPos { x, y: center.y, z },
                    air,
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                );
            }
        }
        world.tick(receiver.drain());
        assert_eq!(
            world.player_pose_id_for_test(ConnectionId(1)),
            VANILLA_ENTITY_POSE_STANDING_ID
        );
        let restored = world.tick(receiver.drain());
        assert_eq!(restored.events.iter().filter(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(update), .. }
                if update.values.iter().any(|value|
                    value.data_id == VANILLA_ENTITY_POSE_DATA_ID
                    && value.value == EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_STANDING_ID))
        )).count(), 1);
    }
}

#[test]
fn horse_shift_dismount_uses_a_crouching_pose_at_the_main_hand_landing() {
    let (sender, receiver, mut world) = horse_ride_world();
    let ground = flatten_around_spawn(&mut world, 12);
    let horse = horse_beside_alice(&mut world);
    // An untamed horse may run around under its rider and move the landing.
    let NbtSummaryValue::Compound(mut record) = world
        .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, horse)
        .expect("the horse answers its record")
    else {
        panic!("an entity record is a compound");
    };
    record.push(NbtSummaryEntry {
        name: "NoAI".into(),
        value: NbtSummaryValue::Byte(1),
    });
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, horse, &record);
    world.tick(receiver.drain());
    let mounted = right_click_horse(&sender, &receiver, &mut world, horse);
    assert!(rides(&world, horse));
    if let Some(teleport_id) = mounted.events.iter().find_map(|event| match event {
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
    let vehicle_position = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, horse)
        .expect("horse")
        .position();
    let candidate =
        world.external_player_dismount_target_for_test(ConnectionId(1), horse, vehicle_position);
    assert_eq!(candidate.y, f64::from(ground));
    let roof = BlockPos {
        x: candidate.x.floor() as i32,
        y: ground + 1,
        z: candidate.z.floor() as i32,
    };
    let top_slab = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:oak_slab",
        &BTreeMap::from([
            ("type".to_string(), "top".to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ]),
    )
    .expect("top slab");
    world.set_block(
        Java26BuiltinDimension::Overworld,
        roof,
        top_slab,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    // Without a ceiling over the slab, standing on it is within the horse's
    // dismount jump limit and wins before crouching is tried.
    world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            y: roof.y + 2,
            ..roof
        },
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:stone",
            &BTreeMap::new(),
        )
        .expect("stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
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
    assert!(!rides(&world, horse));
    assert_eq!(
        world.player_pose_id_for_test(ConnectionId(1)),
        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID
    );
    let landing = world.publication().players[0].position;
    assert_eq!(landing.y, f64::from(ground));
    assert!(
        landing.x - 0.3 < f64::from(roof.x + 1)
            && landing.x + 0.3 > f64::from(roof.x)
            && landing.z - 0.3 < f64::from(roof.z + 1)
            && landing.z + 0.3 > f64::from(roof.z),
        "the player box at {landing:?} overlaps the roof at {roof:?}"
    );
}

#[test]
fn a_portal_dismount_uses_the_vehicle_height_at_detachment() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    let ground = flatten_around_spawn(&mut world, 4);
    let player_position = world.publication().players[0].position;
    let before_tick = Vec3d {
        y: f64::from(ground),
        ..player_position
    };
    let boat = oak_boat(&mut world, before_tick);
    assert!(world.attach_external_player_for_test(ConnectionId(1), boat));
    let current = Vec3d {
        y: before_tick.y + 2.0,
        ..before_tick
    };
    assert!(current.y > player_position.y);
    world.set_stored_entity_position_for_test(Java26BuiltinDimension::Overworld, boat, current);
    let portal = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:nether_portal",
        &BTreeMap::from([("axis".to_string(), "x".to_string())]),
    )
    .expect("nether portal state");
    world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: current.x.floor() as i32,
            y: current.y.floor() as i32,
            z: current.z.floor() as i32,
        },
        portal,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    let target = world.external_player_dismount_target_for_test(ConnectionId(1), boat, current);
    assert_eq!(target.y, current.y);
}

#[test]
fn a_cross_level_teleport_command_carries_a_boats_online_players() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 2);
    join_loaded(&sender, &receiver, &mut world, 3, "carol", 3);
    assert!(world.teleport_player(
        ConnectionId(3),
        crate::world::TeleportTransition {
            new_level: Java26BuiltinDimension::Nether.into(),
            position: Vec3d {
                x: 8.5,
                y: 70.0,
                z: 8.5,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut Vec::new(),
    ));
    drain_chunk_deliveries(&sender, &receiver, &mut world, 3);
    console_command(&sender, &receiver, &mut world, 1, "tick freeze");
    let boat = oak_boat(
        &mut world,
        Vec3d {
            x: SPAWN_X,
            y: SPAWN_Y,
            z: SPAWN_Z,
        },
    );
    let source = Java26BuiltinDimension::Overworld;
    let target = Java26BuiltinDimension::Nether;
    let uuid = world.stored_entity_for_test(source, boat).unwrap().uuid();
    for connection in [ConnectionId(1), ConnectionId(2)] {
        assert!(world.attach_external_player_for_test(connection, boat));
    }
    let tick = console_command(
        &sender,
        &receiver,
        &mut world,
        2,
        "execute as @e[type=minecraft:oak_boat,limit=1] in minecraft:the_nether run tp @s 8.5 70 8.5",
    );
    assert_eq!(console_command_answer(&tick).0, Some(1));
    assert!(world.stored_entity_for_test(source, boat).is_none());
    let new_boat = world
        .stored_entity_ids_for_test(target)
        .into_iter()
        .find(|id| world.stored_entity_for_test(target, *id).unwrap().uuid() == uuid)
        .expect("the ridden boat arrives in the destination level");
    for connection in [ConnectionId(1), ConnectionId(2)] {
        assert_eq!(world.player_dimension(connection), target);
        let (_, _, vehicle, root) = world.external_player_vehicle_state_for_test(connection);
        assert_eq!((vehicle, root), (Some(new_boat), Some(new_boat)));
        assert!(tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::Respawn(_), }
                if *connection_id == connection
        )));
    }
    let passenger_ids = [
        world.player_entity_id_for_test(ConnectionId(1)),
        world.player_entity_id_for_test(ConnectionId(2)),
    ];
    assert_eq!(
        world.stored_passengers_for_test(target, new_boat),
        passenger_ids
    );
    let mut events = tick.events;
    for connection in [ConnectionId(1), ConnectionId(2)] {
        let teleport_id = events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::PlayerPosition(update),
                } if *connection_id == connection => Some(update.id),
                _ => None,
            })
            .expect("each rider receives the dimension-change teleport");
        for packet in [
            PlayServerbound::AcceptTeleportation { id: teleport_id },
            PlayServerbound::PlayerLoaded,
        ] {
            enqueue(
                &sender,
                WorldCommand::Play {
                    connection_id: connection,
                    packet,
                },
            )
            .unwrap();
        }
    }
    let mut acknowledged_events = 0;
    for _ in 0..20 {
        for connection in [ConnectionId(1), ConnectionId(2)] {
            if events[acknowledged_events..].iter().any(|event| matches!(event,
                WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::ChunkBatchFinished { .. }, }
                    if *connection_id == connection
            )) {
                enqueue(&sender, WorldCommand::Play {
                    connection_id: connection,
                    packet: PlayServerbound::ChunkBatchReceived { desired_chunks_per_tick: 64.0 },
                }).unwrap();
            }
        }
        acknowledged_events = events.len();
        events.extend(world.tick(receiver.drain()).events);
    }
    for connection in [ConnectionId(1), ConnectionId(2), ConnectionId(3)] {
        assert!(
            events.iter().any(|event| matches!(event,
                WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::AddEntity(add), }
                    if *connection_id == connection && add.id == new_boat && add.uuid == uuid
            )),
            "{connection:?} sees the arriving boat"
        );
        assert!(events.iter().any(|event| matches!(event,
            WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::SetPassengers(passengers), }
                if *connection_id == connection && passengers.vehicle_id == new_boat && passengers.passenger_ids == passenger_ids
        )), "{connection:?} sees the restored seats");
    }
    let old_position = world
        .stored_entity_for_test(target, new_boat)
        .unwrap()
        .position();
    let moved = console_command(&sender, &receiver, &mut world, 3,
        "execute in minecraft:the_nether as @e[type=minecraft:oak_boat,limit=1] at @s run tp @s ~8 ~ ~2 ~90 ~");
    assert_eq!(console_command_answer(&moved).0, Some(1));
    let position = world
        .stored_entity_for_test(target, new_boat)
        .unwrap()
        .position();
    assert_eq!(
        (position.x, position.z),
        (old_position.x + 8.0, old_position.z + 2.0)
    );
    for connection in [ConnectionId(1), ConnectionId(2)] {
        assert_eq!(
            world.external_player_vehicle_state_for_test(connection).2,
            Some(new_boat)
        );
    }
    assert!(moved.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::TeleportEntity(update), }
            if update.id == new_boat && update.relatives_mask != 0 && update.position.x == 8.0
    )), "the controlling rider receives the vehicle's relative transition");
    assert!(moved.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(2), packet: ClientboundPlayPacket::TeleportEntity(update), }
            if update.id == new_boat && update.relatives_mask == 0 && update.position == position
    )), "the second rider receives the vehicle's absolute arrival");
}

#[test]
fn pigs_minecarts_and_striders_set_a_dismounting_player_down_beside_them() {
    let (_sender, _receiver, mut world) = horse_ride_world();
    let ground = flatten_around_spawn(&mut world, 12);
    let spawn = world.publication().players[0].position;
    for (offset, kind, data) in [
        (
            2.0,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
            EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
                )
                .unwrap(),
            ),
        ),
        (
            -2.0,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
            EntityData::Minecart(crate::entity::MinecartData::new()),
        ),
        (
            5.0,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID,
            EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_STRIDER_ID,
                )
                .unwrap(),
            ),
        ),
    ] {
        let position = Vec3d {
            x: spawn.x.floor() + offset + 0.5,
            y: f64::from(ground),
            z: spawn.z.floor() + 0.5,
        };
        let (vehicle, _) = world.spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            kind,
            position,
            data,
        );
        let target =
            world.external_player_dismount_target_for_test(ConnectionId(1), vehicle, position);
        assert_eq!(target.y, f64::from(ground), "{kind}");
        let horizontal = (target.x - position.x)
            .abs()
            .max((target.z - position.z).abs());
        assert!(horizontal >= 0.5, "{kind} landed on top at {target:?}");
    }
}

/// `ServerPlayer.setGameMode` stops riding on the switch to spectator
/// (`ServerPlayer.java:1741-1743`).
#[test]
fn switching_to_spectator_leaves_the_vehicle() {
    let mut world = ServerWorld::new(operator_config(&[("alice", 1)])).unwrap();
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let boat = oak_boat(
        &mut world,
        Vec3d {
            x: SPAWN_X,
            y: SPAWN_Y,
            z: SPAWN_Z,
        },
    );
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
    assert!(world.attach_external_player_for_test(ConnectionId(1), boat));

    chat_command(&sender, &receiver, &mut world, 1, "gamemode spectator");
    let (_, _, vehicle, _) = world.external_player_vehicle_state_for_test(ConnectionId(1));
    assert_eq!(vehicle, None);
}
