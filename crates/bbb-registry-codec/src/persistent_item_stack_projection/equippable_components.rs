//! `minecraft:equippable`: `Equippable.CODEC` (`DataComponents.java:192-194`,
//! `Equippable`), a record whose slot is an enum name, whose two
//! sounds are `RegistryFileCodec` holders, whose `asset_id` is a
//! `ResourceKey` -- which writes its identifier and resolves no member -- and
//! whose `allowed_entities` is a holder set.
//!
//! `Registries.ENTITY_TYPE` is built in, so the holder set needs no
//! publication and the generated catalog issues the very ids the wire carried,
//! while `Equippable` shows one field the save form does *not*
//! carry: the `Equippable` record itself.
//!
//! The two sounds default to a *named holder*, so the save form omits them
//! exactly while the summary names that same registry entry
//! a direct sound with the same id is a
//! different holder and is written out.

use bbb_facts::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID;
use bbb_facts::equipment_slots::{
    vanilla_equipment_slot_by_id, vanilla_equipment_slot_for_serialized_name,
};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::nbt_summary::entry as field;
use bbb_protocol::packets::{DataComponentPatchSummary, EquippableSummary, HolderSetSummary};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::error::{invalid_persistent_component, Result};
use super::holder::{
    entity_id_in_registry, entity_resource_id_in_registry, persistent_sound_event_nbt,
    project_sound_event_with_content_registry, sound_event_resource_id_in_registry,
    ENTITY_TYPE_REGISTRY, SOUND_EVENT_REGISTRY,
};
use super::nbt_support::{
    compound_fields, latest_field, nbt_string, normalize_resource_id,
    persistent_compact_holder_set_nbt, persistent_flag_nbt, project_compact_holder_set,
    ComponentReader,
};

/// `SoundEvents.ARMOR_EQUIP_GENERIC` (`Equippable.java:48`).
const DEFAULT_EQUIP_SOUND: &str = "minecraft:item.armor.equip_generic";
/// `SoundEvents.SHEARS_SNIP` (`Equippable.java:57-59`).
const DEFAULT_SHEARING_SOUND: &str = "minecraft:item.shears.snip";

/// One of the two sound fields: written only while the summary does *not* name
/// the codec's default entry, because `optionalFieldOf(name, default)` drops a
/// value equal to its default.
fn persistent_sound_field(
    type_id: i32,
    resource_id: &'static str,
    name: &str,
    sound: &bbb_protocol::packets::SoundEventSummary,
    default_resource_id: &str,
    content_registry: &ContentRegistry,
) -> Result<Option<NbtSummaryEntry>> {
    let is_default = sound.registry_id.is_some_and(|holder_id| {
        sound_event_resource_id_in_registry(holder_id, content_registry)
            == Some(default_resource_id)
    });
    if is_default {
        return Ok(None);
    }
    Ok(Some(field(
        name,
        persistent_sound_event_nbt(type_id, resource_id, sound, content_registry)?,
    )))
}

/// `RegistryCodecs.homogeneousList(Registries.ENTITY_TYPE)`
/// written as the compact holder-set form over the
/// built-in and active mod entity-type catalog.
fn persistent_allowed_entities(
    type_id: i32,
    resource_id: &'static str,
    entities: &HolderSetSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let names = entities.registry_ids.iter().map(|&entity_id| {
        entity_resource_id_in_registry(entity_id, content_registry).ok_or_else(|| {
            invalid_persistent_component(
                type_id,
                resource_id,
                format!("unknown {ENTITY_TYPE_REGISTRY} holder id {entity_id}"),
            )
        })
    });
    persistent_compact_holder_set_nbt(type_id, resource_id, entities.tag.as_deref(), names)
}

/// The persistent NBT `Equippable.CODEC` writes for one summary.
pub(super) fn persistent_equippable_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let equippable = patch.equippable.as_ref().ok_or(
        super::error::ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id,
        },
    )?;
    // `EquipmentSlot.CODEC` is `StringRepresentable.fromEnum`
    // so the save form is the serialized name.
    let slot = vanilla_equipment_slot_by_id(equippable.slot_id)
        .ok_or_else(|| invalid(format!("unknown equipment slot id {}", equippable.slot_id)))?;
    let mut fields = vec![field(
        "slot",
        NbtSummaryValue::String(slot.serialized_name.into()),
    )];
    if let Some(equip_sound) = persistent_sound_field(
        type_id,
        resource_id,
        "equip_sound",
        &equippable.equip_sound,
        DEFAULT_EQUIP_SOUND,
        content_registry,
    )? {
        fields.push(equip_sound);
    }
    // `ResourceKey.codec(EquipmentAssets.ROOT_ID)` and
    // `Identifier.CODEC` both write a bare resource
    // location and resolve no registry member.
    for (name, value) in [
        ("asset_id", equippable.asset_id.as_deref()),
        ("camera_overlay", equippable.camera_overlay.as_deref()),
    ] {
        if let Some(value) = value {
            fields.push(field(
                name,
                NbtSummaryValue::String(normalize_resource_id(value)?.into()),
            ));
        }
    }
    if let Some(entities) = &equippable.allowed_entities {
        fields.push(field(
            "allowed_entities",
            persistent_allowed_entities(type_id, resource_id, entities, content_registry)?,
        ));
    }
    // `Codec.BOOL.optionalFieldOf(name, default)` drops the field while it holds
    // its default.
    for (name, value, default) in [
        ("dispensable", equippable.dispensable, true),
        ("swappable", equippable.swappable, true),
        ("damage_on_hurt", equippable.damage_on_hurt, true),
        ("equip_on_interact", equippable.equip_on_interact, false),
        ("can_be_sheared", equippable.can_be_sheared, false),
    ] {
        if value != default {
            fields.push(field(name, persistent_flag_nbt(value)));
        }
    }
    if let Some(shearing_sound) = persistent_sound_field(
        type_id,
        resource_id,
        "shearing_sound",
        &equippable.shearing_sound,
        DEFAULT_SHEARING_SOUND,
        content_registry,
    )? {
        fields.push(shearing_sound);
    }
    Ok(NbtSummaryValue::Compound(fields))
}

fn named_default_sound(
    type_id: i32,
    resource_id: &'static str,
    default_resource_id: &str,
) -> Result<bbb_protocol::packets::SoundEventSummary> {
    let holder_id =
        bbb_facts::sound_event_types::vanilla_sound_event_registry_id(default_resource_id)
            .ok_or_else(|| {
                invalid_persistent_component(
                    type_id,
                    resource_id,
                    format!("unknown {SOUND_EVENT_REGISTRY} holder {default_resource_id}"),
                )
            })?;
    Ok(bbb_protocol::packets::SoundEventSummary {
        registry_id: Some(holder_id),
        sound_id: None,
        fixed_range_bits: None,
    })
}

fn project_flag(
    reader: ComponentReader,
    fields: &[NbtSummaryEntry],
    name: &str,
    default: bool,
) -> Result<bool> {
    latest_field(fields, name).map_or(Ok(default), |value| reader.flag(value, name))
}

fn project_resource_id(
    type_id: i32,
    resource_id: &'static str,
    fields: &[NbtSummaryEntry],
    name: &str,
) -> Result<Option<String>> {
    latest_field(fields, name)
        .map(|value| {
            let raw = nbt_string(value).ok_or_else(|| {
                invalid_persistent_component(
                    type_id,
                    resource_id,
                    format!("equippable {name} is not a resource id"),
                )
            })?;
            super::nbt_support::normalize_resource_id_units(raw.units())
        })
        .transpose()
}

pub(super) fn project_equippable(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    debug_assert_eq!(type_id, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID);
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let fields = compound_fields(value)
        .ok_or_else(|| invalid("equippable is not a compound".to_string()))?;
    let slot = nbt_string(
        latest_field(fields, "slot")
            .ok_or_else(|| invalid("equippable carries no slot".to_string()))?,
    )
    .ok_or_else(|| invalid("equippable slot is not a string".to_string()))?;
    let slot = slot
        .with_utf8(vanilla_equipment_slot_for_serialized_name)
        .flatten()
        .ok_or_else(|| invalid(format!("unknown equipment slot {slot}")))?;
    let sound = |name: &str, default: &str| -> Result<bbb_protocol::packets::SoundEventSummary> {
        match latest_field(fields, name) {
            Some(value) => project_sound_event_with_content_registry(
                type_id,
                resource_id,
                value,
                content_registry,
            ),
            None => named_default_sound(type_id, resource_id, default),
        }
    };
    let allowed_entities = match latest_field(fields, "allowed_entities") {
        Some(entities) => Some(project_allowed_entities(
            type_id,
            resource_id,
            entities,
            content_registry,
        )?),
        None => None,
    };
    let reader = ComponentReader::new(type_id, resource_id);
    patch.equippable = Some(EquippableSummary {
        slot_id: slot.id,
        equip_sound: sound("equip_sound", DEFAULT_EQUIP_SOUND)?,
        asset_id: project_resource_id(type_id, resource_id, fields, "asset_id")?,
        camera_overlay: project_resource_id(type_id, resource_id, fields, "camera_overlay")?,
        allowed_entities,
        dispensable: project_flag(reader, fields, "dispensable", true)?,
        swappable: project_flag(reader, fields, "swappable", true)?,
        damage_on_hurt: project_flag(reader, fields, "damage_on_hurt", true)?,
        equip_on_interact: project_flag(reader, fields, "equip_on_interact", false)?,
        can_be_sheared: project_flag(reader, fields, "can_be_sheared", false)?,
        shearing_sound: sound("shearing_sound", DEFAULT_SHEARING_SOUND)?,
    });
    Ok(())
}

fn project_allowed_entities(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Result<HolderSetSummary> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let entity_id = |raw: &bbb_protocol::JavaString| -> Result<i32> {
        let name = super::nbt_support::normalize_resource_id_units(raw.units())?;
        entity_id_in_registry(&name, content_registry)
            .ok_or_else(|| invalid(format!("unknown {ENTITY_TYPE_REGISTRY} holder {name}")))
    };
    let (registry_ids, tag) =
        project_compact_holder_set(ComponentReader::new(type_id, resource_id), value, entity_id)?;
    Ok(HolderSetSummary { registry_ids, tag })
}
