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

fn decode_repairable(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<DialogRegistryHolderSetState, String> {
    let Some(DialogDataComponentValueState::Repairable { items }) =
        decode_supported_component_value(
            VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
            "minecraft:repairable",
            &NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "items".into(),
                value: value.clone(),
            }]),
            registries,
        )?
    else {
        unreachable!()
    };
    Ok(items)
}

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
        &NbtSummaryValue::string("generic_kill"),
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

    for (value, context) in [
        (NbtSummaryValue::string("generic_kill"), None),
        (
            NbtSummaryValue::string("example:missing"),
            Some(&registries),
        ),
        (NbtSummaryValue::Compound(Vec::new()), Some(&registries)),
    ] {
        assert!(decode_supported_component_value(
            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
            "minecraft:damage_type",
            &value,
            context,
        )
        .is_err());
    }
}

#[test]
fn decodes_compact_list_and_named_registry_holder_sets() {
    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new(ITEM_REGISTRY);
    contents.entries = vec![
        RegistryPacketEntry::stub("example:first"),
        RegistryPacketEntry::stub("minecraft:stone"),
        RegistryPacketEntry::stub("minecraft:dirt"),
    ];
    registries
        .contents
        .insert(ITEM_REGISTRY.to_string(), contents);

    let stone_id = registries
        .holder_id_by_resource_id(ITEM_REGISTRY, "minecraft:stone")
        .unwrap();
    let dirt_id = registries
        .holder_id_by_resource_id(ITEM_REGISTRY, "minecraft:dirt")
        .unwrap();
    registries.tags.insert(
        ITEM_REGISTRY.to_string(),
        RegistryTagState::from_materialized(
            ITEM_REGISTRY.to_string(),
            BTreeMap::from([("example:repair_items".to_string(), vec![stone_id, dirt_id])]),
        ),
    );

    let compact = decode_repairable(&NbtSummaryValue::string("stone"), Some(&registries)).unwrap();
    assert_eq!(compact.holder_set_kind, "direct");
    assert_eq!(compact.tag, None);
    assert_eq!(compact.entries.len(), 1);
    assert_eq!(compact.entries[0].holder_id, stone_id);
    assert_eq!(compact.entries[0].resource_id, "minecraft:stone");

    let list = decode_repairable(
        &NbtSummaryValue::List(vec![
            NbtSummaryValue::string("minecraft:dirt"),
            NbtSummaryValue::string("stone"),
        ]),
        Some(&registries),
    )
    .unwrap();
    assert_eq!(list.holder_set_kind, "direct");
    assert_eq!(list.entries[0].holder_id, dirt_id);
    assert_eq!(list.entries[1].holder_id, stone_id);

    let named = decode_repairable(
        &NbtSummaryValue::string("#example:repair_items"),
        Some(&registries),
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
        RegistryTagState::from_materialized(
            ITEM_REGISTRY.to_string(),
            BTreeMap::from([("example:broken".to_string(), vec![stone_id, i32::MAX])]),
        ),
    );

    for (value, registries, expected) in [
        (
            NbtSummaryValue::string("stone"),
            None,
            "without registry context",
        ),
        (
            NbtSummaryValue::string("minecraft:missing_item"),
            Some(registries.frozen_access()),
            "missing minecraft:item resource",
        ),
        (
            NbtSummaryValue::string("#example:missing"),
            Some(registries.frozen_access()),
            "missing minecraft:item tag",
        ),
        (
            NbtSummaryValue::string("#example:broken"),
            Some(registries.frozen_access()),
            "missing minecraft:item holder",
        ),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(0)]),
            Some(registries.frozen_access()),
            "entry 0 is not a resource id",
        ),
        (
            NbtSummaryValue::Int(0),
            Some(registries.frozen_access()),
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
    let decoded = decode_repairable(&NbtSummaryValue::List(Vec::new()), None).unwrap();
    assert_eq!(decoded.holder_set_kind, "direct");
    assert!(decoded.entries.is_empty());

    for value in [
        NbtSummaryValue::string("stone"),
        NbtSummaryValue::string("#minecraft:planks"),
        NbtSummaryValue::List(vec![NbtSummaryValue::string("stone")]),
    ] {
        assert!(decode_repairable(&value, None).is_err());
    }
}

#[test]
fn holder_set_record_fields_follow_compound_last_wins_semantics() {
    let registries = RegistrySet::default();
    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
        "minecraft:repairable",
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "items".into(),
                value: NbtSummaryValue::Int(0),
            },
            NbtSummaryEntry {
                name: "items".into(),
                value: NbtSummaryValue::string("stone"),
            },
        ]),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Repairable { items: decoded }) = decoded else {
        panic!("expected repairable component")
    };
    assert_eq!(decoded.entries.len(), 1);
    assert_eq!(decoded.entries[0].resource_id, "minecraft:stone");
}

#[test]
fn banner_pattern_layer_and_direct_holder_fields_follow_last_wins_semantics() {
    let layers = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
        "minecraft:banner_patterns",
        &NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "pattern".into(),
                value: NbtSummaryValue::Int(0),
            },
            NbtSummaryEntry {
                name: "pattern".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "asset_id".into(),
                        value: NbtSummaryValue::Int(0),
                    },
                    NbtSummaryEntry {
                        name: "asset_id".into(),
                        value: NbtSummaryValue::string("example:spiral"),
                    },
                    NbtSummaryEntry {
                        name: "translation_key".into(),
                        value: NbtSummaryValue::Int(0),
                    },
                    NbtSummaryEntry {
                        name: "translation_key".into(),
                        value: NbtSummaryValue::string("block.example.spiral"),
                    },
                ]),
            },
            NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::Int(0),
            },
            NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string("blue"),
            },
        ])]),
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::BannerPatterns { layers }) = layers else {
        panic!("expected banner patterns")
    };
    assert!(matches!(
        layers.as_slice(),
        [DialogBannerPatternLayerState {
            pattern: DialogBannerPatternHolderState::Direct {
                pattern: DialogDirectBannerPatternState {
                    asset_id,
                    translation_key,
                },
            },
            color,
            ..
        }] if asset_id == "example:spiral"
            && translation_key == "block.example.spiral"
            && color == "blue"
    ));
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
        RegistryTagState::from_materialized(
            registry.to_string(),
            BTreeMap::from([("example:reversed".to_string(), vec![1, 0])]),
        ),
    );

    let direct = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        "minecraft:provides_banner_patterns",
        &NbtSummaryValue::List(vec![
            NbtSummaryValue::string("minecraft:flower"),
            NbtSummaryValue::string("example:first"),
        ]),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::ProvidesBannerPatterns { patterns: direct }) = direct
    else {
        panic!("expected banner patterns")
    };
    assert_eq!(direct.entries[0].holder_id, 1);
    assert_eq!(direct.entries[1].holder_id, 0);

    let named = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        "minecraft:provides_banner_patterns",
        &NbtSummaryValue::string("#example:reversed"),
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::ProvidesBannerPatterns { patterns: named }) = named
    else {
        panic!("expected banner patterns")
    };
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
        &NbtSummaryValue::string("minecraft:cat"),
        registry,
        Some(registries.frozen_access()),
        "jukebox playable",
    )
    .unwrap();
    assert_eq!(song.holder_id, 1);
    assert_eq!(song.resource_id, "minecraft:cat");

    for (value, registries) in [
        (NbtSummaryValue::string("minecraft:cat"), None),
        (
            NbtSummaryValue::string("example:missing"),
            Some(registries.frozen_access()),
        ),
        (
            NbtSummaryValue::List(Vec::new()),
            Some(registries.frozen_access()),
        ),
    ] {
        assert!(
            decode_registry_holder_reference(&value, registry, registries, "jukebox playable")
                .is_err()
        );
    }
}

#[test]
fn decodes_builtin_villager_variant_without_session_remapping() {
    let value = NbtSummaryValue::string("plains");
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
        (NbtSummaryValue::string("Invalid"), "invalid"),
        (
            NbtSummaryValue::string("minecraft:coast"),
            "missing minecraft:villager_type resource",
        ),
    ] {
        let error = decode_builtin_villager_type_reference(&value, "villager variant").unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}
