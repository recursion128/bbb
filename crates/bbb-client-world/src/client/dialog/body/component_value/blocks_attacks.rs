use bbb_protocol::packets::{BlocksAttacksSummary, ItemDamageFunctionSummary};
use serde::{Deserialize, Serialize};

use crate::FrozenRegistryAccess;

use super::{
    holder_set::{project_registry_holder_set, DialogRegistryHolderSetState},
    sound_event::{project_sound_event_holder, DialogSoundEventHolderState},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogBlocksAttacksDamageReductionState {
    pub horizontal_blocking_angle_bits: u32,
    #[serde(default)]
    pub damage_types: Option<DialogRegistryHolderSetState>,
    pub base_bits: u32,
    pub factor_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogBlocksAttacksState {
    pub block_delay_seconds_bits: u32,
    pub disable_cooldown_scale_bits: u32,
    pub damage_reductions: Vec<DialogBlocksAttacksDamageReductionState>,
    pub item_damage: ItemDamageFunctionSummary,
    #[serde(default)]
    pub bypassed_by: Option<DialogRegistryHolderSetState>,
    #[serde(default)]
    pub block_sound: Option<DialogSoundEventHolderState>,
    #[serde(default)]
    pub disabled_sound: Option<DialogSoundEventHolderState>,
}

pub(super) fn project_blocks_attacks(
    attacks: &BlocksAttacksSummary,
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<DialogBlocksAttacksState, String> {
    let holder_set = |types: &bbb_protocol::packets::HolderSetSummary| {
        project_registry_holder_set(
            &types.registry_ids,
            &types.tag,
            "minecraft:damage_type",
            registries,
        )
    };
    Ok(DialogBlocksAttacksState {
        block_delay_seconds_bits: attacks.block_delay_seconds_bits,
        disable_cooldown_scale_bits: attacks.disable_cooldown_scale_bits,
        damage_reductions: attacks
            .damage_reductions
            .iter()
            .map(|reduction| {
                Ok(DialogBlocksAttacksDamageReductionState {
                    horizontal_blocking_angle_bits: reduction.horizontal_blocking_angle_bits,
                    damage_types: reduction
                        .damage_types
                        .as_ref()
                        .map(holder_set)
                        .transpose()?,
                    base_bits: reduction.base_bits,
                    factor_bits: reduction.factor_bits,
                })
            })
            .collect::<Result<_, String>>()?,
        item_damage: attacks.item_damage.clone(),
        bypassed_by: attacks.bypassed_by.as_ref().map(holder_set).transpose()?,
        block_sound: attacks
            .block_sound
            .as_ref()
            .map(|sound| project_sound_event_holder(sound, registries))
            .transpose()?,
        disabled_sound: attacks
            .disable_sound
            .as_ref()
            .map(|sound| project_sound_event_holder(sound, registries))
            .transpose()?,
    })
}

#[cfg(test)]
mod tests;
