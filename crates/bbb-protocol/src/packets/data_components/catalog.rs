use super::*;
use std::sync::OnceLock;

use crate::block_entity_types::vanilla_block_entity_type;
use crate::data_component_types::{
    DataComponentTypeCatalogEntry, VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID, VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID, VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID, VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
    VANILLA_DATA_COMPONENT_TRIM_TYPE_ID, VANILLA_DATA_COMPONENT_TYPES,
    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use crate::nbt_summary::{encode_nbt_summary_bounded, MAX_NBT_DEPTH};

type WireDecodeHandler = fn(i32, &mut Decoder<'_>, &mut DataComponentPatchSummary) -> Result<()>;
type WireEncodeHandler = fn(i32, &DataComponentPatchSummary, &mut Encoder) -> Result<()>;

#[derive(Clone, Copy)]
pub struct DataComponentDescriptor {
    metadata: &'static DataComponentTypeCatalogEntry,
    decode: WireDecodeHandler,
    encode: Option<WireEncodeHandler>,
}

impl DataComponentDescriptor {
    pub const fn type_id(self) -> i32 {
        self.metadata.type_id
    }

    pub const fn resource_id(self) -> &'static str {
        self.metadata.resource_id
    }

    pub const fn is_persistent(self) -> bool {
        self.metadata.persistent
    }

    pub(super) fn decode(
        self,
        decoder: &mut Decoder<'_>,
        summary: &mut DataComponentPatchSummary,
    ) -> Result<()> {
        (self.decode)(self.metadata.type_id, decoder, summary)
    }

    pub(crate) const fn can_encode(self) -> bool {
        self.encode.is_some()
    }

    pub(super) fn encode(
        self,
        summary: &DataComponentPatchSummary,
        out: &mut Encoder,
    ) -> Result<()> {
        let encode = self.encode.ok_or_else(|| {
            ProtocolError::InvalidData(format!(
                "cannot encode unsupported data component {} ({})",
                self.resource_id(),
                self.type_id()
            ))
        })?;
        encode(self.metadata.type_id, summary, out)
    }
}

static WIRE_DECODE_HANDLERS: &[WireDecodeHandler] = &[
    decode_custom_data,
    decode_max_stack_size,
    decode_max_damage,
    decode_damage,
    decode_unbreakable,
    decode_use_effects_component,
    decode_custom_name,
    decode_minimum_attack_charge,
    decode_damage_type,
    decode_item_name,
    decode_item_model,
    decode_lore_component,
    decode_rarity,
    decode_enchantments,
    decode_can_place_on,
    decode_can_break,
    decode_attribute_modifiers_component,
    decode_custom_model_data_component,
    decode_tooltip_display_component,
    decode_repair_cost,
    skip_unit,
    decode_enchantment_glint_override,
    decode_intangible_projectile,
    decode_food_component,
    decode_consumable_component,
    decode_use_remainder_component,
    decode_use_cooldown_component,
    decode_damage_resistant,
    decode_tool_component,
    decode_weapon_component,
    decode_attack_range_component,
    decode_enchantable,
    decode_equippable_component,
    decode_repairable,
    decode_glider,
    decode_tooltip_style,
    decode_death_protection_component,
    decode_blocks_attacks_component,
    decode_piercing_weapon_component,
    decode_kinetic_weapon_component,
    decode_swing_animation_component,
    skip_var_i32,
    decode_stored_enchantments,
    decode_dye,
    decode_dyed_color,
    decode_map_color,
    decode_map_id,
    decode_codec_backed_nbt,
    decode_map_post_processing_component,
    decode_charged_projectiles,
    decode_bundle_contents,
    decode_potion_contents_component,
    decode_potion_duration_scale,
    decode_suspicious_stew_effects_component,
    decode_writable_book_content_component,
    decode_written_book_content_component,
    decode_trim,
    decode_codec_backed_nbt,
    decode_entity_data,
    decode_bucket_entity_data,
    decode_block_entity_data,
    decode_instrument,
    decode_provides_trim_material,
    decode_ominous_bottle_amplifier,
    decode_jukebox_playable_component,
    decode_provides_banner_patterns,
    decode_codec_backed_nbt,
    decode_lodestone_tracker_component,
    decode_firework_explosion_component,
    decode_fireworks_component,
    decode_profile,
    decode_note_block_sound,
    decode_banner_patterns,
    decode_base_color,
    decode_pot_decorations_component,
    decode_container,
    decode_block_state,
    decode_bees_component,
    decode_codec_backed_nbt,
    decode_container_loot,
    decode_break_sound,
    decode_villager_variant,
    decode_wolf_variant,
    decode_wolf_sound_variant,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_tropical_fish_pattern,
    decode_tropical_fish_base_color,
    decode_tropical_fish_pattern_color,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_pig_variant,
    decode_pig_sound_variant,
    decode_cow_variant,
    decode_cow_sound_variant,
    decode_chicken_variant,
    decode_chicken_sound_variant,
    decode_zombie_nautilus_variant,
    decode_frog_variant,
    decode_builtin_enum,
    decode_painting_variant,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_cat_variant,
    decode_cat_sound_variant,
    decode_builtin_enum,
    decode_builtin_enum,
    decode_builtin_enum,
];

static VANILLA_DATA_COMPONENTS: OnceLock<Vec<DataComponentDescriptor>> = OnceLock::new();

fn initialized_data_component_descriptors() -> &'static [DataComponentDescriptor] {
    VANILLA_DATA_COMPONENTS
        .get_or_init(|| {
            assert_eq!(
                VANILLA_DATA_COMPONENT_TYPES.len(),
                WIRE_DECODE_HANDLERS.len()
            );
            VANILLA_DATA_COMPONENT_TYPES
                .iter()
                .zip(WIRE_DECODE_HANDLERS.iter().copied())
                .map(|(metadata, decode)| DataComponentDescriptor {
                    metadata,
                    decode,
                    encode: wire_encode_handler(metadata.type_id),
                })
                .collect()
        })
        .as_slice()
}

pub fn vanilla_data_component_descriptors() -> &'static [DataComponentDescriptor] {
    initialized_data_component_descriptors()
}

pub fn vanilla_data_component_descriptor(type_id: i32) -> Option<&'static DataComponentDescriptor> {
    let index = usize::try_from(type_id).ok()?;
    let descriptor = initialized_data_component_descriptors().get(index)?;
    (descriptor.type_id() == type_id).then_some(descriptor)
}

const fn wire_encode_handler(type_id: i32) -> Option<WireEncodeHandler> {
    match type_id {
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID => Some(encode_custom_data),
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID => Some(encode_max_stack_size),
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID => Some(encode_max_damage),
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID => Some(encode_damage),
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID => Some(encode_unbreakable),
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID => Some(encode_attribute_modifiers),
        VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID => Some(encode_consumable),
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID => Some(encode_death_protection),
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID => Some(encode_blocks_attacks),
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID => Some(encode_piercing_weapon),
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID => Some(encode_kinetic_weapon),
        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID => Some(encode_swing_animation),
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID => Some(encode_written_book_content),
        VANILLA_DATA_COMPONENT_TRIM_TYPE_ID => Some(encode_trim),
        VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID => Some(encode_entity_data),
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID => Some(encode_bucket_entity_data),
        VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID => Some(encode_block_entity_data),
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID => Some(encode_profile),
        _ => None,
    }
}

fn decode_custom_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.custom_data = Some(decode_required_compound_tag(decoder, "custom_data")?);
    Ok(())
}

fn encode_custom_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_required_compound_tag(summary.custom_data.as_ref(), "custom_data", out)
}

fn encode_attribute_modifiers(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if summary.attribute_modifiers.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            summary.attribute_modifiers.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_var_i32(summary.attribute_modifiers.len() as i32);
    for modifier in &summary.attribute_modifiers {
        if crate::attribute_types::vanilla_attribute_type(modifier.attribute_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid vanilla attribute holder id {}",
                modifier.attribute_id
            )));
        }
        if normalize_resource_location(modifier.modifier_id.clone())? != modifier.modifier_id {
            return Err(ProtocolError::InvalidData(format!(
                "noncanonical attribute modifier resource location `{}`",
                modifier.modifier_id
            )));
        }
        if crate::attribute_modifier_types::attribute_modifier_operation_name(modifier.operation_id)
            .is_none()
        {
            return Err(ProtocolError::InvalidData(format!(
                "invalid attribute modifier operation id {}",
                modifier.operation_id
            )));
        }
        if vanilla_equipment_slot_group(modifier.slot_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid equipment slot group id {}",
                modifier.slot_id
            )));
        }

        encoded.write_var_i32(modifier.attribute_id);
        encoded.write_string(&modifier.modifier_id);
        encoded.write_f64(f64::from_bits(modifier.amount_bits));
        encoded.write_var_i32(modifier.operation_id);
        encoded.write_var_i32(modifier.slot_id);
        encode_attribute_modifier_display(modifier, &mut encoded)?;
    }
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_attribute_modifier_display(
    modifier: &AttributeModifierSummary,
    out: &mut Encoder,
) -> Result<()> {
    use crate::attribute_modifier_types::{
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
    };

    match modifier.display_id {
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID
        | VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID => {
            if modifier.display_text.is_some()
                || modifier.display_text_styled.is_some()
                || modifier.display_text_nbt.is_some()
            {
                return Err(ProtocolError::InvalidData(format!(
                    "attribute modifier display {} cannot contain override text",
                    modifier.display_id
                )));
            }
            out.write_var_i32(modifier.display_id);
        }
        VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID => {
            let nbt = modifier.display_text_nbt.as_ref().ok_or_else(|| {
                ProtocolError::InvalidData(
                    "attribute modifier override display is missing component nbt".to_string(),
                )
            })?;
            let component =
                encode_nbt_summary_bounded(Some(nbt), i32::MAX as usize, MAX_NBT_DEPTH)?;
            let (canonical_runs, canonical_nbt) =
                decode_styled_component_summary_with_nbt_from_decoder(&mut Decoder::new(
                    &component,
                ))?;
            let canonical_text = styled_runs_summary_text(&canonical_runs);
            if modifier.display_text.as_deref() != Some(canonical_text.as_str())
                || modifier.display_text_styled.as_deref() != Some(canonical_runs.as_slice())
                || canonical_nbt.as_ref() != Some(nbt)
            {
                return Err(ProtocolError::InvalidData(
                    "attribute modifier override display contains noncanonical component projections"
                        .to_string(),
                ));
            }
            out.write_var_i32(modifier.display_id);
            out.write_bytes(&component);
        }
        display_id => {
            return Err(ProtocolError::InvalidData(format!(
                "invalid attribute modifier display id {display_id}"
            )));
        }
    }
    Ok(())
}

fn encode_consumable(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let consumable = summary.consumable.ok_or_else(|| {
        ProtocolError::InvalidData("missing consumable component value".to_string())
    })?;
    let sound = summary.consumable_sound.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("consumable component is missing sound holder".to_string())
    })?;
    let has_consume_particles = summary.consumable_has_consume_particles.ok_or_else(|| {
        ProtocolError::InvalidData("consumable component is missing particle flag".to_string())
    })?;
    let effect_count = summary.consumable_effect_count.ok_or_else(|| {
        ProtocolError::InvalidData("consumable component is missing effect count".to_string())
    })?;
    if effect_count != summary.consumable_effects.len() {
        return Err(ProtocolError::InvalidData(format!(
            "consumable effect count {effect_count} does not match {} effects",
            summary.consumable_effects.len()
        )));
    }

    let mut encoded = Encoder::new();
    encoded.write_f32(f32::from_bits(consumable.consume_seconds.to_bits()));
    encoded.write_var_i32(consumable.animation.id());
    encode_sound_event_holder_summary(&mut encoded, sound)?;
    encoded.write_bool(has_consume_particles);
    encode_consume_effect_list(&mut encoded, &summary.consumable_effects)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_death_protection(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let mut encoded = Encoder::new();
    encode_consume_effect_list(&mut encoded, &summary.death_protection_effects)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_consume_effect_list(out: &mut Encoder, effects: &[ConsumeEffectSummary]) -> Result<()> {
    if effects.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            effects.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    out.write_var_i32(effects.len() as i32);
    for effect in effects {
        encode_consume_effect(out, effect)?;
    }
    Ok(())
}

fn encode_consume_effect(out: &mut Encoder, effect: &ConsumeEffectSummary) -> Result<()> {
    use crate::consume_effect_types::{
        VANILLA_CONSUME_EFFECT_TYPE_APPLY_EFFECTS_ID,
        VANILLA_CONSUME_EFFECT_TYPE_CLEAR_ALL_EFFECTS_ID,
        VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID, VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID,
        VANILLA_CONSUME_EFFECT_TYPE_TELEPORT_RANDOMLY_ID,
    };

    match effect {
        ConsumeEffectSummary::ApplyEffects {
            effects,
            probability_bits,
        } => {
            out.write_var_i32(VANILLA_CONSUME_EFFECT_TYPE_APPLY_EFFECTS_ID);
            if effects.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(
                    effects.len(),
                    MAX_DATA_COMPONENT_LIST_ITEMS,
                ));
            }
            out.write_var_i32(effects.len() as i32);
            for effect in effects {
                encode_mob_effect_instance(out, effect)?;
            }
            out.write_f32(f32::from_bits(*probability_bits));
        }
        ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids,
            effect_tag,
        } => {
            out.write_var_i32(VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID);
            encode_holder_set_summary(
                out,
                effect_registry_ids,
                effect_tag.as_deref(),
                NetworkHolderSetRegistry::MobEffect,
                "remove consume effect",
            )?;
        }
        ConsumeEffectSummary::ClearAllEffects => {
            out.write_var_i32(VANILLA_CONSUME_EFFECT_TYPE_CLEAR_ALL_EFFECTS_ID);
        }
        ConsumeEffectSummary::TeleportRandomly { diameter_bits } => {
            out.write_var_i32(VANILLA_CONSUME_EFFECT_TYPE_TELEPORT_RANDOMLY_ID);
            out.write_f32(f32::from_bits(*diameter_bits));
        }
        ConsumeEffectSummary::PlaySound { sound } => {
            out.write_var_i32(VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID);
            encode_sound_event_holder_summary(out, sound)?;
        }
    }
    Ok(())
}

fn encode_mob_effect_instance(out: &mut Encoder, effect: &MobEffectInstanceSummary) -> Result<()> {
    client_state::validate_mob_effect_holder_id(effect.effect_id)?;
    out.write_var_i32(effect.effect_id);
    encode_mob_effect_details(
        out,
        effect.amplifier,
        effect.duration,
        effect.ambient,
        effect.show_particles,
        effect.show_icon,
        effect.hidden_effect.as_deref(),
        0,
    )
}

#[allow(clippy::too_many_arguments)]
fn encode_mob_effect_details(
    out: &mut Encoder,
    amplifier: i32,
    duration: i32,
    ambient: bool,
    show_particles: bool,
    show_icon: bool,
    hidden_effect: Option<&MobEffectDetailsSummary>,
    depth: usize,
) -> Result<()> {
    if depth > MAX_MOB_EFFECT_DETAILS_DEPTH {
        return Err(ProtocolError::InvalidData(
            "mob effect details exceeded max depth".to_string(),
        ));
    }
    if !(0..=255).contains(&amplifier) {
        return Err(ProtocolError::InvalidData(format!(
            "mob effect amplifier {amplifier} is outside 0..=255"
        )));
    }
    out.write_var_i32(amplifier);
    out.write_var_i32(duration);
    out.write_bool(ambient);
    out.write_bool(show_particles);
    out.write_bool(show_icon);
    out.write_bool(hidden_effect.is_some());
    if let Some(hidden_effect) = hidden_effect {
        encode_mob_effect_details(
            out,
            hidden_effect.amplifier,
            hidden_effect.duration,
            hidden_effect.ambient,
            hidden_effect.show_particles,
            hidden_effect.show_icon,
            hidden_effect.hidden_effect.as_deref(),
            depth + 1,
        )?;
    }
    Ok(())
}

fn encode_piercing_weapon(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let weapon = summary.piercing_weapon.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing piercing_weapon component value".to_string())
    })?;
    let mut encoded = Encoder::new();
    encoded.write_bool(weapon.deals_knockback);
    encoded.write_bool(weapon.dismounts);
    encode_optional_sound_event_holder_summary(&mut encoded, weapon.sound.as_ref())?;
    encode_optional_sound_event_holder_summary(&mut encoded, weapon.hit_sound.as_ref())?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_kinetic_weapon(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let weapon = summary.kinetic_weapon.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing kinetic_weapon component value".to_string())
    })?;
    let mut encoded = Encoder::new();
    encoded.write_var_i32(weapon.contact_cooldown_ticks);
    encoded.write_var_i32(weapon.delay_ticks);
    encode_optional_kinetic_weapon_condition(&mut encoded, weapon.dismount_conditions)?;
    encode_optional_kinetic_weapon_condition(&mut encoded, weapon.knockback_conditions)?;
    encode_optional_kinetic_weapon_condition(&mut encoded, weapon.damage_conditions)?;
    encoded.write_f32(f32::from_bits(weapon.forward_movement_bits));
    encoded.write_f32(f32::from_bits(weapon.damage_multiplier_bits));
    encode_optional_sound_event_holder_summary(&mut encoded, weapon.sound.as_ref())?;
    encode_optional_sound_event_holder_summary(&mut encoded, weapon.hit_sound.as_ref())?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_optional_kinetic_weapon_condition(
    out: &mut Encoder,
    condition: Option<KineticWeaponConditionSummary>,
) -> Result<()> {
    let Some(condition) = condition else {
        out.write_bool(false);
        return Ok(());
    };
    out.write_bool(true);
    out.write_var_i32(condition.max_duration_ticks);
    out.write_f32(f32::from_bits(condition.min_speed_bits));
    out.write_f32(f32::from_bits(condition.min_relative_speed_bits));
    Ok(())
}

fn encode_swing_animation(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let animation = summary.swing_animation.ok_or_else(|| {
        ProtocolError::InvalidData("missing swing_animation component value".to_string())
    })?;
    let mut encoded = Encoder::new();
    encoded.write_var_i32(animation.animation_type.id());
    encoded.write_var_i32(animation.duration);
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_blocks_attacks(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let value = summary.blocks_attacks.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing blocks_attacks component value".to_string())
    })?;
    if value.damage_reductions.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            value.damage_reductions.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    let mut encoded = Encoder::new();
    encoded.write_f32(f32::from_bits(value.block_delay_seconds_bits));
    encoded.write_f32(f32::from_bits(value.disable_cooldown_scale_bits));
    encoded.write_var_i32(value.damage_reductions.len() as i32);
    for reduction in &value.damage_reductions {
        encoded.write_f32(f32::from_bits(reduction.horizontal_blocking_angle_bits));
        encode_optional_holder_set_summary(
            &mut encoded,
            reduction.damage_types.as_ref().map(|types| {
                (
                    types.registry_ids.as_slice(),
                    types.tag.as_deref(),
                    "blocks_attacks damage reduction type",
                )
            }),
            NetworkHolderSetRegistry::DamageType,
        )?;
        encoded.write_f32(f32::from_bits(reduction.base_bits));
        encoded.write_f32(f32::from_bits(reduction.factor_bits));
    }
    encoded.write_f32(f32::from_bits(value.item_damage.threshold_bits));
    encoded.write_f32(f32::from_bits(value.item_damage.base_bits));
    encoded.write_f32(f32::from_bits(value.item_damage.factor_bits));
    encode_optional_holder_set_summary(
        &mut encoded,
        value.bypassed_by.as_ref().map(|types| {
            (
                types.registry_ids.as_slice(),
                types.tag.as_deref(),
                "blocks_attacks bypassed_by",
            )
        }),
        NetworkHolderSetRegistry::DamageType,
    )?;
    encode_optional_sound_event_holder_summary(&mut encoded, value.block_sound.as_ref())?;
    encode_optional_sound_event_holder_summary(&mut encoded, value.disable_sound.as_ref())?;

    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_optional_holder_set_summary(
    out: &mut Encoder,
    value: Option<(&[i32], Option<&str>, &'static str)>,
    registry: NetworkHolderSetRegistry,
) -> Result<()> {
    let Some((registry_ids, tag, what)) = value else {
        out.write_bool(false);
        return Ok(());
    };
    out.write_bool(true);
    encode_holder_set_summary(out, registry_ids, tag, registry, what)
}

fn encode_holder_set_summary(
    out: &mut Encoder,
    registry_ids: &[i32],
    tag: Option<&str>,
    registry: NetworkHolderSetRegistry,
    what: &str,
) -> Result<()> {
    if registry_ids.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            registry_ids.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    match (registry_ids.is_empty(), tag) {
        (true, Some(tag)) => {
            if normalize_resource_location(tag.to_string())? != tag {
                return Err(ProtocolError::InvalidData(format!(
                    "{what} has noncanonical tag resource location `{tag}`"
                )));
            }
            out.write_var_i32(0);
            encode_java_string(out, tag, MAX_STRING_CHARS)
        }
        (_, None) => {
            out.write_var_i32(registry_ids.len() as i32 + 1);
            for holder_id in registry_ids {
                registry.validate_holder_id(*holder_id)?;
                out.write_var_i32(*holder_id);
            }
            Ok(())
        }
        (false, Some(_)) => Err(ProtocolError::InvalidData(format!(
            "{what} cannot contain both holder ids and a tag"
        ))),
    }
}

fn encode_optional_sound_event_holder_summary(
    out: &mut Encoder,
    sound: Option<&SoundEventSummary>,
) -> Result<()> {
    let Some(sound) = sound else {
        out.write_bool(false);
        return Ok(());
    };
    out.write_bool(true);
    encode_sound_event_holder_summary(out, sound)
}

fn encode_sound_event_holder_summary(out: &mut Encoder, sound: &SoundEventSummary) -> Result<()> {
    match (
        sound.registry_id,
        sound.sound_id.as_deref(),
        sound.fixed_range_bits,
    ) {
        (Some(registry_id), None, None) if registry_id >= 0 && registry_id < i32::MAX => {
            out.write_var_i32(registry_id + 1);
            Ok(())
        }
        (None, Some(sound_id), fixed_range_bits) => {
            if normalize_resource_location(sound_id.to_string())? != sound_id {
                return Err(ProtocolError::InvalidData(format!(
                    "sound event has noncanonical resource location `{sound_id}`"
                )));
            }
            out.write_var_i32(0);
            encode_java_string(out, sound_id, MAX_STRING_CHARS)?;
            match fixed_range_bits {
                Some(range_bits) => {
                    out.write_bool(true);
                    out.write_f32(f32::from_bits(range_bits));
                }
                None => out.write_bool(false),
            }
            Ok(())
        }
        _ => Err(ProtocolError::InvalidData(
            "sound event holder projections are not canonical".to_string(),
        )),
    }
}

fn encode_written_book_content(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let book = summary.written_book.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing written_book_content component value".to_string())
    })?;
    if !(0..=3).contains(&book.generation) {
        return Err(ProtocolError::InvalidData(format!(
            "written book generation {} is outside 0..=3",
            book.generation
        )));
    }
    let page_count = book.pages.len();
    if page_count > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            page_count,
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    if book.page_filters.len() != page_count
        || summary.written_book_pages_nbt.len() != page_count
        || summary.written_book_page_filters_nbt.len() != page_count
    {
        return Err(ProtocolError::InvalidData(
            "written book page projections are not aligned".to_string(),
        ));
    }

    let mut encoded = Encoder::new();
    encode_filterable_java_string(
        &mut encoded,
        &book.title,
        book.title_filter.as_deref(),
        MAX_WRITTEN_BOOK_TITLE_CHARS,
    )?;
    encode_java_string(&mut encoded, &book.author, MAX_STRING_CHARS)?;
    encoded.write_var_i32(book.generation);
    encoded.write_var_i32(page_count as i32);
    for index in 0..page_count {
        let raw = summary.written_book_pages_nbt[index]
            .as_ref()
            .ok_or_else(|| {
                ProtocolError::InvalidData(format!(
                    "written book page {index} is missing raw component nbt"
                ))
            })?;
        let raw = encode_canonical_written_book_page(
            raw,
            &book.pages[index],
            &format!("written book page {index} raw"),
        )?;
        encoded.write_bytes(&raw);

        match (
            book.page_filters[index].as_deref(),
            summary.written_book_page_filters_nbt[index].as_ref(),
        ) {
            (None, None) => encoded.write_bool(false),
            (Some(text), Some(nbt)) => {
                let filtered = encode_canonical_written_book_page(
                    nbt,
                    text,
                    &format!("written book page {index} filtered"),
                )?;
                encoded.write_bool(true);
                encoded.write_bytes(&filtered);
            }
            _ => {
                return Err(ProtocolError::InvalidData(format!(
                    "written book page {index} filtered projections are not aligned"
                )));
            }
        }
    }
    encoded.write_bool(book.resolved);
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_trim(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let mut encoded = Encoder::new();
    encode_trim_material_holder(
        &mut encoded,
        summary.armor_trim_material_id,
        summary.armor_trim_material_direct.as_deref(),
    )?;
    encode_trim_pattern_holder(
        &mut encoded,
        summary.armor_trim_pattern_id,
        summary.armor_trim_pattern_direct.as_deref(),
    )?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_trim_material_holder(
    out: &mut Encoder,
    registry_id: Option<i32>,
    direct: Option<&TrimMaterialSummary>,
) -> Result<()> {
    match (registry_id, direct) {
        (Some(registry_id), None) if (0..i32::MAX).contains(&registry_id) => {
            out.write_var_i32(registry_id + 1);
            Ok(())
        }
        (None, Some(material)) => {
            if material.override_armor_assets.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(
                    material.override_armor_assets.len(),
                    MAX_DATA_COMPONENT_LIST_ITEMS,
                ));
            }
            validate_trim_material_asset_suffix(&material.asset_name, "asset_name")?;
            out.write_var_i32(0);
            encode_java_string(out, &material.asset_name, MAX_STRING_CHARS)?;
            out.write_var_i32(material.override_armor_assets.len() as i32);
            for (equipment_asset, suffix) in &material.override_armor_assets {
                validate_canonical_resource_location(
                    equipment_asset,
                    "trim material equipment asset",
                )?;
                validate_trim_material_asset_suffix(suffix, "override_armor_assets suffix")?;
                encode_java_string(out, equipment_asset, MAX_STRING_CHARS)?;
                encode_java_string(out, suffix, MAX_STRING_CHARS)?;
            }
            let description = encode_canonical_component_projection(
                material.description_nbt.as_deref(),
                &material.description,
                material.description_styled.as_deref(),
                "trim material description",
            )?;
            out.write_bytes(&description);
            Ok(())
        }
        _ => Err(ProtocolError::InvalidData(
            "trim material holder projections are not canonical".to_string(),
        )),
    }
}

fn encode_trim_pattern_holder(
    out: &mut Encoder,
    registry_id: Option<i32>,
    direct: Option<&TrimPatternSummary>,
) -> Result<()> {
    match (registry_id, direct) {
        (Some(registry_id), None) if (0..i32::MAX).contains(&registry_id) => {
            out.write_var_i32(registry_id + 1);
            Ok(())
        }
        (None, Some(pattern)) => {
            validate_canonical_resource_location(&pattern.asset_id, "trim pattern asset id")?;
            out.write_var_i32(0);
            encode_java_string(out, &pattern.asset_id, MAX_STRING_CHARS)?;
            let description = encode_canonical_component_projection(
                pattern.description_nbt.as_deref(),
                &pattern.description,
                pattern.description_styled.as_deref(),
                "trim pattern description",
            )?;
            out.write_bytes(&description);
            out.write_bool(pattern.decal);
            Ok(())
        }
        _ => Err(ProtocolError::InvalidData(
            "trim pattern holder projections are not canonical".to_string(),
        )),
    }
}

fn validate_trim_material_asset_suffix(suffix: &str, field: &str) -> Result<()> {
    if !is_valid_resource_path(suffix) {
        return Err(ProtocolError::InvalidData(format!(
            "trim material {field} `{suffix}` is not a valid resource path"
        )));
    }
    Ok(())
}

fn validate_canonical_resource_location(resource_id: &str, what: &str) -> Result<()> {
    if normalize_resource_location(resource_id.to_string())? != resource_id {
        return Err(ProtocolError::InvalidData(format!(
            "{what} has noncanonical resource location `{resource_id}`"
        )));
    }
    Ok(())
}

fn encode_canonical_component_projection(
    nbt: Option<&NbtSummaryValue>,
    expected_text: &str,
    expected_styled: Option<&[StyledTextRun]>,
    what: &str,
) -> Result<Vec<u8>> {
    let nbt =
        nbt.ok_or_else(|| ProtocolError::InvalidData(format!("{what} is missing component nbt")))?;
    let expected_styled = expected_styled.ok_or_else(|| {
        ProtocolError::InvalidData(format!("{what} is missing styled projection"))
    })?;
    let encoded = encode_nbt_summary_bounded(Some(nbt), i32::MAX as usize, MAX_NBT_DEPTH)?;
    let (canonical_runs, canonical_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder(&mut Decoder::new(&encoded))?;
    if styled_runs_summary_text(&canonical_runs) != expected_text
        || canonical_runs != expected_styled
        || canonical_nbt.as_ref() != Some(nbt)
    {
        return Err(ProtocolError::InvalidData(format!(
            "{what} contains noncanonical component projections"
        )));
    }
    Ok(encoded)
}

fn encode_filterable_java_string(
    out: &mut Encoder,
    raw: &str,
    filtered: Option<&str>,
    max_utf16_units: usize,
) -> Result<()> {
    encode_java_string(out, raw, max_utf16_units)?;
    encode_optional_java_string(out, filtered, max_utf16_units)
}

fn encode_canonical_written_book_page(
    nbt: &NbtSummaryValue,
    expected_text: &str,
    what: &str,
) -> Result<Vec<u8>> {
    let encoded = encode_nbt_summary_bounded(Some(nbt), i32::MAX as usize, MAX_NBT_DEPTH)?;
    let (runs, canonical_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder(&mut Decoder::new(&encoded))?;
    if styled_runs_summary_text(&runs) != expected_text || canonical_nbt.as_ref() != Some(nbt) {
        return Err(ProtocolError::InvalidData(format!(
            "{what} contains noncanonical component projections"
        )));
    }
    Ok(encoded)
}

pub(super) fn decode_required_compound_tag(
    decoder: &mut Decoder<'_>,
    component: &'static str,
) -> Result<NbtSummaryValue> {
    let Some(value @ NbtSummaryValue::Compound(_)) = decode_nbt_summary_from_decoder(decoder)?
    else {
        return Err(ProtocolError::InvalidData(format!(
            "{component} network value must be a non-null compound tag"
        )));
    };
    Ok(value)
}

fn encode_required_compound_tag(
    value: Option<&NbtSummaryValue>,
    component: &'static str,
    out: &mut Encoder,
) -> Result<()> {
    let value = value.ok_or_else(|| {
        ProtocolError::InvalidData(format!("missing {component} component value"))
    })?;
    if !matches!(value, NbtSummaryValue::Compound(_)) {
        return Err(ProtocolError::InvalidData(format!(
            "{component} network value must be a non-null compound tag"
        )));
    }
    let encoded = encode_nbt_summary_bounded(Some(value), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

fn encode_entity_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_typed_entity_data(
        summary.entity_data_entity_type_id,
        summary.entity_data_tag.as_ref(),
        TypedEntityDataRegistry::EntityType,
        "entity_data",
        out,
    )
}

fn encode_bucket_entity_data(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_required_compound_tag(
        summary.codec_backed_nbt.get(&type_id),
        "bucket_entity_data",
        out,
    )
}

fn encode_block_entity_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_typed_entity_data(
        summary.block_entity_data_type_id,
        summary.block_entity_data_tag.as_ref(),
        TypedEntityDataRegistry::BlockEntityType,
        "block_entity_data",
        out,
    )
}

fn encode_typed_entity_data(
    type_id: Option<i32>,
    tag: Option<&NbtSummaryValue>,
    registry: TypedEntityDataRegistry,
    component: &'static str,
    out: &mut Encoder,
) -> Result<()> {
    let type_id = type_id.ok_or_else(|| {
        ProtocolError::InvalidData(format!("missing {component} registry holder"))
    })?;
    registry.validate(type_id)?;
    if let Some(NbtSummaryValue::Compound(entries)) = tag {
        if entries.iter().any(|entry| entry.name == "id") {
            return Err(ProtocolError::InvalidData(format!(
                "{component} canonical network tag must not contain top-level id"
            )));
        }
    }
    let mut encoded = Encoder::new();
    encoded.write_var_i32(type_id);
    encode_required_compound_tag(tag, component, &mut encoded)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_profile(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let profile = summary
        .profile
        .as_ref()
        .ok_or_else(|| ProtocolError::InvalidData("missing profile component value".to_string()))?;
    if canonical_resolvable_profile_summary(profile)? != *profile {
        return Err(ProtocolError::InvalidData(
            "profile component contains noncanonical derived texture values".to_string(),
        ));
    }

    let mut encoded = Encoder::new();
    match profile.kind {
        ResolvableProfileKindSummary::GameProfile => {
            let uuid = profile.uuid.ok_or_else(|| {
                ProtocolError::InvalidData("game profile component is missing its UUID".to_string())
            })?;
            let name = profile.name.as_deref().ok_or_else(|| {
                ProtocolError::InvalidData("game profile component is missing its name".to_string())
            })?;
            encoded.write_bool(true);
            encoded.write_uuid(uuid);
            encode_java_string(&mut encoded, name, MAX_PLAYER_NAME_CHARS)?;
        }
        ResolvableProfileKindSummary::Partial => {
            encoded.write_bool(false);
            encode_optional_java_string(
                &mut encoded,
                profile.name.as_deref(),
                MAX_PLAYER_NAME_CHARS,
            )?;
            encoded.write_bool(profile.uuid.is_some());
            if let Some(uuid) = profile.uuid {
                encoded.write_uuid(uuid);
            }
        }
    }
    encode_game_profile_properties(&mut encoded, &profile.properties)?;
    encode_player_skin_patch(&mut encoded, &profile.skin_patch)?;
    out.write_bytes(&encoded.into_inner());
    Ok(())
}

fn encode_game_profile_properties(
    out: &mut Encoder,
    properties: &[GameProfilePropertySummary],
) -> Result<()> {
    if properties.len() > MAX_PROFILE_PROPERTIES {
        return Err(ProtocolError::PacketTooLarge(
            properties.len(),
            MAX_PROFILE_PROPERTIES,
        ));
    }
    out.write_var_i32(properties.len() as i32);
    for property in properties {
        encode_java_string(out, &property.name, MAX_PROFILE_PROPERTY_NAME_CHARS)?;
        encode_java_string(out, &property.value, MAX_STRING_CHARS)?;
        encode_optional_java_string(
            out,
            property.signature.as_deref(),
            MAX_PROFILE_SIGNATURE_CHARS,
        )?;
    }
    Ok(())
}

fn encode_player_skin_patch(out: &mut Encoder, patch: &PlayerSkinPatchSummary) -> Result<()> {
    for texture in [&patch.body, &patch.cape, &patch.elytra] {
        out.write_bool(texture.is_some());
        if let Some(texture) = texture {
            encode_java_string(out, &texture.asset_id, MAX_STRING_CHARS)?;
        }
    }
    out.write_bool(patch.model.is_some());
    if let Some(model) = patch.model {
        out.write_bool(model == PlayerModelTypeSummary::Slim);
    }
    Ok(())
}

fn encode_optional_java_string(
    out: &mut Encoder,
    value: Option<&str>,
    max_utf16_units: usize,
) -> Result<()> {
    out.write_bool(value.is_some());
    if let Some(value) = value {
        encode_java_string(out, value, max_utf16_units)?;
    }
    Ok(())
}

fn encode_java_string(out: &mut Encoder, value: &str, max_utf16_units: usize) -> Result<()> {
    validate_java_string_len(value, max_utf16_units)?;
    out.write_string(value);
    Ok(())
}

#[derive(Clone, Copy)]
pub(super) enum TypedEntityDataRegistry {
    EntityType,
    BlockEntityType,
}

impl TypedEntityDataRegistry {
    pub(super) const fn component_name(self) -> &'static str {
        match self {
            Self::EntityType => "entity_data",
            Self::BlockEntityType => "block_entity_data",
        }
    }

    pub(super) fn validate(self, type_id: i32) -> Result<()> {
        let (valid, registry) = match self {
            Self::EntityType => (vanilla_entity_type(type_id).is_some(), "entity type"),
            Self::BlockEntityType => (
                vanilla_block_entity_type(type_id).is_some(),
                "block entity type",
            ),
        };
        if !valid {
            return Err(ProtocolError::InvalidData(format!(
                "invalid vanilla {registry} holder id {type_id}"
            )));
        }
        Ok(())
    }
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

macro_rules! encode_var_i32_field {
    ($name:ident, $field:ident) => {
        fn $name(
            type_id: i32,
            summary: &DataComponentPatchSummary,
            out: &mut Encoder,
        ) -> Result<()> {
            let value = summary.$field.ok_or_else(|| {
                ProtocolError::InvalidData(format!(
                    "data component type id {type_id} is missing {}",
                    stringify!($field)
                ))
            })?;
            out.write_var_i32(value);
            Ok(())
        }
    };
}

encode_var_i32_field!(encode_max_stack_size, max_stack_size);
encode_var_i32_field!(encode_max_damage, max_damage);
encode_var_i32_field!(encode_damage, damage);

fn decode_unbreakable(
    _type_id: i32,
    _decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.unbreakable = true;
    Ok(())
}

fn encode_unbreakable(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    _out: &mut Encoder,
) -> Result<()> {
    if !summary.unbreakable {
        return Err(ProtocolError::InvalidData(format!(
            "data component type id {type_id} is missing unbreakable"
        )));
    }
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

fn decode_styled_name(
    decoder: &mut Decoder<'_>,
) -> Result<(String, Vec<StyledTextRun>, Option<NbtSummaryValue>)> {
    let (runs, nbt) = decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
    Ok((styled_runs_summary_text(&runs), runs, nbt))
}

fn decode_custom_name(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (text, runs, nbt) = decode_styled_name(decoder)?;
    summary.custom_name = Some(text);
    summary.custom_name_styled = Some(runs);
    summary.custom_name_nbt = nbt;
    Ok(())
}

fn decode_item_name(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (text, runs, nbt) = decode_styled_name(decoder)?;
    summary.item_name = Some(text);
    summary.item_name_styled = Some(runs);
    summary.item_name_nbt = nbt;
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

fn decode_damage_type(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.damage_type_id = Some(decode_holder_registry_id(decoder)?);
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
    (summary.lore_styled, summary.lore_nbt) = decode_lore(decoder)?;
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
    let (tooltip, persistent) = decode_adventure_mode_predicate_summary(decoder)?;
    summary.can_place_on = Some(tooltip);
    summary.can_place_on_persistent = Some(persistent);
    Ok(())
}

fn decode_can_break(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let (tooltip, persistent) = decode_adventure_mode_predicate_summary(decoder)?;
    summary.can_break = Some(tooltip);
    summary.can_break_persistent = Some(persistent);
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
    let types = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::DamageType)?;
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
    let items = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::Item)?;
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
    summary.dye_color_id = Some(normalize_vanilla_dye_color_id(decoder.read_var_i32()?));
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

fn decode_bucket_entity_data(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let value = decode_required_compound_tag(decoder, "bucket_entity_data")?;
    summary.codec_backed_nbt.insert(type_id, value);
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
    let written_book = decode_written_book_content(decoder)?;
    summary.written_book = Some(written_book.content);
    summary.written_book_pages_nbt = written_book.pages_nbt;
    summary.written_book_page_filters_nbt = written_book.page_filters_nbt;
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
    let entity_data =
        decode_typed_entity_data_summary(decoder, TypedEntityDataRegistry::EntityType)?;
    summary.entity_data_entity_type_id = Some(entity_data.type_id);
    summary.entity_data_tag = Some(entity_data.tag);
    Ok(())
}

fn decode_block_entity_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let block_entity_data =
        decode_typed_entity_data_summary(decoder, TypedEntityDataRegistry::BlockEntityType)?;
    summary.block_entity_data_type_id = Some(block_entity_data.type_id);
    summary.block_entity_spawn_entity_type =
        typed_entity_data_spawn_entity_type(Some(&block_entity_data.tag));
    summary.block_entity_data_tag = Some(block_entity_data.tag);
    Ok(())
}

fn decode_instrument(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    let instrument = decode_instrument_component_summary(decoder)?;
    summary.instrument_id = instrument.registry_id;
    if let Some(direct) = instrument.direct {
        summary.instrument_description = Some(direct.description.clone());
        summary.instrument_description_styled = direct
            .description_styled
            .as_deref()
            .map(<[StyledTextRun]>::to_vec);
        summary.instrument_direct = Some(Box::new(direct));
    }
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
    let patterns = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::BannerPattern)?;
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
    summary.base_color_id = Some(normalize_vanilla_dye_color_id(decoder.read_var_i32()?));
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
    let id = normalize_builtin_enum_component_value_id(type_id, id).ok_or_else(|| {
        ProtocolError::InvalidData(format!(
            "unsupported built-in enum component type id {type_id}"
        ))
    })?;
    summary.builtin_enum_component_ids.insert(type_id, id);
    Ok(())
}

fn decode_tropical_fish_pattern(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tropical_fish_pattern_id =
        Some(vanilla_tropical_fish_pattern(decoder.read_var_i32()?).packed_id);
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

fn decode_tool_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.tool = Some(decode_tool(decoder)?);
    Ok(())
}

fn decode_equippable_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.equippable = Some(decode_equippable(decoder)?);
    Ok(())
}

fn decode_blocks_attacks_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
) -> Result<()> {
    summary.blocks_attacks = Some(decode_blocks_attacks(decoder)?);
    Ok(())
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
    use std::collections::{BTreeMap, HashSet};

    use crate::{
        block_entity_types::{
            VANILLA_BLOCK_ENTITY_TYPES, VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
        },
        codec::Encoder,
        entity_types::{VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_ZOMBIE_ID},
    };

    use super::*;

    #[test]
    fn handler_registry_binds_every_generated_metadata_entry_in_order() {
        let descriptors = vanilla_data_component_descriptors();
        assert_eq!(descriptors.len(), 110);
        assert_eq!(WIRE_DECODE_HANDLERS.len(), 110);
        assert_eq!(descriptors.len(), VANILLA_DATA_COMPONENT_TYPES.len());
        let mut type_ids = HashSet::new();
        let mut resource_ids = HashSet::new();
        for (expected_id, (descriptor, metadata)) in descriptors
            .iter()
            .zip(VANILLA_DATA_COMPONENT_TYPES)
            .enumerate()
        {
            assert_eq!(descriptor.type_id(), expected_id as i32);
            assert_eq!(descriptor.metadata, metadata);
            assert!(type_ids.insert(descriptor.type_id()));
            assert!(resource_ids.insert(descriptor.resource_id()));
        }
        assert_eq!(
            descriptors
                .iter()
                .filter(|descriptor| !descriptor.is_persistent())
                .map(|descriptor| descriptor.type_id())
                .collect::<Vec<_>>(),
            vec![20, 41, 48]
        );
        assert_eq!(
            descriptors
                .iter()
                .filter(|descriptor| descriptor.can_encode())
                .map(|descriptor| descriptor.type_id())
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4, 16, 24, 36, 37, 38, 39, 40, 55, 56, 58, 59, 60, 70,]
        );
    }

    #[test]
    fn attribute_modifiers_descriptor_round_trips_exact_display_variants() {
        let display_text_nbt = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Override".to_string()),
            },
            NbtSummaryEntry {
                name: "bold".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
        ]);
        let display_text_styled = vec![StyledTextRun {
            text: "Override".to_string(),
            style: crate::ComponentStyle {
                bold: Some(true),
                ..crate::ComponentStyle::default()
            },
        }];
        let summary = DataComponentPatchSummary {
            attribute_modifiers: vec![
                AttributeModifierSummary {
                    attribute_id: crate::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                    modifier_id: "minecraft:test/default".to_string(),
                    amount_bits: (-0.0f64).to_bits(),
                    operation_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
                    slot_id: VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
                    display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
                    display_text: None,
                    display_text_styled: None,
                    display_text_nbt: None,
                },
                AttributeModifierSummary {
                    attribute_id:
                        crate::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
                    modifier_id: "minecraft:test/hidden".to_string(),
                    amount_bits: f64::INFINITY.to_bits(),
                    operation_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
                    slot_id:
                        crate::equipment_slots::VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
                    display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
                    display_text: None,
                    display_text_styled: None,
                    display_text_nbt: None,
                },
                AttributeModifierSummary {
                    attribute_id: crate::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                    modifier_id: "example:override".to_string(),
                    amount_bits: 1.5f64.to_bits(),
                    operation_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                    slot_id: 7,
                    display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                    display_text: Some("Override".to_string()),
                    display_text_styled: Some(display_text_styled),
                    display_text_nbt: Some(display_text_nbt.clone()),
                },
            ],
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let mut expected = Encoder::new();
        expected.write_var_i32(3);
        for (attribute, id, amount, operation, slot, display) in [
            (
                crate::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                "minecraft:test/default",
                -0.0,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
                VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
            ),
            (
                crate::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
                "minecraft:test/hidden",
                f64::INFINITY,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
                crate::equipment_slots::VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
            ),
            (
                crate::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                "example:override",
                1.5,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                7,
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
            ),
        ] {
            expected.write_var_i32(attribute);
            expected.write_string(id);
            expected.write_f64(amount);
            expected.write_var_i32(operation);
            expected.write_var_i32(slot);
            expected.write_var_i32(display);
            if display
                == crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID
            {
                expected.write_bytes(
                    &encode_nbt_summary_bounded(
                        Some(&display_text_nbt),
                        i32::MAX as usize,
                        MAX_NBT_DEPTH,
                    )
                    .unwrap(),
                );
            }
        }
        assert_eq!(encoded, expected.into_inner());

        let mut decoder = Decoder::new(&encoded);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.attribute_modifiers, summary.attribute_modifiers);
    }

    #[test]
    fn attribute_modifiers_descriptor_rejects_noncanonical_summaries_atomically() {
        let valid = AttributeModifierSummary {
            attribute_id: crate::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
            modifier_id: "minecraft:test".to_string(),
            amount_bits: 1.0f64.to_bits(),
            operation_id:
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            slot_id: VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
            display_id:
                crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
            display_text: None,
            display_text_styled: None,
            display_text_nbt: None,
        };
        let invalid = [
            (
                AttributeModifierSummary {
                    attribute_id: i32::try_from(crate::attribute_types::VANILLA_ATTRIBUTE_TYPES.len())
                        .unwrap(),
                    ..valid.clone()
                },
                "invalid vanilla attribute holder",
            ),
            (
                AttributeModifierSummary {
                    modifier_id: "test".to_string(),
                    ..valid.clone()
                },
                "noncanonical attribute modifier resource location",
            ),
            (
                AttributeModifierSummary {
                    operation_id: 3,
                    ..valid.clone()
                },
                "invalid attribute modifier operation",
            ),
            (
                AttributeModifierSummary {
                    slot_id: 99,
                    ..valid.clone()
                },
                "invalid equipment slot group",
            ),
            (
                AttributeModifierSummary {
                    display_id:
                        crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
                    display_text: Some("dangling".to_string()),
                    ..valid.clone()
                },
                "cannot contain override text",
            ),
            (
                AttributeModifierSummary {
                    display_id:
                        crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                    ..valid.clone()
                },
                "missing component nbt",
            ),
            (
                AttributeModifierSummary {
                    display_id:
                        crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                    display_text: Some("stale".to_string()),
                    display_text_styled: Some(vec![StyledTextRun {
                        text: "wire".to_string(),
                        style: crate::ComponentStyle::default(),
                    }]),
                    display_text_nbt: Some(NbtSummaryValue::String("wire".to_string())),
                    ..valid.clone()
                },
                "noncanonical component projections",
            ),
            (
                AttributeModifierSummary {
                    display_id:
                        crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                    display_text: Some("invalid".to_string()),
                    display_text_styled: Some(Vec::new()),
                    display_text_nbt: Some(NbtSummaryValue::Int(1)),
                    ..valid.clone()
                },
                "is not a component",
            ),
            (
                AttributeModifierSummary {
                    display_id: 3,
                    ..valid.clone()
                },
                "invalid attribute modifier display",
            ),
        ];
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID)
                .unwrap();
        for (modifier, expected_error) in invalid {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor
                .encode(
                    &DataComponentPatchSummary {
                        attribute_modifiers: vec![modifier],
                        ..DataComponentPatchSummary::default()
                    },
                    &mut encoded,
                )
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = descriptor
            .encode(
                &DataComponentPatchSummary {
                    attribute_modifiers: vec![valid; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                    ..DataComponentPatchSummary::default()
                },
                &mut encoded,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(actual, max)
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn consumable_descriptor_round_trips_all_effect_branches_in_wire_order() {
        let nan_bits = 0x7fc0_1234;
        let summary = DataComponentPatchSummary {
            consumable: Some(ConsumableSummary {
                consume_seconds: f32::from_bits(nan_bits),
                animation: ItemUseAnimationSummary::Spear,
            }),
            consumable_sound: Some(SoundEventSummary {
                registry_id: Some(8),
                sound_id: None,
                fixed_range_bits: None,
            }),
            consumable_has_consume_particles: Some(false),
            consumable_effect_count: Some(5),
            consumable_effects: vec![
                ConsumeEffectSummary::ApplyEffects {
                    effects: vec![MobEffectInstanceSummary {
                        effect_id: 1,
                        amplifier: 255,
                        duration: -5,
                        ambient: true,
                        show_particles: false,
                        show_icon: true,
                        hidden_effect: Some(Box::new(MobEffectDetailsSummary {
                            amplifier: 0,
                            duration: 10,
                            ambient: false,
                            show_particles: true,
                            show_icon: false,
                            hidden_effect: None,
                        })),
                    }],
                    probability_bits: nan_bits,
                },
                ConsumeEffectSummary::RemoveEffects {
                    effect_registry_ids: Vec::new(),
                    effect_tag: Some("minecraft:beneficial".to_string()),
                },
                ConsumeEffectSummary::ClearAllEffects,
                ConsumeEffectSummary::TeleportRandomly {
                    diameter_bits: (-0.0_f32).to_bits(),
                },
                ConsumeEffectSummary::PlaySound {
                    sound: SoundEventSummary {
                        registry_id: None,
                        sound_id: Some("example:consume".to_string()),
                        fixed_range_bits: Some(f32::INFINITY.to_bits()),
                    },
                },
            ],
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let mut expected = Encoder::new();
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_var_i32(ItemUseAnimationSummary::Spear.id());
        expected.write_var_i32(9);
        expected.write_bool(false);
        expected.write_var_i32(5);
        expected.write_var_i32(
            crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_APPLY_EFFECTS_ID,
        );
        expected.write_var_i32(1);
        expected.write_var_i32(1);
        expected.write_var_i32(255);
        expected.write_var_i32(-5);
        expected.write_bool(true);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_var_i32(10);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_bool(false);
        expected.write_bool(false);
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_var_i32(
            crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID,
        );
        expected.write_var_i32(0);
        expected.write_string("minecraft:beneficial");
        expected.write_var_i32(
            crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_CLEAR_ALL_EFFECTS_ID,
        );
        expected.write_var_i32(
            crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_TELEPORT_RANDOMLY_ID,
        );
        expected.write_f32(-0.0);
        expected
            .write_var_i32(crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID);
        expected.write_var_i32(0);
        expected.write_string("example:consume");
        expected.write_bool(true);
        expected.write_f32(f32::INFINITY);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.consumable, summary.consumable);
        assert_eq!(decoded.consumable_sound, summary.consumable_sound);
        assert_eq!(
            decoded.consumable_has_consume_particles,
            summary.consumable_has_consume_particles
        );
        assert_eq!(
            decoded.consumable_effect_count,
            summary.consumable_effect_count
        );
        assert_eq!(decoded.consumable_effects, summary.consumable_effects);
    }

    #[test]
    fn death_protection_descriptor_round_trips_direct_holders_and_empty_lists() {
        let summary = DataComponentPatchSummary {
            death_protection_effects: vec![
                ConsumeEffectSummary::RemoveEffects {
                    effect_registry_ids: vec![0, 3],
                    effect_tag: None,
                },
                ConsumeEffectSummary::PlaySound {
                    sound: SoundEventSummary {
                        registry_id: Some(12),
                        sound_id: None,
                        fixed_range_bits: None,
                    },
                },
            ],
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let mut expected = Encoder::new();
        expected.write_var_i32(2);
        expected.write_var_i32(
            crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID,
        );
        expected.write_var_i32(3);
        expected.write_var_i32(0);
        expected.write_var_i32(3);
        expected
            .write_var_i32(crate::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID);
        expected.write_var_i32(13);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.death_protection_effects,
            summary.death_protection_effects
        );

        let mut empty = Encoder::new();
        descriptor
            .encode(&DataComponentPatchSummary::default(), &mut empty)
            .unwrap();
        assert_eq!(empty.into_inner(), [0]);
    }

    #[test]
    fn consume_effect_descriptors_enforce_bounds_and_canonical_values_atomically() {
        fn mob_effect() -> MobEffectInstanceSummary {
            MobEffectInstanceSummary {
                effect_id: 0,
                amplifier: 0,
                duration: 20,
                ambient: false,
                show_particles: true,
                show_icon: true,
                hidden_effect: None,
            }
        }

        fn valid_consumable() -> DataComponentPatchSummary {
            DataComponentPatchSummary {
                consumable: Some(ConsumableSummary {
                    consume_seconds: 1.6,
                    animation: ItemUseAnimationSummary::Eat,
                }),
                consumable_sound: Some(SoundEventSummary {
                    registry_id: Some(0),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
                consumable_has_consume_particles: Some(true),
                consumable_effect_count: Some(1),
                consumable_effects: vec![ConsumeEffectSummary::ClearAllEffects],
                ..DataComponentPatchSummary::default()
            }
        }

        let consumable_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID).unwrap();
        let death_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID)
                .unwrap();
        let mut cases = Vec::new();
        cases.push((
            DataComponentPatchSummary::default(),
            "missing consumable component value",
        ));
        let mut missing_sound = valid_consumable();
        missing_sound.consumable_sound = None;
        cases.push((missing_sound, "missing sound holder"));
        let mut missing_particles = valid_consumable();
        missing_particles.consumable_has_consume_particles = None;
        cases.push((missing_particles, "missing particle flag"));
        let mut missing_count = valid_consumable();
        missing_count.consumable_effect_count = None;
        cases.push((missing_count, "missing effect count"));
        let mut stale_count = valid_consumable();
        stale_count.consumable_effect_count = Some(0);
        cases.push((stale_count, "does not match 1 effects"));
        let mut malformed_sound = valid_consumable();
        malformed_sound.consumable_sound = Some(SoundEventSummary {
            registry_id: Some(0),
            sound_id: Some("minecraft:entity.generic.eat".to_string()),
            fixed_range_bits: None,
        });
        cases.push((malformed_sound, "sound event holder projections"));
        let mut invalid_effect_id = valid_consumable();
        invalid_effect_id.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
            effects: vec![MobEffectInstanceSummary {
                effect_id: i32::try_from(crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len())
                    .unwrap(),
                ..mob_effect()
            }],
            probability_bits: 1.0_f32.to_bits(),
        }];
        cases.push((invalid_effect_id, "invalid vanilla mob effect holder"));
        let mut invalid_amplifier = valid_consumable();
        invalid_amplifier.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
            effects: vec![MobEffectInstanceSummary {
                amplifier: 256,
                ..mob_effect()
            }],
            probability_bits: 1.0_f32.to_bits(),
        }];
        cases.push((invalid_amplifier, "amplifier 256 is outside"));
        let mut hidden_effect = None;
        for _ in 0..=MAX_MOB_EFFECT_DETAILS_DEPTH {
            hidden_effect = Some(Box::new(MobEffectDetailsSummary {
                amplifier: 0,
                duration: 1,
                ambient: false,
                show_particles: true,
                show_icon: true,
                hidden_effect,
            }));
        }
        let mut excessive_depth = valid_consumable();
        excessive_depth.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
            effects: vec![MobEffectInstanceSummary {
                hidden_effect,
                ..mob_effect()
            }],
            probability_bits: 1.0_f32.to_bits(),
        }];
        cases.push((excessive_depth, "exceeded max depth"));
        let mut mixed_holder_set = valid_consumable();
        mixed_holder_set.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids: vec![0],
            effect_tag: Some("minecraft:beneficial".to_string()),
        }];
        cases.push((mixed_holder_set, "both holder ids and a tag"));
        let mut noncanonical_tag = valid_consumable();
        noncanonical_tag.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids: Vec::new(),
            effect_tag: Some("beneficial".to_string()),
        }];
        cases.push((noncanonical_tag, "noncanonical tag resource location"));
        let mut negative_holder = valid_consumable();
        negative_holder.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids: vec![-1],
            effect_tag: None,
        }];
        cases.push((negative_holder, "invalid mob effect holder id -1"));

        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = consumable_descriptor
                .encode(&summary, &mut encoded)
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let mut maximal_outer = valid_consumable();
        maximal_outer.consumable_effect_count = Some(MAX_DATA_COMPONENT_LIST_ITEMS);
        maximal_outer.consumable_effects =
            vec![ConsumeEffectSummary::ClearAllEffects; MAX_DATA_COMPONENT_LIST_ITEMS];
        consumable_descriptor
            .encode(&maximal_outer, &mut Encoder::new())
            .unwrap();
        let mut maximal_apply = valid_consumable();
        maximal_apply.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
            effects: vec![mob_effect(); MAX_DATA_COMPONENT_LIST_ITEMS],
            probability_bits: 1.0_f32.to_bits(),
        }];
        consumable_descriptor
            .encode(&maximal_apply, &mut Encoder::new())
            .unwrap();
        let mut maximal_holder_set = valid_consumable();
        maximal_holder_set.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS],
            effect_tag: None,
        }];
        consumable_descriptor
            .encode(&maximal_holder_set, &mut Encoder::new())
            .unwrap();

        for mut oversized in [
            {
                let mut value = valid_consumable();
                value.consumable_effect_count = Some(MAX_DATA_COMPONENT_LIST_ITEMS + 1);
                value.consumable_effects =
                    vec![ConsumeEffectSummary::ClearAllEffects; MAX_DATA_COMPONENT_LIST_ITEMS + 1];
                value
            },
            {
                let mut value = valid_consumable();
                value.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
                    effects: vec![mob_effect(); MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                    probability_bits: 1.0_f32.to_bits(),
                }];
                value
            },
            {
                let mut value = valid_consumable();
                value.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
                    effect_registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                    effect_tag: None,
                }];
                value
            },
        ] {
            oversized.consumable_effect_count = Some(oversized.consumable_effects.len());
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            assert!(matches!(
                consumable_descriptor.encode(&oversized, &mut encoded),
                Err(ProtocolError::PacketTooLarge(actual, max))
                    if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                        && max == MAX_DATA_COMPONENT_LIST_ITEMS
            ));
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let oversized_death = DataComponentPatchSummary {
            death_protection_effects: vec![
                ConsumeEffectSummary::ClearAllEffects;
                MAX_DATA_COMPONENT_LIST_ITEMS + 1
            ],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            death_descriptor.encode(&oversized_death, &mut encoded),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn weapon_interaction_descriptors_round_trip_exact_wire_values() {
        let nan_bits = 0x7fc0_1234;
        let piercing = PiercingWeaponSummary {
            deals_knockback: false,
            dismounts: true,
            sound: Some(SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:pierce".to_string()),
                fixed_range_bits: Some(nan_bits),
            }),
            hit_sound: Some(SoundEventSummary {
                registry_id: Some(7),
                sound_id: None,
                fixed_range_bits: None,
            }),
        };
        let kinetic = KineticWeaponSummary {
            contact_cooldown_ticks: -1,
            delay_ticks: -2,
            dismount_conditions: Some(KineticWeaponConditionSummary {
                max_duration_ticks: -3,
                min_speed_bits: (-0.0_f32).to_bits(),
                min_relative_speed_bits: f32::INFINITY.to_bits(),
            }),
            knockback_conditions: None,
            damage_conditions: Some(KineticWeaponConditionSummary {
                max_duration_ticks: 4,
                min_speed_bits: nan_bits,
                min_relative_speed_bits: 0.25_f32.to_bits(),
            }),
            forward_movement_bits: nan_bits,
            damage_multiplier_bits: (-0.0_f32).to_bits(),
            sound: None,
            hit_sound: Some(SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:hit".to_string()),
                fixed_range_bits: None,
            }),
        };
        let swing = SwingAnimationSummary {
            animation_type: SwingAnimationTypeSummary::Stab,
            duration: -4,
        };
        let summary = DataComponentPatchSummary {
            piercing_weapon: Some(piercing.clone()),
            kinetic_weapon: Some(kinetic.clone()),
            swing_animation: Some(swing),
            ..DataComponentPatchSummary::default()
        };

        let piercing_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID)
                .unwrap();
        let kinetic_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID)
                .unwrap();
        let swing_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID)
                .unwrap();

        let mut piercing_bytes = Encoder::new();
        piercing_descriptor
            .encode(&summary, &mut piercing_bytes)
            .unwrap();
        let piercing_bytes = piercing_bytes.into_inner();
        let mut expected = Encoder::new();
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_string("example:pierce");
        expected.write_bool(true);
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_bool(true);
        expected.write_var_i32(8);
        assert_eq!(piercing_bytes, expected.into_inner());
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&piercing_bytes);
        piercing_descriptor
            .decode(&mut decoder, &mut decoded)
            .unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.piercing_weapon, Some(piercing));

        let mut kinetic_bytes = Encoder::new();
        kinetic_descriptor
            .encode(&summary, &mut kinetic_bytes)
            .unwrap();
        let kinetic_bytes = kinetic_bytes.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(-1);
        expected.write_var_i32(-2);
        expected.write_bool(true);
        expected.write_var_i32(-3);
        expected.write_f32(-0.0);
        expected.write_f32(f32::INFINITY);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_var_i32(4);
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_f32(0.25);
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_f32(-0.0);
        expected.write_bool(false);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_string("example:hit");
        expected.write_bool(false);
        assert_eq!(kinetic_bytes, expected.into_inner());
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&kinetic_bytes);
        kinetic_descriptor
            .decode(&mut decoder, &mut decoded)
            .unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.kinetic_weapon, Some(kinetic));

        let mut swing_bytes = Encoder::new();
        swing_descriptor.encode(&summary, &mut swing_bytes).unwrap();
        let swing_bytes = swing_bytes.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(SwingAnimationTypeSummary::Stab.id());
        expected.write_var_i32(-4);
        assert_eq!(swing_bytes, expected.into_inner());
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&swing_bytes);
        swing_descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.swing_animation, Some(swing));
    }

    #[test]
    fn weapon_interaction_descriptors_reject_noncanonical_summaries_atomically() {
        let piercing_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID)
                .unwrap();
        let kinetic_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID)
                .unwrap();
        let swing_descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID)
                .unwrap();

        for (descriptor, expected_error) in [
            (piercing_descriptor, "missing piercing_weapon"),
            (kinetic_descriptor, "missing kinetic_weapon"),
            (swing_descriptor, "missing swing_animation"),
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor
                .encode(&DataComponentPatchSummary::default(), &mut encoded)
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let malformed_piercing = DataComponentPatchSummary {
            piercing_weapon: Some(PiercingWeaponSummary {
                deals_knockback: true,
                dismounts: false,
                sound: Some(SoundEventSummary {
                    registry_id: Some(0),
                    sound_id: Some("minecraft:item.spear.attack".to_string()),
                    fixed_range_bits: None,
                }),
                hit_sound: None,
            }),
            ..DataComponentPatchSummary::default()
        };
        let malformed_kinetic = DataComponentPatchSummary {
            kinetic_weapon: Some(KineticWeaponSummary {
                contact_cooldown_ticks: 10,
                delay_ticks: 0,
                dismount_conditions: None,
                knockback_conditions: None,
                damage_conditions: None,
                forward_movement_bits: 0.0_f32.to_bits(),
                damage_multiplier_bits: 1.0_f32.to_bits(),
                sound: None,
                hit_sound: Some(SoundEventSummary {
                    registry_id: Some(-1),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
            }),
            ..DataComponentPatchSummary::default()
        };
        for (descriptor, summary) in [
            (piercing_descriptor, &malformed_piercing),
            (kinetic_descriptor, &malformed_kinetic),
        ] {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(summary, &mut encoded).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("sound event holder projections are not canonical"),
                "{error}"
            );
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
    }

    #[test]
    fn blocks_attacks_descriptor_round_trips_holder_and_sound_unions_in_wire_order() {
        let nan_bits = 0x7fc0_1234;
        let summary = DataComponentPatchSummary {
            blocks_attacks: Some(BlocksAttacksSummary {
                block_delay_seconds_bits: (-0.0_f32).to_bits(),
                disable_cooldown_scale_bits: f32::INFINITY.to_bits(),
                damage_reductions: vec![
                    DamageReductionSummary {
                        horizontal_blocking_angle_bits: 45.0_f32.to_bits(),
                        damage_types: Some(DamageTypeHolderSetSummary {
                            registry_ids: Vec::new(),
                            tag: Some("minecraft:bypasses_shield".to_string()),
                        }),
                        base_bits: nan_bits,
                        factor_bits: (-1.25_f32).to_bits(),
                    },
                    DamageReductionSummary {
                        horizontal_blocking_angle_bits: 180.0_f32.to_bits(),
                        damage_types: Some(DamageTypeHolderSetSummary {
                            registry_ids: vec![0, 47],
                            tag: None,
                        }),
                        base_bits: 2.0_f32.to_bits(),
                        factor_bits: 0.5_f32.to_bits(),
                    },
                ],
                item_damage: ItemDamageFunctionSummary {
                    threshold_bits: (-2.0_f32).to_bits(),
                    base_bits: 3.0_f32.to_bits(),
                    factor_bits: nan_bits,
                },
                bypassed_by: Some(DamageTypeHolderSetSummary {
                    registry_ids: Vec::new(),
                    tag: None,
                }),
                block_sound: Some(SoundEventSummary {
                    registry_id: Some(11),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
                disable_sound: Some(SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:shield_disable".to_string()),
                    fixed_range_bits: Some((-0.0_f32).to_bits()),
                }),
            }),
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let mut expected = Encoder::new();
        expected.write_f32(-0.0);
        expected.write_f32(f32::INFINITY);
        expected.write_var_i32(2);
        expected.write_f32(45.0);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_string("minecraft:bypasses_shield");
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_f32(-1.25);
        expected.write_f32(180.0);
        expected.write_bool(true);
        expected.write_var_i32(3);
        expected.write_var_i32(0);
        expected.write_var_i32(47);
        expected.write_f32(2.0);
        expected.write_f32(0.5);
        expected.write_f32(-2.0);
        expected.write_f32(3.0);
        expected.write_f32(f32::from_bits(nan_bits));
        expected.write_bool(true);
        expected.write_var_i32(1);
        expected.write_bool(true);
        expected.write_var_i32(12);
        expected.write_bool(true);
        expected.write_var_i32(0);
        expected.write_string("example:shield_disable");
        expected.write_bool(true);
        expected.write_f32(-0.0);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.blocks_attacks, summary.blocks_attacks);
    }

    #[test]
    fn blocks_attacks_descriptor_enforces_bounds_and_canonical_unions_atomically() {
        let reduction = DamageReductionSummary {
            horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
            damage_types: None,
            base_bits: 0.0_f32.to_bits(),
            factor_bits: 1.0_f32.to_bits(),
        };
        let valid = DataComponentPatchSummary {
            blocks_attacks: Some(BlocksAttacksSummary {
                block_delay_seconds_bits: 0.0_f32.to_bits(),
                disable_cooldown_scale_bits: 1.0_f32.to_bits(),
                damage_reductions: vec![reduction.clone()],
                item_damage: ItemDamageFunctionSummary {
                    threshold_bits: 1.0_f32.to_bits(),
                    base_bits: 0.0_f32.to_bits(),
                    factor_bits: 1.0_f32.to_bits(),
                },
                bypassed_by: None,
                block_sound: None,
                disable_sound: None,
            }),
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID)
                .unwrap();

        let mut empty = valid.clone();
        empty
            .blocks_attacks
            .as_mut()
            .unwrap()
            .damage_reductions
            .clear();
        descriptor.encode(&empty, &mut Encoder::new()).unwrap();
        let mut maximal = valid.clone();
        maximal.blocks_attacks.as_mut().unwrap().damage_reductions =
            vec![reduction; MAX_DATA_COMPONENT_LIST_ITEMS];
        descriptor.encode(&maximal, &mut Encoder::new()).unwrap();
        let mut maximal_holder_set = valid.clone();
        maximal_holder_set
            .blocks_attacks
            .as_mut()
            .unwrap()
            .bypassed_by = Some(DamageTypeHolderSetSummary {
            registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS],
            tag: None,
        });
        descriptor
            .encode(&maximal_holder_set, &mut Encoder::new())
            .unwrap();

        let mut cases = Vec::new();
        cases.push((
            DataComponentPatchSummary::default(),
            "missing blocks_attacks",
        ));
        let mut holder_union = valid.clone();
        holder_union.blocks_attacks.as_mut().unwrap().bypassed_by =
            Some(DamageTypeHolderSetSummary {
                registry_ids: vec![0],
                tag: Some("minecraft:bypasses_shield".to_string()),
            });
        cases.push((holder_union, "both holder ids and a tag"));
        let mut negative_holder = valid.clone();
        negative_holder
            .blocks_attacks
            .as_mut()
            .unwrap()
            .damage_reductions[0]
            .damage_types = Some(DamageTypeHolderSetSummary {
            registry_ids: vec![-1],
            tag: None,
        });
        cases.push((negative_holder, "invalid damage type holder id -1"));
        let mut noncanonical_tag = valid.clone();
        noncanonical_tag
            .blocks_attacks
            .as_mut()
            .unwrap()
            .bypassed_by = Some(DamageTypeHolderSetSummary {
            registry_ids: Vec::new(),
            tag: Some("bypasses_shield".to_string()),
        });
        cases.push((noncanonical_tag, "noncanonical tag resource location"));
        let mut mixed_sound = valid.clone();
        mixed_sound.blocks_attacks.as_mut().unwrap().block_sound = Some(SoundEventSummary {
            registry_id: Some(0),
            sound_id: Some("minecraft:item.shield.block".to_string()),
            fixed_range_bits: None,
        });
        cases.push((mixed_sound, "sound event holder projections"));
        let mut missing_sound = valid.clone();
        missing_sound.blocks_attacks.as_mut().unwrap().block_sound = Some(SoundEventSummary {
            registry_id: None,
            sound_id: None,
            fixed_range_bits: None,
        });
        cases.push((missing_sound, "sound event holder projections"));
        let mut invalid_sound_id = valid.clone();
        invalid_sound_id
            .blocks_attacks
            .as_mut()
            .unwrap()
            .block_sound = Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some("Invalid Path".to_string()),
            fixed_range_bits: None,
        });
        cases.push((invalid_sound_id, "invalid resource"));
        let mut noncanonical_sound_id = valid.clone();
        noncanonical_sound_id
            .blocks_attacks
            .as_mut()
            .unwrap()
            .block_sound = Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some("item.shield.block".to_string()),
            fixed_range_bits: None,
        });
        cases.push((noncanonical_sound_id, "noncanonical resource location"));
        let mut overflowing_sound_holder = valid.clone();
        overflowing_sound_holder
            .blocks_attacks
            .as_mut()
            .unwrap()
            .block_sound = Some(SoundEventSummary {
            registry_id: Some(i32::MAX),
            sound_id: None,
            fixed_range_bits: None,
        });
        cases.push((overflowing_sound_holder, "sound event holder projections"));

        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let mut oversized_holder_set = valid.clone();
        oversized_holder_set
            .blocks_attacks
            .as_mut()
            .unwrap()
            .bypassed_by = Some(DamageTypeHolderSetSummary {
            registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
            tag: None,
        });
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            descriptor.encode(&oversized_holder_set, &mut encoded),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);

        let mut oversized = valid;
        oversized.blocks_attacks.as_mut().unwrap().damage_reductions = vec![
            DamageReductionSummary {
                horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
                damage_types: None,
                base_bits: 0.0_f32.to_bits(),
                factor_bits: 1.0_f32.to_bits(),
            };
            MAX_DATA_COMPONENT_LIST_ITEMS
                + 1
        ];
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            descriptor.encode(&oversized, &mut encoded),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    fn canonical_trim_description(nbt: &NbtSummaryValue) -> (String, Box<[StyledTextRun]>) {
        let encoded =
            encode_nbt_summary_bounded(Some(nbt), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        let (runs, canonical_nbt) =
            decode_styled_component_summary_with_nbt_from_decoder(&mut Decoder::new(&encoded))
                .unwrap();
        assert_eq!(canonical_nbt.as_ref(), Some(nbt));
        (styled_runs_summary_text(&runs), runs.into_boxed_slice())
    }

    fn direct_trim_patch() -> DataComponentPatchSummary {
        let material_nbt = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Ruby".to_string()),
            },
            NbtSummaryEntry {
                name: "color".to_string(),
                value: NbtSummaryValue::String("red".to_string()),
            },
        ]);
        let pattern_nbt = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Spiral".to_string()),
            },
            NbtSummaryEntry {
                name: "italic".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
        ]);
        let (material_description, material_styled) = canonical_trim_description(&material_nbt);
        let (pattern_description, pattern_styled) = canonical_trim_description(&pattern_nbt);
        DataComponentPatchSummary {
            armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
                asset_name: "ruby".to_string(),
                override_armor_assets: BTreeMap::from([
                    ("minecraft:netherite".to_string(), "ruby_darker".to_string()),
                    ("example:opal".to_string(), "ruby_opal".to_string()),
                ]),
                description: material_description,
                description_styled: Some(material_styled),
                description_nbt: Some(Box::new(material_nbt)),
            })),
            armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
                asset_id: "example:spiral".to_string(),
                description: pattern_description,
                description_styled: Some(pattern_styled),
                description_nbt: Some(Box::new(pattern_nbt)),
                decal: true,
            })),
            ..DataComponentPatchSummary::default()
        }
    }

    #[test]
    fn trim_descriptor_round_trips_registry_and_direct_holders_in_wire_order() {
        let direct = direct_trim_patch();
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TRIM_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&direct, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let material = direct.armor_trim_material_direct.as_deref().unwrap();
        let pattern = direct.armor_trim_pattern_direct.as_deref().unwrap();
        let mut expected = Encoder::new();
        expected.write_var_i32(0);
        expected.write_string("ruby");
        expected.write_var_i32(2);
        for (equipment_asset, suffix) in &material.override_armor_assets {
            expected.write_string(equipment_asset);
            expected.write_string(suffix);
        }
        expected.write_bytes(
            &encode_nbt_summary_bounded(
                material.description_nbt.as_deref(),
                i32::MAX as usize,
                MAX_NBT_DEPTH,
            )
            .unwrap(),
        );
        expected.write_var_i32(0);
        expected.write_string("example:spiral");
        expected.write_bytes(
            &encode_nbt_summary_bounded(
                pattern.description_nbt.as_deref(),
                i32::MAX as usize,
                MAX_NBT_DEPTH,
            )
            .unwrap(),
        );
        expected.write_bool(true);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.armor_trim_material_direct,
            direct.armor_trim_material_direct
        );
        assert_eq!(
            decoded.armor_trim_pattern_direct,
            direct.armor_trim_pattern_direct
        );

        let referenced = DataComponentPatchSummary {
            armor_trim_material_id: Some(0),
            armor_trim_pattern_id: Some(i32::MAX - 1),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&referenced, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(1);
        expected.write_var_i32(i32::MAX);
        assert_eq!(encoded, expected.into_inner());
        let mut decoded = DataComponentPatchSummary::default();
        descriptor
            .decode(&mut Decoder::new(&encoded), &mut decoded)
            .unwrap();
        assert_eq!(decoded.armor_trim_material_id, Some(0));
        assert_eq!(decoded.armor_trim_pattern_id, Some(i32::MAX - 1));

        let mixed = DataComponentPatchSummary {
            armor_trim_material_id: Some(41),
            armor_trim_pattern_direct: direct.armor_trim_pattern_direct.clone(),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&mixed, &mut encoded).unwrap();
        let mut decoded = DataComponentPatchSummary::default();
        descriptor
            .decode(&mut Decoder::new(&encoded.into_inner()), &mut decoded)
            .unwrap();
        assert_eq!(decoded.armor_trim_material_id, Some(41));
        assert_eq!(
            decoded.armor_trim_pattern_direct,
            mixed.armor_trim_pattern_direct
        );
    }

    #[test]
    fn trim_descriptor_rejects_noncanonical_holders_and_payloads_atomically() {
        let valid = direct_trim_patch();
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TRIM_TYPE_ID).unwrap();
        let mut cases = Vec::new();

        let mut missing_material = valid.clone();
        missing_material.armor_trim_material_direct = None;
        cases.push((missing_material, "trim material holder projections"));
        let mut mixed_material = valid.clone();
        mixed_material.armor_trim_material_id = Some(0);
        cases.push((mixed_material, "trim material holder projections"));
        for registry_id in [-1, i32::MAX] {
            let mut invalid = valid.clone();
            invalid.armor_trim_material_direct = None;
            invalid.armor_trim_material_id = Some(registry_id);
            cases.push((invalid, "trim material holder projections"));
        }
        let mut invalid_asset_name = valid.clone();
        invalid_asset_name
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .asset_name = "Ruby".to_string();
        cases.push((invalid_asset_name, "not a valid resource path"));
        let mut invalid_equipment_asset = valid.clone();
        invalid_equipment_asset
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .override_armor_assets =
            BTreeMap::from([("Invalid Asset".to_string(), "ruby".to_string())]);
        cases.push((invalid_equipment_asset, "invalid resource location"));
        let mut noncanonical_equipment_asset = valid.clone();
        noncanonical_equipment_asset
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .override_armor_assets =
            BTreeMap::from([("netherite".to_string(), "ruby".to_string())]);
        cases.push((
            noncanonical_equipment_asset,
            "noncanonical resource location",
        ));
        let mut invalid_override_suffix = valid.clone();
        invalid_override_suffix
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .override_armor_assets =
            BTreeMap::from([("minecraft:netherite".to_string(), "Ruby".to_string())]);
        cases.push((invalid_override_suffix, "not a valid resource path"));
        let mut missing_material_nbt = valid.clone();
        missing_material_nbt
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .description_nbt = None;
        cases.push((missing_material_nbt, "missing component nbt"));
        let mut missing_material_styled = valid.clone();
        missing_material_styled
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .description_styled = None;
        cases.push((missing_material_styled, "missing styled projection"));
        let mut stale_material_text = valid.clone();
        stale_material_text
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .description = "Stale".to_string();
        cases.push((stale_material_text, "noncanonical component projections"));
        let mut malformed_material_nbt = valid.clone();
        malformed_material_nbt
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .description_nbt = Some(Box::new(NbtSummaryValue::Int(1)));
        cases.push((malformed_material_nbt, "is not a component"));
        let mut invalid_pattern_asset = valid.clone();
        invalid_pattern_asset
            .armor_trim_pattern_direct
            .as_mut()
            .unwrap()
            .asset_id = "Invalid Pattern".to_string();
        cases.push((invalid_pattern_asset, "invalid resource location"));
        let mut noncanonical_pattern_asset = valid.clone();
        noncanonical_pattern_asset
            .armor_trim_pattern_direct
            .as_mut()
            .unwrap()
            .asset_id = "spiral".to_string();
        cases.push((noncanonical_pattern_asset, "noncanonical resource location"));
        let mut mixed_pattern = valid.clone();
        mixed_pattern.armor_trim_pattern_id = Some(0);
        cases.push((mixed_pattern, "trim pattern holder projections"));
        for registry_id in [-1, i32::MAX] {
            let mut invalid = valid.clone();
            invalid.armor_trim_pattern_direct = None;
            invalid.armor_trim_pattern_id = Some(registry_id);
            cases.push((invalid, "trim pattern holder projections"));
        }
        let mut stale_pattern_styled = valid.clone();
        stale_pattern_styled
            .armor_trim_pattern_direct
            .as_mut()
            .unwrap()
            .description_styled
            .as_mut()
            .unwrap()[0]
            .text = "Stale".to_string();
        cases.push((stale_pattern_styled, "noncanonical component projections"));

        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(
                error.to_string().contains(expected_error),
                "{error} did not contain {expected_error:?}"
            );
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let mut oversized = valid;
        oversized
            .armor_trim_material_direct
            .as_mut()
            .unwrap()
            .override_armor_assets = (0..=MAX_DATA_COMPONENT_LIST_ITEMS)
            .map(|index| (format!("example:asset_{index}"), "ruby".to_string()))
            .collect();
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(matches!(
            descriptor.encode(&oversized, &mut encoded),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn trim_descriptor_rejects_invalid_material_resource_paths_on_decode() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TRIM_TYPE_ID).unwrap();
        for (asset_name, suffix, expected_error) in [
            ("Ruby", "ruby", "asset_name"),
            ("ruby", "Ruby", "override_armor_assets suffix"),
        ] {
            let mut wire = Encoder::new();
            wire.write_var_i32(0);
            wire.write_string(asset_name);
            wire.write_var_i32(1);
            wire.write_string("minecraft:netherite");
            wire.write_string(suffix);
            wire.write_bytes(
                &encode_nbt_summary_bounded(
                    Some(&NbtSummaryValue::String("Ruby".to_string())),
                    i32::MAX as usize,
                    MAX_NBT_DEPTH,
                )
                .unwrap(),
            );
            wire.write_var_i32(1);
            let error = descriptor
                .decode(
                    &mut Decoder::new(&wire.into_inner()),
                    &mut DataComponentPatchSummary::default(),
                )
                .unwrap_err();
            assert!(
                error.to_string().contains(expected_error),
                "{error} did not contain {expected_error:?}"
            );
        }
    }

    #[test]
    fn written_book_descriptor_round_trips_filterable_components_in_wire_order() {
        let raw_page = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("Raw page".to_string()),
            },
            NbtSummaryEntry {
                name: "italic".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
        ]);
        let filtered_page = NbtSummaryValue::String("Filtered page".to_string());
        let summary = DataComponentPatchSummary {
            written_book: Some(WrittenBookContentSummary {
                title: "Guide".to_string(),
                title_filter: Some("Safe Guide".to_string()),
                author: "Alex".to_string(),
                generation: 2,
                pages: vec!["Raw page".to_string(), "Second page".to_string()],
                page_filters: vec![Some("Filtered page".to_string()), None],
                resolved: true,
            }),
            written_book_pages_nbt: vec![
                Some(raw_page.clone()),
                Some(NbtSummaryValue::String("Second page".to_string())),
            ],
            written_book_page_filters_nbt: vec![Some(filtered_page.clone()), None],
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)
                .unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();

        let mut expected = Encoder::new();
        expected.write_string("Guide");
        expected.write_bool(true);
        expected.write_string("Safe Guide");
        expected.write_string("Alex");
        expected.write_var_i32(2);
        expected.write_var_i32(2);
        expected.write_bytes(
            &encode_nbt_summary_bounded(Some(&raw_page), i32::MAX as usize, MAX_NBT_DEPTH).unwrap(),
        );
        expected.write_bool(true);
        expected.write_bytes(
            &encode_nbt_summary_bounded(Some(&filtered_page), i32::MAX as usize, MAX_NBT_DEPTH)
                .unwrap(),
        );
        expected.write_bytes(
            &encode_nbt_summary_bounded(
                Some(&NbtSummaryValue::String("Second page".to_string())),
                i32::MAX as usize,
                MAX_NBT_DEPTH,
            )
            .unwrap(),
        );
        expected.write_bool(false);
        expected.write_bool(true);
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.written_book, summary.written_book);
        assert_eq!(
            decoded.written_book_pages_nbt,
            summary.written_book_pages_nbt
        );
        assert_eq!(
            decoded.written_book_page_filters_nbt,
            summary.written_book_page_filters_nbt
        );
    }

    #[test]
    fn written_book_descriptor_enforces_java_bounds_and_canonical_alignment_atomically() {
        let valid = DataComponentPatchSummary {
            written_book: Some(WrittenBookContentSummary {
                title: "😀".repeat(16),
                title_filter: None,
                author: "Author".to_string(),
                generation: 0,
                pages: vec!["Page".to_string()],
                page_filters: vec![None],
                resolved: false,
            }),
            written_book_pages_nbt: vec![Some(NbtSummaryValue::String("Page".to_string()))],
            written_book_page_filters_nbt: vec![None],
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)
                .unwrap();
        let mut boundary = Encoder::new();
        descriptor.encode(&valid, &mut boundary).unwrap();
        let mut decoded = DataComponentPatchSummary::default();
        descriptor
            .decode(&mut Decoder::new(&boundary.into_inner()), &mut decoded)
            .unwrap();
        assert_eq!(decoded.written_book, valid.written_book);
        let mut author_boundary = valid.clone();
        author_boundary.written_book.as_mut().unwrap().author = format!("{}a", "😀".repeat(16_383));
        descriptor
            .encode(&author_boundary, &mut Encoder::new())
            .unwrap();

        let mut cases = Vec::new();
        cases.push((
            DataComponentPatchSummary::default(),
            "missing written_book_content",
        ));
        let mut invalid_generation = valid.clone();
        invalid_generation.written_book.as_mut().unwrap().generation = 4;
        cases.push((invalid_generation, "outside 0..=3"));
        let mut oversized_title = valid.clone();
        oversized_title.written_book.as_mut().unwrap().title = "😀".repeat(17);
        cases.push((oversized_title, "string has 34 bytes, max is 32"));
        let mut oversized_filtered_title = valid.clone();
        oversized_filtered_title
            .written_book
            .as_mut()
            .unwrap()
            .title_filter = Some("😀".repeat(17));
        cases.push((oversized_filtered_title, "string has 34 bytes, max is 32"));
        let mut oversized_author = valid.clone();
        oversized_author.written_book.as_mut().unwrap().author = "😀".repeat(16_384);
        cases.push((oversized_author, "string has 32768 bytes, max is 32767"));
        let mut misaligned_pages = valid.clone();
        misaligned_pages.written_book_pages_nbt.clear();
        cases.push((misaligned_pages, "not aligned"));
        let mut missing_raw_page = valid.clone();
        missing_raw_page.written_book_pages_nbt[0] = None;
        cases.push((missing_raw_page, "missing raw component nbt"));
        let mut stale_page = valid.clone();
        stale_page.written_book.as_mut().unwrap().pages[0] = "Stale".to_string();
        cases.push((stale_page, "noncanonical component projections"));
        let mut malformed_page = valid.clone();
        malformed_page.written_book_pages_nbt[0] = Some(NbtSummaryValue::Int(1));
        cases.push((malformed_page, "is not a component"));
        let mut mismatched_filter = valid.clone();
        mismatched_filter
            .written_book
            .as_mut()
            .unwrap()
            .page_filters[0] = Some("Filtered".to_string());
        cases.push((mismatched_filter, "filtered projections are not aligned"));

        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let oversized_page_count = MAX_DATA_COMPONENT_LIST_ITEMS + 1;
        let oversized = DataComponentPatchSummary {
            written_book: Some(WrittenBookContentSummary {
                title: "Title".to_string(),
                title_filter: None,
                author: "Author".to_string(),
                generation: 0,
                pages: vec!["Page".to_string(); oversized_page_count],
                page_filters: vec![None; oversized_page_count],
                resolved: false,
            }),
            written_book_pages_nbt: vec![
                Some(NbtSummaryValue::String("Page".to_string()));
                oversized_page_count
            ],
            written_book_page_filters_nbt: vec![None; oversized_page_count],
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = descriptor.encode(&oversized, &mut encoded).unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(actual, max)
                if actual == oversized_page_count && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
        assert_eq!(encoded.into_inner(), [0xaa]);

        for (wire, expected_error) in [
            (
                {
                    let mut wire = Encoder::new();
                    wire.write_string(&"😀".repeat(17));
                    wire.write_bool(false);
                    wire.write_string("Author");
                    wire.write_var_i32(0);
                    wire.write_var_i32(0);
                    wire.write_bool(false);
                    wire.into_inner()
                },
                "string has 34 bytes, max is 32",
            ),
            (
                {
                    let mut wire = Encoder::new();
                    wire.write_string("Title");
                    wire.write_bool(false);
                    wire.write_string("Author");
                    wire.write_var_i32(-1);
                    wire.write_var_i32(0);
                    wire.write_bool(false);
                    wire.into_inner()
                },
                "outside 0..=3",
            ),
        ] {
            let error = descriptor
                .decode(
                    &mut Decoder::new(&wire),
                    &mut DataComponentPatchSummary::default(),
                )
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
        }
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
    fn foundational_descriptors_encode_exact_network_values() {
        let summary = DataComponentPatchSummary {
            max_stack_size: Some(i32::MIN),
            max_damage: Some(0),
            damage: Some(-1),
            unbreakable: true,
            ..DataComponentPatchSummary::default()
        };
        for (type_id, expected_value) in [
            (
                VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                Some(i32::MIN),
            ),
            (VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, Some(0)),
            (VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, Some(-1)),
            (VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, None),
        ] {
            let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            let encoded = encoded.into_inner();
            if let Some(expected_value) = expected_value {
                let mut decoder = Decoder::new(&encoded);
                assert_eq!(decoder.read_var_i32().unwrap(), expected_value);
                assert!(decoder.is_empty());
            } else {
                assert!(encoded.is_empty());
            }
        }
    }

    #[test]
    fn custom_data_descriptor_round_trips_exact_compound_nbt() {
        let custom_data = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "answer".to_string(),
                value: NbtSummaryValue::Int(42),
            },
            NbtSummaryEntry {
                name: "owner".to_string(),
                value: NbtSummaryValue::String("Alex".to_string()),
            },
        ]);
        let summary = DataComponentPatchSummary {
            custom_data: Some(custom_data.clone()),
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        assert_eq!(
            encoded,
            [
                10, // unnamed compound root
                3, 0, 6, b'a', b'n', b's', b'w', b'e', b'r', 0, 0, 0, 42, 8, 0, 5, b'o', b'w',
                b'n', b'e', b'r', 0, 4, b'A', b'l', b'e', b'x', 0, // compound end
            ]
        );

        let mut decoder = Decoder::new(&encoded);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.custom_data, Some(custom_data));
    }

    #[test]
    fn custom_data_descriptor_rejects_missing_null_and_scalar_roots() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID).unwrap();
        for custom_data in [None, Some(NbtSummaryValue::Int(42))] {
            let summary = DataComponentPatchSummary {
                custom_data,
                ..DataComponentPatchSummary::default()
            };
            let error = descriptor
                .encode(&summary, &mut Encoder::new())
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains(if summary.custom_data.is_some() {
                        "must be a non-null compound tag"
                    } else {
                        "missing custom_data"
                    }),
                "{error}"
            );
        }

        for wire in [&[0][..], &[3, 0, 0, 0, 42][..]] {
            let mut decoder = Decoder::new(wire);
            let error = descriptor
                .decode(&mut decoder, &mut DataComponentPatchSummary::default())
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("must be a non-null compound tag"),
                "{error}"
            );
        }
    }

    #[test]
    fn entity_data_descriptors_encode_exact_holder_and_compound_values() {
        let entity_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "Health".to_string(),
            value: NbtSummaryValue::Short(5),
        }]);
        let bucket_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "Variant".to_string(),
            value: NbtSummaryValue::Int(2),
        }]);
        let block_tag = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "Delay".to_string(),
            value: NbtSummaryValue::Short(20),
        }]);
        let summary = DataComponentPatchSummary {
            entity_data_entity_type_id: Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
            entity_data_tag: Some(entity_tag),
            codec_backed_nbt: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                bucket_tag,
            )]),
            block_entity_data_type_id: Some(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID),
            block_entity_data_tag: Some(block_tag),
            ..DataComponentPatchSummary::default()
        };

        for (type_id, expected) in [
            (
                VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
                vec![
                    0x96, 0x01, // zombie holder 150
                    10, 2, 0, 6, b'H', b'e', b'a', b'l', b't', b'h', 0, 5, 0,
                ],
            ),
            (
                VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                vec![
                    10, 3, 0, 7, b'V', b'a', b'r', b'i', b'a', b'n', b't', 0, 0, 0, 2, 0,
                ],
            ),
            (
                VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
                vec![
                    9, // mob spawner holder
                    10, 2, 0, 5, b'D', b'e', b'l', b'a', b'y', 0, 20, 0,
                ],
            ),
        ] {
            let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
            let mut encoded = Encoder::new();
            descriptor.encode(&summary, &mut encoded).unwrap();
            assert_eq!(encoded.into_inner(), expected);
        }
    }

    #[test]
    fn typed_entity_data_decode_strips_id_and_rejects_invalid_values() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID).unwrap();
        let mut wire = Encoder::new();
        wire.write_var_i32(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        wire.write_bytes(&[
            10, // compound
            8, 0, 2, b'i', b'd', 0, 16, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':',
            b'z', b'o', b'm', b'b', b'i', b'e', 2, 0, 6, b'H', b'e', b'a', b'l', b't', b'h', 0, 5,
            0,
        ]);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut summary = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut summary).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            summary.entity_data_tag,
            Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "Health".to_string(),
                value: NbtSummaryValue::Short(5),
            }]))
        );

        for (type_id, wire, expected_error) in [
            (
                VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
                {
                    let mut wire = Encoder::new();
                    wire.write_var_i32(VANILLA_ENTITY_TYPES.len() as i32);
                    wire.write_bytes(&[10, 0]);
                    wire.into_inner()
                },
                "invalid vanilla entity type holder",
            ),
            (
                VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
                {
                    let mut wire = Encoder::new();
                    wire.write_var_i32(VANILLA_BLOCK_ENTITY_TYPES.len() as i32);
                    wire.write_bytes(&[10, 0]);
                    wire.into_inner()
                },
                "invalid vanilla block entity type holder",
            ),
            (
                VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                vec![3, 0, 0, 0, 42],
                "must be a non-null compound tag",
            ),
            (
                VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
                {
                    let mut wire = Encoder::new();
                    wire.write_var_i32(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
                    wire.write_u8(0);
                    wire.into_inner()
                },
                "must be a non-null compound tag",
            ),
            (
                VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
                {
                    let mut wire = Encoder::new();
                    wire.write_var_i32(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID);
                    wire.write_bytes(&[3, 0, 0, 0, 42]);
                    wire.into_inner()
                },
                "must be a non-null compound tag",
            ),
        ] {
            let error = vanilla_data_component_descriptor(type_id)
                .unwrap()
                .decode(
                    &mut Decoder::new(&wire),
                    &mut DataComponentPatchSummary::default(),
                )
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
        }
    }

    #[test]
    fn entity_data_descriptors_reject_missing_scalar_and_embedded_id_summaries() {
        let compound = NbtSummaryValue::Compound(Vec::new());
        let cases = [
            (
                VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
                DataComponentPatchSummary {
                    entity_data_entity_type_id: Some(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
                    ..DataComponentPatchSummary::default()
                },
                "missing entity_data component value",
            ),
            (
                VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                DataComponentPatchSummary {
                    codec_backed_nbt: BTreeMap::from([(
                        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                        NbtSummaryValue::Int(42),
                    )]),
                    ..DataComponentPatchSummary::default()
                },
                "must be a non-null compound tag",
            ),
            (
                VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
                DataComponentPatchSummary {
                    block_entity_data_type_id: Some(VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID),
                    block_entity_data_tag: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "id".to_string(),
                        value: NbtSummaryValue::String("minecraft:mob_spawner".to_string()),
                    }])),
                    ..DataComponentPatchSummary::default()
                },
                "must not contain top-level id",
            ),
        ];
        for (type_id, summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = vanilla_data_component_descriptor(type_id)
                .unwrap()
                .encode(&summary, &mut encoded)
                .unwrap_err();
            assert!(error.to_string().contains(expected_error), "{error}");
            assert_eq!(encoded.into_inner(), [0xaa]);
        }

        let valid_entity = DataComponentPatchSummary {
            entity_data_entity_type_id: Some(VANILLA_ENTITY_TYPES.len() as i32),
            entity_data_tag: Some(compound),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        let error = vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID)
            .unwrap()
            .encode(&valid_entity, &mut encoded)
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("invalid vanilla entity type holder"),
            "{error}"
        );
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    #[test]
    fn profile_descriptor_encodes_exact_game_and_partial_values() {
        let uuid = Uuid::from_u128(1);
        let game_profile = DataComponentPatchSummary {
            profile: Some(ResolvableProfileSummary {
                kind: ResolvableProfileKindSummary::GameProfile,
                uuid: Some(uuid),
                name: Some("Alex".to_string()),
                properties: vec![GameProfilePropertySummary {
                    name: "p".to_string(),
                    value: "v".to_string(),
                    signature: None,
                }],
                profile_textures: None,
                skin_patch: PlayerSkinPatchSummary {
                    model: Some(PlayerModelTypeSummary::Wide),
                    ..PlayerSkinPatchSummary::default()
                },
            }),
            ..DataComponentPatchSummary::default()
        };
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&game_profile, &mut encoded).unwrap();
        assert_eq!(
            encoded.into_inner(),
            [
                1, // Either.left(GameProfile)
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, // UUID
                4, b'A', b'l', b'e', b'x', // name
                1,    // property count
                1, b'p', 1, b'v', 0, // property and absent signature
                0, 0, 0, // absent body, cape, and elytra
                1, 0, // present wide model
            ]
        );

        let partial_profile = DataComponentPatchSummary {
            profile: Some(ResolvableProfileSummary {
                kind: ResolvableProfileKindSummary::Partial,
                uuid: Some(uuid),
                name: None,
                properties: Vec::new(),
                profile_textures: None,
                skin_patch: PlayerSkinPatchSummary {
                    body: Some(ResourceTextureSummary {
                        asset_id: "minecraft:entity/player/wide/steve".to_string(),
                        texture_path: "minecraft:textures/entity/player/wide/steve.png".to_string(),
                    }),
                    cape: None,
                    elytra: Some(ResourceTextureSummary {
                        asset_id: "minecraft:entity/player/elytra".to_string(),
                        texture_path: "minecraft:textures/entity/player/elytra.png".to_string(),
                    }),
                    model: Some(PlayerModelTypeSummary::Slim),
                },
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&partial_profile, &mut encoded).unwrap();
        assert_eq!(
            encoded.into_inner(),
            [
                0, // Either.right(Partial)
                0, // absent name
                1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, // UUID
                0, // property count
                1, 34, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':', b'e', b'n',
                b't', b'i', b't', b'y', b'/', b'p', b'l', b'a', b'y', b'e', b'r', b'/', b'w', b'i',
                b'd', b'e', b'/', b's', b't', b'e', b'v', b'e', // body
                0,    // absent cape
                1, 30, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':', b'e', b'n',
                b't', b'i', b't', b'y', b'/', b'p', b'l', b'a', b'y', b'e', b'r', b'/', b'e', b'l',
                b'y', b't', b'r', b'a', // elytra
                1, 1, // present slim model
            ]
        );
    }

    #[test]
    fn profile_descriptor_round_trips_java_utf16_name_boundary() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID).unwrap();
        let name = "😀".repeat(8);
        let summary = DataComponentPatchSummary {
            profile: Some(ResolvableProfileSummary {
                kind: ResolvableProfileKindSummary::Partial,
                uuid: None,
                name: Some(name.clone()),
                properties: Vec::new(),
                profile_textures: None,
                skin_patch: PlayerSkinPatchSummary::default(),
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.profile, summary.profile);

        let mut oversized = encoded;
        oversized[1] = 1;
        oversized[2] = 36;
        oversized.splice(3..35, "😀".repeat(9).into_bytes());
        let error = descriptor
            .decode(
                &mut Decoder::new(&oversized),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::StringTooLong {
                actual: 18,
                max: MAX_PLAYER_NAME_CHARS
            }
        ));

        let mut too_many_properties = Encoder::new();
        too_many_properties.write_bool(false);
        too_many_properties.write_bool(false);
        too_many_properties.write_bool(false);
        too_many_properties.write_var_i32((MAX_PROFILE_PROPERTIES + 1) as i32);
        let too_many_properties = too_many_properties.into_inner();
        let error = descriptor
            .decode(
                &mut Decoder::new(&too_many_properties),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::PacketTooLarge(
                actual,
                MAX_PROFILE_PROPERTIES
            ) if actual == MAX_PROFILE_PROPERTIES + 1
        ));

        let mut oversized_property_name = Encoder::new();
        oversized_property_name.write_bool(false);
        oversized_property_name.write_bool(false);
        oversized_property_name.write_bool(false);
        oversized_property_name.write_var_i32(1);
        oversized_property_name.write_string(&"😀".repeat(33));
        let oversized_property_name = oversized_property_name.into_inner();
        let error = descriptor
            .decode(
                &mut Decoder::new(&oversized_property_name),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            ProtocolError::StringTooLong {
                actual: 66,
                max: MAX_PROFILE_PROPERTY_NAME_CHARS
            }
        ));
    }

    #[test]
    fn profile_descriptor_rejects_noncanonical_and_oversized_summaries_atomically() {
        let descriptor =
            vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID).unwrap();
        let valid = || ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: None,
            name: Some("Alex".to_string()),
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary::default(),
        };
        let mut cases = Vec::new();

        cases.push((
            DataComponentPatchSummary::default(),
            "missing profile component value",
        ));

        let mut missing_game_uuid = valid();
        missing_game_uuid.kind = ResolvableProfileKindSummary::GameProfile;
        cases.push((
            DataComponentPatchSummary {
                profile: Some(missing_game_uuid),
                ..DataComponentPatchSummary::default()
            },
            "missing its UUID",
        ));

        let mut missing_game_name = valid();
        missing_game_name.kind = ResolvableProfileKindSummary::GameProfile;
        missing_game_name.uuid = Some(Uuid::from_u128(1));
        missing_game_name.name = None;
        cases.push((
            DataComponentPatchSummary {
                profile: Some(missing_game_name),
                ..DataComponentPatchSummary::default()
            },
            "missing its name",
        ));

        let mut oversized_name = valid();
        oversized_name.name = Some("😀".repeat(9));
        cases.push((
            DataComponentPatchSummary {
                profile: Some(oversized_name),
                ..DataComponentPatchSummary::default()
            },
            "string has 18 bytes",
        ));

        let mut too_many_properties = valid();
        too_many_properties.properties = (0..=MAX_PROFILE_PROPERTIES)
            .map(|index| GameProfilePropertySummary {
                name: format!("p{index}"),
                value: "value".to_string(),
                signature: None,
            })
            .collect();
        cases.push((
            DataComponentPatchSummary {
                profile: Some(too_many_properties),
                ..DataComponentPatchSummary::default()
            },
            "exceeds configured maximum 16",
        ));

        for (property, expected_error) in [
            (
                GameProfilePropertySummary {
                    name: "p".repeat(MAX_PROFILE_PROPERTY_NAME_CHARS + 1),
                    value: "value".to_string(),
                    signature: None,
                },
                "max is 64",
            ),
            (
                GameProfilePropertySummary {
                    name: "p".to_string(),
                    value: "v".repeat(MAX_STRING_CHARS + 1),
                    signature: None,
                },
                "max is 32767",
            ),
            (
                GameProfilePropertySummary {
                    name: "p".to_string(),
                    value: "value".to_string(),
                    signature: Some("s".repeat(MAX_PROFILE_SIGNATURE_CHARS + 1)),
                },
                "max is 1024",
            ),
        ] {
            let mut oversized_property = valid();
            oversized_property.properties.push(property);
            cases.push((
                DataComponentPatchSummary {
                    profile: Some(oversized_property),
                    ..DataComponentPatchSummary::default()
                },
                expected_error,
            ));
        }

        let mut stale_profile_textures = valid();
        stale_profile_textures.profile_textures = Some(ProfileTexturesSummary {
            skin: None,
            cape: None,
            elytra: None,
        });
        cases.push((
            DataComponentPatchSummary {
                profile: Some(stale_profile_textures),
                ..DataComponentPatchSummary::default()
            },
            "noncanonical derived texture values",
        ));

        let mut stale_path = valid();
        stale_path.skin_patch.body = Some(ResourceTextureSummary {
            asset_id: "minecraft:entity/player/wide/steve".to_string(),
            texture_path: "minecraft:wrong.png".to_string(),
        });
        cases.push((
            DataComponentPatchSummary {
                profile: Some(stale_path),
                ..DataComponentPatchSummary::default()
            },
            "noncanonical derived texture values",
        ));

        let mut invalid_asset = valid();
        invalid_asset.skin_patch.body = Some(ResourceTextureSummary {
            asset_id: "minecraft:Bad Asset".to_string(),
            texture_path: "minecraft:textures/Bad Asset.png".to_string(),
        });
        cases.push((
            DataComponentPatchSummary {
                profile: Some(invalid_asset),
                ..DataComponentPatchSummary::default()
            },
            "invalid resource location",
        ));

        for (summary, expected_error) in cases {
            let mut encoded = Encoder::new();
            encoded.write_u8(0xaa);
            let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
            assert!(
                error
                    .to_string()
                    .to_ascii_lowercase()
                    .contains(&expected_error.to_ascii_lowercase()),
                "{error}"
            );
            assert_eq!(encoded.into_inner(), [0xaa]);
        }
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
