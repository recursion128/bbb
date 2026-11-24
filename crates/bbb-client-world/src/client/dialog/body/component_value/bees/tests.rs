use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
    entity_types::vanilla_entity_type_for_resource_id, packets::inventory::BeeOccupantSummary,
    NbtSummaryEntry, NbtSummaryValue,
};

use super::*;
use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet};

#[test]
fn decodes_bee_occupants_with_builtin_typed_entity_data() {
    let value = NbtSummaryValue::List(vec![compound(vec![
        value_entry(
            "entity_data",
            compound(vec![
                string_entry("id", "pig"),
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

    let patch = super::super::persistent_patch(VANILLA_DATA_COMPONENT_BEES_TYPE_ID, &value, None);
    assert!(patch.bees_present);
    assert_eq!(
        patch.bees,
        vec![BeeOccupantSummary {
            entity_type_id: occupant.entity_type.holder_id,
            entity_data_tag: Some(occupant.entity_data.clone()),
            ticks_in_hive: 42,
            min_ticks_in_hive: -5,
        }]
    );

    let empty_patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
        &NbtSummaryValue::List(Vec::new()),
        None,
    );
    assert!(empty_patch.bees_present);
    assert!(empty_patch.bees.is_empty());

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

    assert!(decode_value(&NbtSummaryValue::List(Vec::new()))
        .unwrap()
        .is_empty());
}

#[test]
fn rejects_malformed_bee_occupants() {
    for value in [
        compound(Vec::new()),
        NbtSummaryValue::List(vec![NbtSummaryValue::string("bee")]),
        NbtSummaryValue::List(vec![compound(Vec::new())]),
        occupant(NbtSummaryValue::string("bee")),
        occupant(compound(Vec::new())),
        occupant(compound(vec![value_entry("id", NbtSummaryValue::Int(0))])),
        occupant(compound(vec![string_entry("id", "minecraft:missing")])),
        NbtSummaryValue::List(vec![compound(vec![
            value_entry("entity_data", compound(vec![string_entry("id", "bee")])),
            value_entry("min_ticks_in_hive", NbtSummaryValue::Int(0)),
        ])]),
        NbtSummaryValue::List(vec![compound(vec![
            value_entry("entity_data", compound(vec![string_entry("id", "bee")])),
            string_entry("ticks_in_hive", "now"),
            value_entry("min_ticks_in_hive", NbtSummaryValue::Int(0)),
        ])]),
    ] {
        assert!(decode_value(&value).is_err());
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
        name: name.into(),
        value,
    }
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::string(value))
}

fn decode_value(value: &NbtSummaryValue) -> Result<Vec<DialogBeeOccupantState>, String> {
    match super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
        "minecraft:bees",
        value,
        None,
    )? {
        Some(super::super::DialogDataComponentValueState::Bees { occupants }) => Ok(occupants),
        _ => panic!("expected bees component"),
    }
}
