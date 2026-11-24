//! Exact `IntRange.CODEC` source owner (`94-97`).

use crate::registry_codec::{
    recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef,
};
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};

use super::{
    loot_context_value::{extend_context_params, LootContextCodecValue},
    number_provider::decode_number_provider_source,
};

/// Decodes `IntRange.CODEC` without reconstructing ConstantValue semantics
/// from the lossy canonical NBT projection.
pub(super) fn decode_int_range_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    if let Some(value) = input.codec_i32() {
        context.charge_node(field)?;
        // `IntRange.exact` constructs `ConstantValue.exactly(int)`, so the
        // integer crosses a Java float before `unpackExact` casts it back.
        return Ok(scalar_range(java_float_to_i32(value as f32)));
    }
    context.charge_record(input, field, "IntRange")?;
    let min = decode_optional_number(input, "min", field, context)?;
    let max = decode_optional_number(input, "max", field, context)?;
    if let (Some(min), Some(max)) = (&min, &max) {
        if constants_equal(min, max) && constant_is_integral(min) {
            return Ok(scalar_range(java_float_to_i32(f32::from_bits(
                min.semantic_constant_value_bits()
                    .expect("checked constant"),
            ))));
        }
    }
    let mut fields = Vec::new();
    let mut references = Vec::new();
    let mut referenced_context_params = Vec::new();
    for (name, value) in [("min", min), ("max", max)] {
        let Some(value) = value else { continue };
        let (canonical, nested_references) = value.canonical.into_parts();
        fields.push(entry(name, canonical));
        references.extend(nested_references);
        extend_context_params(
            &mut referenced_context_params,
            value.referenced_context_params,
        );
    }
    let canonical =
        CanonicalRegistryCodecValue::from_parts(NbtSummaryValue::Compound(fields), references);
    Ok(LootContextCodecValue {
        canonical,
        referenced_context_params,
        semantic_constant_value_bits: None,
    })
}

fn scalar_range(value: i32) -> LootContextCodecValue {
    LootContextCodecValue::new(CanonicalRegistryCodecValue::new(NbtSummaryValue::Int(
        value,
    )))
}

fn decode_optional_number(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<Option<LootContextCodecValue>, RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(None);
    };
    let nested = format!("{field}.{name}");
    context
        .with_nested(&nested, |context| {
            decode_number_provider_source(value, &nested, context)
        })
        .map(Some)
}

fn constants_equal(left: &LootContextCodecValue, right: &LootContextCodecValue) -> bool {
    let (Some(left), Some(right)) = (
        left.semantic_constant_value_bits(),
        right.semantic_constant_value_bits(),
    ) else {
        return false;
    };
    let (left, right) = (f32::from_bits(left), f32::from_bits(right));
    (left.is_nan() && right.is_nan()) || left.to_bits() == right.to_bits()
}

fn constant_is_integral(value: &LootContextCodecValue) -> bool {
    let value = f32::from_bits(
        value
            .semantic_constant_value_bits()
            .expect("checked constant"),
    );
    value.is_infinite() || (!value.is_nan() && (value as f64).floor() == value as f64)
}

fn java_float_to_i32(value: f32) -> i32 {
    if value.is_nan() {
        0
    } else if value >= i32::MAX as f32 {
        i32::MAX
    } else if value <= i32::MIN as f32 {
        i32::MIN
    } else {
        value as i32
    }
}

#[cfg(test)]
mod tests;
