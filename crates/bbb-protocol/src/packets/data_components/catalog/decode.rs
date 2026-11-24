use super::*;

#[derive(Clone, Copy)]
pub(crate) enum TypedEntityDataRegistry {
    EntityType,
    BlockEntityType,
}

impl TypedEntityDataRegistry {
    pub(crate) const fn component_name(self) -> &'static str {
        match self {
            Self::EntityType => "entity_data",
            Self::BlockEntityType => "block_entity_data",
        }
    }

    pub(crate) fn validate_with_registry(
        self,
        type_id: i32,
        content_registry: &ContentRegistry,
    ) -> Result<()> {
        let (valid, registry) = match self {
            Self::EntityType => (
                vanilla_entity_type(type_id).is_some()
                    || content_registry.entity_by_id(type_id).is_some(),
                "entity type",
            ),
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
        pub(super) fn $name(
            _type_id: i32,
            decoder: &mut Decoder<'_>,
            summary: &mut DataComponentPatchSummary,
            _registry: &ContentRegistry,
        ) -> Result<()> {
            summary.$field = Some(decoder.read_var_i32()?);
            Ok(())
        }
    };
}

decode_var_i32_field!(decode_max_stack_size, max_stack_size);
decode_var_i32_field!(decode_max_damage, max_damage);
decode_var_i32_field!(decode_damage, damage);
decode_var_i32_field!(decode_additional_trade_cost, additional_trade_cost);
decode_var_i32_field!(decode_map_id, map_id);
decode_var_i32_field!(decode_ominous_bottle_amplifier, ominous_bottle_amplifier);

pub(super) fn decode_repair_cost(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let value = decoder.read_var_i32()?;
    if value < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "repair cost must be nonnegative, got {value}"
        )));
    }
    summary.repair_cost = Some(value);
    Ok(())
}

pub(super) fn decode_enchantable(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let value = decoder.read_var_i32()?;
    if value <= 0 {
        return Err(ProtocolError::InvalidData(format!(
            "enchantable value must be positive, got {value}"
        )));
    }
    summary.enchantable_value = Some(value);
    Ok(())
}

macro_rules! encode_var_i32_field {
    ($name:ident, $field:ident) => {
        pub(super) fn $name(
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
encode_var_i32_field!(encode_additional_trade_cost, additional_trade_cost);

pub(super) fn encode_repair_cost(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let value = summary
        .repair_cost
        .ok_or_else(|| ProtocolError::InvalidData("missing repair cost value".to_string()))?;
    if value < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "repair cost must be nonnegative, got {value}"
        )));
    }
    out.write_var_i32(value);
    Ok(())
}

pub(super) fn encode_enchantable(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let value = summary.enchantable_value.ok_or_else(|| {
        ProtocolError::InvalidData("missing enchantable component value".to_string())
    })?;
    if value <= 0 {
        return Err(ProtocolError::InvalidData(format!(
            "enchantable value must be positive, got {value}"
        )));
    }
    out.write_var_i32(value);
    Ok(())
}

pub(super) fn decode_unbreakable(
    _type_id: i32,
    _decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.unbreakable = true;
    Ok(())
}

pub(super) fn encode_unbreakable(
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

pub(super) fn decode_use_effects_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.use_effects = Some(decode_use_effects_summary(decoder)?);
    Ok(())
}

pub(super) fn decode_styled_name(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<(String, Vec<StyledTextRun>, Option<NbtSummaryValue>)> {
    let (runs, nbt) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
    Ok((styled_runs_summary_text(&runs), runs, nbt))
}

pub(super) fn decode_custom_name(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let (text, runs, nbt) = decode_styled_name(decoder, registry)?;
    summary.custom_name = Some(text);
    summary.custom_name_styled = Some(runs);
    summary.custom_name_nbt = nbt;
    Ok(())
}

pub(super) fn decode_item_name(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let (text, runs, nbt) = decode_styled_name(decoder, registry)?;
    summary.item_name = Some(text);
    summary.item_name_styled = Some(runs);
    summary.item_name_nbt = nbt;
    Ok(())
}

pub(super) fn decode_minimum_attack_charge(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.minimum_attack_charge_bits = Some(decoder.read_f32()?.to_bits());
    Ok(())
}

pub(super) fn decode_item_model(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.item_model = Some(read_resource_location(decoder)?);
    Ok(())
}

pub(super) fn decode_lore_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let line_count = read_bounded_len(decoder, MAX_LORE_LINES)?;
    let capacity = decoder.bounded_capacity(line_count, 1);
    summary.lore = Vec::with_capacity(capacity);
    summary.lore_styled = Vec::with_capacity(capacity);
    summary.lore_nbt = Vec::with_capacity(capacity);
    for _ in 0..line_count {
        let (runs, nbt) =
            decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
        summary.lore.push(styled_runs_summary_text(&runs));
        summary.lore_styled.push(runs);
        if let Some(nbt) = nbt {
            summary.lore_nbt.push(nbt);
        }
    }
    Ok(())
}

pub(super) fn decode_rarity(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.rarity = Some(decode_item_rarity(decoder)?);
    Ok(())
}

pub(super) fn decode_enchantments(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.enchantments = decode_varint_map(decoder)?;
    Ok(())
}

pub(super) fn decode_can_place_on(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    summary.can_place_on_persistent =
        Some(decode_adventure_mode_predicate_summary(decoder, registry)?);
    Ok(())
}

pub(super) fn decode_can_break(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    summary.can_break_persistent =
        Some(decode_adventure_mode_predicate_summary(decoder, registry)?);
    Ok(())
}

pub(super) fn decode_attribute_modifiers_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    summary.attribute_modifiers = decode_attribute_modifiers(decoder, registry)?;
    Ok(())
}

pub(super) fn decode_custom_model_data_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let custom_model_data = decode_custom_model_data(decoder)?;
    summary.custom_model_data_floats = custom_model_data.floats.into();
    summary.custom_model_data_flags = custom_model_data.flags;
    summary.custom_model_data_strings = custom_model_data.strings;
    summary.custom_model_data_colors = custom_model_data.colors;
    Ok(())
}

pub(super) fn decode_tooltip_display_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let (hide_tooltip, hidden_components) = decode_tooltip_display_summary(decoder)?;
    summary.tooltip_hide_tooltip = hide_tooltip;
    summary.tooltip_hidden_component_type_ids = hidden_components;
    Ok(())
}

pub(super) fn decode_enchantment_glint_override(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.enchantment_glint_override = Some(decoder.read_bool()?);
    Ok(())
}

pub(super) fn decode_intangible_projectile(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.intangible_projectile = true;
    skip_nbt_tag_from_decoder(decoder)
}

pub(super) fn decode_food_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.food_nutrition = Some(decoder.read_var_i32()?);
    summary.food_saturation_bits = Some(decoder.read_f32()?.to_bits());
    summary.food_can_always_eat = decoder.read_bool()?;
    Ok(())
}

pub(super) fn encode_food(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let nutrition = summary.food_nutrition.ok_or_else(|| {
        ProtocolError::InvalidData("missing food component nutrition".to_string())
    })?;
    let saturation_bits = summary.food_saturation_bits.ok_or_else(|| {
        ProtocolError::InvalidData("missing food component saturation".to_string())
    })?;
    out.write_var_i32(nutrition);
    out.write_f32(f32::from_bits(saturation_bits));
    out.write_bool(summary.food_can_always_eat);
    Ok(())
}

pub(super) fn decode_consumable_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let consume_seconds = decoder.read_f32()?;
    let animation = decode_item_use_animation_summary(decoder)?;
    let sound = decode_sound_event_holder_summary(decoder)?;
    let has_consume_particles = decoder.read_bool()?;
    let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut effects = Vec::with_capacity(decoder.bounded_capacity(effect_count, 1));
    for _ in 0..effect_count {
        effects.push(decode_consume_effect_summary(decoder)?);
    }
    summary.consumable = Some(ConsumableSummary {
        consume_seconds,
        animation,
    });
    summary.consumable_sound = Some(sound);
    summary.consumable_has_consume_particles = Some(has_consume_particles);
    summary.consumable_effects = effects;
    Ok(())
}

pub(super) fn decode_use_remainder_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    summary.use_remainder = Some(Box::new(decode_item_stack_template_with_registry(
        decoder, registry,
    )?));
    Ok(())
}

pub(super) fn decode_use_cooldown_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let seconds = decoder.read_f32()?;
    let cooldown_group = decode_optional_identifier_value(decoder)?;
    summary.use_cooldown_seconds_bits = Some(seconds.to_bits());
    summary.use_cooldown_group = cooldown_group;
    Ok(())
}

pub(super) fn decode_damage_resistant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let types = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::DamageType)?;
    summary.damage_resistant_type_registry_ids = types.registry_ids;
    summary.damage_resistant_type_tag = types.tag;
    Ok(())
}

pub(super) fn encode_damage_resistant(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_holder_set_summary(
        out,
        &summary.damage_resistant_type_registry_ids,
        summary.damage_resistant_type_tag.as_deref(),
        NetworkHolderSetRegistry::DamageType,
        "damage_resistant types",
    )?;
    Ok(())
}

pub(super) fn decode_weapon_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.weapon_item_damage_per_attack = Some(decoder.read_var_i32()?);
    summary.weapon_disable_blocking_seconds_bits = Some(decoder.read_f32()?.to_bits());
    Ok(())
}

pub(super) fn encode_weapon(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let item_damage_per_attack = summary.weapon_item_damage_per_attack.ok_or_else(|| {
        ProtocolError::InvalidData("missing weapon item_damage_per_attack".to_string())
    })?;
    if item_damage_per_attack < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "weapon item_damage_per_attack {item_damage_per_attack} is outside persistent range 0.."
        )));
    }
    let disable_blocking_seconds = f32::from_bits(
        summary
            .weapon_disable_blocking_seconds_bits
            .ok_or_else(|| {
                ProtocolError::InvalidData(
                    "missing weapon disable_blocking_for_seconds".to_string(),
                )
            })?,
    );
    if !disable_blocking_seconds.is_finite() || disable_blocking_seconds.total_cmp(&0.0).is_lt() {
        return Err(ProtocolError::InvalidData(format!(
            "weapon disable_blocking_for_seconds {disable_blocking_seconds} is outside persistent range 0.."
        )));
    }

    out.write_var_i32(item_damage_per_attack);
    out.write_f32(disable_blocking_seconds);
    Ok(())
}

pub(super) fn decode_attack_range_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.attack_range = Some(decode_attack_range_summary(decoder)?);
    Ok(())
}

pub(super) fn decode_repairable(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let items =
        decode_holder_set_summary_with_registry(decoder, NetworkHolderSetRegistry::Item, registry)?;
    summary.repairable_item_registry_ids = items.registry_ids;
    summary.repairable_item_tag = items.tag;
    Ok(())
}

pub(super) fn encode_repairable(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    encode_holder_set_summary_with_registry(
        out,
        &summary.repairable_item_registry_ids,
        summary.repairable_item_tag.as_deref(),
        NetworkHolderSetRegistry::Item,
        "repairable items",
        registry,
    )?;
    Ok(())
}

pub(super) fn decode_glider(
    _type_id: i32,
    _decoder: &mut Decoder<'_>,
    _summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    Ok(())
}

pub(super) fn decode_tooltip_style(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.tooltip_style = Some(read_resource_location(decoder)?);
    Ok(())
}

pub(super) fn decode_death_protection_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.death_protection_effects = decode_consume_effect_list_summary(decoder)?;
    Ok(())
}

pub(super) fn decode_piercing_weapon_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.piercing_weapon = Some(decode_piercing_weapon_summary(decoder)?);
    Ok(())
}

pub(super) fn decode_kinetic_weapon_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.kinetic_weapon = Some(decode_kinetic_weapon_summary(decoder)?);
    Ok(())
}

pub(super) fn decode_swing_animation_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.swing_animation = Some(decode_swing_animation(decoder)?);
    Ok(())
}

pub(super) fn decode_stored_enchantments(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.stored_enchantments = decode_varint_map(decoder)?;
    Ok(())
}

pub(super) fn decode_dye(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.dye_color_id = Some(normalize_vanilla_dye_color_id(decoder.read_var_i32()?));
    Ok(())
}

pub(super) fn decode_dyed_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.dyed_color = Some(decoder.read_i32()?);
    Ok(())
}

pub(super) fn decode_map_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.map_color = Some(decoder.read_i32()?);
    Ok(())
}

pub(super) fn decode_map_decorations(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let value = decode_nbt_summary_from_decoder(decoder)?.ok_or_else(|| {
        ProtocolError::InvalidData("map_decorations codec value is missing".to_string())
    })?;
    let decorations = parse_map_decorations_nbt(&value)?;
    let canonical = canonical_map_decorations_nbt(&decorations);
    summary.map_decorations = Some(decorations);
    summary.codec_backed_nbt.insert(type_id, canonical);
    Ok(())
}

pub(super) fn encode_map_decorations(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let decorations = summary.map_decorations.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("missing map_decorations component value".to_string())
    })?;
    if decorations.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            decorations.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    let mut seen = BTreeSet::new();
    for decoration in decorations {
        if !seen.insert(&decoration.id) {
            return Err(ProtocolError::InvalidData(format!(
                "duplicate map_decorations id {:?}",
                decoration.id
            )));
        }
        let canonical_resource =
            normalize_resource_location(decoration.decoration_type_resource_id.clone())?;
        if canonical_resource != decoration.decoration_type_resource_id {
            return Err(ProtocolError::InvalidData(format!(
                "map_decorations entry {:?} has noncanonical type resource location `{}`",
                decoration.id, decoration.decoration_type_resource_id
            )));
        }
        let Some(expected) = vanilla_map_decoration_type(decoration.decoration_type_id) else {
            return Err(ProtocolError::InvalidData(format!(
                "map_decorations entry {:?} has unknown type holder id {}",
                decoration.id, decoration.decoration_type_id
            )));
        };
        if expected.resource_id != decoration.decoration_type_resource_id {
            return Err(ProtocolError::InvalidData(format!(
                "map_decorations entry {:?} type holder id {} resolves to {}, not {}",
                decoration.id,
                decoration.decoration_type_id,
                expected.resource_id,
                decoration.decoration_type_resource_id
            )));
        }
    }

    let canonical = canonical_map_decorations_nbt(decorations);
    if let Some(retained) = summary.codec_backed_nbt.get(&type_id) {
        if retained != &canonical {
            return Err(ProtocolError::InvalidData(
                "map_decorations retained codec value does not match its typed entries".to_string(),
            ));
        }
    }
    let encoded = encode_nbt_summary_bounded(Some(&canonical), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

pub fn parse_map_decorations_nbt(value: &NbtSummaryValue) -> Result<Vec<ItemMapDecorationSummary>> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(ProtocolError::InvalidData(
            "map_decorations codec value is not a compound map".to_string(),
        ));
    };
    if entries.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            entries.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    // Compound keys overwrite by raw units before the value codec runs.
    let mut positions = BTreeMap::new();
    let mut visible = Vec::with_capacity(entries.len());
    for entry in entries {
        if let Some(&index) = positions.get(&entry.name) {
            visible[index] = entry;
        } else {
            positions.insert(&entry.name, visible.len());
            visible.push(entry);
        }
    }
    visible
        .into_iter()
        .map(|entry| parse_map_decoration_entry(&entry.name, &entry.value))
        .collect()
}

pub(super) fn parse_map_decoration_entry(
    id: &crate::JavaString,
    value: &NbtSummaryValue,
) -> Result<ItemMapDecorationSummary> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(ProtocolError::InvalidData(format!(
            "map_decorations entry {id:?} is not a compound"
        )));
    };
    let field = |name: &str| {
        entries
            .iter()
            .rev()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
            .ok_or_else(|| {
                ProtocolError::InvalidData(format!(
                    "map_decorations entry {id:?} is missing {name}"
                ))
            })
    };
    let raw_type = match field("type")? {
        NbtSummaryValue::String(raw_type) => raw_type,
        _ => {
            return Err(ProtocolError::InvalidData(format!(
                "map_decorations entry {id:?} type is not a resource location"
            )));
        }
    };
    let resource_id = normalize_resource_location(
        raw_type
            .try_to_string()
            .map_err(|error| ProtocolError::InvalidData(error.to_string()))?,
    )?;
    let decoration_type =
        vanilla_map_decoration_type_for_resource_id(&resource_id).ok_or_else(|| {
            ProtocolError::InvalidData(format!(
                "map_decorations entry {id:?} references unknown type {resource_id}"
            ))
        })?;
    Ok(ItemMapDecorationSummary {
        id: id.clone(),
        decoration_type_id: decoration_type.type_id,
        decoration_type_resource_id: decoration_type.resource_id.to_string(),
        x_bits: nbt_number_as_f64_bits(field("x")?, &format!("map_decorations entry {id:?} x"))?,
        z_bits: nbt_number_as_f64_bits(field("z")?, &format!("map_decorations entry {id:?} z"))?,
        rotation_bits: nbt_number_as_f32_bits(
            field("rotation")?,
            &format!("map_decorations entry {id:?} rotation"),
        )?,
    })
}

pub(super) fn nbt_number_as_f64_bits(value: &NbtSummaryValue, what: &str) -> Result<u64> {
    let value = match value {
        NbtSummaryValue::Byte(value) => f64::from(*value),
        NbtSummaryValue::Short(value) => f64::from(*value),
        NbtSummaryValue::Int(value) => f64::from(*value),
        NbtSummaryValue::Long(value) => *value as f64,
        NbtSummaryValue::Float(bits) => f64::from(f32::from_bits(*bits)),
        NbtSummaryValue::Double(bits) => return Ok(*bits),
        _ => return Err(ProtocolError::InvalidData(format!("{what} is not numeric"))),
    };
    Ok(value.to_bits())
}

pub(super) fn nbt_number_as_f32_bits(value: &NbtSummaryValue, what: &str) -> Result<u32> {
    let value = match value {
        NbtSummaryValue::Byte(value) => f32::from(*value),
        NbtSummaryValue::Short(value) => f32::from(*value),
        NbtSummaryValue::Int(value) => *value as f32,
        NbtSummaryValue::Long(value) => *value as f32,
        NbtSummaryValue::Float(bits) => return Ok(*bits),
        NbtSummaryValue::Double(bits) => f64::from_bits(*bits) as f32,
        _ => return Err(ProtocolError::InvalidData(format!("{what} is not numeric"))),
    };
    Ok(value.to_bits())
}

pub fn canonical_map_decorations_nbt(decorations: &[ItemMapDecorationSummary]) -> NbtSummaryValue {
    NbtSummaryValue::Compound(
        decorations
            .iter()
            .map(|decoration| NbtSummaryEntry {
                name: decoration.id.clone(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "type".into(),
                        value: NbtSummaryValue::String(
                            decoration.decoration_type_resource_id.clone().into(),
                        ),
                    },
                    NbtSummaryEntry {
                        name: "x".into(),
                        value: NbtSummaryValue::Double(decoration.x_bits),
                    },
                    NbtSummaryEntry {
                        name: "z".into(),
                        value: NbtSummaryValue::Double(decoration.z_bits),
                    },
                    NbtSummaryEntry {
                        name: "rotation".into(),
                        value: NbtSummaryValue::Float(decoration.rotation_bits),
                    },
                ]),
            })
            .collect(),
    )
}

pub(super) fn decode_recipes(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let value = decode_nbt_summary_from_decoder(decoder)?
        .ok_or_else(|| ProtocolError::InvalidData("recipes codec value is missing".to_string()))?;
    let recipe_keys = parse_recipe_keys_nbt(&value)?;
    let canonical = canonical_recipe_keys_nbt(&recipe_keys);
    summary.recipe_keys = Some(recipe_keys);
    summary.codec_backed_nbt.insert(type_id, canonical);
    Ok(())
}

pub(super) fn encode_recipes(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let recipe_keys = summary
        .recipe_keys
        .as_ref()
        .ok_or_else(|| ProtocolError::InvalidData("missing recipes component value".to_string()))?;
    if recipe_keys.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            recipe_keys.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    for (index, resource_id) in recipe_keys.iter().enumerate() {
        let canonical = normalize_resource_location(resource_id.clone())?;
        if canonical != *resource_id {
            return Err(ProtocolError::InvalidData(format!(
                "recipes[{index}] has noncanonical resource location `{resource_id}`"
            )));
        }
    }
    let canonical = canonical_recipe_keys_nbt(recipe_keys);
    if let Some(retained) = summary.codec_backed_nbt.get(&type_id) {
        if retained != &canonical {
            return Err(ProtocolError::InvalidData(
                "recipes retained codec value does not match its typed keys".to_string(),
            ));
        }
    }
    let encoded = encode_nbt_summary_bounded(Some(&canonical), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

pub fn parse_recipe_keys_nbt(value: &NbtSummaryValue) -> Result<Vec<String>> {
    let NbtSummaryValue::List(values) = value else {
        return Err(ProtocolError::InvalidData(
            "recipes codec value is not a list".to_string(),
        ));
    };
    if values.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            values.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let NbtSummaryValue::String(resource_id) = value else {
                return Err(ProtocolError::InvalidData(format!(
                    "recipes[{index}] is not a resource location"
                )));
            };
            normalize_resource_location(
                resource_id
                    .try_to_string()
                    .map_err(|error| ProtocolError::InvalidData(error.to_string()))?,
            )
        })
        .collect()
}

pub fn canonical_recipe_keys_nbt(recipe_keys: &[String]) -> NbtSummaryValue {
    NbtSummaryValue::List(recipe_keys.iter().map(NbtSummaryValue::string).collect())
}

pub(super) fn decode_debug_stick_state(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let value = decode_required_compound_tag(decoder, "debug_stick_state")?;
    if let NbtSummaryValue::Compound(entries) = &value {
        if entries.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
            return Err(ProtocolError::PacketTooLarge(
                entries.len(),
                MAX_DATA_COMPONENT_LIST_ITEMS,
            ));
        }
    }
    summary.debug_stick_properties = decode_debug_stick_properties_with_registry(&value, registry)?;
    Ok(())
}

pub fn decode_debug_stick_properties(
    value: &NbtSummaryValue,
) -> Result<Vec<DebugStickPropertySummary>> {
    decode_debug_stick_properties_with_registry(value, ContentRegistry::vanilla())
}

pub fn decode_debug_stick_properties_with_registry(
    value: &NbtSummaryValue,
    registry: &ContentRegistry,
) -> Result<Vec<DebugStickPropertySummary>> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(ProtocolError::InvalidData(
            "debug_stick_state value must be a non-null compound tag".to_string(),
        ));
    };
    let mut compound_entries = Vec::<&NbtSummaryEntry>::with_capacity(entries.len());
    for entry in entries {
        if let Some(existing) = compound_entries
            .iter_mut()
            .find(|existing| existing.name == entry.name)
        {
            *existing = entry;
        } else {
            compound_entries.push(entry);
        }
    }

    let mut seen_blocks = BTreeSet::new();
    compound_entries
        .into_iter()
        .map(|entry| {
            let name = String::from_utf16(entry.name.units()).map_err(|_| {
                ProtocolError::InvalidData(format!(
                    "debug_stick_state block resource id is not valid Unicode: {:?}",
                    entry.name
                ))
            })?;
            let resource_id = normalize_resource_location(name)?;
            let (block_id, vanilla_resource_id) =
                if let Some(block) = vanilla_block_type_for_resource_id(&resource_id) {
                    (block.id, Some(block.resource_id))
                } else if let Some(block) = registry.block_by_resource_id(&resource_id) {
                    (block.block_id, None)
                } else {
                    return Err(ProtocolError::InvalidData(format!(
                        "debug_stick_state references unknown block {resource_id}"
                    )));
                };
            if !seen_blocks.insert(block_id) {
                return Err(ProtocolError::InvalidData(format!(
                    "debug_stick_state contains duplicate block {resource_id}"
                )));
            }
            let NbtSummaryValue::String(property) = &entry.value else {
                return Err(ProtocolError::InvalidData(format!(
                    "debug_stick_state property for block {resource_id} is not a string"
                )));
            };
            let property = property
                .try_to_string()
                .map_err(|error| ProtocolError::InvalidData(error.to_string()))?;
            if let Some(vanilla_id) = vanilla_resource_id {
                if !vanilla_block_has_property(vanilla_id, &property) {
                    return Err(ProtocolError::InvalidData(format!(
                        "block {resource_id} has no property {property} for debug_stick_state"
                    )));
                }
            }
            Ok(DebugStickPropertySummary {
                block_registry_id: block_id,
                property,
            })
        })
        .collect()
}

pub(super) fn encode_debug_stick_state(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    if summary.debug_stick_properties.len() > MAX_DATA_COMPONENT_LIST_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            summary.debug_stick_properties.len(),
            MAX_DATA_COMPONENT_LIST_ITEMS,
        ));
    }
    let value = canonical_debug_stick_properties_nbt_with_registry(
        &summary.debug_stick_properties,
        registry,
    )?;
    let encoded = encode_nbt_summary_bounded(Some(&value), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

pub fn canonical_debug_stick_properties_nbt(
    properties: &[DebugStickPropertySummary],
) -> Result<NbtSummaryValue> {
    canonical_debug_stick_properties_nbt_with_registry(properties, ContentRegistry::vanilla())
}

pub fn canonical_debug_stick_properties_nbt_with_registry(
    properties: &[DebugStickPropertySummary],
    registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let mut seen_blocks = BTreeSet::new();
    let mut entries = Vec::with_capacity(properties.len());
    for property in properties {
        let (block_id, resource_id, is_vanilla) =
            if let Some(block) = vanilla_block_type(property.block_registry_id) {
                (block.id, block.resource_id, true)
            } else if let Some(block) = registry.block_by_id(property.block_registry_id) {
                (block.block_id, block.resource_id.as_str(), false)
            } else {
                return Err(ProtocolError::InvalidData(format!(
                    "debug_stick_state has invalid block holder id {}",
                    property.block_registry_id
                )));
            };
        if !seen_blocks.insert(block_id) {
            return Err(ProtocolError::InvalidData(format!(
                "debug_stick_state contains duplicate block {resource_id}"
            )));
        }
        if is_vanilla && !vanilla_block_has_property(resource_id, &property.property) {
            return Err(ProtocolError::InvalidData(format!(
                "block {resource_id} has no property {} for debug_stick_state",
                property.property
            )));
        }
        entries.push(NbtSummaryEntry {
            name: resource_id.into(),
            value: NbtSummaryValue::string(&property.property),
        });
    }
    Ok(NbtSummaryValue::Compound(entries))
}

pub(super) fn decode_bucket_entity_data(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let value = decode_required_compound_tag(decoder, "bucket_entity_data")?;
    summary.codec_backed_nbt.insert(type_id, value);
    Ok(())
}

pub(super) fn decode_map_post_processing_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.map_post_processing = Some(decode_map_post_processing(decoder)?);
    Ok(())
}

pub(super) fn decode_charged_projectiles(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    summary.charged_projectiles_items =
        decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS, registry)?;
    Ok(())
}

pub(super) fn decode_bundle_contents(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    summary.bundle_contents_items =
        decode_item_stack_template_list(decoder, MAX_DATA_COMPONENT_LIST_ITEMS, registry)?;
    Ok(())
}

pub(super) fn decode_potion_contents_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let potion_id = if decoder.read_bool()? {
        let potion_id = decoder.read_var_i32()?;
        if vanilla_potion(potion_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid vanilla potion holder id {potion_id}"
            )));
        }
        Some(potion_id)
    } else {
        None
    };
    let custom_color = decode_optional_i32_value(decoder)?;
    let effect_count = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let mut custom_effects = Vec::with_capacity(decoder.bounded_capacity(effect_count, 7));
    for _ in 0..effect_count {
        custom_effects.push(decode_mob_effect_instance(decoder)?);
    }
    let custom_name = if decoder.read_bool()? {
        Some(decoder.read_java_string(MAX_STRING_CHARS)?)
    } else {
        None
    };
    summary.potion_contents = PotionContentsSummary {
        potion_id,
        custom_color,
        custom_effects,
        custom_name,
    };
    Ok(())
}

pub(super) fn decode_potion_duration_scale(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.potion_duration_scale_bits = Some(decoder.read_f32()?.to_bits());
    Ok(())
}

pub(super) fn decode_suspicious_stew_effects_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.suspicious_stew_effects = decode_suspicious_stew_effects(decoder)?;
    Ok(())
}

pub(super) fn decode_writable_book_content_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let page_count = read_bounded_len(decoder, MAX_BOOK_PAGES)?;
    let page_capacity = decoder.bounded_capacity(page_count, 2);
    let mut pages = Vec::with_capacity(page_capacity);
    let mut page_filters = Vec::with_capacity(page_capacity);
    for _ in 0..page_count {
        pages.push(decoder.read_java_string(MAX_WRITABLE_BOOK_PAGE_CHARS)?);
        let filtered = if decoder.read_bool()? {
            Some(decoder.read_java_string(MAX_WRITABLE_BOOK_PAGE_CHARS)?)
        } else {
            None
        };
        page_filters.push(filtered);
    }
    summary.writable_book_pages = pages;
    summary.writable_book_page_filters = page_filters;
    Ok(())
}

pub(super) fn decode_written_book_content_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let title = decoder.read_java_string(MAX_WRITTEN_BOOK_TITLE_CHARS)?;
    let title_filter = if decoder.read_bool()? {
        Some(decoder.read_java_string(MAX_WRITTEN_BOOK_TITLE_CHARS)?)
    } else {
        None
    };
    let author = decoder.read_java_string(MAX_STRING_CHARS)?;
    let generation = decoder.read_var_i32()?;
    if !(0..=3).contains(&generation) {
        return Err(ProtocolError::InvalidData(format!(
            "written book generation {generation} is outside 0..=3"
        )));
    }
    let pages = read_bounded_len(decoder, MAX_DATA_COMPONENT_LIST_ITEMS)?;
    let page_capacity = decoder.bounded_capacity(pages, 2);
    let mut page_values = Vec::with_capacity(page_capacity);
    let mut page_filters = Vec::with_capacity(page_capacity);
    let mut pages_nbt = Vec::with_capacity(page_capacity);
    let mut page_filters_nbt = Vec::with_capacity(page_capacity);
    for _ in 0..pages {
        let page = decode_filterable_component_summary(decoder, registry)?;
        page_values.push(page.raw);
        page_filters.push(page.filtered);
        pages_nbt.push(page.raw_nbt);
        page_filters_nbt.push(page.filtered_nbt);
    }
    let resolved = decoder.read_bool()?;
    summary.written_book = Some(WrittenBookContentSummary {
        title,
        title_filter,
        author,
        generation,
        pages: page_values,
        page_filters,
        resolved,
    });
    summary.written_book_pages_nbt = pages_nbt;
    summary.written_book_page_filters_nbt = page_filters_nbt;
    Ok(())
}

pub(super) fn decode_trim(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let (material_id, material_direct) = decode_registry_or_inline(decoder, |decoder| {
        decode_direct_trim_material_summary(decoder, registry)
    })?;
    let (pattern_id, pattern_direct) = decode_trim_pattern_holder(decoder, registry)?;
    summary.armor_trim_material_id = material_id;
    summary.armor_trim_material_direct = material_direct.map(Box::new);
    summary.armor_trim_pattern_id = pattern_id;
    summary.armor_trim_pattern_direct = pattern_direct.map(Box::new);
    Ok(())
}

pub(super) fn decode_entity_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let entity_data =
        decode_typed_entity_data_summary(decoder, TypedEntityDataRegistry::EntityType, registry)?;
    summary.entity_data_entity_type_id = Some(entity_data.type_id);
    summary.entity_data_tag = Some(entity_data.tag);
    Ok(())
}

pub(super) fn decode_block_entity_data(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let block_entity_data = decode_typed_entity_data_summary(
        decoder,
        TypedEntityDataRegistry::BlockEntityType,
        registry,
    )?;
    summary.block_entity_data_type_id = Some(block_entity_data.type_id);
    summary.block_entity_spawn_entity_type =
        typed_entity_data_spawn_entity_type(Some(&block_entity_data.tag));
    summary.block_entity_data_tag = Some(block_entity_data.tag);
    Ok(())
}

pub(super) fn decode_instrument(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let (registry_id, direct) = decode_registry_or_inline(decoder, |decoder| {
        decode_direct_instrument_summary(decoder, registry)
    })?;
    summary.instrument = InstrumentComponentSummary {
        registry_id,
        direct: direct.map(Box::new),
    };
    Ok(())
}

pub(super) fn decode_jukebox_playable_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let (registry_id, direct) = decode_registry_or_inline(decoder, |decoder| {
        decode_direct_jukebox_song_summary(decoder, registry)
    })?;
    summary.jukebox_song_id = registry_id;
    summary.jukebox_direct_song = direct.map(Box::new);
    Ok(())
}

pub(super) fn decode_provides_banner_patterns(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let patterns = decode_holder_set_summary(decoder, NetworkHolderSetRegistry::BannerPattern)?;
    summary.provides_banner_pattern_registry_ids = patterns.registry_ids;
    summary.provides_banner_pattern_tag = patterns.tag;
    Ok(())
}

pub(super) fn encode_provides_banner_patterns(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    encode_holder_set_summary(
        out,
        &summary.provides_banner_pattern_registry_ids,
        summary.provides_banner_pattern_tag.as_deref(),
        NetworkHolderSetRegistry::BannerPattern,
        "provides_banner_patterns holder set",
    )?;
    Ok(())
}

pub(super) fn decode_lodestone_tracker_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let (target, tracked) = decode_lodestone_tracker(decoder)?;
    summary.lodestone_target = target;
    summary.lodestone_tracked = Some(tracked);
    Ok(())
}

pub(super) fn decode_firework_explosion_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.firework_explosion = Some(decode_firework_explosion(decoder)?);
    Ok(())
}

pub(super) fn decode_fireworks_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let flight_duration = decoder.read_var_i32()?;
    let explosion_count = read_bounded_len(decoder, MAX_FIREWORK_EXPLOSIONS)?;
    let mut explosions = Vec::with_capacity(decoder.bounded_capacity(explosion_count, 5));
    for _ in 0..explosion_count {
        explosions.push(decode_firework_explosion(decoder)?);
    }
    summary.fireworks_flight_duration = Some(flight_duration);
    summary.fireworks_explosions = explosions;
    Ok(())
}

pub(super) fn decode_profile(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.profile = Some(decode_resolvable_profile(decoder)?);
    Ok(())
}

pub(super) fn decode_note_block_sound(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.note_block_sound = Some(read_resource_location(decoder)?);
    Ok(())
}

pub(super) fn decode_banner_patterns(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.banner_pattern_layers = decode_banner_pattern_layers_summary(decoder)?;
    Ok(())
}

pub(super) fn decode_base_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.base_color_id = Some(normalize_vanilla_dye_color_id(decoder.read_var_i32()?));
    Ok(())
}

pub(super) fn encode_base_color(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let color_id = summary
        .base_color_id
        .ok_or_else(|| ProtocolError::InvalidData("missing base_color value".to_string()))?;
    if normalize_vanilla_dye_color_id(color_id) != color_id {
        return Err(ProtocolError::InvalidData(format!(
            "base_color dye color id {color_id} is not canonical"
        )));
    }
    out.write_var_i32(color_id);
    Ok(())
}

pub(super) fn decode_pot_decorations_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    summary.pot_decorations_item_ids = decode_pot_decorations_item_ids(decoder, registry)?;
    Ok(())
}

pub(super) fn encode_pot_decorations(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    if summary.pot_decorations_item_ids.len() > MAX_POT_DECORATIONS {
        return Err(ProtocolError::PacketTooLarge(
            summary.pot_decorations_item_ids.len(),
            MAX_POT_DECORATIONS,
        ));
    }
    let mut item_ids = [VANILLA_ITEM_BRICK_ID; MAX_POT_DECORATIONS];
    for (target, item_id) in item_ids.iter_mut().zip(&summary.pot_decorations_item_ids) {
        if vanilla_item_type(*item_id).is_none() && registry.item_by_id(*item_id).is_none() {
            return Err(ProtocolError::InvalidData(format!(
                "invalid pot decoration item registry id {item_id}"
            )));
        }
        *target = *item_id;
    }

    out.write_var_i32(MAX_POT_DECORATIONS as i32);
    for item_id in item_ids {
        out.write_var_i32(item_id);
    }
    Ok(())
}

pub(super) fn decode_container(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let container = decode_item_container_contents(decoder, registry)?;
    summary.container = container;
    Ok(())
}

pub(super) fn decode_block_state(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let count = read_bounded_len(decoder, MAX_BLOCK_STATE_PROPERTIES)?;
    let mut entries = Vec::with_capacity(decoder.bounded_capacity(count, 2));
    for _ in 0..count {
        let name = decoder.read_string(MAX_STRING_CHARS)?.into();
        let value = decoder.read_string(MAX_STRING_CHARS)?.into();
        entries.push((name, value));
    }
    summary.block_state_properties = BlockStateProperties::from_entries(entries);
    Ok(())
}

pub(super) fn encode_block_state(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let properties = &summary.block_state_properties;
    if properties.len() > MAX_BLOCK_STATE_PROPERTIES {
        return Err(ProtocolError::PacketTooLarge(
            properties.len(),
            MAX_BLOCK_STATE_PROPERTIES,
        ));
    }

    out.write_var_i32(properties.len() as i32);
    for (name, value) in properties {
        for units in [name.units(), value.units()] {
            if units.len() > MAX_STRING_CHARS {
                return Err(ProtocolError::StringTooLong {
                    actual: units.len(),
                    max: MAX_STRING_CHARS,
                });
            }
            out.write_utf16_string(units);
        }
    }
    Ok(())
}

pub(super) fn decode_bees_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    summary.bees_present = true;
    summary.bees = decode_bees(decoder, registry)?;
    Ok(())
}

pub(super) fn decode_lock(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let value = decode_required_compound_tag(decoder, "lock")?;
    summary.lock_predicate = Some(decode_persistent_item_predicate_summary_with_registry(
        &value, registry,
    )?);
    Ok(())
}

pub(super) fn encode_lock(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
    registry: &ContentRegistry,
) -> Result<()> {
    let predicate = summary
        .lock_predicate
        .as_ref()
        .ok_or_else(|| ProtocolError::InvalidData("missing lock component value".to_string()))?;
    let value = encode_persistent_item_predicate_summary_with_registry(predicate, registry)?;
    let encoded = encode_nbt_summary_bounded(Some(&value), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

pub(super) fn decode_container_loot(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let value = decode_nbt_summary_from_decoder(decoder)?.ok_or_else(|| {
        ProtocolError::InvalidData("container_loot codec value is missing".to_string())
    })?;
    let (loot_table, seed) = parse_container_loot_nbt(&value)?;
    let canonical = canonical_container_loot_nbt(&loot_table, seed);
    summary.container_loot = true;
    summary.container_loot_table = Some(loot_table);
    summary.container_loot_seed = Some(seed);
    summary.codec_backed_nbt.insert(type_id, canonical);
    Ok(())
}

pub(super) fn encode_container_loot(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    if !summary.container_loot {
        return Err(ProtocolError::InvalidData(
            "missing container_loot component value".to_string(),
        ));
    }
    let loot_table = summary.container_loot_table.as_deref().ok_or_else(|| {
        ProtocolError::InvalidData("missing container_loot loot_table".to_string())
    })?;
    let canonical_loot_table = normalize_resource_location(loot_table.to_string())?;
    if canonical_loot_table != loot_table {
        return Err(ProtocolError::InvalidData(format!(
            "container_loot has noncanonical loot_table resource location `{loot_table}`"
        )));
    }
    let seed = summary
        .container_loot_seed
        .ok_or_else(|| ProtocolError::InvalidData("missing container_loot seed".to_string()))?;
    let canonical = canonical_container_loot_nbt(loot_table, seed);
    if let Some(retained) = summary.codec_backed_nbt.get(&type_id) {
        if retained != &canonical {
            return Err(ProtocolError::InvalidData(
                "container_loot retained codec value does not match its typed fields".to_string(),
            ));
        }
    }
    let encoded = encode_nbt_summary_bounded(Some(&canonical), i32::MAX as usize, MAX_NBT_DEPTH)?;
    out.write_bytes(&encoded);
    Ok(())
}

pub fn parse_container_loot_nbt(value: &NbtSummaryValue) -> Result<(String, i64)> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(ProtocolError::InvalidData(
            "container_loot codec value is not a compound".to_string(),
        ));
    };
    let loot_table = entries
        .iter()
        .rev()
        .find(|entry| entry.name == "loot_table")
        .ok_or_else(|| {
            ProtocolError::InvalidData("container_loot is missing loot_table".to_string())
        })?;
    let NbtSummaryValue::String(loot_table) = &loot_table.value else {
        return Err(ProtocolError::InvalidData(
            "container_loot loot_table is not a string".to_string(),
        ));
    };
    let loot_table = normalize_resource_location(
        loot_table
            .try_to_string()
            .map_err(|error| ProtocolError::InvalidData(error.to_string()))?,
    )?;
    let seed = entries
        .iter()
        .rev()
        .find(|entry| entry.name == "seed")
        .map(|entry| nbt_number_as_i64(&entry.value, "container_loot seed"))
        .transpose()?
        .unwrap_or(0);
    Ok((loot_table, seed))
}

pub(super) fn nbt_number_as_i64(value: &NbtSummaryValue, what: &str) -> Result<i64> {
    match value {
        NbtSummaryValue::Byte(value) => Ok(i64::from(*value)),
        NbtSummaryValue::Short(value) => Ok(i64::from(*value)),
        NbtSummaryValue::Int(value) => Ok(i64::from(*value)),
        NbtSummaryValue::Long(value) => Ok(*value),
        NbtSummaryValue::Float(bits) => Ok(f32::from_bits(*bits) as i64),
        NbtSummaryValue::Double(bits) => Ok(f64::from_bits(*bits) as i64),
        _ => Err(ProtocolError::InvalidData(format!("{what} is not numeric"))),
    }
}

pub fn canonical_container_loot_nbt(loot_table: &str, seed: i64) -> NbtSummaryValue {
    let mut entries = vec![NbtSummaryEntry {
        name: "loot_table".into(),
        value: NbtSummaryValue::String(loot_table.to_string().into()),
    }];
    if seed != 0 {
        entries.push(NbtSummaryEntry {
            name: "seed".into(),
            value: NbtSummaryValue::Long(seed),
        });
    }
    NbtSummaryValue::Compound(entries)
}

pub(super) fn decode_break_sound(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.break_sound = Some(decode_sound_event_holder_summary(decoder)?);
    Ok(())
}

pub(super) fn decode_registry_holder_component<const TYPE_ID: i32>(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    let field = match TYPE_ID {
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID => &mut summary.damage_type_id,
        VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID => &mut summary.villager_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID => &mut summary.wolf_variant_id,
        VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID => &mut summary.wolf_sound_variant_id,
        VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID => &mut summary.frog_variant_id,
        VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID => {
            &mut summary.zombie_nautilus_variant_id
        }
        VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID => &mut summary.pig_variant_id,
        VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID => &mut summary.pig_sound_variant_id,
        VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID => &mut summary.cow_variant_id,
        VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID => &mut summary.cow_sound_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID => &mut summary.chicken_variant_id,
        VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID => {
            &mut summary.chicken_sound_variant_id
        }
        VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID => &mut summary.cat_variant_id,
        VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID => &mut summary.cat_sound_variant_id,
        _ => unreachable!("registry holder component binding"),
    };
    *field = Some(decode_holder_registry_id(decoder)?);
    Ok(())
}

pub(super) fn decode_builtin_enum(
    type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
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

pub(super) fn encode_builtin_enum(
    type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let value_id = summary
        .builtin_enum_component_ids
        .get(&type_id)
        .copied()
        .ok_or_else(|| {
            ProtocolError::InvalidData(format!(
                "built-in enum component type id {type_id} is missing"
            ))
        })?;
    let normalized =
        normalize_builtin_enum_component_value_id(type_id, value_id).ok_or_else(|| {
            ProtocolError::InvalidData(format!(
                "built-in enum component type id {type_id} has no value catalog"
            ))
        })?;
    if normalized != value_id {
        return Err(ProtocolError::InvalidData(format!(
            "built-in enum component type id {type_id} value id {value_id} is not canonical"
        )));
    }
    out.write_var_i32(value_id);
    Ok(())
}

pub(super) fn decode_tropical_fish_pattern(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.tropical_fish_pattern_id =
        Some(vanilla_tropical_fish_pattern(decoder.read_var_i32()?).packed_id);
    Ok(())
}

pub(super) fn encode_tropical_fish_pattern(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let pattern_id = summary.tropical_fish_pattern_id.ok_or_else(|| {
        ProtocolError::InvalidData("tropical_fish_pattern is missing".to_string())
    })?;
    if vanilla_tropical_fish_pattern(pattern_id).packed_id != pattern_id {
        return Err(ProtocolError::InvalidData(format!(
            "unsupported tropical_fish_pattern packed id {pattern_id}"
        )));
    }
    out.write_var_i32(pattern_id);
    Ok(())
}

pub(super) fn decode_tropical_fish_base_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.tropical_fish_base_color_id =
        Some(normalize_vanilla_dye_color_id(decoder.read_var_i32()?));
    Ok(())
}

pub(super) fn encode_tropical_fish_base_color(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let color_id = summary.tropical_fish_base_color_id.ok_or_else(|| {
        ProtocolError::InvalidData("tropical_fish_base_color is missing".to_string())
    })?;
    if normalize_vanilla_dye_color_id(color_id) != color_id {
        return Err(ProtocolError::InvalidData(format!(
            "tropical_fish_base_color dye id {color_id} is not canonical"
        )));
    }
    out.write_var_i32(color_id);
    Ok(())
}

pub(super) fn decode_tropical_fish_pattern_color(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.tropical_fish_pattern_color_id =
        Some(normalize_vanilla_dye_color_id(decoder.read_var_i32()?));
    Ok(())
}

pub(super) fn encode_tropical_fish_pattern_color(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    out: &mut Encoder,
) -> Result<()> {
    let color_id = summary.tropical_fish_pattern_color_id.ok_or_else(|| {
        ProtocolError::InvalidData("tropical_fish_pattern_color is missing".to_string())
    })?;
    if normalize_vanilla_dye_color_id(color_id) != color_id {
        return Err(ProtocolError::InvalidData(format!(
            "tropical_fish_pattern_color dye id {color_id} is not canonical"
        )));
    }
    out.write_var_i32(color_id);
    Ok(())
}

pub(super) fn decode_painting_variant(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let (registry_id, direct) = decode_registry_or_inline(decoder, |decoder| {
        decode_direct_painting_variant_summary(decoder, registry)
    })?;
    summary.painting_variant_id = registry_id;
    summary.painting_variant_direct = direct;
    Ok(())
}

pub(super) fn decode_creative_slot_lock(
    _type_id: i32,
    _decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.creative_slot_lock = true;
    Ok(())
}

pub(super) fn encode_creative_slot_lock(
    _type_id: i32,
    summary: &DataComponentPatchSummary,
    _out: &mut Encoder,
) -> Result<()> {
    if !summary.creative_slot_lock {
        return Err(ProtocolError::InvalidData(
            "missing creative_slot_lock component value".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn decode_tool_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    summary.tool = Some(decode_tool(decoder, registry)?);
    Ok(())
}

pub(super) fn decode_equippable_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    summary.equippable = Some(decode_equippable(decoder, registry)?);
    Ok(())
}

pub(super) fn decode_blocks_attacks_component(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    _registry: &ContentRegistry,
) -> Result<()> {
    summary.blocks_attacks = Some(decode_blocks_attacks(decoder)?);
    Ok(())
}

pub(super) fn decode_provides_trim_material(
    _type_id: i32,
    decoder: &mut Decoder<'_>,
    summary: &mut DataComponentPatchSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    let (registry_id, direct) = decode_registry_or_inline(decoder, |decoder| {
        decode_direct_trim_material_summary(decoder, registry)
    })?;
    summary.provides_trim_material_id = registry_id;
    summary.provides_trim_material_direct = direct.map(Box::new);
    Ok(())
}
