use std::collections::BTreeSet;

use bbb_protocol::{decode_nbt_summary, packets::EquipmentSlot, NbtSummaryEntry, NbtSummaryValue};

use crate::{Result, WorldDecodeError};

use super::{
    TrialSpawnerBlockEntityData, TrialSpawnerEntityNbtProjection,
    TrialSpawnerEquipmentNbtProjection,
};

/// Decodes the update-tag subset written by
/// `TrialSpawnerStateData.getUpdateTag`. The `SpawnData` wrapper is discarded:
/// only its `entity` compound is ever passed to
/// `EntityType.loadEntityRecursive` for the client display entity.
pub(crate) fn decode_trial_spawner_block_entity_data(
    raw_nbt: &[u8],
) -> Result<Option<TrialSpawnerBlockEntityData>> {
    let Some(root) = decode_nbt_summary(raw_nbt).map_err(WorldDecodeError::Protocol)? else {
        return Ok(None);
    };
    let NbtSummaryValue::Compound(entries) = root else {
        return Ok(None);
    };
    let spawn_entity = compound_field(&entries, "spawn_data")
        .and_then(|spawn_data| compound_field(spawn_data, "entity"))
        .map(decode_entity_projection);
    let next_mob_spawns_at = integer_field(&entries, "next_mob_spawns_at").unwrap_or(0);
    Ok(Some(TrialSpawnerBlockEntityData {
        spawn_entity,
        next_mob_spawns_at,
    }))
}

fn decode_entity_projection(entries: &[NbtSummaryEntry]) -> TrialSpawnerEntityNbtProjection {
    TrialSpawnerEntityNbtProjection {
        entity_id: string_field(entries, "id").map(str::to_owned),
        rotation_bits: rotation_bits(entries),
        glowing: boolean_field(entries, "Glowing").unwrap_or(false),
        ticks_frozen: integer_field(entries, "TicksFrozen")
            .unwrap_or(0)
            .clamp(0, i64::from(i32::MAX)) as i32,
        fall_flying: boolean_field(entries, "FallFlying").unwrap_or(false),
        baby: boolean_field(entries, "IsBaby").unwrap_or(false)
            || integer_field(entries, "Age").is_some_and(|age| age < 0),
        left_handed: boolean_field(entries, "LeftHanded").unwrap_or(false),
        equipment: equipment(entries),
    }
}

fn equipment(entries: &[NbtSummaryEntry]) -> Vec<TrialSpawnerEquipmentNbtProjection> {
    let Some(entries) = compound_field(entries, "equipment") else {
        return Vec::new();
    };
    if entries.len() > 8 {
        return Vec::new();
    }
    let mut seen = BTreeSet::new();
    let mut equipment = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(slot) = equipment_slot(&entry.name) else {
            return Vec::new();
        };
        if !seen.insert(slot) {
            return Vec::new();
        }
        equipment.push(TrialSpawnerEquipmentNbtProjection {
            slot,
            item: entry.value.clone(),
        });
    }
    equipment.sort_by_key(|entry| entry.slot.ordinal());
    equipment
}

fn equipment_slot(name: &str) -> Option<EquipmentSlot> {
    match name {
        "mainhand" => Some(EquipmentSlot::MainHand),
        "offhand" => Some(EquipmentSlot::OffHand),
        "feet" => Some(EquipmentSlot::Feet),
        "legs" => Some(EquipmentSlot::Legs),
        "chest" => Some(EquipmentSlot::Chest),
        "head" => Some(EquipmentSlot::Head),
        "body" => Some(EquipmentSlot::Body),
        "saddle" => Some(EquipmentSlot::Saddle),
        _ => None,
    }
}

fn rotation_bits(entries: &[NbtSummaryEntry]) -> [u32; 2] {
    let Some(NbtSummaryValue::List(values)) = field(entries, "Rotation") else {
        return [0; 2];
    };
    let [y_rot, x_rot] = values.as_slice() else {
        return [0; 2];
    };
    let Some(y_rot) = numeric_f32(y_rot) else {
        return [0; 2];
    };
    let Some(x_rot) = numeric_f32(x_rot) else {
        return [0; 2];
    };
    [y_rot.to_bits(), x_rot.to_bits()]
}

fn numeric_f32(value: &NbtSummaryValue) -> Option<f32> {
    match value {
        NbtSummaryValue::Byte(value) => Some(f32::from(*value)),
        NbtSummaryValue::Short(value) => Some(f32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value as f32),
        NbtSummaryValue::Long(value) => Some(*value as f32),
        NbtSummaryValue::Float(value) => Some(f32::from_bits(*value)),
        NbtSummaryValue::Double(value) => Some(f64::from_bits(*value) as f32),
        _ => None,
    }
}

fn boolean_field(entries: &[NbtSummaryEntry], name: &str) -> Option<bool> {
    match field(entries, name)? {
        NbtSummaryValue::Byte(value) => Some(*value != 0),
        _ => None,
    }
}

fn string_field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a str> {
    match field(entries, name)? {
        NbtSummaryValue::String(value) => Some(value),
        _ => None,
    }
}

fn compound_field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a [NbtSummaryEntry]> {
    match field(entries, name)? {
        NbtSummaryValue::Compound(entries) => Some(entries),
        _ => None,
    }
}

fn integer_field(entries: &[NbtSummaryEntry], name: &str) -> Option<i64> {
    match field(entries, name)? {
        NbtSummaryValue::Byte(value) => Some(i64::from(*value)),
        NbtSummaryValue::Short(value) => Some(i64::from(*value)),
        NbtSummaryValue::Int(value) => Some(i64::from(*value)),
        NbtSummaryValue::Long(value) => Some(*value),
        _ => None,
    }
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_lowercase_trial_spawner_update_tag_fields() {
        let raw_nbt = compound(&[
            named_compound(
                "spawn_data",
                &[named_compound(
                    "entity",
                    &[
                        named_string("id", "minecraft:zombie"),
                        named_byte("IsBaby", 1),
                    ],
                )],
            ),
            named_long("next_mob_spawns_at", 1234),
        ]);

        let decoded = decode_trial_spawner_block_entity_data(&raw_nbt)
            .unwrap()
            .unwrap();
        assert_eq!(decoded.next_mob_spawns_at, 1234);
        let entity = decoded.spawn_entity.unwrap();
        assert_eq!(entity.entity_id.as_deref(), Some("minecraft:zombie"));
        assert!(entity.baby);
    }

    #[test]
    fn missing_or_wrong_typed_update_fields_use_vanilla_defaults() {
        let raw_nbt = compound(&[
            named_string("spawn_data", "invalid"),
            named_string("next_mob_spawns_at", "invalid"),
        ]);

        let decoded = decode_trial_spawner_block_entity_data(&raw_nbt)
            .unwrap()
            .unwrap();
        assert_eq!(decoded.spawn_entity, None);
        assert_eq!(decoded.next_mob_spawns_at, 0);
    }

    fn compound(entries: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![10];
        for entry in entries {
            out.extend_from_slice(entry);
        }
        out.push(0);
        out
    }

    fn named_compound(name: &str, entries: &[Vec<u8>]) -> Vec<u8> {
        let mut out = named_tag(10, name);
        for entry in entries {
            out.extend_from_slice(entry);
        }
        out.push(0);
        out
    }

    fn named_string(name: &str, value: &str) -> Vec<u8> {
        let mut out = named_tag(8, name);
        write_string(&mut out, value);
        out
    }

    fn named_byte(name: &str, value: i8) -> Vec<u8> {
        let mut out = named_tag(1, name);
        out.push(value as u8);
        out
    }

    fn named_long(name: &str, value: i64) -> Vec<u8> {
        let mut out = named_tag(4, name);
        out.extend_from_slice(&value.to_be_bytes());
        out
    }

    fn named_tag(tag: u8, name: &str) -> Vec<u8> {
        let mut out = vec![tag];
        write_string(&mut out, name);
        out
    }

    fn write_string(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }
}
