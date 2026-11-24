//! The components whose persistent codec is a registry holder, a holder set, or
//! a record built out of them, transcribed in both directions.
//!
//! Each one writes the identifier of a *named* holder, so the encode direction
//! needs the publication that issued the summary's holder ids
//! a holder the publication cannot name is a
//! refusal rather than a dropped component.

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::{DataComponentPatchSummary, PiercingWeaponSummary, SoundEventSummary};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::error::{invalid_persistent_component, ItemStackPersistentProjectionError, Result};
use super::holder::{
    item_id_in_registry, item_resource_id_in_registry, persistent_damage_type_holder_set_nbt,
    persistent_inline_trim_material_nbt, persistent_sound_event_nbt,
    project_damage_type_holder_set, project_inline_trim_material,
    project_sound_event_with_content_registry, registered_holder_name, resolve_holder_id,
    ITEM_REGISTRY, TRIM_MATERIAL_REGISTRY,
};
use super::nbt_support::{
    compound_fields, latest_field, nbt_string, normalize_resource_id_units,
    persistent_compact_holder_set_nbt, persistent_flag_nbt, project_compact_holder_set,
    ComponentReader,
};

/// `minecraft:break_sound` is `persistent(SoundEvent.CODEC)`
/// (`DataComponents.java:325-327`), the `RegistryFileCodec` holder form
/// [`persistent_sound_event_nbt`] writes.
pub(super) fn persistent_break_sound_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let sound = patch.break_sound.as_ref().ok_or(
        ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id,
        },
    )?;
    persistent_sound_event_nbt(type_id, resource_id, sound, content_registry)
}

pub(super) fn project_break_sound(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    patch.break_sound = Some(project_sound_event_with_content_registry(
        type_id,
        resource_id,
        value,
        content_registry,
    )?);
    Ok(())
}

/// `minecraft:provides_trim_material` is `persistent(TrimMaterial.CODEC)`
/// (`DataComponents.java:274-276`), a `RegistryFileCodec.create(Registries.
/// TRIM_MATERIAL, DIRECT_CODEC)`, so a named holder is
/// the bare identifier and an inline material is its record.
pub(super) fn persistent_provides_trim_material_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
) -> Result<NbtSummaryValue> {
    if let Some(material) = patch.provides_trim_material_direct.as_deref() {
        if patch.provides_trim_material_id.is_some() {
            return Err(invalid_persistent_component(
                type_id,
                resource_id,
                "trim material has named and inline holders".to_string(),
            ));
        }
        return persistent_inline_trim_material_nbt(type_id, resource_id, material);
    }
    let material = patch.provides_trim_material_id.ok_or(
        ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id,
        },
    )?;
    Ok(NbtSummaryValue::String(
        registered_holder_name(
            type_id,
            resource_id,
            TRIM_MATERIAL_REGISTRY,
            material,
            holder_resource_id,
        )?
        .into(),
    ))
}

pub(super) fn project_provides_trim_material(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let Some(named) = nbt_string(value) else {
        patch.provides_trim_material_direct = Some(Box::new(project_inline_trim_material(
            super::nbt_support::ComponentReader::new(type_id, resource_id),
            value,
        )?));
        return Ok(());
    };
    patch.provides_trim_material_id = Some(resolve_holder_id(
        TRIM_MATERIAL_REGISTRY,
        &normalize_resource_id_units(named.units())?,
        holder_id,
    )?);
    Ok(())
}

/// `minecraft:repairable` is `persistent(Repairable.CODEC)`
/// (`DataComponents.java:195-197`): one required `items` holder set over
/// `Registries.ITEM`.
///
/// `BuiltInRegistries.ITEM` is built in, so the generated catalog issues the
/// very ids the wire holder set carried and this needs no publication.
pub(super) fn persistent_repairable_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let names = patch.repairable_item_registry_ids.iter().map(|&item_id| {
        item_resource_id_in_registry(item_id, content_registry).ok_or_else(|| {
            invalid_persistent_component(
                type_id,
                resource_id,
                format!("unknown {ITEM_REGISTRY} holder id {item_id}"),
            )
        })
    });
    Ok(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "items".into(),
        value: persistent_compact_holder_set_nbt(
            type_id,
            resource_id,
            patch.repairable_item_tag.as_deref(),
            names,
        )?,
    }]))
}

pub(super) fn project_repairable(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let fields = compound_fields(value).ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            "repairable is not a compound".to_string(),
        )
    })?;
    let items = latest_field(fields, "items").ok_or_else(|| {
        invalid_persistent_component(
            type_id,
            resource_id,
            "repairable carries no items".to_string(),
        )
    })?;
    let (registry_ids, tag) =
        project_compact_holder_set(ComponentReader::new(type_id, resource_id), items, |raw| {
            item_holder_id(raw, content_registry)
        })?;
    patch.repairable_item_registry_ids = registry_ids;
    patch.repairable_item_tag = tag;
    Ok(())
}

/// `Registries.ITEM` is a built-in registry
/// (`BuiltInRegistries.ITEM.holderByNameCodec()`, `Item`), extended by the
/// active `ContentRegistry`.
fn item_holder_id(
    raw: &bbb_protocol::JavaString,
    content_registry: &ContentRegistry,
) -> Result<i32> {
    let item = normalize_resource_id_units(raw.units())?;
    item_id_in_registry(&item, content_registry).ok_or(
        ItemStackPersistentProjectionError::UnresolvedRegistryHolder {
            registry_id: ITEM_REGISTRY,
            resource_id: item,
        },
    )
}

/// `minecraft:damage_resistant` is `persistent(DamageResistant.CODEC)`
/// (`DataComponents.java:179-181`): one required `types` holder set over
/// `Registries.DAMAGE_TYPE`.
///
/// `minecraft:damage_type` is a datapack registry, so both directions name its
/// holders through the session publication and refuse by name when it cannot.
pub(super) fn persistent_damage_resistant_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    holder_resource_id: &dyn Fn(&str, i32) -> Option<String>,
) -> Result<NbtSummaryValue> {
    let types = persistent_damage_type_holder_set_nbt(
        type_id,
        resource_id,
        &patch.damage_resistant_type_registry_ids,
        patch.damage_resistant_type_tag.as_deref(),
        holder_resource_id,
    )?;
    Ok(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "types".into(),
        value: types,
    }]))
}

pub(super) fn project_damage_resistant(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    holder_id: &dyn Fn(&'static str, &str) -> Option<i32>,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let invalid = |detail: String| invalid_persistent_component(type_id, resource_id, detail);
    let fields = compound_fields(value)
        .ok_or_else(|| invalid("damage_resistant is not a compound".to_string()))?;
    let types = latest_field(fields, "types")
        .ok_or_else(|| invalid("damage_resistant carries no types".to_string()))?;
    let set = project_damage_type_holder_set(type_id, resource_id, types, holder_id)?;
    patch.damage_resistant_type_registry_ids = set.registry_ids;
    patch.damage_resistant_type_tag = set.tag;
    Ok(())
}

/// `minecraft:piercing_weapon` is `persistent(PiercingWeapon.CODEC)`
/// (`DataComponents.java:208-210`), four `optionalFieldOf` fields
/// `deals_knockback` defaults to `true`,
/// `dismounts` to `false`, and the two sounds to an empty optional, so each is
/// dropped while it holds its default.
pub(super) fn persistent_piercing_weapon_nbt(
    type_id: i32,
    resource_id: &'static str,
    patch: &DataComponentPatchSummary,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue> {
    let weapon = patch.piercing_weapon.as_ref().ok_or(
        ItemStackPersistentProjectionError::UnsupportedComponent {
            type_id,
            resource_id,
        },
    )?;
    let mut fields = Vec::new();
    if !weapon.deals_knockback {
        fields.push(NbtSummaryEntry {
            name: "deals_knockback".into(),
            value: persistent_flag_nbt(false),
        });
    }
    if weapon.dismounts {
        fields.push(NbtSummaryEntry {
            name: "dismounts".into(),
            value: persistent_flag_nbt(true),
        });
    }
    for (name, sound) in [("sound", &weapon.sound), ("hit_sound", &weapon.hit_sound)] {
        if let Some(sound) = sound {
            fields.push(NbtSummaryEntry {
                name: name.into(),
                value: persistent_sound_event_nbt(type_id, resource_id, sound, content_registry)?,
            });
        }
    }
    Ok(NbtSummaryValue::Compound(fields))
}

pub(super) fn project_piercing_weapon(
    type_id: i32,
    resource_id: &'static str,
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
    patch: &mut DataComponentPatchSummary,
) -> Result<()> {
    let c = ComponentReader::new(type_id, resource_id);
    let fields = c.compound(value, "piercing weapon")?;
    let sound = |name: &str| -> Result<Option<SoundEventSummary>> {
        latest_field(fields, name)
            .map(|value| {
                project_sound_event_with_content_registry(
                    type_id,
                    resource_id,
                    value,
                    content_registry,
                )
            })
            .transpose()
    };
    patch.piercing_weapon = Some(PiercingWeaponSummary {
        deals_knockback: c.optional_flag(fields, "deals_knockback", true)?,
        dismounts: c.optional_flag(fields, "dismounts", false)?,
        sound: sound("sound")?,
        hit_sound: sound("hit_sound")?,
    });
    Ok(())
}
