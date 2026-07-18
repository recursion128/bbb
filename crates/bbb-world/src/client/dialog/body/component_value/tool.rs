use bbb_protocol::NbtSummaryValue;
use serde::{Deserialize, Serialize};

use crate::FrozenRegistryAccess;
#[cfg(test)]
use crate::RegistrySet;

use super::{
    component_bool, component_f32_bits, find_entry,
    holder_set::{decode_registry_holder_set, DialogRegistryHolderSetState},
    optional_component_bool, optional_component_i32, validate_component_float_exclusive_min,
    validate_i32_range,
};

const BLOCK_REGISTRY: &str = "minecraft:block";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogToolRuleState {
    pub blocks: DialogRegistryHolderSetState,
    #[serde(default)]
    pub speed_bits: Option<u32>,
    #[serde(default)]
    pub correct_for_drops: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogToolState {
    pub rules: Vec<DialogToolRuleState>,
    pub default_mining_speed_bits: u32,
    pub damage_per_block: i32,
    pub can_destroy_blocks_in_creative: bool,
}

pub(super) fn decode_tool(
    value: &NbtSummaryValue,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<DialogToolState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let rules_value =
        find_entry(entries, "rules").ok_or_else(|| format!("{what} is missing field rules"))?;
    let NbtSummaryValue::List(rules) = rules_value else {
        return Err(format!("{what} field rules is not a list"));
    };
    let rules = rules
        .iter()
        .enumerate()
        .map(|(index, value)| {
            decode_tool_rule(value, registries, &format!("{what} field rules[{index}]"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let default_mining_speed_bits = find_entry(entries, "default_mining_speed")
        .map_or(Ok(1.0f32.to_bits()), |value| {
            component_f32_bits(value, &format!("{what} field default_mining_speed"))
        })?;
    let damage_per_block = optional_component_i32(entries, "damage_per_block", 1, what)?;
    validate_i32_range(
        damage_per_block,
        0,
        i32::MAX,
        &format!("{what} field damage_per_block"),
    )?;
    Ok(DialogToolState {
        rules,
        default_mining_speed_bits,
        damage_per_block,
        can_destroy_blocks_in_creative: optional_component_bool(
            entries,
            "can_destroy_blocks_in_creative",
            true,
            what,
        )?,
    })
}

fn decode_tool_rule(
    value: &NbtSummaryValue,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<DialogToolRuleState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let blocks =
        find_entry(entries, "blocks").ok_or_else(|| format!("{what} is missing field blocks"))?;
    let speed_bits = find_entry(entries, "speed")
        .map(|value| {
            let bits = component_f32_bits(value, &format!("{what} field speed"))?;
            validate_component_float_exclusive_min(
                bits,
                0.0,
                f32::MAX,
                &format!("{what} field speed"),
            )?;
            Ok::<u32, String>(bits)
        })
        .transpose()?;
    let correct_for_drops = find_entry(entries, "correct_for_drops")
        .map(|value| component_bool(value, &format!("{what} field correct_for_drops")))
        .transpose()?;
    Ok(DialogToolRuleState {
        blocks: decode_registry_holder_set(blocks, BLOCK_REGISTRY, registries, what)?,
        speed_bits,
        correct_for_drops,
    })
}

#[cfg(test)]
mod tests;
