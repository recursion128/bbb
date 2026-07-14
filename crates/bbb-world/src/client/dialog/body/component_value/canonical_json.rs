use bbb_protocol::item_component_value_types::ItemUseAnimationSummary;
use bbb_protocol::packets::inventory::FireworkExplosionSummary;
use bbb_protocol::{
    attribute_modifier_types::{
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
    },
    canonical_component_json_utf16_len, canonical_f32_json_utf16_len, canonical_f64_json_utf16_len,
    canonical_json_string_utf16_len, canonical_nbt_json_utf16_len,
    canonical_resolvable_profile_json_utf16_len,
    equipment_slots::VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
    NbtSummaryEntry, NbtSummaryValue,
};

use super::super::{DialogDataComponentPatchState, DialogItemStackTemplateState};
use super::attribute_modifiers::{
    DialogAttributeModifierDisplayState, DialogAttributeModifierState,
};
use super::blocks_attacks::{
    DialogBlocksAttacksDamageReductionState, DialogBlocksAttacksItemDamageState,
    DialogBlocksAttacksState,
};
use super::consume_effect::{DialogConsumableState, DialogConsumeEffectState};
use super::holder_set::{DialogBannerPatternHolderState, DialogRegistryHolderSetState};
use super::instrument::{DialogInstrumentHolderState, DialogInstrumentState};
use super::potion_contents::{
    DialogMobEffectDetailsState, DialogMobEffectInstanceState, DialogPotionContentsState,
};
use super::sound_event::{
    DialogKineticWeaponConditionState, DialogKineticWeaponState, DialogPiercingWeaponState,
    DialogSoundEventHolderState,
};
use super::trim::{
    DialogTrimMaterialHolderState, DialogTrimMaterialState, DialogTrimPatternHolderState,
    DialogTrimPatternState,
};
use super::written_book::DialogWrittenBookContentState;
use super::DialogDataComponentValueState;

mod predicates;

/// Returns an exact JsonOps length when every patch value has a supported semantic encoder.
/// Complex holder and recursive values remain unknown rather than falling back to raw NBT size.
pub(in crate::client::dialog) fn canonical_data_component_patch_json_utf16_len(
    patch: &DialogDataComponentPatchState,
) -> Result<Option<usize>, String> {
    let mut length = 2usize;
    for (index, entry) in patch.entries.iter().enumerate() {
        let value_len = if entry.removed {
            // DataComponentPatch.PatchKey selects Codec.EMPTY.codec() for removals.
            2
        } else {
            let Some(value) = entry.value_data.as_ref() else {
                return Ok(None);
            };
            validate_supported_json_numbers(value)?;
            let Some(length) = canonical_component_value_json_utf16_len(value) else {
                return Ok(None);
            };
            length
        };
        let key_len = entry
            .resource_id
            .len()
            .saturating_add(2)
            .saturating_add(usize::from(entry.removed));
        length = length
            .saturating_add(usize::from(index != 0))
            .saturating_add(key_len)
            .saturating_add(1)
            .saturating_add(value_len);
    }
    Ok(Some(length))
}

fn validate_supported_json_numbers(value: &DialogDataComponentValueState) -> Result<(), String> {
    let mut values = Vec::new();
    let mut double_values = Vec::new();
    match value {
        DialogDataComponentValueState::TextComponent { component } => {
            canonical_component_json_utf16_len(&component.component)?;
        }
        DialogDataComponentValueState::Lore { lines } => {
            for line in lines {
                canonical_component_json_utf16_len(&line.component)?;
            }
        }
        DialogDataComponentValueState::WrittenBookContent { content } => {
            for page in &content.pages {
                canonical_component_json_utf16_len(&page.raw.component)?;
                if let Some(filtered) = &page.filtered {
                    canonical_component_json_utf16_len(&filtered.component)?;
                }
            }
        }
        DialogDataComponentValueState::AttributeModifiers { modifiers } => {
            for modifier in modifiers {
                double_values.push(modifier.amount_bits);
                if let Some(component) = &modifier.display.value {
                    canonical_component_json_utf16_len(&component.component)?;
                }
            }
        }
        DialogDataComponentValueState::AdventureModePredicate { predicate } => {
            predicates::validate_adventure_mode_predicate(predicate)?;
        }
        DialogDataComponentValueState::Lock { predicate } => {
            predicates::validate_item_predicate(predicate)?;
        }
        DialogDataComponentValueState::UseRemainder { convert_into } => {
            validate_item_stack_template_json(convert_into)?;
        }
        DialogDataComponentValueState::ChargedProjectiles { items }
        | DialogDataComponentValueState::BundleContents { items } => {
            for item in items {
                validate_item_stack_template_json(item)?;
            }
        }
        DialogDataComponentValueState::Container { slots } => {
            for item in slots.iter().flatten() {
                validate_item_stack_template_json(item)?;
            }
        }
        DialogDataComponentValueState::Instrument {
            instrument: DialogInstrumentHolderState::Direct { instrument },
        } => {
            values.extend([instrument.use_duration_bits, instrument.range_bits]);
            append_sound_range_bits(&mut values, Some(&instrument.sound_event));
            canonical_component_json_utf16_len(&instrument.description.component)?;
        }
        DialogDataComponentValueState::ProvidesTrimMaterial {
            material: DialogTrimMaterialHolderState::Direct { material },
        } => {
            canonical_component_json_utf16_len(&material.description.component)?;
        }
        DialogDataComponentValueState::Trim { material, pattern } => {
            if let DialogTrimMaterialHolderState::Direct { material } = material {
                canonical_component_json_utf16_len(&material.description.component)?;
            }
            if let DialogTrimPatternHolderState::Direct { pattern } = pattern {
                canonical_component_json_utf16_len(&pattern.description.component)?;
            }
        }
        DialogDataComponentValueState::Float { bits } => values.push(*bits),
        DialogDataComponentValueState::UseEffects {
            speed_multiplier_bits,
            ..
        } => values.push(*speed_multiplier_bits),
        DialogDataComponentValueState::Food {
            saturation_bits, ..
        } => values.push(*saturation_bits),
        DialogDataComponentValueState::UseCooldown { seconds_bits, .. } => {
            values.push(*seconds_bits);
        }
        DialogDataComponentValueState::Weapon {
            disable_blocking_for_seconds_bits,
            ..
        } => values.push(*disable_blocking_for_seconds_bits),
        DialogDataComponentValueState::AttackRange {
            min_reach_bits,
            max_reach_bits,
            min_creative_reach_bits,
            max_creative_reach_bits,
            hitbox_margin_bits,
            mob_factor_bits,
        } => values.extend([
            *min_reach_bits,
            *max_reach_bits,
            *min_creative_reach_bits,
            *max_creative_reach_bits,
            *hitbox_margin_bits,
            *mob_factor_bits,
        ]),
        DialogDataComponentValueState::CustomModelData { floats_bits, .. } => {
            values.extend(floats_bits);
        }
        DialogDataComponentValueState::MapDecorations { decorations } => {
            for decoration in decorations {
                double_values.extend([decoration.x_bits, decoration.z_bits]);
                values.push(decoration.rotation_bits);
            }
        }
        DialogDataComponentValueState::BreakSound {
            sound:
                DialogSoundEventHolderState::Direct {
                    fixed_range_bits: Some(bits),
                    ..
                },
        } => values.push(*bits),
        DialogDataComponentValueState::PiercingWeapon { weapon } => {
            append_sound_range_bits(&mut values, weapon.sound.as_ref());
            append_sound_range_bits(&mut values, weapon.hit_sound.as_ref());
        }
        DialogDataComponentValueState::KineticWeapon { weapon } => {
            values.extend([weapon.forward_movement_bits, weapon.damage_multiplier_bits]);
            for condition in [
                weapon.dismount_conditions.as_ref(),
                weapon.knockback_conditions.as_ref(),
                weapon.damage_conditions.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                values.extend([condition.min_speed_bits, condition.min_relative_speed_bits]);
            }
            append_sound_range_bits(&mut values, weapon.sound.as_ref());
            append_sound_range_bits(&mut values, weapon.hit_sound.as_ref());
        }
        DialogDataComponentValueState::Tool { tool } => {
            values.push(tool.default_mining_speed_bits);
            values.extend(tool.rules.iter().filter_map(|rule| rule.speed_bits));
        }
        DialogDataComponentValueState::Equippable { equippable } => {
            append_sound_range_bits(&mut values, Some(&equippable.equip_sound));
            append_sound_range_bits(&mut values, Some(&equippable.shearing_sound));
        }
        DialogDataComponentValueState::BlocksAttacks { blocks_attacks } => {
            values.extend([
                blocks_attacks.block_delay_seconds_bits,
                blocks_attacks.disable_cooldown_scale_bits,
            ]);
            for reduction in &blocks_attacks.damage_reductions {
                values.extend([
                    reduction.horizontal_blocking_angle_bits,
                    reduction.base_bits,
                    reduction.factor_bits,
                ]);
            }
            values.extend([
                blocks_attacks.item_damage.threshold_bits,
                blocks_attacks.item_damage.base_bits,
                blocks_attacks.item_damage.factor_bits,
            ]);
            append_sound_range_bits(&mut values, blocks_attacks.block_sound.as_ref());
            append_sound_range_bits(&mut values, blocks_attacks.disabled_sound.as_ref());
        }
        DialogDataComponentValueState::Consumable { consumable } => {
            values.push(consumable.consume_seconds_bits);
            append_sound_range_bits(&mut values, Some(&consumable.sound));
            append_consume_effect_float_bits(&mut values, &consumable.on_consume_effects);
        }
        DialogDataComponentValueState::DeathProtection { effects } => {
            append_consume_effect_float_bits(&mut values, effects);
        }
        DialogDataComponentValueState::Bees { occupants } => {
            for occupant in occupants {
                typed_data_json_utf16_len(
                    &occupant.entity_data,
                    &occupant.entity_type.resource_id,
                )?;
            }
        }
        DialogDataComponentValueState::EntityData { entity } => {
            typed_data_json_utf16_len(&entity.data, &entity.entity_type.resource_id)?;
        }
        DialogDataComponentValueState::BlockEntityData { block_entity } => {
            typed_data_json_utf16_len(
                &block_entity.data,
                &block_entity.block_entity_type.resource_id,
            )?;
        }
        DialogDataComponentValueState::BucketEntityData { data }
        | DialogDataComponentValueState::CustomData { data } => {
            canonical_nbt_json_utf16_len(data)?;
        }
        _ => {}
    }
    if values
        .into_iter()
        .any(|bits| canonical_f32_json_utf16_len(bits).is_none())
        || double_values
            .into_iter()
            .any(|bits| canonical_f64_json_utf16_len(bits).is_none())
    {
        return Err(
            "canonical item component patch contains a non-finite Float/Double value".to_string(),
        );
    }
    Ok(())
}

fn canonical_component_value_json_utf16_len(
    value: &DialogDataComponentValueState,
) -> Option<usize> {
    Some(match value {
        DialogDataComponentValueState::TextComponent { component } => {
            component_json_len(&component.component)?
        }
        DialogDataComponentValueState::Lore { lines } => array_len(
            lines
                .iter()
                .map(|line| component_json_len(&line.component))
                .collect::<Option<Vec<_>>>()?
                .into_iter(),
        ),
        DialogDataComponentValueState::WrittenBookContent { content } => {
            written_book_content_len(content)?
        }
        DialogDataComponentValueState::AttributeModifiers { modifiers } => {
            attribute_modifiers_len(modifiers)?
        }
        DialogDataComponentValueState::AdventureModePredicate { predicate } => {
            predicates::adventure_mode_predicate_len(predicate)?
        }
        DialogDataComponentValueState::Lock { predicate } => {
            predicates::item_predicate_len(predicate)?
        }
        DialogDataComponentValueState::UseRemainder { convert_into } => {
            item_stack_template_len(convert_into)?
        }
        DialogDataComponentValueState::Integer { value }
        | DialogDataComponentValueState::RgbColor { value } => value.to_string().len(),
        DialogDataComponentValueState::Float { bits } => canonical_f32_json_utf16_len(*bits)?,
        DialogDataComponentValueState::Boolean { value } => bool_len(*value),
        DialogDataComponentValueState::Unit => 2,
        DialogDataComponentValueState::ResourceId { resource_id } => string_len(resource_id),
        DialogDataComponentValueState::Rarity { value } => string_len(value.as_str()),
        DialogDataComponentValueState::StringEnum {
            serialized_name, ..
        } => string_len(serialized_name),
        DialogDataComponentValueState::UseEffects {
            can_sprint,
            interact_vibrations,
            speed_multiplier_bits,
        } => {
            let mut fields = Vec::new();
            if *can_sprint {
                fields.push(("can_sprint", bool_len(true)));
            }
            if !*interact_vibrations {
                fields.push(("interact_vibrations", bool_len(false)));
            }
            if *speed_multiplier_bits != 0.2f32.to_bits() {
                fields.push((
                    "speed_multiplier",
                    canonical_f32_json_utf16_len(*speed_multiplier_bits)?,
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::SwingAnimation {
            animation_type,
            duration,
        } => {
            let mut fields = Vec::new();
            if animation_type.as_str() != "whack" {
                fields.push(("type", string_len(animation_type.as_str())));
            }
            if *duration != 6 {
                fields.push(("duration", duration.to_string().len()));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::BlockState { properties } => {
            collection_len(properties.iter().map(|entry| {
                string_len(&entry.key)
                    .saturating_add(1)
                    .saturating_add(string_len(&entry.value))
            }))
        }
        DialogDataComponentValueState::Food {
            nutrition,
            saturation_bits,
            can_always_eat,
        } => {
            let mut fields = vec![
                ("nutrition", nutrition.to_string().len()),
                (
                    "saturation",
                    canonical_f32_json_utf16_len(*saturation_bits)?,
                ),
            ];
            if *can_always_eat {
                fields.push(("can_always_eat", bool_len(true)));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::UseCooldown {
            seconds_bits,
            cooldown_group,
        } => {
            let mut fields = vec![("seconds", canonical_f32_json_utf16_len(*seconds_bits)?)];
            if let Some(cooldown_group) = cooldown_group {
                fields.push(("cooldown_group", string_len(cooldown_group)));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::Weapon {
            item_damage_per_attack,
            disable_blocking_for_seconds_bits,
        } => {
            let mut fields = Vec::new();
            if *item_damage_per_attack != 1 {
                fields.push((
                    "item_damage_per_attack",
                    item_damage_per_attack.to_string().len(),
                ));
            }
            if *disable_blocking_for_seconds_bits != 0.0f32.to_bits() {
                fields.push((
                    "disable_blocking_for_seconds",
                    canonical_f32_json_utf16_len(*disable_blocking_for_seconds_bits)?,
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::AttackRange {
            min_reach_bits,
            max_reach_bits,
            min_creative_reach_bits,
            max_creative_reach_bits,
            hitbox_margin_bits,
            mob_factor_bits,
        } => {
            let mut fields = Vec::new();
            append_non_default_float(&mut fields, "min_reach", *min_reach_bits, 0.0)?;
            append_non_default_float(&mut fields, "max_reach", *max_reach_bits, 3.0)?;
            append_non_default_float(
                &mut fields,
                "min_creative_reach",
                *min_creative_reach_bits,
                0.0,
            )?;
            append_non_default_float(
                &mut fields,
                "max_creative_reach",
                *max_creative_reach_bits,
                5.0,
            )?;
            append_non_default_float(&mut fields, "hitbox_margin", *hitbox_margin_bits, 0.3)?;
            append_non_default_float(&mut fields, "mob_factor", *mob_factor_bits, 1.0)?;
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::Enchantable { value } => {
            object_len([("value", value.to_string().len())].into_iter())
        }
        DialogDataComponentValueState::Recipes { resource_ids } => array_len(
            resource_ids
                .iter()
                .map(|resource_id| string_len(resource_id)),
        ),
        DialogDataComponentValueState::TooltipDisplay {
            hide_tooltip,
            hidden_components,
        } => {
            let mut fields = Vec::new();
            if *hide_tooltip {
                fields.push(("hide_tooltip", bool_len(true)));
            }
            if !hidden_components.is_empty() {
                fields.push((
                    "hidden_components",
                    array_len(
                        hidden_components
                            .iter()
                            .map(|component| string_len(&component.resource_id)),
                    ),
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::ContainerLoot { loot_table, seed } => {
            let mut fields = vec![("loot_table", string_len(loot_table))];
            if *seed != 0 {
                fields.push(("seed", seed.to_string().len()));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::CustomModelData {
            floats_bits,
            flags,
            strings,
            colors,
        } => {
            let mut fields = Vec::new();
            if !floats_bits.is_empty() {
                fields.push((
                    "floats",
                    array_len(
                        floats_bits
                            .iter()
                            .map(|bits| canonical_f32_json_utf16_len(*bits))
                            .collect::<Option<Vec<_>>>()?
                            .into_iter(),
                    ),
                ));
            }
            if !flags.is_empty() {
                fields.push((
                    "flags",
                    array_len(flags.iter().map(|value| bool_len(*value))),
                ));
            }
            if !strings.is_empty() {
                fields.push((
                    "strings",
                    array_len(strings.iter().map(|value| string_len(value))),
                ));
            }
            if !colors.is_empty() {
                fields.push((
                    "colors",
                    array_len(colors.iter().map(|value| value.to_string().len())),
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::FireworkExplosion { explosion } => {
            firework_explosion_len(explosion)
        }
        DialogDataComponentValueState::Fireworks {
            flight_duration,
            explosions,
        } => {
            let mut fields = Vec::new();
            if *flight_duration != 0 {
                fields.push(("flight_duration", flight_duration.to_string().len()));
            }
            if !explosions.is_empty() {
                fields.push((
                    "explosions",
                    array_len(explosions.iter().map(firework_explosion_len)),
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::LodestoneTracker { target, tracked } => {
            let mut fields = Vec::new();
            if let Some(target) = target {
                fields.push((
                    "target",
                    object_len(
                        [
                            ("dimension", string_len(&target.dimension)),
                            (
                                "pos",
                                array_len(
                                    [target.pos.x, target.pos.y, target.pos.z]
                                        .into_iter()
                                        .map(|value| value.to_string().len()),
                                ),
                            ),
                        ]
                        .into_iter(),
                    ),
                ));
            }
            if !*tracked {
                fields.push(("tracked", bool_len(false)));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::Profile { profile } => {
            canonical_resolvable_profile_json_utf16_len(profile)?
        }
        DialogDataComponentValueState::MapDecorations { decorations } => collection_len(
            decorations
                .iter()
                .map(|decoration| {
                    Some(
                        string_len(&decoration.id)
                            .saturating_add(1)
                            .saturating_add(object_len(
                                [
                                    ("type", string_len(&decoration.decoration_type.resource_id)),
                                    ("x", canonical_f64_json_utf16_len(decoration.x_bits)?),
                                    ("z", canonical_f64_json_utf16_len(decoration.z_bits)?),
                                    (
                                        "rotation",
                                        canonical_f32_json_utf16_len(decoration.rotation_bits)?,
                                    ),
                                ]
                                .into_iter(),
                            )),
                    )
                })
                .collect::<Option<Vec<_>>>()?
                .into_iter(),
        ),
        DialogDataComponentValueState::ChargedProjectiles { items }
        | DialogDataComponentValueState::BundleContents { items } => array_len(
            items
                .iter()
                .map(item_stack_template_len)
                .collect::<Option<Vec<_>>>()?
                .into_iter(),
        ),
        DialogDataComponentValueState::Container { slots } => array_len(
            slots
                .iter()
                .enumerate()
                .filter_map(|(slot, item)| {
                    item.as_ref().map(|item| {
                        Some(object_len(
                            [
                                ("slot", slot.to_string().len()),
                                ("item", item_stack_template_len(item)?),
                            ]
                            .into_iter(),
                        ))
                    })
                })
                .collect::<Option<Vec<_>>>()?
                .into_iter(),
        ),
        DialogDataComponentValueState::SuspiciousStewEffects { effects } => {
            array_len(effects.iter().map(|effect| {
                let mut fields = vec![("id", string_len(&effect.effect.resource_id))];
                if effect.duration != 160 {
                    fields.push(("duration", effect.duration.to_string().len()));
                }
                object_len(fields.into_iter())
            }))
        }
        DialogDataComponentValueState::PotDecorations { decorations } => array_len(
            [
                decorations.back.as_ref(),
                decorations.left.as_ref(),
                decorations.right.as_ref(),
                decorations.front.as_ref(),
            ]
            .into_iter()
            .map(|decoration| {
                string_len(
                    decoration
                        .map(|decoration| decoration.resource_id.as_str())
                        .unwrap_or("minecraft:brick"),
                )
            }),
        ),
        DialogDataComponentValueState::RegistryVariant { holder, .. } => {
            string_len(&holder.resource_id)
        }
        DialogDataComponentValueState::JukeboxPlayable { song } => string_len(&song.resource_id),
        DialogDataComponentValueState::Instrument { instrument } => {
            instrument_holder_len(instrument)?
        }
        DialogDataComponentValueState::ProvidesTrimMaterial { material } => {
            trim_material_holder_len(material)?
        }
        DialogDataComponentValueState::Trim { material, pattern } => object_len(
            [
                ("material", trim_material_holder_len(material)?),
                ("pattern", trim_pattern_holder_len(pattern)?),
            ]
            .into_iter(),
        ),
        DialogDataComponentValueState::PaintingVariant { variant } => {
            string_len(&variant.resource_id)
        }
        DialogDataComponentValueState::DamageType { damage_type } => {
            string_len(&damage_type.resource_id)
        }
        DialogDataComponentValueState::Repairable { items } => {
            object_len([("items", registry_holder_set_len(items)?)].into_iter())
        }
        DialogDataComponentValueState::ProvidesBannerPatterns { patterns } => {
            registry_holder_set_len(patterns)?
        }
        DialogDataComponentValueState::DamageResistant { types } => {
            object_len([("types", registry_holder_set_len(types)?)].into_iter())
        }
        DialogDataComponentValueState::BreakSound { sound } => sound_event_len(sound)?,
        DialogDataComponentValueState::PiercingWeapon { weapon } => piercing_weapon_len(weapon)?,
        DialogDataComponentValueState::KineticWeapon { weapon } => kinetic_weapon_len(weapon)?,
        DialogDataComponentValueState::BannerPatterns { layers } => {
            array_len(layers.iter().map(|layer| {
                object_len(
                    [
                        ("pattern", banner_pattern_holder_len(&layer.pattern)),
                        ("color", string_len(&layer.color)),
                    ]
                    .into_iter(),
                )
            }))
        }
        DialogDataComponentValueState::Bees { occupants } => array_len(
            occupants
                .iter()
                .map(|occupant| {
                    Some(object_len(
                        [
                            (
                                "entity_data",
                                typed_data_json_utf16_len(
                                    &occupant.entity_data,
                                    &occupant.entity_type.resource_id,
                                )
                                .ok()?,
                            ),
                            ("ticks_in_hive", occupant.ticks_in_hive.to_string().len()),
                            (
                                "min_ticks_in_hive",
                                occupant.min_ticks_in_hive.to_string().len(),
                            ),
                        ]
                        .into_iter(),
                    ))
                })
                .collect::<Option<Vec<_>>>()?
                .into_iter(),
        ),
        DialogDataComponentValueState::EntityData { entity } => {
            typed_data_json_utf16_len(&entity.data, &entity.entity_type.resource_id).ok()?
        }
        DialogDataComponentValueState::BlockEntityData { block_entity } => {
            typed_data_json_utf16_len(
                &block_entity.data,
                &block_entity.block_entity_type.resource_id,
            )
            .ok()?
        }
        DialogDataComponentValueState::BucketEntityData { data }
        | DialogDataComponentValueState::CustomData { data } => {
            canonical_nbt_json_utf16_len(data).ok()?
        }
        DialogDataComponentValueState::DebugStickState { properties } => {
            collection_len(properties.iter().map(|entry| {
                string_len(&entry.block.resource_id)
                    .saturating_add(1)
                    .saturating_add(string_len(&entry.property))
            }))
        }
        DialogDataComponentValueState::Enchantments { entries }
        | DialogDataComponentValueState::StoredEnchantments { entries } => {
            collection_len(entries.iter().map(|entry| {
                string_len(&entry.enchantment.resource_id)
                    .saturating_add(1)
                    .saturating_add(entry.level.to_string().len())
            }))
        }
        DialogDataComponentValueState::WritableBookContent { pages } => {
            let mut fields = Vec::new();
            if !pages.is_empty() {
                fields.push((
                    "pages",
                    array_len(pages.iter().map(|page| {
                        let mut fields = vec![("raw", string_len(&page.raw))];
                        if let Some(filtered) = &page.filtered {
                            fields.push(("filtered", string_len(filtered)));
                        }
                        object_len(fields.into_iter())
                    })),
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::Tool { tool } => {
            let mut fields = vec![(
                "rules",
                array_len(
                    tool.rules
                        .iter()
                        .map(|rule| {
                            let mut fields =
                                vec![("blocks", registry_holder_set_len(&rule.blocks)?)];
                            if let Some(bits) = rule.speed_bits {
                                fields.push(("speed", canonical_f32_json_utf16_len(bits)?));
                            }
                            if let Some(correct_for_drops) = rule.correct_for_drops {
                                fields.push(("correct_for_drops", bool_len(correct_for_drops)));
                            }
                            Some(object_len(fields.into_iter()))
                        })
                        .collect::<Option<Vec<_>>>()?
                        .into_iter(),
                ),
            )];
            if tool.default_mining_speed_bits != 1.0f32.to_bits() {
                fields.push((
                    "default_mining_speed",
                    canonical_f32_json_utf16_len(tool.default_mining_speed_bits)?,
                ));
            }
            if tool.damage_per_block != 1 {
                fields.push(("damage_per_block", tool.damage_per_block.to_string().len()));
            }
            if !tool.can_destroy_blocks_in_creative {
                fields.push(("can_destroy_blocks_in_creative", bool_len(false)));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::Equippable { equippable } => {
            let mut fields = vec![("slot", string_len(&equippable.slot))];
            if !is_named_sound(
                &equippable.equip_sound,
                "minecraft:item.armor.equip_generic",
            ) {
                fields.push(("equip_sound", sound_event_len(&equippable.equip_sound)?));
            }
            if let Some(asset_id) = &equippable.asset_id {
                fields.push(("asset_id", string_len(asset_id)));
            }
            if let Some(camera_overlay) = &equippable.camera_overlay {
                fields.push(("camera_overlay", string_len(camera_overlay)));
            }
            if let Some(allowed_entities) = &equippable.allowed_entities {
                fields.push((
                    "allowed_entities",
                    registry_holder_set_len(allowed_entities)?,
                ));
            }
            if !equippable.dispensable {
                fields.push(("dispensable", bool_len(false)));
            }
            if !equippable.swappable {
                fields.push(("swappable", bool_len(false)));
            }
            if !equippable.damage_on_hurt {
                fields.push(("damage_on_hurt", bool_len(false)));
            }
            if equippable.equip_on_interact {
                fields.push(("equip_on_interact", bool_len(true)));
            }
            if equippable.can_be_sheared {
                fields.push(("can_be_sheared", bool_len(true)));
            }
            if !is_named_sound(&equippable.shearing_sound, "minecraft:item.shears.snip") {
                fields.push((
                    "shearing_sound",
                    sound_event_len(&equippable.shearing_sound)?,
                ));
            }
            object_len(fields.into_iter())
        }
        DialogDataComponentValueState::BlocksAttacks { blocks_attacks } => {
            blocks_attacks_len(blocks_attacks)?
        }
        DialogDataComponentValueState::PotionContents { contents } => potion_contents_len(contents),
        DialogDataComponentValueState::Consumable { consumable } => consumable_len(consumable)?,
        DialogDataComponentValueState::DeathProtection { effects } => {
            death_protection_len(effects)?
        }
    })
}

fn typed_data_json_utf16_len(data: &NbtSummaryValue, resource_id: &str) -> Result<usize, String> {
    let NbtSummaryValue::Compound(entries) = data else {
        return Err("typed entity data owner did not retain a compound".to_string());
    };
    let mut entries = entries.clone();
    entries.push(NbtSummaryEntry {
        name: "id".to_string(),
        value: NbtSummaryValue::String(resource_id.to_string()),
    });
    canonical_nbt_json_utf16_len(&NbtSummaryValue::Compound(entries))
}

fn component_json_len(component: &NbtSummaryValue) -> Option<usize> {
    // The production caller has already propagated canonical Component errors through
    // validate_supported_json_numbers; None here is reserved for a remaining higher owner.
    canonical_component_json_utf16_len(component).ok().flatten()
}

fn written_book_content_len(content: &DialogWrittenBookContentState) -> Option<usize> {
    let mut title_fields = vec![("raw", string_len(&content.title.raw))];
    if let Some(filtered) = &content.title.filtered {
        title_fields.push(("filtered", string_len(filtered)));
    }
    let mut fields = vec![
        ("title", object_len(title_fields.into_iter())),
        ("author", string_len(&content.author)),
    ];
    if content.generation != 0 {
        fields.push(("generation", content.generation.to_string().len()));
    }
    if !content.pages.is_empty() {
        fields.push((
            "pages",
            array_len(
                content
                    .pages
                    .iter()
                    .map(|page| {
                        let mut fields = vec![("raw", component_json_len(&page.raw.component)?)];
                        if let Some(filtered) = &page.filtered {
                            fields.push(("filtered", component_json_len(&filtered.component)?));
                        }
                        Some(object_len(fields.into_iter()))
                    })
                    .collect::<Option<Vec<_>>>()?
                    .into_iter(),
            ),
        ));
    }
    if content.resolved {
        fields.push(("resolved", bool_len(true)));
    }
    Some(object_len(fields.into_iter()))
}

fn validate_item_stack_template_json(item: &DialogItemStackTemplateState) -> Result<(), String> {
    if let Some(patch) = &item.component_data {
        canonical_data_component_patch_json_utf16_len(patch)?;
    }
    Ok(())
}

fn item_stack_template_len(item: &DialogItemStackTemplateState) -> Option<usize> {
    let mut fields = vec![("id", string_len(&item.resource_id))];
    if item.count != 1 {
        fields.push(("count", item.count.to_string().len()));
    }
    match &item.component_data {
        Some(patch) if !patch.entries.is_empty() => fields.push((
            "components",
            canonical_data_component_patch_json_utf16_len(patch)
                .ok()
                .flatten()?,
        )),
        Some(_) => {}
        None if item.components.is_some() => return None,
        None => {}
    }
    Some(object_len(fields.into_iter()))
}

fn attribute_modifiers_len(modifiers: &[DialogAttributeModifierState]) -> Option<usize> {
    Some(array_len(
        modifiers
            .iter()
            .map(attribute_modifier_len)
            .collect::<Option<Vec<_>>>()?
            .into_iter(),
    ))
}

fn attribute_modifier_len(modifier: &DialogAttributeModifierState) -> Option<usize> {
    let mut fields = vec![
        ("type", string_len(&modifier.attribute.resource_id)),
        ("id", string_len(&modifier.modifier_id)),
        (
            "amount",
            canonical_f64_json_utf16_len(modifier.amount_bits)?,
        ),
        ("operation", string_len(&modifier.operation)),
    ];
    if modifier.slot_group_id != VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID {
        fields.push(("slot", string_len(&modifier.slot_group)));
    }
    if modifier.display.type_id != VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID {
        fields.push((
            "display",
            attribute_modifier_display_len(&modifier.display)?,
        ));
    }
    Some(object_len(fields.into_iter()))
}

fn attribute_modifier_display_len(display: &DialogAttributeModifierDisplayState) -> Option<usize> {
    let mut fields = vec![("type", string_len(&display.serialized_name))];
    if display.type_id == VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID {
        fields.push((
            "value",
            component_json_len(&display.value.as_ref()?.component)?,
        ));
    }
    Some(object_len(fields.into_iter()))
}

fn instrument_holder_len(instrument: &DialogInstrumentHolderState) -> Option<usize> {
    match instrument {
        DialogInstrumentHolderState::Named { resource_id, .. } => Some(string_len(resource_id)),
        DialogInstrumentHolderState::Direct { instrument } => direct_instrument_len(instrument),
    }
}

fn direct_instrument_len(instrument: &DialogInstrumentState) -> Option<usize> {
    Some(object_len(
        [
            ("sound_event", sound_event_len(&instrument.sound_event)?),
            (
                "use_duration",
                canonical_f32_json_utf16_len(instrument.use_duration_bits)?,
            ),
            (
                "range",
                canonical_f32_json_utf16_len(instrument.range_bits)?,
            ),
            (
                "description",
                component_json_len(&instrument.description.component)?,
            ),
        ]
        .into_iter(),
    ))
}

fn trim_material_holder_len(material: &DialogTrimMaterialHolderState) -> Option<usize> {
    match material {
        DialogTrimMaterialHolderState::Named { resource_id, .. } => Some(string_len(resource_id)),
        DialogTrimMaterialHolderState::Direct { material } => direct_trim_material_len(material),
    }
}

fn direct_trim_material_len(material: &DialogTrimMaterialState) -> Option<usize> {
    let mut fields = vec![("asset_name", string_len(&material.asset_name))];
    if !material.override_armor_assets.is_empty() {
        fields.push((
            "override_armor_assets",
            object_len(
                material
                    .override_armor_assets
                    .iter()
                    .map(|(asset, suffix)| (asset.as_str(), string_len(suffix))),
            ),
        ));
    }
    fields.push((
        "description",
        component_json_len(&material.description.component)?,
    ));
    Some(object_len(fields.into_iter()))
}

fn trim_pattern_holder_len(pattern: &DialogTrimPatternHolderState) -> Option<usize> {
    match pattern {
        DialogTrimPatternHolderState::Named { resource_id, .. } => Some(string_len(resource_id)),
        DialogTrimPatternHolderState::Direct { pattern } => direct_trim_pattern_len(pattern),
    }
}

fn direct_trim_pattern_len(pattern: &DialogTrimPatternState) -> Option<usize> {
    Some(object_len(
        [
            ("asset_id", string_len(&pattern.asset_id)),
            (
                "description",
                component_json_len(&pattern.description.component)?,
            ),
            // fieldOf("decal").orElse(false) falls back while decoding but still
            // delegates every encode to the required field codec.
            ("decal", bool_len(pattern.decal)),
        ]
        .into_iter(),
    ))
}

fn banner_pattern_holder_len(holder: &DialogBannerPatternHolderState) -> usize {
    match holder {
        DialogBannerPatternHolderState::Named { resource_id, .. } => string_len(resource_id),
        DialogBannerPatternHolderState::Direct { pattern } => object_len(
            [
                ("asset_id", string_len(&pattern.asset_id)),
                ("translation_key", string_len(&pattern.translation_key)),
            ]
            .into_iter(),
        ),
    }
}

fn append_sound_range_bits(values: &mut Vec<u32>, sound: Option<&DialogSoundEventHolderState>) {
    if let Some(DialogSoundEventHolderState::Direct {
        fixed_range_bits: Some(bits),
        ..
    }) = sound
    {
        values.push(*bits);
    }
}

fn is_named_sound(sound: &DialogSoundEventHolderState, resource_id: &str) -> bool {
    matches!(
        sound,
        DialogSoundEventHolderState::Named {
            resource_id: actual,
            ..
        } if actual == resource_id
    )
}

fn append_consume_effect_float_bits(values: &mut Vec<u32>, effects: &[DialogConsumeEffectState]) {
    for effect in effects {
        match effect {
            DialogConsumeEffectState::ApplyEffects {
                probability_bits, ..
            } => values.push(*probability_bits),
            DialogConsumeEffectState::TeleportRandomly { diameter_bits } => {
                values.push(*diameter_bits);
            }
            DialogConsumeEffectState::PlaySound { sound } => {
                append_sound_range_bits(values, Some(sound));
            }
            DialogConsumeEffectState::RemoveEffects { .. }
            | DialogConsumeEffectState::ClearAllEffects => {}
        }
    }
}

fn potion_contents_len(contents: &DialogPotionContentsState) -> usize {
    let mut fields = Vec::new();
    if let Some(potion) = &contents.potion {
        fields.push(("potion", string_len(&potion.resource_id)));
    }
    if let Some(custom_color) = contents.custom_color {
        fields.push(("custom_color", custom_color.to_string().len()));
    }
    if !contents.custom_effects.is_empty() {
        fields.push((
            "custom_effects",
            array_len(contents.custom_effects.iter().map(mob_effect_instance_len)),
        ));
    }
    if let Some(custom_name) = &contents.custom_name {
        fields.push(("custom_name", string_len(custom_name)));
    }
    object_len(fields.into_iter())
}

fn consumable_len(consumable: &DialogConsumableState) -> Option<usize> {
    let mut fields = Vec::new();
    append_non_default_float(
        &mut fields,
        "consume_seconds",
        consumable.consume_seconds_bits,
        1.6,
    )?;
    if consumable.animation != ItemUseAnimationSummary::Eat {
        fields.push(("animation", string_len(consumable.animation.as_str())));
    }
    if !is_named_sound(&consumable.sound, "minecraft:entity.generic.eat") {
        fields.push(("sound", sound_event_len(&consumable.sound)?));
    }
    if !consumable.has_consume_particles {
        fields.push(("has_consume_particles", bool_len(false)));
    }
    if !consumable.on_consume_effects.is_empty() {
        fields.push((
            "on_consume_effects",
            consume_effects_len(&consumable.on_consume_effects)?,
        ));
    }
    Some(object_len(fields.into_iter()))
}

fn death_protection_len(effects: &[DialogConsumeEffectState]) -> Option<usize> {
    let mut fields = Vec::new();
    if !effects.is_empty() {
        fields.push(("death_effects", consume_effects_len(effects)?));
    }
    Some(object_len(fields.into_iter()))
}

fn consume_effects_len(effects: &[DialogConsumeEffectState]) -> Option<usize> {
    Some(array_len(
        effects
            .iter()
            .map(consume_effect_len)
            .collect::<Option<Vec<_>>>()?
            .into_iter(),
    ))
}

fn consume_effect_len(effect: &DialogConsumeEffectState) -> Option<usize> {
    let mut fields = Vec::new();
    match effect {
        DialogConsumeEffectState::ApplyEffects {
            effects,
            probability_bits,
        } => {
            fields.push(("type", string_len("minecraft:apply_effects")));
            fields.push((
                "effects",
                array_len(effects.iter().map(mob_effect_instance_len)),
            ));
            append_non_default_float(&mut fields, "probability", *probability_bits, 1.0)?;
        }
        DialogConsumeEffectState::RemoveEffects { effects } => {
            fields.push(("type", string_len("minecraft:remove_effects")));
            fields.push(("effects", registry_holder_set_len(effects)?));
        }
        DialogConsumeEffectState::ClearAllEffects => {
            fields.push(("type", string_len("minecraft:clear_all_effects")));
        }
        DialogConsumeEffectState::TeleportRandomly { diameter_bits } => {
            fields.push(("type", string_len("minecraft:teleport_randomly")));
            append_non_default_float(&mut fields, "diameter", *diameter_bits, 16.0)?;
        }
        DialogConsumeEffectState::PlaySound { sound } => {
            fields.push(("type", string_len("minecraft:play_sound")));
            fields.push(("sound", sound_event_len(sound)?));
        }
    }
    Some(object_len(fields.into_iter()))
}

fn mob_effect_instance_len(effect: &DialogMobEffectInstanceState) -> usize {
    let mut fields = vec![("id", string_len(&effect.effect.resource_id))];
    append_mob_effect_details_fields(&mut fields, &effect.details);
    object_len(fields.into_iter())
}

fn mob_effect_details_len(details: &DialogMobEffectDetailsState) -> usize {
    let mut fields = Vec::new();
    append_mob_effect_details_fields(&mut fields, details);
    object_len(fields.into_iter())
}

fn append_mob_effect_details_fields(
    fields: &mut Vec<(&'static str, usize)>,
    details: &DialogMobEffectDetailsState,
) {
    if details.amplifier != 0 {
        fields.push(("amplifier", details.amplifier.to_string().len()));
    }
    if details.duration != 0 {
        fields.push(("duration", details.duration.to_string().len()));
    }
    if details.ambient {
        fields.push(("ambient", bool_len(true)));
    }
    if !details.show_particles {
        fields.push(("show_particles", bool_len(false)));
    }
    // Details.MAP_CODEC decodes an absent field from show_particles, but its getter always
    // returns Optional.of(showIcon), so every encoded details record includes show_icon.
    fields.push(("show_icon", bool_len(details.show_icon)));
    if let Some(hidden_effect) = &details.hidden_effect {
        fields.push(("hidden_effect", mob_effect_details_len(hidden_effect)));
    }
}

fn blocks_attacks_len(blocks_attacks: &DialogBlocksAttacksState) -> Option<usize> {
    let mut fields = Vec::new();
    append_non_default_float(
        &mut fields,
        "block_delay_seconds",
        blocks_attacks.block_delay_seconds_bits,
        0.0,
    )?;
    append_non_default_float(
        &mut fields,
        "disable_cooldown_scale",
        blocks_attacks.disable_cooldown_scale_bits,
        1.0,
    )?;
    if !is_default_damage_reductions(&blocks_attacks.damage_reductions) {
        fields.push((
            "damage_reductions",
            array_len(
                blocks_attacks
                    .damage_reductions
                    .iter()
                    .map(damage_reduction_len)
                    .collect::<Option<Vec<_>>>()?
                    .into_iter(),
            ),
        ));
    }
    if !is_default_item_damage(&blocks_attacks.item_damage) {
        fields.push(("item_damage", item_damage_len(&blocks_attacks.item_damage)?));
    }
    if let Some(bypassed_by) = &blocks_attacks.bypassed_by {
        fields.push(("bypassed_by", registry_holder_set_len(bypassed_by)?));
    }
    if let Some(block_sound) = &blocks_attacks.block_sound {
        fields.push(("block_sound", sound_event_len(block_sound)?));
    }
    if let Some(disabled_sound) = &blocks_attacks.disabled_sound {
        fields.push(("disabled_sound", sound_event_len(disabled_sound)?));
    }
    Some(object_len(fields.into_iter()))
}

fn is_default_damage_reductions(reductions: &[DialogBlocksAttacksDamageReductionState]) -> bool {
    matches!(
        reductions,
        [reduction]
            if reduction.horizontal_blocking_angle_bits == 90.0f32.to_bits()
                && reduction.damage_types.is_none()
                && reduction.base_bits == 0.0f32.to_bits()
                && reduction.factor_bits == 1.0f32.to_bits()
    )
}

fn damage_reduction_len(reduction: &DialogBlocksAttacksDamageReductionState) -> Option<usize> {
    let mut fields = Vec::new();
    append_non_default_float(
        &mut fields,
        "horizontal_blocking_angle",
        reduction.horizontal_blocking_angle_bits,
        90.0,
    )?;
    if let Some(damage_types) = &reduction.damage_types {
        fields.push(("type", registry_holder_set_len(damage_types)?));
    }
    fields.push(("base", canonical_f32_json_utf16_len(reduction.base_bits)?));
    fields.push((
        "factor",
        canonical_f32_json_utf16_len(reduction.factor_bits)?,
    ));
    Some(object_len(fields.into_iter()))
}

fn is_default_item_damage(item_damage: &DialogBlocksAttacksItemDamageState) -> bool {
    item_damage.threshold_bits == 1.0f32.to_bits()
        && item_damage.base_bits == 0.0f32.to_bits()
        && item_damage.factor_bits == 1.0f32.to_bits()
}

fn item_damage_len(item_damage: &DialogBlocksAttacksItemDamageState) -> Option<usize> {
    Some(object_len(
        [
            (
                "threshold",
                canonical_f32_json_utf16_len(item_damage.threshold_bits)?,
            ),
            ("base", canonical_f32_json_utf16_len(item_damage.base_bits)?),
            (
                "factor",
                canonical_f32_json_utf16_len(item_damage.factor_bits)?,
            ),
        ]
        .into_iter(),
    ))
}

fn piercing_weapon_len(weapon: &DialogPiercingWeaponState) -> Option<usize> {
    let mut fields = Vec::new();
    if !weapon.deals_knockback {
        fields.push(("deals_knockback", bool_len(false)));
    }
    if weapon.dismounts {
        fields.push(("dismounts", bool_len(true)));
    }
    if let Some(sound) = &weapon.sound {
        fields.push(("sound", sound_event_len(sound)?));
    }
    if let Some(sound) = &weapon.hit_sound {
        fields.push(("hit_sound", sound_event_len(sound)?));
    }
    Some(object_len(fields.into_iter()))
}

fn kinetic_weapon_len(weapon: &DialogKineticWeaponState) -> Option<usize> {
    let mut fields = Vec::new();
    if weapon.contact_cooldown_ticks != 10 {
        fields.push((
            "contact_cooldown_ticks",
            weapon.contact_cooldown_ticks.to_string().len(),
        ));
    }
    if weapon.delay_ticks != 0 {
        fields.push(("delay_ticks", weapon.delay_ticks.to_string().len()));
    }
    for (name, condition) in [
        ("dismount_conditions", weapon.dismount_conditions.as_ref()),
        ("knockback_conditions", weapon.knockback_conditions.as_ref()),
        ("damage_conditions", weapon.damage_conditions.as_ref()),
    ] {
        if let Some(condition) = condition {
            fields.push((name, kinetic_condition_len(condition)?));
        }
    }
    append_non_default_float(
        &mut fields,
        "forward_movement",
        weapon.forward_movement_bits,
        0.0,
    )?;
    append_non_default_float(
        &mut fields,
        "damage_multiplier",
        weapon.damage_multiplier_bits,
        1.0,
    )?;
    if let Some(sound) = &weapon.sound {
        fields.push(("sound", sound_event_len(sound)?));
    }
    if let Some(sound) = &weapon.hit_sound {
        fields.push(("hit_sound", sound_event_len(sound)?));
    }
    Some(object_len(fields.into_iter()))
}

fn kinetic_condition_len(condition: &DialogKineticWeaponConditionState) -> Option<usize> {
    let mut fields = vec![(
        "max_duration_ticks",
        condition.max_duration_ticks.to_string().len(),
    )];
    append_non_default_float(&mut fields, "min_speed", condition.min_speed_bits, 0.0)?;
    append_non_default_float(
        &mut fields,
        "min_relative_speed",
        condition.min_relative_speed_bits,
        0.0,
    )?;
    Some(object_len(fields.into_iter()))
}

fn sound_event_len(sound: &DialogSoundEventHolderState) -> Option<usize> {
    match sound {
        DialogSoundEventHolderState::Named { resource_id, .. } => Some(string_len(resource_id)),
        DialogSoundEventHolderState::Direct {
            sound_id,
            fixed_range_bits,
        } => {
            let mut fields = vec![("sound_id", string_len(sound_id))];
            if let Some(bits) = fixed_range_bits {
                fields.push(("range", canonical_f32_json_utf16_len(*bits)?));
            }
            Some(object_len(fields.into_iter()))
        }
    }
}

fn registry_holder_set_len(holder_set: &DialogRegistryHolderSetState) -> Option<usize> {
    match holder_set.holder_set_kind.as_str() {
        "named" => holder_set
            .tag
            .as_ref()
            .map(|tag| string_len(&format!("#{tag}"))),
        "direct" => Some(if let [entry] = holder_set.entries.as_slice() {
            string_len(&entry.resource_id)
        } else {
            array_len(
                holder_set
                    .entries
                    .iter()
                    .map(|entry| string_len(&entry.resource_id)),
            )
        }),
        _ => None,
    }
}

fn firework_explosion_len(explosion: &FireworkExplosionSummary) -> usize {
    let mut fields = vec![("shape", string_len(explosion.shape.as_str()))];
    if !explosion.colors.is_empty() {
        fields.push((
            "colors",
            array_len(explosion.colors.iter().map(|value| value.to_string().len())),
        ));
    }
    if !explosion.fade_colors.is_empty() {
        fields.push((
            "fade_colors",
            array_len(
                explosion
                    .fade_colors
                    .iter()
                    .map(|value| value.to_string().len()),
            ),
        ));
    }
    if explosion.has_trail {
        fields.push(("has_trail", bool_len(true)));
    }
    if explosion.has_twinkle {
        fields.push(("has_twinkle", bool_len(true)));
    }
    object_len(fields.into_iter())
}

fn append_non_default_float(
    fields: &mut Vec<(&'static str, usize)>,
    name: &'static str,
    bits: u32,
    default: f32,
) -> Option<()> {
    if bits != default.to_bits() {
        fields.push((name, canonical_f32_json_utf16_len(bits)?));
    }
    Some(())
}

fn bool_len(value: bool) -> usize {
    if value {
        4
    } else {
        5
    }
}

fn string_len(value: &str) -> usize {
    canonical_json_string_utf16_len(value)
}

fn object_len<'a>(fields: impl Iterator<Item = (&'a str, usize)>) -> usize {
    collection_len(
        fields
            .map(|(name, value_len)| string_len(name).saturating_add(1).saturating_add(value_len)),
    )
}

fn array_len(values: impl Iterator<Item = usize>) -> usize {
    collection_len(values)
}

fn collection_len(values: impl Iterator<Item = usize>) -> usize {
    values.enumerate().fold(2usize, |length, (index, value)| {
        length
            .saturating_add(usize::from(index != 0))
            .saturating_add(value)
    })
}

#[cfg(test)]
mod tests {
    use bbb_protocol::item_component_value_types::{
        FireworkExplosionShapeSummary, SwingAnimationTypeSummary,
    };
    use bbb_protocol::packets::chunks::BlockPos;
    use bbb_protocol::packets::inventory::{
        LodestoneTargetSummary, PlayerSkinPatchSummary, ResolvableProfileKindSummary,
        ResolvableProfileSummary,
    };

    use super::super::holder_set::DialogRegistryHolderReferenceState;
    use super::super::map_decorations::DialogMapDecorationState;
    use super::*;

    #[test]
    fn canonicalizes_registry_independent_text_and_lore_components() {
        let compound = |entries: Vec<(&str, NbtSummaryValue)>| {
            NbtSummaryValue::Compound(
                entries
                    .into_iter()
                    .map(|(name, value)| NbtSummaryEntry {
                        name: name.to_string(),
                        value,
                    })
                    .collect(),
            )
        };
        let text_state = |component| super::super::DialogTextState {
            text: String::new(),
            runs: Vec::new(),
            component,
        };
        assert_exact(
            DialogDataComponentValueState::TextComponent {
                component: text_state(NbtSummaryValue::String("Named".to_string())),
            },
            r#""Named""#,
        );
        assert_exact(
            DialogDataComponentValueState::TextComponent {
                component: text_state(compound(vec![
                    ("text", NbtSummaryValue::String("Sword".to_string())),
                    ("bold", NbtSummaryValue::Byte(1)),
                    ("color", NbtSummaryValue::String("dark_purple".to_string())),
                ])),
            },
            r#"{"text":"Sword","color":"dark_purple","bold":true}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Lore {
                lines: vec![
                    text_state(compound(vec![(
                        "text",
                        NbtSummaryValue::String("First".to_string()),
                    )])),
                    text_state(compound(vec![
                        (
                            "translate",
                            NbtSummaryValue::String("item.second".to_string()),
                        ),
                        ("italic", NbtSummaryValue::Byte(0)),
                    ])),
                ],
            },
            r#"["First",{"translate":"item.second","italic":false}]"#,
        );
        assert_eq!(
            canonical_component_value_json_utf16_len(
                &DialogDataComponentValueState::TextComponent {
                    component: text_state(compound(vec![
                        ("object", NbtSummaryValue::String("player".to_string())),
                        ("player", NbtSummaryValue::String("Alex".to_string())),
                    ])),
                },
            ),
            None
        );
    }

    #[test]
    fn canonicalizes_named_and_registry_independent_direct_instruments() {
        assert_exact(
            DialogDataComponentValueState::Instrument {
                instrument: DialogInstrumentHolderState::Named {
                    registry_id: 0,
                    resource_id: "minecraft:ponder_goat_horn".to_string(),
                },
            },
            r#""minecraft:ponder_goat_horn""#,
        );
        assert_exact(
            DialogDataComponentValueState::Instrument {
                instrument: DialogInstrumentHolderState::Direct {
                    instrument: DialogInstrumentState {
                        sound_event: DialogSoundEventHolderState::Direct {
                            sound_id: "example:horn".to_string(),
                            fixed_range_bits: Some(16.0f32.to_bits()),
                        },
                        use_duration_bits: 7.0f32.to_bits(),
                        range_bits: 256.0f32.to_bits(),
                        description: super::super::DialogTextState {
                            text: String::new(),
                            runs: Vec::new(),
                            component: NbtSummaryValue::Compound(vec![
                                NbtSummaryEntry {
                                    name: "text".to_string(),
                                    value: NbtSummaryValue::String("Ponder".to_string()),
                                },
                                NbtSummaryEntry {
                                    name: "italic".to_string(),
                                    value: NbtSummaryValue::Byte(1),
                                },
                                NbtSummaryEntry {
                                    name: "color".to_string(),
                                    value: NbtSummaryValue::String("gold".to_string()),
                                },
                            ]),
                        },
                    },
                },
            },
            concat!(
                r#"{"sound_event":{"sound_id":"example:horn","range":16.0},"use_duration":7.0,"range":256.0,"description":{"text":"Ponder","color":"gold","italic":true}}"#
            ),
        );
    }

    #[test]
    fn canonicalizes_named_and_registry_independent_direct_trim_holders() {
        let text_state = |component| super::super::DialogTextState {
            text: String::new(),
            runs: Vec::new(),
            component,
        };
        assert_exact(
            DialogDataComponentValueState::ProvidesTrimMaterial {
                material: DialogTrimMaterialHolderState::Named {
                    registry_id: 0,
                    resource_id: "minecraft:resin".to_string(),
                },
            },
            r#""minecraft:resin""#,
        );
        assert_exact(
            DialogDataComponentValueState::ProvidesTrimMaterial {
                material: DialogTrimMaterialHolderState::Direct {
                    material: DialogTrimMaterialState {
                        asset_name: "quartz".to_string(),
                        override_armor_assets: std::collections::BTreeMap::from([
                            ("minecraft:iron".to_string(), "iron_darker".to_string()),
                            ("example:copper".to_string(), "nested/copper".to_string()),
                        ]),
                        description: text_state(NbtSummaryValue::Compound(vec![
                            NbtSummaryEntry {
                                name: "text".to_string(),
                                value: NbtSummaryValue::String("Quartz".to_string()),
                            },
                            NbtSummaryEntry {
                                name: "italic".to_string(),
                                value: NbtSummaryValue::Byte(1),
                            },
                        ])),
                    },
                },
            },
            concat!(
                r#"{"asset_name":"quartz","override_armor_assets":{"example:copper":"nested/copper","minecraft:iron":"iron_darker"},"description":{"text":"Quartz","italic":true}}"#
            ),
        );
        assert_exact(
            DialogDataComponentValueState::Trim {
                material: DialogTrimMaterialHolderState::Named {
                    registry_id: 0,
                    resource_id: "minecraft:quartz".to_string(),
                },
                pattern: DialogTrimPatternHolderState::Direct {
                    pattern: DialogTrimPatternState {
                        asset_id: "minecraft:sentry".to_string(),
                        description: text_state(NbtSummaryValue::String("Sentry".to_string())),
                        decal: false,
                    },
                },
            },
            concat!(
                r#"{"material":"minecraft:quartz","pattern":{"asset_id":"minecraft:sentry","description":"Sentry","decal":false}}"#
            ),
        );
        assert_exact(
            DialogDataComponentValueState::Trim {
                material: DialogTrimMaterialHolderState::Named {
                    registry_id: 0,
                    resource_id: "minecraft:quartz".to_string(),
                },
                pattern: DialogTrimPatternHolderState::Named {
                    registry_id: 0,
                    resource_id: "minecraft:coast".to_string(),
                },
            },
            r#"{"material":"minecraft:quartz","pattern":"minecraft:coast"}"#,
        );
    }

    #[test]
    fn canonicalizes_attribute_modifier_defaults_displays_and_components() {
        let attribute =
            |resource_id: &str| super::super::holder_set::DialogRegistryHolderReferenceState {
                holder_id: 0,
                resource_id: resource_id.to_string(),
            };
        assert_exact(
            DialogDataComponentValueState::AttributeModifiers {
                modifiers: vec![DialogAttributeModifierState {
                    attribute: attribute("minecraft:attack_damage"),
                    modifier_id: "minecraft:base_attack_damage".to_string(),
                    amount_bits: (-2.5f64).to_bits(),
                    operation_id: 0,
                    operation: "add_value".to_string(),
                    slot_group_id: 0,
                    slot_group: "any".to_string(),
                    display: DialogAttributeModifierDisplayState {
                        type_id: 0,
                        serialized_name: "default".to_string(),
                        value: None,
                    },
                }],
            },
            concat!(
                r#"[{"type":"minecraft:attack_damage","id":"minecraft:base_attack_damage","amount":-2.5,"operation":"add_value"}]"#
            ),
        );
        assert_exact(
            DialogDataComponentValueState::AttributeModifiers {
                modifiers: vec![DialogAttributeModifierState {
                    attribute: attribute("minecraft:armor"),
                    modifier_id: "example:hidden".to_string(),
                    amount_bits: 0.25f64.to_bits(),
                    operation_id: 1,
                    operation: "add_multiplied_base".to_string(),
                    slot_group_id: 1,
                    slot_group: "mainhand".to_string(),
                    display: DialogAttributeModifierDisplayState {
                        type_id: 1,
                        serialized_name: "hidden".to_string(),
                        value: None,
                    },
                }],
            },
            concat!(
                r#"[{"type":"minecraft:armor","id":"example:hidden","amount":0.25,"operation":"add_multiplied_base","slot":"mainhand","display":{"type":"hidden"}}]"#
            ),
        );
        assert_exact(
            DialogDataComponentValueState::AttributeModifiers {
                modifiers: vec![DialogAttributeModifierState {
                    attribute: attribute("minecraft:movement_speed"),
                    modifier_id: "example:override".to_string(),
                    amount_bits: 1.5f64.to_bits(),
                    operation_id: 2,
                    operation: "add_multiplied_total".to_string(),
                    slot_group_id: 7,
                    slot_group: "head".to_string(),
                    display: DialogAttributeModifierDisplayState {
                        type_id: 2,
                        serialized_name: "override".to_string(),
                        value: Some(super::super::DialogTextState {
                            text: String::new(),
                            runs: Vec::new(),
                            component: NbtSummaryValue::Compound(vec![
                                NbtSummaryEntry {
                                    name: "text".to_string(),
                                    value: NbtSummaryValue::String("Boost".to_string()),
                                },
                                NbtSummaryEntry {
                                    name: "bold".to_string(),
                                    value: NbtSummaryValue::Byte(1),
                                },
                                NbtSummaryEntry {
                                    name: "color".to_string(),
                                    value: NbtSummaryValue::String("red".to_string()),
                                },
                            ]),
                        }),
                    },
                }],
            },
            concat!(
                r#"[{"type":"minecraft:movement_speed","id":"example:override","amount":1.5,"operation":"add_multiplied_total","slot":"head","display":{"type":"override","value":{"text":"Boost","color":"red","bold":true}}}]"#
            ),
        );
        assert_exact(
            DialogDataComponentValueState::AttributeModifiers {
                modifiers: Vec::new(),
            },
            "[]",
        );
    }

    #[test]
    fn canonicalizes_recursive_item_stack_templates_and_sparse_slots() {
        let plain_item = |resource_id: &str| DialogItemStackTemplateState {
            resource_id: resource_id.to_string(),
            count: 1,
            components: None,
            component_data: None,
        };
        let patched_item = DialogItemStackTemplateState {
            resource_id: "minecraft:firework_rocket".to_string(),
            count: 3,
            components: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "repair_cost".to_string(),
                value: NbtSummaryValue::Int(4),
            }])),
            component_data: Some(DialogDataComponentPatchState {
                entries: vec![super::super::super::DialogDataComponentPatchEntryState {
                    type_id: 0,
                    resource_id: "minecraft:repair_cost".to_string(),
                    removed: false,
                    value: Some(NbtSummaryValue::Int(4)),
                    value_data: Some(DialogDataComponentValueState::Integer { value: 4 }),
                }],
            }),
        };
        assert_exact(
            DialogDataComponentValueState::UseRemainder {
                convert_into: plain_item("minecraft:glass_bottle"),
            },
            r#"{"id":"minecraft:glass_bottle"}"#,
        );
        for value in [
            DialogDataComponentValueState::ChargedProjectiles {
                items: vec![plain_item("minecraft:arrow"), patched_item.clone()],
            },
            DialogDataComponentValueState::BundleContents {
                items: vec![plain_item("minecraft:arrow"), patched_item],
            },
        ] {
            assert_exact(
                value,
                concat!(
                    r#"[{"id":"minecraft:arrow"},{"id":"minecraft:firework_rocket","count":3,"components":{"minecraft:repair_cost":4}}]"#
                ),
            );
        }
        assert_exact(
            DialogDataComponentValueState::Container {
                slots: vec![
                    Some(plain_item("minecraft:diamond")),
                    None,
                    Some(DialogItemStackTemplateState {
                        resource_id: "minecraft:dirt".to_string(),
                        count: 5,
                        components: None,
                        component_data: None,
                    }),
                ],
            },
            concat!(
                r#"[{"slot":0,"item":{"id":"minecraft:diamond"}},{"slot":2,"item":{"id":"minecraft:dirt","count":5}}]"#
            ),
        );
        assert_exact(
            DialogDataComponentValueState::BundleContents { items: Vec::new() },
            "[]",
        );
    }

    #[test]
    fn canonicalizes_flat_record_defaults_and_required_fields() {
        assert_exact(
            DialogDataComponentValueState::UseEffects {
                can_sprint: false,
                interact_vibrations: true,
                speed_multiplier_bits: 0.2f32.to_bits(),
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::UseEffects {
                can_sprint: true,
                interact_vibrations: false,
                speed_multiplier_bits: 0.5f32.to_bits(),
            },
            r#"{"can_sprint":true,"interact_vibrations":false,"speed_multiplier":0.5}"#,
        );
        assert_exact(
            DialogDataComponentValueState::SwingAnimation {
                animation_type: SwingAnimationTypeSummary::Whack,
                duration: 6,
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::SwingAnimation {
                animation_type: SwingAnimationTypeSummary::Stab,
                duration: 9,
            },
            r#"{"type":"stab","duration":9}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Food {
                nutrition: 4,
                saturation_bits: 0.5f32.to_bits(),
                can_always_eat: true,
            },
            r#"{"nutrition":4,"saturation":0.5,"can_always_eat":true}"#,
        );
        assert_exact(
            DialogDataComponentValueState::UseCooldown {
                seconds_bits: 1.25f32.to_bits(),
                cooldown_group: Some("minecraft:test".to_string()),
            },
            r#"{"seconds":1.25,"cooldown_group":"minecraft:test"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Weapon {
                item_damage_per_attack: 1,
                disable_blocking_for_seconds_bits: 0.0f32.to_bits(),
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::AttackRange {
                min_reach_bits: 0.0f32.to_bits(),
                max_reach_bits: 3.0f32.to_bits(),
                min_creative_reach_bits: 0.0f32.to_bits(),
                max_creative_reach_bits: 5.0f32.to_bits(),
                hitbox_margin_bits: 0.3f32.to_bits(),
                mob_factor_bits: 1.0f32.to_bits(),
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::Enchantable { value: 12 },
            r#"{"value":12}"#,
        );
    }

    #[test]
    fn canonicalizes_flat_maps_lists_metadata_and_escaped_strings() {
        assert_exact(
            DialogDataComponentValueState::BlockState {
                properties: vec![super::super::DialogStringMapEntryState {
                    key: "a\"".to_string(),
                    value: "line\n".to_string(),
                }],
            },
            r#"{"a\"":"line\n"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Recipes {
                resource_ids: vec!["minecraft:first".to_string(), "example:second".to_string()],
            },
            r#"["minecraft:first","example:second"]"#,
        );
        assert_exact(
            DialogDataComponentValueState::TooltipDisplay {
                hide_tooltip: true,
                hidden_components: vec![super::super::DialogDataComponentReferenceState {
                    type_id: 1,
                    resource_id: "minecraft:max_stack_size".to_string(),
                }],
            },
            r#"{"hide_tooltip":true,"hidden_components":["minecraft:max_stack_size"]}"#,
        );
        assert_exact(
            DialogDataComponentValueState::ContainerLoot {
                loot_table: "minecraft:chests/test".to_string(),
                seed: 0,
            },
            r#"{"loot_table":"minecraft:chests/test"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::CustomModelData {
                floats_bits: vec![0.5f32.to_bits()],
                flags: vec![true, false],
                strings: vec!["quote\"\u{2028}😀".to_string()],
                colors: vec![0x12_34_56],
            },
            r#"{"floats":[0.5],"flags":[true,false],"strings":["quote\"\u2028😀"],"colors":[1193046]}"#,
        );
        let non_finite_patch = DialogDataComponentPatchState {
            entries: vec![super::super::super::DialogDataComponentPatchEntryState {
                type_id: 0,
                resource_id: "minecraft:custom_model_data".to_string(),
                removed: false,
                value: None,
                value_data: Some(DialogDataComponentValueState::CustomModelData {
                    floats_bits: vec![f32::NAN.to_bits()],
                    flags: Vec::new(),
                    strings: Vec::new(),
                    colors: Vec::new(),
                }),
            }],
        };
        assert!(
            canonical_data_component_patch_json_utf16_len(&non_finite_patch)
                .unwrap_err()
                .contains("non-finite")
        );
    }

    #[test]
    fn canonicalizes_firework_and_lodestone_nested_records() {
        let explosion = FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Star,
            colors: vec![0x12_34_56],
            fade_colors: Vec::new(),
            has_trail: true,
            has_twinkle: false,
        };
        assert_exact(
            DialogDataComponentValueState::FireworkExplosion {
                explosion: explosion.clone(),
            },
            r#"{"shape":"star","colors":[1193046],"has_trail":true}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Fireworks {
                flight_duration: 0,
                explosions: Vec::new(),
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::Fireworks {
                flight_duration: 2,
                explosions: vec![explosion],
            },
            r#"{"flight_duration":2,"explosions":[{"shape":"star","colors":[1193046],"has_trail":true}]}"#,
        );
        assert_exact(
            DialogDataComponentValueState::LodestoneTracker {
                target: Some(LodestoneTargetSummary {
                    dimension: "minecraft:overworld".to_string(),
                    pos: BlockPos { x: 1, y: -2, z: 3 },
                }),
                tracked: false,
            },
            r#"{"target":{"dimension":"minecraft:overworld","pos":[1,-2,3]},"tracked":false}"#,
        );
    }

    #[test]
    fn canonicalizes_profile_through_the_protocol_owner() {
        assert_exact(
            DialogDataComponentValueState::Profile {
                profile: ResolvableProfileSummary {
                    kind: ResolvableProfileKindSummary::Partial,
                    uuid: None,
                    name: Some("Alex".to_string()),
                    properties: Vec::new(),
                    profile_textures: None,
                    skin_patch: PlayerSkinPatchSummary::default(),
                },
            },
            r#"{"name":"Alex"}"#,
        );
    }

    #[test]
    fn canonicalizes_map_decoration_maps_and_java_numbers() {
        assert_exact(
            DialogDataComponentValueState::MapDecorations {
                decorations: vec![DialogMapDecorationState {
                    id: "home\"".to_string(),
                    decoration_type: DialogRegistryHolderReferenceState {
                        holder_id: 0,
                        resource_id: "minecraft:mansion".to_string(),
                    },
                    x_bits: (-120.5f64).to_bits(),
                    z_bits: 64.0f64.to_bits(),
                    rotation_bits: 180.0f32.to_bits(),
                }],
            },
            r#"{"home\"":{"type":"minecraft:mansion","x":-120.5,"z":64.0,"rotation":180.0}}"#,
        );
    }

    #[test]
    fn canonicalizes_stew_and_pot_holder_collections() {
        let holder = |holder_id, resource_id: &str| DialogRegistryHolderReferenceState {
            holder_id,
            resource_id: resource_id.to_string(),
        };
        assert_exact(
            DialogDataComponentValueState::SuspiciousStewEffects {
                effects: vec![
                    super::super::holder_set::DialogSuspiciousStewEffectState {
                        effect: holder(0, "minecraft:speed"),
                        duration: 160,
                    },
                    super::super::holder_set::DialogSuspiciousStewEffectState {
                        effect: holder(18, "minecraft:poison"),
                        duration: -20,
                    },
                ],
            },
            r#"[{"id":"minecraft:speed"},{"id":"minecraft:poison","duration":-20}]"#,
        );
        assert_exact(
            DialogDataComponentValueState::PotDecorations {
                decorations: super::super::holder_set::DialogPotDecorationsState {
                    back: None,
                    left: Some(holder(0, "minecraft:angler_pottery_sherd")),
                    right: None,
                    front: None,
                },
            },
            r#"["minecraft:brick","minecraft:angler_pottery_sherd","minecraft:brick","minecraft:brick"]"#,
        );
    }

    #[test]
    fn canonicalizes_registry_fixed_named_holders() {
        let holder = |resource_id: &str| DialogRegistryHolderReferenceState {
            holder_id: 1,
            resource_id: resource_id.to_string(),
        };
        for (value, expected) in [
            (
                DialogDataComponentValueState::RegistryVariant {
                    registry: "minecraft:wolf_variant".to_string(),
                    holder: holder("example:selected"),
                },
                r#""example:selected""#,
            ),
            (
                DialogDataComponentValueState::JukeboxPlayable {
                    song: holder("minecraft:cat"),
                },
                r#""minecraft:cat""#,
            ),
            (
                DialogDataComponentValueState::PaintingVariant {
                    variant: holder("minecraft:kebab"),
                },
                r#""minecraft:kebab""#,
            ),
            (
                DialogDataComponentValueState::DamageType {
                    damage_type: holder("minecraft:generic_kill"),
                },
                r#""minecraft:generic_kill""#,
            ),
        ] {
            assert_exact(value, expected);
        }
    }

    #[test]
    fn canonicalizes_registry_holder_sets_in_compact_and_tag_forms() {
        let entry = |resource_id: &str| DialogRegistryHolderReferenceState {
            holder_id: 0,
            resource_id: resource_id.to_string(),
        };
        let direct =
            |entries: Vec<DialogRegistryHolderReferenceState>| DialogRegistryHolderSetState {
                holder_set_kind: "direct".to_string(),
                tag: None,
                entries,
            };
        assert_exact(
            DialogDataComponentValueState::Repairable {
                items: DialogRegistryHolderSetState {
                    holder_set_kind: "named".to_string(),
                    tag: Some("minecraft:planks".to_string()),
                    entries: Vec::new(),
                },
            },
            r##"{"items":"#minecraft:planks"}"##,
        );
        assert_exact(
            DialogDataComponentValueState::ProvidesBannerPatterns {
                patterns: direct(vec![entry("minecraft:base")]),
            },
            r#""minecraft:base""#,
        );
        assert_exact(
            DialogDataComponentValueState::DamageResistant {
                types: direct(vec![entry("minecraft:generic"), entry("minecraft:fall")]),
            },
            r#"{"types":["minecraft:generic","minecraft:fall"]}"#,
        );
        assert_exact(
            DialogDataComponentValueState::ProvidesBannerPatterns {
                patterns: direct(Vec::new()),
            },
            "[]",
        );
    }

    #[test]
    fn canonicalizes_registry_file_sound_event_holders() {
        for (sound, expected) in [
            (
                DialogSoundEventHolderState::Named {
                    registry_id: 0,
                    resource_id: "minecraft:entity.item.break".to_string(),
                },
                r#""minecraft:entity.item.break""#,
            ),
            (
                DialogSoundEventHolderState::Direct {
                    sound_id: "example:glass".to_string(),
                    fixed_range_bits: None,
                },
                r#"{"sound_id":"example:glass"}"#,
            ),
            (
                DialogSoundEventHolderState::Direct {
                    sound_id: "example:glass".to_string(),
                    fixed_range_bits: Some(12.5f32.to_bits()),
                },
                r#"{"sound_id":"example:glass","range":12.5}"#,
            ),
        ] {
            assert_exact(
                DialogDataComponentValueState::BreakSound { sound },
                expected,
            );
        }
    }

    #[test]
    fn canonicalizes_piercing_and_kinetic_weapon_records() {
        let named_sound = || DialogSoundEventHolderState::Named {
            registry_id: 0,
            resource_id: "minecraft:entity.item.break".to_string(),
        };
        let direct_sound =
            |sound_id: &str, range: Option<f32>| DialogSoundEventHolderState::Direct {
                sound_id: sound_id.to_string(),
                fixed_range_bits: range.map(f32::to_bits),
            };
        assert_exact(
            DialogDataComponentValueState::PiercingWeapon {
                weapon: DialogPiercingWeaponState {
                    deals_knockback: true,
                    dismounts: false,
                    sound: None,
                    hit_sound: None,
                },
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::PiercingWeapon {
                weapon: DialogPiercingWeaponState {
                    deals_knockback: false,
                    dismounts: true,
                    sound: Some(named_sound()),
                    hit_sound: Some(direct_sound("example:hit", Some(8.0))),
                },
            },
            r#"{"deals_knockback":false,"dismounts":true,"sound":"minecraft:entity.item.break","hit_sound":{"sound_id":"example:hit","range":8.0}}"#,
        );
        assert_exact(
            DialogDataComponentValueState::KineticWeapon {
                weapon: DialogKineticWeaponState {
                    contact_cooldown_ticks: 10,
                    delay_ticks: 0,
                    dismount_conditions: None,
                    knockback_conditions: None,
                    damage_conditions: None,
                    forward_movement_bits: 0.0f32.to_bits(),
                    damage_multiplier_bits: 1.0f32.to_bits(),
                    sound: None,
                    hit_sound: None,
                },
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::KineticWeapon {
                weapon: DialogKineticWeaponState {
                    contact_cooldown_ticks: 5,
                    delay_ticks: 2,
                    dismount_conditions: None,
                    knockback_conditions: None,
                    damage_conditions: Some(DialogKineticWeaponConditionState {
                        max_duration_ticks: 20,
                        min_speed_bits: 0.0f32.to_bits(),
                        min_relative_speed_bits: 2.0f32.to_bits(),
                    }),
                    forward_movement_bits: 0.25f32.to_bits(),
                    damage_multiplier_bits: 2.0f32.to_bits(),
                    sound: Some(named_sound()),
                    hit_sound: Some(direct_sound("example:hit", Some(8.0))),
                },
            },
            r#"{"contact_cooldown_ticks":5,"delay_ticks":2,"damage_conditions":{"max_duration_ticks":20,"min_relative_speed":2.0},"forward_movement":0.25,"damage_multiplier":2.0,"sound":"minecraft:entity.item.break","hit_sound":{"sound_id":"example:hit","range":8.0}}"#,
        );
    }

    #[test]
    fn canonicalizes_named_and_direct_banner_pattern_layers() {
        assert_exact(
            DialogDataComponentValueState::BannerPatterns {
                layers: vec![
                    super::super::holder_set::DialogBannerPatternLayerState {
                        pattern: DialogBannerPatternHolderState::Named {
                            registry_id: 1,
                            resource_id: "minecraft:flower".to_string(),
                        },
                        color_id: 14,
                        color: "red".to_string(),
                    },
                    super::super::holder_set::DialogBannerPatternLayerState {
                        pattern: DialogBannerPatternHolderState::Direct {
                            pattern: super::super::holder_set::DialogDirectBannerPatternState {
                                asset_id: "example:spiral".to_string(),
                                translation_key: "block.example.spiral".to_string(),
                            },
                        },
                        color_id: 11,
                        color: "blue".to_string(),
                    },
                ],
            },
            r#"[{"pattern":"minecraft:flower","color":"red"},{"pattern":{"asset_id":"example:spiral","translation_key":"block.example.spiral"},"color":"blue"}]"#,
        );
    }

    #[test]
    fn canonicalizes_typed_nbt_and_bee_occupants() {
        let compound = |entries: Vec<(&str, NbtSummaryValue)>| {
            NbtSummaryValue::Compound(
                entries
                    .into_iter()
                    .map(|(name, value)| NbtSummaryEntry {
                        name: name.to_string(),
                        value,
                    })
                    .collect(),
            )
        };
        let custom_data = compound(vec![
            ("owner", NbtSummaryValue::String("first".to_string())),
            ("owner", NbtSummaryValue::String("Alex".to_string())),
            (
                "flags",
                NbtSummaryValue::List(vec![NbtSummaryValue::Byte(1), NbtSummaryValue::Int(2)]),
            ),
        ]);
        assert_exact(
            DialogDataComponentValueState::CustomData {
                data: custom_data.clone(),
            },
            r#"{"owner":"Alex","flags":[1,2]}"#,
        );
        assert_exact(
            DialogDataComponentValueState::BucketEntityData { data: custom_data },
            r#"{"owner":"Alex","flags":[1,2]}"#,
        );

        let entity_data = compound(vec![("Health", NbtSummaryValue::Float(12.0f32.to_bits()))]);
        let entity_type = DialogRegistryHolderReferenceState {
            holder_id: 0,
            resource_id: "minecraft:zombie".to_string(),
        };
        assert_exact(
            DialogDataComponentValueState::EntityData {
                entity: super::super::typed_data::DialogTypedEntityDataState {
                    entity_type: entity_type.clone(),
                    data: entity_data.clone(),
                },
            },
            r#"{"Health":12.0,"id":"minecraft:zombie"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::BlockEntityData {
                block_entity: super::super::typed_data::DialogTypedBlockEntityDataState {
                    block_entity_type: DialogRegistryHolderReferenceState {
                        holder_id: 0,
                        resource_id: "minecraft:mob_spawner".to_string(),
                    },
                    data: compound(vec![("Delay", NbtSummaryValue::Short(20))]),
                },
            },
            r#"{"Delay":20,"id":"minecraft:mob_spawner"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Bees {
                occupants: vec![super::super::bees::DialogBeeOccupantState {
                    entity_type,
                    entity_data,
                    ticks_in_hive: 42,
                    min_ticks_in_hive: -5,
                }],
            },
            r#"[{"entity_data":{"Health":12.0,"id":"minecraft:zombie"},"ticks_in_hive":42,"min_ticks_in_hive":-5}]"#,
        );

        let non_finite_patch = DialogDataComponentPatchState {
            entries: vec![super::super::super::DialogDataComponentPatchEntryState {
                type_id: 0,
                resource_id: "minecraft:custom_data".to_string(),
                removed: false,
                value: None,
                value_data: Some(DialogDataComponentValueState::CustomData {
                    data: compound(vec![(
                        "value",
                        NbtSummaryValue::Double(f64::INFINITY.to_bits()),
                    )]),
                }),
            }],
        };
        assert!(
            canonical_data_component_patch_json_utf16_len(&non_finite_patch)
                .unwrap_err()
                .contains("non-finite")
        );
    }

    #[test]
    fn canonicalizes_debug_stick_dispatched_map() {
        assert_exact(
            DialogDataComponentValueState::DebugStickState {
                properties: vec![
                    super::super::debug_stick::DialogDebugStickPropertyState {
                        block: DialogRegistryHolderReferenceState {
                            holder_id: 0,
                            resource_id: "minecraft:oak_log".to_string(),
                        },
                        property: "axis".to_string(),
                    },
                    super::super::debug_stick::DialogDebugStickPropertyState {
                        block: DialogRegistryHolderReferenceState {
                            holder_id: 1,
                            resource_id: "minecraft:lever".to_string(),
                        },
                        property: "powered".to_string(),
                    },
                ],
            },
            r#"{"minecraft:oak_log":"axis","minecraft:lever":"powered"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::DebugStickState {
                properties: Vec::new(),
            },
            "{}",
        );
    }

    #[test]
    fn canonicalizes_enchantment_holder_maps() {
        let entries = vec![
            super::super::enchantments::DialogEnchantmentEntryState {
                enchantment: DialogRegistryHolderReferenceState {
                    holder_id: 1,
                    resource_id: "minecraft:sharpness".to_string(),
                },
                level: 5,
            },
            super::super::enchantments::DialogEnchantmentEntryState {
                enchantment: DialogRegistryHolderReferenceState {
                    holder_id: 2,
                    resource_id: "example:last".to_string(),
                },
                level: 255,
            },
        ];
        assert_exact(
            DialogDataComponentValueState::Enchantments {
                entries: entries.clone(),
            },
            r#"{"minecraft:sharpness":5,"example:last":255}"#,
        );
        assert_exact(
            DialogDataComponentValueState::StoredEnchantments { entries },
            r#"{"minecraft:sharpness":5,"example:last":255}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Enchantments {
                entries: Vec::new(),
            },
            "{}",
        );
    }

    #[test]
    fn canonicalizes_writable_book_filterable_pages_as_full_records() {
        assert_exact(
            DialogDataComponentValueState::WritableBookContent { pages: Vec::new() },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::WritableBookContent {
                pages: vec![
                    super::super::writable_book::DialogFilterableStringState {
                        raw: "compact\"".to_string(),
                        filtered: None,
                    },
                    super::super::writable_book::DialogFilterableStringState {
                        raw: "raw".to_string(),
                        filtered: Some("filtered\n".to_string()),
                    },
                ],
            },
            r#"{"pages":[{"raw":"compact\""},{"raw":"raw","filtered":"filtered\n"}]}"#,
        );
    }

    #[test]
    fn canonicalizes_written_book_filterables_and_defaults() {
        let text = |value: &str| super::super::DialogTextState {
            text: value.to_string(),
            runs: Vec::new(),
            component: NbtSummaryValue::String(value.to_string()),
        };
        assert_exact(
            DialogDataComponentValueState::WrittenBookContent {
                content: DialogWrittenBookContentState {
                    title: super::super::writable_book::DialogFilterableStringState {
                        raw: "Title".to_string(),
                        filtered: None,
                    },
                    author: "Author".to_string(),
                    generation: 0,
                    pages: Vec::new(),
                    resolved: false,
                },
            },
            r#"{"title":{"raw":"Title"},"author":"Author"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::WrittenBookContent {
                content: DialogWrittenBookContentState {
                    title: super::super::writable_book::DialogFilterableStringState {
                        raw: "Title".to_string(),
                        filtered: Some("Safe".to_string()),
                    },
                    author: "Author".to_string(),
                    generation: 2,
                    pages: vec![super::super::written_book::DialogFilterableComponentState {
                        raw: text("Raw"),
                        filtered: Some(text("Filtered")),
                    }],
                    resolved: true,
                },
            },
            r#"{"title":{"raw":"Title","filtered":"Safe"},"author":"Author","generation":2,"pages":[{"raw":"Raw","filtered":"Filtered"}],"resolved":true}"#,
        );

        let owner_sensitive = DialogDataComponentValueState::WrittenBookContent {
            content: DialogWrittenBookContentState {
                title: super::super::writable_book::DialogFilterableStringState {
                    raw: "Title".to_string(),
                    filtered: None,
                },
                author: "Author".to_string(),
                generation: 0,
                pages: vec![super::super::written_book::DialogFilterableComponentState {
                    raw: super::super::DialogTextState {
                        text: "Alex".to_string(),
                        runs: Vec::new(),
                        component: NbtSummaryValue::Compound(vec![
                            NbtSummaryEntry {
                                name: "object".to_string(),
                                value: NbtSummaryValue::String("player".to_string()),
                            },
                            NbtSummaryEntry {
                                name: "player".to_string(),
                                value: NbtSummaryValue::String("Alex".to_string()),
                            },
                        ]),
                    },
                    filtered: None,
                }],
                resolved: false,
            },
        };
        assert_eq!(
            canonical_component_value_json_utf16_len(&owner_sensitive),
            None
        );
    }

    #[test]
    fn canonicalizes_tool_rules_and_defaults() {
        assert_exact(
            DialogDataComponentValueState::Tool {
                tool: super::super::tool::DialogToolState {
                    rules: Vec::new(),
                    default_mining_speed_bits: 1.0f32.to_bits(),
                    damage_per_block: 1,
                    can_destroy_blocks_in_creative: true,
                },
            },
            r#"{"rules":[]}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Tool {
                tool: super::super::tool::DialogToolState {
                    rules: vec![super::super::tool::DialogToolRuleState {
                        blocks: DialogRegistryHolderSetState {
                            holder_set_kind: "named".to_string(),
                            tag: Some("example:mineable".to_string()),
                            entries: Vec::new(),
                        },
                        speed_bits: Some(4.0f32.to_bits()),
                        correct_for_drops: Some(true),
                    }],
                    default_mining_speed_bits: 0.5f32.to_bits(),
                    damage_per_block: 0,
                    can_destroy_blocks_in_creative: false,
                },
            },
            r##"{"rules":[{"blocks":"#example:mineable","speed":4.0,"correct_for_drops":true}],"default_mining_speed":0.5,"damage_per_block":0,"can_destroy_blocks_in_creative":false}"##,
        );

        let non_finite_patch = DialogDataComponentPatchState {
            entries: vec![super::super::super::DialogDataComponentPatchEntryState {
                type_id: 0,
                resource_id: "minecraft:tool".to_string(),
                removed: false,
                value: None,
                value_data: Some(DialogDataComponentValueState::Tool {
                    tool: super::super::tool::DialogToolState {
                        rules: Vec::new(),
                        default_mining_speed_bits: f32::NAN.to_bits(),
                        damage_per_block: 1,
                        can_destroy_blocks_in_creative: true,
                    },
                }),
            }],
        };
        assert!(
            canonical_data_component_patch_json_utf16_len(&non_finite_patch)
                .unwrap_err()
                .contains("non-finite")
        );
    }

    #[test]
    fn canonicalizes_equippable_holders_and_defaults() {
        let named_sound = |resource_id: &str| DialogSoundEventHolderState::Named {
            registry_id: 0,
            resource_id: resource_id.to_string(),
        };
        assert_exact(
            DialogDataComponentValueState::Equippable {
                equippable: super::super::equippable::DialogEquippableState {
                    slot_id: 4,
                    slot: "head".to_string(),
                    equip_sound: named_sound("minecraft:item.armor.equip_generic"),
                    asset_id: None,
                    camera_overlay: None,
                    allowed_entities: None,
                    dispensable: true,
                    swappable: true,
                    damage_on_hurt: true,
                    equip_on_interact: false,
                    can_be_sheared: false,
                    shearing_sound: named_sound("minecraft:item.shears.snip"),
                },
            },
            r#"{"slot":"head"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::Equippable {
                equippable: super::super::equippable::DialogEquippableState {
                    slot_id: 6,
                    slot: "body".to_string(),
                    equip_sound: DialogSoundEventHolderState::Direct {
                        sound_id: "minecraft:item.armor.equip_generic".to_string(),
                        fixed_range_bits: Some(6.5f32.to_bits()),
                    },
                    asset_id: Some("example:llama_carpet".to_string()),
                    camera_overlay: Some("minecraft:overlay/powder_snow".to_string()),
                    allowed_entities: Some(DialogRegistryHolderSetState {
                        holder_set_kind: "named".to_string(),
                        tag: Some("example:llamas".to_string()),
                        entries: Vec::new(),
                    }),
                    dispensable: false,
                    swappable: false,
                    damage_on_hurt: false,
                    equip_on_interact: true,
                    can_be_sheared: true,
                    shearing_sound: named_sound("minecraft:item.shears.snip"),
                },
            },
            r##"{"slot":"body","equip_sound":{"sound_id":"minecraft:item.armor.equip_generic","range":6.5},"asset_id":"example:llama_carpet","camera_overlay":"minecraft:overlay/powder_snow","allowed_entities":"#example:llamas","dispensable":false,"swappable":false,"damage_on_hurt":false,"equip_on_interact":true,"can_be_sheared":true}"##,
        );
    }

    #[test]
    fn canonicalizes_consumable_and_death_protection_effects() {
        let effects = vec![
            DialogConsumeEffectState::ApplyEffects {
                effects: vec![DialogMobEffectInstanceState {
                    effect: DialogRegistryHolderReferenceState {
                        holder_id: 1,
                        resource_id: "minecraft:speed".to_string(),
                    },
                    details: DialogMobEffectDetailsState {
                        amplifier: 2,
                        duration: 40,
                        ambient: false,
                        show_particles: true,
                        show_icon: true,
                        hidden_effect: Some(Box::new(DialogMobEffectDetailsState {
                            amplifier: 0,
                            duration: 20,
                            ambient: true,
                            show_particles: false,
                            show_icon: false,
                            hidden_effect: None,
                        })),
                    },
                }],
                probability_bits: 0.25f32.to_bits(),
            },
            DialogConsumeEffectState::RemoveEffects {
                effects: DialogRegistryHolderSetState {
                    holder_set_kind: "named".to_string(),
                    tag: Some("example:negative".to_string()),
                    entries: Vec::new(),
                },
            },
            DialogConsumeEffectState::ClearAllEffects,
            DialogConsumeEffectState::TeleportRandomly {
                diameter_bits: 8.0f32.to_bits(),
            },
            DialogConsumeEffectState::PlaySound {
                sound: DialogSoundEventHolderState::Named {
                    registry_id: 0,
                    resource_id: "minecraft:entity.generic.eat".to_string(),
                },
            },
        ];
        assert_exact(
            DialogDataComponentValueState::Consumable {
                consumable: DialogConsumableState {
                    consume_seconds_bits: 1.6f32.to_bits(),
                    animation: ItemUseAnimationSummary::Eat,
                    sound: DialogSoundEventHolderState::Named {
                        registry_id: 0,
                        resource_id: "minecraft:entity.generic.eat".to_string(),
                    },
                    has_consume_particles: true,
                    on_consume_effects: Vec::new(),
                },
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::Consumable {
                consumable: DialogConsumableState {
                    consume_seconds_bits: 0.5f32.to_bits(),
                    animation: ItemUseAnimationSummary::Drink,
                    sound: DialogSoundEventHolderState::Direct {
                        sound_id: "example:consume".to_string(),
                        fixed_range_bits: Some(4.0f32.to_bits()),
                    },
                    has_consume_particles: false,
                    on_consume_effects: effects.clone(),
                },
            },
            r##"{"consume_seconds":0.5,"animation":"drink","sound":{"sound_id":"example:consume","range":4.0},"has_consume_particles":false,"on_consume_effects":[{"type":"minecraft:apply_effects","effects":[{"id":"minecraft:speed","amplifier":2,"duration":40,"show_icon":true,"hidden_effect":{"duration":20,"ambient":true,"show_particles":false,"show_icon":false}}],"probability":0.25},{"type":"minecraft:remove_effects","effects":"#example:negative"},{"type":"minecraft:clear_all_effects"},{"type":"minecraft:teleport_randomly","diameter":8.0},{"type":"minecraft:play_sound","sound":"minecraft:entity.generic.eat"}]}"##,
        );
        assert_exact(
            DialogDataComponentValueState::DeathProtection {
                effects: Vec::new(),
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::DeathProtection { effects },
            r##"{"death_effects":[{"type":"minecraft:apply_effects","effects":[{"id":"minecraft:speed","amplifier":2,"duration":40,"show_icon":true,"hidden_effect":{"duration":20,"ambient":true,"show_particles":false,"show_icon":false}}],"probability":0.25},{"type":"minecraft:remove_effects","effects":"#example:negative"},{"type":"minecraft:clear_all_effects"},{"type":"minecraft:teleport_randomly","diameter":8.0},{"type":"minecraft:play_sound","sound":"minecraft:entity.generic.eat"}]}"##,
        );
    }

    #[test]
    fn canonicalizes_potion_contents_full_alternative() {
        assert_exact(
            DialogDataComponentValueState::PotionContents {
                contents: DialogPotionContentsState {
                    potion: None,
                    custom_color: None,
                    custom_effects: Vec::new(),
                    custom_name: None,
                },
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::PotionContents {
                contents: DialogPotionContentsState {
                    potion: Some(DialogRegistryHolderReferenceState {
                        holder_id: 0,
                        resource_id: "minecraft:water".to_string(),
                    }),
                    custom_color: None,
                    custom_effects: Vec::new(),
                    custom_name: None,
                },
            },
            r#"{"potion":"minecraft:water"}"#,
        );
        assert_exact(
            DialogDataComponentValueState::PotionContents {
                contents: DialogPotionContentsState {
                    potion: Some(DialogRegistryHolderReferenceState {
                        holder_id: 15,
                        resource_id: "minecraft:strong_swiftness".to_string(),
                    }),
                    custom_color: Some(0x12_34_56),
                    custom_effects: vec![DialogMobEffectInstanceState {
                        effect: DialogRegistryHolderReferenceState {
                            holder_id: 1,
                            resource_id: "minecraft:speed".to_string(),
                        },
                        details: DialogMobEffectDetailsState {
                            amplifier: 2,
                            duration: 40,
                            ambient: false,
                            show_particles: false,
                            show_icon: true,
                            hidden_effect: None,
                        },
                    }],
                    custom_name: Some("example.potion".to_string()),
                },
            },
            r#"{"potion":"minecraft:strong_swiftness","custom_color":1193046,"custom_effects":[{"id":"minecraft:speed","amplifier":2,"duration":40,"show_particles":false,"show_icon":true}],"custom_name":"example.potion"}"#,
        );
    }

    #[test]
    fn canonicalizes_blocks_attacks_nested_records_and_defaults() {
        assert_exact(
            DialogDataComponentValueState::BlocksAttacks {
                blocks_attacks: DialogBlocksAttacksState {
                    block_delay_seconds_bits: 0.0f32.to_bits(),
                    disable_cooldown_scale_bits: 1.0f32.to_bits(),
                    damage_reductions: vec![DialogBlocksAttacksDamageReductionState {
                        horizontal_blocking_angle_bits: 90.0f32.to_bits(),
                        damage_types: None,
                        base_bits: 0.0f32.to_bits(),
                        factor_bits: 1.0f32.to_bits(),
                    }],
                    item_damage: DialogBlocksAttacksItemDamageState {
                        threshold_bits: 1.0f32.to_bits(),
                        base_bits: 0.0f32.to_bits(),
                        factor_bits: 1.0f32.to_bits(),
                    },
                    bypassed_by: None,
                    block_sound: None,
                    disabled_sound: None,
                },
            },
            "{}",
        );
        assert_exact(
            DialogDataComponentValueState::BlocksAttacks {
                blocks_attacks: DialogBlocksAttacksState {
                    block_delay_seconds_bits: 0.25f32.to_bits(),
                    disable_cooldown_scale_bits: 2.0f32.to_bits(),
                    damage_reductions: vec![DialogBlocksAttacksDamageReductionState {
                        horizontal_blocking_angle_bits: 45.0f32.to_bits(),
                        damage_types: Some(DialogRegistryHolderSetState {
                            holder_set_kind: "named".to_string(),
                            tag: Some("example:fire".to_string()),
                            entries: Vec::new(),
                        }),
                        base_bits: (-2.0f32).to_bits(),
                        factor_bits: 0.5f32.to_bits(),
                    }],
                    item_damage: DialogBlocksAttacksItemDamageState {
                        threshold_bits: 3.0f32.to_bits(),
                        base_bits: 1.0f32.to_bits(),
                        factor_bits: (-1.0f32).to_bits(),
                    },
                    bypassed_by: Some(DialogRegistryHolderSetState {
                        holder_set_kind: "direct".to_string(),
                        tag: None,
                        entries: vec![DialogRegistryHolderReferenceState {
                            holder_id: 1,
                            resource_id: "minecraft:explosion".to_string(),
                        }],
                    }),
                    block_sound: Some(DialogSoundEventHolderState::Named {
                        registry_id: 0,
                        resource_id: "minecraft:item.shield.block".to_string(),
                    }),
                    disabled_sound: Some(DialogSoundEventHolderState::Direct {
                        sound_id: "example:disable".to_string(),
                        fixed_range_bits: Some(8.0f32.to_bits()),
                    }),
                },
            },
            r##"{"block_delay_seconds":0.25,"disable_cooldown_scale":2.0,"damage_reductions":[{"horizontal_blocking_angle":45.0,"type":"#example:fire","base":-2.0,"factor":0.5}],"item_damage":{"threshold":3.0,"base":1.0,"factor":-1.0},"bypassed_by":"minecraft:explosion","block_sound":"minecraft:item.shield.block","disabled_sound":{"sound_id":"example:disable","range":8.0}}"##,
        );
    }

    fn assert_exact(value: DialogDataComponentValueState, expected: &str) {
        assert_eq!(
            canonical_component_value_json_utf16_len(&value),
            Some(expected.encode_utf16().count())
        );
    }
}
