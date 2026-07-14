use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    mob_effect_types::{VANILLA_MOB_EFFECT_SPEED_ID, VANILLA_MOB_EFFECT_WEAKNESS_ID},
    packets::inventory::DataComponentPatchSummary,
    NbtSummaryEntry, NbtSummaryValue,
};

use super::super::{
    apply_item_patch_value, decode_supported_component_value, DialogDataComponentValueState,
};
use super::*;

#[test]
fn decodes_potion_contents_compact_holder_and_empty_full_defaults() {
    let compact = decode(&string("water")).unwrap();
    assert_eq!(compact.potion.unwrap().holder_id, 0);
    assert_eq!(compact.custom_color, None);
    assert!(compact.custom_effects.is_empty());
    assert_eq!(compact.custom_name, None);

    let empty = decode(&compound(Vec::new())).unwrap();
    assert_eq!(
        empty,
        DialogPotionContentsState {
            potion: None,
            custom_color: None,
            custom_effects: Vec::new(),
            custom_name: None,
        }
    );
}

#[test]
fn decodes_full_potion_contents_and_recursive_mob_effect_details() {
    let value = compound(vec![
        string_entry("potion", "strong_swiftness"),
        value_entry(
            "custom_color",
            NbtSummaryValue::Double((0x12_34_56 as f64).to_bits()),
        ),
        value_entry(
            "custom_effects",
            NbtSummaryValue::List(vec![
                compound(vec![
                    string_entry("id", "speed"),
                    value_entry("amplifier", NbtSummaryValue::Long(255)),
                    value_entry("duration", NbtSummaryValue::Long(-1)),
                    value_entry("ambient", NbtSummaryValue::Int(2)),
                    value_entry("show_particles", NbtSummaryValue::Byte(0)),
                    value_entry(
                        "hidden_effect",
                        compound(vec![
                            value_entry("amplifier", NbtSummaryValue::Short(1)),
                            value_entry("duration", NbtSummaryValue::Int(20)),
                            value_entry(
                                "hidden_effect",
                                compound(vec![value_entry("show_icon", NbtSummaryValue::Byte(0))]),
                            ),
                        ]),
                    ),
                ]),
                compound(vec![
                    string_entry("id", "weakness"),
                    value_entry("show_particles", NbtSummaryValue::Byte(0)),
                    value_entry("show_icon", NbtSummaryValue::Byte(1)),
                ]),
            ]),
        ),
        string_entry("custom_name", "example.potion"),
    ]);
    let contents = decode(&value).unwrap();
    let potion = contents.potion.as_ref().unwrap();
    assert_eq!(potion.holder_id, 15);
    assert_eq!(potion.resource_id, "minecraft:strong_swiftness");
    assert_eq!(contents.custom_color, Some(0x12_34_56));
    assert_eq!(contents.custom_name.as_deref(), Some("example.potion"));
    assert_eq!(contents.custom_effects.len(), 2);

    let first = &contents.custom_effects[0];
    assert_eq!(first.effect.holder_id, VANILLA_MOB_EFFECT_SPEED_ID);
    assert_eq!(first.details.amplifier, 255);
    assert_eq!(first.details.duration, -1);
    assert!(first.details.ambient);
    assert!(!first.details.show_particles);
    assert!(!first.details.show_icon);
    let hidden = first.details.hidden_effect.as_ref().unwrap();
    assert_eq!(hidden.amplifier, 1);
    assert_eq!(hidden.duration, 20);
    assert!(hidden.show_particles);
    assert!(hidden.show_icon);
    let deepest = hidden.hidden_effect.as_ref().unwrap();
    assert!(deepest.show_particles);
    assert!(!deepest.show_icon);

    let second = &contents.custom_effects[1];
    assert_eq!(second.effect.holder_id, VANILLA_MOB_EFFECT_WEAKNESS_ID);
    assert!(!second.details.show_particles);
    assert!(second.details.show_icon);

    let mut patch = DataComponentPatchSummary::default();
    assert!(apply_item_patch_value(
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        &DialogDataComponentValueState::PotionContents {
            contents: contents.clone(),
        },
        &mut patch,
    ));
    assert_eq!(patch.potion_id, Some(15));
    assert_eq!(patch.potion_custom_color, Some(0x12_34_56));
    assert_eq!(patch.potion_custom_effect_count, Some(2));
    assert_eq!(patch.potion_custom_effects.len(), 2);
    assert_eq!(
        patch.potion_custom_effects[0].effect_id,
        VANILLA_MOB_EFFECT_SPEED_ID
    );
    assert_eq!(patch.potion_custom_effects[0].amplifier, 255);
    assert_eq!(patch.potion_custom_effects[0].duration, -1);
    let hidden = patch.potion_custom_effects[0]
        .hidden_effect
        .as_ref()
        .unwrap();
    assert_eq!(hidden.duration, 20);
    assert!(!hidden.hidden_effect.as_ref().unwrap().show_icon);
    assert_eq!(patch.potion_custom_name.as_deref(), Some("example.potion"));

    assert_eq!(
        decode_supported_component_value(
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
            "minecraft:potion_contents",
            &value,
            None,
        )
        .unwrap(),
        Some(DialogDataComponentValueState::PotionContents { contents })
    );
}

#[test]
fn rejects_malformed_potion_contents_holders_effects_and_ranges() {
    for (value, expected) in [
        (
            NbtSummaryValue::List(Vec::new()),
            "is neither a compound nor a potion resource id",
        ),
        (string("missing"), "missing minecraft:potion resource"),
        (
            compound(vec![value_entry("potion", NbtSummaryValue::Int(0))]),
            "potion holder is not a resource id",
        ),
        (
            compound(vec![value_entry("custom_color", string("blue"))]),
            "custom_color is not numeric",
        ),
        (
            compound(vec![value_entry("custom_effects", NbtSummaryValue::Int(0))]),
            "custom_effects is not a list",
        ),
        (
            effects(vec![NbtSummaryValue::Int(0)]),
            "custom_effects[0] is not a compound",
        ),
        (
            effects(vec![compound(Vec::new())]),
            "custom_effects[0] is missing field id",
        ),
        (
            effects(vec![compound(vec![value_entry(
                "id",
                NbtSummaryValue::Int(0),
            )])]),
            "field id holder is not a resource id",
        ),
        (
            effects(vec![compound(vec![string_entry("id", "missing")])]),
            "missing minecraft:mob_effect resource",
        ),
        (
            effect_with(vec![value_entry("amplifier", NbtSummaryValue::Int(-1))]),
            "amplifier -1 is outside 0..=255",
        ),
        (
            effect_with(vec![value_entry("amplifier", NbtSummaryValue::Int(256))]),
            "amplifier 256 is outside 0..=255",
        ),
        (
            effect_with(vec![value_entry("ambient", string("yes"))]),
            "ambient is not a boolean",
        ),
        (
            effect_with(vec![value_entry("hidden_effect", NbtSummaryValue::Int(0))]),
            "hidden_effect is not a compound",
        ),
        (
            compound(vec![value_entry("custom_name", NbtSummaryValue::Int(0))]),
            "custom_name is not a string",
        ),
    ] {
        let error = decode(&value).unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

fn decode(value: &NbtSummaryValue) -> Result<DialogPotionContentsState, String> {
    decode_potion_contents(value, "dialog item component minecraft:potion_contents")
}

fn effects(effects: Vec<NbtSummaryValue>) -> NbtSummaryValue {
    compound(vec![value_entry(
        "custom_effects",
        NbtSummaryValue::List(effects),
    )])
}

fn effect_with(mut details: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    details.insert(0, string_entry("id", "speed"));
    effects(vec![compound(details)])
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string())
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, string(value))
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.to_string(),
        value,
    }
}
