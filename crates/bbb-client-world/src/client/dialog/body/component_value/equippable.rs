use super::{
    holder_set::{project_registry_holder_set, DialogRegistryHolderSetState},
    sound_event::{project_sound_event_holder, DialogSoundEventHolderState},
};
use crate::FrozenRegistryAccess;
#[cfg(test)]
use crate::RegistrySet;
use bbb_protocol::{equipment_slots::vanilla_equipment_slot_by_id, packets::EquippableSummary};
use serde::{Deserialize, Serialize};

const ENTITY_TYPE_REGISTRY: &str = "minecraft:entity_type";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogEquippableState {
    pub slot_id: i32,
    pub slot: String,
    pub equip_sound: DialogSoundEventHolderState,
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub camera_overlay: Option<String>,
    #[serde(default)]
    pub allowed_entities: Option<DialogRegistryHolderSetState>,
    pub dispensable: bool,
    pub swappable: bool,
    pub damage_on_hurt: bool,
    pub equip_on_interact: bool,
    pub can_be_sheared: bool,
    pub shearing_sound: DialogSoundEventHolderState,
}

pub(super) fn project_equippable(
    value: &EquippableSummary,
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<DialogEquippableState, String> {
    Ok(DialogEquippableState {
        slot_id: value.slot_id,
        slot: vanilla_equipment_slot_by_id(value.slot_id)
            .unwrap()
            .serialized_name
            .to_string(),
        equip_sound: project_sound_event_holder(&value.equip_sound, registries)?,
        asset_id: value.asset_id.clone(),
        camera_overlay: value.camera_overlay.clone(),
        allowed_entities: value
            .allowed_entities
            .as_ref()
            .map(|entities| {
                project_registry_holder_set(
                    &entities.registry_ids,
                    &entities.tag,
                    ENTITY_TYPE_REGISTRY,
                    registries,
                )
            })
            .transpose()?,
        dispensable: value.dispensable,
        swappable: value.swappable,
        damage_on_hurt: value.damage_on_hurt,
        equip_on_interact: value.equip_on_interact,
        can_be_sheared: value.can_be_sheared,
        shearing_sound: project_sound_event_holder(&value.shearing_sound, registries)?,
    })
}

#[cfg(test)]
mod tests;
