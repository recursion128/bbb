use super::*;

use crate::entity::living::mob_effects::ActiveMobEffect;
use bbb_protocol::packets::{TrackedWaypointPacket, WaypointIdentifier, WaypointOperation};

fn waypoint_packets(events: &[WorldEvent], connection_id: u64) -> Vec<&TrackedWaypointPacket> {
    events
        .iter()
        .filter_map(
            |event| match event.packet_for(ConnectionId(connection_id)) {
                Some(ClientboundPlayPacket::TrackedWaypoint(packet)) => Some(packet),
                _ => None,
            },
        )
        .collect()
}

fn packet_for(
    events: &[WorldEvent],
    connection_id: u64,
    operation: WaypointOperation,
    uuid: u128,
) -> Option<&TrackedWaypointPacket> {
    waypoint_packets(events, connection_id)
        .into_iter()
        .find(|packet| {
            packet.operation == operation
                && packet.waypoint.identifier == WaypointIdentifier::Uuid(Uuid::from_u128(uuid))
        })
}

#[test]
fn live_players_follow_join_move_gamerule_team_crouch_and_disconnect_waypoint_edges() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let (bob_join, bob_handshake) = join_loaded_ticks(&sender, &receiver, &mut world, 2, "bob", 2);
    let mut bob_join_events = bob_join.events;
    bob_join_events.extend(bob_handshake.events);
    assert!(packet_for(&bob_join_events, 2, WaypointOperation::Track, 1).is_some());
    assert!(packet_for(&bob_join_events, 1, WaypointOperation::Track, 2).is_some());

    let bob_position = world
        .publication()
        .players
        .iter()
        .find(|player| player.connection_id == ConnectionId(2))
        .expect("bob is connected")
        .position;
    let moved = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d {
                x: bob_position.x + 1.0,
                ..bob_position
            }),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    assert!(packet_for(&moved.events, 1, WaypointOperation::Update, 2).is_some());

    let disabled = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule locator_bar false",
    );
    for (connection_id, other_uuid) in [(1, 2), (2, 1)] {
        let packets = waypoint_packets(&disabled.events, connection_id);
        assert!(
            packet_for(
                &disabled.events,
                connection_id,
                WaypointOperation::Untrack,
                other_uuid,
            )
            .is_some(),
            "connection {connection_id}: {packets:?}; events={:?}",
            disabled.events
        );
    }

    let enabled = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "gamerule locator_bar true",
    );
    for (connection_id, other_uuid) in [(1, 2), (2, 1)] {
        assert!(packet_for(
            &enabled.events,
            connection_id,
            WaypointOperation::Track,
            other_uuid,
        )
        .is_some());
    }

    chat_command(&sender, &receiver, &mut world, 1, "team add red");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "team modify red color black",
    );
    let joined_team = chat_command(&sender, &receiver, &mut world, 1, "team join red bob");
    let team_packet_index = joined_team
        .events
        .iter()
        .position(|event| {
            matches!(
                event.packet_for(ConnectionId(1)),
                Some(ClientboundPlayPacket::SetPlayerTeam(_))
            )
        })
        .expect("alice receives the team packet");
    let waypoint_index = joined_team
        .events
        .iter()
        .position(|event| {
            matches!(
                event.packet_for(ConnectionId(1)),
                Some(ClientboundPlayPacket::TrackedWaypoint(packet))
                    if packet.operation == WaypointOperation::Track
                        && packet.waypoint.identifier
                            == WaypointIdentifier::Uuid(Uuid::from_u128(2))
                        && packet.waypoint.icon.color_rgb == Some(0x30_30_30)
            )
        })
        .expect("the team color remakes bob's waypoint");
    assert!(team_packet_index < waypoint_index);

    let mut effect_events = Vec::new();
    assert_eq!(
        world.add_player_mob_effect(
            ConnectionId(2),
            ActiveMobEffect::new("minecraft:invisibility", 200, 0)
                .expect("invisibility is built in"),
            &mut effect_events,
        ),
        Some(true)
    );
    assert!(packet_for(&effect_events, 1, WaypointOperation::Untrack, 2).is_none());
    let invisibility_applied = world.tick(receiver.drain());
    assert!(packet_for(
        &invisibility_applied.events,
        1,
        WaypointOperation::Untrack,
        2,
    )
    .is_some());
    effect_events.clear();
    assert!(world
        .remove_player_mob_effect(
            ConnectionId(2),
            "minecraft:invisibility",
            &mut effect_events,
        )
        .is_some());
    assert!(packet_for(&effect_events, 1, WaypointOperation::Track, 2).is_some());

    let bob_entity_id = entity_id_of(&world, 2);
    let crouched = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::Interact(InteractEntity {
            entity_id: bob_entity_id.wrapping_add(10_000),
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: true,
        }),
    );
    assert!(packet_for(&crouched.events, 1, WaypointOperation::Untrack, 2).is_none());
    let crouch_applied = world.tick(receiver.drain());
    assert!(packet_for(&crouch_applied.events, 1, WaypointOperation::Untrack, 2).is_some());

    let released = play(
        &sender,
        &receiver,
        &mut world,
        2,
        PlayServerbound::Interact(InteractEntity {
            entity_id: bob_entity_id.wrapping_add(10_000),
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    );
    assert!(packet_for(&released.events, 1, WaypointOperation::Track, 2).is_none());
    let crouch_released = world.tick(receiver.drain());
    assert!(packet_for(&crouch_released.events, 1, WaypointOperation::Track, 2).is_some());

    let disconnected = world.remove_player_for_test(ConnectionId(2));
    assert!(packet_for(&disconnected, 1, WaypointOperation::Untrack, 2).is_some());
}

#[test]
fn stored_living_entity_tracks_updates_and_untracks_on_the_live_level_route() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let player_position = world.publication().players[0].position;
    let pig_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
    let mut pig =
        crate::entity::GeneralLivingData::new(pig_type).expect("a pig owns living attributes");
    let attributes = pig.living_mut().attributes_mut();
    attributes
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID)
        .expect("a pig supplier declares waypoint transmit range")
        .set_base_value(48.0);
    attributes.drain_attributes_to_update();
    let (pig_id, pig_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        pig_type,
        Vec3d {
            x: player_position.x + 2.0,
            ..player_position
        },
        crate::entity::EntityData::GeneralLiving(pig),
    );

    let first_tick = world.tick(receiver.drain());
    assert!(packet_for(
        &first_tick.events,
        1,
        WaypointOperation::Track,
        pig_uuid.as_u128(),
    )
    .is_none());

    world.set_stored_entity_position_for_test(
        Java26BuiltinDimension::Overworld,
        pig_id,
        Vec3d {
            x: player_position.x + 3.0,
            ..player_position
        },
    );
    let tracked = world.tick(receiver.drain());
    assert!(packet_for(
        &tracked.events,
        1,
        WaypointOperation::Track,
        pig_uuid.as_u128(),
    )
    .is_some());

    world.set_stored_entity_position_for_test(
        Java26BuiltinDimension::Overworld,
        pig_id,
        Vec3d {
            x: player_position.x + 4.0,
            ..player_position
        },
    );
    let moved = world.tick(receiver.drain());
    assert!(packet_for(
        &moved.events,
        1,
        WaypointOperation::Update,
        pig_uuid.as_u128(),
    )
    .is_some());

    assert!(world.remove_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        pig_id,
        crate::entity::RemovalReason::Discarded,
    ));
    let removed = world.tick(receiver.drain());
    assert!(packet_for(
        &removed.events,
        1,
        WaypointOperation::Untrack,
        pig_uuid.as_u128(),
    )
    .is_some());
}

#[test]
fn waypoint_command_mutates_player_before_feedback_and_retracks_for_observers() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    join_loaded(&sender, &receiver, &mut world, 3, "carol", 3);

    let changed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "waypoint modify bob color hex A1b2C3",
    );
    assert_eq!(
        world.player_waypoint_icon_for_test(ConnectionId(2)),
        &bbb_protocol::packets::WaypointIcon {
            style: "minecraft:default".to_owned(),
            color_rgb: Some(0xa1_b2_c3),
        }
    );
    for observer in [1, 3] {
        let packets = waypoint_packets(&changed.events, observer)
            .into_iter()
            .filter(|packet| {
                packet.waypoint.identifier == WaypointIdentifier::Uuid(Uuid::from_u128(2))
            })
            .collect::<Vec<_>>();
        assert_eq!(packets.len(), 2, "observer {observer}: {packets:?}");
        assert_eq!(packets[0].operation, WaypointOperation::Untrack);
        assert_eq!(packets[1].operation, WaypointOperation::Track);
        assert_eq!(packets[1].waypoint.icon.color_rgb, Some(0xa1_b2_c3));
    }
    let last_track = changed
        .events
        .iter()
        .rposition(|event| {
            matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::TrackedWaypoint(packet))
                    if packet.operation == WaypointOperation::Track
                        && packet.waypoint.identifier
                            == WaypointIdentifier::Uuid(Uuid::from_u128(2))
            )
        })
        .unwrap();
    let feedback = changed
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::SystemChat(chat),
                } if chat.content.text().contains("commands.waypoint.modify.color")
            )
        })
        .unwrap();
    assert!(last_track < feedback);

    let aliased_reset = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "waypoint modify bob color r_e_s_e_t",
    );
    assert_eq!(
        world.player_waypoint_icon_for_test(ConnectionId(2)),
        &bbb_protocol::packets::WaypointIcon {
            style: "minecraft:default".to_owned(),
            color_rgb: Some(0xa1_b2_c3),
        }
    );
    for observer in [1, 3] {
        let packets = waypoint_packets(&aliased_reset.events, observer)
            .into_iter()
            .filter(|packet| {
                packet.waypoint.identifier == WaypointIdentifier::Uuid(Uuid::from_u128(2))
            })
            .collect::<Vec<_>>();
        assert_eq!(packets.len(), 1, "observer {observer}: {packets:?}");
        assert_eq!(packets[0].operation, WaypointOperation::Untrack);
    }
    let bob = crate::commands::CommandAttributeTarget::Player(
        crate::admin::NameAndId {
            id: Uuid::from_u128(2),
            name: "bob".into(),
        }
        .into(),
    );
    assert!(
        !world.waypoint_manager_tracks_target_for_test(Java26BuiltinDimension::Overworld, &bob,)
    );
    let alias_untrack = aliased_reset
        .events
        .iter()
        .position(|event| {
            matches!(
                event.sent_packet(),
                Some(ClientboundPlayPacket::TrackedWaypoint(packet))
                    if packet.operation == WaypointOperation::Untrack
                        && packet.waypoint.identifier
                            == WaypointIdentifier::Uuid(Uuid::from_u128(2))
            )
        })
        .unwrap();
    let alias_feedback = aliased_reset
        .events
        .iter()
        .position(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::SystemChat(chat),
                } if chat.content.text().contains("command.failed")
            )
        })
        .unwrap();
    assert!(alias_untrack < alias_feedback);

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "waypoint modify bob color reset",
    );
    assert_eq!(
        world
            .player_waypoint_icon_for_test(ConnectionId(2))
            .color_rgb,
        None
    );
    assert!(world.waypoint_manager_tracks_target_for_test(Java26BuiltinDimension::Overworld, &bob));

    world.set_player_waypoint_transmit_range_for_test(ConnectionId(2), 0.0);
    world.tick(receiver.drain());
    assert!(
        !world.waypoint_manager_tracks_target_for_test(Java26BuiltinDimension::Overworld, &bob,)
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "waypoint modify bob style set forced",
    );
    assert!(world.waypoint_manager_tracks_target_for_test(Java26BuiltinDimension::Overworld, &bob,));

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in minecraft:the_nether run waypoint modify bob style reset",
    );
    assert!(world.waypoint_manager_tracks_target_for_test(Java26BuiltinDimension::Nether, &bob));
    let listed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in minecraft:the_nether run waypoint list",
    );
    assert!(listed.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SystemChat(chat),
            } if chat.content.text().contains("commands.waypoint.list.success")
                && chat.content.text().contains("bob")
        )
    }));
}

#[test]
fn waypoint_command_mutates_and_retracks_a_stored_living_entity() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let position = world.publication().players[0].position;
    let pig_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
    let mut pig = crate::entity::GeneralLivingData::new(pig_type).unwrap();
    pig.living_mut()
        .attributes_mut()
        .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID)
        .unwrap()
        .set_base_value(48.0);
    let (pig_id, pig_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        pig_type,
        Vec3d {
            x: position.x + 2.0,
            ..position
        },
        crate::entity::EntityData::GeneralLiving(pig),
    );
    world.tick(receiver.drain());
    world.set_stored_entity_position_for_test(
        Java26BuiltinDimension::Overworld,
        pig_id,
        Vec3d {
            x: position.x + 3.0,
            ..position
        },
    );
    world.tick(receiver.drain());

    let changed = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("waypoint modify {pig_uuid} style set custom/pig"),
    );
    assert_eq!(
        world
            .stored_waypoint_icon_for_test(Java26BuiltinDimension::Overworld, pig_id)
            .style,
        "minecraft:custom/pig"
    );
    let packets = waypoint_packets(&changed.events, 1)
        .into_iter()
        .filter(|packet| packet.waypoint.identifier == WaypointIdentifier::Uuid(pig_uuid))
        .collect::<Vec<_>>();
    assert_eq!(packets.len(), 2, "{packets:?}");
    assert_eq!(packets[0].operation, WaypointOperation::Untrack);
    assert_eq!(packets[1].operation, WaypointOperation::Track);
    assert_eq!(packets[1].waypoint.icon.style, "minecraft:custom/pig");
}

#[test]
fn waypoint_command_keeps_a_cross_dimension_stored_target_dimension_scoped() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let position = world.publication().players[0].position;
    let pig_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
    let (overworld_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        pig_type,
        position,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(pig_type).unwrap(),
        ),
    );
    let (nether_id, nether_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Nether,
        pig_type,
        position,
        crate::entity::EntityData::GeneralLiving(
            crate::entity::GeneralLivingData::new(pig_type).unwrap(),
        ),
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("waypoint modify {nether_uuid} style set nether/pig"),
    );
    assert_eq!(
        world
            .stored_waypoint_icon_for_test(Java26BuiltinDimension::Nether, nether_id)
            .style,
        "minecraft:nether/pig"
    );
    assert_eq!(
        world
            .stored_waypoint_icon_for_test(Java26BuiltinDimension::Overworld, overworld_id)
            .style,
        "minecraft:default"
    );
    let target = crate::commands::CommandAttributeTarget::Stored {
        dimension: Java26BuiltinDimension::Nether.into(),
        entity_id: nether_id,
    };
    assert!(
        world.waypoint_manager_tracks_target_for_test(Java26BuiltinDimension::Overworld, &target,)
    );
    let listed = chat_command(&sender, &receiver, &mut world, 1, "waypoint list");
    assert!(listed.events.iter().any(|event| {
        matches!(
            event,
            WorldEvent::Send {
                connection_id: ConnectionId(1),
                packet: ClientboundPlayPacket::SystemChat(chat),
            } if chat.content.text().contains("commands.waypoint.list.success")
                && chat.content.text().contains("entity.minecraft.pig")
        )
    }));
}
