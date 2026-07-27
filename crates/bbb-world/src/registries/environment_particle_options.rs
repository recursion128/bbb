use bbb_protocol::{packets::ItemStackSummary, NbtSummaryEntry, NbtSummaryValue};
use serde::{Deserialize, Serialize};

use super::environment_attributes::{
    field_value, numeric_f32, numeric_f64, numeric_i32, parse_argb_codec_u32, parse_rgb_codec_u32,
};
use super::normalize_codec_identifier;
use crate::{client::decode_command_item_stack_template_summary, FrozenRegistryAccess};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientParticleOptions {
    pub resource_id: String,
    pub fields: Vec<NbtSummaryEntry>,
    #[serde(default)]
    pub option: ClientParticleOptionValue,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientParticleOptionValue {
    #[default]
    Empty,
    Block {
        block_state_id: i32,
    },
    Power {
        power_bits: u32,
    },
    Dust {
        color: u32,
        scale_bits: u32,
    },
    DustColorTransition {
        from_color: u32,
        to_color: u32,
        scale_bits: u32,
    },
    Spell {
        color: u32,
        power_bits: u32,
    },
    Color {
        argb: u32,
    },
    SculkCharge {
        roll_bits: u32,
    },
    Item {
        stack: ItemStackSummary,
    },
    Vibration {
        block_pos: [i32; 3],
        arrival_in_ticks: i32,
    },
    Trail {
        target_bits: [u64; 3],
        color: u32,
        duration: i32,
    },
    Shriek {
        delay: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientAmbientParticle {
    pub particle: ClientParticleOptions,
    pub probability_bits: u32,
}

pub(super) fn parse_particle(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Option<ClientParticleOptions> {
    let NbtSummaryValue::Compound(fields) = value else {
        return None;
    };
    let NbtSummaryValue::String(resource_id) = field_value(fields, "type")? else {
        return None;
    };
    let resource_id = normalize_codec_identifier(resource_id)?;
    bbb_protocol::particle_types::vanilla_particle_type_id_for_resource_id(&resource_id)?;
    let option = parse_particle_option(&resource_id, fields, Some(registry_access))?;
    Some(ClientParticleOptions {
        resource_id,
        fields: fields.clone(),
        option,
    })
}

pub(super) fn parse_ambient_particles(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Option<Vec<ClientAmbientParticle>> {
    let NbtSummaryValue::List(values) = value else {
        return None;
    };
    values
        .iter()
        .map(|value| {
            let NbtSummaryValue::Compound(fields) = value else {
                return None;
            };
            let particle = parse_particle(field_value(fields, "particle")?, registry_access)?;
            let probability = numeric_f32(field_value(fields, "probability")?)
                .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))?;
            Some(ClientAmbientParticle {
                particle,
                probability_bits: probability.to_bits(),
            })
        })
        .collect()
}

pub(crate) fn validate_command_particle_fields(
    resource_id: &str,
    fields: &[NbtSummaryEntry],
) -> bool {
    parse_particle_option(resource_id, fields, None).is_some()
}

fn parse_particle_option(
    resource_id: &str,
    fields: &[NbtSummaryEntry],
    registry_access: Option<FrozenRegistryAccess<'_>>,
) -> Option<ClientParticleOptionValue> {
    match resource_id {
        "minecraft:block"
        | "minecraft:block_marker"
        | "minecraft:falling_dust"
        | "minecraft:dust_pillar"
        | "minecraft:block_crumble" => Some(ClientParticleOptionValue::Block {
            block_state_id: parse_particle_block_state(field_value(fields, "block_state")?)?,
        }),
        "minecraft:dragon_breath" => Some(ClientParticleOptionValue::Power {
            power_bits: field_value(fields, "power")
                .map(numeric_f32)
                .unwrap_or(Some(1.0))?
                .to_bits(),
        }),
        "minecraft:dust" => Some(ClientParticleOptionValue::Dust {
            color: parse_particle_rgb_codec_u32(field_value(fields, "color")?)?,
            scale_bits: parse_particle_scale(field_value(fields, "scale")?)?.to_bits(),
        }),
        "minecraft:dust_color_transition" => Some(ClientParticleOptionValue::DustColorTransition {
            from_color: parse_particle_rgb_codec_u32(field_value(fields, "from_color")?)?,
            to_color: parse_particle_rgb_codec_u32(field_value(fields, "to_color")?)?,
            scale_bits: parse_particle_scale(field_value(fields, "scale")?)?.to_bits(),
        }),
        "minecraft:effect" | "minecraft:instant_effect" => Some(ClientParticleOptionValue::Spell {
            color: field_value(fields, "color")
                .map(parse_particle_rgb_codec_u32)
                .unwrap_or(Some(u32::MAX))?,
            power_bits: field_value(fields, "power")
                .map(numeric_f32)
                .unwrap_or(Some(1.0))?
                .to_bits(),
        }),
        "minecraft:entity_effect" | "minecraft:tinted_leaves" | "minecraft:flash" => {
            Some(ClientParticleOptionValue::Color {
                argb: parse_argb_codec_u32(field_value(fields, "color")?)?,
            })
        }
        "minecraft:sculk_charge" => Some(ClientParticleOptionValue::SculkCharge {
            roll_bits: numeric_f32(field_value(fields, "roll")?)?.to_bits(),
        }),
        "minecraft:item" => {
            let registry_access = registry_access?;
            let stack = decode_command_item_stack_template_summary(
                field_value(fields, "item")?,
                registry_access,
            )
            .ok()?;
            Some(ClientParticleOptionValue::Item { stack })
        }
        "minecraft:vibration" => {
            let NbtSummaryValue::Compound(destination) = field_value(fields, "destination")? else {
                return None;
            };
            let NbtSummaryValue::String(kind) = field_value(destination, "type")? else {
                return None;
            };
            if normalize_codec_identifier(kind)? != "minecraft:block" {
                return None;
            }
            Some(ClientParticleOptionValue::Vibration {
                block_pos: parse_block_pos(field_value(destination, "pos")?)?,
                arrival_in_ticks: numeric_i32(field_value(fields, "arrival_in_ticks")?)?,
            })
        }
        "minecraft:trail" => {
            let NbtSummaryValue::List(target) = field_value(fields, "target")? else {
                return None;
            };
            if target.len() != 3 {
                return None;
            }
            let mut target_bits = [0; 3];
            for (slot, value) in target_bits.iter_mut().zip(target) {
                *slot = numeric_f64(value)?.to_bits();
            }
            Some(ClientParticleOptionValue::Trail {
                target_bits,
                color: parse_particle_rgb_codec_u32(field_value(fields, "color")?)?,
                duration: numeric_i32(field_value(fields, "duration")?)
                    .filter(|duration| *duration > 0)?,
            })
        }
        "minecraft:shriek" => Some(ClientParticleOptionValue::Shriek {
            delay: numeric_i32(field_value(fields, "delay")?)?,
        }),
        _ => Some(ClientParticleOptionValue::Empty),
    }
}

fn parse_particle_scale(value: &NbtSummaryValue) -> Option<f32> {
    numeric_f32(value).filter(|value| value.is_finite() && (0.01..=4.0).contains(value))
}

fn parse_particle_rgb_codec_u32(value: &NbtSummaryValue) -> Option<u32> {
    match value {
        NbtSummaryValue::List(_) => parse_rgb_codec_u32(value),
        NbtSummaryValue::Byte(value) => Some(i32::from(*value) as u32),
        NbtSummaryValue::Short(value) => Some(i32::from(*value) as u32),
        NbtSummaryValue::Int(value) => Some(*value as u32),
        NbtSummaryValue::Long(value) => Some(*value as u32),
        _ => None,
    }
}

fn parse_particle_block_state(value: &NbtSummaryValue) -> Option<i32> {
    use std::collections::BTreeMap;

    let (name, properties) = match value {
        NbtSummaryValue::String(name) => (normalize_codec_identifier(name)?, BTreeMap::new()),
        NbtSummaryValue::Compound(fields) => {
            let NbtSummaryValue::String(name) = field_value(fields, "Name")? else {
                return None;
            };
            let mut properties = BTreeMap::new();
            if let Some(NbtSummaryValue::Compound(entries)) = field_value(fields, "Properties") {
                for entry in entries {
                    let NbtSummaryValue::String(value) = &entry.value else {
                        return None;
                    };
                    properties.insert(entry.name.clone(), value.clone());
                }
            } else if field_value(fields, "Properties").is_some() {
                return None;
            }
            (normalize_codec_identifier(name)?, properties)
        }
        _ => return None,
    };
    crate::vanilla_block_state_id(&name, &properties)
}

fn parse_block_pos(value: &NbtSummaryValue) -> Option<[i32; 3]> {
    match value {
        NbtSummaryValue::IntArray(values) if values.len() == 3 => {
            Some([values[0], values[1], values[2]])
        }
        NbtSummaryValue::List(values) if values.len() == 3 => Some([
            numeric_i32(&values[0])?,
            numeric_i32(&values[1])?,
            numeric_i32(&values[2])?,
        ]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        data_component_types::{
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
        },
        item_types::vanilla_item_type_for_resource_id,
    };

    use super::*;
    use crate::{RegistryPacketEntry, RegistrySet, WorldStore};

    #[test]
    fn decodes_every_particle_option_map_codec_family() {
        assert_eq!(
            decode("minecraft:cloud", Vec::new()).option,
            ClientParticleOptionValue::Empty
        );

        let stone_state_id =
            crate::vanilla_block_state_id("minecraft:stone", &std::collections::BTreeMap::new())
                .unwrap();
        assert_eq!(
            decode(
                "minecraft:block",
                vec![entry(
                    "block_state",
                    NbtSummaryValue::String("stone".to_string()),
                )],
            )
            .option,
            ClientParticleOptionValue::Block {
                block_state_id: stone_state_id,
            }
        );

        assert_eq!(
            decode("minecraft:dragon_breath", Vec::new()).option,
            ClientParticleOptionValue::Power {
                power_bits: 1.0_f32.to_bits(),
            }
        );
        assert_eq!(
            decode(
                "minecraft:dust",
                vec![
                    entry("color", NbtSummaryValue::Int(0x12_34_56)),
                    entry("scale", float(0.5)),
                ],
            )
            .option,
            ClientParticleOptionValue::Dust {
                color: 0x12_34_56,
                scale_bits: 0.5_f32.to_bits(),
            }
        );
        assert_eq!(
            decode(
                "minecraft:dust_color_transition",
                vec![
                    entry("from_color", NbtSummaryValue::Int(0x12_34_56)),
                    entry("to_color", NbtSummaryValue::Int(0x65_43_21)),
                    entry("scale", float(1.25)),
                ],
            )
            .option,
            ClientParticleOptionValue::DustColorTransition {
                from_color: 0x12_34_56,
                to_color: 0x65_43_21,
                scale_bits: 1.25_f32.to_bits(),
            }
        );
        assert_eq!(
            decode(
                "minecraft:effect",
                vec![
                    entry("color", NbtSummaryValue::Int(0x11_22_33)),
                    entry("power", float(0.75)),
                ],
            )
            .option,
            ClientParticleOptionValue::Spell {
                color: 0x11_22_33,
                power_bits: 0.75_f32.to_bits(),
            }
        );
        assert_eq!(
            decode(
                "minecraft:entity_effect",
                vec![entry("color", NbtSummaryValue::Int(0x7f_11_22_33))],
            )
            .option,
            ClientParticleOptionValue::Color {
                argb: 0x7f_11_22_33,
            }
        );
        assert_eq!(
            decode("minecraft:sculk_charge", vec![entry("roll", float(-0.25))],).option,
            ClientParticleOptionValue::SculkCharge {
                roll_bits: (-0.25_f32).to_bits(),
            }
        );
        assert_eq!(
            decode(
                "minecraft:vibration",
                vec![
                    entry(
                        "destination",
                        compound(vec![
                            entry(
                                "type",
                                NbtSummaryValue::String("minecraft:block".to_string()),
                            ),
                            entry("pos", NbtSummaryValue::IntArray(vec![1, 2, 3])),
                        ]),
                    ),
                    entry("arrival_in_ticks", NbtSummaryValue::Int(12)),
                ],
            )
            .option,
            ClientParticleOptionValue::Vibration {
                block_pos: [1, 2, 3],
                arrival_in_ticks: 12,
            }
        );
        assert_eq!(
            decode(
                "minecraft:trail",
                vec![
                    entry(
                        "target",
                        NbtSummaryValue::List(vec![double(1.5), double(-2.25), double(3.75)]),
                    ),
                    entry("color", NbtSummaryValue::Int(0x22_44_66)),
                    entry("duration", NbtSummaryValue::Int(8)),
                ],
            )
            .option,
            ClientParticleOptionValue::Trail {
                target_bits: [1.5_f64.to_bits(), (-2.25_f64).to_bits(), 3.75_f64.to_bits(),],
                color: 0x22_44_66,
                duration: 8,
            }
        );
        assert_eq!(
            decode(
                "minecraft:shriek",
                vec![entry("delay", NbtSummaryValue::Int(-3))],
            )
            .option,
            ClientParticleOptionValue::Shriek { delay: -3 }
        );
    }

    #[test]
    fn item_particle_uses_registry_backed_item_stack_template_codec() {
        let item = compound(vec![
            entry("id", NbtSummaryValue::String("minecraft:stick".to_string())),
            entry("count", NbtSummaryValue::Int(2)),
            entry(
                "components",
                compound(vec![entry(
                    "minecraft:item_model",
                    NbtSummaryValue::String("minecraft:stone".to_string()),
                )]),
            ),
        ]);
        let decoded = decode("minecraft:item", vec![entry("item", item.clone())]);
        let ClientParticleOptionValue::Item { stack } = decoded.option else {
            panic!("item particle did not retain its stack template");
        };
        assert_eq!(
            stack.item_id,
            Some(
                vanilla_item_type_for_resource_id("minecraft:stick")
                    .unwrap()
                    .id
            )
        );
        assert_eq!(stack.count, 2);
        assert_eq!(stack.component_patch.added, 1);
        assert_eq!(
            stack.component_patch.added_type_ids,
            vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID]
        );
        assert_eq!(
            stack.component_patch.item_model.as_deref(),
            Some("minecraft:stone")
        );

        let invalid_item = compound(vec![
            entry("id", NbtSummaryValue::String("minecraft:stick".to_string())),
            entry(
                "components",
                compound(vec![entry("minecraft:item_model", NbtSummaryValue::Int(1))]),
            ),
        ]);
        assert!(try_decode("minecraft:item", vec![entry("item", invalid_item)],).is_none());
        assert!(try_decode(
            "minecraft:item",
            vec![
                entry("item", item),
                entry("ignored", NbtSummaryValue::Int(1))
            ],
        )
        .is_some());
    }

    #[test]
    fn vibration_map_codec_rejects_entity_position_sources() {
        assert!(try_decode(
            "minecraft:vibration",
            vec![
                entry(
                    "destination",
                    compound(vec![
                        entry(
                            "type",
                            NbtSummaryValue::String("minecraft:entity".to_string()),
                        ),
                        entry("source_entity", NbtSummaryValue::Int(7)),
                        entry("y_offset", float(0.0)),
                    ]),
                ),
                entry("arrival_in_ticks", NbtSummaryValue::Int(4)),
            ],
        )
        .is_none());
    }

    #[test]
    fn item_particle_resolves_nested_holders_against_the_frozen_session_order() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            "minecraft:sound_event",
            0,
            vec![
                RegistryPacketEntry::stub("example:first"),
                RegistryPacketEntry::stub("minecraft:entity.item.break"),
            ],
        );
        let particle = compound(vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:item".to_string()),
            ),
            entry(
                "item",
                compound(vec![
                    entry("id", NbtSummaryValue::String("minecraft:stick".to_string())),
                    entry(
                        "components",
                        compound(vec![entry(
                            "minecraft:break_sound",
                            NbtSummaryValue::String("minecraft:entity.item.break".to_string()),
                        )]),
                    ),
                ]),
            ),
        ]);
        let decoded = parse_particle(&particle, store.frozen_registry_access()).unwrap();
        let ClientParticleOptionValue::Item { stack } = decoded.option else {
            panic!("item particle did not retain its stack template");
        };
        assert_eq!(
            stack.component_patch.added_type_ids,
            vec![VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID]
        );
        assert_eq!(
            stack
                .component_patch
                .break_sound
                .as_ref()
                .and_then(|sound| sound.registry_id),
            Some(1)
        );
    }

    fn decode(resource_id: &str, extra_fields: Vec<NbtSummaryEntry>) -> ClientParticleOptions {
        try_decode(resource_id, extra_fields).expect("valid particle options")
    }

    fn try_decode(
        resource_id: &str,
        mut extra_fields: Vec<NbtSummaryEntry>,
    ) -> Option<ClientParticleOptions> {
        let mut fields = vec![entry(
            "type",
            NbtSummaryValue::String(resource_id.to_string()),
        )];
        fields.append(&mut extra_fields);
        let registries = RegistrySet::vanilla_26_1();
        parse_particle(&compound(fields), registries.frozen_access())
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }

    fn float(value: f32) -> NbtSummaryValue {
        NbtSummaryValue::Float(value.to_bits())
    }

    fn double(value: f64) -> NbtSummaryValue {
        NbtSummaryValue::Double(value.to_bits())
    }
}
