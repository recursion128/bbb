use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID, NbtSummaryEntry,
    NbtSummaryValue,
};
use bbb_render_types::vanilla_map_decoration_type_for_resource_id;

use super::*;
use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet};

#[test]
fn decodes_map_decorations_with_builtin_types_and_numeric_codecs() {
    let value = compound(vec![
        value_entry(
            "trial marker",
            decoration(
                "minecraft:trial_chambers",
                NbtSummaryValue::Float(2.5f32.to_bits()),
                NbtSummaryValue::Long(-7),
                NbtSummaryValue::Double(45.25f64.to_bits()),
            ),
        ),
        value_entry(
            "home",
            decoration(
                "mansion",
                NbtSummaryValue::Double((-120.5f64).to_bits()),
                NbtSummaryValue::Int(64),
                NbtSummaryValue::Float(180.0f32.to_bits()),
            ),
        ),
    ]);
    let decoded = decode_component(&value, None);
    let super::super::DialogDataComponentValueState::MapDecorations { decorations } = decoded
    else {
        panic!("expected map-decorations component");
    };
    assert_eq!(decorations.len(), 2);
    assert_eq!(decorations[1].id, "home");
    assert_eq!(
        decorations[1].decoration_type.resource_id,
        "minecraft:mansion"
    );
    assert_eq!(
        decorations[1].decoration_type.holder_id,
        vanilla_map_decoration_type_for_resource_id("minecraft:mansion")
            .unwrap()
            .type_id
    );
    assert_eq!(f64::from_bits(decorations[1].x_bits), -120.5);
    assert_eq!(f64::from_bits(decorations[1].z_bits), 64.0);
    assert_eq!(f32::from_bits(decorations[1].rotation_bits), 180.0);
    assert_eq!(decorations[0].id, "trial marker");
    assert_eq!(f64::from_bits(decorations[0].x_bits), 2.5);
    assert_eq!(f64::from_bits(decorations[0].z_bits), -7.0);
    assert_eq!(f32::from_bits(decorations[0].rotation_bits), 45.25);

    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
        &value,
        None,
    );
    let typed = patch.map_decorations.as_deref().unwrap();
    assert_eq!(typed.len(), 2);
    assert_eq!(typed[1].id, "home");
    assert_eq!(
        typed[1].decoration_type_id,
        vanilla_map_decoration_type_for_resource_id("minecraft:mansion")
            .unwrap()
            .type_id
    );
    assert_eq!(typed[1].decoration_type_resource_id, "minecraft:mansion");
    assert_eq!(f64::from_bits(typed[1].x_bits), -120.5);
    assert_eq!(f64::from_bits(typed[1].z_bits), 64.0);
    assert_eq!(f32::from_bits(typed[1].rotation_bits), 180.0);
    assert_eq!(typed[0].id, "trial marker");
    assert_eq!(
        typed[0].decoration_type_resource_id,
        "minecraft:trial_chambers"
    );
    assert_eq!(
        patch
            .codec_backed_nbt
            .get(&VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID),
        Some(&compound(vec![
            value_entry(
                "trial marker",
                decoration(
                    "minecraft:trial_chambers",
                    NbtSummaryValue::Double(2.5f64.to_bits()),
                    NbtSummaryValue::Double((-7.0f64).to_bits()),
                    NbtSummaryValue::Float(45.25f32.to_bits()),
                ),
            ),
            value_entry(
                "home",
                decoration(
                    "minecraft:mansion",
                    NbtSummaryValue::Double((-120.5f64).to_bits()),
                    NbtSummaryValue::Double(64.0f64.to_bits()),
                    NbtSummaryValue::Float(180.0f32.to_bits()),
                ),
            ),
        ]))
    );

    let empty_patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
        &compound(Vec::new()),
        None,
    );
    assert_eq!(empty_patch.map_decorations.as_deref(), Some(&[][..]));
    assert_eq!(
        empty_patch
            .codec_backed_nbt
            .get(&VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID),
        Some(&compound(Vec::new()))
    );

    let mut registries = RegistrySet::default();
    let mut contents = RegistryContentState::new("minecraft:map_decoration_type");
    contents.entries = vec![
        RegistryPacketEntry::stub("example:fake"),
        RegistryPacketEntry::stub("minecraft:mansion"),
    ];
    registries
        .contents
        .insert("minecraft:map_decoration_type".to_string(), contents);
    let session_ignored = decode_component(&value, Some(&registries));
    assert!(matches!(
        session_ignored,
        super::super::DialogDataComponentValueState::MapDecorations { decorations }
            if decorations[1].decoration_type.holder_id
                == vanilla_map_decoration_type_for_resource_id("minecraft:mansion").unwrap().type_id
    ));

    assert!(decode_value(&compound(Vec::new())).unwrap().is_empty());
}

#[test]
fn map_and_entry_fields_follow_compound_last_wins_semantics() {
    let decorations = decode_value(&compound(vec![
        value_entry("home", NbtSummaryValue::Int(0)),
        value_entry(
            "home",
            decoration_with(vec![
                string_entry("type", "mansion"),
                double_entry("x", 4.0),
                double_entry("x", 5.0),
                double_entry("z", 6.0),
                float_entry("rotation", 7.0),
            ]),
        ),
    ]))
    .unwrap();
    assert_eq!(decorations.len(), 1);
    assert_eq!(
        decorations[0].decoration_type.resource_id,
        "minecraft:mansion"
    );
    assert_eq!(f64::from_bits(decorations[0].x_bits), 5.0);
    assert_eq!(f64::from_bits(decorations[0].z_bits), 6.0);
    assert_eq!(f32::from_bits(decorations[0].rotation_bits), 7.0);
}

#[test]
fn rejects_malformed_map_decoration_maps_and_entries() {
    for value in [
        NbtSummaryValue::List(Vec::new()),
        compound(vec![value_entry("home", NbtSummaryValue::Int(0))]),
        compound(vec![value_entry("home", compound(Vec::new()))]),
        one_decoration(decoration_with(vec![
            value_entry("type", NbtSummaryValue::Int(0)),
            double_entry("x", 0.0),
            double_entry("z", 0.0),
            float_entry("rotation", 0.0),
        ])),
        one_decoration(decoration("Invalid", int(0), int(0), int(0))),
        one_decoration(decoration("minecraft:missing", int(0), int(0), int(0))),
        one_decoration(decoration_with(vec![
            string_entry("type", "frame"),
            string_entry("x", "near"),
            int_entry("z", 0),
            int_entry("rotation", 0),
        ])),
        one_decoration(decoration_with(vec![
            string_entry("type", "frame"),
            int_entry("x", 0),
            int_entry("z", 0),
        ])),
    ] {
        assert!(decode_value(&value).is_err());
    }
}

fn decode_component(
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> super::super::DialogDataComponentValueState {
    super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
        "minecraft:map_decorations",
        value,
        registries,
    )
    .unwrap()
    .unwrap()
}

fn one_decoration(value: NbtSummaryValue) -> NbtSummaryValue {
    compound(vec![value_entry("home", value)])
}

fn decoration(
    decoration_type: &str,
    x: NbtSummaryValue,
    z: NbtSummaryValue,
    rotation: NbtSummaryValue,
) -> NbtSummaryValue {
    decoration_with(vec![
        string_entry("type", decoration_type),
        value_entry("x", x),
        value_entry("z", z),
        value_entry("rotation", rotation),
    ])
}

fn decoration_with(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    compound(entries)
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn int(value: i32) -> NbtSummaryValue {
    NbtSummaryValue::Int(value)
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

fn int_entry(name: &str, value: i32) -> NbtSummaryEntry {
    value_entry(name, int(value))
}

fn float_entry(name: &str, value: f32) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::Float(value.to_bits()))
}

fn double_entry(name: &str, value: f64) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::Double(value.to_bits()))
}

fn decode_value(value: &NbtSummaryValue) -> Result<Vec<DialogMapDecorationState>, String> {
    match super::super::decode_supported_component_value(
        VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
        "minecraft:map_decorations",
        value,
        None,
    )? {
        Some(super::super::DialogDataComponentValueState::MapDecorations { decorations }) => {
            Ok(decorations)
        }
        _ => panic!("expected map_decorations component"),
    }
}
