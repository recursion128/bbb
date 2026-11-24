use super::*;

#[test]
fn login_warns_once_when_profile_keys_cannot_be_validated() {
    let mut store = WorldStore::new();

    store.apply_login(&login_packet(0, "minecraft:overworld"));

    assert_eq!(store.server_enforces_secure_chat(), Some(true));
    assert!(!store.profile_key_validation_available());
    assert!(!store.effectively_enforces_secure_chat());
    assert!(store.seen_insecure_chat_warning());
    assert_eq!(store.insecure_chat_warning_count(), 1);
}

#[test]
fn dimension_timeline_layers_follow_session_tag_order() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        crate::TIMELINE_REGISTRY,
        0,
        vec![
            crate::RegistryPacketEntry::with_known_pack_data_for_test(
                "example:custom",
                br#"{"clock":"minecraft:overworld"}"#.to_vec(),
            ),
            crate::RegistryPacketEntry::with_known_pack_data_for_test(
                "minecraft:day",
                br#"{"clock":"minecraft:overworld"}"#.to_vec(),
            ),
        ],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: crate::TIMELINE_REGISTRY.to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:dimension_layers".to_string(),
                entries: vec![1, 0],
            }],
        }],
    });
    record_custom_dimension_type_registry(
        &mut store,
        dimension_type_nbt_with_timelines("#example:dimension_layers"),
    )
    .unwrap();

    store.apply_login(&login_packet(0, "example:custom"));

    assert_eq!(
        store.level_info().unwrap().timeline_resource_ids(),
        ["minecraft:day", "example:custom"]
    );
    assert!(store.apply_diagnostics().apply_errors.is_empty());
}

#[test]
fn the_level_loading_screen_window_and_reason_follow_the_dimension_change() {
    let mut store = WorldStore::new();
    assert!(!store.level_load().screen_is_up());

    store.apply_login(&login_packet(0, "minecraft:overworld"));
    assert!(store.level_load().screen_is_up());
    assert_eq!(store.level_load().reason, WorldLevelLoadReason::Other);

    let event = ProtocolGameEvent {
        event_id: VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
        param: 0.0,
    };
    store.apply_game_event(event);
    assert!(store.level_load().screen_is_up());
    assert!(store.complete_client_level_load(false));
    assert!(!store.level_load().screen_is_up());

    store.apply_respawn(&ProtocolRespawn {
        common_spawn_info: ProtocolSpawnInfo {
            dimension: "minecraft:the_nether".to_string(),
            dimension_type_id: 1,
            ..respawn_packet(0).common_spawn_info
        },
        data_to_keep: 0,
    });
    assert!(store.level_load().screen_is_up());
    assert_eq!(
        store.level_load().reason,
        WorldLevelLoadReason::NetherPortal
    );

    store.apply_game_event(event);
    assert!(store.complete_client_level_load(false));
    store.apply_respawn(&respawn_packet(0));
    assert_eq!(
        store.level_load().reason,
        WorldLevelLoadReason::NetherPortal
    );

    store.apply_game_event(event);
    assert!(store.complete_client_level_load(false));
    store.apply_respawn(&ProtocolRespawn {
        common_spawn_info: ProtocolSpawnInfo {
            dimension: "minecraft:the_end".to_string(),
            dimension_type_id: 2,
            ..respawn_packet(0).common_spawn_info
        },
        data_to_keep: 0,
    });
    assert_eq!(store.level_load().reason, WorldLevelLoadReason::EndPortal);
}

#[test]
fn a_death_respawn_keeps_the_other_level_loading_reason() {
    assert_eq!(
        level_loading_reason(true, "minecraft:the_nether", Some("minecraft:overworld")),
        WorldLevelLoadReason::Other
    );
    assert_eq!(
        level_loading_reason(false, "minecraft:overworld", Some("minecraft:the_nether")),
        WorldLevelLoadReason::NetherPortal
    );
    assert_eq!(
        level_loading_reason(false, "minecraft:overworld", Some("minecraft:the_end")),
        WorldLevelLoadReason::EndPortal
    );
    assert_eq!(
        level_loading_reason(false, "minecraft:overworld", None),
        WorldLevelLoadReason::Other
    );
}

#[test]
fn dimension_type_registry_projects_remaining_network_scalars_to_level_state() {
    let mut store = WorldStore::new();
    record_custom_dimension_type_registry(&mut store, dimension_type_nbt_with_network_scalars())
        .unwrap();

    store.apply_login(&login_packet(0, "example:custom"));

    assert_eq!(
        store.level_info().unwrap().dimension_type_properties,
        WorldDimensionTypeProperties {
            has_fixed_time: true,
            has_ender_dragon_fight: true,
            coordinate_scale: 8.0,
            logical_height: 256,
            infiniburn_tag: Some("example:forever".to_string()),
            monster_spawn_light_level: Some(WorldIntProvider::Constant(7)),
            monster_spawn_block_light_limit: Some(4),
        }
    );
}

#[test]
fn login_and_respawn_resolve_dimension_type_holders_in_server_order() {
    let mut store = WorldStore::new();
    store.record_registry_data(RegistryData {
        registry: "minecraft:dimension_type".to_string(),
        raw_payload_len: 0,
        entries: vec![
            RegistryDataEntry {
                id: "example:compact".to_string(),
                raw_data: Some(dimension_type_nbt(
                    Some("nether"),
                    Some(false),
                    Some(true),
                    Some((0, 128, 96)),
                    Some("none"),
                    Some(0.2),
                )),
            },
            RegistryDataEntry {
                id: "example:tall".to_string(),
                raw_data: Some(dimension_type_nbt(
                    Some("default"),
                    Some(true),
                    Some(false),
                    Some((-64, 384, 320)),
                    Some("end"),
                    Some(0.4),
                )),
            },
        ],
    });

    store.begin_registry_configuration();
    store.finish_registry_configuration().unwrap();
    store.apply_login(&login_packet(1, "example:first_level"));
    let login_level = store.level_info().unwrap();
    assert_eq!(login_level.dimension_type_id, 1);
    assert_eq!(
        login_level.dimension_type_name.as_deref(),
        Some("example:tall")
    );
    assert_eq!(
        store.dimension(),
        WorldDimension {
            min_y: -64,
            height: 384,
        }
    );
    assert!(login_level.has_sky_light());
    assert_eq!(login_level.skybox(), WorldSkybox::End);
    assert_eq!(login_level.ambient_light(), 0.4);

    store.apply_respawn(&ProtocolRespawn {
        common_spawn_info: ProtocolSpawnInfo {
            dimension_type_id: 0,
            dimension: "example:second_level".to_string(),
            seed: 67890,
            game_type: 0,
            previous_game_type: 0,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        data_to_keep: 3,
    });
    let respawn_level = store.level_info().unwrap();
    assert_eq!(respawn_level.dimension_type_id, 0);
    assert_eq!(
        respawn_level.dimension_type_name.as_deref(),
        Some("example:compact")
    );
    assert_eq!(
        store.dimension(),
        WorldDimension {
            min_y: 0,
            height: 128,
        }
    );
    assert!(!respawn_level.has_sky_light());
    assert!(respawn_level.has_ceiling());
    assert_eq!(
        respawn_level.cardinal_lighting(),
        WorldCardinalLighting::Nether
    );
    assert_eq!(respawn_level.skybox(), WorldSkybox::None);
    assert_eq!(respawn_level.ambient_light(), 0.2);
}

#[test]
fn dimension_type_registry_projects_environment_attributes_and_rejects_invalid_modifiers() {
    let mut custom = WorldStore::new();
    record_custom_dimension_type_registry(
        &mut custom,
        dimension_type_nbt_with_environment_attributes(false),
    )
    .unwrap();
    custom.apply_login(&login_packet(0, "example:custom"));

    let attributes = custom.level_info().unwrap().environment_attributes();
    assert_eq!(attributes.fog_color, Some([0x12, 0x34, 0x56]));
    assert_eq!(attributes.fog_start_distance, Some(12.0));
    assert_eq!(attributes.fog_end_distance, Some(144.0));
    assert_eq!(attributes.water_fog_start_distance, Some(-4.0));
    assert_eq!(attributes.water_fog_end_distance, Some(48.0));
    assert_eq!(attributes.block_light_tint, Some([0x20, 0x40, 0x60]));
    assert_eq!(attributes.sky_light_factor, Some(0.25));
    assert_eq!(attributes.ambient_light_color, Some([0x30, 0x50, 0x70]));
    assert!(custom.apply_diagnostics().apply_errors.is_empty());

    let mut unsupported = WorldStore::new();
    assert!(record_custom_dimension_type_registry(
        &mut unsupported,
        dimension_type_nbt_with_environment_attributes(true),
    )
    .is_err());
}
