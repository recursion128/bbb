use super::holder_set::{project_registry_holder_set, DialogRegistryHolderSetState};
use crate::FrozenRegistryAccess;
#[cfg(test)]
use crate::RegistrySet;
use bbb_protocol::packets::ToolSummary;
use serde::{Deserialize, Serialize};

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

pub(super) fn project_tool(
    value: &ToolSummary,
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<DialogToolState, String> {
    Ok(DialogToolState {
        rules: value
            .rules
            .iter()
            .map(|rule| {
                Ok(DialogToolRuleState {
                    blocks: project_registry_holder_set(
                        &rule.block_registry_ids,
                        &rule.block_tag,
                        BLOCK_REGISTRY,
                        registries,
                    )?,
                    speed_bits: rule.speed_bits,
                    correct_for_drops: rule.correct_for_drops,
                })
            })
            .collect::<Result<_, String>>()?,
        default_mining_speed_bits: value.default_mining_speed_bits,
        damage_per_block: value.damage_per_block,
        can_destroy_blocks_in_creative: value.can_destroy_blocks_in_creative,
    })
}

#[cfg(test)]
mod tests;
