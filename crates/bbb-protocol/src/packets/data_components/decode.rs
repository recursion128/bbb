use super::*;

pub(crate) fn decode_data_component_exact_predicate_summary_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<DataComponentPatchSummary> {
    let component_count = decoder.read_len()?;
    if component_count > MAX_DATA_COMPONENT_PREDICATE_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            component_count,
            MAX_DATA_COMPONENT_PREDICATE_ENTRIES,
        ));
    }
    let summary = decode_typed_data_component_patch_summary(
        decoder,
        component_count,
        ComponentValueFraming::Direct,
        registry,
    )?;
    Ok(summary)
}

pub(super) fn decode_typed_data_component_patch_summary(
    decoder: &mut Decoder<'_>,
    count: usize,
    framing: ComponentValueFraming,
    registry: &ContentRegistry,
) -> Result<DataComponentPatchSummary> {
    let mut seen = [None; crate::data_component_types::VANILLA_DATA_COMPONENT_TYPES.len()];
    decode_typed_data_component_patch_summary_with_seen(
        decoder, count, framing, registry, &mut seen,
    )
}

pub(super) fn decode_typed_data_component_patch_summary_with_seen(
    decoder: &mut Decoder<'_>,
    count: usize,
    framing: ComponentValueFraming,
    registry: &ContentRegistry,
    seen: &mut [Option<bool>; crate::data_component_types::VANILLA_DATA_COMPONENT_TYPES.len()],
) -> Result<DataComponentPatchSummary> {
    let mut summary = DataComponentPatchSummary {
        added_type_ids: Vec::with_capacity(decoder.bounded_capacity(count, 1)),
        ..DataComponentPatchSummary::default()
    };
    for _ in 0..count {
        let type_id = decoder.read_var_i32()?;
        let descriptor = data_component_descriptor_or_error(type_id)?;
        let already_added = seen[type_id as usize] == Some(false);
        if already_added {
            // DataComponentPatch.java:88-92: duplicate positive entries overwrite in map order.
            fields::clear_component_value(type_id, &mut summary)?;
        }
        match framing {
            ComponentValueFraming::Direct => {
                descriptor.decode_with_registry(decoder, &mut summary, registry)?
            }
            ComponentValueFraming::LengthPrefixed => {
                let len = decoder.read_len()?;
                let value = decoder.read_exact(len, "delimited data component value")?;
                let mut value = Decoder::new(value);
                descriptor.decode_with_registry(&mut value, &mut summary, registry)?;
                if !value.is_empty() {
                    return Err(ProtocolError::InvalidData(format!(
                        "trailing bytes after delimited data component type id {type_id}"
                    )));
                }
            }
        }
        if !already_added {
            seen[type_id as usize] = Some(false);
            summary.added_type_ids.push(type_id);
        }
    }
    Ok(summary)
}

pub(super) fn data_component_descriptor_or_error(
    type_id: i32,
) -> Result<&'static catalog::DataComponentDescriptor> {
    catalog::vanilla_data_component_descriptor(type_id).ok_or_else(|| {
        ProtocolError::InvalidData(format!("unsupported data component type id {type_id}"))
    })
}

pub(super) fn decode_holder_registry_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    Ok(id)
}

pub(super) fn decode_registry_or_inline<T>(
    decoder: &mut Decoder<'_>,
    decode_direct: impl FnOnce(&mut Decoder<'_>) -> Result<T>,
) -> Result<(Option<i32>, Option<T>)> {
    let id = decode_holder_registry_id(decoder)?;
    if id == 0 {
        Ok((None, Some(decode_direct(decoder)?)))
    } else {
        Ok((Some(id - 1), None))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NetworkHolderSetRegistry {
    BannerPattern,
    Block,
    DamageType,
    EntityType,
    Item,
    MobEffect,
}

impl NetworkHolderSetRegistry {
    pub(super) fn validate_holder_id_with_registry(
        self,
        holder_id: i32,
        registry: &ContentRegistry,
    ) -> Result<i32> {
        let valid = match self {
            Self::BannerPattern | Self::DamageType => holder_id >= 0,
            Self::Block => {
                crate::block_types::vanilla_block_type(holder_id).is_some()
                    || registry.block_by_id(holder_id).is_some()
            }
            Self::EntityType => {
                vanilla_entity_type(holder_id).is_some()
                    || registry.entity_by_id(holder_id).is_some()
            }
            Self::Item => {
                crate::item_types::vanilla_item_type(holder_id).is_some()
                    || registry.item_by_id(holder_id).is_some()
            }
            Self::MobEffect => vanilla_mob_effect_type(holder_id).is_some(),
        };
        if valid {
            return Ok(holder_id);
        }
        Err(ProtocolError::InvalidData(format!(
            "invalid {} holder id {holder_id}",
            self.wire_name()
        )))
    }

    pub(super) const fn wire_name(self) -> &'static str {
        match self {
            Self::BannerPattern => "banner pattern",
            Self::Block => "block",
            Self::DamageType => "damage type",
            Self::EntityType => "entity type",
            Self::Item => "item",
            Self::MobEffect => "mob effect",
        }
    }
}

pub(super) fn decode_holder_set_summary(
    decoder: &mut Decoder<'_>,
    registry: NetworkHolderSetRegistry,
) -> Result<HolderSetSummary> {
    decode_holder_set_summary_with_registry(decoder, registry, ContentRegistry::vanilla())
}

pub(super) fn decode_holder_set_summary_with_registry(
    decoder: &mut Decoder<'_>,
    registry: NetworkHolderSetRegistry,
    content_registry: &ContentRegistry,
) -> Result<HolderSetSummary> {
    let encoded_count = decoder.read_var_i32()?;
    if encoded_count < 0 {
        return Err(ProtocolError::NegativeLength(encoded_count));
    }
    if encoded_count == 0 {
        let tag = read_resource_location(decoder)?;
        return Ok(HolderSetSummary {
            registry_ids: Vec::new(),
            tag: Some(tag),
        });
    }

    let count = (encoded_count - 1) as usize;
    if count > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    let mut registry_ids = Vec::with_capacity(decoder.bounded_capacity(count, 1));
    for _ in 0..count {
        registry_ids.push(
            registry.validate_holder_id_with_registry(decoder.read_var_i32()?, content_registry)?,
        );
    }
    Ok(HolderSetSummary {
        registry_ids,
        tag: None,
    })
}

pub(super) fn decode_optional_identifier_value(
    decoder: &mut Decoder<'_>,
) -> Result<Option<String>> {
    if decoder.read_bool()? {
        return read_resource_location(decoder).map(Some);
    }
    Ok(None)
}

pub(super) fn decode_optional_i32_value(decoder: &mut Decoder<'_>) -> Result<Option<i32>> {
    if decoder.read_bool()? {
        return Ok(Some(decoder.read_i32()?));
    }
    Ok(None)
}

pub(super) fn decode_optional_f32_bits(decoder: &mut Decoder<'_>) -> Result<Option<u32>> {
    if decoder.read_bool()? {
        return Ok(Some(decoder.read_f32()?.to_bits()));
    }
    Ok(None)
}

pub(super) fn decode_optional_bool_value(decoder: &mut Decoder<'_>) -> Result<Option<bool>> {
    if decoder.read_bool()? {
        return Ok(Some(decoder.read_bool()?));
    }
    Ok(None)
}

pub(super) fn decode_optional_sound_event_holder_summary(
    decoder: &mut Decoder<'_>,
) -> Result<Option<SoundEventSummary>> {
    if decoder.read_bool()? {
        return decode_sound_event_holder_summary(decoder).map(Some);
    }
    Ok(None)
}

pub(super) fn decode_optional_global_pos(
    decoder: &mut Decoder<'_>,
) -> Result<Option<LodestoneTargetSummary>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    decode_global_pos(decoder).map(Some)
}

pub(super) fn decode_global_pos(decoder: &mut Decoder<'_>) -> Result<LodestoneTargetSummary> {
    Ok(LodestoneTargetSummary {
        dimension: read_resource_location(decoder)?,
        pos: chunks::decode_block_pos(decoder.read_i64()?),
    })
}

pub(super) fn decode_item_rarity(decoder: &mut Decoder<'_>) -> Result<ItemRaritySummary> {
    Ok(ItemRaritySummary::from_id(decoder.read_var_i32()?))
}

pub(super) fn decode_varint_map(decoder: &mut Decoder<'_>) -> Result<Vec<ItemEnchantmentSummary>> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut entries: Vec<ItemEnchantmentSummary> =
        Vec::with_capacity(decoder.bounded_capacity(count, 2));
    let mut positions: BTreeMap<i32, usize> = BTreeMap::new();
    for _ in 0..count {
        let holder_id = decode_holder_registry_id(decoder)?;
        let level = decoder.read_var_i32()?;
        if let Some(index) = positions.get(&holder_id).copied() {
            entries[index].level = level;
        } else {
            positions.insert(holder_id, entries.len());
            entries.push(ItemEnchantmentSummary { holder_id, level });
        }
    }
    // `ByteBufCodecs.map` overwrites duplicate keys before the
    // `ItemEnchantments` constructor validates the final map.
    if let Some(entry) = entries
        .iter()
        .find(|entry| !(0..=u8::MAX.into()).contains(&entry.level))
    {
        return Err(ProtocolError::InvalidData(format!(
            "enchantment level {} is outside network range 0..=255",
            entry.level
        )));
    }
    Ok(entries)
}

pub(super) fn decode_map_post_processing(
    decoder: &mut Decoder<'_>,
) -> Result<MapPostProcessingSummary> {
    Ok(MapPostProcessingSummary::from_id(decoder.read_var_i32()?))
}

pub(super) fn decode_adventure_mode_predicate_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<AdventureModePredicateDataSummary> {
    decoder.with_recursion_limit(
        MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH,
        "adventure mode predicate",
        |decoder| {
            let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
            let mut predicates = Vec::with_capacity(decoder.bounded_capacity(count, 5));
            for _ in 0..count {
                predicates.push(decode_block_predicate_summary(decoder, registry)?);
            }
            Ok(AdventureModePredicateDataSummary { predicates })
        },
    )
}

pub(super) fn decode_block_predicate_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<BlockPredicateSummary> {
    let blocks = if decoder.read_bool()? {
        Some(decode_holder_set_summary_with_registry(
            decoder,
            NetworkHolderSetRegistry::Block,
            registry,
        )?)
    } else {
        None
    };
    let state = if decoder.read_bool()? {
        Some(decode_state_properties_predicate(decoder)?)
    } else {
        None
    };
    let nbt = if decoder.read_bool()? {
        Some(catalog::decode_required_compound_tag(
            decoder,
            "block predicate nbt",
        )?)
    } else {
        None
    };
    let (exact_components, partial_components) = decode_data_component_matchers(decoder, registry)?;
    Ok(BlockPredicateSummary {
        blocks,
        state,
        nbt,
        exact_components,
        partial_components,
    })
}

pub(super) fn decode_state_properties_predicate(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<StatePropertyMatcherSummary>> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut properties = Vec::with_capacity(decoder.bounded_capacity(count, 2));
    for _ in 0..count {
        properties.push(StatePropertyMatcherSummary {
            name: decoder.read_string(MAX_STRING_CHARS)?.into(),
            value: decode_state_property_value_matcher(decoder)?,
        });
    }
    Ok(properties)
}

pub(super) fn decode_state_property_value_matcher(
    decoder: &mut Decoder<'_>,
) -> Result<StatePropertyValueMatcherSummary> {
    if decoder.read_bool()? {
        Ok(StatePropertyValueMatcherSummary::Exact(
            decoder.read_string(MAX_STRING_CHARS)?.into(),
        ))
    } else {
        Ok(StatePropertyValueMatcherSummary::Range {
            min: decode_optional_string_value(decoder, MAX_STRING_CHARS)?.map(Into::into),
            max: decode_optional_string_value(decoder, MAX_STRING_CHARS)?.map(Into::into),
        })
    }
}

pub(super) fn decode_data_component_matchers(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<(
    Option<Box<DataComponentPatchSummary>>,
    Vec<PartialDataComponentPredicateSummary>,
)> {
    let exact_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_PREDICATE_ENTRIES)?;
    let exact_components = if exact_count == 0 {
        None
    } else {
        let summary = decode_typed_data_component_patch_summary(
            decoder,
            exact_count,
            ComponentValueFraming::Direct,
            registry,
        )?;
        Some(Box::new(summary))
    };

    let partial_count = read_bounded_len(decoder, MAX_PARTIAL_DATA_COMPONENT_PREDICATES)?;
    let mut partial_components = Vec::with_capacity(decoder.bounded_capacity(partial_count, 3));
    let mut partial_types = BTreeSet::new();
    for _ in 0..partial_count {
        let concrete = decoder.read_bool()?;
        let type_id = decoder.read_var_i32()?;
        let predicate_type = if concrete {
            if crate::data_component_types::vanilla_data_component_predicate_type(type_id).is_none()
            {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid data component predicate type id {type_id}"
                )));
            }
            DataComponentPredicateTypeSummary::Concrete(type_id)
        } else {
            data_component_descriptor_or_error(type_id)?;
            DataComponentPredicateTypeSummary::AnyComponent(type_id)
        };
        if !partial_types.insert((concrete, type_id)) {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate partial data component predicate type id {type_id}"
            )));
        }
        let wire_value = decode_nbt_summary_from_decoder(decoder)?.ok_or_else(|| {
            ProtocolError::InvalidData(
                "partial data component predicate must use non-null nbt".to_string(),
            )
        })?;
        partial_components.push(PartialDataComponentPredicateSummary {
            predicate_type,
            wire_value: Some(wire_value),
        });
    }
    Ok((exact_components, partial_components))
}

pub(super) fn decode_attribute_modifiers(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<Vec<AttributeModifierSummary>> {
    let count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut modifiers = Vec::with_capacity(decoder.bounded_capacity(count, 13));
    for _ in 0..count {
        let attribute_id = entities::decode_attribute_holder_id(decoder)?;
        let modifier_id = read_resource_location(decoder)?;
        let amount_bits = decoder.read_f64()?.to_bits();
        let operation_id = normalize_attribute_modifier_operation_id(decoder.read_var_i32()?);
        let slot_id = decode_equipment_slot_group_id(decoder)?;
        let display = decode_attribute_modifier_display(decoder, registry)?;
        modifiers.push(AttributeModifierSummary {
            attribute_id,
            modifier_id,
            amount_bits,
            operation_id,
            slot_id,
            display_id: display.display_id,
            display_text: display.display_text,
            display_text_styled: display.display_text_styled,
            display_text_nbt: display.display_text_nbt,
        });
    }
    Ok(modifiers)
}

pub(super) fn decode_equipment_slot_group_id(decoder: &mut Decoder<'_>) -> Result<i32> {
    let id = decoder.read_var_i32()?;
    Ok(if vanilla_equipment_slot_group(id).is_some() {
        id
    } else {
        VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID
    })
}

pub(super) struct AttributeModifierDisplay {
    pub(super) display_id: i32,
    pub(super) display_text: Option<String>,
    pub(super) display_text_styled: Option<Vec<StyledTextRun>>,
    pub(super) display_text_nbt: Option<NbtSummaryValue>,
}

pub(super) fn decode_attribute_modifier_display(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<AttributeModifierDisplay> {
    let display = AttributeModifierDisplayType::from_id(decoder.read_var_i32()?);
    match display {
        AttributeModifierDisplayType::Override => {
            let (display_text_styled, display_text_nbt) =
                decode_styled_component_summary_with_nbt_from_decoder_with_registry(
                    decoder, registry,
                )?;
            let display_text = styled_runs_summary_text(&display_text_styled);
            Ok(AttributeModifierDisplay {
                display_id: display as i32,
                display_text: Some(display_text),
                display_text_styled: Some(display_text_styled),
                display_text_nbt,
            })
        }
        AttributeModifierDisplayType::Default | AttributeModifierDisplayType::Hidden => {
            Ok(AttributeModifierDisplay {
                display_id: display as i32,
                display_text: None,
                display_text_styled: None,
                display_text_nbt: None,
            })
        }
    }
}

pub(super) struct CustomModelData {
    pub(super) floats: Vec<f32>,
    pub(super) flags: Vec<bool>,
    pub(super) strings: Vec<crate::JavaString>,
    pub(super) colors: Vec<i32>,
}

pub(super) fn decode_custom_model_data(decoder: &mut Decoder<'_>) -> Result<CustomModelData> {
    let float_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut float_values = Vec::with_capacity(decoder.bounded_capacity(float_count, 4));
    for _ in 0..float_count {
        float_values.push(decoder.read_f32()?);
    }

    let flag_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut flag_values = Vec::with_capacity(decoder.bounded_capacity(flag_count, 1));
    for _ in 0..flag_count {
        flag_values.push(decoder.read_bool()?);
    }

    let strings = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut string_values = Vec::with_capacity(decoder.bounded_capacity(strings, 1));
    for _ in 0..strings {
        string_values.push(decoder.read_java_string(MAX_STRING_CHARS)?);
    }

    let colors = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut color_values = Vec::with_capacity(decoder.bounded_capacity(colors, 4));
    for _ in 0..colors {
        color_values.push(decoder.read_i32()?);
    }

    Ok(CustomModelData {
        floats: float_values,
        flags: flag_values,
        strings: string_values,
        colors: color_values,
    })
}

pub(super) fn decode_use_effects_summary(decoder: &mut Decoder<'_>) -> Result<UseEffectsSummary> {
    let summary = UseEffectsSummary {
        can_sprint: decoder.read_bool()?,
        interact_vibrations: decoder.read_bool()?,
        speed_multiplier: decoder.read_f32()?,
    };
    Ok(summary)
}

pub(super) fn decode_item_use_animation_summary(
    decoder: &mut Decoder<'_>,
) -> Result<ItemUseAnimationSummary> {
    Ok(ItemUseAnimationSummary::from_id(decoder.read_var_i32()?))
}

pub(super) fn decode_consume_effect_summary(
    decoder: &mut Decoder<'_>,
) -> Result<ConsumeEffectSummary> {
    let type_id = decoder.read_var_i32()?;
    let effect_type = ConsumeEffectType::from_id(type_id).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid consume effect type id {type_id}"))
    })?;
    match effect_type {
        ConsumeEffectType::ApplyEffects => {
            let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
            let mut effects = Vec::with_capacity(decoder.bounded_capacity(effect_count, 7));
            for _ in 0..effect_count {
                effects.push(decode_mob_effect_instance(decoder)?);
            }
            Ok(ConsumeEffectSummary::ApplyEffects {
                effects,
                probability_bits: decoder.read_f32()?.to_bits(),
            })
        }
        ConsumeEffectType::RemoveEffects => {
            let effects = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::MobEffect)?;
            Ok(ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids: effects.registry_ids,
                effect_tag: effects.tag,
            })
        }
        ConsumeEffectType::ClearAllEffects => Ok(ConsumeEffectSummary::ClearAllEffects),
        ConsumeEffectType::TeleportRandomly => Ok(ConsumeEffectSummary::TeleportRandomly {
            diameter_bits: decoder.read_f32()?.to_bits(),
        }),
        ConsumeEffectType::PlaySound => Ok(ConsumeEffectSummary::PlaySound {
            sound: decode_sound_event_holder_summary(decoder)?,
        }),
    }
}

pub(super) fn decode_consume_effect_list_summary(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<ConsumeEffectSummary>> {
    let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut effects = Vec::with_capacity(decoder.bounded_capacity(effect_count, 1));
    for _ in 0..effect_count {
        effects.push(decode_consume_effect_summary(decoder)?);
    }
    Ok(effects)
}

#[cfg(test)]
pub(super) fn decode_item_stack_template(
    decoder: &mut Decoder<'_>,
) -> Result<ItemStackTemplateSummary> {
    decode_item_stack_template_with_registry(decoder, ContentRegistry::vanilla())
}

pub(super) fn decode_item_stack_template_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<ItemStackTemplateSummary> {
    decoder.with_recursion_limit(
        MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH,
        "item stack template",
        |decoder| {
            #[cfg(test)]
            record_item_stack_template_stack_address();

            let item_id =
                inventory::decode_non_empty_item_holder_id_with_registry(decoder, registry)?;
            let count = decoder.read_var_i32()?;
            if count == 0 {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid item stack template count {count}"
                )));
            }
            let component_patch =
                decode_data_component_patch_summary_with_registry(decoder, registry)?.into();
            Ok(ItemStackTemplateSummary {
                item_id,
                count,
                component_patch,
            })
        },
    )
}

pub(super) fn validate_item_stack_template_forest_with_registry<'a>(
    roots: impl IntoIterator<Item = &'a ItemStackTemplateSummary>,
    registry: &ContentRegistry,
) -> Result<()> {
    let mut pending = roots
        .into_iter()
        .map(|template| (template, 1_usize))
        .collect::<Vec<_>>();
    while let Some((template, depth)) = pending.pop() {
        validate_item_stack_template_header_with_registry(template, depth, registry)?;

        let child_depth = depth + 1;
        let patch = &template.component_patch;
        if let Some(remainder) = patch.use_remainder.as_deref() {
            pending.push((remainder, child_depth));
        }
        pending.extend(
            patch
                .charged_projectiles_items
                .iter()
                .map(|item| (item, child_depth)),
        );
        pending.extend(
            patch
                .bundle_contents_items
                .iter()
                .map(|item| (item, child_depth)),
        );
        pending.extend(patch.container.items().map(|item| (item, child_depth)));
    }
    Ok(())
}

pub(super) fn validate_item_stack_template_header_with_registry(
    template: &ItemStackTemplateSummary,
    depth: usize,
    registry: &ContentRegistry,
) -> Result<()> {
    if depth > MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH {
        return Err(ProtocolError::InvalidData(format!(
            "item stack template nesting exceeds {MAX_ITEM_STACK_TEMPLATE_NESTING_DEPTH}"
        )));
    }
    inventory::validate_item_holder_id_with_registry(template.item_id, registry)?;
    if template.item_id == crate::item_types::VANILLA_ITEM_AIR_ID {
        return Err(ProtocolError::InvalidData(
            "item stack template holder must not be minecraft:air".to_string(),
        ));
    }
    if template.count == 0 {
        return Err(ProtocolError::InvalidData(
            "invalid item stack template count 0".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_item_stack_template_list_shape(
    templates: &[ItemStackTemplateSummary],
    max: usize,
    what: &str,
) -> Result<()> {
    if templates.len() > max {
        return Err(ProtocolError::PacketTooLarge(templates.len(), max));
    }
    if templates.len() > i32::MAX as usize {
        return Err(ProtocolError::InvalidData(format!(
            "{what} item count does not fit i32"
        )));
    }
    Ok(())
}

pub(super) fn validate_data_component_patch_recursive_templates_with_registry(
    patch: &DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let mut roots = Vec::new();
    if let Some(remainder) = patch.use_remainder.as_deref() {
        roots.push(remainder);
    }
    roots.extend(patch.charged_projectiles_items.iter());
    roots.extend(patch.bundle_contents_items.iter());
    roots.extend(patch.container.items());
    validate_item_stack_template_forest_with_registry(roots, registry)
}

pub(super) fn encode_item_stack_template_with_registry(
    out: &mut Encoder,
    template: &ItemStackTemplateSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    out.write_var_i32(template.item_id);
    out.write_var_i32(template.count);
    encode_data_component_patch_summary_with_registry(out, &template.component_patch, registry)
}

pub(super) fn encode_item_stack_template_list_with_registry(
    out: &mut Encoder,
    templates: &[ItemStackTemplateSummary],
    max: usize,
    what: &str,
    registry: &ContentRegistry,
) -> Result<()> {
    validate_item_stack_template_list_shape(templates, max, what)?;
    validate_item_stack_template_forest_with_registry(templates, registry)?;
    out.write_var_i32(templates.len() as i32);
    for template in templates {
        encode_item_stack_template_with_registry(out, template, registry)?;
    }
    Ok(())
}

pub(super) fn encode_item_container_contents_with_registry(
    out: &mut Encoder,
    patch: &DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let slot_count = validate_item_container_shape(patch)?;
    validate_item_stack_template_forest_with_registry(patch.container.items(), registry)?;

    out.write_var_i32(slot_count as i32);
    let mut slots = patch.container.slots.iter().peekable();
    for slot_index in 0..slot_count {
        if slots.peek().is_some_and(|slot| slot.slot == slot_index) {
            let slot = slots.next().expect("peeked container slot");
            out.write_bool(true);
            encode_item_stack_template_with_registry(out, &slot.item, registry)?;
        } else {
            out.write_bool(false);
        }
    }
    Ok(())
}

pub(super) fn validate_item_container_shape(patch: &DataComponentPatchSummary) -> Result<usize> {
    let slot_count = patch.container.slot_count.ok_or_else(|| {
        ProtocolError::InvalidData("container is missing exact slot count".to_string())
    })?;
    if slot_count > MAX_CONTAINER_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            slot_count,
            MAX_CONTAINER_ITEMS,
        ));
    }

    let mut previous_slot = None;
    for slot in &patch.container.slots {
        if slot.slot >= slot_count {
            return Err(ProtocolError::InvalidData(format!(
                "container slot {} is outside exact slot count {slot_count}",
                slot.slot
            )));
        }
        if previous_slot.is_some_and(|previous| previous >= slot.slot) {
            return Err(ProtocolError::InvalidData(
                "container slots must be strictly increasing".to_string(),
            ));
        }
        previous_slot = Some(slot.slot);
    }
    Ok(slot_count)
}

#[cfg(test)]
thread_local! {
    pub(super) static ITEM_STACK_TEMPLATE_STACK_EXTREMES: std::cell::Cell<(usize, usize)> =
        const { std::cell::Cell::new((0, usize::MAX)) };
}

#[cfg(test)]
pub(super) fn record_item_stack_template_stack_address() {
    let marker = 0u8;
    let address = std::ptr::from_ref(&marker) as usize;
    ITEM_STACK_TEMPLATE_STACK_EXTREMES.with(|cell| {
        let (top, bottom) = cell.get();
        cell.set((top.max(address), bottom.min(address)));
    })
}

pub(super) fn decode_item_stack_template_list(
    decoder: &mut Decoder<'_>,
    max: usize,
    registry: &ContentRegistry,
) -> Result<Vec<ItemStackTemplateSummary>> {
    let count = read_bounded_len(decoder, max)?;
    let mut items = Vec::with_capacity(decoder.bounded_capacity(count, 4));
    for _ in 0..count {
        items.push(decode_item_stack_template_with_registry(decoder, registry)?);
    }
    Ok(items)
}

pub(super) fn decode_optional_item_stack_template(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<Option<ItemStackTemplateSummary>> {
    if decoder.read_bool()? {
        Ok(Some(decode_item_stack_template_with_registry(
            decoder, registry,
        )?))
    } else {
        Ok(None)
    }
}

pub(super) fn decode_item_container_contents(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<super::ContainerContentsSummary> {
    let count = read_bounded_len(decoder, MAX_CONTAINER_ITEMS)?;
    let mut slots = Vec::new();
    for slot in 0..count {
        if let Some(item) = decode_optional_item_stack_template(decoder, registry)? {
            slots.push(ItemContainerSlotSummary { slot, item });
        }
    }
    Ok(super::ContainerContentsSummary {
        slots,
        slot_count: Some(count),
    })
}

pub(super) fn decode_tool(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<ToolSummary> {
    let rule_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut rules = Vec::with_capacity(decoder.bounded_capacity(rule_count, 3));
    for _ in 0..rule_count {
        let blocks = decode_holder_set_summary_with_registry(
            decoder,
            NetworkHolderSetRegistry::Block,
            registry,
        )?;
        rules.push(ToolRuleSummary {
            block_registry_ids: blocks.registry_ids,
            block_tag: blocks.tag,
            speed_bits: decode_optional_f32_bits(decoder)?,
            correct_for_drops: decode_optional_bool_value(decoder)?,
        });
    }
    Ok(ToolSummary {
        rules,
        default_mining_speed_bits: decoder.read_f32()?.to_bits(),
        damage_per_block: decoder.read_var_i32()?,
        can_destroy_blocks_in_creative: decoder.read_bool()?,
    })
}

pub(super) fn decode_attack_range_summary(decoder: &mut Decoder<'_>) -> Result<AttackRangeSummary> {
    Ok(AttackRangeSummary {
        min_reach: decoder.read_f32()?,
        max_reach: decoder.read_f32()?,
        min_creative_reach: decoder.read_f32()?,
        max_creative_reach: decoder.read_f32()?,
        hitbox_margin: decoder.read_f32()?,
        mob_factor: decoder.read_f32()?,
    })
}

pub(super) fn decode_blocks_attacks(decoder: &mut Decoder<'_>) -> Result<BlocksAttacksSummary> {
    let block_delay_seconds_bits = decoder.read_f32()?.to_bits();
    let disable_cooldown_scale_bits = decoder.read_f32()?.to_bits();

    let reduction_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut damage_reductions = Vec::with_capacity(decoder.bounded_capacity(reduction_count, 13));
    for _ in 0..reduction_count {
        damage_reductions.push(decode_damage_reduction(decoder)?);
    }

    Ok(BlocksAttacksSummary {
        block_delay_seconds_bits,
        disable_cooldown_scale_bits,
        damage_reductions,
        item_damage: decode_item_damage_function(decoder)?,
        bypassed_by: decode_optional_damage_type_holder_set(decoder)?,
        block_sound: decode_optional_sound_event_holder_summary(decoder)?,
        disable_sound: decode_optional_sound_event_holder_summary(decoder)?,
    })
}

pub(super) fn decode_damage_reduction(decoder: &mut Decoder<'_>) -> Result<DamageReductionSummary> {
    Ok(DamageReductionSummary {
        horizontal_blocking_angle_bits: decoder.read_f32()?.to_bits(),
        damage_types: decode_optional_damage_type_holder_set(decoder)?,
        base_bits: decoder.read_f32()?.to_bits(),
        factor_bits: decoder.read_f32()?.to_bits(),
    })
}

pub(super) fn decode_item_damage_function(
    decoder: &mut Decoder<'_>,
) -> Result<ItemDamageFunctionSummary> {
    Ok(ItemDamageFunctionSummary {
        threshold_bits: decoder.read_f32()?.to_bits(),
        base_bits: decoder.read_f32()?.to_bits(),
        factor_bits: decoder.read_f32()?.to_bits(),
    })
}

pub(super) fn decode_optional_damage_type_holder_set(
    decoder: &mut Decoder<'_>,
) -> Result<Option<HolderSetSummary>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    decode_holder_set_summary(decoder, NetworkHolderSetRegistry::DamageType).map(Some)
}

pub(super) fn decode_piercing_weapon_summary(
    decoder: &mut Decoder<'_>,
) -> Result<PiercingWeaponSummary> {
    Ok(PiercingWeaponSummary {
        deals_knockback: decoder.read_bool()?,
        dismounts: decoder.read_bool()?,
        sound: decode_optional_sound_event_holder_summary(decoder)?,
        hit_sound: decode_optional_sound_event_holder_summary(decoder)?,
    })
}

pub(super) fn decode_kinetic_weapon_summary(
    decoder: &mut Decoder<'_>,
) -> Result<KineticWeaponSummary> {
    Ok(KineticWeaponSummary {
        contact_cooldown_ticks: decoder.read_var_i32()?,
        delay_ticks: decoder.read_var_i32()?,
        dismount_conditions: decode_optional_kinetic_weapon_condition(decoder)?,
        knockback_conditions: decode_optional_kinetic_weapon_condition(decoder)?,
        damage_conditions: decode_optional_kinetic_weapon_condition(decoder)?,
        forward_movement_bits: decoder.read_f32()?.to_bits(),
        damage_multiplier_bits: decoder.read_f32()?.to_bits(),
        sound: decode_optional_sound_event_holder_summary(decoder)?,
        hit_sound: decode_optional_sound_event_holder_summary(decoder)?,
    })
}

pub(super) fn decode_optional_kinetic_weapon_condition(
    decoder: &mut Decoder<'_>,
) -> Result<Option<KineticWeaponConditionSummary>> {
    if decoder.read_bool()? {
        return Ok(Some(KineticWeaponConditionSummary {
            max_duration_ticks: decoder.read_var_i32()?,
            min_speed_bits: decoder.read_f32()?.to_bits(),
            min_relative_speed_bits: decoder.read_f32()?.to_bits(),
        }));
    }
    Ok(None)
}

pub(super) fn decode_equippable(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<EquippableSummary> {
    let slot_id = decoder.read_var_i32()?;
    let equip_sound = decode_sound_event_holder_summary(decoder)?;
    let asset_id = decode_optional_identifier_value(decoder)?;
    let camera_overlay = decode_optional_identifier_value(decoder)?;
    let allowed_entities = if decoder.read_bool()? {
        Some(decode_holder_set_summary_with_registry(
            decoder,
            NetworkHolderSetRegistry::EntityType,
            registry,
        )?)
    } else {
        None
    };
    Ok(EquippableSummary {
        slot_id: if vanilla_equipment_slot_by_id(slot_id).is_some() {
            slot_id
        } else {
            VANILLA_EQUIPMENT_SLOT_MAINHAND_ID
        },
        equip_sound,
        asset_id,
        camera_overlay,
        allowed_entities,
        dispensable: decoder.read_bool()?,
        swappable: decoder.read_bool()?,
        damage_on_hurt: decoder.read_bool()?,
        equip_on_interact: decoder.read_bool()?,
        can_be_sheared: decoder.read_bool()?,
        shearing_sound: decode_sound_event_holder_summary(decoder)?,
    })
}

pub(super) struct TypedEntityDataSummary {
    pub(super) type_id: i32,
    pub(super) tag: NbtSummaryValue,
}

pub(super) fn decode_typed_entity_data_summary(
    decoder: &mut Decoder<'_>,
    registry_kind: catalog::TypedEntityDataRegistry,
    content_registry: &ContentRegistry,
) -> Result<TypedEntityDataSummary> {
    let type_id = decode_holder_registry_id(decoder)?;
    registry_kind.validate_with_registry(type_id, content_registry)?;
    let mut tag = catalog::decode_required_compound_tag(decoder, registry_kind.component_name())?;
    let NbtSummaryValue::Compound(entries) = &mut tag else {
        unreachable!("required compound decoder returned another NBT type");
    };
    entries.retain(|entry| entry.name != "id");
    Ok(TypedEntityDataSummary { type_id, tag })
}

/// The `SpawnData.entity.id` a block-entity data tag carries, which a
/// projection rebuilding that tag has to answer the same way the wire decoder
/// does rather than leaving the spawn-entity field stale.
pub fn typed_entity_data_spawn_entity_type(tag: Option<&NbtSummaryValue>) -> Option<String> {
    let spawn_data = nbt_compound_get(tag?, "SpawnData")?;
    let entity = nbt_compound_get(spawn_data, "entity")?;
    let NbtSummaryValue::String(entity_id) = nbt_compound_get(entity, "id")? else {
        return None;
    };
    normalize_resource_location(entity_id.try_to_string().ok()?).ok()
}

pub(super) fn nbt_compound_get<'a>(
    value: &'a NbtSummaryValue,
    key: &str,
) -> Option<&'a NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
}

pub(super) fn decode_direct_instrument_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<InstrumentSummary> {
    let sound_event = decode_sound_event_holder_summary(decoder)?;
    let use_duration_bits = decoder.read_f32()?.to_bits();
    let range_bits = decoder.read_f32()?.to_bits();
    let (description_styled, description_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
    let description = styled_runs_summary_text(&description_styled);
    Ok(InstrumentSummary {
        sound_event,
        use_duration_bits,
        range_bits,
        description,
        description_styled: Some(description_styled.into_boxed_slice()),
        description_nbt: description_nbt.map(Box::new),
    })
}

pub(super) fn decode_direct_trim_material_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<TrimMaterialSummary> {
    let (asset_name, override_armor_assets) = decode_material_asset_group_summary(decoder)?;
    let (description_styled, description_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
    let description = styled_runs_summary_text(&description_styled);
    Ok(TrimMaterialSummary {
        asset_name,
        override_armor_assets,
        description,
        description_styled: Some(description_styled.into_boxed_slice()),
        description_nbt: description_nbt.map(Box::new),
    })
}

pub(super) fn decode_material_asset_group_summary(
    decoder: &mut Decoder<'_>,
) -> Result<(String, BTreeMap<String, String>)> {
    let asset_name = decode_trim_material_asset_suffix(decoder, "asset_name")?;
    let overrides = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut override_armor_assets = BTreeMap::new();
    for _ in 0..overrides {
        let equipment_asset = read_resource_location(decoder)?;
        let suffix = decode_trim_material_asset_suffix(decoder, "override_armor_assets suffix")?;
        override_armor_assets.insert(equipment_asset, suffix);
    }
    Ok((asset_name, override_armor_assets))
}

pub(super) fn decode_trim_material_asset_suffix(
    decoder: &mut Decoder<'_>,
    field: &str,
) -> Result<String> {
    let suffix = decoder.read_string(MAX_STRING_CHARS)?;
    if !is_valid_resource_path(&suffix) {
        return Err(ProtocolError::InvalidData(format!(
            "trim material {field} `{suffix}` is not a valid resource path"
        )));
    }
    Ok(suffix)
}

pub(crate) fn decode_trim_pattern_holder(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<(Option<i32>, Option<TrimPatternSummary>)> {
    decode_registry_or_inline(decoder, |decoder| {
        decode_direct_trim_pattern_summary(decoder, registry)
    })
}

pub(super) fn decode_direct_trim_pattern_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<TrimPatternSummary> {
    let asset_id = read_resource_location(decoder)?;
    let (description_styled, description_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
    let description = styled_runs_summary_text(&description_styled);
    let decal = decoder.read_bool()?;
    Ok(TrimPatternSummary {
        asset_id,
        description,
        description_styled: Some(description_styled.into_boxed_slice()),
        description_nbt: description_nbt.map(Box::new),
        decal,
    })
}

pub(super) fn decode_lodestone_tracker(
    decoder: &mut Decoder<'_>,
) -> Result<(Option<LodestoneTargetSummary>, bool)> {
    let target = decode_optional_global_pos(decoder)?;
    let tracked = decoder.read_bool()?;
    Ok((target, tracked))
}

pub(super) fn decode_resolvable_profile(
    decoder: &mut Decoder<'_>,
) -> Result<ResolvableProfileSummary> {
    let (kind, uuid, name) = if decoder.read_bool()? {
        let uuid = decoder.read_uuid()?;
        let name = decode_java_string(decoder, MAX_PLAYER_NAME_CHARS)?;
        (
            ResolvableProfileKindSummary::GameProfile,
            Some(uuid),
            Some(name),
        )
    } else {
        let name = decode_optional_java_string(decoder, MAX_PLAYER_NAME_CHARS)?;
        let uuid = if decoder.read_bool()? {
            Some(decoder.read_uuid()?)
        } else {
            None
        };
        (ResolvableProfileKindSummary::Partial, uuid, name)
    };
    let properties = decode_game_profile_properties(decoder)?;
    let profile_textures = decode_profile_textures_from_properties(
        properties
            .iter()
            .map(|property| (&property.name, &property.value)),
    );
    let skin_patch = decode_player_skin_patch(decoder)?;
    Ok(ResolvableProfileSummary {
        kind,
        uuid,
        name,
        properties,
        profile_textures,
        skin_patch,
    })
}

pub(super) fn decode_game_profile_properties(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<GameProfilePropertySummary>> {
    let property_count = read_bounded_len(decoder, MAX_PROFILE_PROPERTIES)?;
    let mut properties = Vec::with_capacity(decoder.bounded_capacity(property_count, 3));
    for _ in 0..property_count {
        properties.push(GameProfilePropertySummary {
            name: decoder.read_string(MAX_PROFILE_PROPERTY_NAME_CHARS)?.into(),
            value: decoder.read_string(MAX_STRING_CHARS)?.into(),
            signature: if decoder.read_bool()? {
                Some(decoder.read_string(MAX_PROFILE_SIGNATURE_CHARS)?.into())
            } else {
                None
            },
        });
    }
    Ok(properties)
}

pub(super) fn decode_java_string(
    decoder: &mut Decoder<'_>,
    max_utf16_units: usize,
) -> Result<String> {
    let value = decoder.read_string(max_utf16_units)?;
    validate_java_string_len(&value, max_utf16_units)?;
    Ok(value)
}

pub(super) fn decode_optional_java_string(
    decoder: &mut Decoder<'_>,
    max_utf16_units: usize,
) -> Result<Option<String>> {
    if decoder.read_bool()? {
        return decode_java_string(decoder, max_utf16_units).map(Some);
    }
    Ok(None)
}

pub(super) fn validate_java_string_len(value: &str, max_utf16_units: usize) -> Result<()> {
    let actual = value.encode_utf16().count();
    if actual > max_utf16_units {
        return Err(ProtocolError::StringTooLong {
            actual,
            max: max_utf16_units,
        });
    }
    Ok(())
}

pub(super) fn decode_player_skin_patch(
    decoder: &mut Decoder<'_>,
) -> Result<PlayerSkinPatchSummary> {
    Ok(PlayerSkinPatchSummary {
        body: decode_optional_resource_texture(decoder)?,
        cape: decode_optional_resource_texture(decoder)?,
        elytra: decode_optional_resource_texture(decoder)?,
        model: decode_optional_player_model_type(decoder)?,
    })
}

pub(super) fn decode_optional_resource_texture(
    decoder: &mut Decoder<'_>,
) -> Result<Option<ResourceTextureSummary>> {
    if decoder.read_bool()? {
        let asset_id = read_resource_location(decoder)?;
        return Ok(Some(ResourceTextureSummary {
            texture_path: resource_texture_path(&asset_id),
            asset_id,
        }));
    }
    Ok(None)
}

pub(super) fn resource_texture_path(asset_id: &str) -> String {
    let (namespace, path) = asset_id
        .split_once(':')
        .expect("resource locations decoded by read_resource_location include a namespace");
    format!("{namespace}:textures/{path}.png")
}

pub(super) fn canonical_resolvable_profile_summary(
    profile: &ResolvableProfileSummary,
) -> Result<ResolvableProfileSummary> {
    let mut canonical = profile.clone();
    canonical.profile_textures = decode_profile_textures_from_properties(
        canonical
            .properties
            .iter()
            .map(|property| (&property.name, &property.value)),
    );
    for texture in [
        canonical.skin_patch.body.as_mut(),
        canonical.skin_patch.cape.as_mut(),
        canonical.skin_patch.elytra.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        texture.asset_id = normalize_resource_location(texture.asset_id.clone())?;
        texture.texture_path = resource_texture_path(&texture.asset_id);
    }
    Ok(canonical)
}

pub(super) fn decode_optional_player_model_type(
    decoder: &mut Decoder<'_>,
) -> Result<Option<PlayerModelTypeSummary>> {
    if decoder.read_bool()? {
        return Ok(Some(if decoder.read_bool()? {
            PlayerModelTypeSummary::Slim
        } else {
            PlayerModelTypeSummary::Wide
        }));
    }
    Ok(None)
}

pub(super) fn decode_direct_jukebox_song_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<JukeboxSongSummary> {
    let sound_event = decode_sound_event_holder_summary(decoder)?;
    let (description_styled, description_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
    let description = styled_runs_summary_text(&description_styled);
    let length_in_seconds_bits = decoder.read_f32()?.to_bits();
    let comparator_output = decoder.read_var_i32()?;
    Ok(JukeboxSongSummary {
        sound_event,
        description,
        description_styled: Some(description_styled.into_boxed_slice()),
        description_nbt: description_nbt.map(Box::new),
        length_in_seconds_bits,
        comparator_output,
    })
}

pub(super) fn decode_sound_event_holder_summary(
    decoder: &mut Decoder<'_>,
) -> Result<SoundEventSummary> {
    let id = decoder.read_var_i32()?;
    if id < 0 {
        return Err(ProtocolError::NegativeLength(id));
    }
    if id == 0 {
        decode_direct_sound_event_summary(decoder)
    } else {
        Ok(SoundEventSummary {
            registry_id: Some(id - 1),
            sound_id: None,
            fixed_range_bits: None,
        })
    }
}

pub(super) fn decode_direct_sound_event_summary(
    decoder: &mut Decoder<'_>,
) -> Result<SoundEventSummary> {
    let sound_id = read_resource_location(decoder)?;
    let fixed_range_bits = if decoder.read_bool()? {
        Some(decoder.read_f32()?.to_bits())
    } else {
        None
    };
    Ok(SoundEventSummary {
        registry_id: None,
        sound_id: Some(sound_id),
        fixed_range_bits,
    })
}

pub(super) fn decode_banner_pattern_layers_summary(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<BannerPatternLayerSummary>> {
    let layer_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut layers = Vec::with_capacity(decoder.bounded_capacity(layer_count, 2));
    for _ in 0..layer_count {
        let holder_id = decoder.read_var_i32()?;
        if holder_id < 0 {
            return Err(ProtocolError::NegativeLength(holder_id));
        }
        let (registry_id, asset_id, translation_key) = if holder_id == 0 {
            let asset_id = read_resource_location(decoder)?;
            let translation_key = decoder.read_string(MAX_STRING_CHARS)?;
            (None, Some(asset_id), Some(translation_key))
        } else {
            (Some(holder_id - 1), None, None)
        };
        let color_id = decoder.read_var_i32()?;
        layers.push(BannerPatternLayerSummary {
            registry_id,
            asset_id,
            translation_key: translation_key.map(Into::into),
            color_id: normalize_vanilla_dye_color_id(color_id),
        });
    }
    Ok(layers)
}

pub(super) fn decode_pot_decorations_item_ids(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<Vec<i32>> {
    let item_count = read_bounded_len(decoder, MAX_POT_DECORATIONS)?;
    let mut item_ids = Vec::with_capacity(MAX_POT_DECORATIONS);
    for _ in 0..item_count {
        let item_id = decoder.read_var_i32()?;
        if vanilla_item_type(item_id).is_none() && registry.item_by_id(item_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid pot decoration item registry id {item_id}"
            )));
        }
        item_ids.push(item_id);
    }
    while item_ids.len() < MAX_POT_DECORATIONS {
        item_ids.push(VANILLA_ITEM_BRICK_ID);
    }
    Ok(item_ids)
}

pub(super) fn decode_bees(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<Vec<BeeOccupantSummary>> {
    let bee_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut bees = Vec::with_capacity(decoder.bounded_capacity(bee_count, 4));
    for _ in 0..bee_count {
        let entity_data = decode_typed_entity_data_summary(
            decoder,
            catalog::TypedEntityDataRegistry::EntityType,
            registry,
        )?;
        bees.push(BeeOccupantSummary {
            entity_type_id: entity_data.type_id,
            entity_data_tag: Some(entity_data.tag),
            ticks_in_hive: decoder.read_var_i32()?,
            min_ticks_in_hive: decoder.read_var_i32()?,
        });
    }
    Ok(bees)
}

pub(super) fn decode_direct_painting_variant_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<PaintingVariantSummary> {
    let width = decoder.read_var_i32()?;
    let height = decoder.read_var_i32()?;
    let asset_id = read_resource_location(decoder)?;
    let title =
        decode_optional_component_summary_with_nbt(decoder, "painting variant title", registry)?;
    let author =
        decode_optional_component_summary_with_nbt(decoder, "painting variant author", registry)?;
    Ok(PaintingVariantSummary {
        width,
        height,
        asset_id,
        title: title
            .styled
            .as_ref()
            .map(|runs| styled_runs_summary_text(runs)),
        title_styled: title.styled,
        title_nbt: title.nbt,
        author: author
            .styled
            .as_ref()
            .map(|runs| styled_runs_summary_text(runs)),
        author_styled: author.styled,
        author_nbt: author.nbt,
    })
}

pub(super) fn decode_swing_animation(decoder: &mut Decoder<'_>) -> Result<SwingAnimationSummary> {
    let animation_type = SwingAnimationTypeSummary::from_id(decoder.read_var_i32()?);
    Ok(SwingAnimationSummary {
        animation_type,
        duration: decoder.read_var_i32()?,
    })
}

pub(super) fn decode_suspicious_stew_effects(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<SuspiciousStewEffectSummary>> {
    let effects = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut summaries = Vec::with_capacity(decoder.bounded_capacity(effects, 2));
    for _ in 0..effects {
        summaries.push(SuspiciousStewEffectSummary {
            effect_id: client_state::decode_mob_effect_holder_id(decoder)?,
            duration: decoder.read_var_i32()?,
        });
    }
    Ok(summaries)
}

pub(super) fn decode_mob_effect_instance(
    decoder: &mut Decoder<'_>,
) -> Result<MobEffectInstanceSummary> {
    let effect_id = client_state::decode_mob_effect_holder_id(decoder)?;
    let details = decode_mob_effect_details(decoder, 0)?;
    Ok(MobEffectInstanceSummary {
        effect_id,
        amplifier: details.amplifier,
        duration: details.duration,
        ambient: details.ambient,
        show_particles: details.show_particles,
        show_icon: details.show_icon,
        hidden_effect: details.hidden_effect,
    })
}

pub(super) fn decode_mob_effect_details(
    decoder: &mut Decoder<'_>,
    depth: usize,
) -> Result<MobEffectDetailsSummary> {
    if depth > MAX_MOB_EFFECT_DETAILS_DEPTH {
        return Err(ProtocolError::InvalidData(
            "mob effect details exceeded max depth".to_string(),
        ));
    }
    let amplifier = decoder.read_var_i32()?.clamp(0, 255);
    let duration = decoder.read_var_i32()?;
    let ambient = decoder.read_bool()?;
    let show_particles = decoder.read_bool()?;
    let show_icon = decoder.read_bool()?;
    let hidden_effect = if decoder.read_bool()? {
        Some(Box::new(decode_mob_effect_details(decoder, depth + 1)?))
    } else {
        None
    };
    Ok(MobEffectDetailsSummary {
        amplifier,
        duration,
        ambient,
        show_particles,
        show_icon,
        hidden_effect,
    })
}

pub(super) struct FilterableComponentSummary {
    pub(super) raw: String,
    pub(super) raw_nbt: Option<NbtSummaryValue>,
    pub(super) filtered: Option<String>,
    pub(super) filtered_nbt: Option<NbtSummaryValue>,
}

pub(super) fn decode_filterable_component_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<FilterableComponentSummary> {
    let (raw_runs, raw_nbt) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
    let raw = styled_runs_summary_text(&raw_runs);
    let (filtered, filtered_nbt) = if decoder.read_bool()? {
        let (filtered_runs, filtered_nbt) =
            decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
        (Some(styled_runs_summary_text(&filtered_runs)), filtered_nbt)
    } else {
        (None, None)
    };
    Ok(FilterableComponentSummary {
        raw,
        raw_nbt,
        filtered,
        filtered_nbt,
    })
}

pub(super) struct OptionalComponentSummary {
    pub(super) styled: Option<Vec<StyledTextRun>>,
    pub(super) nbt: Option<Box<NbtSummaryValue>>,
}

pub(super) fn decode_optional_component_summary_with_nbt(
    decoder: &mut Decoder<'_>,
    what: &str,
    registry: &ContentRegistry,
) -> Result<OptionalComponentSummary> {
    if !decoder.read_bool()? {
        return Ok(OptionalComponentSummary {
            styled: None,
            nbt: None,
        });
    }
    let (styled, nbt) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
    let nbt = nbt.ok_or_else(|| {
        ProtocolError::InvalidData(format!("{what} is missing its component NBT root"))
    })?;
    Ok(OptionalComponentSummary {
        styled: Some(styled),
        nbt: Some(Box::new(nbt)),
    })
}

pub(super) fn decode_firework_explosion(
    decoder: &mut Decoder<'_>,
) -> Result<FireworkExplosionSummary> {
    let shape = FireworkExplosionShapeSummary::from_id(decoder.read_var_i32()?);
    let colors = decode_int_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let fade_colors = decode_int_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let has_trail = decoder.read_bool()?;
    let has_twinkle = decoder.read_bool()?;
    Ok(FireworkExplosionSummary {
        shape,
        colors,
        fade_colors,
        has_trail,
        has_twinkle,
    })
}

pub(super) fn decode_int_list(decoder: &mut Decoder<'_>, max: usize) -> Result<Vec<i32>> {
    let count = read_bounded_len(decoder, max)?;
    let mut values = Vec::with_capacity(decoder.bounded_capacity(count, 4));
    for _ in 0..count {
        values.push(decoder.read_i32()?);
    }
    Ok(values)
}

pub(super) fn decode_optional_string_value(
    decoder: &mut Decoder<'_>,
    max_chars: usize,
) -> Result<Option<String>> {
    if decoder.read_bool()? {
        return decoder.read_string(max_chars).map(Some);
    }
    Ok(None)
}

pub(super) fn decode_tooltip_display_summary(
    decoder: &mut Decoder<'_>,
) -> Result<(bool, Vec<i32>)> {
    let hide_tooltip = decoder.read_bool()?;
    let hidden_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut hidden_components = Vec::with_capacity(decoder.bounded_capacity(hidden_count, 1));
    let mut seen = BTreeSet::new();
    for _ in 0..hidden_count {
        let type_id = decoder.read_var_i32()?;
        data_component_descriptor_or_error(type_id)?;
        if seen.insert(type_id) {
            hidden_components.push(type_id);
        }
    }
    Ok((hide_tooltip, hidden_components))
}
