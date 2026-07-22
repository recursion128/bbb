use bbb_protocol::{
    equipment_slots::vanilla_equipment_slot_for_serialized_name, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::RegistrySet;
use crate::{registries::normalize_codec_identifier, FrozenRegistryAccess};

use super::{
    find_entry,
    holder_set::{decode_registry_holder_set, DialogRegistryHolderSetState},
    optional_component_bool,
    sound_event::{decode_sound_event_holder, DialogSoundEventHolderState},
};

const ENTITY_TYPE_REGISTRY: &str = "minecraft:entity_type";
const DEFAULT_EQUIP_SOUND: &str = "minecraft:item.armor.equip_generic";
const DEFAULT_SHEARING_SOUND: &str = "minecraft:item.shears.snip";

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

pub(super) fn decode_equippable(
    value: &NbtSummaryValue,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<DialogEquippableState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let slot_value =
        find_entry(entries, "slot").ok_or_else(|| format!("{what} is missing field slot"))?;
    let NbtSummaryValue::String(raw_slot) = slot_value else {
        return Err(format!("{what} field slot is not an equipment-slot name"));
    };
    let slot = vanilla_equipment_slot_for_serialized_name(raw_slot)
        .ok_or_else(|| format!("unknown {what} equipment slot {raw_slot}"))?;

    Ok(DialogEquippableState {
        slot_id: slot.id,
        slot: slot.serialized_name.to_string(),
        equip_sound: sound_event_field(
            entries,
            "equip_sound",
            DEFAULT_EQUIP_SOUND,
            registries,
            what,
        )?,
        asset_id: optional_resource_id(entries, "asset_id", what)?,
        camera_overlay: optional_resource_id(entries, "camera_overlay", what)?,
        allowed_entities: find_entry(entries, "allowed_entities")
            .map(|value| {
                decode_registry_holder_set(
                    value,
                    ENTITY_TYPE_REGISTRY,
                    registries,
                    &format!("{what} field allowed_entities"),
                )
            })
            .transpose()?,
        dispensable: optional_component_bool(entries, "dispensable", true, what)?,
        swappable: optional_component_bool(entries, "swappable", true, what)?,
        damage_on_hurt: optional_component_bool(entries, "damage_on_hurt", true, what)?,
        equip_on_interact: optional_component_bool(entries, "equip_on_interact", false, what)?,
        can_be_sheared: optional_component_bool(entries, "can_be_sheared", false, what)?,
        shearing_sound: sound_event_field(
            entries,
            "shearing_sound",
            DEFAULT_SHEARING_SOUND,
            registries,
            what,
        )?,
    })
}

fn sound_event_field(
    entries: &[NbtSummaryEntry],
    field: &str,
    default: &str,
    registries: Option<FrozenRegistryAccess<'_>>,
    what: &str,
) -> Result<DialogSoundEventHolderState, String> {
    let default_value;
    let value = match find_entry(entries, field) {
        Some(value) => value,
        None => {
            default_value = NbtSummaryValue::String(default.to_string());
            &default_value
        }
    };
    decode_sound_event_holder(value, registries, &format!("{what} field {field}"))
}

fn optional_resource_id(
    entries: &[NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<Option<String>, String> {
    let Some(value) = find_entry(entries, field) else {
        return Ok(None);
    };
    let NbtSummaryValue::String(raw) = value else {
        return Err(format!("{what} field {field} is not a resource id"));
    };
    normalize_codec_identifier(raw)
        .map(Some)
        .ok_or_else(|| format!("invalid {what} field {field} resource id {raw}"))
}

#[cfg(test)]
mod tests;
