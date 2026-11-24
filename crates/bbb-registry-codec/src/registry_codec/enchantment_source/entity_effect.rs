//! Exact source owners shared by `EnchantmentEntityEffect.CODEC` and
//! `EnchantmentLocationBasedEffect.CODEC`.
//!
//! The concrete payload families live in disjoint leaf and world-mutation
//! modules. This facade owns their generated-catalog dispatch and the two
//! distinct recursive `all_of` codecs.

pub(super) mod leaf;
pub(super) mod world;

use crate::{
    enchantment_codec_types::{
        enchantment_entity_effect_codec_type, enchantment_location_based_effect_codec_type,
        EnchantmentEntityEffectCodecFamily, EnchantmentLocationBasedEffectCodecFamily,
    },
    registry_codec::{
        recursive_values::{
            decode_list as decode_effect_list, required_field, CanonicalRegistryCodecValue,
            RegistryCodecDecodeContext, RegistryCodecRecord,
        },
        RegistryCodecError, RegistryElementValueRef,
    },
};

use self::{
    leaf::{decode_entity_leaf_effect_source, decode_location_leaf_effect_source},
    world::{decode_entity_world_effect_source, decode_location_world_effect_source},
};
use super::value::{child, decode_dispatch_type};

/// Decodes the generated 15-family `EnchantmentEntityEffect.CODEC` catalog.
pub(super) fn decode_enchantment_entity_effect_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "enchantment entity effect")?;
    let (type_id, family) = decode_dispatch_type(
        input,
        field,
        context,
        "enchantment entity effect",
        |resource_id| {
            enchantment_entity_effect_codec_type(resource_id).map(|entry| entry.codec_family)
        },
    )?;
    if family == EnchantmentEntityEffectCodecFamily::AllOfEntityEffects {
        return decode_entity_all_of(type_id, input, field, context);
    }
    if let Some(decoded) =
        decode_entity_leaf_effect_source(family, type_id.clone(), input, field, context)
    {
        return decoded;
    }
    if let Some(decoded) = decode_entity_world_effect_source(family, type_id, input, field, context)
    {
        return decoded;
    }
    Err(context.codec_error(field, "generated entity-effect family has no exact owner"))
}

/// Decodes the generated 16-family `EnchantmentLocationBasedEffect.CODEC` catalog.
pub(super) fn decode_enchantment_location_based_effect_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "enchantment location effect")?;
    let (type_id, family) = decode_dispatch_type(
        input,
        field,
        context,
        "enchantment location effect",
        |resource_id| {
            enchantment_location_based_effect_codec_type(resource_id)
                .map(|entry| entry.codec_family)
        },
    )?;
    if family == EnchantmentLocationBasedEffectCodecFamily::AllOfLocationBasedEffects {
        return decode_location_all_of(type_id, input, field, context);
    }
    if let Some(decoded) =
        decode_location_leaf_effect_source(family, type_id.clone(), input, field, context)
    {
        return decoded;
    }
    if let Some(decoded) =
        decode_location_world_effect_source(family, type_id, input, field, context)
    {
        return decoded;
    }
    Err(context.codec_error(field, "generated location-effect family has no exact owner"))
}

fn decode_entity_all_of(
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let effects_field = child(field, "effects");
    let effects = required_field(input, "effects", field, context)?;
    let effects = context.with_nested(&effects_field, |context| {
        decode_effect_list(
            effects,
            &effects_field,
            context,
            decode_enchantment_entity_effect_source,
        )
    })?;
    let mut output = RegistryCodecRecord::default();
    output.push("effects", effects);
    Ok(output.finish_typed(type_id))
}

fn decode_location_all_of(
    type_id: String,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let effects_field = child(field, "effects");
    let effects = required_field(input, "effects", field, context)?;
    let effects = context.with_nested(&effects_field, |context| {
        decode_effect_list(
            effects,
            &effects_field,
            context,
            decode_enchantment_location_based_effect_source,
        )
    })?;
    let mut output = RegistryCodecRecord::default();
    output.push("effects", effects);
    Ok(output.finish_typed(type_id))
}

#[cfg(test)]
mod tests;
