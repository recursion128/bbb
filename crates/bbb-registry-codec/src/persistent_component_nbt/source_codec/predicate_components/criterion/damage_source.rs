//! Source-aware criterion `DamageSourcePredicate.CODEC`.
//!
//! Vanilla 26.1 sources: `DamageSourcePredicate` and
//! `TagPredicate`. Damage-type tag keys are normalized resource
//! identifiers, not `#` holder sets, and therefore do not create frozen
//! registry references.

use crate::registry_codec::{
    recursive_values::{required_field, CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef,
};

use super::super::{
    child, decode_bool_value, decode_identifier_value, decode_list, RegistryCodecRecord,
};

/// Decodes `DamageSourcePredicate.CODEC` while delegating each direct Entity
/// predicate to the caller's one exact Entity owner and shared context.
pub(super) fn decode(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    mut decode_entity: impl FnMut(
        RegistryElementValueRef<'_>,
        &str,
        &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<CanonicalRegistryCodecValue, RegistryCodecError>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "damage source predicate")?;
    let mut output = RegistryCodecRecord::default();

    if let Some(tags) = input.field("tags") {
        let nested = child(field, "tags");
        let decoded = context.with_nested(&nested, |context| {
            decode_list(tags, &nested, context, decode_tag_predicate)
        })?;
        if !matches!(decoded.network_nbt_unchecked(), bbb_protocol::NbtSummaryValue::List(values) if values.is_empty())
        {
            output.push("tags", decoded);
        }
    }
    if let Some(entity) = input.field("direct_entity") {
        let nested = child(field, "direct_entity");
        let decoded =
            context.with_nested(&nested, |context| decode_entity(entity, &nested, context))?;
        output.push("direct_entity", decoded);
    }
    if let Some(entity) = input.field("source_entity") {
        let nested = child(field, "source_entity");
        let decoded =
            context.with_nested(&nested, |context| decode_entity(entity, &nested, context))?;
        output.push("source_entity", decoded);
    }
    if let Some(is_direct) = input.field("is_direct") {
        let nested = child(field, "is_direct");
        let decoded = context.with_nested(&nested, |context| {
            decode_bool_value(is_direct, &nested, context)
        })?;
        output.push("is_direct", decoded);
    }

    Ok(output.finish())
}

fn decode_tag_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "damage type tag predicate")?;
    let mut output = RegistryCodecRecord::default();

    let id_field = child(field, "id");
    let id = required_field(input, "id", field, context)?;
    let id = context.with_nested(&id_field, |context| {
        decode_identifier_value(id, &id_field, context)
    })?;
    output.push("id", id);

    let expected_field = child(field, "expected");
    let expected = required_field(input, "expected", field, context)?;
    let expected = context.with_nested(&expected_field, |context| {
        decode_bool_value(expected, &expected_field, context)
    })?;
    output.push("expected", expected);

    Ok(output.finish())
}

#[cfg(test)]
mod tests;
