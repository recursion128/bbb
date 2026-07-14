use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
    },
    NbtSummaryEntry,
};

use super::super::{decode_supported_component_value, DialogDataComponentValueState};
use super::*;

#[test]
fn decodes_ordered_charged_projectile_and_bundle_item_template_lists() {
    let items = NbtSummaryValue::List(vec![
        string("arrow"),
        compound(vec![
            string_entry("id", "firework_rocket"),
            value_entry("count", NbtSummaryValue::Int(3)),
            value_entry(
                "components",
                compound(vec![value_entry("repair_cost", NbtSummaryValue::Int(4))]),
            ),
        ]),
    ]);
    let charged = decode(
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        "minecraft:charged_projectiles",
        &items,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::ChargedProjectiles { items: decoded }) = charged else {
        panic!("expected charged-projectiles component");
    };
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0].resource_id, "minecraft:arrow");
    assert_eq!(decoded[0].count, 1);
    assert_eq!(decoded[1].resource_id, "minecraft:firework_rocket");
    assert_eq!(decoded[1].count, 3);
    assert_eq!(
        decoded[1].component_data.as_ref().unwrap().entries[0].value_data,
        Some(DialogDataComponentValueState::Integer { value: 4 })
    );

    let bundle = decode(
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        "minecraft:bundle_contents",
        &items,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::BundleContents { items: decoded }) = bundle else {
        panic!("expected bundle-contents component");
    };
    assert_eq!(decoded.len(), 2);
    assert_eq!(decoded[0].resource_id, "minecraft:arrow");
    assert_eq!(decoded[1].resource_id, "minecraft:firework_rocket");

    for type_id in [
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    ] {
        let decoded = decode(
            type_id,
            "minecraft:item_template_list",
            &NbtSummaryValue::IntArray(Vec::new()),
        )
        .unwrap()
        .unwrap();
        assert!(matches!(
            decoded,
            DialogDataComponentValueState::ChargedProjectiles { ref items }
                | DialogDataComponentValueState::BundleContents { ref items }
                if items.is_empty()
        ));
    }
}

#[test]
fn decodes_sparse_container_slots_and_preserves_last_duplicate() {
    let value = NbtSummaryValue::List(vec![
        slot(2, string("stone")),
        slot(0, string("diamond")),
        slot(
            2,
            compound(vec![
                string_entry("id", "dirt"),
                value_entry("count", NbtSummaryValue::Int(5)),
            ]),
        ),
    ]);
    let decoded = decode(
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
        "minecraft:container",
        &value,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Container { slots }) = decoded else {
        panic!("expected container component");
    };
    assert_eq!(slots.len(), 3);
    assert_eq!(slots[0].as_ref().unwrap().resource_id, "minecraft:diamond");
    assert!(slots[1].is_none());
    assert_eq!(slots[2].as_ref().unwrap().resource_id, "minecraft:dirt");
    assert_eq!(slots[2].as_ref().unwrap().count, 5);

    let maximum = NbtSummaryValue::List(vec![slot(255, string("stone")); 256]);
    let decoded = decode_item_stack_template_slots(
        &maximum,
        Some(&RegistrySet::default()),
        "dialog item component minecraft:container",
    )
    .unwrap();
    assert_eq!(decoded.len(), 256);
    assert_eq!(
        decoded[255].as_ref().unwrap().resource_id,
        "minecraft:stone"
    );
}

#[test]
fn rejects_malformed_item_template_lists_container_slots_and_bounds() {
    for (value, expected) in [
        (NbtSummaryValue::Int(0), "is not a list"),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(0)]),
            "[0] failed item stack template decoding",
        ),
        (
            NbtSummaryValue::List(vec![string("air")]),
            "must not be minecraft:air",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![
                string_entry("id", "stone"),
                value_entry("count", NbtSummaryValue::Int(100)),
            ])]),
            "count 100 is outside 1..=99",
        ),
    ] {
        let error = decode_item_stack_template_list(
            &value,
            Some(&RegistrySet::default()),
            "dialog item component minecraft:bundle_contents",
        )
        .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }

    let too_many = NbtSummaryValue::List(vec![slot(0, string("stone")); 257]);
    for (value, expected) in [
        (NbtSummaryValue::Int(0), "is not a list"),
        (
            NbtSummaryValue::List(vec![NbtSummaryValue::Int(0)]),
            "[0] is not a compound",
        ),
        (
            NbtSummaryValue::List(vec![compound(Vec::new())]),
            "missing field slot",
        ),
        (
            NbtSummaryValue::List(vec![compound(vec![value_entry(
                "slot",
                NbtSummaryValue::Int(0),
            )])]),
            "missing field item",
        ),
        (
            NbtSummaryValue::List(vec![slot(-1, string("stone"))]),
            "slot -1 is outside 0..=255",
        ),
        (
            NbtSummaryValue::List(vec![slot(256, string("stone"))]),
            "slot 256 is outside 0..=255",
        ),
        (
            NbtSummaryValue::List(vec![slot(0, string("air"))]),
            "must not be minecraft:air",
        ),
        (too_many, "257 slot records, maximum is 256"),
    ] {
        let error = decode_item_stack_template_slots(
            &value,
            Some(&RegistrySet::default()),
            "dialog item component minecraft:container",
        )
        .unwrap_err();
        assert!(error.contains(expected), "unexpected error: {error}");
    }
}

fn decode(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
) -> Result<Option<DialogDataComponentValueState>, String> {
    decode_supported_component_value(type_id, resource_id, value, Some(&RegistrySet::default()))
}

fn slot(slot: i32, item: NbtSummaryValue) -> NbtSummaryValue {
    compound(vec![
        value_entry("slot", NbtSummaryValue::Int(slot)),
        value_entry("item", item),
    ])
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
