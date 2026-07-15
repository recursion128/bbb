use crate::{
    chunks::{decode_nbt_root, find_nbt_entry, NbtValue},
    WorldStore,
};

use super::normalize_codec_identifier;

pub(crate) const CAT_SOUND_VARIANT_REGISTRY: &str = "minecraft:cat_sound_variant";
pub(crate) const CHICKEN_SOUND_VARIANT_REGISTRY: &str = "minecraft:chicken_sound_variant";
pub(crate) const COW_SOUND_VARIANT_REGISTRY: &str = "minecraft:cow_sound_variant";
pub(crate) const PIG_SOUND_VARIANT_REGISTRY: &str = "minecraft:pig_sound_variant";
pub(crate) const WOLF_SOUND_VARIANT_REGISTRY: &str = "minecraft:wolf_sound_variant";

const CAT_SOUND_FIELDS: &[&str] = &[
    "ambient_sound",
    "stray_ambient_sound",
    "hiss_sound",
    "hurt_sound",
    "death_sound",
    "eat_sound",
    "beg_for_food_sound",
    "purr_sound",
    "purreow_sound",
];
const CHICKEN_SOUND_FIELDS: &[&str] = &["ambient_sound", "hurt_sound", "death_sound", "step_sound"];
const COW_SOUND_FIELDS: &[&str] = &["ambient_sound", "hurt_sound", "death_sound", "step_sound"];
const PIG_SOUND_FIELDS: &[&str] = &[
    "ambient_sound",
    "hurt_sound",
    "death_sound",
    "step_sound",
    "eat_sound",
];
const WOLF_SOUND_FIELDS: &[&str] = &[
    "ambient_sound",
    "death_sound",
    "growl_sound",
    "hurt_sound",
    "pant_sound",
    "whine_sound",
    "step_sound",
];

/// A typed `SoundEvent.CODEC` holder from one synchronized sound-variant element.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RegistrySoundEvent {
    Named {
        resource_id: String,
    },
    Direct {
        location: String,
        fixed_range: Option<f32>,
    },
}

/// Identifies the exact NETWORK_CODEC field that could not be projected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SoundVariantProjectionError {
    pub field: String,
    pub reason: &'static str,
}

impl WorldStore {
    /// Decodes a synchronized sound-variant element in current server holder order.
    /// Known-pack stubs remain `None` so the consumer can apply source-aware fallback.
    pub(crate) fn synchronized_sound_variant_death_sound(
        &self,
        registry: &str,
        holder_id: i32,
        baby: bool,
    ) -> Result<Option<RegistrySoundEvent>, SoundVariantProjectionError> {
        let Some(entry) = self.registry_entry_by_holder_id(registry, holder_id) else {
            return Ok(None);
        };
        let Some(raw_data) = entry.raw_data() else {
            return Ok(None);
        };
        decode_sound_variant_death_sound(registry, raw_data, baby).map(Some)
    }
}

fn decode_sound_variant_death_sound(
    registry: &str,
    raw_data: &[u8],
    baby: bool,
) -> Result<RegistrySoundEvent, SoundVariantProjectionError> {
    let root = decode_nbt_root(raw_data)
        .map_err(|_| error("<root>", "invalid NBT"))?
        .ok_or_else(|| error("<root>", "missing root compound"))?;
    let NbtValue::Compound(fields) = root else {
        return Err(error("<root>", "expected compound"));
    };
    match registry {
        COW_SOUND_VARIANT_REGISTRY => decode_sound_set(&fields, COW_SOUND_FIELDS, ""),
        CAT_SOUND_VARIANT_REGISTRY => decode_age_sound_sets(&fields, CAT_SOUND_FIELDS, baby),
        CHICKEN_SOUND_VARIANT_REGISTRY => {
            decode_age_sound_sets(&fields, CHICKEN_SOUND_FIELDS, baby)
        }
        PIG_SOUND_VARIANT_REGISTRY => decode_age_sound_sets(&fields, PIG_SOUND_FIELDS, baby),
        WOLF_SOUND_VARIANT_REGISTRY => decode_age_sound_sets(&fields, WOLF_SOUND_FIELDS, baby),
        _ => Err(error("<registry>", "unsupported sound-variant registry")),
    }
}

fn decode_age_sound_sets(
    fields: &[(String, NbtValue)],
    required_sound_fields: &[&str],
    baby: bool,
) -> Result<RegistrySoundEvent, SoundVariantProjectionError> {
    let adult = required_compound(fields, "adult_sounds", "adult_sounds")?;
    let baby_fields = required_compound(fields, "baby_sounds", "baby_sounds")?;
    let adult_death = decode_sound_set(adult, required_sound_fields, "adult_sounds")?;
    let baby_death = decode_sound_set(baby_fields, required_sound_fields, "baby_sounds")?;
    Ok(if baby { baby_death } else { adult_death })
}

fn decode_sound_set(
    fields: &[(String, NbtValue)],
    required_sound_fields: &[&str],
    prefix: &str,
) -> Result<RegistrySoundEvent, SoundVariantProjectionError> {
    let mut death_sound = None;
    for field in required_sound_fields {
        let path = field_path(prefix, field);
        let value = find_nbt_entry(fields, field)
            .ok_or_else(|| error(path.clone(), "missing required field"))?;
        let sound = decode_sound_event_holder(value, &path)?;
        if *field == "death_sound" {
            death_sound = Some(sound);
        }
    }
    death_sound.ok_or_else(|| error(field_path(prefix, "death_sound"), "missing required field"))
}

fn decode_sound_event_holder(
    value: &NbtValue,
    field: &str,
) -> Result<RegistrySoundEvent, SoundVariantProjectionError> {
    match value {
        NbtValue::String(resource_id) => Ok(RegistrySoundEvent::Named {
            resource_id: normalize_codec_identifier(resource_id)
                .ok_or_else(|| error(field, "invalid sound event identifier"))?,
        }),
        NbtValue::Compound(fields) => {
            let sound_id_field = field_path(field, "sound_id");
            let sound_id = match find_nbt_entry(fields, "sound_id") {
                Some(NbtValue::String(sound_id)) => normalize_codec_identifier(sound_id)
                    .ok_or_else(|| error(&sound_id_field, "invalid sound event identifier"))?,
                Some(_) => return Err(error(sound_id_field, "expected string")),
                None => return Err(error(sound_id_field, "missing required field")),
            };
            let range_field = field_path(field, "range");
            let fixed_range = match find_nbt_entry(fields, "range") {
                Some(value) => Some(
                    nbt_float(value).ok_or_else(|| error(range_field, "expected numeric range"))?,
                ),
                None => None,
            };
            Ok(RegistrySoundEvent::Direct {
                location: sound_id,
                fixed_range,
            })
        }
        _ => Err(error(field, "expected named or direct sound event holder")),
    }
}

fn required_compound<'a>(
    fields: &'a [(String, NbtValue)],
    field: &str,
    path: &str,
) -> Result<&'a [(String, NbtValue)], SoundVariantProjectionError> {
    match find_nbt_entry(fields, field) {
        Some(NbtValue::Compound(fields)) => Ok(fields),
        Some(_) => Err(error(path, "expected compound")),
        None => Err(error(path, "missing required field")),
    }
}

fn nbt_float(value: &NbtValue) -> Option<f32> {
    match value {
        NbtValue::Byte(value) => Some(f32::from(*value)),
        NbtValue::Short(value) => Some(f32::from(*value)),
        NbtValue::Int(value) => Some(*value as f32),
        NbtValue::Long(value) => Some(*value as f32),
        NbtValue::Float(value) => Some(*value),
        NbtValue::Double(value) => Some(*value as f32),
        _ => None,
    }
}

fn field_path(prefix: &str, field: &str) -> String {
    if prefix.is_empty() {
        field.to_string()
    } else {
        format!("{prefix}.{field}")
    }
}

fn error(field: impl Into<String>, reason: &'static str) -> SoundVariantProjectionError {
    SoundVariantProjectionError {
        field: field.into(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;

    #[test]
    fn decodes_all_synchronized_sound_variant_shapes_and_holder_forms() {
        let cases = [
            (CAT_SOUND_VARIANT_REGISTRY, CAT_SOUND_FIELDS, true),
            (CHICKEN_SOUND_VARIANT_REGISTRY, CHICKEN_SOUND_FIELDS, true),
            (COW_SOUND_VARIANT_REGISTRY, COW_SOUND_FIELDS, false),
            (PIG_SOUND_VARIANT_REGISTRY, PIG_SOUND_FIELDS, true),
            (WOLF_SOUND_VARIANT_REGISTRY, WOLF_SOUND_FIELDS, true),
        ];

        for (registry, fields, age_sets) in cases {
            let raw_data = sound_variant_nbt(fields, age_sets);
            assert_eq!(
                decode_sound_variant_death_sound(registry, &raw_data, false).unwrap(),
                RegistrySoundEvent::Named {
                    resource_id: "minecraft:entity.example.adult_death".to_string(),
                }
            );
            let expected_baby = if age_sets {
                RegistrySoundEvent::Direct {
                    location: "example:baby_death".to_string(),
                    fixed_range: Some(24.0),
                }
            } else {
                RegistrySoundEvent::Named {
                    resource_id: "minecraft:entity.example.adult_death".to_string(),
                }
            };
            assert_eq!(
                decode_sound_variant_death_sound(registry, &raw_data, true).unwrap(),
                expected_baby
            );
        }
    }

    #[test]
    fn rejects_incomplete_or_malformed_sound_variant_payloads() {
        let mut missing_field = sound_variant_nbt(CHICKEN_SOUND_FIELDS, true);
        let field = b"\x00\x0astep_sound";
        let offset = missing_field
            .windows(field.len())
            .position(|window| window == field)
            .unwrap();
        missing_field[offset + 2] = b'x';
        assert_eq!(
            decode_sound_variant_death_sound(
                CHICKEN_SOUND_VARIANT_REGISTRY,
                &missing_field,
                false,
            )
            .unwrap_err(),
            SoundVariantProjectionError {
                field: "adult_sounds.step_sound".to_string(),
                reason: "missing required field",
            }
        );

        let mut invalid_identifier = sound_variant_nbt(COW_SOUND_FIELDS, false);
        let identifier = b"entity.example.adult_death";
        let offset = invalid_identifier
            .windows(identifier.len())
            .position(|window| window == identifier)
            .unwrap();
        invalid_identifier[offset] = b'E';
        assert_eq!(
            decode_sound_variant_death_sound(
                COW_SOUND_VARIANT_REGISTRY,
                &invalid_identifier,
                false,
            )
            .unwrap_err(),
            SoundVariantProjectionError {
                field: "death_sound".to_string(),
                reason: "invalid sound event identifier",
            }
        );
    }

    #[test]
    fn world_queries_use_server_holder_order_for_every_sound_variant_shape() {
        let cases = [
            (CAT_SOUND_VARIANT_REGISTRY, CAT_SOUND_FIELDS, true),
            (CHICKEN_SOUND_VARIANT_REGISTRY, CHICKEN_SOUND_FIELDS, true),
            (COW_SOUND_VARIANT_REGISTRY, COW_SOUND_FIELDS, false),
            (PIG_SOUND_VARIANT_REGISTRY, PIG_SOUND_FIELDS, true),
            (WOLF_SOUND_VARIANT_REGISTRY, WOLF_SOUND_FIELDS, true),
        ];

        for (registry, fields, age_sets) in cases {
            let mut store = WorldStore::new();
            store.record_registry_entries(
                registry,
                0,
                vec![
                    RegistryPacketEntry::with_raw_data(
                        "example:custom",
                        sound_variant_nbt(fields, age_sets),
                    ),
                    RegistryPacketEntry::stub("minecraft:classic"),
                ],
            );

            let expected = if age_sets {
                RegistrySoundEvent::Direct {
                    location: "example:baby_death".to_string(),
                    fixed_range: Some(24.0),
                }
            } else {
                RegistrySoundEvent::Named {
                    resource_id: "minecraft:entity.example.adult_death".to_string(),
                }
            };
            assert_eq!(
                store
                    .synchronized_sound_variant_death_sound(registry, 0, true)
                    .unwrap(),
                Some(expected),
                "{registry}"
            );
            assert_eq!(
                store
                    .synchronized_sound_variant_death_sound(registry, 1, false)
                    .unwrap(),
                None,
                "{registry}"
            );
        }
    }

    fn sound_variant_nbt(fields: &[&str], age_sets: bool) -> Vec<u8> {
        let mut out = vec![10];
        if age_sets {
            begin_compound(&mut out, "adult_sounds");
            write_sound_set(&mut out, fields, false);
            out.push(0);
            begin_compound(&mut out, "baby_sounds");
            write_sound_set(&mut out, fields, true);
            out.push(0);
        } else {
            write_sound_set(&mut out, fields, false);
        }
        out.push(0);
        out
    }

    fn write_sound_set(out: &mut Vec<u8>, fields: &[&str], baby: bool) {
        for field in fields {
            if *field == "death_sound" && baby {
                begin_compound(out, field);
                write_string(out, "sound_id", "example:baby_death");
                write_float(out, "range", 24.0);
                out.push(0);
            } else {
                let resource = if *field == "death_sound" {
                    "entity.example.adult_death"
                } else {
                    "minecraft:entity.example.other"
                };
                write_string(out, field, resource);
            }
        }
    }

    fn begin_compound(out: &mut Vec<u8>, name: &str) {
        out.push(10);
        write_utf(out, name);
    }

    fn write_float(out: &mut Vec<u8>, name: &str, value: f32) {
        out.push(5);
        write_utf(out, name);
        out.extend_from_slice(&value.to_be_bytes());
    }

    fn write_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_utf(out, name);
        write_utf(out, value);
    }

    fn write_utf(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
}
