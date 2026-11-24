use std::collections::BTreeMap;

use super::*;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;
use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, EntityEvent as ProtocolEntityEvent, PlayerLookAtTarget,
    Vec3d as ProtocolVec3d, PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT,
    PLAYER_RELATIVE_Y_ROT,
};
use uuid::Uuid;

#[test]
fn login_player_abilities_match_vanilla_game_types() {
    let survival = LocalPlayerAbilitiesState {
        invulnerable: false,
        flying: false,
        can_fly: false,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.1,
    };
    assert_eq!(game_type_player_abilities(GameType::Survival), survival);
    assert_eq!(game_type_player_abilities(GameType::Adventure), survival);
    assert_eq!(
        game_type_player_abilities(GameType::Creative),
        LocalPlayerAbilitiesState {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }
    );
    assert_eq!(
        game_type_player_abilities(GameType::Spectator),
        LocalPlayerAbilitiesState {
            invulnerable: true,
            flying: true,
            can_fly: true,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }
    );
}

#[test]
fn local_player_packets_update_canonical_state() {
    let mut store = WorldStore::new();

    store.apply_player_abilities(ProtocolPlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    store.apply_player_health(ProtocolPlayerHealth {
        health: 7.5,
        food: 16,
        saturation: 2.0,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.75,
        level: 8,
        total: 123,
    });
    assert!(store.apply_held_slot(ProtocolSetHeldSlot { slot: 5 }));
    assert!(!store.apply_held_slot(ProtocolSetHeldSlot { slot: 99 }));
    store.apply_default_spawn_position(ProtocolSetDefaultSpawnPosition {
        dimension: "minecraft:overworld".to_string(),
        pos: bbb_protocol::packets::BlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        yaw: 90.0,
        pitch: -10.0,
    });
    store.apply_simulation_distance(ProtocolSetSimulationDistance { distance: 12 });

    let local = store.local_player();
    assert_eq!(
        local.abilities,
        Some(LocalPlayerAbilitiesState {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        })
    );
    assert_eq!(
        local.health,
        Some(LocalPlayerHealthState {
            health: 7.5,
            food: 16,
            saturation: 2.0,
        })
    );
    assert_eq!(
        local.experience,
        Some(LocalPlayerExperienceState {
            progress: 0.75,
            level: 8,
            total: 123,
        })
    );
    assert_eq!(local.selected_hotbar_slot, 5);
    assert_eq!(
        local.default_spawn,
        Some(DefaultSpawnState {
            dimension: "minecraft:overworld".to_string(),
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            yaw: 90.0,
            pitch: -10.0,
        })
    );
    assert_eq!(local.simulation_distance, Some(12));

    let counters = store.counters();
    assert_eq!(counters.player_abilities_packets, 1);
    assert_eq!(counters.player_health_packets, 1);
    assert_eq!(counters.player_experience_packets, 1);
    assert_eq!(counters.held_slot_packets, 2);
    assert_eq!(counters.held_slot_updates_applied, 1);
    assert_eq!(counters.held_slot_updates_ignored, 1);
    assert_eq!(counters.default_spawn_position_packets, 1);
    assert_eq!(counters.simulation_distance_packets, 1);
}

#[test]
fn local_player_is_dead_tracks_canonical_health() {
    let mut store = WorldStore::new();

    assert!(!store.local_player_is_dead());

    store.apply_player_health(ProtocolPlayerHealth {
        health: 7.5,
        food: 16,
        saturation: 2.0,
    });
    assert!(!store.local_player_is_dead());

    store.apply_player_health(ProtocolPlayerHealth {
        health: 0.0,
        food: 16,
        saturation: 2.0,
    });
    assert!(store.local_player_is_dead());

    store.apply_player_health(ProtocolPlayerHealth {
        health: 1.0,
        food: 16,
        saturation: 2.0,
    });
    assert!(!store.local_player_is_dead());
}

#[test]
fn local_player_permission_entity_events_update_gamemaster_gate_without_entity_projection() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(7);

    assert_eq!(
        store.local_player_permission_level(),
        LocalPlayerPermissionLevel::NoPermissions
    );
    assert!(!store.local_player_has_gamemaster_permission());

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 7,
        event_id: 26,
    }));
    assert_eq!(
        store.local_player_permission_level(),
        LocalPlayerPermissionLevel::Gamemaster
    );
    assert!(store.local_player_has_gamemaster_permission());

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 7,
        event_id: 25,
    }));
    assert_eq!(
        store.local_player_permission_level(),
        LocalPlayerPermissionLevel::Moderator
    );
    assert!(!store.local_player_has_gamemaster_permission());

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 7,
        event_id: 28,
    }));
    assert_eq!(
        store.local_player_permission_level(),
        LocalPlayerPermissionLevel::Owner
    );
    assert!(store.local_player_has_gamemaster_permission());

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 7,
        event_id: 24,
    }));
    assert_eq!(
        store.local_player_permission_level(),
        LocalPlayerPermissionLevel::NoPermissions
    );
    assert!(!store.local_player_has_gamemaster_permission());
    assert_eq!(store.counters().entity_events_applied, 4);
    assert_eq!(store.counters().entity_events_ignored, 0);
}

#[test]
fn remote_permission_entity_event_does_not_update_local_player_permission() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(7);
    store.apply_add_entity(ProtocolAddEntity {
        id: 8,
        uuid: Uuid::from_u128(8),
        entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 8,
        event_id: 26,
    }));

    assert_eq!(
        store.local_player_permission_level(),
        LocalPlayerPermissionLevel::NoPermissions
    );
    assert!(!store.local_player_has_gamemaster_permission());
    assert_eq!(store.counters().entity_events_applied, 1);
    assert_eq!(store.counters().entity_events_ignored, 0);
}

#[test]
fn local_hotbar_selection_updates_without_counting_server_packet() {
    let mut store = WorldStore::new();

    assert!(store.set_local_selected_hotbar_slot(7));
    assert_eq!(store.local_player().selected_hotbar_slot, 7);
    assert_eq!(store.counters().held_slot_packets, 0);
    assert_eq!(store.counters().held_slot_updates_applied, 0);
    assert_eq!(store.counters().held_slot_updates_ignored, 0);

    assert!(!store.set_local_selected_hotbar_slot(9));
    assert_eq!(store.local_player().selected_hotbar_slot, 7);
    assert_eq!(store.counters().held_slot_packets, 0);
    assert_eq!(store.counters().held_slot_updates_applied, 0);
    assert_eq!(store.counters().held_slot_updates_ignored, 0);
}

#[test]
fn local_flying_updates_only_when_server_allows_flight() {
    let mut store = WorldStore::new();

    assert!(!store.set_local_flying(true));

    store.apply_player_abilities(ProtocolPlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: false,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    assert!(!store.set_local_flying(true));
    assert!(!store.local_player().abilities.unwrap().flying);

    store.apply_player_abilities(ProtocolPlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: true,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    assert!(store.set_local_flying(true));
    assert!(store.local_player().abilities.unwrap().flying);
    assert!(store.set_local_flying(false));
    assert!(!store.local_player().abilities.unwrap().flying);
    assert_eq!(store.counters().player_abilities_packets, 2);
}

#[test]
fn local_flying_speed_adjusts_without_counting_server_packet() {
    let mut store = WorldStore::new();

    assert!(!store.adjust_local_flying_speed(0.005));

    store.apply_player_abilities(ProtocolPlayerAbilities {
        invulnerable: false,
        flying: true,
        can_fly: true,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });

    assert!(store.adjust_local_flying_speed(0.005));
    assert_eq!(store.local_player().abilities.unwrap().flying_speed, 0.055);
    assert!(store.adjust_local_flying_speed(1.0));
    assert_eq!(store.local_player().abilities.unwrap().flying_speed, 0.2);
    assert!(!store.adjust_local_flying_speed(1.0));
    assert_eq!(store.counters().player_abilities_packets, 1);
}

#[test]
fn local_player_input_moves_forward_with_minecraft_yaw() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.0, 64.0, 0.0),
        y_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });

    let pose = store
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                ..LocalPlayerInputState::default()
            },
            0.05,
        )
        .unwrap();

    // Airborne `travelInAir` from rest: 0.98 input scale times the 0.02
    // air acceleration moves the entity, then friction (1.0 * 0.91)
    // carries into the delta while gravity builds -0.08 * 0.98.
    let air_step = 0.98 * 0.02;
    assert_f64_near(pose.position.x, 0.0, 0.000001);
    assert_f64_near(pose.position.y, 64.0, 0.000001);
    assert_f64_near(pose.position.z, air_step, 0.000001);
    assert_f64_near(pose.delta_movement.z, air_step * 0.91, 0.000001);
    assert_f64_near(pose.delta_movement.y, -0.08 * 0.98, 0.000001);
    assert!(!pose.on_ground);
    assert_eq!(store.local_player_pose(), Some(pose));
}

#[test]
fn local_player_input_rotates_and_clamps_pitch() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.0, 64.0, 0.0),
        ..LocalPlayerPoseState::default()
    });

    let pose = store
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                mouse_delta_x: 100.0,
                mouse_delta_y: 1000.0,
                ..LocalPlayerInputState::default()
            },
            0.0,
        )
        .unwrap();

    assert_eq!(pose.y_rot, 15.000001);
    assert_eq!(pose.x_rot, 90.0);
}

#[test]
fn local_player_tick_retains_partial_camera_pose_endpoints() {
    let mut store = WorldStore::new();
    let initial = LocalPlayerPoseState {
        position: vec3(0.0, 64.0, 0.0),
        y_rot: 170.0,
        x_rot: -10.0,
        ..LocalPlayerPoseState::default()
    };
    store.set_local_player_pose(initial);

    let current = store
        .advance_local_player_input(
            LocalPlayerInputState {
                focused: true,
                forward: true,
                mouse_delta_x: 200.0,
                mouse_delta_y: 100.0,
                ..LocalPlayerInputState::default()
            },
            0.05,
        )
        .unwrap();
    assert_eq!(store.local_player_old_pose, Some(initial));

    let old = store.local_player_pose_at_partial_tick(0.0).unwrap();
    let mid = store.local_player_pose_at_partial_tick(0.5).unwrap();
    let end = store.local_player_pose_at_partial_tick(1.0).unwrap();
    assert_eq!(old.position, initial.position);
    assert_eq!(end.position, current.position);
    assert_f64_near(
        mid.position.z,
        (initial.position.z + current.position.z) * 0.5,
        0.000001,
    );
    assert_eq!(old.y_rot, 170.0);
    assert_eq!(mid.y_rot, 185.0);
    assert_eq!(end.y_rot, current.y_rot);
    assert_f64_near(f64::from(mid.x_rot), -2.5, 0.000001);
}

#[test]
fn local_player_look_input_rotates_without_moving_position() {
    let mut store = WorldStore::new();
    let initial = LocalPlayerPoseState {
        position: vec3(3.0, 64.0, -2.0),
        delta_movement: vec3(0.2, 0.0, -0.1),
        y_rot: 170.0,
        x_rot: 10.0,
        on_ground: true,
        ..LocalPlayerPoseState::default()
    };
    store.set_local_player_pose(initial);

    let pose = store
        .advance_local_player_look_input(LocalPlayerInputState {
            focused: true,
            mouse_delta_x: 200.0,
            mouse_delta_y: -1000.0,
            ..LocalPlayerInputState::default()
        })
        .unwrap();

    assert_eq!(pose.position, initial.position);
    assert_eq!(pose.delta_movement, initial.delta_movement);
    assert_eq!(pose.y_rot, -160.0);
    assert_eq!(pose.x_rot, -90.0);
    assert_eq!(store.local_player_pose(), Some(pose));
}

#[test]
fn local_player_input_is_ignored_without_pose() {
    let mut store = WorldStore::new();

    assert_eq!(
        store.advance_local_player_input(LocalPlayerInputState::default(), 1.0),
        None
    );
}

#[test]
fn local_interaction_state_tracks_prediction_destroy_and_use_item() {
    let mut store = WorldStore::new();

    let pos = BlockPos { x: 4, y: 70, z: -6 };
    store.set_local_destroying_block(pos);
    store.local_player.interaction.destroying_block_progress = 123;
    store.local_player.interaction.destroying_block_stage = Some(1);
    assert_eq!(store.local_player().interaction.destroying_block, Some(pos));
    assert_eq!(store.local_player().interaction.destroying_block_face, None);
    assert_eq!(
        store.take_local_destroying_block_hit(),
        Some((pos, ProtocolDirection::Down))
    );
    assert_eq!(
        store.local_player().interaction.destroying_block_stage,
        None
    );
    assert_eq!(store.take_local_destroying_block_hit(), None);

    store.set_local_destroying_block_hit(pos, ProtocolDirection::North);
    store.local_player.interaction.destroying_block_progress = 456;
    store.local_player.interaction.destroying_block_stage = Some(4);
    assert_eq!(store.local_player().interaction.destroying_block, Some(pos));
    assert_eq!(
        store.local_player().interaction.destroying_block_face,
        Some(ProtocolDirection::North)
    );
    assert_eq!(store.take_local_destroying_block(), Some(pos));
    assert_eq!(store.take_local_destroying_block(), None);
    assert_eq!(store.local_player().interaction.destroying_block_face, None);
    assert_eq!(
        store.local_player().interaction.destroying_block_stage,
        None
    );

    store.set_local_using_item(true);
    assert_eq!(store.local_player().interaction.using_item_ticks, 0);
    store.advance_local_using_item_ticks(3);
    assert_eq!(store.local_player().interaction.using_item_ticks, 3);
    assert!(store.take_local_using_item());
    assert_eq!(store.local_player().interaction.using_item_ticks, 0);
    assert!(!store.take_local_using_item());
}

#[test]
fn local_interaction_state_survives_world_store_json_round_trip() {
    let mut store = WorldStore::new();
    store.set_local_destroying_block(BlockPos { x: 1, y: 2, z: 3 });
    store.set_local_using_item(true);
    assert_eq!(store.next_local_prediction_sequence(), 1);

    let restored: WorldStore = serde_json::from_value(serde_json::to_value(&store).unwrap())
        .expect("world store local interaction should deserialize");

    assert_eq!(
        restored.local_player().interaction,
        LocalPlayerInteractionState {
            destroying_block: Some(BlockPos { x: 1, y: 2, z: 3 }),
            destroying_block_face: None,
            destroying_item_signature: Some(LocalPlayerDestroyItemSignature::from_item_stack(
                &ProtocolItemStackSummary::empty(),
            )),
            destroying_block_progress: 0,
            destroying_block_stage: None,
            destroying_block_ticks: 0,
            destroy_delay_ticks: 0,
            using_item: true,
            using_item_hand: Some(InteractionHand::MainHand),
            using_item_ticks: 0,
            auto_spin_attack: None,
            attack: LocalPlayerAttackState::default(),
        }
    );
    assert_eq!(restored.local_prediction_sequence(), 1);
}

#[test]
fn local_attack_ticks_follow_main_hand_match_and_item_identity_changes() {
    let mut store = local_attack_player_world();
    store.advance_local_player_attack_strength_ticks(3);
    assert_eq!(store.local_player_attack_strength_ticks(), 3);
    assert_eq!(store.local_player_item_swap_ticks(), 3);
    assert_eq!(store.local_player_item_swap_scale(1.0), 0.8);
    assert_eq!(store.local_player_item_swap_scale_ticks_ago(2, 1.0), 0.4);

    store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: local_attack_item(42, 1),
    });
    store.advance_local_player_attack_strength_ticks(3);
    assert_eq!(store.local_player_attack_strength_ticks(), 2);
    assert_eq!(store.local_player_item_swap_ticks(), 2);

    store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: local_attack_item(42, 2),
    });
    store.advance_local_player_attack_strength_ticks(2);
    assert_eq!(store.local_player_attack_strength_ticks(), 4);
    assert_eq!(store.local_player_item_swap_ticks(), 4);
    assert_eq!(
        store
            .local_player()
            .interaction
            .attack
            .last_main_hand_item
            .as_ref()
            .map(|item| item.count),
        Some(2)
    );

    let mut patched = local_attack_item(42, 2);
    patched.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID];
    patched.component_patch.minimum_attack_charge_bits = Some(0.5_f32.to_bits());
    store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: patched,
    });
    store.advance_local_player_attack_strength_ticks(1);
    assert_eq!(store.local_player_attack_strength_ticks(), 5);

    store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: local_attack_item(43, 1),
    });
    store.advance_local_player_attack_strength_ticks(4);
    assert_eq!(store.local_player_attack_strength_ticks(), 3);
    assert_eq!(store.local_player_item_swap_ticks(), 3);

    store.reset_local_player_attack_strength();
    assert_eq!(store.local_player_attack_strength_ticks(), 0);
    assert_eq!(store.local_player_item_swap_ticks(), 0);
}

#[test]
fn local_attack_charge_uses_actual_attack_speed_and_inclusive_boundary() {
    let mut store = local_attack_player_world();
    store.items.commit_item_component_defaults(BTreeMap::from([(
        42,
        DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID],
            minimum_attack_charge_bits: Some(1.0_f32.to_bits()),
            ..Default::default()
        },
    )]));
    store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: local_attack_item(42, 1),
    });
    store.advance_local_player_attack_strength_ticks(1);
    assert_eq!(
        store.local_player_attribute_value(VANILLA_ATTRIBUTE_ATTACK_SPEED_ID),
        Some(4.0)
    );
    store.advance_local_player_attack_strength_ticks(4);
    assert!(store.local_player_cannot_attack_with_selected_item());
    store.advance_local_player_attack_strength_ticks(1);
    assert!(!store.local_player_cannot_attack_with_selected_item());

    let mut patched = local_attack_item(42, 1);
    patched.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID];
    patched.component_patch.minimum_attack_charge_bits = Some(0.5_f32.to_bits());
    store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: patched,
    });
    assert!(
        store.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
            entity_id: 7,
            attributes: vec![bbb_protocol::packets::AttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
                base: 2.0,
                modifiers: Vec::new(),
            }],
        })
    );
    store.reset_local_player_attack_strength();
    store.advance_local_player_attack_strength_ticks(4);
    assert!(store.local_player_cannot_attack_with_selected_item());
    store.advance_local_player_attack_strength_ticks(1);
    assert!(!store.local_player_cannot_attack_with_selected_item());

    assert!(
        store.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
            entity_id: 7,
            attributes: vec![bbb_protocol::packets::AttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
                base: 0.0,
                modifiers: Vec::new(),
            }],
        })
    );
    assert!(store.local_player_cannot_attack_with_selected_item());
}

#[test]
fn local_attack_state_serializes_and_resets_with_the_client_level() {
    let mut store = local_attack_player_world();
    store.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
        slot: 0,
        item: local_attack_item(42, 3),
    });
    store.advance_local_player_attack_strength_ticks(6);
    let restored: WorldStore = serde_json::from_value(serde_json::to_value(&store).unwrap())
        .expect("local attack state should deserialize");
    assert_eq!(
        restored.local_player().interaction.attack,
        store.local_player().interaction.attack
    );

    store.clear_client_level();
    assert_eq!(
        store.local_player().interaction.attack,
        LocalPlayerAttackState::default()
    );
}

#[test]
fn camera_updates_only_for_local_or_known_entities() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(9);

    assert!(!store.apply_set_camera(ProtocolSetCamera { camera_id: 123 }));
    assert_eq!(store.local_player().camera, CameraState::default());

    assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 9 }));
    assert_eq!(
        store.local_player().camera,
        CameraState {
            entity_id: Some(9),
            follows_player: true,
            entity_known: true,
        }
    );

    store.apply_add_entity(protocol_add_entity(123));
    assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 123 }));
    assert_eq!(
        store.local_player().camera,
        CameraState {
            entity_id: Some(123),
            follows_player: false,
            entity_known: true,
        }
    );
    assert_eq!(store.counters().set_camera_packets, 3);
    assert_eq!(store.counters().set_camera_updates_applied, 2);
    assert_eq!(store.counters().set_camera_updates_ignored, 1);
}

#[test]
fn clear_client_level_resets_the_camera_to_the_local_player() {
    // Vanilla `GameRenderer.resetData` runs `mainCamera.reset()` on
    // clearLevel, so a SetCamera retarget never survives the teardown.
    let mut store = WorldStore::new();
    store.local_player_id = Some(9);
    store.apply_add_entity(protocol_add_entity(123));
    assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 123 }));
    assert!(!store.local_player().camera.follows_player);

    store.clear_client_level();
    assert_eq!(store.local_player().camera, CameraState::default());
}

#[test]
fn the_hud_camera_player_only_resolves_for_the_local_player_or_a_player_entity() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(9);
    assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 9 }));
    assert_eq!(store.hud_camera_player_id(), Some(9));

    store.apply_add_entity(protocol_add_entity(123));
    assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 123 }));
    assert_eq!(store.hud_camera_player_id(), None);

    store.apply_add_entity(ProtocolAddEntity {
        entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        ..protocol_add_entity(55)
    });
    assert!(store.apply_set_camera(ProtocolSetCamera { camera_id: 55 }));
    assert_eq!(store.hud_camera_player_id(), Some(55));
}

#[test]
fn local_player_position_and_rotation_update_canonical_pose() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(LocalPlayerPoseState {
        sneaking: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = store
        .apply_player_position(ProtocolPlayerPositionUpdate {
            id: 7,
            position: vec3(10.0, 64.0, -5.0),
            delta_movement: vec3(0.125, 0.0, 0.0),
            y_rot: 90.0,
            x_rot: 15.0,
            relatives_mask: 0,
        })
        .unwrap();
    assert_eq!(pose.position, vec3(10.0, 64.0, -5.0));
    assert_eq!(pose.delta_movement, vec3(0.125, 0.0, 0.0));
    assert!(pose.sneaking);
    assert_eq!(pose.last_teleport_id, 7);
    assert_eq!(store.local_player_pose_at_partial_tick(0.5), Some(pose));

    store.local_player_old_pose.as_mut().unwrap().position.x = 8.0;
    let pose = store
        .apply_player_position(ProtocolPlayerPositionUpdate {
            id: 8,
            position: vec3(1.5, -2.0, 7.0),
            delta_movement: vec3(0.25, 0.5, 0.75),
            y_rot: 20.0,
            x_rot: -120.0,
            relatives_mask: PLAYER_RELATIVE_X
                | PLAYER_RELATIVE_Y_ROT
                | PLAYER_RELATIVE_X_ROT
                | PLAYER_RELATIVE_DELTA_X,
        })
        .unwrap();
    assert_eq!(pose.position, vec3(11.5, -2.0, 7.0));
    assert_eq!(pose.delta_movement, vec3(0.375, 0.5, 0.75));
    assert_eq!(pose.y_rot, 110.0);
    assert_eq!(pose.x_rot, -90.0);
    assert!(pose.sneaking);
    assert_eq!(pose.last_teleport_id, 8);
    assert_eq!(
        store
            .local_player_pose_at_partial_tick(0.5)
            .unwrap()
            .position,
        vec3(10.5, -2.0, 7.0)
    );

    let pose = store.apply_player_rotation(ProtocolPlayerRotationUpdate {
        y_rot: -10.0,
        relative_y: true,
        x_rot: 30.0,
        relative_x: false,
    });
    assert_eq!(pose.position, vec3(11.5, -2.0, 7.0));
    assert_eq!(pose.delta_movement, vec3(0.375, 0.5, 0.75));
    assert_eq!(pose.y_rot, 100.0);
    assert_eq!(pose.x_rot, 30.0);
    assert!(pose.sneaking);
    assert_eq!(pose.last_teleport_id, 8);
    assert_eq!(store.local_player_pose(), Some(pose));
    assert_eq!(
        store.local_player_pose_at_partial_tick(0.0).unwrap().y_rot,
        100.0
    );

    let counters = store.counters();
    assert_eq!(counters.player_position_packets, 2);
    assert_eq!(counters.player_rotation_packets, 1);
}

#[test]
fn local_player_position_and_rotation_preserve_swimming_pose() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(LocalPlayerPoseState {
        swimming: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = store
        .apply_player_position(ProtocolPlayerPositionUpdate {
            id: 7,
            position: vec3(10.0, 64.0, -5.0),
            delta_movement: vec3(0.125, 0.0, 0.0),
            y_rot: 90.0,
            x_rot: 15.0,
            relatives_mask: 0,
        })
        .unwrap();
    assert!(!pose.sneaking);
    assert!(pose.swimming);

    let pose = store.apply_player_rotation(ProtocolPlayerRotationUpdate {
        y_rot: -10.0,
        relative_y: true,
        x_rot: 30.0,
        relative_x: false,
    });
    assert!(!pose.sneaking);
    assert!(pose.swimming);
    assert_eq!(store.local_player_pose(), Some(pose));
}

#[test]
fn local_player_position_does_not_move_mounted_player() {
    let mut store = WorldStore::new();
    let initial_pose = LocalPlayerPoseState {
        position: vec3(0.0, 64.0, 0.0),
        delta_movement: vec3(0.0, 0.0, 0.0),
        y_rot: 0.0,
        x_rot: 0.0,
        last_teleport_id: 3,
        ..LocalPlayerPoseState::default()
    };
    store.set_local_player_pose(initial_pose);
    store.local_player_vehicle_id = Some(10);

    assert_eq!(
        store.apply_player_position(ProtocolPlayerPositionUpdate {
            id: 9,
            position: vec3(10.0, 70.0, 10.0),
            delta_movement: vec3(1.0, 1.0, 1.0),
            y_rot: 90.0,
            x_rot: 45.0,
            relatives_mask: 0,
        }),
        None
    );
    assert_eq!(store.local_player_pose(), Some(initial_pose));
    assert_eq!(store.local_prediction_last_teleport_sequence(), Some(0));
    assert_eq!(store.counters().player_position_packets, 1);
}

#[test]
fn local_player_look_at_uses_known_target_entity_anchor() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.0, 64.0, 0.0),
        delta_movement: vec3(0.0, 0.0, 0.0),
        y_rot: 90.0,
        x_rot: 30.0,
        last_teleport_id: 7,
        ..LocalPlayerPoseState::default()
    });
    store.apply_add_entity(protocol_add_entity_at(123, vec3(0.0, 70.0, 10.0)));

    let pose = store
        .apply_player_look_at(ProtocolPlayerLookAt {
            from_anchor: EntityAnchor::Eyes,
            position: vec3(50.0, 50.0, 50.0),
            target: Some(PlayerLookAtTarget {
                entity_id: 123,
                to_anchor: EntityAnchor::Feet,
            }),
        })
        .unwrap();

    let expected_x_rot = -((70.0 - 65.62_f64).atan2(10.0).to_degrees() as f32);
    assert!((pose.y_rot - 0.0).abs() < 0.001);
    assert!((pose.x_rot - expected_x_rot).abs() < 0.001);
    assert_eq!(pose.last_teleport_id, 7);
    assert_eq!(
        store.local_player().last_look_at,
        Some(LocalPlayerLookAtState {
            from_anchor: EntityAnchor::Eyes,
            position: vec3(0.0, 70.0, 10.0),
            target_entity_id: Some(123),
            to_anchor: Some(EntityAnchor::Feet),
        })
    );
    assert_eq!(store.counters().player_look_at_packets, 1);
}

#[test]
fn local_player_look_at_uses_target_entity_specific_eye_height() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.0, 64.0, 0.0),
        ..LocalPlayerPoseState::default()
    });
    store.apply_add_entity(protocol_add_entity_at(123, vec3(0.0, 70.0, 10.0)));
    let target_eye_height = f64::from(
        store
            .probe_entity_camera_pose(123)
            .expect("known target camera pose")
            .eye_height,
    );

    let pose = store
        .apply_player_look_at(ProtocolPlayerLookAt {
            from_anchor: EntityAnchor::Eyes,
            position: vec3(50.0, 50.0, 50.0),
            target: Some(PlayerLookAtTarget {
                entity_id: 123,
                to_anchor: EntityAnchor::Eyes,
            }),
        })
        .unwrap();

    let expected_x_rot = -((70.0 + target_eye_height - 65.62_f64)
        .atan2(10.0)
        .to_degrees() as f32);
    assert!((pose.x_rot - expected_x_rot).abs() < 0.001);
    assert_eq!(
        store.local_player().last_look_at.unwrap().position,
        vec3(0.0, 70.0 + target_eye_height, 10.0)
    );
}

#[test]
fn local_player_look_at_uses_crouching_eye_height_when_sneaking() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.0, 64.0, 0.0),
        sneaking: true,
        ..LocalPlayerPoseState::default()
    });

    let pose = store
        .apply_player_look_at(ProtocolPlayerLookAt {
            from_anchor: EntityAnchor::Eyes,
            position: vec3(0.0, 70.0, 10.0),
            target: None,
        })
        .unwrap();

    let expected_x_rot = -((70.0 - (64.0 + 1.27_f64)).atan2(10.0).to_degrees() as f32);
    assert!((pose.y_rot - 0.0).abs() < 0.001);
    assert!((pose.x_rot - expected_x_rot).abs() < 0.001);
    assert!(pose.sneaking);
}

fn protocol_add_entity(id: i32) -> ProtocolAddEntity {
    protocol_add_entity_at(
        id,
        ProtocolVec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
    )
}

fn local_attack_player_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.local_player_id = Some(7);
    world.apply_add_entity(ProtocolAddEntity {
        id: 7,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: vec3(0.5, 64.0, 0.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world
}

fn local_attack_item(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn protocol_add_entity_at(id: i32, position: ProtocolVec3d) -> ProtocolAddEntity {
    ProtocolAddEntity {
        id,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        position,
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: -10.0,
        y_rot: 20.0,
        y_head_rot: 30.0,
        data: 99,
    }
}

fn vec3(x: f64, y: f64, z: f64) -> ProtocolVec3d {
    ProtocolVec3d { x, y, z }
}

fn assert_f64_near(actual: f64, expected: f64, epsilon: f64) {
    assert!(
        (actual - expected).abs() <= epsilon,
        "expected {actual} to be within {epsilon} of {expected}"
    );
}

fn player_world_with_armor(
    modifiers: Vec<bbb_protocol::packets::AttributeModifier>,
    base: f64,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.local_player_id = Some(7);
    world.apply_add_entity(ProtocolAddEntity {
        id: 7,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: vec3(0.5, 1.0, 0.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(
        world.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
            entity_id: 7,
            attributes: vec![bbb_protocol::packets::AttributeSnapshot {
                // ARMOR is registration index 0 in BuiltInRegistries.ATTRIBUTE.
                attribute_id: VANILLA_ATTRIBUTE_ARMOR_ID,
                base,
                modifiers,
            }],
        })
    );
    world
}

#[test]
fn local_player_armor_value_defaults_to_zero_without_attribute() {
    // No local player at all.
    let world = WorldStore::new();
    assert_eq!(world.local_player_armor_value(), 0);

    // A local player entity with no synced ARMOR attribute still reads 0
    // (vanilla `RangedAttribute` default), not a panic.
    let mut world = WorldStore::new();
    world.local_player_id = Some(7);
    world.apply_add_entity(ProtocolAddEntity {
        id: 7,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: vec3(0.5, 1.0, 0.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert_eq!(world.local_player_armor_value(), 0);
}

#[test]
fn local_player_armor_value_floors_the_armor_attribute() {
    // Vanilla `getArmorValue()` = `Mth.floor(...)`, so 7.5 -> 7.
    let world = player_world_with_armor(Vec::new(), 7.5);
    assert_eq!(world.local_player_armor_value(), 7);

    // Exact integers pass through unchanged.
    let world = player_world_with_armor(Vec::new(), 20.0);
    assert_eq!(world.local_player_armor_value(), 20);
}

#[test]
fn local_player_armor_value_folds_modifiers_through_calculate_value() {
    // `AttributeInstance.calculateValue`: base add (op 0), then multiply_base
    // (op 1), then multiply_total (op 2).
    //   base    = 4.0 + 2.0            = 6.0
    //   result  = 6.0 + 6.0 * 0.5      = 9.0
    //   result  = 9.0 * (1.0 + 0.1)    = 9.9
    //   floor(9.9)                     = 9
    let world = player_world_with_armor(
        vec![
            bbb_protocol::packets::AttributeModifier {
                id: "test:add".to_string(),
                amount: 2.0,
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            },
            bbb_protocol::packets::AttributeModifier {
                id: "test:multiply_base".to_string(),
                amount: 0.5,
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
            },
            bbb_protocol::packets::AttributeModifier {
                id: "test:multiply_total".to_string(),
                amount: 0.1,
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
            },
        ],
        4.0,
    );
    assert_eq!(world.local_player_armor_value(), 9);
    assert_eq!(world.local_player_attribute_base_value(0), Some(4.0));
    assert_eq!(
        world.local_player_attribute_base_value(
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_SPEED_ID
        ),
        Some(4.0),
    );
    assert_eq!(
        world.local_player_attribute_base_value(
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID
        ),
        None,
    );
}

#[test]
fn local_player_fall_flying_gate_composes_vanilla_state_and_glider_rules() {
    fn world_with_glider() -> WorldStore {
        let mut world = spawned_player_world();
        world.set_local_player_pose(LocalPlayerPoseState {
            position: vec3(0.5, 1.0, 0.5),
            on_ground: false,
            ..LocalPlayerPoseState::default()
        });
        world.set_default_item_component_codec_values(BTreeMap::from([(
            42,
            BTreeMap::from([(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
                serde_json::json!({}),
            )]),
        )]));
        world.set_test_equippable_defaults([(
            42,
            crate::item_profiles::test_equippable(
                bbb_protocol::equipment_slots::EquipmentSlot::Chest,
            ),
        )]);
        world.apply_set_player_inventory(bbb_protocol::packets::SetPlayerInventory {
            slot: bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
            item: ProtocolItemStackSummary {
                item_id: Some(42),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        });
        world
    }

    assert!(world_with_glider().local_player_can_start_fall_flying());

    let mut grounded = world_with_glider();
    let mut pose = grounded.local_player_pose().unwrap();
    pose.on_ground = true;
    grounded.set_local_player_pose(pose);
    assert!(!grounded.local_player_can_start_fall_flying());

    let mut creative_flying = world_with_glider();
    creative_flying.apply_player_abilities(ProtocolPlayerAbilities {
        invulnerable: true,
        flying: true,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    assert!(!creative_flying.local_player_can_start_fall_flying());

    let mut levitating = world_with_glider();
    assert!(
        levitating.apply_update_mob_effect(bbb_protocol::packets::UpdateMobEffect {
            entity_id: 7,
            effect_id: VANILLA_MOB_EFFECT_LEVITATION_ID,
            amplifier: 0,
            duration_ticks: 200,
            flags: bbb_protocol::packets::MobEffectFlags::default(),
        })
    );
    assert!(!levitating.local_player_can_start_fall_flying());

    let mut already_flying = world_with_glider();
    let accessors = entity_metadata_accessors();
    assert!(set_entity_data(
        &mut already_flying,
        7,
        accessors.shared_flags_data_id,
        bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        bbb_protocol::packets::EntityDataValueKind::Byte(accessors.fall_flying_mask),
    ));
    assert!(!already_flying.local_player_can_start_fall_flying());

    let mut passenger = world_with_glider();
    passenger.apply_add_entity(ProtocolAddEntity {
        id: 8,
        uuid: Uuid::from_u128(0x87654321876543218765432187654321),
        entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID,
        position: vec3(0.5, 1.0, 0.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(
        passenger.apply_set_passengers(bbb_protocol::packets::SetPassengers {
            vehicle_id: 8,
            passenger_ids: vec![7],
        })
    );
    assert!(!passenger.local_player_can_start_fall_flying());
}

fn spawned_player_world() -> WorldStore {
    let mut world = WorldStore::new();
    world.local_player_id = Some(7);
    world.apply_add_entity(ProtocolAddEntity {
        id: 7,
        uuid: Uuid::from_u128(0x12345678123456781234567812345678),
        entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: vec3(0.5, 1.0, 0.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world
}

fn set_entity_data(
    world: &mut WorldStore,
    entity_id: i32,
    data_id: u8,
    serializer_id: i32,
    value: bbb_protocol::packets::EntityDataValueKind,
) -> bool {
    world.apply_set_entity_data(bbb_protocol::packets::SetEntityData {
        id: entity_id,
        values: vec![bbb_protocol::packets::EntityDataValue {
            data_id,
            serializer_id,
            value,
        }],
    })
}

fn entity_metadata_accessors() -> crate::VanillaEntityMetadataAccessors {
    crate::vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors")
}

#[test]
fn local_player_air_supply_reads_the_synced_air_metadata() {
    // Vanilla `Entity.DATA_AIR_SUPPLY_ID` is a generated INT accessor with the virtual
    // define-time default `getMaxAirSupply()` = 300 (Entity.java:312,2725-2727).
    let accessors = entity_metadata_accessors();

    // No local player at all: the full (hidden-row) supply.
    let world = WorldStore::new();
    assert_eq!(world.local_player_air_supply(), 300);
    assert_eq!(world.local_player_max_air_supply(), 300);

    // A spawned player without a synced value still reads the 300 default.
    let mut world = spawned_player_world();
    assert_eq!(world.local_player_air_supply(), 300);

    assert!(set_entity_data(
        &mut world,
        7,
        accessors.air_supply_data_id,
        bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        bbb_protocol::packets::EntityDataValueKind::Int(150),
    ));
    assert_eq!(world.local_player_air_supply(), 150);
    assert_eq!(world.local_player_max_air_supply(), 300);
}

#[test]
fn local_player_absorption_reads_the_synced_player_metadata() {
    // Vanilla `Player.DATA_PLAYER_ABSORPTION_ID` is a generated Player-only FLOAT accessor
    // with define-time default 0.0F (Player.java:134,224).
    let (absorption_data_id, absorption_default) =
        crate::vanilla_entity_metadata_player_absorption_accessor_for_entity_type(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        )
        .expect("generated Player absorption accessor");
    assert_eq!(absorption_default, 0.0);

    let world = WorldStore::new();
    assert_eq!(world.local_player_absorption(), 0.0);

    let mut world = spawned_player_world();
    assert_eq!(world.local_player_absorption(), 0.0);

    assert!(set_entity_data(
        &mut world,
        7,
        absorption_data_id,
        bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
        bbb_protocol::packets::EntityDataValueKind::Float(8.0),
    ));
    assert_eq!(world.local_player_absorption(), 8.0);

    // A non-Player using the same numeric slot must not project Player absorption.
    let mut world = WorldStore::new();
    world.local_player_id = Some(8);
    world.apply_add_entity(ProtocolAddEntity {
        id: 8,
        uuid: Uuid::from_u128(8),
        entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        position: vec3(0.5, 1.0, 0.5),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(!set_entity_data(
        &mut world,
        8,
        absorption_data_id,
        bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
        bbb_protocol::packets::EntityDataValueKind::Float(9.0),
    ));
    assert_eq!(world.local_player_absorption(), 0.0);
}

#[test]
fn local_player_vehicle_health_projects_living_vehicles_only() {
    // Vanilla `LivingEntity.DATA_HEALTH_ID` is a generated inherited FLOAT accessor with
    // define-time default 1.0F (LivingEntity.java:179,314).
    let living_accessors = crate::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID,
    )
    .expect("generated LivingEntity metadata accessors");
    assert_eq!(living_accessors.health_default, 1.0);
    // `minecraft:max_health` uses the generated `Attributes.java` registration id.

    let add_vehicle = |world: &mut WorldStore, id: i32, entity_type_id: i32| {
        world.apply_add_entity(ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(0x2222_0000 + id as u128),
            entity_type_id,
            position: vec3(0.5, 1.0, 0.5),
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
    };
    let mount = |world: &mut WorldStore, vehicle_id: i32| {
        assert!(
            world.apply_set_passengers(bbb_protocol::packets::SetPassengers {
                vehicle_id,
                passenger_ids: vec![7],
            })
        );
    };

    // No vehicle at all.
    let mut world = spawned_player_world();
    assert_eq!(world.local_player_vehicle_health(), None);

    // A living vehicle (horse) with synced health and MAX_HEALTH: both
    // project (vanilla `getPlayerVehicleWithHealth` + `getMaxHealth`).
    add_vehicle(
        &mut world,
        30,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HORSE_ID,
    );
    mount(&mut world, 30);
    assert!(set_entity_data(
        &mut world,
        30,
        living_accessors.health_data_id,
        bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
        bbb_protocol::packets::EntityDataValueKind::Float(7.0),
    ));
    assert!(
        world.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
            entity_id: 30,
            attributes: vec![bbb_protocol::packets::AttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                base: 15.0,
                modifiers: Vec::new(),
            }],
        })
    );
    assert_eq!(
        world.local_player_vehicle_health(),
        Some(crate::LocalPlayerVehicleHealth {
            health: 7.0,
            max_health: 15.0,
        })
    );

    // Without a synced MAX_HEALTH attribute the max falls back to Pig's
    // `DefaultAttributes` family override 10.0, and without a synced
    // health the metadata default 1.0F applies.
    let mut world = spawned_player_world();
    add_vehicle(
        &mut world,
        31,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID,
    );
    mount(&mut world, 31);
    assert_eq!(
        world.local_player_vehicle_health(),
        Some(crate::LocalPlayerVehicleHealth {
            health: 1.0,
            max_health: 10.0,
        })
    );

    // A non-living vehicle (boat) is vanilla `showVehicleHealth() == false`
    // (`Entity` base: `this instanceof LivingEntity`, Entity.java:2349-2351).
    let mut world = spawned_player_world();
    add_vehicle(
        &mut world,
        32,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    );
    mount(&mut world, 32);
    assert_eq!(world.local_player_vehicle_health(), None);
}

#[test]
fn local_player_max_health_reads_the_synced_attribute() {
    // `minecraft:max_health` uses the generated `Attributes.java` registration id; the
    // `RangedAttribute` default is 20.0 (Attributes.java:58-60).

    // No local player / no synced attribute: the RangedAttribute default.
    assert_eq!(WorldStore::new().local_player_max_health(), 20.0);
    let mut world = spawned_player_world();
    assert_eq!(world.local_player_max_health(), 20.0);

    // A Health Boost / attribute bump folds through `calculateValue`.
    assert!(
        world.apply_update_attributes(bbb_protocol::packets::UpdateAttributes {
            entity_id: 7,
            attributes: vec![bbb_protocol::packets::AttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
                base: 30.0,
                modifiers: Vec::new(),
            }],
        })
    );
    assert_eq!(world.local_player_max_health(), 30.0);
}

#[test]
fn local_player_is_fully_frozen_tracks_the_freeze_threshold() {
    // Vanilla `isFullyFrozen()` = `getTicksFrozen() >= 140`.
    // `DATA_TICKS_FROZEN_ID` is a generated base Entity INT accessor.
    let accessors = entity_metadata_accessors();
    assert_eq!(accessors.ticks_frozen_default, 0);

    assert!(!WorldStore::new().local_player_is_fully_frozen());
    let mut world = spawned_player_world();
    assert!(!world.local_player_is_fully_frozen());

    // One tick short of the threshold: not yet fully frozen.
    assert!(set_entity_data(
        &mut world,
        7,
        accessors.ticks_frozen_data_id,
        bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        bbb_protocol::packets::EntityDataValueKind::Int(139),
    ));
    assert!(!world.local_player_is_fully_frozen());

    // At the 140-tick threshold: fully frozen.
    assert!(set_entity_data(
        &mut world,
        7,
        accessors.ticks_frozen_data_id,
        bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        bbb_protocol::packets::EntityDataValueKind::Int(140),
    ));
    assert!(world.local_player_is_fully_frozen());
}

#[test]
fn set_local_player_look_angles_updates_interpolated_pose_immediately() {
    let mut store = WorldStore::new();
    store.set_local_player_pose(LocalPlayerPoseState {
        position: vec3(0.5, 64.0, 0.5),
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });

    let pose = store.set_local_player_look_angles(135.0, -25.0).unwrap();
    assert_eq!(pose.y_rot, 135.0);
    assert_eq!(pose.x_rot, -25.0);
    // Mid-tick clicks read `local_player_pose_at_partial_tick`, so old pose
    // rotation must match immediately without drifting back toward the old view.
    let partial = store.local_player_pose_at_partial_tick(0.0).unwrap();
    assert_eq!(partial.y_rot, 135.0);
    assert_eq!(partial.x_rot, -25.0);
}
