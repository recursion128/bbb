use bbb_protocol::{
    consume_effect_types::{
        VANILLA_CONSUME_EFFECT_TYPES, VANILLA_CONSUME_EFFECT_TYPE_APPLY_EFFECTS_ID,
        VANILLA_CONSUME_EFFECT_TYPE_CLEAR_ALL_EFFECTS_ID,
        VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID, VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID,
        VANILLA_CONSUME_EFFECT_TYPE_TELEPORT_RANDOMLY_ID,
    },
    item_component_value_types::ItemUseAnimationSummary,
    NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::RegistrySet;
use crate::{registries::normalize_codec_identifier, FrozenRegistryAccess};

use super::{
    component_bool, component_f32_bits, find_entry,
    holder_set::{decode_registry_holder_set, DialogRegistryHolderSetState},
    potion_contents::{decode_mob_effect_instance, DialogMobEffectInstanceState},
    sound_event::{decode_sound_event_holder, DialogSoundEventHolderState},
    validate_component_float, validate_component_float_exclusive_min,
};

const MOB_EFFECT_REGISTRY: &str = "minecraft:mob_effect";
const DEFAULT_CONSUME_SOUND: &str = "minecraft:entity.generic.eat";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogConsumableState {
    pub consume_seconds_bits: u32,
    pub animation: ItemUseAnimationSummary,
    pub sound: DialogSoundEventHolderState,
    pub has_consume_particles: bool,
    #[serde(default)]
    pub on_consume_effects: Vec<DialogConsumeEffectState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DialogConsumeEffectState {
    ApplyEffects {
        effects: Vec<DialogMobEffectInstanceState>,
        probability_bits: u32,
    },
    RemoveEffects {
        effects: DialogRegistryHolderSetState,
    },
    ClearAllEffects,
    TeleportRandomly {
        diameter_bits: u32,
    },
    PlaySound {
        sound: DialogSoundEventHolderState,
    },
}

pub(super) fn decode_consumable(
    value: &NbtSummaryValue,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<DialogConsumableState, String> {
    let entries = compound(value, what)?;
    let consume_seconds_bits = optional_float(entries, "consume_seconds", 1.6, what)?;
    validate_component_float(
        consume_seconds_bits,
        0.0,
        f32::MAX,
        &format!("{what} field consume_seconds"),
    )?;
    let animation = match find_entry(entries, "animation") {
        None => ItemUseAnimationSummary::Eat,
        Some(NbtSummaryValue::String(name)) => ItemUseAnimationSummary::from_serialized_name(name)
            .ok_or_else(|| format!("unknown {what} field animation {name}"))?,
        Some(_) => return Err(format!("{what} field animation is not a string")),
    };
    let sound = match find_entry(entries, "sound") {
        Some(value) => decode_sound_event_holder(value, &format!("{what} field sound"))?,
        None => decode_sound_event_holder(
            &NbtSummaryValue::String(DEFAULT_CONSUME_SOUND.to_string()),
            &format!("{what} default sound"),
        )?,
    };
    let has_consume_particles = find_entry(entries, "has_consume_particles")
        .map_or(Ok(true), |value| {
            component_bool(value, &format!("{what} field has_consume_particles"))
        })?;
    let on_consume_effects =
        decode_optional_effects(entries, "on_consume_effects", registries, what)?;
    Ok(DialogConsumableState {
        consume_seconds_bits,
        animation,
        sound,
        has_consume_particles,
        on_consume_effects,
    })
}

pub(super) fn decode_death_protection(
    value: &NbtSummaryValue,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<Vec<DialogConsumeEffectState>, String> {
    let entries = compound(value, what)?;
    decode_optional_effects(entries, "death_effects", registries, what)
}

fn decode_optional_effects(
    entries: &[NbtSummaryEntry],
    field: &str,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<Vec<DialogConsumeEffectState>, String> {
    let Some(value) = find_entry(entries, field) else {
        return Ok(Vec::new());
    };
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} field {field} is not a list"));
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            decode_consume_effect(value, registries, &format!("{what} field {field}[{index}]"))
        })
        .collect()
}

fn decode_consume_effect(
    value: &NbtSummaryValue,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<DialogConsumeEffectState, String> {
    let entries = compound(value, what)?;
    let type_id = match find_entry(entries, "type") {
        Some(NbtSummaryValue::String(raw)) => normalize_codec_identifier(raw)
            .ok_or_else(|| format!("invalid {what} field type resource id {raw}"))?,
        Some(_) => return Err(format!("{what} field type is not a resource id")),
        None => return Err(format!("{what} is missing field type")),
    };
    let effect_type = VANILLA_CONSUME_EFFECT_TYPES
        .iter()
        .find(|entry| entry.resource_id == type_id)
        .ok_or_else(|| format!("unknown {what} type {type_id}"))?;
    match effect_type.id {
        VANILLA_CONSUME_EFFECT_TYPE_APPLY_EFFECTS_ID => decode_apply_effects(entries, what),
        VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID => {
            let effects = find_entry(entries, "effects")
                .ok_or_else(|| format!("{what} is missing field effects"))?;
            Ok(DialogConsumeEffectState::RemoveEffects {
                effects: decode_registry_holder_set(
                    effects,
                    MOB_EFFECT_REGISTRY,
                    registries,
                    &format!("{what} field effects"),
                )?,
            })
        }
        VANILLA_CONSUME_EFFECT_TYPE_CLEAR_ALL_EFFECTS_ID => {
            Ok(DialogConsumeEffectState::ClearAllEffects)
        }
        VANILLA_CONSUME_EFFECT_TYPE_TELEPORT_RANDOMLY_ID => {
            let diameter_bits = optional_float(entries, "diameter", 16.0, what)?;
            validate_component_float_exclusive_min(
                diameter_bits,
                0.0,
                f32::MAX,
                &format!("{what} field diameter"),
            )?;
            Ok(DialogConsumeEffectState::TeleportRandomly { diameter_bits })
        }
        VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID => {
            let sound = find_entry(entries, "sound")
                .ok_or_else(|| format!("{what} is missing field sound"))?;
            Ok(DialogConsumeEffectState::PlaySound {
                sound: decode_sound_event_holder(sound, &format!("{what} field sound"))?,
            })
        }
        _ => Err(format!("unknown {what} type {type_id}")),
    }
}

fn decode_apply_effects(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<DialogConsumeEffectState, String> {
    let effects = match find_entry(entries, "effects") {
        Some(NbtSummaryValue::List(values)) => values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                decode_mob_effect_instance(value, &format!("{what} field effects[{index}]"))
            })
            .collect::<Result<Vec<_>, String>>()?,
        Some(_) => return Err(format!("{what} field effects is not a list")),
        None => return Err(format!("{what} is missing field effects")),
    };
    let probability_bits = optional_float(entries, "probability", 1.0, what)?;
    validate_component_float(
        probability_bits,
        0.0,
        1.0,
        &format!("{what} field probability"),
    )?;
    Ok(DialogConsumeEffectState::ApplyEffects {
        effects,
        probability_bits,
    })
}

fn optional_float(
    entries: &[NbtSummaryEntry],
    field: &str,
    default: f32,
    what: &str,
) -> Result<u32, String> {
    find_entry(entries, field).map_or(Ok(default.to_bits()), |value| {
        component_f32_bits(value, &format!("{what} field {field}"))
    })
}

fn compound<'a>(value: &'a NbtSummaryValue, what: &str) -> Result<&'a [NbtSummaryEntry], String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    Ok(entries)
}

#[cfg(test)]
mod tests;
