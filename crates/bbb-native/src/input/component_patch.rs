use super::*;

type PersistentSnbtEncoder = fn(
    i32,
    &bbb_protocol::packets::DataComponentPatchSummary,
    Option<&NativeItemRuntime>,
) -> Option<String>;

#[derive(Clone, Copy)]
struct PersistentSnbtEncoderDescriptor {
    type_id: i32,
    encode: PersistentSnbtEncoder,
}

const fn encoder(type_id: i32, encode: PersistentSnbtEncoder) -> PersistentSnbtEncoderDescriptor {
    PersistentSnbtEncoderDescriptor { type_id, encode }
}

static PERSISTENT_SNBT_ENCODERS: &[PersistentSnbtEncoderDescriptor] = &[
    encoder(0, encode_custom_data),
    encoder(1, encode_max_stack_size),
    encoder(2, encode_max_damage),
    encoder(3, encode_damage),
    encoder(4, encode_unit),
    encoder(5, encode_use_effects),
    encoder(7, encode_minimum_attack_charge),
    encoder(10, encode_item_model),
    encoder(12, encode_rarity),
    encoder(13, encode_enchantments),
    encoder(17, encode_custom_model_data),
    encoder(18, encode_tooltip_display),
    encoder(19, encode_repair_cost),
    encoder(21, encode_enchantment_glint_override),
    encoder(22, encode_unit),
    encoder(23, encode_food),
    encoder(24, encode_consumable),
    encoder(25, encode_use_remainder),
    encoder(26, encode_use_cooldown),
    encoder(27, encode_damage_resistant),
    encoder(29, encode_weapon),
    encoder(30, encode_attack_range),
    encoder(31, encode_enchantable),
    encoder(33, encode_repairable),
    encoder(34, encode_unit),
    encoder(35, encode_tooltip_style),
    encoder(36, encode_death_protection),
    encoder(38, encode_piercing_weapon),
    encoder(39, encode_kinetic_weapon),
    encoder(40, encode_swing_animation),
    encoder(42, encode_enchantments),
    encoder(43, encode_dye),
    encoder(44, encode_dyed_color),
    encoder(45, encode_map_color),
    encoder(46, encode_map_id),
    encoder(47, encode_codec_backed_nbt),
    encoder(49, encode_charged_projectiles),
    encoder(50, encode_bundle_contents),
    encoder(51, encode_potion_contents),
    encoder(52, encode_potion_duration_scale),
    encoder(53, encode_suspicious_stew_effects),
    encoder(54, encode_writable_book_content),
    encoder(57, encode_codec_backed_nbt),
    encoder(58, encode_entity_data),
    encoder(59, encode_codec_backed_nbt),
    encoder(60, encode_block_entity_data),
    encoder(61, encode_instrument),
    encoder(63, encode_ominous_bottle_amplifier),
    encoder(65, encode_provides_banner_patterns),
    encoder(66, encode_codec_backed_nbt),
    encoder(67, encode_lodestone_tracker),
    encoder(68, encode_firework_explosion),
    encoder(69, encode_fireworks),
    encoder(70, encode_profile),
    encoder(71, encode_note_block_sound),
    encoder(72, encode_banner_patterns),
    encoder(73, encode_base_color),
    encoder(74, encode_pot_decorations),
    encoder(75, encode_container),
    encoder(76, encode_block_state),
    encoder(77, encode_bees),
    encoder(78, encode_codec_backed_nbt),
    encoder(79, encode_codec_backed_nbt),
    encoder(80, encode_break_sound),
    encoder(84, encode_builtin_enum),
    encoder(85, encode_builtin_enum),
    encoder(86, encode_builtin_enum),
    encoder(87, encode_builtin_enum),
    encoder(88, encode_tropical_fish_pattern),
    encoder(89, encode_tropical_fish_base_color),
    encoder(90, encode_tropical_fish_pattern_color),
    encoder(91, encode_builtin_enum),
    encoder(92, encode_builtin_enum),
    encoder(101, encode_builtin_enum),
    encoder(103, encode_builtin_enum),
    encoder(104, encode_builtin_enum),
    encoder(107, encode_builtin_enum),
    encoder(108, encode_builtin_enum),
    encoder(109, encode_builtin_enum),
];

fn persistent_snbt_encoder(type_id: i32) -> Option<&'static PersistentSnbtEncoderDescriptor> {
    PERSISTENT_SNBT_ENCODERS
        .binary_search_by_key(&type_id, |descriptor| descriptor.type_id)
        .ok()
        .map(|index| &PERSISTENT_SNBT_ENCODERS[index])
}

pub(super) fn encode_added_component_value_snbt(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let descriptor = persistent_snbt_encoder(type_id)?;
    (descriptor.encode)(type_id, patch, item_runtime)
}

fn encode_custom_data(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_nbt_summary_snbt(patch.custom_data.as_ref()?))
}

fn encode_max_stack_size(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
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
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(patch.max_damage.filter(|value| *value > 0)?.to_string())
}

fn encode_damage(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(patch.damage.filter(|value| *value >= 0)?.to_string())
}

fn encode_unit(
    _type_id: i32,
    _patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some("{}".to_string())
}

fn encode_use_effects(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_use_effects_component_snbt(patch.use_effects?)
}

fn encode_minimum_attack_charge(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_range_checked_float_component_snbt(patch.minimum_attack_charge_bits?, 0.0, Some(1.0))
}

fn encode_item_model(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(patch.item_model.as_deref()?))
}

fn encode_rarity(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(match patch.rarity? {
        ItemRaritySummary::Common => "common",
        ItemRaritySummary::Uncommon => "uncommon",
        ItemRaritySummary::Rare => "rare",
        ItemRaritySummary::Epic => "epic",
    }))
}

fn encode_enchantments(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let enchantments = match type_id {
        13 => &patch.enchantments,
        42 => &patch.stored_enchantments,
        _ => return None,
    };
    let entries = enchantments
        .iter()
        .map(|enchantment| {
            let level = (1..=255)
                .contains(&enchantment.level)
                .then_some(enchantment.level)?;
            let resource_id = vanilla_enchantment_resource_id(enchantment.holder_id)?;
            Some(format!("{}: {level}", debug_snbt_string(resource_id)))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(format!("{{{}}}", entries.join(", ")))
}

fn encode_custom_model_data(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_custom_model_data_component_snbt(patch))
}

fn encode_tooltip_display(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_tooltip_display_component_snbt(patch)
}

fn encode_repair_cost(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(patch.repair_cost.filter(|value| *value >= 0)?.to_string())
}

fn encode_enchantment_glint_override(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_bool(patch.enchantment_glint_override?).to_string())
}

fn encode_food(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_food_component_snbt(patch)
}

fn encode_consumable(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_consumable_component_snbt(patch)
}

fn encode_use_remainder(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_item_stack_template_snbt(patch.use_remainder.as_deref()?, item_runtime)
}

fn encode_use_cooldown(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_use_cooldown_component_snbt(patch)
}

fn encode_damage_resistant(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_damage_resistant_component_snbt(patch)
}

fn encode_weapon(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_weapon_component_snbt(patch)
}

fn encode_attack_range(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_attack_range_component_snbt(patch.attack_range?)
}

fn encode_enchantable(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let value = patch.enchantable_value.filter(|value| *value > 0)?;
    Some(format!("{{value: {value}}}"))
}

fn encode_repairable(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_repairable_component_snbt(patch, item_runtime)
}

fn encode_tooltip_style(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(patch.tooltip_style.as_deref()?))
}

fn encode_death_protection(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_death_protection_component_snbt(patch)
}

fn encode_piercing_weapon(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_piercing_weapon_component_snbt(patch.piercing_weapon.as_ref()?)
}

fn encode_kinetic_weapon(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_kinetic_weapon_component_snbt(patch.kinetic_weapon.as_ref()?)
}

fn encode_swing_animation(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_swing_animation_component_snbt(patch.swing_animation?)
}

fn encode_dye(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(debug_dye_color_name(patch.dye_color_id?)))
}

fn encode_dyed_color(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(patch.dyed_color?.to_string())
}

fn encode_map_color(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(patch.map_color?.to_string())
}

fn encode_map_id(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(patch.map_id?.to_string())
}

fn encode_codec_backed_nbt(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_nbt_summary_snbt(
        patch.codec_backed_nbt.get(&type_id)?,
    ))
}

fn encode_entity_data(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_entity_data_component_snbt(patch)
}

fn encode_block_entity_data(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_block_entity_data_component_snbt(patch)
}

fn encode_instrument(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(vanilla_instrument_resource_id(
        patch.instrument_id?,
    )?))
}

fn encode_charged_projectiles(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_item_stack_template_list_snbt(&patch.charged_projectiles_items, item_runtime)
}

fn encode_bundle_contents(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_item_stack_template_list_snbt(&patch.bundle_contents_items, item_runtime)
}

fn encode_potion_contents(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_potion_contents_component_snbt(patch)
}

fn encode_potion_duration_scale(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_range_checked_float_component_snbt(patch.potion_duration_scale_bits?, 0.0, None)
}

fn encode_suspicious_stew_effects(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let effects = patch
        .suspicious_stew_effects
        .iter()
        .map(|effect| {
            let mut fields = vec![format!(
                "id: {}",
                debug_snbt_string(vanilla_mob_effect_resource_id(effect.effect_id)?)
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
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_writable_book_content_component_snbt(patch))
}

fn encode_ominous_bottle_amplifier(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(
        patch
            .ominous_bottle_amplifier
            .filter(|value| (0..=4).contains(value))?
            .to_string(),
    )
}

fn encode_provides_banner_patterns(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_provides_banner_patterns_component_snbt(patch)
}

fn encode_lodestone_tracker(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_lodestone_tracker_component_snbt(patch))
}

fn encode_firework_explosion(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
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
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_fireworks_component_snbt(patch))
}

fn encode_profile(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_profile_component_snbt(patch.profile.as_ref()?)
}

fn encode_note_block_sound(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(patch.note_block_sound.as_deref()?))
}

fn encode_banner_patterns(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let layers = patch
        .banner_pattern_layers
        .iter()
        .map(|layer| {
            let pattern = if let Some(registry_id) = layer.registry_id {
                debug_snbt_string(&vanilla_banner_pattern_resource_id(registry_id)?)
            } else {
                format!(
                    "{{asset_id: {}, translation_key: {}}}",
                    debug_snbt_string(layer.asset_id.as_deref()?),
                    debug_snbt_string(layer.translation_key.as_deref()?)
                )
            };
            Some(format!(
                "{{pattern: {pattern}, color: {}}}",
                debug_snbt_string(debug_dye_color_name(layer.color_id))
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(format!("[{}]", layers.join(", ")))
}

fn encode_base_color(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(debug_dye_color_name(
        patch.base_color_id?,
    )))
}

fn encode_pot_decorations(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_pot_decorations_component_snbt(patch, item_runtime)
}

fn encode_container(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_item_container_contents_component_snbt(patch, item_runtime)
}

fn encode_block_state(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
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
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_bees_component_snbt(&patch.bees)
}

fn encode_break_sound(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_sound_event_component_snbt(patch.break_sound.as_ref()?)
}

fn encode_tropical_fish_pattern(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(debug_tropical_fish_pattern_name(
        patch.tropical_fish_pattern_id?,
    )))
}

fn encode_tropical_fish_base_color(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(debug_dye_color_name(
        patch.tropical_fish_base_color_id?,
    )))
}

fn encode_tropical_fish_pattern_color(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(debug_dye_color_name(
        patch.tropical_fish_pattern_color_id?,
    )))
}

fn encode_builtin_enum(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let id = *patch.builtin_enum_component_ids.get(&type_id)?;
    Some(debug_snbt_string(debug_builtin_enum_component_name(
        type_id, id,
    )?))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn encoder_registry_is_unique_sorted_and_persistent() {
        assert_eq!(PERSISTENT_SNBT_ENCODERS.len(), 79);
        let mut ids = HashSet::new();
        let mut previous = None;
        for descriptor in PERSISTENT_SNBT_ENCODERS {
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
        assert_eq!(encode_added_component_value_snbt(8, &patch, None), None);
    }

    #[test]
    fn suspicious_stew_effects_use_codec_defaults_and_omit_unknown_holders() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            suspicious_stew_effects: vec![
                bbb_protocol::packets::SuspiciousStewEffectSummary {
                    effect_id: 0,
                    duration: 160,
                },
                bbb_protocol::packets::SuspiciousStewEffectSummary {
                    effect_id: 18,
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

        patch.suspicious_stew_effects[1].effect_id = 40;
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
                holder_id: 37,
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
    fn banner_pattern_layers_encode_named_and_direct_holders() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            banner_pattern_layers: vec![
                bbb_protocol::packets::BannerPatternLayerSummary {
                    registry_id: Some(5),
                    asset_id: None,
                    translation_key: None,
                    color_id: 14,
                },
                bbb_protocol::packets::BannerPatternLayerSummary {
                    registry_id: None,
                    asset_id: Some("example:custom_banner/pattern".to_string()),
                    translation_key: Some("block.example.banner.custom".to_string()),
                    color_id: 3,
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

        patch.banner_pattern_layers[0].registry_id = Some(43);
        assert_eq!(encode_added_component_value_snbt(72, &patch, None), None);
        patch.banner_pattern_layers.remove(0);
        patch.banner_pattern_layers[0].asset_id = None;
        assert_eq!(encode_added_component_value_snbt(72, &patch, None), None);
        patch.banner_pattern_layers.clear();
        assert_eq!(
            encode_added_component_value_snbt(72, &patch, None),
            Some("[]".to_string())
        );
    }

    #[test]
    fn instrument_encoder_supports_vanilla_named_holders_only() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            instrument_id: Some(5),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(61, &patch, None),
            Some("\"minecraft:call_goat_horn\"".to_string())
        );

        patch.instrument_id = Some(8);
        assert_eq!(encode_added_component_value_snbt(61, &patch, None), None);
        patch.instrument_id = None;
        patch.instrument_description = Some("Custom horn".to_string());
        assert_eq!(encode_added_component_value_snbt(61, &patch, None), None);
    }
}
