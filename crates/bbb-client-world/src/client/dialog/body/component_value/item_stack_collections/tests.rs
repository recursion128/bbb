use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
    },
    item_types::vanilla_item_type_for_resource_id,
    NbtSummaryEntry,
};

use super::super::{decode_supported_component_value, DialogDataComponentValueState};
use super::*;
use crate::RegistrySet;

#[test]
fn decodes_ordered_charged_projectile_and_bundle_item_template_lists() {
    let items = NbtSummaryValue::List(vec![
        string("arrow"),
        compound(vec![
            string_entry("id", "firework_rocket"),
            value_entry("count", NbtSummaryValue::Int(3)),
            value_entry(
                "components",
                compound(vec![
                    value_entry("repair_cost", NbtSummaryValue::Int(4)),
                    value_entry("max_damage", NbtSummaryValue::Byte(8)),
                ]),
            ),
        ]),
        compound(vec![
            string_entry("id", ":stone"),
            value_entry("components", compound(Vec::new())),
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
    assert_eq!(decoded.len(), 3);
    assert_eq!(decoded[0].resource_id, "minecraft:arrow");
    assert_eq!(decoded[0].count, 1);
    assert_eq!(decoded[1].resource_id, "minecraft:firework_rocket");
    assert_eq!(decoded[1].count, 3);
    assert_eq!(
        decoded[1].component_data.as_ref().unwrap().entries[0].value_data,
        Some(DialogDataComponentValueState::Integer { value: 4 })
    );
    assert!(decoded.iter().all(|item| item.prepared_summary.is_none()));
    assert_eq!(decoded[0].components, None);
    assert_eq!(
        decoded[1].component_data.as_ref().unwrap().entries[1].value,
        Some(NbtSummaryValue::Byte(8))
    );
    assert_eq!(decoded[2].components, Some(compound(Vec::new())));
    assert!(decoded[2]
        .component_data
        .as_ref()
        .unwrap()
        .entries
        .is_empty());
    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        &items,
        None,
    );
    assert_eq!(patch.charged_projectiles_items.len(), 3);
    assert_eq!(
        patch.charged_projectiles_items[0].item_id,
        vanilla_item_type_for_resource_id("minecraft:arrow")
            .unwrap()
            .id
    );
    assert_eq!(patch.charged_projectiles_items[1].count, 3);
    assert_eq!(
        patch.charged_projectiles_items[1]
            .component_patch
            .repair_cost,
        Some(4)
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
    assert_eq!(decoded.len(), 3);
    assert_eq!(decoded[0].resource_id, "minecraft:arrow");
    assert_eq!(decoded[1].resource_id, "minecraft:firework_rocket");
    let patch = super::super::persistent_patch(
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        &items,
        None,
    );
    assert_eq!(patch.bundle_contents_items.len(), 3);

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
                value_entry(
                    "components",
                    compound(vec![value_entry("repair_cost", NbtSummaryValue::Byte(9))]),
                ),
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
    let winning = slots[2].as_ref().unwrap();
    assert_eq!(
        winning.components,
        Some(compound(vec![value_entry(
            "repair_cost",
            NbtSummaryValue::Byte(9)
        )]))
    );
    assert_eq!(
        winning.component_data.as_ref().unwrap().entries[0].value_data,
        Some(DialogDataComponentValueState::Integer { value: 9 })
    );

    let patch =
        super::super::persistent_patch(VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID, &value, None);
    assert_eq!(patch.container.slots.len(), 2);
    assert_eq!(
        patch
            .container
            .slots
            .iter()
            .map(|entry| entry.slot)
            .collect::<Vec<_>>(),
        vec![0, 2]
    );
    assert_eq!(patch.container.slots[1].item.count, 5);

    let maximum = NbtSummaryValue::List(vec![slot(255, string("stone")); 256]);
    let Some(DialogDataComponentValueState::Container { slots: decoded }) = decode(
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
        "minecraft:container",
        &maximum,
    )
    .unwrap() else {
        panic!("expected container component")
    };
    assert_eq!(decoded.len(), 256);
    assert_eq!(
        decoded[255].as_ref().unwrap().resource_id,
        "minecraft:stone"
    );
}

#[test]
fn rejects_malformed_item_template_lists_container_slots_and_bounds() {
    for value in [
        NbtSummaryValue::Int(0),
        NbtSummaryValue::List(vec![NbtSummaryValue::Int(0)]),
        NbtSummaryValue::List(vec![string("air")]),
        NbtSummaryValue::List(vec![string("stone"); 4097]),
        NbtSummaryValue::List(vec![compound(vec![
            string_entry("id", "stone"),
            value_entry("count", NbtSummaryValue::Int(100)),
        ])]),
    ] {
        assert!(decode(
            VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
            "minecraft:item_template_collection",
            &value,
        )
        .is_err());
    }

    let too_many = NbtSummaryValue::List(vec![slot(0, string("stone")); 257]);
    for value in [
        NbtSummaryValue::Int(0),
        NbtSummaryValue::List(vec![NbtSummaryValue::Int(0)]),
        NbtSummaryValue::List(vec![compound(Vec::new())]),
        NbtSummaryValue::List(vec![compound(vec![value_entry(
            "slot",
            NbtSummaryValue::Int(0),
        )])]),
        NbtSummaryValue::List(vec![slot(-1, string("stone"))]),
        NbtSummaryValue::List(vec![slot(256, string("stone"))]),
        NbtSummaryValue::List(vec![slot(0, string("air"))]),
        too_many,
    ] {
        assert!(decode(
            VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
            "minecraft:item_template_collection",
            &value,
        )
        .is_err());
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
    NbtSummaryValue::string(value)
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    value_entry(name, string(value))
}

fn value_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}
