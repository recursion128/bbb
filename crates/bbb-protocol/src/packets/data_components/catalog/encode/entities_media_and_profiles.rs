use super::*;

pub(in super::super) fn encode_entity_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    encode_typed_entity_data(
        summary.entity_data_entity_type_id,
        summary.entity_data_tag.as_ref(),
        TypedEntityDataRegistry::EntityType,
        "entity_data",
        out,
        registry,
    )
}

pub(in super::super) fn encode_bucket_entity_data(
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

pub(in super::super) fn encode_block_entity_data(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    encode_typed_entity_data(
        summary.block_entity_data_type_id,
        summary.block_entity_data_tag.as_ref(),
        TypedEntityDataRegistry::BlockEntityType,
        "block_entity_data",
        out,
        registry,
    )
}

pub(in super::super) fn encode_typed_entity_data(
    type_id: Option<i32>,
    tag: Option<&NbtSummaryValue>,
    registry_kind: TypedEntityDataRegistry,
    component: &'static str,
    out: &mut Encoder,
    content_registry: &ContentRegistry,
) -> Result<()> {
    let type_id = type_id.ok_or_else(|| {
        ProtocolError::InvalidData(format!("missing {component} registry holder"))
    })?;
    registry_kind.validate_with_registry(type_id, content_registry)?;
    if let Some(NbtSummaryValue::Compound(entries)) = tag {
        if entries.iter().any(|entry| entry.name == "id") {
            return Err(ProtocolError::InvalidData(format!(
                "{component} canonical network tag must not contain top-level id"
            )));
        }
    }
    out.write_var_i32(type_id);
    encode_required_compound_tag(tag, component, out)?;
    Ok(())
}

pub(in super::super) fn encode_profile(
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

    match profile.kind {
        ResolvableProfileKindSummary::GameProfile => {
            let uuid = profile.uuid.ok_or_else(|| {
                ProtocolError::InvalidData("game profile component is missing its UUID".to_string())
            })?;
            let name = profile.name.as_deref().ok_or_else(|| {
                ProtocolError::InvalidData("game profile component is missing its name".to_string())
            })?;
            out.write_bool(true);
            out.write_uuid(uuid);
            encode_java_string(out, name, MAX_PLAYER_NAME_CHARS)?;
        }
        ResolvableProfileKindSummary::Partial => {
            out.write_bool(false);
            encode_optional_java_string(out, profile.name.as_deref(), MAX_PLAYER_NAME_CHARS)?;
            out.write_bool(profile.uuid.is_some());
            if let Some(uuid) = profile.uuid {
                out.write_uuid(uuid);
            }
        }
    }
    encode_game_profile_properties(out, &profile.properties)?;
    encode_player_skin_patch(out, &profile.skin_patch)?;
    Ok(())
}

pub(in super::super) fn encode_map_id(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let map_id = summary
        .map_id
        .ok_or_else(|| ProtocolError::InvalidData("missing map_id value".to_string()))?;
    out.write_var_i32(map_id);
    Ok(())
}

pub(in super::super) fn encode_dye(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let color_id = summary
        .dye_color_id
        .ok_or_else(|| ProtocolError::InvalidData("missing dye value".to_string()))?;
    if normalize_vanilla_dye_color_id(color_id) != color_id {
        return Err(ProtocolError::InvalidData(format!(
            "dye color id {color_id} is not canonical"
        )));
    }
    out.write_var_i32(color_id);
    Ok(())
}

pub(in super::super) fn encode_dyed_color(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let color = summary
        .dyed_color
        .ok_or_else(|| ProtocolError::InvalidData("missing dyed_color value".to_string()))?;
    out.write_i32(color);
    Ok(())
}

pub(in super::super) fn encode_map_color(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let color = summary
        .map_color
        .ok_or_else(|| ProtocolError::InvalidData("missing map_color value".to_string()))?;
    out.write_i32(color);
    Ok(())
}

pub(in super::super) fn encode_map_post_processing(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let value = summary.map_post_processing.ok_or_else(|| {
        ProtocolError::InvalidData("missing map_post_processing value".to_string())
    })?;
    out.write_var_i32(value.id());
    Ok(())
}

pub(in super::super) fn encode_jukebox_playable(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    match (
        summary.jukebox_song_id,
        summary.jukebox_direct_song.as_deref(),
    ) {
        (Some(holder_id), None) if (0..i32::MAX).contains(&holder_id) => {
            out.write_var_i32(holder_id + 1);
        }
        (None, Some(song)) => {
            let length_in_seconds = decode_persistent_positive_float(
                song.length_in_seconds_bits,
                "jukebox song length_in_seconds",
            )?;
            if !(0..=15).contains(&song.comparator_output) {
                return Err(ProtocolError::InvalidData(format!(
                    "jukebox song comparator_output {} is outside 0..=15",
                    song.comparator_output
                )));
            }
            out.write_var_i32(0);
            encode_sound_event_holder_summary(out, &song.sound_event)?;
            out.write_bytes(&encode_canonical_component_projection(
                song.description_nbt.as_deref(),
                &song.description,
                song.description_styled.as_deref(),
                "jukebox song description",
                registry,
            )?);
            out.write_f32(length_in_seconds);
            out.write_var_i32(song.comparator_output);
        }
        _ => {
            return Err(ProtocolError::InvalidData(
                "jukebox_playable holder projections are not canonical".to_string(),
            ));
        }
    }
    Ok(())
}

pub(in super::super) fn encode_instrument(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    match (
        summary.instrument.registry_id,
        summary.instrument.direct.as_deref(),
    ) {
        (Some(holder_id), None) if (0..i32::MAX).contains(&holder_id) => {
            out.write_var_i32(holder_id + 1);
        }
        (None, Some(instrument)) => {
            let use_duration = decode_persistent_positive_float(
                instrument.use_duration_bits,
                "instrument use_duration",
            )?;
            let range =
                decode_persistent_positive_float(instrument.range_bits, "instrument range")?;
            out.write_var_i32(0);
            encode_sound_event_holder_summary(out, &instrument.sound_event)?;
            out.write_f32(use_duration);
            out.write_f32(range);
            out.write_bytes(&encode_canonical_component_projection(
                instrument.description_nbt.as_deref(),
                &instrument.description,
                instrument.description_styled.as_deref(),
                "instrument description",
                registry,
            )?);
        }
        _ => {
            return Err(ProtocolError::InvalidData(
                "instrument holder projections are not canonical".to_string(),
            ));
        }
    }
    Ok(())
}

pub(in super::super) fn decode_persistent_positive_float(bits: u32, what: &str) -> Result<f32> {
    let value = f32::from_bits(bits);
    if value > 0.0 && value <= f32::MAX {
        return Ok(value);
    }
    Err(ProtocolError::InvalidData(format!(
        "{what} must be positive, got {value}"
    )))
}

pub(in super::super) fn encode_banner_patterns(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if summary.banner_pattern_layers.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            summary.banner_pattern_layers.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    out.write_var_i32(summary.banner_pattern_layers.len() as i32);
    for layer in &summary.banner_pattern_layers {
        encode_banner_pattern_holder(out, layer)?;
        if normalize_vanilla_dye_color_id(layer.color_id) != layer.color_id {
            return Err(ProtocolError::InvalidData(format!(
                "banner pattern has noncanonical dye color id {}",
                layer.color_id
            )));
        }
        out.write_var_i32(layer.color_id);
    }
    Ok(())
}

pub(in super::super) fn encode_banner_pattern_holder(
    out: &mut Encoder,
    layer: &BannerPatternLayerSummary,
) -> Result<()> {
    match (
        layer.registry_id,
        layer.asset_id.as_deref(),
        layer.translation_key.as_ref(),
    ) {
        (Some(registry_id), None, None) if (0..i32::MAX).contains(&registry_id) => {
            out.write_var_i32(registry_id + 1);
            Ok(())
        }
        (None, Some(asset_id), Some(translation_key)) => {
            validate_canonical_resource_location(asset_id, "banner pattern asset id")?;
            out.write_var_i32(0);
            encode_java_string(out, asset_id, MAX_STRING_CHARS)?;
            if translation_key.units().len() > MAX_STRING_CHARS {
                return Err(ProtocolError::StringTooLong {
                    actual: translation_key.units().len(),
                    max: MAX_STRING_CHARS,
                });
            }
            out.write_utf16_string(translation_key.units());
            Ok(())
        }
        _ => Err(ProtocolError::InvalidData(
            "banner pattern holder projections are not canonical".to_string(),
        )),
    }
}

pub(in super::super) fn encode_lodestone_tracker(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let tracked = summary.lodestone_tracked.ok_or_else(|| {
        ProtocolError::InvalidData("lodestone_tracker is missing tracked projection".to_string())
    })?;
    out.write_bool(summary.lodestone_target.is_some());
    if let Some(target) = &summary.lodestone_target {
        validate_canonical_resource_location(
            &target.dimension,
            "lodestone_tracker target dimension",
        )?;
        encode_java_string(out, &target.dimension, MAX_STRING_CHARS)?;
        let packed = chunks::encode_block_pos(target.pos);
        out.write_i64(packed);
    }
    out.write_bool(tracked);
    Ok(())
}

pub(in super::super) fn encode_container(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    encode_item_container_contents_with_registry(out, summary, registry)
}

pub(in super::super) fn encode_bees(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    if !summary.bees_present {
        return Err(ProtocolError::InvalidData(
            "bees presence projection is not canonical".to_string(),
        ));
    }
    if summary.bees.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            summary.bees.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }

    out.write_var_i32(summary.bees.len() as i32);
    for bee in &summary.bees {
        encode_typed_entity_data(
            Some(bee.entity_type_id),
            bee.entity_data_tag.as_ref(),
            TypedEntityDataRegistry::EntityType,
            "bees entity_data",
            out,
            registry,
        )?;
        out.write_var_i32(bee.ticks_in_hive);
        out.write_var_i32(bee.min_ticks_in_hive);
    }
    Ok(())
}

pub(in super::super) fn encode_painting_variant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    match (
        summary.painting_variant_id,
        summary.painting_variant_direct.as_ref(),
    ) {
        (Some(holder_id), None) if (0..i32::MAX).contains(&holder_id) => {
            out.write_var_i32(holder_id + 1);
        }
        (None, Some(variant)) => {
            validate_canonical_resource_location(&variant.asset_id, "painting variant asset id")?;
            out.write_var_i32(0);
            out.write_var_i32(variant.width);
            out.write_var_i32(variant.height);
            encode_java_string(out, &variant.asset_id, MAX_STRING_CHARS)?;
            encode_optional_painting_component(
                out,
                variant.title.as_deref(),
                variant.title_styled.as_deref(),
                variant.title_nbt.as_deref(),
                "painting variant title",
                registry,
            )?;
            encode_optional_painting_component(
                out,
                variant.author.as_deref(),
                variant.author_styled.as_deref(),
                variant.author_nbt.as_deref(),
                "painting variant author",
                registry,
            )?;
        }
        _ => {
            return Err(ProtocolError::InvalidData(
                "painting variant holder projections are not canonical".to_string(),
            ));
        }
    }
    Ok(())
}

pub(in super::super) fn encode_optional_painting_component(
    out: &mut Encoder,
    text: Option<&str>,
    styled: Option<&[StyledTextRun]>,
    nbt: Option<&NbtSummaryValue>,
    what: &str,
    registry: &ContentRegistry,
) -> Result<()> {
    match (text, styled, nbt) {
        (None, None, None) => {
            out.write_bool(false);
            Ok(())
        }
        (Some(text), Some(styled), Some(nbt)) => {
            out.write_bool(true);
            out.write_bytes(&encode_canonical_component_projection(
                Some(nbt),
                text,
                Some(styled),
                what,
                registry,
            )?);
            Ok(())
        }
        _ => Err(ProtocolError::InvalidData(format!(
            "{what} optional component projections are not aligned"
        ))),
    }
}

pub(in super::super) fn encode_game_profile_properties(
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
        for (value, max) in [
            (&property.name, MAX_PROFILE_PROPERTY_NAME_CHARS),
            (&property.value, MAX_STRING_CHARS),
        ] {
            if value.units().len() > max {
                return Err(ProtocolError::StringTooLong {
                    actual: value.units().len(),
                    max,
                });
            }
            out.write_utf16_string(value.units());
        }
        out.write_bool(property.signature.is_some());
        if let Some(signature) = &property.signature {
            if signature.units().len() > MAX_PROFILE_SIGNATURE_CHARS {
                return Err(ProtocolError::StringTooLong {
                    actual: signature.units().len(),
                    max: MAX_PROFILE_SIGNATURE_CHARS,
                });
            }
            out.write_utf16_string(signature.units());
        }
    }
    Ok(())
}

pub(in super::super) fn encode_player_skin_patch(
    out: &mut Encoder,
    patch: &PlayerSkinPatchSummary,
) -> Result<()> {
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

pub(in super::super) fn encode_optional_java_string(
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

pub(in super::super) fn encode_java_string(
    out: &mut Encoder,
    value: &str,
    max_utf16_units: usize,
) -> Result<()> {
    validate_java_string_len(value, max_utf16_units)?;
    out.write_string(value);
    Ok(())
}
