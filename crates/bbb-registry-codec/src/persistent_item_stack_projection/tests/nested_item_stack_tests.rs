//! Round trips and refusals for the four components that nest
//! `ItemStackTemplate`s: `minecraft:container`, `minecraft:bundle_contents`,
//! `minecraft:charged_projectiles` and `minecraft:use_remainder`.

use super::*;
use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
};
use bbb_facts::item_types::{
    VANILLA_ITEM_ARROW_ID, VANILLA_ITEM_BOWL_ID, VANILLA_ITEM_CHEST_ID, VANILLA_ITEM_STONE_ID,
};
use bbb_protocol::packets::ItemStackTemplateSummary;

/// `ItemStackTemplate.MAP_CODEC` as
/// `ItemStackTemplate.CODEC` writes it: `id` always, `count` only when it is
/// not the `1` its `optionalFieldOf` defaults to.
fn template(item: &str, count: Option<i32>) -> NbtSummaryValue {
    let mut fields = vec![entry(
        "id",
        NbtSummaryValue::String(item.to_string().into()),
    )];
    if let Some(count) = count {
        fields.push(entry("count", NbtSummaryValue::Int(count)));
    }
    NbtSummaryValue::Compound(fields)
}

fn container_slot(slot: i32, item: NbtSummaryValue) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("slot", NbtSummaryValue::Int(slot)),
        entry("item", item),
    ])
}

/// `ItemContainerContents.CODEC` writes only the occupied slots, and the
/// projection rebuilds the optional wire list `fromSlots` builds from them:
/// one entry per index up to the highest occupied one
#[test]
fn container_round_trips_a_sparse_slot_list() {
    let saved = saved_stack_with_components(
        "minecraft:shulker_box",
        vec![entry(
            "minecraft:container",
            NbtSummaryValue::List(vec![
                container_slot(
                    0,
                    NbtSummaryValue::String("minecraft:stone".to_string().into()),
                ),
                container_slot(
                    2,
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "id",
                            NbtSummaryValue::String("minecraft:arrow".to_string().into()),
                        ),
                        entry("count", NbtSummaryValue::Int(16)),
                    ]),
                ),
            ]),
        )],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    let patch = &summary.component_patch;
    assert_eq!(patch.container.slot_count, Some(3));
    assert_eq!(patch.container.slots.len(), 2);
    assert_eq!(
        patch
            .container
            .slots
            .iter()
            .map(|slot| slot.slot)
            .collect::<Vec<_>>(),
        [0, 2]
    );
    assert_eq!(
        patch.container.items().cloned().collect::<Vec<_>>(),
        vec![
            ItemStackTemplateSummary {
                item_id: VANILLA_ITEM_STONE_ID,
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
            ItemStackTemplateSummary {
                item_id: VANILLA_ITEM_ARROW_ID,
                count: 16,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        ]
    );

    // The save form drops trailing empty slots (`asSlots`,
    // `ItemContainerContents`), so the second round trip is the
    // fixpoint and the first one is only reached through the wire shape.
    let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    let decoded = raw.clone();
    assert_eq!(
        crate::read_item_stack(&decoded, &crate::UnavailableHolderResolver).unwrap(),
        summary
    );
}

#[test]
fn nested_containers_round_trip_from_persistent_input() {
    let mut item = NbtSummaryValue::String("minecraft:stone".to_string().into());
    for _ in 0..8 {
        item = NbtSummaryValue::Compound(vec![
            entry(
                "id",
                NbtSummaryValue::String("minecraft:shulker_box".to_string().into()),
            ),
            entry(
                "components",
                NbtSummaryValue::Compound(vec![entry(
                    "minecraft:container",
                    NbtSummaryValue::List(vec![container_slot(0, item)]),
                )]),
            ),
        ]);
    }
    let saved = saved_stack_with_components(
        "minecraft:shulker_box",
        vec![entry(
            "minecraft:container",
            NbtSummaryValue::List(vec![container_slot(0, item)]),
        )],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    let mut patch = &summary.component_patch;
    for _ in 0..=8 {
        assert_eq!(patch.container.slot_count, Some(1));
        assert_eq!(patch.container.slots.len(), 1);
        patch = &patch.container.slots[0].item.component_patch;
    }
    let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    let decoded = raw.clone();
    assert_eq!(
        crate::read_item_stack(&decoded, &crate::UnavailableHolderResolver).unwrap(),
        summary
    );
}

/// A nested patch crosses the boundary through the same retained and typed
/// entry points the enclosing stack uses, so a nested `custom_data` is
/// projected with its own NBT.
#[test]
fn charged_projectiles_round_trip_with_a_nested_component() {
    let nested = NbtSummaryValue::Compound(vec![
        entry(
            "id",
            NbtSummaryValue::String("minecraft:arrow".to_string().into()),
        ),
        entry(
            "components",
            NbtSummaryValue::Compound(vec![entry(
                "minecraft:custom_data",
                NbtSummaryValue::Compound(vec![entry("pierce", NbtSummaryValue::Byte(1))]),
            )]),
        ),
    ]);
    let saved = saved_stack_with_components(
        "minecraft:crossbow",
        vec![entry(
            "minecraft:charged_projectiles",
            NbtSummaryValue::List(vec![nested.clone()]),
        )],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    let items = &summary.component_patch.charged_projectiles_items;
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item_id, VANILLA_ITEM_ARROW_ID);
    assert_eq!(items[0].count, 1);
    assert_eq!(
        items[0].component_patch.custom_data,
        Some(NbtSummaryValue::Compound(vec![entry(
            "pierce",
            NbtSummaryValue::Byte(1)
        )]))
    );

    let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    let decoded = raw.clone();
    let restored = crate::read_item_stack(&decoded, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(restored, summary);
    assert_eq!(
        projected_components(&raw).unwrap(),
        &NbtSummaryValue::Compound(vec![entry(
            "minecraft:charged_projectiles",
            NbtSummaryValue::List(vec![nested]),
        )])
    );
}

/// `ItemStackTemplate.CODEC` is `Codec.withAlternative(MAP_CODEC.codec(),
/// Item.CODEC, ...)`, so a bare identifier is a one-count template that has no
/// components, and the writer always emits the
/// compound.
#[test]
fn use_remainder_reads_a_bare_identifier_and_writes_a_record() {
    let saved = saved_stack_with_components(
        "minecraft:chest",
        vec![entry(
            "minecraft:use_remainder",
            NbtSummaryValue::String("minecraft:bowl".to_string().into()),
        )],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(
        summary.component_patch.use_remainder.as_deref(),
        Some(&ItemStackTemplateSummary {
            item_id: VANILLA_ITEM_BOWL_ID,
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        })
    );
    assert_eq!(summary.item_id, Some(VANILLA_ITEM_CHEST_ID));

    let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(
        projected_components(&raw).unwrap(),
        &NbtSummaryValue::Compound(vec![entry(
            "minecraft:use_remainder",
            template("minecraft:bowl", None),
        )])
    );
    let decoded = raw.clone();
    assert_eq!(
        crate::read_item_stack(&decoded, &crate::UnavailableHolderResolver).unwrap(),
        summary
    );
}

/// A nested `custom_data` is retained verbatim, so a nested patch that only
/// carries one still has to come back out of the save decoder unchanged.
#[test]
fn nested_custom_data_survives_the_persistent_decoder() {
    let mut nested_patch = added_patch(&[VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID]);
    nested_patch.custom_data = Some(NbtSummaryValue::Compound(vec![entry(
        "trial",
        NbtSummaryValue::String("ominous".to_string().into()),
    )]));
    let mut patch = added_patch(&[VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID]);
    patch.charged_projectiles_items = vec![ItemStackTemplateSummary {
        item_id: VANILLA_ITEM_ARROW_ID,
        count: 2,
        component_patch: nested_patch.into(),
    }];
    let summary = stack(VANILLA_ITEM_CHEST_ID, 1, patch);

    let raw = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    let decoded = raw.clone();
    assert_eq!(
        crate::read_item_stack(&decoded, &crate::UnavailableHolderResolver).unwrap(),
        summary
    );
}
