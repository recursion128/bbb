use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
    entity_types::vanilla_entity_type_for_resource_id, NbtSummaryEntry, NbtSummaryValue,
};

use super::*;
use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet};

#[test]
fn decodes_bee_occupants_with_builtin_typed_entity_data() {
    let value = NbtSummaryValue::List(vec![compound(vec![
        value_entry(
            "entity_data",
            compound(vec![
                string_entry("id", "bee"),
                value_entry("Health", NbtSummaryValue::Float(8.0f32.to_bits())),
            ]),
        ),
        value_entry("ticks_in_hive", NbtSummaryValue::Short(42)),
        value_entry("min_ticks_in_hive", NbtSummaryValue::Int(-5)),
    ])]);
    let decoded = super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
        "minecraft:bees",
        &value,
        None,
    )
    .unwrap();
    let Some(super::super::DialogDataComponentValueState::Bees { occupants }) = decoded else {
        panic!("expected bees component");
    };
    assert_eq!(occupants.len(), 1);
    let occupant = &occupants[0];
    assert_eq!(occupant.entity_type.resource_id, "minecraft:bee");
    assert_eq!(
        occupant.entity_type.holder_id,
        vanilla_entity_type_for_resource_id("minecraft:bee")
            .unwrap()
            .id
    );
    assert_eq!(occupant.ticks_in_hive, 42);
    assert_eq!(occupant.min_ticks_in_hive, -5);
    let NbtSummaryValue::Compound(entity_data) = &occupant.entity_data else {
        panic!("expected stripped entity data compound");
    };
    assert_eq!(entity_data.len(), 1);
    assert_eq!(entity_data[0].name, "Health");

    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new("minecraft:entity_type");
    contents.entries = vec![
        RegistryPacketEntry::stub("example:fake"),
        RegistryPacketEntry::stub("minecraft:bee"),
    ];
    registries
        .contents
        .insert("minecraft:entity_type".to_string(), contents);
    let session_ignored = super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
        "minecraft:bees",
        &value,
        Some(&registries),
    )
    .unwrap();
    assert!(matches!(
        session_ignored,
        Some(super::super::DialogDataComponentValueState::Bees { occupants })
            if occupants[0].entity_type.holder_id == occupant.entity_type.holder_id
    ));

    assert!(decode_bees(&NbtSummaryValue::List(Vec::new()), "bees")
        .unwrap()
        .is_empty());
}

#[test]
fn rejects_malformed_bee_occupants() {
    for (value, expected) in [
        (compound(Vec::new()), "is not a list"),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::String("bee".to_string())]),
            "[0] is not a compound",
        ),
        (
            NbtSummaryValue::List(vec![compound(Vec::new())]),
            "missing field entity_data",
        ),
        (
            occupant(NbtSummaryValue::String("bee".to_string())),
            "entity_data is not a compound",
        ),
        (
            occupant(compound(Vec::new())),
            "entity_data is missing field id",
        ),
        (
            occupant(compound(vec![value_entry("id", NbtSummaryValue::Int(0))])),
            "id is not a resource id",
        ),
        (
            occupant(compound(vec![string_entry("id", "minecraft:missing")])),
            "missing minecraft:entity_type resource",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![
                value_entry("entity_data", compound(vec![string_entry("id", "bee")])),
                value_entry("min_ticks_in_hive", NbtSummaryValue::Int(0)),
            ])]),
            "missing field ticks_in_hive",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![
                value_entry("entity_data", compound(vec![string_entry("id", "bee")])),
                string_entry("ticks_in_hive", "now"),
                value_entry("min_ticks_in_hive", NbtSummaryValue::Int(0)),
            ])]),
            "ticks_in_hive is not numeric",
        ),
    ] {
        let error = decode_bees(&value, "bees").unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

fn occupant(entity_data: NbtSummaryValue) -> NbtSummaryValue {
    NbtSummaryValue::List(vec![compound(vec![
        value_entry("entity_data", entity_data),
        value_entry("ticks_in_hive", NbtSummaryValue::Int(0)),
        value_entry("min_ticks_in_hive", NbtSummaryValue::Int(0)),
    ])])
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::String(value.to_string()))
}
