use super::*;

#[test]
fn synchronized_codec_lookup_never_accepts_static_candidate_elements() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:sound_event",
        0,
        vec![RegistryPacketEntry::stub("example:custom_sound")],
    );
    store.record_registry_entries(
        "minecraft:block_entity_type",
        0,
        vec![RegistryPacketEntry::stub("example:custom_block_entity")],
    );

    let access = store.frozen_registry_access();
    assert!(access.contains_element("minecraft:sound_event", "minecraft:ambient.cave"));
    assert!(!access.contains_element("minecraft:sound_event", "example:custom_sound"));
    assert!(!access.contains_element("minecraft:block_entity_type", "example:custom_block_entity"));
}

#[test]
fn new_server_session_drops_active_and_pending_remote_registry_state() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:item",
        0,
        vec![RegistryPacketEntry::stub("example:old_server_item")],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:old_server_tag".to_string(),
                entries: vec![0],
            }],
        }],
    });
    store.begin_registry_configuration();
    store.record_registry_data(RegistryData {
        registry: "minecraft:item".to_string(),
        entries: vec![RegistryDataEntry {
            id: "example:pending_item".to_string(),
            raw_data: None,
        }],
        raw_payload_len: 0,
    });

    store.reset_registry_session();

    assert!(store
        .resolved_registry_entry_by_resource_id("minecraft:item", "example:old_server_item")
        .is_none());
    assert!(store
        .resolved_registry_entry_by_resource_id("minecraft:item", "example:pending_item")
        .is_none());
    assert!(store.registry_tags("minecraft:item").is_none());
    assert_eq!(
        store.registry_resource_id_by_holder_id(
            "minecraft:item",
            bbb_protocol::item_types::VANILLA_ITEM_SNOWBALL_ID,
        ),
        Some("minecraft:snowball")
    );
    assert_eq!(store.finish_registry_configuration(), Ok(()));
    assert_eq!(store.counters().registries_seen, 0);
    assert_eq!(store.counters().registry_content_registries_tracked, 0);
    assert_eq!(store.counters().tag_registries_tracked, 0);
    assert_eq!(store.counters().last_registry_data_registry, None);
}

#[test]
fn configuration_registry_validation_rejects_empty_non_empty_registry_atomically() {
    let registry = "minecraft:wolf_variant";
    let mut store = WorldStore::new();
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::stub("minecraft:pale")],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: registry.to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:old".to_string(),
                entries: vec![0],
            }],
        }],
    });

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, Some(registry));
    store.record_registry_data(RegistryData {
        registry: registry.to_string(),
        entries: Vec::new(),
        raw_payload_len: 0,
    });
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: registry.to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:new".to_string(),
                entries: Vec::new(),
            }],
        }],
    });

    let err = store.finish_registry_configuration().unwrap_err();
    assert_eq!(err.registry, registry);
    assert_eq!(err.validator, "RegistryValidator.nonEmpty ()");
    assert_eq!(
        err.message,
        "Registry must be non-empty: minecraft:wolf_variant"
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id(registry, 0),
        Some("minecraft:pale")
    );
    assert!(store
        .registry_tags(registry)
        .unwrap()
        .tags
        .contains_key("minecraft:old"));
    assert!(!store
        .registry_tags(registry)
        .unwrap()
        .tags
        .contains_key("minecraft:new"));
    assert_eq!(
        store
            .apply_diagnostics()
            .last_apply_error
            .as_ref()
            .map(|error| (error.source.as_str(), error.message.as_str())),
        Some((
            "registry_configuration_validation",
            "Registry must be non-empty: minecraft:wolf_variant",
        ))
    );

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, Some(registry));
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:custom",
            minimal_client_variant_json(registry)
                .unwrap()
                .as_bytes()
                .to_vec(),
        )],
    );
    store.finish_registry_configuration().unwrap();
    assert_eq!(
        store.registry_resource_id_by_holder_id(registry, 0),
        Some("example:custom")
    );
}

#[test]
fn configuration_registry_validation_rejects_duplicate_keys_atomically() {
    let registry = "minecraft:damage_type";
    let mut store = WorldStore::new();
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::stub("minecraft:old")],
    );
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        registry,
        0,
        vec![
            RegistryPacketEntry::stub("example:duplicate"),
            RegistryPacketEntry::stub("example:duplicate"),
        ],
    );

    let err = store.finish_registry_configuration().unwrap_err();
    assert_eq!(err.registry, registry);
    assert_eq!(err.validator, "MappedRegistry.register");
    assert!(err.message.contains("example:duplicate"));
    assert_eq!(
        store.registry_resource_id_by_holder_id(registry, 0),
        Some("minecraft:old")
    );
    assert_eq!(store.counters().registry_duplicate_entries, 0);
}

#[test]
fn configuration_tags_resolve_after_all_registry_packets_arrive() {
    let registry = "minecraft:damage_type";
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: registry.to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:ordered".to_string(),
                entries: vec![1, 7, 0],
            }],
        }],
    });
    store.record_registry_entries(
        registry,
        0,
        vec![
            RegistryPacketEntry::with_raw_data("example:first", damage_type_nbt("first")),
            RegistryPacketEntry::with_raw_data("example:second", damage_type_nbt("second")),
        ],
    );

    store.finish_registry_configuration().unwrap();

    assert_eq!(
        store
            .registry_resource_ids_for_tag(registry, "example:ordered")
            .unwrap(),
        Some(vec!["example:second", "example:first"])
    );
    assert_eq!(
        store.registry_unresolved_holder_ids_for_tag(registry, "example:ordered"),
        Some([7].as_slice())
    );
    assert_eq!(store.counters().tag_entries_tracked, 2);
    assert_eq!(store.counters().last_update_tags_total_value_count, 3);
}

#[test]
fn configuration_candidate_resolves_synchronized_and_static_tags_against_their_layers() {
    let damage_registry = "minecraft:damage_type";
    let item_registry = "minecraft:item";
    let snowball = bbb_protocol::item_types::VANILLA_ITEM_SNOWBALL_ID;
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        damage_registry,
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:damage",
            damage_type_nbt("example.damage"),
        )],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![
            RegistryTags {
                registry: damage_registry.to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "example:synchronized".to_string(),
                    entries: vec![0, 1],
                }],
            },
            RegistryTags {
                registry: item_registry.to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "example:static".to_string(),
                    entries: vec![snowball, i32::MAX],
                }],
            },
        ],
    });

    store.finish_registry_configuration().unwrap();

    assert_eq!(
        store
            .registry_resource_ids_for_tag(damage_registry, "example:synchronized")
            .unwrap(),
        Some(vec!["example:damage"])
    );
    assert_eq!(
        store.registry_unresolved_holder_ids_for_tag(damage_registry, "example:synchronized"),
        Some([1].as_slice())
    );
    assert_eq!(
        store
            .registry_resource_ids_for_tag(item_registry, "example:static")
            .unwrap(),
        Some(vec!["minecraft:snowball"])
    );
    assert_eq!(
        store.registry_unresolved_holder_ids_for_tag(item_registry, "example:static"),
        Some([i32::MAX].as_slice())
    );
}

#[test]
fn configuration_registry_validation_allows_empty_registry_without_validator() {
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_data(RegistryData {
        registry: "minecraft:world_clock".to_string(),
        entries: Vec::new(),
        raw_payload_len: 0,
    });

    store.finish_registry_configuration().unwrap();
    assert_eq!(
        store
            .registry_resource_ids_in_holder_order("minecraft:world_clock")
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        store.registries().contents.len(),
        bbb_protocol::registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES.len()
    );
    for data_type in bbb_protocol::registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
        assert!(
            store.registry_content(data_type.resource_id).is_some(),
            "missing authoritative synchronized registry {}",
            data_type.resource_id
        );
    }
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:world_clock", 0),
        None,
        "an authoritative empty synchronized registry must not use the bootstrap fallback"
    );
}

#[test]
fn any_contents_batch_requires_every_declared_non_empty_registry() {
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    store.record_registry_entries(
        "minecraft:damage_type",
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:damage",
            nbt_compound_with_string("message_id", "example.damage"),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    let data_type =
        bbb_protocol::registry_data_types::synchronized_registry_data_type(&error.registry)
            .expect("failure belongs to the generated synchronized inventory");
    assert_eq!(data_type.validator, Some("RegistryValidator.nonEmpty ()"));
    assert_eq!(error.validator, "RegistryValidator.nonEmpty ()");
    assert_eq!(store.registry_access_revision(), Default::default());
    assert!(store.registry_content("minecraft:damage_type").is_none());
    // The rejected batch leaves the loader-order fallback in place, whose
    // holder 0 is the alphabetically first damage type.
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:damage_type", 0),
        Some("minecraft:arrow")
    );
}

#[test]
fn failed_registry_candidate_keeps_static_tags_and_tag_derived_item_state() {
    let item_id = |resource_id| {
        bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)
            .unwrap()
            .id
    };
    let stick = item_id("minecraft:stick");
    let oak_log = item_id("minecraft:oak_log");
    let birch_log = item_id("minecraft:birch_log");
    let mut store = WorldStore::new();
    store.set_furnace_fuel_profile(
        BTreeSet::new(),
        vec![
            FurnaceFuelRule::AddItem(stick),
            FurnaceFuelRule::AddTag("minecraft:logs".to_string()),
        ],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:logs".to_string(),
                entries: vec![oak_log],
            }],
        }],
    });

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, Some("minecraft:wolf_variant"));
    store.record_registry("minecraft:wolf_variant", 0);
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:logs".to_string(),
                entries: vec![birch_log],
            }],
        }],
    });

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:wolf_variant");
    assert_eq!(
        store.registry_tags("minecraft:item").unwrap().tags["minecraft:logs"],
        vec![oak_log]
    );
    assert_eq!(
        store.furnace_fuel_item_ids(),
        &BTreeSet::from([stick, oak_log])
    );
}

#[test]
fn selected_known_pack_materializes_stub_data_before_registry_commit() {
    let manager = test_known_packs_manager("materialize");
    let core = core_known_pack();
    let data_source = manager
        .data_source_for(std::slice::from_ref(&core))
        .unwrap();
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.apply_select_known_packs_with_data(vec![core.clone()], vec![core], data_source);
    store.record_registry_data(RegistryData {
        registry: "minecraft:damage_type".to_string(),
        entries: vec![RegistryDataEntry {
            id: "minecraft:base".to_string(),
            raw_data: None,
        }],
        raw_payload_len: 0,
    });

    store.finish_registry_configuration().unwrap();

    let entry = store
        .registry_entry_by_holder_id("minecraft:damage_type", 0)
        .unwrap();
    assert!(!entry.has_data);
    assert_eq!(
        entry.known_pack_data(),
        Some(r#"{"message_id":"base","scaling":"never","exhaustion":0.0}"#.as_bytes())
    );
    assert_eq!(
        entry.element_data().unwrap().source,
        RegistryElementSource::ClientKnownPackJson
    );
    assert_eq!(
        store
            .frozen_registry_access()
            .entry_by_holder_id("minecraft:damage_type", 0)
            .unwrap()
            .payload,
        RegistryEntryPayload::Server {
            has_data: false,
            raw_data: None,
            nbt: None,
            known_pack_data: entry.known_pack_data(),
        }
    );
}

#[test]
fn omitted_registry_data_without_a_selected_known_pack_fails_closed() {
    let mut store = WorldStore::new();
    let committed_revision = store.registry_access_revision();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_data(RegistryData {
        registry: "minecraft:damage_type".to_string(),
        entries: vec![RegistryDataEntry {
            id: "minecraft:base".to_string(),
            raw_data: None,
        }],
        raw_payload_len: 0,
    });

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:damage_type");
    assert_eq!(error.validator, "RegistryDataLoader.elementCodec");
    assert!(error
        .message
        .contains("minecraft:base typed payload unavailable"));
    assert_eq!(store.registry_access_revision(), committed_revision);
}

#[test]
fn malformed_network_registry_element_rejects_configuration_atomically() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:damage_type",
        0,
        vec![RegistryPacketEntry::stub("minecraft:old")],
    );
    store.begin_registry_configuration();
    store.record_registry_entries(
        "minecraft:damage_type",
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:invalid",
            vec![0],
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:damage_type");
    assert_eq!(error.validator, "RegistryDataLoader.elementCodec");
    assert!(error.message.contains("root must not be TAG_End"));
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:damage_type", 0),
        Some("minecraft:old")
    );
}

#[test]
fn registered_typed_codec_failure_rejects_candidate_atomically() {
    let registry = "minecraft:world_clock";
    let mut store = WorldStore::new();
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::stub("minecraft:day")],
    );
    let active_revision = store.registry_access_revision();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:not_a_clock",
            b"[]".to_vec(),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, registry);
    assert_eq!(error.validator, "WorldClock.DIRECT_CODEC");
    assert_eq!(
        error.message,
        "example:not_a_clock <root>: expected a compound"
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id(registry, 0),
        Some("minecraft:day")
    );
    assert_eq!(store.registry_access_revision(), active_revision);
    assert_eq!(
        store
            .apply_diagnostics()
            .last_apply_error
            .as_ref()
            .map(|error| error.source.as_str()),
        Some("registry_configuration_exact_codecs")
    );
}

#[test]
fn source_aware_direct_leaf_candidates_preserve_json_semantics() {
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:damage_type",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:rounding",
            br#"{
                "message_id":"custom.rounding",
                "scaling":"never",
                "exhaustion":1.0000000596046447753906250000000000001,
                "effects":null,
                "unknown":{"nested":null}
            }"#
            .to_vec(),
        )],
    );
    store.record_registry_entries(
        "minecraft:banner_pattern",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:banner",
            br#"{
                "asset_id":"example:banner",
                "translation_key":"block.example.banner",
                "unknown":{"nested":null}
            }"#
            .to_vec(),
        )],
    );
    store.record_registry_entries(
        "minecraft:world_clock",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:clock",
            br#"{"unknown":{"nested":null}}"#.to_vec(),
        )],
    );

    store.finish_registry_configuration().unwrap();

    assert_eq!(
        store.damage_type_effects_by_holder_id(0),
        Some(crate::DamageEffects::Hurt)
    );
    assert_eq!(
        store
            .synchronized_banner_pattern_presentation(0)
            .unwrap()
            .asset_id,
        "example:banner"
    );
    assert_eq!(
        store
            .frozen_registry_access()
            .entry_by_holder_id("minecraft:world_clock", 0)
            .unwrap()
            .resource_id,
        "example:clock"
    );
    for registry in [
        "minecraft:chicken_variant",
        "minecraft:cow_variant",
        "minecraft:pig_variant",
    ] {
        let presentation = store
            .synchronized_farm_animal_variant_presentation(registry, 0)
            .unwrap();
        assert_eq!(presentation.model, crate::FarmAnimalVariantModel::Normal);
        assert_eq!(
            presentation.adult_texture_path,
            "example:textures/adult.png"
        );
        assert_eq!(presentation.baby_texture_path, "example:textures/baby.png");
    }
    let cat = store.synchronized_cat_variant_presentation(0).unwrap();
    assert_eq!(cat.adult_texture_path, "example:textures/adult.png");
    assert_eq!(cat.baby_texture_path, "example:textures/baby.png");
    assert_eq!(
        store
            .synchronized_frog_variant_presentation(0)
            .unwrap()
            .texture_path,
        "example:textures/variant.png"
    );
    let wolf = store.synchronized_wolf_variant_presentation(0).unwrap();
    assert_eq!(wolf.adult.wild_texture_path, "example:textures/wild.png");
    assert_eq!(wolf.adult.tame_texture_path, "example:textures/tame.png");
    assert_eq!(wolf.adult.angry_texture_path, "example:textures/angry.png");
    assert_eq!(
        wolf.baby.wild_texture_path,
        "example:textures/baby_wild.png"
    );
    assert_eq!(
        wolf.baby.tame_texture_path,
        "example:textures/baby_tame.png"
    );
    assert_eq!(
        wolf.baby.angry_texture_path,
        "example:textures/baby_angry.png"
    );
    let nautilus = store
        .synchronized_zombie_nautilus_variant_presentation(0)
        .unwrap();
    assert_eq!(
        nautilus.model,
        crate::ZombieNautilusVariantModelType::Normal
    );
    assert_eq!(nautilus.texture_path, "example:textures/variant.png");
}

#[test]
fn malformed_dialog_direct_codec_rejects_candidate_atomically() {
    let registry = "minecraft:dialog";
    let mut store = WorldStore::new();
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::stub("example:active")],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: registry.to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:active".to_string(),
                entries: vec![0],
            }],
        }],
    });
    let active_revision = store.registry_access_revision();

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:malformed",
            b"[]".to_vec(),
        )],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: registry.to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:pending".to_string(),
                entries: vec![0],
            }],
        }],
    });

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, registry);
    assert_eq!(error.validator, "Dialog.DIRECT_CODEC");
    assert_eq!(
        error.message,
        "example:malformed <root>: dialog is not a map"
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id(registry, 0),
        Some("example:active")
    );
    assert_eq!(
        store.registry_tags(registry).unwrap().tags,
        BTreeMap::from([("example:active".to_string(), vec![0])])
    );
    assert_eq!(store.registry_access_revision(), active_revision);
    assert_eq!(
        store
            .apply_diagnostics()
            .last_apply_error
            .as_ref()
            .map(|error| error.source.as_str()),
        Some("registry_configuration_exact_codecs")
    );
}

#[test]
fn minimal_dialog_direct_codec_commits_with_candidate() {
    let registry = "minecraft:dialog";
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        registry,
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:notice",
            minimal_notice_dialog_json(serde_json::json!("Notice")),
        )],
    );

    store.finish_registry_configuration().unwrap();

    assert_eq!(
        store.registry_resource_id_by_holder_id(registry, 0),
        Some("example:notice")
    );
}

#[test]
fn configuration_candidate_stores_sound_variant_and_test_instance_typed_values() {
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(
        &mut store,
        Some("minecraft:cow_sound_variant"),
    );
    store.record_registry_entries(
        "minecraft:cow_sound_variant",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:custom_cow",
            br#"{
                "ambient_sound":"minecraft:entity.cow.ambient",
                "hurt_sound":{
                    "sound_id":"example:cow_hurt",
                    "range":null,
                    "unknown":{"nested":null}
                },
                "death_sound":{"sound_id":"example:cow_death"},
                "step_sound":"minecraft:entity.cow.step",
                "unknown":{"nested":null}
            }"#
            .to_vec(),
        )],
    );
    store.record_registry_entries(
        "minecraft:test_environment",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:rules",
            br#"{
                "type":"minecraft:game_rules",
                "rules":{
                    "minecraft:keep_inventory":true,
                    "minecraft:max_snow_accumulation_height":8
                }
            }"#
            .to_vec(),
        )],
    );
    store.record_registry_entries(
        "minecraft:test_instance",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:candidate_instance",
            br#"{
                "type":"minecraft:block_based",
                "environment":"example:rules",
                "structure":"example:structure",
                "max_ticks":40,
                "required":false,
                "rotation":"clockwise_90",
                "padding":16
            }"#
            .to_vec(),
        )],
    );
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:candidate_enchantment",
            serde_json::to_vec(&complete_enchantment_json(
                "Candidate",
                serde_json::json!(["minecraft:dirt", "minecraft:stone"]),
                serde_json::json!({}),
            ))
            .unwrap(),
        )],
    );

    store.finish_registry_configuration().unwrap();

    let Some(bbb_registry_codec::registry_codec::sound_variants::SoundEventHolder::Inline(sound)) =
        store.synchronized_sound_variant_sound(
            "minecraft:cow_sound_variant",
            0,
            true,
            super::super::RegistrySoundKind::Hurt,
        )
    else {
        panic!("expected inline candidate hurt sound");
    };
    assert_eq!(sound.sound_id.as_str(), "example:cow_hurt");
    assert_eq!(sound.range, None);
    assert!(store
        .registries
        .typed
        .test_instances
        .contains_key("example:candidate_instance"));
    let block_data = crate::TestInstanceBlockEntityDataState {
        test: Some("example:candidate_instance".to_string()),
        ..Default::default()
    };
    assert_eq!(store.test_instance_padding(&block_data), 16);
    assert!(!store.test_instance_is_required(&block_data));
    assert_eq!(
        store.test_instance_rotation(&block_data),
        bbb_protocol::packets::TestInstanceBlockRotation::Clockwise90
    );
    assert!(store
        .registries
        .typed
        .enchantments
        .contains_key("example:candidate_enchantment"));
    assert_eq!(
        store
            .enchantment_tooltip_definitions_in_holder_order()
            .unwrap()[0]
            .as_ref()
            .map(|definition| (definition.description.as_str(), definition.max_level)),
        Some(("Candidate", 2))
    );
}

#[test]
fn enchantment_projection_consumes_exact_known_pack_and_last_wins_network_values() {
    let mut known_pack = complete_enchantment_json(
        "Known pack",
        serde_json::json!("minecraft:stone"),
        serde_json::json!({
            "minecraft:damage": [{
                "effect": { "type": "minecraft:add", "value": 1.0 },
                "requirements": {
                    "condition": "minecraft:entity_properties",
                    "entity": "this",
                    "predicate": {
                        "components": {
                            "minecraft:custom_data": {
                                "type": "minecraft:apply_mob_effect",
                            },
                        },
                    },
                },
            }],
        }),
    );
    known_pack["max_level"] = serde_json::json!(2.0);

    let mut network = complete_enchantment_json(
        "Network",
        serde_json::json!("minecraft:stone"),
        serde_json::json!({}),
    );
    network["max_level"] = serde_json::json!(3);
    let mut network = bbb_protocol::json_value_to_nbt_summary(&network).unwrap();
    let NbtSummaryValue::Compound(fields) = &mut network else {
        panic!("complete enchantment fixture must encode as a compound");
    };
    let max_level_index = fields
        .iter()
        .position(|entry| entry.name == "max_level")
        .unwrap();
    fields.insert(
        max_level_index,
        NbtSummaryEntry {
            name: "max_level".into(),
            value: NbtSummaryValue::Int(1),
        },
    );

    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:known_pack",
                serde_json::to_vec(&known_pack).unwrap(),
            ),
            RegistryPacketEntry::with_raw_data(
                "example:network",
                bbb_protocol::encode_nbt_summary(&network, 1 << 20).unwrap(),
            ),
        ],
    );

    store.finish_registry_configuration().unwrap();

    let tooltips = store
        .enchantment_tooltip_definitions_in_holder_order()
        .unwrap();
    assert_eq!(
        tooltips[0]
            .as_ref()
            .map(|definition| (definition.description.as_str(), definition.max_level)),
        Some(("Known pack", 2))
    );
    assert_eq!(
        tooltips[1]
            .as_ref()
            .map(|definition| (definition.description.as_str(), definition.max_level)),
        Some(("Network", 3)),
        "network compounds must retain Vanilla's duplicate-key last-wins semantics"
    );
}

#[test]
fn enchantment_projection_preserves_clamped_values_with_nan_bounds() {
    let mut network = bbb_protocol::json_value_to_nbt_summary(&complete_enchantment_json(
        "Clamped bounds",
        serde_json::json!("minecraft:stone"),
        serde_json::json!({}),
    ))
    .unwrap();
    let NbtSummaryValue::Compound(fields) = &mut network else {
        unreachable!();
    };
    let field = |name: &str, value| NbtSummaryEntry {
        name: name.into(),
        value,
    };
    let float = |value: f32| NbtSummaryValue::Float(value.to_bits());
    let effects = [
        (f32::NAN, 4.0, 3.0),
        (0.0, f32::NAN, 3.0),
        (0.0, f32::NAN, -1.0),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (min, max, value))| {
        NbtSummaryValue::Compound(vec![
            field(
                "id",
                NbtSummaryValue::string(format!("example:clamped_{index}")),
            ),
            field(
                "attribute",
                NbtSummaryValue::string("minecraft:mining_efficiency"),
            ),
            field("operation", NbtSummaryValue::string("add_value")),
            field(
                "amount",
                NbtSummaryValue::Compound(vec![
                    field("type", NbtSummaryValue::string("minecraft:clamped")),
                    field("value", float(value)),
                    field("min", float(min)),
                    field("max", float(max)),
                ]),
            ),
        ])
    })
    .collect();
    fields
        .iter_mut()
        .find(|entry| entry.name == "effects")
        .unwrap()
        .value = NbtSummaryValue::Compound(vec![field(
        "minecraft:attributes",
        NbtSummaryValue::List(effects),
    )]);

    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:clamped",
            bbb_protocol::encode_nbt_summary(&network, 1 << 20).unwrap(),
        )],
    );
    store.finish_registry_configuration().unwrap();

    let modifiers =
        store.enchantment_attribute_modifiers(&[bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: 0,
            level: 1,
        }]);
    assert_eq!(modifiers.len(), 3);
    assert_eq!(modifiers[0].amount, 3.0);
    assert!(modifiers[1].amount.is_nan());
    assert_eq!(modifiers[2].amount, 0.0);
}

#[test]
fn unknown_instrument_and_jukebox_sounds_reject_candidates_atomically() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:instrument",
        0,
        vec![RegistryPacketEntry::stub("example:old_instrument")],
    );
    store.record_registry_entries(
        "minecraft:jukebox_song",
        0,
        vec![RegistryPacketEntry::stub("example:old_song")],
    );
    let committed_revision = store.registry_access_revision();

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:instrument",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_instrument",
            br#"{
                "sound_event":"example:missing_sound",
                "use_duration":1.0,
                "range":16.0,
                "description":"Invalid instrument"
            }"#
            .to_vec(),
        )],
    );
    let error = store.finish_registry_configuration().unwrap_err();
    assert_eq!(error.registry, "minecraft:instrument");
    assert_eq!(error.validator, "Instrument.DIRECT_CODEC");
    assert!(error.message.contains("example:missing_sound"), "{error:?}");
    assert!(error.message.contains("<references>"), "{error:?}");
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store
            .apply_diagnostics()
            .last_apply_error
            .as_ref()
            .map(|error| error.source.as_str()),
        Some("registry_configuration_exact_codecs")
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:instrument", 0),
        Some("example:old_instrument")
    );

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:jukebox_song",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_song",
            br#"{
                "sound_event":"example:missing_sound",
                "description":"Invalid song",
                "length_in_seconds":1.0,
                "comparator_output":1
            }"#
            .to_vec(),
        )],
    );
    let error = store.finish_registry_configuration().unwrap_err();
    assert_eq!(error.registry, "minecraft:jukebox_song");
    assert_eq!(error.validator, "JukeboxSong.DIRECT_CODEC");
    assert!(error.message.contains("example:missing_sound"), "{error:?}");
    assert!(error.message.contains("<references>"), "{error:?}");
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:jukebox_song", 0),
        Some("example:old_song")
    );
}

#[test]
fn invalid_game_rule_candidate_keeps_previous_typed_generation() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:test_environment",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:old_weather",
            br#"{"type":"minecraft:weather","weather":"clear"}"#.to_vec(),
        )],
    );
    let committed_revision = store.registry_access_revision();

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:test_environment",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_rules",
            br#"{
                "type":"minecraft:game_rules",
                "rules":{"minecraft:max_snow_accumulation_height":9}
            }"#
            .to_vec(),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:test_environment");
    assert_eq!(error.validator, "TestEnvironmentDefinition.DIRECT_CODEC");
    assert!(error.message.contains("integer outside 0..=8"));
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:test_environment", 0),
        Some("example:old_weather")
    );
}

#[test]
fn game_test_candidate_uses_canonical_numeric_projection_without_source_guard_rejection() {
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:test_environment",
        0,
        vec![
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:rules",
                br#"{
                    "type":"minecraft:game_rules",
                    "rules":{"minecraft:random_tick_speed":3.9}
                }"#
                .to_vec(),
            ),
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:self",
                br#"{"type":"minecraft:all_of","definitions":["example:self"]}"#.to_vec(),
            ),
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:forward",
                br#"{"type":"minecraft:all_of","definitions":["example:mutual"]}"#.to_vec(),
            ),
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:mutual",
                br#"{"type":"minecraft:all_of","definitions":["example:forward"]}"#.to_vec(),
            ),
        ],
    );
    store.record_registry_entries(
        "minecraft:test_instance",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:instance",
            br#"{
                "type":"minecraft:block_based",
                "environment":"example:rules",
                "structure":"example:structure",
                "max_ticks":40.9,
                "required":null
            }"#
            .to_vec(),
        )],
    );

    store.finish_registry_configuration().unwrap();

    let instance = store
        .frozen_registry_access()
        .typed_test_instance("example:instance")
        .unwrap();
    assert_eq!(instance.max_ticks, 40);
    assert!(instance.required);
}

#[test]
fn missing_nested_game_test_references_reject_configuration_atomically() {
    let cases: &[(&[u8], &str)] = &[
        (
            br#"{
                "type":"minecraft:all_of",
                "definitions":[{
                    "type":"minecraft:clock_time",
                    "clock":"example:missing_clock",
                    "time":0
                }]
            }"#
            .as_slice(),
            "example:missing_clock",
        ),
        (
            br#"{
                "type":"minecraft:all_of",
                "definitions":[{
                    "type":"minecraft:timeline_attributes",
                    "timelines":["example:missing_timeline"]
                }]
            }"#
            .as_slice(),
            "example:missing_timeline",
        ),
    ];
    for (payload, missing) in cases {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            "minecraft:test_environment",
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:old_weather",
                br#"{"type":"minecraft:weather","weather":"clear"}"#.to_vec(),
            )],
        );
        let committed_revision = store.registry_access_revision();

        store.begin_registry_configuration();
        record_minimal_non_empty_synchronized_registries(&mut store, None);
        store.record_registry_entries(
            "minecraft:test_environment",
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:invalid_reference",
                payload.to_vec(),
            )],
        );

        let error = store.finish_registry_configuration().unwrap_err();
        assert_eq!(error.registry, "minecraft:test_environment");
        assert_eq!(error.validator, "TestEnvironmentDefinition.DIRECT_CODEC");
        assert!(error.message.contains(missing), "{error:?}");
        assert_eq!(store.registry_access_revision(), committed_revision);
        assert_eq!(
            store.registry_resource_id_by_holder_id("minecraft:test_environment", 0),
            Some("example:old_weather")
        );
    }
}

#[test]
fn unknown_sound_variant_reference_rejects_candidate_atomically() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:cow_sound_variant",
        0,
        vec![RegistryPacketEntry::stub("example:old_cow")],
    );
    let committed_revision = store.registry_access_revision();

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(
        &mut store,
        Some("minecraft:cow_sound_variant"),
    );
    store.record_registry_entries(
        "minecraft:cow_sound_variant",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_cow",
            br#"{
                "ambient_sound":"example:missing_sound",
                "hurt_sound":"minecraft:entity.cow.hurt",
                "death_sound":"minecraft:entity.cow.death",
                "step_sound":"minecraft:entity.cow.step"
            }"#
            .to_vec(),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:cow_sound_variant");
    assert_eq!(error.validator, "CowSoundVariant.DIRECT_CODEC");
    assert!(error
        .message
        .contains("missing element example:missing_sound in minecraft:sound_event"));
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:cow_sound_variant", 0),
        Some("example:old_cow")
    );
}

#[test]
fn configuration_candidate_cannot_add_a_static_sound_event_element() {
    let mut store = WorldStore::new();
    let committed_revision = store.registry_access_revision();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(
        &mut store,
        Some("minecraft:cow_sound_variant"),
    );
    store.record_registry_entries(
        "minecraft:sound_event",
        0,
        vec![RegistryPacketEntry::stub("example:custom_sound")],
    );
    store.record_registry_entries(
        "minecraft:cow_sound_variant",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:custom_cow",
            br#"{
                "ambient_sound":"example:custom_sound",
                "hurt_sound":"example:custom_sound",
                "death_sound":"example:custom_sound",
                "step_sound":"example:custom_sound",
                "unknown":{"nested":null}
            }"#
            .to_vec(),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:cow_sound_variant");
    assert_eq!(error.validator, "CowSoundVariant.DIRECT_CODEC");
    assert!(error
        .message
        .contains("missing element example:custom_sound in minecraft:sound_event"));
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert!(store
        .registry_holder_id_by_resource_id("minecraft:sound_event", "example:custom_sound")
        .is_none());
}

#[test]
fn rich_direct_registry_candidates_use_exact_values_and_trusted_projection() {
    const URI: &str = "http://[fe80::1%eth0]/";

    let component = |text: &str| {
        serde_json::json!({
            "text": text,
            "click_event": {"action": "open_url", "url": URI}
        })
    };
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(
        &mut store,
        Some("minecraft:painting_variant"),
    );
    store.record_registry_entries(
        "minecraft:chat_type",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:scoped_chat",
            serde_json::to_vec(&serde_json::json!({
                "chat": {
                    "translation_key": "example.chat",
                    "parameters": ["sender", "content"],
                    "style": {"click_event": {"action": "open_url", "url": URI}}
                },
                "narration": {
                    "translation_key": "example.narration",
                    "parameters": []
                },
                "ignored": null
            }))
            .unwrap(),
        )],
    );
    store.record_registry_entries(
        "minecraft:trim_material",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:quartz",
            serde_json::to_vec(&serde_json::json!({
                "asset_name": "quartz",
                "description": component("Quartz")
            }))
            .unwrap(),
        )],
    );
    store.record_registry_entries(
        "minecraft:trim_pattern",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:spire",
            serde_json::to_vec(&serde_json::json!({
                "asset_id": "spire",
                "description": component("Spire"),
                "decal": "malformed-lenient-optional"
            }))
            .unwrap(),
        )],
    );
    store.record_registry_entries(
        "minecraft:painting_variant",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:portrait",
            serde_json::to_vec(&serde_json::json!({
                "width": 2,
                "height": 3,
                "asset_id": "portrait",
                "title": component("Portrait"),
                "author": component("Artist")
            }))
            .unwrap(),
        )],
    );
    store.record_registry_entries(
        "minecraft:instrument",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:horn",
            serde_json::to_vec(&serde_json::json!({
                "sound_event": {"sound_id": "example:inline_horn"},
                "use_duration": 2.5,
                "range": 32.0,
                "description": component("Horn")
            }))
            .unwrap(),
        )],
    );
    store.record_registry_entries(
        "minecraft:jukebox_song",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:song",
            serde_json::to_vec(&serde_json::json!({
                "sound_event": {"sound_id": "example:inline_song"},
                "description": component("Song"),
                "length_in_seconds": 12.5,
                "comparator_output": 7
            }))
            .unwrap(),
        )],
    );

    store.finish_registry_configuration().unwrap();

    let expected_click = ComponentClickEvent::OpenUrl { url: URI.into() };
    let chat = super::super::chat_types::resolve_chat_type(
        store.frozen_registry_access(),
        &ChatTypeHolder::Registry { id: 0 },
    )
    .unwrap();
    assert_eq!(chat.resource_id.as_deref(), Some("example:scoped_chat"));
    assert_eq!(chat.chat.style.click_event.as_ref(), Some(&expected_click));
    assert!(chat.narration.style.click_event.is_none());

    let material = store.synchronized_trim_material_presentation(0).unwrap();
    assert_eq!(material.asset_name, "quartz");
    assert!(material.override_armor_assets.is_empty());
    assert_eq!(
        material.description_styled[0].style.click_event.as_ref(),
        Some(&expected_click)
    );

    let pattern = store.synchronized_trim_pattern_presentation(0).unwrap();
    assert_eq!(pattern.asset_id, "minecraft:spire");
    assert!(!pattern.decal);
    assert_eq!(
        pattern.description_styled[0].style.click_event.as_ref(),
        Some(&expected_click)
    );

    let painting = store.painting_variant_presentation(0).unwrap();
    assert_eq!((painting.width, painting.height), (2, 3));
    assert_eq!(painting.asset_id, "minecraft:portrait");
    assert_eq!(
        painting.title_styled[0].style.click_event.as_ref(),
        Some(&expected_click)
    );
    assert_eq!(
        painting.author_styled[0].style.click_event.as_ref(),
        Some(&expected_click)
    );

    let instrument = store.synchronized_instrument_presentation(0).unwrap();
    assert_eq!(instrument.sound_event.resource_id(), "example:inline_horn");
    assert_eq!((instrument.use_duration, instrument.range), (2.5, 32.0));
    assert_eq!(
        instrument.description_styled[0].style.click_event.as_ref(),
        Some(&expected_click)
    );

    let song = store.synchronized_jukebox_song_presentation(0).unwrap();
    assert_eq!(song.sound_event.resource_id(), "example:inline_song");
    assert_eq!((song.length_in_seconds, song.comparator_output), (12.5, 7));
    assert_eq!(
        song.description_styled[0].style.click_event.as_ref(),
        Some(&expected_click)
    );
}

#[test]
fn network_timeline_candidate_does_not_run_the_persistent_registry_wide_marker_validator() {
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:world_clock",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:clock",
            b"{}".to_vec(),
        )],
    );
    store.record_registry_entries(
        "minecraft:timeline",
        0,
        ["example:first", "example:second"]
            .into_iter()
            .map(|id| {
                RegistryPacketEntry::with_known_pack_data_for_test(
                    id,
                    br#"{
                        "clock":"example:clock",
                        "time_markers":{"example:shared":0}
                    }"#
                    .to_vec(),
                )
            })
            .collect(),
    );

    store.finish_registry_configuration().unwrap();

    for (holder_id, expected) in [(0, "example:first"), (1, "example:second")] {
        assert_eq!(
            store.registry_resource_id_by_holder_id("minecraft:timeline", holder_id),
            Some(expected)
        );
        assert!(store.registries.typed.timelines.contains_key(expected));
    }
}

#[test]
fn environment_position_rejects_non_positional_biome_attribute_atomically() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:worldgen/biome",
        0,
        vec![RegistryPacketEntry::stub("example:old_biome")],
    );
    let committed_revision = store.registry_access_revision();

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:worldgen/biome",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_biome",
            br##"{
                "has_precipitation":true,
                "temperature":0.5,
                "downfall":0.5,
                "effects":{"water_color":"#3f76e4"},
                "attributes":{"minecraft:gameplay/sky_light_level":12.0}
            }"##
            .to_vec(),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:worldgen/biome");
    assert_eq!(error.validator, "Biome.NETWORK_CODEC");
    assert!(error.message.contains("attribute is not positional"));
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:worldgen/biome", 0),
        Some("example:old_biome")
    );
}

#[test]
fn environment_position_rejects_malformed_biome_sound_atomically() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:worldgen/biome",
        0,
        vec![RegistryPacketEntry::stub("example:old_biome")],
    );
    let committed_revision = store.registry_access_revision();

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:worldgen/biome",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_biome",
            br##"{
                "has_precipitation":true,
                "temperature":0.5,
                "downfall":0.5,
                "effects":{"water_color":"#3f76e4"},
                "attributes":{
                    "minecraft:audio/background_music":{
                        "default":{
                            "sound":"example:missing_music",
                            "min_delay":10,
                            "max_delay":20
                        }
                    }
                }
            }"##
            .to_vec(),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:worldgen/biome");
    assert_eq!(error.validator, "Biome.NETWORK_CODEC");
    assert!(
        error
            .message
            .contains("missing element example:missing_music in minecraft:sound_event"),
        "{error:?}"
    );
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:worldgen/biome", 0),
        Some("example:old_biome")
    );
}

#[test]
fn unknown_environment_sound_reference_rejects_candidate_atomically() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:dimension_type",
        0,
        vec![RegistryPacketEntry::stub("example:old_dimension")],
    );
    let committed_revision = store.registry_access_revision();

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:dimension_type",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_dimension",
            dimension_type_json(
                r#",
                "attributes":{
                    "minecraft:audio/background_music":{
                        "default":{
                            "sound":"example:missing_sound",
                            "min_delay":10,
                            "max_delay":20
                        }
                    }
                }
            "#,
            ),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:dimension_type");
    assert_eq!(error.validator, "DimensionType.NETWORK_CODEC");
    assert!(
        error
            .message
            .contains("missing element example:missing_sound in minecraft:sound_event"),
        "{error:?}"
    );
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:dimension_type", 0),
        Some("example:old_dimension")
    );
}

#[test]
fn malformed_dimension_hashed_tag_rejects_candidate_atomically() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:dimension_type",
        0,
        vec![RegistryPacketEntry::stub("example:old_dimension")],
    );
    let committed_revision = store.registry_access_revision();

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:dimension_type",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_dimension",
            dimension_type_json_with_infiniburn("example:not_hashed", ""),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:dimension_type");
    assert_eq!(error.validator, "DimensionType.NETWORK_CODEC");
    assert!(error.message.contains("infiniburn"), "{error:?}");
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:dimension_type", 0),
        Some("example:old_dimension")
    );
}

#[test]
fn malformed_dimension_monster_provider_rejects_candidate_atomically() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:dimension_type",
        0,
        vec![RegistryPacketEntry::stub("example:old_dimension")],
    );
    let committed_revision = store.registry_access_revision();

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    let invalid = String::from_utf8(dimension_type_json("")).unwrap().replace(
        "\"monster_spawn_light_level\":0",
        "\"monster_spawn_light_level\":{\"type\":\"uniform\",\"min_inclusive\":0,\"max_inclusive\":16}",
    );
    store.record_registry_entries(
        "minecraft:dimension_type",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_dimension",
            invalid.into_bytes(),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:dimension_type");
    assert_eq!(error.validator, "DimensionType.NETWORK_CODEC");
    assert!(
        error.message.contains("monster_spawn_light_level") && error.message.contains("too high"),
        "{error:?}"
    );
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:dimension_type", 0),
        Some("example:old_dimension")
    );
}

#[test]
fn selected_known_pack_list_must_match_its_frozen_data_source() {
    let manager = test_known_packs_manager("selection-mismatch");
    let core = core_known_pack();
    let data_source = manager
        .data_source_for(std::slice::from_ref(&core))
        .unwrap();
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
    store.apply_select_known_packs_with_data(vec![core], Vec::new(), data_source);

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:root");
    assert_eq!(
        error.message,
        "selected known packs do not match the frozen SERVER_DATA view"
    );
}
