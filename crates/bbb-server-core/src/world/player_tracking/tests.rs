use super::*;
use crate::entity::store::EntityIdCounter;
use crate::entity::tracking::tests::{kind, origin, packets_for, store_with_one_item};
use crate::entity::tracking::{recipient_packets, TrackingPacket};
use crate::entity::{EntityTracker, ExternalPlayerCanRide, TrackingViewer};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, Vec3d};

/// A joined player's state, reused as the template every placed player is
/// cloned from.
fn template() -> PlayerState {
    let mut player = ServerWorld::detached_player_for_test(0, origin(), GameType::Survival);
    // What the join marked dirty is not what these tests publish.
    player.drain_tracking_frame(false);
    player
}

/// Files a player for `connection` at `x` on the z axis.
fn place(template: &PlayerState, store: &mut EntityStore, connection: u64, id: i32, x: f64) {
    let mut player = template.clone();
    let uuid = uuid::Uuid::from_u128(0xB000 + u128::from(connection));
    player.connection_id = ConnectionId(connection);
    player.profile.uuid = uuid;
    player.body = crate::entity::Entity::new(
        id,
        uuid,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        Vec3d { x, ..origin() },
        crate::entity::EntityData::Player,
    );
    store.insert_player(id, uuid, Box::new(player));
}

fn player(store: &mut EntityStore, id: i32) -> &mut PlayerState {
    store.player_mut(id).expect("the placed player")
}

fn viewer(store: &EntityStore, id: i32, view_distance: i32) -> TrackingViewer {
    let player = store.player(id).expect("the placed player");
    TrackingViewer {
        connection_id: player.connection_id,
        position: player.position(),
        view_distance,
        uuid: player.uuid(),
        entity_id: id,
    }
}

fn tick(
    tracker: &mut EntityTracker,
    store: &mut EntityStore,
    viewers: &[TrackingViewer],
) -> Vec<TrackingPacket> {
    let mut out = Vec::new();
    tracker.tick_with_players_and_framed_maps(
        store,
        viewers,
        |_, _, _| true,
        |_, _| true,
        |_| Vec::new(),
        &mut out,
    );
    out
}

#[test]
fn a_new_viewer_pairs_with_the_current_session_pose() {
    let template = template();
    let mut store = EntityStore::new(95, EntityIdCounter::new());
    place(&template, &mut store, 1, 1011, 0.0);
    place(&template, &mut store, 2, 1012, 1.0);
    player(&mut store, 1011).pose_id = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID;
    let target = viewer(&store, 1011, 32);
    let mover = viewer(&store, 1012, 32);
    let mut tracker = EntityTracker::new();
    tracker.add_player(1011, &[target], &store, |_, _, _| true, &mut Vec::new());
    tracker.add_player(
        1012,
        &[target, mover],
        &store,
        |_, _, _| true,
        &mut Vec::new(),
    );
    let mut out = Vec::new();
    tracker.move_player_body(&mover, &[target, mover], &store, |_, _, _| true, &mut out);
    assert!(recipient_packets(&out).any(|(connection, packet)| {
        *connection == ConnectionId(2)
            && matches!(packet,
            ClientboundPlayPacket::SetEntityData(update)
                if update.id == 1011 && update.values.iter().any(|value|
                    value.data_id == bbb_protocol::entity_synched_data::VANILLA_ENTITY_POSE_DATA_ID
                    && value.value == EntityDataValueKind::Pose(
                        bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID)))
    }));
}

/// Whether the viewer at `x` pairs with the target at the origin.
fn player_is_seen(
    template: &PlayerState,
    x: f64,
    view_distance: i32,
    gate: bool,
    stage: impl FnOnce(&mut EntityStore),
) -> bool {
    let mut store = EntityStore::new(91, EntityIdCounter::new());
    place(template, &mut store, 1, 101, 0.0);
    place(template, &mut store, 2, 102, x);
    stage(&mut store);
    let viewers = [viewer(&store, 101, 32), viewer(&store, 102, view_distance)];
    let mut tracker = EntityTracker::new();
    tracker.add_player(101, &viewers, &store, |_, _, _| gate, &mut Vec::new());
    tracker.viewers_of(101) == [ConnectionId(2)]
}

#[test]
fn player_admission_uses_range_view_chunk_self_vertical_and_spectator_gates() {
    let template = template();
    assert!(player_is_seen(&template, 512.0, 32, true, |_| ()));
    assert!(!player_is_seen(&template, 512.01, 32, true, |_| ()));
    assert!(player_is_seen(&template, 160.0, 10, true, |_| ()));
    assert!(!player_is_seen(&template, 160.01, 10, true, |_| ()));
    assert!(player_is_seen(&template, 512.0, 32, true, |store| {
        player(store, 102).set_position(Vec3d {
            x: 512.0,
            y: 10_000.0,
            z: 0.0,
        });
    }));
    assert!(!player_is_seen(&template, 1.0, 32, false, |_| ()));

    let mut store = EntityStore::new(92, EntityIdCounter::new());
    place(&template, &mut store, 1, 101, 0.0);
    let mut tracker = EntityTracker::new();
    tracker.add_player(
        101,
        &[viewer(&store, 101, 32)],
        &store,
        |_, _, _| true,
        &mut Vec::new(),
    );
    assert!(tracker.viewers_of(101).is_empty());

    let spectating_viewer = |store: &mut EntityStore| {
        player(store, 102).game_type = GameType::Spectator;
    };
    assert!(player_is_seen(&template, 1.0, 32, true, spectating_viewer));
    assert!(!player_is_seen(&template, 1.0, 32, true, |store| {
        spectating_viewer(store);
        player(store, 101).cold.camera = Some(crate::commands::CommandCameraTarget::Player {
            uuid: uuid::Uuid::from_u128(7),
        });
    }));
    assert!(!player_is_seen(&template, 1.0, 32, true, |store| {
        player(store, 101).game_type = GameType::Spectator;
    }));
}

#[test]
fn player_pairing_is_ordered_and_uses_latest_metadata_snapshot() {
    let template = template();
    let (mut store, vehicle) = store_with_one_item(origin());
    place(&template, &mut store, 1, 201, 0.0);
    place(&template, &mut store, 2, 202, 1.0);
    let target = player(&mut store, 201);
    target
        .attributes
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .expect("players carry a scale")
        .set_base_value(1.25);
    target.inventory.set_item_in_hand(
        bbb_protocol::packets::InteractionHand::MainHand,
        bbb_protocol::packets::ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    assert!(store.attach_test_player(201, vehicle, ExternalPlayerCanRide::Allowed));
    let target = viewer(&store, 201, 32);
    let mut tracker = EntityTracker::new();
    tracker.add_player(201, &[target], &store, |_, _, _| true, &mut Vec::new());
    player(&mut store, 201).set_shared_flag(
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
        true,
    );
    tick(&mut tracker, &mut store, &[target]);

    let mover = viewer(&store, 202, 32);
    tracker.add_player(
        202,
        &[target, mover],
        &store,
        |_, _, _| true,
        &mut Vec::new(),
    );
    let mut out = Vec::new();
    tracker.move_player_body(&mover, &[target, mover], &store, |_, _, _| true, &mut out);
    assert!(packets_for(&out, 2)
        .iter()
        .filter(
            |packet| !matches!(packet, ClientboundPlayPacket::AddEntity(add) if add.id == vehicle)
        )
        .map(kind)
        .collect::<Vec<_>>()
        .windows(5)
        .any(|window| window == ["add", "data", "attributes", "equipment", "passengers"]));
    assert!(packets_for(&out, 2).iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::SetEntityData(data)
            if data.id == 201 && data.values.iter().any(|row| row.value == EntityDataValueKind::Byte(0x08))
    )));
}

#[test]
fn unified_tick_routes_tracking_and_self_without_duplicating_owner() {
    let template = template();
    let mut store = EntityStore::new(95, EntityIdCounter::new());
    place(&template, &mut store, 1, 301, 0.0);
    place(&template, &mut store, 2, 302, 1.0);
    let viewers = [viewer(&store, 301, 32), viewer(&store, 302, 32)];
    let mut tracker = EntityTracker::new();
    tracker.add_player(301, &viewers, &store, |_, _, _| true, &mut Vec::new());
    let changed = player(&mut store, 301);
    changed.set_position(Vec3d {
        x: 0.25,
        ..origin()
    });
    changed.shared_flags = 1;
    changed.shared_flags_dirty = true;
    changed.set_y_rot(90.0);
    changed
        .attributes
        .get_instance(VANILLA_ATTRIBUTE_SCALE_ID)
        .expect("players carry a scale")
        .set_base_value(1.5);
    let viewers = [viewer(&store, 301, 32), viewer(&store, 302, 32)];
    let out = tick(&mut tracker, &mut store, &viewers);
    let recipients = |kind_name| {
        recipient_packets(&out)
            .filter(|(_, packet)| kind(packet) == kind_name)
            .map(|(connection, _)| *connection)
            .collect::<Vec<_>>()
    };
    assert_eq!(recipients("move"), [ConnectionId(2)]);
    assert_eq!(recipients("head"), [ConnectionId(2)]);
    assert_eq!(recipients("data"), [ConnectionId(2), ConnectionId(1)]);
    assert_eq!(recipients("attributes"), [ConnectionId(2), ConnectionId(1)]);
    for name in ["data", "attributes"] {
        let packets = out
            .iter()
            .filter(|targeted| kind(&targeted.packet) == name)
            .collect::<Vec<_>>();
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].recipients, [ConnectionId(2), ConnectionId(1)]);
    }
}

#[test]
fn same_section_move_reconciles_player_target_and_stored_target_both_directions() {
    let template = template();
    let (mut store, stored_id) = store_with_one_item(Vec3d { x: 8.0, ..origin() });
    place(&template, &mut store, 1, 401, 8.0);
    place(&template, &mut store, 2, 402, 25.0);
    let viewers = |store: &EntityStore| [viewer(store, 401, 1), viewer(store, 402, 1)];
    let mut tracker = EntityTracker::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers(&store),
        |_, _, _| true,
        &mut Vec::new(),
    );
    tracker.add_player(
        401,
        &viewers(&store),
        &store,
        |_, _, _| true,
        &mut Vec::new(),
    );
    tracker.add_player(
        402,
        &viewers(&store),
        &store,
        |_, _, _| true,
        &mut Vec::new(),
    );
    assert!(tracker.viewers_of(401).is_empty());
    assert!(tracker.viewers_of(402).is_empty());
    assert!(!tracker.viewers_of(stored_id).contains(&ConnectionId(2)));

    for (x, paired) in [(24.0, true), (25.0, false)] {
        player(&mut store, 402).set_position(Vec3d { x, ..origin() });
        let current = viewers(&store);
        tracker.move_player_body(
            &current[1],
            &current,
            &store,
            |_, _, _| true,
            &mut Vec::new(),
        );
        assert_eq!(tracker.viewers_of(401) == [ConnectionId(2)], paired);
        assert_eq!(tracker.viewers_of(402) == [ConnectionId(1)], paired);
        assert_eq!(
            tracker.viewers_of(stored_id).contains(&ConnectionId(2)),
            paired
        );
    }
}

#[test]
fn player_removal_unpairs_the_leaver_from_its_viewers() {
    let template = template();
    let (mut store, stored_id) = store_with_one_item(origin());
    place(&template, &mut store, 1, 501, 0.0);
    place(&template, &mut store, 2, 502, 1.0);
    let viewers = [viewer(&store, 501, 32), viewer(&store, 502, 32)];
    let mut tracker = EntityTracker::new();
    tracker.apply_lifecycle_events(
        &store.drain_lifecycle_events(),
        &viewers,
        |_, _, _| true,
        &mut Vec::new(),
    );
    tracker.add_player(501, &viewers, &store, |_, _, _| true, &mut Vec::new());
    tracker.add_player(502, &viewers, &store, |_, _, _| true, &mut Vec::new());
    let mut out = Vec::new();
    tracker.remove_player(ConnectionId(1), 501, &mut out);
    assert_eq!(tracker.len(), 2);
    assert!(tracker.viewers_of(stored_id).contains(&ConnectionId(2)));
    assert!(matches!(
        packets_for(&out, 2).as_slice(),
        [ClientboundPlayPacket::RemoveEntities(remove)] if remove.entity_ids == [501]
    ));
}

#[test]
fn player_environment_metadata_reset_reaches_self_and_observer_once() {
    use bbb_protocol::entity_synched_data::{
        VANILLA_ENTITY_AIR_SUPPLY_DATA_ID, VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
    };

    let template = template();
    let mut store = EntityStore::new(95, EntityIdCounter::new());
    place(&template, &mut store, 1, 1015, 0.0);
    place(&template, &mut store, 2, 1016, 1.0);
    let target = player(&mut store, 1015);
    target.air_supply = 280;
    target.ticks_frozen = 140;
    let viewers = [viewer(&store, 1015, 32), viewer(&store, 1016, 32)];
    let mut tracker = EntityTracker::new();
    tracker.add_player(1015, &viewers, &store, |_, _, _| true, &mut Vec::new());
    // The first publication carries the join's health.
    tick(&mut tracker, &mut store, &viewers);
    let target = player(&mut store, 1015);
    target.air_supply = 300;
    target.air_supply_dirty = true;
    target.ticks_frozen = 0;
    target.ticks_frozen_dirty = true;
    let out = tick(&mut tracker, &mut store, &viewers);
    let updates: Vec<_> = recipient_packets(&out)
        .filter_map(|(connection, packet)| match packet {
            ClientboundPlayPacket::SetEntityData(data) if data.id == 1015 => {
                Some((*connection, data))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        updates
            .iter()
            .map(|(connection, _)| *connection)
            .collect::<Vec<_>>(),
        [ConnectionId(2), ConnectionId(1)]
    );
    for (_, data) in updates {
        assert_eq!(
            data.values
                .iter()
                .map(|row| (row.data_id, row.value.clone()))
                .collect::<Vec<_>>(),
            [
                (
                    VANILLA_ENTITY_AIR_SUPPLY_DATA_ID,
                    EntityDataValueKind::Int(300)
                ),
                (
                    VANILLA_ENTITY_TICKS_FROZEN_DATA_ID,
                    EntityDataValueKind::Int(0)
                )
            ]
        );
    }
    let out = tick(&mut tracker, &mut store, &viewers);
    assert!(recipient_packets(&out).all(|(_, packet)| !matches!(
        packet,
        ClientboundPlayPacket::SetEntityData(data) if data.id == 1015
    )));
}

#[test]
fn player_silent_metadata_pairing_and_dirty_changes_reach_self_and_observer_once() {
    use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
    use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SILENT_DATA_ID;

    let template = template();
    let mut store = EntityStore::new(96, EntityIdCounter::new());
    place(&template, &mut store, 1, 1017, 0.0);
    place(&template, &mut store, 2, 1018, 1.0);
    player(&mut store, 1017).silent = true;
    let viewers = [viewer(&store, 1017, 32), viewer(&store, 1018, 32)];
    let mut tracker = EntityTracker::new();
    let mut out = Vec::new();
    tracker.add_player(1017, &viewers, &store, |_, _, _| true, &mut out);
    let rows = |out: &[TrackingPacket]| {
        recipient_packets(out)
            .filter_map(|(connection, packet)| match packet {
                ClientboundPlayPacket::SetEntityData(data) if data.id == 1017 => Some((
                    *connection,
                    data.values
                        .iter()
                        .filter(|row| row.data_id == VANILLA_ENTITY_SILENT_DATA_ID)
                        .cloned()
                        .collect::<Vec<_>>(),
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let silent_row = |silent| EntityDataValue {
        data_id: VANILLA_ENTITY_SILENT_DATA_ID,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(silent),
    };
    assert_eq!(rows(&out), [(ConnectionId(2), vec![silent_row(true)])]);

    for silent in [false, true, false] {
        let target = player(&mut store, 1017);
        target.silent = silent;
        target.silent_dirty = true;
        assert_eq!(
            rows(&tick(&mut tracker, &mut store, &viewers)),
            [
                (ConnectionId(2), vec![silent_row(silent)]),
                (ConnectionId(1), vec![silent_row(silent)]),
            ]
        );
        assert!(rows(&tick(&mut tracker, &mut store, &viewers)).is_empty());
    }
}

/// The `SetEntityData` float rows `connection` received for `entity_id`.
fn float_rows(out: &[TrackingPacket], connection: u64, entity_id: i32, data_id: u8) -> Vec<f32> {
    packets_for(out, connection)
        .iter()
        .filter_map(|packet| match packet {
            ClientboundPlayPacket::SetEntityData(update) if update.id == entity_id => {
                Some(update.values.clone())
            }
            _ => None,
        })
        .flatten()
        .filter(|value| value.data_id == data_id)
        .filter_map(|value| match value.value {
            EntityDataValueKind::Float(value) => Some(value),
            _ => None,
        })
        .collect()
}

/// `Player.internalSetAbsorptionAmount` and `LivingEntity.setHealth` write
/// synched floats (`Player.java:1695`, `LivingEntity.java:1147`): the golden
/// hearts reach the player itself, and a death reaches the trackers, whose
/// client starts the dying animation from the health row.
#[test]
fn player_absorption_and_health_reach_the_session_and_its_trackers() {
    use bbb_protocol::entity_synched_data::{
        VANILLA_LIVING_ENTITY_HEALTH_DATA_ID, VANILLA_PLAYER_ABSORPTION_DATA_ID,
    };
    let template = template();
    let mut store = EntityStore::new(96, EntityIdCounter::new());
    place(&template, &mut store, 1, 301, 0.0);
    place(&template, &mut store, 2, 302, 1.0);
    let viewers = [viewer(&store, 301, 32), viewer(&store, 302, 32)];
    let mut tracker = EntityTracker::new();
    tracker.add_player(301, &viewers, &store, |_, _, _| true, &mut Vec::new());
    tracker.add_player(302, &viewers, &store, |_, _, _| true, &mut Vec::new());
    tick(&mut tracker, &mut store, &viewers);

    player(&mut store, 301).damage.absorption_amount = 4.0;
    let out = tick(&mut tracker, &mut store, &viewers);
    for connection in [1, 2] {
        assert_eq!(
            float_rows(&out, connection, 301, VANILLA_PLAYER_ABSORPTION_DATA_ID),
            [4.0]
        );
    }
    let out = tick(&mut tracker, &mut store, &viewers);
    assert!(float_rows(&out, 2, 301, VANILLA_PLAYER_ABSORPTION_DATA_ID).is_empty());

    player(&mut store, 301).damage.health = 0.0;
    let out = tick(&mut tracker, &mut store, &viewers);
    assert_eq!(
        float_rows(&out, 2, 301, VANILLA_LIVING_ENTITY_HEALTH_DATA_ID),
        [0.0]
    );
}
