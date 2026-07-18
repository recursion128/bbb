use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

use crate::FrozenRegistryAccess;
#[cfg(test)]
use crate::RegistrySet;

use super::{
    component_f32_bits, find_entry,
    holder_set::{decode_registry_holder_set, DialogRegistryHolderSetState},
    optional_ranged_component_float, required_component_f32_bits,
    sound_event::{decode_sound_event_holder, DialogSoundEventHolderState},
    validate_component_float, validate_component_float_exclusive_min,
};

const DAMAGE_TYPE_REGISTRY: &str = "minecraft:damage_type";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogBlocksAttacksDamageReductionState {
    pub horizontal_blocking_angle_bits: u32,
    #[serde(default)]
    pub damage_types: Option<DialogRegistryHolderSetState>,
    pub base_bits: u32,
    pub factor_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogBlocksAttacksItemDamageState {
    pub threshold_bits: u32,
    pub base_bits: u32,
    pub factor_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogBlocksAttacksState {
    pub block_delay_seconds_bits: u32,
    pub disable_cooldown_scale_bits: u32,
    pub damage_reductions: Vec<DialogBlocksAttacksDamageReductionState>,
    pub item_damage: DialogBlocksAttacksItemDamageState,
    #[serde(default)]
    pub bypassed_by: Option<DialogRegistryHolderSetState>,
    #[serde(default)]
    pub block_sound: Option<DialogSoundEventHolderState>,
    #[serde(default)]
    pub disabled_sound: Option<DialogSoundEventHolderState>,
}

pub(super) fn decode_blocks_attacks(
    value: &NbtSummaryValue,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<DialogBlocksAttacksState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    Ok(DialogBlocksAttacksState {
        block_delay_seconds_bits: optional_ranged_component_float(
            entries,
            "block_delay_seconds",
            0.0,
            0.0,
            f32::MAX,
            what,
        )?,
        disable_cooldown_scale_bits: optional_ranged_component_float(
            entries,
            "disable_cooldown_scale",
            1.0,
            0.0,
            f32::MAX,
            what,
        )?,
        damage_reductions: damage_reductions(entries, registries, what)?,
        item_damage: item_damage(entries, what)?,
        bypassed_by: optional_holder_set(entries, "bypassed_by", registries, what)?,
        block_sound: optional_sound_event(entries, "block_sound", what)?,
        disabled_sound: optional_sound_event(entries, "disabled_sound", what)?,
    })
}

fn damage_reductions(
    entries: &[NbtSummaryEntry],
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<Vec<DialogBlocksAttacksDamageReductionState>, String> {
    let Some(value) = find_entry(entries, "damage_reductions") else {
        return Ok(vec![DialogBlocksAttacksDamageReductionState {
            horizontal_blocking_angle_bits: 90.0f32.to_bits(),
            damage_types: None,
            base_bits: 0.0f32.to_bits(),
            factor_bits: 1.0f32.to_bits(),
        }]);
    };
    let NbtSummaryValue::List(values) = value else {
        return Err(format!("{what} field damage_reductions is not a list"));
    };
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            decode_damage_reduction(
                value,
                registries,
                &format!("{what} field damage_reductions[{index}]"),
            )
        })
        .collect()
}

fn decode_damage_reduction(
    value: &NbtSummaryValue,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<DialogBlocksAttacksDamageReductionState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let horizontal_blocking_angle_bits = find_entry(entries, "horizontal_blocking_angle")
        .map_or(Ok(90.0f32.to_bits()), |value| {
            component_f32_bits(value, &format!("{what} field horizontal_blocking_angle"))
        })?;
    validate_component_float_exclusive_min(
        horizontal_blocking_angle_bits,
        0.0,
        f32::MAX,
        &format!("{what} field horizontal_blocking_angle"),
    )?;
    Ok(DialogBlocksAttacksDamageReductionState {
        horizontal_blocking_angle_bits,
        damage_types: optional_holder_set(entries, "type", registries, what)?,
        base_bits: required_component_f32_bits(entries, "base", what)?,
        factor_bits: required_component_f32_bits(entries, "factor", what)?,
    })
}

fn item_damage(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<DialogBlocksAttacksItemDamageState, String> {
    let Some(value) = find_entry(entries, "item_damage") else {
        return Ok(DialogBlocksAttacksItemDamageState {
            threshold_bits: 1.0f32.to_bits(),
            base_bits: 0.0f32.to_bits(),
            factor_bits: 1.0f32.to_bits(),
        });
    };
    let item_what = format!("{what} field item_damage");
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{item_what} is not a compound"));
    };
    let threshold_bits = required_component_f32_bits(entries, "threshold", &item_what)?;
    validate_component_float(
        threshold_bits,
        0.0,
        f32::MAX,
        &format!("{item_what} field threshold"),
    )?;
    Ok(DialogBlocksAttacksItemDamageState {
        threshold_bits,
        base_bits: required_component_f32_bits(entries, "base", &item_what)?,
        factor_bits: required_component_f32_bits(entries, "factor", &item_what)?,
    })
}

fn optional_holder_set(
    entries: &[NbtSummaryEntry],
    field: &str,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<Option<DialogRegistryHolderSetState>, String> {
    find_entry(entries, field)
        .map(|value| {
            decode_registry_holder_set(
                value,
                DAMAGE_TYPE_REGISTRY,
                registries,
                &format!("{what} field {field}"),
            )
        })
        .transpose()
}

fn optional_sound_event(
    entries: &[NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<Option<DialogSoundEventHolderState>, String> {
    find_entry(entries, field)
        .map(|value| decode_sound_event_holder(value, &format!("{what} field {field}")))
        .transpose()
}

#[cfg(test)]
mod tests;
