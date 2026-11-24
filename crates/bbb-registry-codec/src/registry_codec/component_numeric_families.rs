use bbb_protocol::{encode_nbt_summary, NbtSummaryEntry, NbtSummaryValue};
use serde_json::json;

use super::{
    decode_client_known_pack_registry_json, decode_direct_registry_json,
    decode_exact_synchronized_registry_element, decode_network_registry_element,
    RegistryElementData, RegistryReferenceLookup,
};

const REGISTRY: &str = "minecraft:trim_material";
const CUSTOM_MODEL_DATA: &str = "minecraft:custom_model_data";
const FOOD: &str = "minecraft:food";
const MAP_DECORATIONS: &str = "minecraft:map_decorations";

struct PlayerLookup;

impl RegistryReferenceLookup for PlayerLookup {
    fn contains_element(&self, registry: &str, resource_id: &str) -> bool {
        registry == "minecraft:map_decoration_type" && resource_id == "minecraft:player"
    }

    fn contains_tag(&self, _registry: &str, _resource_id: &str) -> bool {
        false
    }
}

fn compound(fields: Vec<(&str, NbtSummaryValue)>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        fields
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.into(),
                value,
            })
            .collect(),
    )
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.into())
}

fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(value.to_bits())
}

fn double(value: f64) -> NbtSummaryValue {
    NbtSummaryValue::Double(value.to_bits())
}

fn network(component: &str, value: NbtSummaryValue) -> RegistryElementData {
    let value = compound(vec![
        ("asset_name", string("quartz")),
        (
            "description",
            compound(vec![
                ("text", string("x")),
                (
                    "hover_event",
                    compound(vec![
                        ("action", string("show_item")),
                        ("id", string("stone")),
                        ("components", compound(vec![(component, value)])),
                    ]),
                ),
            ]),
        ),
    ]);
    let bytes = encode_nbt_summary(&value, 1 << 20).unwrap();
    decode_network_registry_element(&bytes).expect("numeric fixture must parse as network NBT")
}

fn field<'a>(value: &'a NbtSummaryValue, name: &str) -> &'a NbtSummaryValue {
    let NbtSummaryValue::Compound(entries) = value else {
        panic!("expected compound containing {name}")
    };
    &entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap()
        .value
}

fn encoded_component(data: &RegistryElementData, component: &str) -> NbtSummaryValue {
    let decoded = decode_exact_synchronized_registry_element(REGISTRY, data).unwrap();
    let output = decoded.encode_network(&PlayerLookup).unwrap();
    assert_eq!(field(&output, "asset_name"), &string("quartz"));
    let description = field(&output, "description");
    assert_eq!(field(description, "text"), &string("x"));
    let hover = field(description, "hover_event");
    assert_eq!(field(hover, "action"), &string("show_item"));
    assert_eq!(field(hover, "id"), &string("minecraft:stone"));
    field(field(hover, "components"), component).clone()
}

fn assert_float(value: &NbtSummaryValue, expected: f32) {
    let NbtSummaryValue::Float(bits) = value else {
        panic!("Codec.FLOAT must publish a float NBT tag")
    };
    if expected.is_nan() {
        assert!(f32::from_bits(*bits).is_nan());
    } else {
        assert_eq!(*bits, expected.to_bits());
    }
}

fn assert_double(value: &NbtSummaryValue, expected: f64) {
    let NbtSummaryValue::Double(bits) = value else {
        panic!("Codec.DOUBLE must publish a double NBT tag")
    };
    if expected.is_nan() {
        assert!(f64::from_bits(*bits).is_nan());
    } else {
        assert_eq!(*bits, expected.to_bits());
    }
}

fn assert_map_numbers(value: &NbtSummaryValue, x: f64, z: f64, rotation: f32) {
    let marker = field(value, "marker");
    assert_eq!(field(marker, "type"), &string("minecraft:player"));
    assert_double(field(marker, "x"), x);
    assert_double(field(marker, "z"), z);
    assert_float(field(marker, "rotation"), rotation);
}

#[test]
fn component_numeric_families_both_json_sources_publish_finite_numbers() {
    for (component, value) in [
        (CUSTOM_MODEL_DATA, json!({"floats": [1.5]})),
        (FOOD, json!({"nutrition": 4, "saturation": 1.5})),
        (
            MAP_DECORATIONS,
            json!({"marker": {
                "type": "player", "x": -2.25, "z": 3.5, "rotation": 1.5
            }}),
        ),
    ] {
        let bytes = serde_json::to_vec(&json!({
            "asset_name": "quartz", "description": {"text": "x", "hover_event": {
                "action": "show_item", "id": "stone", "components": {(component): value}
            }}
        }))
        .unwrap();
        for data in [
            decode_direct_registry_json(&bytes).unwrap(),
            decode_client_known_pack_registry_json(&bytes).unwrap(),
        ] {
            let output = encoded_component(&data, component);
            match component {
                CUSTOM_MODEL_DATA => {
                    let NbtSummaryValue::List(values) = field(&output, "floats") else {
                        panic!("expected custom model data float list")
                    };
                    assert_eq!(values.len(), 1);
                    assert_float(&values[0], 1.5);
                }
                FOOD => {
                    assert_eq!(field(&output, "nutrition"), &NbtSummaryValue::Int(4));
                    assert_float(field(&output, "saturation"), 1.5);
                }
                MAP_DECORATIONS => assert_map_numbers(&output, -2.25, 3.5, 1.5),
                _ => unreachable!(),
            }
        }
    }
}

const ADDITIONAL_FLOAT_COMPONENTS: [&str; 4] = [
    "minecraft:tool",
    "minecraft:blocks_attacks",
    "minecraft:kinetic_weapon",
    "minecraft:break_sound",
];

fn additional_float_fixture(component: &str, value: f32) -> NbtSummaryValue {
    match component {
        "minecraft:tool" => compound(vec![
            ("rules", NbtSummaryValue::List(vec![])),
            ("default_mining_speed", float(value)),
        ]),
        "minecraft:blocks_attacks" => compound(vec![
            (
                "damage_reductions",
                NbtSummaryValue::List(vec![compound(vec![
                    ("base", float(value)),
                    ("factor", float(value)),
                ])]),
            ),
            (
                "item_damage",
                compound(vec![
                    ("threshold", float(1.0)),
                    ("base", float(value)),
                    ("factor", float(value)),
                ]),
            ),
        ]),
        "minecraft:kinetic_weapon" => {
            let condition = compound(vec![
                ("max_duration_ticks", NbtSummaryValue::Int(1)),
                ("min_speed", float(value)),
                ("min_relative_speed", float(value)),
            ]);
            compound(vec![
                ("forward_movement", float(value)),
                ("damage_multiplier", float(value)),
                ("dismount_conditions", condition.clone()),
                ("knockback_conditions", condition.clone()),
                ("damage_conditions", condition),
            ])
        }
        "minecraft:break_sound" => compound(vec![
            ("sound_id", string("test:custom")),
            ("range", float(value)),
        ]),
        _ => unreachable!(),
    }
}

fn assert_default_float(value: &NbtSummaryValue, name: &str, expected: f32, default: f32) {
    let NbtSummaryValue::Compound(entries) = value else {
        panic!("expected record containing optional {name}")
    };
    match entries.iter().find(|entry| entry.name == name) {
        Some(entry) => assert_float(&entry.value, expected),
        None => assert_eq!(
            expected, default,
            "only a canonical default may omit {name}"
        ),
    }
}

fn assert_additional_float_output(component: &str, output: &NbtSummaryValue, expected: f32) {
    match component {
        "minecraft:tool" => {
            assert_eq!(field(output, "rules"), &NbtSummaryValue::List(vec![]));
            assert_default_float(output, "default_mining_speed", expected, 1.0);
        }
        "minecraft:blocks_attacks" => {
            let NbtSummaryValue::List(reductions) = field(output, "damage_reductions") else {
                panic!("expected damage reduction list")
            };
            assert_eq!(reductions.len(), 1);
            let item_damage = field(output, "item_damage");
            assert_float(field(item_damage, "threshold"), 1.0);
            for record in [&reductions[0], item_damage] {
                assert_float(field(record, "base"), expected);
                assert_float(field(record, "factor"), expected);
            }
        }
        "minecraft:kinetic_weapon" => {
            assert_default_float(output, "forward_movement", expected, 0.0);
            assert_default_float(output, "damage_multiplier", expected, 1.0);
            for name in [
                "dismount_conditions",
                "knockback_conditions",
                "damage_conditions",
            ] {
                let condition = field(output, name);
                assert_eq!(
                    field(condition, "max_duration_ticks"),
                    &NbtSummaryValue::Int(1)
                );
                assert_default_float(condition, "min_speed", expected, 0.0);
                assert_default_float(condition, "min_relative_speed", expected, 0.0);
            }
        }
        "minecraft:break_sound" => {
            assert_eq!(field(output, "sound_id"), &string("test:custom"));
            assert_float(field(output, "range"), expected);
        }
        _ => unreachable!(),
    }
}

fn assert_additional_float_values(component: &str, values: &[f32]) {
    for &input in values {
        let data = network(component, additional_float_fixture(component, input));
        let expected = if input == 0.0 { 0.0 } else { input };
        assert_additional_float_output(component, &encoded_component(&data, component), expected);
    }
}

#[test]
fn component_numeric_families_additional_float_zero_and_finite_values_publish_canonically() {
    for component in ADDITIONAL_FLOAT_COMPONENTS {
        assert_additional_float_values(component, &[-0.0, 0.0, 1.5, -2.25]);
    }
}

fn double_bounds_fixture(amount: NbtSummaryValue) -> NbtSummaryValue {
    compound(vec![(
        "predicates",
        compound(vec![(
            "minecraft:attribute_modifiers",
            compound(vec![(
                "modifiers",
                compound(vec![(
                    "contains",
                    NbtSummaryValue::List(vec![compound(vec![("amount", amount)])]),
                )]),
            )]),
        )]),
    )])
}

fn encoded_double_bounds(amount: NbtSummaryValue) -> NbtSummaryValue {
    let data = network("minecraft:lock", double_bounds_fixture(amount));
    encoded_double_bounds_data(&data)
}

fn encoded_double_bounds_data(data: &RegistryElementData) -> NbtSummaryValue {
    let output = encoded_component(data, "minecraft:lock");
    let modifiers = field(
        field(
            field(&output, "predicates"),
            "minecraft:attribute_modifiers",
        ),
        "modifiers",
    );
    let NbtSummaryValue::List(contains) = field(modifiers, "contains") else {
        panic!("expected attribute modifier predicate collection")
    };
    assert_eq!(contains.len(), 1);
    field(&contains[0], "amount").clone()
}

fn assert_double_bounds_points_and_partial_ranges(values: &[f64]) {
    for &input in values {
        let expected = if input == 0.0 { 0.0 } else { input };
        assert_double(&encoded_double_bounds(double(input)), expected);
        for name in ["min", "max"] {
            let output = encoded_double_bounds(compound(vec![(name, double(input))]));
            let NbtSummaryValue::Compound(fields) = &output else {
                panic!("expected a partial double bounds compound")
            };
            assert_eq!(fields.len(), 1);
            assert_double(field(&output, name), expected);
        }
    }
}

#[test]
fn component_numeric_families_double_bounds_zero_and_finite_values_publish_canonically() {
    assert_double_bounds_points_and_partial_ranges(&[-0.0, 0.0, 1.5, -2.25]);
}
