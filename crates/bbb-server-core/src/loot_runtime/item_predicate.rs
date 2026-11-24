use std::borrow::Cow;

use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::DataComponentPatchSummary;
use bbb_protocol::PersistentRegistryTagMembers;
use bbb_registry_codec::{PersistentItemStackHolderResolver, UnavailableHolderResolver};
use bbb_server_datapack::loot::{
    ComponentPredicate, EnchantmentPredicate, HolderSet, IntBounds, ItemPredicate,
};
use bbb_server_datapack::recipe::ItemTagIndex;

use super::{unsupported_field, LootRuntimeReason, LootTool};

#[cfg(test)]
mod tests;

/// `ItemPredicate.test` against the tool, with item tags from its registry
/// publication or the generated 26.1 membership when no publication is available.
/// The first false term answers (`ItemPredicate.java:27-32`,
/// `DataComponentMatchers.java:33-45`).
pub(crate) fn item_predicate_matches(
    predicate: &ItemPredicate,
    tool: &LootTool,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> std::result::Result<bool, LootRuntimeReason> {
    matches_with_tags(predicate, tool, None, resolver)
}

/// The same with item tags from a loaded registry generation.
pub fn matches_item_predicate(
    predicate: &ItemPredicate,
    tool: &LootTool,
    item_tags: &ItemTagIndex,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> std::result::Result<bool, LootRuntimeReason> {
    matches_with_tags(predicate, tool, Some(item_tags), resolver)
}

fn matches_with_tags(
    predicate: &ItemPredicate,
    tool: &LootTool,
    item_tags: Option<&ItemTagIndex>,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> std::result::Result<bool, LootRuntimeReason> {
    if let Some(items) = &predicate.items {
        // An empty stack is `minecraft:air`.
        let content_registry = resolver.content_registry();
        let item = tool.item_resource_id_with_registry(content_registry)?;
        if !items.contains(item, |tag| match item_tags {
            Some(tags) => Ok(vanilla_item_type_for_resource_id(item)
                .map(|entry| entry.id)
                .or_else(|| {
                    content_registry
                        .item_by_resource_id(item)
                        .map(|entry| entry.id)
                })
                .is_some_and(|id| tags.contains(tag, id))),
            None => resolver
                .tag_contains("minecraft:item", tag, item)
                .map_or_else(|| item_tag_contains(tag, item), Ok),
        })? {
            return Ok(false);
        }
    }
    if !predicate.count.matches(tool.count()) {
        return Ok(false);
    }
    let get = |type_id| {
        let id = bbb_protocol::data_component_types::vanilla_data_component_type(type_id)
            .expect("known component type")
            .resource_id;
        match component_stack(tool, id)? {
            Some(stack) => stack
                .effective_component(type_id, resolver)
                .map_err(|error| unsupported_field("match_tool", error.to_string())),
            None => Ok(None),
        }
    };
    let has = |type_id| {
        let id = bbb_protocol::data_component_types::vanilla_data_component_type(type_id)
            .expect("known component type")
            .resource_id;
        match component_stack(tool, id)? {
            Some(stack) => stack
                .has_component(type_id)
                .map_err(|error| unsupported_field("match_tool", error.to_string())),
            None => Ok(false),
        }
    };
    if !component_predicates_match(
        &predicate.components,
        &predicate.predicates,
        &ComponentReads {
            get: &get,
            has: &has,
        },
        item_tags,
        resolver,
    )? {
        return Ok(false);
    }
    Ok(true)
}

pub(super) struct ComponentReads<'a> {
    pub(super) get:
        &'a dyn Fn(i32) -> Result<Option<Cow<'a, DataComponentPatchSummary>>, LootRuntimeReason>,
    pub(super) has: &'a dyn Fn(i32) -> Result<bool, LootRuntimeReason>,
}

pub(super) fn component_predicates_match(
    exact: &[(String, bbb_protocol::NbtSummaryValue)],
    partial: &[ComponentPredicate],
    reads: &ComponentReads<'_>,
    item_tags: Option<&ItemTagIndex>,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<bool, LootRuntimeReason> {
    for (id, expected) in exact {
        let component =
            bbb_protocol::data_component_types::vanilla_data_component_type_for_resource_id(id)
                .ok_or_else(|| {
                    unsupported_field("match_tool", format!("unknown component {id}"))
                })?;
        let term = match (reads.get)(component.type_id)? {
            Some(actual) => {
                match bbb_registry_codec::read_item_component(component.type_id, expected, resolver)
                {
                    Ok(expected) => {
                        bbb_protocol::packets::data_components::fields::component_values_equal(
                            component.type_id,
                            &expected,
                            &actual,
                        )
                        .map_err(|error| unsupported_field("match_tool", error.to_string()))?
                    }
                    Err(_) => false,
                }
            }
            None => false,
        };
        if !term {
            return Ok(false);
        }
    }
    for component in partial {
        let term = match component {
            ComponentPredicate::CustomData(expected) => {
                custom_data_predicate_matches(expected, reads)?
            }
            ComponentPredicate::Damage { durability, damage } => {
                damage_predicate_matches(durability, damage, reads)?
            }
            ComponentPredicate::PotionContents(potions) => {
                potions_predicate_matches(potions, reads, resolver)?
            }
            ComponentPredicate::Enchantments(terms) => {
                enchantments_predicate_matches(terms, reads, resolver, false)?
            }
            ComponentPredicate::StoredEnchantments(terms) => {
                enchantments_predicate_matches(terms, reads, resolver, true)?
            }
            ComponentPredicate::Container { .. } | ComponentPredicate::BundleContents { .. } => {
                contents_predicate_matches(component, reads, item_tags, resolver)?
            }
            ComponentPredicate::WritableBook { pages } => {
                let actual = (reads.get)(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)?;
                match actual {
                    Some(actual) => match pages {
                        Some(pages) => pages
                            .matches(&actual.writable_book_pages, |expected, actual| {
                                Ok::<_, LootRuntimeReason>(expected == actual)
                            })?,
                        None => true,
                    },
                    None => false,
                }
            }
            ComponentPredicate::WrittenBook {
                pages,
                author,
                title,
                generation,
                resolved,
            } => {
                let actual = (reads.get)(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)?;
                match actual {
                    Some(actual) => match &actual.written_book {
                        Some(book) => {
                            author
                                .as_ref()
                                .is_none_or(|expected| expected == &book.author)
                                && title
                                    .as_ref()
                                    .is_none_or(|expected| expected == &book.title)
                                && generation.matches(book.generation)
                                && resolved.is_none_or(|expected| expected == book.resolved)
                                && match pages {
                                    Some(pages) => pages.matches(
                                        &actual.written_book_pages_nbt,
                                        |expected, actual| {
                                            actual.as_ref().map_or(Ok(false), |actual| {
                                                bbb_protocol::component_codec_values_equal(
                                                    expected, actual,
                                                )
                                                .map_err(|error| {
                                                    unsupported_field(
                                                        "match_tool",
                                                        error.to_string(),
                                                    )
                                                })
                                            })
                                        },
                                    )?,
                                    None => true,
                                }
                        }
                        None => false,
                    },
                    None => false,
                }
            }
            ComponentPredicate::FireworkExplosion(expected) => {
                let actual = (reads.get)(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID)?;
                actual.is_some_and(|actual| {
                    actual.firework_explosion.as_ref().is_some_and(|explosion| {
                        expected.matches(
                            explosion.shape,
                            explosion.has_twinkle,
                            explosion.has_trail,
                        )
                    })
                })
            }
            ComponentPredicate::Fireworks {
                explosions,
                flight_duration,
            } => {
                let actual = (reads.get)(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
                )?;
                match actual {
                    Some(actual) => {
                        flight_duration.matches(actual.fireworks_flight_duration.unwrap_or(0))
                            && match explosions {
                                Some(explosions) => explosions.matches(
                                    &actual.fireworks_explosions,
                                    |expected, actual| {
                                        Ok::<_, LootRuntimeReason>(expected.matches(
                                            actual.shape,
                                            actual.has_twinkle,
                                            actual.has_trail,
                                        ))
                                    },
                                )?,
                                None => true,
                            }
                    }
                    None => false,
                }
            }
            ComponentPredicate::Trim { .. }
            | ComponentPredicate::JukeboxPlayable { .. }
            | ComponentPredicate::VillagerVariant(_) => {
                registry_value_predicate_matches(component, reads, resolver)?
            }
            ComponentPredicate::AnyValue(id) => any_value_matches(id, reads)?,
            ComponentPredicate::AttributeModifiers { modifiers } => {
                let actual = (reads.get)(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID)?;
                match actual {
                    Some(actual) => match modifiers {
                        Some(modifiers) => modifiers.matches(&actual.attribute_modifiers, |expected, actual| {
                            Ok::<_, LootRuntimeReason>(expected.id.as_ref().is_none_or(|id| id == &actual.modifier_id)
                                && expected.amount.matches(f64::from_bits(actual.amount_bits))
                                && expected.operation.is_none_or(|operation| operation == actual.operation_id)
                                && expected.slot.is_none_or(|slot| slot == actual.slot_id)
                                && match &expected.attribute {
                                    Some(attributes) => holder_set_contains_id(
                                        attributes, "minecraft:attribute", actual.attribute_id, resolver,
                                        |id| bbb_protocol::attribute_types::vanilla_attribute_type_for_resource_id(id).map(|entry| entry.id),
                                    )?,
                                    None => true,
                                })
                        })?,
                        None => true,
                    },
                    None => false,
                }
            }
        };
        if !term {
            return Ok(false);
        }
    }
    Ok(true)
}

fn registry_value_predicate_matches(
    predicate: &ComponentPredicate,
    reads: &ComponentReads<'_>,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> std::result::Result<bool, LootRuntimeReason> {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
    };
    let type_id = match predicate {
        ComponentPredicate::Trim { .. } => VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        ComponentPredicate::JukeboxPlayable { .. } => {
            VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID
        }
        ComponentPredicate::VillagerVariant(_) => VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
        _ => unreachable!("registry value predicate"),
    };
    let Some(actual) = (reads.get)(type_id)? else {
        return Ok(false);
    };
    let matches = |set: Option<&HolderSet>, registry, holder: Option<i32>| match set {
        None => Ok(true),
        Some(set) => match holder {
            Some(holder) => holder_set_contains_id(set, registry, holder, resolver, |id| {
                if registry == "minecraft:villager_type" {
                    bbb_facts::VillagerModelType::for_resource_id(id)
                        .map(|variant| variant.vanilla_registry_index() as i32)
                } else {
                    resolver.holder_id(registry, id)
                }
            }),
            None => Ok(false),
        },
    };
    match predicate {
        ComponentPredicate::Trim { material, pattern } => Ok(matches(
            material.as_ref(),
            "minecraft:trim_material",
            actual.armor_trim_material_id,
        )? && matches(
            pattern.as_ref(),
            "minecraft:trim_pattern",
            actual.armor_trim_pattern_id,
        )?),
        ComponentPredicate::JukeboxPlayable { song } => matches(
            song.as_ref(),
            "minecraft:jukebox_song",
            actual.jukebox_song_id,
        ),
        ComponentPredicate::VillagerVariant(variants) => matches(
            Some(variants),
            "minecraft:villager_type",
            actual.villager_variant_id,
        ),
        _ => unreachable!("registry value predicate"),
    }
}

fn contents_predicate_matches(
    predicate: &ComponentPredicate,
    reads: &ComponentReads<'_>,
    item_tags: Option<&ItemTagIndex>,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> std::result::Result<bool, LootRuntimeReason> {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
    };
    let (type_id, items) = match predicate {
        ComponentPredicate::Container { items } => {
            (VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID, items)
        }
        ComponentPredicate::BundleContents { items } => {
            (VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID, items)
        }
        _ => unreachable!("contents predicate"),
    };
    let Some(actual) = (reads.get)(type_id)? else {
        return Ok(false);
    };
    let Some(items) = items else {
        return Ok(true);
    };
    let matches = |expected: &ItemPredicate,
                   actual: &bbb_protocol::packets::ItemStackTemplateSummary| {
        let stack = bbb_server_types::item_stack::RuntimeItemStack::from_template(actual);
        let nested = LootTool {
            item_id: stack.item_id(),
            count: stack.count(),
            stack,
            ..Default::default()
        };
        matches_with_tags(expected, &nested, item_tags, resolver)
    };
    if type_id == VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID {
        items.matches(&actual.bundle_contents_items, matches)
    } else {
        items.matches(&actual.container.slots, |expected, slot| {
            matches(expected, &slot.item)
        })
    }
}

/// The stack a component read needs; a tool projected without one refuses.
fn component_stack<'a>(
    tool: &'a LootTool,
    id: &str,
) -> std::result::Result<
    Option<&'a bbb_server_types::item_stack::RuntimeItemStack>,
    LootRuntimeReason,
> {
    if tool.is_empty() {
        return Ok(None);
    }
    if tool.stack.is_empty() {
        return Err(unsupported_field(
            "match_tool",
            format!("component predicate {id} needs the tool's components"),
        ));
    }
    Ok(Some(&tool.stack))
}

fn custom_data_predicate_matches(
    expected: &bbb_protocol::NbtSummaryValue,
    reads: &ComponentReads<'_>,
) -> std::result::Result<bool, LootRuntimeReason> {
    let actual = (reads.get)(
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
    )?;
    let actual = actual.as_ref().and_then(|patch| patch.custom_data.as_ref());
    let empty = bbb_protocol::NbtSummaryValue::Compound(Vec::new());
    Ok(
        bbb_server_types::block_state_parse::block_entity_data_matches(
            Some(expected),
            Some(actual.unwrap_or(&empty)),
        ),
    )
}

fn any_value_matches(id: &str, reads: &ComponentReads<'_>) -> Result<bool, LootRuntimeReason> {
    let Some(component) =
        bbb_protocol::data_component_types::vanilla_data_component_type_for_resource_id(id)
    else {
        return Ok(false);
    };
    (reads.has)(component.type_id)
}

fn enchantments_predicate_matches(
    terms: &[EnchantmentPredicate],
    reads: &ComponentReads<'_>,
    resolver: &dyn PersistentItemStackHolderResolver,
    stored: bool,
) -> std::result::Result<bool, LootRuntimeReason> {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    };
    let type_id = if stored {
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID
    } else {
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID
    };
    let Some(patch) = (reads.get)(type_id)? else {
        return Ok(false);
    };
    let levels = if stored {
        &patch.stored_enchantments
    } else {
        &patch.enchantments
    };
    for term in terms {
        let matched = if let Some(enchantments) = &term.enchantments {
            let mut found = false;
            for entry in levels {
                if entry.level != 0 && term.levels.matches(entry.level) {
                    found |= holder_set_contains_id(
                        enchantments,
                        "minecraft:enchantment",
                        entry.holder_id,
                        resolver,
                        |id| resolver.holder_id("minecraft:enchantment", id),
                    )?;
                }
            }
            found
        } else if term.levels.min.is_some() || term.levels.max.is_some() {
            levels.iter().any(|entry| term.levels.matches(entry.level))
        } else {
            !levels.is_empty()
        };
        if !matched {
            return Ok(false);
        }
    }
    Ok(true)
}

fn damage_predicate_matches(
    durability: &IntBounds,
    damage: &IntBounds,
    reads: &ComponentReads<'_>,
) -> std::result::Result<bool, LootRuntimeReason> {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    };
    let Some(current) =
        (reads.get)(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID)?.and_then(|patch| patch.damage)
    else {
        return Ok(false);
    };
    let maximum = (reads.get)(VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID)?
        .and_then(|patch| patch.max_damage)
        .unwrap_or(0);
    Ok(durability.matches(maximum - current) && damage.matches(current))
}

fn potions_predicate_matches(
    potions: &HolderSet,
    reads: &ComponentReads<'_>,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> std::result::Result<bool, LootRuntimeReason> {
    let Some(potion) = (reads.get)(
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    )?
    .and_then(|patch| patch.potion_contents.potion_id) else {
        return Ok(false);
    };
    holder_set_contains_id(potions, "minecraft:potion", potion, resolver, |id| {
        bbb_protocol::potion_types::vanilla_potion_for_resource_id(id).map(|entry| entry.id)
    })
}

fn holder_set_contains_id(
    set: &HolderSet,
    registry: &str,
    holder: i32,
    resolver: &dyn PersistentItemStackHolderResolver,
    resolve: impl Fn(&str) -> Option<i32>,
) -> std::result::Result<bool, LootRuntimeReason> {
    match set {
        HolderSet::Ids(ids) => Ok(ids.iter().any(|id| resolve(id) == Some(holder))),
        HolderSet::Tag(tag) => match resolver.tag_members(registry, tag) {
            PersistentRegistryTagMembers::Present(ids) => {
                Ok(ids.iter().any(|id| resolve(id) == Some(holder)))
            }
            PersistentRegistryTagMembers::Absent => Ok(false),
            PersistentRegistryTagMembers::Unavailable => Err(unsupported_field(
                "match_tool",
                format!("registry {registry} tag #{tag} unavailable"),
            )),
        },
    }
}

/// Everything [`item_predicate_matches`] would refuse, whatever the tool.
pub(super) fn check_item_predicate(
    predicate: &ItemPredicate,
) -> std::result::Result<(), LootRuntimeReason> {
    if let Some(items) = &predicate.items {
        items.contains("minecraft:air", |tag| {
            item_tag_contains(tag, "minecraft:air")
        })?;
    }
    check_component_predicates(&predicate.components, &predicate.predicates)
}

pub(super) fn check_component_predicates(
    exact: &[(String, bbb_protocol::NbtSummaryValue)],
    partial: &[ComponentPredicate],
) -> Result<(), LootRuntimeReason> {
    for component in partial {
        if let ComponentPredicate::Container { items: Some(items) }
        | ComponentPredicate::BundleContents { items: Some(items) } = component
        {
            for nested in items.predicates() {
                check_item_predicate(nested)?;
            }
        }
    }
    // Term by term, so a refusal behind a term the empty reads fail is reached.
    let reads = ComponentReads {
        get: &|_| Ok(None),
        has: &|_| Ok(false),
    };
    for entry in exact {
        component_predicates_match(
            std::slice::from_ref(entry),
            &[],
            &reads,
            None,
            &UnavailableHolderResolver,
        )?;
    }
    for component in partial {
        component_predicates_match(
            &[],
            std::slice::from_ref(component),
            &reads,
            None,
            &UnavailableHolderResolver,
        )?;
    }
    Ok(())
}

/// One item tag from the generated 26.1 facts; a tag nothing declares is empty.
fn item_tag_contains(tag_id: &str, item: &str) -> std::result::Result<bool, LootRuntimeReason> {
    match vanilla_persistent_registry_tag_members_26_1("minecraft:item", tag_id) {
        PersistentRegistryTagMembers::Present(members) => {
            Ok(members.iter().any(|member| member == item))
        }
        PersistentRegistryTagMembers::Absent => Ok(false),
        PersistentRegistryTagMembers::Unavailable => Err(unsupported_field(
            "item predicate",
            format!("item tag #{tag_id} has no generated membership"),
        )),
    }
}
