use bbb_protocol::packets::{
    KineticWeaponConditionSummary, KineticWeaponSummary, PiercingWeaponSummary,
};
use serde::{Deserialize, Serialize};

use crate::FrozenRegistryAccess;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "holder_kind", rename_all = "snake_case")]
pub enum DialogSoundEventHolderState {
    Named {
        registry_id: i32,
        resource_id: String,
    },
    Direct {
        sound_id: String,
        #[serde(default)]
        fixed_range_bits: Option<u32>,
    },
}

pub(super) fn project_sound_event_holder(
    sound: &bbb_protocol::packets::SoundEventSummary,
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<DialogSoundEventHolderState, String> {
    project_owned_sound_event_holder(sound.clone(), registries)
}

pub(super) fn project_owned_sound_event_holder(
    sound: bbb_protocol::packets::SoundEventSummary,
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<DialogSoundEventHolderState, String> {
    if let Some((registry_id, resource_id)) = validated_sound_event_reference(&sound, registries)? {
        return Ok(DialogSoundEventHolderState::Named {
            registry_id,
            resource_id: resource_id.to_string(),
        });
    }
    Ok(DialogSoundEventHolderState::Direct {
        sound_id: sound
            .sound_id
            .expect("a direct sound retains its resource id"),
        fixed_range_bits: sound.fixed_range_bits,
    })
}

pub(super) fn validated_sound_event_reference(
    sound: &bbb_protocol::packets::SoundEventSummary,
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<Option<(i32, &'static str)>, String> {
    let Some(id) = sound.registry_id else {
        return Ok(None);
    };
    let resource_id = bbb_protocol::sound_event_types::vanilla_sound_event_resource_id(id)
        .ok_or_else(|| format!("unknown minecraft:sound_event holder {id}"))?;
    let registry_id = match registries {
        Some(registries) => {
            registries
                .require_entry_by_resource_id("minecraft:sound_event", resource_id)
                .map_err(|error| error.to_string())?
                .holder_id
        }
        None => id,
    };
    Ok(Some((registry_id, resource_id)))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogPiercingWeaponState {
    pub deals_knockback: bool,
    pub dismounts: bool,
    #[serde(default)]
    pub sound: Option<DialogSoundEventHolderState>,
    #[serde(default)]
    pub hit_sound: Option<DialogSoundEventHolderState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogKineticWeaponState {
    pub contact_cooldown_ticks: i32,
    pub delay_ticks: i32,
    #[serde(default)]
    pub dismount_conditions: Option<KineticWeaponConditionSummary>,
    #[serde(default)]
    pub knockback_conditions: Option<KineticWeaponConditionSummary>,
    #[serde(default)]
    pub damage_conditions: Option<KineticWeaponConditionSummary>,
    pub forward_movement_bits: u32,
    pub damage_multiplier_bits: u32,
    #[serde(default)]
    pub sound: Option<DialogSoundEventHolderState>,
    #[serde(default)]
    pub hit_sound: Option<DialogSoundEventHolderState>,
}

pub(super) fn project_piercing_weapon(
    weapon: &PiercingWeaponSummary,
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<DialogPiercingWeaponState, String> {
    Ok(DialogPiercingWeaponState {
        deals_knockback: weapon.deals_knockback,
        dismounts: weapon.dismounts,
        sound: weapon
            .sound
            .as_ref()
            .map(|sound| project_sound_event_holder(sound, registries))
            .transpose()?,
        hit_sound: weapon
            .hit_sound
            .as_ref()
            .map(|sound| project_sound_event_holder(sound, registries))
            .transpose()?,
    })
}

pub(super) fn project_kinetic_weapon(
    weapon: &KineticWeaponSummary,
    registries: Option<FrozenRegistryAccess<'_>>,
) -> Result<DialogKineticWeaponState, String> {
    Ok(DialogKineticWeaponState {
        contact_cooldown_ticks: weapon.contact_cooldown_ticks,
        delay_ticks: weapon.delay_ticks,
        dismount_conditions: weapon.dismount_conditions,
        knockback_conditions: weapon.knockback_conditions,
        damage_conditions: weapon.damage_conditions,
        forward_movement_bits: weapon.forward_movement_bits,
        damage_multiplier_bits: weapon.damage_multiplier_bits,
        sound: weapon
            .sound
            .as_ref()
            .map(|sound| project_sound_event_holder(sound, registries))
            .transpose()?,
        hit_sound: weapon
            .hit_sound
            .as_ref()
            .map(|sound| project_sound_event_holder(sound, registries))
            .transpose()?,
    })
}

#[cfg(test)]
mod tests {
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
    };
    use bbb_protocol::{
        sound_event_types::vanilla_sound_event_registry_id, NbtSummaryEntry, NbtSummaryValue,
    };

    use super::super::{decode_supported_component_value, DialogDataComponentValueState};
    use super::*;

    #[test]
    fn decodes_named_and_direct_break_sound_holders() {
        let named = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
            "minecraft:break_sound",
            &NbtSummaryValue::string("entity.item.break"),
            None,
        )
        .unwrap();
        assert!(matches!(
            named,
            Some(DialogDataComponentValueState::BreakSound {
                sound: DialogSoundEventHolderState::Named {
                    registry_id,
                    resource_id,
                },
            }) if registry_id == vanilla_sound_event_registry_id("minecraft:entity.item.break").unwrap()
                && resource_id == "minecraft:entity.item.break"
        ));

        let world = crate::WorldStore::new();
        let with_access = break_sound(
            &NbtSummaryValue::string("entity.item.break"),
            Some(world.registries()),
        )
        .unwrap();
        assert!(matches!(with_access,
            DialogSoundEventHolderState::Named { registry_id, .. }
                if registry_id == vanilla_sound_event_registry_id("minecraft:entity.item.break").unwrap()
        ));

        let direct = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
            "minecraft:break_sound",
            &compound([
                ("sound_id", NbtSummaryValue::string("example:glass")),
                ("range", NbtSummaryValue::Double(12.5f64.to_bits())),
            ]),
            None,
        )
        .unwrap();
        assert!(matches!(
            direct,
            Some(DialogDataComponentValueState::BreakSound {
                sound: DialogSoundEventHolderState::Direct {
                    sound_id,
                    fixed_range_bits: Some(bits),
                },
            }) if sound_id == "example:glass" && f32::from_bits(bits) == 12.5
        ));
    }

    #[test]
    fn direct_break_sound_range_is_lenient_and_optional() {
        for (range, expected) in [
            (None, None),
            (Some(NbtSummaryValue::Int(8)), Some(8.0f32.to_bits())),
            (Some(NbtSummaryValue::string("invalid")), None),
        ] {
            let mut entries = vec![(
                "sound_id",
                NbtSummaryValue::string("minecraft:entity.item.break"),
            )];
            if let Some(range) = range {
                entries.push(("range", range));
            }
            let decoded = break_sound(&compound(entries), None).unwrap();
            assert!(matches!(
                decoded,
                DialogSoundEventHolderState::Direct {
                    fixed_range_bits,
                    ..
                } if fixed_range_bits == expected
            ));
        }
    }

    #[test]
    fn direct_break_sound_fields_follow_compound_last_wins_semantics() {
        let decoded = break_sound(
            &compound([
                ("sound_id", NbtSummaryValue::Int(0)),
                ("sound_id", NbtSummaryValue::string("example:glass")),
                ("range", NbtSummaryValue::string("invalid")),
                ("range", NbtSummaryValue::Float(12.5f32.to_bits())),
            ]),
            None,
        )
        .unwrap();
        assert!(matches!(
            decoded,
            DialogSoundEventHolderState::Direct {
                sound_id,
                fixed_range_bits: Some(bits),
            } if sound_id == "example:glass" && f32::from_bits(bits) == 12.5
        ));
    }

    #[test]
    fn rejects_malformed_or_unknown_break_sound_holders() {
        for value in [
            NbtSummaryValue::string("minecraft:missing"),
            NbtSummaryValue::string("Invalid"),
            NbtSummaryValue::Int(0),
            compound([]),
            compound([("sound_id", NbtSummaryValue::Int(0))]),
            compound([("sound_id", NbtSummaryValue::string("Invalid"))]),
        ] {
            assert!(break_sound(&value, None).is_err());
        }
    }

    #[test]
    fn decodes_piercing_weapon_defaults_and_sound_holders() {
        let defaults = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
            "minecraft:piercing_weapon",
            &compound([]),
            None,
        )
        .unwrap();
        assert!(matches!(
            defaults,
            Some(DialogDataComponentValueState::PiercingWeapon {
                weapon: DialogPiercingWeaponState {
                    deals_knockback: true,
                    dismounts: false,
                    sound: None,
                    hit_sound: None,
                },
            })
        ));

        let decoded = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
            "minecraft:piercing_weapon",
            &compound([
                ("deals_knockback", NbtSummaryValue::Int(256)),
                ("dismounts", NbtSummaryValue::Int(1)),
                ("sound", NbtSummaryValue::string("entity.item.break")),
                (
                    "hit_sound",
                    compound([
                        ("sound_id", NbtSummaryValue::string("example:spear_hit")),
                        ("range", NbtSummaryValue::Float(24.0f32.to_bits())),
                    ]),
                ),
            ]),
            None,
        )
        .unwrap();
        assert!(matches!(
            decoded,
            Some(DialogDataComponentValueState::PiercingWeapon {
                weapon: DialogPiercingWeaponState {
                    deals_knockback: false,
                    dismounts: true,
                    sound: Some(DialogSoundEventHolderState::Named { resource_id, .. }),
                    hit_sound: Some(DialogSoundEventHolderState::Direct {
                        sound_id,
                        fixed_range_bits: Some(bits),
                    }),
                },
            }) if resource_id == "minecraft:entity.item.break"
                && sound_id == "example:spear_hit"
                && f32::from_bits(bits) == 24.0
        ));
    }

    #[test]
    fn rejects_malformed_piercing_weapon_records() {
        for value in [
            NbtSummaryValue::List(Vec::new()),
            compound([("deals_knockback", NbtSummaryValue::string("true"))]),
            compound([("sound", NbtSummaryValue::Int(0))]),
            compound([("hit_sound", NbtSummaryValue::string("minecraft:missing"))]),
        ] {
            assert!(decode_supported_component_value(
                VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
                "minecraft:piercing_weapon",
                &value,
                None,
            )
            .is_err());
        }
    }

    #[test]
    fn decodes_kinetic_weapon_defaults_and_nested_conditions() {
        let defaults = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
            "minecraft:kinetic_weapon",
            &compound([]),
            None,
        )
        .unwrap();
        assert!(matches!(
            defaults,
            Some(DialogDataComponentValueState::KineticWeapon {
                weapon: DialogKineticWeaponState {
                    contact_cooldown_ticks: 10,
                    delay_ticks: 0,
                    dismount_conditions: None,
                    knockback_conditions: None,
                    damage_conditions: None,
                    forward_movement_bits,
                    damage_multiplier_bits,
                    sound: None,
                    hit_sound: None,
                },
            }) if forward_movement_bits == 0.0f32.to_bits()
                && damage_multiplier_bits == 1.0f32.to_bits()
        ));

        let nan_bits = f32::from_bits(0x7fc0_4321).to_bits();
        let decoded = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
            "minecraft:kinetic_weapon",
            &compound([
                ("contact_cooldown_ticks", NbtSummaryValue::Int(3)),
                ("delay_ticks", NbtSummaryValue::Short(2)),
                (
                    "dismount_conditions",
                    compound([
                        ("max_duration_ticks", NbtSummaryValue::Int(20)),
                        ("min_speed", NbtSummaryValue::Double(1.25f64.to_bits())),
                    ]),
                ),
                (
                    "knockback_conditions",
                    compound([
                        ("max_duration_ticks", NbtSummaryValue::Byte(0)),
                        (
                            "min_relative_speed",
                            NbtSummaryValue::Float(2.5f32.to_bits()),
                        ),
                    ]),
                ),
                ("forward_movement", NbtSummaryValue::Float(nan_bits)),
                (
                    "damage_multiplier",
                    NbtSummaryValue::Float((-2.0f32).to_bits()),
                ),
                ("sound", NbtSummaryValue::string("entity.item.break")),
                (
                    "hit_sound",
                    compound([("sound_id", NbtSummaryValue::string("example:spear_impact"))]),
                ),
            ]),
            None,
        )
        .unwrap();
        assert!(matches!(
            decoded,
            Some(DialogDataComponentValueState::KineticWeapon {
                weapon: DialogKineticWeaponState {
                    contact_cooldown_ticks: 3,
                    delay_ticks: 2,
                    dismount_conditions: Some(KineticWeaponConditionSummary {
                        max_duration_ticks: 20,
                        min_speed_bits,
                        min_relative_speed_bits,
                    }),
                    knockback_conditions: Some(KineticWeaponConditionSummary {
                        max_duration_ticks: 0,
                        min_speed_bits: knockback_min_speed_bits,
                        min_relative_speed_bits: knockback_min_relative_speed_bits,
                    }),
                    damage_conditions: None,
                    forward_movement_bits,
                    damage_multiplier_bits,
                    sound: Some(DialogSoundEventHolderState::Named { resource_id, .. }),
                    hit_sound: Some(DialogSoundEventHolderState::Direct {
                        sound_id,
                        fixed_range_bits: None,
                    }),
                },
            }) if f32::from_bits(min_speed_bits) == 1.25
                && min_relative_speed_bits == 0.0f32.to_bits()
                && knockback_min_speed_bits == 0.0f32.to_bits()
                && f32::from_bits(knockback_min_relative_speed_bits) == 2.5
                && forward_movement_bits == nan_bits
                && f32::from_bits(damage_multiplier_bits) == -2.0
                && resource_id == "minecraft:entity.item.break"
                && sound_id == "example:spear_impact"
        ));
    }

    #[test]
    fn kinetic_weapon_optional_records_floats_and_sounds_follow_last_wins_semantics() {
        let decoded = kinetic_weapon(&compound([
            ("damage_conditions", NbtSummaryValue::Int(0)),
            (
                "damage_conditions",
                compound([("max_duration_ticks", NbtSummaryValue::Int(20))]),
            ),
            ("forward_movement", NbtSummaryValue::string("invalid")),
            ("forward_movement", NbtSummaryValue::Float(0.5f32.to_bits())),
            ("sound", NbtSummaryValue::Int(0)),
            ("sound", NbtSummaryValue::string("entity.item.break")),
        ]))
        .unwrap();
        assert!(matches!(
            decoded,
            DialogKineticWeaponState {
                damage_conditions: Some(KineticWeaponConditionSummary {
                    max_duration_ticks: 20,
                    ..
                }),
                forward_movement_bits,
                sound: Some(DialogSoundEventHolderState::Named { resource_id, .. }),
                ..
            } if f32::from_bits(forward_movement_bits) == 0.5
                && resource_id == "minecraft:entity.item.break"
        ));
    }

    #[test]
    fn rejects_malformed_kinetic_weapon_records() {
        for value in [
            NbtSummaryValue::List(Vec::new()),
            compound([("contact_cooldown_ticks", NbtSummaryValue::Int(-1))]),
            compound([("delay_ticks", NbtSummaryValue::Long(-1))]),
            compound([("damage_conditions", NbtSummaryValue::Int(0))]),
            compound([("damage_conditions", compound([]))]),
            compound([(
                "damage_conditions",
                compound([("max_duration_ticks", NbtSummaryValue::Int(-1))]),
            )]),
            compound([(
                "knockback_conditions",
                compound([
                    ("max_duration_ticks", NbtSummaryValue::Int(1)),
                    ("min_relative_speed", NbtSummaryValue::string("fast")),
                ]),
            )]),
            compound([("sound", NbtSummaryValue::string("missing"))]),
        ] {
            assert!(kinetic_weapon(&value).is_err());
        }
    }

    fn break_sound(
        value: &NbtSummaryValue,
        registries: Option<&crate::RegistrySet>,
    ) -> Result<DialogSoundEventHolderState, String> {
        match decode_supported_component_value(
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
            "minecraft:break_sound",
            value,
            registries,
        )? {
            Some(DialogDataComponentValueState::BreakSound { sound }) => Ok(sound),
            _ => panic!("expected break sound component"),
        }
    }

    fn kinetic_weapon(value: &NbtSummaryValue) -> Result<DialogKineticWeaponState, String> {
        match decode_supported_component_value(
            VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
            "minecraft:kinetic_weapon",
            value,
            None,
        )? {
            Some(DialogDataComponentValueState::KineticWeapon { weapon }) => Ok(weapon),
            _ => panic!("expected kinetic weapon component"),
        }
    }

    fn compound<'a>(
        entries: impl IntoIterator<Item = (&'a str, NbtSummaryValue)>,
    ) -> NbtSummaryValue {
        NbtSummaryValue::Compound(
            entries
                .into_iter()
                .map(|(name, value)| NbtSummaryEntry {
                    name: name.into(),
                    value,
                })
                .collect(),
        )
    }
}
