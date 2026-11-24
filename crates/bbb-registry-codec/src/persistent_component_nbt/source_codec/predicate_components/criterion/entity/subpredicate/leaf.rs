//! Non-recursive `EntitySubPredicate` MapCodec payload owners.
//!
//! Vanilla 26.1 sources: `FishingHookPredicate`,
//! `SlimePredicate`, `RaiderPredicate`, and

use crate::registry_codec::{
    recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef,
};

use super::super::super::super::{
    child, decode_bool_value, decode_int_bounds, RegistryCodecRecord,
};

pub(super) fn decode_fishing_hook(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "fishing hook predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(value) = input.field("in_open_water") {
        let nested = child(field, "in_open_water");
        let value = context.with_nested(&nested, |context| {
            decode_bool_value(value, &nested, context)
        })?;
        output.push("in_open_water", value);
    }
    Ok(output.finish())
}

pub(super) fn decode_slime(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "slime predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(value) = input.field("size") {
        let nested = child(field, "size");
        let value = context.with_nested(&nested, |context| {
            decode_int_bounds(value, &nested, context)
        })?;
        if !value.is_any {
            output.push("size", value.value);
        }
    }
    Ok(output.finish())
}

pub(super) fn decode_raider(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "raider predicate")?;
    let mut output = RegistryCodecRecord::default();
    for name in ["has_raid", "is_captain"] {
        if let Some(value) = input.field(name) {
            let nested = child(field, name);
            let value = context.with_nested(&nested, |context| {
                decode_bool_value(value, &nested, context)
            })?;
            if value.network_nbt_unchecked() != &bbb_protocol::NbtSummaryValue::Byte(0) {
                output.push(name, value);
            }
        }
    }
    Ok(output.finish())
}

pub(super) fn decode_sheep(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "sheep predicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(value) = input.field("sheared") {
        let nested = child(field, "sheared");
        let value = context.with_nested(&nested, |context| {
            decode_bool_value(value, &nested, context)
        })?;
        output.push("sheared", value);
    }
    Ok(output.finish())
}

#[cfg(test)]
mod tests;
