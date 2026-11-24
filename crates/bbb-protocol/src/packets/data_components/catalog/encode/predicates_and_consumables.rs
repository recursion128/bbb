use super::*;

pub(in super::super) fn encode_adventure_mode_predicate(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    let predicate = validate_adventure_mode_predicate_component(type_id, summary)?;
    encode_adventure_mode_predicate_value_with(
        out,
        predicate,
        registry,
        encode_data_component_matchers_with_registry,
    )?;
    Ok(())
}

pub(super) fn validate_adventure_mode_predicate_component(
    type_id: i32,
    summary: &DataComponentPatchSummary,
) -> Result<&AdventureModePredicateDataSummary> {
    let (predicate, what) = match type_id {
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID => {
            (summary.can_place_on_persistent.as_ref(), "can_place_on")
        }
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID => {
            (summary.can_break_persistent.as_ref(), "can_break")
        }
        _ => unreachable!("adventure mode predicate encoder registered for type id {type_id}"),
    };
    let predicate = predicate
        .ok_or_else(|| ProtocolError::InvalidData(format!("missing {what} component value")))?;
    validate_adventure_mode_predicate_nesting(predicate, 0)?;
    Ok(predicate)
}

/// Derives adventure mode predicate tooltip content.
pub fn adventure_mode_predicate_tooltip_summary(
    predicate: &AdventureModePredicateDataSummary,
) -> AdventureModePredicateSummary {
    let mut summary = AdventureModePredicateSummary::default();
    for predicate in &predicate.predicates {
        let Some(blocks) = &predicate.blocks else {
            summary.unknown = true;
            continue;
        };
        summary.unresolved_block_tag_count += usize::from(blocks.tag.is_some());
        for registry_id in &blocks.registry_ids {
            if !summary.block_registry_ids.contains(registry_id) {
                summary.block_registry_ids.push(*registry_id);
            }
        }
    }
    summary
}

pub(in super::super) fn validate_adventure_mode_predicate_nesting(
    predicate: &AdventureModePredicateDataSummary,
    depth: usize,
) -> Result<()> {
    if depth >= MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH {
        return Err(ProtocolError::InvalidData(format!(
            "adventure mode predicate nesting exceeds \
             {MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH}"
        )));
    }
    for block in &predicate.predicates {
        let Some(exact) = block.exact_components.as_deref() else {
            continue;
        };
        if let Some(nested) = exact.can_place_on_persistent.as_ref() {
            validate_adventure_mode_predicate_nesting(nested, depth + 1)?;
        }
        if let Some(nested) = exact.can_break_persistent.as_ref() {
            validate_adventure_mode_predicate_nesting(nested, depth + 1)?;
        }
    }
    Ok(())
}

pub(super) fn encode_adventure_mode_predicate_value_with<F>(
    out: &mut Encoder,
    predicate: &AdventureModePredicateDataSummary,
    registry: &ContentRegistry,
    encode_matchers: F,
) -> Result<()>
where
    F: Fn(
            &mut Encoder,
            Option<&DataComponentPatchSummary>,
            &[PartialDataComponentPredicateSummary],
            &ContentRegistry,
        ) -> Result<()>
        + Copy,
{
    if predicate.predicates.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            predicate.predicates.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    out.write_var_i32(predicate.predicates.len() as i32);
    for block in &predicate.predicates {
        encode_block_predicate_with(out, block, registry, encode_matchers)?;
    }
    Ok(())
}

fn encode_block_predicate_with<F>(
    out: &mut Encoder,
    predicate: &BlockPredicateSummary,
    registry: &ContentRegistry,
    encode_matchers: F,
) -> Result<()>
where
    F: Fn(
            &mut Encoder,
            Option<&DataComponentPatchSummary>,
            &[PartialDataComponentPredicateSummary],
            &ContentRegistry,
        ) -> Result<()>
        + Copy,
{
    match &predicate.blocks {
        Some(blocks) => {
            out.write_bool(true);
            encode_holder_set_summary_with_registry(
                out,
                &blocks.registry_ids,
                blocks.tag.as_deref(),
                NetworkHolderSetRegistry::Block,
                "block predicate holder set",
                registry,
            )?;
        }
        None => out.write_bool(false),
    }

    match &predicate.state {
        Some(properties) => {
            if properties.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(
                    properties.len(),
                    MAX_DATA_COMPONENT_LIST_ITEMS,
                ));
            }
            out.write_bool(true);
            out.write_var_i32(properties.len() as i32);
            for property in properties {
                encode_java_string(out, &property.name.to_jdk_string(), MAX_STRING_CHARS)?;
                match &property.value {
                    StatePropertyValueMatcherSummary::Exact(value) => {
                        out.write_bool(true);
                        encode_java_string(out, &value.to_jdk_string(), MAX_STRING_CHARS)?;
                    }
                    StatePropertyValueMatcherSummary::Range { min, max } => {
                        out.write_bool(false);
                        encode_optional_java_string(
                            out,
                            min.as_ref().map(|value| value.to_jdk_string()).as_deref(),
                            MAX_STRING_CHARS,
                        )?;
                        encode_optional_java_string(
                            out,
                            max.as_ref().map(|value| value.to_jdk_string()).as_deref(),
                            MAX_STRING_CHARS,
                        )?;
                    }
                }
            }
        }
        None => out.write_bool(false),
    }

    match predicate.nbt.as_ref() {
        Some(nbt @ NbtSummaryValue::Compound(_)) => {
            out.write_bool(true);
            out.write_bytes(&encode_nbt_summary_bounded(
                Some(nbt),
                i32::MAX as usize,
                MAX_NBT_DEPTH,
            )?);
        }
        Some(_) => {
            return Err(ProtocolError::InvalidData(
                "block predicate nbt must be a compound tag".to_string(),
            ));
        }
        None => out.write_bool(false),
    }

    encode_matchers(
        out,
        predicate.exact_components.as_deref(),
        &predicate.partial_components,
        registry,
    )
}

pub(super) fn encode_persistent_matchers(
    out: &mut Encoder,
    _: Option<&DataComponentPatchSummary>,
    partial: &[PartialDataComponentPredicateSummary],
    registry: &ContentRegistry,
) -> Result<()> {
    encode_data_component_matchers_with_registry(out, None, partial, registry)
}

pub(in super::super) fn encode_data_component_matchers_with_registry(
    out: &mut Encoder,
    exact: Option<&DataComponentPatchSummary>,
    partial: &[PartialDataComponentPredicateSummary],
    registry: &ContentRegistry,
) -> Result<()> {
    if let Some(exact) = exact {
        if !exact.removed_type_ids.is_empty() {
            return Err(ProtocolError::InvalidData(
                "exact data component predicate cannot remove components".to_string(),
            ));
        }
        if exact.added_type_ids.len() > MAX_DATA_COMPONENT_PREDICATE_ENTRIES {
            return Err(ProtocolError::PacketTooLarge(
                exact.added_type_ids.len(),
                MAX_DATA_COMPONENT_PREDICATE_ENTRIES,
            ));
        }
        // `DataComponentExactPredicate.EMPTY` encodes as length 0 on the wire
        // (`DataComponentMatchers.java:18-31`).
        let descriptors = validate_encodable_data_component_patch_summary(exact)?;
        out.write_var_i32(exact.added_type_ids.len() as i32);
        for (type_id, descriptor) in descriptors {
            out.write_var_i32(type_id);
            descriptor.encode_with_registry(exact, out, registry)?;
        }
    } else {
        out.write_var_i32(0);
    }

    if partial.len() > MAX_PARTIAL_DATA_COMPONENT_PREDICATES {
        return Err(ProtocolError::PacketTooLarge(
            partial.len(),
            MAX_PARTIAL_DATA_COMPONENT_PREDICATES,
        ));
    }
    out.write_var_i32(partial.len() as i32);
    let mut seen = BTreeSet::new();
    for predicate in partial {
        let (concrete, type_id) = match predicate.predicate_type {
            DataComponentPredicateTypeSummary::Concrete(type_id) => {
                if crate::data_component_types::vanilla_data_component_predicate_type(type_id)
                    .is_none()
                {
                    return Err(ProtocolError::InvalidData(format!(
                        "invalid data component predicate type id {type_id}"
                    )));
                }
                (true, type_id)
            }
            DataComponentPredicateTypeSummary::AnyComponent(type_id) => {
                data_component_descriptor_or_error(type_id)?;
                (false, type_id)
            }
        };
        if !seen.insert((concrete, type_id)) {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate partial data component predicate type id {type_id}"
            )));
        }
        let value = predicate.wire_value.as_ref().ok_or_else(|| {
            ProtocolError::InvalidData(
                "partial data component predicate is missing non-null nbt".to_string(),
            )
        })?;
        if !concrete && !matches!(value, NbtSummaryValue::Compound(entries) if entries.is_empty()) {
            return Err(ProtocolError::InvalidData(format!(
                "any-component predicate type id {type_id} must use an empty compound value"
            )));
        }
        out.write_bool(concrete);
        out.write_var_i32(type_id);
        out.write_bytes(&encode_nbt_summary_bounded(
            Some(value),
            i32::MAX as usize,
            MAX_NBT_DEPTH,
        )?);
    }
    Ok(())
}

pub(in super::super) fn encode_attribute_modifiers(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    if summary.attribute_modifiers.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            summary.attribute_modifiers.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    out.write_var_i32(summary.attribute_modifiers.len() as i32);
    for modifier in &summary.attribute_modifiers {
        if crate::attribute_types::vanilla_attribute_type(modifier.attribute_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid vanilla attribute holder id {}",
                modifier.attribute_id
            )));
        }
        if !crate::packets::is_canonical_resource_location(&modifier.modifier_id)? {
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

        out.write_var_i32(modifier.attribute_id);
        out.write_string(&modifier.modifier_id);
        out.write_f64(f64::from_bits(modifier.amount_bits));
        out.write_var_i32(modifier.operation_id);
        out.write_var_i32(modifier.slot_id);
        encode_attribute_modifier_display(modifier, out, registry)?;
    }
    Ok(())
}

pub(in super::super) fn encode_attribute_modifier_display(
    modifier: &AttributeModifierSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
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
                decode_styled_component_summary_with_nbt_from_decoder_with_registry(
                    &mut Decoder::new(&component),
                    registry,
                )?;
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

pub(in super::super) fn encode_consumable(
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
    out.write_f32(f32::from_bits(consumable.consume_seconds.to_bits()));
    out.write_var_i32(consumable.animation.id());
    encode_sound_event_holder_summary(out, sound)?;
    out.write_bool(has_consume_particles);
    encode_consume_effect_list(out, &summary.consumable_effects)?;
    Ok(())
}

pub(in super::super) fn encode_use_remainder(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    let template = require_use_remainder(summary)?;
    validate_item_stack_template_forest_with_registry(std::iter::once(template), registry)?;
    encode_item_stack_template_with_registry(out, template, registry)
}

pub(super) fn require_use_remainder(
    summary: &DataComponentPatchSummary,
) -> Result<&ItemStackTemplateSummary> {
    summary.use_remainder.as_deref().ok_or_else(|| {
        ProtocolError::InvalidData("use_remainder is missing item stack template".to_string())
    })
}

pub(in super::super) fn encode_use_cooldown(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let seconds_bits = summary.use_cooldown_seconds_bits.ok_or_else(|| {
        ProtocolError::InvalidData("use_cooldown is missing seconds projection".to_string())
    })?;
    let seconds = decode_persistent_positive_float(seconds_bits, "use_cooldown seconds")?;
    out.write_f32(seconds);
    out.write_bool(summary.use_cooldown_group.is_some());
    if let Some(group) = &summary.use_cooldown_group {
        validate_canonical_resource_location(group, "use_cooldown group")?;
        encode_java_string(out, group, MAX_STRING_CHARS)?;
    }
    Ok(())
}

pub(in super::super) fn encode_death_protection(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_consume_effect_list(out, &summary.death_protection_effects)?;
    Ok(())
}

pub(in super::super) fn encode_consume_effect_list(
    out: &mut Encoder,
    effects: &[ConsumeEffectSummary],
) -> Result<()> {
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

pub(in super::super) fn encode_consume_effect(
    out: &mut Encoder,
    effect: &ConsumeEffectSummary,
) -> Result<()> {
    use bbb_facts::consume_effect_types::{
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

pub(in super::super) fn encode_mob_effect_instance(
    out: &mut Encoder,
    effect: &MobEffectInstanceSummary,
) -> Result<()> {
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
pub(in super::super) fn encode_mob_effect_details(
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

pub(in super::super) fn encode_potion_contents(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let custom_effect_count = summary.potion_contents.custom_effects.len();
    if custom_effect_count > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            custom_effect_count,
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    out.write_bool(summary.potion_contents.potion_id.is_some());
    if let Some(potion_id) = summary.potion_contents.potion_id {
        if vanilla_potion(potion_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid vanilla potion holder id {potion_id}"
            )));
        }
        out.write_var_i32(potion_id);
    }
    out.write_bool(summary.potion_contents.custom_color.is_some());
    if let Some(custom_color) = summary.potion_contents.custom_color {
        out.write_i32(custom_color);
    }
    out.write_var_i32(custom_effect_count as i32);
    for effect in &summary.potion_contents.custom_effects {
        // PotionContents.java:150-152 copies each instance without its queued hidden effect.
        encode_mob_effect_instance(
            out,
            &MobEffectInstanceSummary {
                effect_id: effect.effect_id,
                amplifier: effect.amplifier,
                duration: effect.duration,
                ambient: effect.ambient,
                show_particles: effect.show_particles,
                show_icon: effect.show_icon,
                hidden_effect: None,
            },
        )?;
    }
    out.write_bool(summary.potion_contents.custom_name.is_some());
    if let Some(name) = &summary.potion_contents.custom_name {
        out.write_java_string(name, MAX_STRING_CHARS)?;
    }
    Ok(())
}

pub(in super::super) fn encode_potion_duration_scale(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let bits = summary.potion_duration_scale_bits.ok_or_else(|| {
        ProtocolError::InvalidData("missing potion_duration_scale component value".to_string())
    })?;
    out.write_f32(f32::from_bits(bits));
    Ok(())
}

pub(in super::super) fn encode_suspicious_stew_effects(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if summary.suspicious_stew_effects.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            summary.suspicious_stew_effects.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    out.write_var_i32(summary.suspicious_stew_effects.len() as i32);
    for effect in &summary.suspicious_stew_effects {
        client_state::validate_mob_effect_holder_id(effect.effect_id)?;
        out.write_var_i32(effect.effect_id);
        out.write_var_i32(effect.duration);
    }
    Ok(())
}
