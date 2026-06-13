use bbb_protocol::packets::{RegistryTags, TagNetworkPayload, UpdateTags};

use crate::{RegistryPacketEntry, RegistrySet, WorldStore};

#[test]
fn loads_vanilla_block_state_registry() {
    let registries = RegistrySet::vanilla_26_1();
    assert_eq!(registries.block_state_count(), 29873);
    assert_eq!(registries.block_state(0).unwrap().name, "minecraft:air");
    let grass = registries.block_state(9).unwrap();
    assert_eq!(grass.name, "minecraft:grass_block");
    assert_eq!(grass.properties.get("snowy").unwrap(), "false");
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
