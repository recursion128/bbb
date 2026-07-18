use bbb_protocol::packets::{
    RegistryData, RegistryDataEntry, RegistryTags, TagNetworkPayload, UpdateTags,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use crate::{FurnaceFuelRule, RegistryEntrySource};
use crate::{RegistryPacketEntry, RegistrySet, WorldStore};

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
fn block_state_fixtures_do_not_restore_fixed_air_id_aliases() {
    fn visit(directory: &Path, violations: &mut Vec<String>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, violations);
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            let source = fs::read_to_string(&path).unwrap();
            for (line_index, line) in source.lines().enumerate() {
                let declaration = line.trim_start();
                let name = declaration
                    .strip_prefix("const ")
                    .and_then(|rest| rest.split_once(':'))
                    .map(|(name, _)| name);
                if matches!(
                    name,
                    Some("AIR_BLOCK_STATE_ID" | "VANILLA_AIR_BLOCK_STATE_ID")
                ) {
                    violations.push(format!("{}:{}", path.display(), line_index + 1));
                }
            }
        }
    }

    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut violations = Vec::new();
    visit(&crates.join("bbb-world/src"), &mut violations);
    visit(&crates.join("bbb-native/src"), &mut violations);
    assert!(
        violations.is_empty(),
        "fixed air block-state fixture aliases: {violations:?}"
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
fn registry_packets_classify_configuration_element_sources_from_generated_inventory() {
    let mut store = WorldStore::new();
    store.record_registry("minecraft:worldgen/biome", 0);
    store.record_registry("minecraft:item", 0);
    store.record_registry("example:custom", 0);

    assert!(store.registries().registries[0].configuration_synchronized);
    assert!(!store.registries().registries[0].client_built_in);
    assert!(store.registries().registries[0].holder_consumer_count > 0);
    assert!(!store.registries().registries[1].configuration_synchronized);
    assert!(store.registries().registries[1].client_built_in);
    assert!(store.registries().registries[1].holder_consumer_count > 0);
    assert!(!store.registries().registries[2].configuration_synchronized);
    assert!(!store.registries().registries[2].client_built_in);
    assert_eq!(store.registries().registries[2].holder_consumer_count, 0);
}

#[test]
fn registry_data_entries_decode_nbt_payload_summary() {
    let mut store = WorldStore::new();
    let raw_chat = nbt_compound_with_string("translation_key", "chat.type.text");
    store.record_registry_data(RegistryData {
        registry: "minecraft:chat_type".to_string(),
        entries: vec![
            RegistryDataEntry {
                id: "minecraft:chat".to_string(),
                raw_data: Some(raw_chat.clone()),
            },
            RegistryDataEntry {
                id: "minecraft:system".to_string(),
                raw_data: None,
            },
        ],
        raw_payload_len: 128,
    });

    let content = store.registry_content("minecraft:chat_type").unwrap();
    let chat = &content.entries[0];
    assert_eq!(chat.raw_data(), Some(raw_chat.as_slice()));
    assert_eq!(chat.nbt.as_ref().unwrap().root_type, 10);
    assert_eq!(chat.nbt.as_ref().unwrap().byte_len, raw_chat.len());
    assert!(content.entries[1].nbt.is_none());
    assert_eq!(store.counters().registry_entries_with_data, 1);
    assert_eq!(store.counters().registry_entry_stubs, 1);
}

#[test]
fn production_registry_data_keeps_non_synchronized_elements_out_of_holder_state() {
    let mut store = WorldStore::new();
    let ambient_cave_id =
        bbb_protocol::sound_event_types::vanilla_sound_event_registry_id("minecraft:ambient.cave")
            .unwrap();

    store.record_registry_data(RegistryData {
        registry: "minecraft:sound_event".to_string(),
        entries: vec![RegistryDataEntry {
            id: "example:injected".to_string(),
            raw_data: Some(nbt_compound_with_string("sound_id", "example:injected")),
        }],
        raw_payload_len: 48,
    });

    let packet = &store.registries().registries[0];
    assert!(!packet.configuration_synchronized);
    assert!(packet.client_built_in);
    assert!(packet.holder_consumer_count > 0);
    assert_eq!(packet.entries[0].id, "example:injected");
    assert!(store.registry_content("minecraft:sound_event").is_none());
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:sound_event", ambient_cave_id),
        Some("minecraft:ambient.cave")
    );
    assert_eq!(store.counters().registries_seen, 1);
    assert_eq!(store.counters().registry_entries_seen, 1);
    assert_eq!(store.counters().registry_content_registries_tracked, 0);
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
fn registry_holder_lookup_uses_checked_wire_order_identity() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:test",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:first"),
            RegistryPacketEntry::with_raw_data("minecraft:second", vec![10, 0, 0]),
            RegistryPacketEntry::stub("minecraft:duplicate"),
            RegistryPacketEntry::stub("minecraft:duplicate"),
        ],
    );

    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:test", 0),
        Some("minecraft:first")
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:test", 1),
        Some("minecraft:second")
    );
    assert_eq!(
        store
            .registry_resource_ids_in_holder_order("minecraft:test")
            .unwrap()
            .collect::<Vec<_>>(),
        vec![
            "minecraft:first",
            "minecraft:second",
            "minecraft:duplicate",
            "minecraft:duplicate",
        ]
    );
    assert!(store
        .registry_resource_ids_in_holder_order("minecraft:missing")
        .is_none());
    assert!(
        store
            .registry_entry_by_holder_id("minecraft:test", 1)
            .unwrap()
            .has_data
    );
    assert_eq!(
        store.registry_holder_id_by_resource_id("minecraft:test", "minecraft:second"),
        Some(1)
    );
    assert_eq!(
        store.registry_holder_id_by_resource_id("minecraft:test", "minecraft:duplicate"),
        None
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:test", -1),
        None
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:test", 4),
        None
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:missing", 0),
        None
    );
}

#[test]
fn registry_lookup_uses_static_fallback_only_without_server_contents() {
    let mut store = WorldStore::new();
    let snowball_id = bbb_protocol::item_types::VANILLA_ITEM_SNOWBALL_ID;
    let zombie_id = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
    let poison_id = bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_POISON_ID;
    let ambient_cave_id =
        bbb_protocol::sound_event_types::vanilla_sound_event_registry_id("minecraft:ambient.cave")
            .unwrap();

    assert_eq!(
        store.resolved_registry_entry_by_holder_id("minecraft:item", snowball_id),
        Some(crate::ResolvedRegistryEntry {
            holder_id: snowball_id,
            resource_id: "minecraft:snowball",
            source: RegistryEntrySource::VanillaFallback,
        })
    );
    assert_eq!(
        store.resolved_registry_entry_by_holder_id("minecraft:block", 1),
        Some(crate::ResolvedRegistryEntry {
            holder_id: 1,
            resource_id: "minecraft:stone",
            source: RegistryEntrySource::VanillaFallback,
        })
    );
    assert_eq!(
        store.resolved_registry_entry_by_holder_id("minecraft:entity_type", zombie_id),
        Some(crate::ResolvedRegistryEntry {
            holder_id: zombie_id,
            resource_id: "minecraft:zombie",
            source: RegistryEntrySource::VanillaFallback,
        })
    );
    assert_eq!(
        store.resolved_registry_entry_by_resource_id("minecraft:item", "minecraft:snowball"),
        store.resolved_registry_entry_by_holder_id("minecraft:item", snowball_id)
    );
    assert_eq!(
        store.resolved_registry_entry_by_holder_id("minecraft:mob_effect", poison_id),
        Some(crate::ResolvedRegistryEntry {
            holder_id: poison_id,
            resource_id: "minecraft:poison",
            source: RegistryEntrySource::VanillaFallback,
        })
    );
    assert_eq!(
        store.resolved_registry_entry_by_resource_id("minecraft:mob_effect", "minecraft:poison"),
        store.resolved_registry_entry_by_holder_id("minecraft:mob_effect", poison_id)
    );
    let resolved_sound = store
        .resolved_registry_entry_by_holder_id("minecraft:sound_event", ambient_cave_id)
        .unwrap();
    assert_eq!(resolved_sound.holder_id, ambient_cave_id);
    assert_eq!(resolved_sound.resource_id, "minecraft:ambient.cave");
    assert_ne!(resolved_sound.source, RegistryEntrySource::Server);
    assert_eq!(
        store.resolved_registry_entry_by_resource_id(
            "minecraft:sound_event",
            "minecraft:ambient.cave",
        ),
        store.resolved_registry_entry_by_holder_id("minecraft:sound_event", ambient_cave_id)
    );
    assert!(store
        .resolved_registry_entry_by_resource_id("minecraft:item", "snowball")
        .is_none());

    store.record_registry_entries(
        "minecraft:block",
        0,
        vec![RegistryPacketEntry::stub("example:custom_block")],
    );
    assert_eq!(
        store.resolved_registry_entry_by_holder_id("minecraft:block", 0),
        Some(crate::ResolvedRegistryEntry {
            holder_id: 0,
            resource_id: "example:custom_block",
            source: RegistryEntrySource::Server,
        })
    );
    assert!(store
        .resolved_registry_entry_by_resource_id("minecraft:block", "minecraft:stone")
        .is_none());

    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:projectiles".to_string(),
                entries: vec![snowball_id, bbb_protocol::item_types::VANILLA_ITEM_EGG_ID],
            }],
        }],
    });
    assert_eq!(
        store
            .registry_resource_ids_for_tag("minecraft:item", "minecraft:projectiles")
            .unwrap(),
        Some(vec!["minecraft:snowball", "minecraft:egg"])
    );

    store.record_registry_entries(
        "minecraft:item",
        0,
        vec![RegistryPacketEntry::stub("example:custom_item")],
    );
    assert_eq!(
        store.resolved_registry_entry_by_holder_id("minecraft:item", 0),
        Some(crate::ResolvedRegistryEntry {
            holder_id: 0,
            resource_id: "example:custom_item",
            source: RegistryEntrySource::Server,
        })
    );
    assert!(store
        .resolved_registry_entry_by_holder_id("minecraft:item", snowball_id)
        .is_none());
    assert!(store
        .resolved_registry_entry_by_resource_id("minecraft:item", "minecraft:snowball")
        .is_none());
}

#[test]
fn registry_tag_holder_resolution_is_ordered_and_reports_unknown_holders() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:test",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:first"),
            RegistryPacketEntry::stub("minecraft:second"),
            RegistryPacketEntry::stub("minecraft:third"),
        ],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:test".to_string(),
            tags: vec![
                TagNetworkPayload {
                    tag: "minecraft:ordered".to_string(),
                    entries: vec![2, 0],
                },
                TagNetworkPayload {
                    tag: "minecraft:invalid".to_string(),
                    entries: vec![1, 7],
                },
            ],
        }],
    });

    assert_eq!(
        store.registry_holder_ids_for_tag("minecraft:test", "minecraft:ordered"),
        Some([2, 0].as_slice())
    );
    assert!(store.registry_holder_is_in_tag("minecraft:test", "minecraft:ordered", 2));
    assert!(!store.registry_holder_is_in_tag("minecraft:test", "minecraft:ordered", 1));
    assert_eq!(
        store
            .registry_tag_names_containing_holder_id("minecraft:test", 0)
            .unwrap()
            .collect::<Vec<_>>(),
        vec!["minecraft:ordered"]
    );
    assert_eq!(
        store
            .registry_resource_ids_for_tag("minecraft:test", "minecraft:ordered")
            .unwrap(),
        Some(vec!["minecraft:third", "minecraft:first"])
    );
    assert_eq!(
        store
            .registry_resource_ids_for_tag("minecraft:test", "minecraft:missing")
            .unwrap(),
        None
    );
    assert_eq!(
        store
            .registry_resource_ids_for_tag("minecraft:test", "minecraft:invalid")
            .unwrap(),
        Some(vec!["minecraft:second"])
    );
    assert_eq!(
        store.registry_unresolved_holder_ids_for_tag("minecraft:test", "minecraft:invalid"),
        Some([7].as_slice())
    );
}

#[test]
fn configuration_registry_collection_commits_atomically_and_replaces_holder_order() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:test",
        0,
        vec![RegistryPacketEntry::stub("minecraft:old")],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:test".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:old_tag".to_string(),
                entries: vec![0],
            }],
        }],
    });

    store.begin_registry_configuration();
    store.record_registry_entries(
        "minecraft:test",
        0,
        vec![RegistryPacketEntry::stub("minecraft:new")],
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:test", 0),
        Some("minecraft:old")
    );
    assert_eq!(
        store
            .registry_resource_ids_in_holder_order("minecraft:test")
            .unwrap()
            .collect::<Vec<_>>(),
        vec!["minecraft:old"]
    );

    store.finish_registry_configuration().unwrap();
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:test", 0),
        Some("minecraft:new")
    );
    assert_eq!(
        store
            .registry_resource_ids_in_holder_order("minecraft:test")
            .unwrap()
            .collect::<Vec<_>>(),
        vec!["minecraft:new"]
    );
    assert!(store.registry_tags("minecraft:test").is_none());
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
    store.record_registry_data(RegistryData {
        registry: registry.to_string(),
        entries: vec![RegistryDataEntry {
            id: "example:custom".to_string(),
            raw_data: None,
        }],
        raw_payload_len: 0,
    });
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
            RegistryPacketEntry::stub("example:first"),
            RegistryPacketEntry::stub("example:second"),
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
fn configuration_registry_validation_allows_empty_registry_without_validator() {
    let mut store = WorldStore::new();
    store.begin_registry_configuration();
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
}

#[test]
fn tag_only_configuration_preserves_registry_contents_and_replaces_named_tags() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:test",
        0,
        vec![RegistryPacketEntry::stub("minecraft:kept")],
    );
    store.begin_registry_configuration();
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:test".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:new_tag".to_string(),
                entries: vec![0],
            }],
        }],
    });
    store.finish_registry_configuration().unwrap();

    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:test", 0),
        Some("minecraft:kept")
    );
    assert_eq!(
        store.registry_tags("minecraft:test").unwrap().tags["minecraft:new_tag"],
        vec![0]
    );
}

#[test]
fn update_tags_replace_network_tag_state() {
    let mut store = WorldStore::new();
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![
                TagNetworkPayload {
                    tag: "minecraft:logs".to_string(),
                    entries: vec![5, 6, 7],
                },
                TagNetworkPayload {
                    tag: "minecraft:planks".to_string(),
                    entries: vec![42],
                },
            ],
        }],
    });

    let item_tags = store
        .registry_tags("minecraft:item")
        .expect("item registry tags tracked");
    assert_eq!(item_tags.tags["minecraft:logs"], vec![5, 6, 7]);
    assert_eq!(item_tags.tags["minecraft:planks"], vec![42]);
    assert_eq!(store.counters().update_tags_packets, 1);
    assert_eq!(store.counters().tag_registries_tracked, 1);
    assert_eq!(store.counters().tags_tracked, 2);
    assert_eq!(store.counters().tag_entries_tracked, 4);

    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:mineable/pickaxe".to_string(),
                entries: vec![100, 101],
            }],
        }],
    });

    assert!(store.registry_tags("minecraft:item").is_some());
    assert_eq!(
        store.registry_tags("minecraft:block").unwrap().tags["minecraft:mineable/pickaxe"],
        vec![100, 101]
    );
    assert_eq!(store.counters().update_tags_packets, 2);
    assert_eq!(store.counters().tag_registries_tracked, 2);
    assert_eq!(store.counters().tags_tracked, 3);
    assert_eq!(store.counters().tag_entries_tracked, 6);

    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:wool".to_string(),
                entries: vec![200],
            }],
        }],
    });

    let item_tags = store.registry_tags("minecraft:item").unwrap();
    assert!(item_tags.tags.get("minecraft:logs").is_none());
    assert_eq!(item_tags.tags["minecraft:wool"], vec![200]);
    assert!(store.registry_tags("minecraft:block").is_some());
    assert_eq!(store.counters().update_tags_packets, 3);
    assert_eq!(store.counters().tag_registries_tracked, 2);
    assert_eq!(store.counters().tags_tracked, 2);
    assert_eq!(store.counters().tag_entries_tracked, 3);
    assert_eq!(store.counters().last_update_tags_registry_count, 1);
    assert_eq!(store.counters().last_update_tags_total_tag_count, 1);
    assert_eq!(store.counters().last_update_tags_total_value_count, 1);
}

#[test]
fn update_tags_recomputes_furnace_fuels_after_immediate_and_transactional_tag_apply() {
    let item_id = |resource_id| {
        bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)
            .unwrap()
            .id
    };
    let stick = item_id("minecraft:stick");
    let oak_log = item_id("minecraft:oak_log");
    let birch_log = item_id("minecraft:birch_log");
    let crimson_stem = item_id("minecraft:crimson_stem");
    let mut store = WorldStore::new();
    store.set_furnace_fuel_profile(
        BTreeSet::from([999]),
        vec![
            FurnaceFuelRule::AddItem(stick),
            FurnaceFuelRule::AddTag("minecraft:logs".to_string()),
            FurnaceFuelRule::RemoveTag("minecraft:non_flammable_wood".to_string()),
        ],
    );

    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![
                TagNetworkPayload {
                    tag: "minecraft:logs".to_string(),
                    entries: vec![oak_log, crimson_stem],
                },
                TagNetworkPayload {
                    tag: "minecraft:non_flammable_wood".to_string(),
                    entries: vec![crimson_stem],
                },
            ],
        }],
    });

    assert_eq!(
        store.furnace_fuel_item_ids(),
        &BTreeSet::from([stick, oak_log])
    );

    store.begin_registry_configuration();
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![
                TagNetworkPayload {
                    tag: "minecraft:logs".to_string(),
                    entries: vec![birch_log, crimson_stem],
                },
                TagNetworkPayload {
                    tag: "minecraft:non_flammable_wood".to_string(),
                    entries: vec![crimson_stem],
                },
            ],
        }],
    });
    assert_eq!(
        store.furnace_fuel_item_ids(),
        &BTreeSet::from([stick, oak_log])
    );

    store.finish_registry_configuration().unwrap();
    assert_eq!(
        store.furnace_fuel_item_ids(),
        &BTreeSet::from([stick, birch_log])
    );
}

#[test]
fn creative_tag_search_reads_immediate_and_transactional_tag_reloads() {
    let mut store = WorldStore::new();
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:building_blocks".to_string(),
                entries: vec![10],
            }],
        }],
    });
    assert!(store.creative_item_matches_tag_search(10, "building"));
    assert!(store.creative_item_matches_tag_search(10, "MINECRAFT:BLOCK"));
    assert!(store.creative_item_matches_tag_search(10, " minecraft : block "));
    assert!(!store.creative_item_matches_tag_search(10, " building"));
    assert!(!store.creative_item_matches_tag_search(11, "building"));

    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:tools/axes".to_string(),
                entries: vec![11],
            }],
        }],
    });
    assert!(!store.creative_item_matches_tag_search(10, "building"));
    assert!(store.creative_item_matches_tag_search(11, "tools/"));
    assert!(store.creative_item_matches_tag_search(11, "example:axes"));

    store.begin_registry_configuration();
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:wool".to_string(),
                entries: vec![12],
            }],
        }],
    });
    assert!(store.creative_item_matches_tag_search(11, "tools/"));
    assert!(!store.creative_item_matches_tag_search(12, "wool"));

    store.finish_registry_configuration().unwrap();
    assert!(!store.creative_item_matches_tag_search(11, "tools/"));
    assert!(store.creative_item_matches_tag_search(12, "wool"));
}

fn nbt_compound_with_string(name: &str, value: &str) -> Vec<u8> {
    let mut payload = vec![10, 8];
    write_nbt_string(&mut payload, name);
    write_nbt_string(&mut payload, value);
    payload.push(0);
    payload
}

fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    out.extend((bytes.len() as u16).to_be_bytes());
    out.extend(bytes);
}
