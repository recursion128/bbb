use bbb_protocol::{
    item_component_value_types::ItemUseAnimationSummary,
    packets::{ConsumeEffectSummary, DataComponentPatchSummary},
};
use serde::{Deserialize, Serialize};

use crate::FrozenRegistryAccess;

use super::{
    holder_set::{project_registry_holder_set, DialogRegistryHolderSetState},
    potion_contents::{project_mob_effect_instance, DialogMobEffectInstanceState},
    sound_event::{project_sound_event_holder, DialogSoundEventHolderState},
};

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

pub(super) fn project_consumable(
    patch: &DataComponentPatchSummary,
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<DialogConsumableState, String> {
    let value = patch.consumable.as_ref().unwrap();
    Ok(DialogConsumableState {
        consume_seconds_bits: value.consume_seconds.to_bits(),
        animation: value.animation,
        sound: project_sound_event_holder(patch.consumable_sound.as_ref().unwrap(), registries)?,
        has_consume_particles: patch.consumable_has_consume_particles.unwrap(),
        on_consume_effects: project_effects(&patch.consumable_effects, registries)?,
    })
}

pub(super) fn project_effects(
    effects: &[ConsumeEffectSummary],
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<Vec<DialogConsumeEffectState>, String> {
    effects
        .iter()
        .map(|effect| {
            Ok(match effect {
                ConsumeEffectSummary::ApplyEffects {
                    effects,
                    probability_bits,
                } => DialogConsumeEffectState::ApplyEffects {
                    effects: effects.iter().map(project_mob_effect_instance).collect(),
                    probability_bits: *probability_bits,
                },
                ConsumeEffectSummary::RemoveEffects {
                    effect_registry_ids,
                    effect_tag,
                } => DialogConsumeEffectState::RemoveEffects {
                    effects: project_registry_holder_set(
                        effect_registry_ids,
                        effect_tag,
                        "minecraft:mob_effect",
                        registries,
                    )?,
                },
                ConsumeEffectSummary::ClearAllEffects => DialogConsumeEffectState::ClearAllEffects,
                ConsumeEffectSummary::TeleportRandomly { diameter_bits } => {
                    DialogConsumeEffectState::TeleportRandomly {
                        diameter_bits: *diameter_bits,
                    }
                }
                ConsumeEffectSummary::PlaySound { sound } => DialogConsumeEffectState::PlaySound {
                    sound: project_sound_event_holder(sound, registries)?,
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
