use bbb_protocol::{
    potion_types::vanilla_potion_for_resource_id, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::registries::normalize_codec_identifier;

use super::{
    component_bool, component_i32, find_entry,
    holder_set::{decode_builtin_mob_effect_holder_reference, DialogRegistryHolderReferenceState},
};

const MAX_MOB_EFFECT_DETAILS_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogPotionContentsState {
    #[serde(default)]
    pub potion: Option<DialogRegistryHolderReferenceState>,
    #[serde(default)]
    pub custom_color: Option<i32>,
    #[serde(default)]
    pub custom_effects: Vec<DialogMobEffectInstanceState>,
    #[serde(default)]
    pub custom_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogMobEffectInstanceState {
    pub effect: DialogRegistryHolderReferenceState,
    pub details: DialogMobEffectDetailsState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogMobEffectDetailsState {
    pub amplifier: i32,
    pub duration: i32,
    pub ambient: bool,
    pub show_particles: bool,
    pub show_icon: bool,
    #[serde(default)]
    pub hidden_effect: Option<Box<DialogMobEffectDetailsState>>,
}

pub(super) fn decode_potion_contents(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogPotionContentsState, String> {
    if matches!(value, NbtSummaryValue::String(_)) {
        return Ok(DialogPotionContentsState {
            potion: Some(decode_builtin_potion_holder(value, what)?),
            custom_color: None,
            custom_effects: Vec::new(),
            custom_name: None,
        });
    }
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!(
            "{what} is neither a compound nor a potion resource id"
        ));
    };
    let potion = find_entry(entries, "potion")
        .map(|value| decode_builtin_potion_holder(value, &format!("{what} field potion")))
        .transpose()?;
    let custom_color = find_entry(entries, "custom_color")
        .map(|value| component_i32(value, &format!("{what} field custom_color")))
        .transpose()?;
    let custom_effects = match find_entry(entries, "custom_effects") {
        None => Vec::new(),
        Some(NbtSummaryValue::List(effects)) => effects
            .iter()
            .enumerate()
            .map(|(index, value)| {
                decode_mob_effect_instance(value, &format!("{what} field custom_effects[{index}]"))
            })
            .collect::<Result<Vec<_>, String>>()?,
        Some(_) => return Err(format!("{what} field custom_effects is not a list")),
    };
    let custom_name = find_entry(entries, "custom_name")
        .map(|value| {
            let NbtSummaryValue::String(name) = value else {
                return Err(format!("{what} field custom_name is not a string"));
            };
            Ok::<String, String>(name.clone())
        })
        .transpose()?;
    Ok(DialogPotionContentsState {
        potion,
        custom_color,
        custom_effects,
        custom_name,
    })
}

fn decode_builtin_potion_holder(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogRegistryHolderReferenceState, String> {
    let NbtSummaryValue::String(raw) = value else {
        return Err(format!("{what} holder is not a resource id"));
    };
    let resource_id = normalize_codec_identifier(raw)
        .ok_or_else(|| format!("invalid {what} holder resource id {raw}"))?;
    let potion = vanilla_potion_for_resource_id(&resource_id)
        .ok_or_else(|| format!("missing minecraft:potion resource {resource_id} for {what}"))?;
    Ok(DialogRegistryHolderReferenceState {
        holder_id: potion.id,
        resource_id: potion.resource_id.to_string(),
    })
}

pub(super) fn decode_mob_effect_instance(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogMobEffectInstanceState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let id = find_entry(entries, "id").ok_or_else(|| format!("{what} is missing field id"))?;
    Ok(DialogMobEffectInstanceState {
        effect: decode_builtin_mob_effect_holder_reference(id, &format!("{what} field id"))?,
        details: decode_mob_effect_details(entries, what, 0)?,
    })
}

fn decode_mob_effect_details(
    entries: &[NbtSummaryEntry],
    what: &str,
    depth: usize,
) -> Result<DialogMobEffectDetailsState, String> {
    if depth > MAX_MOB_EFFECT_DETAILS_DEPTH {
        return Err(format!(
            "{what} exceeds maximum mob-effect details depth {MAX_MOB_EFFECT_DETAILS_DEPTH}"
        ));
    }
    let amplifier = optional_i32(entries, "amplifier", 0, what)?;
    if !(0..=255).contains(&amplifier) {
        return Err(format!(
            "{what} field amplifier {amplifier} is outside 0..=255"
        ));
    }
    let duration = optional_i32(entries, "duration", 0, what)?;
    let ambient = optional_bool(entries, "ambient", false, what)?;
    let show_particles = optional_bool(entries, "show_particles", true, what)?;
    let show_icon = optional_bool(entries, "show_icon", show_particles, what)?;
    let hidden_effect = find_entry(entries, "hidden_effect")
        .map(|value| {
            let NbtSummaryValue::Compound(entries) = value else {
                return Err(format!("{what} field hidden_effect is not a compound"));
            };
            decode_mob_effect_details(entries, &format!("{what} field hidden_effect"), depth + 1)
                .map(Box::new)
        })
        .transpose()?;
    Ok(DialogMobEffectDetailsState {
        amplifier,
        duration,
        ambient,
        show_particles,
        show_icon,
        hidden_effect,
    })
}

fn optional_i32(
    entries: &[NbtSummaryEntry],
    field: &str,
    default: i32,
    what: &str,
) -> Result<i32, String> {
    find_entry(entries, field).map_or(Ok(default), |value| {
        component_i32(value, &format!("{what} field {field}"))
    })
}

fn optional_bool(
    entries: &[NbtSummaryEntry],
    field: &str,
    default: bool,
    what: &str,
) -> Result<bool, String> {
    find_entry(entries, field).map_or(Ok(default), |value| {
        component_bool(value, &format!("{what} field {field}"))
    })
}

#[cfg(test)]
mod tests;
