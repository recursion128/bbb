//! Exact `LootItemCondition.DIRECT_CODEC` source owner and validation facts.
//!
//! The direct codec first tries the generated typed dispatch and then the
//! inline `all_of` list alternative.  Either source shape always writes back
//! through the primary typed codec, so canonical NBT is payload-first with the
//! normalized `condition` discriminator last (`LootItemCondition`,
//! `CompositeLootItemCondition`, DFU 9.0.19
//! `Codec.withAlternative`).
//!
//! Decoding deliberately retains Vanilla's `LootContextUser` facts separately
//! from registry-holder references.  `EnchantmentEffectComponents` validates
//! those facts with a resolver-free `ValidationContext`, where any predicate
//! registry reference is forbidden (`Validatable`).

mod predicate;
mod simple;

use crate::{
    codec_dispatch_types::{loot_condition_codec_type, LootConditionCodecFamily},
    loot_context_types::{VanillaLootContextParam, VanillaLootContextParamSet},
    registry_codec::{
        recursive_values::{
            required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext,
        },
        RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
    },
};
use bbb_protocol::{nbt_summary::entry, NbtSummaryValue};

use super::loot_context_value::{extend_context_params, LootContextCodecValue};

const CONDITION_FIELD: &str = "condition";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LootConditionCodecValue {
    pub(super) value: LootContextCodecValue,
    has_predicate_reference: bool,
}

impl LootConditionCodecValue {
    fn new(value: LootContextCodecValue) -> Self {
        Self {
            value,
            has_predicate_reference: false,
        }
    }

    /// Applies the resolver-free `ValidationContext` used by enchantment
    /// effect-component codecs.  Required-vs-optional is irrelevant here:
    /// Vanilla checks condition usage against `ContextKeySet.allowed()`.
    pub(super) fn validate_without_references(
        &self,
        param_set: &VanillaLootContextParamSet,
        field: &str,
        context: &RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        if self.has_predicate_reference {
            return Err(context.codec_error(
                field,
                "predicate reference is not allowed in this validation context",
            ));
        }
        let missing = self
            .value
            .referenced_context_params
            .iter()
            .copied()
            .filter(|param| !param_set.allows(*param))
            .map(VanillaLootContextParam::resource_id)
            .collect::<Vec<_>>();
        if missing.is_empty() {
            return Ok(());
        }
        Err(context.codec_error(
            field,
            format!(
                "loot context parameters are not provided: {}",
                missing.join(", ")
            ),
        ))
    }
}

pub(super) fn decode_loot_condition_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<LootConditionCodecValue, RegistryCodecError> {
    if input.list_len().is_some() {
        return decode_condition_list(input, field, context, "minecraft:all_of".to_string());
    }
    if let Some(decoded) = simple::decode_simple_condition_source(input, field, context) {
        return decoded.map(LootConditionCodecValue::new);
    }

    context.charge_record(input, field, "loot condition")?;
    let (type_id, family) = decode_condition_type(input, field, context)?;
    if let Some(decoded) =
        predicate::decode_predicate_condition_payload(family, &type_id, input, field, context)
    {
        return decoded.map(LootConditionCodecValue::new);
    }

    match family {
        LootConditionCodecFamily::InvertedLootItemCondition => {
            decode_inverted(input, field, context, type_id)
        }
        LootConditionCodecFamily::AnyOfCondition | LootConditionCodecFamily::AllOfCondition => {
            decode_composite(input, field, context, type_id)
        }
        LootConditionCodecFamily::ConditionReference => {
            decode_reference(input, field, context, type_id)
        }
        _ => Err(context.codec_error(
            &child(field, CONDITION_FIELD),
            "loot condition family has no exact owner",
        )),
    }
}

fn decode_inverted(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: String,
) -> Result<LootConditionCodecValue, RegistryCodecError> {
    let term_field = child(field, "term");
    let term = required_field(input, "term", field, context)?;
    let term = context.with_nested(&term_field, |context| {
        decode_loot_condition_source(term, &term_field, context)
    })?;
    let (value, references) = term.value.canonical.into_parts();
    let canonical = CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(vec![
            entry("term", value),
            entry(CONDITION_FIELD, NbtSummaryValue::String(type_id.into())),
        ]),
        references,
    );
    Ok(LootConditionCodecValue {
        value: LootContextCodecValue {
            canonical,
            referenced_context_params: term.value.referenced_context_params,
            semantic_constant_value_bits: None,
        },
        has_predicate_reference: term.has_predicate_reference,
    })
}

fn decode_composite(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: String,
) -> Result<LootConditionCodecValue, RegistryCodecError> {
    let terms_field = child(field, "terms");
    let terms = required_field(input, "terms", field, context)?;
    context.with_nested(&terms_field, |context| {
        decode_condition_list(terms, &terms_field, context, type_id)
    })
}

fn decode_condition_list(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: String,
) -> Result<LootConditionCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    let len = input
        .list_len()
        .ok_or_else(|| context.codec_error(field, "loot condition terms are not a list"))?;
    context.check_collection(field, len)?;
    let mut values = Vec::with_capacity(len);
    let mut references = Vec::new();
    let mut params = Vec::new();
    let mut has_predicate_reference = false;
    for index in 0..len {
        let term_field = format!("{field}[{index}]");
        let term = input
            .list_element(index)
            .expect("loot condition list length and elements stay consistent");
        let term = context.with_nested(&term_field, |context| {
            decode_loot_condition_source(term, &term_field, context)
        })?;
        let (value, nested_references) = term.value.canonical.into_parts();
        values.push(value);
        references.extend(nested_references);
        extend_context_params(&mut params, term.value.referenced_context_params);
        has_predicate_reference |= term.has_predicate_reference;
    }
    let canonical = CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(vec![
            entry("terms", NbtSummaryValue::List(values)),
            entry(CONDITION_FIELD, NbtSummaryValue::String(type_id.into())),
        ]),
        references,
    );
    Ok(LootConditionCodecValue {
        value: LootContextCodecValue {
            canonical,
            referenced_context_params: params,
            semantic_constant_value_bits: None,
        },
        has_predicate_reference,
    })
}

fn decode_reference(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    type_id: String,
) -> Result<LootConditionCodecValue, RegistryCodecError> {
    let name_field = child(field, "name");
    context.charge_node(&name_field)?;
    let raw = required_field(input, "name", field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error(&name_field, "predicate reference is not a string"))?;
    let name = RegistryResourceIdentifier::parse_java_string(raw)
        .ok_or_else(|| context.codec_error(&name_field, "invalid predicate reference id"))?;
    let canonical = CanonicalRegistryCodecValue::new(NbtSummaryValue::Compound(vec![
        entry("name", NbtSummaryValue::String(name.as_str().into())),
        entry(CONDITION_FIELD, NbtSummaryValue::String(type_id.into())),
    ]));
    Ok(LootConditionCodecValue {
        value: LootContextCodecValue::new(canonical),
        has_predicate_reference: true,
    })
}

fn decode_condition_type(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(String, LootConditionCodecFamily), RegistryCodecError> {
    let type_field = child(field, CONDITION_FIELD);
    context.charge_node(&type_field)?;
    let raw = required_field(input, CONDITION_FIELD, field, context)?
        .java_string_ref()
        .ok_or_else(|| context.codec_error(&type_field, "condition is not a string"))?;
    let type_id = RegistryResourceIdentifier::parse_java_string(raw)
        .ok_or_else(|| context.codec_error(&type_field, "invalid loot condition id"))?
        .into_string();
    let family = loot_condition_codec_type(&type_id)
        .map(|entry| entry.codec_family)
        .ok_or_else(|| context.codec_error(&type_field, "unknown loot condition id"))?;
    Ok((type_id, family))
}

pub(super) fn child(parent: &str, name: &str) -> String {
    format!("{parent}.{name}")
}

#[cfg(test)]
mod tests;
