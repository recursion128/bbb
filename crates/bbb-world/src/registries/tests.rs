use bbb_protocol::packets::{
    RegistryData, RegistryDataEntry, RegistryTags, TagNetworkPayload, UpdateTags,
};
use std::collections::BTreeMap;

use crate::RegistryEntrySource;
use crate::{RegistryPacketEntry, RegistrySet, UnresolvedRegistryTagHolder, WorldStore};

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
    assert_eq!(
        registries[0].entries,
        vec![
            RegistryPacketEntry::with_raw_data("minecraft:chat", vec![10; 24]),
            RegistryPacketEntry::stub("minecraft:raw"),
        ]
    );
    assert_eq!(registries[1].entries[0].id, "minecraft:chat");
    assert_eq!(registries[2].entries[0].id, "minecraft:in_fire");
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
    assert!(store
        .resolved_registry_entry_by_resource_id("minecraft:item", "snowball")
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
fn registry_tag_holder_resolution_is_ordered_and_rejects_unknown_holders() {
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
            .unwrap_err(),
        UnresolvedRegistryTagHolder {
            registry: "minecraft:test".to_string(),
            tag: "minecraft:invalid".to_string(),
            holder_id: 7,
        }
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

    store.finish_registry_configuration();
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
    store.finish_registry_configuration();

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
