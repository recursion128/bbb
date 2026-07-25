use bbb_protocol::packets::{
    EntityDataRegistryHolder, KnownPack, RegistryData, RegistryDataEntry, RegistryTags,
    TagNetworkPayload, UpdateTags,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    FrozenRegistryHolderSetKind, FurnaceFuelRule, NbtPayloadSummary, RegistryElementSource,
    RegistryEntryPayload, RegistryEntrySource, RegistryHolderSetIdentity,
};
use crate::{RegistryPacketEntry, RegistrySet, WorldStore};
use bbb_pack::RegistryDependentItemComponentInitializer;

fn record_minimal_non_empty_synchronized_registries(
    store: &mut WorldStore,
    omitted_registry: Option<&str>,
) {
    for data_type in bbb_protocol::registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
        if data_type.validator != Some("RegistryValidator.nonEmpty ()")
            || omitted_registry == Some(data_type.resource_id)
        {
            continue;
        }
        if let Some(data) = minimal_sound_variant_nbt(data_type.resource_id) {
            store.record_registry_entries(
                data_type.resource_id,
                0,
                vec![RegistryPacketEntry::with_raw_data(
                    format!(
                        "example:minimal_{}",
                        data_type.constant_name.to_ascii_lowercase()
                    ),
                    data,
                )],
            );
            continue;
        }
        if let Some(data) = minimal_client_variant_json(data_type.resource_id) {
            store.record_registry_entries(
                data_type.resource_id,
                0,
                vec![RegistryPacketEntry::with_known_pack_data_for_test(
                    format!(
                        "example:minimal_{}",
                        data_type.constant_name.to_ascii_lowercase()
                    ),
                    data.as_bytes().to_vec(),
                )],
            );
            continue;
        }
        store.record_registry_entries(
            data_type.resource_id,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                format!(
                    "example:minimal_{}",
                    data_type.constant_name.to_ascii_lowercase()
                ),
                vec![10, 0],
            )],
        );
    }
}

fn minimal_client_variant_json(registry: &str) -> Option<&'static str> {
    match registry {
        "minecraft:chicken_variant"
        | "minecraft:cow_variant"
        | "minecraft:pig_variant"
        | "minecraft:cat_variant" => {
            Some(r#"{"asset_id":"example:adult","baby_asset_id":"example:baby"}"#)
        }
        "minecraft:frog_variant" | "minecraft:zombie_nautilus_variant" => {
            Some(r#"{"asset_id":"example:variant"}"#)
        }
        "minecraft:wolf_variant" => Some(
            r#"{"assets":{"wild":"example:wild","tame":"example:tame","angry":"example:angry"},"baby_assets":{"wild":"example:baby_wild","tame":"example:baby_tame","angry":"example:baby_angry"}}"#,
        ),
        "minecraft:painting_variant" => {
            Some(r#"{"width":1,"height":1,"asset_id":"example:painting"}"#)
        }
        "minecraft:dimension_type" => Some(
            r##"{
                "has_skylight":true,
                "has_ceiling":false,
                "has_ender_dragon_fight":false,
                "coordinate_scale":1.0,
                "min_y":-64,
                "height":384,
                "logical_height":384,
                "infiniburn":"#minecraft:infiniburn_overworld",
                "ambient_light":0.0,
                "monster_spawn_light_level":0,
                "monster_spawn_block_light_limit":0
            }"##,
        ),
        _ => None,
    }
}

fn dimension_type_json(extra_fields: &str) -> Vec<u8> {
    dimension_type_json_with_infiniburn("#minecraft:infiniburn_overworld", extra_fields)
}

fn dimension_type_json_with_infiniburn(infiniburn_tag: &str, extra_fields: &str) -> Vec<u8> {
    format!(
        r##"{{
            "has_skylight":true,
            "has_ceiling":false,
            "has_ender_dragon_fight":false,
            "coordinate_scale":1.0,
            "min_y":-64,
            "height":384,
            "logical_height":384,
            "infiniburn":"{infiniburn_tag}",
            "ambient_light":0.0,
            "monster_spawn_light_level":0,
            "monster_spawn_block_light_limit":0
            {extra_fields}
        }}"##
    )
    .into_bytes()
}

fn minimal_sound_variant_nbt(registry: &str) -> Option<Vec<u8>> {
    let fields = match registry {
        "minecraft:cat_sound_variant" => [
            "ambient_sound",
            "stray_ambient_sound",
            "hiss_sound",
            "hurt_sound",
            "death_sound",
            "eat_sound",
            "beg_for_food_sound",
            "purr_sound",
            "purreow_sound",
        ]
        .as_slice(),
        "minecraft:chicken_sound_variant" | "minecraft:cow_sound_variant" => {
            ["ambient_sound", "hurt_sound", "death_sound", "step_sound"].as_slice()
        }
        "minecraft:pig_sound_variant" => [
            "ambient_sound",
            "hurt_sound",
            "death_sound",
            "step_sound",
            "eat_sound",
        ]
        .as_slice(),
        "minecraft:wolf_sound_variant" => [
            "ambient_sound",
            "death_sound",
            "growl_sound",
            "hurt_sound",
            "pant_sound",
            "whine_sound",
            "step_sound",
        ]
        .as_slice(),
        _ => return None,
    };
    let write_sound_set = |out: &mut Vec<u8>| {
        for field in fields {
            out.push(10);
            write_nbt_string(out, field);
            out.push(8);
            write_nbt_string(out, "sound_id");
            write_nbt_string(out, "minecraft:entity.cat.ambient");
            out.push(0);
        }
    };
    let mut data = vec![10];
    if registry == "minecraft:cow_sound_variant" {
        write_sound_set(&mut data);
    } else {
        data.push(10);
        write_nbt_string(&mut data, "adult_sounds");
        write_sound_set(&mut data);
        data.push(0);
        data.push(10);
        write_nbt_string(&mut data, "baby_sounds");
        write_sound_set(&mut data);
        data.push(0);
    }
    data.push(0);
    Some(data)
}

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
fn frozen_registry_consumers_do_not_read_pending_state_or_cache_registry_packets() {
    fn visit(directory: &Path, check: &mut impl FnMut(&Path, &str)) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().and_then(|name| name.to_str()) != Some("tests") {
                    visit(&path, check);
                }
                continue;
            }
            if path.extension().and_then(|value| value.to_str()) != Some("rs")
                || path.file_name().and_then(|name| name.to_str()) == Some("tests.rs")
            {
                continue;
            }
            let source = fs::read_to_string(&path).unwrap();
            check(
                &path,
                source.split("#[cfg(test)]").next().unwrap_or(&source),
            );
        }
    }

    let registries = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/registries");
    let mut pending_readers = Vec::new();
    visit(&registries, &mut |path, source| {
        let owner = path.file_name().and_then(|name| name.to_str());
        if !matches!(owner, Some("revision.rs" | "state.rs" | "store.rs"))
            && source.contains(".pending")
        {
            pending_readers.push(path.display().to_string());
        }
    });
    assert!(
        pending_readers.is_empty(),
        "registry consumers read uncommitted pending state: {pending_readers:?}"
    );

    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut packet_caches = Vec::new();
    for directory in [
        crates.join("bbb-native/src"),
        crates.join("bbb-item-model/src"),
    ] {
        visit(&directory, &mut |path, source| {
            if ["RegistrySet", "RegistryContentState", "RegistryPacketEntry"]
                .iter()
                .any(|token| source.contains(token))
            {
                packet_caches.push(path.display().to_string());
            }
        });
    }
    assert!(
        packet_caches.is_empty(),
        "immutable consumers cache registry packet/holder state: {packet_caches:?}"
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
fn frozen_registry_access_exposes_payload_source_and_atomic_revisions() {
    let mut store = WorldStore::new();
    assert_eq!(store.registry_access_revision(), Default::default());
    let typed_raw = damage_type_nbt("example.typed");

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:damage_type",
        0,
        vec![
            RegistryPacketEntry::with_raw_data("example:typed", typed_raw.clone()),
            RegistryPacketEntry::stub("example:known_pack_stub"),
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
            known_pack_data: None,
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

#[test]
fn frozen_registry_access_has_exact_fallback_for_every_entity_data_holder_registry() {
    let store = WorldStore::new();
    let access = store.frozen_registry_access();
    for (serializer, first_resource_id) in [
        (EntityDataRegistryHolder::CatVariant, "minecraft:tabby"),
        (
            EntityDataRegistryHolder::CatSoundVariant,
            "minecraft:classic",
        ),
        (EntityDataRegistryHolder::CowVariant, "minecraft:temperate"),
        (
            EntityDataRegistryHolder::CowSoundVariant,
            "minecraft:classic",
        ),
        (EntityDataRegistryHolder::WolfVariant, "minecraft:pale"),
        (
            EntityDataRegistryHolder::WolfSoundVariant,
            "minecraft:classic",
        ),
        (EntityDataRegistryHolder::FrogVariant, "minecraft:temperate"),
        (EntityDataRegistryHolder::PigVariant, "minecraft:temperate"),
        (
            EntityDataRegistryHolder::PigSoundVariant,
            "minecraft:classic",
        ),
        (
            EntityDataRegistryHolder::ChickenVariant,
            "minecraft:temperate",
        ),
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
    assert_eq!(
        painting.resource_id,
        crate::painting_variants::vanilla_painting_variant(0)
            .unwrap()
            .resource_id
    );
    assert_eq!(painting.source, RegistryEntrySource::VanillaFallback);
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
fn persistent_holder_decoder_uses_resource_identity_and_typed_failures() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:test",
        0,
        vec![
            RegistryPacketEntry::stub("example:second"),
            RegistryPacketEntry::stub("example:first"),
        ],
    );
    let access = store.frozen_registry_access();

    let entry = access
        .persistent_holder_from_nbt(
            "minecraft:test",
            &NbtSummaryValue::String("example:first".to_string()),
        )
        .unwrap();
    assert_eq!((entry.holder_id, entry.resource_id), (1, "example:first"));
    let game_event = access
        .persistent_holder_from_nbt(
            "minecraft:game_event",
            &NbtSummaryValue::String("minecraft:block_change".to_string()),
        )
        .unwrap();
    assert_eq!(
        (
            game_event.holder_id,
            game_event.resource_id,
            game_event.source
        ),
        (
            2,
            "minecraft:block_change",
            crate::RegistryEntrySource::VanillaFallback
        )
    );

    assert!(matches!(
        access
            .persistent_holder_or_inline_from_nbt(
                "minecraft:test",
                &NbtSummaryValue::String("example:first".to_string()),
            )
            .unwrap(),
        crate::registries::PersistentHolderOrInline::Named(entry)
            if entry.holder_id == 1 && entry.resource_id == "example:first"
    ));
    let direct = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "value".to_string(),
        value: NbtSummaryValue::Int(7),
    }]);
    assert!(matches!(
        access
            .persistent_holder_or_inline_from_nbt("minecraft:test", &direct)
            .unwrap(),
        crate::registries::PersistentHolderOrInline::Direct(value) if value == &direct
    ));

    for (value, expected) in [
        (
            NbtSummaryValue::String("example:missing".to_string()),
            "missing minecraft:test resource example:missing",
        ),
        (
            NbtSummaryValue::String("not a resource".to_string()),
            "invalid minecraft:test persistent holder resource id not a resource",
        ),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::String("example:first".to_string())]),
            "minecraft:test persistent holder is not a resource id",
        ),
    ] {
        assert_eq!(
            access
                .persistent_holder_from_nbt("minecraft:test", &value)
                .unwrap_err()
                .to_string(),
            expected
        );
    }
}

#[test]
fn persistent_holder_set_decoder_uses_resource_order_tags_and_typed_failures() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:test",
        0,
        vec![
            RegistryPacketEntry::stub("example:second"),
            RegistryPacketEntry::stub("example:first"),
        ],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:test".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:ordered".to_string(),
                entries: vec![1, 0],
            }],
        }],
    });
    let access = store.frozen_registry_access();

    let direct = access
        .persistent_holder_set_from_nbt(
            "minecraft:test",
            &NbtSummaryValue::List(vec![
                NbtSummaryValue::String("example:first".to_string()),
                NbtSummaryValue::String("example:second".to_string()),
            ]),
        )
        .unwrap();
    assert_eq!(direct.kind, FrozenRegistryHolderSetKind::Direct);
    assert_eq!(
        direct
            .entries
            .iter()
            .map(|entry| (entry.holder_id, entry.resource_id))
            .collect::<Vec<_>>(),
        vec![(1, "example:first"), (0, "example:second")]
    );

    let named = access
        .persistent_holder_set_from_nbt(
            "minecraft:test",
            &NbtSummaryValue::String("#example:ordered".to_string()),
        )
        .unwrap();
    assert!(matches!(
        named.kind,
        FrozenRegistryHolderSetKind::Named {
            tag: "example:ordered",
            ..
        }
    ));
    assert_eq!(
        named
            .entries
            .iter()
            .map(|entry| entry.resource_id)
            .collect::<Vec<_>>(),
        vec!["example:first", "example:second"]
    );

    for (value, expected) in [
        (
            NbtSummaryValue::String("#example:missing".to_string()),
            "missing minecraft:test tag #example:missing",
        ),
        (
            NbtSummaryValue::String("example:missing".to_string()),
            "missing minecraft:test resource example:missing",
        ),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(0)]),
            "minecraft:test persistent holder-set entry 0 is not a resource id",
        ),
        (
            NbtSummaryValue::Int(0),
            "minecraft:test persistent holder set is neither a tag, resource id, nor list",
        ),
    ] {
        assert_eq!(
            access
                .persistent_holder_set_from_nbt("minecraft:test", &value)
                .unwrap_err()
                .to_string(),
            expected
        );
    }
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
    record_minimal_non_empty_synchronized_registries(&mut store, None);
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
        vec![RegistryPacketEntry::stub("example:damage")],
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
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:damage_type", 0),
        Some("minecraft:in_fire")
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

fn damage_type_nbt(message_id: &str) -> Vec<u8> {
    let mut payload = vec![10, 8];
    write_nbt_string(&mut payload, "message_id");
    write_nbt_string(&mut payload, message_id);
    payload.push(8);
    write_nbt_string(&mut payload, "scaling");
    write_nbt_string(&mut payload, "never");
    payload.push(5);
    write_nbt_string(&mut payload, "exhaustion");
    payload.extend(0.0_f32.to_bits().to_be_bytes());
    payload.push(0);
    payload
}

fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    out.extend((bytes.len() as u16).to_be_bytes());
    out.extend(bytes);
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
        RegistryElementSource::KnownPackJson
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
        "example:not_a_clock <root>: WorldClock.DIRECT_CODEC expects a compound"
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
        Some("registry_configuration_typed_values")
    );
}

#[test]
fn component_initializer_failure_keeps_registry_revision_and_item_defaults_atomic() {
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:damage_type",
        0,
        vec![RegistryPacketEntry::stub("minecraft:old")],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:damage_type".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:is_fire".to_string(),
                entries: vec![0],
            }],
        }],
    });
    store.set_registry_dependent_item_component_initializers(BTreeMap::from([(
        7,
        vec![RegistryDependentItemComponentInitializer::DamageResistant {
            tag: "minecraft:is_fire".to_string(),
        }],
    )]));
    store.rebuild_registry_dependent_item_defaults().unwrap();
    let active_revision = store.registry_access_revision();
    let stack = bbb_protocol::packets::ItemStackSummary {
        item_id: Some(7),
        count: 1,
        component_patch: Default::default(),
    };
    assert_eq!(
        store
            .effective_item_stack(&stack)
            .component_patch
            .damage_resistant_type_registry_ids,
        vec![0]
    );

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:item");
    assert_eq!(error.validator, "DataComponentInitializers.apply");
    assert_eq!(
        store.registry_access_revision(),
        active_revision,
        "failed initializer must not publish the candidate holder order"
    );
    assert_eq!(
        store
            .effective_item_stack(&stack)
            .component_patch
            .damage_resistant_type_registry_ids,
        vec![0],
        "failed initializer must retain the previous prototype component generation"
    );
}

#[test]
fn configuration_candidate_stores_sound_variant_and_test_environment_typed_values() {
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
                "hurt_sound":{"sound_id":"example:cow_hurt","range":18.0},
                "death_sound":{"sound_id":"example:cow_death"},
                "step_sound":"minecraft:entity.cow.step"
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
                "max_ticks":40
            }"#
            .to_vec(),
        )],
    );
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:candidate_enchantment",
            br#"{
                "description":{"text":"Candidate"},
                "supported_items":["minecraft:dirt","minecraft:stone"],
                "max_level":2,
                "slots":["mainhand"],
                "effects":{}
            }"#
            .to_vec(),
        )],
    );

    store.finish_registry_configuration().unwrap();

    assert_eq!(
        store
            .synchronized_sound_variant_hurt_sound("minecraft:cow_sound_variant", 0, true)
            .unwrap(),
        Some(crate::registries::RegistrySoundEvent::Direct {
            location: "example:cow_hurt".to_string(),
            fixed_range: Some(18.0),
        })
    );
    let environment = store
        .synchronized_test_environment_definition(0)
        .unwrap()
        .unwrap();
    assert_eq!(
        environment.element_source,
        RegistryElementSource::KnownPackJson
    );
    assert!(matches!(
        environment.kind,
        crate::TestEnvironmentKind::GameRules { rules }
            if rules.len() == 2
                && rules[0].value == crate::TestGameRuleTypedValue::Boolean(true)
                && rules[1].value == crate::TestGameRuleTypedValue::Integer(8)
    ));
    assert!(store
        .registries
        .typed
        .test_instances
        .contains_key("example:candidate_instance"));
    assert!(store
        .registries
        .typed
        .enchantments
        .contains_key("example:candidate_enchantment"));
    assert!(matches!(
        store
            .synchronized_test_instance_definition(0)
            .unwrap()
            .unwrap()
            .environment,
        crate::TestEnvironmentHolder::Named(reference)
            if reference.resource_id == "example:rules" && reference.holder_id == 0
    ));
    assert_eq!(
        store
            .enchantment_tooltip_definitions_in_holder_order()
            .unwrap()[0]
            .as_ref()
            .map(|definition| (definition.description.as_str(), definition.max_level)),
        Some(("Candidate", 2))
    );
    assert_eq!(
        store.synchronized_enchantment_holder_sets(0),
        Some(crate::EnchantmentHolderSetDefinition {
            supported_items: RegistryHolderSetIdentity::Direct {
                resource_ids: vec!["minecraft:dirt".to_string(), "minecraft:stone".to_string(),],
            },
            primary_items: None,
            exclusive_enchantments: RegistryHolderSetIdentity::Direct {
                resource_ids: Vec::new(),
            },
        })
    );
}

#[test]
fn synchronized_enchantment_holder_sets_follow_tags_and_reject_invalid_effects_atomically() {
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
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:first",
                br##"{
                    "supported_items":"#example:supported",
                    "primary_items":"minecraft:stone",
                    "exclusive_set":"#example:exclusive",
                    "effects":{"example:test":[
                        {"type":"minecraft:apply_mob_effect","to_apply":"#example:curable"},
                        {"type":"minecraft:damage_entity","damage_type":"minecraft:generic"},
                        {"type":"minecraft:explode","damage_type":"minecraft:generic","immune_blocks":["minecraft:stone"],"sound":"minecraft:entity.generic.explode"},
                        {"type":"minecraft:play_sound","sound":["minecraft:block.note_block.harp",{"sound_id":"example:direct_note"}]},
                        {"type":"minecraft:summon_entity","entity":"minecraft:pig"},
                        {"type":"minecraft:replace_block","trigger_game_event":"minecraft:block_change"},
                        {"type":"minecraft:replace_disk","trigger_game_event":"minecraft:block_change"},
                        {"type":"minecraft:set_block_properties","trigger_game_event":"minecraft:block_change"}
                    ],"minecraft:trident_sound":["minecraft:item.trident.return",{"sound_id":"example:direct_trident"}]}
                }"##
                .to_vec(),
            ),
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:second",
                br#"{"supported_items":"minecraft:dirt","effects":{}}"#.to_vec(),
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

    let expected = crate::EnchantmentHolderSetDefinition {
        supported_items: RegistryHolderSetIdentity::Named {
            tag: "example:supported".to_string(),
        },
        primary_items: Some(RegistryHolderSetIdentity::Direct {
            resource_ids: vec!["minecraft:stone".to_string()],
        }),
        exclusive_enchantments: RegistryHolderSetIdentity::Named {
            tag: "example:exclusive".to_string(),
        },
    };
    assert_eq!(
        store.synchronized_enchantment_holder_sets(0),
        Some(expected.clone())
    );
    assert_eq!(
        store.synchronized_enchantment_holder_sets(1),
        Some(crate::EnchantmentHolderSetDefinition {
            supported_items: RegistryHolderSetIdentity::Direct {
                resource_ids: vec!["minecraft:dirt".to_string()],
            },
            primary_items: None,
            exclusive_enchantments: RegistryHolderSetIdentity::Direct {
                resource_ids: Vec::new(),
            },
        })
    );

    let committed_revision = store.registry_access_revision();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid",
            br#"{
                "supported_items":"minecraft:stone",
                "effects":{"example:test":{
                    "type":"minecraft:apply_mob_effect",
                    "to_apply":"example:missing_effect"
                }}
            }"#
            .to_vec(),
        )],
    );
    let error = store.finish_registry_configuration().unwrap_err();
    assert_eq!(error.validator, "Enchantment.DIRECT_CODEC");
    assert!(error.message.contains("to_apply"), "{error:?}");
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.synchronized_enchantment_holder_sets(0),
        Some(expected)
    );

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_reference",
            br#"{
                "supported_items":"minecraft:stone",
                "effects":{"example:test":{
                    "type":"minecraft:replace_block",
                    "trigger_game_event":"example:missing_event"
                }}
            }"#
            .to_vec(),
        )],
    );
    let error = store.finish_registry_configuration().unwrap_err();
    assert_eq!(error.validator, "Enchantment.DIRECT_CODEC");
    assert!(error.message.contains("trigger_game_event"), "{error:?}");
    assert_eq!(store.registry_access_revision(), committed_revision);

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid_sound",
            br#"{
                "supported_items":"minecraft:stone",
                "effects":{"example:test":{
                    "type":"minecraft:play_sound",
                    "sound":"example:missing_sound"
                }}
            }"#
            .to_vec(),
        )],
    );
    let error = store.finish_registry_configuration().unwrap_err();
    assert_eq!(error.validator, "Enchantment.DIRECT_CODEC");
    assert!(error.message.contains("sound"), "{error:?}");
    assert!(error.message.contains("example:missing_sound"), "{error:?}");
    assert_eq!(store.registry_access_revision(), committed_revision);

    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: Vec::new(),
        }],
    });
    assert_eq!(store.synchronized_enchantment_holder_sets(0), None);
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
    assert_eq!(store.registry_access_revision(), committed_revision);
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
    assert!(error.message.contains("expected integer in range 0..=8"));
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:test_environment", 0),
        Some("example:old_weather")
    );
    assert!(matches!(
        store
            .synchronized_test_environment_definition(0)
            .unwrap()
            .unwrap()
            .kind,
        crate::TestEnvironmentKind::Weather {
            weather: crate::TestWeather::Clear
        }
    ));
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
        .contains("missing minecraft:sound_event resource example:missing_sound"));
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:cow_sound_variant", 0),
        Some("example:old_cow")
    );
}

#[test]
fn configuration_candidate_stores_dimension_timeline_and_clock_typed_values() {
    let mut store = WorldStore::new();
    let stone_id = store
        .registry_holder_id_by_resource_id("minecraft:block", "minecraft:stone")
        .unwrap();
    let dirt_id = store
        .registry_holder_id_by_resource_id("minecraft:block", "minecraft:dirt")
        .unwrap();
    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:world_clock",
        0,
        vec![
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:unused_clock",
                b"{}".to_vec(),
            ),
            RegistryPacketEntry::with_known_pack_data_for_test("example:clock", b"{}".to_vec()),
        ],
    );
    store.record_registry_entries(
        "minecraft:timeline",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:environment",
            br##"{
                "clock":"example:clock",
                "period_ticks":20,
                "tracks":{
                    "minecraft:visual/cloud_height":{
                        "modifier":"add",
                        "keyframes":[{"ticks":0,"value":7.5}]
                    },
                    "minecraft:gameplay/fast_lava":{
                        "modifier":"or",
                        "keyframes":[{"ticks":0,"value":true}]
                    }
                }
            }"##
            .to_vec(),
        )],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![
            RegistryTags {
                registry: "minecraft:timeline".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "example:layers".to_string(),
                    entries: vec![0],
                }],
            },
            RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:infiniburn_overworld".to_string(),
                    entries: vec![dirt_id, stone_id],
                }],
            },
        ],
    });
    store.record_registry_entries(
        "minecraft:dimension_type",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:dimension",
            dimension_type_json(
                r##",
                "attributes":{
                    "minecraft:visual/cloud_height":{
                        "modifier":"multiply",
                        "argument":0.5
                    },
                    "minecraft:gameplay/piglins_zombify":false,
                    "minecraft:audio/background_music":{
                        "default":{
                            "sound":{"sound_id":"example:music"},
                            "min_delay":10,
                            "max_delay":20
                        },
                        "underwater":{
                            "sound":{
                                "sound_id":"example:direct_music",
                                "range":12.0
                            },
                            "min_delay":30,
                            "max_delay":40,
                            "replace_current_music":true
                        }
                    }
                },
                "timelines":"#example:layers",
                "default_clock":"example:clock"
            "##,
            ),
        )],
    );

    store.finish_registry_configuration().unwrap();

    assert_eq!(
        store
            .synchronized_world_clock_definition(1)
            .unwrap()
            .unwrap()
            .resource_id,
        "example:clock"
    );
    let timeline = store.synchronized_timeline_definition(0).unwrap().unwrap();
    assert_eq!(timeline.clock.holder_id, 1);
    assert_eq!(timeline.tracks.len(), 2);
    let dimension = store
        .registries()
        .dimension_type_client_projection(0)
        .unwrap();
    assert_eq!(dimension.has_fixed_time, Some(false));
    assert_eq!(dimension.has_ender_dragon_fight, Some(false));
    assert_eq!(dimension.coordinate_scale, Some(1.0));
    assert_eq!(dimension.logical_height, Some(384));
    assert_eq!(
        dimension.infiniburn_tag.as_deref(),
        Some("minecraft:infiniburn_overworld")
    );
    assert_eq!(
        dimension.monster_spawn_light_level,
        Some(crate::WorldIntProvider::Constant(0))
    );
    assert_eq!(dimension.monster_spawn_block_light_limit, Some(0));
    assert_eq!(
        store
            .registry_resource_ids_for_tag("minecraft:block", "minecraft:infiniburn_overworld")
            .unwrap()
            .unwrap(),
        ["minecraft:dirt", "minecraft:stone"]
    );
    assert_eq!(dimension.timeline_resource_ids, ["example:environment"]);
    assert_eq!(dimension.environment_attributes.cloud_height, Some(96.165));
    assert_eq!(
        dimension.environment_attributes.piglins_zombify,
        Some(false)
    );
    let background = dimension
        .environment_attributes
        .background_music
        .as_ref()
        .unwrap();
    assert_eq!(
        background.default.as_ref().map(|music| &music.sound),
        Some(&crate::ClientSoundEventHolder::Direct {
            sound_id: "example:music".to_string(),
            range_bits: None,
        })
    );
    assert_eq!(
        background.underwater.as_ref().map(|music| &music.sound),
        Some(&crate::ClientSoundEventHolder::Direct {
            sound_id: "example:direct_music".to_string(),
            range_bits: Some(12.0_f32.to_bits()),
        })
    );

    store.level = Some(crate::WorldLevelInfo {
        dimension: "example:dimension".to_string(),
        dimension_type_id: 0,
        dimension_type_name: Some("example:dimension".to_string()),
        dimension_type_properties: crate::WorldDimensionTypeProperties::default(),
        biome_zoom_seed: 0,
        default_clock_resource_id: Some("example:clock".to_string()),
        timeline_resource_ids: dimension.timeline_resource_ids,
        has_sky_light: true,
        has_ceiling: false,
        cardinal_lighting: crate::WorldCardinalLighting::Default,
        skybox: crate::WorldSkybox::Overworld,
        ambient_light: 0.0,
        environment_attributes: dimension.environment_attributes,
        last_death_location: None,
        sea_level: 63,
        is_debug: false,
        is_flat: false,
    });
    store.apply_world_time(bbb_protocol::packets::PlayTime {
        game_time: 7,
        clock_updates: vec![bbb_protocol::packets::ClockUpdate {
            clock_id: 1,
            total_ticks: 7,
            partial_tick: 0.0,
            rate: 1.0,
        }],
    });
    let sampled = store.sampled_client_environment_attributes().unwrap();
    assert_eq!(sampled.cloud_height, Some(103.665));
    assert_eq!(sampled.fast_lava, Some(true));
    assert_eq!(sampled.piglins_zombify, Some(false));

    store.begin_registry_configuration();
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:timeline".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "example:layers".to_string(),
                entries: Vec::new(),
            }],
        }],
    });
    store.finish_registry_configuration().unwrap();
    assert!(store
        .registries()
        .dimension_type_client_projection(0)
        .unwrap()
        .timeline_resource_ids
        .is_empty());

    store.begin_registry_configuration();
    record_minimal_non_empty_synchronized_registries(&mut store, None);
    store.record_registry_entries(
        "minecraft:world_clock",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:replacement_clock",
            b"{}".to_vec(),
        )],
    );
    store.record_registry_entries("minecraft:timeline", 0, Vec::new());
    store.record_registry_entries(
        "minecraft:dimension_type",
        0,
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:replacement_dimension",
            dimension_type_json(
                r#",
                "attributes":{
                    "minecraft:visual/cloud_height":40.0,
                    "minecraft:gameplay/fast_lava":false
                }
            "#,
            ),
        )],
    );
    store.finish_registry_configuration().unwrap();

    let refreshed = store.sampled_client_environment_attributes().unwrap();
    assert_eq!(refreshed.cloud_height, Some(40.0));
    assert_eq!(refreshed.fast_lava, Some(false));
}

#[test]
fn malformed_timeline_candidate_keeps_previous_typed_generation() {
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
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:valid",
            br#"{"clock":"example:clock"}"#.to_vec(),
        )],
    );
    store.finish_registry_configuration().unwrap();
    let committed_revision = store.registry_access_revision();

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
        vec![RegistryPacketEntry::with_known_pack_data_for_test(
            "example:invalid",
            br#"{
                "clock":"example:clock",
                "tracks":{
                    "minecraft:gameplay/fast_lava":{
                        "modifier":"multiply",
                        "keyframes":[{"ticks":0,"value":1.0}]
                    }
                }
            }"#
            .to_vec(),
        )],
    );

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:timeline");
    assert_eq!(error.validator, "Timeline.NETWORK_CODEC");
    assert!(error.message.contains("modifier is not allowed"));
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store
            .synchronized_timeline_definition(0)
            .unwrap()
            .unwrap()
            .resource_id,
        "example:valid"
    );
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
            .contains("missing minecraft:sound_event resource example:missing_music"),
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
    assert_eq!(error.validator, "DimensionType.CLIENT_PROJECTION");
    assert!(
        error
            .message
            .contains("missing minecraft:sound_event resource example:missing_sound"),
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
    assert_eq!(error.validator, "DimensionType.CLIENT_PROJECTION");
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
    assert_eq!(error.validator, "DimensionType.CLIENT_PROJECTION");
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
fn selected_known_pack_missing_stub_data_rejects_configuration_atomically() {
    let manager = test_known_packs_manager("missing");
    let core = core_known_pack();
    let data_source = manager
        .data_source_for(std::slice::from_ref(&core))
        .unwrap();
    let mut store = WorldStore::new();
    store.record_registry_entries(
        "minecraft:damage_type",
        0,
        vec![RegistryPacketEntry::stub("minecraft:old")],
    );
    store.begin_registry_configuration();
    store.apply_select_known_packs_with_data(vec![core.clone()], vec![core], data_source);
    store.record_registry_data(RegistryData {
        registry: "minecraft:damage_type".to_string(),
        entries: vec![RegistryDataEntry {
            id: "example:missing".to_string(),
            raw_data: None,
        }],
        raw_payload_len: 0,
    });

    let error = store.finish_registry_configuration().unwrap_err();

    assert_eq!(error.registry, "minecraft:damage_type");
    assert_eq!(error.validator, "KnownPacksManager.createResourceManager");
    assert_eq!(
        error.message,
        "selected known packs do not provide registry element minecraft:damage_type/example:missing"
    );
    assert_eq!(
        store.registry_resource_id_by_holder_id("minecraft:damage_type", 0),
        Some("minecraft:old")
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

fn test_known_packs_manager(label: &str) -> Arc<bbb_pack::KnownPacksManager> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bbb-world-known-packs-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    write_test_file(
        &root.join("version.json"),
        &format!(
            r#"{{"id":"26.1","protocol_version":{}}}"#,
            bbb_protocol::PROTOCOL_VERSION
        ),
    );
    for registry in bbb_protocol::registry_data_types::VANILLA_SYNCHRONIZED_REGISTRY_DATA_TYPES {
        let (_, registry_path) = registry.resource_id.split_once(':').unwrap();
        let contents = if registry.resource_id == "minecraft:damage_type" {
            r#"{"message_id":"base","scaling":"never","exhaustion":0.0}"#.to_string()
        } else {
            format!(
                r#"{{"registry":"{}","source":"core"}}"#,
                registry.resource_id
            )
        };
        write_test_file(
            &root
                .join("data/minecraft")
                .join(registry_path)
                .join("base.json"),
            &contents,
        );
    }
    let roots = bbb_pack::PackRoots::from_paths(
        root.clone(),
        root.clone(),
        root.join("assets/minecraft"),
        None,
    );
    let manager = Arc::new(bbb_pack::KnownPacksManager::load(&roots).unwrap());
    fs::remove_dir_all(root).unwrap();
    manager
}

fn write_test_file(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn core_known_pack() -> KnownPack {
    KnownPack {
        namespace: "minecraft".to_string(),
        id: "core".to_string(),
        version: "26.1".to_string(),
    }
}
