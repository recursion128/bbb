use bbb_protocol::{canonical_f64_json_utf16_len, canonical_nbt_json_utf16_len};

use super::super::predicates::{
    DialogAdventureModePredicateState, DialogAttributeModifierEntryPredicateState,
    DialogBlockPredicateState, DialogCollectionPredicateState, DialogDataComponentMatchersState,
    DialogDataComponentPredicateState, DialogDoubleBoundsState, DialogEnchantmentPredicateState,
    DialogFireworkExplosionPredicateState, DialogIntBoundsState, DialogItemPredicateState,
    DialogPartialDataComponentPredicateState, DialogStatePropertyValueMatcherState,
};
use super::{
    array_len, bool_len, canonical_component_value_json_utf16_len, collection_len,
    component_json_len, object_len, registry_holder_set_len, string_len,
    validate_supported_json_numbers,
};

pub(super) fn validate_adventure_mode_predicate(
    predicate: &DialogAdventureModePredicateState,
) -> Result<(), String> {
    for block in &predicate.predicates {
        if let Some(nbt) = &block.nbt {
            canonical_nbt_json_utf16_len(nbt)?;
        }
        validate_matchers(&block.components)?;
    }
    Ok(())
}

pub(super) fn validate_item_predicate(predicate: &DialogItemPredicateState) -> Result<(), String> {
    validate_matchers(&predicate.components)
}

fn validate_matchers(matchers: &DialogDataComponentMatchersState) -> Result<(), String> {
    for predicate in &matchers.exact {
        validate_supported_json_numbers(&predicate.value)?;
    }
    for predicate in &matchers.partial {
        if let DialogPartialDataComponentPredicateState::Concrete { value, .. } = predicate {
            validate_component_predicate(value)?;
        }
    }
    Ok(())
}

fn validate_component_predicate(
    predicate: &DialogDataComponentPredicateState,
) -> Result<(), String> {
    match predicate {
        DialogDataComponentPredicateState::CustomData { value } => {
            canonical_nbt_json_utf16_len(value)?;
        }
        DialogDataComponentPredicateState::Container { items }
        | DialogDataComponentPredicateState::BundleContents { items } => {
            validate_optional_collection(items.as_ref(), validate_item_predicate)?;
        }
        DialogDataComponentPredicateState::Fireworks { explosions, .. } => {
            validate_optional_collection(explosions.as_ref(), |_| Ok(()))?;
        }
        DialogDataComponentPredicateState::WritableBookContent { pages } => {
            validate_optional_collection(pages.as_ref(), |_| Ok(()))?;
        }
        DialogDataComponentPredicateState::WrittenBookContent { pages, .. } => {
            validate_optional_collection(pages.as_ref(), |page| {
                canonical_component_value_for_text(page)
            })?;
        }
        DialogDataComponentPredicateState::AttributeModifiers { modifiers } => {
            validate_optional_collection(modifiers.as_ref(), |modifier| {
                validate_double_bounds(&modifier.amount)
            })?;
        }
        DialogDataComponentPredicateState::Damage { .. }
        | DialogDataComponentPredicateState::Enchantments { .. }
        | DialogDataComponentPredicateState::StoredEnchantments { .. }
        | DialogDataComponentPredicateState::PotionContents { .. }
        | DialogDataComponentPredicateState::FireworkExplosion { .. }
        | DialogDataComponentPredicateState::Trim { .. }
        | DialogDataComponentPredicateState::JukeboxPlayable { .. }
        | DialogDataComponentPredicateState::VillagerVariant { .. } => {}
    }
    Ok(())
}

fn canonical_component_value_for_text(text: &super::super::DialogTextState) -> Result<(), String> {
    bbb_protocol::canonical_component_json_utf16_len(&text.component)?;
    Ok(())
}

fn validate_optional_collection<T>(
    predicate: Option<&DialogCollectionPredicateState<T>>,
    validate: impl Fn(&T) -> Result<(), String> + Copy,
) -> Result<(), String> {
    let Some(predicate) = predicate else {
        return Ok(());
    };
    if let Some(contains) = &predicate.contains {
        for value in contains {
            validate(value)?;
        }
    }
    if let Some(counts) = &predicate.count {
        for count in counts {
            validate(&count.test)?;
        }
    }
    Ok(())
}

fn validate_double_bounds(bounds: &DialogDoubleBoundsState) -> Result<(), String> {
    if bounds
        .min_bits
        .into_iter()
        .chain(bounds.max_bits)
        .any(|bits| canonical_f64_json_utf16_len(bits).is_none())
    {
        return Err(
            "canonical item component patch contains a non-finite Float/Double value".to_string(),
        );
    }
    Ok(())
}

pub(super) fn adventure_mode_predicate_len(
    predicate: &DialogAdventureModePredicateState,
) -> Option<usize> {
    let lengths = predicate
        .predicates
        .iter()
        .map(block_predicate_len)
        .collect::<Option<Vec<_>>>()?;
    match lengths.as_slice() {
        [] => None,
        [single] => Some(*single),
        _ => Some(array_len(lengths.into_iter())),
    }
}

fn block_predicate_len(predicate: &DialogBlockPredicateState) -> Option<usize> {
    let mut fields = Vec::new();
    if let Some(blocks) = &predicate.blocks {
        fields.push(("blocks", registry_holder_set_len(blocks)?));
    }
    if let Some(state) = &predicate.state {
        fields.push((
            "state",
            collection_len(state.iter().map(|property| {
                string_len(&property.name)
                    .saturating_add(1)
                    .saturating_add(match &property.value {
                        DialogStatePropertyValueMatcherState::Exact { value } => string_len(value),
                        DialogStatePropertyValueMatcherState::Range { min, max } => {
                            let mut fields = Vec::new();
                            if let Some(min) = min {
                                fields.push(("min", string_len(min)));
                            }
                            if let Some(max) = max {
                                fields.push(("max", string_len(max)));
                            }
                            object_len(fields.into_iter())
                        }
                    })
            })),
        ));
    }
    if let Some(nbt) = &predicate.nbt {
        fields.push(("nbt", canonical_nbt_json_utf16_len(nbt).ok()?));
    }
    append_matchers(&mut fields, &predicate.components)?;
    Some(object_len(fields.into_iter()))
}

pub(super) fn item_predicate_len(predicate: &DialogItemPredicateState) -> Option<usize> {
    let mut fields = Vec::new();
    if let Some(items) = &predicate.items {
        fields.push(("items", registry_holder_set_len(items)?));
    }
    if !int_bounds_is_any(&predicate.count) {
        fields.push(("count", int_bounds_len(&predicate.count)));
    }
    append_matchers(&mut fields, &predicate.components)?;
    Some(object_len(fields.into_iter()))
}

fn append_matchers(
    fields: &mut Vec<(&'static str, usize)>,
    matchers: &DialogDataComponentMatchersState,
) -> Option<()> {
    if !matchers.exact.is_empty() {
        fields.push((
            "components",
            collection_len(
                matchers
                    .exact
                    .iter()
                    .map(|predicate| {
                        Some(
                            string_len(&predicate.resource_id)
                                .saturating_add(1)
                                .saturating_add(canonical_component_value_json_utf16_len(
                                    &predicate.value,
                                )?),
                        )
                    })
                    .collect::<Option<Vec<_>>>()?
                    .into_iter(),
            ),
        ));
    }
    if !matchers.partial.is_empty() {
        fields.push((
            "predicates",
            collection_len(
                matchers
                    .partial
                    .iter()
                    .map(|predicate| match predicate {
                        DialogPartialDataComponentPredicateState::Concrete {
                            resource_id,
                            value,
                            ..
                        } => Some(
                            string_len(resource_id)
                                .saturating_add(1)
                                .saturating_add(component_predicate_len(value)?),
                        ),
                        DialogPartialDataComponentPredicateState::AnyComponent {
                            resource_id,
                            ..
                        } => Some(string_len(resource_id).saturating_add(3)),
                    })
                    .collect::<Option<Vec<_>>>()?
                    .into_iter(),
            ),
        ));
    }
    Some(())
}

fn component_predicate_len(predicate: &DialogDataComponentPredicateState) -> Option<usize> {
    Some(match predicate {
        DialogDataComponentPredicateState::Damage { durability, damage } => {
            let mut fields = Vec::new();
            append_non_any_int_bounds(&mut fields, "durability", durability);
            append_non_any_int_bounds(&mut fields, "damage", damage);
            object_len(fields.into_iter())
        }
        DialogDataComponentPredicateState::Enchantments { enchantments }
        | DialogDataComponentPredicateState::StoredEnchantments { enchantments } => array_len(
            enchantments
                .iter()
                .map(enchantment_predicate_len)
                .collect::<Option<Vec<_>>>()?
                .into_iter(),
        ),
        DialogDataComponentPredicateState::PotionContents { potions } => {
            registry_holder_set_len(potions)?
        }
        DialogDataComponentPredicateState::CustomData { value } => {
            canonical_nbt_json_utf16_len(value).ok()?
        }
        DialogDataComponentPredicateState::Container { items }
        | DialogDataComponentPredicateState::BundleContents { items } => {
            optional_collection_object_len("items", items.as_ref(), item_predicate_len)?
        }
        DialogDataComponentPredicateState::FireworkExplosion { predicate } => {
            firework_explosion_predicate_len(predicate)?
        }
        DialogDataComponentPredicateState::Fireworks {
            explosions,
            flight_duration,
        } => {
            let mut fields = Vec::new();
            if let Some(explosions) = explosions {
                fields.push((
                    "explosions",
                    collection_predicate_len(explosions, firework_explosion_predicate_len)?,
                ));
            }
            append_non_any_int_bounds(&mut fields, "flight_duration", flight_duration);
            object_len(fields.into_iter())
        }
        DialogDataComponentPredicateState::WritableBookContent { pages } => {
            optional_collection_object_len("pages", pages.as_ref(), |page| Some(string_len(page)))?
        }
        DialogDataComponentPredicateState::WrittenBookContent {
            pages,
            author,
            title,
            generation,
            resolved,
        } => {
            let mut fields = Vec::new();
            if let Some(pages) = pages {
                fields.push((
                    "pages",
                    collection_predicate_len(pages, |page| component_json_len(&page.component))?,
                ));
            }
            if let Some(author) = author {
                fields.push(("author", string_len(author)));
            }
            if let Some(title) = title {
                fields.push(("title", string_len(title)));
            }
            append_non_any_int_bounds(&mut fields, "generation", generation);
            if let Some(resolved) = resolved {
                fields.push(("resolved", bool_len(*resolved)));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentPredicateState::AttributeModifiers { modifiers } => {
            optional_collection_object_len(
                "modifiers",
                modifiers.as_ref(),
                attribute_modifier_predicate_len,
            )?
        }
        DialogDataComponentPredicateState::Trim { material, pattern } => {
            let mut fields = Vec::new();
            if let Some(material) = material {
                fields.push(("material", registry_holder_set_len(material)?));
            }
            if let Some(pattern) = pattern {
                fields.push(("pattern", registry_holder_set_len(pattern)?));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentPredicateState::JukeboxPlayable { song } => {
            let mut fields = Vec::new();
            if let Some(song) = song {
                fields.push(("song", registry_holder_set_len(song)?));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentPredicateState::VillagerVariant { villager_types } => {
            registry_holder_set_len(villager_types)?
        }
    })
}

fn optional_collection_object_len<T>(
    name: &'static str,
    predicate: Option<&DialogCollectionPredicateState<T>>,
    value_len: impl Fn(&T) -> Option<usize> + Copy,
) -> Option<usize> {
    let mut fields = Vec::new();
    if let Some(predicate) = predicate {
        fields.push((name, collection_predicate_len(predicate, value_len)?));
    }
    Some(object_len(fields.into_iter()))
}

fn collection_predicate_len<T>(
    predicate: &DialogCollectionPredicateState<T>,
    value_len: impl Fn(&T) -> Option<usize> + Copy,
) -> Option<usize> {
    let mut fields = Vec::new();
    if let Some(contains) = &predicate.contains {
        fields.push((
            "contains",
            array_len(
                contains
                    .iter()
                    .map(value_len)
                    .collect::<Option<Vec<_>>>()?
                    .into_iter(),
            ),
        ));
    }
    if let Some(counts) = &predicate.count {
        fields.push((
            "count",
            array_len(
                counts
                    .iter()
                    .map(|count| {
                        Some(object_len(
                            [
                                ("test", value_len(&count.test)?),
                                ("count", int_bounds_len(&count.count)),
                            ]
                            .into_iter(),
                        ))
                    })
                    .collect::<Option<Vec<_>>>()?
                    .into_iter(),
            ),
        ));
    }
    if let Some(size) = &predicate.size {
        fields.push(("size", int_bounds_len(size)));
    }
    Some(object_len(fields.into_iter()))
}

fn enchantment_predicate_len(predicate: &DialogEnchantmentPredicateState) -> Option<usize> {
    let mut fields = Vec::new();
    if let Some(enchantments) = &predicate.enchantments {
        fields.push(("enchantments", registry_holder_set_len(enchantments)?));
    }
    append_non_any_int_bounds(&mut fields, "levels", &predicate.levels);
    Some(object_len(fields.into_iter()))
}

fn firework_explosion_predicate_len(
    predicate: &DialogFireworkExplosionPredicateState,
) -> Option<usize> {
    let mut fields = Vec::new();
    if let Some(shape) = predicate.shape {
        fields.push(("shape", string_len(shape.as_str())));
    }
    if let Some(has_twinkle) = predicate.has_twinkle {
        fields.push(("has_twinkle", bool_len(has_twinkle)));
    }
    if let Some(has_trail) = predicate.has_trail {
        fields.push(("has_trail", bool_len(has_trail)));
    }
    Some(object_len(fields.into_iter()))
}

fn attribute_modifier_predicate_len(
    predicate: &DialogAttributeModifierEntryPredicateState,
) -> Option<usize> {
    let mut fields = Vec::new();
    if let Some(attribute) = &predicate.attribute {
        fields.push(("attribute", registry_holder_set_len(attribute)?));
    }
    if let Some(modifier_id) = &predicate.modifier_id {
        fields.push(("id", string_len(modifier_id)));
    }
    if !double_bounds_is_any(&predicate.amount) {
        fields.push(("amount", double_bounds_len(&predicate.amount)?));
    }
    if let Some(operation) = &predicate.operation {
        fields.push(("operation", string_len(operation)));
    }
    if let Some(slot) = &predicate.slot_group {
        fields.push(("slot", string_len(slot)));
    }
    Some(object_len(fields.into_iter()))
}

fn append_non_any_int_bounds(
    fields: &mut Vec<(&'static str, usize)>,
    name: &'static str,
    bounds: &DialogIntBoundsState,
) {
    if !int_bounds_is_any(bounds) {
        fields.push((name, int_bounds_len(bounds)));
    }
}

fn int_bounds_is_any(bounds: &DialogIntBoundsState) -> bool {
    bounds.min.is_none() && bounds.max.is_none()
}

fn int_bounds_len(bounds: &DialogIntBoundsState) -> usize {
    if let (Some(min), Some(max)) = (bounds.min, bounds.max) {
        if min == max {
            return min.to_string().len();
        }
    }
    let mut fields = Vec::new();
    if let Some(min) = bounds.min {
        fields.push(("min", min.to_string().len()));
    }
    if let Some(max) = bounds.max {
        fields.push(("max", max.to_string().len()));
    }
    object_len(fields.into_iter())
}

fn double_bounds_is_any(bounds: &DialogDoubleBoundsState) -> bool {
    bounds.min_bits.is_none() && bounds.max_bits.is_none()
}

fn double_bounds_len(bounds: &DialogDoubleBoundsState) -> Option<usize> {
    if let (Some(min), Some(max)) = (bounds.min_bits, bounds.max_bits) {
        if min == max {
            return canonical_f64_json_utf16_len(min);
        }
    }
    let mut fields = Vec::new();
    if let Some(min) = bounds.min_bits {
        fields.push(("min", canonical_f64_json_utf16_len(min)?));
    }
    if let Some(max) = bounds.max_bits {
        fields.push(("max", canonical_f64_json_utf16_len(max)?));
    }
    Some(object_len(fields.into_iter()))
}

#[cfg(test)]
mod tests {
    use bbb_protocol::item_component_value_types::FireworkExplosionShapeSummary;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    use crate::client::dialog::DialogTextState;

    use super::super::super::holder_set::{
        DialogRegistryHolderReferenceState, DialogRegistryHolderSetState,
    };
    use super::super::super::predicates::{
        DialogCollectionCountPredicateState, DialogExactDataComponentPredicateState,
        DialogStatePropertyMatcherState,
    };
    use super::super::super::DialogDataComponentValueState;
    use super::*;

    #[test]
    fn canonicalizes_compact_and_list_adventure_mode_predicates() {
        let predicate = DialogBlockPredicateState {
            blocks: Some(holder_set(&["minecraft:stone"])),
            state: Some(vec![
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
            ]),
            nbt: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "foo".to_string(),
                value: NbtSummaryValue::Int(1),
            }])),
            components: DialogDataComponentMatchersState {
                exact: vec![DialogExactDataComponentPredicateState {
                    component_type_id: 0,
                    resource_id: "minecraft:max_stack_size".to_string(),
                    value: DialogDataComponentValueState::Integer { value: 64 },
                }],
                partial: vec![
                    DialogPartialDataComponentPredicateState::Concrete {
                        predicate_type_id: 0,
                        resource_id: "minecraft:damage".to_string(),
                        value: DialogDataComponentPredicateState::Damage {
                            durability: DialogIntBoundsState {
                                min: Some(1),
                                max: Some(4),
                            },
                            damage: DialogIntBoundsState {
                                min: Some(3),
                                max: Some(3),
                            },
                        },
                    },
                    DialogPartialDataComponentPredicateState::AnyComponent {
                        component_type_id: 0,
                        resource_id: "minecraft:rarity".to_string(),
                    },
                ],
            },
        };
        let expected = r#"{"blocks":"minecraft:stone","state":{"axis":"y","age":{"min":"1","max":"3"}},"nbt":{"foo":1},"components":{"minecraft:max_stack_size":64},"predicates":{"minecraft:damage":{"durability":{"min":1,"max":4},"damage":3},"minecraft:rarity":{}}}"#;
        let compact = DialogAdventureModePredicateState {
            predicates: vec![predicate.clone()],
        };
        assert_eq!(adventure_mode_predicate_len(&compact), Some(expected.len()));
        assert!(validate_adventure_mode_predicate(&compact).is_ok());

        let list = DialogAdventureModePredicateState {
            predicates: vec![
                predicate,
                DialogBlockPredicateState {
                    blocks: None,
                    state: None,
                    nbt: None,
                    components: DialogDataComponentMatchersState::default(),
                },
            ],
        };
        assert_eq!(
            adventure_mode_predicate_len(&list),
            Some(expected.len() + 5)
        );
    }

    #[test]
    fn canonicalizes_recursive_lock_collection_predicates() {
        let nested = DialogItemPredicateState {
            items: Some(holder_set(&["minecraft:stone"])),
            count: DialogIntBoundsState {
                min: Some(1),
                max: Some(1),
            },
            components: DialogDataComponentMatchersState {
                exact: Vec::new(),
                partial: vec![DialogPartialDataComponentPredicateState::AnyComponent {
                    component_type_id: 0,
                    resource_id: "minecraft:rarity".to_string(),
                }],
            },
        };
        let predicate = DialogItemPredicateState {
            items: Some(holder_set(&["minecraft:stone", "minecraft:dirt"])),
            count: DialogIntBoundsState {
                min: Some(1),
                max: Some(2),
            },
            components: DialogDataComponentMatchersState {
                exact: Vec::new(),
                partial: vec![DialogPartialDataComponentPredicateState::Concrete {
                    predicate_type_id: 0,
                    resource_id: "minecraft:container".to_string(),
                    value: DialogDataComponentPredicateState::Container {
                        items: Some(DialogCollectionPredicateState {
                            contains: Some(vec![nested]),
                            count: Some(vec![DialogCollectionCountPredicateState {
                                test: empty_item_predicate(),
                                count: DialogIntBoundsState {
                                    min: Some(0),
                                    max: None,
                                },
                            }]),
                            size: Some(DialogIntBoundsState {
                                min: Some(1),
                                max: Some(1),
                            }),
                        }),
                    },
                }],
            },
        };
        let expected = r#"{"items":["minecraft:stone","minecraft:dirt"],"count":{"min":1,"max":2},"predicates":{"minecraft:container":{"items":{"contains":[{"items":"minecraft:stone","count":1,"predicates":{"minecraft:rarity":{}}}],"count":[{"test":{},"count":{"min":0}}],"size":1}}}}"#;
        assert_eq!(item_predicate_len(&predicate), Some(expected.len()));
        assert!(validate_item_predicate(&predicate).is_ok());
    }

    #[test]
    fn canonicalizes_every_concrete_data_component_predicate() {
        let cases = vec![
            (
                DialogDataComponentPredicateState::Damage {
                    durability: DialogIntBoundsState::default(),
                    damage: DialogIntBoundsState {
                        min: Some(2),
                        max: Some(2),
                    },
                },
                r#"{"damage":2}"#,
            ),
            (
                DialogDataComponentPredicateState::Enchantments {
                    enchantments: vec![DialogEnchantmentPredicateState {
                        enchantments: None,
                        levels: DialogIntBoundsState::default(),
                    }],
                },
                "[{}]",
            ),
            (
                DialogDataComponentPredicateState::StoredEnchantments {
                    enchantments: Vec::new(),
                },
                "[]",
            ),
            (
                DialogDataComponentPredicateState::PotionContents {
                    potions: holder_set(&["minecraft:water"]),
                },
                r#""minecraft:water""#,
            ),
            (
                DialogDataComponentPredicateState::CustomData {
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "x".to_string(),
                        value: NbtSummaryValue::Int(1),
                    }]),
                },
                r#"{"x":1}"#,
            ),
            (
                DialogDataComponentPredicateState::Container {
                    items: Some(singleton_collection(empty_item_predicate())),
                },
                r#"{"items":{"contains":[{}]}}"#,
            ),
            (
                DialogDataComponentPredicateState::BundleContents { items: None },
                "{}",
            ),
            (
                DialogDataComponentPredicateState::FireworkExplosion {
                    predicate: DialogFireworkExplosionPredicateState {
                        shape: Some(FireworkExplosionShapeSummary::Star),
                        has_twinkle: Some(false),
                        has_trail: Some(true),
                    },
                },
                r#"{"shape":"star","has_twinkle":false,"has_trail":true}"#,
            ),
            (
                DialogDataComponentPredicateState::Fireworks {
                    explosions: Some(DialogCollectionPredicateState {
                        contains: None,
                        count: None,
                        size: None,
                    }),
                    flight_duration: DialogIntBoundsState {
                        min: Some(1),
                        max: None,
                    },
                },
                r#"{"explosions":{},"flight_duration":{"min":1}}"#,
            ),
            (
                DialogDataComponentPredicateState::WritableBookContent {
                    pages: Some(singleton_collection("page".to_string())),
                },
                r#"{"pages":{"contains":["page"]}}"#,
            ),
            (
                DialogDataComponentPredicateState::WrittenBookContent {
                    pages: Some(singleton_collection(text("page"))),
                    author: Some("Author".to_string()),
                    title: None,
                    generation: DialogIntBoundsState::default(),
                    resolved: Some(true),
                },
                r#"{"pages":{"contains":["page"]},"author":"Author","resolved":true}"#,
            ),
            (
                DialogDataComponentPredicateState::AttributeModifiers {
                    modifiers: Some(singleton_collection(
                        DialogAttributeModifierEntryPredicateState {
                            attribute: None,
                            modifier_id: Some("example:boost".to_string()),
                            amount: DialogDoubleBoundsState {
                                min_bits: Some(1.5f64.to_bits()),
                                max_bits: Some(1.5f64.to_bits()),
                            },
                            operation_id: Some(0),
                            operation: Some("add_value".to_string()),
                            slot_group_id: Some(0),
                            slot_group: Some("any".to_string()),
                        },
                    )),
                },
                r#"{"modifiers":{"contains":[{"id":"example:boost","amount":1.5,"operation":"add_value","slot":"any"}]}}"#,
            ),
            (
                DialogDataComponentPredicateState::Trim {
                    material: Some(tag("example:materials")),
                    pattern: None,
                },
                r##"{"material":"#example:materials"}"##,
            ),
            (
                DialogDataComponentPredicateState::JukeboxPlayable { song: None },
                "{}",
            ),
            (
                DialogDataComponentPredicateState::VillagerVariant {
                    villager_types: holder_set(&[]),
                },
                "[]",
            ),
        ];

        for (predicate, expected) in cases {
            assert_eq!(
                component_predicate_len(&predicate),
                Some(expected.encode_utf16().count()),
                "{predicate:?}"
            );
            assert!(validate_component_predicate(&predicate).is_ok());
        }
    }

    #[test]
    fn rejects_non_finite_attribute_modifier_bounds_recursively() {
        let predicate = DialogItemPredicateState {
            items: None,
            count: DialogIntBoundsState::default(),
            components: DialogDataComponentMatchersState {
                exact: Vec::new(),
                partial: vec![DialogPartialDataComponentPredicateState::Concrete {
                    predicate_type_id: 0,
                    resource_id: "minecraft:attribute_modifiers".to_string(),
                    value: DialogDataComponentPredicateState::AttributeModifiers {
                        modifiers: Some(DialogCollectionPredicateState {
                            contains: Some(vec![DialogAttributeModifierEntryPredicateState {
                                attribute: None,
                                modifier_id: None,
                                amount: DialogDoubleBoundsState {
                                    min_bits: Some(f64::NAN.to_bits()),
                                    max_bits: None,
                                },
                                operation_id: None,
                                operation: None,
                                slot_group_id: None,
                                slot_group: None,
                            }]),
                            count: None,
                            size: None,
                        }),
                    },
                }],
            },
        };
        assert!(validate_item_predicate(&predicate)
            .unwrap_err()
            .contains("non-finite"));
        assert_eq!(item_predicate_len(&predicate), None);
    }

    fn holder_set(resource_ids: &[&str]) -> DialogRegistryHolderSetState {
        DialogRegistryHolderSetState {
            holder_set_kind: "direct".to_string(),
            tag: None,
            entries: resource_ids
                .iter()
                .enumerate()
                .map(
                    |(holder_id, resource_id)| DialogRegistryHolderReferenceState {
                        holder_id: holder_id as i32,
                        resource_id: (*resource_id).to_string(),
                    },
                )
                .collect(),
        }
    }

    fn tag(resource_id: &str) -> DialogRegistryHolderSetState {
        DialogRegistryHolderSetState {
            holder_set_kind: "named".to_string(),
            tag: Some(resource_id.to_string()),
            entries: Vec::new(),
        }
    }

    fn empty_item_predicate() -> DialogItemPredicateState {
        DialogItemPredicateState {
            items: None,
            count: DialogIntBoundsState::default(),
            components: DialogDataComponentMatchersState::default(),
        }
    }

    fn singleton_collection<T>(value: T) -> DialogCollectionPredicateState<T> {
        DialogCollectionPredicateState {
            contains: Some(vec![value]),
            count: None,
            size: None,
        }
    }

    fn text(value: &str) -> DialogTextState {
        DialogTextState {
            text: value.to_string(),
            runs: Vec::new(),
            component: NbtSummaryValue::String(value.to_string()),
        }
    }
}
