//! Criterion- and environment-backed loot-condition payloads.

use crate::{
    codec_dispatch_types::LootConditionCodecFamily,
    loot_context_types::{vanilla_loot_entity_target, VanillaLootContextParam},
    persistent_component_nbt::source_codec::{
        decode_criterion_damage_source_predicate_source, decode_criterion_entity_predicate_source,
        decode_criterion_item_predicate_source, decode_criterion_location_predicate_source,
        decode_criterion_state_properties_predicate_source, decode_fixed_holder_reference_source,
    },
    registry_codec::{
        environment_source::decode_attribute_value_source,
        recursive_values::{required_field, RegistryCodecDecodeContext, RegistryCodecRecord},
        RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
    },
};
use bbb_facts::{
    block_state_types::vanilla_block_has_property,
    environment_attribute_types::vanilla_environment_attribute,
};
use bbb_protocol::NbtSummaryValue;

use super::child;
use crate::registry_codec::enchantment_source::loot_context_value::LootContextCodecValue;

const BLOCK_REGISTRY: &str = "minecraft:block";

pub(super) fn decode_predicate_condition_payload(
    family: LootConditionCodecFamily,
    type_id: &str,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Option<Result<LootContextCodecValue, RegistryCodecError>> {
    let result = match family {
        LootConditionCodecFamily::LootItemEntityPropertyCondition => {
            decode_entity_properties(input, field, context, type_id)
        }
        LootConditionCodecFamily::LootItemBlockStatePropertyCondition => {
            decode_block_state_property(input, field, context, type_id)
        }
        LootConditionCodecFamily::MatchTool => decode_match_tool(input, field, context, type_id),
        LootConditionCodecFamily::DamageSourceCondition => {
            decode_damage_source(input, field, context, type_id)
        }
        LootConditionCodecFamily::LocationCheck => decode_location(input, field, context, type_id),
        LootConditionCodecFamily::EnvironmentAttributeCheck => {
            decode_environment_attribute(input, field, context, type_id)
        }
        _ => return None,
    };
    Some(result)
}

fn decode_entity_properties(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: &str,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let mut output = Output::default();
    if let Some(predicate) = input.field("predicate") {
        let predicate_field = child(field, "predicate");
        let predicate = context.with_nested(&predicate_field, |context| {
            decode_criterion_entity_predicate_source(predicate, &predicate_field, context)
        })?;
        output.record.push("predicate", predicate);
    }
    let target_field = child(field, "entity");
    context.charge_node(&target_field)?;
    let raw_target = required_field(input, "entity", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error(&target_field, "loot entity target is not a string"))?;
    let target = vanilla_loot_entity_target(&raw_target)
        .ok_or_else(|| context.codec_error(&target_field, "unknown loot entity target"))?;
    output
        .record
        .push_value("entity", NbtSummaryValue::String(raw_target.into()));
    output.push_param(VanillaLootContextParam::Origin);
    output.push_param(target.context_param);
    Ok(output.finish(type_id))
}

fn decode_block_state_property(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: &str,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let mut output = Output::default();
    let block_field = child(field, "block");
    let block_input = required_field(input, "block", field, context)?;
    let raw_block = block_input
        .java_string_ref()
        .ok_or_else(|| context.codec_error(&block_field, "block holder is not an identifier"))?;
    let block = RegistryResourceIdentifier::parse_java_string(raw_block)
        .ok_or_else(|| context.codec_error(&block_field, "invalid block holder id"))?;
    let block_value = context.with_nested(&block_field, |context| {
        decode_fixed_holder_reference_source(block_input, &block_field, context, BLOCK_REGISTRY)
    })?;
    output.record.push("block", block_value);

    if let Some(properties) = input.field("properties") {
        validate_block_property_names(properties, block.as_str(), field, context)?;
        let properties_field = child(field, "properties");
        let properties = context.with_nested(&properties_field, |context| {
            decode_criterion_state_properties_predicate_source(
                properties,
                &properties_field,
                context,
            )
        })?;
        output.record.push("properties", properties);
    }
    output.push_param(VanillaLootContextParam::BlockState);
    Ok(output.finish(type_id))
}

fn validate_block_property_names(
    properties: RegistryElementValueRef<'_>,
    block: &str,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<(), RegistryCodecError> {
    let len = properties.map_len().ok_or_else(|| {
        context.codec_error(
            &child(field, "properties"),
            "block properties are not a map",
        )
    })?;
    let mut visible = Vec::with_capacity(len);
    for index in 0..len {
        let (name, _) = properties
            .map_entry_java_string_ref(index)
            .expect("block property map length and entries stay consistent");
        if !visible.contains(&name) {
            visible.push(name);
        }
    }
    for name in visible {
        if !name
            .with_utf8(|name| vanilla_block_has_property(block, name))
            .unwrap_or(false)
        {
            return Err(context.codec_error(
                &format!("{}.properties.{name}", field),
                format!("block {block} has no property `{name}`"),
            ));
        }
    }
    Ok(())
}

fn decode_match_tool(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: &str,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let mut output = Output::default();
    if let Some(predicate) = input.field("predicate") {
        let predicate_field = child(field, "predicate");
        let predicate = context.with_nested(&predicate_field, |context| {
            decode_criterion_item_predicate_source(predicate, &predicate_field, context)
        })?;
        output.record.push("predicate", predicate);
    }
    output.push_param(VanillaLootContextParam::Tool);
    Ok(output.finish(type_id))
}

fn decode_damage_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: &str,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let mut output = Output::default();
    if let Some(predicate) = input.field("predicate") {
        let predicate_field = child(field, "predicate");
        let predicate = context.with_nested(&predicate_field, |context| {
            decode_criterion_damage_source_predicate_source(predicate, &predicate_field, context)
        })?;
        output.record.push("predicate", predicate);
    }
    output.push_param(VanillaLootContextParam::Origin);
    output.push_param(VanillaLootContextParam::DamageSource);
    Ok(output.finish(type_id))
}

fn decode_location(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: &str,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let mut output = Output::default();
    if let Some(predicate) = input.field("predicate") {
        let predicate_field = child(field, "predicate");
        let predicate = context.with_nested(&predicate_field, |context| {
            decode_criterion_location_predicate_source(predicate, &predicate_field, context)
        })?;
        output.record.push("predicate", predicate);
    }
    for name in ["offsetX", "offsetY", "offsetZ"] {
        let Some(value) = input.field(name) else {
            continue;
        };
        let nested = child(field, name);
        context.charge_node(&nested)?;
        let value = value
            .codec_i32()
            .ok_or_else(|| context.codec_error(&nested, "location offset is not Codec.INT"))?;
        if value != 0 {
            output.record.push_value(name, NbtSummaryValue::Int(value));
        }
    }
    output.push_param(VanillaLootContextParam::Origin);
    Ok(output.finish(type_id))
}

fn decode_environment_attribute(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: &str,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let attribute_field = child(field, "attribute");
    context.charge_node(&attribute_field)?;
    let raw_attribute = required_field(input, "attribute", field, context)?
        .java_string_ref()
        .ok_or_else(|| {
            context.codec_error(&attribute_field, "environment attribute is not a string")
        })?;
    let attribute = RegistryResourceIdentifier::parse_java_string(raw_attribute)
        .ok_or_else(|| context.codec_error(&attribute_field, "invalid environment attribute id"))?;
    let descriptor = vanilla_environment_attribute(attribute.as_str())
        .ok_or_else(|| context.codec_error(&attribute_field, "unknown environment attribute id"))?;
    let value_field = child(field, "value");
    let value = required_field(input, "value", field, context)?;
    let value = context.with_nested(&value_field, |context| {
        decode_attribute_value_source(descriptor, value, &value_field, context)
    })?;
    let mut output = Output::default();
    // `dispatchMap` writes the selected payload before its discriminator.
    output.record.push("value", value);
    output.record.push_value(
        "attribute",
        NbtSummaryValue::String(attribute.as_str().into()),
    );
    if descriptor.positional {
        output.push_param(VanillaLootContextParam::Origin);
    }
    Ok(output.finish(type_id))
}

#[derive(Default)]
struct Output {
    record: RegistryCodecRecord,
    params: Vec<VanillaLootContextParam>,
}

impl Output {
    fn push_param(&mut self, param: VanillaLootContextParam) {
        if !self.params.contains(&param) {
            self.params.push(param);
        }
    }

    fn finish(mut self, type_id: &str) -> LootContextCodecValue {
        self.record
            .push_value("condition", NbtSummaryValue::String(type_id.into()));
        let canonical = self.record.finish();
        LootContextCodecValue {
            canonical,
            referenced_context_params: self.params,
            semantic_constant_value_bits: None,
        }
    }
}

#[cfg(test)]
mod tests;
