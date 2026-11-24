//! Non-recursive source-aware leaves shared by `EntityPredicate.CODEC` and
//! `PlayerPredicate.CODEC`.
//!
//! This module deliberately does not own the recursive entity root or the six
//! `EntitySubPredicate` dispatch families.  It reuses the existing criterion
//! bounds, item-predicate, holder, and flattened-NBT owners so the enclosing
//! SCC can keep one decode context and one component recursion owner.

use std::collections::HashSet;

use crate::{
    persistent_component_nbt::source_codec::decode_fixed_holder_keyed_map_source,
    registry_codec::{
        recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
        source_nbt::decode_lenient_flattened_compound_source,
        RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
    },
};
use bbb_facts::stat_types::vanilla_stat_type_by_resource_id;
use bbb_protocol::packets::commands::VANILLA_COMMAND_SLOTS;

use super::super::super::super::holders::{
    decode_fixed_holder_reference, decode_homogeneous_fixed_holder_set,
};
use super::super::super::{
    child, decode_bool_value, decode_double_bounds, decode_identifier_value, decode_int_bounds,
    decode_item_predicate, decode_list, decode_named_value, RegistryCodecRecord,
};

const ENTITY_TYPE_REGISTRY: &str = "minecraft:entity_type";
const MOB_EFFECT_REGISTRY: &str = "minecraft:mob_effect";

/// A leaf value paired only with the equality fact needed by an enclosing
/// `optionalFieldOf(..., default)` writer. This is not a second semantic tree.
pub(super) struct DecodedDefaultLeaf {
    pub(super) value: CanonicalRegistryCodecValue,
    pub(super) is_default: bool,
}

pub(super) fn decode_entity_type_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_homogeneous_fixed_holder_set(input, field, context, ENTITY_TYPE_REGISTRY, false)
}

pub(super) fn decode_distance_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_double_bounds_record(
        input,
        field,
        context,
        "distance predicate",
        &["x", "y", "z", "horizontal", "absolute"],
    )
}

pub(super) fn decode_movement_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_double_bounds_record(
        input,
        field,
        context,
        "movement predicate",
        &[
            "x",
            "y",
            "z",
            "speed",
            "horizontal_speed",
            "vertical_speed",
            "fall_distance",
        ],
    )
}

pub(super) fn decode_mob_effects_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_fixed_holder_keyed_map_source(
        input,
        field,
        context,
        MOB_EFFECT_REGISTRY,
        decode_mob_effect_instance_predicate,
    )
}

pub(super) fn decode_entity_nbt_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_lenient_flattened_compound_source(input, field, context)
}

pub(super) fn decode_entity_flags_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "entity flags predicate")?;
    let mut output = RegistryCodecRecord::default();
    for name in [
        "is_on_ground",
        "is_on_fire",
        "is_sneaking",
        "is_sprinting",
        "is_swimming",
        "is_flying",
        "is_baby",
        "is_in_water",
        "is_fall_flying",
    ] {
        if let Some(value) = input.field(name) {
            let nested = child(field, name);
            let value = context.with_nested(&nested, |context| {
                decode_bool_value(value, &nested, context)
            })?;
            output.push(name, value);
        }
    }
    Ok(output.finish())
}

pub(super) fn decode_entity_equipment_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "entity equipment predicate")?;
    let mut output = RegistryCodecRecord::default();
    for name in [
        "head", "chest", "legs", "feet", "body", "mainhand", "offhand",
    ] {
        if let Some(value) = input.field(name) {
            let nested = child(field, name);
            let value = context.with_nested(&nested, |context| {
                decode_item_predicate(value, &nested, context)
            })?;
            output.push(name, value);
        }
    }
    Ok(output.finish())
}

pub(super) fn decode_slots_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let len = context.charge_record(input, field, "slot predicate map")?;
    let entries = input.visible_map_entries(len);
    let mut output = RegistryCodecRecord::default();
    for (name, value) in entries {
        if !VANILLA_COMMAND_SLOTS
            .iter()
            .any(|(candidate, _single)| name == *candidate)
        {
            return Err(
                context.codec_error(&child(field, name), format!("unknown slot range {name}"))
            );
        }
        let nested = child(field, name);
        let value = context.with_nested(&nested, |context| {
            decode_item_predicate(value, &nested, context)
        })?;
        output.push(name, value);
    }
    Ok(output.finish())
}

pub(super) fn decode_food_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<DecodedDefaultLeaf, RegistryCodecError> {
    context.charge_record(input, field, "food predicate")?;
    let mut output = RegistryCodecRecord::default();
    let mut is_default = true;
    if let Some(level) = input.field("level") {
        let nested = child(field, "level");
        let level = context.with_nested(&nested, |context| {
            decode_int_bounds(level, &nested, context)
        })?;
        if !level.is_any {
            is_default = false;
            output.push("level", level.value);
        }
    }
    if let Some(saturation) = input.field("saturation") {
        let nested = child(field, "saturation");
        let saturation = context.with_nested(&nested, |context| {
            decode_double_bounds(saturation, &nested, context)
        })?;
        if !saturation.is_any {
            is_default = false;
            output.push("saturation", saturation.value);
        }
    }
    Ok(DecodedDefaultLeaf {
        value: output.finish(),
        is_default,
    })
}

pub(super) fn decode_game_type_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<DecodedDefaultLeaf, RegistryCodecError> {
    let value = decode_list(input, field, context, |value, item_field, context| {
        decode_named_value(value, item_field, context, "game type", |name| {
            matches!(name, "survival" | "creative" | "adventure" | "spectator")
        })
    })?;
    let is_default = match value.network_nbt_unchecked() {
        bbb_protocol::NbtSummaryValue::List(values) => values
            .iter()
            .zip(["survival", "creative", "adventure", "spectator"])
            .all(|(value, expected)| {
                matches!(value, bbb_protocol::NbtSummaryValue::String(name) if name == expected)
            })
            && values.len() == 4,
        _ => false,
    };
    Ok(DecodedDefaultLeaf { value, is_default })
}

pub(super) fn decode_player_stats(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_list(input, field, context, decode_stat_matcher)
}

pub(super) fn decode_player_recipes(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    decode_identifier_bool_map(input, field, context, "recipe map")
}

pub(super) fn decode_player_advancements(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let len = context.charge_record(input, field, "advancement map")?;
    let entries = input.visible_map_entries(len);
    let mut normalized = HashSet::with_capacity(entries.len());
    let mut output = RegistryCodecRecord::default();
    for (raw_key, value) in entries {
        let key_field = child(field, raw_key);
        let key = RegistryResourceIdentifier::parse_java_string(raw_key)
            .ok_or_else(|| {
                context.codec_error(
                    &key_field,
                    format!("invalid advancement identifier {raw_key}"),
                )
            })?
            .into_string();
        if !normalized.insert(key.clone()) {
            return Err(context.codec_error(
                &key_field,
                format!("duplicate normalized advancement {key}"),
            ));
        }
        let value = context.with_nested(&key_field, |context| {
            decode_advancement_predicate(value, &key_field, context)
        })?;
        output.push(&key, value);
    }
    Ok(output.finish())
}

pub(super) fn decode_input_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "input predicate")?;
    let mut output = RegistryCodecRecord::default();
    for name in [
        "forward", "backward", "left", "right", "jump", "sneak", "sprint",
    ] {
        if let Some(value) = input.field(name) {
            let nested = child(field, name);
            let value = context.with_nested(&nested, |context| {
                decode_bool_value(value, &nested, context)
            })?;
            output.push(name, value);
        }
    }
    Ok(output.finish())
}

fn decode_double_bounds_record(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    expected: &str,
    fields: &[&str],
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, expected)?;
    let mut output = RegistryCodecRecord::default();
    for name in fields {
        if let Some(value) = input.field(name) {
            let nested = child(field, name);
            let value = context.with_nested(&nested, |context| {
                decode_double_bounds(value, &nested, context)
            })?;
            if !value.is_any {
                output.push(*name, value.value);
            }
        }
    }
    Ok(output.finish())
}

fn decode_mob_effect_instance_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "mob effect instance predicate")?;
    let mut output = RegistryCodecRecord::default();
    for name in ["amplifier", "duration"] {
        if let Some(value) = input.field(name) {
            let nested = child(field, name);
            let value = context.with_nested(&nested, |context| {
                decode_int_bounds(value, &nested, context)
            })?;
            if !value.is_any {
                output.push(name, value.value);
            }
        }
    }
    for name in ["ambient", "visible"] {
        if let Some(value) = input.field(name) {
            let nested = child(field, name);
            let value = context.with_nested(&nested, |context| {
                decode_bool_value(value, &nested, context)
            })?;
            output.push(name, value);
        }
    }
    Ok(output.finish())
}

fn decode_stat_matcher(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "player stat matcher")?;
    let type_field = child(field, "type");
    let type_input = input
        .field("type")
        .ok_or_else(|| context.codec_error(&type_field, "missing required field `type`"))?;
    let type_value = context.with_nested(&type_field, |context| {
        decode_identifier_value(type_input, &type_field, context)
    })?;
    let type_id = canonical_string(&type_value, &type_field, context)?;
    let stat_type = type_id
        .with_utf8(vanilla_stat_type_by_resource_id)
        .flatten()
        .ok_or_else(|| context.codec_error(&type_field, format!("unknown stat type {type_id}")))?;

    let stat_field = child(field, "stat");
    let stat = input
        .field("stat")
        .ok_or_else(|| context.codec_error(&stat_field, "missing required field `stat`"))?;
    let stat = context.with_nested(&stat_field, |context| {
        decode_fixed_holder_reference(stat, &stat_field, context, stat_type.value_registry)
    })?;

    let mut output = RegistryCodecRecord::default();
    output.push("stat", stat);
    if let Some(value) = input.field("value") {
        let value_field = child(field, "value");
        let value = context.with_nested(&value_field, |context| {
            decode_int_bounds(value, &value_field, context)
        })?;
        if !value.is_any {
            output.push("value", value.value);
        }
    }
    output.push("type", type_value);
    Ok(output.finish())
}

fn decode_identifier_bool_map(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    expected: &str,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let len = context.charge_record(input, field, expected)?;
    let entries = input.visible_map_entries(len);
    let mut normalized = HashSet::with_capacity(entries.len());
    let mut output = RegistryCodecRecord::default();
    for (raw_key, value) in entries {
        let key_field = child(field, raw_key);
        let key = RegistryResourceIdentifier::parse_java_string(raw_key)
            .ok_or_else(|| {
                context.codec_error(&key_field, format!("invalid resource identifier {raw_key}"))
            })?
            .into_string();
        if !normalized.insert(key.clone()) {
            return Err(
                context.codec_error(&key_field, format!("duplicate normalized identifier {key}"))
            );
        }
        let value = context.with_nested(&key_field, |context| {
            decode_bool_value(value, &key_field, context)
        })?;
        output.push(&key, value);
    }
    Ok(output.finish())
}

fn decode_advancement_predicate(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if input.as_bool().is_some() {
        return decode_bool_value(input, field, context);
    }
    let len = context.charge_record(input, field, "advancement criterion map")?;
    let entries = input.visible_map_entries(len);
    let mut output = RegistryCodecRecord::default();
    for (name, value) in entries {
        let nested = child(field, name);
        let value = context.with_nested(&nested, |context| {
            decode_bool_value(value, &nested, context)
        })?;
        output.push(name, value);
    }
    Ok(output.finish())
}

fn canonical_string<'a>(
    value: &'a CanonicalRegistryCodecValue,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<&'a bbb_protocol::JavaString, RegistryCodecError> {
    match value.network_nbt_unchecked() {
        bbb_protocol::NbtSummaryValue::String(value) => Ok(value),
        _ => Err(context.codec_error(field, "identifier decoder did not emit a string")),
    }
}

#[cfg(test)]
mod tests;
