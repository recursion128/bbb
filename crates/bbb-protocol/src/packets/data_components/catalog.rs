use super::*;
use std::sync::OnceLock;

use crate::block_entity_types::vanilla_block_entity_type;
use crate::data_component_types::{
    DataComponentTypeCatalogEntry, VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_ENTITY_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID, VANILLA_DATA_COMPONENT_TYPES,
    VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID,
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
    use std::collections::HashSet;

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
            vec![0, 1, 2, 3, 4, 16, 58, 59, 60, 70]
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
