//! `minecraft:can_break` / `minecraft:can_place_on`: the
//! `AdventureModePredicate` compact list over `BlockPredicate`, and the
//! `DataComponentMatchers` map that `BlockPredicate` flattens into its own
//! record.
//!
//! The save shape differs from the network shape in three places at once: the
//! list compacts to one bare predicate, the `nbt` predicate flattens to SNBT,
//! and the exact-component map is an ordinary component patch one level down.

use bbb_facts::block_types::VANILLA_BLOCK_STONE_ID;
use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
};
use bbb_protocol::packets::data_components::catalog::adventure_mode_predicate_tooltip_summary;
use bbb_protocol::packets::{
    AdventureModePredicateDataSummary, AdventureModePredicateSummary,
    DataComponentPredicateTypeSummary, HolderSetSummary, StatePropertyMatcherSummary,
    StatePropertyValueMatcherSummary,
};

use super::*;

const STONE_TAG: &str = "#minecraft:stone_bricks";

fn one_component(name: &str, value: NbtSummaryValue) -> NbtSummaryValue {
    saved_stack_with_components("minecraft:stone", vec![entry(name, value)])
}

fn block_predicate(blocks: &str) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![entry(
        "blocks",
        NbtSummaryValue::String(blocks.to_string().into()),
    )])
}

/// NBT -> summary -> NBT, then the same summary again from the projected NBT,
/// which is the fixpoint the save form has to reach because three of this
/// component's terms change shape between the two codecs.
fn round_trip(saved: &NbtSummaryValue) -> (DataComponentPatchSummary, DataComponentPatchSummary) {
    let summary = crate::read_item_stack(saved, &BuiltinResolver)
        .expect("a block predicate names only built-in registries");
    let projected = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(projected, saved.clone());
    let again = decode_with(&projected.clone()).unwrap();
    (
        summary.component_patch.into_inner(),
        again.component_patch.into_inner(),
    )
}

fn decode_with(
    saved: &NbtSummaryValue,
) -> std::result::Result<ItemStackSummary, ItemStackPersistentProjectionError> {
    crate::read_item_stack(saved, &BuiltinResolver)
}

fn can_break_of(patch: &DataComponentPatchSummary) -> &AdventureModePredicateDataSummary {
    patch
        .can_break_persistent
        .as_ref()
        .expect("can_break carries its predicate data")
}

/// `AdventureModePredicate.CODEC` is
/// `ExtraCodecs.compactListCodec(BlockPredicate.CODEC, nonEmptyList(...))`
/// so one predicate saves as a bare
/// compound and two save as a list, while the network form is a list in both
/// cases.
#[test]
fn one_predicate_compacts_and_two_stay_a_list() {
    let (patch, _) = round_trip(&one_component(
        "minecraft:can_break",
        block_predicate("minecraft:stone"),
    ));
    assert_eq!(can_break_of(&patch).predicates.len(), 1);
    assert_eq!(
        patch
            .can_break_persistent
            .as_ref()
            .map(adventure_mode_predicate_tooltip_summary),
        Some(AdventureModePredicateSummary {
            unknown: false,
            block_registry_ids: vec![VANILLA_BLOCK_STONE_ID],
            unresolved_block_tag_count: 0,
        })
    );

    let two = one_component(
        "minecraft:can_break",
        NbtSummaryValue::List(vec![
            block_predicate("minecraft:stone"),
            NbtSummaryValue::Compound(Vec::new()),
        ]),
    );
    let (patch, _) = round_trip(&two);
    assert_eq!(can_break_of(&patch).predicates.len(), 2);
    // The second predicate names no block, which is the `unknown` tooltip arm
    assert!(adventure_mode_predicate_tooltip_summary(can_break_of(&patch)).unknown);
}

/// Every `BlockPredicate` term at once: a block tag, a `state` map holding both
/// the exact and the ranged `ValueMatcher` arms, the flattened `nbt` predicate
/// and the two `DataComponentMatchers` maps
/// (`net/minecraft/advancements/criterion/BlockPredicate.java`,
#[test]
fn a_full_predicate_round_trips_with_the_flattened_matcher_maps() {
    let saved = one_component(
        "minecraft:can_place_on",
        NbtSummaryValue::Compound(vec![
            entry(
                "blocks",
                NbtSummaryValue::String(STONE_TAG.to_string().into()),
            ),
            entry(
                "state",
                NbtSummaryValue::Compound(vec![
                    entry(
                        "facing",
                        NbtSummaryValue::String("north".to_string().into()),
                    ),
                    entry(
                        "power",
                        NbtSummaryValue::Compound(vec![
                            entry("min", NbtSummaryValue::String("1".to_string().into())),
                            entry("max", NbtSummaryValue::String("9".to_string().into())),
                        ]),
                    ),
                ]),
            ),
            entry(
                "nbt",
                NbtSummaryValue::String("{answer:42}".to_string().into()),
            ),
            entry(
                "components",
                NbtSummaryValue::Compound(vec![entry(
                    "minecraft:max_damage",
                    NbtSummaryValue::Int(7),
                )]),
            ),
            entry(
                "predicates",
                NbtSummaryValue::Compound(vec![
                    entry(
                        "minecraft:damage",
                        NbtSummaryValue::Compound(vec![entry(
                            "durability",
                            NbtSummaryValue::Compound(vec![entry(
                                "min",
                                NbtSummaryValue::Double(0.5_f64.to_bits()),
                            )]),
                        )]),
                    ),
                    entry(
                        "minecraft:kinetic_weapon",
                        NbtSummaryValue::Compound(Vec::new()),
                    ),
                ]),
            ),
        ]),
    );
    let (patch, again) = round_trip(&saved);
    let predicate = &patch
        .can_place_on_persistent
        .as_ref()
        .expect("can_place_on carries its predicate data")
        .predicates[0];
    assert_eq!(
        predicate.blocks,
        Some(HolderSetSummary {
            registry_ids: Vec::new(),
            tag: Some("minecraft:stone_bricks".to_string()),
        })
    );
    assert_eq!(
        predicate.state,
        Some(vec![
            StatePropertyMatcherSummary {
                name: "facing".into(),
                value: StatePropertyValueMatcherSummary::Exact("north".into()),
            },
            StatePropertyMatcherSummary {
                name: "power".into(),
                value: StatePropertyValueMatcherSummary::Range {
                    min: Some("1".into()),
                    max: Some("9".into()),
                },
            },
        ])
    );
    // The save form is SNBT and the network form is the compound it parses to
    assert_eq!(
        predicate.nbt,
        Some(NbtSummaryValue::Compound(vec![entry(
            "answer",
            NbtSummaryValue::Int(42)
        )]))
    );
    let exact = predicate.exact_components.as_deref().unwrap();
    assert_eq!(
        exact.added_type_ids,
        [VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID]
    );
    assert_eq!(exact.max_damage, Some(7));
    assert_eq!(
        predicate
            .partial_components
            .iter()
            .map(|predicate| predicate.predicate_type)
            .collect::<Vec<_>>(),
        [
            DataComponentPredicateTypeSummary::Concrete(0),
            DataComponentPredicateTypeSummary::AnyComponent(
                VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID
            ),
        ]
    );
    // A partial predicate's value rides the predicate codec's own NBT on both
    // sides, so it crosses verbatim.
    assert_eq!(
        predicate.partial_components[0].wire_value,
        Some(NbtSummaryValue::Compound(vec![entry(
            "durability",
            NbtSummaryValue::Compound(vec![entry(
                "min",
                NbtSummaryValue::Double(0.5_f64.to_bits())
            )])
        )]))
    );
    assert_eq!(again.can_place_on_persistent, patch.can_place_on_persistent);
}

/// The exact-component map is `DataComponentType.VALUE_MAP_CODEC`
/// the same per-component encoder
/// the enclosing stack patch uses, so `can_break` can pin itself one level down
/// and both directions recurse through the same transcoder.
#[test]
fn nested_can_break_predicates_recurse_through_the_same_encoder() {
    let saved = one_component(
        "minecraft:can_break",
        NbtSummaryValue::Compound(vec![entry(
            "components",
            NbtSummaryValue::Compound(vec![entry(
                "minecraft:can_break",
                block_predicate("minecraft:stone"),
            )]),
        )]),
    );
    let (patch, again) = round_trip(&saved);
    let nested = can_break_of(&patch).predicates[0]
        .exact_components
        .as_deref()
        .unwrap();
    assert_eq!(
        nested.added_type_ids,
        [VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID]
    );
    assert_eq!(
        can_break_of(nested).predicates[0].blocks,
        Some(HolderSetSummary {
            registry_ids: vec![VANILLA_BLOCK_STONE_ID],
            tag: None,
        })
    );
    // The nested tooltip is the same pure function of the nested predicate.
    assert_eq!(
        nested
            .can_break_persistent
            .as_ref()
            .map(adventure_mode_predicate_tooltip_summary),
        Some(AdventureModePredicateSummary {
            unknown: false,
            block_registry_ids: vec![VANILLA_BLOCK_STONE_ID],
            unresolved_block_tag_count: 0,
        })
    );
    assert_eq!(can_break_of(&again), can_break_of(&patch));
}

/// Blocks are a built-in registry, so a predicate crosses in both directions
/// without a publication.
#[test]
fn built_in_block_predicates_cross_without_a_publication() {
    let saved = one_component("minecraft:can_break", block_predicate("minecraft:stone"));
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved
    );
}

#[test]
fn empty_exact_components_in_block_predicate_normalizes_and_omits_on_encode() {
    let saved_with_empty = one_component(
        "minecraft:can_break",
        NbtSummaryValue::Compound(vec![
            entry(
                "blocks",
                NbtSummaryValue::String("minecraft:stone".to_string().into()),
            ),
            entry("components", NbtSummaryValue::Compound(Vec::new())),
        ]),
    );
    let summary =
        crate::read_item_stack(&saved_with_empty, &crate::UnavailableHolderResolver).unwrap();
    let predicate = &can_break_of(&summary.component_patch).predicates[0];
    assert!(predicate.exact_components.is_none());
    let projected = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(
        projected,
        one_component("minecraft:can_break", block_predicate("minecraft:stone"))
    );

    // An empty `Some(DataComponentPatchSummary::default())` also omits `"components"` on encode.
    let mut with_some_empty = summary.clone();
    with_some_empty
        .component_patch
        .can_break_persistent
        .as_mut()
        .unwrap()
        .predicates[0]
        .exact_components = Some(Box::new(DataComponentPatchSummary::default()));
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&with_some_empty).unwrap(),
        projected
    );

    // Non-compound `"components"` is still rejected.
    let invalid = one_component(
        "minecraft:can_break",
        NbtSummaryValue::Compound(vec![entry("components", NbtSummaryValue::Int(42))]),
    );
    assert!(crate::read_item_stack(&invalid, &crate::UnavailableHolderResolver).is_err());
}

#[test]
fn predicate_and_template_nesting_transient_duplicate_and_byte_array_guards() {
    use bbb_protocol::packets::data_components::{
        MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH, MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
    };

    // Nested `can_break` beyond `MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH` is rejected.
    let mut nested_pred = block_predicate("minecraft:stone");
    for _ in 0..MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH {
        nested_pred = NbtSummaryValue::Compound(vec![entry(
            "components",
            NbtSummaryValue::Compound(vec![entry("minecraft:can_break", nested_pred)]),
        )]);
    }
    assert!(crate::read_item_stack(
        &one_component("minecraft:can_break", nested_pred),
        &crate::UnavailableHolderResolver
    )
    .is_err());

    // Nested `use_remainder` beyond `MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH` is rejected,
    // while exact depth `MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH` succeeds without stack overflow.
    let make_nested_remainder = |depth: usize| {
        let mut inner = NbtSummaryValue::Compound(vec![entry(
            "id",
            NbtSummaryValue::string("minecraft:stone"),
        )]);
        for _ in 1..depth {
            inner = NbtSummaryValue::Compound(vec![
                entry("id", NbtSummaryValue::string("minecraft:stone")),
                entry(
                    "components",
                    NbtSummaryValue::Compound(vec![entry("minecraft:use_remainder", inner)]),
                ),
            ]);
        }
        one_component("minecraft:use_remainder", inner)
    };
    assert!(crate::read_item_stack(
        &make_nested_remainder(MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH),
        &crate::UnavailableHolderResolver
    )
    .is_ok());
    assert!(crate::read_item_stack(
        &make_nested_remainder(MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH + 1),
        &crate::UnavailableHolderResolver
    )
    .is_err());

    // Transient component inside exact component predicate is rejected on read.
    let transient_exact = one_component(
        "minecraft:can_break",
        NbtSummaryValue::Compound(vec![entry(
            "components",
            NbtSummaryValue::Compound(vec![entry(
                "minecraft:creative_slot_lock",
                NbtSummaryValue::Compound(Vec::new()),
            )]),
        )]),
    );
    assert!(crate::read_item_stack(&transient_exact, &crate::UnavailableHolderResolver).is_err());

    // Duplicate normalized keys in `predicates` are rejected on read.
    let dup_partial = one_component(
        "minecraft:can_break",
        NbtSummaryValue::Compound(vec![entry(
            "predicates",
            NbtSummaryValue::Compound(vec![
                entry("damage", NbtSummaryValue::Compound(Vec::new())),
                entry("minecraft:damage", NbtSummaryValue::Compound(Vec::new())),
            ]),
        )]),
    );
    assert!(crate::read_item_stack(&dup_partial, &crate::UnavailableHolderResolver).is_err());

    // Large primitive arrays passed into list-accepting components are rejected without
    // eager element boxing or unbounded pre-allocation.
    let huge_bytes = NbtSummaryValue::ByteArray(vec![0_i8; 100_000]);
    for (component, payload) in [
        ("minecraft:can_break", huge_bytes.clone()),
        ("minecraft:bees", huge_bytes.clone()),
        ("minecraft:bundle_contents", huge_bytes.clone()),
        ("minecraft:dyed_color", huge_bytes.clone()),
        (
            "minecraft:written_book_content",
            NbtSummaryValue::Compound(vec![
                entry("title", NbtSummaryValue::string("t")),
                entry("author", NbtSummaryValue::string("a")),
                entry("pages", huge_bytes),
            ]),
        ),
    ] {
        assert!(
            crate::read_item_stack(
                &one_component(component, payload),
                &crate::UnavailableHolderResolver
            )
            .is_err(),
            "expected oversized byte array in {component} to fail"
        );
    }
}
