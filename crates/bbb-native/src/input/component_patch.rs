use super::*;
use bbb_world::BlockStateRegistry;

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

const VANILLA_DAMAGE_TYPE_RESOURCE_IDS: [&str; 50] = [
    "minecraft:in_fire",
    "minecraft:campfire",
    "minecraft:lightning_bolt",
    "minecraft:on_fire",
    "minecraft:lava",
    "minecraft:hot_floor",
    "minecraft:in_wall",
    "minecraft:cramming",
    "minecraft:drown",
    "minecraft:starve",
    "minecraft:cactus",
    "minecraft:fall",
    "minecraft:ender_pearl",
    "minecraft:fly_into_wall",
    "minecraft:out_of_world",
    "minecraft:generic",
    "minecraft:magic",
    "minecraft:wither",
    "minecraft:dragon_breath",
    "minecraft:dry_out",
    "minecraft:sweet_berry_bush",
    "minecraft:freeze",
    "minecraft:stalagmite",
    "minecraft:falling_block",
    "minecraft:falling_anvil",
    "minecraft:falling_stalactite",
    "minecraft:sting",
    "minecraft:mob_attack",
    "minecraft:mob_attack_no_aggro",
    "minecraft:player_attack",
    "minecraft:spear",
    "minecraft:arrow",
    "minecraft:trident",
    "minecraft:mob_projectile",
    "minecraft:spit",
    "minecraft:fireworks",
    "minecraft:unattributed_fireball",
    "minecraft:fireball",
    "minecraft:wither_skull",
    "minecraft:thrown",
    "minecraft:indirect_magic",
    "minecraft:thorns",
    "minecraft:explosion",
    "minecraft:player_explosion",
    "minecraft:sonic_boom",
    "minecraft:bad_respawn_point",
    "minecraft:outside_border",
    "minecraft:generic_kill",
    "minecraft:wind_charge",
    "minecraft:mace_smash",
];

static PERSISTENT_SNBT_ENCODERS: &[PersistentSnbtEncoderDescriptor] = &[
    encoder(0, encode_custom_data),
    encoder(1, encode_max_stack_size),
    encoder(2, encode_max_damage),
    encoder(3, encode_damage),
    encoder(4, encode_unit),
    encoder(5, encode_use_effects),
    encoder(6, encode_component_tree),
    encoder(7, encode_minimum_attack_charge),
    encoder(8, encode_damage_type),
    encoder(9, encode_component_tree),
    encoder(10, encode_item_model),
    encoder(11, encode_lore),
    encoder(12, encode_rarity),
    encoder(13, encode_enchantments),
    encoder(16, encode_attribute_modifiers),
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
    encoder(28, encode_tool),
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
    encoder(55, encode_written_book_content),
    encoder(56, encode_armor_trim),
    encoder(57, encode_codec_backed_nbt),
    encoder(58, encode_entity_data),
    encoder(59, encode_codec_backed_nbt),
    encoder(60, encode_block_entity_data),
    encoder(61, encode_instrument),
    encoder(62, encode_provides_trim_material),
    encoder(63, encode_ominous_bottle_amplifier),
    encoder(64, encode_jukebox_playable),
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
    encoder(81, encode_named_variant),
    encoder(82, encode_named_variant),
    encoder(83, encode_named_variant),
    encoder(84, encode_builtin_enum),
    encoder(85, encode_builtin_enum),
    encoder(86, encode_builtin_enum),
    encoder(87, encode_builtin_enum),
    encoder(88, encode_tropical_fish_pattern),
    encoder(89, encode_tropical_fish_base_color),
    encoder(90, encode_tropical_fish_pattern_color),
    encoder(91, encode_builtin_enum),
    encoder(92, encode_builtin_enum),
    encoder(93, encode_named_variant),
    encoder(94, encode_named_variant),
    encoder(95, encode_named_variant),
    encoder(96, encode_named_variant),
    encoder(97, encode_named_variant),
    encoder(98, encode_named_variant),
    encoder(99, encode_named_variant),
    encoder(100, encode_named_variant),
    encoder(101, encode_builtin_enum),
    encoder(102, encode_painting_variant),
    encoder(103, encode_builtin_enum),
    encoder(104, encode_builtin_enum),
    encoder(105, encode_named_variant),
    encoder(106, encode_named_variant),
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

fn encode_component_tree(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let component = match type_id {
        6 => patch.custom_name_nbt.as_ref()?,
        9 => patch.item_name_nbt.as_ref()?,
        _ => return None,
    };
    Some(debug_nbt_summary_snbt(component))
}

fn encode_lore(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
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
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    debug_range_checked_float_component_snbt(patch.minimum_attack_charge_bits?, 0.0, Some(1.0))
}

fn encode_damage_type(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(damage_type_resource_id(
        patch.damage_type_id?,
    )?))
}

fn damage_type_resource_id(id: i32) -> Option<&'static str> {
    VANILLA_DAMAGE_TYPE_RESOURCE_IDS
        .get(usize::try_from(id).ok()?)
        .copied()
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

fn encode_attribute_modifiers(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let entries = patch
        .attribute_modifiers
        .iter()
        .map(|modifier| {
            let attribute = vanilla_attribute_resource_id(modifier.attribute_id)?;
            let operation = ["add_value", "add_multiplied_base", "add_multiplied_total"]
                .get(usize::try_from(modifier.operation_id).ok()?)?;
            let slot = [
                "any", "mainhand", "offhand", "hand", "feet", "legs", "chest", "head", "armor",
                "body", "saddle",
            ]
            .get(usize::try_from(modifier.slot_id).ok()?)?;
            let mut fields = vec![
                format!("type: {}", debug_snbt_string(attribute)),
                format!("id: {}", debug_snbt_string(&modifier.modifier_id)),
                format!(
                    "amount: {}d",
                    debug_nbt_java_double_string(f64::from_bits(modifier.amount_bits))
                ),
                format!("operation: {}", debug_snbt_string(operation)),
            ];
            if modifier.slot_id != 0 {
                fields.push(format!("slot: {}", debug_snbt_string(slot)));
            }
            match modifier.display_id {
                0 => {}
                1 => fields.push("display: {type: \"hidden\"}".to_string()),
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

fn encode_tool(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
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
                    rule.block_tag.as_deref()
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
) -> Option<String> {
    if let Some(tag) = tag {
        return registry_ids
            .is_empty()
            .then(|| debug_snbt_string(&format!("#{tag}")));
    }
    let resource_ids = registry_ids
        .iter()
        .map(|id| blocks.block_name_by_registry_id(*id))
        .collect::<Option<Vec<_>>>()?;
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

fn encode_provides_trim_material(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    if patch.provides_trim_material_direct.is_some() {
        return None;
    }
    Some(debug_snbt_string(vanilla_trim_material_resource_id(
        patch.provides_trim_material_id?,
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

fn encode_written_book_content(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
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
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    if patch.armor_trim_material_direct.is_some() || patch.armor_trim_pattern_direct.is_some() {
        return None;
    }
    let material = vanilla_trim_material_resource_id(patch.armor_trim_material_id?)?;
    let pattern = vanilla_trim_pattern_resource_id(patch.armor_trim_pattern_id?)?;
    Some(format!(
        "{{material: {}, pattern: {}}}",
        debug_snbt_string(material),
        debug_snbt_string(pattern)
    ))
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

fn encode_jukebox_playable(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    Some(debug_snbt_string(
        item_runtime?.jukebox_song_resource_id(patch.jukebox_song_id?)?,
    ))
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

fn encode_named_variant(
    type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    let resource_id = match type_id {
        81 => vanilla_villager_type_resource_id(patch.villager_variant_id?)?,
        82 => vanilla_wolf_variant_resource_id(patch.wolf_variant_id?)?,
        83 => vanilla_wolf_sound_variant_resource_id(patch.wolf_sound_variant_id?)?,
        93 => pig_variant_resource_id(patch.pig_variant_id?)?,
        94 => pig_sound_variant_resource_id(patch.pig_sound_variant_id?)?,
        95 => cow_variant_resource_id(patch.cow_variant_id?)?,
        96 => cow_sound_variant_resource_id(patch.cow_sound_variant_id?)?,
        97 => chicken_variant_resource_id(patch.chicken_variant_id?)?,
        98 => chicken_sound_variant_resource_id(patch.chicken_sound_variant_id?)?,
        99 => zombie_nautilus_variant_resource_id(patch.zombie_nautilus_variant_id?)?,
        100 => frog_variant_resource_id(patch.frog_variant_id?)?,
        105 => cat_variant_resource_id(patch.cat_variant_id?)?,
        106 => cat_sound_variant_resource_id(patch.cat_sound_variant_id?)?,
        _ => return None,
    };
    Some(debug_snbt_string(resource_id))
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

fn encode_painting_variant(
    _type_id: i32,
    patch: &bbb_protocol::packets::DataComponentPatchSummary,
    _item_runtime: Option<&NativeItemRuntime>,
) -> Option<String> {
    if patch.painting_variant_direct.is_some() {
        return None;
    }
    Some(debug_snbt_string(vanilla_painting_variant_resource_id(
        patch.painting_variant_id?,
    )?))
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
        assert_eq!(PERSISTENT_SNBT_ENCODERS.len(), 103);
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
        assert_eq!(encode_added_component_value_snbt(32, &patch, None), None);
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
    fn damage_type_encoder_follows_vanilla_bootstrap_order() {
        assert_eq!(
            VANILLA_DAMAGE_TYPE_RESOURCE_IDS,
            [
                "minecraft:in_fire",
                "minecraft:campfire",
                "minecraft:lightning_bolt",
                "minecraft:on_fire",
                "minecraft:lava",
                "minecraft:hot_floor",
                "minecraft:in_wall",
                "minecraft:cramming",
                "minecraft:drown",
                "minecraft:starve",
                "minecraft:cactus",
                "minecraft:fall",
                "minecraft:ender_pearl",
                "minecraft:fly_into_wall",
                "minecraft:out_of_world",
                "minecraft:generic",
                "minecraft:magic",
                "minecraft:wither",
                "minecraft:dragon_breath",
                "minecraft:dry_out",
                "minecraft:sweet_berry_bush",
                "minecraft:freeze",
                "minecraft:stalagmite",
                "minecraft:falling_block",
                "minecraft:falling_anvil",
                "minecraft:falling_stalactite",
                "minecraft:sting",
                "minecraft:mob_attack",
                "minecraft:mob_attack_no_aggro",
                "minecraft:player_attack",
                "minecraft:spear",
                "minecraft:arrow",
                "minecraft:trident",
                "minecraft:mob_projectile",
                "minecraft:spit",
                "minecraft:fireworks",
                "minecraft:unattributed_fireball",
                "minecraft:fireball",
                "minecraft:wither_skull",
                "minecraft:thrown",
                "minecraft:indirect_magic",
                "minecraft:thorns",
                "minecraft:explosion",
                "minecraft:player_explosion",
                "minecraft:sonic_boom",
                "minecraft:bad_respawn_point",
                "minecraft:outside_border",
                "minecraft:generic_kill",
                "minecraft:wind_charge",
                "minecraft:mace_smash",
            ]
        );

        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            damage_type_id: Some(49),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(8, &patch, None),
            Some("\"minecraft:mace_smash\"".to_string())
        );
        patch.damage_type_id = Some(50);
        assert_eq!(encode_added_component_value_snbt(8, &patch, None), None);
        patch.damage_type_id = Some(-1);
        assert_eq!(encode_added_component_value_snbt(8, &patch, None), None);
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
    fn attribute_modifier_encoder_preserves_order_defaults_and_hidden_display() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            attribute_modifiers: vec![
                bbb_protocol::packets::AttributeModifierSummary {
                    attribute_id: 2,
                    modifier_id: "minecraft:base_attack_damage".to_string(),
                    amount_bits: 3.5_f64.to_bits(),
                    operation_id: 0,
                    slot_id: 0,
                    display_id: 0,
                    display_text: None,
                    display_text_styled: None,
                },
                bbb_protocol::packets::AttributeModifierSummary {
                    attribute_id: 16,
                    modifier_id: "minecraft:test/hidden".to_string(),
                    amount_bits: (-0.25_f64).to_bits(),
                    operation_id: 2,
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

        patch.attribute_modifiers[0].attribute_id = 35;
        assert_eq!(encode_added_component_value_snbt(16, &patch, None), None);
        patch.attribute_modifiers[0].attribute_id = 2;
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

    #[test]
    fn provides_trim_material_encoder_supports_vanilla_named_holders_only() {
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
                override_armor_assets: Default::default(),
                description: "Custom material".to_string(),
                description_styled: None,
            }));
        assert_eq!(encode_added_component_value_snbt(62, &patch, None), None);
    }

    #[test]
    fn armor_trim_encoder_supports_complete_vanilla_named_holders_only() {
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
            }));
        assert_eq!(encode_added_component_value_snbt(56, &patch, None), None);
        patch.armor_trim_material_direct = None;
        patch.armor_trim_pattern_direct =
            Some(Box::new(bbb_protocol::packets::TrimPatternSummary {
                asset_id: "example:custom".to_string(),
                description: "Custom pattern".to_string(),
                description_styled: None,
                decal: false,
            }));
        assert_eq!(encode_added_component_value_snbt(56, &patch, None), None);
    }

    #[test]
    fn jukebox_playable_encoder_uses_fixed_registry_holders_only() {
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
            length_in_seconds_bits: 60.0_f32.to_bits(),
            comparator_output: 7,
        }));
        assert_eq!(
            encode_added_component_value_snbt(64, &patch, Some(&item_runtime)),
            None
        );
    }

    #[test]
    fn painting_variant_encoder_uses_fixed_registry_holders_only() {
        let mut patch = bbb_protocol::packets::DataComponentPatchSummary {
            painting_variant_id: Some(50),
            ..Default::default()
        };
        assert_eq!(
            encode_added_component_value_snbt(102, &patch, None),
            Some("\"minecraft:dennis\"".to_string())
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
    fn named_variant_encoders_use_distinct_holder_fields() {
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
    }
}
