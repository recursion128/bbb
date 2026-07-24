use bbb_protocol::{
    attribute_modifier_types::{
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID, VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_TYPES,
        VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS,
    },
    attribute_types::{VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_TYPES},
    data_component_types::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
    equipment_slots::{VANILLA_EQUIPMENT_SLOT_GROUPS, VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID},
    packets::inventory::{AttributeModifierSummary, DataComponentPatchSummary},
    NbtSummaryEntry, NbtSummaryValue,
};

use super::super::{
    apply_item_patch_value, decode_supported_component_value, DialogDataComponentValueState,
};
use super::*;

#[test]
fn decodes_attribute_modifier_defaults_and_component_dispatch() {
    assert_eq!(
        decode_attribute_modifiers(&NbtSummaryValue::List(Vec::new()), "modifiers").unwrap(),
        Vec::new()
    );

    let value = NbtSummaryValue::List(vec![modifier(
        "attack_damage",
        "base_attack_damage",
        NbtSummaryValue::Double((-2.5f64).to_bits()),
        "add_value",
        None,
        None,
    )]);
    let modifiers = decode_attribute_modifiers(&value, "modifiers").unwrap();
    assert_eq!(
        modifiers,
        vec![DialogAttributeModifierState {
            attribute: DialogRegistryHolderReferenceState {
                holder_id: VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                resource_id: "minecraft:attack_damage".to_string(),
            },
            modifier_id: "minecraft:base_attack_damage".to_string(),
            amount_bits: (-2.5f64).to_bits(),
            operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            operation: "add_value".to_string(),
            slot_group_id: VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
            slot_group: "any".to_string(),
            display: DialogAttributeModifierDisplayState {
                type_id: VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
                serialized_name: "default".to_string(),
                value: None,
            },
        }]
    );
    assert_eq!(
        decode_supported_component_value(
            VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
            "minecraft:attribute_modifiers",
            &value,
            None,
        )
        .unwrap(),
        Some(DialogDataComponentValueState::AttributeModifiers { modifiers })
    );
}

#[test]
fn decodes_all_builtin_attributes_operations_slot_groups_and_displays() {
    let values = VANILLA_ATTRIBUTE_TYPES
        .iter()
        .enumerate()
        .map(|(index, attribute)| {
            let operation = &VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS
                [index % VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS.len()];
            let slot = &VANILLA_EQUIPMENT_SLOT_GROUPS[index % VANILLA_EQUIPMENT_SLOT_GROUPS.len()];
            let display = &VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_TYPES
                [index % VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_TYPES.len()];
            let display_value = if display.serialized_name == "override" {
                display_record(
                    display.serialized_name,
                    Some(NbtSummaryValue::String(format!("Override {index}"))),
                )
            } else {
                display_record(
                    display.serialized_name,
                    Some(NbtSummaryValue::Int(index as i32)),
                )
            };
            modifier(
                attribute.resource_id,
                &format!("example:modifier_{index}"),
                NbtSummaryValue::Int(index as i32),
                operation.serialized_name,
                Some(slot.serialized_name),
                Some(display_value),
            )
        })
        .collect::<Vec<_>>();
    let decoded = decode_attribute_modifiers(&NbtSummaryValue::List(values), "modifiers").unwrap();
    assert_eq!(decoded.len(), VANILLA_ATTRIBUTE_TYPES.len());
    for (index, entry) in decoded.iter().enumerate() {
        let attribute = &VANILLA_ATTRIBUTE_TYPES[index];
        let operation = &VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS
            [index % VANILLA_ATTRIBUTE_MODIFIER_OPERATIONS.len()];
        let slot = &VANILLA_EQUIPMENT_SLOT_GROUPS[index % VANILLA_EQUIPMENT_SLOT_GROUPS.len()];
        let display = &VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_TYPES
            [index % VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_TYPES.len()];
        assert_eq!(entry.attribute.holder_id, attribute.id);
        assert_eq!(entry.attribute.resource_id, attribute.resource_id);
        assert_eq!(entry.operation_id, operation.id);
        assert_eq!(entry.operation, operation.serialized_name);
        assert_eq!(entry.slot_group_id, slot.id);
        assert_eq!(entry.slot_group, slot.serialized_name);
        assert_eq!(entry.display.type_id, display.id);
        assert_eq!(entry.display.serialized_name, display.serialized_name);
        if display.serialized_name == "override" {
            let expected = format!("Override {index}");
            assert_eq!(
                entry
                    .display
                    .value
                    .as_ref()
                    .map(|value| value.text.as_str()),
                Some(expected.as_str())
            );
        } else {
            assert_eq!(entry.display.value, None);
        }
    }
}

#[test]
fn projects_ordered_attribute_modifier_summaries_and_display_text() {
    let value = NbtSummaryValue::List(vec![
        modifier(
            "attack_damage",
            "base_attack_damage",
            NbtSummaryValue::Double((-2.5f64).to_bits()),
            "add_value",
            None,
            None,
        ),
        modifier(
            "armor",
            "example:hidden",
            NbtSummaryValue::Double(0.25f64.to_bits()),
            "add_multiplied_base",
            Some("mainhand"),
            Some(display_record("hidden", None)),
        ),
        modifier(
            "movement_speed",
            "example:override",
            NbtSummaryValue::Double(1.5f64.to_bits()),
            "add_multiplied_total",
            Some("head"),
            Some(display_record(
                "override",
                Some(NbtSummaryValue::String("Boost".to_string())),
            )),
        ),
    ]);
    let decoded = decode_supported_component_value(
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        "minecraft:attribute_modifiers",
        &value,
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::AttributeModifiers { modifiers }) = decoded else {
        panic!("expected attribute modifiers component");
    };
    let override_text = modifiers[2].display.value.as_ref().unwrap().clone();

    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        &DialogDataComponentValueState::AttributeModifiers {
            modifiers: modifiers.clone(),
        },
        &mut patch,
    ));
    assert_eq!(
        patch.attribute_modifiers,
        vec![
            AttributeModifierSummary {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                modifier_id: "minecraft:base_attack_damage".to_string(),
                amount_bits: (-2.5f64).to_bits(),
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                slot_id: 0,
                display_id: 0,
                display_text: None,
                display_text_styled: None,
                display_text_nbt: None,
            },
            AttributeModifierSummary {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ARMOR_ID,
                modifier_id: "example:hidden".to_string(),
                amount_bits: 0.25f64.to_bits(),
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
                slot_id: 1,
                display_id: 1,
                display_text: None,
                display_text_styled: None,
                display_text_nbt: None,
            },
            AttributeModifierSummary {
                attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                modifier_id: "example:override".to_string(),
                amount_bits: 1.5f64.to_bits(),
                operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
                slot_id: 7,
                display_id: 2,
                display_text: Some("Boost".to_string()),
                display_text_styled: Some(override_text.runs.clone()),
                display_text_nbt: Some(override_text.component.clone()),
            },
        ]
    );

    let mut invalid_default = modifiers.clone();
    invalid_default[0].display.value = Some(override_text);
    assert!(!apply_item_patch_value(
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        &DialogDataComponentValueState::AttributeModifiers {
            modifiers: invalid_default,
        },
        &mut DataComponentPatchSummary::default(),
    ));
    let mut invalid_override = modifiers;
    invalid_override[2].display.value = None;
    assert!(!apply_item_patch_value(
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        &DialogDataComponentValueState::AttributeModifiers {
            modifiers: invalid_override,
        },
        &mut DataComponentPatchSummary::default(),
    ));
}

#[test]
fn converts_all_nbt_numeric_tags_to_codec_double_bits() {
    let float_bits = f32::NAN.to_bits();
    let double_bits = 0x7ff8_0000_0000_1234;
    let values = [
        (NbtSummaryValue::Byte(-1), (-1.0f64).to_bits()),
        (NbtSummaryValue::Short(2), 2.0f64.to_bits()),
        (NbtSummaryValue::Int(-3), (-3.0f64).to_bits()),
        (
            NbtSummaryValue::Long(9_007_199_254_740_993),
            (9_007_199_254_740_993i64 as f64).to_bits(),
        ),
        (
            NbtSummaryValue::Float(float_bits),
            f64::from(f32::from_bits(float_bits)).to_bits(),
        ),
        (NbtSummaryValue::Double(double_bits), double_bits),
    ];
    for (index, (amount, expected)) in values.into_iter().enumerate() {
        let decoded = decode_attribute_modifiers(
            &NbtSummaryValue::List(vec![modifier(
                "armor",
                &format!("example:numeric_{index}"),
                amount,
                "add_multiplied_total",
                Some("body"),
                None,
            )]),
            "modifiers",
        )
        .unwrap();
        assert_eq!(decoded[0].amount_bits, expected);
    }
}

#[test]
fn rejects_malformed_attribute_modifier_records() {
    let valid = modifier(
        "armor",
        "example:modifier",
        NbtSummaryValue::Double(1.0f64.to_bits()),
        "add_value",
        None,
        None,
    );
    let cases = [
        (compound(Vec::new()), "is not a list"),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::String("entry".to_string())]),
            "[0] is not a compound",
        ),
        (
            list_entry(vec![string_entry("id", "example:id")]),
            "is missing field type",
        ),
        (
            list_entry(vec![string_entry("type", "armor")]),
            "is missing field id",
        ),
        (
            list_entry(vec![
                string_entry("type", "armor"),
                string_entry("id", "example:id"),
            ]),
            "is missing field amount",
        ),
        (
            list_entry(vec![
                string_entry("type", "armor"),
                string_entry("id", "example:id"),
                value_entry("amount", NbtSummaryValue::Int(1)),
            ]),
            "is missing field operation",
        ),
        (
            list_entry(vec![
                value_entry("type", NbtSummaryValue::Int(1)),
                string_entry("id", "example:id"),
                value_entry("amount", NbtSummaryValue::Int(1)),
                string_entry("operation", "add_value"),
            ]),
            "field type is not an attribute resource id",
        ),
        (
            list_entry(vec![
                string_entry("type", "Invalid"),
                string_entry("id", "example:id"),
                value_entry("amount", NbtSummaryValue::Int(1)),
                string_entry("operation", "add_value"),
            ]),
            "invalid modifiers[0] attribute resource id Invalid",
        ),
        (
            list_entry(vec![
                string_entry("type", "example:missing"),
                string_entry("id", "example:id"),
                value_entry("amount", NbtSummaryValue::Int(1)),
                string_entry("operation", "add_value"),
            ]),
            "missing minecraft:attribute resource example:missing",
        ),
        (
            list_entry(vec![
                string_entry("type", "armor"),
                value_entry("id", NbtSummaryValue::Int(1)),
                value_entry("amount", NbtSummaryValue::Int(1)),
                string_entry("operation", "add_value"),
            ]),
            "field id is not a resource id",
        ),
        (
            list_entry(vec![
                string_entry("type", "armor"),
                string_entry("id", "Invalid"),
                value_entry("amount", NbtSummaryValue::Int(1)),
                string_entry("operation", "add_value"),
            ]),
            "invalid modifiers[0] modifier resource id Invalid",
        ),
        (
            list_entry(vec![
                string_entry("type", "armor"),
                string_entry("id", "example:id"),
                string_entry("amount", "one"),
                string_entry("operation", "add_value"),
            ]),
            "field amount is not numeric",
        ),
        (
            list_entry(vec![
                string_entry("type", "armor"),
                string_entry("id", "example:id"),
                value_entry("amount", NbtSummaryValue::Int(1)),
                value_entry("operation", NbtSummaryValue::Int(0)),
            ]),
            "field operation is not an operation name",
        ),
        (
            list_entry(vec![
                string_entry("type", "armor"),
                string_entry("id", "example:id"),
                value_entry("amount", NbtSummaryValue::Int(1)),
                string_entry("operation", "multiply"),
            ]),
            "unknown modifiers[0] operation multiply",
        ),
        (
            with_field(valid.clone(), "slot", NbtSummaryValue::Int(0)),
            "field slot is not an equipment slot group name",
        ),
        (
            with_field(
                valid.clone(),
                "slot",
                NbtSummaryValue::String("hands".to_string()),
            ),
            "unknown modifiers[0] equipment slot group hands",
        ),
        (
            with_field(valid.clone(), "display", NbtSummaryValue::Int(0)),
            "field display is not a compound",
        ),
        (
            with_field(valid.clone(), "display", compound(Vec::new())),
            "field display is missing field type",
        ),
        (
            with_field(
                valid.clone(),
                "display",
                compound(vec![value_entry("type", NbtSummaryValue::Int(0))]),
            ),
            "field display field type is not a display type name",
        ),
        (
            with_field(
                valid.clone(),
                "display",
                compound(vec![string_entry("type", "visible")]),
            ),
            "unknown modifiers[0] field display type visible",
        ),
        (
            with_field(
                valid.clone(),
                "display",
                compound(vec![string_entry("type", "override")]),
            ),
            "field display is missing field value",
        ),
        (
            with_field(
                valid,
                "display",
                compound(vec![
                    string_entry("type", "override"),
                    value_entry("value", NbtSummaryValue::Int(1)),
                ]),
            ),
            "field display field value is not a component",
        ),
    ];
    for (value, expected) in cases {
        let error = decode_attribute_modifiers(&value, "modifiers").unwrap_err();
        assert!(
            error.contains(expected),
            "{error:?} does not contain {expected:?}"
        );
    }
}

fn modifier(
    attribute: &str,
    id: &str,
    amount: NbtSummaryValue,
    operation: &str,
    slot: Option<&str>,
    display: Option<NbtSummaryValue>,
) -> NbtSummaryValue {
    let mut entries = vec![
        string_entry("type", attribute),
        string_entry("id", id),
        value_entry("amount", amount),
        string_entry("operation", operation),
    ];
    if let Some(slot) = slot {
        entries.push(string_entry("slot", slot));
    }
    if let Some(display) = display {
        entries.push(value_entry("display", display));
    }
    compound(entries)
}

fn display_record(name: &str, value: Option<NbtSummaryValue>) -> NbtSummaryValue {
    let mut entries = vec![string_entry("type", name)];
    if let Some(value) = value {
        entries.push(value_entry("value", value));
    }
    compound(entries)
}

fn list_entry(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::List(vec![compound(entries)])
}

fn with_field(mut value: NbtSummaryValue, name: &str, field: NbtSummaryValue) -> NbtSummaryValue {
    let NbtSummaryValue::Compound(entries) = &mut value else {
        unreachable!();
    };
    entries.push(value_entry(name, field));
    NbtSummaryValue::List(vec![value])
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, NbtSummaryValue::String(value.to_string()))
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}
