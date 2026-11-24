//! Exact source codecs for enchantment level values and value effects.
//!
//! `LevelBasedValue.CODEC` first accepts an inline `Codec.FLOAT`, otherwise it
//! dispatches the six map codecs registered by `LevelBasedValue.bootstrap`.
//! Recursive fields follow their record encounter order
//! `EnchantmentValueEffect.CODEC` dispatches six map codecs, and NbtOps writes
//! each payload before the normalized `type` key

use crate::{
    enchantment_codec_types::{
        enchantment_value_effect_codec_type, level_based_value_codec_type,
        EnchantmentValueEffectCodecFamily, LevelBasedValueCodecFamily,
    },
    registry_codec::{
        recursive_values::{
            decode_list, required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
            RegistryCodecRecord,
        },
        RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
    },
};
use bbb_protocol::{nbt_summary::canonical_float_tag_bits, NbtSummaryValue};

/// Decodes `LevelBasedValue.CODEC` from JSON or network NBT.
pub(super) fn decode_level_based_value_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if let Some(value) = input.codec_f32() {
        context.charge_node(field)?;
        return Ok(float_value(value));
    }

    context.charge_record(input, field, "level-based value")?;
    let (type_id, value_type) =
        decode_dispatch_type(input, field, context, "level-based value", |resource_id| {
            level_based_value_codec_type(resource_id).map(|entry| entry.codec_family)
        })?;
    let mut output = RegistryCodecRecord::default();
    match value_type {
        LevelBasedValueCodecFamily::Clamped => {
            output.push_recursive_level(input, "value", field, context)?;
            let min = decode_float_field(input, "min", field, context)?;
            let max = decode_float_field(input, "max", field, context)?;
            // Vanilla deliberately uses the primitive comparison. In
            // particular, either NaN makes this condition false.
            if max <= min {
                return Err(context.codec_error(
                    field,
                    format!("max must be larger than min, min: {min}, max: {max}"),
                ));
            }
            output.push("min", float_value(min));
            output.push("max", float_value(max));
        }
        LevelBasedValueCodecFamily::Fraction => {
            output.push_recursive_level(input, "numerator", field, context)?;
            output.push_recursive_level(input, "denominator", field, context)?;
        }
        LevelBasedValueCodecFamily::LevelsSquared => {
            output.push_float(input, "added", field, context)?;
        }
        LevelBasedValueCodecFamily::Linear => {
            output.push_float(input, "base", field, context)?;
            output.push_float(input, "per_level_above_first", field, context)?;
        }
        LevelBasedValueCodecFamily::Exponent => {
            output.push_recursive_level(input, "base", field, context)?;
            output.push_recursive_level(input, "power", field, context)?;
        }
        LevelBasedValueCodecFamily::Lookup => {
            let values = required_field(input, "values", field, context)?;
            let values = context.with_nested("values", |context| {
                decode_float_list(values, "values", context)
            })?;
            output.push("values", values);
            output.push_recursive_level(input, "fallback", field, context)?;
        }
    }
    Ok(output.finish_typed(type_id))
}

/// Decodes `EnchantmentValueEffect.CODEC` from JSON or network NBT.
pub(super) fn decode_enchantment_value_effect_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "enchantment value effect")?;
    let (type_id, effect_type) = decode_dispatch_type(
        input,
        field,
        context,
        "enchantment value effect",
        |resource_id| {
            enchantment_value_effect_codec_type(resource_id).map(|entry| entry.codec_family)
        },
    )?;
    let mut output = RegistryCodecRecord::default();
    match effect_type {
        EnchantmentValueEffectCodecFamily::AddValue
        | EnchantmentValueEffectCodecFamily::SetValue => {
            output.push_recursive_level(input, "value", field, context)?;
        }
        EnchantmentValueEffectCodecFamily::AllOfValueEffects => {
            let effects = required_field(input, "effects", field, context)?;
            let effects = context.with_nested("effects", |context| {
                decode_list(
                    effects,
                    "effects",
                    context,
                    decode_enchantment_value_effect_source,
                )
            })?;
            output.push("effects", effects);
        }
        EnchantmentValueEffectCodecFamily::MultiplyValue => {
            output.push_recursive_level(input, "factor", field, context)?;
        }
        EnchantmentValueEffectCodecFamily::RemoveBinomial => {
            output.push_recursive_level(input, "chance", field, context)?;
        }
        EnchantmentValueEffectCodecFamily::ScaleExponentially => {
            output.push_recursive_level(input, "base", field, context)?;
            output.push_recursive_level(input, "exponent", field, context)?;
        }
    }
    Ok(output.finish_typed(type_id))
}

impl RegistryCodecRecord {
    pub(super) fn push_float(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        self.push(
            name,
            float_value(decode_float_field(input, name, field, context)?),
        );
        Ok(())
    }

    pub(super) fn push_recursive_level(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested = required_field(input, name, field, context)?;
        let nested = context.with_nested(name, |context| {
            decode_level_based_value_source(nested, name, context)
        })?;
        self.push(name, nested);
        Ok(())
    }
}

fn decode_float_list(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let len = input
        .list_len()
        .ok_or_else(|| context.codec_error(field, "expected a Codec.FLOAT list"))?;
    context.check_collection(field, len)?;
    let mut values = Vec::with_capacity(len);
    for index in 0..len {
        let item = input
            .list_element(index)
            .expect("float list length and indexed values stay consistent");
        let value = context.with_nested(field, |context| context.decode_f32(item, field))?;
        values.push(NbtSummaryValue::Float(canonical_float_tag_bits(value)));
    }
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::List(
        values,
    )))
}

pub(super) fn decode_float_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<f32, RegistryCodecError> {
    let value = required_field(input, name, field, context)?;
    context.decode_f32(value, name)
}

pub(super) fn decode_dispatch_type<T: Copy>(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    kind: &str,
    resolve: impl FnOnce(&str) -> Option<T>,
) -> Result<(String, T), RegistryCodecError> {
    context.charge_node("type")?;
    let raw = required_field(input, "type", field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error("type", "expected a dispatch identifier string"))?;
    let normalized = RegistryResourceIdentifier::parse_java_string(raw)
        .ok_or_else(|| context.codec_error("type", "invalid dispatch identifier"))?
        .into_string();
    let family = resolve(&normalized)
        .ok_or_else(|| context.codec_error("type", format!("unknown {kind} type `{raw}`")))?;
    Ok((normalized, family))
}

pub(super) fn float_value(value: f32) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(NbtSummaryValue::Float(canonical_float_tag_bits(value)))
}

pub(super) fn child(parent: &str, name: &str) -> String {
    format!("{parent}.{name}")
}

#[cfg(test)]
mod tests;
