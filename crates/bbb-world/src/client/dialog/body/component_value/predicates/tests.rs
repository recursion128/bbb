use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
    },
    NbtSummaryEntry, NbtSummaryValue,
};

use super::super::{decode_supported_component_value, DialogDataComponentValueState};
use super::*;

#[test]
fn decodes_adventure_compact_block_predicate_and_matchers() {
    let decoded = decode(
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
        "minecraft:can_place_on",
        &compound([
            ("blocks", string("stone")),
            (
                "state",
                compound([
                    ("axis", string("y")),
                    (
                        "age",
                        compound([("min", string("1")), ("max", string("3"))]),
                    ),
                ]),
            ),
            ("nbt", string("{foo:1}")),
            (
                "components",
                compound([
                    ("max_stack_size", NbtSummaryValue::Int(64)),
                    ("lock", compound([])),
                ]),
            ),
            (
                "predicates",
                compound([
                    (
                        "damage",
                        compound([
                            (
                                "durability",
                                compound([
                                    ("min", NbtSummaryValue::Int(1)),
                                    ("max", NbtSummaryValue::Int(4)),
                                ]),
                            ),
                            ("damage", NbtSummaryValue::Int(3)),
                        ]),
                    ),
                    ("rarity", compound([("ignored", NbtSummaryValue::Int(1))])),
                ]),
            ),
        ]),
        Some(&RegistrySet::default()),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::AdventureModePredicate { predicate }) = decoded else {
        panic!("expected adventure-mode predicate")
    };
    assert_eq!(predicate.predicates.len(), 1);
    let block = &predicate.predicates[0];
    assert_eq!(
        block
            .blocks
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.resource_id.as_str())
            .collect::<Vec<_>>(),
        ["minecraft:stone"]
    );
    assert_eq!(
        block.state,
        Some(vec![
            DialogStatePropertyMatcherState {
                name: "axis".to_string(),
                value: DialogStatePropertyValueMatcherState::Exact {
                    value: "y".to_string(),
                },
            },
            DialogStatePropertyMatcherState {
                name: "age".to_string(),
                value: DialogStatePropertyValueMatcherState::Range {
                    min: Some("1".to_string()),
                    max: Some("3".to_string()),
                },
            },
        ])
    );
    assert!(matches!(block.nbt, Some(NbtSummaryValue::Compound(_))));
    assert_eq!(block.components.exact.len(), 2);
    assert_eq!(
        block.components.exact[0].component_type_id,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID
    );
    assert!(matches!(
        block.components.exact[0].value,
        DialogDataComponentValueState::Integer { value: 64 }
    ));
    assert!(matches!(
        block.components.exact[1].value,
        DialogDataComponentValueState::Lock { .. }
    ));
    assert!(matches!(
        &block.components.partial[0],
        DialogPartialDataComponentPredicateState::Concrete {
            value: DialogDataComponentPredicateState::Damage {
                durability: DialogIntBoundsState {
                    min: Some(1),
                    max: Some(4)
                },
                damage: DialogIntBoundsState {
                    min: Some(3),
                    max: Some(3)
                }
            },
            ..
        }
    ));
    assert_eq!(
        block.components.partial[1],
        DialogPartialDataComponentPredicateState::AnyComponent {
            component_type_id: VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
            resource_id: "minecraft:rarity".to_string(),
        }
    );
}

#[test]
fn decodes_adventure_list_and_lock_recursive_item_collections() {
    let break_value = NbtSummaryValue::List(vec![
        compound([]),
        compound([("blocks", NbtSummaryValue::List(Vec::new()))]),
    ]);
    let decoded = decode(
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
        "minecraft:can_break",
        &break_value,
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::AdventureModePredicate { predicate }) = decoded else {
        panic!("expected adventure-mode predicate")
    };
    assert_eq!(predicate.predicates.len(), 2);
    assert!(predicate.predicates[0].blocks.is_none());
    assert!(predicate.predicates[1]
        .blocks
        .as_ref()
        .unwrap()
        .entries
        .is_empty());

    let lock = compound([
        (
            "items",
            NbtSummaryValue::List(vec![string("stone"), string("dirt")]),
        ),
        (
            "count",
            compound([
                ("min", NbtSummaryValue::Int(1)),
                ("max", NbtSummaryValue::Int(2)),
            ]),
        ),
        (
            "predicates",
            compound([(
                "container",
                compound([(
                    "items",
                    compound([
                        (
                            "contains",
                            NbtSummaryValue::List(vec![compound([
                                ("items", string("stone")),
                                ("count", NbtSummaryValue::Int(1)),
                                ("predicates", compound([("rarity", compound([]))])),
                            ])]),
                        ),
                        (
                            "count",
                            NbtSummaryValue::List(vec![compound([
                                ("test", compound([])),
                                ("count", compound([("min", NbtSummaryValue::Int(0))])),
                            ])]),
                        ),
                        ("size", NbtSummaryValue::Int(1)),
                    ]),
                )]),
            )]),
        ),
    ]);
    let decoded = decode(
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        "minecraft:lock",
        &lock,
        Some(&RegistrySet::default()),
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Lock { predicate }) = decoded else {
        panic!("expected lock predicate")
    };
    assert_eq!(predicate.items.as_ref().unwrap().entries.len(), 2);
    assert_eq!(predicate.count.min, Some(1));
    let DialogPartialDataComponentPredicateState::Concrete {
        value: DialogDataComponentPredicateState::Container { items: Some(items) },
        ..
    } = &predicate.components.partial[0]
    else {
        panic!("expected container predicate")
    };
    assert_eq!(items.contains.as_ref().unwrap().len(), 1);
    assert_eq!(items.count.as_ref().unwrap()[0].count.min, Some(0));
    assert_eq!(items.size.as_ref().unwrap().min, Some(1));
}

#[test]
fn decodes_all_concrete_component_predicate_types() {
    let value = compound([(
        "predicates",
        compound([
            ("damage", compound([])),
            ("enchantments", NbtSummaryValue::List(Vec::new())),
            ("stored_enchantments", NbtSummaryValue::List(Vec::new())),
            ("potion_contents", NbtSummaryValue::List(Vec::new())),
            ("custom_data", compound([])),
            ("container", compound([])),
            ("bundle_contents", compound([])),
            ("firework_explosion", compound([])),
            ("fireworks", compound([])),
            ("writable_book_content", compound([])),
            ("written_book_content", compound([])),
            ("attribute_modifiers", compound([])),
            ("trim", compound([])),
            ("jukebox_playable", compound([])),
            ("villager/variant", NbtSummaryValue::List(Vec::new())),
        ]),
    )]);
    let decoded = decode(
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        "minecraft:lock",
        &value,
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Lock { predicate }) = decoded else {
        panic!("expected lock predicate")
    };
    assert_eq!(predicate.components.partial.len(), 15);
    assert!(matches!(
        predicate.components.partial.first(),
        Some(DialogPartialDataComponentPredicateState::Concrete {
            predicate_type_id: 0,
            value: DialogDataComponentPredicateState::Damage { .. },
            ..
        })
    ));
    assert!(matches!(
        predicate.components.partial.last(),
        Some(DialogPartialDataComponentPredicateState::Concrete {
            predicate_type_id: 14,
            value: DialogDataComponentPredicateState::VillagerVariant { .. },
            ..
        })
    ));
}

#[test]
fn decodes_attribute_firework_and_book_predicate_details() {
    let nan_bits = f64::NAN.to_bits();
    let value = compound([(
        "predicates",
        compound([
            (
                "attribute_modifiers",
                compound([(
                    "modifiers",
                    compound([
                        (
                            "contains",
                            NbtSummaryValue::List(vec![compound([
                                ("attribute", NbtSummaryValue::List(Vec::new())),
                                ("id", string("example:bonus")),
                                (
                                    "amount",
                                    compound([
                                        ("min", NbtSummaryValue::Int(-1)),
                                        ("max", NbtSummaryValue::Double(nan_bits)),
                                    ]),
                                ),
                                ("operation", string("add_multiplied_base")),
                                ("slot", string("hand")),
                            ])]),
                        ),
                        ("size", NbtSummaryValue::Int(1)),
                    ]),
                )]),
            ),
            (
                "fireworks",
                compound([
                    (
                        "explosions",
                        compound([(
                            "contains",
                            NbtSummaryValue::List(vec![compound([
                                ("shape", string("star")),
                                ("has_twinkle", NbtSummaryValue::Byte(1)),
                                ("has_trail", NbtSummaryValue::Byte(0)),
                            ])]),
                        )]),
                    ),
                    (
                        "flight_duration",
                        compound([
                            ("min", NbtSummaryValue::Byte(1)),
                            ("max", NbtSummaryValue::Long(3)),
                        ]),
                    ),
                ]),
            ),
            (
                "written_book_content",
                compound([
                    (
                        "pages",
                        compound([(
                            "contains",
                            NbtSummaryValue::List(vec![compound([("text", string("page"))])]),
                        )]),
                    ),
                    ("author", string("author")),
                    ("title", string("title")),
                    ("generation", NbtSummaryValue::Int(2)),
                    ("resolved", NbtSummaryValue::Byte(1)),
                ]),
            ),
        ]),
    )]);
    let decoded = decode(
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        "minecraft:lock",
        &value,
        None,
    )
    .unwrap();
    let Some(DialogDataComponentValueState::Lock { predicate }) = decoded else {
        panic!("expected lock predicate")
    };
    let DialogPartialDataComponentPredicateState::Concrete {
        value:
            DialogDataComponentPredicateState::AttributeModifiers {
                modifiers: Some(modifiers),
            },
        ..
    } = &predicate.components.partial[0]
    else {
        panic!("expected attribute-modifiers predicate")
    };
    let modifier = &modifiers.contains.as_ref().unwrap()[0];
    assert_eq!(modifier.modifier_id.as_deref(), Some("example:bonus"));
    assert_eq!(modifier.amount.min_bits, Some((-1.0f64).to_bits()));
    assert_eq!(modifier.amount.max_bits, Some(nan_bits));
    assert_eq!(modifier.operation_id, Some(1));
    assert_eq!(modifier.slot_group_id, Some(3));

    let DialogPartialDataComponentPredicateState::Concrete {
        value:
            DialogDataComponentPredicateState::Fireworks {
                explosions: Some(explosions),
                flight_duration,
            },
        ..
    } = &predicate.components.partial[1]
    else {
        panic!("expected fireworks predicate")
    };
    assert_eq!(
        explosions.contains.as_ref().unwrap()[0].shape,
        Some(FireworkExplosionShapeSummary::Star)
    );
    assert_eq!(flight_duration.min, Some(1));
    assert_eq!(flight_duration.max, Some(3));

    let DialogPartialDataComponentPredicateState::Concrete {
        value:
            DialogDataComponentPredicateState::WrittenBookContent {
                pages: Some(pages),
                generation,
                resolved,
                ..
            },
        ..
    } = &predicate.components.partial[2]
    else {
        panic!("expected written-book predicate")
    };
    assert_eq!(pages.contains.as_ref().unwrap()[0].text, "page");
    assert_eq!(generation.min, Some(2));
    assert_eq!(*resolved, Some(true));
}

#[test]
fn rejects_malformed_adventure_and_item_predicate_codecs() {
    let registries = RegistrySet::default();
    let invalid = [
        (
            VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
            NbtSummaryValue::List(Vec::new()),
            "at least one",
        ),
        (
            VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
            compound([("state", compound([("axis", NbtSummaryValue::Int(1))]))]),
            "neither a string nor a range",
        ),
        (
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
            compound([(
                "count",
                compound([
                    ("min", NbtSummaryValue::Int(2)),
                    ("max", NbtSummaryValue::Int(1)),
                ]),
            )]),
            "swapped bounds",
        ),
        (
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
            compound([(
                "components",
                compound([("creative_slot_lock", compound([]))]),
            )]),
            "not persistent",
        ),
        (
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
            compound([("components", compound([("example:missing", compound([]))]))]),
            "unknown",
        ),
        (
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
            compound([("predicates", compound([("rarity", string("bad"))]))]),
            "is not a compound",
        ),
        (
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
            compound([(
                "predicates",
                compound([("firework_explosion", compound([("shape", string("bad"))]))]),
            )]),
            "unknown",
        ),
        (
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
            compound([(
                "predicates",
                compound([(
                    "container",
                    compound([(
                        "items",
                        compound([("count", NbtSummaryValue::List(vec![compound([])]))]),
                    )]),
                )]),
            )]),
            "missing field test",
        ),
        (
            VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
            compound([(
                "predicates",
                compound([(
                    "attribute_modifiers",
                    compound([(
                        "modifiers",
                        compound([(
                            "contains",
                            NbtSummaryValue::List(vec![compound([(
                                "amount",
                                compound([
                                    ("min", NbtSummaryValue::Double(f64::NAN.to_bits())),
                                    ("max", NbtSummaryValue::Int(1)),
                                ]),
                            )])]),
                        )]),
                    )]),
                )]),
            )]),
            "swapped bounds",
        ),
    ];
    for (type_id, value, message) in invalid {
        let error = decode(
            type_id,
            if type_id == VANILLA_DATA_COMPONENT_LOCK_TYPE_ID {
                "minecraft:lock"
            } else {
                "minecraft:can_place_on"
            },
            &value,
            Some(&registries),
        )
        .unwrap_err();
        assert!(
            error.contains(message),
            "{error:?} did not contain {message:?}"
        );
    }
}

#[test]
fn rejects_duplicate_dynamic_matcher_keys_and_missing_registry_entries() {
    let duplicate_exact = compound([(
        "components",
        compound([
            ("max_stack_size", NbtSummaryValue::Int(1)),
            ("minecraft:max_stack_size", NbtSummaryValue::Int(2)),
        ]),
    )]);
    assert!(decode(
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        "minecraft:lock",
        &duplicate_exact,
        None,
    )
    .unwrap_err()
    .contains("duplicate"));

    let missing_registry = compound([("blocks", string("example:missing"))]);
    assert!(decode(
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
        "minecraft:can_break",
        &missing_registry,
        Some(&RegistrySet::default()),
    )
    .unwrap_err()
    .contains("missing minecraft:block resource"));
}

fn decode(
    type_id: i32,
    resource_id: &str,
    value: &NbtSummaryValue,
    registries: Option<&RegistrySet>,
) -> Result<Option<DialogDataComponentValueState>, String> {
    decode_supported_component_value(type_id, resource_id, value, registries)
}

fn compound<const N: usize>(entries: [(&str, NbtSummaryValue); N]) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        entries
            .into_iter()
            .map(|(name, value)| NbtSummaryEntry {
                name: name.to_string(),
                value,
            })
            .collect(),
    )
}

fn string(value: &str) -> NbtSummaryValue {
    NbtSummaryValue::String(value.to_string())
}
