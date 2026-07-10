use super::*;

type WireDecodeHandler = fn(i32, &mut Decoder<'_>, &mut DataComponentPatchSummary) -> Result<()>;

#[derive(Clone, Copy)]
pub struct DataComponentDescriptor {
    type_id: i32,
    resource_id: &'static str,
    persistent: bool,
    decode: WireDecodeHandler,
}

impl DataComponentDescriptor {
    pub const fn type_id(self) -> i32 {
        self.type_id
    }

    pub const fn resource_id(self) -> &'static str {
        self.resource_id
    }

    pub const fn is_persistent(self) -> bool {
        self.persistent
    }

    pub(super) fn decode(
        self,
        decoder: &mut Decoder<'_>,
        summary: &mut DataComponentPatchSummary,
    ) -> Result<()> {
        (self.decode)(self.type_id, decoder, summary)
    }
}

const fn component(
    type_id: i32,
    resource_id: &'static str,
    persistent: bool,
    decode: WireDecodeHandler,
) -> DataComponentDescriptor {
    DataComponentDescriptor {
        type_id,
        resource_id,
        persistent,
        decode,
    }
}

static VANILLA_DATA_COMPONENTS: &[DataComponentDescriptor] = &[
    component(0, "minecraft:custom_data", true, decode_custom_data),
    component(1, "minecraft:max_stack_size", true, decode_max_stack_size),
    component(2, "minecraft:max_damage", true, decode_max_damage),
    component(3, "minecraft:damage", true, decode_damage),
    component(4, "minecraft:unbreakable", true, decode_unbreakable),
    component(
        5,
        "minecraft:use_effects",
        true,
        decode_use_effects_component,
    ),
    component(6, "minecraft:custom_name", true, decode_custom_name),
    component(
        7,
        "minecraft:minimum_attack_charge",
        true,
        decode_minimum_attack_charge,
    ),
    component(8, "minecraft:damage_type", true, skip_holder_registry),
    component(9, "minecraft:item_name", true, decode_item_name),
    component(10, "minecraft:item_model", true, decode_item_model),
    component(11, "minecraft:lore", true, decode_lore_component),
    component(12, "minecraft:rarity", true, decode_rarity),
    component(13, "minecraft:enchantments", true, decode_enchantments),
    component(14, "minecraft:can_place_on", true, decode_can_place_on),
    component(15, "minecraft:can_break", true, decode_can_break),
    component(
        16,
        "minecraft:attribute_modifiers",
        true,
        decode_attribute_modifiers_component,
    ),
    component(
        17,
        "minecraft:custom_model_data",
        true,
        decode_custom_model_data_component,
    ),
    component(
        18,
        "minecraft:tooltip_display",
        true,
        decode_tooltip_display_component,
    ),
    component(19, "minecraft:repair_cost", true, decode_repair_cost),
    component(20, "minecraft:creative_slot_lock", false, skip_unit),
    component(
        21,
        "minecraft:enchantment_glint_override",
        true,
        decode_enchantment_glint_override,
    ),
    component(
        22,
        "minecraft:intangible_projectile",
        true,
        decode_intangible_projectile,
    ),
    component(23, "minecraft:food", true, decode_food_component),
    component(
        24,
        "minecraft:consumable",
        true,
        decode_consumable_component,
    ),
    component(
        25,
        "minecraft:use_remainder",
        true,
        decode_use_remainder_component,
    ),
    component(
        26,
        "minecraft:use_cooldown",
        true,
        decode_use_cooldown_component,
    ),
    component(
        27,
        "minecraft:damage_resistant",
        true,
        decode_damage_resistant,
    ),
    component(28, "minecraft:tool", true, skip_tool),
    component(29, "minecraft:weapon", true, decode_weapon_component),
    component(
        30,
        "minecraft:attack_range",
        true,
        decode_attack_range_component,
    ),
    component(31, "minecraft:enchantable", true, decode_enchantable),
    component(32, "minecraft:equippable", true, skip_equippable),
    component(33, "minecraft:repairable", true, decode_repairable),
    component(34, "minecraft:glider", true, decode_glider),
    component(35, "minecraft:tooltip_style", true, decode_tooltip_style),
    component(
        36,
        "minecraft:death_protection",
        true,
        decode_death_protection_component,
    ),
    component(37, "minecraft:blocks_attacks", true, skip_blocks_attacks),
    component(
        38,
        "minecraft:piercing_weapon",
        true,
        decode_piercing_weapon_component,
    ),
    component(
        39,
        "minecraft:kinetic_weapon",
        true,
        decode_kinetic_weapon_component,
    ),
    component(
        40,
        "minecraft:swing_animation",
        true,
        decode_swing_animation_component,
    ),
    component(41, "minecraft:additional_trade_cost", false, skip_var_i32),
    component(
        42,
        "minecraft:stored_enchantments",
        true,
        decode_stored_enchantments,
    ),
    component(43, "minecraft:dye", true, decode_dye),
    component(44, "minecraft:dyed_color", true, decode_dyed_color),
    component(45, "minecraft:map_color", true, decode_map_color),
    component(46, "minecraft:map_id", true, decode_map_id),
    component(
        47,
        "minecraft:map_decorations",
        true,
        decode_codec_backed_nbt,
    ),
    component(
        48,
        "minecraft:map_post_processing",
        false,
        decode_map_post_processing_component,
    ),
    component(
        49,
        "minecraft:charged_projectiles",
        true,
        decode_charged_projectiles,
    ),
    component(
        50,
        "minecraft:bundle_contents",
        true,
        decode_bundle_contents,
    ),
    component(
        51,
        "minecraft:potion_contents",
        true,
        decode_potion_contents_component,
    ),
    component(
        52,
        "minecraft:potion_duration_scale",
        true,
        decode_potion_duration_scale,
    ),
    component(
        53,
        "minecraft:suspicious_stew_effects",
        true,
        decode_suspicious_stew_effects_component,
    ),
    component(
        54,
        "minecraft:writable_book_content",
        true,
        decode_writable_book_content_component,
    ),
    component(
        55,
        "minecraft:written_book_content",
        true,
        decode_written_book_content_component,
    ),
    component(56, "minecraft:trim", true, decode_trim),
    component(
        57,
        "minecraft:debug_stick_state",
        true,
        decode_codec_backed_nbt,
    ),
    component(58, "minecraft:entity_data", true, decode_entity_data),
    component(
        59,
        "minecraft:bucket_entity_data",
        true,
        decode_codec_backed_nbt,
    ),
    component(
        60,
        "minecraft:block_entity_data",
        true,
        decode_block_entity_data,
    ),
    component(61, "minecraft:instrument", true, decode_instrument),
    component(
        62,
        "minecraft:provides_trim_material",
        true,
        decode_provides_trim_material,
    ),
    component(
        63,
        "minecraft:ominous_bottle_amplifier",
        true,
        decode_ominous_bottle_amplifier,
    ),
    component(
        64,
        "minecraft:jukebox_playable",
        true,
        decode_jukebox_playable_component,
    ),
    component(
        65,
        "minecraft:provides_banner_patterns",
        true,
        decode_provides_banner_patterns,
    ),
    component(66, "minecraft:recipes", true, decode_codec_backed_nbt),
    component(
        67,
        "minecraft:lodestone_tracker",
        true,
        decode_lodestone_tracker_component,
    ),
    component(
        68,
        "minecraft:firework_explosion",
        true,
        decode_firework_explosion_component,
    ),
    component(69, "minecraft:fireworks", true, decode_fireworks_component),
    component(70, "minecraft:profile", true, decode_profile),
    component(
        71,
        "minecraft:note_block_sound",
        true,
        decode_note_block_sound,
    ),
    component(
        72,
        "minecraft:banner_patterns",
        true,
        decode_banner_patterns,
    ),
    component(73, "minecraft:base_color", true, decode_base_color),
    component(
        74,
        "minecraft:pot_decorations",
        true,
        decode_pot_decorations_component,
    ),
    component(75, "minecraft:container", true, decode_container),
    component(76, "minecraft:block_state", true, decode_block_state),
    component(77, "minecraft:bees", true, decode_bees_component),
    component(78, "minecraft:lock", true, decode_codec_backed_nbt),
    component(79, "minecraft:container_loot", true, decode_container_loot),
    component(80, "minecraft:break_sound", true, decode_break_sound),
    component(
        81,
        "minecraft:villager/variant",
        true,
        decode_villager_variant,
    ),
    component(82, "minecraft:wolf/variant", true, decode_wolf_variant),
    component(
        83,
        "minecraft:wolf/sound_variant",
        true,
        decode_wolf_sound_variant,
    ),
    component(84, "minecraft:wolf/collar", true, decode_builtin_enum),
    component(85, "minecraft:fox/variant", true, decode_builtin_enum),
    component(86, "minecraft:salmon/size", true, decode_builtin_enum),
    component(87, "minecraft:parrot/variant", true, decode_builtin_enum),
    component(
        88,
        "minecraft:tropical_fish/pattern",
        true,
        decode_tropical_fish_pattern,
    ),
    component(
        89,
        "minecraft:tropical_fish/base_color",
        true,
        decode_tropical_fish_base_color,
    ),
    component(
        90,
        "minecraft:tropical_fish/pattern_color",
        true,
        decode_tropical_fish_pattern_color,
    ),
    component(91, "minecraft:mooshroom/variant", true, decode_builtin_enum),
    component(92, "minecraft:rabbit/variant", true, decode_builtin_enum),
    component(93, "minecraft:pig/variant", true, decode_pig_variant),
    component(
        94,
        "minecraft:pig/sound_variant",
        true,
        decode_pig_sound_variant,
    ),
    component(95, "minecraft:cow/variant", true, decode_cow_variant),
    component(
        96,
        "minecraft:cow/sound_variant",
        true,
        decode_cow_sound_variant,
    ),
    component(
        97,
        "minecraft:chicken/variant",
        true,
        decode_chicken_variant,
    ),
    component(
        98,
        "minecraft:chicken/sound_variant",
        true,
        decode_chicken_sound_variant,
    ),
    component(
        99,
        "minecraft:zombie_nautilus/variant",
        true,
        decode_zombie_nautilus_variant,
    ),
    component(100, "minecraft:frog/variant", true, decode_frog_variant),
    component(101, "minecraft:horse/variant", true, decode_builtin_enum),
    component(
        102,
        "minecraft:painting/variant",
        true,
        decode_painting_variant,
    ),
    component(103, "minecraft:llama/variant", true, decode_builtin_enum),
    component(104, "minecraft:axolotl/variant", true, decode_builtin_enum),
    component(105, "minecraft:cat/variant", true, decode_cat_variant),
    component(
        106,
        "minecraft:cat/sound_variant",
        true,
        decode_cat_sound_variant,
    ),
    component(107, "minecraft:cat/collar", true, decode_builtin_enum),
    component(108, "minecraft:sheep/color", true, decode_builtin_enum),
    component(109, "minecraft:shulker/color", true, decode_builtin_enum),
];

pub fn vanilla_data_component_descriptors() -> &'static [DataComponentDescriptor] {
    VANILLA_DATA_COMPONENTS
}

pub fn vanilla_data_component_descriptor(type_id: i32) -> Option<&'static DataComponentDescriptor> {
    let index = usize::try_from(type_id).ok()?;
    let descriptor = VANILLA_DATA_COMPONENTS.get(index)?;
    (descriptor.type_id == type_id).then_some(descriptor)
}

fn decode_custom_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.custom_data = decode_nbt_summary_from_decoder(decoder)?;
    Ok(())
}

macro_rules! decode_var_i32_field {
    ($name:ident, $field:ident) => {
        fn $name(
            _type_id: i32,
            decoder: &mut Decoder<'_>,
            summary: &mut DataComponentPatchSummary,
        ) -> Result<()> {
            summary.$field = Some(decoder.read_var_i32()?);
            Ok(())
        }
    };
}

decode_var_i32_field!(decode_max_stack_size, max_stack_size);
decode_var_i32_field!(decode_max_damage, max_damage);
decode_var_i32_field!(decode_damage, damage);
decode_var_i32_field!(decode_repair_cost, repair_cost);
decode_var_i32_field!(decode_enchantable, enchantable_value);
decode_var_i32_field!(decode_map_id, map_id);
decode_var_i32_field!(decode_ominous_bottle_amplifier, ominous_bottle_amplifier);

fn decode_unbreakable(
    _type_id: i32,
    _decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.unbreakable = true;
    Ok(())
}

fn decode_use_effects_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.use_effects = Some(decode_use_effects_summary(decoder)?);
    Ok(())
}

fn decode_styled_name(decoder: &mut Decoder<'_>) -> Result<(String, Vec<StyledTextRun>)> {
    let runs = decode_styled_component_summary_from_decoder(decoder)?;
    Ok((styled_runs_summary_text(&runs), runs))
}

fn decode_custom_name(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (text, runs) = decode_styled_name(decoder)?;
    summary.custom_name = Some(text);
    summary.custom_name_styled = Some(runs);
    Ok(())
}

fn decode_item_name(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (text, runs) = decode_styled_name(decoder)?;
    summary.item_name = Some(text);
    summary.item_name_styled = Some(runs);
    Ok(())
}

fn decode_minimum_attack_charge(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.minimum_attack_charge_bits = Some(decoder.read_f32()?.to_bits());
    Ok(())
}

fn decode_item_model(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.item_model = Some(read_resource_location(decoder)?);
    Ok(())
}

fn decode_lore_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.lore_styled = decode_lore(decoder)?;
    summary.lore = summary
        .lore_styled
        .iter()
        .map(|line| styled_runs_summary_text(line))
        .collect();
    Ok(())
}

fn decode_rarity(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.rarity = Some(decode_item_rarity(decoder)?);
    Ok(())
}

fn decode_enchantments(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.enchantments = decode_varint_map(decoder)?;
    Ok(())
}

fn decode_can_place_on(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.can_place_on = Some(decode_adventure_mode_predicate_summary(decoder)?);
    Ok(())
}

fn decode_can_break(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.can_break = Some(decode_adventure_mode_predicate_summary(decoder)?);
    Ok(())
}

fn decode_attribute_modifiers_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.attribute_modifiers = decode_attribute_modifiers(decoder)?;
    Ok(())
}

fn decode_custom_model_data_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (floats, flags, strings, colors) = decode_custom_model_data(decoder)?;
    summary.custom_model_data_floats = floats.into();
    summary.custom_model_data_flags = flags;
    summary.custom_model_data_strings = strings;
    summary.custom_model_data_colors = colors;
    Ok(())
}

fn decode_tooltip_display_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (hide_tooltip, hidden_components) = decode_tooltip_display_summary(decoder)?;
    summary.tooltip_hide_tooltip = hide_tooltip;
    summary.tooltip_hidden_component_type_ids = hidden_components;
    Ok(())
}

fn decode_enchantment_glint_override(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.enchantment_glint_override = Some(decoder.read_bool()?);
    Ok(())
}

fn decode_intangible_projectile(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.intangible_projectile = true;
    skip_nbt_tag_from_decoder(decoder)
}

fn decode_food_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.food_nutrition = Some(decoder.read_var_i32()?);
    summary.food_saturation_bits = Some(decoder.read_f32()?.to_bits());
    summary.food_can_always_eat = decoder.read_bool()?;
    Ok(())
}

fn decode_consumable_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let consumable = decode_consumable_component_summary(decoder)?;
    summary.consumable = Some(consumable.summary);
    summary.consumable_sound = Some(consumable.sound);
    summary.consumable_has_consume_particles = Some(consumable.has_consume_particles);
    summary.consumable_effect_count = Some(consumable.effect_count);
    summary.consumable_effects = consumable.effects;
    Ok(())
}

fn decode_use_remainder_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.use_remainder = Some(Box::new(decode_item_stack_template(decoder)?));
    Ok(())
}

fn decode_use_cooldown_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let cooldown = decode_use_cooldown_summary(decoder)?;
    summary.use_cooldown_seconds_bits = Some(cooldown.seconds_bits);
    summary.use_cooldown_ticks = Some(cooldown.ticks);
    summary.use_cooldown_group = cooldown.cooldown_group;
    Ok(())
}

fn decode_damage_resistant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let types = decode_holder_set_summary(decoder)?;
    summary.damage_resistant_type_registry_ids = types.registry_ids;
    summary.damage_resistant_type_tag = types.tag;
    Ok(())
}

fn decode_weapon_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.weapon_item_damage_per_attack = Some(decoder.read_var_i32()?);
    summary.weapon_disable_blocking_seconds_bits = Some(decoder.read_f32()?.to_bits());
    Ok(())
}

fn decode_attack_range_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.attack_range = Some(decode_attack_range_summary(decoder)?);
    Ok(())
}

fn decode_repairable(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let items = decode_holder_set_summary(decoder)?;
    summary.repairable_item_registry_ids = items.registry_ids;
    summary.repairable_item_tag = items.tag;
    Ok(())
}

fn decode_glider(
    _type_id: i32,
    _decoder: &mut Decoder<'_>,
    _summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    Ok(())
}

fn decode_tooltip_style(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tooltip_style = Some(read_resource_location(decoder)?);
    Ok(())
}

fn decode_death_protection_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.death_protection_effects = decode_consume_effect_list_summary(decoder)?;
    Ok(())
}

fn decode_piercing_weapon_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.piercing_weapon = Some(decode_piercing_weapon_summary(decoder)?);
    Ok(())
}

fn decode_kinetic_weapon_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.kinetic_weapon = Some(decode_kinetic_weapon_summary(decoder)?);
    Ok(())
}

fn decode_swing_animation_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.swing_animation = Some(decode_swing_animation(decoder)?);
    Ok(())
}

fn decode_stored_enchantments(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.stored_enchantments = decode_varint_map(decoder)?;
    Ok(())
}

fn decode_dye(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.dye_color_id = Some(normalize_dye_color_id(decoder.read_var_i32()?));
    Ok(())
}

fn decode_dyed_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.dyed_color = Some(decoder.read_i32()?);
    Ok(())
}

fn decode_map_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.map_color = Some(decoder.read_i32()?);
    Ok(())
}

fn decode_codec_backed_nbt(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    if let Some(value) = decode_nbt_summary_from_decoder(decoder)? {
        summary.codec_backed_nbt.insert(type_id, value);
    }
    Ok(())
}

fn decode_map_post_processing_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.map_post_processing = Some(decode_map_post_processing(decoder)?);
    Ok(())
}

fn decode_charged_projectiles(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.charged_projectiles_items =
        decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    Ok(())
}

fn decode_bundle_contents(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.bundle_contents_items =
        decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    summary.bundle_contents_item_count = Some(summary.bundle_contents_items.len());
    Ok(())
}

fn decode_potion_contents_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let potion = decode_potion_contents(decoder)?;
    summary.potion_id = potion.potion_id;
    summary.potion_custom_color = potion.custom_color;
    summary.potion_custom_effect_count = Some(potion.custom_effects.len());
    summary.potion_custom_effects = potion.custom_effects;
    summary.potion_custom_name = potion.custom_name;
    Ok(())
}

fn decode_potion_duration_scale(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.potion_duration_scale_bits = Some(decoder.read_f32()?.to_bits());
    Ok(())
}

fn decode_suspicious_stew_effects_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.suspicious_stew_effects = decode_suspicious_stew_effects(decoder)?;
    Ok(())
}

fn decode_writable_book_content_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let writable_book = decode_writable_book_content(decoder)?;
    summary.writable_book_pages = writable_book.pages;
    summary.writable_book_page_filters = writable_book.page_filters;
    Ok(())
}

fn decode_written_book_content_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.written_book = Some(decode_written_book_content(decoder)?);
    Ok(())
}

fn decode_trim(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let trim = decode_armor_trim(decoder)?;
    summary.armor_trim_material_id = trim.material_id;
    summary.armor_trim_material_direct = trim.material_direct.map(Box::new);
    summary.armor_trim_pattern_id = trim.pattern_id;
    summary.armor_trim_pattern_direct = trim.pattern_direct.map(Box::new);
    Ok(())
}

fn decode_entity_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let entity_data = decode_typed_entity_data_summary(decoder)?;
    summary.entity_data_entity_type_id = Some(entity_data.type_id);
    summary.entity_data_tag = entity_data.tag;
    Ok(())
}

fn decode_block_entity_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let block_entity_data = decode_typed_entity_data_summary(decoder)?;
    summary.block_entity_data_type_id = Some(block_entity_data.type_id);
    summary.block_entity_spawn_entity_type =
        typed_entity_data_spawn_entity_type(block_entity_data.tag.as_ref());
    summary.block_entity_data_tag = block_entity_data.tag;
    Ok(())
}

fn decode_instrument(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let instrument = decode_instrument_component_summary(decoder)?;
    summary.instrument_id = instrument.registry_id;
    summary.instrument_description = instrument.description;
    summary.instrument_description_styled = instrument.description_styled;
    Ok(())
}

fn decode_jukebox_playable_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let song = decode_jukebox_song_holder(decoder)?;
    summary.jukebox_song_id = song.registry_id;
    summary.jukebox_direct_song = song.direct.map(Box::new);
    Ok(())
}

fn decode_provides_banner_patterns(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let patterns = decode_holder_set_summary(decoder)?;
    summary.provides_banner_pattern_registry_ids = patterns.registry_ids;
    summary.provides_banner_pattern_tag = patterns.tag;
    Ok(())
}

fn decode_lodestone_tracker_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (target, tracked) = decode_lodestone_tracker(decoder)?;
    summary.lodestone_target = target;
    summary.lodestone_tracked = Some(tracked);
    Ok(())
}

fn decode_firework_explosion_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let explosion = decode_firework_explosion(decoder)?;
    summary.firework_explosion_colors = explosion.colors;
    summary.firework_explosion_fade_colors = explosion.fade_colors;
    summary.firework_explosion_shape = Some(explosion.shape);
    summary.firework_explosion_has_trail = Some(explosion.has_trail);
    summary.firework_explosion_has_twinkle = Some(explosion.has_twinkle);
    Ok(())
}

fn decode_fireworks_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let fireworks = decode_fireworks(decoder)?;
    summary.fireworks_flight_duration = Some(fireworks.flight_duration);
    summary.fireworks_explosions_count = Some(fireworks.explosions.len());
    summary.fireworks_explosions = fireworks.explosions;
    Ok(())
}

fn decode_profile(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.profile = Some(decode_resolvable_profile(decoder)?);
    Ok(())
}

fn decode_note_block_sound(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.note_block_sound = Some(read_resource_location(decoder)?);
    Ok(())
}

fn decode_banner_patterns(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.banner_pattern_layers = decode_banner_pattern_layers_summary(decoder)?;
    Ok(())
}

fn decode_base_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.base_color_id = Some(normalize_dye_color_id(decoder.read_var_i32()?));
    Ok(())
}

fn decode_pot_decorations_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.pot_decorations_item_ids = decode_pot_decorations_item_ids(decoder)?;
    Ok(())
}

fn decode_container(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let container = decode_item_container_contents(decoder)?;
    summary.container_item_count = Some(container.items.len());
    summary.container_items = container.items;
    summary.container_slots = container.slots;
    Ok(())
}

fn decode_block_state(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.block_state_properties = decode_string_map(decoder, MAX_BLOCK_STATE_PROPERTIES)?;
    Ok(())
}

fn decode_bees_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.bees_present = true;
    summary.bees = decode_bees(decoder)?;
    summary.bees_count = summary.bees.len();
    Ok(())
}

fn decode_container_loot(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.container_loot = true;
    decode_codec_backed_nbt(type_id, decoder, summary)
}

fn decode_break_sound(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.break_sound = Some(decode_sound_event_holder_summary(decoder)?);
    Ok(())
}

fn decode_villager_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.villager_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_wolf_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.wolf_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_wolf_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.wolf_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_frog_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.frog_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_zombie_nautilus_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.zombie_nautilus_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_pig_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.pig_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_pig_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.pig_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_cow_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cow_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_cow_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cow_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_chicken_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.chicken_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_chicken_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.chicken_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_cat_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cat_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_cat_sound_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.cat_sound_variant_id = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

fn decode_builtin_enum(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let id = decoder.read_var_i32()?;
    summary
        .builtin_enum_component_ids
        .insert(type_id, normalize_builtin_enum_component_id(type_id, id));
    Ok(())
}

fn decode_tropical_fish_pattern(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tropical_fish_pattern_id = Some(decoder.read_var_i32()?);
    Ok(())
}

fn decode_tropical_fish_base_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tropical_fish_base_color_id = Some(decoder.read_var_i32()?);
    Ok(())
}

fn decode_tropical_fish_pattern_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tropical_fish_pattern_color_id = Some(decoder.read_var_i32()?);
    Ok(())
}

fn decode_painting_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let painting = decode_painting_variant_holder_summary(decoder)?;
    summary.painting_variant_id = painting.registry_id;
    summary.painting_variant_direct = painting.direct;
    Ok(())
}

fn skip_var_i32(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    _summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    decoder.read_var_i32()?;
    Ok(())
}

fn skip_unit(
    _type_id: i32,
    _decoder: &mut Decoder<'_>,
    _summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    Ok(())
}

fn skip_holder_registry(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    _summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    decode_holder_registry(decoder)
}

fn skip_tool(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    _summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    decode_tool(decoder)
}

fn skip_equippable(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    _summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    decode_equippable(decoder)
}

fn skip_blocks_attacks(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    _summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    decode_blocks_attacks(decoder)
}

fn decode_provides_trim_material(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let material = decode_trim_material_holder_summary(decoder)?;
    summary.provides_trim_material_id = material.registry_id;
    summary.provides_trim_material_direct = material.direct.map(Box::new);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    use crate::codec::Encoder;

    use super::*;

    #[test]
    fn catalog_is_complete_unique_and_matches_vanilla_registration_order() {
        assert_eq!(VANILLA_DATA_COMPONENTS.len(), 110);
        let mut type_ids = HashSet::new();
        let mut resource_ids = HashSet::new();
        for (expected_id, descriptor) in VANILLA_DATA_COMPONENTS.iter().enumerate() {
            assert_eq!(descriptor.type_id(), expected_id as i32);
            assert!(type_ids.insert(descriptor.type_id()));
            assert!(resource_ids.insert(descriptor.resource_id()));
        }
        assert_eq!(
            VANILLA_DATA_COMPONENTS
                .iter()
                .filter(|descriptor| !descriptor.is_persistent())
                .map(|descriptor| descriptor.type_id())
                .collect::<Vec<_>>(),
            vec![20, 41, 48]
        );

        let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../mc-code/sources/26.1/net/minecraft/core/component")
            .join("DataComponents.java");
        if !source_path.exists() {
            eprintln!(
                "skipping local DataComponents source check; missing {}",
                source_path.display()
            );
            return;
        }

        let source = std::fs::read_to_string(source_path).unwrap();
        let mut remaining = source.as_str();
        let mut registered = Vec::new();
        while let Some(index) = remaining.find("= register(") {
            remaining = &remaining[index + "= register(".len()..];
            let start = remaining.find('"').unwrap() + 1;
            let tail = &remaining[start..];
            let end = tail.find('"').unwrap();
            registered.push(format!("minecraft:{}", &tail[..end]));
            remaining = &tail[end + 1..];
        }
        assert_eq!(
            registered,
            VANILLA_DATA_COMPONENTS
                .iter()
                .map(|descriptor| descriptor.resource_id())
                .collect::<Vec<_>>()
        );

        let transient = source
            .split("= register(")
            .skip(1)
            .take(VANILLA_DATA_COMPONENTS.len())
            .enumerate()
            .filter_map(|(type_id, definition)| {
                (!definition.contains(".persistent(")).then_some(type_id as i32)
            })
            .collect::<Vec<_>>();
        assert_eq!(transient, vec![20, 41, 48]);
    }

    #[test]
    fn descriptor_handler_projects_component_value() {
        let mut payload = Encoder::new();
        payload.write_var_i32(99);
        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let mut summary = DataComponentPatchSummary::default();

        vanilla_data_component_descriptor(1)
            .unwrap()
            .decode(&mut decoder, &mut summary)
            .unwrap();

        assert_eq!(summary.max_stack_size, Some(99));
        assert!(decoder.is_empty());
    }

    #[test]
    fn component_patch_loop_dispatches_through_catalog_descriptor() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_var_i32(64);
        payload.write_var_i32(20);

        let payload = payload.into_inner();
        let mut decoder = Decoder::new(&payload);
        let summary = decode_data_component_patch_summary(&mut decoder).unwrap();

        assert_eq!(summary.added_type_ids, vec![1]);
        assert_eq!(summary.max_stack_size, Some(64));
        assert_eq!(summary.removed_type_ids, vec![20]);
        assert!(decoder.is_empty());
    }
}
