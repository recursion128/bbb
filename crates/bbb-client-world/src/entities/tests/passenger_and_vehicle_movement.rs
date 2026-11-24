use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID;
use bbb_protocol::item_types::{
    VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
};

use crate::LocalPlayerPoseState;
use bbb_protocol::equipment_slots::EquipmentSlot;

use super::*;
use crate::entities::schema::vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type;

mod boats;
mod camels_and_nautilus;
mod ghasts_and_leashes;
mod horses_and_pigs;
mod minecarts_and_sync;
mod pushing_and_schedule;

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
    // The login's LocalPlayer is scanned with the tracked entities.
    let work = store.advance_entity_client_animations_with_observation(1);
    assert_eq!(work.scanned_entities, 5);
    assert_eq!(work.mutated_entities, 3);
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
    // Only the always-ticking LocalPlayer root still advances.
    let paused = store.advance_entity_client_animations_with_observation(1);
    assert_eq!(paused.mutated_entities, 1);
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

    // The login's LocalPlayer is a Player root and keeps ticking too.
    let work = store.advance_entity_client_animations_with_observation(1);
    assert_eq!(work.mutated_entities, 5);
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
fn passenger_ride_tick_clears_motion_before_client_entity_tick() {
    let mut store = WorldStore::new();
    let vehicle_id = 10;
    let passenger_id = 20;
    store.apply_add_entity(protocol_add_entity_with_type(
        vehicle_id,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        passenger_id,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
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
fn passenger_render_positions_follow_the_partial_tick_root_pose_through_nested_edges() {
    let mut store = WorldStore::new();
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 12.0,
            y: 72.0,
            z: 7.0,
        },
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_OAK_BOAT_ID)
    });
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 50.0,
            y: 90.0,
            z: 50.0,
        },
        ..protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_COW_ID)
    });
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: -30.0,
            y: 40.0,
            z: -30.0,
        },
        ..protocol_add_entity_with_type(30, VANILLA_ENTITY_TYPE_CHICKEN_ID)
    });
    store.entities.with_transform_mut(10, |transform| {
        transform.old_position = EntityVec3 {
            x: 8.0,
            y: 68.0,
            z: 3.0,
        };
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![30],
    }));

    let source_position = |entity_id| {
        store
            .entity_model_source_at_partial_tick(entity_id, 0.5)
            .expect("nested passenger model source")
            .position
    };
    assert_entity_vec3_close(
        source_position(20),
        EntityVec3 {
            x: 10.0,
            y: 70.1875,
            z: 5.0,
        },
    );
    assert_entity_vec3_close(
        source_position(30),
        EntityVec3 {
            x: 10.0,
            y: 71.55625,
            z: 5.0,
        },
    );
    assert_eq!(
        store.entities.transform(20).unwrap().position,
        EntityVec3 {
            x: 50.0,
            y: 90.0,
            z: 50.0,
        },
        "render projection must not mutate the packet-owned passenger transform"
    );
}

#[test]
fn camel_passenger_render_positions_consume_the_body_anchor_curve() {
    let mut store = WorldStore::new();
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 4.0,
            y: 70.0,
            z: -3.0,
        },
        y_rot: 0.0,
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_CAMEL_ID)
    });
    for id in [20, 21] {
        store.apply_add_entity(protocol_add_entity_with_type(
            id,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
        ));
    }
    let entity_accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated Entity metadata accessors must resolve");
    let (pose_tick_data_id, _) =
        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CAMEL_ID,
        )
        .expect("generated Camel pose tick accessor");
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![
            protocol_pose_data(
                entity_accessors.pose_data_id,
                bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SITTING_ID,
            ),
            protocol_long_data(pose_tick_data_id, -100),
        ],
    }));
    store.apply_world_time(ProtocolPlayTime {
        game_time: 128,
        clock_updates: Vec::new(),
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20, 21],
    }));

    let position = |entity_id| {
        store
            .entity_model_source_at_partial_tick(entity_id, 0.0)
            .expect("Camel passenger model source")
            .position
    };
    // Vanilla Camel.getPassengerAttachmentPoint uses the front and rear
    // getBodyAnchorAnimationYOffset curves. Player.getVehicleAttachmentPoint
    // then subtracts 0.6 from each riding-position Y coordinate.
    assert_entity_vec3_close(
        position(20),
        EntityVec3 {
            x: 4.0,
            y: 70.785,
            z: -2.5,
        },
    );
    assert_entity_vec3_close(
        position(21),
        EntityVec3 {
            x: 4.0,
            y: 71.277,
            z: -3.7,
        },
    );
}

#[test]
fn strider_passenger_attachment_includes_the_client_walk_bob() {
    let mut store = WorldStore::new();
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        ..protocol_add_entity_with_type(10, VANILLA_ENTITY_TYPE_STRIDER_ID)
    });
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20],
    }));
    store.advance_entity_client_animations(1);
    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 10,
            position: ProtocolVec3d {
                x: 0.5,
                y: 64.0,
                z: 0.0,
            },
            delta_movement: ProtocolVec3d::default(),
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );
    store.advance_entity_client_animations(1);

    let strider = store
        .entity_model_source_at_partial_tick(10, 1.0)
        .expect("Strider source");
    assert!(strider.render.walk_animation_speed > 0.0);
    let expected_bob = 0.12
        * (strider.render.walk_animation_pos * 1.5).cos()
        * 2.0
        * strider.render.walk_animation_speed.min(0.25);
    let passenger = store
        .entity_model_source_at_partial_tick(20, 1.0)
        .expect("Strider passenger source");
    assert!(
        (passenger.position.y - (strider.position.y + 1.7 + f64::from(expected_bob))).abs()
            < 1.0e-6,
        "passenger y: {}, strider y: {}, bob: {expected_bob}",
        passenger.position.y,
        strider.position.y
    );
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
fn tracks_local_player_passenger() {
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
    assert_eq!(store.probe_entity(99).unwrap().vehicle_id, Some(10));
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
            content: bbb_protocol::RichTextComponent::plain("Press Left Shift to Dismount"),
            display_ticks: 60,
            animate_color: false,
        })
    );
    let narrations = store.take_pending_chat_narrations();
    assert_eq!(narrations.len(), 1);
    assert_eq!(narrations[0].kind, crate::ChatNarrationKind::SystemNow);
    assert_eq!(narrations[0].content, "Press Left Shift to Dismount");

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
    assert!(store.take_pending_chat_narrations().is_empty());
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
fn local_player_rideable_jumping_vehicle_cooldown_tracks_camel_dash_cooldown() {
    let (camel_dash_data_id, _) =
        vanilla_entity_metadata_camel_dash_accessor_for_entity_type(VANILLA_ENTITY_TYPE_CAMEL_ID)
            .expect("generated Camel dash accessor must resolve");
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    store.apply_login(&protocol_play_login(99));
    store.apply_world_time(bbb_protocol::packets::PlayTime {
        game_time: 100,
        clock_updates: Vec::new(),
    });
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
fn local_abstract_horse_jump_release_is_retained_then_consumed_by_ridden_tick() {
    let mut store = locally_controlled_horse();
    store.set_local_player_pose(crate::LocalPlayerPoseState {
        y_rot: 90.0,
        ..crate::LocalPlayerPoseState::default()
    });
    store.entities.with_transform_mut(10, |transform| {
        transform.on_ground = Some(true);
    });
    let generation_a = store.publication_snapshot();
    let before = store.entities.transform(10).unwrap();

    assert!(store.apply_local_player_riding_jump(45));
    let pending = store.entities.ridden_jump_state(10).unwrap();
    assert!((pending.player_jump_pending_scale - 0.6).abs() < 1.0e-6);
    assert!(pending.allow_stand_sliding);
    assert_eq!(pending.local_stand_ticks_remaining, 20);
    let accessors = vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_HORSE_ID,
    )
    .unwrap();
    assert_ne!(
        store
            .entities
            .metadata_byte(10, accessors.flags_data_id, accessors.flags_default)
            .unwrap()
            & accessors.standing_mask,
        0
    );

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
    let jump_strength = store
        .entities
        .attribute_value(
            10,
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID,
        )
        .unwrap();
    let expected_impulse = jump_strength * 0.6;
    assert!((after.position.y - (before.position.y + expected_impulse)).abs() < 1.0e-6);
    assert!(after.position.x < before.position.x - 0.23);
    assert_eq!(
        store
            .entities
            .ridden_jump_state(10)
            .unwrap()
            .player_jump_pending_scale,
        0.0
    );

    assert_eq!(
        generation_a
            .entities
            .ridden_jump_state(10)
            .unwrap()
            .player_jump_pending_scale,
        0.0
    );
    assert_eq!(generation_a.entities.transform(10).unwrap(), before);
}

#[test]
fn local_camel_dash_consumes_pending_jump_and_starts_exact_cooldown() {
    let mut store = locally_controlled_camel();
    store.set_local_player_pose(crate::LocalPlayerPoseState {
        y_rot: 0.0,
        ..crate::LocalPlayerPoseState::default()
    });
    let generation_a = store.publication_snapshot();
    let before = store.entities.transform(10).unwrap();

    assert!(store.apply_local_player_riding_jump(90));
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
    assert!(after.position.z > before.position.z + 1.9);
    assert!(after.position.y > before.position.y + 0.5);
    assert_eq!(store.entities.camel_jump_cooldown_ticks(10), Some(54));
    assert_eq!(
        store.local_player_rideable_jumping_vehicle_cooldown_ticks(),
        Some(54)
    );
    let (dash_data_id, dash_default) =
        vanilla_entity_metadata_camel_dash_accessor_for_entity_type(VANILLA_ENTITY_TYPE_CAMEL_ID)
            .unwrap();
    assert!(store
        .entities
        .metadata_bool_for_entity(10, dash_data_id, dash_default)
        .unwrap());
    assert!(!store.apply_local_player_riding_jump(90));

    assert_eq!(generation_a.entities.transform(10).unwrap(), before);
    assert_eq!(generation_a.entities.camel_jump_cooldown_ticks(10), Some(0));
}

#[test]
fn recreated_camel_id_does_not_retain_local_dash_state() {
    let mut store = locally_controlled_camel();
    assert!(store.apply_local_player_riding_jump(90));
    store
        .advance_local_abstract_horse_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    let generation_a = store.publication_snapshot();
    assert_eq!(
        generation_a.entities.camel_jump_cooldown_ticks(10),
        Some(54)
    );

    assert_eq!(
        store.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![10],
        }),
        1
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    equip_test_saddle(&mut store, 10);
    store.entities.with_transform_mut(10, |transform| {
        transform.on_ground = Some(true);
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));

    let (dash_data_id, dash_default) =
        vanilla_entity_metadata_camel_dash_accessor_for_entity_type(VANILLA_ENTITY_TYPE_CAMEL_ID)
            .unwrap();
    assert_eq!(store.entities.camel_jump_cooldown_ticks(10), Some(0));
    assert!(!store
        .entities
        .metadata_bool_for_entity(10, dash_data_id, dash_default)
        .unwrap());
    assert!(store.apply_local_player_riding_jump(90));
    assert_eq!(
        generation_a.entities.camel_jump_cooldown_ticks(10),
        Some(54)
    );
    assert!(generation_a
        .entities
        .metadata_bool_for_entity(10, dash_data_id, dash_default)
        .unwrap());
}

#[test]
fn sitting_camel_forward_input_stands_then_refuses_current_tick_travel() {
    let mut store = locally_controlled_camel();
    let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    let (last_pose_data_id, _) =
        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CAMEL_ID,
        )
        .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![
            ProtocolEntityDataValue {
                data_id: entity_accessors.pose_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                value: EntityDataValueKind::Pose(
                    bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SITTING_ID,
                ),
            },
            ProtocolEntityDataValue {
                data_id: last_pose_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
                value: EntityDataValueKind::Long(-1),
            },
        ],
    }));
    store.entities.with_transform_mut(10, |transform| {
        transform.delta_movement.x = 0.5;
        transform.delta_movement.z = 0.5;
        transform.on_ground = Some(true);
    });
    let before = store.entities.transform(10).unwrap();
    assert_eq!(store.local_player_rideable_jumping_vehicle_id(), None);

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
    assert_eq!(
        store.entities.pose(10),
        Some(VANILLA_ENTITY_POSE_STANDING_ID)
    );
    assert_eq!(
        store
            .entities
            .metadata_long(10, last_pose_data_id, 0)
            .unwrap(),
        100
    );
}

#[test]
fn local_camel_dash_flag_clears_after_minimum_duration_on_ground() {
    let mut store = locally_controlled_camel();
    assert!(store.apply_local_player_riding_jump(90));
    store
        .advance_local_abstract_horse_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();
    store.advance_entity_client_animations(6);
    store.insert_decoded_chunk(empty_test_chunk());
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 1, y: -64, z: 1 },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    store.entities.with_transform_mut(10, |transform| {
        transform.position = EntityVec3 {
            x: 1.0,
            y: -63.0,
            z: 1.0,
        };
        transform.old_position = transform.position;
        transform.delta_movement = EntityVec3 {
            y: -0.1,
            ..EntityVec3::default()
        };
        transform.on_ground = Some(true);
    });
    store
        .advance_local_abstract_horse_vehicle_input(crate::LocalPlayerInputState::default(), 1)
        .unwrap();

    assert_eq!(store.entities.camel_jump_cooldown_ticks(10), Some(48));
    assert!(store
        .entities
        .transform(10)
        .unwrap()
        .on_ground
        .unwrap_or(false));

    let (dash_data_id, dash_default) =
        vanilla_entity_metadata_camel_dash_accessor_for_entity_type(VANILLA_ENTITY_TYPE_CAMEL_ID)
            .unwrap();
    assert!(!store
        .entities
        .metadata_bool_for_entity(10, dash_data_id, dash_default)
        .unwrap());
}

#[test]
fn camel_dash_cooldown_reaches_zero() {
    let mut store = locally_controlled_camel();
    let (dash_data_id, _) =
        vanilla_entity_metadata_camel_dash_accessor_for_entity_type(VANILLA_ENTITY_TYPE_CAMEL_ID)
            .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: dash_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));

    store.advance_entity_client_animations(55);
    assert_eq!(store.entities.camel_jump_cooldown_ticks(10), Some(0));
}

#[test]
fn remote_camel_tick_clears_dash_after_minimum_duration() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.entities.with_transform_mut(10, |transform| {
        transform.on_ground = Some(true);
    });
    let (dash_data_id, dash_default) =
        vanilla_entity_metadata_camel_dash_accessor_for_entity_type(VANILLA_ENTITY_TYPE_CAMEL_ID)
            .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: dash_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(7);

    assert!(!store
        .entities
        .metadata_bool_for_entity(10, dash_data_id, dash_default)
        .unwrap());
    assert_eq!(store.entities.camel_jump_cooldown_ticks(10), Some(48));
    assert!(generation_a
        .entities
        .metadata_bool_for_entity(10, dash_data_id, dash_default)
        .unwrap());
}

#[test]
fn remote_sitting_camel_in_water_stands_instantly() {
    let mut store = WorldStore::new();
    store.apply_world_time(bbb_protocol::packets::PlayTime {
        game_time: 100,
        clock_updates: Vec::new(),
    });
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 1,
            y: -63,
            z: -2
        },
        block_state_id: vanilla_block_state_id("minecraft:water", [("level", "0")]),
    }));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.entities.with_transform_mut(10, |transform| {
        transform.position.y = -63.0;
        transform.old_position = transform.position;
    });
    let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    let (last_pose_data_id, _) =
        vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CAMEL_ID,
        )
        .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![
            ProtocolEntityDataValue {
                data_id: entity_accessors.pose_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                value: EntityDataValueKind::Pose(
                    bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SITTING_ID,
                ),
            },
            ProtocolEntityDataValue {
                data_id: last_pose_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_LONG_ID,
                value: EntityDataValueKind::Long(-100),
            },
        ],
    }));
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    assert_eq!(
        store.entities.pose(10),
        Some(VANILLA_ENTITY_POSE_STANDING_ID)
    );
    assert_eq!(
        store.entities.metadata_long(10, last_pose_data_id, 0),
        Some(47)
    );
    assert!(store.take_entity_client_movement_effect_states().is_empty());
    assert_eq!(
        generation_a.entities.pose(10),
        Some(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SITTING_ID)
    );
    assert_eq!(
        generation_a
            .entities
            .metadata_long(10, last_pose_data_id, 0),
        Some(-100)
    );
}

#[test]
fn nautilus_synced_dash_update_seeds_cooldown_even_when_false() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ));
    store.advance_entity_client_animations(1);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(false),
        }],
    }));

    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(40));
    store.advance_entity_client_animations(1);
    assert_eq!(store.entities.nautilus_jump_cooldown_ticks(10), Some(39));
}

fn locally_controlled_horse() -> WorldStore {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
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

fn locally_controlled_camel() -> WorldStore {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    store.apply_login(&protocol_play_login(99));
    store.apply_world_time(bbb_protocol::packets::PlayTime {
        game_time: 100,
        clock_updates: Vec::new(),
    });
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    equip_test_saddle(&mut store, 10);
    store.entities.with_transform_mut(10, |transform| {
        transform.on_ground = Some(true);
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    store.set_local_player_pose(crate::LocalPlayerPoseState::default());
    store
}

fn locally_controlled_nautilus(entity_type_id: i32) -> WorldStore {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(10, entity_type_id));
    equip_test_saddle(&mut store, 10);
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    store.set_local_player_pose(LocalPlayerPoseState::default());
    store
}

fn locally_controlled_item_based_steering(
    entity_type_id: i32,
    steering_item_id: i32,
) -> WorldStore {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
    store.apply_login(&protocol_play_login(99));
    store.apply_add_entity(protocol_add_entity_with_type(10, entity_type_id));
    equip_test_saddle(&mut store, 10);
    assert!(
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 0,
            item: item_stack(steering_item_id, 1),
        })
    );
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    store.entities.with_transform_mut(10, |transform| {
        transform.on_ground = Some(true);
    });
    store.set_local_player_pose(LocalPlayerPoseState::default());
    store
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
fn ordinary_living_tick_uses_scaled_local_bounds_and_one_immutable_push_generation() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    let mut cow = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_COW_ID);
    cow.uuid = Uuid::from_u128(0x20);
    store.apply_add_entity(cow);
    let mut local_player = protocol_add_entity_with_type(99, VANILLA_ENTITY_TYPE_PLAYER_ID);
    local_player.uuid = Uuid::from_u128(0x99);
    local_player.position.x = 1.8;
    store.apply_add_entity(local_player);
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.8,
            y: 64.0,
            z: -2.0,
        },
        ..LocalPlayerPoseState::default()
    });

    store.advance_entity_client_animations(1);
    assert_eq!(
        store.local_player_pose().unwrap().delta_movement,
        ProtocolVec3d::default(),
        "unscaled player bounds do not intersect the cow"
    );
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 99,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    let retained = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let distance = 0.8_f64.sqrt();
    let expected_push = (0.8 / distance) * f64::from(0.05_f32);
    let live_pose = store.local_player_pose().unwrap();
    let live_cow = store.entities.transform(20).unwrap();
    assert!((live_pose.delta_movement.x - expected_push).abs() < 1.0e-12);
    assert!((live_cow.delta_movement.x + expected_push).abs() < 1.0e-12);
    assert_eq!(live_pose.delta_movement.z, 0.0);
    assert_eq!(live_cow.delta_movement.z, 0.0);
    assert_eq!(
        retained.local_player_pose().unwrap().delta_movement,
        ProtocolVec3d::default()
    );
    assert_eq!(
        retained.probe_entity(20).unwrap().delta_movement,
        EntityVec3::default()
    );
}

#[test]
fn ordinary_living_push_rejects_same_vehicle_and_spectator_local_player() {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut loaded = empty_test_chunk();
    loaded.pos = crate::ChunkPos { x: 0, z: -1 };
    store.insert_decoded_chunk(loaded);
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    let mut cow = protocol_add_entity_with_type(20, VANILLA_ENTITY_TYPE_COW_ID);
    cow.uuid = Uuid::from_u128(0x20);
    store.apply_add_entity(cow);
    let local_uuid = Uuid::from_u128(0x99);
    let mut local_player = protocol_add_entity_with_type(99, VANILLA_ENTITY_TYPE_PLAYER_ID);
    local_player.uuid = local_uuid;
    local_player.position.x = 1.4;
    store.apply_add_entity(local_player);
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 1.4,
            y: 64.0,
            z: -2.0,
        },
        ..LocalPlayerPoseState::default()
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![20, 99],
    }));

    store.advance_entity_client_animations(1);
    assert_eq!(
        store.local_player_pose().unwrap().delta_movement,
        ProtocolVec3d::default(),
        "entities sharing one root vehicle do not push"
    );

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 10,
        passenger_ids: Vec::new(),
    }));
    store.apply_game_event(ProtocolGameEvent {
        event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID,
        param: 3.0,
    });
    store.advance_entity_client_animations(1);
    assert_eq!(
        store.local_player_pose().unwrap().delta_movement,
        ProtocolVec3d::default(),
        "the local spectator no-physics target remains excluded"
    );
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
        .render_position_at_partial_tick(99, 1.0, 0, false)
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
    let mounted_pose = store.local_player_pose().unwrap();
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
        mounted_pose.position
    );
    assert_eq!(generation_a.local_player_old_pose, Some(mounted_pose));
    let generation_b = store.publication_snapshot();
    assert_eq!(
        generation_b.probe_entity(10).unwrap().position,
        report.position
    );
    assert_eq!(generation_b.local_player_old_pose, Some(pose));
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
        mount.set_test_equippable_defaults([(
            TEST_SADDLE_ITEM_ID,
            crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
        )]);
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
        mount.set_test_equippable_defaults([(
            TEST_SADDLE_ITEM_ID,
            crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
        )]);
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
    assert!(source.render.kind.minecart().new_render);
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
    assert!(source.render.kind.minecart().new_render);
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
    assert!(source.render.kind.minecart().new_render);
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
            .render
            .kind
            .minecart()
            .new_render
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
            .render
            .kind
            .minecart()
            .new_render
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

fn enable_minecart_improvements(store: &mut WorldStore) {
    store.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec![
            "minecraft:vanilla".to_string(),
            "minecraft:minecart_improvements".to_string(),
        ],
    });
}

fn world_with_dimension_and_block(block_name: &str) -> WorldStore {
    let mut store = WorldStore::new();
    store.apply_login(&protocol_play_login(99));
    let mut chunk = empty_test_chunk();
    let dimension = store.dimension();
    let section = chunk.sections[0].clone();
    chunk.sections = vec![section; (dimension.height / 16).max(1) as usize];
    store.insert_decoded_chunk(chunk);
    set_test_block(&mut store, block_name);
    store
}

fn set_test_block(store: &mut WorldStore, block_name: &str) {
    let block_state_id = crate::BlockStateRegistry::vanilla_26_1()
        .iter()
        .find(|state| state.name == block_name)
        .unwrap_or_else(|| panic!("missing Vanilla block state {block_name}"))
        .id;
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 8, y: 2, z: 8 },
        block_state_id,
    }));
}

#[test]
fn baby_equine_and_panda_passenger_attachments_use_vanilla_declared_points() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        30,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DONKEY_ID,
        0.0,
    ));
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 8.0,
            y: 70.0,
            z: 4.0,
        },
        ..protocol_add_entity_with_type(31, VANILLA_ENTITY_TYPE_COW_ID)
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 30,
        passenger_ids: vec![31],
    }));

    // The F3+B passenger-vehicle slab renders at vanilla
    // `getPassengerRidingPosition`, which exposes the attachment point for
    // remote riders each frame.
    let riding_position = |store: &WorldStore, passenger_id: i32| {
        store
            .entity_debug_passenger_vehicle_target(passenger_id)
            .expect("passenger vehicle debug target")
            .position
    };
    let adult = riding_position(&store, 31);

    let (donkey_baby_id, _) = super::schema::vanilla_entity_metadata_baby_accessor_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DONKEY_ID,
    )
    .expect("generated Donkey baby accessor");
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 30,
        values: vec![protocol_bool_data(donkey_baby_id, true)],
    }));
    let baby = riding_position(&store, 31);

    // Vanilla `AbstractChestedHorse` babies re-declare the passenger point as
    // `(0, height + 0.03125, -0.3125)` before the 0.5 scale: the adult table
    // point 1.1125 drops to 1.53125 * 0.5 = 0.765625 and the rider slides
    // 0.3125 * 0.5 behind the origin, instead of the plain half-scaled
    // 1.1125 * 0.5 = 0.55625 at z 0.
    assert!((baby.x - adult.x).abs() < 1.0e-6);
    assert!(
        ((adult.y - baby.y) - (1.1125 - 0.765_625)).abs() < 1.0e-6,
        "adult {adult:?} baby {baby:?}"
    );
    assert!(((baby.z - adult.z) - (-0.156_25)).abs() < 1.0e-6);

    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        32,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PANDA_ID,
        0.0,
    ));
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: -3.0,
            y: 50.0,
            z: 12.0,
        },
        ..protocol_add_entity_with_type(33, VANILLA_ENTITY_TYPE_COW_ID)
    });
    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 32,
        passenger_ids: vec![33],
    }));
    let adult = riding_position(&store, 33);

    let (panda_baby_id, _) = super::schema::vanilla_entity_metadata_baby_accessor_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PANDA_ID,
    )
    .expect("generated Panda baby accessor");
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 32,
        values: vec![protocol_bool_data(panda_baby_id, true)],
    }));
    let baby = riding_position(&store, 33);

    // Vanilla panda babies attach at the post-scale 0.40625, not at half of
    // the adult 1.25 box top (0.625).
    assert!(
        ((adult.y - baby.y) - (1.25 - 0.406_25)).abs() < 1.0e-6,
        "adult {adult:?} baby {baby:?}"
    );
    assert!((baby.z - adult.z).abs() < 1.0e-6);
}

#[test]
fn camel_leash_offset_keeps_attribute_scale_out_of_the_animation_scale() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        69,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID,
    ));
    let (baby_data_id, _) = super::schema::vanilla_entity_metadata_baby_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_CAMEL_ID,
    )
    .expect("generated Camel baby accessor");
    let (pose_tick_data_id, _) =
        super::schema::vanilla_entity_metadata_camel_last_pose_change_tick_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CAMEL_ID,
        )
        .expect("generated Camel pose tick accessor");
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 69,
        values: vec![
            protocol_bool_data(baby_data_id, true),
            protocol_long_data(pose_tick_data_id, 0),
        ],
    }));
    assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
        entity_id: 69,
        attributes: vec![ProtocolAttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 2.0,
            modifiers: Vec::new(),
        }],
    }));
    store.apply_world_time(ProtocolPlayTime {
        game_time: 200,
        clock_updates: Vec::new(),
    });
    assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
        source_id: 69,
        dest_id: 70,
    }));

    let state = store
        .entity_leash_render_states_at_partial_tick(69, 0.0)
        .expect("baby Camel rope")[0];
    // Dimensions are 1.7 x 2.375 scaled by age 0.6 and attribute 2.0.
    // The standing animation base subtracts `0.375 * ageScale`, then the
    // leash offset subtracts another `0.2 * ageScale`.
    assert!((state.offset[1] - 2.505).abs() < 1.0e-6, "state {state:?}");
    let horizontal_offset = state.offset[0].hypot(state.offset[2]);
    assert!(
        (horizontal_offset - 1.1424).abs() < 1.0e-6,
        "state {state:?}"
    );
}
