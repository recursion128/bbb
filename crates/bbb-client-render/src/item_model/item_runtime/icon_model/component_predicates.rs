//! Matching for the typed `minecraft:component` predicates decoded at load.

use super::*;
use bbb_client_assets::{
    holder_set_matches, int_bounds_match, AttributeModifierPredicate, EnchantmentPredicate,
    ExactAttributeModifier, ExactJukeboxPlayable, ExactTrimMaterial, ExactTrimPattern,
    FireworkExplosionPredicate, ItemComponentPredicate, ItemExactComponent, ItemModelScalar,
    ItemPredicate,
};

/// The stack a predicate inspects: the resolved stack itself or a template
/// nested in its container / bundle contents.
#[derive(Clone, Copy)]
pub(super) struct PredicateSubject<'a> {
    pub protocol_id: i32,
    pub item_profiles: Option<ItemComponentDefaultsView<'a>>,
    pub patch: Option<&'a DataComponentPatchSummary>,
    pub default_component_codec_values: Option<&'a BTreeMap<i32, serde_json::Value>>,
    pub default_max_damage: Option<i32>,
    pub default_max_stack_size: Option<i32>,
    pub item_id: &'a str,
}

impl<'a> PredicateSubject<'a> {
    pub(super) fn of_stack(ctx: &IconResolveContext<'a>) -> Self {
        Self {
            protocol_id: ctx.protocol_id,
            item_profiles: ctx.item_profiles,
            patch: ctx.component_patch,
            default_component_codec_values: ctx.default_component_codec_values,
            default_max_damage: ctx.default_max_damage,
            default_max_stack_size: ctx.default_max_stack_size,
            item_id: ctx.default_item_model_id,
        }
    }

    fn component_source(&self, type_id: i32) -> Option<&'a DataComponentPatchSummary> {
        let patch = self.patch?;
        Some(self.item_profiles.map_or(patch, |profiles| {
            profiles.component_source(Some(self.protocol_id), patch, type_id)
        }))
    }

    /// The patch when it explicitly carries `type_id` (no removal).
    fn explicit(&self, type_id: i32) -> Option<&'a DataComponentPatchSummary> {
        self.patch.filter(|patch| {
            patch.added_type_ids.contains(&type_id) && !patch.removed_type_ids.contains(&type_id)
        })
    }

    fn removed(&self, type_id: i32) -> bool {
        self.patch
            .is_some_and(|patch| patch.removed_type_ids.contains(&type_id))
    }
}

pub(super) fn component_predicate_matches(
    predicate: &ItemComponentPredicate,
    subject: &PredicateSubject<'_>,
    ctx: &IconResolveContext<'_>,
) -> bool {
    match predicate {
        ItemComponentPredicate::AnyValue(type_id) => item_stack_has_component_id(
            *type_id,
            subject.component_source(*type_id),
            subject.default_component_codec_values,
            Some(subject.item_id),
            false,
        ),
        ItemComponentPredicate::Damage { damage, durability } => {
            damage_component_predicate_state(subject).is_some_and(|(value, max)| {
                int_bounds_match(*damage, value) && int_bounds_match(*durability, max - value)
            })
        }
        ItemComponentPredicate::CustomData(expected) => {
            let empty = NbtSummaryValue::Compound(Vec::new());
            let actual = subject
                .patch
                .filter(|_| {
                    !subject.removed(component_id::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID)
                })
                .and_then(|patch| patch.custom_data.as_ref())
                .unwrap_or(&empty);
            nbt_summary_matches(expected, actual, true)
        }
        ItemComponentPredicate::AttributeModifiers { modifiers } => {
            let type_id = component_id::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID;
            let actual = subject
                .component_source(type_id)
                .map(|source| source.attribute_modifiers.as_slice())
                .unwrap_or_default();
            !subject.removed(type_id)
                && modifiers.as_ref().is_none_or(|modifiers| {
                    modifiers.matches(actual, actual.len(), |expected, actual| {
                        attribute_modifier_matches(expected, actual, ctx)
                    })
                })
        }
        ItemComponentPredicate::Container(items)
        | ItemComponentPredicate::BundleContents(items) => {
            let container = matches!(predicate, ItemComponentPredicate::Container(_));
            let type_id = if container {
                component_id::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID
            } else {
                component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID
            };
            if !item_stack_has_component_id(
                type_id,
                subject.patch,
                subject.default_component_codec_values,
                Some(subject.item_id),
                false,
            ) {
                return false;
            }
            subject.patch.is_some_and(|patch| {
                if container {
                    items.matches(
                        &patch.container.slots,
                        patch.container.slots.len(),
                        |expected, slot| item_predicate_matches(expected, &slot.item, ctx),
                    )
                } else {
                    items.matches(
                        &patch.bundle_contents_items,
                        patch.bundle_contents_items.len(),
                        |expected, item| item_predicate_matches(expected, item, ctx),
                    )
                }
            })
        }
        ItemComponentPredicate::Enchantments { stored, predicates } => {
            let type_id = enchantments_type_id(*stored);
            if subject.removed(type_id) {
                return false;
            }
            let enchantments = subject
                .patch
                .map(|patch| enchantments_of(patch, *stored))
                .unwrap_or(&[]);
            let present = !*stored
                || !enchantments.is_empty()
                || subject
                    .patch
                    .is_some_and(|patch| patch.added_type_ids.contains(&type_id))
                || subject.item_id == "minecraft:enchanted_book";
            present
                && predicates
                    .iter()
                    .all(|predicate| enchantment_predicate_matches(predicate, enchantments, ctx))
        }
        ItemComponentPredicate::FireworkExplosion(expected) => subject
            .explicit(component_id::VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID)
            .and_then(|patch| patch.firework_explosion.as_ref())
            .is_some_and(|actual| firework_explosion_predicate_matches(expected, actual)),
        ItemComponentPredicate::Fireworks {
            explosions,
            flight_duration,
        } => subject
            .explicit(component_id::VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID)
            .is_some_and(|patch| {
                let actual = &patch.fireworks_explosions;
                explosions.as_ref().is_none_or(|explosions| {
                    explosions.matches(actual, actual.len(), firework_explosion_predicate_matches)
                }) && flight_duration.is_none_or(|bounds| {
                    patch
                        .fireworks_flight_duration
                        .is_some_and(|duration| bounds.matches(duration))
                })
            }),
        ItemComponentPredicate::Trim { material, pattern } => subject
            .explicit(component_id::VANILLA_DATA_COMPONENT_TRIM_TYPE_ID)
            .is_some_and(|patch| {
                material.as_ref().is_none_or(|material| {
                    patch
                        .armor_trim_material_id
                        .and_then(|id| registry_key(ctx.trim_material_keys, id))
                        .is_some_and(|key| material.matches(key, ctx.trim_material_tags))
                }) && pattern.as_ref().is_none_or(|pattern| {
                    patch
                        .armor_trim_pattern_id
                        .and_then(|id| {
                            trim_pattern_resource_id(
                                id,
                                ctx.trim_pattern_keys,
                                ctx.registry_access.map(|(_, access)| access),
                            )
                        })
                        .is_some_and(|key| pattern.matches(key.as_ref(), ctx.trim_pattern_tags))
                })
            }),
        ItemComponentPredicate::JukeboxPlayable { song } => subject
            .explicit(component_id::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID)
            .is_some_and(|patch| {
                song.as_ref().is_none_or(|song| {
                    patch
                        .jukebox_song_id
                        .and_then(JukeboxSongRegistry::vanilla_26_1_song_id)
                        .is_some_and(|key| song.matches(key, ctx.jukebox_song_tags))
                })
            }),
        ItemComponentPredicate::PotionContents(potions) => subject
            .explicit(component_id::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID)
            .and_then(|patch| vanilla_potion(patch.potion_contents.potion_id?))
            .is_some_and(|potion| potions.matches(potion.resource_id, ctx.potion_tags)),
        ItemComponentPredicate::WritableBookContent { pages } => subject
            .explicit(component_id::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)
            .is_some_and(|patch| {
                let actual = &patch.writable_book_pages;
                pages.as_ref().is_none_or(|pages| {
                    pages.matches(actual, actual.len(), |expected, actual| expected == actual)
                })
            }),
        ItemComponentPredicate::WrittenBookContent {
            author,
            title,
            generation,
            resolved,
            pages,
        } => subject
            .explicit(component_id::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)
            .and_then(|patch| patch.written_book.as_ref())
            .is_some_and(|book| {
                author.as_ref().is_none_or(|author| *author == book.author)
                    && title.as_ref().is_none_or(|title| *title == book.title)
                    && int_bounds_match(*generation, book.generation)
                    && resolved.is_none_or(|resolved| resolved == book.resolved)
                    && pages.as_ref().is_none_or(|pages| {
                        pages.matches(&book.pages, book.pages.len(), |expected, actual| {
                            expected == actual
                        })
                    })
            }),
        ItemComponentPredicate::VillagerVariant(variants) => subject
            .explicit(component_id::VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID)
            .and_then(|patch| {
                VillagerModelType::for_vanilla_registry_id(patch.villager_variant_id?)
            })
            .is_some_and(|variant| variants.matches(variant.resource_id(), ctx.villager_type_tags)),
    }
}

fn enchantments_type_id(stored: bool) -> i32 {
    if stored {
        component_id::VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID
    } else {
        component_id::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID
    }
}

fn enchantments_of(patch: &DataComponentPatchSummary, stored: bool) -> &[ItemEnchantmentSummary] {
    if stored {
        &patch.stored_enchantments
    } else {
        &patch.enchantments
    }
}

fn enchantment_predicate_matches(
    predicate: &EnchantmentPredicate,
    enchantments: &[ItemEnchantmentSummary],
    ctx: &IconResolveContext<'_>,
) -> bool {
    match &predicate.enchantments {
        Some(set) => ctx.enchantment_keys.is_some_and(|keys| {
            enchantments.iter().any(|enchantment| {
                enchantment.level != 0
                    && registry_key(Some(keys), enchantment.holder_id)
                        .is_some_and(|key| set.matches(key, ctx.enchantment_tags))
                    && int_bounds_match(predicate.levels, enchantment.level)
            })
        }),
        None => match predicate.levels {
            Some(levels) => enchantments
                .iter()
                .any(|enchantment| levels.matches(enchantment.level)),
            None => !enchantments.is_empty(),
        },
    }
}

fn firework_explosion_predicate_matches(
    expected: &FireworkExplosionPredicate,
    actual: &FireworkExplosionSummary,
) -> bool {
    expected.shape.is_none_or(|shape| actual.shape == shape)
        && expected
            .has_twinkle
            .is_none_or(|twinkle| actual.has_twinkle == twinkle)
        && expected
            .has_trail
            .is_none_or(|trail| actual.has_trail == trail)
}

fn attribute_modifier_matches(
    expected: &AttributeModifierPredicate,
    actual: &AttributeModifierSummary,
    ctx: &IconResolveContext<'_>,
) -> bool {
    expected
        .id
        .as_ref()
        .is_none_or(|id| *id == actual.modifier_id)
        && expected.attribute.as_ref().is_none_or(|attribute| {
            registry_key(ctx.attribute_keys, actual.attribute_id)
                .is_some_and(|key| attribute.matches(key, ctx.attribute_tags))
        })
        && expected
            .amount
            .is_none_or(|amount| amount.matches(f64::from_bits(actual.amount_bits)))
        && expected
            .operation
            .is_none_or(|operation| actual.operation_id == operation)
        && expected.slot.is_none_or(|slot| actual.slot_id == slot)
}

fn item_predicate_matches(
    predicate: &ItemPredicate,
    item: &ItemStackTemplateSummary,
    ctx: &IconResolveContext<'_>,
) -> bool {
    let resource_id = usize::try_from(item.item_id)
        .ok()
        .and_then(|index| ctx.item_resource_ids?.get(index))
        .map(String::as_str)
        .or_else(|| {
            ctx.content_registry?
                .item_by_id(item.item_id)
                .map(|spec| spec.resource_id())
        });
    if predicate.items.is_some()
        && !resource_id
            .is_some_and(|id| holder_set_matches(predicate.items.as_ref(), id, ctx.item_tags))
    {
        return false;
    }
    if !int_bounds_match(predicate.count, item.count) {
        return false;
    }
    if predicate.components.is_empty() && predicate.predicates.is_empty() {
        return true;
    }
    let Some(resource_id) = resource_id else {
        return false;
    };
    let subject = PredicateSubject {
        protocol_id: item.item_id,
        item_profiles: ctx.item_profiles,
        patch: Some(&item.component_patch),
        default_component_codec_values: None,
        default_max_damage: ctx
            .item_profiles
            .and_then(|profiles| profiles.item_max_damage_for_protocol_id(item.item_id)),
        default_max_stack_size: ctx
            .item_profiles
            .map(|profiles| profiles.item_max_stack_size_for_protocol_id(item.item_id)),
        item_id: resource_id,
    };
    predicate
        .components
        .iter()
        .all(|component| exact_component_matches(component, &item.component_patch, &subject, ctx))
        && predicate
            .predicates
            .iter()
            .all(|predicate| component_predicate_matches(predicate, &subject, ctx))
}

fn exact_component_matches(
    expected: &ItemExactComponent,
    patch: &DataComponentPatchSummary,
    subject: &PredicateSubject<'_>,
    ctx: &IconResolveContext<'_>,
) -> bool {
    let removed = |type_id: i32| patch.removed_type_ids.contains(&type_id);
    let explicit = |type_id: i32| patch.added_type_ids.contains(&type_id) && !removed(type_id);
    match expected {
        ItemExactComponent::CustomName(expected) => {
            !removed(component_id::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID)
                && patch.custom_name.as_ref() == Some(expected)
        }
        ItemExactComponent::ItemName(expected) => {
            !removed(component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID)
                && patch.item_name.as_ref() == Some(expected)
        }
        ItemExactComponent::Lore(expected) => {
            !removed(component_id::VANILLA_DATA_COMPONENT_LORE_TYPE_ID) && patch.lore == *expected
        }
        ItemExactComponent::Scalar { type_id, value } => {
            scalar_component_value(
                *type_id,
                subject.component_source(*type_id).unwrap_or(patch),
                subject.default_max_stack_size,
                subject.default_max_damage,
                subject.item_id,
            )
            .as_ref()
                == Some(value)
        }
        ItemExactComponent::Unbreakable => {
            subject
                .component_source(component_id::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID)
                .is_some_and(|source| source.unbreakable)
                && !removed(component_id::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID)
        }
        ItemExactComponent::CustomData(expected) => {
            !removed(component_id::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID)
                && patch
                    .custom_data
                    .as_ref()
                    .is_some_and(|actual| nbt_summary_exact_matches(expected, actual))
        }
        ItemExactComponent::LodestoneTracker { target, tracked } => {
            explicit(component_id::VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID)
                && patch.lodestone_tracked == Some(*tracked)
                && patch.lodestone_target.as_ref() == target.as_ref()
        }
        ItemExactComponent::AttributeModifiers(expected) => {
            let type_id = component_id::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID;
            let actual = subject
                .component_source(type_id)
                .map(|source| source.attribute_modifiers.as_slice())
                .unwrap_or_default();
            !removed(type_id)
                && actual.len() == expected.len()
                && expected.iter().zip(actual).all(|(expected, actual)| {
                    exact_attribute_modifier_matches(expected, actual, ctx)
                })
        }
        ItemExactComponent::Enchantments { stored, levels } => {
            let type_id = enchantments_type_id(*stored);
            if removed(type_id) {
                return false;
            }
            let actual = if patch.added_type_ids.contains(&type_id) {
                enchantments_of(patch, *stored)
            } else if !*stored || subject.item_id == "minecraft:enchanted_book" {
                &[]
            } else {
                return false;
            };
            if actual.len() != levels.len() {
                return false;
            }
            let Some(keys) = ctx.enchantment_keys else {
                return actual.is_empty();
            };
            levels.iter().all(|(key, level)| {
                actual.iter().any(|actual| {
                    registry_key(Some(keys), actual.holder_id) == Some(key.as_str())
                        && actual.level == *level
                })
            })
        }
        ItemExactComponent::PotionContents(expected) => {
            explicit(component_id::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID)
                && potion_contents_matches(expected, patch)
        }
        ItemExactComponent::WritableBookContent {
            pages,
            page_filters,
        } => {
            explicit(component_id::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)
                && filterable_pages_match(
                    (
                        &patch.writable_book_pages,
                        &patch.writable_book_page_filters,
                    ),
                    (pages, page_filters),
                )
        }
        ItemExactComponent::WrittenBookContent(expected) => {
            explicit(component_id::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)
                && patch.written_book.as_ref().is_some_and(|book| {
                    book.title == expected.title
                        && book.title_filter == expected.title_filter
                        && book.author == expected.author
                        && book.generation == expected.generation
                        && book.resolved == expected.resolved
                        && filterable_pages_match(
                            (&book.pages, &book.page_filters),
                            (&expected.pages, &expected.page_filters),
                        )
                })
        }
        ItemExactComponent::FireworkExplosion(expected) => {
            explicit(component_id::VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID)
                && patch.firework_explosion.as_ref() == Some(expected)
        }
        ItemExactComponent::Fireworks {
            flight_duration,
            explosions,
        } => {
            explicit(component_id::VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID)
                && patch.fireworks_flight_duration == Some(*flight_duration)
                && patch.fireworks_explosions == *explosions
        }
        ItemExactComponent::JukeboxPlayable(expected) => {
            explicit(component_id::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID)
                && match expected {
                    ExactJukeboxPlayable::RegistryKey(expected) => {
                        patch
                            .jukebox_song_id
                            .and_then(JukeboxSongRegistry::vanilla_26_1_song_id)
                            == Some(expected.as_str())
                    }
                    ExactJukeboxPlayable::DirectSong {
                        sound_event,
                        description,
                        length_in_seconds_bits,
                        comparator_output,
                    } => patch.jukebox_direct_song.as_ref().is_some_and(|actual| {
                        actual.sound_event == *sound_event
                            && actual.description == *description
                            && actual.length_in_seconds_bits == *length_in_seconds_bits
                            && actual.comparator_output == *comparator_output
                    }),
                }
        }
        ItemExactComponent::Trim { material, pattern } => {
            explicit(component_id::VANILLA_DATA_COMPONENT_TRIM_TYPE_ID)
                && match material {
                    ExactTrimMaterial::RegistryKey(expected) => {
                        patch
                            .armor_trim_material_id
                            .and_then(|id| registry_key(ctx.trim_material_keys, id))
                            == Some(expected.as_str())
                    }
                    ExactTrimMaterial::Direct {
                        asset_name,
                        override_armor_assets,
                        description,
                    } => patch
                        .armor_trim_material_direct
                        .as_ref()
                        .is_some_and(|actual| {
                            actual.asset_name == *asset_name
                                && actual.override_armor_assets == *override_armor_assets
                                && actual.description == *description
                        }),
                }
                && match pattern {
                    ExactTrimPattern::RegistryKey(expected) => {
                        patch
                            .armor_trim_pattern_id
                            .and_then(|id| {
                                trim_pattern_resource_id(
                                    id,
                                    ctx.trim_pattern_keys,
                                    ctx.registry_access.map(|(_, access)| access),
                                )
                            })
                            .as_deref()
                            == Some(expected.as_str())
                    }
                    ExactTrimPattern::Direct {
                        asset_id,
                        description,
                        decal,
                    } => patch
                        .armor_trim_pattern_direct
                        .as_ref()
                        .is_some_and(|actual| {
                            actual.asset_id == *asset_id
                                && actual.description == *description
                                && actual.decal == *decal
                        }),
                }
        }
        ItemExactComponent::VillagerVariant(expected) => {
            explicit(component_id::VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID)
                && patch
                    .villager_variant_id
                    .and_then(VillagerModelType::for_vanilla_registry_id)
                    .is_some_and(|actual| actual.resource_id() == expected)
        }
    }
}

// Compare each filter with its page so an early mismatch stays cheap.
fn filterable_pages_match<T: PartialEq>(
    (pages, filters): (&[T], &[Option<T>]),
    (expected_pages, expected_filters): (&[T], &[Option<T>]),
) -> bool {
    pages.len() == expected_pages.len()
        && filters.len() == expected_filters.len()
        && pages
            .iter()
            .zip(filters)
            .eq(expected_pages.iter().zip(expected_filters))
}

fn exact_attribute_modifier_matches(
    expected: &ExactAttributeModifier,
    actual: &AttributeModifierSummary,
    ctx: &IconResolveContext<'_>,
) -> bool {
    registry_key(ctx.attribute_keys, actual.attribute_id) == Some(expected.attribute.as_str())
        && actual.modifier_id == expected.modifier_id
        && actual.amount_bits == expected.amount_bits
        && actual.operation_id == expected.operation_id
        && actual.slot_id == expected.slot_id
        && actual.display_id == expected.display_id
        && actual.display_text == expected.display_text
}

fn potion_contents_matches(
    expected: &bbb_protocol::packets::PotionContentsSummary,
    patch: &DataComponentPatchSummary,
) -> bool {
    let actual = &patch.potion_contents;
    actual.potion_id == expected.potion_id
        && actual.custom_color == expected.custom_color
        && actual.custom_name == expected.custom_name
        && actual.custom_effects.len() == expected.custom_effects.len()
        && expected
            .custom_effects
            .iter()
            .zip(&actual.custom_effects)
            .all(|(expected, actual)| expected.java_equals(actual))
}

fn registry_key(keys: Option<&[String]>, holder_id: i32) -> Option<&str> {
    keys?
        .get(usize::try_from(holder_id).ok()?)
        .map(String::as_str)
}

pub(super) fn trim_pattern_resource_id<'a>(
    holder_id: i32,
    trim_pattern_keys: Option<&'a [String]>,
    registry_access: Option<&dyn bbb_registry_codec::PersistentItemStackHolderResolver>,
) -> Option<std::borrow::Cow<'a, str>> {
    match trim_pattern_keys {
        Some(keys) => registry_key(Some(keys), holder_id).map(std::borrow::Cow::Borrowed),
        None => registry_access?
            .holder_resource_id("minecraft:trim_pattern", holder_id)
            .map(std::borrow::Cow::Owned),
    }
}

/// `NbtPredicate`: compounds match by subset, lists (when partial) by
/// containment.
pub(super) fn nbt_summary_matches(
    expected: &NbtSummaryValue,
    actual: &NbtSummaryValue,
    partial_list_matches: bool,
) -> bool {
    match (expected, actual) {
        (NbtSummaryValue::Compound(expected), NbtSummaryValue::Compound(actual)) => {
            actual.len() >= expected.len()
                && expected.iter().all(|entry| {
                    actual
                        .iter()
                        .find(|actual_entry| actual_entry.name == entry.name)
                        .is_some_and(|actual_entry| {
                            nbt_summary_matches(
                                &entry.value,
                                &actual_entry.value,
                                partial_list_matches,
                            )
                        })
                })
        }
        (NbtSummaryValue::List(expected), NbtSummaryValue::List(actual))
            if partial_list_matches =>
        {
            if expected.is_empty() {
                return actual.is_empty();
            }
            actual.len() >= expected.len()
                && expected.iter().all(|expected_item| {
                    actual
                        .iter()
                        .any(|actual_item| nbt_summary_matches(expected_item, actual_item, true))
                })
        }
        _ => expected == actual,
    }
}

fn nbt_summary_exact_matches(expected: &NbtSummaryValue, actual: &NbtSummaryValue) -> bool {
    match (expected, actual) {
        (NbtSummaryValue::Compound(expected), NbtSummaryValue::Compound(actual)) => {
            expected.len() == actual.len()
                && expected.iter().all(|entry| {
                    actual
                        .iter()
                        .find(|actual_entry| actual_entry.name == entry.name)
                        .is_some_and(|actual_entry| {
                            nbt_summary_exact_matches(&entry.value, &actual_entry.value)
                        })
                })
        }
        (NbtSummaryValue::List(expected), NbtSummaryValue::List(actual)) => {
            expected.len() == actual.len()
                && expected
                    .iter()
                    .zip(actual)
                    .all(|(expected, actual)| nbt_summary_exact_matches(expected, actual))
        }
        _ => expected == actual,
    }
}

fn damage_component_predicate_state(subject: &PredicateSubject<'_>) -> Option<(i32, i32)> {
    if subject.removed(component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID) {
        return None;
    }
    let damage = subject
        .component_source(component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID)?
        .damage?;
    let max_damage = if subject.removed(component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID) {
        0
    } else {
        subject
            .component_source(component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID)
            .and_then(|source| source.max_damage)
            .unwrap_or(0)
    };
    Some((damage, max_damage))
}

pub(super) fn item_stack_has_component_id(
    component_id: i32,
    component_patch: Option<&DataComponentPatchSummary>,
    default_component_codec_values: Option<&BTreeMap<i32, serde_json::Value>>,
    default_item_model_id: Option<&str>,
    ignore_default: bool,
) -> bool {
    let added = component_patch.is_some_and(|patch| patch.added_type_ids.contains(&component_id));
    let removed =
        component_patch.is_some_and(|patch| patch.removed_type_ids.contains(&component_id));
    if ignore_default {
        return added || removed;
    }
    !removed
        && (added
            || item_default_has_component(
                component_id,
                default_component_codec_values,
                default_item_model_id,
            ))
}

fn item_default_has_component(
    component_id: i32,
    default_component_codec_values: Option<&BTreeMap<i32, serde_json::Value>>,
    default_item_model_id: Option<&str>,
) -> bool {
    default_component_codec_values.is_some_and(|values| values.contains_key(&component_id))
        || matches!(
            component_id,
            component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID
                | component_id::VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID
                | component_id::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID
                | component_id::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID
        )
        || (component_id == component_id::VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID
            && default_item_model_id == Some("minecraft:enchanted_book"))
}

/// Effective scalar value of the components `ItemExactComponent::Scalar`
/// compares, with prototype defaults.
pub(super) fn scalar_component_value(
    type_id: i32,
    component_patch: &DataComponentPatchSummary,
    default_max_stack_size: Option<i32>,
    default_max_damage: Option<i32>,
    default_item_model_id: &str,
) -> Option<ItemModelScalar> {
    if component_patch.removed_type_ids.contains(&type_id) {
        return None;
    }
    match type_id {
        component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => Some(ItemModelScalar::I32(
            effective_item_max_stack_size(Some(component_patch), default_max_stack_size),
        )),
        component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => component_patch
            .max_damage
            .or(default_max_damage)
            .map(ItemModelScalar::I32),
        component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => {
            component_patch.damage.map(ItemModelScalar::I32)
        }
        component_id::VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID => Some(ItemModelScalar::String(
            component_patch
                .item_model
                .as_deref()
                .unwrap_or(default_item_model_id)
                .into(),
        )),
        component_id::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => Some(ItemModelScalar::String(
            component_patch
                .rarity
                .unwrap_or(ItemRaritySummary::Common)
                .as_str()
                .into(),
        )),
        component_id::VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => component_patch
            .enchantment_glint_override
            .map(ItemModelScalar::Bool),
        component_id::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => {
            component_patch.map_id.map(ItemModelScalar::I32)
        }
        component_id::VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => {
            component_patch.dyed_color.map(ItemModelScalar::I32)
        }
        component_id::VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID => {
            component_patch.map_color.map(ItemModelScalar::I32)
        }
        _ => None,
    }
}
