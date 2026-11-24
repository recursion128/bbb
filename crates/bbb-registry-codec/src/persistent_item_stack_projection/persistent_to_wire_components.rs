//! The persistent-to-wire direction: one dispatch over
//! `DataComponentPatchSummary`'s typed fields for every component whose
//! persistent codec is a total function of what the wire decode kept.

use std::collections::{BTreeMap, BTreeSet};

use bbb_facts::builtin_enum_component_values::builtin_enum_component_value_for_name;
use bbb_facts::data_component_types::{
    vanilla_data_component_type, VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
    VANILLA_DATA_COMPONENT_BEES_TYPE_ID, VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
    VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID, VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
    VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID, VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
    VANILLA_DATA_COMPONENT_FOOD_TYPE_ID, VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
    VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
    VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
    VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID, VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID, VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
    VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
    VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID, VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID, VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
    VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID, VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
    VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID, VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_TOOL_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
    VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
    VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_facts::item_component_value_types::ItemRaritySummary;
use bbb_facts::item_types::VANILLA_ITEM_BRICK_ID;
use bbb_facts::mob_effect_types::vanilla_mob_effect_type_for_resource_id;
use bbb_protocol::component::{styled_component_runs_from_nbt_value, styled_runs_summary_text};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::nbt_summary::{codec_float_value, codec_int_value, NbtSummaryDecodeLimits};
use bbb_protocol::packets::{
    BannerPatternLayerSummary, DataComponentPatchSummary, ItemContainerSlotSummary,
    ItemEnchantmentSummary, SuspiciousStewEffectSummary, ToolRuleSummary, ToolSummary,
};
use bbb_protocol::NbtSummaryValue;

use super::{
    attribute_modifier_components, builtin_enum_components, component_matchers,
    component_sets::project_retained_persistent_component,
    consumable_components, embedded_record_components, equippable_components,
    error::{ItemStackPersistentProjectionError, Result},
    holder::{
        item_id_in_registry, project_block_holder_set, project_inline_trim_material,
        project_inline_trim_pattern, resolve_holder_id, BANNER_PATTERN_REGISTRY,
        ENCHANTMENT_REGISTRY, ITEM_REGISTRY, MOB_EFFECT_REGISTRY, TRIM_MATERIAL_REGISTRY,
        TRIM_PATTERN_REGISTRY,
    },
    holder_component_projection,
    nbt_support::{
        latest_field, nbt_string, ComponentReader, NbtListView, MAX_PERSISTENT_LIST_ITEMS,
        MAX_PREALLOC_CAPACITY, PERSISTENT_POT_DECORATION_SIDES,
        PERSISTENT_SUSPICIOUS_STEW_DEFAULT_DURATION,
    },
    nested_item_stack, potion_contents, profile_components, simple_record_components,
    tag_and_data_components, variant_components, weapon_and_block_attacks,
};

/// Validates a supported scalar without constructing an item patch.
/// Other components return `None` and keep their existing structured reader.
pub fn validate_persistent_scalar_component(
    type_id: i32,
    value: &NbtSummaryValue,
) -> Option<Result<()>> {
    let component = vanilla_data_component_type(type_id)?;
    read_scalar_component(
        ComponentReader::new(type_id, component.resource_id),
        value,
        None,
    )
}

fn read_scalar_component(
    reader: ComponentReader,
    value: &NbtSummaryValue,
    patch: Option<&mut DataComponentPatchSummary>,
) -> Option<Result<()>> {
    Some(match reader.type_id {
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => {
            reader.int_in(value, "max stack size", 1, 99).map(|value| {
                if let Some(patch) = patch {
                    patch.max_stack_size = Some(value);
                }
            })
        }
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => reader
            .int_in(value, "max damage", 1, i32::MAX)
            .map(|value| {
                if let Some(patch) = patch {
                    patch.max_damage = Some(value);
                }
            }),
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => {
            reader.int_in(value, "damage", 0, i32::MAX).map(|value| {
                if let Some(patch) = patch {
                    patch.damage = Some(value);
                }
            })
        }
        VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID => reader
            .int_in(value, "repair cost", 0, i32::MAX)
            .map(|value| {
                if let Some(patch) = patch {
                    patch.repair_cost = Some(value);
                }
            }),
        VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID => reader.int(value, "map colour").map(|value| {
            if let Some(patch) = patch {
                patch.map_color = Some(value);
            }
        }),
        VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID => reader.int(value, "map id").map(|value| {
            if let Some(patch) = patch {
                patch.map_id = Some(value);
            }
        }),
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID => {
            reader.int_in(value, "amplifier", 0, 4).map(|value| {
                if let Some(patch) = patch {
                    patch.ominous_bottle_amplifier = Some(value);
                }
            })
        }
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID => {
            reader.flag(value, "glint override").map(|value| {
                if let Some(patch) = patch {
                    patch.enchantment_glint_override = Some(value);
                }
            })
        }
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID => reader.unit(value).map(|()| {
            if let Some(patch) = patch {
                patch.unbreakable = true;
            }
        }),
        VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID => reader.unit(value).map(|()| {
            if let Some(patch) = patch {
                patch.intangible_projectile = true;
            }
        }),
        VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID => reader.unit(value),
        _ => return None,
    })
}

/// `depth` counts the enclosing `minecraft:can_break` / `minecraft:can_place_on`
/// predicates, which an exact-component map may nest again
/// (`component_matchers.rs`).
///
/// This read is the component's whole validation: a value the projection
/// cannot read is refused, and the caller refuses the stack with it.
#[allow(clippy::too_many_arguments)]
pub(super) fn project_persistent_component(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
    depth: usize,
    item_depth: usize,
) -> Result<()> {
    let c = ComponentReader::new(type_id, resource_id);
    if let Some(result) = read_scalar_component(c, value, Some(&mut *patch)) {
        return result;
    }
    if project_retained_persistent_component(type_id, value, patch) {
        return Ok(());
    }
    match type_id {
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => {
            c.text_with_registry(value, "custom name", content_registry)?;
            let styled = styled_component_runs_from_nbt_value(value);
            patch.custom_name = Some(styled_runs_summary_text(&styled));
            patch.custom_name_styled = Some(styled);
            patch.custom_name_nbt = Some(value.clone());
        }
        VANILLA_DATA_COMPONENT_LORE_TYPE_ID => {
            // `ItemLore.CODEC` is `ComponentSerialization.CODEC.sizeLimitedListOf(256)`
            let lines = c.list(value, "lore")?;
            if lines.len() > 256 {
                return Err(c.error(format!("lore has {} lines, more than 256", lines.len())));
            }
            for line in lines.iter() {
                c.text_with_registry(&line, "lore line", content_registry)?;
            }
            let lore_nbt = lines.to_vec();
            patch.lore_styled = lore_nbt
                .iter()
                .map(styled_component_runs_from_nbt_value)
                .collect();
            patch.lore = patch
                .lore_styled
                .iter()
                .map(|styled| styled_runs_summary_text(styled))
                .collect();
            patch.lore_nbt = lore_nbt;
        }
        VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID => {
            patch.item_model = Some(c.resource(value, "item model")?)
        }
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID => {
            patch.tooltip_style = Some(c.resource(value, "tooltip style")?)
        }
        VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID => {
            patch.note_block_sound = Some(c.resource(value, "note block sound")?)
        }
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID => {
            patch.minimum_attack_charge_bits =
                Some(c.float_in(value, "attack charge", 0.0, 1.0)?.to_bits());
        }
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID => {
            patch.potion_duration_scale_bits = Some(
                c.float_in(value, "duration scale", 0.0, f32::MAX)?
                    .to_bits(),
            );
        }
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID => {
            let fields = c.compound(value, "trim")?;
            let material = c.field(fields, "material")?;
            match nbt_string(material) {
                Some(named) => {
                    patch.armor_trim_material_id = Some(resolve_holder_id(
                        TRIM_MATERIAL_REGISTRY,
                        &c.resource_units(named.units(), "trim material")?,
                        holder_id,
                    )?);
                }
                None => {
                    patch.armor_trim_material_direct =
                        Some(Box::new(project_inline_trim_material(c, material)?));
                }
            }
            let pattern = c.field(fields, "pattern")?;
            match nbt_string(pattern) {
                Some(named) => {
                    patch.armor_trim_pattern_id = Some(resolve_holder_id(
                        TRIM_PATTERN_REGISTRY,
                        &c.resource_units(named.units(), "trim pattern")?,
                        holder_id,
                    )?);
                }
                None => {
                    patch.armor_trim_pattern_direct =
                        Some(Box::new(project_inline_trim_pattern(c, pattern)?));
                }
            }
        }
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID => {
            let fields = c.compound(value, "block state")?;
            if fields.len() > MAX_PERSISTENT_LIST_ITEMS {
                return Err(c.error(format!(
                    "block state has {} properties, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
                    fields.len()
                )));
            }
            let mut properties = Vec::with_capacity(fields.len().min(MAX_PREALLOC_CAPACITY));
            for field in fields {
                properties.push((
                    field.name.clone(),
                    c.java_string(&field.value, "block state value")?,
                ));
            }
            patch.block_state_properties =
                bbb_protocol::packets::data_components::BlockStateProperties::from_entries(
                    properties,
                );
        }
        VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID => {
            let fields = c.compound(value, "custom model data")?;
            let list = |name: &str| -> Result<NbtListView<'_>> {
                let entries = latest_field(fields, name)
                    .map_or(Ok(NbtListView::EMPTY), |value| c.list(value, name))?;
                if entries.len() > MAX_PERSISTENT_LIST_ITEMS {
                    return Err(c.error(format!(
                        "{name} has {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
                        entries.len()
                    )));
                }
                Ok(entries)
            };
            patch.custom_model_data_floats = list("floats")?
                .iter()
                .map(|value| c.float(&value, "model data float"))
                .collect::<Result<Vec<f32>>>()?
                .into();
            patch.custom_model_data_flags = list("flags")?
                .iter()
                .map(|value| c.flag(value.as_ref(), "model data flag"))
                .collect::<Result<_>>()?;
            patch.custom_model_data_strings = list("strings")?
                .iter()
                .map(|value| c.java_string(&value, "model data string"))
                .collect::<Result<_>>()?;
            patch.custom_model_data_colors = list("colors")?
                .iter()
                .map(|value| project_rgb_color(c, &value))
                .collect::<Result<_>>()?;
        }
        VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID => {
            let fields = c.compound(value, "use cooldown")?;
            // `ExtraCodecs.POSITIVE_FLOAT`.
            let seconds = c.float(c.field(fields, "seconds")?, "cooldown seconds")?;
            if !(seconds > 0.0 && seconds <= f32::MAX) {
                return Err(c.error(format!("cooldown seconds {seconds} is not positive")));
            }
            patch.use_cooldown_seconds_bits = Some(seconds.to_bits());
            patch.use_cooldown_group = latest_field(fields, "cooldown_group")
                .map(|group| c.resource(group, "cooldown group"))
                .transpose()?;
        }
        VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID => {
            let fields = c.compound(value, "weapon")?;
            patch.weapon_item_damage_per_attack = Some(
                latest_field(fields, "item_damage_per_attack")
                    .map(|value| c.int_in(value, "item damage per attack", 0, i32::MAX))
                    .transpose()?
                    .unwrap_or(1),
            );
            patch.weapon_disable_blocking_seconds_bits = Some(
                latest_field(fields, "disable_blocking_for_seconds")
                    .map(|value| c.float_in(value, "disable blocking seconds", 0.0, f32::MAX))
                    .transpose()?
                    .unwrap_or(0.0)
                    .to_bits(),
            );
        }
        VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID => {
            patch.firework_explosion = Some(super::firework_components::read_explosion(
                crate::registry_codec::RegistryElementValueRef::Nbt(value),
            )?);
        }
        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID => {
            let (flight, explosions) = super::firework_components::read_fireworks(
                crate::registry_codec::RegistryElementValueRef::Nbt(value),
            )?;
            patch.fireworks_flight_duration = Some(flight);
            patch.fireworks_explosions = explosions;
        }
        VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID => {
            let fields = c.compound(value, "enchantable")?;
            patch.enchantable_value =
                Some(c.int_in(c.field(fields, "value")?, "enchantable value", 1, i32::MAX)?);
        }
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => {
            patch.stored_enchantments = project_enchantments(c, value, holder_id)?;
        }
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID => {
            patch.enchantments = project_enchantments(c, value, holder_id)?;
        }
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID => {
            patch.potion_contents = potion_contents::read(
                crate::registry_codec::RegistryElementValueRef::Nbt(value),
                NbtSummaryDecodeLimits::default().max_depth,
            )?;
        }
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => {
            project_container(
                c,
                value,
                holder_id,
                content_registry,
                patch,
                depth,
                item_depth,
            )?;
        }
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => {
            let items = nested_item_stack::project_item_stack_template_list(
                type_id,
                resource_id,
                value,
                holder_id,
                content_registry,
                depth,
                item_depth,
            )?;
            patch.bundle_contents_items = items;
        }
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => {
            patch.charged_projectiles_items = nested_item_stack::project_item_stack_template_list(
                type_id,
                resource_id,
                value,
                holder_id,
                content_registry,
                depth,
                item_depth,
            )?;
        }
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => {
            patch.use_remainder = Some(Box::new(nested_item_stack::project_item_stack_template(
                type_id,
                resource_id,
                value,
                holder_id,
                content_registry,
                depth,
                item_depth,
            )?));
        }
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID => {
            patch.attribute_modifiers = attribute_modifier_components::project_attribute_modifiers(
                type_id,
                resource_id,
                value,
            )?;
        }
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID => {
            patch.profile = Some(profile_components::project_profile(
                type_id,
                resource_id,
                value,
            )?);
        }
        VANILLA_DATA_COMPONENT_FOOD_TYPE_ID => {
            consumable_components::project_food(type_id, resource_id, value, patch)?;
        }
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID => {
            consumable_components::project_consumable(
                type_id,
                resource_id,
                value,
                content_registry,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID => {
            patch.banner_pattern_layers = project_banner_patterns(c, value, holder_id)?;
        }
        VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID => {
            let (hide, hidden) = project_tooltip_display(c, value)?;
            patch.tooltip_hide_tooltip = hide;
            patch.tooltip_hidden_component_type_ids = hidden;
        }
        VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => {
            c.text_with_registry(value, "item name", content_registry)?;
            let styled = styled_component_runs_from_nbt_value(value);
            patch.item_name = Some(styled_runs_summary_text(&styled));
            patch.item_name_styled = Some(styled);
            patch.item_name_nbt = Some(value.clone());
        }
        VANILLA_DATA_COMPONENT_RARITY_TYPE_ID => {
            let name = c.string(value, "rarity")?;
            patch.rarity = Some(
                name.with_utf8(ItemRaritySummary::from_serialized_name)
                    .flatten()
                    .ok_or_else(|| c.error(format!("unknown rarity {name}")))?,
            );
        }
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID => {
            patch.tool = Some(project_tool(c, value, content_registry)?);
        }
        VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID => {
            patch.dyed_color = Some(project_rgb_color(c, value)?);
        }
        VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID => {
            patch.suspicious_stew_effects = project_suspicious_stew_effects(c, value)?;
        }
        VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID => {
            let name = c.string(value, "base color")?;
            patch.base_color_id = Some(
                name.with_utf8(|name| builtin_enum_component_value_for_name(type_id, name))
                    .flatten()
                    .ok_or_else(|| c.error(format!("unknown base color {name}")))?
                    .id,
            );
        }
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID => {
            patch.pot_decorations_item_ids = project_pot_decorations(c, value, content_registry)?;
        }
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID => {
            let (pages, filters) = super::book_components::read_writable(
                crate::registry_codec::RegistryElementValueRef::Nbt(value),
            )?;
            patch.writable_book_pages = pages;
            patch.writable_book_page_filters = filters;
        }
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID => {
            let (content, pages_nbt, page_filters_nbt) = super::book_components::read_written(
                crate::registry_codec::RegistryElementValueRef::Nbt(value),
            )?;
            patch.written_book = Some(content);
            patch.written_book_pages_nbt = pages_nbt;
            patch.written_book_page_filters_nbt = page_filters_nbt;
        }
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID => {
            holder_component_projection::project_break_sound(
                type_id,
                resource_id,
                value,
                content_registry,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID => {
            holder_component_projection::project_piercing_weapon(
                type_id,
                resource_id,
                value,
                content_registry,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID => {
            holder_component_projection::project_repairable(
                type_id,
                resource_id,
                value,
                content_registry,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID => {
            holder_component_projection::project_damage_resistant(
                type_id,
                resource_id,
                value,
                holder_id,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID => {
            holder_component_projection::project_provides_trim_material(
                type_id,
                resource_id,
                value,
                holder_id,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID => {
            embedded_record_components::project_lock(type_id, resource_id, value, patch)?;
        }
        VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID => {
            let (target, tracked) =
                embedded_record_components::read_lodestone_tracker(value.into())?;
            patch.lodestone_target = target;
            patch.lodestone_tracked = Some(tracked);
        }
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID => {
            equippable_components::project_equippable(
                type_id,
                resource_id,
                value,
                content_registry,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID
        | VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID => {
            builtin_enum_components::project_builtin_enum(type_id, resource_id, value, patch)?;
        }
        type_id if builtin_enum_components::is_builtin_enum_component(type_id) => {
            builtin_enum_components::project_builtin_enum(type_id, resource_id, value, patch)?;
        }
        type_id if variant_components::variant_registry(type_id).is_some() => {
            variant_components::project_variant(type_id, resource_id, value, holder_id, patch)?;
        }
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID => {
            tag_and_data_components::project_provides_banner_patterns(
                type_id,
                resource_id,
                value,
                holder_id,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID => {
            tag_and_data_components::project_debug_stick_state(
                type_id,
                resource_id,
                value,
                content_registry,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID
        | VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => {
            tag_and_data_components::project_typed_data(
                type_id,
                resource_id,
                value,
                content_registry,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_BEES_TYPE_ID => {
            tag_and_data_components::project_bees(
                type_id,
                resource_id,
                value,
                content_registry,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID
        | VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => {
            weapon_and_block_attacks::project_weapon_and_block_attacks(
                type_id,
                resource_id,
                value,
                holder_id,
                content_registry,
                patch,
            )?;
        }
        type_id if simple_record_components::is_simple_record_component(type_id) => {
            simple_record_components::project_simple_record(
                type_id,
                resource_id,
                value,
                holder_id,
                content_registry,
                patch,
            )?;
        }
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID | VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID => {
            component_matchers::project_adventure_mode_predicate(
                type_id,
                resource_id,
                value,
                holder_id,
                content_registry,
                patch,
                depth,
                item_depth,
            )?;
        }
        _ => {
            return Err(ItemStackPersistentProjectionError::UnsupportedComponent {
                type_id,
                resource_id,
            })
        }
    }
    Ok(())
}

/// `ItemContainerContents.CODEC` is a
/// `sizeLimitedListOf(256)` of `Slot` records mapped through `fromSlots`
/// the wire form has one entry per index
/// up to the highest occupied one, and a repeated index is last-wins.
fn project_container(
    c: ComponentReader,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
    depth: usize,
    item_depth: usize,
) -> Result<()> {
    let slots = c.list(value, "container")?;
    if slots.len() > 256 {
        return Err(c.error(format!(
            "container has {} slots, more than 256",
            slots.len()
        )));
    }
    let mut occupied = BTreeMap::new();
    for slot in slots.iter() {
        let fields = c.compound(&slot, "container slot")?;
        let index = c.int_in(c.field(fields, "slot")?, "container slot index", 0, 255)?;
        let item = nested_item_stack::project_item_stack_template(
            c.type_id,
            c.resource_id,
            c.field(fields, "item")?,
            holder_id,
            content_registry,
            depth,
            item_depth,
        )?;
        occupied.insert(index, item);
    }
    let slot_count = occupied.keys().next_back().map_or(0, |highest| highest + 1);
    patch.container.slot_count = Some(slot_count as usize);
    for (index, item) in occupied {
        patch.container.slots.push(ItemContainerSlotSummary {
            slot: index as usize,
            item,
        });
    }
    Ok(())
}

fn project_banner_patterns(
    c: ComponentReader,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
) -> Result<Vec<BannerPatternLayerSummary>> {
    let layers = c.list(value, "banner patterns")?;
    if layers.len() > MAX_PERSISTENT_LIST_ITEMS {
        return Err(c.error(format!(
            "banner patterns have {} layers, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
            layers.len()
        )));
    }
    layers
        .iter()
        .map(|layer| {
            let fields = c.compound(&layer, "banner layer")?;
            let color = c.string(c.field(fields, "color")?, "banner color")?;
            let color_id = color
                .with_utf8(|color| {
                    builtin_enum_component_value_for_name(
                        bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                        color,
                    )
                })
                .flatten()
                .ok_or_else(|| c.error(format!("unknown banner color {color}")))?
                .id;
            match c.field(fields, "pattern")? {
                NbtSummaryValue::String(name) => Ok(BannerPatternLayerSummary {
                    registry_id: Some(resolve_holder_id(
                        BANNER_PATTERN_REGISTRY,
                        &c.resource_units(name.units(), "banner pattern")?,
                        holder_id,
                    )?),
                    color_id,
                    ..BannerPatternLayerSummary::default()
                }),
                NbtSummaryValue::Compound(pattern) => Ok(BannerPatternLayerSummary {
                    registry_id: None,
                    asset_id: Some(c.resource(c.field(pattern, "asset_id")?, "pattern asset")?),
                    translation_key: Some(c.java_string(
                        c.field(pattern, "translation_key")?,
                        "pattern translation",
                    )?),
                    color_id,
                }),
                _ => Err(c.error("banner pattern is neither a name nor a record")),
            }
        })
        .collect()
}

/// `ExtraCodecs.RGB_COLOR_CODEC`: an int, or the
/// three channels `ARGB.colorFromFloat` folds at full alpha.
fn project_rgb_color(c: ComponentReader, value: &NbtSummaryValue) -> Result<i32> {
    if let Some(rgb) = codec_int_value(value) {
        return Ok(rgb);
    }
    let [red, green, blue] = rgb_color_channels(value)
        .ok_or_else(|| c.error("RGB color is neither an int nor three numbers"))?;
    Ok(argb_color_from_float(1.0, red, green, blue))
}

/// `NbtOps.getList` reads every `CollectionTag`, so
/// `ExtraCodecs.VECTOR3F` also reads the
/// specialized array tags without intermediate allocation.
fn rgb_color_channels(value: &NbtSummaryValue) -> Option<[f32; 3]> {
    match value {
        NbtSummaryValue::List(values) => {
            let [r, g, b] = values.as_slice() else {
                return None;
            };
            Some([
                codec_float_value(r)?,
                codec_float_value(g)?,
                codec_float_value(b)?,
            ])
        }
        NbtSummaryValue::ByteArray(values) => {
            let &[r, g, b] = values.as_slice() else {
                return None;
            };
            Some([f32::from(r), f32::from(g), f32::from(b)])
        }
        NbtSummaryValue::IntArray(values) => {
            let &[r, g, b] = values.as_slice() else {
                return None;
            };
            Some([r as f32, g as f32, b as f32])
        }
        NbtSummaryValue::LongArray(values) => {
            let &[r, g, b] = values.as_slice() else {
                return None;
            };
            Some([r as f32, g as f32, b as f32])
        }
        _ => None,
    }
}

/// `ARGB.colorFromFloat` over `ARGB.as8BitChannel`
/// and `ARGB.color`.
fn argb_color_from_float(alpha: f32, red: f32, green: f32, blue: f32) -> i32 {
    // `Mth.floor` saturates and maps NaN to zero exactly as a Rust `as` cast does.
    let channel = |value: f32| ((value * 255.0).floor() as i32) & 0xFF;
    (channel(alpha) << 24) | (channel(red) << 16) | (channel(green) << 8) | channel(blue)
}

/// `SuspiciousStewEffects.CODEC` over
/// `Entry.CODEC`.
fn project_suspicious_stew_effects(
    c: ComponentReader,
    value: &NbtSummaryValue,
) -> Result<Vec<SuspiciousStewEffectSummary>> {
    let entries = c.list(value, "stew effects")?;
    if entries.len() > MAX_PERSISTENT_LIST_ITEMS {
        return Err(c.error(format!(
            "stew effects have {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
            entries.len()
        )));
    }
    entries
        .iter()
        .map(|entry| {
            let fields = c.compound(&entry, "stew effect")?;
            let effect = c.string(c.field(fields, "id")?, "stew effect")?;
            Ok(SuspiciousStewEffectSummary {
                effect_id: mob_effect_holder_id(c, effect)?,
                // `lenientOptionalFieldOf` reads its default for a field that
                // is missing or is not an int.
                duration: latest_field(fields, "duration")
                    .and_then(codec_int_value)
                    .unwrap_or(PERSISTENT_SUSPICIOUS_STEW_DEFAULT_DURATION),
            })
        })
        .collect()
}

/// `PotDecorations.CODEC` reads up to four item names
/// a short list and `minecraft:brick` fold onto
/// an empty side, which `ordered` writes back as brick.
fn project_pot_decorations(
    c: ComponentReader,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Result<Vec<i32>> {
    let items = c.list(value, "pot decorations")?;
    if items.len() > PERSISTENT_POT_DECORATION_SIDES {
        return Err(c.error(format!("pot decorations name {} sides", items.len())));
    }
    let mut sides = vec![VANILLA_ITEM_BRICK_ID; PERSISTENT_POT_DECORATION_SIDES];
    for (side, item) in sides.iter_mut().zip(items.iter()) {
        let item = c.resource(&item, "pot decoration")?;
        *side = item_id_in_registry(&item, content_registry).ok_or_else(|| {
            ItemStackPersistentProjectionError::UnresolvedRegistryHolder {
                registry_id: ITEM_REGISTRY,
                resource_id: item.clone(),
            }
        })?;
    }
    Ok(sides)
}

/// `Registries.MOB_EFFECT` is a built-in registry, so the
/// generated catalog issues the ids the wire projection carries.
fn mob_effect_holder_id(c: ComponentReader, raw: &bbb_protocol::JavaString) -> Result<i32> {
    let effect = c.resource_units(raw.units(), "mob effect")?;
    vanilla_mob_effect_type_for_resource_id(&effect)
        .map(|effect| effect.id)
        .ok_or_else(|| c.error(format!("unknown {MOB_EFFECT_REGISTRY} holder {effect}")))
}

fn project_tooltip_display(
    c: ComponentReader,
    value: &NbtSummaryValue,
) -> Result<(bool, Vec<i32>)> {
    let fields = c.compound(value, "tooltip display")?;
    let hide = latest_field(fields, "hide_tooltip")
        .map(|value| c.flag(value, "hide tooltip"))
        .transpose()?
        .unwrap_or(false);
    let mut hidden = Vec::new();
    let mut seen = BTreeSet::new();
    if let Some(values) = latest_field(fields, "hidden_components") {
        let entries = c.list(values, "hidden components")?;
        if entries.len() > MAX_PERSISTENT_LIST_ITEMS {
            return Err(c.error(format!(
                "hidden components list has {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
                entries.len()
            )));
        }
        for value in entries.iter() {
            let resource_id = c.resource(&value, "hidden component")?;
            let type_id =
                bbb_facts::data_component_types::vanilla_data_component_type_for_resource_id(
                    &resource_id,
                )
                .ok_or_else(|| c.error(format!("unknown hidden component {resource_id}")))?
                .type_id;
            if seen.insert(type_id) {
                hidden.push(type_id);
            }
        }
    }
    Ok((hide, hidden))
}

/// `ItemEnchantments.CODEC` maps `Enchantment.CODEC` to `Codec.intRange(1, 255)`
fn project_enchantments(
    c: ComponentReader,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
) -> Result<Vec<ItemEnchantmentSummary>> {
    let mut seen_keys = BTreeSet::new();
    let mut enchantments = c
        .compound(value, "enchantments")?
        .iter()
        .rev()
        .filter(|entry| seen_keys.insert(&entry.name))
        .map(|entry| {
            let resource_id = c.resource_units(entry.name.units(), "enchantment")?;
            Ok(ItemEnchantmentSummary {
                holder_id: resolve_holder_id(ENCHANTMENT_REGISTRY, &resource_id, holder_id)?,
                level: c.int_in(&entry.value, "enchantment level", 1, 255)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    enchantments.sort_by_key(|enchantment| enchantment.holder_id);
    if enchantments
        .windows(2)
        .any(|pair| pair[0].holder_id == pair[1].holder_id)
    {
        return Err(c.error("an enchantment is named twice"));
    }
    Ok(enchantments)
}

/// The wire projection of `Tool.CODEC`, whose rule block
/// set is a `"#tag"`, a bare identifier or a list.
fn project_tool(
    c: ComponentReader,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Result<ToolSummary> {
    let fields = c.compound(value, "tool")?;
    let rules_list = c.list(c.field(fields, "rules")?, "tool rules")?;
    if rules_list.len() > MAX_PERSISTENT_LIST_ITEMS {
        return Err(c.error(format!(
            "tool rules have {} entries, maximum is {MAX_PERSISTENT_LIST_ITEMS}",
            rules_list.len()
        )));
    }
    Ok(ToolSummary {
        rules: rules_list
            .iter()
            .map(|rule| project_tool_rule(c, &rule, content_registry))
            .collect::<Result<Vec<_>>>()?,
        default_mining_speed_bits: latest_field(fields, "default_mining_speed")
            .map(|value| c.float(value, "default mining speed"))
            .transpose()?
            .unwrap_or(1.0)
            .to_bits(),
        damage_per_block: latest_field(fields, "damage_per_block")
            .map(|value| c.int_in(value, "damage per block", 0, i32::MAX))
            .transpose()?
            .unwrap_or(1),
        can_destroy_blocks_in_creative: latest_field(fields, "can_destroy_blocks_in_creative")
            .map(|value| c.flag(value, "can destroy blocks in creative"))
            .transpose()?
            .unwrap_or(true),
    })
}

fn project_tool_rule(
    c: ComponentReader,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Result<ToolRuleSummary> {
    let fields = c.compound(value, "tool rule")?;
    let blocks = project_block_holder_set(
        c.type_id,
        c.resource_id,
        c.field(fields, "blocks")?,
        content_registry,
    )?;
    Ok(ToolRuleSummary {
        block_registry_ids: blocks.registry_ids,
        block_tag: blocks.tag,
        speed_bits: latest_field(fields, "speed")
            .map(|value| {
                let speed = c.float(value, "tool speed")?;
                // Tool: Rule.CODEC uses ExtraCodecs.POSITIVE_FLOAT.
                if !(speed > 0.0 && speed <= f32::MAX) {
                    return Err(c.error(format!("tool speed {speed} is not positive and finite")));
                }
                Ok(speed.to_bits())
            })
            .transpose()?,
        correct_for_drops: latest_field(fields, "correct_for_drops")
            .map(|value| c.flag(value, "correct for drops"))
            .transpose()?,
    })
}
