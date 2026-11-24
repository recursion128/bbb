use super::*;

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
    store.apply_world_time(bbb_protocol::packets::PlayTime {
        game_time: 100,
        clock_updates: Vec::new(),
    });

    let (work, running_ticks) = store.advance_client_level_entity_ticks_with_observation(3);
    // The login's LocalPlayer ticks every raw tick like the Player id 11.
    assert_eq!(work.scanned_entities, 9);
    assert_eq!(work.mutated_entities, 7);
    assert_eq!(running_ticks, 1);
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
    assert_eq!(store.ticking().frozen_ticks_to_run, 0);
    assert_eq!(store.world_time().unwrap().game_time, 101);
}

#[test]
fn move_vehicle_snaps_root_vehicle_and_returns_ack() {
    let mut store = WorldStore::new();
    store.set_test_equippable_defaults([(
        TEST_SADDLE_ITEM_ID,
        crate::item_profiles::test_equippable(EquipmentSlot::Saddle),
    )]);
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
fn local_minecart_passenger_camera_follows_vehicle() {
    for improved in [false, true] {
        let mut store = WorldStore::new();
        store.apply_login(&protocol_play_login(99));
        store.set_local_player_pose(LocalPlayerPoseState::default());
        let mut chunk = empty_test_chunk();
        chunk.pos = crate::ChunkPos { x: 0, z: -1 };
        store.insert_decoded_chunk(chunk);
        if improved {
            enable_minecart_improvements(&mut store);
        }
        store.apply_add_entity(protocol_add_entity_with_type(
            10,
            VANILLA_ENTITY_TYPE_MINECART_ID,
        ));
        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        }));
        let seated = store.local_player_pose_at_partial_tick(0.5).unwrap();
        assert!((seated.position.x - 1.0).abs() < 1.0e-6);
        assert!((seated.position.y - 63.5875).abs() < 1.0e-6);
        if improved {
            assert!(
                store.apply_move_minecart_along_track(ProtocolMoveMinecartAlongTrack {
                    entity_id: 10,
                    lerp_steps: vec![
                        minecart_step(1.25, 64.1, -2.25, 0.2, 0.0, -0.2, 45.0, -10.0, 0.5),
                        minecart_step(1.75, 64.2, -2.75, 0.4, 0.0, -0.4, 90.0, 5.0, 1.25),
                    ],
                })
            );
        } else {
            assert!(store.apply_entity_move(ProtocolEntityMove {
                id: 10,
                has_position: true,
                delta_x: 4096,
                delta_y: 0,
                delta_z: 0,
                y_rot: None,
                x_rot: None,
                on_ground: true,
            }));
        }
        store.advance_entity_client_animations(1);
        store.sync_local_player_entity();
        let midpoint = store.local_player_pose_at_partial_tick(0.5).unwrap();
        let expected = if improved {
            EntityVec3 {
                x: 1.1458333333333333,
                y: 63.645833333333336,
                z: -2.1458333333333335,
            }
        } else {
            EntityVec3 {
                x: 1.1666666666666667,
                y: 63.5875,
                z: -2.0,
            }
        };
        assert!((midpoint.position.x - expected.x).abs() < 1.0e-6);
        assert!((midpoint.position.y - expected.y).abs() < 1.0e-6);
        assert!((midpoint.position.z - expected.z).abs() < 1.0e-6);
        let arrived = store.local_player_pose().unwrap().position;
        assert!(arrived.x > seated.position.x);
        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 10,
            passenger_ids: Vec::new(),
        }));
        store.advance_entity_client_animations(1);
        store.sync_local_player_entity();
        assert_eq!(store.local_player_pose().unwrap().position, arrived);
    }
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
fn new_minecart_passengers_follow_the_weighted_vehicle_render_pose() {
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
            x: 1.0,
            y: 64.1875,
            z: -2.0,
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

    // After the promotion tick, the canonical cart is only one third through
    // the batch. At partial 0.5 the weighted render position is
    // (1.1458333, 64.0583333, -2.1458333); the cow follows that pose plus the
    // minecart's generated 0.1875 passenger attachment.
    let shifted = passenger_position(&store, 0.5);
    assert!(
        (shifted.x - 1.1458333333333333).abs() < 1.0e-6,
        "x position: {}",
        shifted.x
    );
    assert!(
        (shifted.y - 64.24583333333334).abs() < 1.0e-6,
        "y position: {}",
        shifted.y
    );
    assert!(
        (shifted.z - -2.1458333333333335).abs() < 1.0e-6,
        "z position: {}",
        shifted.z
    );

    store.advance_entity_client_animations(3);
    assert_eq!(
        passenger_position(&store, 0.5),
        EntityVec3 {
            x: 1.75,
            y: 64.3875,
            z: -2.75,
        },
        "after the lerp drains, passengers stay attached to the final cart pose"
    );
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
    assert!(!source.render.kind.minecart().new_render);
    assert_eq!(source.render.kind.minecart().pos_on_rail, None);
    assert_eq!(source.render.kind.minecart().front_pos, None);
    assert_eq!(source.render.kind.minecart().back_pos, None);

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
    assert!(!source.render.kind.minecart().new_render);
    assert_close3_f32(
        source.render.kind.minecart().pos_on_rail.unwrap(),
        [2.5, 1.5625, 3.5],
    );
    assert_close3_f32(
        source.render.kind.minecart().front_pos.unwrap(),
        [2.8, 1.8625, 3.5],
    );
    assert_close3_f32(
        source.render.kind.minecart().back_pos.unwrap(),
        [2.2, 1.2625, 3.5],
    );
}
