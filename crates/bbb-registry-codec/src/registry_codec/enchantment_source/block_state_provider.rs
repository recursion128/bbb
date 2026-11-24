//! Exact `BlockStateProvider.CODEC` source owner.
//!
//! The generated dispatch catalog is exhaustive for Vanilla 26.1's eight
//! provider registrations (`BlockStateProvider`,
//! `BlockStateProviderType`). Each `MapCodec` writes its payload in
//! `RecordCodecBuilder` order and DFU's dispatch appends the normalized `type`.

use crate::{
    codec_dispatch_types::{block_state_provider_codec_type, BlockStateProviderCodecFamily},
    registry_codec::{
        block_state_source::{decode_block_default_state_source, decode_block_state_source},
        recursive_values::{
            decode_list, required_field as required, CanonicalRegistryCodecValue,
            RegistryCodecDecodeContext, RegistryCodecRecord,
        },
        value_provider_source::decode_int_provider_source,
        RegistryCodecError, RegistryElementValueRef,
    },
};
use bbb_protocol::{
    nbt_summary::{canonical_double_tag_bits, canonical_float_tag_bits, entry},
    NbtSummaryValue,
};

use super::block_predicate::decode_block_predicate_source;
use super::value::{child, decode_dispatch_type};

/// Decodes `BlockStateProvider.CODEC` from server JSON, known-pack JSON, or NBT.
pub(crate) fn decode_block_state_provider_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "block state provider")?;
    let (type_id, family) = decode_dispatch_type(
        input,
        field,
        context,
        "block state provider",
        |resource_id| block_state_provider_codec_type(resource_id).map(|entry| entry.codec_family),
    )?;
    let mut output = RegistryCodecRecord::default();
    match family {
        BlockStateProviderCodecFamily::SimpleStateProvider => {
            output.push_state(input, "state", field, context)?
        }
        BlockStateProviderCodecFamily::WeightedStateProvider => {
            output.push_weighted_states(input, field, context)?
        }
        BlockStateProviderCodecFamily::NoiseThresholdProvider => {
            output.push_noise_base(input, field, context)?;
            output.push_float_inclusive(input, "threshold", field, context, -1.0, 1.0)?;
            output.push_float_inclusive(input, "high_chance", field, context, 0.0, 1.0)?;
            output.push_state(input, "default_state", field, context)?;
            output.push_nonempty_states(input, "low_states", field, context)?;
            output.push_nonempty_states(input, "high_states", field, context)?;
        }
        BlockStateProviderCodecFamily::NoiseProvider => {
            output.push_noise_base(input, field, context)?;
            output.push_nonempty_states(input, "states", field, context)?;
        }
        BlockStateProviderCodecFamily::DualNoiseProvider => {
            output.push_variety(input, field, context)?;
            output.push_noise_parameters(input, "slow_noise", field, context)?;
            output.push_positive_float(input, "slow_scale", field, context)?;
            output.push_noise_base(input, field, context)?;
            output.push_nonempty_states(input, "states", field, context)?;
        }
        BlockStateProviderCodecFamily::RotatedBlockProvider => {
            output.push_default_state(input, "state", field, context)?
        }
        BlockStateProviderCodecFamily::RandomizedIntStateProvider => {
            output.push_provider(input, "source", field, context)?;
            output.push_string(input, "property", field, context)?;
            output.push_int_provider(input, "values", field, context)?;
        }
        BlockStateProviderCodecFamily::RuleBasedStateProvider => {
            output.push_optional_provider(input, "fallback", field, context)?;
            output.push_rules(input, field, context)?;
        }
    }
    Ok(output.finish_typed(type_id))
}

impl RegistryCodecRecord {
    fn push_state(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested_field = child(field, name);
        let source = required(input, name, field, context)?;
        let value = context.with_nested(&nested_field, |context| {
            decode_block_state_source(source, &nested_field, context)
        })?;
        self.push(name, value);
        Ok(())
    }
    fn push_default_state(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested_field = child(field, name);
        let source = required(input, name, field, context)?;
        let value = context.with_nested(&nested_field, |context| {
            decode_block_default_state_source(source, &nested_field, context)
        })?;
        self.push(name, value);
        Ok(())
    }
    fn push_provider(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested_field = child(field, name);
        let source = required(input, name, field, context)?;
        let value = context.with_nested(&nested_field, |context| {
            decode_block_state_provider_source(source, &nested_field, context)
        })?;
        self.push(name, value);
        Ok(())
    }
    fn push_optional_provider(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        if let Some(source) = input.field(name) {
            self.push_provider_value(name, source, field, context)?;
        }
        Ok(())
    }
    fn push_provider_value(
        &mut self,
        name: &str,
        source: RegistryElementValueRef<'_>,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested_field = child(field, name);
        let value = context.with_nested(&nested_field, |context| {
            decode_block_state_provider_source(source, &nested_field, context)
        })?;
        self.push(name, value);
        Ok(())
    }
    fn push_string(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested = child(field, name);
        context.charge_node(&nested)?;
        let value = required(input, name, field, context)?
            .java_string_ref()
            .ok_or_else(|| context.codec_error(&nested, "expected Codec.STRING value"))?;
        self.push_value(name, NbtSummaryValue::String(value.clone()));
        Ok(())
    }
    fn push_int_provider(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested = child(field, name);
        let source = required(input, name, field, context)?;
        let value = context.with_nested(&nested, |context| {
            decode_int_provider_source(source, &nested, context, i32::MIN, i32::MAX)
        })?;
        self.push(name, value);
        Ok(())
    }
    fn push_noise_base(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        self.push_i64(input, "seed", field, context)?;
        self.push_noise_parameters(input, "noise", field, context)?;
        self.push_positive_float(input, "scale", field, context)
    }
    fn push_i64(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested = child(field, name);
        context.charge_node(&nested)?;
        let value = required(input, name, field, context)?
            .codec_i64()
            .ok_or_else(|| context.codec_error(&nested, "expected Codec.LONG value"))?;
        self.push_value(name, NbtSummaryValue::Long(value));
        Ok(())
    }
    fn push_positive_float(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let value = float_field(input, name, field, context)?;
        if !value.is_finite() || value.total_cmp(&0.0).is_le() {
            return Err(context.codec_error(&child(field, name), "expected positive finite float"));
        }
        self.push_value(name, float(value));
        Ok(())
    }
    fn push_float_inclusive(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
        minimum: f32,
        maximum: f32,
    ) -> Result<(), RegistryCodecError> {
        let value = float_field(input, name, field, context)?;
        // `Codec.floatRange` uses boxed `Float.compareTo`: NaN and infinities
        // fail; negative zero sorts before positive zero for the [0, 1] case.
        if !value.is_finite()
            || value.total_cmp(&minimum).is_lt()
            || value.total_cmp(&maximum).is_gt()
        {
            return Err(
                context.codec_error(&child(field, name), "float is outside the codec range")
            );
        }
        self.push_value(name, float(value));
        Ok(())
    }
    fn push_noise_parameters(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested = child(field, name);
        let source = required(input, name, field, context)?;
        let value = context.with_nested(&nested, |context| {
            decode_noise_parameters(source, &nested, context)
        })?;
        self.push(name, value);
        Ok(())
    }
    fn push_nonempty_states(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested = child(field, name);
        let source = required(input, name, field, context)?;
        let value = context.with_nested(&nested, |context| {
            decode_state_list(source, &nested, context, true)
        })?;
        self.push(name, value);
        Ok(())
    }
    fn push_weighted_states(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested = child(field, "entries");
        let source = required(input, "entries", field, context)?;
        let value = context.with_nested(&nested, |context| {
            decode_weighted_states(source, &nested, context)
        })?;
        self.push("entries", value);
        Ok(())
    }
    fn push_variety(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested = child(field, "variety");
        let source = required(input, "variety", field, context)?;
        let value =
            context.with_nested(&nested, |context| decode_variety(source, &nested, context))?;
        self.push("variety", value);
        Ok(())
    }
    fn push_rules(
        &mut self,
        input: RegistryElementValueRef<'_>,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested = child(field, "rules");
        let source = required(input, "rules", field, context)?;
        let value =
            context.with_nested(&nested, |context| decode_rules(source, &nested, context))?;
        self.push("rules", value);
        Ok(())
    }
}

fn decode_state_list(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    nonempty: bool,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let decoded = decode_list(input, field, context, decode_block_state_source)?;
    if nonempty
        && matches!(decoded.network_nbt_unchecked(), NbtSummaryValue::List(values) if values.is_empty())
    {
        return Err(context.codec_error(field, "expected non-empty block-state list"));
    }
    Ok(decoded)
}

fn decode_weighted_states(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let len = input
        .list_len()
        .ok_or_else(|| context.codec_error(field, "expected weighted entry list"))?;
    context.check_collection(field, len)?;
    if len == 0 {
        return Err(context.codec_error(field, "expected non-empty weighted entry list"));
    }
    let mut values = Vec::with_capacity(len);
    let mut refs = Vec::new();
    let mut sum = 0_i64;
    for index in 0..len {
        let item_field = format!("{field}[{index}]");
        let item = input
            .list_element(index)
            .expect("indexed list stays present");
        let value = context.with_nested(&item_field, |context| {
            context.charge_record(item, &item_field, "weighted entry")?;
            let data_field = child(&item_field, "data");
            let data = required(item, "data", &item_field, context)?;
            let data = context.with_nested(&data_field, |context| {
                decode_block_state_source(data, &data_field, context)
            })?;
            let weight_field = child(&item_field, "weight");
            context.charge_node(&weight_field)?;
            let weight = required(item, "weight", &item_field, context)?
                .codec_i32()
                .ok_or_else(|| context.codec_error(&weight_field, "expected Codec.INT weight"))?;
            if weight < 0 {
                return Err(context.codec_error(&weight_field, "weight must be non-negative"));
            }
            sum += i64::from(weight);
            if sum > i64::from(i32::MAX) {
                return Err(context.codec_error(field, "total weight exceeds i32"));
            }
            let (data, nested_refs) = data.into_parts();
            Ok(CanonicalRegistryCodecValue::from_parts(
                NbtSummaryValue::Compound(vec![
                    entry("data", data),
                    entry("weight", NbtSummaryValue::Int(weight)),
                ]),
                nested_refs,
            ))
        })?;
        let (value, nested_refs) = value.into_parts();
        values.push(value);
        refs.extend(nested_refs);
    }
    if sum == 0 {
        return Err(context.codec_error(field, "weighted entries have zero total weight"));
    }
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::List(values),
        refs,
    ))
}

fn decode_noise_parameters(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "noise parameters")?;
    let octave_field = child(field, "firstOctave");
    context.charge_node(&octave_field)?;
    let octave = required(input, "firstOctave", field, context)?
        .codec_i32()
        .ok_or_else(|| context.codec_error(&octave_field, "expected Codec.INT firstOctave"))?;
    let amplitudes_field = child(field, "amplitudes");
    let amplitudes = required(input, "amplitudes", field, context)?;
    let amplitudes = context.with_nested(&amplitudes_field, |context| {
        context.charge_node(&amplitudes_field)?;
        let len = amplitudes
            .list_len()
            .ok_or_else(|| context.codec_error(&amplitudes_field, "expected Codec.DOUBLE list"))?;
        context.check_collection(&amplitudes_field, len)?;
        let mut values = Vec::with_capacity(len);
        for index in 0..len {
            let item_field = format!("{amplitudes_field}[{index}]");
            context.charge_node(&item_field)?;
            let value = amplitudes
                .list_element(index)
                .expect("indexed list stays present")
                .codec_f64()
                .ok_or_else(|| context.codec_error(&item_field, "expected Codec.DOUBLE value"))?;
            values.push(NbtSummaryValue::Double(canonical_double_tag_bits(value)));
        }
        Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::List(
            values,
        )))
    })?;
    let (amplitudes, references) = amplitudes.into_parts();
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(vec![
            entry("firstOctave", NbtSummaryValue::Int(octave)),
            entry("amplitudes", amplitudes),
        ]),
        references,
    ))
}

fn decode_variety(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if let Some(value) = input.codec_i32() {
        context.charge_node(field)?;
        return range_value(value, value, field, context);
    }
    if let Some(len) = input.list_len() {
        context.charge_node(field)?;
        context.check_collection(field, len)?;
        if len != 2 {
            return Err(context.codec_error(field, "expected exactly two range values"));
        }
        let min = range_int(
            input.list_element(0).expect("indexed list stays present"),
            &format!("{field}[0]"),
            context,
        )?;
        let max = range_int(
            input.list_element(1).expect("indexed list stays present"),
            &format!("{field}[1]"),
            context,
        )?;
        return range_value(min, max, field, context);
    }
    context.charge_record(input, field, "inclusive range")?;
    let min = range_int(
        required(input, "min_inclusive", field, context)?,
        &child(field, "min_inclusive"),
        context,
    )?;
    let max = range_int(
        required(input, "max_inclusive", field, context)?,
        &child(field, "max_inclusive"),
        context,
    )?;
    range_value(min, max, field, context)
}

fn range_int(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<i32, RegistryCodecError> {
    context.charge_node(field)?;
    input
        .codec_i32()
        .ok_or_else(|| context.codec_error(field, "expected Codec.INT range value"))
}
fn range_value(
    min: i32,
    max: i32,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if !(1..=64).contains(&min) || !(1..=64).contains(&max) || min > max {
        return Err(context.codec_error(field, "variety range must be within 1..=64"));
    }
    Ok(CanonicalRegistryCodecValue::new(if min == max {
        NbtSummaryValue::Int(min)
    } else {
        NbtSummaryValue::List(vec![NbtSummaryValue::Int(min), NbtSummaryValue::Int(max)])
    }))
}

fn decode_rules(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let len = input
        .list_len()
        .ok_or_else(|| context.codec_error(field, "expected rules list"))?;
    context.check_collection(field, len)?;
    let mut values = Vec::with_capacity(len);
    let mut references = Vec::new();
    for index in 0..len {
        let item_field = format!("{field}[{index}]");
        let item = input
            .list_element(index)
            .expect("indexed list stays present");
        let rule = context.with_nested(&item_field, |context| {
            context.charge_record(item, &item_field, "rule")?;
            let predicate_field = child(&item_field, "if_true");
            let predicate = required(item, "if_true", &item_field, context)?;
            let predicate = context.with_nested(&predicate_field, |context| {
                decode_block_predicate_source(predicate, &predicate_field, context)
            })?;
            let then_field = child(&item_field, "then");
            let then = required(item, "then", &item_field, context)?;
            let then = context.with_nested(&then_field, |context| {
                decode_block_state_provider_source(then, &then_field, context)
            })?;
            let (predicate, mut refs) = predicate.into_parts();
            let (then, then_refs) = then.into_parts();
            refs.extend(then_refs);
            Ok(CanonicalRegistryCodecValue::from_parts(
                NbtSummaryValue::Compound(vec![entry("if_true", predicate), entry("then", then)]),
                refs,
            ))
        })?;
        let (rule, refs) = rule.into_parts();
        values.push(rule);
        references.extend(refs);
    }
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::List(values),
        references,
    ))
}

fn float_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<f32, RegistryCodecError> {
    let nested = child(field, name);
    context.charge_node(&nested)?;
    required(input, name, field, context)?
        .codec_f32()
        .ok_or_else(|| context.codec_error(&nested, "expected Codec.FLOAT value"))
}
fn float(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(canonical_float_tag_bits(value))
}

#[cfg(test)]
mod tests;
