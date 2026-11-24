use crate::FrozenRegistryAccess;
use bbb_protocol::{
    environment_particles::{AmbientParticle, ParticleOptions},
    NbtSummaryEntry, NbtSummaryValue,
};
use bbb_registry_codec::registry_codec::environment_source::particles::{
    ambient_particles_from_canonical, particle_from_canonical, particle_from_source,
};

pub(super) fn parse_particle(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Option<ParticleOptions> {
    particle_from_canonical(value, Some(&registry_access)).ok()
}

pub(super) fn parse_ambient_particles(
    value: &NbtSummaryValue,
    registry_access: FrozenRegistryAccess<'_>,
) -> Option<Vec<AmbientParticle>> {
    ambient_particles_from_canonical(value, Some(&registry_access)).ok()
}

pub(crate) fn validate_command_particle_fields(
    resource_id: &str,
    fields: &[NbtSummaryEntry],
) -> bool {
    let mut all_fields = vec![NbtSummaryEntry {
        name: "type".into(),
        value: NbtSummaryValue::string(resource_id),
    }];
    all_fields.extend_from_slice(fields);
    particle_from_source(
        bbb_registry_codec::registry_codec::RegistryElementValueRef::Nbt(
            &NbtSummaryValue::Compound(all_fields),
        ),
        bbb_registry_codec::RegistryElementSource::NetworkNbt,
        bbb_registry_codec::PersistentNbtDecodeLimits::default(),
        None::<&FrozenRegistryAccess<'_>>,
    )
    .is_ok()
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        data_component_types::{
            VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID, VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID,
            VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
        },
        item_types::vanilla_item_type_for_resource_id,
    };

    use super::*;
    use crate::{RegistryPacketEntry, RegistrySet, WorldStore};
    use bbb_protocol::environment_particles::ParticleOptionValue;

    #[test]
    fn decodes_every_particle_option_map_codec_family() {
        assert_eq!(
            decode("minecraft:cloud", Vec::new()).option,
            ParticleOptionValue::Empty
        );

        let stone_state_id =
            crate::vanilla_block_state_id("minecraft:stone", &std::collections::BTreeMap::new())
                .unwrap();
        assert_eq!(
            decode(
                "minecraft:block",
                vec![entry(
                    "block_state",
                    compound(vec![entry("Name", NbtSummaryValue::string("stone"))])
                )],
            )
            .option,
            ParticleOptionValue::Block {
                block_state_id: stone_state_id,
            }
        );

        assert_eq!(
            decode("minecraft:dragon_breath", Vec::new()).option,
            ParticleOptionValue::Power {
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
            ParticleOptionValue::Dust {
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
            ParticleOptionValue::DustColorTransition {
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
            ParticleOptionValue::Spell {
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
            ParticleOptionValue::Color {
                argb: 0x7f_11_22_33,
            }
        );
        assert_eq!(
            decode("minecraft:sculk_charge", vec![entry("roll", float(-0.25))],).option,
            ParticleOptionValue::SculkCharge {
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
                            entry("type", NbtSummaryValue::string("minecraft:block"),),
                            entry("pos", NbtSummaryValue::IntArray(vec![1, 2, 3])),
                        ]),
                    ),
                    entry("arrival_in_ticks", NbtSummaryValue::Int(12)),
                ],
            )
            .option,
            ParticleOptionValue::Vibration {
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
            ParticleOptionValue::Trail {
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
            ParticleOptionValue::Shriek { delay: -3 }
        );
    }

    #[test]
    fn item_particle_uses_registry_backed_item_stack_template_codec() {
        let item = compound(vec![
            entry("id", NbtSummaryValue::string("minecraft:stick")),
            entry("count", NbtSummaryValue::Int(2)),
            entry(
                "components",
                compound(vec![entry(
                    "minecraft:item_model",
                    NbtSummaryValue::string("minecraft:stone"),
                )]),
            ),
        ]);
        let decoded = decode("minecraft:item", vec![entry("item", item.clone())]);
        let ParticleOptionValue::Item { stack } = decoded.option else {
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
        assert_eq!(stack.component_patch.added_type_ids.len(), 1);
        assert_eq!(
            stack.component_patch.added_type_ids,
            vec![VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID]
        );
        assert_eq!(
            stack.component_patch.item_model.as_deref(),
            Some("minecraft:stone")
        );

        let invalid_item = compound(vec![
            entry("id", NbtSummaryValue::string("minecraft:stick")),
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
                        entry("type", NbtSummaryValue::string("minecraft:entity"),),
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
            "minecraft:trim_material",
            0,
            vec![
                RegistryPacketEntry::stub("example:first"),
                RegistryPacketEntry::stub("minecraft:quartz"),
            ],
        );
        let particle = compound(vec![
            entry("type", NbtSummaryValue::string("minecraft:item")),
            entry(
                "item",
                compound(vec![
                    entry("id", NbtSummaryValue::string("minecraft:stick")),
                    entry(
                        "components",
                        compound(vec![
                            entry(
                                "minecraft:break_sound",
                                NbtSummaryValue::string("minecraft:entity.item.break"),
                            ),
                            entry(
                                "minecraft:provides_trim_material",
                                NbtSummaryValue::string("minecraft:quartz"),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]);
        let access = store.frozen_registry_access();
        assert!(particle_from_canonical(&particle, None::<&FrozenRegistryAccess<'_>>).is_err());
        let decoded = particle_from_source(
            bbb_registry_codec::registry_codec::RegistryElementValueRef::Nbt(&particle),
            bbb_registry_codec::RegistryElementSource::NetworkNbt,
            bbb_registry_codec::PersistentNbtDecodeLimits::default(),
            Some(&access),
        )
        .unwrap();
        let ParticleOptionValue::Item { stack } = decoded.option else {
            panic!("item particle did not retain its stack template");
        };
        assert_eq!(
            stack.component_patch.added_type_ids,
            vec![
                VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID,
                VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID
            ]
        );
        assert_eq!(
            stack
                .component_patch
                .break_sound
                .as_ref()
                .and_then(|sound| sound.registry_id),
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:entity.item.break"
            )
        );
        assert_eq!(stack.component_patch.provides_trim_material_id, Some(1));
    }

    fn decode(resource_id: &str, extra_fields: Vec<NbtSummaryEntry>) -> ParticleOptions {
        try_decode(resource_id, extra_fields).expect("valid particle options")
    }

    fn try_decode(
        resource_id: &str,
        mut extra_fields: Vec<NbtSummaryEntry>,
    ) -> Option<ParticleOptions> {
        let mut fields = vec![entry("type", NbtSummaryValue::string(resource_id))];
        fields.append(&mut extra_fields);
        let registries = RegistrySet::vanilla_26_1();
        particle_from_source(
            bbb_registry_codec::registry_codec::RegistryElementValueRef::Nbt(&compound(fields)),
            bbb_registry_codec::RegistryElementSource::NetworkNbt,
            bbb_registry_codec::PersistentNbtDecodeLimits::default(),
            Some(&registries.frozen_access()),
        )
        .ok()
    }

    fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
        NbtSummaryValue::Compound(entries)
    }

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
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
