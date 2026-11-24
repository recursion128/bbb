use super::*;

#[test]
fn loads_vanilla_block_state_registry() {
    let registries = RegistrySet::vanilla_26_1();
    assert_eq!(registries.block_state_count(), 29873);
    assert_eq!(registries.block_state(0).unwrap().name, "minecraft:air");
    let grass = registries.block_state(9).unwrap();
    assert_eq!(grass.name, "minecraft:grass_block");
    assert_eq!(grass.properties.get("snowy").unwrap(), "false");

    assert_eq!(registries.block_registry_count(), 1168);
    assert_eq!(
        registries.block_name_by_registry_id(0),
        Some("minecraft:air")
    );
    assert_eq!(
        registries.block_name_by_registry_id(1),
        Some("minecraft:stone")
    );
    assert_eq!(
        registries.block_name_by_registry_id(8),
        Some("minecraft:grass_block")
    );
    assert_eq!(
        registries.block_name_by_registry_id(193),
        Some("minecraft:obsidian")
    );

    let chest_state_id = registries
        .block_state_id_by_name_and_properties(
            "minecraft:chest",
            &BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("type".to_string(), "single".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]),
        )
        .unwrap();
    assert_eq!(
        registries.block_entity_type_id_for_block_state(chest_state_id),
        Some(bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID)
    );
    assert_eq!(registries.block_entity_type_id_for_block_state(1), None);
}

#[test]
fn vanilla_block_state_queries_use_the_embedded_report() {
    let blocks = crate::BlockStateRegistry::vanilla_26_1();
    assert!(blocks.block_has_property("minecraft:oak_log", "axis"));
    assert!(blocks.block_has_property("minecraft:lever", "powered"));
    assert!(!blocks.block_has_property("minecraft:stone", "axis"));
    assert!(!blocks.block_has_property("minecraft:missing", "axis"));
    assert_eq!(crate::vanilla_air_block_state_id(), 0);
    assert_eq!(
        crate::vanilla_block_state_id(
            "minecraft:grass_block",
            &BTreeMap::from([("snowy".to_string(), "false".to_string())]),
        ),
        Some(9)
    );
    assert_eq!(
        crate::vanilla_block_state_id("minecraft:missing", &BTreeMap::new()),
        None
    );
}

#[test]
fn registry_data_entries_are_ordered_and_counted() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:chat_type",
        128,
        vec![
            RegistryPacketEntry::with_raw_data("minecraft:chat", vec![10; 24]),
            RegistryPacketEntry::stub("minecraft:raw"),
        ],
    );
    store.record_registry_entries(
        "minecraft:chat_type",
        96,
        vec![RegistryPacketEntry::with_raw_data(
            "minecraft:chat",
            vec![11; 12],
        )],
    );
    store.record_registry_entries(
        "minecraft:damage_type",
        64,
        vec![RegistryPacketEntry::with_raw_data(
            "minecraft:in_fire",
            vec![8; 17],
        )],
    );

    let registries = &store.registries().registries;
    assert_eq!(registries.len(), 3);
    assert_eq!(registries[0].name, "minecraft:chat_type");
    assert!(registries[0].configuration_synchronized);
    assert_eq!(
        registries[0].entries,
        vec![
            RegistryPacketEntry::with_raw_data("minecraft:chat", vec![10; 24]),
            RegistryPacketEntry::stub("minecraft:raw"),
        ]
    );
    assert_eq!(registries[1].entries[0].id, "minecraft:chat");
    assert_eq!(registries[2].entries[0].id, "minecraft:in_fire");
    assert!(registries[2].configuration_synchronized);
    assert_eq!(registries[2].entries[0].raw_data(), Some(&[8; 17][..]));

    let chat_content = store
        .registry_content("minecraft:chat_type")
        .expect("chat_type content is collected");
    assert_eq!(chat_content.packet_count, 2);
    assert_eq!(
        chat_content
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec!["minecraft:chat", "minecraft:raw", "minecraft:chat"]
    );
    assert_eq!(chat_content.duplicate_entry_ids["minecraft:chat"], 1);
    assert_eq!(store.registries().contents.len(), 2);

    let counters = store.counters();
    assert_eq!(counters.registries_seen, 3);
    assert_eq!(counters.registry_entries_seen, 4);
    assert_eq!(counters.registry_entries_with_data, 3);
    assert_eq!(counters.registry_entry_stubs, 1);
    assert_eq!(counters.registry_entry_payload_bytes, 53);
    assert_eq!(counters.registry_content_registries_tracked, 2);
    assert_eq!(counters.registry_content_packets_tracked, 3);
    assert_eq!(counters.registry_content_entries_tracked, 4);
    assert_eq!(counters.registry_duplicate_entries, 1);
    assert_eq!(counters.registry_duplicate_entry_ids_tracked, 1);
    assert_eq!(
        counters.last_registry_data_registry.as_deref(),
        Some("minecraft:damage_type")
    );
    assert_eq!(counters.last_registry_data_entry_count, 1);
}

#[test]
fn item_registry_data_cannot_remap_the_builtin_holder_order() {
    let mut store = WorldStore::new();
    let snowball_id = bbb_protocol::item_types::VANILLA_ITEM_SNOWBALL_ID;

    store.record_registry_data(RegistryData {
        registry: "minecraft:item".to_string(),
        entries: vec![
            RegistryDataEntry {
                id: "minecraft:snowball".to_string(),
                raw_data: None,
            },
            RegistryDataEntry {
                id: "minecraft:air".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 32,
    });

    let packet = &store.registries().registries[0];
    assert!(packet.client_built_in);
    assert!(!packet.configuration_synchronized);
    assert!(store.registry_content("minecraft:item").is_none());
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:item", 0),
        Some("minecraft:air")
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:item", snowball_id),
        Some("minecraft:snowball")
    );
    assert_eq!(
        store.registry_holder_id_by_resource_id("minecraft:item", "minecraft:snowball"),
        Some(snowball_id)
    );
}

#[test]
fn fluid_registry_fallback_preserves_vanilla_holder_order_and_resolves_tags() {
    let mut store = WorldStore::new();
    let expected = [
        "minecraft:empty",
        "minecraft:flowing_water",
        "minecraft:water",
        "minecraft:flowing_lava",
        "minecraft:lava",
    ];

    for (holder_id, resource_id) in expected.into_iter().enumerate() {
        let holder_id = i32::try_from(holder_id).unwrap();
        assert_eq!(
            store.registry_resource_id_by_holder_id("minecraft:fluid", holder_id),
            Some(resource_id)
        );
        assert_eq!(
            store.registry_holder_id_by_resource_id("minecraft:fluid", resource_id),
            Some(holder_id)
        );
    }

    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:fluid".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:supports_lily_pad".to_string(),
                entries: vec![2],
            }],
        }],
    });
    assert_eq!(
        store.registry_holder_ids_for_tag("minecraft:fluid", "minecraft:supports_lily_pad"),
        Some([2].as_slice())
    );
    assert_eq!(
        store.registry_unresolved_holder_ids_for_tag(
            "minecraft:fluid",
            "minecraft:supports_lily_pad"
        ),
        None
    );
}

#[test]
fn mob_effect_registry_data_cannot_remap_the_builtin_holder_order() {
    let mut store = WorldStore::new();
    let poison_id = bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_POISON_ID;

    store.record_registry_data(RegistryData {
        registry: "minecraft:mob_effect".to_string(),
        entries: vec![
            RegistryDataEntry {
                id: "minecraft:poison".to_string(),
                raw_data: None,
            },
            RegistryDataEntry {
                id: "minecraft:speed".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 32,
    });

    let packet = &store.registries().registries[0];
    assert!(packet.client_built_in);
    assert!(!packet.configuration_synchronized);
    assert!(store.registry_content("minecraft:mob_effect").is_none());
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:mob_effect", 0),
        Some("minecraft:speed")
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:mob_effect", poison_id),
        Some("minecraft:poison")
    );
    assert_eq!(
        store.registry_holder_id_by_resource_id("minecraft:mob_effect", "minecraft:poison"),
        Some(poison_id)
    );
}

#[test]
fn attribute_registry_data_cannot_remap_the_builtin_holder_order() {
    let mut store = WorldStore::new();
    let scale_id = bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID;

    store.record_registry_data(RegistryData {
        registry: "minecraft:attribute".to_string(),
        entries: vec![
            RegistryDataEntry {
                id: "minecraft:scale".to_string(),
                raw_data: None,
            },
            RegistryDataEntry {
                id: "minecraft:armor".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 32,
    });

    let packet = &store.registries().registries[0];
    assert!(packet.client_built_in);
    assert!(!packet.configuration_synchronized);
    assert!(store.registry_content("minecraft:attribute").is_none());
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:attribute", 0),
        Some("minecraft:armor")
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:attribute", scale_id),
        Some("minecraft:scale")
    );
    assert_eq!(
        store.registry_holder_id_by_resource_id("minecraft:attribute", "minecraft:scale"),
        Some(scale_id)
    );
}

#[test]
fn potion_registry_data_cannot_remap_the_builtin_holder_order() {
    let mut store = WorldStore::new();
    let infested_id =
        bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:infested")
            .unwrap()
            .id;

    store.record_registry_data(RegistryData {
        registry: "minecraft:potion".to_string(),
        entries: vec![
            RegistryDataEntry {
                id: "minecraft:infested".to_string(),
                raw_data: None,
            },
            RegistryDataEntry {
                id: "minecraft:water".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 32,
    });

    let packet = &store.registries().registries[0];
    assert!(packet.client_built_in);
    assert!(!packet.configuration_synchronized);
    assert!(store.registry_content("minecraft:potion").is_none());
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:potion", 0),
        Some("minecraft:water")
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:potion", infested_id),
        Some("minecraft:infested")
    );
    assert_eq!(
        store.registry_holder_id_by_resource_id("minecraft:potion", "minecraft:infested"),
        Some(infested_id)
    );
}

#[test]
fn frozen_registry_access_exposes_payload_source_and_atomic_revisions() {
    let mut store = WorldStore::new();
    assert_eq!(store.registry_access_revision(), Default::default());
    let typed_raw = damage_type_nbt("example.typed");
    let known_pack_data =
        br#"{"message_id":"example.known","scaling":"never","exhaustion":0}"#.to_vec();

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:damage_type",
        0,
        vec![
            RegistryPacketEntry::with_raw_data("example:typed", typed_raw.clone()),
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:known_pack",
                known_pack_data.clone(),
            ),
        ],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:damage_type".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:test".to_string(),
                entries: vec![1, 4, 0],
            }],
        }],
    });
    assert_eq!(store.registry_access_revision(), Default::default());

    store.finish_registry_configuration().unwrap();

    let access = store.frozen_registry_access();
    assert_eq!(access.revision().session, 0);
    assert_eq!(access.revision().contents, 1);
    assert_eq!(access.revision().tags, 1);
    let typed = access
        .entry_by_resource_id("minecraft:damage_type", "example:typed")
        .unwrap();
    assert_eq!(typed.holder_id, 0);
    assert_eq!(typed.source, RegistryEntrySource::Server);
    assert_eq!(
        typed.payload,
        RegistryEntryPayload::Server {
            has_data: true,
            raw_data: Some(typed_raw.as_slice()),
            nbt: Some(&NbtPayloadSummary {
                root_type: 10,
                byte_len: typed_raw.len(),
            }),
            known_pack_data: None,
        }
    );
    assert_eq!(
        typed.element_data.unwrap().source,
        RegistryElementSource::NetworkNbt
    );
    assert_eq!(
        access
            .entry_by_holder_id("minecraft:damage_type", 1)
            .unwrap()
            .payload,
        RegistryEntryPayload::Server {
            has_data: false,
            raw_data: None,
            nbt: None,
            known_pack_data: Some(known_pack_data.as_slice()),
        }
    );
    assert_eq!(
        access.holder_ids_for_tag("minecraft:damage_type", "example:test"),
        Some([1, 0].as_slice())
    );
    assert_eq!(
        access.unresolved_holder_ids_for_tag("minecraft:damage_type", "example:test"),
        Some([4].as_slice())
    );

    store.reset_registry_session();
    assert_eq!(store.registry_access_revision().session, 1);
    assert_eq!(store.registry_access_revision().contents, 0);
    assert_eq!(store.registry_access_revision().tags, 0);
    assert_eq!(
        store
            .frozen_registry_access()
            .entry_by_resource_id("minecraft:item", "minecraft:snowball")
            .unwrap()
            .payload,
        RegistryEntryPayload::VanillaFallback
    );
}

/// `minecraft:chat_type` is data-loaded (RegistryDataLoader.java:86), so the
/// built-in decorations behind an unsynchronized holder id have to be keyed on
/// the resolved resource id: holder 1 is `minecraft:emote_command` in loader
/// order, while the bootstrap declaration order puts `minecraft:say_command`
/// there.
#[test]
fn vanilla_fallback_chat_type_decoration_follows_loader_order() {
    let store = WorldStore::new();
    let resolved = super::super::chat_types::resolve_chat_type(
        store.frozen_registry_access(),
        &bbb_protocol::packets::ChatTypeHolder::Registry { id: 1 },
    )
    .expect("vanilla chat-type fallback");
    assert_eq!(
        resolved.resource_id.as_deref(),
        Some("minecraft:emote_command")
    );
    assert_eq!(resolved.source, Some(RegistryEntrySource::VanillaFallback));
    assert_eq!(resolved.chat.translation_key, "chat.type.emote");
    assert_eq!(resolved.narration.translation_key, "chat.type.emote");
}

/// Which order the vanilla fallback counts holder ids in depends on how the
/// server builds the registry.
///
/// `minecraft:damage_type` is loaded from `data/minecraft/damage_type/*.json`
/// (RegistryDataLoader.java:115) and registered in sorted `Identifier` order
/// (ResourceManagerRegistryLoadTask.java:62), which is why the official 26.1
/// server reports `minecraft:out_of_world` as source type 32 rather than its
/// bootstrap index 14. `minecraft:mob_effect` is a `BuiltInRegistries` registry
/// (BuiltInRegistries.java:179) and keeps bootstrap order.
#[test]
fn vanilla_fallback_splits_loader_order_from_bootstrap_order() {
    let store = WorldStore::new();
    let access = store.frozen_registry_access();

    assert_eq!(
        access
            .entry_by_holder_id("minecraft:damage_type", 32)
            .map(|entry| entry.resource_id),
        Some("minecraft:out_of_world")
    );
    assert_eq!(
        access
            .entry_by_resource_id("minecraft:damage_type", "minecraft:out_of_world")
            .map(|entry| entry.holder_id),
        Some(32)
    );
    let damage_types = access
        .resource_ids_in_holder_order("minecraft:damage_type")
        .expect("damage-type fallback order");
    let mut sorted = damage_types.clone();
    sorted.sort_unstable();
    assert_eq!(damage_types, sorted);

    assert_eq!(
        access
            .entry_by_holder_id("minecraft:mob_effect", 0)
            .map(|entry| entry.resource_id),
        Some("minecraft:speed")
    );
    assert_eq!(
        access
            .entry_by_resource_id("minecraft:mob_effect", "minecraft:absorption")
            .map(|entry| entry.holder_id),
        Some(21),
        "a built-in registry must not be re-sorted by resource id"
    );
}

#[test]
fn frozen_registry_access_has_exact_fallback_for_every_entity_data_holder_registry() {
    let store = WorldStore::new();
    let access = store.frozen_registry_access();
    // Every registry below is loaded from data files, so holder 0 is the
    // alphabetically first member, not the first bootstrap declaration.
    for (serializer, first_resource_id) in [
        (EntityDataRegistryHolder::CatVariant, "minecraft:all_black"),
        (
            EntityDataRegistryHolder::CatSoundVariant,
            "minecraft:classic",
        ),
        (EntityDataRegistryHolder::CowVariant, "minecraft:cold"),
        (
            EntityDataRegistryHolder::CowSoundVariant,
            "minecraft:classic",
        ),
        (EntityDataRegistryHolder::WolfVariant, "minecraft:ashen"),
        (
            EntityDataRegistryHolder::WolfSoundVariant,
            "minecraft:angry",
        ),
        (EntityDataRegistryHolder::FrogVariant, "minecraft:cold"),
        (EntityDataRegistryHolder::PigVariant, "minecraft:cold"),
        (EntityDataRegistryHolder::PigSoundVariant, "minecraft:big"),
        (EntityDataRegistryHolder::ChickenVariant, "minecraft:cold"),
        (
            EntityDataRegistryHolder::ChickenSoundVariant,
            "minecraft:classic",
        ),
        (
            EntityDataRegistryHolder::ZombieNautilusVariant,
            "minecraft:temperate",
        ),
    ] {
        let entry = access
            .entry_by_holder_id(serializer.registry(), 0)
            .unwrap_or_else(|| panic!("missing fallback for {}", serializer.registry()));
        assert_eq!(entry.resource_id, first_resource_id);
        assert_eq!(entry.source, RegistryEntrySource::VanillaFallback);
        assert_eq!(
            access
                .entry_by_resource_id(serializer.registry(), first_resource_id)
                .map(|entry| entry.holder_id),
            Some(0)
        );
        assert!(access
            .entry_by_holder_id(serializer.registry(), i32::MAX)
            .is_none());
    }

    let painting = access
        .entry_by_holder_id("minecraft:painting_variant", 0)
        .unwrap();
    assert_eq!(painting.resource_id, "minecraft:alban");
    assert_eq!(painting.source, RegistryEntrySource::VanillaFallback);
    assert_ne!(
        painting.resource_id,
        crate::painting_variants::vanilla_painting_variant(0)
            .unwrap()
            .resource_id,
        "painting_variant is data-loaded, so holder 0 is not the bootstrap head"
    );
}

#[test]
fn frozen_registry_access_resolves_reference_and_holder_set_forms() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:test",
        0,
        vec![
            RegistryPacketEntry::stub("example:custom"),
            RegistryPacketEntry::stub("minecraft:first"),
            RegistryPacketEntry::stub("minecraft:second"),
        ],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:test".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:mixed".to_string(),
                entries: vec![2, 9, 0],
            }],
        }],
    });

    let access = store.frozen_registry_access();
    assert!(access.has_server_contents("minecraft:test"));
    assert!(!access.has_server_contents("minecraft:damage_type"));
    assert_eq!(
        access
            .require_entry_by_holder_id("minecraft:test", 0)
            .unwrap()
            .resource_id,
        "example:custom"
    );
    assert_eq!(
        access
            .require_entry_by_resource_id("minecraft:test", "minecraft:first")
            .unwrap()
            .holder_id,
        1
    );

    let direct = access
        .holder_set_by_holder_ids("minecraft:test", &[2, 0])
        .unwrap();
    assert_eq!(direct.kind, FrozenRegistryHolderSetKind::Direct);
    assert_eq!(
        direct
            .entries
            .iter()
            .map(|entry| entry.resource_id)
            .collect::<Vec<_>>(),
        vec!["minecraft:second", "example:custom"]
    );

    let resource_ids = ["minecraft:first".to_string(), "example:custom".to_string()];
    let direct = access
        .holder_set_by_resource_ids("minecraft:test", &resource_ids)
        .unwrap();
    assert_eq!(
        direct
            .entries
            .iter()
            .map(|entry| entry.holder_id)
            .collect::<Vec<_>>(),
        vec![1, 0]
    );

    let named = access
        .holder_set_by_tag("minecraft:test", "example:mixed")
        .unwrap();
    assert_eq!(
        named.kind,
        FrozenRegistryHolderSetKind::Named {
            tag: "example:mixed",
            unresolved_holder_ids: &[9],
        }
    );
    assert_eq!(
        named
            .entries
            .iter()
            .map(|entry| entry.resource_id)
            .collect::<Vec<_>>(),
        vec!["minecraft:second", "example:custom"]
    );

    assert_eq!(
        access
            .require_entry_by_holder_id("minecraft:test", 9)
            .unwrap_err()
            .to_string(),
        "missing minecraft:test holder 9"
    );
    assert_eq!(
        access
            .require_entry_by_resource_id("minecraft:test", "example:missing")
            .unwrap_err()
            .to_string(),
        "missing minecraft:test resource example:missing"
    );
    assert_eq!(
        access
            .holder_set_by_tag("minecraft:test", "example:missing")
            .unwrap_err()
            .to_string(),
        "missing minecraft:test tag #example:missing"
    );
}

#[test]
fn enchantment_configuration_rejects_unresolved_effect_and_sound_holders() {
    let mut store = WorldStore::new();
    let stone = store
        .registry_holder_id_by_resource_id("minecraft:item", "minecraft:stone")
        .unwrap();
    let dirt = store
        .registry_holder_id_by_resource_id("minecraft:item", "minecraft:dirt")
        .unwrap();
    let speed = store
        .registry_holder_id_by_resource_id("minecraft:mob_effect", "minecraft:speed")
        .unwrap();

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:damage_type",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "minecraft:generic",
            br#"{"message_id":"generic","scaling":"never","exhaustion":0.0}"#.to_vec(),
        )],
    );
    let mut first_enchantment = complete_enchantment_json(
        "First",
        serde_json::json!("#example:supported"),
        serde_json::json!({
            "minecraft:hit_block": [
                {
                    "effect": {
                        "type": "minecraft:apply_mob_effect",
                        "to_apply": "#example:curable",
                        "min_duration": 1.0,
                        "max_duration": 1.0,
                        "min_amplifier": 0.0,
                        "max_amplifier": 0.0,
                    },
                },
                {
                    "effect": {
                        "type": "minecraft:damage_entity",
                        "damage_type": "minecraft:generic",
                        "min_damage": 1.0,
                        "max_damage": 1.0,
                    },
                },
                {
                    "effect": {
                        "type": "minecraft:explode",
                        "damage_type": "minecraft:generic",
                        "immune_blocks": ["minecraft:stone"],
                        "radius": 1.0,
                        "block_interaction": "trigger",
                        "small_particle": { "type": "minecraft:gust_emitter_small" },
                        "large_particle": { "type": "minecraft:gust_emitter_large" },
                        "sound": "minecraft:entity.generic.explode",
                    },
                },
                {
                    "effect": {
                        "type": "minecraft:play_sound",
                        "sound": [
                            "minecraft:block.note_block.harp",
                            { "sound_id": "example:direct_note" },
                        ],
                        "volume": 1.0,
                        "pitch": 1.0,
                    },
                },
                {
                    "effect": {
                        "type": "minecraft:summon_entity",
                        "entity": "minecraft:pig",
                    },
                },
                {
                    "effect": {
                        "type": "minecraft:replace_block",
                        "block_state": {
                            "type": "minecraft:simple_state_provider",
                            "state": { "Name": "minecraft:stone" },
                        },
                        "trigger_game_event": "minecraft:block_change",
                    },
                },
                {
                    "effect": {
                        "type": "minecraft:replace_disk",
                        "radius": 1.0,
                        "height": 1.0,
                        "block_state": {
                            "type": "minecraft:simple_state_provider",
                            "state": { "Name": "minecraft:stone" },
                        },
                        "trigger_game_event": "minecraft:block_change",
                    },
                },
                {
                    "effect": {
                        "type": "minecraft:set_block_properties",
                        "properties": {},
                        "trigger_game_event": "minecraft:block_change",
                    },
                },
            ],
            "minecraft:trident_sound": [
                "minecraft:item.trident.return",
                { "sound_id": "example:direct_trident" },
            ],
        }),
    );
    first_enchantment["primary_items"] = serde_json::json!("minecraft:stone");
    first_enchantment["exclusive_set"] = serde_json::json!("#example:exclusive");
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:first",
                serde_json::to_vec(&first_enchantment).unwrap(),
            ),
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:second",
                serde_json::to_vec(&complete_enchantment_json(
                    "Second",
                    serde_json::json!("minecraft:dirt"),
                    serde_json::json!({}),
                ))
                .unwrap(),
            ),
        ],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![
            RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "example:supported".to_string(),
                    entries: vec![dirt, stone],
                }],
            },
            RegistryTags {
                registry: "minecraft:enchantment".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "example:exclusive".to_string(),
                    entries: vec![1],
                }],
            },
            RegistryTags {
                registry: "minecraft:mob_effect".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "example:curable".to_string(),
                    entries: vec![speed],
                }],
            },
        ],
    });
    store.finish_registry_configuration().unwrap();

    let committed_revision = store.registry_access_revision();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid",
            serde_json::to_vec(&complete_enchantment_json(
                "Invalid effect",
                serde_json::json!("minecraft:stone"),
                serde_json::json!({
                    "minecraft:hit_block": [{
                        "effect": {
                            "type": "minecraft:apply_mob_effect",
                            "to_apply": "example:missing_effect",
                            "min_duration": 1.0,
                            "max_duration": 1.0,
                            "min_amplifier": 0.0,
                            "max_amplifier": 0.0,
                        },
                    }],
                }),
            ))
            .unwrap(),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();
    assert_eq!(error.validator, "Enchantment.DIRECT_CODEC");
    assert!(
        error.message.contains("example:missing_effect"),
        "{error:?}"
    );
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

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_reference",
            serde_json::to_vec(&complete_enchantment_json(
                "Invalid reference",
                serde_json::json!("minecraft:stone"),
                serde_json::json!({
                    "minecraft:hit_block": [{
                        "effect": {
                            "type": "minecraft:replace_block",
                            "block_state": {
                                "type": "minecraft:simple_state_provider",
                                "state": { "Name": "minecraft:stone" },
                            },
                            "trigger_game_event": "example:missing_event",
                        },
                    }],
                }),
            ))
            .unwrap(),
        )],
    );
    let error = store.finish_registry_configuration().unwrap_err();
    assert_eq!(error.validator, "Enchantment.DIRECT_CODEC");
    assert!(error.message.contains("example:missing_event"), "{error:?}");
    assert!(error.message.contains("<references>"), "{error:?}");
    assert_eq!(store.registry_access_revision(), committed_revision);

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_sound",
            serde_json::to_vec(&complete_enchantment_json(
                "Invalid sound",
                serde_json::json!("minecraft:stone"),
                serde_json::json!({
                    "minecraft:hit_block": [{
                        "effect": {
                            "type": "minecraft:play_sound",
                            "sound": "example:missing_sound",
                            "volume": 1.0,
                            "pitch": 1.0,
                        },
                    }],
                }),
            ))
            .unwrap(),
        )],
    );
    let error = store.finish_registry_configuration().unwrap_err();
    assert_eq!(error.validator, "Enchantment.DIRECT_CODEC");
    assert!(error.message.contains("example:missing_sound"), "{error:?}");
    assert!(error.message.contains("<references>"), "{error:?}");
    assert_eq!(store.registry_access_revision(), committed_revision);

    for field in ["supported_items", "primary_items", "exclusive_set"] {
        let mut enchantment = complete_enchantment_json(
            "Missing tag",
            serde_json::json!("minecraft:stone"),
            serde_json::json!({}),
        );
        enchantment[field] = serde_json::json!("#example:missing");
        store.begin_registry_configuration();
        record_minimal_non_empty_synchronized_registries(&mut store, None);
        store.record_registry_entries(
            "minecraft:enchantment",
            0,
            vec![RegistryPacketEntry::with_known_pack_data_for_test(
                "example:missing_tag",
                serde_json::to_vec(&enchantment).unwrap(),
            )],
        );
        assert!(store.finish_registry_configuration().is_err(), "{field}");
    }
}

/// `minecraft:point_of_interest_type` is one of the seven static registries
/// `RegistrySynchronization.networkSafeRegistries` puts in `UpdateTags`
/// (RegistrySynchronization.java:81-84) while sending no
/// `ClientboundRegistryDataPacket` for it, so a tag's holder ids only resolve
/// through the bootstrap order `BuiltInRegistries` gives it
/// (BuiltInRegistries.java:211).
#[test]
fn a_poi_tag_resolves_through_the_bootstrap_order_no_packet_carries() {
    let store = WorldStore::new();
    let access = store.frozen_registry_access();

    assert_eq!(
        access
            .entry_by_holder_id("minecraft:point_of_interest_type", 0)
            .map(|entry| entry.resource_id),
        Some("minecraft:armorer")
    );
    assert_eq!(
        access
            .entry_by_resource_id("minecraft:point_of_interest_type", "minecraft:armorer")
            .map(|entry| entry.holder_id),
        Some(0)
    );
}

#[test]
fn native_availability_preserves_complete_fallback_catalogs_and_builtin_ownership() {
    use bbb_registry_codec::registry_codec::RegistryReferenceMembership;
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:map_decoration_type",
        0,
        vec![RegistryPacketEntry::stub("example:shadow")],
    );
    let lookup = store.frozen_registry_access();
    for (registry, member) in [
        ("minecraft:map_decoration_type", "minecraft:player"),
        ("minecraft:banner_pattern", "minecraft:base"),
        ("minecraft:enchantment", "minecraft:unbreaking"),
        ("minecraft:instrument", "minecraft:ponder_goat_horn"),
        ("minecraft:trim_pattern", "minecraft:coast"),
        ("minecraft:trim_material", "minecraft:iron"),
    ] {
        assert!(lookup.registry_available(registry), "{registry}");
        assert_eq!(
            lookup.element_membership(registry, member),
            RegistryReferenceMembership::Present,
            "{registry}"
        );
        assert_eq!(
            lookup.element_membership(registry, "example:missing"),
            RegistryReferenceMembership::Absent,
            "{registry}"
        );
    }
    assert_eq!(
        lookup.element_membership("minecraft:map_decoration_type", "example:shadow"),
        RegistryReferenceMembership::Absent
    );
    assert!(!lookup.registry_available("example:unpublished"));
    assert_eq!(
        lookup.element_membership("example:unpublished", "example:member"),
        RegistryReferenceMembership::Unavailable
    );
}
