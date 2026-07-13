use std::collections::BTreeMap;

use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet, RegistryTagState};

use super::*;

const ITEM_REGISTRY: &str = "minecraft:item";

#[test]
fn decodes_compact_list_and_named_registry_holder_sets() {
    let mut registries = RegistrySet::default();
    let stone_id = registries
        .holder_id_by_resource_id(ITEM_REGISTRY, "minecraft:stone")
        .unwrap();
    let dirt_id = registries
        .holder_id_by_resource_id(ITEM_REGISTRY, "minecraft:dirt")
        .unwrap();
    registries.tags.insert(
        ITEM_REGISTRY.to_string(),
        RegistryTagState {
            registry: ITEM_REGISTRY.to_string(),
            tags: BTreeMap::from([("example:repair_items".to_string(), vec![stone_id, dirt_id])]),
        },
    );

    let compact = decode_registry_holder_set(
        &NbtSummaryValue::String("stone".to_string()),
        ITEM_REGISTRY,
        Some(&registries),
        "repairable",
    )
    .unwrap();
    assert_eq!(compact.holder_set_kind, "direct");
    assert_eq!(compact.tag, None);
    assert_eq!(compact.entries.len(), 1);
    assert_eq!(compact.entries[0].holder_id, stone_id);
    assert_eq!(compact.entries[0].resource_id, "minecraft:stone");

    let list = decode_registry_holder_set(
        &NbtSummaryValue::List(vec![
            NbtSummaryValue::String("minecraft:dirt".to_string()),
            NbtSummaryValue::String("stone".to_string()),
        ]),
        ITEM_REGISTRY,
        Some(&registries),
        "repairable",
    )
    .unwrap();
    assert_eq!(list.holder_set_kind, "direct");
    assert_eq!(list.entries[0].holder_id, dirt_id);
    assert_eq!(list.entries[1].holder_id, stone_id);

    let named = decode_registry_holder_set(
        &NbtSummaryValue::String("#example:repair_items".to_string()),
        ITEM_REGISTRY,
        Some(&registries),
        "repairable",
    )
    .unwrap();
    assert_eq!(named.holder_set_kind, "named");
    assert_eq!(named.tag.as_deref(), Some("example:repair_items"));
    assert_eq!(
        named
            .entries
            .iter()
            .map(|entry| entry.resource_id.as_str())
            .collect::<Vec<_>>(),
        ["minecraft:stone", "minecraft:dirt"]
    );
}

#[test]
fn rejects_unavailable_or_malformed_registry_holder_sets() {
    let mut registries = RegistrySet::default();
    let stone_id = registries
        .holder_id_by_resource_id(ITEM_REGISTRY, "minecraft:stone")
        .unwrap();
    registries.tags.insert(
        ITEM_REGISTRY.to_string(),
        RegistryTagState {
            registry: ITEM_REGISTRY.to_string(),
            tags: BTreeMap::from([("example:broken".to_string(), vec![stone_id, i32::MAX])]),
        },
    );

    for (value, registries, expected) in [
        (
            NbtSummaryValue::String("stone".to_string()),
            None,
            "without registry context",
        ),
        (
            NbtSummaryValue::String("minecraft:missing_item".to_string()),
            Some(&registries),
            "missing minecraft:item resource",
        ),
        (
            NbtSummaryValue::String("#example:missing".to_string()),
            Some(&registries),
            "missing minecraft:item tag",
        ),
        (
            NbtSummaryValue::String("#example:broken".to_string()),
            Some(&registries),
            "contains missing minecraft:item holder",
        ),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(0)]),
            Some(&registries),
            "entry 0 is not a resource id",
        ),
        (
            NbtSummaryValue::Int(0),
            Some(&registries),
            "neither a tag, resource id, nor list",
        ),
    ] {
        let error = decode_registry_holder_set(&value, ITEM_REGISTRY, registries, "repairable")
            .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn context_free_holder_set_accepts_only_the_empty_direct_list() {
    let decoded = decode_registry_holder_set(
        &NbtSummaryValue::List(Vec::new()),
        ITEM_REGISTRY,
        None,
        "repairable",
    )
    .unwrap();
    assert_eq!(decoded.holder_set_kind, "direct");
    assert!(decoded.entries.is_empty());

    for value in [
        NbtSummaryValue::String("stone".to_string()),
        NbtSummaryValue::String("#minecraft:planks".to_string()),
        NbtSummaryValue::List(vec![NbtSummaryValue::String("stone".to_string())]),
    ] {
        assert!(
            decode_registry_holder_set(&value, ITEM_REGISTRY, None, "repairable")
                .unwrap_err()
                .contains("without registry context")
        );
    }
}

#[test]
fn synchronized_registry_holder_set_uses_session_order_and_tags() {
    let registry = "minecraft:banner_pattern";
    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new(registry);
    contents.entries = vec![
        RegistryPacketEntry::stub("example:first"),
        RegistryPacketEntry::stub("minecraft:flower"),
    ];
    registries.contents.insert(registry.to_string(), contents);
    registries.tags.insert(
        registry.to_string(),
        RegistryTagState {
            registry: registry.to_string(),
            tags: BTreeMap::from([("example:reversed".to_string(), vec![1, 0])]),
        },
    );

    let direct = decode_registry_holder_set(
        &NbtSummaryValue::List(vec![
            NbtSummaryValue::String("minecraft:flower".to_string()),
            NbtSummaryValue::String("example:first".to_string()),
        ]),
        registry,
        Some(&registries),
        "banner patterns",
    )
    .unwrap();
    assert_eq!(direct.entries[0].holder_id, 1);
    assert_eq!(direct.entries[1].holder_id, 0);

    let named = decode_registry_holder_set(
        &NbtSummaryValue::String("#example:reversed".to_string()),
        registry,
        Some(&registries),
        "banner patterns",
    )
    .unwrap();
    assert_eq!(named.holder_set_kind, "named");
    assert_eq!(
        named
            .entries
            .iter()
            .map(|entry| entry.resource_id.as_str())
            .collect::<Vec<_>>(),
        ["minecraft:flower", "example:first"]
    );
}
