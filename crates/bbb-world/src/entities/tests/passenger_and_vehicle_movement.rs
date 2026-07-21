use std::collections::BTreeMap;

use bbb_protocol::item_types::{
    VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
};

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
    assert_eq!(
        store.entities.mount(10).unwrap().passengers.into_value(),
        vec![20, 21]
    );
    assert_eq!(store.entities.mount(20).unwrap().vehicle_id, Some(10));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![20],
    }));
    assert_eq!(store.probe_entity(10).unwrap().passengers, vec![21]);
    assert_eq!(store.probe_entity(20).unwrap().vehicle_id, Some(30));
    assert_eq!(store.probe_entity(30).unwrap().passengers, vec![20]);
    assert_eq!(store.entities.mount(20).unwrap().vehicle_id, Some(30));
    assert_eq!(
        store.entities.mount(30).unwrap().passengers.into_value(),
        vec![20]
    );

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
fn client_tick_schedule_follows_loaded_roots_and_recursive_player_exception() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(999));

    let root_id = 10;
    let unloaded_passenger_id = 20;
    let player_passenger_id = 21;
    let nested_player_id = 22;
    store.apply_add_entity(protocol_add_entity_with_type(
        root_id,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    for (id, entity_type_id) in [
        (unloaded_passenger_id, VANILLA_ENTITY_TYPE_COW_ID),
        (player_passenger_id, VANILLA_ENTITY_TYPE_PLAYER_ID),
        (nested_player_id, VANILLA_ENTITY_TYPE_PLAYER_ID),
    ] {
        let mut packet = protocol_add_entity_with_type(id, entity_type_id);
        packet.position.x = 160.0;
        store.apply_add_entity(packet);
    }
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: root_id,
        passenger_ids: vec![unloaded_passenger_id, player_passenger_id],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: unloaded_passenger_id,
        passenger_ids: vec![nested_player_id],
    }));

    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    let generation_a = store.publication_snapshot();
    let work = store.advance_entity_client_animations_with_observation(1);
    assert_eq!(work.scanned_entities, 4);
    assert_eq!(work.mutated_entities, 2);
    assert_eq!(
        store
            .probe_entity(root_id)
            .unwrap()
            .client_animations
            .age_ticks,
        1
    );
    assert_eq!(
        store
            .probe_entity(player_passenger_id)
            .unwrap()
            .client_animations
            .age_ticks,
        1,
        "a direct Player passenger uses ClientLevel.tickPassenger's exception"
    );
    assert_eq!(
        store
            .probe_entity(unloaded_passenger_id)
            .unwrap()
            .client_animations
            .age_ticks,
        0
    );
    assert_eq!(
        store
            .probe_entity(nested_player_id)
            .unwrap()
            .client_animations
            .age_ticks,
        0,
        "an unticked parent prevents recursion even when the nested passenger is a Player"
    );
    let generation_b = store.publication_snapshot();
    assert_eq!(
        generation_a
            .probe_entity(root_id)
            .unwrap()
            .client_animations
            .age_ticks,
        0,
        "the retained pre-tick generation remains immutable"
    );
    assert_eq!(
        generation_b
            .probe_entity(root_id)
            .unwrap()
            .client_animations
            .age_ticks,
        1
    );

    assert!(store.forget_chunk(crate::ChunkPos { x: 0, z: -1 }));
    let paused = store.advance_entity_client_animations_with_observation(1);
    assert_eq!(paused.mutated_entities, 0);
    assert_eq!(
        store
            .probe_entity(root_id)
            .unwrap()
            .client_animations
            .age_ticks,
        1
    );
    assert_eq!(
        store
            .probe_entity(player_passenger_id)
            .unwrap()
            .client_animations
            .age_ticks,
        1,
        "the Player exception is reached only through a ticking root"
    );
}

#[test]
fn frozen_client_tick_schedule_keeps_player_roots_and_player_passenger_trees_running() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(999));
    let frozen_root_id = 10;
    let player_root_id = 11;
    let player_vehicle_id = 20;
    let intermediate_passenger_id = 21;
    let nested_player_id = 22;
    for (id, entity_type_id) in [
        (frozen_root_id, VANILLA_ENTITY_TYPE_COW_ID),
        (player_root_id, VANILLA_ENTITY_TYPE_PLAYER_ID),
        (player_vehicle_id, VANILLA_ENTITY_TYPE_OAK_BOAT_ID),
        (intermediate_passenger_id, VANILLA_ENTITY_TYPE_COW_ID),
        (nested_player_id, VANILLA_ENTITY_TYPE_PLAYER_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
    }
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: player_vehicle_id,
        passenger_ids: vec![intermediate_passenger_id],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: intermediate_passenger_id,
        passenger_ids: vec![nested_player_id],
    }));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    store.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 20.0,
        frozen: true,
    });

    let work = store.advance_entity_client_animations_with_observation(1);
    assert_eq!(work.mutated_entities, 4);
    assert_eq!(
        store
            .probe_entity(frozen_root_id)
            .unwrap()
            .client_animations
            .age_ticks,
        0,
        "an ordinary non-Player root is frozen"
    );
    for id in [
        player_root_id,
        player_vehicle_id,
        intermediate_passenger_id,
        nested_player_id,
    ] {
        assert_eq!(
            store.probe_entity(id).unwrap().client_animations.age_ticks,
            1,
            "Player roots and the loaded chain to an indirect Player passenger stay running"
        );
    }
}

#[test]
fn frozen_step_batch_only_runs_ordinary_entities_for_available_step_ticks() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(999));
    let ordinary_id = 10;
    let player_id = 11;
    store.apply_add_entity(protocol_add_entity_with_type(
        ordinary_id,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        player_id,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    store.apply_ticking_state(bbb_protocol::packets::TickingState {
        tick_rate: 20.0,
        frozen: true,
    });
    store.apply_ticking_step(bbb_protocol::packets::TickingStep { tick_steps: 1 });

    let work = store.advance_entity_client_animations_with_observation(3);
    assert_eq!(work.scanned_entities, 6);
    assert_eq!(work.mutated_entities, 4);
    assert_eq!(
        store
            .probe_entity(ordinary_id)
            .unwrap()
            .client_animations
            .age_ticks,
        1
    );
    assert_eq!(
        store
            .probe_entity(player_id)
            .unwrap()
            .client_animations
            .age_ticks,
        3
    );
    assert_eq!(
        store.ticking().frozen_ticks_to_run,
        1,
        "the runtime clock owner consumes the shared frozen-step budget after entity ticks"
    );
    assert_eq!(store.consume_running_render_ticks(3), 1);
    assert_eq!(store.ticking().frozen_ticks_to_run, 0);
}

#[test]
fn passenger_ride_tick_clears_motion_before_client_entity_tick() {
    let mut store = WorldStore::new();
    let vehicle_id = 10;
    let passenger_id = 20;
    store.apply_add_entity(protocol_add_entity_with_type(
        vehicle_id,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        passenger_id,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: vehicle_id,
        delta_movement: ProtocolVec3d {
            x: 0.25,
            y: 0.5,
            z: -0.75,
        },
    }));
    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: passenger_id,
        delta_movement: ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id,
        passenger_ids: vec![passenger_id],
    }));

    store.advance_entity_client_animations(1);

    assert_eq!(
        store
            .entities
            .transform(passenger_id)
            .unwrap()
            .delta_movement,
        EntityVec3::default()
    );
    assert_eq!(
        store.entities.transform(vehicle_id).unwrap().delta_movement,
        EntityVec3 {
            x: 0.25,
            y: 0.5,
            z: -0.75,
        }
    );
}

#[test]
fn living_passenger_body_rotation_follows_interpolated_living_vehicle() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.advance_entity_client_animations(1);
    store.entities.with_transform_mut(10, |transform| {
        transform.old_y_rot = 0.0;
        transform.y_rot = 20.0;
    });
    store.entities.with_transform_mut(20, |transform| {
        transform.old_y_head_rot = 100.0;
        transform.y_head_rot = 140.0;
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));

    let source = store
        .entity_model_source_at_partial_tick(20, 0.5)
        .expect("living passenger source");
    assert_eq!(source.y_head_rot, 120.0);
    // Vanilla LivingEntityRenderer.solveBodyRot: vehicle body 10, head delta
    // clamps to 85, then the over-50 correction adds 17 degrees.
    assert_eq!(source.y_rot, 52.0);
}

#[test]
fn shulker_mount_edges_reset_attach_position_interpolation() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
    ));
    store.advance_entity_client_animations(1);
    let initial = store.entities.transform(20).unwrap().position;
    let move_shulker = |store: &mut WorldStore, x: f64| {
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 20,
            position: ProtocolVec3d {
                x,
                y: initial.y,
                z: initial.z,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
        })
    };

    assert!(move_shulker(&mut store, initial.x + 3.0));
    assert_eq!(
        store
            .probe_entity(20)
            .unwrap()
            .client_animations
            .shulker_teleport
            .unwrap()
            .interpolation_ticks,
        6
    );
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    assert_eq!(
        store
            .probe_entity(20)
            .unwrap()
            .client_animations
            .shulker_teleport,
        None
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![],
    }));
    let stopped = store
        .probe_entity(20)
        .unwrap()
        .client_animations
        .shulker_teleport
        .unwrap();
    assert_eq!(stopped.interpolation_ticks, 0);
    assert!(stopped.old_attach_position.is_some());

    assert!(move_shulker(&mut store, initial.x + 6.0));
    assert_eq!(
        store
            .probe_entity(20)
            .unwrap()
            .client_animations
            .shulker_teleport
            .unwrap()
            .interpolation_ticks,
        6
    );
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
fn turtle_passengers_render_at_adult_baby_scale_and_vehicle_attachments() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        10,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        90.0,
    ));
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 20.0,
            y: 80.0,
            z: 9.0,
        },
        ..protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_COW_ID)
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));

    let passenger_position = |store: &WorldStore, passenger_id: i32| {
        store
            .entity_model_source_at_partial_tick(passenger_id, 0.0)
            .expect("Turtle passenger model source")
            .position
    };
    assert_entity_vec3_close(
        passenger_position(&store, 20),
        EntityVec3 {
            x: 1.25,
            y: 64.55625,
            z: -2.0,
        },
    );

    let (baby_data_id, baby_default) =
        super::schema::vanilla_entity_metadata_baby_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_TURTLE_ID,
        )
        .expect("generated Turtle baby accessor");
    assert!(!baby_default);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![protocol_bool_data(baby_data_id, true)],
    }));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 10,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    assert_entity_vec3_close(
        passenger_position(&store, 20),
        EntityVec3 {
            x: 1.15,
            y: 64.24,
            z: -2.0,
        },
    );
    assert_entity_vec3_close(
        store
            .entity_debug_passenger_vehicle_target(20)
            .expect("baby Turtle passenger attachment target")
            .position,
        EntityVec3 {
            x: 1.15,
            y: 64.24,
            z: -2.0,
        },
    );

    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: -12.0,
            y: 40.0,
            z: 3.0,
        },
        ..protocol_add_entity_with_type(21, VANILLA_ENTITY_TYPE_PLAYER_ID)
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![21],
    }));
    assert_entity_vec3_close(
        passenger_position(&store, 21),
        EntityVec3 {
            x: 1.15,
            y: 63.64,
            z: -2.0,
        },
    );

    store.apply_add_entity(protocol_add_entity_with_type(
        22,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    let (zombie_baby_data_id, zombie_baby_default) =
        super::schema::vanilla_entity_metadata_baby_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        )
        .expect("generated Zombie baby accessor");
    assert!(!zombie_baby_default);
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 22,
        values: vec![protocol_bool_data(zombie_baby_data_id, true)],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![22],
    }));
    assert_entity_vec3_close(
        passenger_position(&store, 22),
        EntityVec3 {
            x: 1.15,
            y: 64.0525,
            z: -2.0,
        },
    );
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
    assert!(store.probe_entity(10).is_none());
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

    let (applied, recovery, vehicle_report) =
        store.apply_teleport_entity_with_recovery(ProtocolTeleportEntity {
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
    assert_eq!(vehicle_report, None);
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
    let (camel_dash_data_id, _) =
        vanilla_entity_metadata_camel_dash_accessor_for_entity_type(VANILLA_ENTITY_TYPE_CAMEL_ID)
            .expect("generated Camel dash accessor must resolve");
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(
        TEST_SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
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
            data_id: camel_dash_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
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
            1,
        )
        .unwrap();

    assert_eq!(report.vehicle_id, 20);
    assert!(!report.snapped);
    assert_eq!(report.y_rot, initial.y_rot + 1.0);
    let yaw = report.y_rot * (std::f64::consts::PI / 180.0) as f32;
    let expected_x = initial.position.x
        + f64::from(super::super::client_movement::vanilla_mth_sin(-f64::from(yaw)) * 0.04);
    let expected_z = initial.position.z
        + f64::from(super::super::client_movement::vanilla_mth_cos(f64::from(yaw)) * 0.04);
    assert!((report.position.x - expected_x).abs() < 1.0e-12);
    assert!((report.position.z - expected_z).abs() < 1.0e-12);
    assert_eq!(report.position, store.probe_entity(20).unwrap().position);
    assert_eq!(store.local_player_root_boat_vehicle_id(), Some(20));
}

#[test]
fn local_boat_input_ignores_unfocused_controls_but_keeps_vanilla_physics() {
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
            1,
        )
        .unwrap();

    assert_eq!(report.position.x, initial.position.x);
    assert_eq!(report.position.z, initial.position.z);
    assert!((report.position.y - (initial.position.y - 0.04)).abs() < 1.0e-12);
    assert_eq!(report.y_rot, initial.y_rot);
}

#[test]
fn local_happy_ghast_input_uses_vanilla_ridden_rotation_and_flying_drag() {
    let mut store = locally_controlled_happy_ghast();
    store.set_local_player_pose(crate::LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        y_rot: 100.0,
        x_rot: 30.0,
        ..crate::LocalPlayerPoseState::default()
    });
    let before = store.entities.transform(10).unwrap();

    let report = store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                left: true,
                jump: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();

    assert_eq!(report.vehicle_id, 10);
    assert_eq!(report.position, after.position);
    assert!((after.y_rot - 26.4).abs() < 1.0e-6);
    assert_eq!(after.old_y_rot, after.y_rot);
    assert!((after.x_rot - 15.0).abs() < 1.0e-6);
    assert_eq!(after.y_head_rot, after.y_rot);
    assert!(after.position.x > before.position.x);
    assert!(after.position.z > before.position.z);
    assert!((after.position.y - before.position.y).abs() < 1.0e-6);
    assert!(
        (after.delta_movement.x - (after.position.x - before.position.x) * 0.91).abs() < 1.0e-8
    );
    assert!(
        (after.delta_movement.z - (after.position.z - before.position.z) * 0.91).abs() < 1.0e-8
    );
}

#[test]
fn local_abstract_horse_input_uses_ridden_rotation_air_travel_and_gravity() {
    let mut store = locally_controlled_horse();
    store.set_local_player_pose(crate::LocalPlayerPoseState {
        y_rot: 90.0,
        x_rot: 30.0,
        ..crate::LocalPlayerPoseState::default()
    });
    let before = store.entities.transform(10).unwrap();

    let report = store
        .advance_local_abstract_horse_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();

    assert_eq!(report.vehicle_id, 10);
    assert_eq!(report.position, after.position);
    assert_eq!(after.y_rot, 90.0);
    assert_eq!(after.old_y_rot, 90.0);
    assert_eq!(after.x_rot, 15.0);
    assert_eq!(after.y_head_rot, 90.0);
    assert!(after.position.x < before.position.x);
    assert!((after.position.z - before.position.z).abs() < 1.0e-7);
    assert_eq!(after.position.y, before.position.y);
    assert!((after.delta_movement.y - -0.08 * f64::from(0.98_f32)).abs() < 1.0e-12);
}

#[test]
fn local_abstract_horse_standing_gate_and_controller_gate_match_vanilla() {
    let mut store = locally_controlled_horse();
    store.entities.with_transform_mut(10, |transform| {
        transform.on_ground = Some(true);
    });
    let accessors = vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HORSE_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(accessors.standing_mask),
        }],
    }));
    let before = store.entities.transform(10).unwrap();
    store
        .advance_local_abstract_horse_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();
    assert_eq!(after.position.x, before.position.x);
    assert_eq!(after.position.z, before.position.z);

    let mut unsaddled = WorldStore::new();
    unsaddled.apply_login(&protocol_play_login(99));
    unsaddled.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    assert!(unsaddled.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    unsaddled.set_local_player_pose(crate::LocalPlayerPoseState::default());
    assert_eq!(
        unsaddled.advance_local_abstract_horse_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        ),
        None
    );
}

#[test]
fn local_abstract_horse_movement_isolates_retained_publication() {
    let mut store = locally_controlled_horse();
    let generation_a = store.publication_snapshot();
    let before = generation_a.probe_entity(10).unwrap();

    store
        .advance_local_abstract_horse_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let generation_b = store.publication_snapshot();

    assert_eq!(generation_a.probe_entity(10).unwrap(), before);
    assert_ne!(
        generation_b.probe_entity(10).unwrap().position,
        before.position
    );
}

#[test]
fn local_abstract_horse_water_travel_uses_generated_ridden_float_tag() {
    let mut store = locally_controlled_horse();
    store.insert_decoded_chunk(empty_test_chunk());
    store.entities.with_transform_mut(10, |transform| {
        transform.position = EntityVec3 {
            x: 1.0,
            y: -63.0,
            z: 1.0,
        };
        transform.old_position = transform.position;
    });
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 1, y: -63, z: 1 },
        block_state_id: vanilla_block_state_id("minecraft:water", [("level", "0")]),
    }));

    store
        .advance_local_abstract_horse_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let after = store.entities.transform(10).unwrap();

    assert!(after.delta_movement.y > 0.03);
    assert!(
        bbb_protocol::entity_types::vanilla_entity_type_can_float_while_ridden(
            VANILLA_ENTITY_TYPE_HORSE_ID,
        )
    );
}

#[test]
fn local_abstract_horse_uses_step_height_attribute_for_one_block_step() {
    let mut store = locally_controlled_horse();
    store.insert_decoded_chunk(empty_test_chunk());
    let bounds = store.entities.pick_bounds(10).unwrap();
    store.entities.with_transform_mut(10, |transform| {
        transform.position = EntityVec3 {
            x: 1.0,
            y: -63.0,
            z: 2.0 - f64::from(bounds.max[2]),
        };
        transform.old_position = transform.position;
        transform.y_rot = 0.0;
        transform.old_y_rot = 0.0;
        transform.on_ground = Some(true);
    });
    let before = store.entities.transform(10).unwrap();
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 1, y: -63, z: 2 },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));

    store
        .advance_local_abstract_horse_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();

    assert!(after.position.z > before.position.z);
    assert!((after.position.y - (before.position.y + 1.0)).abs() < 1.0e-12);
}

fn locally_controlled_horse() -> WorldStore {
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
    equip_test_saddle(&mut store, 10);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    store.set_local_player_pose(crate::LocalPlayerPoseState::default());
    store
}

#[test]
fn local_happy_ghast_backward_pitch_and_focus_follow_controller_input() {
    let mut store = locally_controlled_happy_ghast();
    store.set_local_player_pose(crate::LocalPlayerPoseState {
        position: bbb_protocol::packets::Vec3d::default(),
        x_rot: 30.0,
        ..crate::LocalPlayerPoseState::default()
    });
    let before = store.entities.transform(10).unwrap();

    store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                backward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let backward = store.entities.transform(10).unwrap();
    assert!(backward.position.y > before.position.y);
    assert!(backward.position.z < before.position.z);

    let before_unfocused = backward;
    store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: false,
                forward: true,
                jump: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let unfocused = store.entities.transform(10).unwrap();
    assert_eq!(
        unfocused.position,
        EntityVec3 {
            x: before_unfocused.position.x + before_unfocused.delta_movement.x,
            y: before_unfocused.position.y + before_unfocused.delta_movement.y,
            z: before_unfocused.position.z + before_unfocused.delta_movement.z,
        }
    );
}

#[test]
fn local_happy_ghast_movement_requires_exact_controller_gate() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    store.set_local_player_pose(crate::LocalPlayerPoseState::default());
    let input = crate::LocalPlayerInputState {
        focused: true,
        forward: true,
        ..crate::LocalPlayerInputState::default()
    };
    assert_eq!(
        store.advance_local_happy_ghast_vehicle_input(input, 1),
        None
    );

    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 10,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: item_stack(77, 1),
        }],
    }));
    assert!(store
        .advance_local_happy_ghast_vehicle_input(input, 1)
        .is_some());
    let accessors = vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.stays_still_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert_eq!(
        store.advance_local_happy_ghast_vehicle_input(input, 1),
        None
    );
}

#[test]
fn local_happy_ghast_movement_isolates_retained_publication() {
    let mut store = locally_controlled_happy_ghast();
    let generation_a = store.publication_snapshot();
    let before = generation_a.probe_entity(10).unwrap();

    store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let generation_b = store.publication_snapshot();

    assert_eq!(generation_a.probe_entity(10).unwrap(), before);
    assert_ne!(
        generation_b.probe_entity(10).unwrap().position,
        before.position
    );
}

#[test]
fn local_happy_ghast_move_clips_against_world_collision() {
    let mut store = locally_controlled_happy_ghast();
    store.insert_decoded_chunk(empty_test_chunk());
    store.entities.with_transform_mut(10, |transform| {
        transform.position.y = -63.0;
        transform.old_position.y = -63.0;
        transform.y_rot = 0.0;
        transform.old_y_rot = 0.0;
    });
    let bounds = store.entities.pick_bounds(10).unwrap();
    let before = store.entities.transform(10).unwrap();
    let wall_z = (before.position.z + f64::from(bounds.max[2])).floor() as i32;
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: before.position.x.floor() as i32,
            y: before.position.y.floor() as i32,
            z: wall_z,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));

    store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();

    assert_eq!(after.position.z, before.position.z);
    assert_eq!(after.delta_movement.z, 0.0);
    assert_eq!(after.on_ground, Some(false));
}

#[test]
fn local_happy_ghast_move_uses_vanilla_candidate_height_step_up() {
    let mut store = locally_controlled_happy_ghast();
    store.insert_decoded_chunk(empty_test_chunk());
    store.entities.with_transform_mut(10, |transform| {
        transform.position.y = -63.0;
        transform.old_position.y = -63.0;
        transform.y_rot = 0.0;
        transform.old_y_rot = 0.0;
        transform.on_ground = Some(true);
    });
    let bounds = store.entities.pick_bounds(10).unwrap();
    let before = store.entities.transform(10).unwrap();
    let wall_z = (before.position.z + f64::from(bounds.max[2])).floor() as i32;
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: before.position.x.floor() as i32,
            y: before.position.y.floor() as i32,
            z: wall_z,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));

    store
        .advance_local_happy_ghast_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let after = store.entities.transform(10).unwrap();

    assert!(after.position.z > before.position.z);
    assert!((after.position.y - (before.position.y + 1.0)).abs() < 1.0e-12);
}

fn locally_controlled_happy_ghast() -> WorldStore {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    ));
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 10,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: item_stack(77, 1),
        }],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    store.set_local_player_pose(crate::LocalPlayerPoseState::default());
    store
}

#[test]
fn local_boat_input_updates_generated_paddle_metadata() {
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
    let accessors =
        super::super::schema::vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        )
        .expect("generated boat accessors");

    store
        .advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    assert_eq!(
        store.entities.metadata_bool_for_entity(
            20,
            accessors.paddle_left_data_id,
            accessors.paddle_left_default,
        ),
        Some(true)
    );
    assert_eq!(
        store.entities.metadata_bool_for_entity(
            20,
            accessors.paddle_right_data_id,
            accessors.paddle_right_default,
        ),
        Some(true)
    );

    store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    assert_eq!(
        store.entities.metadata_bool_for_entity(
            20,
            accessors.paddle_left_data_id,
            accessors.paddle_left_default,
        ),
        Some(false)
    );
    assert_eq!(
        store.entities.metadata_bool_for_entity(
            20,
            accessors.paddle_right_data_id,
            accessors.paddle_right_default,
        ),
        Some(false)
    );
}

#[test]
fn local_boat_paddles_advance_after_control_and_emit_post_move_water_sounds() {
    const ADVANCE: f32 = std::f32::consts::PI / 8.0;
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 1,
            y: -64,
            z: -2,
        },
        block_state_id: vanilla_block_state_id("minecraft:water", [("level", "0")]),
    }));
    let mut boat = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    boat.position.y = -63.2;
    store.apply_add_entity(boat);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let retained = store.publication_snapshot();
    let forward = crate::LocalPlayerInputState {
        focused: true,
        forward: true,
        ..crate::LocalPlayerInputState::default()
    };

    store.advance_entity_client_animations(1);
    store.advance_local_boat_vehicle_input(forward, 1).unwrap();
    let first = store.entity_model_source_at_partial_tick(20, 1.0).unwrap();
    assert!((first.boat_rowing_time_left - ADVANCE).abs() < 1.0e-6);
    assert!((first.boat_rowing_time_right - ADVANCE).abs() < 1.0e-6);
    assert!(store
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());

    store.advance_entity_client_animations(1);
    store.advance_local_boat_vehicle_input(forward, 1).unwrap();
    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(effects.len(), 2);
    let sounds = effects
        .into_iter()
        .map(|effect| match effect {
            EntityClientMovementEffectState::Sound(sound) => store
                .record_entity_client_movement_sound_if_audible(sound)
                .expect("water paddle sound"),
            EntityClientMovementEffectState::Particle(_) => panic!("unexpected particle"),
        })
        .collect::<Vec<_>>();
    assert!(sounds.iter().all(|sound| {
        sound.sound.location.as_deref() == Some("minecraft:entity.boat.paddle_water")
            && sound.source == "neutral"
            && sound.volume == 1.0
            && (0.8..=1.2).contains(&sound.pitch)
    }));
    assert_ne!(sounds[0].seed, sounds[1].seed);
    let transform = store.entities.transform(20).unwrap();
    assert!(
        (sounds[0].position.x + sounds[1].position.x - 2.0 * transform.position.x).abs() < 1.0e-12
    );
    assert!(
        (sounds[0].position.z + sounds[1].position.z - 2.0 * transform.position.z).abs() < 1.0e-12
    );

    let retained_boat = retained
        .entity_model_source_at_partial_tick(20, 1.0)
        .unwrap();
    assert_eq!(retained_boat.boat_rowing_time_left, 0.0);
    assert_eq!(retained_boat.boat_rowing_time_right, 0.0);
}

#[test]
fn remote_boat_paddle_sound_respects_air_and_silent_gates() {
    let accessors = vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    )
    .expect("generated boat accessors");
    let entity_accessors =
        vanilla_entity_metadata_entity_accessors().expect("generated entity accessors");
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        99,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 20,
        values: vec![
            protocol_bool_data(accessors.paddle_left_data_id, true),
            protocol_bool_data(accessors.paddle_right_data_id, true),
        ],
    }));

    store.advance_entity_client_animations(2);
    let air_effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(air_effects.len(), 2);
    assert!(air_effects.into_iter().all(|effect| match effect {
        EntityClientMovementEffectState::Sound(sound) => store
            .record_entity_client_movement_sound_if_audible(sound)
            .is_none(),
        EntityClientMovementEffectState::Particle(_) => false,
    }));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 20,
        values: vec![protocol_bool_data(entity_accessors.silent_data_id, true)],
    }));
    store.advance_entity_client_animations(1);
    assert!(store
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());
}

#[test]
fn remote_boat_paddle_sound_uses_post_move_land_status() {
    let accessors = vanilla_entity_metadata_abstract_boat_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    )
    .expect("generated boat accessors");
    let mut store = WorldStore::new();
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 1,
            y: -64,
            z: -2,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    let mut boat = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    boat.position.y = -63.0;
    store.apply_add_entity(boat);
    let mut passenger = protocol_add_entity_with_type(99, VANILLA_ENTITY_TYPE_CHICKEN_ID);
    passenger.position.y = -63.0;
    store.apply_add_entity(passenger);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 20,
        values: vec![
            protocol_bool_data(accessors.paddle_left_data_id, true),
            protocol_bool_data(accessors.paddle_right_data_id, true),
        ],
    }));

    store.advance_entity_client_animations(2);
    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(effects.len(), 2);
    assert!(effects.into_iter().all(|effect| match effect {
        EntityClientMovementEffectState::Sound(sound) => store
            .record_entity_client_movement_sound_if_audible(sound)
            .is_some_and(|sound| {
                sound.sound.location.as_deref() == Some("minecraft:entity.boat.paddle_land")
            }),
        EntityClientMovementEffectState::Particle(_) => false,
    }));
}

#[test]
fn zero_client_ticks_do_not_advance_boat_physics_or_paddles() {
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
    let before = store.publication_snapshot();

    store
        .advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                forward: true,
                right: true,
                ..crate::LocalPlayerInputState::default()
            },
            0,
        )
        .unwrap();

    assert_eq!(store.probe_entity(20), before.probe_entity(20));
    assert_eq!(
        store.entities.boat_movement(20),
        before.entities.boat_movement(20)
    );
}

#[test]
fn boat_physics_state_isolated_in_publication_and_reset_on_entity_recreation() {
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
    let generation_a = store.publication_snapshot();

    store
        .advance_local_boat_vehicle_input(
            crate::LocalPlayerInputState {
                focused: true,
                right: true,
                ..crate::LocalPlayerInputState::default()
            },
            1,
        )
        .unwrap();
    let generation_b = store.publication_snapshot();
    assert_eq!(
        generation_a.entities.boat_movement(20),
        Some(super::super::components::EntityBoatMovement::default())
    );
    let advanced = generation_b.entities.boat_movement(20).unwrap();
    assert_eq!(advanced.old_status, None);
    assert_eq!(advanced.status, Some(super::super::EntityBoatStatus::InAir));
    assert_eq!(advanced.delta_rotation, 1.0);
    assert_ne!(
        generation_a.probe_entity(20).unwrap().position,
        generation_b.probe_entity(20).unwrap().position
    );

    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert_eq!(
        store.entities.boat_movement(20),
        Some(super::super::components::EntityBoatMovement::default())
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert_eq!(store.entities.boat_movement(20), None);
}

#[test]
fn local_boat_uses_vanilla_land_friction_and_collision_response() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 1,
            y: -64,
            z: -2
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    let mut boat = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    boat.position.y = -63.0;
    boat.delta_movement.x = 0.1;
    store.apply_add_entity(boat);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));

    let report = store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let transform = store.entities.transform(20).unwrap();
    let movement = store.entities.boat_movement(20).unwrap();

    assert_eq!(movement.old_status, None);
    assert_eq!(
        movement.status,
        Some(super::super::EntityBoatStatus::OnLand)
    );
    assert_eq!(movement.land_friction, 0.3);
    assert!(
        (report.position.x - 1.06).abs() < 1.0e-8,
        "x: {}, delta: {:?}",
        report.position.x,
        transform.delta_movement
    );
    assert_eq!(report.position.y, -63.0);
    assert_eq!(transform.delta_movement.y, 0.0);
    assert_eq!(transform.on_ground, Some(true));
    assert_eq!(
        store.entities.supporting_block(20).unwrap().main,
        Some(crate::BlockPos {
            x: 1,
            y: -64,
            z: -2
        })
    );
}

#[test]
fn local_boat_first_water_tick_uses_buoyancy_without_air_transition_snap() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 1,
            y: -64,
            z: -2
        },
        block_state_id: vanilla_block_state_id("minecraft:water", [("level", "0")]),
    }));
    let mut boat = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    boat.position.y = -63.2;
    store.apply_add_entity(boat);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let bounds = store.entities.pick_bounds(20).unwrap();
    let height = f64::from(bounds.max[1] - bounds.min[1]);
    let water_level = f64::from(-64.0_f32 + 8.0_f32 / 9.0_f32);
    let buoyancy = (water_level - -63.2) / height;
    let expected_delta_y = (-0.04 + buoyancy * (0.04 / 0.65)) * 0.75;

    let report = store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let movement = store.entities.boat_movement(20).unwrap();
    let transform = store.entities.transform(20).unwrap();

    assert_eq!(movement.old_status, None);
    assert_eq!(
        movement.status,
        Some(super::super::EntityBoatStatus::InWater)
    );
    assert_eq!(movement.water_level, water_level);
    assert!((transform.delta_movement.y - expected_delta_y).abs() < 1.0e-12);
    assert!((report.position.y - (-63.2 + expected_delta_y)).abs() < 1.0e-12);
}

#[test]
fn local_boat_move_clips_against_generated_entity_colliders() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut boat = protocol_add_entity_with_type_y_rot(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, 0.0);
    boat.delta_movement.z = 0.6;
    store.apply_add_entity(boat);
    store.apply_add_entity(protocol_add_entity_with_type(
        99,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    let mut cow = protocol_add_entity_with_type(30, VANILLA_ENTITY_TYPE_COW_ID);
    cow.position.z = -0.5;
    store.apply_add_entity(cow);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let boat_before = store.entities.transform(20).unwrap();
    let boat_bounds = store.entities.pick_bounds(20).unwrap();
    let cow_transform = store.entities.transform(30).unwrap();
    let cow_bounds = store.entities.pick_bounds(30).unwrap();
    let expected_z = cow_transform.position.z + f64::from(cow_bounds.min[2])
        - (boat_before.position.z + f64::from(boat_bounds.max[2]));

    let report = store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let transform = store.entities.transform(20).unwrap();

    assert!((report.position.z - (boat_before.position.z + expected_z)).abs() < 1.0e-12);
    assert_eq!(transform.delta_movement.z, 0.0);
    assert!(
        report.position.z + f64::from(boat_bounds.max[2])
            <= cow_transform.position.z + f64::from(cow_bounds.min[2])
    );
}

#[test]
fn local_boat_collision_excludes_spectator_players_and_same_vehicle_passengers() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut boat = protocol_add_entity_with_type_y_rot(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, 0.0);
    boat.delta_movement.z = 0.6;
    store.apply_add_entity(boat);
    store.apply_add_entity(protocol_add_entity_with_type(
        99,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    let spectator_uuid = Uuid::from_u128(0x7788);
    let mut spectator = protocol_add_entity_with_type(30, VANILLA_ENTITY_TYPE_PLAYER_ID);
    spectator.uuid = spectator_uuid;
    spectator.position.z = -0.5;
    store.apply_add_entity(spectator);
    store.apply_player_info_update(ProtocolPlayerInfoUpdate {
        actions: vec![
            ProtocolPlayerInfoAction::AddPlayer,
            ProtocolPlayerInfoAction::UpdateGameMode,
        ],
        entries: vec![protocol_player_info_entry_with_mode(
            spectator_uuid,
            ProtocolGameType::Spectator,
        )],
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let before = store.entities.transform(20).unwrap();

    let report = store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();

    let expected_delta_z = 0.6 * f64::from(0.9_f32);
    assert!((report.position.z - (before.position.z + expected_delta_z)).abs() < 1.0e-12);
}

#[test]
fn local_boat_air_to_water_snap_no_collision_checks_entity_targets() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        30,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let transform = store.entities.transform(20).unwrap();
    let bounds = store.entities.pick_bounds(20).unwrap();
    let targets = store.entities.vehicle_entity_collision_targets();

    assert!(!store.entity_collision_bounds_are_free(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        [
            transform.position.x,
            transform.position.y,
            transform.position.z,
        ],
        bounds.min.map(f64::from),
        bounds.max.map(f64::from),
        false,
        &targets,
    ));
}

#[test]
fn local_boat_move_clips_against_world_border_collision_shape() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_set_border_size(bbb_protocol::packets::SetBorderSize { size: 4.0 });
    let mut boat = protocol_add_entity_with_type_y_rot(20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID, 0.0);
    boat.delta_movement.x = 1.0;
    store.apply_add_entity(boat);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    let before = store.entities.transform(20).unwrap();
    let bounds = store.entities.pick_bounds(20).unwrap();
    let expected_position_x = 2.0 - f64::from(bounds.max[0]);

    let report = store
        .advance_local_boat_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let transform = store.entities.transform(20).unwrap();

    assert!((report.position.x - expected_position_x).abs() < 1.0e-12);
    assert!(report.position.x > before.position.x);
    assert_eq!(transform.delta_movement.x, 0.0);
    assert_eq!(report.position.x + f64::from(bounds.max[0]), 2.0);
}

#[test]
fn move_vehicle_snaps_root_vehicle_and_returns_ack() {
    let mut store = WorldStore::new();
    store.set_default_item_equipment_slots(BTreeMap::from([(
        TEST_SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    equip_test_saddle(&mut store, 20);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    assert!(store.entity_is_locally_authoritative(20));
    assert!(store.entity_is_locally_authoritative(10));

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
    assert_eq!(
        root.position_base,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
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
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));

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
            1,
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
        has_position: true,
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
fn snapped_position_sync_repositions_noncontrolling_local_passenger() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: -10.0,
            y: 20.0,
            z: 30.0,
        },
        ..LocalPlayerPoseState::default()
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity(123));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![123, 99],
    }));
    assert!(!store.entity_is_locally_authoritative(10));

    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 10,
            position: ProtocolVec3d {
                x: 8.0,
                y: 70.0,
                z: -9.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 90.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );

    let expected = store
        .entities
        .local_player_passenger_position(10, 99, false, 0)
        .unwrap();
    let pose = store.local_player_pose().unwrap();
    assert_eq!(pose.position.x, expected.x);
    assert_eq!(pose.position.y, expected.y);
    assert_eq!(pose.position.z, expected.z);
    assert_eq!(store.local_player_old_pose, Some(pose));
}

#[test]
fn snapped_controlled_vehicle_teleport_repositions_rider_and_reports_after_state() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.set_local_player_pose(LocalPlayerPoseState::default());
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    let generation_a = store.publication_snapshot();

    let (applied, recovery, vehicle_report) =
        store.apply_teleport_entity_with_recovery(ProtocolTeleportEntity {
            id: 10,
            position: ProtocolVec3d {
                x: 12.0,
                y: 75.0,
                z: -6.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 135.0,
            x_rot: -10.0,
            relatives_mask: 0,
            on_ground: true,
        });

    assert!(applied);
    assert_eq!(recovery, None);
    let report = vehicle_report.expect("controlled vehicle correction");
    let vehicle = store.probe_entity(10).unwrap();
    assert_eq!(report.vehicle_id, 10);
    assert_eq!(report.position, vehicle.position);
    assert_eq!(report.y_rot, vehicle.y_rot);
    assert_eq!(report.x_rot, vehicle.x_rot);
    assert!(report.on_ground);
    assert!(report.snapped);
    let pose = store.local_player_pose().unwrap();
    assert_ne!(pose.position, ProtocolVec3d::default());
    assert_eq!(store.local_player_old_pose, Some(pose));
    assert_eq!(
        generation_a.probe_entity(10).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(
        generation_a.local_player_pose().unwrap().position,
        ProtocolVec3d::default()
    );
    assert_eq!(
        generation_a.local_player_old_pose,
        Some(LocalPlayerPoseState::default())
    );
    let generation_b = store.publication_snapshot();
    assert_eq!(
        generation_b.probe_entity(10).unwrap().position,
        report.position
    );
    assert_eq!(generation_b.local_player_old_pose, Some(pose));
}

#[test]
fn local_player_relative_move_only_advances_server_position_base() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(
        99,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    let initial = store.probe_entity(99).unwrap();

    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 99,
        has_position: true,
        delta_x: 4096,
        delta_y: 0,
        delta_z: -2048,
        y_rot: Some(90.0),
        x_rot: Some(30.0),
        on_ground: true,
    }));

    let moved = store.probe_entity(99).unwrap();
    assert_eq!(moved.position, initial.position);
    assert_eq!(moved.y_rot, initial.y_rot);
    assert_eq!(moved.x_rot, initial.x_rot);
    assert_eq!(moved.on_ground, initial.on_ground);
    assert_eq!(moved.interpolation, None);
    assert_eq!(moved.position_base.x, initial.position_base.x + 1.0);
    assert_eq!(moved.position_base.y, initial.position_base.y);
    assert_eq!(moved.position_base.z, initial.position_base.z - 0.5);
}

#[test]
fn relative_move_uses_old_minecart_interpolation_only_without_improvements() {
    let move_packet = ProtocolEntityMove {
        id: 10,
        has_position: true,
        delta_x: 4096,
        delta_y: 0,
        delta_z: 0,
        y_rot: Some(90.0),
        x_rot: Some(0.0),
        on_ground: true,
    };

    let mut old = WorldStore::new();
    old.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    let old_initial = old.probe_entity(10).unwrap();
    assert!(old.apply_entity_move(move_packet));
    let old_armed = old.probe_entity(10).unwrap();
    assert_eq!(old_armed.position, old_initial.position);
    assert_eq!(old_armed.interpolation.unwrap().remaining_steps, 3);
    old.advance_entity_client_animations(3);
    let old_finished = old.probe_entity(10).unwrap();
    assert_eq!(old_finished.position, old_finished.position_base);
    assert_eq!(old_finished.interpolation, None);

    let mut improved = WorldStore::new();
    enable_minecart_improvements(&mut improved);
    improved.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    assert!(improved.apply_entity_move(move_packet));
    let improved = improved.probe_entity(10).unwrap();
    assert_eq!(improved.position, improved.position_base);
    assert_eq!(improved.interpolation, None);
}

#[test]
fn move_vehicle_small_delta_acks_without_snap() {
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
fn local_vehicle_authority_uses_vanilla_controller_conditions() {
    let mut boat = WorldStore::new();
    boat.apply_login(&protocol_play_login(99));
    boat.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(boat.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert!(boat.entity_is_locally_authoritative(10));

    for entity_type_id in [
        VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ] {
        let mut mount = WorldStore::new();
        mount.set_default_item_equipment_slots(BTreeMap::from([(
            TEST_SADDLE_ITEM_ID,
            ItemEquipmentSlot::Saddle,
        )]));
        mount.apply_login(&protocol_play_login(99));
        mount.apply_add_entity(protocol_add_entity_with_type(10, entity_type_id));
        assert!(mount.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        }));
        assert!(!mount.entity_is_locally_authoritative(10));
        equip_test_saddle(&mut mount, 10);
        assert!(mount.entity_is_locally_authoritative(10));
    }

    for (entity_type_id, steering_item_id) in [
        (
            VANILLA_ENTITY_TYPE_PIG_ID,
            VANILLA_ITEM_CARROT_ON_A_STICK_ID,
        ),
        (
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
        ),
    ] {
        let mut mount = WorldStore::new();
        mount.set_default_item_equipment_slots(BTreeMap::from([(
            TEST_SADDLE_ITEM_ID,
            ItemEquipmentSlot::Saddle,
        )]));
        mount.apply_login(&protocol_play_login(99));
        mount.apply_add_entity(protocol_add_entity_with_type(10, entity_type_id));
        equip_test_saddle(&mut mount, 10);
        assert!(mount.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        }));
        assert!(!mount.entity_is_locally_authoritative(10));
        assert!(
            mount.apply_set_player_inventory(ProtocolSetPlayerInventory {
                slot: 0,
                item: item_stack(steering_item_id, 1),
            })
        );
        assert!(mount.entity_is_locally_authoritative(10));
    }

    let mut happy_ghast = WorldStore::new();
    happy_ghast.apply_login(&protocol_play_login(99));
    happy_ghast.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    ));
    assert!(happy_ghast.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert!(!happy_ghast.entity_is_locally_authoritative(10));
    assert!(happy_ghast.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 10,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: item_stack(77, 1),
        }],
    }));
    assert!(happy_ghast.entity_is_locally_authoritative(10));
    let accessors = vanilla_entity_metadata_happy_ghast_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    )
    .unwrap();
    assert!(happy_ghast.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.stays_still_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(!happy_ghast.entity_is_locally_authoritative(10));
}

#[test]
fn generic_mob_controller_uses_no_ai_and_generated_non_controlling_rider_tag() {
    let mut no_ai = WorldStore::new();
    no_ai.set_default_item_equipment_slots(BTreeMap::from([(
        TEST_SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    no_ai.apply_login(&protocol_play_login(99));
    no_ai.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    no_ai.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_HORSE_ID,
    ));
    equip_test_saddle(&mut no_ai, 20);
    assert!(no_ai.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    assert!(no_ai.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    assert!(no_ai.entity_is_locally_authoritative(10));
    let mob_accessors =
        vanilla_entity_metadata_mob_accessors_for_entity_type(VANILLA_ENTITY_TYPE_ZOMBIE_ID)
            .unwrap();
    assert!(no_ai.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: mob_accessors.flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(mob_accessors.no_ai_mask),
        }],
    }));
    assert!(!no_ai.entity_is_locally_authoritative(10));

    let mut tag = WorldStore::new();
    tag.set_default_item_equipment_slots(BTreeMap::from([(
        TEST_SADDLE_ITEM_ID,
        ItemEquipmentSlot::Saddle,
    )]));
    tag.apply_login(&protocol_play_login(99));
    for (id, entity_type_id) in [
        (10, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        (20, VANILLA_ENTITY_TYPE_SLIME_ID),
        (30, VANILLA_ENTITY_TYPE_HORSE_ID),
    ] {
        tag.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
    }
    equip_test_saddle(&mut tag, 30);
    assert!(tag.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![99],
    }));
    assert!(tag.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![30],
    }));
    assert!(tag.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    assert!(tag.entity_is_locally_authoritative(20));
    assert!(!tag.entity_is_locally_authoritative(10));
}

#[test]
fn move_vehicle_compares_active_target_before_cancel_and_abs_snap() {
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
    let target = EntityVec3 {
        x: 9.0,
        y: 70.0,
        z: -4.0,
    };
    store
        .entities
        .with_transform_mut(10, |transform| {
            transform.move_or_interpolate_to(Some(3), Some(target), Some(80.0), Some(12.0));
        })
        .unwrap();

    let same_target = store
        .apply_move_vehicle(ProtocolMoveVehicle {
            position: ProtocolVec3d {
                x: target.x,
                y: target.y,
                z: target.z,
            },
            y_rot: 80.0,
            x_rot: 12.0,
        })
        .unwrap();
    assert!(!same_target.snapped);
    assert!(store.probe_entity(10).unwrap().interpolation.is_some());

    let correction = store
        .apply_move_vehicle(ProtocolMoveVehicle {
            position: ProtocolVec3d {
                x: 40_000_001.0,
                y: 72.0,
                z: -40_000_001.0,
            },
            y_rot: 450.0,
            x_rot: 120.0,
        })
        .unwrap();
    assert!(correction.snapped);
    assert_eq!(correction.position.x, 30_000_000.0);
    assert_eq!(correction.position.z, -30_000_000.0);
    assert_eq!(correction.y_rot, 90.0);
    assert_eq!(correction.x_rot, 90.0);
    let corrected = store.probe_entity(10).unwrap();
    assert_eq!(corrected.interpolation, None);
    assert_eq!(corrected.position_base.x, 1.0);
    let transform = store.entities.transform(10).unwrap();
    assert_eq!(transform.old_position, correction.position);
    assert_eq!(transform.old_y_rot, correction.y_rot);
    assert_eq!(transform.old_x_rot, correction.x_rot);
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
fn tnt_minecart_smoke_uses_post_new_behavior_lerp_pose() {
    let mut store = WorldStore::new();
    enable_minecart_improvements(&mut store);
    store.apply_add_entity(protocol_add_entity_with_type(
        28,
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
    ));
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 28,
        event_id: 10,
    }));
    assert!(
        store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
            entity_id: 28,
            lerp_steps: vec![minecart_step(4.0, 67.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,)],
        })
    );

    store.advance_entity_client_animations(1);

    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    let [EntityClientMovementEffectState::Particle(EntityClientMovementParticleState::Simple(smoke))] =
        effects.as_slice()
    else {
        panic!("expected one TNT-minecart smoke effect, got {effects:?}");
    };
    assert_eq!(smoke.entity_id, 28);
    assert_eq!(smoke.kind, EntityClientMovementSimpleParticleKind::Smoke);
    assert_eq!(smoke.velocity, EntityVec3::default());
    assert!((smoke.position.x - 2.0).abs() < 1.0e-6);
    assert!((smoke.position.y - 65.5).abs() < 1.0e-6);
    assert!((smoke.position.z + 1.0).abs() < 1.0e-6);
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
    assert_eq!(
        store.entities.minecart_lerp(10).unwrap().steps.into_value(),
        vec![first]
    );

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
    assert_eq!(lerp.steps.as_slice(), &[first]);
    assert_eq!(lerp.pending_steps.as_slice(), &[second, third]);
    assert_eq!(store.counters().minecart_lerp_steps_tracked, 3);

    store.advance_entity_client_animations(3);
    let lerp = store.entities.minecart_lerp(10).unwrap();
    assert_eq!(lerp.steps.as_slice(), &[second, third]);
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
