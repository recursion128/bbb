use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    },
    NbtSummaryEntry, NbtSummaryValue,
};

use crate::{RegistryContentState, RegistryPacketEntry};

use super::super::{decode_supported_component_value, DialogDataComponentValueState};
use super::*;

#[test]
fn decodes_enchantment_maps_with_session_holder_ids() {
    let registries = enchantment_registries();
    let value = compound([
        ("sharpness", NbtSummaryValue::Byte(5)),
        ("example:last", NbtSummaryValue::Long(255)),
        ("example:first", NbtSummaryValue::Double(2.9f64.to_bits())),
    ]);
    let decoded = decode(
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        &value,
        Some(&registries),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Enchantments { entries }) = decoded else {
        panic!("expected enchantments component");
    };
    assert_eq!(
        entries
            .iter()
            .map(|entry| (
                entry.enchantment.holder_id,
                entry.enchantment.resource_id.as_str(),
                entry.level,
            ))
            .collect::<Vec<_>>(),
        [
            (1, "minecraft:sharpness", 5),
            (2, "example:last", 255),
            (0, "example:first", 2),
        ]
    );
}

#[test]
fn decodes_stored_enchantments_and_context_free_empty_maps() {
    let registries = enchantment_registries();
    let decoded = decode(
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
        &compound([("example:first", NbtSummaryValue::Short(1))]),
        Some(&registries),
    )
    .unwrap();
    assert!(matches!(
        decoded,
        Some(DialogDataComponentValueState::StoredEnchantments { entries })
            if entries.len() == 1
                && entries[0].enchantment.holder_id == 0
                && entries[0].level == 1
    ));

    for type_id in [
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    ] {
        let decoded = decode(type_id, &compound([]), None).unwrap();
        assert!(matches!(
            decoded,
            Some(DialogDataComponentValueState::Enchantments { entries })
                | Some(DialogDataComponentValueState::StoredEnchantments { entries })
                if entries.is_empty()
        ));
    }
}

#[test]
fn rejects_malformed_enchantment_maps_holders_and_levels() {
    let registries = enchantment_registries();
    for (value, context, expected) in [
        (
            NbtSummaryValue::List(Vec::new()),
            None,
            "is not an enchantment map",
        ),
        (
            compound([("sharpness", NbtSummaryValue::Int(1))]),
            None,
            "without registry context",
        ),
        (
            compound([("example:missing", NbtSummaryValue::Int(1))]),
            Some(&registries),
            "missing minecraft:enchantment resource",
        ),
        (
            compound([("Bad", NbtSummaryValue::Int(1))]),
            Some(&registries),
            "invalid",
        ),
        (
            compound([("sharpness", NbtSummaryValue::String("5".to_string()))]),
            Some(&registries),
            "level is not numeric",
        ),
        (
            compound([("sharpness", NbtSummaryValue::Int(0))]),
            Some(&registries),
            "outside 1..=255",
        ),
        (
            compound([("sharpness", NbtSummaryValue::Int(256))]),
            Some(&registries),
            "outside 1..=255",
        ),
        (
            compound([
                ("sharpness", NbtSummaryValue::Int(1)),
                ("minecraft:sharpness", NbtSummaryValue::Int(2)),
            ]),
            Some(&registries),
            "duplicate enchantment holder",
        ),
    ] {
        let error =
            decode(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, &value, context).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

fn enchantment_registries() -> RegistrySet {
    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new(ENCHANTMENT_REGISTRY);
    contents.entries = vec![
        RegistryPacketEntry::stub("example:first"),
        RegistryPacketEntry::stub("minecraft:sharpness"),
        RegistryPacketEntry::stub("example:last"),
    ];
    registries
        .contents
        .insert(ENCHANTMENT_REGISTRY.to_string(), contents);
    registries
}

fn decode(
    type_id: i32,
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<Option<DialogDataComponentValueState>, String> {
    let resource_id = if type_id == VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID {
        "minecraft:enchantments"
    } else {
        "minecraft:stored_enchantments"
    };
    decode_supported_component_value(type_id, resource_id, value, registries)
}

fn compound(entries: impl IntoIterator<Item = (&'static str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.to_string(),
                value,
            })
            .collect(),
    )
}
