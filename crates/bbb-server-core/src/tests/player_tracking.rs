use super::*;

use crate::entity::{BoatData, EntityData};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;

fn move_without_displacement(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    connection_id: u64,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        connection_id,
        PlayServerbound::MovePlayer(MovePlayer {
            position: None,
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    )
}

fn packet_entity_id(packet: &ClientboundPlayPacket) -> Option<i32> {
    match packet {
        ClientboundPlayPacket::AddEntity(packet) => Some(packet.id),
        ClientboundPlayPacket::SetEntityData(packet) => Some(packet.id),
        ClientboundPlayPacket::UpdateAttributes(packet) => Some(packet.entity_id),
        ClientboundPlayPacket::SetEquipment(packet) => Some(packet.entity_id),
        ClientboundPlayPacket::SetPassengers(packet) => Some(packet.vehicle_id),
        _ => None,
    }
}

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
fn player_info_precedes_pairing_when_the_pending_chunk_is_delivered() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);

    let (bob_join, bob_handshake) = join_loaded_ticks(&sender, &receiver, &mut world, 2, "bob", 2);
    let acknowledged = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::ChunkBatchReceived {
            desired_chunks_per_tick: 9.0,
        },
    );
    let bob = entity_id_of(&world, 2);
    let info = bob_join
        .events
        .iter()
        .position(|event| {
            matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::PlayerInfoUpdate(update),
            } if update.entries.iter().any(|entry| entry.profile_id == Uuid::from_u128(2)))
        })
        .expect("alice receives bob's player info");
    let pairing = bob_join
        .events
        .iter()
        .position(|event| {
            matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::AddEntity(add),
            } if add.id == bob)
        })
        .expect("alice's delivered spawn chunk admits bob");
    assert!(info < pairing, "{:#?}", bob_join.events);

    let alice = entity_id_of(&world, 1);
    let mut delivered = false;
    let mut alice_info = false;
    let mut additions = 0;
    for tick in [&bob_join, &bob_handshake, &acknowledged] {
        for event in &tick.events {
            if let WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet,
            } = event
            {
                match packet {
                    ClientboundPlayPacket::PlayerInfoUpdate(update)
                        if update
                            .entries
                            .iter()
                            .any(|entry| entry.profile_id == Uuid::from_u128(1)) =>
                    {
                        alice_info = true
                    }
                    ClientboundPlayPacket::LevelChunkWithLight(chunk)
                        if (chunk.x, chunk.z) == (0, 0) =>
                    {
                        delivered = true
                    }
                    ClientboundPlayPacket::AddEntity(add) if add.id == alice => {
                        assert!(
                            alice_info && delivered,
                            "player info and the column precede pairing"
                        );
                        additions += 1;
                    }
                    _ => {}
                }
            }
        }
    }
    assert!(delivered);
    assert_eq!(additions, 1);
    assert_eq!(
        world.player_tracking_viewers_for_test(Java26BuiltinDimension::Overworld, alice),
        [ConnectionId(2)]
    );
    world.set_player_attribute_base_for_test(
        ConnectionId(1),
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        24.0,
    );
    let sync = world.tick(receiver.drain());
    assert!(sync.events.iter().any(|event| matches!(
        event.packet_for(ConnectionId(2)),
        Some(ClientboundPlayPacket::UpdateAttributes(update))
            if update.entity_id == alice
                && update.attributes.iter().any(|attribute| {
                    attribute.attribute_id
                        == bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID
                        && attribute.base == 24.0
                })
    )));
    assert!(!sync.events.iter().any(|event| matches!(
        event.packet_for(ConnectionId(2)),
        Some(ClientboundPlayPacket::AddEntity(add)) if add.id == alice
    )));
}

#[test]
fn same_section_move_pairs_and_unpairs_once_and_blocks_later_sync() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.reset_pending_chunks_for_test(ConnectionId(1), &[]);
    world.reset_pending_chunks_for_test(ConnectionId(2), &[]);
    move_without_displacement(&sender, &receiver, &mut world, 1);
    move_without_displacement(&sender, &receiver, &mut world, 2);
    let alice = entity_id_of(&world, 1);

    let origin = world
        .publication()
        .players
        .iter()
        .find(|player| player.entity_id == alice)
        .expect("alice is published")
        .position;
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d { x: -8.0, ..origin }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    // The separation is the fixture; the 0.1 step below is the subject.
    place_player(
        &sender,
        &receiver,
        &mut world,
        2,
        Vec3d { x: 24.0, ..origin },
        None,
    );
    assert_eq!(
        world.player_tracking_viewers_for_test(Java26BuiltinDimension::Overworld, alice),
        [ConnectionId(2)]
    );

    let removed = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d { x: 24.1, ..origin }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    let alice_removals = removed
        .events
        .iter()
        .filter(|event| {
            matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::RemoveEntities(remove),
            } if remove.entity_ids == [alice])
        })
        .count();
    assert_eq!(alice_removals, 1, "{:#?}", removed.events);

    world.set_player_attribute_base_for_test(
        ConnectionId(1),
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        24.0,
    );
    let sync = world.tick(Vec::new());
    assert!(!sync.events.iter().any(|event| matches!(event,
        WorldEvent::Send {
            connection_id: ConnectionId(2),
            packet,
        } if packet_entity_id(packet) == Some(alice))));
}

#[test]
fn pairing_bundle_orders_metadata_attributes_and_equipment() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let alice = entity_id_of(&world, 1);
    world.set_player_attribute_base_for_test(
        ConnectionId(1),
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        24.0,
    );
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    world.set_player_shared_flag_for_test(
        ConnectionId(1),
        VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
        true,
    );

    let (joined, handshake) = join_loaded_ticks(&sender, &receiver, &mut world, 2, "bob", 2);
    let acknowledged = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::ChunkBatchReceived {
            desired_chunks_per_tick: 9.0,
        },
    );
    let packets: Vec<_> = [&joined, &handshake, &acknowledged]
        .into_iter()
        .flat_map(|tick| tick.events.iter())
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet,
            } => Some(packet),
            _ => None,
        })
        .collect();
    let add = packets
        .iter()
        .position(
            |packet| matches!(packet, ClientboundPlayPacket::AddEntity(add) if add.id == alice),
        )
        .expect("alice's first pairing reaches bob");
    let start = packets[..add]
        .iter()
        .rposition(|packet| matches!(packet, ClientboundPlayPacket::BundleDelimiter))
        .expect("the pairing bundle opens");
    let end = packets[add + 1..]
        .iter()
        .position(|packet| matches!(packet, ClientboundPlayPacket::BundleDelimiter))
        .map(|offset| add + 1 + offset)
        .expect("the pairing bundle closes");
    let bundle = &packets[start..=end];
    let kinds = bundle
        .iter()
        .map(|packet| match packet {
            ClientboundPlayPacket::BundleDelimiter => "delimiter",
            ClientboundPlayPacket::AddEntity(_) => "add",
            ClientboundPlayPacket::SetEntityData(_) => "metadata",
            ClientboundPlayPacket::UpdateAttributes(_) => "attributes",
            ClientboundPlayPacket::SetEquipment(_) => "equipment",
            _ => "other",
        })
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        [
            "delimiter",
            "add",
            "metadata",
            "attributes",
            "equipment",
            "delimiter"
        ]
    );
    assert!(bundle.iter().any(|packet| matches!(packet, ClientboundPlayPacket::SetEntityData(update)
        if update.id == alice && update.values.iter().any(|row| row.data_id == bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAGS_DATA_ID && row.value == bbb_protocol::packets::EntityDataValueKind::Byte(0x08)))));
    assert!(bundle.iter().any(|packet| matches!(packet, ClientboundPlayPacket::UpdateAttributes(update)
        if update.entity_id == alice && update.attributes.iter().any(|attribute| attribute.attribute_id == bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID && attribute.base == 24.0))));
    assert!(bundle.iter().any(|packet| matches!(packet, ClientboundPlayPacket::SetEquipment(update)
        if update.entity_id == alice && update.slots.iter().any(|slot| slot.item.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID) && slot.item.count == 1))));
}

#[test]
fn accepted_boat_move_reconciles_both_players_and_pairs_the_vehicle_edge() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    world.reset_pending_chunks_for_test(ConnectionId(1), &[]);
    world.reset_pending_chunks_for_test(ConnectionId(2), &[]);
    world.reconcile_player_tracking_for_test(ConnectionId(1));
    world.reconcile_player_tracking_for_test(ConnectionId(2));

    let alice = entity_id_of(&world, 1);
    let bob = entity_id_of(&world, 2);
    let alice_position = world
        .publication()
        .players
        .iter()
        .find(|player| player.entity_id == alice)
        .expect("alice is published")
        .position;
    // The separation is the fixture; the boat edge below is the subject.
    place_player(
        &sender,
        &receiver,
        &mut world,
        2,
        Vec3d {
            x: alice_position.x + 32.2,
            ..alice_position
        },
        None,
    );
    assert!(world
        .player_tracking_viewers_for_test(Java26BuiltinDimension::Overworld, alice)
        .is_empty());
    assert!(world
        .player_tracking_viewers_for_test(Java26BuiltinDimension::Overworld, bob)
        .is_empty());

    let boat = oak_boat(
        &mut world,
        Vec3d {
            y: alice_position.y + 2.0,
            ..alice_position
        },
    );
    world.apply_level_entity_lifecycle_for_test(Java26BuiltinDimension::Overworld, &mut Vec::new());
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
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } => Some(update.id),
            _ => None,
        })
        .expect("mounting sends the rider correction");
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: teleport_id },
    );
    world.reset_pending_chunks_for_test(ConnectionId(1), &[]);
    world.reset_pending_chunks_for_test(ConnectionId(2), &[]);
    let (_, _, baseline, _) = world.external_player_vehicle_listener_for_test(ConnectionId(1));
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    world.set_player_shared_flag_for_test(
        ConnectionId(1),
        VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
        true,
    );
    world.set_player_attribute_base_for_test(
        ConnectionId(1),
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        24.0,
    );
    let moved = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MoveVehicle(MoveVehicleCommand {
            position: Vec3d {
                x: baseline.x + 0.25,
                ..baseline
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        }),
    );

    let alice_add = moved
        .events
        .iter()
        .position(|event| {
            matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::AddEntity(add),
            } if add.id == alice)
        })
        .expect("the accepted vehicle move admits alice to bob");
    assert!(moved.events.iter().any(|event| matches!(event,
        WorldEvent::Send {
            connection_id: ConnectionId(1),
            packet: ClientboundPlayPacket::AddEntity(add),
        } if add.id == bob)));
    let bundle_start = moved.events[..alice_add]
        .iter()
        .rposition(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(2),
                    packet: ClientboundPlayPacket::BundleDelimiter,
                }
            )
        })
        .expect("alice's pairing bundle opens");
    let bundle_end = moved.events[alice_add + 1..]
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(2),
                    packet: ClientboundPlayPacket::BundleDelimiter,
                }
            )
        })
        .map(|offset| alice_add + 1 + offset)
        .expect("alice's pairing bundle closes");
    let pairing_kinds = moved.events[bundle_start..=bundle_end]
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet,
            } => Some(match packet {
                ClientboundPlayPacket::BundleDelimiter => "delimiter",
                ClientboundPlayPacket::AddEntity(_) => "add",
                ClientboundPlayPacket::SetEntityData(_) => "metadata",
                ClientboundPlayPacket::UpdateAttributes(_) => "attributes",
                ClientboundPlayPacket::SetEquipment(_) => "equipment",
                ClientboundPlayPacket::SetPassengers(update)
                    if update.vehicle_id == boat && update.passenger_ids == [alice] =>
                {
                    "passengers"
                }
                _ => "other",
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        pairing_kinds,
        [
            "delimiter",
            "add",
            "metadata",
            "attributes",
            "equipment",
            "passengers",
            "delimiter"
        ]
    );
}

#[test]
fn death_clears_the_chunk_view_before_both_real_pairings_are_removed() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 1);
    drain_chunk_deliveries(&sender, &receiver, &mut world, 2);
    let alice = entity_id_of(&world, 1);
    let bob = entity_id_of(&world, 2);
    join_loaded(&sender, &receiver, &mut world, 3, "charlie", 3);
    let charlie = entity_id_of(&world, 3);
    let origin = world.player_transition_state_for_test(ConnectionId(1)).1;
    place_player(
        &sender,
        &receiver,
        &mut world,
        3,
        Vec3d {
            x: origin.x + 48.0,
            ..origin
        },
        None,
    );
    assert!(world
        .player_tracking_viewers_for_test(Java26BuiltinDimension::Overworld, charlie)
        .is_empty());
    assert_eq!(
        world.player_tracking_viewers_for_test(Java26BuiltinDimension::Overworld, alice),
        [ConnectionId(2)]
    );

    world.prime_player_body_removal_for_test(ConnectionId(1));
    let removed = world.tick(Vec::new());
    let poofs = removed
        .events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::EntityEvent(update),
            } if update.entity_id == alice && update.event_id == 60 => Some(*connection_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(poofs, [ConnectionId(2), ConnectionId(1)]);
    assert_eq!(world.chunk_delivery_state_for_test(ConnectionId(1)).1, None);
    assert!(world
        .player_tracking_viewers_for_test(Java26BuiltinDimension::Overworld, alice)
        .is_empty());
    assert!(world
        .player_tracking_viewers_for_test(Java26BuiltinDimension::Overworld, bob)
        .is_empty());

    assert!(!removed.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::ForgetLevelChunk(_),
            }
        )
    }));
    let poof = removed
        .events
        .iter()
        .position(|event| {
            matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(update), .. }
                    if update.entity_id == alice && update.event_id == 60
            )
        })
        .unwrap();
    let remove_from_dead = removed
        .events
        .iter()
        .position(|event| {
            matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::RemoveEntities(update),
            } if update.entity_ids == [bob])
        })
        .expect("the still-connected dead session unpairs bob");
    let remove_from_bob = removed
        .events
        .iter()
        .position(|event| {
            matches!(event,
            WorldEvent::Send {
                connection_id: ConnectionId(2),
                packet: ClientboundPlayPacket::RemoveEntities(update),
            } if update.entity_ids == [alice])
        })
        .expect("bob unpairs the dead avatar");
    assert!(poof < remove_from_dead && poof < remove_from_bob);
    assert!(!removed.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ConnectionId(3), packet: ClientboundPlayPacket::RemoveEntities(update) }
        if update.entity_ids.contains(&alice) || update.entity_ids.contains(&bob))));
}

#[test]
fn client_information_metadata_pairs_updates_and_returns_to_defaults() {
    use bbb_protocol::entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
    };
    use bbb_protocol::entity_synched_data::{
        VANILLA_AVATAR_MAIN_HAND_DATA_ID, VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
    };
    use bbb_protocol::packets::{ClientMainHand, EntityDataValueKind};
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig::default());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.reset_pending_chunks_for_test(ConnectionId(1), &[]);
    let information = ClientInformation {
        main_hand: ClientMainHand::Left,
        displayed_skin_parts: 0xff,
        ..Default::default()
    };
    enqueue(
        &sender,
        WorldCommand::Join {
            connection_id: ConnectionId(2),
            profile: profile("bob", 2),
            client_information: information.clone(),
            context: JoinContext::default(),
        },
    )
    .unwrap();
    let joined = world.tick(receiver.drain());
    let bob = entity_id_of(&world, 2);
    let rows = |tick: &TickResult, recipient: u64| {
        tick.events
            .iter()
            .filter_map(|event| match event.packet_for(ConnectionId(recipient)) {
                Some(ClientboundPlayPacket::SetEntityData(data)) if data.id == bob => {
                    Some(&data.values)
                }
                _ => None,
            })
            .flatten()
            .filter(|row| {
                matches!(
                    row.data_id,
                    VANILLA_AVATAR_MAIN_HAND_DATA_ID | VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID
                )
            })
            .map(|row| (row.data_id, row.serializer_id, row.value.clone()))
            .collect::<Vec<_>>()
    };
    let left = (
        VANILLA_AVATAR_MAIN_HAND_DATA_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
        EntityDataValueKind::HumanoidArm(ClientMainHand::Left.id()),
    );
    let skin = |value| {
        (
            VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            EntityDataValueKind::Byte(value),
        )
    };
    let paired = |tick: &TickResult| {
        let add = tick.events.iter().position(|event| matches!(event,
            WorldEvent::Send { connection_id: ConnectionId(1), packet: ClientboundPlayPacket::AddEntity(add) }
            if add.id == bob)).expect("bob's pairing starts");
        let end = tick.events[add + 1..]
            .iter()
            .position(|event| {
                matches!(
                    event,
                    WorldEvent::Send {
                        connection_id: ConnectionId(1),
                        packet: ClientboundPlayPacket::BundleDelimiter
                    }
                )
            })
            .unwrap()
            + add
            + 1;
        tick.events[add + 1..end]
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::SetEntityData(data),
                } if data.id == bob => Some(&data.values),
                _ => None,
            })
            .flatten()
            .filter(|row| {
                matches!(
                    row.data_id,
                    VANILLA_AVATAR_MAIN_HAND_DATA_ID | VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID
                )
            })
            .map(|row| (row.data_id, row.serializer_id, row.value.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(paired(&joined), [left.clone(), skin(-1)]);

    let unchanged = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::ClientInformation(information.clone()),
    );
    assert!(rows(&unchanged, 1).is_empty());
    assert!(rows(&unchanged, 2).is_empty());
    world.prime_player_body_removal_for_test(ConnectionId(2));
    world.tick(Vec::new());
    let respawned = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::ClientCommand(ClientCommandAction::PerformRespawn),
    );
    assert_eq!(paired(&respawned), [left.clone(), skin(-1)]);
    assert_eq!(rows(&respawned, 2), [left, skin(-1)]);
    assert!(rows(&world.tick(Vec::new()), 1).is_empty());

    let changed = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::ClientInformation(ClientInformation {
            displayed_skin_parts: 0,
            ..information.clone()
        }),
    );
    for recipient in [1, 2] {
        assert_eq!(rows(&changed, recipient), [skin(0)]);
    }
    let hat = changed
        .events
        .iter()
        .position(|event| {
            matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::PlayerInfoUpdate(update))
                    if update.actions == [PlayerInfoAction::UpdateHat]
            )
        })
        .unwrap();
    let metadata = changed
        .events
        .iter()
        .position(|event| {
            matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::SetEntityData(data)) if data.id == bob
            )
        })
        .unwrap();
    assert!(hat < metadata);

    let right = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::ClientInformation(ClientInformation {
            main_hand: ClientMainHand::Right,
            displayed_skin_parts: 0,
            ..information
        }),
    );
    for recipient in [1, 2] {
        assert_eq!(
            rows(&right, recipient),
            [(
                VANILLA_AVATAR_MAIN_HAND_DATA_ID,
                VANILLA_ENTITY_DATA_SERIALIZER_HUMANOID_ARM_ID,
                EntityDataValueKind::HumanoidArm(ClientMainHand::Right.id())
            )]
        );
    }
    assert!(rows(&world.tick(receiver.drain()), 1).is_empty());
}
