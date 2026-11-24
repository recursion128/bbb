//! Exact leaf payloads shared by entity and location enchantment effects.

use crate::{
    enchantment_codec_types::{
        EnchantmentEntityEffectCodecFamily, EnchantmentLocationBasedEffectCodecFamily,
    },
    persistent_component_nbt::source_codec::{
        decode_fixed_holder_reference_source, decode_homogeneous_fixed_holder_set_source,
        decode_sound_event_holder_source,
    },
    registry_codec::{
        environment_particle_source::decode_vec3_source,
        recursive_values::{
            decode_compact_list, required_field, CanonicalRegistryCodecValue,
            RegistryCodecDecodeContext, RegistryCodecRecord,
        },
        value_provider_source::decode_float_provider_source,
        RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
    },
};
use bbb_facts::attribute_modifier_types::attribute_modifier_operation_id;
use bbb_protocol::NbtSummaryValue;

use crate::registry_codec::enchantment_source::value::{child, decode_level_based_value_source};

const ATTRIBUTE_REGISTRY: &str = "minecraft:attribute";
const DAMAGE_TYPE_REGISTRY: &str = "minecraft:damage_type";
const ENTITY_TYPE_REGISTRY: &str = "minecraft:entity_type";
const MOB_EFFECT_REGISTRY: &str = "minecraft:mob_effect";

/// Decodes only the entity-effect families owned by this leaf module.
pub(super) fn decode_entity_leaf_effect_source(
    family: EnchantmentEntityEffectCodecFamily,
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Option<Result<CanonicalRegistryCodecValue, RegistryCodecError>> {
    let result = match family {
        EnchantmentEntityEffectCodecFamily::ApplyMobEffect => {
            decode_apply_mob_effect(input, field, context)
        }
        EnchantmentEntityEffectCodecFamily::ChangeItemDamage
        | EnchantmentEntityEffectCodecFamily::ApplyExhaustion => {
            decode_one_level(input, "amount", field, context)
        }
        EnchantmentEntityEffectCodecFamily::DamageEntity => {
            decode_damage_entity(input, field, context)
        }
        EnchantmentEntityEffectCodecFamily::Ignite => {
            decode_one_level(input, "duration", field, context)
        }
        EnchantmentEntityEffectCodecFamily::ApplyEntityImpulse => {
            decode_impulse(input, field, context)
        }
        EnchantmentEntityEffectCodecFamily::PlaySoundEffect => {
            decode_play_sound(input, field, context)
        }
        EnchantmentEntityEffectCodecFamily::RunFunction => {
            decode_run_function(input, field, context)
        }
        EnchantmentEntityEffectCodecFamily::SummonEntityEffect => {
            decode_summon_entity(input, field, context)
        }
        _ => return None,
    };
    Some(result.map(|output| output.finish_typed(type_id)))
}

/// Decodes only the location-effect families owned by this leaf module.
pub(super) fn decode_location_leaf_effect_source(
    family: EnchantmentLocationBasedEffectCodecFamily,
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Option<Result<CanonicalRegistryCodecValue, RegistryCodecError>> {
    let result = match family {
        EnchantmentLocationBasedEffectCodecFamily::ApplyMobEffect => {
            decode_apply_mob_effect(input, field, context)
        }
        EnchantmentLocationBasedEffectCodecFamily::EnchantmentAttributeEffect => {
            decode_attribute(input, field, context)
        }
        EnchantmentLocationBasedEffectCodecFamily::ChangeItemDamage
        | EnchantmentLocationBasedEffectCodecFamily::ApplyExhaustion => {
            decode_one_level(input, "amount", field, context)
        }
        EnchantmentLocationBasedEffectCodecFamily::DamageEntity => {
            decode_damage_entity(input, field, context)
        }
        EnchantmentLocationBasedEffectCodecFamily::Ignite => {
            decode_one_level(input, "duration", field, context)
        }
        EnchantmentLocationBasedEffectCodecFamily::ApplyEntityImpulse => {
            decode_impulse(input, field, context)
        }
        EnchantmentLocationBasedEffectCodecFamily::PlaySoundEffect => {
            decode_play_sound(input, field, context)
        }
        EnchantmentLocationBasedEffectCodecFamily::RunFunction => {
            decode_run_function(input, field, context)
        }
        EnchantmentLocationBasedEffectCodecFamily::SummonEntityEffect => {
            decode_summon_entity(input, field, context)
        }
        _ => return None,
    };
    Some(result.map(|output| output.finish_typed(type_id)))
}

fn decode_apply_mob_effect(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_holder_set(
        &mut output,
        input,
        "to_apply",
        field,
        context,
        MOB_EFFECT_REGISTRY,
    )?;
    for name in [
        "min_duration",
        "max_duration",
        "min_amplifier",
        "max_amplifier",
    ] {
        push_level(&mut output, input, name, field, context)?;
    }
    Ok(output)
}
fn decode_one_level(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_level(&mut output, input, name, field, context)?;
    Ok(output)
}
fn decode_damage_entity(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_level(&mut output, input, "min_damage", field, context)?;
    push_level(&mut output, input, "max_damage", field, context)?;
    push_direct_holder(
        &mut output,
        input,
        "damage_type",
        field,
        context,
        DAMAGE_TYPE_REGISTRY,
    )?;
    Ok(output)
}
fn decode_impulse(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_vec3(&mut output, input, "direction", field, context)?;
    push_vec3(&mut output, input, "coordinate_scale", field, context)?;
    push_level(&mut output, input, "magnitude", field, context)?;
    Ok(output)
}
fn decode_attribute(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_identifier(&mut output, input, "id", field, context)?;
    push_direct_holder(
        &mut output,
        input,
        "attribute",
        field,
        context,
        ATTRIBUTE_REGISTRY,
    )?;
    push_level(&mut output, input, "amount", field, context)?;
    let operation_field = child(field, "operation");
    context.charge_node(&operation_field)?;
    let operation = required_field(input, "operation", field, context)?
        .java_string_ref()
        .filter(|value| {
            value
                .with_utf8(attribute_modifier_operation_id)
                .flatten()
                .is_some()
        })
        .ok_or_else(|| {
            context.codec_error(&operation_field, "unknown attribute modifier operation")
        })?;
    output.push(
        "operation",
        CanonicalRegistryCodecValue::new(NbtSummaryValue::String(operation.into())),
    );
    Ok(output)
}

/// Decodes the un-dispatched `EnchantmentAttributeEffect.MAP_CODEC` payload.
///
/// `EnchantmentLocationBasedEffect.CODEC` appends its own `type` discriminator,
/// while the enchantment `attributes` effect component stores this record
/// directly. Keeping the payload owner here ensures both routes share field
/// order, holder retention, and level-value semantics
pub(in crate::registry_codec::enchantment_source) fn decode_enchantment_attribute_effect_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "enchantment attribute effect")?;
    decode_attribute(input, field, context).map(RegistryCodecRecord::finish)
}

fn decode_run_function(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_identifier(&mut output, input, "function", field, context)?;
    Ok(output)
}
fn decode_summon_entity(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_holder_set(
        &mut output,
        input,
        "entity",
        field,
        context,
        ENTITY_TYPE_REGISTRY,
    )?;
    if let Some(value) = input.field("join_team") {
        let nested = child(field, "join_team");
        context.charge_node(&nested)?;
        let value = value
            .as_bool()
            .ok_or_else(|| context.codec_error(&nested, "expected Codec.BOOL input"))?;
        if value {
            output.push(
                "join_team",
                CanonicalRegistryCodecValue::new(NbtSummaryValue::Byte(1)),
            );
        }
    }
    Ok(output)
}
fn decode_play_sound(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut output = RegistryCodecRecord::default();
    push_sound_list(&mut output, input, field, context)?;
    push_float_provider(&mut output, input, "volume", field, context, 1.0e-5, 10.0)?;
    push_float_provider(&mut output, input, "pitch", field, context, 1.0e-5, 2.0)?;
    Ok(output)
}

fn push_level(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let nested = child(field, name);
    let value = required_field(input, name, field, context)?;
    let value = context.with_nested(&nested, |context| {
        decode_level_based_value_source(value, &nested, context)
    })?;
    output.push(name, value);
    Ok(())
}
fn push_holder_set(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    registry: &'static str,
) -> Result<(), RegistryCodecError> {
    let nested = child(field, name);
    let value = required_field(input, name, field, context)?;
    let value = context.with_nested(&nested, |context| {
        decode_homogeneous_fixed_holder_set_source(value, &nested, context, registry)
    })?;
    output.push(name, value);
    Ok(())
}
fn push_direct_holder(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    registry: &'static str,
) -> Result<(), RegistryCodecError> {
    let nested = child(field, name);
    let value = required_field(input, name, field, context)?;
    let value = context.with_nested(&nested, |context| {
        decode_fixed_holder_reference_source(value, &nested, context, registry)
    })?;
    output.push(name, value);
    Ok(())
}
fn push_identifier(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let nested = child(field, name);
    context.charge_node(&nested)?;
    let raw = required_field(input, name, field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error(&nested, "expected identifier string"))?;
    let value = RegistryResourceIdentifier::parse_java_string(raw)
        .map(RegistryResourceIdentifier::into_string)
        .ok_or_else(|| context.codec_error(&nested, "invalid identifier"))?;
    output.push(
        name,
        CanonicalRegistryCodecValue::new(NbtSummaryValue::String(value.into())),
    );
    Ok(())
}
fn push_vec3(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let nested = child(field, name);
    let source = required_field(input, name, field, context)?;
    let value = context.with_nested(&nested, |context| {
        decode_vec3_source(source, &nested, context)
    })?;
    output.push(name, value.canonical);
    Ok(())
}
fn push_float_provider(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    minimum: f32,
    maximum: f32,
) -> Result<(), RegistryCodecError> {
    let nested = child(field, name);
    let source = required_field(input, name, field, context)?;
    let value = context.with_nested(&nested, |context| {
        decode_float_provider_source(source, &nested, context, minimum, maximum)
    })?;
    output.push(name, value);
    Ok(())
}
fn push_sound_list(
    output: &mut RegistryCodecRecord,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let nested = child(field, "sound");
    let source = required_field(input, "sound", field, context)?;
    let value = context.with_nested(&nested, |context| {
        decode_compact_sounds(source, &nested, context)
    })?;
    output.push("sound", value);
    Ok(())
}
fn decode_compact_sounds(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let Some(len) = input.list_len() else {
        return decode_sound_event_holder_source(input, field, context);
    };
    if len > 255 {
        context.charge_node(field)?;
        context.check_collection(field, len)?;
        return Err(context.codec_error(field, "sound list length must be at most 255"));
    }
    decode_compact_list(input, field, context, decode_sound_event_holder_source)
}

#[cfg(test)]
mod tests;
