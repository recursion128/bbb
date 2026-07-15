use std::collections::BTreeMap;

use crate::{ItemEquipmentSlot, LocalPlayerPoseState};

use super::*;

const TEST_SADDLE_ITEM_ID: i32 = 8_901;

#[test]
fn tracks_entity_passenger_updates() {
    let mut store = WorldStore::new();
    for id in [10, 20, 21, 30] {
        store.apply_add_entity(protocol_add_entity(id));
    }

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20, 21, 999, 20],
    }));
    assert_eq!(store.probe_entity(10).unwrap().passengers, vec![20, 21]);
    assert_eq!(store.probe_entity(20).unwrap().vehicle_id, Some(10));
    assert_eq!(store.probe_entity(21).unwrap().vehicle_id, Some(10));
    assert_eq!(store.entities.mount(10).unwrap().passengers, vec![20, 21]);
    assert_eq!(store.entities.mount(20).unwrap().vehicle_id, Some(10));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![20],
    }));
    assert_eq!(store.probe_entity(10).unwrap().passengers, vec![21]);
    assert_eq!(store.probe_entity(20).unwrap().vehicle_id, Some(30));
    assert_eq!(store.probe_entity(30).unwrap().passengers, vec![20]);
    assert_eq!(store.entities.mount(20).unwrap().vehicle_id, Some(30));
    assert_eq!(store.entities.mount(30).unwrap().passengers, vec![20]);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: Vec::new(),
    }));
    assert!(store.probe_entity(10).unwrap().passengers.is_empty());
    assert_eq!(store.probe_entity(21).unwrap().vehicle_id, None);

    assert!(!store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 999,
        passenger_ids: vec![21],
    }));
    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![30],
        }),
        1
    );
    assert_eq!(store.probe_entity(20).unwrap().vehicle_id, None);
    assert!(store.probe_entity(30).is_none());
    assert_eq!(store.entities.mount(20).unwrap().vehicle_id, None);

    assert_eq!(store.counters().entity_passenger_updates_received, 4);
    assert_eq!(store.counters().entity_passenger_ids_received, 6);
    assert_eq!(store.counters().entity_passenger_updates_applied, 3);
    assert_eq!(store.counters().entity_passenger_updates_ignored, 1);
}

#[test]
fn debug_passenger_vehicle_targets_follow_vanilla_boat_attachment_slab() {
    let mut store = WorldStore::new();
    store.apply_add_entity(ProtocolAddEntity {
        y_rot: 90.0,
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_OAK_BOAT_ID)
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        21,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20, 21],
    }));

    let front = store
        .entity_debug_passenger_vehicle_target(20)
        .expect("front passenger target");
    assert_entity_vec3_close(
        front.position,
        EntityVec3 {
            x: 0.6,
            y: 64.1875,
            z: -2.0,
        },
    );
    assert_pick_bounds_close(
        Some(front.bounds),
        EntityPickBoundsState::from_base_size(0.4, 0.0625, 0.0),
    );

    let rear = store
        .entity_debug_passenger_vehicle_target(21)
        .expect("rear passenger target");
    assert_entity_vec3_close(
        rear.position,
        EntityVec3 {
            x: 1.4,
            y: 64.1875,
            z: -2.0,
        },
    );
    assert_pick_bounds_close(
        Some(rear.bounds),
        EntityPickBoundsState::from_base_size(0.9, 0.0625, 0.0),
    );

    store.apply_add_entity(protocol_add_entity_with_type(
        11,
        VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        22,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 11,
        passenger_ids: vec![22],
    }));
    let raft = store
        .entity_debug_passenger_vehicle_target(22)
        .expect("raft passenger target");
    assert!((raft.position.y - 64.5).abs() < 1.0e-6);
    assert_eq!(store.entity_debug_passenger_vehicle_target(10), None);
}

#[test]
fn tracks_local_player_passenger_without_entity() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    for id in [10, 20, 30] {
        store.apply_add_entity(protocol_add_entity(id));
    }

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99, 20],
    }));
    assert_eq!(store.local_player_id(), Some(99));
    assert_eq!(store.local_player_vehicle_id(), Some(10));
    assert!(store.probe_entity(99).is_none());
    assert_eq!(store.probe_entity(10).unwrap().passengers, vec![99, 20]);
    assert_eq!(store.probe_entity(20).unwrap().vehicle_id, Some(10));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_vehicle_id(), Some(30));
    assert_eq!(store.probe_entity(10).unwrap().passengers, vec![20]);
    assert_eq!(store.probe_entity(30).unwrap().passengers, vec![99]);

    assert!(!store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 999,
        passenger_ids: Vec::new(),
    }));
    assert_eq!(store.local_player_vehicle_id(), Some(30));
    assert_eq!(store.probe_entity(30).unwrap().passengers, vec![99]);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: Vec::new(),
    }));
    assert_eq!(store.local_player_vehicle_id(), None);
    assert!(store.probe_entity(30).unwrap().passengers.is_empty());

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    store.apply_login(&protocol_play_login(100));
    assert_eq!(store.local_player_id(), Some(100));
    assert_eq!(store.local_player_vehicle_id(), None);
    assert_eq!(
        store.probe_entity(10).unwrap().passengers,
        Vec::<i32>::new()
    );
}

#[test]
fn first_local_boat_mount_aligns_yaw_and_shows_dismount_overlay_once() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 12.0,
        ..LocalPlayerPoseState::default()
    });
    store.apply_add_entity(ProtocolAddEntity {
        y_rot: 75.0,
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_OAK_BOAT_ID)
    });
    store.apply_add_entity(ProtocolAddEntity {
        y_rot: 12.0,
        y_head_rot: -30.0,
        ..protocol_add_entity_with_type(99, VANILLA_ENTITY_TYPE_PLAYER_ID)
    });

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_pose().unwrap().y_rot, 75.0);
    let local_player = store.probe_entity(99).unwrap();
    assert_eq!(local_player.y_rot, 75.0);
    assert_eq!(local_player.y_head_rot, 75.0);
    assert_eq!(
        store.action_bar(),
        Some(&crate::ActionBarState {
            content: "Press Left Shift to Dismount".to_string(),
            display_ticks: 60,
            animate_color: false,
        })
    );

    store.advance_hud_text_ticks(50);
    let mut pose = store.local_player_pose().unwrap();
    pose.y_rot = -45.0;
    store.set_local_player_pose(pose);
    store
        .entities
        .with_transform_mut(99, |transform| transform.y_head_rot = -20.0);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_pose().unwrap().y_rot, -45.0);
    assert_eq!(store.probe_entity(99).unwrap().y_head_rot, -20.0);
    assert_eq!(store.action_bar().unwrap().display_ticks, 10);
}

#[test]
fn first_local_non_boat_mount_shows_overlay_without_changing_yaw() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 12.0,
        ..LocalPlayerPoseState::default()
    });
    store.apply_add_entity(ProtocolAddEntity {
        y_rot: 75.0,
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_MINECART_ID)
    });

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_pose().unwrap().y_rot, 12.0);
    assert_eq!(
        store.action_bar().unwrap().content,
        "Press Left Shift to Dismount"
    );
}

#[test]
fn remote_boat_passenger_does_not_change_local_mount_presentation() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 12.0,
        ..LocalPlayerPoseState::default()
    });
    store.apply_add_entity(ProtocolAddEntity {
        y_rot: 75.0,
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_OAK_BOAT_ID)
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    assert_eq!(store.local_player_pose().unwrap().y_rot, 12.0);
    assert_eq!(store.action_bar(), None);
}

#[test]
fn reparenting_within_existing_vehicle_chain_does_not_replay_first_mount() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 12.0,
        ..LocalPlayerPoseState::default()
    });
    store.apply_add_entity(ProtocolAddEntity {
        y_rot: 75.0,
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_OAK_BOAT_ID)
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    store.advance_hud_text_ticks(50);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_pose().unwrap().y_rot, 12.0);
    assert_eq!(store.action_bar().unwrap().display_ticks, 10);
}

#[test]
fn removed_player_vehicle_teleport_recovers_the_local_player() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.1,
            y: 0.0,
            z: 0.2,
        },
        y_rot: 10.0,
        x_rot: 5.0,
        ..LocalPlayerPoseState::default()
    });
    store.apply_add_entity(protocol_add_entity(10));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![10],
        }),
        1
    );
    assert_eq!(store.local_player_vehicle_id(), None);
    assert_eq!(store.removed_player_vehicle_id(), Some(10));

    let (applied, recovery) = store.apply_teleport_entity_with_recovery(ProtocolTeleportEntity {
        id: 10,
        position: ProtocolVec3d {
            x: 8.0,
            y: 70.0,
            z: -4.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.5,
            z: 0.0,
        },
        y_rot: 90.0,
        x_rot: -15.0,
        relatives_mask: 0,
        on_ground: true,
    });
    assert!(applied);
    let recovery = recovery.unwrap();
    assert_eq!(recovery.removed_vehicle_id, 10);
    assert_eq!(recovery.player_pose.position.x, 8.0);
    assert_eq!(recovery.player_pose.position.y, 70.0);
    assert_eq!(recovery.player_pose.position.z, -4.0);
    assert_eq!(recovery.player_pose.y_rot, 90.0);
    assert_eq!(recovery.player_pose.x_rot, -15.0);
    assert_eq!(store.removed_player_vehicle_id(), Some(10));

    store.apply_add_entity(protocol_add_entity(10));
    assert_eq!(store.removed_player_vehicle_id(), None);
}

#[test]
fn local_player_root_boat_vehicle_id_tracks_vanilla_boat_roots() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        30,
        VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_root_vehicle_id(), Some(10));
    assert_eq!(store.local_player_root_boat_vehicle_id(), None);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_root_vehicle_id(), Some(20));
    assert_eq!(store.local_player_root_boat_vehicle_id(), Some(20));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![20],
    }));
    assert_eq!(store.local_player_root_vehicle_id(), Some(30));
    assert_eq!(store.local_player_root_boat_vehicle_id(), Some(30));
}

#[test]
fn local_player_rideable_jumping_vehicle_id_tracks_vanilla_controlled_mounts() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(
        TEST_SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        30,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        40,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_rideable_jumping_vehicle_id(),
        None,
        "vanilla PlayerRideableJumping.canJump requires a saddle"
    );
    equip_test_saddle(&mut store, 10);
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), Some(10));

    equip_test_saddle(&mut store, 20);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), Some(20));

    equip_test_saddle(&mut store, 30);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), Some(30));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 40,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), None);
}

#[test]
fn local_player_rideable_jumping_vehicle_cooldown_tracks_camel_dash_cooldown() {
    const CAMEL_DASH_DATA_ID: u8 = 19;

    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(
        TEST_SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    equip_test_saddle(&mut store, 10);
    equip_test_saddle(&mut store, 20);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_rideable_jumping_vehicle_cooldown(1.0),
        Some(0.0)
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_rideable_jumping_vehicle_cooldown(1.0),
        Some(0.0)
    );
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 20,
        values: vec![ProtocolEntityDataValue {
            data_id: CAMEL_DASH_DATA_ID,
            serializer_id: 8,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    store.advance_entity_client_animations(1);

    assert_eq!(
        store.local_player_rideable_jumping_vehicle_cooldown(1.0),
        Some(53.0)
    );
}

fn equip_test_saddle(store: &mut WorldStore, entity_id: i32) {
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Saddle,
            item: item_stack(TEST_SADDLE_ITEM_ID, 1),
        }],
    }));
}

#[test]
fn local_player_rideable_jumping_vehicle_requires_controlling_passenger() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(
        TEST_SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    equip_test_saddle(&mut store, 10);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![123, 99],
    }));

    assert_eq!(store.local_player_vehicle_id(), Some(10));
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), None);
}

#[test]
fn local_player_sprintable_vehicle_tracks_vanilla_controlled_camels() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        30,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        40,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_sprintable_vehicle_id(), Some(10));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_sprintable_vehicle_id(), Some(20));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_sprintable_vehicle_id(), None);

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 40,
        passenger_ids: vec![99],
    }));
    assert_eq!(store.local_player_sprintable_vehicle_id(), None);
}

#[test]
fn local_player_sprintable_vehicle_requires_controlling_passenger() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        123,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![123, 99],
    }));

    assert_eq!(store.local_player_vehicle_id(), Some(10));
    assert_eq!(store.local_player_sprintable_vehicle_id(), None);
}

#[test]
fn local_player_server_controlled_inventory_vehicle_tracks_vanilla_types() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        30,
        VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        40,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_server_controlled_inventory_vehicle_id(),
        Some(10)
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_server_controlled_inventory_vehicle_id(),
        Some(20)
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_server_controlled_inventory_vehicle_id(),
        Some(30)
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 40,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_server_controlled_inventory_vehicle_id(),
        None
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 50,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        store.local_player_server_controlled_inventory_vehicle_id(),
        None
    );
}

#[test]
fn local_boat_input_advances_root_boat_and_reports_move() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let initial = store.probe_entity(20).unwrap();

    let report = store
        .advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                right: true,
                ..crate::LocalPlayerInputState::default()
            },
            0.05,
        )
        .unwrap();

    assert_eq!(report.vehicle_id, 20);
    assert!(!report.snapped);
    assert!(report.y_rot > initial.y_rot);
    assert!(report.position.z > initial.position.z);
    assert_eq!(report.position, store.probe_entity(20).unwrap().position);
    assert_eq!(store.local_player_root_boat_vehicle_id(), Some(20));
}

#[test]
fn local_boat_input_ignores_unfocused_controls() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let initial = store.probe_entity(20).unwrap();

    let report = store
        .advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: false,
                forward: true,
                right: true,
                ..crate::LocalPlayerInputState::default()
            },
            0.05,
        )
        .unwrap();

    assert_eq!(report.position, initial.position);
    assert_eq!(report.y_rot, initial.y_rot);
}

#[test]
fn move_vehicle_snaps_root_vehicle_and_returns_ack() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity(10));
    store.apply_add_entity(protocol_add_entity(20));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));

    let report = store
        .apply_move_vehicle(ProtocolMoveVehicle {
            position: ProtocolVec3d {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
        })
        .unwrap();

    assert_eq!(store.local_player_vehicle_id(), Some(20));
    assert_eq!(store.local_player_root_vehicle_id(), Some(10));
    assert_eq!(
        report,
        VehicleMoveReport {
            vehicle_id: 10,
            position: EntityVec3 {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
            on_ground: false,
            snapped: true,
        }
    );
    let root = store.probe_entity(10).unwrap();
    assert_eq!(root.position, report.position);
    assert_eq!(root.position_base, report.position);
    assert_eq!(root.y_rot, 45.0);
    assert_eq!(root.x_rot, -5.0);
    assert_eq!(
        store.probe_entity(20).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(store.counters().vehicle_moves_received, 1);
    assert_eq!(store.counters().vehicle_moves_applied, 1);
    assert_eq!(store.counters().vehicle_moves_acked, 1);
    assert_eq!(store.counters().vehicle_moves_snapped, 1);
    assert_eq!(store.counters().vehicle_moves_ignored, 0);
}

#[test]
fn move_vehicle_without_mount_is_noop() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity(10));

    assert_eq!(
        store.apply_move_vehicle(ProtocolMoveVehicle {
            position: ProtocolVec3d {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
        }),
        None
    );

    let entity = store.probe_entity(10).unwrap();
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(store.counters().vehicle_moves_received, 1);
    assert_eq!(store.counters().vehicle_moves_applied, 0);
    assert_eq!(store.counters().vehicle_moves_acked, 0);
    assert_eq!(store.counters().vehicle_moves_snapped, 0);
    assert_eq!(store.counters().vehicle_moves_ignored, 1);
}

#[test]
fn noncontrolling_passenger_cannot_apply_vehicle_correction_or_drive_boat() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity(123));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![123, 99],
    }));
    let initial = store.probe_entity(10).unwrap();

    assert_eq!(store.local_player_root_boat_vehicle_id(), Some(10));
    assert_eq!(
        store.local_player_first_passenger_root_boat_vehicle_id(),
        None
    );
    assert_eq!(
        store.apply_move_vehicle(ProtocolMoveVehicle {
            position: ProtocolVec3d {
                x: 5.0,
                y: 66.0,
                z: -7.0,
            },
            y_rot: 45.0,
            x_rot: -5.0,
        }),
        None
    );
    assert_eq!(
        store.advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            0.05,
        ),
        None
    );
    assert_eq!(store.probe_entity(10).unwrap(), initial);
    assert_eq!(store.counters().vehicle_moves_ignored, 1);
}

#[test]
fn local_boat_relative_move_only_advances_server_position_base() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    let initial = store.probe_entity(10).unwrap();

    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 10,
        delta_x: 4096,
        delta_y: 0,
        delta_z: -2048,
        y_rot: Some(90.0),
        x_rot: Some(30.0),
        on_ground: true,
    }));

    let moved = store.probe_entity(10).unwrap();
    assert_eq!(moved.position, initial.position);
    assert_eq!(moved.y_rot, initial.y_rot);
    assert_eq!(moved.x_rot, initial.x_rot);
    assert_eq!(moved.on_ground, initial.on_ground);
    assert_eq!(moved.position_base.x, initial.position_base.x + 1.0);
    assert_eq!(moved.position_base.y, initial.position_base.y);
    assert_eq!(moved.position_base.z, initial.position_base.z - 0.5);
}

#[test]
fn local_boat_position_sync_only_rebases_server_position_codec() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    let initial = store.probe_entity(10).unwrap();

    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 10,
            position: ProtocolVec3d {
                x: 8.0,
                y: 70.0,
                z: -9.0,
            },
            delta_movement: ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            y_rot: 90.0,
            x_rot: 30.0,
            on_ground: true,
        })
    );

    let synced = store.probe_entity(10).unwrap();
    assert_eq!(synced.position, initial.position);
    assert_eq!(synced.delta_movement, initial.delta_movement);
    assert_eq!(synced.y_rot, initial.y_rot);
    assert_eq!(synced.x_rot, initial.x_rot);
    assert_eq!(synced.on_ground, initial.on_ground);
    assert_eq!(
        synced.position_base,
        EntityVec3 {
            x: 8.0,
            y: 70.0,
            z: -9.0
        }
    );
}

#[test]
fn move_vehicle_small_delta_acks_without_snap() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity(10));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));

    let report = store
        .apply_move_vehicle(ProtocolMoveVehicle {
            position: ProtocolVec3d {
                x: 1.000001,
                y: 64.0,
                z: -2.0,
            },
            y_rot: 80.0,
            x_rot: 35.0,
        })
        .unwrap();

    assert_eq!(
        report,
        VehicleMoveReport {
            vehicle_id: 10,
            position: EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            y_rot: 20.0,
            x_rot: -10.0,
            on_ground: false,
            snapped: false,
        }
    );
    let entity = store.probe_entity(10).unwrap();
    assert_eq!(entity.position, report.position);
    assert_eq!(entity.y_rot, 20.0);
    assert_eq!(entity.x_rot, -10.0);
    assert_eq!(store.counters().vehicle_moves_received, 1);
    assert_eq!(store.counters().vehicle_moves_applied, 1);
    assert_eq!(store.counters().vehicle_moves_acked, 1);
    assert_eq!(store.counters().vehicle_moves_snapped, 0);
    assert_eq!(store.counters().vehicle_moves_ignored, 0);
}

#[test]
fn minecart_lerp_component_only_attached_to_minecart_entities() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    // `protocol_add_entity` spawns a non-minecart type.
    store.apply_add_entity(protocol_add_entity(20));

    // Only the minecart carries the lerp component; the plain entity does not.
    assert!(store.entities.minecart_lerp(10).is_some());
    assert!(store.entities.minecart_lerp(20).is_none());

    // The non-minecart still projects fully (project_entity must not require the component),
    // reporting the empty lerp defaults.
    let bystander = store.probe_entity(20).expect("non-minecart projects");
    assert!(bystander.minecart_lerp_pending_steps.is_empty());
    assert!(bystander.minecart_lerp_steps.is_empty());
    assert_eq!(bystander.minecart_lerp_old_step, None);
    assert_eq!(bystander.minecart_lerp_delay, 0);
}

#[test]
fn minecart_along_track_queues_then_advances_on_client_ticks() {
    let mut store = WorldStore::new();
    enable_minecart_improvements(&mut store);
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    store.apply_add_entity(protocol_add_entity(20));

    assert!(
        store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 10,
            lerp_steps: vec![
                minecart_step(1.25, 64.1, -2.25, 0.2, 0.0, -0.2, 45.0, -10.0, 0.5),
                minecart_step(1.75, 64.2, -2.75, 0.4, 0.0, -0.4, 90.0, 5.0, 1.25),
            ],
        })
    );

    let entity = store.probe_entity(10).unwrap();
    assert_eq!(
        entity.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(entity.y_rot, 20.0);
    assert_eq!(entity.x_rot, -10.0);
    assert_eq!(entity.minecart_lerp_pending_steps.len(), 2);
    assert!(entity.minecart_lerp_steps.is_empty());
    let lerp = store.entities.minecart_lerp(10).unwrap();
    assert_eq!(lerp.pending_steps.len(), 2);
    assert!(lerp.steps.is_empty());
    assert_eq!(lerp.old_step, None);
    assert_eq!(lerp.delay, 0);

    // The network packet only appends pending steps. The next client tick
    // promotes the batch, captures the current transform as oldLerp, and moves
    // the canonical entity to partial=1 of the first 3-tick interval.
    store.advance_entity_client_animations(1);
    let lerp = store.entities.minecart_lerp(10).unwrap();
    assert!(lerp.pending_steps.is_empty());
    assert_eq!(lerp.steps.len(), 2);
    assert_eq!(lerp.delay, 3);
    assert_eq!(lerp.old_step.unwrap().position.x, 1.0);
    let source = store
        .entity_model_sources_at_partial_tick(0.5)
        .into_iter()
        .find(|source| source.entity_id == 10)
        .expect("minecart source");
    assert!(source.minecart_new_render);
    assert!((source.position.x - 1.1458333333333333).abs() < 1.0e-6);
    assert!((source.position.y - 64.05833333333334).abs() < 1.0e-6);
    assert!((source.position.z + 2.1458333333333335).abs() < 1.0e-6);
    assert!((source.y_rot - 34.583332).abs() < 1.0e-5);
    assert!((source.x_rot + 10.0).abs() < 1.0e-6);

    store.advance_entity_client_animations(1);
    let source = store
        .entity_model_sources_at_partial_tick(0.5)
        .into_iter()
        .find(|source| source.entity_id == 10)
        .expect("minecart source");
    assert!(source.minecart_new_render);
    assert!((source.position.x - 1.4).abs() < 1.0e-6);
    assert!((source.position.y - 64.13).abs() < 1.0e-6);
    assert!((source.position.z + 2.4).abs() < 1.0e-6);
    assert!((source.y_rot - 58.5).abs() < 1.0e-6);
    assert!((source.x_rot + 5.5).abs() < 1.0e-6);
    assert_eq!(store.entities.minecart_lerp(10).unwrap().delay, 2);

    store.advance_entity_client_animations(2);
    assert_eq!(store.entities.minecart_lerp(10).unwrap().steps.len(), 0);
    assert_eq!(store.counters().minecart_lerp_steps_tracked, 0);
    let source = store
        .entity_model_sources_at_partial_tick(0.5)
        .into_iter()
        .find(|source| source.entity_id == 10)
        .expect("minecart source");
    assert!(source.minecart_new_render);
    assert_eq!(
        source.position,
        EntityVec3 {
            x: 1.75,
            y: 64.2,
            z: -2.75,
        }
    );
    assert_eq!(source.y_rot, 90.0);
    assert_eq!(source.x_rot, 5.0);

    assert!(
        !store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 999,
            lerp_steps: vec![minecart_step(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0)],
        })
    );
    assert!(
        !store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 20,
            lerp_steps: vec![minecart_step(3.0, 64.0, -4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0)],
        })
    );
    assert_eq!(store.counters().minecart_moves_received, 3);
    assert_eq!(store.counters().minecart_moves_applied, 1);
    assert_eq!(store.counters().minecart_moves_ignored, 2);
    assert_eq!(store.counters().minecart_lerp_steps_received, 4);
    assert_eq!(store.counters().minecart_lerp_steps_tracked, 0);
}

#[test]
fn minecart_along_track_appends_pending_steps_behind_active_batch() {
    let mut store = WorldStore::new();
    enable_minecart_improvements(&mut store);
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    let first = minecart_step(1.5, 64.1, -2.5, 0.2, 0.0, -0.2, 45.0, 0.0, 1.0);
    let second = minecart_step(2.0, 64.2, -3.0, 0.3, 0.0, -0.3, 90.0, 5.0, 1.0);
    let third = minecart_step(2.5, 64.3, -3.5, 0.4, 0.0, -0.4, 135.0, 10.0, 1.0);

    assert!(
        store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 10,
            lerp_steps: vec![first],
        })
    );
    store.advance_entity_client_animations(1);
    assert_eq!(store.entities.minecart_lerp(10).unwrap().steps, vec![first]);

    assert!(
        store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 10,
            lerp_steps: vec![second],
        })
    );
    assert!(
        store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 10,
            lerp_steps: vec![third],
        })
    );
    let lerp = store.entities.minecart_lerp(10).unwrap();
    assert_eq!(lerp.steps, vec![first]);
    assert_eq!(lerp.pending_steps, vec![second, third]);
    assert_eq!(store.counters().minecart_lerp_steps_tracked, 3);

    store.advance_entity_client_animations(3);
    let lerp = store.entities.minecart_lerp(10).unwrap();
    assert_eq!(lerp.steps, vec![second, third]);
    assert!(lerp.pending_steps.is_empty());
    assert_eq!(lerp.delay, 3);
    let entity = store.probe_entity(10).unwrap();
    assert!(entity.position.x > first.position.x);
    assert!(entity.position.x < third.position.x);
}

#[test]
fn minecart_along_track_requires_the_improvements_feature() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));

    assert!(
        !store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 10,
            lerp_steps: vec![minecart_step(
                2.0, 64.0, -3.0, 0.0, 0.0, 0.0, 90.0, 0.0, 1.0,
            )],
        })
    );
    let entity = store.probe_entity(10).unwrap();
    assert_eq!(entity.position.x, 1.0);
    assert!(entity.minecart_lerp_pending_steps.is_empty());
    assert!(entity.minecart_lerp_steps.is_empty());
    assert!(
        !store
            .entity_model_sources_at_partial_tick(0.5)
            .into_iter()
            .find(|source| source.entity_id == 10)
            .unwrap()
            .minecart_new_render
    );
    assert_eq!(store.counters().minecart_moves_ignored, 1);
    assert_eq!(store.counters().minecart_lerp_steps_tracked, 0);

    enable_minecart_improvements(&mut store);
    assert!(
        store
            .entity_model_sources_at_partial_tick(0.5)
            .into_iter()
            .find(|source| source.entity_id == 10)
            .unwrap()
            .minecart_new_render
    );
}

#[test]
fn minecart_pending_queue_preserves_the_combined_step_limit() {
    const MAX_STEPS: usize = 65_536;

    let mut store = WorldStore::new();
    enable_minecart_improvements(&mut store);
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    let step = minecart_step(2.0, 64.0, -3.0, 0.0, 0.0, 0.0, 90.0, 0.0, 1.0);

    assert!(
        store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 10,
            lerp_steps: vec![step; MAX_STEPS],
        })
    );
    assert!(
        !store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 10,
            lerp_steps: vec![step],
        })
    );
    assert_eq!(
        store
            .entities
            .minecart_lerp(10)
            .unwrap()
            .pending_steps
            .len(),
        MAX_STEPS
    );
    assert_eq!(store.counters().minecart_moves_applied, 1);
    assert_eq!(store.counters().minecart_moves_ignored, 1);
    assert_eq!(store.counters().minecart_lerp_steps_tracked, MAX_STEPS);
}

#[test]
fn new_minecart_passengers_render_with_vehicle_lerp_offset() {
    let mut store = WorldStore::new();
    enable_minecart_improvements(&mut store);
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 5.0,
            y: 70.0,
            z: 9.0,
        },
        ..protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_COW_ID)
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));

    let passenger_position = |store: &WorldStore, partial_ticks: f32| {
        store
            .entity_model_sources_at_partial_tick(partial_ticks)
            .into_iter()
            .find(|source| source.entity_id == 20)
            .expect("passenger source")
            .position
    };

    let still = passenger_position(&store, 0.5);
    assert_eq!(
        still,
        EntityVec3 {
            x: 5.0,
            y: 70.0,
            z: 9.0,
        }
    );

    assert!(
        store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 10,
            lerp_steps: vec![
                minecart_step(1.25, 64.1, -2.25, 0.2, 0.0, -0.2, 45.0, -10.0, 0.5),
                minecart_step(1.75, 64.2, -2.75, 0.4, 0.0, -0.4, 90.0, 5.0, 1.25),
            ],
        })
    );

    // Packet arrival alone only identifies the new behavior; the pending batch
    // has not entered the active render interpolation until the client tick.
    assert_eq!(passenger_position(&store, 0.5), still);
    store.advance_entity_client_animations(1);

    // Vanilla `EntityRenderer.extractRenderState`: passengerOffset =
    // `NewMinecartBehavior.getCartLerpPosition(partialTicks)` minus the cart's normal
    // xOld/getX interpolation. After the promotion tick, the canonical cart is only
    // one third through the batch. At partial 0.5 the weighted render position is
    // (1.1458333, 64.0583333, -2.1458333), while the normal interpolation is
    // (1.1416667, 64.0533333, -2.1416667).
    let shifted = passenger_position(&store, 0.5);
    assert!(
        (shifted.x - 5.004166666666666).abs() < 1.0e-6,
        "x offset: {}",
        shifted.x
    );
    assert!(
        (shifted.y - 70.005).abs() < 1.0e-6,
        "y offset: {}",
        shifted.y
    );
    assert!(
        (shifted.z - 8.995833333333334).abs() < 1.0e-6,
        "z offset: {}",
        shifted.z
    );

    store.advance_entity_client_animations(3);
    assert_eq!(
        passenger_position(&store, 0.5),
        EntityVec3 {
            x: 5.0,
            y: 70.0,
            z: 9.0,
        },
        "after the minecart lerp drains, passengers no longer get a render-only offset"
    );
}

fn enable_minecart_improvements(store: &mut WorldStore) {
    store.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec![
            "minecraft:vanilla".to_string(),
            "minecraft:minecart_improvements".to_string(),
        ],
    });
}

#[test]
fn entity_model_sources_project_old_minecart_rail_render_points() {
    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 2.5,
            y: 1.0,
            z: 3.5,
        },
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_MINECART_ID)
    });

    let source = store
        .entity_model_sources_at_partial_tick(1.0)
        .into_iter()
        .find(|source| source.entity_id == 10)
        .expect("minecart source");
    assert!(!source.minecart_new_render);
    assert_eq!(source.minecart_pos_on_rail, None);
    assert_eq!(source.minecart_front_pos, None);
    assert_eq!(source.minecart_back_pos, None);

    let rail_id = vanilla_block_state_id(
        "minecraft:rail",
        [("shape", "ascending_east"), ("waterlogged", "false")],
    );
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 2, y: 1, z: 3 },
        block_state_id: rail_id,
    }));

    let source = store
        .entity_model_sources_at_partial_tick(1.0)
        .into_iter()
        .find(|source| source.entity_id == 10)
        .expect("minecart source");
    assert!(!source.minecart_new_render);
    assert_close3_f32(source.minecart_pos_on_rail.unwrap(), [2.5, 1.5625, 3.5]);
    assert_close3_f32(source.minecart_front_pos.unwrap(), [2.8, 1.8625, 3.5]);
    assert_close3_f32(source.minecart_back_pos.unwrap(), [2.2, 1.2625, 3.5]);
}
