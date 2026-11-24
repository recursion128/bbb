use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID;
use bbb_protocol::menu_types::VANILLA_MENU_TYPE_GENERIC_9X3_ID;
use bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_MINING_FATIGUE_ID;
use bbb_protocol::packets::ClockUpdate as ProtocolClockUpdate;
use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, AdvancementCriterionProgressSummary, AdvancementDisplaySummary,
    AdvancementFrameType, AdvancementIconSummary, AdvancementProgressSummary, AdvancementSummary,
    AttributeModifier as ProtocolAttributeModifier, AttributeSnapshot as ProtocolAttributeSnapshot,
    BlockDestruction as ProtocolBlockDestruction, BlockEvent as ProtocolBlockEvent,
    BlockPos as ProtocolBlockPos, EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
    EntityEvent as ProtocolEntityEvent, EquipmentSlot,
    EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate, GlobalPos as ProtocolGlobalPos,
    ItemStackSummary as ProtocolItemStackSummary, LevelEvent as ProtocolLevelEvent,
    MapItemData as ProtocolMapItemData, MobEffectFlags, OpenScreen as ProtocolOpenScreen,
    PlayerExperience as ProtocolPlayerExperience, PlayerHealth as ProtocolPlayerHealth,
    RegistryData, RegistryDataEntry, RegistryTags, SetCamera as ProtocolSetCamera,
    SetEntityData as ProtocolSetEntityData, TagNetworkPayload, Transfer as ProtocolTransfer,
    UpdateAdvancements, UpdateAttributes as ProtocolUpdateAttributes,
    UpdateMobEffect as ProtocolUpdateMobEffect, UpdateTags, Vec3d as ProtocolVec3d,
};
use uuid::Uuid;

use crate::entities::VANILLA_ENTITY_TYPE_PLAYER_ID;
use crate::{
    BlockPos, CameraState, ChunkColumn, ChunkPos, ChunkState, LightData, LocalPlayerPoseState,
};

mod fixtures;
mod gameplay_state;
mod login_and_respawn;
mod time_and_weather;
use fixtures::*;

#[test]
fn play_login_updates_world_dimension_and_level_info() {
    let mut store = WorldStore::new();
    store.chunks.push(stale_chunk().into());

    store.apply_login(&ProtocolPlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec![
            "minecraft:overworld".to_string(),
            "minecraft:the_nether".to_string(),
            "minecraft:the_end".to_string(),
            "minecraft:overworld".to_string(),
        ],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: true,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: ProtocolSpawnInfo {
            dimension_type_id: 1,
            dimension: "minecraft:the_nether".to_string(),
            seed: 12345,
            game_type: 1,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: Some(ProtocolGlobalPos {
                dimension: "minecraft:overworld".to_string(),
                pos: ProtocolBlockPos { x: 8, y: 64, z: -3 },
            }),
            portal_cooldown: 7,
            sea_level: 32,
        },
        enforces_secure_chat: true,
    });

    assert_eq!(
        store.dimension(),
        WorldDimension {
            min_y: 0,
            height: 256,
        }
    );
    assert_eq!(store.chunk_count(), 0);
    assert_eq!(store.chunk_cache_center(), None);
    assert_eq!(store.chunk_cache_radius(), Some(8));
    assert_eq!(store.local_player().simulation_distance, Some(6));
    assert_eq!(store.counters().chunk_cache_radius_updates_received, 0);
    assert_eq!(store.counters().simulation_distance_packets, 0);
    assert_eq!(store.counters().play_logins_received, 1);
    assert_eq!(store.client_level_generation(), 1);
    assert_eq!(
        store.available_levels(),
        &BTreeSet::from([
            "minecraft:overworld".to_string(),
            "minecraft:the_nether".to_string(),
            "minecraft:the_end".to_string(),
        ])
    );
    assert_eq!(store.server_enforces_secure_chat(), Some(true));
    assert_eq!(store.local_player().portal_cooldown, 7);
    for x in [11, 12] {
        let mut chunk = stale_chunk();
        chunk.pos = ChunkPos { x, z: 0 };
        store.chunks.push(chunk.into());
    }
    store.apply_set_chunk_cache_center(bbb_protocol::packets::SetChunkCacheCenter {
        chunk_x: 0,
        chunk_z: 0,
    });
    assert_eq!(store.chunk_positions(), vec![ChunkPos { x: 11, z: 0 }]);
    let level = store.level_info().unwrap();
    assert_eq!(level.biome_zoom_seed, 12345);
    assert_eq!(level.dimension, "minecraft:the_nether");
    assert_eq!(level.dimension_type_id, 1);
    assert_eq!(
        level.dimension_type_name.as_deref(),
        Some("minecraft:the_nether")
    );
    assert_eq!(level.cardinal_lighting(), WorldCardinalLighting::Nether);
    assert_eq!(
        level.last_death_location,
        Some(WorldGlobalPos {
            dimension: "minecraft:overworld".to_string(),
            pos: BlockPos { x: 8, y: 64, z: -3 },
        })
    );
    assert_eq!(level.sea_level, 32);
    assert_eq!(
        store.gameplay(),
        &WorldGameplayState {
            game_type: 1,
            game_type_name: "creative".to_string(),
            previous_game_type: None,
            previous_game_type_name: None,
            show_death_screen: true,
            do_limited_crafting: false,
            reduced_debug_info: true,
            hardcore: false,
        }
    );
    assert!(store.local_player_has_reduced_debug_info());
    store.advance_client_time(3);
    assert_eq!(store.local_player().portal_cooldown, 4);
}

#[test]
fn respawn_retains_toast_manager_handoff_and_advancement_tree() {
    let mut store = WorldStore::new();
    store.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![AdvancementSummary {
            id: "test:root".to_string(),
            parent: None,
            display: Some(AdvancementDisplaySummary {
                title: bbb_protocol::RichTextComponent::plain("Root"),
                description: bbb_protocol::RichTextComponent::plain(""),
                icon: AdvancementIconSummary {
                    item_id: 1,
                    count: 1,
                    component_patch: Default::default(),
                },
                frame_type: AdvancementFrameType::Task,
                show_toast: true,
                hidden: false,
                background: None,
                x: 0.0,
                y: 0.0,
            }),
            requirements: vec![vec!["done".into()]],
            sends_telemetry_event: false,
        }],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    store.apply_update_advancements(UpdateAdvancements {
        reset: false,
        added: Vec::new(),
        removed: Vec::new(),
        progress: vec![AdvancementProgressSummary {
            id: "test:root".to_string(),
            criteria: vec![AdvancementCriterionProgressSummary {
                name: "done".into(),
                obtained_epoch_millis: Some(1),
            }],
        }],
        show_advancements: true,
    });
    assert_eq!(store.client_toast_events().len(), 1);

    store.apply_respawn(&respawn_packet(0));

    assert_eq!(store.client_toast_events().len(), 1);
    assert_eq!(store.client_toasts().next_sequence, 1);
    assert!(store
        .client_advancements()
        .advancements
        .contains_key("test:root"));
}

#[test]
fn play_login_retains_toast_manager_but_level_clear_drops_it() {
    let mut store = WorldStore::new();
    store.apply_low_disk_space_warning();
    let manager_generation = store.client_toasts().generation;

    store.apply_login(&login_packet(0, "minecraft:overworld"));

    assert_eq!(store.client_toasts().generation, manager_generation);
    assert!(matches!(
        store.client_toast_events()[0].system(),
        Some(crate::ClientSystemToastEvent::LowDiskSpace)
    ));

    store.clear_client_level();
    assert!(store.client_toast_events().is_empty());
    assert_eq!(
        store.client_toasts().generation,
        manager_generation.saturating_add(1)
    );
}

#[test]
fn login_rebuilds_local_player_camera_and_game_mode_abilities() {
    let mut store = WorldStore::new();
    store.local_player_id = Some(7);
    store.chunks.push(stale_chunk().into());
    store.apply_add_entity(protocol_add_entity(99));
    store.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: true,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.2,
        walking_speed: 0.3,
    });
    store.apply_player_health(ProtocolPlayerHealth {
        health: 4.0,
        food: 7,
        saturation: 0.5,
    });
    store.apply_player_experience(ProtocolPlayerExperience {
        progress: 0.25,
        level: 3,
        total: 40,
    });
    assert!(store.set_local_selected_hotbar_slot(8));
    store.local_player.camera = CameraState {
        entity_id: Some(99),
        follows_player: false,
        entity_known: true,
    };
    store.local_player.pose = Some(LocalPlayerPoseState::default());

    let mut login = login_packet(0, "minecraft:overworld");
    login.common_spawn_info.game_type = 3;
    login.common_spawn_info.portal_cooldown = 4;
    store.apply_login(&login);

    assert_eq!(store.local_player_id(), Some(42));
    assert_eq!(store.chunk_count(), 0);
    // Only the new LocalPlayer entity remains.
    assert_eq!(store.entity_count(), 1);
    assert!(store.probe_entities().is_empty());
    assert_eq!(
        store.local_player().abilities,
        Some(crate::LocalPlayerAbilitiesState {
            invulnerable: true,
            flying: true,
            can_fly: true,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        })
    );
    assert!(store.local_player().health.is_none());
    assert!(store.local_player().experience.is_none());
    assert_eq!(store.local_player().selected_hotbar_slot, 0);
    assert_eq!(store.local_player().simulation_distance, Some(6));
    assert_eq!(store.local_player().portal_cooldown, 4);
    assert!(store.local_player().pose.is_none());
    assert_eq!(
        store.local_player().camera,
        CameraState {
            entity_id: Some(42),
            follows_player: true,
            entity_known: true,
        }
    );
    assert_eq!(store.counters().player_abilities_packets, 1);
    assert_eq!(store.counters().player_health_packets, 1);
    assert_eq!(store.counters().player_experience_packets, 1);
}

#[test]
fn insecure_chat_warning_guard_survives_transfer_level_reset() {
    let mut store = WorldStore::new();
    store.replace_services_public_keys(Vec::new(), vec![vec![1]]);
    store.apply_login(&login_packet(0, "minecraft:overworld"));
    assert!(store.effectively_enforces_secure_chat());
    assert!(!store.seen_insecure_chat_warning());
    assert_eq!(store.insecure_chat_warning_count(), 0);

    let mut insecure_login = login_packet(0, "minecraft:overworld");
    insecure_login.enforces_secure_chat = false;
    store.apply_login(&insecure_login);
    assert!(!store.effectively_enforces_secure_chat());
    assert!(store.seen_insecure_chat_warning());
    assert_eq!(store.insecure_chat_warning_count(), 1);

    store.apply_transfer(ProtocolTransfer {
        host: "next.example.com".into(),
        port: 25566,
    });
    store.clear_client_level();
    assert!(store.profile_key_validation_available());
    assert!(store.seen_insecure_chat_warning());
    assert_eq!(store.insecure_chat_warning_count(), 1);

    store.apply_login(&insecure_login);
    assert_eq!(store.insecure_chat_warning_count(), 1);
}

#[test]
fn respawn_updates_dimension_and_clears_old_chunks() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 256,
    });
    store
        .available_levels
        .insert("minecraft:overworld".to_string());
    store.server_enforces_secure_chat = Some(true);
    store.chunks.push(stale_chunk().into());
    store.first_chunk = Some(ChunkPos { x: 1, z: -2 });
    store.apply_add_entity(protocol_add_entity(123));
    assert!(store.apply_map_item_data(ProtocolMapItemData {
        map_id: 42,
        scale: 2,
        locked: true,
        decorations: None,
        color_patch: None,
    }));
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    store.client_debug_game.last_debug_block_value = Some(crate::DebugBlockValueState {
        pos: BlockPos { x: 1, y: 2, z: 3 },
        subscription_id: 6,
        value_present: true,
    });
    store.client_debug_game.last_debug_chunk_value = Some(crate::DebugChunkValueState {
        pos: ChunkPos { x: 1, z: -2 },
        subscription_id: 11,
        value_present: false,
    });
    store.client_debug_game.last_debug_entity_value = Some(crate::DebugEntityValueState {
        entity_id: 123,
        subscription_id: 3,
        value_present: false,
    });
    store.client_debug_game.last_debug_event = Some(crate::DebugEventState {
        subscription_id: 14,
    });
    store.client_debug_game.last_debug_sample = Some(crate::DebugSampleState {
        sample_len: 2,
        sample_type: "tick_time".to_string(),
    });
    store.local_player.default_spawn = Some(crate::DefaultSpawnState {
        dimension: "minecraft:the_nether".to_string(),
        pos: BlockPos { x: 8, y: 64, z: 8 },
        yaw: 0.0,
        pitch: 0.0,
    });
    store.set_local_prediction_sequence(12);
    store.mark_local_prediction_teleport();
    store.record_local_block_prediction(12, BlockPos { x: 3, y: 4, z: 5 }, 1, 2, None);

    let same_dimension = store.apply_respawn(&ProtocolRespawn {
        common_spawn_info: ProtocolSpawnInfo {
            dimension_type_id: 1,
            dimension: "minecraft:the_nether".to_string(),
            seed: 12345,
            game_type: 1,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 6,
            sea_level: 32,
        },
        data_to_keep: 3,
    });
    assert_eq!(same_dimension.closed_server_container_id, Some(7));
    assert!(!same_dimension.dimension_changed);
    assert_eq!(store.open_container_id(), None);
    assert_eq!(store.chunk_count(), 1);
    assert_eq!(store.first_chunk(), Some(ChunkPos { x: 1, z: -2 }));
    assert_eq!(store.entity_count(), 1);
    assert_eq!(store.level_info().unwrap().biome_zoom_seed, 12345);
    assert_eq!(store.local_player().portal_cooldown, 6);
    assert!(store.last_debug_block_value().is_some());
    assert!(store.local_player().default_spawn.is_some());
    assert_eq!(store.client_level_generation(), 0);
    assert_eq!(store.local_prediction_sequence(), 12);
    assert_eq!(store.local_prediction_last_teleport_sequence(), Some(12));
    assert_eq!(store.local_block_predictions().len(), 1);
    let available_levels = store.available_levels().clone();
    let server_enforces_secure_chat = store.server_enforces_secure_chat();

    let changed_dimension = store.apply_respawn(&ProtocolRespawn {
        common_spawn_info: ProtocolSpawnInfo {
            dimension_type_id: 2,
            dimension: "minecraft:the_end".to_string(),
            seed: 98765,
            game_type: 1,
            previous_game_type: 1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 2,
            sea_level: 63,
        },
        data_to_keep: 3,
    });
    assert_eq!(changed_dimension.closed_server_container_id, None);
    assert!(changed_dimension.dimension_changed);

    assert_eq!(
        store.dimension(),
        WorldDimension {
            min_y: 0,
            height: 256,
        }
    );
    assert_eq!(store.chunk_count(), 0);
    assert_eq!(store.first_chunk(), None);
    assert_eq!(store.entity_count(), 0);
    assert_eq!(store.local_prediction_sequence(), 0);
    assert_eq!(store.local_prediction_last_teleport_sequence(), None);
    assert!(store.local_block_predictions().is_empty());
    assert!(store.last_debug_block_value().is_none());
    assert!(store.last_debug_chunk_value().is_none());
    assert!(store.last_debug_entity_value().is_none());
    assert!(store.last_debug_event().is_none());
    assert_eq!(
        store.last_debug_sample(),
        Some(&crate::DebugSampleState {
            sample_len: 2,
            sample_type: "tick_time".to_string(),
        })
    );
    assert!(store.local_player().default_spawn.is_none());
    let map = store.map_item(42).expect("map survives dimension respawn");
    assert_eq!(map.scale, 2);
    assert!(map.locked);
    assert_eq!(store.counters().maps_tracked, 1);
    assert_eq!(store.counters().entities_tracked, 0);
    assert_eq!(store.counters().respawns_received, 2);
    let level = store.level_info().unwrap();
    assert_eq!(level.biome_zoom_seed, 98765);
    assert_eq!(store.local_player().portal_cooldown, 2);
    assert_eq!(store.client_level_generation(), 1);
    assert_eq!(store.available_levels(), &available_levels);
    assert_eq!(
        store.server_enforces_secure_chat(),
        server_enforces_secure_chat
    );
    assert_eq!(level.dimension, "minecraft:the_end");
    assert_eq!(level.dimension_type_id, 2);
    assert_eq!(
        level.dimension_type_name.as_deref(),
        Some("minecraft:the_end")
    );
    assert_eq!(level.cardinal_lighting(), WorldCardinalLighting::Default);
    assert_eq!(store.gameplay().game_type, 1);
    assert_eq!(store.gameplay().game_type_name, "creative");
    assert_eq!(store.gameplay().previous_game_type, Some(1));
    assert_eq!(
        store.gameplay().previous_game_type_name.as_deref(),
        Some("creative")
    );
}

#[test]
fn client_level_generation_follows_construct_replace_and_clear_identity_only() {
    let mut store = WorldStore::new();
    store.apply_login(&login_packet(0, "minecraft:overworld"));
    let login_generation = store.client_level_generation();
    assert_eq!(login_generation, 1);
    let retained_login = store.publication_snapshot();

    store.apply_respawn(&respawn_packet(0));
    assert_eq!(store.client_level_generation(), login_generation);
    assert_eq!(retained_login.client_level_generation(), login_generation);
    assert_eq!(
        retained_login
            .level_info()
            .expect("retained login level")
            .dimension
            .as_str(),
        "minecraft:overworld"
    );

    let mut changed_dimension = respawn_packet(0);
    changed_dimension.common_spawn_info.dimension_type_id = 1;
    changed_dimension.common_spawn_info.dimension = "minecraft:the_nether".to_string();
    store.apply_respawn(&changed_dimension);
    assert_eq!(
        store.client_level_generation(),
        login_generation.saturating_add(1)
    );
    assert_eq!(
        retained_login.client_level_generation(),
        login_generation,
        "retained publications must not observe replacement"
    );

    store.clear_client_level();
    assert_eq!(
        store.client_level_generation(),
        login_generation.saturating_add(2)
    );
    assert!(store.level_info().is_none());
}

#[test]
fn respawn_without_keep_data_resets_local_player_runtime_state() {
    let mut store = respawn_state_with_local_player_data();

    store.apply_respawn(&respawn_packet(0));

    assert_eq!(
        store.local_player().abilities,
        Some(crate::LocalPlayerAbilitiesState {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
        })
    );
    assert!(store.local_player().health.is_none());
    assert!(store.local_player().experience.is_none());
    assert_eq!(store.local_player().selected_hotbar_slot, 0);
    assert_eq!(
        store.local_player().permission_level,
        crate::LocalPlayerPermissionLevel::NoPermissions
    );
    assert!(store.local_player().default_spawn.is_some());
    assert_eq!(store.local_player().simulation_distance, Some(12));
    assert_eq!(store.local_player_pose(), None);
    assert!(!store.local_player_is_sprinting());
    assert_eq!(
        store.local_player().camera,
        CameraState {
            entity_id: Some(123),
            follows_player: true,
            entity_known: true,
        }
    );
    assert_eq!(
        store.local_player().interaction,
        crate::LocalPlayerInteractionState::default()
    );

    let entity = store.probe_entity(123).unwrap();
    assert_eq!(entity.data, 0);
    assert_eq!(entity.position, Default::default());
    assert_eq!(entity.delta_movement, Default::default());
    assert_eq!(entity.y_rot, -180.0);
    assert_eq!(entity.x_rot, 0.0);
    assert!(entity.data_values.is_empty());
    assert!(entity.equipment.is_empty());
    assert_eq!(entity.leash_holder_id, None);
    assert_eq!(entity.last_event_id, None);
    assert_eq!(entity.main_supporting_block_pos, None);
    assert!(!entity.on_ground_no_blocks);
    assert!(entity.mob_effects.is_empty());
    assert_eq!(store.counters().active_mob_effects_tracked, 0);
    assert_eq!(entity.attributes.len(), 1);
    assert_eq!(entity.attributes[0].base, 0.1);
    assert!(entity.attributes[0].modifiers.is_empty());
}

#[test]
fn respawn_keep_all_data_preserves_entity_data_pose_and_attribute_modifiers() {
    let mut store = respawn_state_with_local_player_data();
    let old_pose = store.local_player_pose().unwrap();
    store.local_player_old_pose = Some(old_pose);
    let old_data = store.probe_entity(123).unwrap().data_values;

    store.apply_respawn(&respawn_packet(3));

    assert!(store.local_player().health.is_none());
    assert!(store.local_player().experience.is_none());
    assert_eq!(
        store.local_player_pose(),
        Some(LocalPlayerPoseState {
            position: Default::default(),
            delta_movement: old_pose.delta_movement,
            on_ground: false,
            horizontal_collision: false,
            minor_horizontal_collision: false,
            fall_distance: 0.0,
            sneaking: old_pose.sneaking,
            swimming: old_pose.swimming,
            no_jump_delay_ticks: 0,
            y_rot: old_pose.y_rot,
            x_rot: old_pose.x_rot,
            last_teleport_id: 0,
        })
    );
    assert_eq!(
        store.local_player().camera,
        CameraState {
            entity_id: Some(123),
            follows_player: true,
            entity_known: true,
        }
    );
    assert_eq!(store.local_player_old_pose, None);
    assert_eq!(
        store.local_player().interaction,
        crate::LocalPlayerInteractionState::default()
    );

    // KEEP_ENTITY_DATA copies the old LocalPlayer's synced data, including
    // the pose and flags it set itself.
    let entity = store.probe_entity(123).unwrap();
    assert_eq!(entity.data_values, old_data);
    assert!(entity.mob_effects.is_empty());
    assert_eq!(store.counters().active_mob_effects_tracked, 0);
    assert_eq!(entity.attributes.len(), 1);
    assert_eq!(entity.attributes[0].base, 0.1);
    assert_eq!(
        entity.attributes[0].modifiers,
        vec![ProtocolAttributeModifier {
            id: "minecraft:test_speed".to_string(),
            amount: 0.25,
            operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
        }]
    );
}

#[test]
fn respawn_keep_bits_preserve_entity_data_and_attribute_modifiers_independently() {
    let mut attributes_only = respawn_state_with_local_player_data();
    attributes_only.apply_respawn(&respawn_packet(RESPAWN_KEEP_ATTRIBUTE_MODIFIERS));
    assert!(attributes_only.local_player_pose().is_none());
    assert!(!attributes_only.local_player_is_sprinting());
    let entity = attributes_only.probe_entity(123).unwrap();
    assert!(entity.data_values.is_empty());
    assert_eq!(entity.attributes[0].modifiers.len(), 1);

    let mut entity_data_only = respawn_state_with_local_player_data();
    let old_pose = entity_data_only.local_player_pose().unwrap();
    let old_data = entity_data_only.probe_entity(123).unwrap().data_values;
    entity_data_only.apply_respawn(&respawn_packet(RESPAWN_KEEP_ENTITY_DATA));
    assert!(entity_data_only.local_player_is_sprinting());
    assert_eq!(
        entity_data_only.local_player_pose(),
        Some(LocalPlayerPoseState {
            position: Default::default(),
            delta_movement: old_pose.delta_movement,
            on_ground: false,
            horizontal_collision: false,
            minor_horizontal_collision: false,
            fall_distance: 0.0,
            sneaking: old_pose.sneaking,
            swimming: old_pose.swimming,
            no_jump_delay_ticks: 0,
            y_rot: old_pose.y_rot,
            x_rot: old_pose.x_rot,
            last_teleport_id: 0,
        })
    );
    let entity = entity_data_only.probe_entity(123).unwrap();
    assert_eq!(entity.data_values, old_data);
    assert!(entity.attributes[0].modifiers.is_empty());
}

#[test]
fn world_time_weather_and_ticking_are_canonical_state() {
    let mut store = WorldStore::new();

    store.apply_world_time(ProtocolPlayTime {
        game_time: 123,
        clock_updates: vec![ProtocolClockUpdate {
            clock_id: 0,
            total_ticks: 6000,
            partial_tick: 0.25,
            rate: 1.0,
        }],
    });
    store.apply_game_event(ProtocolGameEvent {
        event_id: 7,
        param: 0.5,
    });
    store.apply_game_event(ProtocolGameEvent {
        event_id: 8,
        param: 0.75,
    });
    store.apply_ticking_state(ProtocolTickingState {
        tick_rate: 0.25,
        frozen: true,
    });
    store.apply_ticking_step(ProtocolTickingStep { tick_steps: 7 });

    let time = store.world_time().unwrap();
    assert_eq!(time.game_time, 123);
    assert_eq!(time.day_time, 6000);
    assert_eq!(
        time.clock_updates,
        vec![ClockUpdateState {
            clock_id: 0,
            total_ticks: 6000,
            partial_tick: 0.25,
            rate: 1.0,
        }]
    );

    assert_eq!(
        store.weather(),
        WorldWeatherState {
            raining: true,
            rain_level: 0.5,
            thunder_level: 0.75,
            last_game_event_id: Some(8),
            last_game_event_param: 0.75,
        }
    );
    assert_eq!(
        store.ticking(),
        WorldTickingState {
            tick_rate: 1.0,
            frozen: true,
            frozen_ticks_to_run: 7,
        }
    );

    let counters = store.counters();
    assert_eq!(counters.world_time_packets, 1);
    assert_eq!(counters.game_event_packets, 2);
    assert_eq!(counters.ticking_state_packets, 1);
    assert_eq!(counters.ticking_step_packets, 1);
}

#[test]
fn world_time_selects_dimension_default_clock_by_session_holder_identity() {
    let mut store = WorldStore::new();
    store.record_registry_data(RegistryData {
        registry: "minecraft:world_clock".to_string(),
        raw_payload_len: 0,
        entries: vec![
            RegistryDataEntry {
                id: "minecraft:the_end".to_string(),
                raw_data: Some(vec![10, 0]),
            },
            RegistryDataEntry {
                id: "minecraft:overworld".to_string(),
                raw_data: Some(vec![10, 0]),
            },
        ],
    });
    record_custom_dimension_type_registry(
        &mut store,
        dimension_type_nbt_with_default_clock("minecraft:overworld"),
    )
    .unwrap();
    store.apply_login(&login_packet(0, "example:custom"));

    store.apply_world_time(ProtocolPlayTime {
        game_time: 100,
        clock_updates: vec![
            ProtocolClockUpdate {
                clock_id: 0,
                total_ticks: 9_000,
                partial_tick: 0.0,
                rate: 1.0,
            },
            ProtocolClockUpdate {
                clock_id: 1,
                total_ticks: 6_000,
                partial_tick: 0.0,
                rate: 1.0,
            },
        ],
    });

    assert_eq!(
        store
            .level_info()
            .unwrap()
            .default_clock_resource_id
            .as_deref(),
        Some("minecraft:overworld")
    );
    assert_eq!(store.world_time().unwrap().day_time, 6_000);

    store.apply_respawn(&ProtocolRespawn {
        common_spawn_info: login_packet(2, "minecraft:the_end").common_spawn_info,
        data_to_keep: 3,
    });
    assert_eq!(store.world_time().unwrap().day_time, 9_000);

    store.apply_respawn(&ProtocolRespawn {
        common_spawn_info: login_packet(1, "minecraft:the_nether").common_spawn_info,
        data_to_keep: 3,
    });
    assert_eq!(store.level_info().unwrap().default_clock_resource_id, None);
    assert_eq!(store.world_time().unwrap().day_time, 0);
}

#[test]
fn world_time_merges_partial_clock_updates_like_client_clock_manager() {
    let mut store = WorldStore::new();
    store.apply_world_time(ProtocolPlayTime {
        game_time: 100,
        clock_updates: vec![
            ProtocolClockUpdate {
                clock_id: 0,
                total_ticks: 10,
                partial_tick: 0.0,
                rate: 1.0,
            },
            ProtocolClockUpdate {
                clock_id: 1,
                total_ticks: 20,
                partial_tick: 0.0,
                rate: 1.0,
            },
        ],
    });
    store.apply_world_time(ProtocolPlayTime {
        game_time: 102,
        clock_updates: vec![ProtocolClockUpdate {
            clock_id: 1,
            total_ticks: 50,
            partial_tick: 0.5,
            rate: 0.0,
        }],
    });

    let time = store.world_time().unwrap();
    assert_eq!(time.day_time, 12);
    assert_eq!(time.clock_updates.len(), 2);
    assert_eq!(time.clock_updates[0].total_ticks, 12);
    assert_eq!(time.clock_updates[1].total_ticks, 50);
}

#[test]
fn advance_client_time_ticks_game_time_and_clock_instances_like_vanilla() {
    let mut store = WorldStore::new();
    store.apply_world_time(ProtocolPlayTime {
        game_time: 100,
        clock_updates: vec![
            ProtocolClockUpdate {
                clock_id: 0,
                total_ticks: 1_485,
                partial_tick: 0.75,
                rate: 0.5,
            },
            ProtocolClockUpdate {
                clock_id: 2,
                total_ticks: 10,
                partial_tick: 0.25,
                rate: 2.25,
            },
        ],
    });

    store.advance_client_time(1);

    let time = store.world_time().unwrap();
    assert_eq!(time.game_time, 101);
    assert_eq!(time.day_time, 1_486);
    assert_eq!(
        time.clock_updates,
        vec![
            ClockUpdateState {
                clock_id: 0,
                total_ticks: 1_486,
                partial_tick: 0.25,
                rate: 0.5,
            },
            ClockUpdateState {
                clock_id: 2,
                total_ticks: 12,
                partial_tick: 0.5,
                rate: 2.25,
            },
        ]
    );
}

#[test]
fn advance_client_time_preserves_paused_clock_updates() {
    let mut store = WorldStore::new();
    store.apply_world_time(ProtocolPlayTime {
        game_time: 100,
        clock_updates: vec![ProtocolClockUpdate {
            clock_id: 0,
            total_ticks: 1_485,
            partial_tick: 0.75,
            rate: 0.0,
        }],
    });

    store.advance_client_time(4);

    let time = store.world_time().unwrap();
    assert_eq!(time.game_time, 104);
    assert_eq!(time.day_time, 1_485);
    assert_eq!(
        time.clock_updates,
        vec![ClockUpdateState {
            clock_id: 0,
            total_ticks: 1_485,
            partial_tick: 0.75,
            rate: 0.0,
        }]
    );
}

#[test]
fn sky_flash_time_decrements_on_client_ticks_and_clears_with_level() {
    let mut store = WorldStore::new();

    store.set_sky_flash_time(2);
    assert_eq!(store.sky_flash_time(), 2);

    store.advance_sky_flash_time(1);
    assert_eq!(store.sky_flash_time(), 1);

    store.advance_sky_flash_time(10);
    assert_eq!(store.sky_flash_time(), 0);

    store.set_sky_flash_time(-1);
    assert_eq!(store.sky_flash_time(), 0);

    store.trigger_sky_flash();
    assert_eq!(store.sky_flash_time(), 2);
    store.clear_client_level();
    assert_eq!(store.sky_flash_time(), 0);
}

#[test]
fn frozen_ticking_gates_production_time_advance_but_not_set_time_resync() {
    let mut store = WorldStore::new();
    store.apply_world_time(ProtocolPlayTime {
        game_time: 100,
        clock_updates: vec![ProtocolClockUpdate {
            clock_id: 0,
            total_ticks: 6_000,
            partial_tick: 0.0,
            rate: 1.0,
        }],
    });
    store.apply_ticking_state(ProtocolTickingState {
        tick_rate: 20.0,
        frozen: true,
    });

    // The production entry consumes zero running ticks while frozen, so
    // neither gameTime nor any clock instance advances (vanilla
    // tickRateManager.runsNormally gating ClientLevel.tickTime).
    let (_, running_ticks) = store.advance_client_level_entity_ticks_with_observation(3);
    assert_eq!(running_ticks, 0);
    let time = store.world_time().expect("world time");
    assert_eq!(time.game_time, 100);
    assert_eq!(time.clock_updates[0].total_ticks, 6_000);

    // Sky flash decay uses raw client ticks, outside the freeze gate.
    store.trigger_sky_flash();
    store.advance_sky_flash_time(2);
    assert_eq!(store.sky_flash_time(), 0);

    // Vanilla handleSetTime resyncs unconditionally while frozen.
    store.apply_world_time(ProtocolPlayTime {
        game_time: 220,
        clock_updates: Vec::new(),
    });
    assert_eq!(store.world_time().expect("world time").game_time, 220);
}

#[test]
fn consume_running_render_ticks_respects_frozen_tick_steps() {
    let mut store = WorldStore::new();
    assert_eq!(store.consume_running_render_ticks(3), 3);

    store.apply_ticking_state(ProtocolTickingState {
        tick_rate: 20.0,
        frozen: true,
    });
    assert_eq!(store.consume_running_render_ticks(3), 0);

    store.apply_ticking_step(ProtocolTickingStep { tick_steps: 2 });
    assert_eq!(store.consume_running_render_ticks(3), 2);
    assert_eq!(
        store.ticking(),
        WorldTickingState {
            tick_rate: 20.0,
            frozen: true,
            frozen_ticks_to_run: 0,
        }
    );

    store.apply_ticking_state(ProtocolTickingState {
        tick_rate: 20.0,
        frozen: false,
    });
    store.apply_ticking_step(ProtocolTickingStep { tick_steps: 2 });
    assert_eq!(store.consume_running_render_ticks(3), 3);
    assert_eq!(store.ticking().frozen_ticks_to_run, 0);
}

#[test]
fn game_events_update_local_gameplay_state() {
    let mut store = WorldStore::new();

    store.apply_login(&ProtocolPlayLogin {
        player_id: 42,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: ProtocolSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 12345,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });

    assert!(!store.local_player_is_spectator());
    store.set_local_player_pose(crate::LocalPlayerPoseState {
        delta_movement: ProtocolVec3d {
            x: 0.1,
            y: -0.5,
            z: 0.2,
        },
        ..crate::LocalPlayerPoseState::default()
    });
    store.apply_game_event(ProtocolGameEvent {
        event_id: 3,
        param: 3.0,
    });
    assert!(store.local_player_is_spectator());
    let pose = store.local_player().pose.expect("pose survives the switch");
    assert_eq!(
        pose.delta_movement.y, 0.0,
        "entering spectator zeroes vertical motion like onGameModeChanged"
    );
    assert_eq!(pose.delta_movement.x, 0.1);
    store.apply_game_event(ProtocolGameEvent {
        event_id: 11,
        param: 1.0,
    });
    store.apply_game_event(ProtocolGameEvent {
        event_id: 12,
        param: 1.0,
    });

    assert_eq!(
        store.gameplay(),
        &WorldGameplayState {
            game_type: 3,
            game_type_name: "spectator".to_string(),
            previous_game_type: Some(0),
            previous_game_type_name: Some("survival".to_string()),
            show_death_screen: false,
            do_limited_crafting: true,
            reduced_debug_info: false,
            hardcore: false,
        }
    );
    assert_eq!(store.counters().game_event_packets, 3);
}

#[test]
fn no_respawn_block_game_event_records_translatable_client_system_message() {
    let mut store = WorldStore::new();

    store.apply_game_event(ProtocolGameEvent {
        event_id: VANILLA_CLIENT_GAME_EVENT_NO_RESPAWN_BLOCK_AVAILABLE_ID,
        param: 0.0,
    });

    let message = store.client_chat().messages.last().unwrap();
    assert_eq!(message.kind, crate::ChatMessageKind::ClientSystem);
    assert_eq!(message.content, "block.minecraft.spawn.not_valid");
    assert_eq!(
        message.content_translation_key.as_deref(),
        Some("block.minecraft.spawn.not_valid")
    );
    assert!(message.styled_content.is_empty());
    assert!(message.content_component.is_some());
    let narration = store
        .client_chat()
        .pending_narrations
        .last()
        .expect("system-chat narration");
    assert_eq!(narration.kind, ChatNarrationKind::SystemChat);
    assert!(narration.content_component.is_some());
}

#[test]
fn win_and_demo_intro_game_events_replace_supported_screens_by_generation() {
    let mut store = WorldStore::new();
    store.apply_game_event(ProtocolGameEvent {
        event_id: VANILLA_CLIENT_GAME_EVENT_WIN_GAME_ID,
        param: 1.0,
    });

    store.apply_game_event(ProtocolGameEvent {
        event_id: VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID,
        param: 0.0,
    });
    assert_eq!(
        store.screen_requests(),
        [
            crate::ScreenRequest::GameEvent(ClientGameEventScreenState::Win { generation: 1 }),
            crate::ScreenRequest::GameEvent(ClientGameEventScreenState::DemoHelp { generation: 2 }),
        ]
    );
}

#[test]
fn demo_hint_game_events_record_rich_translations_and_system_narration() {
    let mut store = WorldStore::new();
    let hints = [
        (101.0, "demo.help.movement", &["W", "A", "S", "D"][..]),
        (102.0, "demo.help.jump", &["Space"][..]),
        (103.0, "demo.help.inventory", &["E"][..]),
        (104.0, "demo.day.6", &["F2"][..]),
    ];

    for (param, key, arguments) in hints {
        store.apply_game_event(ProtocolGameEvent {
            event_id: VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID,
            param,
        });
        let message = store.client_chat().messages.last().unwrap();
        assert_eq!(message.content_translation_key.as_deref(), Some(key));
        let component = message.content_component.as_ref().unwrap();
        let bbb_protocol::NbtSummaryValue::Compound(entries) = component else {
            panic!("translated hint must be a component compound");
        };
        let with = entries
            .iter()
            .find(|entry| entry.name == "with")
            .map(|entry| &entry.value);
        assert_eq!(
            with,
            Some(&bbb_protocol::NbtSummaryValue::List(
                arguments
                    .iter()
                    .map(|argument| { bbb_protocol::NbtSummaryValue::string(*argument) })
                    .collect()
            ))
        );
        let narration = store.client_chat().pending_narrations.last().unwrap();
        assert_eq!(narration.kind, ChatNarrationKind::System);
        assert_eq!(narration.content_component.as_ref(), Some(component));
    }

    let message_count = store.client_chat().messages.len();
    store.apply_game_event(ProtocolGameEvent {
        event_id: VANILLA_CLIENT_GAME_EVENT_DEMO_EVENT_ID,
        param: 100.0,
    });
    assert_eq!(store.client_chat().messages.len(), message_count);
}

#[test]
fn respawn_and_level_clear_close_death_screen() {
    let mut store = WorldStore::new();
    store.apply_login(&login_packet(0, "minecraft:overworld"));
    store.apply_player_combat_kill(bbb_protocol::packets::PlayerCombatKill {
        player_id: 42,
        message: bbb_protocol::RichTextComponent::plain("First death"),
    });
    store.take_screen_requests();

    store.apply_respawn(&respawn_packet(0));
    assert_eq!(
        store.take_screen_requests().first(),
        Some(&crate::ScreenRequest::CloseDeathScreen)
    );

    store.clear_client_level();
    assert_eq!(
        store.screen_requests(),
        [crate::ScreenRequest::CloseLevelScreens]
    );
}

#[test]
fn level_chunks_load_start_advances_active_level_load_tracker_once() {
    let mut store = WorldStore::new();
    let event = ProtocolGameEvent {
        event_id: VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
        param: 0.0,
    };
    store.apply_game_event(event);
    assert_eq!(store.level_load(), &WorldLevelLoadState::default());

    store.apply_login(&login_packet(0, "minecraft:overworld"));
    assert_eq!(
        store.level_load().phase,
        WorldLevelLoadPhase::WaitingForServer
    );
    assert_eq!(store.level_load().starts, 1);

    store.apply_game_event(event);
    store.apply_game_event(event);

    assert_eq!(
        store.level_load().phase,
        WorldLevelLoadPhase::WaitingForPlayerChunk
    );
    assert_eq!(store.level_load().loading_packets_received, 2);

    assert!(store.complete_client_level_load(false));
    assert_eq!(store.level_load().phase, WorldLevelLoadPhase::ClientLoaded);
    assert_eq!(store.level_load().loads_completed, 1);
    assert_eq!(store.level_load().timeouts, 0);
    assert!(!store.complete_client_level_load(true));
    store.apply_game_event(event);
    assert_eq!(store.level_load().loading_packets_received, 2);

    store.apply_respawn(&respawn_packet(0));
    assert_eq!(
        store.level_load().phase,
        WorldLevelLoadPhase::WaitingForServer
    );
    assert_eq!(store.level_load().starts, 2);
    assert_eq!(store.level_load().loading_packets_received, 0);
    assert!(!store.complete_client_level_load(true));

    store.apply_game_event(event);
    assert!(store.complete_client_level_load(true));
    assert_eq!(store.level_load().loads_completed, 2);
    assert_eq!(store.level_load().timeouts, 1);
}

#[test]
fn login_resets_map_store_while_dimension_respawn_carries_it_over() {
    let mut store = WorldStore::new();
    store.apply_login(&login_packet(0, "minecraft:overworld"));
    assert!(store.apply_map_item_data(ProtocolMapItemData {
        map_id: 9,
        scale: 1,
        locked: false,
        decorations: None,
        color_patch: None,
    }));
    assert_eq!(store.counters().maps_tracked, 1);

    store.apply_respawn(&ProtocolRespawn {
        common_spawn_info: ProtocolSpawnInfo {
            dimension_type_id: 1,
            dimension: "minecraft:the_nether".to_string(),
            seed: 1,
            game_type: 0,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 32,
        },
        data_to_keep: 0,
    });
    assert!(
        store.map_item(9).is_some(),
        "dimension respawn carries the map table over"
    );

    store.apply_login(&login_packet(0, "minecraft:overworld"));
    assert!(
        store.map_item(9).is_none(),
        "login starts from an empty map table"
    );
    assert!(store.map_items().is_empty());
    assert!(store.last_map_color_patch().is_none());
    assert_eq!(store.counters().maps_tracked, 0);
    assert_eq!(store.counters().map_decorations_tracked, 0);
}

#[test]
fn spawn_info_uses_builtin_cardinal_lighting_when_dimension_type_registry_is_missing() {
    let mut nether = WorldStore::new();
    nether.apply_login(&login_packet(1, "minecraft:custom_nether_key"));
    assert!(!nether.level_info().unwrap().has_sky_light());
    assert_eq!(nether.debug_sky_light_section_status([0, 0, 0]), "n/a");
    assert_eq!(
        nether.debug_sky_light_section_occupancy([0, 0, 0]),
        [false, false]
    );
    assert_eq!(
        nether.level_info().unwrap().cardinal_lighting(),
        WorldCardinalLighting::Nether
    );

    let mut remapped_nether = WorldStore::new();
    remapped_nether.apply_login(&login_packet(7, "minecraft:the_nether"));
    assert!(!remapped_nether.level_info().unwrap().has_sky_light());
    assert_eq!(
        remapped_nether.level_info().unwrap().cardinal_lighting(),
        WorldCardinalLighting::Nether
    );

    let mut end = WorldStore::new();
    end.apply_login(&login_packet(2, "minecraft:the_end"));
    assert!(!end.level_info().unwrap().has_sky_light());
    assert_eq!(
        end.level_info().unwrap().cardinal_lighting(),
        WorldCardinalLighting::Default
    );

    let mut overworld = WorldStore::new();
    overworld.apply_login(&login_packet(0, "minecraft:overworld"));
    assert!(overworld.level_info().unwrap().has_sky_light());
}

#[test]
fn dimension_type_registry_cardinal_light_overrides_builtin_default() {
    let mut custom_nether = WorldStore::new();
    record_custom_dimension_type_registry(
        &mut custom_nether,
        dimension_type_nbt(
            Some("nether"),
            Some(false),
            Some(true),
            None,
            Some("none"),
            Some(0.1),
        ),
    )
    .unwrap();
    custom_nether.apply_login(&login_packet(0, "example:custom"));
    assert!(!custom_nether.level_info().unwrap().has_sky_light());
    assert_eq!(
        custom_nether.debug_sky_light_section_status([0, 0, 0]),
        "n/a"
    );
    assert_eq!(
        custom_nether.level_info().unwrap().cardinal_lighting(),
        WorldCardinalLighting::Nether
    );

    let mut omitted_field = WorldStore::new();
    record_custom_dimension_type_registry(
        &mut omitted_field,
        dimension_type_nbt(None, None, None, None, None, None),
    )
    .unwrap();
    omitted_field.apply_login(&login_packet(0, "minecraft:the_nether"));
    assert_eq!(
        omitted_field.level_info().unwrap().cardinal_lighting(),
        WorldCardinalLighting::Default
    );
}

#[test]
fn dimension_type_registry_shape_overrides_builtin_and_rejects_invalid_shape() {
    let mut custom = WorldStore::new();
    record_custom_dimension_type_registry(
        &mut custom,
        dimension_type_nbt(
            None,
            Some(true),
            Some(true),
            Some((-32, 512, 384)),
            Some("end"),
            Some(0.35),
        ),
    )
    .unwrap();
    custom.apply_login(&login_packet(0, "example:custom"));
    assert_eq!(
        custom.level_info().unwrap().dimension_type_name.as_deref(),
        Some("example:custom")
    );
    assert_eq!(
        custom.dimension(),
        WorldDimension {
            min_y: -32,
            height: 512,
        }
    );
    assert_eq!(custom.level_info().unwrap().skybox(), WorldSkybox::End);
    assert_eq!(custom.level_info().unwrap().ambient_light(), 0.35);
    assert!(custom.level_info().unwrap().has_ceiling());
    assert!(!custom.level_info().unwrap().can_have_weather());

    for invalid_shape in [
        (-31, 512, 384),
        (-32, 512, 513),
        (0, 15, 15),
        (0, 4080, 384),
        (2016, 32, 32),
        (-2048, 16, 16),
    ] {
        let mut invalid = WorldStore::new();
        assert!(record_custom_dimension_type_registry(
            &mut invalid,
            dimension_type_nbt(None, Some(true), None, Some(invalid_shape), None, None),
        )
        .is_err());
    }
}

#[test]
fn apply_login_records_the_hardcore_flag() {
    let mut world = WorldStore::new();
    assert!(!world.is_hardcore());

    let mut login = login_packet(0, "minecraft:overworld");
    login.hardcore = true;
    world.apply_login(&login);
    assert!(world.is_hardcore());
    assert!(world.gameplay().hardcore);

    // A later non-hardcore login clears it.
    world.apply_login(&login_packet(0, "minecraft:overworld"));
    assert!(!world.is_hardcore());
}

#[test]
fn connection_services_survive_respawn_and_reset_only_with_the_listener() {
    let mut world = WorldStore::new();
    world.apply_login(&login_packet(0, "minecraft:overworld"));
    assert!(
        world.apply_set_objective(bbb_protocol::packets::SetObjective {
            objective_name: "session".into(),
            method: bbb_protocol::packets::SetObjectiveMethod::Add.into(),
            parameters: Some(bbb_protocol::packets::SetObjectiveParameters {
                display_name: bbb_protocol::RichTextComponent::plain("Session"),
                render_type: bbb_protocol::objective_render_types::ObjectiveRenderType::Integer,
                number_format: None,
            }),
        })
    );
    world.apply_update_recipes(bbb_protocol::packets::UpdateRecipes {
        property_sets: vec![bbb_protocol::packets::RecipePropertySetSummary {
            key: "minecraft:smithing_base".to_string(),
            item_ids: vec![7],
        }],
        stonecutter_recipes: Vec::new(),
    });
    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec!["minecraft:vanilla".to_string()],
    });
    world.set_brewing_potion_item_ids(BTreeSet::from([44]));
    world.set_furnace_fuel_item_ids(BTreeSet::from([43]));

    // Vanilla keeps the same ClientPacketListener across any respawn, so
    // every connection-scoped service survives a dimension change.
    world.apply_respawn(&respawn_packet(0));
    assert_eq!(world.scoreboard.objectives.len(), 1);
    assert_eq!(world.recipes().property_sets.len(), 1);
    assert!(world.is_feature_enabled("minecraft:vanilla"));
    assert_eq!(*world.items.brewing_potion_item_ids, BTreeSet::from([44]));
    assert_eq!(*world.furnace_fuel_item_ids(), BTreeSet::from([43]));

    // A fresh listener (login/configuration re-entry) owns fresh final
    // Scoreboard and pending-recipe state, while CommonListenerCookie
    // carries enabled features and the registry-derived brewing/fuel
    // values persist until the next registry rebuild overwrites them.
    world.clear_client_level();
    assert!(world.scoreboard.objectives.is_empty());
    assert!(world.recipes().property_sets.is_empty());
    assert!(world.is_feature_enabled("minecraft:vanilla"));
    assert_eq!(*world.items.brewing_potion_item_ids, BTreeSet::from([44]));
    assert_eq!(*world.furnace_fuel_item_ids(), BTreeSet::from([43]));
}

#[test]
fn horizon_height_and_void_darkness_range_follow_the_flat_level_flag() {
    let mut world = WorldStore::new();
    // Without a level the non-flat defaults apply.
    assert_eq!(world.client_horizon_height(), 63.0);
    assert_eq!(world.client_void_darkness_onset_range(), 32.0);

    world.apply_login(&login_packet(0, "minecraft:overworld"));
    assert_eq!(world.client_horizon_height(), 63.0);
    assert_eq!(world.client_void_darkness_onset_range(), 32.0);

    let mut flat_login = login_packet(0, "minecraft:overworld");
    flat_login.common_spawn_info.is_flat = true;
    world.apply_login(&flat_login);
    assert_eq!(
        world.client_horizon_height(),
        f64::from(world.dimension().min_y)
    );
    assert_eq!(world.client_void_darkness_onset_range(), 1.0);
}

#[test]
fn dark_disc_gate_compares_the_player_eye_against_the_horizon_height() {
    let mut world = WorldStore::new();
    world.apply_login(&login_packet(0, "minecraft:overworld"));
    // No player pose keeps the disc hidden.
    assert!(!world.local_player_eye_below_horizon());

    let mut pose = LocalPlayerPoseState::default();
    pose.position.y = 61.0;
    world.local_player.pose = Some(pose);
    // Standing eye height 1.62 puts the eye at 62.62, under the 63.0
    // overworld horizon.
    assert!(world.local_player_eye_below_horizon());

    pose.position.y = 61.5;
    world.local_player.pose = Some(pose);
    assert!(!world.local_player_eye_below_horizon());
}

#[test]
fn dimension_respawn_resets_weather_and_border_while_preserving_portal_intensity() {
    let mut world = WorldStore::new();
    world.apply_login(&login_packet(0, "minecraft:overworld"));
    world.weather.raining = true;
    world.weather.rain_level = 1.0;
    world.weather.thunder_level = 1.0;
    world.sky_flash_time = 7;
    world.world_border.size = 128.0;
    world.local_player.portal_effect_intensity = 0.85;
    world.local_player.previous_portal_effect_intensity = 0.75;

    let mut nether_respawn = respawn_packet(0);
    nether_respawn.common_spawn_info.dimension = "minecraft:the_nether".to_string();
    world.apply_respawn(&nether_respawn);

    assert!(!world.weather.raining);
    assert_eq!(world.weather.rain_level, 0.0);
    assert_eq!(world.weather.thunder_level, 0.0);
    assert_eq!(world.sky_flash_time, 0);
    assert_eq!(world.world_border, crate::WorldBorderState::default());
    assert_eq!(world.local_player.portal_effect_intensity, 0.85);
    assert_eq!(world.local_player.previous_portal_effect_intensity, 0.75);
}
