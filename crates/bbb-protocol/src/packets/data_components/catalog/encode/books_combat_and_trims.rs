use super::*;

pub(in super::super) fn encode_writable_book_content(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let page_count = summary.writable_book_pages.len();
    if page_count > MAX_BOOK_PAGES {
        return Err(ProtocolError::PacketTooLarge(page_count, MAX_BOOK_PAGES));
    }
    if summary.writable_book_page_filters.len() != page_count {
        return Err(ProtocolError::InvalidData(
            "writable book page projections are not aligned".to_string(),
        ));
    }

    out.write_var_i32(page_count as i32);
    for (page, filtered) in summary
        .writable_book_pages
        .iter()
        .zip(&summary.writable_book_page_filters)
    {
        for value in std::iter::once(page).chain(filtered.iter()) {
            if value.units().len() > MAX_WRITABLE_BOOK_PAGE_CHARS {
                return Err(ProtocolError::StringTooLong {
                    actual: value.units().len(),
                    max: MAX_WRITABLE_BOOK_PAGE_CHARS,
                });
            }
        }
        out.write_utf16_string(page.units());
        out.write_bool(filtered.is_some());
        if let Some(filtered) = filtered {
            out.write_utf16_string(filtered.units());
        }
    }
    Ok(())
}

pub(in super::super) fn encode_piercing_weapon(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let weapon = summary.piercing_weapon.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing piercing_weapon component value".to_string())
    })?;
    out.write_bool(weapon.deals_knockback);
    out.write_bool(weapon.dismounts);
    encode_optional_sound_event_holder_summary(out, weapon.sound.as_ref())?;
    encode_optional_sound_event_holder_summary(out, weapon.hit_sound.as_ref())?;
    Ok(())
}

pub(in super::super) fn encode_kinetic_weapon(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let weapon = summary.kinetic_weapon.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing kinetic_weapon component value".to_string())
    })?;
    out.write_var_i32(weapon.contact_cooldown_ticks);
    out.write_var_i32(weapon.delay_ticks);
    encode_optional_kinetic_weapon_condition(out, weapon.dismount_conditions)?;
    encode_optional_kinetic_weapon_condition(out, weapon.knockback_conditions)?;
    encode_optional_kinetic_weapon_condition(out, weapon.damage_conditions)?;
    out.write_f32(f32::from_bits(weapon.forward_movement_bits));
    out.write_f32(f32::from_bits(weapon.damage_multiplier_bits));
    encode_optional_sound_event_holder_summary(out, weapon.sound.as_ref())?;
    encode_optional_sound_event_holder_summary(out, weapon.hit_sound.as_ref())?;
    Ok(())
}

pub(in super::super) fn encode_optional_kinetic_weapon_condition(
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

pub(in super::super) fn encode_swing_animation(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let animation = summary.swing_animation.ok_or_else(|| {
        ProtocolError::InvalidData("missing swing_animation component value".to_string())
    })?;
    out.write_var_i32(animation.animation_type.id());
    out.write_var_i32(animation.duration);
    Ok(())
}

pub(in super::super) fn encode_blocks_attacks(
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

    out.write_f32(f32::from_bits(value.block_delay_seconds_bits));
    out.write_f32(f32::from_bits(value.disable_cooldown_scale_bits));
    out.write_var_i32(value.damage_reductions.len() as i32);
    for reduction in &value.damage_reductions {
        out.write_f32(f32::from_bits(reduction.horizontal_blocking_angle_bits));
        encode_optional_holder_set_summary(
            out,
            reduction.damage_types.as_ref().map(|types| {
                (
                    types.registry_ids.as_slice(),
                    types.tag.as_deref(),
                    "blocks_attacks damage reduction type",
                )
            }),
            NetworkHolderSetRegistry::DamageType,
        )?;
        out.write_f32(f32::from_bits(reduction.base_bits));
        out.write_f32(f32::from_bits(reduction.factor_bits));
    }
    out.write_f32(f32::from_bits(value.item_damage.threshold_bits));
    out.write_f32(f32::from_bits(value.item_damage.base_bits));
    out.write_f32(f32::from_bits(value.item_damage.factor_bits));
    encode_optional_holder_set_summary(
        out,
        value.bypassed_by.as_ref().map(|types| {
            (
                types.registry_ids.as_slice(),
                types.tag.as_deref(),
                "blocks_attacks bypassed_by",
            )
        }),
        NetworkHolderSetRegistry::DamageType,
    )?;
    encode_optional_sound_event_holder_summary(out, value.block_sound.as_ref())?;
    encode_optional_sound_event_holder_summary(out, value.disable_sound.as_ref())?;

    Ok(())
}

pub(in super::super) fn encode_optional_holder_set_summary(
    out: &mut Encoder,
    value: Option<(&[i32], Option<&str>, &'static str)>,
    registry: NetworkHolderSetRegistry,
) -> Result<()> {
    encode_optional_holder_set_summary_with_registry(
        out,
        value,
        registry,
        ContentRegistry::vanilla(),
    )
}

pub(in super::super) fn encode_optional_holder_set_summary_with_registry(
    out: &mut Encoder,
    value: Option<(&[i32], Option<&str>, &'static str)>,
    registry: NetworkHolderSetRegistry,
    content_registry: &ContentRegistry,
) -> Result<()> {
    let Some((registry_ids, tag, what)) = value else {
        out.write_bool(false);
        return Ok(());
    };
    out.write_bool(true);
    encode_holder_set_summary_with_registry(
        out,
        registry_ids,
        tag,
        registry,
        what,
        content_registry,
    )
}

pub(in super::super) fn encode_holder_set_summary(
    out: &mut Encoder,
    registry_ids: &[i32],
    tag: Option<&str>,
    registry: NetworkHolderSetRegistry,
    what: &str,
) -> Result<()> {
    encode_holder_set_summary_with_registry(
        out,
        registry_ids,
        tag,
        registry,
        what,
        ContentRegistry::vanilla(),
    )
}

pub(in super::super) fn encode_holder_set_summary_with_registry(
    out: &mut Encoder,
    registry_ids: &[i32],
    tag: Option<&str>,
    registry: NetworkHolderSetRegistry,
    what: &str,
    content_registry: &ContentRegistry,
) -> Result<()> {
    // A named set retains resolved members for gameplay; its wire value remains the tag.
    match tag {
        Some(tag) => {
            if !crate::packets::is_canonical_resource_location(tag)? {
                return Err(ProtocolError::InvalidData(format!(
                    "{what} has noncanonical tag resource location `{tag}`"
                )));
            }
            out.write_var_i32(0);
            encode_java_string(out, tag, MAX_STRING_CHARS)
        }
        None => {
            if registry_ids.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(
                    registry_ids.len(),
                    MAX_DATA_COMPONENT_LIST_ITEMS,
                ));
            }
            out.write_var_i32(registry_ids.len() as i32 + 1);
            for holder_id in registry_ids {
                registry.validate_holder_id_with_registry(*holder_id, content_registry)?;
                out.write_var_i32(*holder_id);
            }
            Ok(())
        }
    }
}

pub(in super::super) fn encode_optional_sound_event_holder_summary(
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

pub(in super::super) fn encode_sound_event_holder_summary(
    out: &mut Encoder,
    sound: &SoundEventSummary,
) -> Result<()> {
    match (
        sound.registry_id,
        sound.sound_id.as_deref(),
        sound.fixed_range_bits,
    ) {
        (Some(registry_id), None, None) if (0..i32::MAX).contains(&registry_id) => {
            out.write_var_i32(registry_id + 1);
            Ok(())
        }
        (None, Some(sound_id), fixed_range_bits) => {
            if !crate::packets::is_canonical_resource_location(sound_id)? {
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

pub(in super::super) fn encode_written_book_content(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
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

    for title in std::iter::once(&book.title).chain(book.title_filter.iter()) {
        if title.units().len() > MAX_WRITTEN_BOOK_TITLE_CHARS {
            return Err(ProtocolError::StringTooLong {
                actual: title.units().len(),
                max: MAX_WRITTEN_BOOK_TITLE_CHARS,
            });
        }
    }
    out.write_utf16_string(book.title.units());
    out.write_bool(book.title_filter.is_some());
    if let Some(filtered) = &book.title_filter {
        out.write_utf16_string(filtered.units());
    }
    if book.author.units().len() > MAX_STRING_CHARS {
        return Err(ProtocolError::StringTooLong {
            actual: book.author.units().len(),
            max: MAX_STRING_CHARS,
        });
    }
    out.write_utf16_string(book.author.units());
    out.write_var_i32(book.generation);
    out.write_var_i32(page_count as i32);
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
            registry,
        )?;
        out.write_bytes(&raw);

        match (
            book.page_filters[index].as_deref(),
            summary.written_book_page_filters_nbt[index].as_ref(),
        ) {
            (None, None) => out.write_bool(false),
            (Some(text), Some(nbt)) => {
                let filtered = encode_canonical_written_book_page(
                    nbt,
                    text,
                    &format!("written book page {index} filtered"),
                    registry,
                )?;
                out.write_bool(true);
                out.write_bytes(&filtered);
            }
            _ => {
                return Err(ProtocolError::InvalidData(format!(
                    "written book page {index} filtered projections are not aligned"
                )));
            }
        }
    }
    out.write_bool(book.resolved);
    Ok(())
}

pub(in super::super) fn encode_trim(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    encode_trim_material_holder(
        out,
        summary.armor_trim_material_id,
        summary.armor_trim_material_direct.as_deref(),
        registry,
    )?;
    encode_trim_pattern_holder(
        out,
        summary.armor_trim_pattern_id,
        summary.armor_trim_pattern_direct.as_deref(),
        registry,
    )?;
    Ok(())
}

pub(in super::super) fn encode_provides_trim_material(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    encode_trim_material_holder(
        out,
        summary.provides_trim_material_id,
        summary.provides_trim_material_direct.as_deref(),
        registry,
    )?;
    Ok(())
}

pub(in super::super) fn encode_ominous_bottle_amplifier(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let amplifier = summary.ominous_bottle_amplifier.ok_or_else(|| {
        ProtocolError::InvalidData("missing ominous_bottle_amplifier component value".to_string())
    })?;
    if !(0..=4).contains(&amplifier) {
        return Err(ProtocolError::InvalidData(format!(
            "ominous_bottle_amplifier {amplifier} is outside persistent range 0..=4"
        )));
    }
    out.write_var_i32(amplifier);
    Ok(())
}

pub(in super::super) fn encode_break_sound(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let sound = summary
        .break_sound
        .as_ref()
        .ok_or_else(|| ProtocolError::InvalidData("missing break_sound value".to_string()))?;
    encode_sound_event_holder_summary(out, sound)?;
    Ok(())
}

pub(in super::super) fn encode_firework_explosion(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let explosion = summary.firework_explosion.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing firework_explosion value".to_string())
    })?;
    encode_firework_explosion_value(out, explosion)
}

pub(in super::super) fn encode_fireworks(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let flight_duration = summary.fireworks_flight_duration.ok_or_else(|| {
        ProtocolError::InvalidData("missing fireworks flight duration".to_string())
    })?;
    let explosion_count = summary.fireworks_explosions.len();
    if explosion_count > MAX_FIREWORK_EXPLOSIONS {
        return Err(ProtocolError::PacketTooLarge(
            explosion_count,
            MAX_FIREWORK_EXPLOSIONS,
        ));
    }

    out.write_var_i32(flight_duration);
    out.write_var_i32(explosion_count as i32);
    for explosion in &summary.fireworks_explosions {
        encode_firework_explosion_value(out, explosion)?;
    }
    Ok(())
}

pub(in super::super) fn encode_charged_projectiles(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    encode_item_stack_template_list_with_registry(
        out,
        &summary.charged_projectiles_items,
        MAX_DATA_COMPONENT_LIST_ITEMS,
        "charged_projectiles",
        registry,
    )
}

pub(in super::super) fn encode_bundle_contents(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    encode_item_stack_template_list_with_registry(
        out,
        &summary.bundle_contents_items,
        MAX_DATA_COMPONENT_LIST_ITEMS,
        "bundle_contents",
        registry,
    )
}

pub(crate) fn validate_persistent_recursive_component_with_registry(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<bool> {
    match type_id {
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => {
            require_use_remainder(summary)?;
        }
        VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => {
            validate_item_stack_template_list_shape(
                &summary.charged_projectiles_items,
                MAX_DATA_COMPONENT_LIST_ITEMS,
                "charged_projectiles",
            )?;
        }
        VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => {
            validate_item_stack_template_list_shape(
                &summary.bundle_contents_items,
                MAX_DATA_COMPONENT_LIST_ITEMS,
                "bundle_contents",
            )?;
        }
        VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => {
            validate_item_container_shape(summary)?;
        }
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID | VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID => {
            let predicate = validate_adventure_mode_predicate_component(type_id, summary)?;
            encode_adventure_mode_predicate_value_with(
                &mut Encoder::new(),
                predicate,
                registry,
                encode_persistent_matchers,
            )?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

pub(in super::super) fn encode_firework_explosion_value(
    out: &mut Encoder,
    explosion: &FireworkExplosionSummary,
) -> Result<()> {
    out.write_var_i32(explosion.shape.id());
    encode_firework_color_list(out, &explosion.colors)?;
    encode_firework_color_list(out, &explosion.fade_colors)?;
    out.write_bool(explosion.has_trail);
    out.write_bool(explosion.has_twinkle);
    Ok(())
}

pub(in super::super) fn encode_firework_color_list(
    out: &mut Encoder,
    colors: &[i32],
) -> Result<()> {
    if colors.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            colors.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    out.write_var_i32(colors.len() as i32);
    for color in colors {
        out.write_i32(*color);
    }
    Ok(())
}

pub(in super::super) fn encode_trim_material_holder(
    out: &mut Encoder,
    registry_id: Option<i32>,
    direct: Option<&TrimMaterialSummary>,
    registry: &ContentRegistry,
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
                registry,
            )?;
            out.write_bytes(&description);
            Ok(())
        }
        _ => Err(ProtocolError::InvalidData(
            "trim material holder projections are not canonical".to_string(),
        )),
    }
}

pub(crate) fn encode_trim_pattern_holder(
    out: &mut Encoder,
    registry_id: Option<i32>,
    direct: Option<&TrimPatternSummary>,
    registry: &ContentRegistry,
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
                registry,
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

pub(in super::super) fn validate_trim_material_asset_suffix(
    suffix: &str,
    field: &str,
) -> Result<()> {
    if !is_valid_resource_path(suffix) {
        return Err(ProtocolError::InvalidData(format!(
            "trim material {field} `{suffix}` is not a valid resource path"
        )));
    }
    Ok(())
}

pub(in super::super) fn validate_canonical_resource_location(
    resource_id: &str,
    what: &str,
) -> Result<()> {
    if !crate::packets::is_canonical_resource_location(resource_id)? {
        return Err(ProtocolError::InvalidData(format!(
            "{what} has noncanonical resource location `{resource_id}`"
        )));
    }
    Ok(())
}

pub(in super::super) fn encode_canonical_component_projection(
    nbt: Option<&NbtSummaryValue>,
    expected_text: &str,
    expected_styled: Option<&[StyledTextRun]>,
    what: &str,
    registry: &ContentRegistry,
) -> Result<Vec<u8>> {
    let nbt =
        nbt.ok_or_else(|| ProtocolError::InvalidData(format!("{what} is missing component nbt")))?;
    let expected_styled = expected_styled.ok_or_else(|| {
        ProtocolError::InvalidData(format!("{what} is missing styled projection"))
    })?;
    let encoded = encode_nbt_summary_bounded(Some(nbt), i32::MAX as usize, MAX_NBT_DEPTH)?;
    let (canonical_runs, canonical_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(
            &mut Decoder::new(&encoded),
            registry,
        )?;
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

pub(in super::super) fn encode_canonical_written_book_page(
    nbt: &NbtSummaryValue,
    expected_text: &str,
    what: &str,
    registry: &ContentRegistry,
) -> Result<Vec<u8>> {
    let encoded = encode_nbt_summary_bounded(Some(nbt), i32::MAX as usize, MAX_NBT_DEPTH)?;
    let (runs, canonical_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(
            &mut Decoder::new(&encoded),
            registry,
        )?;
    if styled_runs_summary_text(&runs) != expected_text || canonical_nbt.as_ref() != Some(nbt) {
        return Err(ProtocolError::InvalidData(format!(
            "{what} contains noncanonical component projections"
        )));
    }
    Ok(encoded)
}

pub(crate) fn decode_required_compound_tag(
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

pub(in super::super) fn encode_required_compound_tag(
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
