use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    },
    packets::inventory::ItemEnchantmentSummary,
    NbtSummaryEntry, NbtSummaryValue,
};

use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet};

use super::super::{decode_supported_component_value, DialogDataComponentValueState};

const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";

#[test]
fn decodes_enchantment_maps_with_session_holder_ids() {
    let registries = enchantment_registries();
    let value = compound([
        ("sharpness", NbtSummaryValue::string("overridden")),
        ("example:last", NbtSummaryValue::Long(255)),
        ("example:first", NbtSummaryValue::Double(2.9f64.to_bits())),
        ("sharpness", NbtSummaryValue::Byte(5)),
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
    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        &value,
        Some(&registries),
    );
    assert_eq!(
        patch
            .enchantments
            .into_iter()
            .map(|entry| (entry.holder_id, entry.level))
            .collect::<std::collections::BTreeMap<_, _>>(),
        std::collections::BTreeMap::from([(0, 2), (1, 5), (2, 255)])
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
    let Some(DialogDataComponentValueState::StoredEnchantments { entries }) = decoded else {
        panic!("expected stored enchantments component");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].enchantment.holder_id, 0);
    assert_eq!(entries[0].level, 1);
    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
        &compound([("example:first", NbtSummaryValue::Short(1))]),
        Some(&registries),
    );
    assert_eq!(
        patch.stored_enchantments,
        vec![ItemEnchantmentSummary {
            holder_id: 0,
            level: 1,
        }]
    );

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
    for (value, context) in [
        (NbtSummaryValue::List(Vec::new()), None),
        (compound([("sharpness", NbtSummaryValue::Int(1))]), None),
        (
            compound([("example:missing", NbtSummaryValue::Int(1))]),
            Some(&registries),
        ),
        (
            compound([("Bad", NbtSummaryValue::Int(1))]),
            Some(&registries),
        ),
        (
            compound([("sharpness", NbtSummaryValue::string("5"))]),
            Some(&registries),
        ),
        (
            compound([("sharpness", NbtSummaryValue::Int(0))]),
            Some(&registries),
        ),
        (
            compound([("sharpness", NbtSummaryValue::Int(256))]),
            Some(&registries),
        ),
        (
            compound([
                ("sharpness", NbtSummaryValue::Int(1)),
                ("minecraft:sharpness", NbtSummaryValue::Int(2)),
            ]),
            Some(&registries),
        ),
    ] {
        assert!(decode(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, &value, context).is_err());
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
                name: name.into(),
                value,
            })
            .collect(),
    )
}
