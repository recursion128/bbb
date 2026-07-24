use super::*;
use bbb_protocol::attribute_modifier_types::{
    attribute_modifier_display_name, attribute_modifier_operation_name,
    VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID, VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
};
use bbb_protocol::builtin_enum_component_values::{
    builtin_enum_component_value_name, vanilla_dye_color_name,
};
use bbb_protocol::data_component_types as component_id;
use bbb_protocol::equipment_slots::{
    vanilla_equipment_slot_by_id, vanilla_equipment_slot_group, VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
};
use bbb_protocol::tropical_fish_patterns::vanilla_tropical_fish_pattern;
use bbb_world::{
    direct_banner_pattern_presentation, direct_trim_material_presentation,
    direct_trim_pattern_presentation, BlockStateRegistry,
};

type PersistentSnbtEncoder = for<'a> fn(
    i32,
    &bbb_protocol::packets::DataComponentPatchSummary,
    PersistentSnbtContext<'a>,
) -> Option<String>;

#[derive(Clone, Copy, Default)]
pub(super) struct PersistentSnbtContext<'a> {
    pub item_runtime: Option<&'a NativeItemRuntime>,
    pub registry_access: Option<bbb_world::FrozenRegistryAccess<'a>>,
    pub(super) item_stack_template_depth: usize,
}

impl<'a> PersistentSnbtContext<'a> {
    pub(super) fn descend_item_stack_template(mut self) -> Option<Self> {
        if self.item_stack_template_depth
            >= bbb_protocol::packets::MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH
        {
            return None;
        }
        self.item_stack_template_depth += 1;
        Some(self)
    }

    pub(super) fn registry_resource_id(
        self,
        registry: &str,
        holder_id: i32,
        fallback: impl FnOnce(i32) -> Option<String>,
    ) -> Option<String> {
        match self.registry_access {
            Some(access) => match access.entry_by_holder_id(registry, holder_id) {
                Some(entry) => Some(entry.resource_id.to_string()),
                None if !access.has_server_contents(registry) => fallback(holder_id),
                None => None,
            },
            None => fallback(holder_id),
        }
    }

    pub(super) fn registry_holder_set_snbt(
        self,
        registry: &str,
        holder_ids: &[i32],
        tag: Option<&str>,
        fallback: impl Fn(i32) -> Option<String>,
    ) -> Option<String> {
        if let Some(tag) = tag {
            if !holder_ids.is_empty() {
                return None;
            }
            let tag = match self.registry_access {
                Some(access) => {
                    let holder_set = access.holder_set_by_tag(registry, tag).ok()?;
                    let bbb_world::FrozenRegistryHolderSetKind::Named { tag, .. } = holder_set.kind
                    else {
                        return None;
                    };
                    tag
                }
                None => tag,
            };
            return Some(debug_snbt_string(&format!("#{tag}")));
        }

        let resource_ids = match self.registry_access {
            Some(access) if access.has_server_contents(registry) => access
                .holder_set_by_holder_ids(registry, holder_ids)
                .ok()?
                .entries
                .into_iter()
                .map(|entry| entry.resource_id.to_string())
                .collect::<Vec<_>>(),
            None => holder_ids
                .iter()
                .map(|holder_id| fallback(*holder_id))
                .collect::<Option<Vec<_>>>()?,
            Some(_) => holder_ids
                .iter()
                .map(|holder_id| fallback(*holder_id))
                .collect::<Option<Vec<_>>>()?,
        };
        Some(match resource_ids.as_slice() {
            [] => "[]".to_string(),
            [resource_id] => debug_snbt_string(resource_id),
            resource_ids => format!(
                "[{}]",
                resource_ids
                    .iter()
                    .map(|resource_id| debug_snbt_string(resource_id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
    }
}

#[derive(Clone, Copy)]
struct PersistentSnbtEncoderDescriptor {
    type_id: i32,
    encode: PersistentSnbtEncoder,
}

const fn encoder(type_id: i32, encode: PersistentSnbtEncoder) -> PersistentSnbtEncoderDescriptor {
    PersistentSnbtEncoderDescriptor { type_id, encode }
}

static PERSISTENT_SNBT_ENCODERS: &[PersistentSnbtEncoderDescriptor] = &[
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
        encode_custom_data,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        encode_max_stack_size,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        encode_max_damage,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        encode_damage,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
        encode_unit,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
        encode_use_effects,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        encode_component_tree,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        encode_minimum_attack_charge,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
        encode_damage_type,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
        encode_component_tree,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
        encode_item_model,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
        encode_lore,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_RARITY_TYPE_ID,
        encode_rarity,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        encode_enchantments,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
        encode_adventure_mode_predicate,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
        encode_adventure_mode_predicate,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        encode_attribute_modifiers,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID,
        encode_custom_model_data,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID,
        encode_tooltip_display,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID,
        encode_repair_cost,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
        encode_enchantment_glint_override,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID,
        encode_unit,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
        encode_food,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
        encode_consumable,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
        encode_use_remainder,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        encode_use_cooldown,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        encode_damage_resistant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
        encode_tool,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
        encode_weapon,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
        encode_attack_range,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
        encode_enchantable,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
        encode_equippable,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
        encode_repairable,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID,
        encode_unit,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID,
        encode_tooltip_style,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
        encode_death_protection,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        encode_blocks_attacks,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
        encode_piercing_weapon,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        encode_kinetic_weapon,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
        encode_swing_animation,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
        encode_enchantments,
    ),
    encoder(component_id::VANILLA_DATA_COMPONENT_DYE_TYPE_ID, encode_dye),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
        encode_dyed_color,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID,
        encode_map_color,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID,
        encode_map_id,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
        encode_codec_backed_nbt,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        encode_charged_projectiles,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        encode_bundle_contents,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        encode_potion_contents,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
        encode_potion_duration_scale,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
        encode_suspicious_stew_effects,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        encode_writable_book_content,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
        encode_written_book_content,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        encode_armor_trim,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
        encode_codec_backed_nbt,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
        encode_entity_data,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        encode_bucket_entity_data,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
        encode_block_entity_data,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        encode_instrument,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        encode_provides_trim_material,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
        encode_ominous_bottle_amplifier,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        encode_jukebox_playable,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
        encode_provides_banner_patterns,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
        encode_codec_backed_nbt,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
        encode_lodestone_tracker,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
        encode_firework_explosion,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        encode_fireworks,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
        encode_profile,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID,
        encode_note_block_sound,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
        encode_banner_patterns,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID,
        encode_base_color,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
        encode_pot_decorations,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
        encode_container,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID,
        encode_block_state,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_BEES_TYPE_ID,
        encode_bees,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
        encode_codec_backed_nbt,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
        encode_codec_backed_nbt,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
        encode_break_sound,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
        encode_builtin_enum,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
        encode_builtin_enum,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
        encode_builtin_enum,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
        encode_builtin_enum,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID,
        encode_tropical_fish_pattern,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID,
        encode_tropical_fish_base_color,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
        encode_tropical_fish_pattern_color,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
        encode_builtin_enum,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
        encode_builtin_enum,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
        encode_builtin_enum,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        encode_painting_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
        encode_builtin_enum,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
        encode_builtin_enum,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID,
        encode_named_variant,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
        encode_builtin_enum,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
        encode_builtin_enum,
    ),
    encoder(
        component_id::VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
        encode_builtin_enum,
    ),
];

fn persistent_snbt_encoder(type_id: i32) -> Option<&'static PersistentSnbtEncoderDescriptor> {
    PERSISTENT_SNBT_ENCODERS
        .binary_search_by_key(&type_id, |descriptor| descriptor.type_id)
        .ok()
        .map(|index| &PERSISTENT_SNBT_ENCODERS[index])
}

#[cfg(test)]
pub(super) fn encode_added_component_value_snbt(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    encode_added_component_value_snbt_with_context(
        type_id,
        patch,
        PersistentSnbtContext {
            item_runtime,
            ..PersistentSnbtContext::default()
        },
    )
}

pub(super) fn encode_added_component_value_snbt_with_context(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let descriptor = persistent_snbt_encoder(type_id)?;
    (descriptor.encode)(type_id, patch, context)
}

fn encode_custom_data(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let value = patch.custom_data.as_ref()?;
    if !matches!(value, bbb_protocol::packets::NbtSummaryValue::Compound(_)) {
        return None;
    }
    Some(debug_nbt_summary_snbt(value))
}

fn encode_max_stack_size(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(
        patch
            .max_stack_size
            .filter(|value| (1..=99).contains(value))?
            .to_string(),
    )
}

fn encode_max_damage(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(patch.max_damage.filter(|value| *value > 0)?.to_string())
}

fn encode_damage(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(patch.damage.filter(|value| *value >= 0)?.to_string())
}

fn encode_unit(
    _type_id: i32,
    _patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some("{}".to_string())
}

fn encode_use_effects(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_use_effects_component_snbt(patch.use_effects?)
}

fn encode_component_tree(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let component = match type_id {
        component_id::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => {
            patch.custom_name_nbt.as_ref()?
        }
        component_id::VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => patch.item_name_nbt.as_ref()?,
        _ => return None,
    };
    Some(debug_nbt_summary_snbt(component))
}

fn encode_lore(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    if patch.lore_nbt.len() != patch.lore.len() {
        return None;
    }
    Some(format!(
        "[{}]",
        patch
            .lore_nbt
            .iter()
            .map(debug_nbt_summary_snbt)
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn encode_minimum_attack_charge(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_range_checked_float_component_snbt(patch.minimum_attack_charge_bits?, 0.0, Some(1.0))
}

fn encode_damage_type(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let resource_id = debug_damage_type_resource_id(patch.damage_type_id?, context)?;
    Some(debug_snbt_string(&resource_id))
}

fn encode_item_model(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_snbt_string(patch.item_model.as_deref()?))
}

fn encode_rarity(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_snbt_string(patch.rarity?.as_str()))
}

fn encode_enchantments(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let enchantments = match type_id {
        component_id::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID => &patch.enchantments,
        component_id::VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => {
            &patch.stored_enchantments
        }
        _ => return None,
    };
    let entries = enchantments
        .iter()
        .map(|enchantment| {
            let level = (1..=255)
                .contains(&enchantment.level)
                .then_some(enchantment.level)?;
            let resource_id = context.registry_resource_id(
                "minecraft:enchantment",
                enchantment.holder_id,
                |holder_id| {
                    VanillaEnchantmentIdentity::for_vanilla_registry_id(holder_id)
                        .map(VanillaEnchantmentIdentity::resource_id)
                        .map(str::to_string)
                },
            )?;
            Some(format!("{}: {level}", debug_snbt_string(&resource_id)))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(format!("{{{}}}", entries.join(", ")))
}

fn encode_adventure_mode_predicate(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let predicate = match type_id {
        component_id::VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID => {
            patch.can_place_on_persistent.as_ref()?
        }
        component_id::VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID => {
            patch.can_break_persistent.as_ref()?
        }
        _ => return None,
    };
    let predicates = predicate
        .predicates
        .iter()
        .map(|predicate| block_predicate_snbt(predicate, context))
        .collect::<Option<Vec<_>>>()?;
    match predicates.as_slice() {
        [] => None,
        [predicate] => Some(predicate.clone()),
        predicates => Some(format!("[{}]", predicates.join(", "))),
    }
}

fn block_predicate_snbt(
    predicate: &bbb_protocol::packets::BlockPredicateSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let blocks = BlockStateRegistry::vanilla_26_1();
    let mut fields = Vec::new();
    if let Some(holder_set) = &predicate.blocks {
        fields.push(format!(
            "blocks: {}",
            block_holder_set_snbt(
                &blocks,
                &holder_set.registry_ids,
                holder_set.tag.as_deref(),
                context,
            )?
        ));
    }
    if let Some(state) = &predicate.state {
        let mut names = HashSet::new();
        let state = state
            .iter()
            .map(|matcher| {
                names.insert(matcher.name.as_str()).then_some(())?;
                let value = match &matcher.value {
                    bbb_protocol::packets::StatePropertyValueMatcherSummary::Exact(value) => {
                        debug_snbt_string(value)
                    }
                    bbb_protocol::packets::StatePropertyValueMatcherSummary::Range { min, max } => {
                        let mut fields = Vec::new();
                        if let Some(min) = min {
                            fields.push(format!("min: {}", debug_snbt_string(min)));
                        }
                        if let Some(max) = max {
                            fields.push(format!("max: {}", debug_snbt_string(max)));
                        }
                        format!("{{{}}}", fields.join(", "))
                    }
                };
                Some(format!("{}: {value}", debug_snbt_string(&matcher.name)))
            })
            .collect::<Option<Vec<_>>>()?;
        fields.push(format!("state: {{{}}}", state.join(", ")));
    }
    if let Some(nbt) = &predicate.nbt {
        if !matches!(nbt, bbb_protocol::packets::NbtSummaryValue::Compound(_)) {
            return None;
        }
        fields.push(format!(
            "nbt: {}",
            debug_snbt_string(&debug_nbt_summary_snbt(nbt))
        ));
    }
    if let Some(exact) = &predicate.exact_components {
        let exact = exact_component_predicate_snbt(exact, context)?;
        if !exact.is_empty() {
            fields.push(format!("components: {{{}}}", exact.join(", ")));
        }
    }
    if !predicate.partial_components.is_empty() {
        let mut resource_ids = HashSet::new();
        let partial = predicate
            .partial_components
            .iter()
            .map(|predicate| {
                let resource_id = match predicate.predicate_type {
                    bbb_protocol::packets::DataComponentPredicateTypeSummary::Concrete(type_id) => {
                        bbb_protocol::data_component_types::vanilla_data_component_predicate_resource_id(type_id)?
                    }
                    bbb_protocol::packets::DataComponentPredicateTypeSummary::AnyComponent(type_id) => {
                        bbb_protocol::data_component_types::vanilla_data_component_descriptor(type_id)?
                            .resource_id()
                    }
                };
                resource_ids.insert(resource_id).then_some(())?;
                Some(format!(
                    "{}: {}",
                    debug_snbt_string(resource_id),
                    partial_data_component_predicate_snbt(predicate)?
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        fields.push(format!("predicates: {{{}}}", partial.join(", ")));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn exact_component_predicate_snbt(
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<Vec<String>> {
    if !patch.removed_type_ids.is_empty() {
        return None;
    }
    let mut resource_ids = HashSet::new();
    let mut fields = Vec::new();
    for type_id in &patch.added_type_ids {
        let descriptor =
            bbb_protocol::data_component_types::vanilla_data_component_descriptor(*type_id)?;
        if !descriptor.is_persistent() {
            continue;
        }
        resource_ids
            .insert(descriptor.resource_id())
            .then_some(())?;
        let value = encode_added_component_value_snbt_with_context(*type_id, patch, context)?;
        fields.push(format!(
            "{}: {value}",
            debug_snbt_string(descriptor.resource_id())
        ));
    }
    Some(fields)
}

fn partial_data_component_predicate_snbt(
    predicate: &bbb_protocol::packets::PartialDataComponentPredicateSummary,
) -> Option<String> {
    Some(debug_nbt_summary_snbt(predicate.wire_value.as_ref()?))
}

fn encode_attribute_modifiers(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let entries = patch
        .attribute_modifiers
        .iter()
        .map(|modifier| {
            let attribute = vanilla_attribute_type(modifier.attribute_id)?.resource_id;
            let operation = attribute_modifier_operation_name(modifier.operation_id)?;
            let slot = vanilla_equipment_slot_group(modifier.slot_id)?.serialized_name;
            let mut fields = vec![
                format!("type: {}", debug_snbt_string(attribute)),
                format!("id: {}", debug_snbt_string(&modifier.modifier_id)),
                format!(
                    "amount: {}d",
                    debug_nbt_java_double_string(f64::from_bits(modifier.amount_bits))
                ),
                format!("operation: {}", debug_snbt_string(operation)),
            ];
            if modifier.slot_id != VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID {
                fields.push(format!("slot: {}", debug_snbt_string(slot)));
            }
            match modifier.display_id {
                VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID => {}
                VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID => fields.push(format!(
                    "display: {{type: {}}}",
                    debug_snbt_string(attribute_modifier_display_name(modifier.display_id)?)
                )),
                _ => return None,
            }
            Some(format!("{{{}}}", fields.join(", ")))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(format!("[{}]", entries.join(", ")))
}

fn encode_custom_model_data(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_custom_model_data_component_snbt(patch))
}

fn encode_tooltip_display(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_tooltip_display_component_snbt(patch)
}

fn encode_repair_cost(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(patch.repair_cost.filter(|value| *value >= 0)?.to_string())
}

fn encode_enchantment_glint_override(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_snbt_bool(patch.enchantment_glint_override?).to_string())
}

fn encode_food(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_food_component_snbt(patch)
}

fn encode_consumable(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_consumable_component_snbt_with_context(patch, context)
}

fn encode_use_remainder(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_item_stack_template_snbt(patch.use_remainder.as_deref()?, context)
}

fn encode_use_cooldown(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_use_cooldown_component_snbt(patch)
}

fn encode_damage_resistant(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_damage_resistant_component_snbt_with_context(patch, context)
}

fn encode_tool(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let tool = patch.tool.as_ref()?;
    if tool.damage_per_block < 0 {
        return None;
    }
    let blocks = BlockStateRegistry::vanilla_26_1();
    let rules = tool
        .rules
        .iter()
        .map(|rule| {
            let mut fields = vec![format!(
                "blocks: {}",
                block_holder_set_snbt(
                    &blocks,
                    &rule.block_registry_ids,
                    rule.block_tag.as_deref(),
                    context,
                )?
            )];
            if let Some(bits) = rule.speed_bits {
                let speed = f32::from_bits(bits);
                if speed <= 0.0 || !speed.is_finite() {
                    return None;
                }
                fields.push(format!("speed: {}", debug_snbt_float(speed)?));
            }
            if let Some(correct_for_drops) = rule.correct_for_drops {
                fields.push(format!(
                    "correct_for_drops: {}",
                    debug_snbt_bool(correct_for_drops)
                ));
            }
            Some(format!("{{{}}}", fields.join(", ")))
        })
        .collect::<Option<Vec<_>>>()?
        .join(", ");
    let mut fields = vec![format!("rules: [{rules}]")];
    if tool.default_mining_speed_bits != 1.0_f32.to_bits() {
        fields.push(format!(
            "default_mining_speed: {}",
            debug_snbt_float(f32::from_bits(tool.default_mining_speed_bits))?
        ));
    }
    if tool.damage_per_block != 1 {
        fields.push(format!("damage_per_block: {}", tool.damage_per_block));
    }
    if !tool.can_destroy_blocks_in_creative {
        fields.push("can_destroy_blocks_in_creative: 0b".to_string());
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn block_holder_set_snbt(
    blocks: &BlockStateRegistry,
    registry_ids: &[i32],
    tag: Option<&str>,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    context.registry_holder_set_snbt("minecraft:block", registry_ids, tag, |holder_id| {
        blocks
            .block_name_by_registry_id(holder_id)
            .map(str::to_string)
    })
}

fn encode_equippable(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let equippable = patch.equippable.as_ref()?;
    let slot = vanilla_equipment_slot_by_id(equippable.slot_id)?.serialized_name;
    let mut fields = vec![format!("slot: {}", debug_snbt_string(slot))];
    let equip_sound_is_default = equippable
        .equip_sound
        .registry_id
        .is_some_and(|registry_id| {
            vanilla_sound_event_resource_id(registry_id)
                == Some("minecraft:item.armor.equip_generic")
        });
    if !equip_sound_is_default {
        fields.push(format!(
            "equip_sound: {}",
            debug_sound_event_component_snbt(&equippable.equip_sound)?
        ));
    }
    if let Some(asset_id) = &equippable.asset_id {
        fields.push(format!("asset_id: {}", debug_snbt_string(asset_id)));
    }
    if let Some(camera_overlay) = &equippable.camera_overlay {
        fields.push(format!(
            "camera_overlay: {}",
            debug_snbt_string(camera_overlay)
        ));
    }
    if let Some(allowed_entities) = &equippable.allowed_entities {
        fields.push(format!(
            "allowed_entities: {}",
            entity_type_holder_set_snbt(
                &allowed_entities.registry_ids,
                allowed_entities.tag.as_deref(),
                context,
            )?
        ));
    }
    if !equippable.dispensable {
        fields.push("dispensable: 0b".to_string());
    }
    if !equippable.swappable {
        fields.push("swappable: 0b".to_string());
    }
    if !equippable.damage_on_hurt {
        fields.push("damage_on_hurt: 0b".to_string());
    }
    if equippable.equip_on_interact {
        fields.push("equip_on_interact: 1b".to_string());
    }
    if equippable.can_be_sheared {
        fields.push("can_be_sheared: 1b".to_string());
    }
    let shearing_sound_is_default =
        equippable
            .shearing_sound
            .registry_id
            .is_some_and(|registry_id| {
                vanilla_sound_event_resource_id(registry_id) == Some("minecraft:item.shears.snip")
            });
    if !shearing_sound_is_default {
        fields.push(format!(
            "shearing_sound: {}",
            debug_sound_event_component_snbt(&equippable.shearing_sound)?
        ));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn entity_type_holder_set_snbt(
    registry_ids: &[i32],
    tag: Option<&str>,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    context.registry_holder_set_snbt("minecraft:entity_type", registry_ids, tag, |holder_id| {
        vanilla_entity_resource_id_for_type_id(holder_id)
    })
}

fn encode_weapon(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_weapon_component_snbt(patch)
}

fn encode_attack_range(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_attack_range_component_snbt(patch.attack_range?)
}

fn encode_enchantable(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let value = patch.enchantable_value.filter(|value| *value > 0)?;
    Some(format!("{{value: {value}}}"))
}

fn encode_repairable(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_repairable_component_snbt_with_context(patch, context)
}

fn encode_tooltip_style(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_snbt_string(patch.tooltip_style.as_deref()?))
}

fn encode_death_protection(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_death_protection_component_snbt_with_context(patch, context)
}

fn encode_blocks_attacks(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let blocks_attacks = patch.blocks_attacks.as_ref()?;
    let block_delay = f32::from_bits(blocks_attacks.block_delay_seconds_bits);
    let disable_cooldown = f32::from_bits(blocks_attacks.disable_cooldown_scale_bits);
    if block_delay < 0.0
        || !block_delay.is_finite()
        || disable_cooldown < 0.0
        || !disable_cooldown.is_finite()
    {
        return None;
    }
    let mut fields = Vec::new();
    if blocks_attacks.block_delay_seconds_bits != 0.0_f32.to_bits() {
        fields.push(format!(
            "block_delay_seconds: {}",
            debug_snbt_float(block_delay)?
        ));
    }
    if blocks_attacks.disable_cooldown_scale_bits != 1.0_f32.to_bits() {
        fields.push(format!(
            "disable_cooldown_scale: {}",
            debug_snbt_float(disable_cooldown)?
        ));
    }
    let default_reductions = matches!(
        blocks_attacks.damage_reductions.as_slice(),
        [reduction]
            if reduction.horizontal_blocking_angle_bits == 90.0_f32.to_bits()
                && reduction.damage_types.is_none()
                && reduction.base_bits == 0.0_f32.to_bits()
                && reduction.factor_bits == 1.0_f32.to_bits()
    );
    if !default_reductions {
        let reductions = blocks_attacks
            .damage_reductions
            .iter()
            .map(|reduction| damage_reduction_snbt(reduction, context))
            .collect::<Option<Vec<_>>>()?
            .join(", ");
        fields.push(format!("damage_reductions: [{reductions}]"));
    }
    let item_damage = &blocks_attacks.item_damage;
    let threshold = f32::from_bits(item_damage.threshold_bits);
    if threshold < 0.0 || !threshold.is_finite() {
        return None;
    }
    let default_item_damage = item_damage.threshold_bits == 1.0_f32.to_bits()
        && item_damage.base_bits == 0.0_f32.to_bits()
        && item_damage.factor_bits == 1.0_f32.to_bits();
    if !default_item_damage {
        fields.push(format!(
            "item_damage: {{threshold: {}, base: {}, factor: {}}}",
            debug_snbt_float(threshold)?,
            debug_snbt_float(f32::from_bits(item_damage.base_bits))?,
            debug_snbt_float(f32::from_bits(item_damage.factor_bits))?
        ));
    }
    if let Some(bypassed_by) = &blocks_attacks.bypassed_by {
        fields.push(format!(
            "bypassed_by: {}",
            debug_damage_type_holder_set_snbt(
                &bypassed_by.registry_ids,
                bypassed_by.tag.as_deref(),
                context,
            )?
        ));
    }
    if let Some(block_sound) = &blocks_attacks.block_sound {
        fields.push(format!(
            "block_sound: {}",
            debug_sound_event_component_snbt(block_sound)?
        ));
    }
    if let Some(disable_sound) = &blocks_attacks.disable_sound {
        fields.push(format!(
            "disabled_sound: {}",
            debug_sound_event_component_snbt(disable_sound)?
        ));
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn damage_reduction_snbt(
    reduction: &bbb_protocol::packets::DamageReductionSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let angle = f32::from_bits(reduction.horizontal_blocking_angle_bits);
    if angle <= 0.0 || !angle.is_finite() {
        return None;
    }
    let mut fields = Vec::new();
    if reduction.horizontal_blocking_angle_bits != 90.0_f32.to_bits() {
        fields.push(format!(
            "horizontal_blocking_angle: {}",
            debug_snbt_float(angle)?
        ));
    }
    if let Some(damage_types) = &reduction.damage_types {
        fields.push(format!(
            "type: {}",
            debug_damage_type_holder_set_snbt(
                &damage_types.registry_ids,
                damage_types.tag.as_deref(),
                context,
            )?
        ));
    }
    fields.push(format!(
        "base: {}",
        debug_snbt_float(f32::from_bits(reduction.base_bits))?
    ));
    fields.push(format!(
        "factor: {}",
        debug_snbt_float(f32::from_bits(reduction.factor_bits))?
    ));
    Some(format!("{{{}}}", fields.join(", ")))
}

fn encode_piercing_weapon(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_piercing_weapon_component_snbt(patch.piercing_weapon.as_ref()?)
}

fn encode_kinetic_weapon(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_kinetic_weapon_component_snbt(patch.kinetic_weapon.as_ref()?)
}

fn encode_swing_animation(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_swing_animation_component_snbt(patch.swing_animation?)
}

fn encode_dye(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_snbt_string(vanilla_dye_color_name(
        patch.dye_color_id?,
    )))
}

fn encode_dyed_color(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(patch.dyed_color?.to_string())
}

fn encode_map_color(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(patch.map_color?.to_string())
}

fn encode_map_id(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(patch.map_id?.to_string())
}

fn encode_codec_backed_nbt(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_nbt_summary_snbt(
        patch.codec_backed_nbt.get(&type_id)?,
    ))
}

fn encode_bucket_entity_data(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let value = patch.codec_backed_nbt.get(&type_id)?;
    if !matches!(value, bbb_protocol::packets::NbtSummaryValue::Compound(_)) {
        return None;
    }
    Some(debug_nbt_summary_snbt(value))
}

fn encode_entity_data(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_entity_data_component_snbt(patch)
}

fn encode_block_entity_data(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_block_entity_data_component_snbt(patch)
}

fn encode_instrument(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    if let Some(persistent) = patch.codec_backed_nbt.get(&type_id) {
        return Some(debug_nbt_summary_snbt(persistent));
    }
    if let Some(holder_id) = patch.instrument_id {
        if patch.instrument_direct.is_some() {
            return None;
        }
        let resource_id =
            context.registry_resource_id("minecraft:instrument", holder_id, |holder_id| {
                VanillaInstrumentIdentity::for_vanilla_registry_id(holder_id)
                    .map(VanillaInstrumentIdentity::resource_id)
                    .map(str::to_string)
            })?;
        return Some(debug_snbt_string(&resource_id));
    }
    encode_direct_instrument(patch.instrument_direct.as_deref()?, context)
}

fn encode_direct_instrument(
    instrument: &bbb_protocol::packets::InstrumentSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let use_duration = f32::from_bits(instrument.use_duration_bits);
    let range = f32::from_bits(instrument.range_bits);
    if !(use_duration > 0.0 && use_duration <= f32::MAX) || !(range > 0.0 && range <= f32::MAX) {
        return None;
    }
    Some(format!(
        "{{sound_event: {}, use_duration: {}, range: {}, description: {}}}",
        encode_persistent_sound_event(&instrument.sound_event, context)?,
        debug_snbt_float(use_duration)?,
        debug_snbt_float(range)?,
        debug_nbt_summary_snbt(instrument.description_nbt.as_deref()?),
    ))
}

fn encode_persistent_sound_event(
    sound: &bbb_protocol::packets::SoundEventSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    match (sound.registry_id, sound.sound_id.as_deref()) {
        (Some(holder_id), None) if sound.fixed_range_bits.is_none() => {
            let resource_id =
                context.registry_resource_id("minecraft:sound_event", holder_id, |holder_id| {
                    vanilla_sound_event_resource_id(holder_id).map(str::to_string)
                })?;
            Some(debug_snbt_string(&resource_id))
        }
        (None, Some(sound_id)) => {
            let sound_id = bbb_protocol::normalize_resource_location(sound_id.to_string()).ok()?;
            let mut fields = vec![format!("sound_id: {}", debug_snbt_string(&sound_id))];
            if let Some(bits) = sound.fixed_range_bits {
                fields.push(format!(
                    "range: {}",
                    debug_snbt_float(f32::from_bits(bits))?
                ));
            }
            Some(format!("{{{}}}", fields.join(", ")))
        }
        _ => None,
    }
}

fn encode_provides_trim_material(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    if let Some(persistent) = patch.codec_backed_nbt.get(&type_id) {
        return Some(debug_nbt_summary_snbt(persistent));
    }
    if let Some(material) = patch.provides_trim_material_direct.as_deref() {
        return encode_direct_trim_material(material);
    }
    let holder_id = patch.provides_trim_material_id?;
    let resource_id =
        context.registry_resource_id("minecraft:trim_material", holder_id, |holder_id| {
            VanillaTrimMaterialIdentity::for_vanilla_registry_id(holder_id)
                .map(VanillaTrimMaterialIdentity::resource_id)
                .map(str::to_string)
        })?;
    Some(debug_snbt_string(&resource_id))
}

fn encode_charged_projectiles(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_item_stack_template_list_snbt(&patch.charged_projectiles_items, context)
}

fn encode_bundle_contents(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_item_stack_template_list_snbt(&patch.bundle_contents_items, context)
}

fn encode_potion_contents(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_potion_contents_component_snbt(patch)
}

fn encode_potion_duration_scale(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_range_checked_float_component_snbt(patch.potion_duration_scale_bits?, 0.0, None)
}

fn encode_suspicious_stew_effects(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let effects = patch
        .suspicious_stew_effects
        .iter()
        .map(|effect| {
            let mut fields = vec![format!(
                "id: {}",
                debug_snbt_string(vanilla_mob_effect_type(effect.effect_id)?.resource_id)
            )];
            if effect.duration != 160 {
                fields.push(format!("duration: {}", effect.duration));
            }
            Some(format!("{{{}}}", fields.join(", ")))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(format!("[{}]", effects.join(", ")))
}

fn encode_writable_book_content(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_writable_book_content_component_snbt(patch))
}

fn encode_written_book_content(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let book = patch.written_book.as_ref()?;
    if book.title.chars().count() > 32 || !(0..=3).contains(&book.generation) {
        return None;
    }
    let page_count = book.pages.len();
    if book.page_filters.len() != page_count
        || patch.written_book_pages_nbt.len() != page_count
        || patch.written_book_page_filters_nbt.len() != page_count
    {
        return None;
    }

    let mut fields = vec![format!(
        "title: {}",
        filterable_string_snbt(&book.title, book.title_filter.as_deref())
    )];
    fields.push(format!("author: {}", debug_snbt_string(&book.author)));
    if book.generation != 0 {
        fields.push(format!("generation: {}", book.generation));
    }
    if page_count != 0 {
        let pages = (0..page_count)
            .map(|index| {
                let raw = patch.written_book_pages_nbt[index].as_ref()?;
                let filtered = match (
                    book.page_filters[index].as_ref(),
                    patch.written_book_page_filters_nbt[index].as_ref(),
                ) {
                    (None, None) => None,
                    (Some(_), Some(filtered)) => Some(filtered),
                    _ => return None,
                };
                Some(filterable_component_snbt(raw, filtered))
            })
            .collect::<Option<Vec<_>>>()?
            .join(", ");
        fields.push(format!("pages: [{pages}]"));
    }
    if book.resolved {
        fields.push("resolved: 1b".to_string());
    }
    Some(format!("{{{}}}", fields.join(", ")))
}

fn filterable_string_snbt(raw: &str, filtered: Option<&str>) -> String {
    let mut fields = vec![format!("raw: {}", debug_snbt_string(raw))];
    if let Some(filtered) = filtered {
        fields.push(format!("filtered: {}", debug_snbt_string(filtered)));
    }
    format!("{{{}}}", fields.join(", "))
}

fn filterable_component_snbt(
    raw: &bbb_protocol::packets::NbtSummaryValue,
    filtered: Option<&bbb_protocol::packets::NbtSummaryValue>,
) -> String {
    let mut fields = vec![format!("raw: {}", debug_nbt_summary_snbt(raw))];
    if let Some(filtered) = filtered {
        fields.push(format!("filtered: {}", debug_nbt_summary_snbt(filtered)));
    }
    format!("{{{}}}", fields.join(", "))
}

fn encode_armor_trim(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    if let Some(persistent) = patch.codec_backed_nbt.get(&type_id) {
        return Some(debug_nbt_summary_snbt(persistent));
    }
    let material = if let Some(material) = patch.armor_trim_material_direct.as_deref() {
        encode_direct_trim_material(material)?
    } else {
        let holder_id = patch.armor_trim_material_id?;
        let resource_id =
            context.registry_resource_id("minecraft:trim_material", holder_id, |holder_id| {
                VanillaTrimMaterialIdentity::for_vanilla_registry_id(holder_id)
                    .map(VanillaTrimMaterialIdentity::resource_id)
                    .map(str::to_string)
            })?;
        debug_snbt_string(&resource_id)
    };
    let pattern = if let Some(pattern) = patch.armor_trim_pattern_direct.as_deref() {
        encode_direct_trim_pattern(pattern)?
    } else {
        let holder_id = patch.armor_trim_pattern_id?;
        let resource_id =
            context.registry_resource_id("minecraft:trim_pattern", holder_id, |holder_id| {
                VanillaTrimPatternIdentity::for_vanilla_registry_id(holder_id)
                    .map(VanillaTrimPatternIdentity::resource_id)
                    .map(str::to_string)
            })?;
        debug_snbt_string(&resource_id)
    };
    Some(format!("{{material: {}, pattern: {}}}", material, pattern))
}

fn encode_direct_trim_material(
    material: &bbb_protocol::packets::TrimMaterialSummary,
) -> Option<String> {
    let material = direct_trim_material_presentation(material)?;
    let mut fields = vec![format!(
        "asset_name: {}",
        debug_snbt_string(&material.asset_name)
    )];
    if !material.override_armor_assets.is_empty() {
        fields.push(format!(
            "override_armor_assets: {{{}}}",
            material
                .override_armor_assets
                .iter()
                .map(|(asset, suffix)| format!(
                    "{}: {}",
                    debug_snbt_compound_key(asset),
                    debug_snbt_string(suffix)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    fields.push(format!(
        "description: {}",
        debug_nbt_summary_snbt(material.description_nbt.as_ref()?)
    ));
    Some(format!("{{{}}}", fields.join(", ")))
}

fn encode_direct_trim_pattern(
    pattern: &bbb_protocol::packets::TrimPatternSummary,
) -> Option<String> {
    let pattern = direct_trim_pattern_presentation(pattern)?;
    Some(format!(
        "{{asset_id: {}, description: {}, decal: {}b}}",
        debug_snbt_string(&pattern.asset_id),
        debug_nbt_summary_snbt(pattern.description_nbt.as_ref()?),
        u8::from(pattern.decal)
    ))
}

fn encode_ominous_bottle_amplifier(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(
        patch
            .ominous_bottle_amplifier
            .filter(|value| (0..=4).contains(value))?
            .to_string(),
    )
}

fn encode_jukebox_playable(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let holder_id = patch.jukebox_song_id?;
    let resource_id =
        context.registry_resource_id("minecraft:jukebox_song", holder_id, |holder_id| {
            context
                .item_runtime?
                .jukebox_song_resource_id(holder_id)
                .map(str::to_string)
        })?;
    Some(debug_snbt_string(&resource_id))
}

fn encode_provides_banner_patterns(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_provides_banner_patterns_component_snbt_with_context(patch, context)
}

fn encode_lodestone_tracker(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_lodestone_tracker_component_snbt(patch))
}

fn encode_firework_explosion(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_firework_explosion_snbt(&FireworkExplosionSummary {
        shape: patch.firework_explosion_shape?,
        colors: patch.firework_explosion_colors.clone(),
        fade_colors: patch.firework_explosion_fade_colors.clone(),
        has_trail: patch.firework_explosion_has_trail.unwrap_or(false),
        has_twinkle: patch.firework_explosion_has_twinkle.unwrap_or(false),
    }))
}

fn encode_fireworks(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_fireworks_component_snbt(patch))
}

fn encode_profile(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_profile_component_snbt(patch.profile.as_ref()?)
}

fn encode_note_block_sound(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_snbt_string(patch.note_block_sound.as_deref()?))
}

fn encode_banner_patterns(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let layers = patch
        .banner_pattern_layers
        .iter()
        .map(|layer| {
            let pattern = if let Some(registry_id) = layer.registry_id {
                let resource_id = debug_banner_pattern_resource_id(registry_id, context)?;
                debug_snbt_string(&resource_id)
            } else {
                let presentation = direct_banner_pattern_presentation(
                    layer.asset_id.as_deref()?,
                    layer.translation_key.as_deref()?,
                )?;
                format!(
                    "{{asset_id: {}, translation_key: {}}}",
                    debug_snbt_string(&presentation.asset_id),
                    debug_snbt_string(&presentation.translation_key)
                )
            };
            Some(format!(
                "{{pattern: {pattern}, color: {}}}",
                debug_snbt_string(vanilla_dye_color_name(layer.color_id))
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(format!("[{}]", layers.join(", ")))
}

fn encode_base_color(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_snbt_string(vanilla_dye_color_name(
        patch.base_color_id?,
    )))
}

fn encode_pot_decorations(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_pot_decorations_component_snbt(patch, context.item_runtime)
}

fn encode_container(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_item_container_contents_component_snbt(patch, context)
}

fn encode_block_state(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(format!(
        "{{{}}}",
        patch
            .block_state_properties
            .iter()
            .map(|(name, value)| format!(
                "{}: {}",
                debug_snbt_compound_key(name),
                debug_snbt_string(value)
            ))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn encode_bees(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_bees_component_snbt(&patch.bees)
}

fn encode_break_sound(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    debug_sound_event_component_snbt(patch.break_sound.as_ref()?)
}

fn encode_named_variant(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let holder_id = named_variant_holder_id(type_id, patch)?;
    let resource_id = match component_id::vanilla_data_component_animal_variant_registry(type_id) {
        Some(registry) => context.registry_resource_id(registry, holder_id, |holder_id| {
            vanilla_named_variant_resource_id(type_id, holder_id).map(str::to_string)
        })?,
        None => vanilla_named_variant_resource_id(type_id, holder_id)?.to_string(),
    };
    Some(debug_snbt_string(&resource_id))
}

fn named_variant_holder_id(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
) -> Option<i32> {
    match type_id {
        component_id::VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => patch.villager_variant_id,
        component_id::VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => patch.wolf_variant_id,
        component_id::VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => {
            patch.wolf_sound_variant_id
        }
        component_id::VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => patch.pig_variant_id,
        component_id::VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => {
            patch.pig_sound_variant_id
        }
        component_id::VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => patch.cow_variant_id,
        component_id::VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => {
            patch.cow_sound_variant_id
        }
        component_id::VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => patch.chicken_variant_id,
        component_id::VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => {
            patch.chicken_sound_variant_id
        }
        component_id::VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => {
            patch.zombie_nautilus_variant_id
        }
        component_id::VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => patch.frog_variant_id,
        component_id::VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => patch.cat_variant_id,
        component_id::VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => {
            patch.cat_sound_variant_id
        }
        _ => None,
    }
}

fn vanilla_named_variant_resource_id(type_id: i32, holder_id: i32) -> Option<&'static str> {
    let resource_id = match type_id {
        component_id::VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => {
            VillagerModelType::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => {
            WolfModelVariant::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => {
            WolfSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => {
            PigModelVariant::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => {
            PigSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => {
            CowModelVariant::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => {
            CowSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => {
            ChickenModelVariant::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => {
            ChickenSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => {
            ZombieNautilusModelVariant::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => {
            FrogModelVariant::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => {
            CatModelVariant::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        component_id::VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => {
            CatSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id()
        }
        _ => return None,
    };
    Some(resource_id)
}

fn encode_tropical_fish_pattern(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_snbt_string(
        vanilla_tropical_fish_pattern(patch.tropical_fish_pattern_id?).serialized_name,
    ))
}

fn encode_tropical_fish_base_color(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_snbt_string(vanilla_dye_color_name(
        patch.tropical_fish_base_color_id?,
    )))
}

fn encode_tropical_fish_pattern_color(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    Some(debug_snbt_string(vanilla_dye_color_name(
        patch.tropical_fish_pattern_color_id?,
    )))
}

fn encode_painting_variant(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    context: PersistentSnbtContext<'_>,
) -> Option<String> {
    if patch.painting_variant_direct.is_some() {
        return None;
    }
    let holder_id = patch.painting_variant_id?;
    let resource_id =
        context.registry_resource_id("minecraft:painting_variant", holder_id, |holder_id| {
            vanilla_painting_variant_resource_id(holder_id).map(str::to_string)
        })?;
    Some(debug_snbt_string(&resource_id))
}

fn encode_builtin_enum(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _context: PersistentSnbtContext<'_>,
) -> Option<String> {
    let id = *patch.builtin_enum_component_ids.get(&type_id)?;
    Some(debug_snbt_string(builtin_enum_component_value_name(
        type_id, id,
    )?))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use bbb_protocol::builtin_enum_component_values::{
        VANILLA_DYE_COLOR_LIGHT_BLUE_ID, VANILLA_DYE_COLOR_RED_ID,
    };
    use bbb_protocol::mob_effect_types::{
        VANILLA_MOB_EFFECT_POISON_ID, VANILLA_MOB_EFFECT_SPEED_ID, VANILLA_MOB_EFFECT_TYPES,
    };

    fn world_with_registries(registries: &[(&str, &[&str])]) -> WorldStore {
        let mut world = WorldStore::new();
        for (registry, resource_ids) in registries {
            world.record_registry_entries(
                *registry,
                0,
                resource_ids
                    .iter()
                    .map(|resource_id| bbb_world::RegistryPacketEntry::stub(*resource_id))
                    .collect(),
            );
        }
        world
    }

    fn registry_context(world: &WorldStore) -> PersistentSnbtContext<'_> {
        PersistentSnbtContext {
            registry_access: Some(world.frozen_registry_access()),
            ..Default::default()
        }
    }

    #[test]
    fn production_encoder_matches_do_not_use_raw_component_type_ids() {
        let production = include_str!("component_patch.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        let violations = production
            .lines()
            .filter(|line| {
                let Some((pattern, _)) = line.split_once("=>") else {
                    return false;
                };
                pattern
                    .split(|character: char| !character.is_ascii_digit())
                    .any(|token| !token.is_empty() && token.parse::<i32>().is_ok())
            })
            .collect::<Vec<_>>();
        assert!(
            violations.is_empty(),
            "raw numeric encoder match arms: {violations:?}"
        );
    }

    #[test]
    fn encoder_registry_is_unique_sorted_and_persistent() {
        let expected_type_ids = bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_TYPES
            .iter()
            .filter(|descriptor| descriptor.persistent)
            .map(|descriptor| descriptor.type_id)
            .collect::<Vec<_>>();
        assert_eq!(PERSISTENT_SNBT_ENCODERS.len(), expected_type_ids.len());
        let mut ids = HashSet::new();
        let mut previous = None;
        for (descriptor, expected_type_id) in PERSISTENT_SNBT_ENCODERS.iter().zip(expected_type_ids)
        {
            assert_eq!(descriptor.type_id, expected_type_id);
            assert!(ids.insert(descriptor.type_id));
            if let Some(previous) = previous {
                assert!(previous < descriptor.type_id);
            }
            previous = Some(descriptor.type_id);
            assert!(
                bbb_protocol::data_component_types::vanilla_data_component_descriptor(
                    descriptor.type_id
                )
                .unwrap()
                .is_persistent()
            );
        }
    }

    #[test]
    fn encoder_registry_dispatches_and_omits_unregistered_values() {
        let patch = bbb_protocol::packets::DataComponentPatchSummary {
            max_stack_size: Some(64),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(1, &patch, None),
            Some("64".to_string())
        );
        assert_eq!(encode_added_component_value_snbt(20, &patch, None), None);
    }

    #[test]
    fn custom_data_persistent_projection_requires_and_preserves_compounds() {
        let compound = bbb_protocol::packets::NbtSummaryValue::Compound(vec![
            bbb_protocol::packets::NbtSummaryEntry {
                name: "answer".to_string(),
                value: bbb_protocol::packets::NbtSummaryValue::Int(42),
            },
        ]);
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            custom_data: Some(compound),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(
                component_id::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                &patch,
                None,
            ),
            Some("{answer: 42}".to_string())
        );

        patch.custom_data = Some(bbb_protocol::packets::NbtSummaryValue::Int(42));
        assert_eq!(
            encode_added_component_value_snbt(
                component_id::VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
                &patch,
                None,
            ),
            None
        );
    }

    #[test]
    fn bucket_entity_data_persistent_projection_requires_and_preserves_compounds() {
        let type_id = component_id::VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID;
        let compound = bbb_protocol::packets::NbtSummaryValue::Compound(vec![
            bbb_protocol::packets::NbtSummaryEntry {
                name: "Variant".to_string(),
                value: bbb_protocol::packets::NbtSummaryValue::Int(2),
            },
        ]);
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            codec_backed_nbt: std::collections::BTreeMap::from([(type_id, compound)]),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(type_id, &patch, None),
            Some("{Variant: 2}".to_string())
        );

        patch
            .codec_backed_nbt
            .insert(type_id, bbb_protocol::packets::NbtSummaryValue::Int(2));
        assert_eq!(
            encode_added_component_value_snbt(type_id, &patch, None),
            None
        );
    }

    #[test]
    fn adventure_mode_predicate_encoder_preserves_codec_fields_and_compact_lists() {
        use bbb_protocol::packets::{
            AdventureModePredicateDataSummary, BlockHolderSetSummary, BlockPredicateSummary,
            DataComponentPatchSummary, DataComponentPredicateTypeSummary, NbtSummaryEntry,
            NbtSummaryValue, PartialDataComponentPredicateSummary, StatePropertyMatcherSummary,
            StatePropertyValueMatcherSummary,
        };

        let rich_predicate = BlockPredicateSummary {
            blocks: Some(BlockHolderSetSummary {
                registry_ids: vec![1],
                tag: None,
            }),
            state: Some(vec![
                StatePropertyMatcherSummary {
                    name: "facing".to_string(),
                    value: StatePropertyValueMatcherSummary::Exact("north".to_string()),
                },
                StatePropertyMatcherSummary {
                    name: "age".to_string(),
                    value: StatePropertyValueMatcherSummary::Range {
                        min: Some("1".to_string()),
                        max: Some("3".to_string()),
                    },
                },
            ]),
            nbt: Some(NbtSummaryValue::Compound(Vec::new())),
            exact_components: Some(Box::new(DataComponentPatchSummary {
                added: 2,
                added_type_ids: vec![1, component_id::VANILLA_DATA_COMPONENT_LOCK_TYPE_ID],
                max_stack_size: Some(64),
                codec_backed_nbt: std::collections::BTreeMap::from([(
                    component_id::VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
                    NbtSummaryValue::Compound(Vec::new()),
                )]),
                ..Default::default()
            })),
            partial_components: vec![
                PartialDataComponentPredicateSummary {
                    predicate_type: DataComponentPredicateTypeSummary::AnyComponent(6),
                    wire_value: Some(NbtSummaryValue::Compound(Vec::new())),
                },
                PartialDataComponentPredicateSummary {
                    predicate_type: DataComponentPredicateTypeSummary::Concrete(0),
                    wire_value: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "durability".to_string(),
                        value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                            name: "min".to_string(),
                            value: NbtSummaryValue::Int(1),
                        }]),
                    }])),
                },
            ],
        };
        let mut patch = DataComponentPatchSummary {
            can_place_on_persistent: Some(AdventureModePredicateDataSummary {
                predicates: vec![rich_predicate],
            }),
            can_break_persistent: Some(AdventureModePredicateDataSummary {
                predicates: vec![
                    BlockPredicateSummary::default(),
                    BlockPredicateSummary {
                        blocks: Some(BlockHolderSetSummary {
                            registry_ids: Vec::new(),
                            tag: Some("minecraft:mineable/pickaxe".to_string()),
                        }),
                        ..Default::default()
                    },
                ],
            }),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(14, &patch, None),
            Some(
                "{blocks: \"minecraft:stone\", state: {\"facing\": \"north\", \
                 \"age\": {min: \"1\", max: \"3\"}}, nbt: \"{}\", components: \
                 {\"minecraft:max_stack_size\": 64, \"minecraft:lock\": {}}, predicates: \
                 {\"minecraft:custom_name\": {}, \"minecraft:damage\": \
                 {durability: {min: 1}}}}"
                    .to_string()
            )
        );
        assert_eq!(
            encode_added_component_value_snbt(15, &patch, None),
            Some("[{}, {blocks: \"#minecraft:mineable/pickaxe\"}]".to_string())
        );

        let mut transient_exact = patch.clone();
        transient_exact
            .can_place_on_persistent
            .as_mut()
            .unwrap()
            .predicates[0]
            .exact_components = Some(Box::new(DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![20],
            ..Default::default()
        }));
        assert!(
            !encode_added_component_value_snbt(14, &transient_exact, None)
                .unwrap()
                .contains("components:")
        );

        let mut unsupported_exact = patch.clone();
        unsupported_exact
            .can_place_on_persistent
            .as_mut()
            .unwrap()
            .predicates[0]
            .exact_components = Some(Box::new(DataComponentPatchSummary {
            added: 1,
            added_type_ids: vec![32],
            ..Default::default()
        }));
        assert_eq!(
            encode_added_component_value_snbt(14, &unsupported_exact, None),
            None
        );

        let mut invalid_partial = patch.clone();
        invalid_partial
            .can_place_on_persistent
            .as_mut()
            .unwrap()
            .predicates[0]
            .partial_components[1]
            .wire_value = None;
        assert_eq!(
            encode_added_component_value_snbt(14, &invalid_partial, None),
            None
        );

        patch.can_place_on_persistent.as_mut().unwrap().predicates[0]
            .blocks
            .as_mut()
            .unwrap()
            .registry_ids[0] = 1168;
        assert_eq!(encode_added_component_value_snbt(14, &patch, None), None);
        patch
            .can_break_persistent
            .as_mut()
            .unwrap()
            .predicates
            .clear();
        assert_eq!(encode_added_component_value_snbt(15, &patch, None), None);
    }

    #[test]
    fn tool_encoder_resolves_block_holders_and_validates_codec_ranges() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            tool: Some(bbb_protocol::packets::ToolSummary {
                rules: vec![
                    bbb_protocol::packets::ToolRuleSummary {
                        block_registry_ids: Vec::new(),
                        block_tag: Some("minecraft:mineable/pickaxe".to_string()),
                        speed_bits: Some(8.0_f32.to_bits()),
                        correct_for_drops: Some(true),
                    },
                    bbb_protocol::packets::ToolRuleSummary {
                        block_registry_ids: vec![1, 193],
                        block_tag: None,
                        speed_bits: None,
                        correct_for_drops: Some(false),
                    },
                ],
                default_mining_speed_bits: 1.5_f32.to_bits(),
                damage_per_block: 2,
                can_destroy_blocks_in_creative: false,
            }),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(28, &patch, None),
            Some(
                "{rules: [{blocks: \"#minecraft:mineable/pickaxe\", speed: 8.0f, \
                 correct_for_drops: 1b}, {blocks: [\"minecraft:stone\", \
                 \"minecraft:obsidian\"], correct_for_drops: 0b}], \
                 default_mining_speed: 1.5f, damage_per_block: 2, \
                 can_destroy_blocks_in_creative: 0b}"
                    .to_string()
            )
        );

        let mut world = WorldStore::new();
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            None
        );
        world.apply_update_tags(bbb_protocol::packets::UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![bbb_protocol::packets::TagNetworkPayload {
                    tag: "minecraft:mineable/pickaxe".to_string(),
                    entries: vec![1],
                }],
            }],
        });
        assert!(encode_added_component_value_snbt_with_context(
            component_id::VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
            &patch,
            registry_context(&world),
        )
        .is_some());

        patch.tool.as_mut().unwrap().rules[1].block_registry_ids[1] = 1168;
        assert_eq!(encode_added_component_value_snbt(28, &patch, None), None);
        patch.tool.as_mut().unwrap().rules[1].block_registry_ids[1] = 193;
        patch.tool.as_mut().unwrap().rules[0].speed_bits = Some(0.0_f32.to_bits());
        assert_eq!(encode_added_component_value_snbt(28, &patch, None), None);
        patch.tool.as_mut().unwrap().rules[0].speed_bits = Some(f32::INFINITY.to_bits());
        assert_eq!(encode_added_component_value_snbt(28, &patch, None), None);
        patch.tool.as_mut().unwrap().rules[0].speed_bits = Some(8.0_f32.to_bits());
        patch.tool.as_mut().unwrap().damage_per_block = -1;
        assert_eq!(encode_added_component_value_snbt(28, &patch, None), None);
    }

    #[test]
    fn equippable_encoder_preserves_holders_and_codec_defaults() {
        use bbb_protocol::packets::{
            DataComponentPatchSummary, EntityTypeHolderSetSummary, EquippableSummary,
            SoundEventSummary,
        };

        let sound_registry_id = |resource_id| {
            (0..2048)
                .find(|id| vanilla_sound_event_resource_id(*id) == Some(resource_id))
                .unwrap()
        };
        let mut patch = DataComponentPatchSummary {
            equippable: Some(EquippableSummary {
                slot_id: 0,
                equip_sound: SoundEventSummary {
                    registry_id: Some(sound_registry_id("minecraft:item.armor.equip_generic")),
                    sound_id: None,
                    fixed_range_bits: None,
                },
                asset_id: None,
                camera_overlay: None,
                allowed_entities: None,
                dispensable: true,
                swappable: true,
                damage_on_hurt: true,
                equip_on_interact: false,
                can_be_sheared: false,
                shearing_sound: SoundEventSummary {
                    registry_id: Some(sound_registry_id("minecraft:item.shears.snip")),
                    sound_id: None,
                    fixed_range_bits: None,
                },
            }),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(32, &patch, None),
            Some("{slot: \"mainhand\"}".to_string())
        );

        patch.equippable = Some(EquippableSummary {
            slot_id: 5,
            equip_sound: SoundEventSummary {
                registry_id: None,
                sound_id: Some("minecraft:item.armor.equip_generic".to_string()),
                fixed_range_bits: None,
            },
            asset_id: Some("minecraft:diamond".to_string()),
            camera_overlay: Some("minecraft:misc/pumpkinblur".to_string()),
            allowed_entities: Some(EntityTypeHolderSetSummary {
                registry_ids: Vec::new(),
                tag: Some("minecraft:skeletons".to_string()),
            }),
            dispensable: true,
            swappable: false,
            damage_on_hurt: true,
            equip_on_interact: false,
            can_be_sheared: true,
            shearing_sound: SoundEventSummary {
                registry_id: None,
                sound_id: Some("minecraft:item.shears.snip".to_string()),
                fixed_range_bits: None,
            },
        });
        assert_eq!(
            encode_added_component_value_snbt(32, &patch, None),
            Some(
                "{slot: \"offhand\", equip_sound: \
                 {sound_id: \"minecraft:item.armor.equip_generic\"}, asset_id: \
                 \"minecraft:diamond\", camera_overlay: \"minecraft:misc/pumpkinblur\", \
                 allowed_entities: \"#minecraft:skeletons\", swappable: 0b, \
                 can_be_sheared: 1b, shearing_sound: \
                 {sound_id: \"minecraft:item.shears.snip\"}}"
                    .to_string()
            )
        );

        let mut world = WorldStore::new();
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            None
        );
        world.apply_update_tags(bbb_protocol::packets::UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:entity_type".to_string(),
                tags: vec![bbb_protocol::packets::TagNetworkPayload {
                    tag: "minecraft:skeletons".to_string(),
                    entries: vec![0],
                }],
            }],
        });
        assert!(encode_added_component_value_snbt_with_context(
            component_id::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            &patch,
            registry_context(&world),
        )
        .is_some());

        patch.equippable.as_mut().unwrap().slot_id = 8;
        assert_eq!(encode_added_component_value_snbt(32, &patch, None), None);
        patch.equippable.as_mut().unwrap().slot_id = 5;
        patch.equippable.as_mut().unwrap().allowed_entities = Some(EntityTypeHolderSetSummary {
            registry_ids: vec![10_000],
            tag: None,
        });
        assert_eq!(encode_added_component_value_snbt(32, &patch, None), None);
    }

    #[test]
    fn blocks_attacks_encoder_preserves_reductions_holders_and_defaults() {
        use bbb_protocol::packets::{
            BlocksAttacksSummary, DamageReductionSummary, DamageTypeHolderSetSummary,
            DataComponentPatchSummary, ItemDamageFunctionSummary, SoundEventSummary,
        };

        let mut patch = DataComponentPatchSummary {
            blocks_attacks: Some(BlocksAttacksSummary {
                block_delay_seconds_bits: 0.25_f32.to_bits(),
                disable_cooldown_scale_bits: 1.5_f32.to_bits(),
                damage_reductions: vec![DamageReductionSummary {
                    horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
                    damage_types: Some(DamageTypeHolderSetSummary {
                        registry_ids: Vec::new(),
                        tag: Some("minecraft:bypasses_shield".to_string()),
                    }),
                    base_bits: 1.0_f32.to_bits(),
                    factor_bits: 0.5_f32.to_bits(),
                }],
                item_damage: ItemDamageFunctionSummary {
                    threshold_bits: 1.0_f32.to_bits(),
                    base_bits: 0.0_f32.to_bits(),
                    factor_bits: 1.0_f32.to_bits(),
                },
                bypassed_by: Some(DamageTypeHolderSetSummary {
                    registry_ids: vec![3],
                    tag: None,
                }),
                block_sound: Some(SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("minecraft:item.shield.block".to_string()),
                    fixed_range_bits: None,
                }),
                disable_sound: None,
            }),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(37, &patch, None),
            Some(
                "{block_delay_seconds: 0.25f, disable_cooldown_scale: 1.5f, \
                 damage_reductions: [{type: \"#minecraft:bypasses_shield\", \
                 base: 1.0f, factor: 0.5f}], bypassed_by: \"minecraft:on_fire\", \
                 block_sound: {sound_id: \"minecraft:item.shield.block\"}}"
                    .to_string()
            )
        );

        patch
            .blocks_attacks
            .as_mut()
            .unwrap()
            .bypassed_by
            .as_mut()
            .unwrap()
            .registry_ids = vec![0];
        let mut world = world_with_registries(&[("minecraft:damage_type", &["example:custom"])]);
        world.apply_update_tags(bbb_protocol::packets::UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:damage_type".to_string(),
                tags: vec![bbb_protocol::packets::TagNetworkPayload {
                    tag: "minecraft:bypasses_shield".to_string(),
                    entries: vec![0],
                }],
            }],
        });
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            Some(
                "{block_delay_seconds: 0.25f, disable_cooldown_scale: 1.5f, \
                 damage_reductions: [{type: \"#minecraft:bypasses_shield\", \
                 base: 1.0f, factor: 0.5f}], bypassed_by: \"example:custom\", \
                 block_sound: {sound_id: \"minecraft:item.shield.block\"}}"
                    .to_string()
            )
        );

        patch.blocks_attacks = Some(BlocksAttacksSummary {
            block_delay_seconds_bits: 0.0_f32.to_bits(),
            disable_cooldown_scale_bits: 1.0_f32.to_bits(),
            damage_reductions: vec![DamageReductionSummary {
                horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
                damage_types: None,
                base_bits: 0.0_f32.to_bits(),
                factor_bits: 1.0_f32.to_bits(),
            }],
            item_damage: ItemDamageFunctionSummary {
                threshold_bits: 1.0_f32.to_bits(),
                base_bits: 0.0_f32.to_bits(),
                factor_bits: 1.0_f32.to_bits(),
            },
            bypassed_by: None,
            block_sound: None,
            disable_sound: None,
        });
        assert_eq!(
            encode_added_component_value_snbt(37, &patch, None),
            Some("{}".to_string())
        );

        patch.blocks_attacks.as_mut().unwrap().damage_reductions[0]
            .horizontal_blocking_angle_bits = 0.0_f32.to_bits();
        assert_eq!(encode_added_component_value_snbt(37, &patch, None), None);
        patch.blocks_attacks.as_mut().unwrap().damage_reductions[0]
            .horizontal_blocking_angle_bits = 90.0_f32.to_bits();
        patch
            .blocks_attacks
            .as_mut()
            .unwrap()
            .item_damage
            .threshold_bits = f32::INFINITY.to_bits();
        assert_eq!(encode_added_component_value_snbt(37, &patch, None), None);
    }

    #[test]
    fn damage_type_encoders_use_session_order_or_fixed_catalog() {
        assert_eq!(
            bbb_protocol::damage_types::vanilla_damage_type_resource_id(49),
            Some("minecraft:mace_smash")
        );
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            damage_type_id: Some(49),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(8, &patch, None),
            Some("\"minecraft:mace_smash\"".to_string())
        );

        let world = world_with_registries(&[(
            "minecraft:damage_type",
            &["example:second", "example:first"],
        )]);
        patch.damage_type_id = Some(1);
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            Some("\"example:first\"".to_string())
        );

        patch.damage_type_id = None;
        patch.damage_resistant_type_registry_ids = vec![0, 1];
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            Some("{types: [\"example:second\", \"example:first\"]}".to_string())
        );

        patch.damage_resistant_type_registry_ids.clear();
        patch.damage_type_id = Some(50);
        assert_eq!(encode_added_component_value_snbt(8, &patch, None), None);
        patch.damage_type_id = Some(-1);
        assert_eq!(encode_added_component_value_snbt(8, &patch, None), None);
    }

    #[test]
    fn fixed_network_holder_set_encoders_follow_tag_revisions_and_reject_missing_tags() {
        use bbb_protocol::packets::{ConsumeEffectSummary, DataComponentPatchSummary};

        let repairable = DataComponentPatchSummary {
            repairable_item_tag: Some("example:repairs".to_string()),
            ..Default::default()
        };
        let death_protection = DataComponentPatchSummary {
            death_protection_effects: vec![ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids: Vec::new(),
                effect_tag: Some("example:curable".to_string()),
            }],
            ..Default::default()
        };
        let mut world = WorldStore::new();
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
                &repairable,
                registry_context(&world),
            ),
            None
        );
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
                &death_protection,
                registry_context(&world),
            ),
            None
        );

        world.apply_update_tags(bbb_protocol::packets::UpdateTags {
            registries: vec![
                bbb_protocol::packets::RegistryTags {
                    registry: "minecraft:item".to_string(),
                    tags: vec![bbb_protocol::packets::TagNetworkPayload {
                        tag: "example:repairs".to_string(),
                        entries: vec![42],
                    }],
                },
                bbb_protocol::packets::RegistryTags {
                    registry: "minecraft:mob_effect".to_string(),
                    tags: vec![bbb_protocol::packets::TagNetworkPayload {
                        tag: "example:curable".to_string(),
                        entries: vec![0],
                    }],
                },
            ],
        });
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
                &repairable,
                registry_context(&world),
            ),
            Some("{items: \"#example:repairs\"}".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
                &death_protection,
                registry_context(&world),
            ),
            Some(
                "{death_effects: [{type: \"minecraft:remove_effects\", effects: \"#example:curable\"}]}"
                    .to_string()
            )
        );

        world.apply_update_tags(bbb_protocol::packets::UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: Vec::new(),
            }],
        });
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
                &repairable,
                registry_context(&world),
            ),
            None
        );
    }

    #[test]
    fn component_tree_encoders_preserve_exact_nbt_and_omit_missing_trees() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            custom_name_nbt: Some(bbb_protocol::packets::NbtSummaryValue::Compound(vec![
                bbb_protocol::packets::NbtSummaryEntry {
                    name: "translate".to_string(),
                    value: bbb_protocol::packets::NbtSummaryValue::String(
                        "item.example.name".to_string(),
                    ),
                },
                bbb_protocol::packets::NbtSummaryEntry {
                    name: "bold".to_string(),
                    value: bbb_protocol::packets::NbtSummaryValue::Byte(1),
                },
            ])),
            item_name_nbt: Some(bbb_protocol::packets::NbtSummaryValue::String(
                "Exact item name".to_string(),
            )),
            lore: vec!["First".to_string(), "Second".to_string()],
            lore_nbt: vec![
                bbb_protocol::packets::NbtSummaryValue::String("First".to_string()),
                bbb_protocol::packets::NbtSummaryValue::Compound(vec![
                    bbb_protocol::packets::NbtSummaryEntry {
                        name: "text".to_string(),
                        value: bbb_protocol::packets::NbtSummaryValue::String("Second".to_string()),
                    },
                    bbb_protocol::packets::NbtSummaryEntry {
                        name: "italic".to_string(),
                        value: bbb_protocol::packets::NbtSummaryValue::Byte(0),
                    },
                ]),
            ],
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(6, &patch, None),
            Some("{translate: \"item.example.name\", bold: 1b}".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(9, &patch, None),
            Some("\"Exact item name\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(11, &patch, None),
            Some("[\"First\", {text: \"Second\", italic: 0b}]".to_string())
        );

        patch.custom_name_nbt = None;
        assert_eq!(encode_added_component_value_snbt(6, &patch, None), None);
        patch.lore_nbt.pop();
        assert_eq!(encode_added_component_value_snbt(11, &patch, None), None);
    }

    #[test]
    fn written_book_encoder_preserves_filterable_component_trees_and_codec_defaults() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            written_book: Some(bbb_protocol::packets::WrittenBookContentSummary {
                title: "Guide".to_string(),
                title_filter: Some("Filtered Guide".to_string()),
                author: "Alex".to_string(),
                generation: 2,
                pages: vec!["Chapter".to_string()],
                page_filters: vec![Some("Filtered Chapter".to_string())],
                resolved: true,
            }),
            written_book_pages_nbt: vec![Some(bbb_protocol::packets::NbtSummaryValue::Compound(
                vec![bbb_protocol::packets::NbtSummaryEntry {
                    name: "translate".to_string(),
                    value: bbb_protocol::packets::NbtSummaryValue::String(
                        "book.example.chapter".to_string(),
                    ),
                }],
            ))],
            written_book_page_filters_nbt: vec![Some(
                bbb_protocol::packets::NbtSummaryValue::String("Filtered Chapter".to_string()),
            )],
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(55, &patch, None),
            Some(
                "{title: {raw: \"Guide\", filtered: \"Filtered Guide\"}, \
                 author: \"Alex\", generation: 2, pages: [{raw: \
                 {translate: \"book.example.chapter\"}, filtered: \
                 \"Filtered Chapter\"}], resolved: 1b}"
                    .to_string()
            )
        );

        let default_patch = bbb_protocol::packets::DataComponentPatchSummary {
            written_book: Some(bbb_protocol::packets::WrittenBookContentSummary {
                title: "".to_string(),
                title_filter: None,
                author: "".to_string(),
                generation: 0,
                pages: Vec::new(),
                page_filters: Vec::new(),
                resolved: false,
            }),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(55, &default_patch, None),
            Some("{title: {raw: \"\"}, author: \"\"}".to_string())
        );

        patch.written_book.as_mut().unwrap().generation = 4;
        assert_eq!(encode_added_component_value_snbt(55, &patch, None), None);
        patch.written_book.as_mut().unwrap().generation = 0;
        patch.written_book_pages_nbt[0] = None;
        assert_eq!(encode_added_component_value_snbt(55, &patch, None), None);
    }

    #[test]
    fn suspicious_stew_effects_use_codec_defaults_and_omit_unknown_holders() {
        let unknown_effect_id = i32::try_from(VANILLA_MOB_EFFECT_TYPES.len()).unwrap();
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            suspicious_stew_effects: vec![
                bbb_protocol::packets::SuspiciousStewEffectSummary {
                    effect_id: VANILLA_MOB_EFFECT_SPEED_ID,
                    duration: 160,
                },
                bbb_protocol::packets::SuspiciousStewEffectSummary {
                    effect_id: VANILLA_MOB_EFFECT_POISON_ID,
                    duration: 200,
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(53, &patch, None),
            Some(
                "[{id: \"minecraft:speed\"}, {id: \"minecraft:poison\", duration: 200}]"
                    .to_string()
            )
        );

        patch.suspicious_stew_effects[1].effect_id = unknown_effect_id;
        assert_eq!(encode_added_component_value_snbt(53, &patch, None), None);

        patch.suspicious_stew_effects.clear();
        assert_eq!(
            encode_added_component_value_snbt(53, &patch, None),
            Some("[]".to_string())
        );
    }

    #[test]
    fn enchantment_components_share_map_codec_and_validate_persistent_values() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                // `Enchantments.bootstrap`: DENSITY is holder id 38 in 26.1.
                holder_id: 38,
                level: 4,
            }],
            stored_enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: 13,
                level: 5,
            }],
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(13, &patch, None),
            Some("{\"minecraft:density\": 4}".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(42, &patch, None),
            Some("{\"minecraft:sharpness\": 5}".to_string())
        );

        let world = world_with_registries(&[("minecraft:enchantment", &["example:custom"])]);
        patch.enchantments[0].holder_id = 0;
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            Some("{\"example:custom\": 4}".to_string())
        );
        patch.enchantments[0].holder_id = 1;
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            None
        );

        patch.enchantments.clear();
        assert_eq!(
            encode_added_component_value_snbt(13, &patch, None),
            Some("{}".to_string())
        );

        patch.stored_enchantments[0].holder_id = 43;
        assert_eq!(encode_added_component_value_snbt(42, &patch, None), None);
        patch.stored_enchantments[0].holder_id = 13;
        patch.stored_enchantments[0].level = 0;
        assert_eq!(encode_added_component_value_snbt(42, &patch, None), None);
        patch.stored_enchantments[0].level = 256;
        assert_eq!(encode_added_component_value_snbt(42, &patch, None), None);
    }

    #[test]
    fn attribute_modifier_encoder_preserves_order_defaults_and_hidden_display() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            attribute_modifiers: vec![
                bbb_protocol::packets::AttributeModifierSummary {
                    attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                    modifier_id: "minecraft:base_attack_damage".to_string(),
                    amount_bits: 3.5_f64.to_bits(),
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                    slot_id: 0,
                    display_id: 0,
                    display_text: None,
                    display_text_styled: None,
                },
                bbb_protocol::packets::AttributeModifierSummary {
                    attribute_id: bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
                    modifier_id: "minecraft:test/hidden".to_string(),
                    amount_bits: (-0.25_f64).to_bits(),
                    operation_id: bbb_protocol::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
                    slot_id: 1,
                    display_id: 1,
                    display_text: None,
                    display_text_styled: None,
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(16, &patch, None),
            Some(
                "[{type: \"minecraft:attack_damage\", id: \"minecraft:base_attack_damage\", \
                 amount: 3.5d, operation: \"add_value\"}, \
                 {type: \"minecraft:knockback_resistance\", id: \"minecraft:test/hidden\", \
                 amount: -0.25d, operation: \"add_multiplied_total\", slot: \"mainhand\", \
                 display: {type: \"hidden\"}}]"
                    .to_string()
            )
        );

        patch.attribute_modifiers[0].attribute_id =
            i32::try_from(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_TYPES.len()).unwrap();
        assert_eq!(encode_added_component_value_snbt(16, &patch, None), None);
        patch.attribute_modifiers[0].attribute_id =
            bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID;
        patch.attribute_modifiers[1].display_id = 2;
        patch.attribute_modifiers[1].display_text = Some("Override".to_string());
        assert_eq!(encode_added_component_value_snbt(16, &patch, None), None);

        patch.attribute_modifiers.clear();
        assert_eq!(
            encode_added_component_value_snbt(16, &patch, None),
            Some("[]".to_string())
        );
    }

    #[test]
    fn banner_pattern_encoders_use_session_order_or_fixed_catalog() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            banner_pattern_layers: vec![
                bbb_protocol::packets::BannerPatternLayerSummary {
                    registry_id: Some(5),
                    asset_id: None,
                    translation_key: None,
                    color_id: VANILLA_DYE_COLOR_RED_ID,
                },
                bbb_protocol::packets::BannerPatternLayerSummary {
                    registry_id: None,
                    asset_id: Some("example:custom_banner/pattern".to_string()),
                    translation_key: Some("block.example.banner.custom".to_string()),
                    color_id: VANILLA_DYE_COLOR_LIGHT_BLUE_ID,
                },
            ],
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(72, &patch, None),
            Some(
                "[{pattern: \"minecraft:stripe_bottom\", color: \"red\"}, \
                 {pattern: {asset_id: \"example:custom_banner/pattern\", \
                 translation_key: \"block.example.banner.custom\"}, color: \"light_blue\"}]"
                    .to_string()
            )
        );

        let world = world_with_registries(&[("minecraft:banner_pattern", &["example:custom"])]);
        patch.banner_pattern_layers[0].registry_id = Some(0);
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            Some(
                "[{pattern: \"example:custom\", color: \"red\"}, \
                 {pattern: {asset_id: \"example:custom_banner/pattern\", \
                 translation_key: \"block.example.banner.custom\"}, color: \"light_blue\"}]"
                    .to_string()
            )
        );

        patch.banner_pattern_layers[0].registry_id = Some(1);
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            None
        );

        patch.banner_pattern_layers[0].registry_id = Some(43);
        assert_eq!(encode_added_component_value_snbt(72, &patch, None), None);
        patch.banner_pattern_layers.remove(0);
        patch.banner_pattern_layers[0].asset_id = None;
        assert_eq!(encode_added_component_value_snbt(72, &patch, None), None);
        patch.banner_pattern_layers[0].asset_id = Some("Invalid Asset".to_string());
        assert_eq!(encode_added_component_value_snbt(72, &patch, None), None);
        patch.banner_pattern_layers.clear();
        assert_eq!(
            encode_added_component_value_snbt(72, &patch, None),
            Some("[]".to_string())
        );

        patch.provides_banner_pattern_registry_ids = vec![0, 42];
        assert_eq!(
            encode_added_component_value_snbt(
                component_id::VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
                &patch,
                None,
            ),
            Some("[\"minecraft:base\", \"minecraft:guster\"]".to_string())
        );
        patch.provides_banner_pattern_registry_ids = vec![0];
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            Some("\"example:custom\"".to_string())
        );
        patch.provides_banner_pattern_tag = Some("example:patterns".to_string());
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            None
        );
        patch.provides_banner_pattern_registry_ids.clear();
        assert_eq!(
            encode_added_component_value_snbt(
                component_id::VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
                &patch,
                None,
            ),
            Some("\"#example:patterns\"".to_string())
        );
    }

    #[test]
    fn instrument_encoder_uses_session_order_or_fixed_catalog() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            instrument_id: Some(5),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(61, &patch, None),
            Some("\"minecraft:call_goat_horn\"".to_string())
        );

        let world = world_with_registries(&[("minecraft:instrument", &["example:custom"])]);
        patch.instrument_id = Some(0);
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            Some("\"example:custom\"".to_string())
        );

        patch.instrument_id = Some(1);
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            None
        );

        patch.instrument_id = Some(8);
        assert_eq!(encode_added_component_value_snbt(61, &patch, None), None);
        patch.instrument_id = None;
        patch.instrument_description = Some("Custom horn".to_string());
        assert_eq!(encode_added_component_value_snbt(61, &patch, None), None);

        patch.instrument_direct = Some(Box::new(bbb_protocol::packets::InstrumentSummary {
            sound_event: bbb_protocol::packets::SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:signal".to_string()),
                fixed_range_bits: Some(24.0f32.to_bits()),
            },
            use_duration_bits: 3.25f32.to_bits(),
            range_bits: 96.0f32.to_bits(),
            description: "Custom horn".to_string(),
            description_styled: None,
            description_nbt: Some(Box::new(bbb_protocol::packets::NbtSummaryValue::String(
                "Custom horn".to_string(),
            ))),
        }));
        assert_eq!(
            encode_added_component_value_snbt(61, &patch, None),
            Some(
                "{sound_event: {sound_id: \"example:signal\", range: 24.0f}, \
                 use_duration: 3.25f, range: 96.0f, description: \"Custom horn\"}"
                    .to_string()
            )
        );
        patch.instrument_direct.as_mut().unwrap().description_nbt = None;
        assert_eq!(encode_added_component_value_snbt(61, &patch, None), None);
        patch.instrument_direct.as_mut().unwrap().description_nbt = Some(Box::new(
            bbb_protocol::packets::NbtSummaryValue::String("Custom horn".to_string()),
        ));
        patch.instrument_direct.as_mut().unwrap().range_bits = 0.0f32.to_bits();
        assert_eq!(encode_added_component_value_snbt(61, &patch, None), None);
        patch.instrument_direct.as_mut().unwrap().range_bits = 96.0f32.to_bits();

        patch.codec_backed_nbt.insert(
            component_id::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
            bbb_protocol::packets::NbtSummaryValue::String("example:persistent".to_string()),
        );
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
                &patch,
                registry_context(&world),
            ),
            Some("\"example:persistent\"".to_string())
        );
    }

    #[test]
    fn provides_trim_material_encoder_supports_named_and_exact_direct_holders() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            provides_trim_material_id: Some(10),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(62, &patch, None),
            Some("\"minecraft:resin\"".to_string())
        );

        patch.provides_trim_material_id = Some(11);
        assert_eq!(encode_added_component_value_snbt(62, &patch, None), None);
        patch.provides_trim_material_id = None;
        patch.provides_trim_material_direct =
            Some(Box::new(bbb_protocol::packets::TrimMaterialSummary {
                asset_name: "custom".to_string(),
                override_armor_assets: BTreeMap::from([(
                    "minecraft:iron".to_string(),
                    "custom_darker".to_string(),
                )]),
                description: "Custom material".to_string(),
                description_styled: None,
                description_nbt: None,
            }));
        assert_eq!(encode_added_component_value_snbt(62, &patch, None), None);
        patch
            .provides_trim_material_direct
            .as_mut()
            .unwrap()
            .description_nbt = Some(Box::new(bbb_protocol::packets::NbtSummaryValue::String(
            "Custom material".to_string(),
        )));
        assert_eq!(
            encode_added_component_value_snbt(62, &patch, None),
            Some(
                "{asset_name: \"custom\", override_armor_assets: {\"minecraft:iron\": \"custom_darker\"}, description: \"Custom material\"}"
                    .to_string()
            )
        );
    }

    #[test]
    fn armor_trim_encoder_supports_named_direct_and_mixed_holders() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            armor_trim_material_id: Some(1),
            armor_trim_pattern_id: Some(17),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(56, &patch, None),
            Some("{material: \"minecraft:iron\", pattern: \"minecraft:bolt\"}".to_string())
        );

        patch.armor_trim_material_id = Some(11);
        assert_eq!(encode_added_component_value_snbt(56, &patch, None), None);
        patch.armor_trim_material_id = Some(1);
        patch.armor_trim_pattern_id = Some(18);
        assert_eq!(encode_added_component_value_snbt(56, &patch, None), None);
        patch.armor_trim_pattern_id = Some(17);
        patch.armor_trim_material_direct =
            Some(Box::new(bbb_protocol::packets::TrimMaterialSummary {
                asset_name: "custom".to_string(),
                override_armor_assets: Default::default(),
                description: "Custom material".to_string(),
                description_styled: None,
                description_nbt: None,
            }));
        assert_eq!(encode_added_component_value_snbt(56, &patch, None), None);
        patch
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .description_nbt = Some(Box::new(bbb_protocol::packets::NbtSummaryValue::String(
            "Custom material".to_string(),
        )));
        assert_eq!(
            encode_added_component_value_snbt(56, &patch, None),
            Some(
                "{material: {asset_name: \"custom\", description: \"Custom material\"}, pattern: \"minecraft:bolt\"}"
                    .to_string()
            )
        );
        patch.armor_trim_material_direct = None;
        patch.armor_trim_pattern_direct =
            Some(Box::new(bbb_protocol::packets::TrimPatternSummary {
                asset_id: "example:custom".to_string(),
                description: "Custom pattern".to_string(),
                description_styled: None,
                description_nbt: None,
                decal: false,
            }));
        assert_eq!(encode_added_component_value_snbt(56, &patch, None), None);
        patch
            .armor_trim_pattern_direct
            .as_mut()
            .unwrap()
            .description_nbt = Some(Box::new(bbb_protocol::packets::NbtSummaryValue::String(
            "Custom pattern".to_string(),
        )));
        assert_eq!(
            encode_added_component_value_snbt(56, &patch, None),
            Some(
                "{material: \"minecraft:iron\", pattern: {asset_id: \"example:custom\", description: \"Custom pattern\", decal: 0b}}"
                    .to_string()
            )
        );
    }

    #[test]
    fn trim_encoders_use_synchronized_holder_order_when_available() {
        let world = world_with_registries(&[
            ("minecraft:trim_material", &["example:custom_material"]),
            ("minecraft:trim_pattern", &["example:custom_pattern"]),
        ]);
        let context = registry_context(&world);
        let patch = bbb_protocol::packets::DataComponentPatchSummary {
            provides_trim_material_id: Some(0),
            armor_trim_material_id: Some(0),
            armor_trim_pattern_id: Some(0),
            ..Default::default()
        };

        assert_eq!(
            encode_added_component_value_snbt_with_context(62, &patch, context),
            Some("\"example:custom_material\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt_with_context(56, &patch, context),
            Some(
                "{material: \"example:custom_material\", pattern: \"example:custom_pattern\"}"
                    .to_string()
            )
        );
    }

    #[test]
    fn persistent_holder_encoders_preserve_validated_dialog_values() {
        for (type_id, persistent) in [
            (
                61,
                bbb_protocol::packets::NbtSummaryValue::Compound(vec![
                    bbb_protocol::packets::NbtSummaryEntry {
                        name: "sound_event".to_string(),
                        value: bbb_protocol::packets::NbtSummaryValue::String(
                            "example:horn".to_string(),
                        ),
                    },
                    bbb_protocol::packets::NbtSummaryEntry {
                        name: "use_duration".to_string(),
                        value: bbb_protocol::packets::NbtSummaryValue::Float(1.0f32.to_bits()),
                    },
                    bbb_protocol::packets::NbtSummaryEntry {
                        name: "range".to_string(),
                        value: bbb_protocol::packets::NbtSummaryValue::Float(16.0f32.to_bits()),
                    },
                    bbb_protocol::packets::NbtSummaryEntry {
                        name: "description".to_string(),
                        value: bbb_protocol::packets::NbtSummaryValue::String(
                            "Custom horn".to_string(),
                        ),
                    },
                ]),
            ),
            (
                62,
                bbb_protocol::packets::NbtSummaryValue::Compound(vec![
                    bbb_protocol::packets::NbtSummaryEntry {
                        name: "asset_name".to_string(),
                        value: bbb_protocol::packets::NbtSummaryValue::String("custom".to_string()),
                    },
                    bbb_protocol::packets::NbtSummaryEntry {
                        name: "description".to_string(),
                        value: bbb_protocol::packets::NbtSummaryValue::String(
                            "Custom material".to_string(),
                        ),
                    },
                ]),
            ),
            (
                56,
                bbb_protocol::packets::NbtSummaryValue::Compound(vec![
                    bbb_protocol::packets::NbtSummaryEntry {
                        name: "material".to_string(),
                        value: bbb_protocol::packets::NbtSummaryValue::String(
                            "example:material".to_string(),
                        ),
                    },
                    bbb_protocol::packets::NbtSummaryEntry {
                        name: "pattern".to_string(),
                        value: bbb_protocol::packets::NbtSummaryValue::String(
                            "example:pattern".to_string(),
                        ),
                    },
                ]),
            ),
        ] {
            let mut patch = bbb_protocol::packets::DataComponentPatchSummary::default();
            patch.codec_backed_nbt.insert(type_id, persistent.clone());
            assert_eq!(
                encode_added_component_value_snbt(type_id, &patch, None),
                Some(debug_nbt_summary_snbt(&persistent))
            );
        }
    }

    #[test]
    fn jukebox_playable_encoder_uses_session_order_or_resource_fallback() {
        let item_runtime = NativeItemRuntime::empty_for_test();
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            jukebox_song_id: Some(1),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(64, &patch, Some(&item_runtime)),
            Some("\"minecraft:cat\"".to_string())
        );
        assert_eq!(encode_added_component_value_snbt(64, &patch, None), None);

        let world = world_with_registries(&[("minecraft:jukebox_song", &["example:custom"])]);
        patch.jukebox_song_id = Some(0);
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
                &patch,
                PersistentSnbtContext {
                    item_runtime: Some(&item_runtime),
                    registry_access: Some(world.frozen_registry_access()),
                    ..Default::default()
                },
            ),
            Some("\"example:custom\"".to_string())
        );

        patch.jukebox_song_id = Some(1);
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                component_id::VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
                &patch,
                PersistentSnbtContext {
                    item_runtime: Some(&item_runtime),
                    registry_access: Some(world.frozen_registry_access()),
                    ..Default::default()
                },
            ),
            None
        );

        patch.jukebox_song_id = Some(21);
        assert_eq!(
            encode_added_component_value_snbt(64, &patch, Some(&item_runtime)),
            None
        );

        patch.jukebox_song_id = None;
        patch.jukebox_direct_song = Some(Box::new(bbb_protocol::packets::JukeboxSongSummary {
            sound_event: bbb_protocol::packets::SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:music.custom".to_string()),
                fixed_range_bits: None,
            },
            description: "Custom song".to_string(),
            description_styled: None,
            description_nbt: None,
            length_in_seconds_bits: 60.0_f32.to_bits(),
            comparator_output: 7,
        }));
        assert_eq!(
            encode_added_component_value_snbt(64, &patch, Some(&item_runtime)),
            None
        );
    }

    #[test]
    fn painting_variant_encoder_uses_session_order_or_fixed_baseline() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            painting_variant_id: Some(50),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(102, &patch, None),
            Some("\"minecraft:dennis\"".to_string())
        );

        let world = world_with_registries(&[("minecraft:painting_variant", &["example:custom"])]);
        patch.painting_variant_id = Some(0);
        assert_eq!(
            encode_added_component_value_snbt_with_context(102, &patch, registry_context(&world)),
            Some("\"example:custom\"".to_string())
        );

        patch.painting_variant_id = Some(51);
        assert_eq!(encode_added_component_value_snbt(102, &patch, None), None);
        patch.painting_variant_id = None;
        patch.painting_variant_direct = Some(bbb_protocol::packets::PaintingVariantSummary {
            width: 1,
            height: 1,
            asset_id: "example:custom".to_string(),
            title: Some("Custom".to_string()),
            title_styled: None,
            author: None,
            author_styled: None,
        });
        assert_eq!(encode_added_component_value_snbt(102, &patch, None), None);
    }

    #[test]
    fn named_variant_encoders_use_session_order_or_fixed_catalog() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            villager_variant_id: Some(2),
            wolf_variant_id: Some(8),
            wolf_sound_variant_id: Some(6),
            frog_variant_id: Some(2),
            zombie_nautilus_variant_id: Some(1),
            pig_variant_id: Some(2),
            pig_sound_variant_id: Some(1),
            cow_variant_id: Some(2),
            cow_sound_variant_id: Some(1),
            chicken_variant_id: Some(2),
            chicken_sound_variant_id: Some(1),
            cat_variant_id: Some(10),
            cat_sound_variant_id: Some(1),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(81, &patch, None),
            Some("\"minecraft:plains\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(82, &patch, None),
            Some("\"minecraft:striped\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(83, &patch, None),
            Some("\"minecraft:cute\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(93, &patch, None),
            Some("\"minecraft:cold\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(94, &patch, None),
            Some("\"minecraft:big\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(95, &patch, None),
            Some("\"minecraft:cold\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(96, &patch, None),
            Some("\"minecraft:moody\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(97, &patch, None),
            Some("\"minecraft:cold\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(98, &patch, None),
            Some("\"minecraft:picky\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(99, &patch, None),
            Some("\"minecraft:warm\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(100, &patch, None),
            Some("\"minecraft:cold\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(105, &patch, None),
            Some("\"minecraft:all_black\"".to_string())
        );
        assert_eq!(
            encode_added_component_value_snbt(106, &patch, None),
            Some("\"minecraft:royal\"".to_string())
        );

        patch.villager_variant_id = Some(7);
        assert_eq!(encode_added_component_value_snbt(81, &patch, None), None);
        patch.villager_variant_id = Some(2);
        patch.wolf_variant_id = Some(9);
        assert_eq!(encode_added_component_value_snbt(82, &patch, None), None);
        patch.wolf_variant_id = Some(8);
        patch.wolf_sound_variant_id = Some(7);
        assert_eq!(encode_added_component_value_snbt(83, &patch, None), None);
        patch.wolf_sound_variant_id = Some(6);
        patch.pig_variant_id = Some(3);
        assert_eq!(encode_added_component_value_snbt(93, &patch, None), None);
        patch.pig_variant_id = Some(2);
        patch.pig_sound_variant_id = Some(3);
        assert_eq!(encode_added_component_value_snbt(94, &patch, None), None);
        patch.pig_sound_variant_id = Some(1);
        patch.cow_variant_id = Some(3);
        assert_eq!(encode_added_component_value_snbt(95, &patch, None), None);
        patch.cow_variant_id = Some(2);
        patch.cow_sound_variant_id = Some(2);
        assert_eq!(encode_added_component_value_snbt(96, &patch, None), None);
        patch.cow_sound_variant_id = Some(1);
        patch.chicken_variant_id = Some(3);
        assert_eq!(encode_added_component_value_snbt(97, &patch, None), None);
        patch.chicken_variant_id = Some(2);
        patch.chicken_sound_variant_id = Some(2);
        assert_eq!(encode_added_component_value_snbt(98, &patch, None), None);
        patch.chicken_sound_variant_id = Some(1);
        patch.zombie_nautilus_variant_id = Some(2);
        assert_eq!(encode_added_component_value_snbt(99, &patch, None), None);
        patch.zombie_nautilus_variant_id = Some(1);
        patch.frog_variant_id = Some(3);
        assert_eq!(encode_added_component_value_snbt(100, &patch, None), None);
        patch.cat_variant_id = Some(11);
        assert_eq!(encode_added_component_value_snbt(105, &patch, None), None);
        patch.cat_variant_id = Some(10);
        patch.cat_sound_variant_id = Some(2);
        assert_eq!(encode_added_component_value_snbt(106, &patch, None), None);

        patch.wolf_variant_id = Some(0);
        patch.wolf_sound_variant_id = Some(0);
        patch.pig_variant_id = Some(0);
        patch.pig_sound_variant_id = Some(0);
        patch.cow_variant_id = Some(0);
        patch.cow_sound_variant_id = Some(0);
        patch.chicken_variant_id = Some(0);
        patch.chicken_sound_variant_id = Some(0);
        patch.zombie_nautilus_variant_id = Some(0);
        patch.frog_variant_id = Some(0);
        patch.cat_variant_id = Some(0);
        patch.cat_sound_variant_id = Some(0);
        let world = world_with_registries(&[
            ("minecraft:wolf_variant", &["example:wolf"]),
            ("minecraft:wolf_sound_variant", &["example:wolf_sound"]),
            ("minecraft:pig_variant", &["example:pig"]),
            ("minecraft:pig_sound_variant", &["example:pig_sound"]),
            ("minecraft:cow_variant", &["example:cow"]),
            ("minecraft:cow_sound_variant", &["example:cow_sound"]),
            ("minecraft:chicken_variant", &["example:chicken"]),
            (
                "minecraft:chicken_sound_variant",
                &["example:chicken_sound"],
            ),
            (
                "minecraft:zombie_nautilus_variant",
                &["example:zombie_nautilus"],
            ),
            ("minecraft:frog_variant", &["example:frog"]),
            ("minecraft:cat_variant", &["example:cat"]),
            ("minecraft:cat_sound_variant", &["example:cat_sound"]),
        ]);
        let context = registry_context(&world);
        for (type_id, expected) in [
            (82, "\"example:wolf\""),
            (83, "\"example:wolf_sound\""),
            (93, "\"example:pig\""),
            (94, "\"example:pig_sound\""),
            (95, "\"example:cow\""),
            (96, "\"example:cow_sound\""),
            (97, "\"example:chicken\""),
            (98, "\"example:chicken_sound\""),
            (99, "\"example:zombie_nautilus\""),
            (100, "\"example:frog\""),
            (105, "\"example:cat\""),
            (106, "\"example:cat_sound\""),
        ] {
            assert_eq!(
                encode_added_component_value_snbt_with_context(type_id, &patch, context),
                Some(expected.to_string())
            );
        }
        assert_eq!(
            encode_added_component_value_snbt_with_context(81, &patch, context),
            Some("\"minecraft:plains\"".to_string())
        );

        patch.wolf_variant_id = Some(1);
        assert_eq!(
            encode_added_component_value_snbt_with_context(82, &patch, context),
            None
        );

        let only_wolf_variant =
            world_with_registries(&[("minecraft:wolf_variant", &["example:wolf"])]);
        patch.pig_variant_id = Some(2);
        assert_eq!(
            encode_added_component_value_snbt_with_context(
                93,
                &patch,
                registry_context(&only_wolf_variant),
            ),
            Some("\"minecraft:cold\"".to_string())
        );
    }
}
