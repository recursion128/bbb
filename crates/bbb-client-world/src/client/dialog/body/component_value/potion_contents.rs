use bbb_protocol::{
    mob_effect_types::vanilla_mob_effect_type,
    packets::{DataComponentPatchSummary, MobEffectDetailsSummary, MobEffectInstanceSummary},
    potion_types::vanilla_potion,
    JavaString,
};
use serde::{Deserialize, Serialize};

use super::holder_set::DialogRegistryHolderReferenceState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogPotionContentsState {
    #[serde(default)]
    pub potion: Option<DialogRegistryHolderReferenceState>,
    #[serde(default)]
    pub custom_color: Option<i32>,
    #[serde(default)]
    pub custom_effects: Vec<DialogMobEffectInstanceState>,
    #[serde(default)]
    pub custom_name: Option<JavaString>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogMobEffectInstanceState {
    pub effect: DialogRegistryHolderReferenceState,
    pub details: MobEffectDetailsSummary,
}

pub(super) fn project_potion_contents(
    patch: &DataComponentPatchSummary,
) -> DialogPotionContentsState {
    DialogPotionContentsState {
        potion: patch.potion_contents.potion_id.map(|id| {
            let potion = vanilla_potion(id).unwrap();
            DialogRegistryHolderReferenceState {
                holder_id: id,
                resource_id: potion.resource_id.to_string(),
            }
        }),
        custom_color: patch.potion_contents.custom_color,
        custom_effects: patch
            .potion_contents
            .custom_effects
            .iter()
            .map(project_mob_effect_instance)
            .collect(),
        custom_name: patch.potion_contents.custom_name.clone(),
    }
}

pub(super) fn project_mob_effect_instance(
    value: &MobEffectInstanceSummary,
) -> DialogMobEffectInstanceState {
    let effect = vanilla_mob_effect_type(value.effect_id).unwrap();
    DialogMobEffectInstanceState {
        effect: DialogRegistryHolderReferenceState {
            holder_id: value.effect_id,
            resource_id: effect.resource_id.to_string(),
        },
        details: MobEffectDetailsSummary {
            amplifier: value.amplifier,
            duration: value.duration,
            ambient: value.ambient,
            show_particles: value.show_particles,
            show_icon: value.show_icon,
            hidden_effect: value.hidden_effect.clone(),
        },
    }
}

#[cfg(test)]
mod tests;
