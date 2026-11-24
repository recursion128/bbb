use bbb_registry_codec::registry_codec::sound_variants::{SoundEventHolder, SoundVariantElement};

use crate::WorldStore;

pub(crate) const CAT_SOUND_VARIANT_REGISTRY: &str = "minecraft:cat_sound_variant";
pub(crate) const CHICKEN_SOUND_VARIANT_REGISTRY: &str = "minecraft:chicken_sound_variant";
pub(crate) const COW_SOUND_VARIANT_REGISTRY: &str = "minecraft:cow_sound_variant";
pub(crate) const PIG_SOUND_VARIANT_REGISTRY: &str = "minecraft:pig_sound_variant";
pub(crate) const WOLF_SOUND_VARIANT_REGISTRY: &str = "minecraft:wolf_sound_variant";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistrySoundKind {
    Hurt,
    Death,
}

impl WorldStore {
    pub(crate) fn synchronized_sound_variant_sound(
        &self,
        registry: &str,
        holder_id: i32,
        baby: bool,
        kind: RegistrySoundKind,
    ) -> Option<&SoundEventHolder> {
        let entry = self.registry_entry_by_holder_id(registry, holder_id)?;
        let variant = self
            .registries
            .typed
            .sound_variants
            .get(registry)?
            .get(&entry.id)?;
        let (hurt, death) = match variant {
            SoundVariantElement::Wolf(variant) => {
                let sounds = if baby {
                    &variant.baby_sounds
                } else {
                    &variant.adult_sounds
                };
                (&sounds.hurt_sound, &sounds.death_sound)
            }
            SoundVariantElement::Pig(variant) => {
                let sounds = if baby {
                    &variant.baby_sounds
                } else {
                    &variant.adult_sounds
                };
                (&sounds.hurt_sound, &sounds.death_sound)
            }
            SoundVariantElement::Cat(variant) => {
                let sounds = if baby {
                    &variant.baby_sounds
                } else {
                    &variant.adult_sounds
                };
                (&sounds.hurt_sound, &sounds.death_sound)
            }
            SoundVariantElement::Cow(variant) => (&variant.hurt_sound, &variant.death_sound),
            SoundVariantElement::Chicken(variant) => {
                let sounds = if baby {
                    &variant.baby_sounds
                } else {
                    &variant.adult_sounds
                };
                (&sounds.hurt_sound, &sounds.death_sound)
            }
        };
        Some(match kind {
            RegistrySoundKind::Hurt => hurt,
            RegistrySoundKind::Death => death,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::super::revision::RegistryRevisionCandidate;
    use super::super::state::PendingRegistryCollection;
    use super::*;
    use crate::{RegistryContentState, RegistryPacketEntry, RegistrySet};

    #[test]
    fn source_aware_known_pack_json_tolerates_lenient_range_and_unknown_null() {
        let store = sound_variant_world(
            COW_SOUND_VARIANT_REGISTRY,
            RegistryPacketEntry::with_known_pack_data_for_test(
                "example:variant",
                br#"{
                    "ambient_sound": {
                        "sound_id":"example:ambient", "range":null, "unknown_nested":null
                    },
                    "hurt_sound":"minecraft:entity.generic.hurt",
                    "death_sound":{"sound_id":"example:death", "range":null},
                    "step_sound":"minecraft:block.stone.step"
                }"#
                .to_vec(),
            ),
        )
        .unwrap();
        let Some(SoundEventHolder::Inline(death)) = store.synchronized_sound_variant_sound(
            COW_SOUND_VARIANT_REGISTRY,
            0,
            false,
            RegistrySoundKind::Death,
        ) else {
            panic!("expected published inline death sound");
        };
        assert_eq!(death.sound_id.as_str(), "example:death");
        assert_eq!(death.range, None);
        assert!(matches!(store.synchronized_sound_variant_sound(
            COW_SOUND_VARIANT_REGISTRY, 0, false, RegistrySoundKind::Hurt),
            Some(SoundEventHolder::Named(id)) if id.as_str() == "minecraft:entity.generic.hurt"));
    }

    #[test]
    fn rejects_incomplete_or_malformed_sound_variant_payloads() {
        let mut missing_field = sound_variant_nbt(&chicken_fields(), true);
        let field = b"\x00\x0astep_sound";
        let offset = missing_field
            .windows(field.len())
            .position(|window| window == field)
            .unwrap();
        missing_field[offset + 2] = b'x';
        assert!(sound_variant_world(
            CHICKEN_SOUND_VARIANT_REGISTRY,
            RegistryPacketEntry::with_raw_data("example:invalid", missing_field)
        )
        .is_err());

        let mut invalid_identifier = sound_variant_nbt(&cow_fields(), false);
        let identifier = b"entity.generic.death";
        let offset = invalid_identifier
            .windows(identifier.len())
            .position(|window| window == identifier)
            .unwrap();
        invalid_identifier[offset] = b'E';
        assert!(sound_variant_world(
            COW_SOUND_VARIANT_REGISTRY,
            RegistryPacketEntry::with_raw_data("example:invalid", invalid_identifier)
        )
        .is_err());
    }

    #[test]
    fn world_queries_use_server_holder_order_for_every_sound_variant_shape() {
        let cases = [
            (CAT_SOUND_VARIANT_REGISTRY, cat_fields(), true),
            (CHICKEN_SOUND_VARIANT_REGISTRY, chicken_fields(), true),
            (COW_SOUND_VARIANT_REGISTRY, cow_fields(), false),
            (PIG_SOUND_VARIANT_REGISTRY, pig_fields(), true),
            (WOLF_SOUND_VARIANT_REGISTRY, wolf_fields(), true),
        ];
        for (registry, fields, age_sets) in cases {
            let store = sound_variant_world(
                registry,
                RegistryPacketEntry::with_raw_data(
                    "example:custom",
                    sound_variant_nbt(&fields, age_sets),
                ),
            )
            .unwrap();
            let baby_death = store
                .synchronized_sound_variant_sound(registry, 0, true, RegistrySoundKind::Death)
                .unwrap();
            if age_sets {
                let SoundEventHolder::Inline(death) = baby_death else {
                    panic!("expected inline baby death sound");
                };
                assert_eq!(death.sound_id.as_str(), "example:baby_death");
                assert_eq!(death.range.map(|range| range.to_f32()), Some(24.0));
            } else {
                assert!(
                    matches!(baby_death, SoundEventHolder::Named(id) if id.as_str() == "minecraft:entity.generic.death")
                );
            }
            assert!(
                matches!(store.synchronized_sound_variant_sound(registry, 0, false, RegistrySoundKind::Death),
                Some(SoundEventHolder::Named(id)) if id.as_str() == "minecraft:entity.generic.death")
            );
            assert!(
                matches!(store.synchronized_sound_variant_sound(registry, 0, true, RegistrySoundKind::Hurt),
                Some(SoundEventHolder::Named(id)) if id.as_str() == "minecraft:entity.generic.hurt")
            );
            assert_eq!(
                store.synchronized_sound_variant_sound(
                    registry,
                    1,
                    false,
                    RegistrySoundKind::Death
                ),
                None
            );
        }
    }

    #[test]
    fn custom_named_sound_event_cannot_come_from_configuration_contents() {
        for include_static_candidate in [true, false] {
            let active = registry_set_with_custom_named_cow(include_static_candidate);
            let error = match build_existing_contents_candidate(&active) {
                Ok(_) => panic!("custom static sound-event reference unexpectedly committed"),
                Err(error) => error,
            };
            assert_eq!(error.stage, "registry_configuration_exact_codecs");
            assert_eq!(error.error.registry, COW_SOUND_VARIANT_REGISTRY);
            assert_eq!(error.error.validator, "CowSoundVariant.DIRECT_CODEC");
            assert!(error.error.message.contains("example:custom_cow"));
            assert!(error
                .error
                .message
                .contains("missing element example:custom_sound in minecraft:sound_event"));
            assert!(
                active.typed.sound_variants.is_empty(),
                "fallible candidate construction must not publish typed values"
            );
        }
    }

    #[test]
    fn inline_sound_events_do_not_require_sound_event_registry_references() {
        let inline_cow = br#"{
                "ambient_sound":{"sound_id":"example:ambient"},
                "hurt_sound":{"sound_id":"example:hurt","range":12.0},
                "death_sound":{"sound_id":"example:death"},
                "step_sound":{"sound_id":"example:step"}
            }"#;
        let mut registries = RegistrySet::vanilla_26_1();
        registries.contents.insert(
            "minecraft:sound_event".to_string(),
            RegistryContentState::new("minecraft:sound_event"),
        );
        registries.contents.insert(
            COW_SOUND_VARIANT_REGISTRY.to_string(),
            RegistryContentState::from_entries(
                COW_SOUND_VARIANT_REGISTRY,
                vec![RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:inline_cow",
                    inline_cow.to_vec(),
                )],
            ),
        );

        assert!(build_existing_contents_candidate(&registries).is_ok());
    }

    fn registry_set_with_custom_named_cow(include_sound: bool) -> RegistrySet {
        let mut registries = RegistrySet::vanilla_26_1();
        registries.contents.insert(
            "minecraft:sound_event".to_string(),
            RegistryContentState::from_entries(
                "minecraft:sound_event",
                include_sound
                    .then(|| RegistryPacketEntry::stub("example:custom_sound"))
                    .into_iter()
                    .collect(),
            ),
        );
        registries.contents.insert(
            COW_SOUND_VARIANT_REGISTRY.to_string(),
            RegistryContentState::from_entries(
                COW_SOUND_VARIANT_REGISTRY,
                vec![RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:custom_cow",
                    cow_known_pack_data_json("example:custom_sound"),
                )],
            ),
        );
        registries
    }

    fn build_existing_contents_candidate(
        active: &RegistrySet,
    ) -> Result<RegistryRevisionCandidate, super::super::revision::RegistryCandidateError> {
        RegistryRevisionCandidate::build(
            active,
            PendingRegistryCollection::default(),
            None,
            None,
            false,
            &Default::default(),
        )
    }

    fn cow_known_pack_data_json(named_sound: &str) -> Vec<u8> {
        format!(
            r#"{{
                "ambient_sound":"{named_sound}",
                "hurt_sound":"{named_sound}",
                "death_sound":"{named_sound}",
                "step_sound":"{named_sound}"
            }}"#
        )
        .into_bytes()
    }

    fn cat_fields() -> Vec<&'static str> {
        vec![
            "ambient_sound",
            "stray_ambient_sound",
            "hiss_sound",
            "hurt_sound",
            "death_sound",
            "eat_sound",
            "beg_for_food_sound",
            "purr_sound",
            "purreow_sound",
        ]
    }

    fn chicken_fields() -> Vec<&'static str> {
        vec!["ambient_sound", "hurt_sound", "death_sound", "step_sound"]
    }

    fn cow_fields() -> Vec<&'static str> {
        chicken_fields()
    }

    fn pig_fields() -> Vec<&'static str> {
        vec![
            "ambient_sound",
            "hurt_sound",
            "death_sound",
            "step_sound",
            "eat_sound",
        ]
    }

    fn wolf_fields() -> Vec<&'static str> {
        vec![
            "ambient_sound",
            "death_sound",
            "growl_sound",
            "hurt_sound",
            "pant_sound",
            "whine_sound",
            "step_sound",
        ]
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

    fn sound_variant_world(
        registry: &str,
        entry: RegistryPacketEntry,
    ) -> Result<WorldStore, crate::RegistryValidationError> {
        let mut store = WorldStore::new();
        store.record_registry_entries(registry, 0, vec![entry]);
        store.begin_registry_configuration();
        store.finish_registry_configuration()?;
        Ok(store)
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
