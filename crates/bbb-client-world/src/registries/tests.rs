use bbb_protocol::environment_audio::SoundEventHolderValue;
use bbb_protocol::packets::{
    ChatTypeHolder, EntityDataRegistryHolder, KnownPack, RegistryData, RegistryDataEntry,
    RegistryTags, TagNetworkPayload, UpdateTags,
};
use bbb_protocol::{ComponentClickEvent, NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::registry_codec::RegistryReferenceLookup;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    FrozenRegistryHolderSetKind, FurnaceFuelRule, NbtPayloadSummary, RegistryElementSource,
    RegistryEntryPayload, RegistryEntrySource,
};
use crate::{RegistryPacketEntry, RegistrySet, WorldStore};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID;

mod codec_terminal;
mod component_numeric;
mod component_numeric_families;
mod component_score;
mod configuration;
mod direct_codecs;
mod fallbacks_and_data;
mod static_catalog_tags;

pub(crate) fn record_minimal_non_empty_synchronized_registries(
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

fn minimal_notice_dialog_json(title: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "type": "minecraft:notice",
        "title": title,
    }))
    .unwrap()
}

fn complete_enchantment_json(
    description: &str,
    supported_items: serde_json::Value,
    effects: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "description": { "text": description },
        "supported_items": supported_items,
        "weight": 1,
        "max_level": 2,
        "min_cost": { "base": 1, "per_level_above_first": 1 },
        "max_cost": { "base": 10, "per_level_above_first": 1 },
        "anvil_cost": 1,
        "slots": ["mainhand"],
        "effects": effects,
    })
}

fn dialog_reference_title(resource_id: &str) -> serde_json::Value {
    serde_json::json!({
        "text": "Open",
        "click_event": {
            "action": "show_dialog",
            "dialog": resource_id,
        },
    })
}

fn written_book_dialog_reference_title(resource_id: &str) -> serde_json::Value {
    serde_json::json!({
        "text": "Book",
        "hover_event": {
            "action": "show_item",
            "id": "minecraft:written_book",
            "components": {
                "minecraft:written_book_content": {
                    "title": { "raw": "Cycle" },
                    "author": "Author",
                    "pages": [{
                        "raw": {
                            "text": "Open",
                            "click_event": {
                                "action": "show_dialog",
                                "dialog": resource_id,
                            },
                        },
                    }],
                },
            },
        },
    })
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
fn registry_packets_classify_configuration_element_sources_from_generated_inventory() {
    let mut store = WorldStore::new();
    store.record_registry("minecraft:worldgen/biome", 0);
    store.record_registry("minecraft:item", 0);
    store.record_registry("example:custom", 0);

    assert!(store.registries().registries[0].configuration_synchronized);
    assert!(!store.registries().registries[0].client_built_in);
    assert!(!store.registries().registries[1].configuration_synchronized);
    assert!(store.registries().registries[1].client_built_in);
    assert!(!store.registries().registries[2].configuration_synchronized);
    assert!(!store.registries().registries[2].client_built_in);
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
    assert!(!item_tags.tags.contains_key("minecraft:logs"));
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
fn furnace_burn_ticks_resolves_vanilla_items_tags_and_exclusions() {
    let item_id = |resource_id| {
        bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)
            .unwrap()
            .id
    };
    let lava_bucket = item_id("minecraft:lava_bucket");
    let coal = item_id("minecraft:coal");
    let oak_planks = item_id("minecraft:oak_planks");
    let crimson_planks = item_id("minecraft:crimson_planks");

    let mut store = WorldStore::new();
    store.set_furnace_fuel_profile(
        BTreeSet::from([lava_bucket, coal]),
        vec![
            FurnaceFuelRule::AddItem(lava_bucket),
            FurnaceFuelRule::AddItem(coal),
            FurnaceFuelRule::AddTag("minecraft:planks".to_string()),
            FurnaceFuelRule::RemoveTag("minecraft:non_flammable_wood".to_string()),
        ],
    );
    store.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![
                TagNetworkPayload {
                    tag: "minecraft:planks".to_string(),
                    entries: vec![oak_planks, crimson_planks],
                },
                TagNetworkPayload {
                    tag: "minecraft:non_flammable_wood".to_string(),
                    entries: vec![crimson_planks],
                },
            ],
        }],
    });

    assert_eq!(store.furnace_burn_ticks(lava_bucket), Some(20_000));
    assert_eq!(store.furnace_burn_ticks(coal), Some(1_600));
    assert_eq!(store.furnace_burn_ticks(oak_planks), Some(300));
    assert_eq!(store.furnace_burn_ticks(crimson_planks), None);
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
    store.set_default_item_component_codec_values(BTreeMap::from([(
        7,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
            serde_json::json!({"types": "#minecraft:is_fire"}),
        )]),
    )]));
    store.rebuild_item_component_defaults().unwrap();
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
                "coordinate_scale":0.000009999999747378752,
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
            .frozen_registry_access()
            .entry_by_holder_id("minecraft:world_clock", 1)
            .unwrap()
            .resource_id,
        "example:clock"
    );
    let timeline = &store.registries.typed.timelines["example:environment"];
    assert_eq!(timeline.clock_id, 1);
    assert_eq!(timeline.tracks.len(), 2);
    let dimension = store
        .registries()
        .dimension_type_client_projection(0)
        .unwrap();
    let facts = &dimension.facts;
    let scalars = &facts.scalars;
    assert!(!scalars.has_fixed_time);
    assert!(!scalars.has_ender_dragon_fight);
    assert_eq!(scalars.coordinate_scale, 1.0e-5_f32 as f64);
    assert_eq!(scalars.logical_height, 384);
    assert_eq!(facts.infiniburn_tag, "minecraft:infiniburn_overworld");
    assert_eq!(
        facts.monster_spawn_light_level,
        crate::WorldIntProvider::Constant(0)
    );
    assert_eq!(scalars.monster_spawn_block_light_limit, 0);
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
        Some(&SoundEventHolderValue::Direct {
            sound_id: "example:music".to_string(),
            range_bits: None,
        })
    );
    assert_eq!(
        background.underwater.as_ref().map(|music| &music.sound),
        Some(&SoundEventHolderValue::Direct {
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
        timeline_resource_ids: dimension.timeline_resource_ids.clone(),
        has_sky_light: true,
        has_ceiling: false,
        cardinal_lighting: crate::WorldCardinalLighting::Default,
        skybox: crate::WorldSkybox::Overworld,
        ambient_light: 0.0,
        environment_attributes: dimension.environment_attributes.clone(),
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
    assert!(error.message.contains("modifier `multiply` is not allowed"));
    assert_eq!(store.registry_access_revision(), committed_revision);
    assert_eq!(
        store.registries.typed.timelines["example:valid"].resource_id,
        "example:valid"
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

fn test_known_packs_manager(label: &str) -> Arc<bbb_client_assets::KnownPacksManager> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "bbb-client-world-known-packs-{label}-{}-{nonce}",
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
    let roots = bbb_client_assets::PackRoots::from_paths(
        root.clone(),
        root.clone(),
        root.join("assets/minecraft"),
    );
    let manager = Arc::new(bbb_client_assets::KnownPacksManager::load(&roots).unwrap());
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

mod component_strings;
