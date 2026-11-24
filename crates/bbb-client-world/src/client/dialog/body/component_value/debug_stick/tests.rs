use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID, NbtSummaryEntry,
    NbtSummaryValue,
};

use super::*;
use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet};

#[test]
fn decodes_debug_stick_block_property_map_from_builtin_registry() {
    let value = compound(vec![
        value_entry("oak_log", NbtSummaryValue::Int(0)),
        string_entry("oak_log", "axis"),
        string_entry("minecraft:lever", "powered"),
    ]);
    let decoded = super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
        "minecraft:debug_stick_state",
        &value,
        None,
    )
    .unwrap();
    let Some(super::super::DialogDataComponentValueState::DebugStickState { properties }) = decoded
    else {
        panic!("expected debug-stick-state component");
    };
    assert_eq!(properties.len(), 2);
    assert_eq!(properties[0].block.resource_id, "minecraft:oak_log");
    assert_eq!(properties[0].property, "axis");
    assert_eq!(properties[1].block.resource_id, "minecraft:lever");
    assert_eq!(properties[1].property, "powered");
    assert_ne!(properties[0].block.holder_id, properties[1].block.holder_id);
    let oak_log_id = properties[0].block.holder_id;

    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
        &value,
        None,
    );
    assert_eq!(
        patch.debug_stick_properties,
        vec![
            bbb_protocol::packets::DebugStickPropertySummary {
                block_registry_id: properties[0].block.holder_id,
                property: "axis".to_string(),
            },
            bbb_protocol::packets::DebugStickPropertySummary {
                block_registry_id: properties[1].block.holder_id,
                property: "powered".to_string(),
            },
        ]
    );
    assert!(!patch
        .codec_backed_nbt
        .contains_key(&VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID));

    let empty_patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
        &compound(Vec::new()),
        None,
    );
    assert!(empty_patch.debug_stick_properties.is_empty());

    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new("minecraft:block");
    contents.entries = vec![
        RegistryPacketEntry::stub("example:fake"),
        RegistryPacketEntry::stub("minecraft:oak_log"),
    ];
    registries
        .contents
        .insert("minecraft:block".to_string(), contents);
    let session_ignored = super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
        "minecraft:debug_stick_state",
        &value,
        Some(&registries),
    )
    .unwrap();
    assert!(matches!(
        session_ignored,
        Some(super::super::DialogDataComponentValueState::DebugStickState { properties })
            if properties[0].block.holder_id == oak_log_id
    ));

    assert!(decode_value(&compound(Vec::new())).unwrap().is_empty());
}

#[test]
fn rejects_unknown_blocks_properties_and_malformed_debug_stick_maps() {
    for value in [
        NbtSummaryValue::List(Vec::new()),
        string_map_entry("minecraft:missing", "axis"),
        string_map_entry("minecraft:stone", "axis"),
        string_map_entry("minecraft:oak_log", "missing"),
        compound(vec![value_entry(
            "minecraft:oak_log",
            NbtSummaryValue::Int(0),
        )]),
        string_map_entry("Bad Namespace:oak_log", "axis"),
        compound(vec![
            string_entry("oak_log", "axis"),
            string_entry("minecraft:oak_log", "axis"),
        ]),
    ] {
        assert!(decode_value(&value).is_err());
    }
}

fn string_map_entry(block: &str, property: &str) -> NbtSummaryValue {
    compound(vec![string_entry(block, property)])
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

fn decode_value(value: &NbtSummaryValue) -> Result<Vec<DialogDebugStickPropertyState>, String> {
    match super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
        "minecraft:debug_stick_state",
        value,
        None,
    )? {
        Some(super::super::DialogDataComponentValueState::DebugStickState { properties }) => {
            Ok(properties)
        }
        _ => panic!("expected debug_stick_state component"),
    }
}
