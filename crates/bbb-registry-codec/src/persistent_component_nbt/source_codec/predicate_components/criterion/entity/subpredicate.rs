//! Exact six-family `EntitySubPredicate.CODEC` dispatcher.
//!
//! The built-in registry selects one MapCodec payload and DFU writes the
//! payload fields before the normalized `type` discriminator

use crate::{
    codec_dispatch_types::{entity_sub_predicate_codec_type, EntitySubPredicateCodecFamily},
    registry_codec::{
        recursive_values::{CanonicalRegistryCodecValue, RegistryCodecDecodeContext},
        RegistryCodecError, RegistryElementValueRef,
    },
};
use bbb_protocol::NbtSummaryValue;

use super::super::super::{child, decode_identifier_value, decode_int_bounds, RegistryCodecRecord};

mod leaf;

pub(super) fn decode(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let type_field = child(field, "type");
    let type_input = input
        .field("type")
        .ok_or_else(|| context.codec_error(&type_field, "missing required field `type`"))?;
    let type_value = context.with_nested(&type_field, |context| {
        decode_identifier_value(type_input, &type_field, context)
    })?;
    let type_id = match type_value.network_nbt_unchecked() {
        NbtSummaryValue::String(type_id) => type_id.clone(),
        _ => unreachable!("identifier decoder always emits a string"),
    };
    let family = type_id
        .with_utf8(entity_sub_predicate_codec_type)
        .flatten()
        .map(|entry| entry.codec_family)
        .ok_or_else(|| {
            context.codec_error(
                &type_field,
                format!("unknown entity subpredicate type {type_id}"),
            )
        })?;

    let payload = match family {
        EntitySubPredicateCodecFamily::LightningBoltPredicate => {
            decode_lightning(input, field, context)
        }
        EntitySubPredicateCodecFamily::FishingHookPredicate => {
            leaf::decode_fishing_hook(input, field, context)
        }
        EntitySubPredicateCodecFamily::PlayerPredicate => decode_player(input, field, context),
        EntitySubPredicateCodecFamily::SlimePredicate => leaf::decode_slime(input, field, context),
        EntitySubPredicateCodecFamily::RaiderPredicate => {
            leaf::decode_raider(input, field, context)
        }
        EntitySubPredicateCodecFamily::SheepPredicate => leaf::decode_sheep(input, field, context),
    }?;
    finish_dispatch(payload, type_value, field, context)
}

fn decode_lightning(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "lightning entity subpredicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(value) = input.field("blocks_set_on_fire") {
        let nested = child(field, "blocks_set_on_fire");
        let value = context.with_nested(&nested, |context| {
            decode_int_bounds(value, &nested, context)
        })?;
        if !value.is_any {
            output.push("blocks_set_on_fire", value.value);
        }
    }
    if let Some(value) = input.field("entity_struck") {
        let nested = child(field, "entity_struck");
        let value =
            context.with_nested(&nested, |context| super::decode(value, &nested, context))?;
        output.push("entity_struck", value);
    }
    Ok(output.finish())
}

fn decode_player(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "player entity subpredicate")?;
    let mut output = RegistryCodecRecord::default();
    if let Some(value) = input.field("level") {
        let nested = child(field, "level");
        let value = context.with_nested(&nested, |context| {
            decode_int_bounds(value, &nested, context)
        })?;
        if !value.is_any {
            output.push("level", value.value);
        }
    }
    if let Some(value) = input.field("food") {
        let nested = child(field, "food");
        let value = context.with_nested(&nested, |context| {
            super::leaf::decode_food_predicate(value, &nested, context)
        })?;
        if !value.is_default {
            output.push("food", value.value);
        }
    }
    if let Some(value) = input.field("gamemode") {
        let nested = child(field, "gamemode");
        let value = context.with_nested(&nested, |context| {
            super::leaf::decode_game_type_predicate(value, &nested, context)
        })?;
        if !value.is_default {
            output.push("gamemode", value.value);
        }
    }
    if let Some(value) = input.field("stats") {
        let nested = child(field, "stats");
        let value = context.with_nested(&nested, |context| {
            super::leaf::decode_player_stats(value, &nested, context)
        })?;
        if !is_empty_list(&value) {
            output.push("stats", value);
        }
    }
    if let Some(value) = input.field("recipes") {
        let nested = child(field, "recipes");
        let value = context.with_nested(&nested, |context| {
            super::leaf::decode_player_recipes(value, &nested, context)
        })?;
        if !is_empty_compound(&value) {
            output.push("recipes", value);
        }
    }
    if let Some(value) = input.field("advancements") {
        let nested = child(field, "advancements");
        let value = context.with_nested(&nested, |context| {
            super::leaf::decode_player_advancements(value, &nested, context)
        })?;
        if !is_empty_compound(&value) {
            output.push("advancements", value);
        }
    }
    if let Some(value) = input.field("looking_at") {
        let nested = child(field, "looking_at");
        let value =
            context.with_nested(&nested, |context| super::decode(value, &nested, context))?;
        output.push("looking_at", value);
    }
    if let Some(value) = input.field("input") {
        let nested = child(field, "input");
        let value = context.with_nested(&nested, |context| {
            super::leaf::decode_input_predicate(value, &nested, context)
        })?;
        output.push("input", value);
    }
    Ok(output.finish())
}

fn finish_dispatch(
    payload: CanonicalRegistryCodecValue,
    type_value: CanonicalRegistryCodecValue,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let (network_nbt, mut references) = payload.into_parts();
    let NbtSummaryValue::Compound(mut fields) = network_nbt else {
        return Err(context.codec_error(field, "subpredicate payload is not a compound"));
    };
    let (type_nbt, type_references) = type_value.into_parts();
    references.extend(type_references);
    fields.push(bbb_protocol::NbtSummaryEntry {
        name: "type".into(),
        value: type_nbt,
    });
    Ok(CanonicalRegistryCodecValue::from_parts(
        NbtSummaryValue::Compound(fields),
        references,
    ))
}

fn is_empty_list(value: &CanonicalRegistryCodecValue) -> bool {
    matches!(value.network_nbt_unchecked(), NbtSummaryValue::List(values) if values.is_empty())
}

fn is_empty_compound(value: &CanonicalRegistryCodecValue) -> bool {
    matches!(value.network_nbt_unchecked(), NbtSummaryValue::Compound(values) if values.is_empty())
}

#[cfg(test)]
mod tests;
