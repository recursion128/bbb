use std::collections::BTreeMap;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::{FrozenRegistryAccess, RegistryElementSource, WorldStore};

use super::{decode_persistent_sound_event, PersistentSoundEvent};

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

/// Complete NETWORK_CODEC projection shared by the five synchronized animal
/// sound-variant registries. Family-specific fields remain `None`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RegistrySoundSet {
    ambient_sound: RegistrySoundEvent,
    stray_ambient_sound: Option<RegistrySoundEvent>,
    hiss_sound: Option<RegistrySoundEvent>,
    hurt_sound: RegistrySoundEvent,
    death_sound: RegistrySoundEvent,
    step_sound: Option<RegistrySoundEvent>,
    eat_sound: Option<RegistrySoundEvent>,
    beg_for_food_sound: Option<RegistrySoundEvent>,
    purr_sound: Option<RegistrySoundEvent>,
    purreow_sound: Option<RegistrySoundEvent>,
    growl_sound: Option<RegistrySoundEvent>,
    pant_sound: Option<RegistrySoundEvent>,
    whine_sound: Option<RegistrySoundEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RegistrySoundVariant {
    pub element_source: RegistryElementSource,
    adult_sounds: RegistrySoundSet,
    baby_sounds: Option<RegistrySoundSet>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistrySoundKind {
    Hurt,
    Death,
}

/// Identifies the exact NETWORK_CODEC field that could not be projected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SoundVariantProjectionError {
    pub field: String,
    pub reason: String,
}

impl WorldStore {
    /// Decodes a synchronized sound-variant element in current server holder order.
    /// Unresolved data-less stubs remain `None` so the consumer can apply source-aware fallback.
    pub(crate) fn synchronized_sound_variant_death_sound(
        &self,
        registry: &str,
        holder_id: i32,
        baby: bool,
    ) -> Result<Option<RegistrySoundEvent>, SoundVariantProjectionError> {
        self.synchronized_sound_variant_sound(registry, holder_id, baby, RegistrySoundKind::Death)
    }

    pub(crate) fn synchronized_sound_variant_hurt_sound(
        &self,
        registry: &str,
        holder_id: i32,
        baby: bool,
    ) -> Result<Option<RegistrySoundEvent>, SoundVariantProjectionError> {
        self.synchronized_sound_variant_sound(registry, holder_id, baby, RegistrySoundKind::Hurt)
    }

    fn synchronized_sound_variant_sound(
        &self,
        registry: &str,
        holder_id: i32,
        baby: bool,
        kind: RegistrySoundKind,
    ) -> Result<Option<RegistrySoundEvent>, SoundVariantProjectionError> {
        let Some(entry) = self.registry_entry_by_holder_id(registry, holder_id) else {
            return Ok(None);
        };
        if let Some(variant) = self
            .registries
            .typed
            .sound_variants
            .get(registry)
            .and_then(|entries| entries.get(&entry.id))
        {
            return Ok(Some(variant.sound(baby, kind).clone()));
        }
        let Some(data) = entry.element_data() else {
            return Ok(None);
        };
        decode_sound_variant_definition(registry, data.source, &data.value)
            .map(|variant| Some(variant.sound(baby, kind).clone()))
    }
}

impl RegistrySoundVariant {
    fn sound(&self, baby: bool, kind: RegistrySoundKind) -> &RegistrySoundEvent {
        let sounds = if baby {
            self.baby_sounds.as_ref().unwrap_or(&self.adult_sounds)
        } else {
            &self.adult_sounds
        };
        match kind {
            RegistrySoundKind::Hurt => &sounds.hurt_sound,
            RegistrySoundKind::Death => &sounds.death_sound,
        }
    }
}

#[cfg(test)]
fn decode_sound_variant_death_sound(
    registry: &str,
    value: &NbtSummaryValue,
    baby: bool,
) -> Result<RegistrySoundEvent, SoundVariantProjectionError> {
    decode_sound_variant_definition(registry, RegistryElementSource::NetworkNbt, value)
        .map(|variant| variant.sound(baby, RegistrySoundKind::Death).clone())
}

pub(super) fn decode_sound_variant_definition(
    registry: &str,
    element_source: RegistryElementSource,
    value: &NbtSummaryValue,
) -> Result<RegistrySoundVariant, SoundVariantProjectionError> {
    let NbtSummaryValue::Compound(fields) = value else {
        return Err(error("<root>", "expected compound"));
    };
    let (adult_sounds, baby_sounds) = match registry {
        COW_SOUND_VARIANT_REGISTRY => (decode_sound_set(fields, COW_SOUND_FIELDS, "")?, None),
        CAT_SOUND_VARIANT_REGISTRY => decode_age_sound_sets(fields, CAT_SOUND_FIELDS)?,
        CHICKEN_SOUND_VARIANT_REGISTRY => decode_age_sound_sets(fields, CHICKEN_SOUND_FIELDS)?,
        PIG_SOUND_VARIANT_REGISTRY => decode_age_sound_sets(fields, PIG_SOUND_FIELDS)?,
        WOLF_SOUND_VARIANT_REGISTRY => decode_age_sound_sets(fields, WOLF_SOUND_FIELDS)?,
        _ => return Err(error("<registry>", "unsupported sound-variant registry")),
    };
    Ok(RegistrySoundVariant {
        element_source,
        adult_sounds,
        baby_sounds,
    })
}

fn decode_age_sound_sets(
    fields: &[NbtSummaryEntry],
    required_sound_fields: &[&str],
) -> Result<(RegistrySoundSet, Option<RegistrySoundSet>), SoundVariantProjectionError> {
    let adult = required_compound(fields, "adult_sounds", "adult_sounds")?;
    let baby_fields = required_compound(fields, "baby_sounds", "baby_sounds")?;
    Ok((
        decode_sound_set(adult, required_sound_fields, "adult_sounds")?,
        Some(decode_sound_set(
            baby_fields,
            required_sound_fields,
            "baby_sounds",
        )?),
    ))
}

fn decode_sound_set(
    fields: &[NbtSummaryEntry],
    required_sound_fields: &[&str],
    prefix: &str,
) -> Result<RegistrySoundSet, SoundVariantProjectionError> {
    let mut decoded = BTreeMap::new();
    for field in required_sound_fields {
        let path = field_path(prefix, field);
        let value = field_value(fields, field)
            .ok_or_else(|| error(path.clone(), "missing required field"))?;
        decoded.insert(*field, decode_sound_event_holder(value, &path)?);
    }
    Ok(RegistrySoundSet {
        ambient_sound: take_sound(&mut decoded, "ambient_sound"),
        stray_ambient_sound: decoded.remove("stray_ambient_sound"),
        hiss_sound: decoded.remove("hiss_sound"),
        hurt_sound: take_sound(&mut decoded, "hurt_sound"),
        death_sound: take_sound(&mut decoded, "death_sound"),
        step_sound: decoded.remove("step_sound"),
        eat_sound: decoded.remove("eat_sound"),
        beg_for_food_sound: decoded.remove("beg_for_food_sound"),
        purr_sound: decoded.remove("purr_sound"),
        purreow_sound: decoded.remove("purreow_sound"),
        growl_sound: decoded.remove("growl_sound"),
        pant_sound: decoded.remove("pant_sound"),
        whine_sound: decoded.remove("whine_sound"),
    })
}

fn take_sound(decoded: &mut BTreeMap<&str, RegistrySoundEvent>, field: &str) -> RegistrySoundEvent {
    decoded
        .remove(field)
        .expect("base SoundSet field is included in every family codec")
}

pub(super) fn validate_sound_variant_references(
    variant: &RegistrySoundVariant,
    access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    validate_sound_set_references(&variant.adult_sounds, "adult_sounds", access)?;
    if let Some(baby_sounds) = &variant.baby_sounds {
        validate_sound_set_references(baby_sounds, "baby_sounds", access)?;
    }
    Ok(())
}

fn validate_sound_set_references(
    sounds: &RegistrySoundSet,
    prefix: &str,
    access: FrozenRegistryAccess<'_>,
) -> Result<(), String> {
    for (field, sound) in sounds.fields() {
        if let RegistrySoundEvent::Named { resource_id } = sound {
            access
                .require_entry_by_resource_id("minecraft:sound_event", resource_id)
                .map_err(|error| format!("{}: {error}", field_path(prefix, field)))?;
        }
    }
    Ok(())
}

impl RegistrySoundSet {
    fn fields(&self) -> impl Iterator<Item = (&'static str, &RegistrySoundEvent)> {
        [
            ("ambient_sound", Some(&self.ambient_sound)),
            ("stray_ambient_sound", self.stray_ambient_sound.as_ref()),
            ("hiss_sound", self.hiss_sound.as_ref()),
            ("hurt_sound", Some(&self.hurt_sound)),
            ("death_sound", Some(&self.death_sound)),
            ("step_sound", self.step_sound.as_ref()),
            ("eat_sound", self.eat_sound.as_ref()),
            ("beg_for_food_sound", self.beg_for_food_sound.as_ref()),
            ("purr_sound", self.purr_sound.as_ref()),
            ("purreow_sound", self.purreow_sound.as_ref()),
            ("growl_sound", self.growl_sound.as_ref()),
            ("pant_sound", self.pant_sound.as_ref()),
            ("whine_sound", self.whine_sound.as_ref()),
        ]
        .into_iter()
        .filter_map(|(field, sound)| sound.map(|sound| (field, sound)))
    }
}

fn decode_sound_event_holder(
    value: &NbtSummaryValue,
    field: &str,
) -> Result<RegistrySoundEvent, SoundVariantProjectionError> {
    decode_persistent_sound_event(value, None)
        .map(|sound| match sound {
            PersistentSoundEvent::Named { resource_id, .. } => {
                RegistrySoundEvent::Named { resource_id }
            }
            PersistentSoundEvent::Direct {
                resource_id,
                fixed_range,
            } => RegistrySoundEvent::Direct {
                location: resource_id,
                fixed_range,
            },
        })
        .map_err(|failure| {
            let field = match failure.field {
                "holder" => field.to_string(),
                nested => field_path(field, nested),
            };
            error(field, failure.reason)
        })
}

fn required_compound<'a>(
    fields: &'a [NbtSummaryEntry],
    field: &str,
    path: &str,
) -> Result<&'a [NbtSummaryEntry], SoundVariantProjectionError> {
    match field_value(fields, field) {
        Some(NbtSummaryValue::Compound(fields)) => Ok(fields),
        Some(_) => Err(error(path, "expected compound")),
        None => Err(error(path, "missing required field")),
    }
}

fn field_value<'a>(fields: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    fields
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn field_path(prefix: &str, field: &str) -> String {
    if prefix.is_empty() {
        field.to_string()
    } else {
        format!("{prefix}.{field}")
    }
}

fn error(field: impl Into<String>, reason: impl Into<String>) -> SoundVariantProjectionError {
    SoundVariantProjectionError {
        field: field.into(),
        reason: reason.into(),
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
            let data = super::super::element_value::decode_network_element(&raw_data).unwrap();
            let variant =
                decode_sound_variant_definition(registry, data.source, &data.value).unwrap();
            assert_eq!(variant.element_source, RegistryElementSource::NetworkNbt);
            assert_eq!(variant.adult_sounds.fields().count(), fields.len());
            assert_eq!(
                variant
                    .baby_sounds
                    .as_ref()
                    .map(|sounds| sounds.fields().count()),
                age_sets.then_some(fields.len())
            );
            assert_eq!(
                variant.sound(false, RegistrySoundKind::Hurt),
                &RegistrySoundEvent::Named {
                    resource_id: "minecraft:entity.generic.hurt".to_string(),
                }
            );
            assert_eq!(
                decode_sound_variant_death_sound_nbt(registry, &raw_data, false).unwrap(),
                RegistrySoundEvent::Named {
                    resource_id: "minecraft:entity.generic.death".to_string(),
                }
            );
            let expected_baby = if age_sets {
                RegistrySoundEvent::Direct {
                    location: "example:baby_death".to_string(),
                    fixed_range: Some(24.0),
                }
            } else {
                RegistrySoundEvent::Named {
                    resource_id: "minecraft:entity.generic.death".to_string(),
                }
            };
            assert_eq!(
                decode_sound_variant_death_sound_nbt(registry, &raw_data, true).unwrap(),
                expected_baby
            );
        }
    }

    #[test]
    fn decodes_known_pack_json_through_the_same_sound_variant_projection() {
        let data = super::super::element_value::decode_known_pack_element(
            br#"{
                "ambient_sound": "minecraft:entity.generic.hurt",
                "hurt_sound": "minecraft:entity.generic.hurt",
                "death_sound": {"sound_id": "example:death", "range": 24.0},
                "step_sound": "minecraft:block.stone.step"
            }"#,
        )
        .unwrap();

        assert_eq!(
            decode_sound_variant_death_sound(COW_SOUND_VARIANT_REGISTRY, &data.value, false)
                .unwrap(),
            RegistrySoundEvent::Direct {
                location: "example:death".to_string(),
                fixed_range: Some(24.0),
            }
        );
        let variant =
            decode_sound_variant_definition(COW_SOUND_VARIANT_REGISTRY, data.source, &data.value)
                .unwrap();
        assert_eq!(
            variant.adult_sounds.fields().count(),
            COW_SOUND_FIELDS.len()
        );
        assert_eq!(
            variant.sound(false, RegistrySoundKind::Hurt),
            &RegistrySoundEvent::Named {
                resource_id: "minecraft:entity.generic.hurt".to_string(),
            }
        );
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
            decode_sound_variant_death_sound_nbt(
                CHICKEN_SOUND_VARIANT_REGISTRY,
                &missing_field,
                false,
            )
            .unwrap_err(),
            SoundVariantProjectionError {
                field: "adult_sounds.step_sound".to_string(),
                reason: "missing required field".to_string(),
            }
        );

        let mut invalid_identifier = sound_variant_nbt(COW_SOUND_FIELDS, false);
        let identifier = b"entity.generic.death";
        let offset = invalid_identifier
            .windows(identifier.len())
            .position(|window| window == identifier)
            .unwrap();
        invalid_identifier[offset] = b'E';
        assert_eq!(
            decode_sound_variant_death_sound_nbt(
                COW_SOUND_VARIANT_REGISTRY,
                &invalid_identifier,
                false,
            )
            .unwrap_err(),
            SoundVariantProjectionError {
                field: "death_sound".to_string(),
                reason: "invalid resource identifier Entity.generic.death".to_string(),
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
                    resource_id: "minecraft:entity.generic.death".to_string(),
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
                    .synchronized_sound_variant_hurt_sound(registry, 0, true)
                    .unwrap(),
                Some(RegistrySoundEvent::Named {
                    resource_id: "minecraft:entity.generic.hurt".to_string(),
                }),
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

    fn decode_sound_variant_death_sound_nbt(
        registry: &str,
        raw_data: &[u8],
        baby: bool,
    ) -> Result<RegistrySoundEvent, SoundVariantProjectionError> {
        let data = super::super::element_value::decode_network_element(raw_data).unwrap();
        decode_sound_variant_death_sound(registry, &data.value, baby)
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
                    "entity.generic.death"
                } else {
                    "minecraft:entity.generic.hurt"
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
