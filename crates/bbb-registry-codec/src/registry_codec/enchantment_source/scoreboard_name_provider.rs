//! Exact `ScoreboardNameProviders.CODEC` source owner.
//!
//! The codec tries the inline entity-target alternative before the two-family
//! typed dispatch. Its encoder collapses every context provider back to the
//! inline target string, while a fixed provider remains a payload-first map

use crate::{
    codec_dispatch_types::{
        scoreboard_name_provider_codec_type, ScoreboardNameProviderCodecFamily,
    },
    loot_context_types::vanilla_loot_entity_target,
    registry_codec::{
        recursive_values::{
            required_field as required, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
        },
        RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
    },
};
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};

use super::loot_context_value::LootContextCodecValue;

pub(super) fn decode_scoreboard_name_provider_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    if let Some(raw_target) = input.utf8_string() {
        context.charge_node(field)?;
        return decode_context_target(&raw_target, field, context);
    }

    context.charge_record(input, field, "scoreboard name provider")?;
    context.charge_node("type")?;
    let raw_type = required(input, "type", field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error("type", "expected a dispatch identifier string"))?;
    let type_id = RegistryResourceIdentifier::parse_java_string(raw_type)
        .map(RegistryResourceIdentifier::into_string)
        .ok_or_else(|| context.codec_error("type", "invalid dispatch identifier"))?;
    let family = scoreboard_name_provider_codec_type(&type_id)
        .map(|entry| entry.codec_family)
        .ok_or_else(|| {
            context.codec_error(
                "type",
                format!("unknown scoreboard name provider type `{raw_type}`"),
            )
        })?;

    match family {
        ScoreboardNameProviderCodecFamily::FixedScoreboardNameProvider => {
            context.charge_node("name")?;
            let name = required(input, "name", field, context)?
                .java_string_ref()
                .ok_or_else(|| context.codec_error("name", "expected Codec.STRING value"))?;
            Ok(LootContextCodecValue::new(
                CanonicalRegistryCodecValue::new(NbtSummaryValue::Compound(vec![
                    entry("name", NbtSummaryValue::String(name.clone())),
                    entry("type", NbtSummaryValue::String(type_id.into())),
                ])),
            ))
        }
        ScoreboardNameProviderCodecFamily::ContextScoreboardNameProvider => {
            context.charge_node("target")?;
            let raw_target = required(input, "target", field, context)?
                .utf8_string()
                .ok_or_else(|| context.codec_error("target", "expected an entity target string"))?;
            // The outer `Either` encoder recognizes the semantic context
            // provider and writes its inline alternative, not this typed map.
            decode_context_target(&raw_target, "target", context)
        }
    }
}

fn decode_context_target(
    raw_target: &str,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<LootContextCodecValue, RegistryCodecError> {
    let target = vanilla_loot_entity_target(raw_target).ok_or_else(|| {
        context.codec_error(field, format!("unknown entity target `{raw_target}`"))
    })?;
    Ok(LootContextCodecValue {
        canonical: CanonicalRegistryCodecValue::new(NbtSummaryValue::String(
            target.serialized_name.into(),
        )),
        referenced_context_params: vec![target.context_param],
        semantic_constant_value_bits: None,
    })
}

#[cfg(test)]
mod tests;
