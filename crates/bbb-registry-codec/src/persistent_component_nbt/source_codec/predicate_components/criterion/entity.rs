//! Source-aware criterion `EntityPredicate.CODEC` owner.
//!
//! Vanilla's root is recursive through vehicle/passenger/targeted predicates,
//! the Player `looking_at` subtype, and Lightning's `entity_struck` subtype.
//! Every edge keeps the caller's one decode context and recursive component
//! owner (`EntityPredicate`, `PlayerPredicate`).

use crate::registry_codec::{
    recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
    RegistryCodecError, RegistryElementValueRef,
};

use super::super::{
    child, decode_data_component_matchers, decode_i32_value, decode_string, RegistryCodecRecord,
};

pub(super) mod leaf;
mod subpredicate;

pub(crate) fn decode(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "entity predicate")?;
    let mut output = RegistryCodecRecord::default();

    macro_rules! push_optional {
        ($name:literal, $decoder:expr) => {
            if let Some(value) = input.field($name) {
                let nested = child(field, $name);
                let value =
                    context.with_nested(&nested, |context| $decoder(value, &nested, context))?;
                output.push($name, value);
            }
        };
    }

    push_optional!("type", leaf::decode_entity_type_predicate);
    push_optional!("distance", leaf::decode_distance_predicate);
    push_optional!("movement", leaf::decode_movement_predicate);
    for name in ["location", "stepping_on", "movement_affected_by"] {
        if let Some(value) = input.field(name) {
            let nested = child(field, name);
            let value = context.with_nested(&nested, |context| {
                super::location::decode(value, &nested, context)
            })?;
            output.push(name, value);
        }
    }
    push_optional!("effects", leaf::decode_mob_effects_predicate);
    push_optional!("nbt", leaf::decode_entity_nbt_predicate);
    push_optional!("flags", leaf::decode_entity_flags_predicate);
    if let Some(value) = input.field("equipment") {
        let nested = child(field, "equipment");
        let value = context.with_nested(&nested, |context| {
            leaf::decode_entity_equipment_predicate(value, &nested, context)
        })?;
        output.push("equipment", value);
    }
    if let Some(value) = input.field("type_specific") {
        let nested = child(field, "type_specific");
        let value = context.with_nested(&nested, |context| {
            subpredicate::decode(value, &nested, context)
        })?;
        output.push("type_specific", value);
    }
    if let Some(value) = input.field("periodic_tick") {
        let nested = child(field, "periodic_tick");
        let value = context.with_nested(&nested, |context| {
            let decoded = decode_i32_value(value, &nested, context)?;
            match decoded.network_nbt_unchecked() {
                bbb_protocol::NbtSummaryValue::Int(value) if *value > 0 => Ok(decoded),
                _ => Err(context.codec_error(&nested, "expected a positive integer")),
            }
        })?;
        output.push("periodic_tick", value);
    }
    for name in ["vehicle", "passenger", "targeted_entity"] {
        if let Some(value) = input.field(name) {
            let nested = child(field, name);
            let value = context.with_nested(&nested, |context| decode(value, &nested, context))?;
            output.push(name, value);
        }
    }
    if let Some(value) = input.field("team") {
        let nested = child(field, "team");
        let value = context.with_nested(&nested, |context| {
            decode_string(value, &nested, context, "team")
        })?;
        output.push("team", value);
    }
    if let Some(value) = input.field("slots") {
        let nested = child(field, "slots");
        let value = context.with_nested(&nested, |context| {
            leaf::decode_slots_predicate(value, &nested, context)
        })?;
        output.push("slots", value);
    }

    let matchers = decode_data_component_matchers(input, field, context)?;
    output.append(matchers);
    Ok(output.finish())
}

#[cfg(test)]
mod tests;
