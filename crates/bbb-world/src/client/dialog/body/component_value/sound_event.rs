use bbb_protocol::{
    sound_event_types::vanilla_sound_event_registry_id, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use crate::registries::normalize_codec_identifier;

use super::{
    component_f32_bits, optional_component_bool, optional_component_i32, required_component_i32,
    validate_i32_range,
};

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
pub struct DialogKineticWeaponConditionState {
    pub max_duration_ticks: i32,
    pub min_speed_bits: u32,
    pub min_relative_speed_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogKineticWeaponState {
    pub contact_cooldown_ticks: i32,
    pub delay_ticks: i32,
    #[serde(default)]
    pub dismount_conditions: Option<DialogKineticWeaponConditionState>,
    #[serde(default)]
    pub knockback_conditions: Option<DialogKineticWeaponConditionState>,
    #[serde(default)]
    pub damage_conditions: Option<DialogKineticWeaponConditionState>,
    pub forward_movement_bits: u32,
    pub damage_multiplier_bits: u32,
    #[serde(default)]
    pub sound: Option<DialogSoundEventHolderState>,
    #[serde(default)]
    pub hit_sound: Option<DialogSoundEventHolderState>,
}

pub(super) fn decode_piercing_weapon(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogPiercingWeaponState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    Ok(DialogPiercingWeaponState {
        deals_knockback: optional_component_bool(entries, "deals_knockback", true, what)?,
        dismounts: optional_component_bool(entries, "dismounts", false, what)?,
        sound: optional_sound_event(entries, "sound", what)?,
        hit_sound: optional_sound_event(entries, "hit_sound", what)?,
    })
}

pub(super) fn decode_kinetic_weapon(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogKineticWeaponState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let contact_cooldown_ticks =
        optional_component_i32(entries, "contact_cooldown_ticks", 10, what)?;
    validate_i32_range(
        contact_cooldown_ticks,
        0,
        i32::MAX,
        &format!("{what} field contact_cooldown_ticks"),
    )?;
    let delay_ticks = optional_component_i32(entries, "delay_ticks", 0, what)?;
    validate_i32_range(
        delay_ticks,
        0,
        i32::MAX,
        &format!("{what} field delay_ticks"),
    )?;
    Ok(DialogKineticWeaponState {
        contact_cooldown_ticks,
        delay_ticks,
        dismount_conditions: optional_kinetic_condition(entries, "dismount_conditions", what)?,
        knockback_conditions: optional_kinetic_condition(entries, "knockback_conditions", what)?,
        damage_conditions: optional_kinetic_condition(entries, "damage_conditions", what)?,
        forward_movement_bits: optional_unrestricted_float(entries, "forward_movement", 0.0, what)?,
        damage_multiplier_bits: optional_unrestricted_float(
            entries,
            "damage_multiplier",
            1.0,
            what,
        )?,
        sound: optional_sound_event(entries, "sound", what)?,
        hit_sound: optional_sound_event(entries, "hit_sound", what)?,
    })
}

fn optional_kinetic_condition(
    entries: &[NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<Option<DialogKineticWeaponConditionState>, String> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == field)
        .map(|entry| decode_kinetic_condition(&entry.value, &format!("{what} field {field}")))
        .transpose()
}

fn decode_kinetic_condition(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogKineticWeaponConditionState, String> {
    let NbtSummaryValue::Compound(entries) = value else {
        return Err(format!("{what} is not a compound"));
    };
    let max_duration_ticks = required_component_i32(entries, "max_duration_ticks", what)?;
    validate_i32_range(
        max_duration_ticks,
        0,
        i32::MAX,
        &format!("{what} field max_duration_ticks"),
    )?;
    Ok(DialogKineticWeaponConditionState {
        max_duration_ticks,
        min_speed_bits: optional_unrestricted_float(entries, "min_speed", 0.0, what)?,
        min_relative_speed_bits: optional_unrestricted_float(
            entries,
            "min_relative_speed",
            0.0,
            what,
        )?,
    })
}

fn optional_unrestricted_float(
    entries: &[NbtSummaryEntry],
    field: &str,
    default: f32,
    what: &str,
) -> Result<u32, String> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == field)
        .map_or(Ok(default.to_bits()), |entry| {
            component_f32_bits(&entry.value, &format!("{what} field {field}"))
        })
}

fn optional_sound_event(
    entries: &[NbtSummaryEntry],
    field: &str,
    what: &str,
) -> Result<Option<DialogSoundEventHolderState>, String> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == field)
        .map(|entry| decode_sound_event_holder(&entry.value, &format!("{what} field {field}")))
        .transpose()
}

pub(super) fn decode_sound_event_holder(
    value: &NbtSummaryValue,
    what: &str,
) -> Result<DialogSoundEventHolderState, String> {
    match value {
        NbtSummaryValue::String(raw) => decode_named_sound_event(raw, what),
        NbtSummaryValue::Compound(entries) => decode_direct_sound_event(entries, what),
        _ => Err(format!(
            "{what} holder is neither a resource id nor a direct sound event"
        )),
    }
}

fn decode_named_sound_event(raw: &str, what: &str) -> Result<DialogSoundEventHolderState, String> {
    let resource_id = normalize_codec_identifier(raw)
        .ok_or_else(|| format!("invalid {what} holder resource id {raw}"))?;
    let registry_id = vanilla_sound_event_registry_id(&resource_id).ok_or_else(|| {
        format!("missing minecraft:sound_event resource {resource_id} for {what}")
    })?;
    Ok(DialogSoundEventHolderState::Named {
        registry_id,
        resource_id,
    })
}

fn decode_direct_sound_event(
    entries: &[NbtSummaryEntry],
    what: &str,
) -> Result<DialogSoundEventHolderState, String> {
    let sound_id = entries
        .iter()
        .rev()
        .find(|entry| entry.name == "sound_id")
        .map(|entry| &entry.value)
        .ok_or_else(|| format!("{what} direct holder is missing field sound_id"))?;
    let NbtSummaryValue::String(raw_sound_id) = sound_id else {
        return Err(format!(
            "{what} direct holder field sound_id is not a resource id"
        ));
    };
    let sound_id = normalize_codec_identifier(raw_sound_id)
        .ok_or_else(|| format!("invalid {what} direct holder field sound_id {raw_sound_id}"))?;
    let fixed_range_bits = entries
        .iter()
        .rev()
        .find(|entry| entry.name == "range")
        .and_then(|entry| {
            component_f32_bits(&entry.value, &format!("{what} direct holder field range")).ok()
        });
    Ok(DialogSoundEventHolderState::Direct {
        sound_id,
        fixed_range_bits,
    })
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        data_component_types::{
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
            VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
            VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
        },
        sound_event_types::vanilla_sound_event_registry_id,
    };

    use super::super::{decode_supported_component_value, DialogDataComponentValueState};
    use super::*;

    #[test]
    fn decodes_named_and_direct_break_sound_holders() {
        let named = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
            "minecraft:break_sound",
            &NbtSummaryValue::String("entity.item.break".to_string()),
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

        let direct = decode_supported_component_value(
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID,
            "minecraft:break_sound",
            &compound([
                (
                    "sound_id",
                    NbtSummaryValue::String("example:glass".to_string()),
                ),
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
            (Some(NbtSummaryValue::String("invalid".to_string())), None),
        ] {
            let mut entries = vec![(
                "sound_id",
                NbtSummaryValue::String("minecraft:entity.item.break".to_string()),
            )];
            if let Some(range) = range {
                entries.push(("range", range));
            }
            let decoded = decode_sound_event_holder(&compound(entries), "break sound").unwrap();
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
        let decoded = decode_sound_event_holder(
            &compound([
                ("sound_id", NbtSummaryValue::Int(0)),
                (
                    "sound_id",
                    NbtSummaryValue::String("example:glass".to_string()),
                ),
                ("range", NbtSummaryValue::String("invalid".to_string())),
                ("range", NbtSummaryValue::Float(12.5f32.to_bits())),
            ]),
            "break sound",
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
        for (value, expected) in [
            (
                NbtSummaryValue::String("minecraft:missing".to_string()),
                "missing minecraft:sound_event resource",
            ),
            (NbtSummaryValue::String("Invalid".to_string()), "invalid"),
            (NbtSummaryValue::Int(0), "neither a resource id"),
            (compound([]), "missing field sound_id"),
            (
                compound([("sound_id", NbtSummaryValue::Int(0))]),
                "field sound_id is not a resource id",
            ),
            (
                compound([("sound_id", NbtSummaryValue::String("Invalid".to_string()))]),
                "invalid",
            ),
        ] {
            let error = decode_sound_event_holder(&value, "break sound").unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
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
                ("deals_knockback", NbtSummaryValue::Byte(0)),
                ("dismounts", NbtSummaryValue::Int(1)),
                (
                    "sound",
                    NbtSummaryValue::String("entity.item.break".to_string()),
                ),
                (
                    "hit_sound",
                    compound([
                        (
                            "sound_id",
                            NbtSummaryValue::String("example:spear_hit".to_string()),
                        ),
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
        for (value, expected) in [
            (NbtSummaryValue::List(Vec::new()), "is not a compound"),
            (
                compound([(
                    "deals_knockback",
                    NbtSummaryValue::String("true".to_string()),
                )]),
                "deals_knockback is not a boolean",
            ),
            (
                compound([("sound", NbtSummaryValue::Int(0))]),
                "holder is neither a resource id",
            ),
            (
                compound([(
                    "hit_sound",
                    NbtSummaryValue::String("minecraft:missing".to_string()),
                )]),
                "missing minecraft:sound_event resource",
            ),
        ] {
            let error = decode_piercing_weapon(&value, "piercing weapon").unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
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
                (
                    "sound",
                    NbtSummaryValue::String("entity.item.break".to_string()),
                ),
                (
                    "hit_sound",
                    compound([(
                        "sound_id",
                        NbtSummaryValue::String("example:spear_impact".to_string()),
                    )]),
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
                    dismount_conditions: Some(DialogKineticWeaponConditionState {
                        max_duration_ticks: 20,
                        min_speed_bits,
                        min_relative_speed_bits,
                    }),
                    knockback_conditions: Some(DialogKineticWeaponConditionState {
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
        let decoded = decode_kinetic_weapon(
            &compound([
                ("damage_conditions", NbtSummaryValue::Int(0)),
                (
                    "damage_conditions",
                    compound([("max_duration_ticks", NbtSummaryValue::Int(20))]),
                ),
                (
                    "forward_movement",
                    NbtSummaryValue::String("invalid".to_string()),
                ),
                ("forward_movement", NbtSummaryValue::Float(0.5f32.to_bits())),
                ("sound", NbtSummaryValue::Int(0)),
                (
                    "sound",
                    NbtSummaryValue::String("entity.item.break".to_string()),
                ),
            ]),
            "kinetic weapon",
        )
        .unwrap();
        assert!(matches!(
            decoded,
            DialogKineticWeaponState {
                damage_conditions: Some(DialogKineticWeaponConditionState {
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
        for (value, expected) in [
            (NbtSummaryValue::List(Vec::new()), "is not a compound"),
            (
                compound([("contact_cooldown_ticks", NbtSummaryValue::Int(-1))]),
                "contact_cooldown_ticks",
            ),
            (
                compound([("delay_ticks", NbtSummaryValue::Long(-1))]),
                "delay_ticks",
            ),
            (
                compound([("damage_conditions", NbtSummaryValue::Int(0))]),
                "damage_conditions is not a compound",
            ),
            (
                compound([("damage_conditions", compound([]))]),
                "missing field max_duration_ticks",
            ),
            (
                compound([(
                    "damage_conditions",
                    compound([("max_duration_ticks", NbtSummaryValue::Int(-1))]),
                )]),
                "max_duration_ticks",
            ),
            (
                compound([(
                    "knockback_conditions",
                    compound([
                        ("max_duration_ticks", NbtSummaryValue::Int(1)),
                        (
                            "min_relative_speed",
                            NbtSummaryValue::String("fast".to_string()),
                        ),
                    ]),
                )]),
                "min_relative_speed is not numeric",
            ),
            (
                compound([("sound", NbtSummaryValue::String("missing".to_string()))]),
                "missing minecraft:sound_event resource",
            ),
        ] {
            let error = decode_kinetic_weapon(&value, "kinetic weapon").unwrap_err();
            assert!(error.contains(expected), "unexpected error: {error}");
        }
    }

    fn compound<'a>(
        entries: impl IntoIterator<Item = (&'a str, NbtSummaryValue)>,
    ) -> NbtSummaryValue {
        NbtSummaryValue::Compound(
            entries
                .into_iter()
                .map(|(name, value)| NbtSummaryEntry {
                    name: name.to_string(),
                    value,
                })
                .collect(),
        )
    }
}
