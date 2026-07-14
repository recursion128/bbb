use std::collections::BTreeMap;

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID, VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
};
use bbb_render_types::VillagerModelType;

use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet, RegistryTagState};

use super::super::decode_supported_component_value;
use super::*;

const ITEM_REGISTRY: &str = "minecraft:item";
const DAMAGE_TYPE_REGISTRY: &str = "minecraft:damage_type";

#[test]
fn decodes_damage_type_named_holder_from_session_registry() {
    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new(DAMAGE_TYPE_REGISTRY);
    contents.entries = vec![
        RegistryPacketEntry::stub("example:first"),
        RegistryPacketEntry::stub("minecraft:generic_kill"),
    ];
    registries
        .contents
        .insert(DAMAGE_TYPE_REGISTRY.to_string(), contents);

    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
        "minecraft:damage_type",
        &NbtSummaryValue::String("generic_kill".to_string()),
        Some(&registries),
    )
    .unwrap();
    assert!(matches!(
        decoded,
        Some(DialogDataComponentValueState::DamageType { damage_type })
            if damage_type.holder_id == 1
                && damage_type.resource_id == "minecraft:generic_kill"
    ));
}

#[test]
fn rejects_context_free_direct_and_unknown_damage_type_holders() {
    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new(DAMAGE_TYPE_REGISTRY);
    contents.entries = vec![RegistryPacketEntry::stub("minecraft:generic_kill")];
    registries
        .contents
        .insert(DAMAGE_TYPE_REGISTRY.to_string(), contents);

    for (value, context, expected) in [
        (
            NbtSummaryValue::String("generic_kill".to_string()),
            None,
            "without registry context",
        ),
        (
            NbtSummaryValue::String("example:missing".to_string()),
            Some(&registries),
            "missing minecraft:damage_type resource",
        ),
        (
            NbtSummaryValue::Compound(Vec::new()),
            Some(&registries),
            "holder is not a resource id",
        ),
    ] {
        let error = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
            "minecraft:damage_type",
            &value,
            context,
        )
        .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

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

#[test]
fn decodes_single_synchronized_registry_holder_reference() {
    let registry = "minecraft:jukebox_song";
    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new(registry);
    contents.entries = vec![
        RegistryPacketEntry::stub("example:first"),
        RegistryPacketEntry::stub("minecraft:cat"),
    ];
    registries.contents.insert(registry.to_string(), contents);

    let song = decode_registry_holder_reference(
        &NbtSummaryValue::String("minecraft:cat".to_string()),
        registry,
        Some(&registries),
        "jukebox playable",
    )
    .unwrap();
    assert_eq!(song.holder_id, 1);
    assert_eq!(song.resource_id, "minecraft:cat");

    for (value, registries, expected) in [
        (
            NbtSummaryValue::String("minecraft:cat".to_string()),
            None,
            "without registry context",
        ),
        (
            NbtSummaryValue::String("example:missing".to_string()),
            Some(&registries),
            "missing minecraft:jukebox_song resource",
        ),
        (
            NbtSummaryValue::List(Vec::new()),
            Some(&registries),
            "is not a resource id",
        ),
    ] {
        let error =
            decode_registry_holder_reference(&value, registry, registries, "jukebox playable")
                .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

#[test]
fn decodes_builtin_villager_variant_without_session_remapping() {
    let value = NbtSummaryValue::String("plains".to_string());
    let decoded = super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
        "minecraft:villager/variant",
        &value,
        None,
    )
    .unwrap();
    assert!(matches!(
        decoded,
        Some(DialogDataComponentValueState::RegistryVariant { registry, holder })
            if registry == "minecraft:villager_type"
                && holder.holder_id == VillagerModelType::Plains.vanilla_registry_index() as i32
                && holder.resource_id == "minecraft:plains"
    ));

    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new("minecraft:villager_type");
    contents.entries = vec![
        RegistryPacketEntry::stub("example:fake"),
        RegistryPacketEntry::stub("minecraft:plains"),
    ];
    registries
        .contents
        .insert("minecraft:villager_type".to_string(), contents);
    let session_ignored = super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
        "minecraft:villager/variant",
        &value,
        Some(&registries),
    )
    .unwrap();
    assert!(matches!(
        session_ignored,
        Some(DialogDataComponentValueState::RegistryVariant { holder, .. })
            if holder.holder_id == VillagerModelType::Plains.vanilla_registry_index() as i32
    ));

    for (value, expected) in [
        (NbtSummaryValue::Int(0), "is not a resource id"),
        (NbtSummaryValue::String("Invalid".to_string()), "invalid"),
        (
            NbtSummaryValue::String("minecraft:coast".to_string()),
            "missing minecraft:villager_type resource",
        ),
    ] {
        let error = decode_builtin_villager_type_reference(&value, "villager variant").unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}
