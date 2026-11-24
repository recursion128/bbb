use super::*;

#[test]
fn firework_descriptors_round_trip_exact_wire_values() {
    let explosion_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID)
            .unwrap();
    let standalone = DataComponentPatchSummary {
        firework_explosion: Some(FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Burst,
            colors: vec![i32::MIN, 0x11_22_33, i32::MAX],
            fade_colors: vec![-1, 0x44_55_66],
            has_trail: true,
            has_twinkle: false,
        }),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    explosion_descriptor
        .encode(&standalone, &mut encoded)
        .unwrap();
    let encoded = encoded.into_inner();
    let mut expected = Encoder::new();
    expected.write_var_i32(FireworkExplosionShapeSummary::Burst.id());
    expected.write_var_i32(3);
    expected.write_i32(i32::MIN);
    expected.write_i32(0x11_22_33);
    expected.write_i32(i32::MAX);
    expected.write_var_i32(2);
    expected.write_i32(-1);
    expected.write_i32(0x44_55_66);
    expected.write_bool(true);
    expected.write_bool(false);
    assert_eq!(encoded, expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    explosion_descriptor
        .decode(&mut decoder, &mut decoded)
        .unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.firework_explosion, standalone.firework_explosion);

    let shapes = [
        FireworkExplosionShapeSummary::SmallBall,
        FireworkExplosionShapeSummary::LargeBall,
        FireworkExplosionShapeSummary::Star,
        FireworkExplosionShapeSummary::Creeper,
        FireworkExplosionShapeSummary::Burst,
    ];
    let explosions = shapes
        .into_iter()
        .enumerate()
        .map(|(index, shape)| FireworkExplosionSummary {
            shape,
            colors: vec![index as i32, -(index as i32) - 1],
            fade_colors: (index % 2 == 0)
                .then_some(index as i32)
                .into_iter()
                .collect(),
            has_trail: index % 2 == 0,
            has_twinkle: index % 2 != 0,
        })
        .collect::<Vec<_>>();
    let fireworks = DataComponentPatchSummary {
        fireworks_flight_duration: Some(-1),
        fireworks_explosions: explosions,
        ..DataComponentPatchSummary::default()
    };
    let fireworks_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    fireworks_descriptor
        .encode(&fireworks, &mut encoded)
        .unwrap();
    let encoded = encoded.into_inner();
    let mut expected = Encoder::new();
    expected.write_var_i32(-1);
    expected.write_var_i32(fireworks.fireworks_explosions.len() as i32);
    for explosion in &fireworks.fireworks_explosions {
        expected.write_var_i32(explosion.shape.id());
        expected.write_var_i32(explosion.colors.len() as i32);
        for color in &explosion.colors {
            expected.write_i32(*color);
        }
        expected.write_var_i32(explosion.fade_colors.len() as i32);
        for color in &explosion.fade_colors {
            expected.write_i32(*color);
        }
        expected.write_bool(explosion.has_trail);
        expected.write_bool(explosion.has_twinkle);
    }
    assert_eq!(encoded, expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    fireworks_descriptor
        .decode(&mut decoder, &mut decoded)
        .unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.fireworks_flight_duration, Some(-1));
    assert_eq!(decoded.fireworks_explosions, fireworks.fireworks_explosions);
}

#[test]
fn firework_descriptors_enforce_collection_bounds_and_presence() {
    let explosion_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID)
            .unwrap();
    let valid_explosion = DataComponentPatchSummary {
        firework_explosion: Some(FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Star,
            has_trail: false,
            has_twinkle: true,
            ..Default::default()
        }),
        ..DataComponentPatchSummary::default()
    };
    let mut maximal_colors = valid_explosion.clone();
    maximal_colors.firework_explosion.as_mut().unwrap().colors =
        vec![0; MAX_DATA_COMPONENT_LIST_ITEMS];
    explosion_descriptor
        .encode(&maximal_colors, &mut Encoder::new())
        .unwrap();

    let mut missing = valid_explosion.clone();
    missing.firework_explosion = None;
    assert!(explosion_descriptor
        .encode(&missing, &mut Encoder::new())
        .is_err());
    for colors_are_fades in [false, true] {
        let mut oversized = valid_explosion.clone();
        let colors = vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1];
        if colors_are_fades {
            oversized.firework_explosion.as_mut().unwrap().fade_colors = colors;
        } else {
            oversized.firework_explosion.as_mut().unwrap().colors = colors;
        }
        let mut encoded = Encoder::new();
        assert!(matches!(
            explosion_descriptor.encode(&oversized, &mut encoded),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
    }

    let fireworks_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID).unwrap();
    let explosion = FireworkExplosionSummary {
        shape: FireworkExplosionShapeSummary::SmallBall,
        colors: Vec::new(),
        fade_colors: Vec::new(),
        has_trail: false,
        has_twinkle: false,
    };
    let valid_fireworks = DataComponentPatchSummary {
        fireworks_flight_duration: Some(0),
        fireworks_explosions: vec![explosion.clone()],
        ..DataComponentPatchSummary::default()
    };
    let maximal_fireworks = DataComponentPatchSummary {
        fireworks_flight_duration: Some(i32::MIN),
        fireworks_explosions: vec![explosion.clone(); MAX_FIREWORK_EXPLOSIONS],
        ..DataComponentPatchSummary::default()
    };
    fireworks_descriptor
        .encode(&maximal_fireworks, &mut Encoder::new())
        .unwrap();

    let mut fireworks_cases = Vec::new();
    let mut missing_duration = valid_fireworks.clone();
    missing_duration.fireworks_flight_duration = None;
    fireworks_cases.push((missing_duration, "missing fireworks flight duration"));
    let oversized_explosions = DataComponentPatchSummary {
        fireworks_flight_duration: Some(0),
        fireworks_explosions: vec![explosion.clone(); MAX_FIREWORK_EXPLOSIONS + 1],
        ..DataComponentPatchSummary::default()
    };
    let mut oversized_nested_colors = valid_fireworks;
    oversized_nested_colors.fireworks_explosions[0].colors =
        vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1];

    for (summary, expected_error) in fireworks_cases {
        let mut encoded = Encoder::new();
        let error = fireworks_descriptor
            .encode(&summary, &mut encoded)
            .unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error}");
    }
    for (summary, expected_max) in [
        (oversized_explosions, MAX_FIREWORK_EXPLOSIONS),
        (oversized_nested_colors, MAX_DATA_COMPONENT_LIST_ITEMS),
    ] {
        let mut encoded = Encoder::new();
        assert!(matches!(
            fireworks_descriptor.encode(&summary, &mut encoded),
            Err(ProtocolError::PacketTooLarge(_, max)) if max == expected_max
        ));
    }
}

#[test]
fn use_effects_descriptor_round_trips_raw_float_bits_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID).unwrap();
    for bits in [
        0.0_f32.to_bits(),
        (-0.0_f32).to_bits(),
        1.0_f32.to_bits(),
        f32::INFINITY.to_bits(),
        f32::NAN.to_bits(),
    ] {
        let summary = DataComponentPatchSummary {
            use_effects: Some(UseEffectsSummary {
                can_sprint: true,
                interact_vibrations: false,
                speed_multiplier: f32::from_bits(bits),
            }),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        assert_eq!(&encoded[..2], &[1, 0]);
        assert_eq!(&encoded[2..], &bits.to_be_bytes());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.use_effects, summary.use_effects);
    }

    let mut encoded = Encoder::new();
    encoded.write_u8(0xaa);
    let error = descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("missing use_effects component value"));
    assert_eq!(encoded.into_inner(), [0xaa]);
}

#[test]
fn minimum_attack_charge_descriptor_round_trips_raw_float_bits() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID)
            .unwrap();
    for bits in [
        0.0_f32.to_bits(),
        (-0.0_f32).to_bits(),
        1.0_f32.to_bits(),
        f32::INFINITY.to_bits(),
        f32::NAN.to_bits(),
    ] {
        let summary = DataComponentPatchSummary {
            minimum_attack_charge_bits: Some(bits),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        assert_eq!(encoded, bits.to_be_bytes());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.minimum_attack_charge_bits, Some(bits));
    }

    let mut encoded = Encoder::new();
    encoded.write_u8(0xaa);
    let error = descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("missing minimum_attack_charge component value"));
    assert_eq!(encoded.into_inner(), [0xaa]);
}

#[test]
fn damage_type_descriptor_round_trips_holder_ids_and_rejects_missing_or_negative_values() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID).unwrap();
    for (holder_id, expected) in [(0, vec![0]), (49, vec![49]), (300, vec![0xac, 2])] {
        let summary = DataComponentPatchSummary {
            damage_type_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        assert_eq!(encoded, expected);

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.damage_type_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            damage_type_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn item_model_descriptor_round_trips_canonical_identifier() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID).unwrap();
    let summary = DataComponentPatchSummary {
        item_model: Some("example:models/variant".to_string()),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let mut expected = Encoder::new();
    expected.write_string("example:models/variant");
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.item_model, summary.item_model);
}

#[test]
fn item_model_descriptor_rejects_missing_noncanonical_and_oversized_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ITEM_MODEL_TYPE_ID).unwrap();
    let oversized = format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1));
    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            item_model: Some("stone".to_string()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            item_model: Some("Minecraft:stone".to_string()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            item_model: Some(oversized),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn tooltip_style_descriptor_round_trips_canonical_identifier() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID).unwrap();
    let summary = DataComponentPatchSummary {
        tooltip_style: Some("example:rare".to_string()),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let mut expected = Encoder::new();
    expected.write_string("example:rare");
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.tooltip_style, summary.tooltip_style);
}

#[test]
fn tooltip_style_descriptor_rejects_missing_noncanonical_and_oversized_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID).unwrap();
    let oversized = format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1));
    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            tooltip_style: Some("rare".to_string()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            tooltip_style: Some("Minecraft:rare".to_string()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            tooltip_style: Some(oversized),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn note_block_sound_descriptor_round_trips_canonical_identifier() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID).unwrap();
    let summary = DataComponentPatchSummary {
        note_block_sound: Some("bbb:custom/chime".to_string()),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let mut expected = Encoder::new();
    expected.write_string("bbb:custom/chime");
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.note_block_sound, summary.note_block_sound);

    let mut encoded = Encoder::new();
    encoded.write_string("custom/chime");
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(
        decoded.note_block_sound.as_deref(),
        Some("minecraft:custom/chime")
    );
}

#[test]
fn note_block_sound_descriptor_rejects_missing_noncanonical_and_oversized_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID).unwrap();
    let oversized = format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1));
    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            note_block_sound: Some("custom/chime".to_string()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            note_block_sound: Some("BBB:custom/chime".to_string()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            note_block_sound: Some(oversized),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn pot_decorations_descriptor_normalizes_missing_faces_to_four_bricks() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID).unwrap();
    let stone = crate::item_types::VANILLA_ITEM_STONE_ID;
    let summary = DataComponentPatchSummary {
        pot_decorations_item_ids: vec![stone],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let mut expected = Encoder::new();
    expected.write_var_i32(4);
    expected.write_var_i32(stone);
    expected.write_var_i32(VANILLA_ITEM_BRICK_ID);
    expected.write_var_i32(VANILLA_ITEM_BRICK_ID);
    expected.write_var_i32(VANILLA_ITEM_BRICK_ID);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(
        decoded.pot_decorations_item_ids,
        vec![
            stone,
            VANILLA_ITEM_BRICK_ID,
            VANILLA_ITEM_BRICK_ID,
            VANILLA_ITEM_BRICK_ID,
        ]
    );
}

#[test]
fn base_color_descriptor_round_trips_canonical_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID).unwrap();
    for color_id in [0, VANILLA_DYE_COLOR_RED_ID, 15] {
        let summary = DataComponentPatchSummary {
            base_color_id: Some(color_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(color_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.base_color_id, Some(color_id));
    }

    for wire_color_id in [-1, 16] {
        let mut encoded = Encoder::new();
        encoded.write_var_i32(wire_color_id);
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.base_color_id, Some(0));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            base_color_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            base_color_id: Some(16),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn villager_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID).unwrap();
    for holder_id in [0, 2, i32::MAX] {
        let summary = DataComponentPatchSummary {
            villager_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.villager_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            villager_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn wolf_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID).unwrap();
    for holder_id in [0, 8, i32::MAX] {
        let summary = DataComponentPatchSummary {
            wolf_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.wolf_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            wolf_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn wolf_sound_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID)
            .unwrap();
    for holder_id in [0, 6, i32::MAX] {
        let summary = DataComponentPatchSummary {
            wolf_sound_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.wolf_sound_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            wolf_sound_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn wolf_collar_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID).unwrap();
    for color_id in [0, 11, 15] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
                color_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(color_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, color_id)])
        );
    }

    for raw_color_id in [-1, 16, 99] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_color_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, 0)])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
                -1,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
                16,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn fox_variant_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID).unwrap();
    for variant_id in [0, 1] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
                variant_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(variant_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID, variant_id)])
        );
    }

    for raw_variant_id in [-1, 2, 99] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_variant_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID, 0)])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
                -1,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
                2,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn pig_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID).unwrap();
    for holder_id in [0, 2, i32::MAX] {
        let summary = DataComponentPatchSummary {
            pig_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.pig_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            pig_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn pig_sound_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID)
            .unwrap();
    for holder_id in [0, 1, i32::MAX] {
        let summary = DataComponentPatchSummary {
            pig_sound_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.pig_sound_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            pig_sound_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn cow_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID).unwrap();
    for holder_id in [0, 2, i32::MAX] {
        let summary = DataComponentPatchSummary {
            cow_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.cow_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            cow_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn cow_sound_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID)
            .unwrap();
    for holder_id in [0, 1, i32::MAX] {
        let summary = DataComponentPatchSummary {
            cow_sound_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.cow_sound_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            cow_sound_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn chicken_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID).unwrap();
    for holder_id in [0, 2, i32::MAX] {
        let summary = DataComponentPatchSummary {
            chicken_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.chicken_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            chicken_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn chicken_sound_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID)
            .unwrap();
    for holder_id in [0, 1, i32::MAX] {
        let summary = DataComponentPatchSummary {
            chicken_sound_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.chicken_sound_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            chicken_sound_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn zombie_nautilus_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID)
            .unwrap();
    for holder_id in [0, 1, i32::MAX] {
        let summary = DataComponentPatchSummary {
            zombie_nautilus_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.zombie_nautilus_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            zombie_nautilus_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn frog_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID).unwrap();
    for holder_id in [0, 2, i32::MAX] {
        let summary = DataComponentPatchSummary {
            frog_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.frog_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            frog_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn cat_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID).unwrap();
    for holder_id in [0, 10, i32::MAX] {
        let summary = DataComponentPatchSummary {
            cat_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.cat_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            cat_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn cat_sound_variant_descriptor_round_trips_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID)
            .unwrap();
    for holder_id in [0, 1, i32::MAX] {
        let summary = DataComponentPatchSummary {
            cat_sound_variant_id: Some(holder_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(holder_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.cat_sound_variant_id, Some(holder_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            cat_sound_variant_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn cat_collar_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID).unwrap();
    for color_id in [0, 5, 15] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
                color_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(color_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID, color_id)])
        );
    }

    for raw_color_id in [-1, 16, 99] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_color_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID, 0)])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
                -1,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
                16,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn salmon_size_descriptor_round_trips_canonical_ids_normalizes_decode_clamp_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID).unwrap();
    for variant_id in [0, 1, 2] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
                variant_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(variant_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID, variant_id)])
        );
    }

    for (raw_variant_id, expected_variant_id) in [(-1, 0), (3, 2), (99, 2)] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_variant_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
                expected_variant_id,
            )])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
                -1,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
                3,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn parrot_variant_descriptor_round_trips_canonical_ids_normalizes_decode_clamp_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID).unwrap();
    for variant_id in [0, 2, 4] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
                variant_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(variant_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID, variant_id,)])
        );
    }

    for (raw_variant_id, expected_variant_id) in [(-1, 0), (5, 4), (99, 4)] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_variant_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
                expected_variant_id,
            )])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
                -1,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
                5,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn tropical_fish_pattern_descriptor_round_trips_sparse_ids_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID)
            .unwrap();
    for pattern_id in [0, 257, 1281] {
        let summary = DataComponentPatchSummary {
            tropical_fish_pattern_id: Some(pattern_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(pattern_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.tropical_fish_pattern_id, Some(pattern_id));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            tropical_fish_pattern_id: Some(2),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            tropical_fish_pattern_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn tropical_fish_pattern_color_descriptor_round_trips_canonical_ids_normalizes_decode_fallback_and_rejects_invalid_values_atomically(
) {
    let descriptor = vanilla_data_component_descriptor(
        VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID,
    )
    .unwrap();
    for color_id in [0, 9, 15] {
        let summary = DataComponentPatchSummary {
            tropical_fish_pattern_color_id: Some(color_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(color_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.tropical_fish_pattern_color_id, Some(color_id));
    }

    for raw_color_id in [-1, 16] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_color_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.tropical_fish_pattern_color_id, Some(0));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            tropical_fish_pattern_color_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            tropical_fish_pattern_color_id: Some(16),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn tropical_fish_base_color_descriptor_round_trips_canonical_ids_normalizes_decode_fallback_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID)
            .unwrap();
    for color_id in [0, 11, 15] {
        let summary = DataComponentPatchSummary {
            tropical_fish_base_color_id: Some(color_id),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(color_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.tropical_fish_base_color_id, Some(color_id));
    }

    for raw_color_id in [-1, 16] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_color_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.tropical_fish_base_color_id, Some(0));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            tropical_fish_base_color_id: Some(-1),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            tropical_fish_base_color_id: Some(16),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn mooshroom_variant_descriptor_round_trips_canonical_ids_normalizes_decode_clamp_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID)
            .unwrap();
    for variant_id in [0, 1] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
                variant_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(variant_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID, variant_id,)])
        );
    }

    for (raw_variant_id, expected_variant_id) in [(-1, 0), (2, 1), (99, 1)] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_variant_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
                expected_variant_id,
            )])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
                -1,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
                2,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn rabbit_variant_descriptor_round_trips_sparse_ids_normalizes_decode_fallback_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID).unwrap();
    for variant_id in [0, 5, 99] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
                variant_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(variant_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, variant_id)])
        );
    }

    for raw_variant_id in [-1, 6, 98, 100] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_variant_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID, 0)])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
                6,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
                98,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn horse_variant_descriptor_round_trips_canonical_ids_normalizes_decode_wrap_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID).unwrap();
    for variant_id in [0, 3, 6] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
                variant_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(variant_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID, variant_id)])
        );
    }

    for (raw_variant_id, expected_variant_id) in [(-1, 6), (7, 0), (8, 1)] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_variant_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
                expected_variant_id,
            )])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
                -1,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
                7,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn llama_variant_descriptor_round_trips_canonical_ids_normalizes_decode_clamp_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID).unwrap();
    for variant_id in [0, 2, 3] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                variant_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(variant_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID, variant_id)])
        );
    }

    for (raw_variant_id, expected_variant_id) in [(-1, 0), (4, 3), (99, 3)] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_variant_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(
                VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                expected_variant_id,
            )])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                -1,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                4,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn axolotl_variant_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID).unwrap();
    for variant_id in [0, 2, 4] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
                variant_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(variant_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID, variant_id,)])
        );
    }

    for raw_variant_id in [-1, 5, 99] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_variant_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID, 0)])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
                -1,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
                5,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn sheep_color_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID).unwrap();
    for color_id in [0, 9, 15] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
                color_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(color_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID, color_id)])
        );
    }

    for raw_color_id in [-1, 16, 99] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_color_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID, 0)])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
                -1,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
                16,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn shulker_color_descriptor_round_trips_canonical_ids_normalizes_decode_zero_fallback_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID).unwrap();
    for color_id in [0, 11, 15] {
        let summary = DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
                color_id,
            )]),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(color_id);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID, color_id)])
        );
    }

    for raw_color_id in [-1, 16, 99] {
        let mut wire = Encoder::new();
        wire.write_var_i32(raw_color_id);
        let wire = wire.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.builtin_enum_component_ids,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID, 0)])
        );
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
                -1,
            )]),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            builtin_enum_component_ids: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
                16,
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn ominous_bottle_amplifier_descriptor_round_trips_persistent_range_and_rejects_invalid_values_atomically(
) {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID)
            .unwrap();
    for amplifier in [0, 2, 4] {
        let summary = DataComponentPatchSummary {
            ominous_bottle_amplifier: Some(amplifier),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(amplifier);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.ominous_bottle_amplifier, Some(amplifier));
    }

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            ominous_bottle_amplifier: Some(-1),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            ominous_bottle_amplifier: Some(5),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }
}

#[test]
fn provides_banner_patterns_descriptor_round_trips_direct_and_named_holder_sets() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID)
            .unwrap();
    for summary in [
        DataComponentPatchSummary {
            provides_banner_pattern_registry_ids: vec![3, 1],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            provides_banner_pattern_tag: Some("minecraft:no_item_required".to_string()),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.provides_banner_pattern_registry_ids,
            summary.provides_banner_pattern_registry_ids
        );
        assert_eq!(
            decoded.provides_banner_pattern_tag,
            summary.provides_banner_pattern_tag
        );
    }
}

#[test]
fn provides_banner_patterns_descriptor_rejects_noncanonical_sets() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID)
            .unwrap();
    for summary in [
        DataComponentPatchSummary {
            provides_banner_pattern_registry_ids: vec![-1],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            provides_banner_pattern_tag: Some("no_item_required".to_string()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            provides_banner_pattern_registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn block_state_descriptor_round_trips_empty_and_ordered_properties() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID).unwrap();
    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            block_state_properties: BTreeMap::from([
                ("powered".into(), "true".into()),
                ("facing".into(), "north".into()),
            ])
            .into(),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(summary.block_state_properties.len() as i32);
        for (name, value) in &summary.block_state_properties {
            expected.write_string(&name.to_jdk_string());
            expected.write_string(&value.to_jdk_string());
        }
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(
            decoded.block_state_properties,
            summary.block_state_properties
        );
    }
}

#[test]
fn block_state_descriptor_rejects_collection_and_java_string_overflow() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID).unwrap();
    let maximal = DataComponentPatchSummary {
        block_state_properties: (0..MAX_BLOCK_STATE_PROPERTIES)
            .map(|index| (format!("property_{index}").into(), "value".into()))
            .collect(),
        ..DataComponentPatchSummary::default()
    };
    let mut maximal_wire = Encoder::new();
    descriptor.encode(&maximal, &mut maximal_wire).unwrap();
    let maximal_wire = maximal_wire.into_inner();
    let mut decoder = Decoder::new(&maximal_wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(
        decoded.block_state_properties,
        maximal.block_state_properties
    );

    let oversized = "😀".repeat(MAX_STRING_CHARS / 2 + 1);
    for summary in [
        DataComponentPatchSummary {
            block_state_properties: (0..=MAX_BLOCK_STATE_PROPERTIES)
                .map(|index| (format!("property_{index}").into(), "value".into()))
                .collect(),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            block_state_properties: BTreeMap::from([(oversized.clone().into(), "value".into())])
                .into(),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            block_state_properties: BTreeMap::from([("property".into(), oversized.clone().into())])
                .into(),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn pot_decorations_descriptor_rejects_oversized_and_unknown_items() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID).unwrap();
    for summary in [
        DataComponentPatchSummary {
            pot_decorations_item_ids: vec![VANILLA_ITEM_BRICK_ID; MAX_POT_DECORATIONS + 1],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            pot_decorations_item_ids: vec![i32::MAX],
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }

    let mut invalid_wire = Encoder::new();
    invalid_wire.write_var_i32(1);
    invalid_wire.write_var_i32(i32::MAX);
    let wire = invalid_wire.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    let error = descriptor.decode(&mut decoder, &mut decoded).unwrap_err();
    assert!(error
        .to_string()
        .contains("invalid pot decoration item registry id"));
    assert!(decoded.pot_decorations_item_ids.is_empty());
}

#[test]
fn rarity_descriptor_round_trips_exact_vanilla_ids_and_rejects_missing_value_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_RARITY_TYPE_ID).unwrap();
    for (rarity, expected_id) in [
        (ItemRaritySummary::Common, 0),
        (ItemRaritySummary::Uncommon, 1),
        (ItemRaritySummary::Rare, 2),
        (ItemRaritySummary::Epic, 3),
    ] {
        let summary = DataComponentPatchSummary {
            rarity: Some(rarity),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        assert_eq!(encoded.clone().into_inner(), [expected_id]);

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.rarity, Some(rarity));
    }

    let mut encoded = Encoder::new();
    encoded.write_u8(0xaa);
    assert!(descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .is_err());
    assert_eq!(encoded.into_inner(), [0xaa]);
}

#[test]
fn enchantment_glint_override_descriptor_round_trips_booleans_and_rejects_missing_atomically() {
    let descriptor = vanilla_data_component_descriptor(
        VANILLA_DATA_COMPONENT_ENCHANTMENT_GLINT_OVERRIDE_TYPE_ID,
    )
    .unwrap();
    for override_value in [false, true] {
        let summary = DataComponentPatchSummary {
            enchantment_glint_override: Some(override_value),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        assert_eq!(encoded.clone().into_inner(), [u8::from(override_value)]);

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.enchantment_glint_override, Some(override_value));
    }

    let mut encoded = Encoder::new();
    encoded.write_u8(0xaa);
    assert!(descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .is_err());
    assert_eq!(encoded.into_inner(), [0xaa]);
}

#[test]
fn intangible_projectile_descriptor_round_trips_persistent_fallback_unit_nbt() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID)
            .unwrap();
    let summary = DataComponentPatchSummary {
        intangible_projectile: true,
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    assert_eq!(encoded.clone().into_inner(), [10, 0]);

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert!(decoded.intangible_projectile);
}

#[test]
fn intangible_projectile_descriptor_rejects_absent_unit_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_INTANGIBLE_PROJECTILE_TYPE_ID)
            .unwrap();
    let mut encoded = Encoder::new();
    encoded.write_u8(0xaa);
    let error = descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("missing intangible_projectile component value"));
    assert_eq!(encoded.into_inner(), [0xaa]);
}

#[test]
fn item_enchantment_descriptors_round_trip_maps_and_reject_noncanonical_values() {
    for type_id in [
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    ] {
        let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
        let entries = vec![
            ItemEnchantmentSummary {
                holder_id: 37,
                level: 4,
            },
            ItemEnchantmentSummary {
                holder_id: 300,
                level: 255,
            },
        ];
        let mut summary = DataComponentPatchSummary::default();
        if type_id == VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID {
            summary.enchantments.clone_from(&entries);
        } else {
            summary.stored_enchantments.clone_from(&entries);
        }

        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        assert_eq!(encoded, [2, 37, 4, 0xac, 2, 0xff, 1]);

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        if type_id == VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID {
            assert_eq!(decoded.enchantments, entries);
        } else {
            assert_eq!(decoded.stored_enchantments, entries);
        }

        for invalid_entries in [
            vec![ItemEnchantmentSummary {
                holder_id: -1,
                level: 1,
            }],
            vec![
                ItemEnchantmentSummary {
                    holder_id: 1,
                    level: 1,
                },
                ItemEnchantmentSummary {
                    holder_id: 1,
                    level: 2,
                },
            ],
            vec![ItemEnchantmentSummary {
                holder_id: 1,
                level: -1,
            }],
            vec![ItemEnchantmentSummary {
                holder_id: 1,
                level: 256,
            }],
        ] {
            let mut invalid = DataComponentPatchSummary::default();
            if type_id == VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID {
                invalid.enchantments = invalid_entries;
            } else {
                invalid.stored_enchantments = invalid_entries;
            }
            let mut encoded = Encoder::new();
            assert!(descriptor.encode(&invalid, &mut encoded).is_err());
        }
    }

    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID).unwrap();
    let oversized = DataComponentPatchSummary {
        enchantments: (0..=MAX_DATA_COMPONENT_LIST_ITEMS)
            .map(|holder_id| ItemEnchantmentSummary {
                holder_id: holder_id as i32,
                level: 1,
            })
            .collect(),
        ..DataComponentPatchSummary::default()
    };
    assert!(matches!(
        descriptor.encode(&oversized, &mut Encoder::new()),
        Err(ProtocolError::PacketTooLarge(
            _,
            MAX_DATA_COMPONENT_LIST_ITEMS
        ))
    ));
}

#[test]
fn name_component_descriptors_round_trip_exact_component_trees() {
    let custom_nbt = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String("Custom".into()),
        },
        NbtSummaryEntry {
            name: "bold".into(),
            value: NbtSummaryValue::Byte(1),
        },
        NbtSummaryEntry {
            name: "color".into(),
            value: NbtSummaryValue::String("gold".into()),
        },
    ]);
    let cases = [
        (
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            DataComponentPatchSummary {
                custom_name: Some("Custom".to_string()),
                custom_name_styled: Some(vec![StyledTextRun {
                    text: "Custom".to_string(),
                    style: crate::ComponentStyle {
                        bold: Some(true),
                        color: Some(0xFF_AA_00),
                        ..Default::default()
                    },
                }]),
                custom_name_nbt: Some(custom_nbt),
                ..DataComponentPatchSummary::default()
            },
        ),
        (
            VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
            DataComponentPatchSummary {
                item_name: Some("Item".to_string()),
                item_name_styled: Some(vec![StyledTextRun {
                    text: "Item".to_string(),
                    style: crate::ComponentStyle::default(),
                }]),
                item_name_nbt: Some(NbtSummaryValue::String("Item".into())),
                ..DataComponentPatchSummary::default()
            },
        ),
    ];

    for (type_id, summary) in cases {
        let descriptor = vanilla_data_component_descriptor(type_id).unwrap();
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();
        let encoded = encoded.into_inner();
        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        match type_id {
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID => {
                assert_eq!(decoded.custom_name, summary.custom_name);
                assert_eq!(decoded.custom_name_styled, summary.custom_name_styled);
                assert_eq!(decoded.custom_name_nbt, summary.custom_name_nbt);
            }
            VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID => {
                assert_eq!(
                    encoded,
                    [8, 0, 4, b'I', b't', b'e', b'm'],
                    "item name must be one unwrapped ComponentSerialization root"
                );
                assert_eq!(decoded.item_name, summary.item_name);
                assert_eq!(decoded.item_name_styled, summary.item_name_styled);
                assert_eq!(decoded.item_name_nbt, summary.item_name_nbt);
            }
            _ => unreachable!(),
        }
    }
}
