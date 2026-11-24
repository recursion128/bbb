//! Number-provider constants and typed or untyped uniform records.
//!
//! The writer collapses constants to a float and writes other providers
//! payload-first with normalized `type` last

use crate::{
    codec_dispatch_types::{number_provider_codec_type, NumberProviderCodecFamily},
    loot_context_types::VanillaLootContextParam,
    registry_codec::{
        recursive_values::{
            required_field as required, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
            RegistryCodecRecord,
        },
        RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
    },
};
use bbb_facts::environment_attribute_types::{
    vanilla_environment_attribute, VanillaEnvironmentAttributeType,
};
use bbb_protocol::{
    nbt_summary::canonical_float_tag_bits, parse_command_nbt_path_value_prefix_utf16,
    NbtSummaryValue,
};

use super::{
    loot_context_value::{extend_context_params, LootContextCodecValue},
    scoreboard_name_provider::decode_scoreboard_name_provider_source,
    value::decode_level_based_value_source,
};

pub(super) fn decode_number_provider_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    if let Some(value) = input.codec_f32() {
        context.charge_node(field)?;
        return Ok(LootContextCodecValue::constant(float_value(value), value));
    }

    context.charge_record(input, field, "number provider")?;
    let Some(raw_type) = input.field("type") else {
        return decode_uniform(input, field, context, "minecraft:uniform".to_string());
    };
    context.charge_node("type")?;
    let raw_type = raw_type
        .java_string_ref()
        .ok_or_else(|| context.codec_error("type", "expected a dispatch identifier string"))?;
    let type_id = RegistryResourceIdentifier::parse_java_string(raw_type)
        .map(RegistryResourceIdentifier::into_string)
        .ok_or_else(|| context.codec_error("type", "invalid dispatch identifier"))?;
    let family = number_provider_codec_type(&type_id)
        .map(|entry| entry.codec_family)
        .ok_or_else(|| {
            context.codec_error("type", format!("unknown number provider type `{raw_type}`"))
        })?;
    match family {
        NumberProviderCodecFamily::ConstantValue => decode_typed_constant(input, field, context),
        NumberProviderCodecFamily::UniformGenerator => {
            decode_uniform(input, field, context, type_id)
        }
        NumberProviderCodecFamily::BinomialDistributionGenerator => {
            decode_binomial(input, field, context, type_id)
        }
        NumberProviderCodecFamily::ScoreboardValue => decode_score(input, field, context, type_id),
        NumberProviderCodecFamily::StorageValue => decode_storage(input, field, context, type_id),
        NumberProviderCodecFamily::Sum => decode_sum(input, field, context, type_id),
        NumberProviderCodecFamily::EnchantmentLevelProvider => {
            decode_enchantment_level(input, field, context, type_id)
        }
        NumberProviderCodecFamily::EnvironmentAttributeValue => {
            decode_environment_attribute(input, field, context, type_id)
        }
    }
}

fn decode_typed_constant(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let value = context.decode_f32(required(input, "value", field, context)?, "value")?;
    // `NumberProviders.CODEC` recognizes the semantic ConstantValue on write
    // and selects `ConstantValue.INLINE_CODEC`.
    Ok(LootContextCodecValue::constant(float_value(value), value))
}

fn decode_uniform(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: String,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let mut output = Output::default();
    output.push_number(input, "min", field, context)?;
    output.push_number(input, "max", field, context)?;
    output.finish(type_id)
}

fn decode_binomial(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: String,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let mut output = Output::default();
    output.push_number(input, "n", field, context)?;
    output.push_number(input, "p", field, context)?;
    output.finish(type_id)
}

fn decode_score(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: String,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let mut output = Output::default();
    let target = required(input, "target", field, context)?;
    let target = context.with_nested("target", |context| {
        decode_scoreboard_name_provider_source(target, "target", context)
    })?;
    output.push("target", target);
    output.push_string(input, "score", field, context)?;
    let scale = match input.field("scale").and_then(|value| value.codec_f32()) {
        Some(value) => {
            // Shape rejection belongs to `orElse`, but a valid scale still
            // owns its node: a safety-budget failure must not become default.
            context.charge_node("scale")?;
            value
        }
        None => 1.0,
    };
    output.record.push_value("scale", float_nbt(scale));
    output.finish(type_id)
}

fn decode_storage(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: String,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let mut output = Output::default();
    let storage = decode_identifier_field(input, "storage", field, context)?;
    output
        .record
        .push_value("storage", NbtSummaryValue::String(storage.into()));

    context.charge_node("path")?;
    let raw_path = required(input, "path", field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error("path", "expected an NBT path string"))?;
    // `NbtPath.CODEC` writes `asString()`: parsing stops at the first space and
    // does not require the reader cursor to reach the input end. A zero-node
    // path is consequently valid for an empty or initially-spaced string
    let (consumed, _) = parse_command_nbt_path_value_prefix_utf16(raw_path.units())
        .map_err(|_| context.codec_error("path", "invalid command NBT path"))?;
    output.record.push_value(
        "path",
        NbtSummaryValue::String(raw_path.units()[..consumed].to_vec().into()),
    );
    output.finish(type_id)
}

fn decode_sum(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: String,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let summands = required(input, "summands", field, context)?;
    let summands = context.with_nested("summands", |context| {
        decode_number_list(summands, "summands", context)
    })?;
    let mut output = Output::default();
    output.push("summands", summands);
    output.finish(type_id)
}

fn decode_enchantment_level(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: String,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let amount = required(input, "amount", field, context)?;
    let amount = context.with_nested("amount", |context| {
        decode_level_based_value_source(amount, "amount", context)
    })?;
    let mut output = Output::default();
    output.record.push("amount", amount);
    output.finish(type_id)
}

fn decode_environment_attribute(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: String,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    context.charge_node("attribute")?;
    let raw_attribute = required(input, "attribute", field, context)?
        .java_string_ref()
        .ok_or_else(|| {
            context.codec_error("attribute", "expected an environment attribute identifier")
        })?;
    let attribute_id = RegistryResourceIdentifier::parse_java_string(raw_attribute)
        .map(RegistryResourceIdentifier::into_string)
        .ok_or_else(|| context.codec_error("attribute", "invalid resource identifier"))?;
    let descriptor = vanilla_environment_attribute(&attribute_id).ok_or_else(|| {
        context.codec_error(
            "attribute",
            format!("unknown environment attribute `{raw_attribute}`"),
        )
    })?;
    if !matches!(
        descriptor.value_type,
        VanillaEnvironmentAttributeType::Float
            | VanillaEnvironmentAttributeType::AngleDegrees
            | VanillaEnvironmentAttributeType::Integer
    ) {
        return Err(context.codec_error(
            "attribute",
            format!("environment attribute `{attribute_id}` cannot be converted to a number"),
        ));
    }

    let mut output = Output::default();
    output
        .record
        .push_value("attribute", NbtSummaryValue::String(attribute_id.into()));
    if descriptor.positional {
        extend_context_params(
            &mut output.context_params,
            [VanillaLootContextParam::Origin],
        );
    }
    output.finish(type_id)
}

#[derive(Default)]
struct Output {
    record: RegistryCodecRecord,
    context_params: Vec<VanillaLootContextParam>,
}

impl Output {
    fn push(&mut self, name: &str, value: LootContextCodecValue) {
        self.record.push(name, value.canonical);
        extend_context_params(&mut self.context_params, value.referenced_context_params);
    }

    fn push_number(
        &mut self,
        input: RegistryElementValueRef<'_>,
        name: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let nested = required(input, name, field, context)?;
        let value = context.with_nested(name, |context| {
            decode_number_provider_source(nested, name, context)
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
        context.charge_node(name)?;
        let value = required(input, name, field, context)?
            .java_string_ref()
            .ok_or_else(|| context.codec_error(name, "expected Codec.STRING value"))?;
        self.record
            .push_value(name, NbtSummaryValue::String(value.clone()));
        Ok(())
    }

    fn finish(self, type_id: String) -> Result<LootContextCodecValue, RegistryCodecError> {
        let canonical = self.record.finish_typed(type_id);
        Ok(LootContextCodecValue {
            canonical,
            referenced_context_params: self.context_params,
            semantic_constant_value_bits: None,
        })
    }
}

fn decode_number_list(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let len = input
        .list_len()
        .ok_or_else(|| context.codec_error(field, "expected a number-provider list"))?;
    context.check_collection(field, len)?;
    let mut values = Vec::with_capacity(len);
    let mut references = Vec::new();
    let mut referenced_context_params = Vec::new();
    for index in 0..len {
        let item = input
            .list_element(index)
            .expect("number-provider list length and indexed values stay consistent");
        let decoded = context.with_nested(field, |context| {
            decode_number_provider_source(item, field, context)
        })?;
        let (canonical, nested_references) = decoded.canonical.into_parts();
        values.push(canonical);
        references.extend(nested_references);
        extend_context_params(
            &mut referenced_context_params,
            decoded.referenced_context_params,
        );
    }
    let canonical =
        CanonicalRegistryCodecValue::from_parts(NbtSummaryValue::List(values), references);
    Ok(LootContextCodecValue {
        canonical,
        referenced_context_params,
        semantic_constant_value_bits: None,
    })
}

fn decode_identifier_field(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<String, RegistryCodecError> {
    context.charge_node(name)?;
    let raw = required(input, name, field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error(name, "expected a resource identifier"))?;
    RegistryResourceIdentifier::parse_java_string(raw)
        .map(RegistryResourceIdentifier::into_string)
        .ok_or_else(|| context.codec_error(name, "invalid resource identifier"))
}

fn float_value(value: f32) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(float_nbt(value))
}

fn float_nbt(value: f32) -> NbtSummaryValue {
    NbtSummaryValue::Float(canonical_float_tag_bits(value))
}

#[cfg(test)]
mod tests;
