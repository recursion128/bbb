use super::*;

#[test]
fn weapon_interaction_descriptors_reject_noncanonical_summaries() {
    let piercing_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID).unwrap();
    let kinetic_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID).unwrap();
    let swing_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID).unwrap();

    for (descriptor, expected_error) in [
        (piercing_descriptor, "missing piercing_weapon"),
        (kinetic_descriptor, "missing kinetic_weapon"),
        (swing_descriptor, "missing swing_animation"),
    ] {
        let mut encoded = Encoder::new();
        let error = descriptor
            .encode(&DataComponentPatchSummary::default(), &mut encoded)
            .unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error}");
    }

    let malformed_piercing = DataComponentPatchSummary {
        piercing_weapon: Some(PiercingWeaponSummary {
            deals_knockback: true,
            dismounts: false,
            sound: Some(SoundEventSummary {
                registry_id: Some(0),
                sound_id: Some("minecraft:item.spear.attack".to_string()),
                fixed_range_bits: None,
            }),
            hit_sound: None,
        }),
        ..DataComponentPatchSummary::default()
    };
    let malformed_kinetic = DataComponentPatchSummary {
        kinetic_weapon: Some(KineticWeaponSummary {
            contact_cooldown_ticks: 10,
            delay_ticks: 0,
            dismount_conditions: None,
            knockback_conditions: None,
            damage_conditions: None,
            forward_movement_bits: 0.0_f32.to_bits(),
            damage_multiplier_bits: 1.0_f32.to_bits(),
            sound: None,
            hit_sound: Some(SoundEventSummary {
                registry_id: Some(-1),
                sound_id: None,
                fixed_range_bits: None,
            }),
        }),
        ..DataComponentPatchSummary::default()
    };
    for (descriptor, summary) in [
        (piercing_descriptor, &malformed_piercing),
        (kinetic_descriptor, &malformed_kinetic),
    ] {
        let mut encoded = Encoder::new();
        let error = descriptor.encode(summary, &mut encoded).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("sound event holder projections are not canonical"),
            "{error}"
        );
    }
}

#[test]
fn blocks_attacks_descriptor_round_trips_holder_and_sound_unions_in_wire_order() {
    let nan_bits = 0x7fc0_1234;
    let summary = DataComponentPatchSummary {
        blocks_attacks: Some(BlocksAttacksSummary {
            block_delay_seconds_bits: (-0.0_f32).to_bits(),
            disable_cooldown_scale_bits: f32::INFINITY.to_bits(),
            damage_reductions: vec![
                DamageReductionSummary {
                    horizontal_blocking_angle_bits: 45.0_f32.to_bits(),
                    damage_types: Some(HolderSetSummary {
                        registry_ids: Vec::new(),
                        tag: Some("minecraft:bypasses_shield".to_string()),
                    }),
                    base_bits: nan_bits,
                    factor_bits: (-1.25_f32).to_bits(),
                },
                DamageReductionSummary {
                    horizontal_blocking_angle_bits: 180.0_f32.to_bits(),
                    damage_types: Some(HolderSetSummary {
                        registry_ids: vec![0, 47],
                        tag: None,
                    }),
                    base_bits: 2.0_f32.to_bits(),
                    factor_bits: 0.5_f32.to_bits(),
                },
            ],
            item_damage: ItemDamageFunctionSummary {
                threshold_bits: (-2.0_f32).to_bits(),
                base_bits: 3.0_f32.to_bits(),
                factor_bits: nan_bits,
            },
            bypassed_by: Some(HolderSetSummary {
                registry_ids: Vec::new(),
                tag: None,
            }),
            block_sound: Some(SoundEventSummary {
                registry_id: Some(11),
                sound_id: None,
                fixed_range_bits: None,
            }),
            disable_sound: Some(SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:shield_disable".to_string()),
                fixed_range_bits: Some((-0.0_f32).to_bits()),
            }),
        }),
        ..DataComponentPatchSummary::default()
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    let encoded = encoded.into_inner();

    let mut expected = Encoder::new();
    expected.write_f32(-0.0);
    expected.write_f32(f32::INFINITY);
    expected.write_var_i32(2);
    expected.write_f32(45.0);
    expected.write_bool(true);
    expected.write_var_i32(0);
    expected.write_string("minecraft:bypasses_shield");
    expected.write_f32(f32::from_bits(nan_bits));
    expected.write_f32(-1.25);
    expected.write_f32(180.0);
    expected.write_bool(true);
    expected.write_var_i32(3);
    expected.write_var_i32(0);
    expected.write_var_i32(47);
    expected.write_f32(2.0);
    expected.write_f32(0.5);
    expected.write_f32(-2.0);
    expected.write_f32(3.0);
    expected.write_f32(f32::from_bits(nan_bits));
    expected.write_bool(true);
    expected.write_var_i32(1);
    expected.write_bool(true);
    expected.write_var_i32(12);
    expected.write_bool(true);
    expected.write_var_i32(0);
    expected.write_string("example:shield_disable");
    expected.write_bool(true);
    expected.write_f32(-0.0);
    assert_eq!(encoded, expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.blocks_attacks, summary.blocks_attacks);
}

#[test]
fn blocks_attacks_descriptor_enforces_bounds_and_canonical_unions() {
    let reduction = DamageReductionSummary {
        horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
        damage_types: None,
        base_bits: 0.0_f32.to_bits(),
        factor_bits: 1.0_f32.to_bits(),
    };
    let valid = DataComponentPatchSummary {
        blocks_attacks: Some(BlocksAttacksSummary {
            block_delay_seconds_bits: 0.0_f32.to_bits(),
            disable_cooldown_scale_bits: 1.0_f32.to_bits(),
            damage_reductions: vec![reduction.clone()],
            item_damage: ItemDamageFunctionSummary {
                threshold_bits: 1.0_f32.to_bits(),
                base_bits: 0.0_f32.to_bits(),
                factor_bits: 1.0_f32.to_bits(),
            },
            bypassed_by: None,
            block_sound: None,
            disable_sound: None,
        }),
        ..DataComponentPatchSummary::default()
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID).unwrap();

    let mut empty = valid.clone();
    empty
        .blocks_attacks
        .as_mut()
        .unwrap()
        .damage_reductions
        .clear();
    descriptor.encode(&empty, &mut Encoder::new()).unwrap();
    let mut maximal = valid.clone();
    maximal.blocks_attacks.as_mut().unwrap().damage_reductions =
        vec![reduction; MAX_DATA_COMPONENT_LIST_ITEMS];
    descriptor.encode(&maximal, &mut Encoder::new()).unwrap();
    let mut maximal_holder_set = valid.clone();
    maximal_holder_set
        .blocks_attacks
        .as_mut()
        .unwrap()
        .bypassed_by = Some(HolderSetSummary {
        registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS],
        tag: None,
    });
    descriptor
        .encode(&maximal_holder_set, &mut Encoder::new())
        .unwrap();

    let mut cases = Vec::new();
    cases.push((
        DataComponentPatchSummary::default(),
        "missing blocks_attacks",
    ));
    let mut negative_holder = valid.clone();
    negative_holder
        .blocks_attacks
        .as_mut()
        .unwrap()
        .damage_reductions[0]
        .damage_types = Some(HolderSetSummary {
        registry_ids: vec![-1],
        tag: None,
    });
    cases.push((negative_holder, "invalid damage type holder id -1"));
    let mut noncanonical_tag = valid.clone();
    noncanonical_tag
        .blocks_attacks
        .as_mut()
        .unwrap()
        .bypassed_by = Some(HolderSetSummary {
        registry_ids: Vec::new(),
        tag: Some("bypasses_shield".to_string()),
    });
    cases.push((noncanonical_tag, "noncanonical tag resource location"));
    let mut mixed_sound = valid.clone();
    mixed_sound.blocks_attacks.as_mut().unwrap().block_sound = Some(SoundEventSummary {
        registry_id: Some(0),
        sound_id: Some("minecraft:item.shield.block".to_string()),
        fixed_range_bits: None,
    });
    cases.push((mixed_sound, "sound event holder projections"));
    let mut missing_sound = valid.clone();
    missing_sound.blocks_attacks.as_mut().unwrap().block_sound = Some(SoundEventSummary {
        registry_id: None,
        sound_id: None,
        fixed_range_bits: None,
    });
    cases.push((missing_sound, "sound event holder projections"));
    let mut invalid_sound_id = valid.clone();
    invalid_sound_id
        .blocks_attacks
        .as_mut()
        .unwrap()
        .block_sound = Some(SoundEventSummary {
        registry_id: None,
        sound_id: Some("Invalid Path".to_string()),
        fixed_range_bits: None,
    });
    cases.push((invalid_sound_id, "invalid resource"));
    let mut noncanonical_sound_id = valid.clone();
    noncanonical_sound_id
        .blocks_attacks
        .as_mut()
        .unwrap()
        .block_sound = Some(SoundEventSummary {
        registry_id: None,
        sound_id: Some("item.shield.block".to_string()),
        fixed_range_bits: None,
    });
    cases.push((noncanonical_sound_id, "noncanonical resource location"));
    let mut overflowing_sound_holder = valid.clone();
    overflowing_sound_holder
        .blocks_attacks
        .as_mut()
        .unwrap()
        .block_sound = Some(SoundEventSummary {
        registry_id: Some(i32::MAX),
        sound_id: None,
        fixed_range_bits: None,
    });
    cases.push((overflowing_sound_holder, "sound event holder projections"));

    for (summary, expected_error) in cases {
        let mut encoded = Encoder::new();
        let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error}");
    }

    let mut oversized_holder_set = valid.clone();
    oversized_holder_set
        .blocks_attacks
        .as_mut()
        .unwrap()
        .bypassed_by = Some(HolderSetSummary {
        registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
        tag: None,
    });
    let mut encoded = Encoder::new();
    assert!(matches!(
        descriptor.encode(&oversized_holder_set, &mut encoded),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                && max == MAX_DATA_COMPONENT_LIST_ITEMS
    ));

    let mut oversized = valid;
    oversized.blocks_attacks.as_mut().unwrap().damage_reductions = vec![
        DamageReductionSummary {
            horizontal_blocking_angle_bits: 90.0_f32.to_bits(),
            damage_types: None,
            base_bits: 0.0_f32.to_bits(),
            factor_bits: 1.0_f32.to_bits(),
        };
        MAX_DATA_COMPONENT_LIST_ITEMS
            + 1
    ];
    let mut encoded = Encoder::new();
    assert!(matches!(
        descriptor.encode(&oversized, &mut encoded),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                && max == MAX_DATA_COMPONENT_LIST_ITEMS
    ));
}

#[test]
fn instrument_descriptor_round_trips_reference_and_direct_holders() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID).unwrap();
    for holder_id in [0, 300] {
        let summary = DataComponentPatchSummary {
            instrument: InstrumentComponentSummary {
                registry_id: Some(holder_id),
                ..InstrumentComponentSummary::default()
            },
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id + 1);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.instrument.registry_id, Some(holder_id));
        assert!(decoded.instrument.direct.is_none());
    }

    let direct = direct_instrument_patch();
    let json = serde_json::to_value(&direct).unwrap();
    assert!(json["instrument_id"].is_null());
    assert_eq!(json["instrument_description"], "Custom horn");
    assert_eq!(
        json["instrument_description_styled"],
        json["instrument_direct"]["description_styled"]
    );
    assert_eq!(
        serde_json::from_value::<DataComponentPatchSummary>(json).unwrap(),
        direct
    );
    let mut encoded = Encoder::new();
    descriptor.encode(&direct, &mut encoded).unwrap();
    assert_eq!(
        encoded.clone().into_inner(),
        [
            0, // direct instrument holder
            8, // sound event registry holder id 7 + 1
            0x40, 0x50, 0, 0, // use_duration = 3.25
            0x42, 0xc0, 0, 0, // range = 96.0
            8, 0, 11, b'C', b'u', b's', b't', b'o', b'm', b' ', b'h', b'o', b'r', b'n',
        ]
    );

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, direct);

    let mut boundary = direct_instrument_patch();
    let instrument = boundary.instrument.direct.as_mut().unwrap();
    instrument.sound_event = SoundEventSummary {
        registry_id: None,
        sound_id: Some("example:signal".to_string()),
        fixed_range_bits: Some((-0.0_f32).to_bits()),
    };
    instrument.use_duration_bits = f32::MAX.to_bits();
    instrument.range_bits = f32::from_bits(1).to_bits();
    let mut encoded = Encoder::new();
    descriptor.encode(&boundary, &mut encoded).unwrap();
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, boundary);
}

#[test]
fn instrument_descriptor_rejects_noncanonical_or_persistently_invalid_values() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID).unwrap();
    let valid = direct_instrument_patch();
    let mut cases = vec![
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            instrument: InstrumentComponentSummary {
                registry_id: Some(-1),
                ..InstrumentComponentSummary::default()
            },
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            instrument: InstrumentComponentSummary {
                registry_id: Some(i32::MAX),
                ..InstrumentComponentSummary::default()
            },
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            instrument: InstrumentComponentSummary {
                registry_id: Some(0),
                ..valid.instrument.clone()
            },
            ..valid.clone()
        },
    ];
    for field in ["use_duration", "range"] {
        for invalid_bits in [
            0.0_f32.to_bits(),
            (-0.0_f32).to_bits(),
            (-1.0_f32).to_bits(),
            f32::INFINITY.to_bits(),
            f32::NAN.to_bits(),
        ] {
            let mut invalid = valid.clone();
            let instrument = invalid.instrument.direct.as_mut().unwrap();
            match field {
                "use_duration" => instrument.use_duration_bits = invalid_bits,
                "range" => instrument.range_bits = invalid_bits,
                _ => unreachable!(),
            }
            cases.push(invalid);
        }
    }
    let mut mixed_sound = valid.clone();
    mixed_sound
        .instrument
        .direct
        .as_mut()
        .unwrap()
        .sound_event
        .sound_id = Some("example:mixed".to_string());
    cases.push(mixed_sound);
    let mut missing_component = valid;
    missing_component
        .instrument
        .direct
        .as_mut()
        .unwrap()
        .description_nbt = None;
    cases.push(missing_component);

    for summary in cases {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn banner_patterns_descriptor_round_trips_reference_and_direct_layers_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID).unwrap();
    let empty = DataComponentPatchSummary::default();
    let mut encoded = Encoder::new();
    descriptor.encode(&empty, &mut encoded).unwrap();
    assert_eq!(encoded.clone().into_inner(), [0]);
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, empty);

    let summary = DataComponentPatchSummary {
        banner_pattern_layers: banner_pattern_layers(),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    let mut expected = Encoder::new();
    expected.write_var_i32(2);
    expected.write_var_i32(5);
    expected.write_var_i32(VANILLA_DYE_COLOR_RED_ID);
    expected.write_var_i32(0);
    expected.write_string("minecraft:stripe_bottom");
    expected.write_string("block.minecraft.banner.stripe_bottom");
    expected.write_var_i32(VANILLA_DYE_COLOR_BLUE_ID);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, summary);

    let maximal_translation = format!("{}a", "😀".repeat((MAX_STRING_CHARS.saturating_sub(1)) / 2));
    assert_eq!(maximal_translation.encode_utf16().count(), MAX_STRING_CHARS);
    let boundary = DataComponentPatchSummary {
        banner_pattern_layers: vec![
            BannerPatternLayerSummary {
                registry_id: Some(i32::MAX - 1),
                color_id: VANILLA_DYE_COLOR_RED_ID,
                ..BannerPatternLayerSummary::default()
            },
            BannerPatternLayerSummary {
                asset_id: Some("example:signal".to_string()),
                translation_key: Some(maximal_translation.into()),
                color_id: VANILLA_DYE_COLOR_BLUE_ID,
                ..BannerPatternLayerSummary::default()
            },
        ],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&boundary, &mut encoded).unwrap();
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, boundary);

    let maximal = DataComponentPatchSummary {
        banner_pattern_layers: vec![
            BannerPatternLayerSummary {
                registry_id: Some(0),
                color_id: VANILLA_DYE_COLOR_RED_ID,
                ..BannerPatternLayerSummary::default()
            };
            MAX_DATA_COMPONENT_LIST_ITEMS
        ],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&maximal, &mut encoded).unwrap();
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, maximal);
}

#[test]
fn banner_patterns_descriptor_rejects_noncanonical_layers() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID).unwrap();
    let valid = DataComponentPatchSummary {
        banner_pattern_layers: banner_pattern_layers(),
        ..DataComponentPatchSummary::default()
    };
    let mut cases = Vec::new();
    for invalid_layer in [
        BannerPatternLayerSummary {
            registry_id: Some(-1),
            color_id: VANILLA_DYE_COLOR_RED_ID,
            ..BannerPatternLayerSummary::default()
        },
        BannerPatternLayerSummary {
            registry_id: Some(i32::MAX),
            color_id: VANILLA_DYE_COLOR_RED_ID,
            ..BannerPatternLayerSummary::default()
        },
        BannerPatternLayerSummary {
            registry_id: Some(0),
            asset_id: Some("example:mixed".to_string()),
            color_id: VANILLA_DYE_COLOR_RED_ID,
            ..BannerPatternLayerSummary::default()
        },
        BannerPatternLayerSummary {
            color_id: VANILLA_DYE_COLOR_RED_ID,
            ..BannerPatternLayerSummary::default()
        },
        BannerPatternLayerSummary {
            asset_id: Some("example:missing_translation".to_string()),
            color_id: VANILLA_DYE_COLOR_RED_ID,
            ..BannerPatternLayerSummary::default()
        },
        BannerPatternLayerSummary {
            asset_id: Some("Bad:asset".to_string()),
            translation_key: Some("block.example.bad".into()),
            color_id: VANILLA_DYE_COLOR_RED_ID,
            ..BannerPatternLayerSummary::default()
        },
        BannerPatternLayerSummary {
            asset_id: Some("example:oversized".to_string()),
            translation_key: Some("x".repeat(MAX_STRING_CHARS + 1).into()),
            color_id: VANILLA_DYE_COLOR_RED_ID,
            ..BannerPatternLayerSummary::default()
        },
        BannerPatternLayerSummary {
            registry_id: Some(0),
            color_id: -1,
            ..BannerPatternLayerSummary::default()
        },
        BannerPatternLayerSummary {
            registry_id: Some(0),
            color_id: 16,
            ..BannerPatternLayerSummary::default()
        },
    ] {
        cases.push(DataComponentPatchSummary {
            banner_pattern_layers: vec![banner_pattern_layers()[0].clone(), invalid_layer],
            ..DataComponentPatchSummary::default()
        });
    }

    for summary in cases {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }

    let oversized = DataComponentPatchSummary {
        banner_pattern_layers: vec![
            banner_pattern_layers()[0].clone();
            MAX_DATA_COMPONENT_LIST_ITEMS + 1
        ],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    assert!(matches!(
        descriptor.encode(&oversized, &mut encoded),
        Err(ProtocolError::PacketTooLarge(count, max))
            if count == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                && max == MAX_DATA_COMPONENT_LIST_ITEMS
    ));

    descriptor.encode(&valid, &mut Encoder::new()).unwrap();
}

#[test]
fn lodestone_tracker_descriptor_round_trips_optional_global_position_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID)
            .unwrap();
    let untargeted = DataComponentPatchSummary {
        lodestone_tracked: Some(true),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&untargeted, &mut encoded).unwrap();
    assert_eq!(encoded.clone().into_inner(), [0, 1]);
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, untargeted);

    let targeted = DataComponentPatchSummary {
        lodestone_target: Some(LodestoneTargetSummary {
            dimension: "minecraft:the_nether".to_string(),
            pos: chunks::BlockPos {
                x: 12,
                y: -64,
                z: 34,
            },
        }),
        lodestone_tracked: Some(false),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&targeted, &mut encoded).unwrap();
    let mut expected = Encoder::new();
    expected.write_bool(true);
    expected.write_string("minecraft:the_nether");
    expected.write_i64(chunks::encode_block_pos(chunks::BlockPos {
        x: 12,
        y: -64,
        z: 34,
    }));
    expected.write_bool(false);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, targeted);

    for pos in [
        chunks::BlockPos {
            x: -33_554_432,
            y: -2_048,
            z: -33_554_432,
        },
        chunks::BlockPos {
            x: 33_554_431,
            y: 2_047,
            z: 33_554_431,
        },
    ] {
        let boundary = DataComponentPatchSummary {
            lodestone_target: Some(LodestoneTargetSummary {
                dimension: "minecraft:overworld".to_string(),
                pos,
            }),
            lodestone_tracked: Some(true),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&boundary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, boundary);
    }
}

#[test]
fn lodestone_tracker_descriptor_rejects_noncanonical_projections() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID)
            .unwrap();
    let mut cases = vec![DataComponentPatchSummary::default()];
    for dimension in [
        "overworld".to_string(),
        "Bad:dimension".to_string(),
        format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1)),
    ] {
        cases.push(DataComponentPatchSummary {
            lodestone_target: Some(LodestoneTargetSummary {
                dimension,
                pos: chunks::BlockPos { x: 0, y: 0, z: 0 },
            }),
            lodestone_tracked: Some(true),
            ..DataComponentPatchSummary::default()
        });
    }
    for summary in cases {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn lodestone_tracker_wire_wraps_out_of_range_coordinates_like_java() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID)
            .unwrap();
    let cases = [
        (
            chunks::BlockPos {
                x: 33_554_432,
                y: 2_048,
                z: -33_554_433,
            },
            true,
            [
                0x01, 0x13, 0x6d, 0x69, 0x6e, 0x65, 0x63, 0x72, 0x61, 0x66, 0x74, 0x3a, 0x6f, 0x76,
                0x65, 0x72, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x80, 0x00, 0x00, 0x1f, 0xff, 0xff, 0xf8,
                0x00, 0x01,
            ]
            .as_slice(),
            chunks::BlockPos {
                x: -33_554_432,
                y: -2_048,
                z: 33_554_431,
            },
        ),
        (
            chunks::BlockPos {
                x: i32::MAX,
                y: i32::MIN,
                z: 0,
            },
            false,
            [
                0x01, 0x13, 0x6d, 0x69, 0x6e, 0x65, 0x63, 0x72, 0x61, 0x66, 0x74, 0x3a, 0x6f, 0x76,
                0x65, 0x72, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0xff, 0xff, 0xff, 0xc0, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ]
            .as_slice(),
            chunks::BlockPos { x: -1, y: 0, z: 0 },
        ),
    ];
    for (pos, tracked, golden, wrapped) in cases {
        let source = DataComponentPatchSummary {
            lodestone_target: Some(LodestoneTargetSummary {
                dimension: "minecraft:overworld".into(),
                pos,
            }),
            lodestone_tracked: Some(tracked),
            ..Default::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&source, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        assert_eq!(wire, golden);
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.lodestone_target.unwrap().pos, wrapped);
        assert_eq!(decoded.lodestone_tracked, Some(tracked));
    }
}

#[test]
fn painting_variant_descriptor_round_trips_reference_and_direct_holders_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID).unwrap();
    for holder_id in [0, i32::MAX - 1] {
        let reference = DataComponentPatchSummary {
            painting_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&reference, &mut encoded).unwrap();
        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id + 1);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, reference);
    }

    let direct = DataComponentPatchSummary {
        painting_variant_direct: Some(direct_painting_variant()),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&direct, &mut encoded).unwrap();
    let mut expected = Encoder::new();
    expected.write_var_i32(0);
    expected.write_var_i32(4);
    expected.write_var_i32(2);
    expected.write_string("minecraft:gallery");
    expected.write_bool(true);
    expected.write_bytes(
        &encode_nbt_summary_bounded(
            Some(&NbtSummaryValue::String("Gallery".into())),
            i32::MAX as usize,
            MAX_NBT_DEPTH,
        )
        .unwrap(),
    );
    expected.write_bool(false);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, direct);

    // PaintingVariant.java:28-32 uses unrestricted VarInts on the wire.
    for (width, height) in [
        (1, 16),
        (16, 1),
        (0, 17),
        (20, 0),
        (-1, -2),
        (i32::MIN, i32::MAX),
    ] {
        let boundary = DataComponentPatchSummary {
            painting_variant_direct: Some(PaintingVariantSummary {
                width,
                height,
                ..direct_painting_variant()
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&boundary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, boundary);
    }
}

#[test]
fn painting_variant_descriptor_rejects_noncanonical_holders() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID).unwrap();
    let direct = direct_painting_variant();
    let mut cases = vec![
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            painting_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            painting_variant_id: Some(i32::MAX),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            painting_variant_id: Some(0),
            painting_variant_direct: Some(direct.clone()),
            ..DataComponentPatchSummary::default()
        },
    ];
    for invalid in [
        PaintingVariantSummary {
            asset_id: "gallery".to_string(),
            ..direct.clone()
        },
        PaintingVariantSummary {
            asset_id: "Bad:gallery".to_string(),
            ..direct.clone()
        },
        PaintingVariantSummary {
            asset_id: format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1)),
            ..direct.clone()
        },
        PaintingVariantSummary {
            title: Some("Stale".to_string()),
            ..direct.clone()
        },
        PaintingVariantSummary {
            title_styled: None,
            ..direct.clone()
        },
        PaintingVariantSummary {
            title_nbt: None,
            ..direct.clone()
        },
        PaintingVariantSummary {
            author_nbt: Some(Box::new(NbtSummaryValue::String("Dangling".into()))),
            ..direct.clone()
        },
    ] {
        cases.push(DataComponentPatchSummary {
            painting_variant_direct: Some(invalid),
            ..DataComponentPatchSummary::default()
        });
    }

    for summary in cases {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn bees_descriptor_round_trips_empty_and_populated_lists_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BEES_TYPE_ID).unwrap();
    let empty = DataComponentPatchSummary {
        bees_present: true,
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&empty, &mut encoded).unwrap();
    assert_eq!(encoded.clone().into_inner(), [0]);
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, empty);

    let summary = DataComponentPatchSummary {
        bees_present: true,
        bees: vec![bee_occupant()],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    let mut expected = Encoder::new();
    expected.write_var_i32(1);
    expected.write_var_i32(VANILLA_ENTITY_TYPE_BEE_ID);
    expected.write_bytes(&[10, 0]);
    expected.write_var_i32(i32::MIN);
    expected.write_var_i32(i32::MAX);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, summary);
    let mut report = serde_json::to_value(&summary).unwrap();
    assert_eq!(report["bees_count"], 1);
    report["bees_count"] = serde_json::json!(99);
    assert_eq!(
        serde_json::from_value::<DataComponentPatchSummary>(report).unwrap(),
        summary
    );
    assert_eq!(
        serde_json::to_value(DataComponentPatchSummary::default()).unwrap()["bees_count"],
        0
    );
}

#[test]
fn bees_descriptor_rejects_noncanonical_nested_values() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BEES_TYPE_ID).unwrap();
    let valid = DataComponentPatchSummary {
        bees_present: true,
        bees: vec![bee_occupant()],
        ..DataComponentPatchSummary::default()
    };
    let mut cases = vec![
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            bees: vec![bee_occupant()],
            ..DataComponentPatchSummary::default()
        },
    ];
    for invalid_bee in [
        BeeOccupantSummary {
            entity_type_id: -1,
            ..bee_occupant()
        },
        BeeOccupantSummary {
            entity_type_id: VANILLA_ENTITY_TYPES.len() as i32,
            ..bee_occupant()
        },
        BeeOccupantSummary {
            entity_data_tag: None,
            ..bee_occupant()
        },
        BeeOccupantSummary {
            entity_data_tag: Some(NbtSummaryValue::String("not a compound".into())),
            ..bee_occupant()
        },
        BeeOccupantSummary {
            entity_data_tag: Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String("minecraft:bee".into()),
            }])),
            ..bee_occupant()
        },
    ] {
        cases.push(DataComponentPatchSummary {
            bees_present: true,
            bees: vec![bee_occupant(), invalid_bee],
            ..DataComponentPatchSummary::default()
        });
    }

    for summary in cases {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }

    let oversized = DataComponentPatchSummary {
        bees_present: true,
        bees: vec![bee_occupant(); MAX_DATA_COMPONENT_LIST_ITEMS + 1],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    assert!(matches!(
        descriptor.encode(&oversized, &mut encoded),
        Err(ProtocolError::PacketTooLarge(count, max))
            if count == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                && max == MAX_DATA_COMPONENT_LIST_ITEMS
    ));

    let mut encoded = Encoder::new();
    descriptor.encode(&valid, &mut encoded).unwrap();
    assert!(!encoded.into_inner().is_empty());

    let maximal = DataComponentPatchSummary {
        bees_present: true,
        bees: vec![bee_occupant(); MAX_DATA_COMPONENT_LIST_ITEMS],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&maximal, &mut encoded).unwrap();
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, maximal);
}

#[test]
fn jukebox_playable_descriptor_round_trips_reference_and_direct_song_holders() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID).unwrap();
    for holder_id in [0, 300] {
        let summary = DataComponentPatchSummary {
            jukebox_song_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id + 1);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.jukebox_song_id, Some(holder_id));
        assert!(decoded.jukebox_direct_song.is_none());
    }

    let direct = direct_jukebox_playable_patch();
    let mut encoded = Encoder::new();
    descriptor.encode(&direct, &mut encoded).unwrap();
    assert_eq!(
        encoded.clone().into_inner(),
        [
            0, // direct jukebox song holder
            8, // sound event registry holder id 7 + 1
            8, 0, 11, b'C', b'u', b's', b't', b'o', b'm', b' ', b's', b'o', b'n', b'g', 0x42, 0x70,
            0, 0, // length_in_seconds = 60.0
            7, // comparator_output
        ]
    );

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, direct);

    let mut boundary = direct_jukebox_playable_patch();
    let song = boundary.jukebox_direct_song.as_mut().unwrap();
    song.sound_event = SoundEventSummary {
        registry_id: None,
        sound_id: Some("example:music.custom".to_string()),
        fixed_range_bits: Some((-0.0_f32).to_bits()),
    };
    song.length_in_seconds_bits = f32::MAX.to_bits();
    song.comparator_output = 15;
    let mut encoded = Encoder::new();
    descriptor.encode(&boundary, &mut encoded).unwrap();
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, boundary);
}

#[test]
fn jukebox_playable_descriptor_rejects_noncanonical_or_persistently_invalid_values() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID).unwrap();
    let valid = direct_jukebox_playable_patch();
    let mut cases = vec![
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            jukebox_song_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            jukebox_song_id: Some(i32::MAX),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            jukebox_song_id: Some(0),
            ..valid.clone()
        },
    ];
    for invalid_length_bits in [
        0.0_f32.to_bits(),
        (-0.0_f32).to_bits(),
        (-1.0_f32).to_bits(),
        f32::INFINITY.to_bits(),
        f32::NAN.to_bits(),
    ] {
        let mut invalid = valid.clone();
        invalid
            .jukebox_direct_song
            .as_mut()
            .unwrap()
            .length_in_seconds_bits = invalid_length_bits;
        cases.push(invalid);
    }
    for comparator_output in [-1, 16] {
        let mut invalid = valid.clone();
        invalid
            .jukebox_direct_song
            .as_mut()
            .unwrap()
            .comparator_output = comparator_output;
        cases.push(invalid);
    }
    let mut mixed_sound = valid.clone();
    mixed_sound
        .jukebox_direct_song
        .as_mut()
        .unwrap()
        .sound_event
        .sound_id = Some("example:mixed".to_string());
    cases.push(mixed_sound);
    let mut missing_component = valid;
    missing_component
        .jukebox_direct_song
        .as_mut()
        .unwrap()
        .description_nbt = None;
    cases.push(missing_component);

    for summary in cases {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn break_sound_descriptor_round_trips_reference_and_direct_holders_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID).unwrap();
    for holder_id in [0, i32::MAX - 1] {
        let referenced = DataComponentPatchSummary {
            break_sound: Some(SoundEventSummary {
                registry_id: Some(holder_id),
                sound_id: None,
                fixed_range_bits: None,
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&referenced, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id + 1);
        assert_eq!(wire, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, referenced);
    }

    for fixed_range_bits in [None, Some(12.5_f32.to_bits())] {
        let direct = DataComponentPatchSummary {
            break_sound: Some(SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:item.custom_break".to_string()),
                fixed_range_bits,
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&direct, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(0);
        expected.write_string("example:item.custom_break");
        expected.write_bool(fixed_range_bits.is_some());
        if let Some(bits) = fixed_range_bits {
            expected.write_f32(f32::from_bits(bits));
        }
        assert_eq!(wire, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, direct);
    }
}

#[test]
fn break_sound_descriptor_rejects_noncanonical_holders() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID).unwrap();
    let cases = [
        None,
        Some(SoundEventSummary {
            registry_id: Some(-1),
            sound_id: None,
            fixed_range_bits: None,
        }),
        Some(SoundEventSummary {
            registry_id: Some(i32::MAX),
            sound_id: None,
            fixed_range_bits: None,
        }),
        Some(SoundEventSummary {
            registry_id: Some(0),
            sound_id: Some("example:item.custom_break".to_string()),
            fixed_range_bits: None,
        }),
        Some(SoundEventSummary {
            registry_id: Some(0),
            sound_id: None,
            fixed_range_bits: Some(12.5_f32.to_bits()),
        }),
        Some(SoundEventSummary {
            registry_id: None,
            sound_id: None,
            fixed_range_bits: Some(12.5_f32.to_bits()),
        }),
        Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some("item.custom_break".to_string()),
            fixed_range_bits: None,
        }),
        Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some("Bad:custom_break".to_string()),
            fixed_range_bits: None,
        }),
        Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some(format!("example:{}", "a".repeat(MAX_STRING_CHARS + 1))),
            fixed_range_bits: None,
        }),
    ];
    for break_sound in cases {
        let summary = DataComponentPatchSummary {
            break_sound,
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn map_id_descriptor_round_trips_the_full_signed_domain_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID).unwrap();
    for map_id in [i32::MIN, -1, 0, 1, i32::MAX] {
        let summary = DataComponentPatchSummary {
            map_id: Some(map_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(map_id);
        assert_eq!(wire, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, summary);
    }
}

#[test]
fn map_id_descriptor_rejects_a_missing_value_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    encoded.write_u8(0xaa);
    assert!(descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .is_err());
    assert_eq!(encoded.into_inner(), [0xaa]);
}

#[test]
fn map_decorations_descriptor_round_trips_all_builtin_holders_and_canonicalizes_numeric_nbt() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID).unwrap();
    let decorations = vanilla_map_decoration_types()
        .enumerate()
        .map(|(index, decoration_type)| ItemMapDecorationSummary {
            id: format!("marker {index}").into(),
            decoration_type_id: decoration_type.type_id,
            decoration_type_resource_id: decoration_type.resource_id.to_string(),
            x_bits: (index as f64 - 12.5).to_bits(),
            z_bits: (64.0 - index as f64).to_bits(),
            rotation_bits: (index as f32 * 11.25).to_bits(),
        })
        .collect::<Vec<_>>();
    for expected in [Vec::new(), decorations] {
        let expected =
            parse_map_decorations_nbt(&canonical_map_decorations_nbt(&expected)).unwrap();
        let summary = DataComponentPatchSummary {
            map_decorations: Some(expected.clone()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.map_decorations.as_deref(),
            Some(expected.as_slice())
        );
        assert_eq!(
            decoded
                .codec_backed_nbt
                .get(&VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID),
            Some(&canonical_map_decorations_nbt(&expected))
        );
    }

    let shorthand = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
        name: "home".into(),
        value: NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "ignored".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "type".into(),
                value: NbtSummaryValue::String("mansion".into()),
            },
            NbtSummaryEntry {
                name: "x".into(),
                value: NbtSummaryValue::Int(-12),
            },
            NbtSummaryEntry {
                name: "z".into(),
                value: NbtSummaryValue::Float(2.5_f32.to_bits()),
            },
            NbtSummaryEntry {
                name: "rotation".into(),
                value: NbtSummaryValue::Double(45.25_f64.to_bits()),
            },
        ]),
    }]);
    let wire =
        encode_nbt_summary_bounded(Some(&shorthand), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
    let mut decoded = DataComponentPatchSummary::default();
    descriptor
        .decode(&mut Decoder::new(&wire), &mut decoded)
        .unwrap();
    let decoration = &decoded.map_decorations.as_ref().unwrap()[0];
    let mansion = vanilla_map_decoration_type_for_resource_id("minecraft:mansion").unwrap();
    assert_eq!(decoration.id, "home");
    assert_eq!(decoration.decoration_type_id, mansion.type_id);
    assert_eq!(decoration.decoration_type_resource_id, "minecraft:mansion");
    assert_eq!(f64::from_bits(decoration.x_bits), -12.0);
    assert_eq!(f64::from_bits(decoration.z_bits), 2.5);
    assert_eq!(f32::from_bits(decoration.rotation_bits), 45.25);
    assert_eq!(
        decoded
            .codec_backed_nbt
            .get(&VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID),
        Some(&canonical_map_decorations_nbt(
            decoded.map_decorations.as_deref().unwrap()
        ))
    );
}

#[test]
fn map_decorations_descriptor_rejects_noncanonical_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID).unwrap();
    let frame = vanilla_map_decoration_type_for_resource_id("minecraft:frame").unwrap();
    let valid = ItemMapDecorationSummary {
        id: "home".into(),
        decoration_type_id: frame.type_id,
        decoration_type_resource_id: frame.resource_id.to_string(),
        x_bits: 1.0_f64.to_bits(),
        z_bits: 2.0_f64.to_bits(),
        rotation_bits: 90.0_f32.to_bits(),
    };
    let mut cases = vec![
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            map_decorations: Some(vec![valid.clone(), valid.clone()]),
            ..DataComponentPatchSummary::default()
        },
    ];
    let mut unknown_id = valid.clone();
    unknown_id.decoration_type_id = i32::MAX;
    cases.push(DataComponentPatchSummary {
        map_decorations: Some(vec![unknown_id]),
        ..DataComponentPatchSummary::default()
    });
    let mut mismatched = valid.clone();
    mismatched.decoration_type_resource_id = "minecraft:mansion".to_string();
    cases.push(DataComponentPatchSummary {
        map_decorations: Some(vec![mismatched]),
        ..DataComponentPatchSummary::default()
    });
    let mut shorthand = valid.clone();
    shorthand.decoration_type_resource_id = "frame".to_string();
    cases.push(DataComponentPatchSummary {
        map_decorations: Some(vec![shorthand]),
        ..DataComponentPatchSummary::default()
    });
    cases.push(DataComponentPatchSummary {
        map_decorations: Some(vec![valid.clone()]),
        codec_backed_nbt: BTreeMap::from([(
            VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
            canonical_map_decorations_nbt(&[]),
        )]),
        ..DataComponentPatchSummary::default()
    });
    cases.push(DataComponentPatchSummary {
        map_decorations: Some(vec![valid; MAX_DATA_COMPONENT_LIST_ITEMS + 1]),
        ..DataComponentPatchSummary::default()
    });

    for summary in cases {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn map_decorations_descriptor_rejects_malformed_codec_nbt() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID).unwrap();
    let entry = |fields| {
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "home".into(),
            value: NbtSummaryValue::Compound(fields),
        }])
    };
    for malformed in [
        NbtSummaryValue::String("home".into()),
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "home".into(),
            value: NbtSummaryValue::Int(1),
        }]),
        entry(Vec::new()),
        entry(vec![
            NbtSummaryEntry {
                name: "type".into(),
                value: NbtSummaryValue::String("minecraft:missing".into()),
            },
            NbtSummaryEntry {
                name: "x".into(),
                value: NbtSummaryValue::Double(0.0_f64.to_bits()),
            },
            NbtSummaryEntry {
                name: "z".into(),
                value: NbtSummaryValue::Double(0.0_f64.to_bits()),
            },
            NbtSummaryEntry {
                name: "rotation".into(),
                value: NbtSummaryValue::Float(0.0_f32.to_bits()),
            },
        ]),
        entry(vec![
            NbtSummaryEntry {
                name: "type".into(),
                value: NbtSummaryValue::String("minecraft:frame".into()),
            },
            NbtSummaryEntry {
                name: "x".into(),
                value: NbtSummaryValue::String("near".into()),
            },
            NbtSummaryEntry {
                name: "z".into(),
                value: NbtSummaryValue::Double(0.0_f64.to_bits()),
            },
            NbtSummaryEntry {
                name: "rotation".into(),
                value: NbtSummaryValue::Float(0.0_f32.to_bits()),
            },
        ]),
    ] {
        let wire =
            encode_nbt_summary_bounded(Some(&malformed), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        assert!(descriptor
            .decode(
                &mut Decoder::new(&wire),
                &mut DataComponentPatchSummary::default()
            )
            .is_err());
    }
    let missing = encode_nbt_summary_bounded(None, i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
    assert!(descriptor
        .decode(
            &mut Decoder::new(&missing),
            &mut DataComponentPatchSummary::default()
        )
        .is_err());
}

#[test]
fn recipes_descriptor_preserves_order_duplicates_and_canonicalizes_resource_keys() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID).unwrap();
    for expected in [
        Vec::new(),
        vec![
            "minecraft:bread".to_string(),
            "example:tools/hammer".to_string(),
            "minecraft:bread".to_string(),
        ],
    ] {
        let summary = DataComponentPatchSummary {
            recipe_keys: Some(expected.clone()),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.recipe_keys.as_deref(), Some(expected.as_slice()));
        assert_eq!(
            decoded
                .codec_backed_nbt
                .get(&VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID),
            Some(&canonical_recipe_keys_nbt(&expected))
        );
    }

    let shorthand = NbtSummaryValue::List(vec![
        NbtSummaryValue::String("bread".into()),
        NbtSummaryValue::String(":torch".into()),
        NbtSummaryValue::String("bread".into()),
    ]);
    let wire =
        encode_nbt_summary_bounded(Some(&shorthand), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
    let mut decoded = DataComponentPatchSummary::default();
    descriptor
        .decode(&mut Decoder::new(&wire), &mut decoded)
        .unwrap();
    assert_eq!(
        decoded.recipe_keys.as_deref(),
        Some(
            [
                "minecraft:bread".to_string(),
                "minecraft:torch".to_string(),
                "minecraft:bread".to_string(),
            ]
            .as_slice()
        )
    );
}

#[test]
fn recipes_descriptor_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID).unwrap();
    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            recipe_keys: Some(vec!["bread".to_string()]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            recipe_keys: Some(vec!["Invalid".to_string()]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            recipe_keys: Some(vec!["minecraft:bread".to_string()]),
            codec_backed_nbt: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
                canonical_recipe_keys_nbt(&[]),
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            recipe_keys: Some(vec![
                "minecraft:bread".to_string();
                MAX_DATA_COMPONENT_LIST_ITEMS + 1
            ]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    for malformed in [
        NbtSummaryValue::Compound(Vec::new()),
        NbtSummaryValue::List(vec![NbtSummaryValue::Int(1)]),
        NbtSummaryValue::List(vec![NbtSummaryValue::String("Invalid".into())]),
    ] {
        let wire =
            encode_nbt_summary_bounded(Some(&malformed), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        assert!(descriptor
            .decode(
                &mut Decoder::new(&wire),
                &mut DataComponentPatchSummary::default()
            )
            .is_err());
    }
    let missing = encode_nbt_summary_bounded(None, i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
    assert!(descriptor
        .decode(
            &mut Decoder::new(&missing),
            &mut DataComponentPatchSummary::default()
        )
        .is_err());
}

#[test]
fn map_post_processing_descriptor_round_trips_both_wire_ids() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID)
            .unwrap();
    for value in [
        MapPostProcessingSummary::Lock,
        MapPostProcessingSummary::Scale,
    ] {
        let summary = DataComponentPatchSummary {
            map_post_processing: Some(value),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(value.id());
        assert_eq!(wire, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, summary);
    }
}

#[test]
fn map_post_processing_descriptor_rejects_a_missing_value_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID)
            .unwrap();
    let mut encoded = Encoder::new();
    encoded.write_u8(0xaa);
    assert!(descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .is_err());
    assert_eq!(encoded.into_inner(), [0xaa]);
}

#[test]
fn dye_descriptor_round_trips_all_canonical_wire_ids() {
    let descriptor = vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DYE_TYPE_ID).unwrap();
    for value in VANILLA_DYE_COLOR_VALUES {
        let summary = DataComponentPatchSummary {
            dye_color_id: Some(value.id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(value.id);
        assert_eq!(wire, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, summary);
    }
}

#[test]
fn dye_descriptor_rejects_missing_and_noncanonical_ids_atomically() {
    let descriptor = vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DYE_TYPE_ID).unwrap();
    let past_end = i32::try_from(VANILLA_DYE_COLOR_VALUES.len()).unwrap();
    for dye_color_id in [None, Some(-1), Some(past_end)] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor
            .encode(
                &DataComponentPatchSummary {
                    dye_color_id,
                    ..DataComponentPatchSummary::default()
                },
                &mut encoded,
            )
            .is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn dyed_color_descriptor_round_trips_the_full_signed_domain_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID).unwrap();
    for color in [i32::MIN, -1, 0, 1, i32::MAX] {
        let summary = DataComponentPatchSummary {
            dyed_color: Some(color),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_i32(color);
        assert_eq!(wire, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, summary);
    }
}

#[test]
fn dyed_color_descriptor_rejects_a_missing_value_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    encoded.write_u8(0xaa);
    assert!(descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .is_err());
    assert_eq!(encoded.into_inner(), [0xaa]);
}

#[test]
fn map_color_descriptor_round_trips_the_full_signed_domain_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID).unwrap();
    for color in [i32::MIN, -1, 0, 1, i32::MAX] {
        let summary = DataComponentPatchSummary {
            map_color: Some(color),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_i32(color);
        assert_eq!(wire, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, summary);
    }
}

#[test]
fn map_color_descriptor_rejects_a_missing_value_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    encoded.write_u8(0xaa);
    assert!(descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .is_err());
    assert_eq!(encoded.into_inner(), [0xaa]);
}

#[test]
fn provides_trim_material_descriptor_round_trips_reference_and_direct_holders_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID)
            .unwrap();
    for holder_id in [0, i32::MAX - 1] {
        let referenced = DataComponentPatchSummary {
            provides_trim_material_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&referenced, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id + 1);
        assert_eq!(wire, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&wire);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded, referenced);
    }

    let material = direct_trim_patch()
        .armor_trim_material_direct
        .expect("direct trim fixture has a material");
    let direct = DataComponentPatchSummary {
        provides_trim_material_direct: Some(material.clone()),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&direct, &mut encoded).unwrap();
    let wire = encoded.into_inner();
    let mut expected = Encoder::new();
    expected.write_var_i32(0);
    expected.write_string(&material.asset_name);
    expected.write_var_i32(material.override_armor_assets.len() as i32);
    for (equipment_asset, suffix) in &material.override_armor_assets {
        expected.write_string(equipment_asset);
        expected.write_string(suffix);
    }
    expected.write_bytes(
        &encode_nbt_summary_bounded(
            material.description_nbt.as_deref(),
            i32::MAX as usize,
            MAX_NBT_DEPTH,
        )
        .unwrap(),
    );
    assert_eq!(wire, expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&wire);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, direct);
}

#[test]
fn provides_trim_material_descriptor_rejects_noncanonical_values() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID)
            .unwrap();
    let material = direct_trim_patch()
        .armor_trim_material_direct
        .expect("direct trim fixture has a material");
    let mut cases = vec![
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            provides_trim_material_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            provides_trim_material_id: Some(i32::MAX),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            provides_trim_material_id: Some(0),
            provides_trim_material_direct: Some(material.clone()),
            ..DataComponentPatchSummary::default()
        },
    ];
    for invalid in [
        TrimMaterialSummary {
            asset_name: "Ruby".to_string(),
            ..material.as_ref().clone()
        },
        TrimMaterialSummary {
            description_nbt: None,
            ..material.as_ref().clone()
        },
        TrimMaterialSummary {
            description: "Stale".to_string(),
            ..material.as_ref().clone()
        },
        TrimMaterialSummary {
            override_armor_assets: (0..=MAX_DATA_COMPONENT_LIST_ITEMS)
                .map(|index| (format!("example:asset_{index}"), "ruby".to_string()))
                .collect(),
            ..material.as_ref().clone()
        },
    ] {
        cases.push(DataComponentPatchSummary {
            provides_trim_material_direct: Some(Box::new(invalid)),
            ..DataComponentPatchSummary::default()
        });
    }

    for summary in cases {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn trim_descriptor_round_trips_registry_and_direct_holders_in_wire_order() {
    let direct = direct_trim_patch();
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TRIM_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    descriptor.encode(&direct, &mut encoded).unwrap();
    let encoded = encoded.into_inner();

    let material = direct.armor_trim_material_direct.as_deref().unwrap();
    let pattern = direct.armor_trim_pattern_direct.as_deref().unwrap();
    let mut expected = Encoder::new();
    expected.write_var_i32(0);
    expected.write_string("ruby");
    expected.write_var_i32(2);
    for (equipment_asset, suffix) in &material.override_armor_assets {
        expected.write_string(equipment_asset);
        expected.write_string(suffix);
    }
    expected.write_bytes(
        &encode_nbt_summary_bounded(
            material.description_nbt.as_deref(),
            i32::MAX as usize,
            MAX_NBT_DEPTH,
        )
        .unwrap(),
    );
    expected.write_var_i32(0);
    expected.write_string("example:spiral");
    expected.write_bytes(
        &encode_nbt_summary_bounded(
            pattern.description_nbt.as_deref(),
            i32::MAX as usize,
            MAX_NBT_DEPTH,
        )
        .unwrap(),
    );
    expected.write_bool(true);
    assert_eq!(encoded, expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(
        decoded.armor_trim_material_direct,
        direct.armor_trim_material_direct
    );
    assert_eq!(
        decoded.armor_trim_pattern_direct,
        direct.armor_trim_pattern_direct
    );

    let referenced = DataComponentPatchSummary {
        armor_trim_material_id: Some(0),
        armor_trim_pattern_id: Some(i32::MAX - 1),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&referenced, &mut encoded).unwrap();
    let encoded = encoded.into_inner();
    let mut expected = Encoder::new();
    expected.write_var_i32(1);
    expected.write_var_i32(i32::MAX);
    assert_eq!(encoded, expected.into_inner());
    let mut decoded = DataComponentPatchSummary::default();
    descriptor
        .decode(&mut Decoder::new(&encoded), &mut decoded)
        .unwrap();
    assert_eq!(decoded.armor_trim_material_id, Some(0));
    assert_eq!(decoded.armor_trim_pattern_id, Some(i32::MAX - 1));

    let mixed = DataComponentPatchSummary {
        armor_trim_material_id: Some(41),
        armor_trim_pattern_direct: direct.armor_trim_pattern_direct.clone(),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&mixed, &mut encoded).unwrap();
    let mut decoded = DataComponentPatchSummary::default();
    descriptor
        .decode(&mut Decoder::new(&encoded.into_inner()), &mut decoded)
        .unwrap();
    assert_eq!(decoded.armor_trim_material_id, Some(41));
    assert_eq!(
        decoded.armor_trim_pattern_direct,
        mixed.armor_trim_pattern_direct
    );
}

#[test]
fn trim_descriptor_rejects_noncanonical_holders_and_payloads() {
    let valid = direct_trim_patch();
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TRIM_TYPE_ID).unwrap();
    let mut cases = Vec::new();

    let mut missing_material = valid.clone();
    missing_material.armor_trim_material_direct = None;
    cases.push((missing_material, "trim material holder projections"));
    let mut mixed_material = valid.clone();
    mixed_material.armor_trim_material_id = Some(0);
    cases.push((mixed_material, "trim material holder projections"));
    for registry_id in [-1, i32::MAX] {
        let mut invalid = valid.clone();
        invalid.armor_trim_material_direct = None;
        invalid.armor_trim_material_id = Some(registry_id);
        cases.push((invalid, "trim material holder projections"));
    }
    let mut invalid_asset_name = valid.clone();
    invalid_asset_name
        .armor_trim_material_direct
        .as_mut()
        .unwrap()
        .asset_name = "Ruby".to_string();
    cases.push((invalid_asset_name, "not a valid resource path"));
    let mut invalid_equipment_asset = valid.clone();
    invalid_equipment_asset
        .armor_trim_material_direct
        .as_mut()
        .unwrap()
        .override_armor_assets =
        BTreeMap::from([("Invalid Asset".to_string(), "ruby".to_string())]);
    cases.push((invalid_equipment_asset, "invalid resource location"));
    let mut noncanonical_equipment_asset = valid.clone();
    noncanonical_equipment_asset
        .armor_trim_material_direct
        .as_mut()
        .unwrap()
        .override_armor_assets = BTreeMap::from([("netherite".to_string(), "ruby".to_string())]);
    cases.push((
        noncanonical_equipment_asset,
        "noncanonical resource location",
    ));
    let mut invalid_override_suffix = valid.clone();
    invalid_override_suffix
        .armor_trim_material_direct
        .as_mut()
        .unwrap()
        .override_armor_assets =
        BTreeMap::from([("minecraft:netherite".to_string(), "Ruby".to_string())]);
    cases.push((invalid_override_suffix, "not a valid resource path"));
    let mut missing_material_nbt = valid.clone();
    missing_material_nbt
        .armor_trim_material_direct
        .as_mut()
        .unwrap()
        .description_nbt = None;
    cases.push((missing_material_nbt, "missing component nbt"));
    let mut missing_material_styled = valid.clone();
    missing_material_styled
        .armor_trim_material_direct
        .as_mut()
        .unwrap()
        .description_styled = None;
    cases.push((missing_material_styled, "missing styled projection"));
    let mut stale_material_text = valid.clone();
    stale_material_text
        .armor_trim_material_direct
        .as_mut()
        .unwrap()
        .description = "Stale".to_string();
    cases.push((stale_material_text, "noncanonical component projections"));
    let mut malformed_material_nbt = valid.clone();
    malformed_material_nbt
        .armor_trim_material_direct
        .as_mut()
        .unwrap()
        .description_nbt = Some(Box::new(NbtSummaryValue::Int(1)));
    cases.push((malformed_material_nbt, "is not a component"));
    let mut invalid_pattern_asset = valid.clone();
    invalid_pattern_asset
        .armor_trim_pattern_direct
        .as_mut()
        .unwrap()
        .asset_id = "Invalid Pattern".to_string();
    cases.push((invalid_pattern_asset, "invalid resource location"));
    let mut noncanonical_pattern_asset = valid.clone();
    noncanonical_pattern_asset
        .armor_trim_pattern_direct
        .as_mut()
        .unwrap()
        .asset_id = "spiral".to_string();
    cases.push((noncanonical_pattern_asset, "noncanonical resource location"));
    let mut mixed_pattern = valid.clone();
    mixed_pattern.armor_trim_pattern_id = Some(0);
    cases.push((mixed_pattern, "trim pattern holder projections"));
    for registry_id in [-1, i32::MAX] {
        let mut invalid = valid.clone();
        invalid.armor_trim_pattern_direct = None;
        invalid.armor_trim_pattern_id = Some(registry_id);
        cases.push((invalid, "trim pattern holder projections"));
    }
    let mut stale_pattern_styled = valid.clone();
    stale_pattern_styled
        .armor_trim_pattern_direct
        .as_mut()
        .unwrap()
        .description_styled
        .as_mut()
        .unwrap()[0]
        .text = "Stale".to_string();
    cases.push((stale_pattern_styled, "noncanonical component projections"));

    for (summary, expected_error) in cases {
        let mut encoded = Encoder::new();
        let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
        assert!(
            error.to_string().contains(expected_error),
            "{error} did not contain {expected_error:?}"
        );
    }

    let mut oversized = valid;
    oversized
        .armor_trim_material_direct
        .as_mut()
        .unwrap()
        .override_armor_assets = (0..=MAX_DATA_COMPONENT_LIST_ITEMS)
        .map(|index| (format!("example:asset_{index}"), "ruby".to_string()))
        .collect();
    let mut encoded = Encoder::new();
    assert!(matches!(
        descriptor.encode(&oversized, &mut encoded),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                && max == MAX_DATA_COMPONENT_LIST_ITEMS
    ));
}

#[test]
fn trim_descriptor_rejects_invalid_material_resource_paths_on_decode() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TRIM_TYPE_ID).unwrap();
    for (asset_name, suffix, expected_error) in [
        ("Ruby", "ruby", "asset_name"),
        ("ruby", "Ruby", "override_armor_assets suffix"),
    ] {
        let mut wire = Encoder::new();
        wire.write_var_i32(0);
        wire.write_string(asset_name);
        wire.write_var_i32(1);
        wire.write_string("minecraft:netherite");
        wire.write_string(suffix);
        wire.write_bytes(
            &encode_nbt_summary_bounded(
                Some(&NbtSummaryValue::String("Ruby".into())),
                i32::MAX as usize,
                MAX_NBT_DEPTH,
            )
            .unwrap(),
        );
        wire.write_var_i32(1);
        let error = descriptor
            .decode(
                &mut Decoder::new(&wire.into_inner()),
                &mut DataComponentPatchSummary::default(),
            )
            .unwrap_err();
        assert!(
            error.to_string().contains(expected_error),
            "{error} did not contain {expected_error:?}"
        );
    }
}

#[test]
fn potion_effect_descriptors_round_trip_exact_wire_values() {
    let potion_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID).unwrap();
    let potion = DataComponentPatchSummary {
        potion_contents: PotionContentsSummary {
            potion_id: Some(3),
            custom_color: Some(i32::MIN),
            custom_effects: vec![MobEffectInstanceSummary {
                effect_id: 2,
                amplifier: 255,
                duration: -1,
                ambient: true,
                show_particles: false,
                show_icon: true,
                hidden_effect: Some(Box::new(MobEffectDetailsSummary {
                    amplifier: 1,
                    duration: i32::MAX,
                    ambient: false,
                    show_particles: true,
                    show_icon: false,
                    hidden_effect: None,
                })),
            }],
            custom_name: Some("swift😀".into()),
        },
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    potion_descriptor.encode(&potion, &mut encoded).unwrap();
    let encoded = encoded.into_inner();
    let mut expected = Encoder::new();
    expected.write_bool(true);
    expected.write_var_i32(3);
    expected.write_bool(true);
    expected.write_i32(i32::MIN);
    expected.write_var_i32(1);
    expected.write_var_i32(2);
    expected.write_var_i32(255);
    expected.write_var_i32(-1);
    expected.write_bool(true);
    expected.write_bool(false);
    expected.write_bool(true);
    expected.write_bool(false);
    expected.write_bool(true);
    expected.write_string("swift😀");
    assert_eq!(encoded, expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    potion_descriptor
        .decode(&mut decoder, &mut decoded)
        .unwrap();
    assert!(decoder.is_empty());
    assert_eq!(
        decoded.potion_contents.potion_id,
        potion.potion_contents.potion_id
    );
    assert_eq!(
        decoded.potion_contents.custom_color,
        potion.potion_contents.custom_color
    );
    let mut visible_effects = potion.potion_contents.custom_effects.clone();
    visible_effects[0].hidden_effect = None;
    assert_eq!(decoded.potion_contents.custom_effects, visible_effects);
    assert_eq!(
        decoded.potion_contents.custom_name,
        potion.potion_contents.custom_name
    );

    let stew_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID)
            .unwrap();
    let stew = DataComponentPatchSummary {
        suspicious_stew_effects: vec![
            SuspiciousStewEffectSummary {
                effect_id: 0,
                duration: i32::MIN,
            },
            SuspiciousStewEffectSummary {
                effect_id: 1,
                duration: i32::MAX,
            },
        ],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    stew_descriptor.encode(&stew, &mut encoded).unwrap();
    let encoded = encoded.into_inner();
    let mut expected = Encoder::new();
    expected.write_var_i32(2);
    expected.write_var_i32(0);
    expected.write_var_i32(i32::MIN);
    expected.write_var_i32(1);
    expected.write_var_i32(i32::MAX);
    assert_eq!(encoded, expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    stew_descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(
        decoded.suspicious_stew_effects,
        stew.suspicious_stew_effects
    );
}

#[test]
fn potion_effect_wire_uses_java_visible_copy_golden() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID).unwrap();
    let patch = DataComponentPatchSummary {
        potion_contents: PotionContentsSummary {
            custom_effects: vec![MobEffectInstanceSummary {
                effect_id: 2,
                amplifier: 1,
                duration: 200,
                ambient: false,
                show_particles: true,
                show_icon: true,
                hidden_effect: Some(Box::new(MobEffectDetailsSummary {
                    amplifier: 0,
                    duration: 40,
                    ambient: false,
                    show_particles: true,
                    show_icon: false,
                    hidden_effect: None,
                })),
            }],
            ..PotionContentsSummary::default()
        },
        ..DataComponentPatchSummary::default()
    };
    let mut wire = Encoder::new();
    descriptor.encode(&patch, &mut wire).unwrap();
    assert_eq!(
        wire.into_inner(),
        [0x00, 0x00, 0x01, 0x02, 0x01, 0xc8, 0x01, 0x00, 0x01, 0x01, 0x00, 0x00]
    );
    assert!(patch.potion_contents.custom_effects[0]
        .hidden_effect
        .is_some());
}

#[test]
fn potion_duration_scale_descriptor_round_trips_all_raw_float_bits() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID)
            .unwrap();
    for bits in [
        0.0_f32.to_bits(),
        (-0.0_f32).to_bits(),
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
        0x7fc1_2345,
    ] {
        let patch = DataComponentPatchSummary {
            potion_duration_scale_bits: Some(bits),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&patch, &mut encoded).unwrap();
        assert_eq!(encoded.into_inner(), bits.to_be_bytes());

        let mut decoded = DataComponentPatchSummary::default();
        let bytes = bits.to_be_bytes();
        let mut decoder = Decoder::new(&bytes);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.potion_duration_scale_bits, Some(bits));
    }

    let mut encoded = Encoder::new();
    let error = descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("missing potion_duration_scale component value"));
    assert!(encoded.into_inner().is_empty());
}

#[test]
fn potion_effect_descriptors_enforce_bounds() {
    let effect = MobEffectInstanceSummary {
        effect_id: 0,
        amplifier: 0,
        duration: 1,
        ambient: false,
        show_particles: true,
        show_icon: true,
        hidden_effect: None,
    };
    let potion_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID).unwrap();
    let empty_potion = DataComponentPatchSummary::default();
    potion_descriptor
        .encode(&empty_potion, &mut Encoder::new())
        .unwrap();

    let maximal_name = format!("{}a", "😀".repeat((MAX_STRING_CHARS.saturating_sub(1)) / 2));
    assert_eq!(maximal_name.encode_utf16().count(), MAX_STRING_CHARS);
    let maximal_potion = DataComponentPatchSummary {
        potion_contents: PotionContentsSummary {
            custom_effects: vec![effect.clone(); MAX_DATA_COMPONENT_LIST_ITEMS],
            custom_name: Some(maximal_name.clone().into()),
            ..PotionContentsSummary::default()
        },
        ..DataComponentPatchSummary::default()
    };
    potion_descriptor
        .encode(&maximal_potion, &mut Encoder::new())
        .unwrap();

    let mut oversized_wire_name = Encoder::new();
    oversized_wire_name.write_bool(false);
    oversized_wire_name.write_bool(false);
    oversized_wire_name.write_var_i32(0);
    oversized_wire_name.write_bool(true);
    oversized_wire_name.write_string(&"😀".repeat(MAX_STRING_CHARS / 2 + 1));
    let decode_error = potion_descriptor
        .decode(
            &mut Decoder::new(&oversized_wire_name.into_inner()),
            &mut DataComponentPatchSummary::default(),
        )
        .unwrap_err();
    assert!(decode_error.to_string().contains("max is 32767"));

    let mut potion_cases = Vec::new();
    let mut invalid_potion = empty_potion.clone();
    invalid_potion.potion_contents.potion_id =
        Some(crate::potion_types::VANILLA_POTIONS.len() as i32);
    potion_cases.push((invalid_potion, "invalid vanilla potion holder id"));
    let mut invalid_effect = empty_potion.clone();
    invalid_effect.potion_contents.custom_effects = vec![MobEffectInstanceSummary {
        effect_id: crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len() as i32,
        ..effect.clone()
    }];
    potion_cases.push((invalid_effect, "invalid vanilla mob effect holder id"));
    for amplifier in [-1, 256] {
        let mut invalid_amplifier = empty_potion.clone();
        invalid_amplifier.potion_contents.custom_effects = vec![MobEffectInstanceSummary {
            amplifier,
            ..effect.clone()
        }];
        potion_cases.push((invalid_amplifier, "outside 0..=255"));
    }
    let mut oversized_name = empty_potion.clone();
    oversized_name.potion_contents.custom_name = Some(format!("{maximal_name}b").into());
    potion_cases.push((oversized_name, "max is 32767"));

    for (summary, expected_error) in potion_cases {
        let mut encoded = Encoder::new();
        let error = potion_descriptor
            .encode(&summary, &mut encoded)
            .unwrap_err();
        assert!(
            error.to_string().contains(expected_error),
            "{error} did not contain {expected_error:?}"
        );
    }

    let oversized_potion = DataComponentPatchSummary {
        potion_contents: PotionContentsSummary {
            custom_effects: vec![effect; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
            ..PotionContentsSummary::default()
        },
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    assert!(matches!(
        potion_descriptor.encode(&oversized_potion, &mut encoded),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                && max == MAX_DATA_COMPONENT_LIST_ITEMS
    ));

    let stew_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID)
            .unwrap();
    let stew_effect = SuspiciousStewEffectSummary {
        effect_id: 0,
        duration: 160,
    };
    stew_descriptor
        .encode(
            &DataComponentPatchSummary {
                suspicious_stew_effects: vec![stew_effect.clone(); MAX_DATA_COMPONENT_LIST_ITEMS],
                ..DataComponentPatchSummary::default()
            },
            &mut Encoder::new(),
        )
        .unwrap();

    let mut invalid_stew_effect = stew_effect.clone();
    invalid_stew_effect.effect_id = crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len() as i32;
    let mut encoded = Encoder::new();
    let error = stew_descriptor
        .encode(
            &DataComponentPatchSummary {
                suspicious_stew_effects: vec![invalid_stew_effect],
                ..DataComponentPatchSummary::default()
            },
            &mut encoded,
        )
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("invalid vanilla mob effect holder id"));

    let mut encoded = Encoder::new();
    assert!(matches!(
        stew_descriptor.encode(
            &DataComponentPatchSummary {
                suspicious_stew_effects: vec![
                    stew_effect;
                    MAX_DATA_COMPONENT_LIST_ITEMS + 1
                ],
                ..DataComponentPatchSummary::default()
            },
            &mut encoded,
        ),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                && max == MAX_DATA_COMPONENT_LIST_ITEMS
    ));
}
