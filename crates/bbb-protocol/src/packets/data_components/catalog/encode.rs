use super::*;

mod books_combat_and_trims;
mod entities_media_and_profiles;
mod predicates_and_consumables;

pub(super) use books_combat_and_trims::*;
pub(crate) use books_combat_and_trims::{
    decode_required_compound_tag, encode_trim_pattern_holder,
    validate_persistent_recursive_component_with_registry,
};
pub(super) use entities_media_and_profiles::*;
pub use predicates_and_consumables::*;

pub(super) fn decode_custom_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.custom_data = Some(decode_required_compound_tag(decoder, "custom_data")?);
    Ok(())
}

pub(super) fn encode_custom_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_required_compound_tag(summary.custom_data.as_ref(), "custom_data", out)
}

pub(super) fn encode_name_component(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    let (text, styled, nbt, what) = match type_id {
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => (
            summary.custom_name.as_deref(),
            summary.custom_name_styled.as_deref(),
            summary.custom_name_nbt.as_ref(),
            "custom name",
        ),
        VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => (
            summary.item_name.as_deref(),
            summary.item_name_styled.as_deref(),
            summary.item_name_nbt.as_ref(),
            "item name",
        ),
        _ => unreachable!("name component encoder registered for type id {type_id}"),
    };
    let text = text.ok_or_else(|| ProtocolError::InvalidData(format!("{what} text is missing")))?;
    out.write_bytes(&encode_canonical_component_projection(
        nbt, text, styled, what, registry,
    )?);
    Ok(())
}

pub(super) fn encode_use_effects(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let effects = summary.use_effects.ok_or_else(|| {
        ProtocolError::InvalidData("missing use_effects component value".to_string())
    })?;
    out.write_bool(effects.can_sprint);
    out.write_bool(effects.interact_vibrations);
    out.write_f32(effects.speed_multiplier);
    Ok(())
}

pub(super) fn encode_minimum_attack_charge(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let bits = summary.minimum_attack_charge_bits.ok_or_else(|| {
        ProtocolError::InvalidData("missing minimum_attack_charge component value".to_string())
    })?;
    out.write_f32(f32::from_bits(bits));
    Ok(())
}

pub(super) fn encode_registry_holder_component<const TYPE_ID: i32>(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let holder_id = match TYPE_ID {
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => summary.damage_type_id,
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => summary.villager_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => summary.wolf_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => summary.wolf_sound_variant_id,
        VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => summary.frog_variant_id,
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => {
            summary.zombie_nautilus_variant_id
        }
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => summary.pig_variant_id,
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => summary.pig_sound_variant_id,
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => summary.cow_variant_id,
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => summary.cow_sound_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => summary.chicken_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => summary.chicken_sound_variant_id,
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => summary.cat_variant_id,
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => summary.cat_sound_variant_id,
        _ => unreachable!("registry holder component binding"),
    }
    .ok_or_else(|| {
        ProtocolError::InvalidData(format!("missing registry holder component {TYPE_ID}"))
    })?;
    if holder_id < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "component {TYPE_ID} has negative holder id {holder_id}"
        )));
    }
    out.write_var_i32(holder_id);
    Ok(())
}

pub(super) fn encode_item_model(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let item_model = summary.item_model.as_deref().ok_or_else(|| {
        ProtocolError::InvalidData("missing item_model component value".to_string())
    })?;
    validate_canonical_resource_location(item_model, "item_model")?;

    encode_java_string(out, item_model, MAX_STRING_CHARS)?;
    Ok(())
}

pub(super) fn encode_tooltip_style(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let tooltip_style = summary.tooltip_style.as_deref().ok_or_else(|| {
        ProtocolError::InvalidData("missing tooltip_style component value".to_string())
    })?;
    validate_canonical_resource_location(tooltip_style, "tooltip_style")?;

    encode_java_string(out, tooltip_style, MAX_STRING_CHARS)?;
    Ok(())
}

pub(super) fn encode_note_block_sound(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let note_block_sound = summary.note_block_sound.as_deref().ok_or_else(|| {
        ProtocolError::InvalidData("missing note_block_sound component value".to_string())
    })?;
    validate_canonical_resource_location(note_block_sound, "note_block_sound")?;

    encode_java_string(out, note_block_sound, MAX_STRING_CHARS)?;
    Ok(())
}

pub(super) fn encode_rarity(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let rarity = summary
        .rarity
        .ok_or_else(|| ProtocolError::InvalidData("missing rarity component value".to_string()))?;
    out.write_var_i32(rarity.id());
    Ok(())
}

pub(super) fn encode_item_enchantments(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let (entries, what) = match type_id {
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID => {
            (summary.enchantments.as_slice(), "enchantments")
        }
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID => (
            summary.stored_enchantments.as_slice(),
            "stored_enchantments",
        ),
        _ => unreachable!("item enchantments encoder registered for type id {type_id}"),
    };
    if entries.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            entries.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    out.write_var_i32(entries.len() as i32);
    let mut holder_ids = BTreeSet::new();
    for entry in entries {
        if entry.holder_id < 0 {
            return Err(ProtocolError::InvalidData(format!(
                "{what} has negative holder id {}",
                entry.holder_id
            )));
        }
        if !holder_ids.insert(entry.holder_id) {
            return Err(ProtocolError::InvalidData(format!(
                "{what} has duplicate holder id {}",
                entry.holder_id
            )));
        }
        if !(0..=u8::MAX.into()).contains(&entry.level) {
            return Err(ProtocolError::InvalidData(format!(
                "{what} level {} is outside network range 0..=255",
                entry.level
            )));
        }
        out.write_var_i32(entry.holder_id);
        out.write_var_i32(entry.level);
    }
    Ok(())
}

pub(super) fn encode_custom_model_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    for (what, len) in [
        ("floats", summary.custom_model_data_floats.len()),
        ("flags", summary.custom_model_data_flags.len()),
        ("strings", summary.custom_model_data_strings.len()),
        ("colors", summary.custom_model_data_colors.len()),
    ] {
        if len > MAX_DATA_COMPONENT_LIST_ITEMS {
            return Err(ProtocolError::InvalidData(format!(
                "custom_model_data {what} list has {len} entries, maximum is \
                 {MAX_DATA_COMPONENT_LIST_ITEMS}"
            )));
        }
    }

    out.write_var_i32(summary.custom_model_data_floats.len() as i32);
    for value in summary.custom_model_data_floats.iter().copied() {
        out.write_f32(value);
    }
    out.write_var_i32(summary.custom_model_data_flags.len() as i32);
    for value in &summary.custom_model_data_flags {
        out.write_bool(*value);
    }
    out.write_var_i32(summary.custom_model_data_strings.len() as i32);
    for value in &summary.custom_model_data_strings {
        out.write_java_string(value, MAX_STRING_CHARS)?;
    }
    out.write_var_i32(summary.custom_model_data_colors.len() as i32);
    for value in &summary.custom_model_data_colors {
        out.write_i32(*value);
    }
    Ok(())
}

pub(super) fn encode_tooltip_display(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let hidden_components = &summary.tooltip_hidden_component_type_ids;
    if hidden_components.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            hidden_components.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    let mut seen = BTreeSet::new();
    for type_id in hidden_components {
        if crate::data_component_types::vanilla_data_component_type(*type_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "tooltip_display has invalid hidden component type id {type_id}"
            )));
        }
        if !seen.insert(*type_id) {
            return Err(ProtocolError::InvalidData(format!(
                "tooltip_display has duplicate hidden component type id {type_id}"
            )));
        }
    }

    out.write_bool(summary.tooltip_hide_tooltip);
    out.write_var_i32(hidden_components.len() as i32);
    for type_id in hidden_components {
        out.write_var_i32(*type_id);
    }
    Ok(())
}

pub(super) fn encode_enchantment_glint_override(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let override_value = summary.enchantment_glint_override.ok_or_else(|| {
        ProtocolError::InvalidData("missing enchantment_glint_override component value".to_string())
    })?;
    out.write_bool(override_value);
    Ok(())
}

pub(super) fn encode_intangible_projectile(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if !summary.intangible_projectile {
        return Err(ProtocolError::InvalidData(
            "missing intangible_projectile component value".to_string(),
        ));
    }
    let unit = NbtSummaryValue::Compound(Vec::new());
    let encoded = encode_nbt_summary_bounded(Some(&unit), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

pub(super) fn encode_tool(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    let tool = summary
        .tool
        .as_ref()
        .ok_or_else(|| ProtocolError::InvalidData("missing tool component value".to_string()))?;
    if tool.rules.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            tool.rules.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    if tool.damage_per_block < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "tool damage_per_block {} is negative",
            tool.damage_per_block
        )));
    }

    out.write_var_i32(tool.rules.len() as i32);
    for (index, rule) in tool.rules.iter().enumerate() {
        encode_holder_set_summary_with_registry(
            out,
            &rule.block_registry_ids,
            rule.block_tag.as_deref(),
            NetworkHolderSetRegistry::Block,
            "tool rule block holder set",
            registry,
        )?;
        match rule.speed_bits {
            Some(bits) => {
                let speed = f32::from_bits(bits);
                if !speed.is_finite() || speed <= 0.0 {
                    return Err(ProtocolError::InvalidData(format!(
                        "tool rule {index} speed {speed} is not a positive finite float"
                    )));
                }
                out.write_bool(true);
                out.write_f32(speed);
            }
            None => out.write_bool(false),
        }
        match rule.correct_for_drops {
            Some(correct_for_drops) => {
                out.write_bool(true);
                out.write_bool(correct_for_drops);
            }
            None => out.write_bool(false),
        }
    }
    out.write_f32(f32::from_bits(tool.default_mining_speed_bits));
    out.write_var_i32(tool.damage_per_block);
    out.write_bool(tool.can_destroy_blocks_in_creative);
    Ok(())
}

pub(super) fn encode_attack_range(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let range = summary.attack_range.ok_or_else(|| {
        ProtocolError::InvalidData("missing attack_range component value".to_string())
    })?;
    for (field, value, max) in [
        ("min_reach", range.min_reach, 64.0),
        ("max_reach", range.max_reach, 64.0),
        ("min_creative_reach", range.min_creative_reach, 64.0),
        ("max_creative_reach", range.max_creative_reach, 64.0),
        ("hitbox_margin", range.hitbox_margin, 1.0),
        ("mob_factor", range.mob_factor, 2.0),
    ] {
        if !value.is_finite() || value.total_cmp(&0.0).is_lt() || value.total_cmp(&max).is_gt() {
            return Err(ProtocolError::InvalidData(format!(
                "attack_range {field} {value} is outside persistent range 0..={max}"
            )));
        }
    }

    out.write_f32(range.min_reach);
    out.write_f32(range.max_reach);
    out.write_f32(range.min_creative_reach);
    out.write_f32(range.max_creative_reach);
    out.write_f32(range.hitbox_margin);
    out.write_f32(range.mob_factor);
    Ok(())
}

pub(super) fn encode_glider(
    _type_id: i32,
    _summary: &DataComponentPatchSummary,
    _out: &mut Encoder,
) -> Result<()> {
    Ok(())
}

pub(super) fn encode_equippable(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    let equippable = summary.equippable.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing equippable component value".to_string())
    })?;
    if vanilla_equipment_slot_by_id(equippable.slot_id).is_none() {
        return Err(ProtocolError::InvalidData(format!(
            "invalid equippable slot id {}",
            equippable.slot_id
        )));
    }

    out.write_var_i32(equippable.slot_id);
    encode_sound_event_holder_summary(out, &equippable.equip_sound)?;
    encode_optional_resource_location(out, equippable.asset_id.as_deref(), "equippable asset_id")?;
    encode_optional_resource_location(
        out,
        equippable.camera_overlay.as_deref(),
        "equippable camera_overlay",
    )?;
    encode_optional_holder_set_summary_with_registry(
        out,
        equippable.allowed_entities.as_ref().map(|entities| {
            (
                entities.registry_ids.as_slice(),
                entities.tag.as_deref(),
                "equippable allowed_entities",
            )
        }),
        NetworkHolderSetRegistry::EntityType,
        registry,
    )?;
    out.write_bool(equippable.dispensable);
    out.write_bool(equippable.swappable);
    out.write_bool(equippable.damage_on_hurt);
    out.write_bool(equippable.equip_on_interact);
    out.write_bool(equippable.can_be_sheared);
    encode_sound_event_holder_summary(out, &equippable.shearing_sound)?;
    Ok(())
}

pub(super) fn encode_optional_resource_location(
    out: &mut Encoder,
    resource_id: Option<&str>,
    what: &str,
) -> Result<()> {
    let Some(resource_id) = resource_id else {
        out.write_bool(false);
        return Ok(());
    };
    validate_canonical_resource_location(resource_id, what)?;
    out.write_bool(true);
    encode_java_string(out, resource_id, MAX_STRING_CHARS)
}

pub(super) fn encode_lore(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    let line_count = summary.lore.len();
    if line_count > MAX_LORE_LINES {
        return Err(ProtocolError::PacketTooLarge(line_count, MAX_LORE_LINES));
    }
    if summary.lore_styled.len() != line_count || summary.lore_nbt.len() != line_count {
        return Err(ProtocolError::InvalidData(
            "lore component projections are not aligned".to_string(),
        ));
    }

    out.write_var_i32(line_count as i32);
    for ((line, styled), nbt) in summary
        .lore
        .iter()
        .zip(&summary.lore_styled)
        .zip(&summary.lore_nbt)
    {
        out.write_bytes(&encode_canonical_component_projection(
            Some(nbt),
            line,
            Some(styled),
            "lore line",
            registry,
        )?);
    }
    Ok(())
}
