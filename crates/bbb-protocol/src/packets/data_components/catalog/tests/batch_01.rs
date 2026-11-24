use super::*;

#[test]
fn handler_registry_binds_every_generated_metadata_entry_in_order() {
    let descriptors = vanilla_data_component_descriptors();
    assert_eq!(descriptors.len(), 110);
    assert_eq!(WIRE_DECODE_HANDLERS.len(), 110);
    assert_eq!(descriptors.len(), VANILLA_DATA_COMPONENT_TYPES.len());
    let mut type_ids = HashSet::new();
    let mut resource_ids = HashSet::new();
    for (expected_id, (descriptor, metadata)) in descriptors
        .iter()
        .zip(VANILLA_DATA_COMPONENT_TYPES)
        .enumerate()
    {
        assert_eq!(descriptor.type_id(), expected_id as i32);
        assert_eq!(descriptor.metadata, metadata);
        assert!(type_ids.insert(descriptor.type_id()));
        assert!(resource_ids.insert(descriptor.resource_id()));
    }
    assert_eq!(
        descriptors
            .iter()
            .filter(|descriptor| !descriptor.is_persistent())
            .map(|descriptor| descriptor.type_id())
            .collect::<Vec<_>>(),
        vec![20, 41, 48]
    );
}

#[test]
fn custom_model_data_descriptor_round_trips_all_stream_lists_bit_exactly() {
    let summary = DataComponentPatchSummary {
        custom_model_data_floats: vec![f32::from_bits(1), -0.0, f32::from_bits(0x7fc0_0001)].into(),
        custom_model_data_flags: vec![true, false],
        custom_model_data_strings: vec!["variant".into(), "😀".into()],
        custom_model_data_colors: vec![-1, 0x11_22_33],
        ..DataComponentPatchSummary::default()
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID)
            .unwrap();
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let mut expected = Encoder::new();
    expected.write_var_i32(3);
    expected.write_f32(f32::from_bits(1));
    expected.write_f32(-0.0);
    expected.write_f32(f32::from_bits(0x7fc0_0001));
    expected.write_var_i32(2);
    expected.write_bool(true);
    expected.write_bool(false);
    expected.write_var_i32(2);
    expected.write_string("variant");
    expected.write_string("😀");
    expected.write_var_i32(2);
    expected.write_i32(-1);
    expected.write_i32(0x11_22_33);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(
        decoded.custom_model_data_floats,
        summary.custom_model_data_floats
    );
    assert_eq!(
        decoded.custom_model_data_flags,
        summary.custom_model_data_flags
    );
    assert_eq!(
        decoded.custom_model_data_strings,
        summary.custom_model_data_strings
    );
    assert_eq!(
        decoded.custom_model_data_colors,
        summary.custom_model_data_colors
    );
}

#[test]
fn repair_cost_descriptor_round_trips_nonnegative_var_int_values() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID).unwrap();
    for value in [0, i32::MAX] {
        let summary = DataComponentPatchSummary {
            repair_cost: Some(value),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(value);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.repair_cost, Some(value));
    }
}

#[test]
fn repair_cost_descriptor_rejects_negative_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID).unwrap();
    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            repair_cost: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0x7f);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0x7f]);
    }

    for value in [-1, i32::MIN] {
        let mut encoded = Encoder::new();
        encoded.write_var_i32(value);
        let wire = encoded.into_inner();
        let mut decoded = DataComponentPatchSummary {
            repair_cost: Some(7),
            ..DataComponentPatchSummary::default()
        };
        assert!(descriptor
            .decode(&mut Decoder::new(&wire), &mut decoded)
            .is_err());
        assert_eq!(decoded.repair_cost, Some(7));
    }
}

#[test]
fn additional_trade_cost_descriptor_round_trips_signed_var_int_values() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID)
            .unwrap();
    for value in [i32::MIN, 0, i32::MAX] {
        let summary = DataComponentPatchSummary {
            additional_trade_cost: Some(value),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(value);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.additional_trade_cost, Some(value));
    }
}

#[test]
fn additional_trade_cost_descriptor_rejects_a_missing_value_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID)
            .unwrap();
    let mut encoded = Encoder::new();
    encoded.write_u8(0x7f);
    assert!(descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .is_err());
    assert_eq!(encoded.into_inner(), [0x7f]);
}

#[test]
fn creative_slot_lock_descriptor_round_trips_empty_unit_payload() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID)
            .unwrap();
    let summary = DataComponentPatchSummary {
        creative_slot_lock: true,
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let wire = encoded.into_inner();
    assert!(wire.is_empty());
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert!(decoded.creative_slot_lock);
}

#[test]
fn creative_slot_lock_descriptor_rejects_absent_unit_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID)
            .unwrap();
    let mut encoded = Encoder::new();
    encoded.write_u8(0xaa);
    let error = descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("missing creative_slot_lock component value"));
    assert_eq!(encoded.into_inner(), [0xaa]);
}

#[test]
fn debug_stick_state_descriptor_round_trips_validated_block_property_maps() {
    let oak_log = vanilla_block_type_for_resource_id("minecraft:oak_log").unwrap();
    let lever = vanilla_block_type_for_resource_id("minecraft:lever").unwrap();
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID)
            .unwrap();
    let summary = DataComponentPatchSummary {
        debug_stick_properties: vec![
            DebugStickPropertySummary {
                block_registry_id: oak_log.id,
                property: "axis".to_string(),
            },
            DebugStickPropertySummary {
                block_registry_id: lever.id,
                property: "powered".to_string(),
            },
        ],
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
        decoded.debug_stick_properties,
        summary.debug_stick_properties
    );

    let mut empty = Encoder::new();
    descriptor
        .encode(&DataComponentPatchSummary::default(), &mut empty)
        .unwrap();
    assert_eq!(empty.into_inner(), [10, 0]);
}

#[test]
fn debug_stick_state_descriptor_rejects_invalid_maps_atomically() {
    let oak_log = vanilla_block_type_for_resource_id("minecraft:oak_log")
        .unwrap()
        .id;
    let stone = vanilla_block_type_for_resource_id("minecraft:stone")
        .unwrap()
        .id;
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID)
            .unwrap();
    for properties in [
        vec![DebugStickPropertySummary {
            block_registry_id: i32::MAX,
            property: "axis".to_string(),
        }],
        vec![DebugStickPropertySummary {
            block_registry_id: stone,
            property: "axis".to_string(),
        }],
        vec![
            DebugStickPropertySummary {
                block_registry_id: oak_log,
                property: "axis".to_string(),
            },
            DebugStickPropertySummary {
                block_registry_id: oak_log,
                property: "axis".to_string(),
            },
        ],
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor
            .encode(
                &DataComponentPatchSummary {
                    debug_stick_properties: properties,
                    ..DataComponentPatchSummary::default()
                },
                &mut encoded,
            )
            .is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    for invalid in [
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "minecraft:missing".into(),
            value: NbtSummaryValue::String("axis".into()),
        }]),
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "minecraft:stone".into(),
            value: NbtSummaryValue::String("axis".into()),
        }]),
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "oak_log".into(),
                value: NbtSummaryValue::String("axis".into()),
            },
            NbtSummaryEntry {
                name: "minecraft:oak_log".into(),
                value: NbtSummaryValue::String("axis".into()),
            },
        ]),
    ] {
        let wire =
            encode_nbt_summary_bounded(Some(&invalid), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        assert!(descriptor
            .decode(
                &mut Decoder::new(&wire),
                &mut DataComponentPatchSummary::default(),
            )
            .is_err());
    }
}

#[test]
fn lock_descriptor_round_trips_persistent_item_predicates() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LOCK_TYPE_ID).unwrap();
    let summary = DataComponentPatchSummary {
        lock_predicate: Some(PersistentItemPredicateSummary {
            items: Some(HolderSetSummary {
                registry_ids: vec![crate::item_types::VANILLA_ITEM_STONE_ID],
                tag: None,
            }),
            count_min: Some(1),
            count_max: Some(64),
            exact_components: vec![PersistentDataComponentValueSummary {
                component_type_id: VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                value: NbtSummaryValue::Int(16),
            }],
            partial_components: vec![PartialDataComponentPredicateSummary {
                predicate_type: DataComponentPredicateTypeSummary::AnyComponent(
                    VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
                ),
                wire_value: Some(NbtSummaryValue::Compound(Vec::new())),
            }],
        }),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.lock_predicate, summary.lock_predicate);

    let mut named = summary;
    named
        .lock_predicate
        .as_mut()
        .unwrap()
        .items
        .as_mut()
        .unwrap()
        .tag = Some("minecraft:tools".to_string());
    let mut encoded = Encoder::new();
    descriptor.encode(&named, &mut encoded).unwrap();
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    let items = decoded.lock_predicate.unwrap().items.unwrap();
    assert_eq!(items.tag.as_deref(), Some("minecraft:tools"));
    assert!(items.registry_ids.is_empty());

    let mut empty = Encoder::new();
    descriptor
        .encode(
            &DataComponentPatchSummary {
                lock_predicate: Some(PersistentItemPredicateSummary::default()),
                ..DataComponentPatchSummary::default()
            },
            &mut empty,
        )
        .unwrap();
    assert_eq!(empty.into_inner(), [10, 0]);
}

#[test]
fn lock_descriptor_rejects_invalid_predicates_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_LOCK_TYPE_ID).unwrap();
    for predicate in [
        None,
        Some(PersistentItemPredicateSummary {
            count_min: Some(2),
            count_max: Some(1),
            ..PersistentItemPredicateSummary::default()
        }),
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor
            .encode(
                &DataComponentPatchSummary {
                    lock_predicate: predicate,
                    ..DataComponentPatchSummary::default()
                },
                &mut encoded,
            )
            .is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    for invalid in [
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "items".into(),
            value: NbtSummaryValue::String("minecraft:missing".into()),
        }]),
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "predicates".into(),
            value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "minecraft:lock".into(),
                value: NbtSummaryValue::Int(1),
            }]),
        }]),
    ] {
        let wire =
            encode_nbt_summary_bounded(Some(&invalid), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
        assert!(descriptor
            .decode(
                &mut Decoder::new(&wire),
                &mut DataComponentPatchSummary::default(),
            )
            .is_err());
    }
}

#[test]
fn damage_resistant_descriptor_round_trips_direct_and_named_holder_sets_and_rejects_invalid_values()
{
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID).unwrap();

    for summary in [
        DataComponentPatchSummary {
            damage_resistant_type_registry_ids: vec![2, 4],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            damage_resistant_type_tag: Some("minecraft:is_fire".to_string()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            damage_resistant_type_registry_ids: vec![2; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
            damage_resistant_type_tag: Some("minecraft:is_fire".to_string()),
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
        if summary.damage_resistant_type_tag.is_some() {
            assert!(decoded.damage_resistant_type_registry_ids.is_empty());
        } else {
            assert_eq!(
                decoded.damage_resistant_type_registry_ids,
                summary.damage_resistant_type_registry_ids
            );
        }
        assert_eq!(
            decoded.damage_resistant_type_tag,
            summary.damage_resistant_type_tag
        );
    }

    for summary in [
        DataComponentPatchSummary {
            damage_resistant_type_registry_ids: vec![-1],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            damage_resistant_type_tag: Some("is_fire".to_string()),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn repairable_descriptor_round_trips_direct_and_named_holder_sets_and_rejects_invalid_values() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID).unwrap();

    for summary in [
        DataComponentPatchSummary {
            repairable_item_registry_ids: vec![1, 193],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            repairable_item_tag: Some("minecraft:repairs_leather_armor".to_string()),
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
            decoded.repairable_item_registry_ids,
            summary.repairable_item_registry_ids
        );
        assert_eq!(decoded.repairable_item_tag, summary.repairable_item_tag);
    }

    for summary in [
        DataComponentPatchSummary {
            repairable_item_registry_ids: vec![-1],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            repairable_item_registry_ids: vec![i32::MAX],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            repairable_item_tag: Some("repairs_leather_armor".to_string()),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn weapon_descriptor_round_trips_wire_fields_and_rejects_invalid_persistent_values() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID).unwrap();
    let summary = DataComponentPatchSummary {
        weapon_item_damage_per_attack: Some(3),
        weapon_disable_blocking_seconds_bits: Some(5.0_f32.to_bits()),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let mut expected = Encoder::new();
    expected.write_var_i32(3);
    expected.write_f32(5.0);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.weapon_item_damage_per_attack, Some(3));
    assert_eq!(
        decoded.weapon_disable_blocking_seconds_bits,
        Some(5.0_f32.to_bits())
    );

    for summary in [
        DataComponentPatchSummary {
            weapon_disable_blocking_seconds_bits: Some(0.0_f32.to_bits()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            weapon_item_damage_per_attack: Some(1),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            weapon_item_damage_per_attack: Some(-1),
            weapon_disable_blocking_seconds_bits: Some(0.0_f32.to_bits()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            weapon_item_damage_per_attack: Some(1),
            weapon_disable_blocking_seconds_bits: Some((-0.0_f32).to_bits()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            weapon_item_damage_per_attack: Some(1),
            weapon_disable_blocking_seconds_bits: Some(f32::NAN.to_bits()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            weapon_item_damage_per_attack: Some(1),
            weapon_disable_blocking_seconds_bits: Some(f32::INFINITY.to_bits()),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn container_loot_descriptor_canonicalizes_codec_nbt_and_rejects_invalid_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID).unwrap();
    for summary in [
        DataComponentPatchSummary {
            container_loot: true,
            container_loot_table: Some("minecraft:empty".to_string()),
            container_loot_seed: Some(0),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            container_loot: true,
            container_loot_table: Some("minecraft:chests/simple_dungeon".to_string()),
            container_loot_seed: Some(12),
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
        assert!(decoded.container_loot);
        assert_eq!(decoded.container_loot_table, summary.container_loot_table);
        assert_eq!(decoded.container_loot_seed, summary.container_loot_seed);
        assert_eq!(
            decoded
                .codec_backed_nbt
                .get(&VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID),
            Some(&canonical_container_loot_nbt(
                summary.container_loot_table.as_deref().unwrap(),
                summary.container_loot_seed.unwrap()
            ))
        );
    }

    let shorthand = canonical_container_loot_nbt("empty", 0);
    let wire =
        encode_nbt_summary_bounded(Some(&shorthand), i32::MAX as usize, MAX_NBT_DEPTH).unwrap();
    let mut decoded = DataComponentPatchSummary::default();
    descriptor
        .decode(&mut Decoder::new(&wire), &mut decoded)
        .unwrap();
    assert_eq!(
        decoded.container_loot_table.as_deref(),
        Some("minecraft:empty")
    );
    assert_eq!(
        decoded
            .codec_backed_nbt
            .get(&VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID),
        Some(&canonical_container_loot_nbt("minecraft:empty", 0))
    );

    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            container_loot: true,
            container_loot_seed: Some(0),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            container_loot: true,
            container_loot_table: Some("empty".to_string()),
            container_loot_seed: Some(0),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            container_loot: true,
            container_loot_table: Some("minecraft:empty".to_string()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            container_loot: true,
            container_loot_table: Some("minecraft:empty".to_string()),
            container_loot_seed: Some(0),
            codec_backed_nbt: BTreeMap::from([(
                VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
                canonical_container_loot_nbt("minecraft:chests/simple_dungeon", 12),
            )]),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0xaa);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0xaa]);
    }

    for malformed in [
        NbtSummaryValue::String("minecraft:empty".into()),
        NbtSummaryValue::Compound(Vec::new()),
        NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "loot_table".into(),
            value: NbtSummaryValue::Int(1),
        }]),
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "loot_table".into(),
                value: NbtSummaryValue::String("minecraft:empty".into()),
            },
            NbtSummaryEntry {
                name: "seed".into(),
                value: NbtSummaryValue::String("bad".into()),
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
fn custom_model_data_descriptor_enforces_collection_and_utf16_bounds() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CUSTOM_MODEL_DATA_TYPE_ID)
            .unwrap();
    let oversized_string = "😀".repeat(MAX_STRING_CHARS / 2 + 1);
    let cases = [
        DataComponentPatchSummary {
            custom_model_data_floats: vec![0.0; MAX_DATA_COMPONENT_LIST_ITEMS + 1].into(),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            custom_model_data_flags: vec![false; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            custom_model_data_strings: vec!["".into(); MAX_DATA_COMPONENT_LIST_ITEMS + 1],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            custom_model_data_colors: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            custom_model_data_strings: vec![oversized_string.clone().into()],
            ..DataComponentPatchSummary::default()
        },
    ];
    for summary in cases {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }

    let maximal_string = format!("{}a", "😀".repeat((MAX_STRING_CHARS.saturating_sub(1)) / 2));
    assert_eq!(maximal_string.encode_utf16().count(), MAX_STRING_CHARS);
    let maximal = DataComponentPatchSummary {
        custom_model_data_floats: vec![0.0; MAX_DATA_COMPONENT_LIST_ITEMS].into(),
        custom_model_data_strings: vec![maximal_string.into()],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&maximal, &mut encoded).unwrap();
    let wire = encoded.into_inner();
    let mut decoded = DataComponentPatchSummary::default();
    descriptor
        .decode(&mut Decoder::new(&wire), &mut decoded)
        .unwrap();
    assert_eq!(
        decoded.custom_model_data_floats.len(),
        MAX_DATA_COMPONENT_LIST_ITEMS
    );
    assert_eq!(
        decoded.custom_model_data_strings[0].units().len(),
        MAX_STRING_CHARS
    );

    let mut invalid_wire = Encoder::new();
    invalid_wire.write_var_i32(0);
    invalid_wire.write_var_i32(0);
    invalid_wire.write_var_i32(1);
    invalid_wire.write_string(&oversized_string);
    invalid_wire.write_var_i32(0);
    let wire = invalid_wire.into_inner();
    let error = descriptor
        .decode(
            &mut Decoder::new(&wire),
            &mut DataComponentPatchSummary::default(),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::StringTooLong {
            actual,
            max: MAX_STRING_CHARS
        } if actual == MAX_STRING_CHARS + 1
    ));
}

#[test]
fn tooltip_display_descriptor_round_trips_order_and_collapses_wire_duplicates() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID).unwrap();
    let summary = DataComponentPatchSummary {
        tooltip_hide_tooltip: true,
        tooltip_hidden_component_type_ids: vec![11, 20, 109],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    assert_eq!(encoded.into_inner(), [1, 3, 11, 20, 109]);

    let mut wire = Encoder::new();
    wire.write_bool(false);
    wire.write_var_i32(5);
    for type_id in [13, 11, 13, 20, 11] {
        wire.write_var_i32(type_id);
    }
    let wire = wire.into_inner();
    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&wire);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert!(!decoded.tooltip_hide_tooltip);
    assert_eq!(decoded.tooltip_hidden_component_type_ids, [13, 11, 20]);
}

#[test]
fn tooltip_display_descriptor_rejects_noncanonical_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOLTIP_DISPLAY_TYPE_ID).unwrap();
    for hidden_components in [
        vec![-1],
        vec![i32::try_from(VANILLA_DATA_COMPONENT_TYPES.len()).unwrap()],
        vec![11, 11],
        vec![11; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
    ] {
        let summary = DataComponentPatchSummary {
            tooltip_hidden_component_type_ids: hidden_components,
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        encoded.write_u8(0x7f);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0x7f]);
    }

    for invalid_type_id in [
        -1,
        i32::try_from(VANILLA_DATA_COMPONENT_TYPES.len()).unwrap(),
    ] {
        let mut wire = Encoder::new();
        wire.write_bool(true);
        wire.write_var_i32(2);
        wire.write_var_i32(11);
        wire.write_var_i32(invalid_type_id);
        let wire = wire.into_inner();
        let mut decoded = DataComponentPatchSummary::default();
        assert!(descriptor
            .decode(&mut Decoder::new(&wire), &mut decoded)
            .is_err());
        assert_eq!(decoded, DataComponentPatchSummary::default());
    }

    let mut oversized = Encoder::new();
    oversized.write_bool(false);
    oversized.write_var_i32((MAX_DATA_COMPONENT_LIST_ITEMS + 1) as i32);
    let wire = oversized.into_inner();
    assert!(matches!(
        descriptor.decode(
            &mut Decoder::new(&wire),
            &mut DataComponentPatchSummary::default()
        ),
        Err(ProtocolError::PacketTooLarge(
            _,
            MAX_DATA_COMPONENT_LIST_ITEMS
        ))
    ));
}

#[test]
fn tool_descriptor_round_trips_rules_holders_and_raw_defaults_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOL_TYPE_ID).unwrap();
    let summary = DataComponentPatchSummary {
        tool: Some(ToolSummary {
            rules: vec![
                ToolRuleSummary {
                    block_registry_ids: Vec::new(),
                    block_tag: Some("minecraft:mineable/pickaxe".to_string()),
                    speed_bits: Some(8.0_f32.to_bits()),
                    correct_for_drops: Some(true),
                },
                ToolRuleSummary {
                    block_registry_ids: vec![1, 193],
                    block_tag: None,
                    speed_bits: None,
                    correct_for_drops: Some(false),
                },
            ],
            default_mining_speed_bits: (-0.0_f32).to_bits(),
            damage_per_block: 2,
            can_destroy_blocks_in_creative: false,
        }),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let mut expected = Encoder::new();
    expected.write_var_i32(2);
    expected.write_var_i32(0);
    expected.write_string("minecraft:mineable/pickaxe");
    expected.write_bool(true);
    expected.write_f32(8.0);
    expected.write_bool(true);
    expected.write_bool(true);
    expected.write_var_i32(3);
    expected.write_var_i32(1);
    expected.write_var_i32(193);
    expected.write_bool(false);
    expected.write_bool(true);
    expected.write_bool(false);
    expected.write_f32(-0.0);
    expected.write_var_i32(2);
    expected.write_bool(false);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let wire = encoded.into_inner();
    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&wire);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.tool, summary.tool);
}

#[test]
fn tool_descriptor_rejects_persistent_range_and_holder_errors() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_TOOL_TYPE_ID).unwrap();
    let valid = ToolSummary {
        rules: vec![ToolRuleSummary {
            block_registry_ids: vec![1],
            block_tag: None,
            speed_bits: Some(1.0_f32.to_bits()),
            correct_for_drops: None,
        }],
        default_mining_speed_bits: 1.0_f32.to_bits(),
        damage_per_block: 1,
        can_destroy_blocks_in_creative: true,
    };
    let invalid_holder_id = i32::try_from(crate::block_types::VANILLA_BLOCK_TYPES.len()).unwrap();
    let cases = [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            tool: Some(ToolSummary {
                rules: vec![valid.rules[0].clone(); MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                ..valid.clone()
            }),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            tool: Some(ToolSummary {
                damage_per_block: -1,
                ..valid.clone()
            }),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            tool: Some(ToolSummary {
                rules: vec![ToolRuleSummary {
                    speed_bits: Some(0.0_f32.to_bits()),
                    ..valid.rules[0].clone()
                }],
                ..valid.clone()
            }),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            tool: Some(ToolSummary {
                rules: vec![ToolRuleSummary {
                    speed_bits: Some(f32::INFINITY.to_bits()),
                    ..valid.rules[0].clone()
                }],
                ..valid.clone()
            }),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            tool: Some(ToolSummary {
                rules: vec![ToolRuleSummary {
                    block_registry_ids: vec![invalid_holder_id],
                    ..valid.rules[0].clone()
                }],
                ..valid.clone()
            }),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            tool: Some(ToolSummary {
                rules: vec![ToolRuleSummary {
                    block_registry_ids: Vec::new(),
                    block_tag: Some("mineable/pickaxe".to_string()),
                    ..valid.rules[0].clone()
                }],
                ..valid.clone()
            }),
            ..DataComponentPatchSummary::default()
        },
    ];
    for summary in cases {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn attack_range_descriptor_round_trips_six_raw_floats_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID).unwrap();
    let range = AttackRangeSummary {
        min_reach: 0.0,
        max_reach: 64.0,
        min_creative_reach: f32::from_bits(0x3f80_0001),
        max_creative_reach: 6.5,
        hitbox_margin: 0.125,
        mob_factor: 2.0,
    };
    let summary = DataComponentPatchSummary {
        attack_range: Some(range),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let mut expected = Encoder::new();
    expected.write_f32(range.min_reach);
    expected.write_f32(range.max_reach);
    expected.write_f32(range.min_creative_reach);
    expected.write_f32(range.max_creative_reach);
    expected.write_f32(range.hitbox_margin);
    expected.write_f32(range.mob_factor);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let wire = encoded.into_inner();
    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&wire);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.attack_range, Some(range));
}

#[test]
fn attack_range_descriptor_rejects_persistent_ranges_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID).unwrap();
    let valid = AttackRangeSummary {
        min_reach: 0.0,
        max_reach: 3.0,
        min_creative_reach: 0.0,
        max_creative_reach: 5.0,
        hitbox_margin: 0.3,
        mob_factor: 1.0,
    };
    let cases = [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            attack_range: Some(AttackRangeSummary {
                min_reach: -0.0,
                ..valid
            }),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            attack_range: Some(AttackRangeSummary {
                min_reach: -0.01,
                ..valid
            }),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            attack_range: Some(AttackRangeSummary {
                max_reach: 64.01,
                ..valid
            }),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            attack_range: Some(AttackRangeSummary {
                min_creative_reach: f32::NAN,
                ..valid
            }),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            attack_range: Some(AttackRangeSummary {
                max_creative_reach: f32::INFINITY,
                ..valid
            }),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            attack_range: Some(AttackRangeSummary {
                hitbox_margin: 1.01,
                ..valid
            }),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            attack_range: Some(AttackRangeSummary {
                mob_factor: 2.01,
                ..valid
            }),
            ..DataComponentPatchSummary::default()
        },
    ];
    for summary in cases {
        let mut encoded = Encoder::new();
        encoded.write_u8(0x7f);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0x7f]);
    }
}

#[test]
fn enchantable_descriptor_round_trips_positive_var_int_values() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID).unwrap();
    for value in [1, i32::MAX] {
        let summary = DataComponentPatchSummary {
            enchantable_value: Some(value),
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(value);
        assert_eq!(encoded.clone().into_inner(), expected.into_inner());

        let wire = encoded.into_inner();
        let mut decoder = Decoder::new(&wire);
        let mut decoded = DataComponentPatchSummary::default();
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.enchantable_value, Some(value));
    }
}

#[test]
fn enchantable_descriptor_rejects_nonpositive_values_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID).unwrap();
    for summary in [
        DataComponentPatchSummary::default(),
        DataComponentPatchSummary {
            enchantable_value: Some(0),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            enchantable_value: Some(-1),
            ..DataComponentPatchSummary::default()
        },
    ] {
        let mut encoded = Encoder::new();
        encoded.write_u8(0x7f);
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
        assert_eq!(encoded.into_inner(), [0x7f]);
    }

    for value in [0, -1] {
        let mut encoded = Encoder::new();
        encoded.write_var_i32(value);
        let wire = encoded.into_inner();
        let mut decoded = DataComponentPatchSummary {
            enchantable_value: Some(7),
            ..DataComponentPatchSummary::default()
        };
        assert!(descriptor
            .decode(&mut Decoder::new(&wire), &mut decoded)
            .is_err());
        assert_eq!(decoded.enchantable_value, Some(7));
    }
}

#[test]
fn equippable_descriptor_round_trips_options_holders_and_flags_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID).unwrap();
    let summary = DataComponentPatchSummary {
        equippable: Some(EquippableSummary {
            slot_id: 4,
            equip_sound: SoundEventSummary {
                registry_id: Some(0),
                sound_id: None,
                fixed_range_bits: None,
            },
            asset_id: Some("minecraft:diamond".to_string()),
            camera_overlay: Some("minecraft:misc/pumpkinblur".to_string()),
            allowed_entities: Some(HolderSetSummary {
                registry_ids: vec![VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID],
                tag: None,
            }),
            dispensable: true,
            swappable: false,
            damage_on_hurt: true,
            equip_on_interact: false,
            can_be_sheared: true,
            shearing_sound: SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:item.shear".to_string()),
                fixed_range_bits: Some(7.5_f32.to_bits()),
            },
        }),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let mut expected = Encoder::new();
    expected.write_var_i32(4);
    expected.write_var_i32(1);
    expected.write_bool(true);
    expected.write_string("minecraft:diamond");
    expected.write_bool(true);
    expected.write_string("minecraft:misc/pumpkinblur");
    expected.write_bool(true);
    expected.write_var_i32(3);
    expected.write_var_i32(VANILLA_ENTITY_TYPE_BEE_ID);
    expected.write_var_i32(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    expected.write_bool(true);
    expected.write_bool(false);
    expected.write_bool(true);
    expected.write_bool(false);
    expected.write_bool(true);
    expected.write_var_i32(0);
    expected.write_string("example:item.shear");
    expected.write_bool(true);
    expected.write_f32(7.5);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let wire = encoded.into_inner();
    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&wire);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, summary);

    let tagged = DataComponentPatchSummary {
        equippable: Some(EquippableSummary {
            slot_id: 7,
            equip_sound: SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:item.equip".to_string()),
                fixed_range_bits: None,
            },
            asset_id: None,
            camera_overlay: None,
            allowed_entities: Some(HolderSetSummary {
                registry_ids: Vec::new(),
                tag: Some("minecraft:can_equip_saddle".to_string()),
            }),
            dispensable: false,
            swappable: true,
            damage_on_hurt: false,
            equip_on_interact: true,
            can_be_sheared: false,
            shearing_sound: SoundEventSummary {
                registry_id: Some(1),
                sound_id: None,
                fixed_range_bits: None,
            },
        }),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&tagged, &mut encoded).unwrap();
    let wire = encoded.into_inner();
    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&wire);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, tagged);
}

#[test]
fn equippable_descriptor_rejects_noncanonical_projections() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID).unwrap();
    let valid = EquippableSummary {
        slot_id: 4,
        equip_sound: SoundEventSummary {
            registry_id: Some(0),
            sound_id: None,
            fixed_range_bits: None,
        },
        asset_id: Some("minecraft:diamond".to_string()),
        camera_overlay: Some("minecraft:misc/pumpkinblur".to_string()),
        allowed_entities: Some(HolderSetSummary {
            registry_ids: vec![VANILLA_ENTITY_TYPE_BEE_ID],
            tag: None,
        }),
        dispensable: true,
        swappable: true,
        damage_on_hurt: true,
        equip_on_interact: false,
        can_be_sheared: false,
        shearing_sound: SoundEventSummary {
            registry_id: Some(1),
            sound_id: None,
            fixed_range_bits: None,
        },
    };
    let mut cases = vec![DataComponentPatchSummary::default()];
    for equippable in [
        EquippableSummary {
            slot_id: 8,
            ..valid.clone()
        },
        EquippableSummary {
            asset_id: Some("Example:diamond".to_string()),
            ..valid.clone()
        },
        EquippableSummary {
            camera_overlay: Some("bad path".to_string()),
            ..valid.clone()
        },
        EquippableSummary {
            allowed_entities: Some(HolderSetSummary {
                registry_ids: vec![-1],
                tag: None,
            }),
            ..valid.clone()
        },
        EquippableSummary {
            equip_sound: SoundEventSummary {
                registry_id: Some(0),
                sound_id: Some("example:mixed".to_string()),
                fixed_range_bits: None,
            },
            ..valid.clone()
        },
        EquippableSummary {
            shearing_sound: SoundEventSummary {
                registry_id: None,
                sound_id: None,
                fixed_range_bits: None,
            },
            ..valid
        },
    ] {
        cases.push(DataComponentPatchSummary {
            equippable: Some(equippable),
            ..DataComponentPatchSummary::default()
        });
    }
    for summary in cases {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
}

#[test]
fn use_cooldown_descriptor_round_trips_raw_seconds_and_optional_group_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID).unwrap();
    let ungrouped = DataComponentPatchSummary {
        use_cooldown_seconds_bits: Some(1.25_f32.to_bits()),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&ungrouped, &mut encoded).unwrap();
    let mut expected = Encoder::new();
    expected.write_f32(1.25);
    expected.write_bool(false);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, ungrouped);

    let grouped = DataComponentPatchSummary {
        use_cooldown_seconds_bits: Some(2.5_f32.to_bits()),
        use_cooldown_group: Some("minecraft:ender_pearl".to_string()),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    descriptor.encode(&grouped, &mut encoded).unwrap();
    let mut expected = Encoder::new();
    expected.write_f32(2.5);
    expected.write_bool(true);
    expected.write_string("minecraft:ender_pearl");
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, grouped);

    for (seconds, ticks) in [(f32::from_bits(1), 0), (f32::MAX, i32::MAX)] {
        let boundary = DataComponentPatchSummary {
            use_cooldown_seconds_bits: Some(seconds.to_bits()),
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
        assert_eq!(decoded.use_cooldown_ticks(), Some(ticks));
    }
}

#[test]
fn use_cooldown_descriptor_rejects_noncanonical_projections() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID).unwrap();
    let valid = DataComponentPatchSummary {
        use_cooldown_seconds_bits: Some(1.25_f32.to_bits()),
        ..DataComponentPatchSummary::default()
    };
    let mut cases = vec![DataComponentPatchSummary::default()];
    for seconds in [0.0, -0.0, -1.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        cases.push(DataComponentPatchSummary {
            use_cooldown_seconds_bits: Some(seconds.to_bits()),
            ..DataComponentPatchSummary::default()
        });
    }
    for group in [
        "ender_pearl".to_string(),
        "Bad:group".to_string(),
        format!("minecraft:{}", "a".repeat(MAX_STRING_CHARS + 1)),
    ] {
        cases.push(DataComponentPatchSummary {
            use_cooldown_group: Some(group),
            ..valid.clone()
        });
    }

    for summary in cases {
        let mut encoded = Encoder::new();
        assert!(descriptor.encode(&summary, &mut encoded).is_err());
    }
    descriptor.encode(&valid, &mut Encoder::new()).unwrap();
}

#[test]
fn adventure_mode_predicate_descriptor_round_trips_all_stream_fields() {
    let value = AdventureModePredicateDataSummary {
        predicates: vec![
            BlockPredicateSummary {
                blocks: Some(HolderSetSummary {
                    registry_ids: vec![1],
                    tag: None,
                }),
                state: Some(vec![
                    StatePropertyMatcherSummary {
                        name: "facing".into(),
                        value: StatePropertyValueMatcherSummary::Exact("north".into()),
                    },
                    StatePropertyMatcherSummary {
                        name: "age".into(),
                        value: StatePropertyValueMatcherSummary::Range {
                            min: Some("1".into()),
                            max: Some("3".into()),
                        },
                    },
                ]),
                nbt: Some(NbtSummaryValue::Compound(Vec::new())),
                exact_components: Some(Box::new(DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                    max_stack_size: Some(64),
                    ..DataComponentPatchSummary::default()
                })),
                partial_components: vec![
                    PartialDataComponentPredicateSummary {
                        predicate_type: DataComponentPredicateTypeSummary::AnyComponent(
                            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                        ),
                        wire_value: Some(NbtSummaryValue::Compound(Vec::new())),
                    },
                    PartialDataComponentPredicateSummary {
                        predicate_type: DataComponentPredicateTypeSummary::Concrete(0),
                        wire_value: Some(NbtSummaryValue::Compound(Vec::new())),
                    },
                ],
            },
            BlockPredicateSummary {
                blocks: Some(HolderSetSummary {
                    registry_ids: Vec::new(),
                    tag: Some("minecraft:mineable/pickaxe".to_string()),
                }),
                ..BlockPredicateSummary::default()
            },
        ],
    };
    let tooltip = AdventureModePredicateSummary {
        unknown: false,
        block_registry_ids: vec![1],
        unresolved_block_tag_count: 1,
    };
    let summary = DataComponentPatchSummary {
        can_place_on_persistent: Some(value.clone()),
        ..DataComponentPatchSummary::default()
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();

    let mut expected = Encoder::new();
    expected.write_var_i32(2);
    expected.write_bool(true);
    expected.write_var_i32(2);
    expected.write_var_i32(1);
    expected.write_bool(true);
    expected.write_var_i32(2);
    expected.write_string("facing");
    expected.write_bool(true);
    expected.write_string("north");
    expected.write_string("age");
    expected.write_bool(false);
    expected.write_bool(true);
    expected.write_string("1");
    expected.write_bool(true);
    expected.write_string("3");
    expected.write_bool(true);
    expected.write_bytes(&[10, 0]);
    expected.write_var_i32(1);
    expected.write_var_i32(VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID);
    expected.write_var_i32(64);
    expected.write_var_i32(2);
    expected.write_bool(false);
    expected.write_var_i32(VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID);
    expected.write_bytes(&[10, 0]);
    expected.write_bool(true);
    expected.write_var_i32(0);
    expected.write_bytes(&[10, 0]);
    expected.write_bool(true);
    expected.write_var_i32(0);
    expected.write_string("minecraft:mineable/pickaxe");
    expected.write_bool(false);
    expected.write_bool(false);
    expected.write_var_i32(0);
    expected.write_var_i32(0);
    assert_eq!(encoded.clone().into_inner(), expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let wire = encoded.into_inner();
    let mut decoder = Decoder::new(&wire);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    let mut report = serde_json::to_value(&decoded).unwrap();
    assert_eq!(
        report["can_place_on"],
        serde_json::to_value(&tooltip).unwrap()
    );
    report["can_place_on"] = serde_json::Value::Null;
    assert_eq!(
        serde_json::from_value::<DataComponentPatchSummary>(report).unwrap(),
        decoded
    );
    assert_eq!(
        decoded
            .can_place_on_persistent
            .as_ref()
            .map(adventure_mode_predicate_tooltip_summary),
        Some(tooltip)
    );
    assert_eq!(decoded.can_place_on_persistent, Some(value));
}

#[test]
fn adventure_mode_predicate_descriptor_rejects_invalid_values() {
    let base_value = AdventureModePredicateDataSummary {
        predicates: vec![BlockPredicateSummary {
            blocks: Some(HolderSetSummary {
                registry_ids: vec![1],
                tag: None,
            }),
            ..BlockPredicateSummary::default()
        }],
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID).unwrap();
    let mut cases = Vec::new();

    let mut scalar_nbt = base_value.clone();
    scalar_nbt.predicates[0].nbt = Some(NbtSummaryValue::Int(1));
    cases.push((
        DataComponentPatchSummary {
            can_break_persistent: Some(scalar_nbt),
            ..DataComponentPatchSummary::default()
        },
        "nbt must be a compound tag",
    ));

    let mut missing_partial_value = base_value.clone();
    missing_partial_value.predicates[0].partial_components =
        vec![PartialDataComponentPredicateSummary {
            predicate_type: DataComponentPredicateTypeSummary::Concrete(0),
            wire_value: None,
        }];
    cases.push((
        DataComponentPatchSummary {
            can_break_persistent: Some(missing_partial_value),
            ..DataComponentPatchSummary::default()
        },
        "missing non-null nbt",
    ));

    for (summary, expected_error) in cases {
        let mut encoded = Encoder::new();
        let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error}");
    }
}

#[test]
fn adventure_mode_predicate_decode_and_encode_bound_recursive_exact_components() {
    let mut wire = Encoder::new();
    write_nested_adventure_mode_predicate(
        &mut wire,
        MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH + 1,
    );
    let wire = wire.into_inner();
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID).unwrap();
    let error = descriptor
        .decode(
            &mut Decoder::new(&wire),
            &mut DataComponentPatchSummary::default(),
        )
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("adventure mode predicate nesting exceeds"),
        "{error}"
    );

    let nested =
        nested_adventure_mode_predicate_summary(MAX_ADVENTURE_MODE_PREDICATE_NESTING_DEPTH + 1);
    let summary = DataComponentPatchSummary {
        can_place_on_persistent: Some(nested),
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    encoded.write_u8(0xaa);
    let error = descriptor.encode(&summary, &mut encoded).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("adventure mode predicate nesting exceeds"),
        "{error}"
    );
    assert_eq!(encoded.into_inner(), [0xaa]);
}

#[test]
fn attribute_modifiers_descriptor_round_trips_exact_display_variants() {
    let display_text_nbt = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::String("Override".into()),
        },
        NbtSummaryEntry {
            name: "bold".into(),
            value: NbtSummaryValue::Byte(1),
        },
    ]);
    let display_text_styled = vec![StyledTextRun {
        text: "Override".to_string(),
        style: crate::ComponentStyle {
            bold: Some(true),
            ..crate::ComponentStyle::default()
        },
    }];
    let summary = DataComponentPatchSummary {
        attribute_modifiers: vec![
            AttributeModifierSummary {
                attribute_id: crate::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
                modifier_id: "minecraft:test/default".to_string(),
                amount_bits: (-0.0f64).to_bits(),
                operation_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
                slot_id: VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
                display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
                display_text: None,
                display_text_styled: None,
                display_text_nbt: None,
            },
            AttributeModifierSummary {
                attribute_id:
                    crate::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
                modifier_id: "minecraft:test/hidden".to_string(),
                amount_bits: f64::INFINITY.to_bits(),
                operation_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
                slot_id:
                    crate::equipment_slots::VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
                display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
                display_text: None,
                display_text_styled: None,
                display_text_nbt: None,
            },
            AttributeModifierSummary {
                attribute_id: crate::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                modifier_id: "example:override".to_string(),
                amount_bits: 1.5f64.to_bits(),
                operation_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
                slot_id: 7,
                display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                display_text: Some("Override".to_string()),
                display_text_styled: Some(display_text_styled),
                display_text_nbt: Some(display_text_nbt.clone()),
            },
        ],
        ..DataComponentPatchSummary::default()
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID)
            .unwrap();
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    let encoded = encoded.into_inner();

    let mut expected = Encoder::new();
    expected.write_var_i32(3);
    for (attribute, id, amount, operation, slot, display) in [
        (
            crate::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
            "minecraft:test/default",
            -0.0,
            crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
            VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
            crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
        ),
        (
            crate::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
            "minecraft:test/hidden",
            f64::INFINITY,
            crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
            crate::equipment_slots::VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
            crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_HIDDEN_ID,
        ),
        (
            crate::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
            "example:override",
            1.5,
            crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
            7,
            crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
        ),
    ] {
        expected.write_var_i32(attribute);
        expected.write_string(id);
        expected.write_f64(amount);
        expected.write_var_i32(operation);
        expected.write_var_i32(slot);
        expected.write_var_i32(display);
        if display
            == crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID
        {
            expected.write_bytes(
                &encode_nbt_summary_bounded(
                    Some(&display_text_nbt),
                    i32::MAX as usize,
                    MAX_NBT_DEPTH,
                )
                .unwrap(),
            );
        }
    }
    assert_eq!(encoded, expected.into_inner());

    let mut decoder = Decoder::new(&encoded);
    let mut decoded = DataComponentPatchSummary::default();
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.attribute_modifiers, summary.attribute_modifiers);
}

#[test]
fn attribute_modifiers_descriptor_rejects_noncanonical_summaries() {
    let valid = AttributeModifierSummary {
        attribute_id: crate::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
        modifier_id: "minecraft:test".to_string(),
        amount_bits: 1.0f64.to_bits(),
        operation_id:
            crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
        slot_id: VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID,
        display_id: crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
        display_text: None,
        display_text_styled: None,
        display_text_nbt: None,
    };
    let invalid = [
        (
            AttributeModifierSummary {
                attribute_id: i32::try_from(crate::attribute_types::VANILLA_ATTRIBUTE_TYPES.len())
                    .unwrap(),
                ..valid.clone()
            },
            "invalid vanilla attribute holder",
        ),
        (
            AttributeModifierSummary {
                modifier_id: "test".to_string(),
                ..valid.clone()
            },
            "noncanonical attribute modifier resource location",
        ),
        (
            AttributeModifierSummary {
                operation_id: 3,
                ..valid.clone()
            },
            "invalid attribute modifier operation",
        ),
        (
            AttributeModifierSummary {
                slot_id: 99,
                ..valid.clone()
            },
            "invalid equipment slot group",
        ),
        (
            AttributeModifierSummary {
                display_id:
                    crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_DEFAULT_ID,
                display_text: Some("dangling".to_string()),
                ..valid.clone()
            },
            "cannot contain override text",
        ),
        (
            AttributeModifierSummary {
                display_id:
                    crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                ..valid.clone()
            },
            "missing component nbt",
        ),
        (
            AttributeModifierSummary {
                display_id:
                    crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                display_text: Some("stale".to_string()),
                display_text_styled: Some(vec![StyledTextRun {
                    text: "wire".to_string(),
                    style: crate::ComponentStyle::default(),
                }]),
                display_text_nbt: Some(NbtSummaryValue::String("wire".into())),
                ..valid.clone()
            },
            "noncanonical component projections",
        ),
        (
            AttributeModifierSummary {
                display_id:
                    crate::attribute_modifier_types::VANILLA_ATTRIBUTE_MODIFIER_DISPLAY_OVERRIDE_ID,
                display_text: Some("invalid".to_string()),
                display_text_styled: Some(Vec::new()),
                display_text_nbt: Some(NbtSummaryValue::Int(1)),
                ..valid.clone()
            },
            "is not a component",
        ),
        (
            AttributeModifierSummary {
                display_id: 3,
                ..valid.clone()
            },
            "invalid attribute modifier display",
        ),
    ];
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID)
            .unwrap();
    for (modifier, expected_error) in invalid {
        let mut encoded = Encoder::new();
        let error = descriptor
            .encode(
                &DataComponentPatchSummary {
                    attribute_modifiers: vec![modifier],
                    ..DataComponentPatchSummary::default()
                },
                &mut encoded,
            )
            .unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error}");
    }

    let mut encoded = Encoder::new();
    let error = descriptor
        .encode(
            &DataComponentPatchSummary {
                attribute_modifiers: vec![valid; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                ..DataComponentPatchSummary::default()
            },
            &mut encoded,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(actual, max)
            if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                && max == MAX_DATA_COMPONENT_LIST_ITEMS
    ));
}

#[test]
fn food_descriptor_round_trips_raw_network_values_in_wire_order() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FOOD_TYPE_ID).unwrap();
    for (nutrition, saturation_bits, can_always_eat) in [
        (6, 7.2_f32.to_bits(), true),
        (-1, f32::from_bits(0x7fc0_1234).to_bits(), false),
    ] {
        let summary = DataComponentPatchSummary {
            food_nutrition: Some(nutrition),
            food_saturation_bits: Some(saturation_bits),
            food_can_always_eat: can_always_eat,
            ..DataComponentPatchSummary::default()
        };
        let mut encoded = Encoder::new();
        descriptor.encode(&summary, &mut encoded).unwrap();

        let mut expected = Encoder::new();
        expected.write_var_i32(nutrition);
        expected.write_f32(f32::from_bits(saturation_bits));
        expected.write_bool(can_always_eat);
        let encoded = encoded.into_inner();
        assert_eq!(encoded, expected.into_inner());

        let mut decoded = DataComponentPatchSummary::default();
        let mut decoder = Decoder::new(&encoded);
        descriptor.decode(&mut decoder, &mut decoded).unwrap();
        assert!(decoder.is_empty());
        assert_eq!(decoded.food_nutrition, Some(nutrition));
        assert_eq!(decoded.food_saturation_bits, Some(saturation_bits));
        assert_eq!(decoded.food_can_always_eat, can_always_eat);
    }
}

#[test]
fn food_descriptor_rejects_incomplete_projection_atomically() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_FOOD_TYPE_ID).unwrap();
    for summary in [
        DataComponentPatchSummary {
            food_saturation_bits: Some(0.0_f32.to_bits()),
            ..DataComponentPatchSummary::default()
        },
        DataComponentPatchSummary {
            food_nutrition: Some(0),
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
fn glider_descriptor_round_trips_empty_unit_payload() {
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    descriptor
        .encode(&DataComponentPatchSummary::default(), &mut encoded)
        .unwrap();

    let encoded = encoded.into_inner();
    assert!(encoded.is_empty());
    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded, DataComponentPatchSummary::default());
}

#[test]
fn consumable_descriptor_round_trips_all_effect_branches_in_wire_order() {
    let nan_bits = 0x7fc0_1234;
    let summary = DataComponentPatchSummary {
        consumable: Some(ConsumableSummary {
            consume_seconds: f32::from_bits(nan_bits),
            animation: ItemUseAnimationSummary::Spear,
        }),
        consumable_sound: Some(SoundEventSummary {
            registry_id: Some(8),
            sound_id: None,
            fixed_range_bits: None,
        }),
        consumable_has_consume_particles: Some(false),
        consumable_effects: vec![
            ConsumeEffectSummary::ApplyEffects {
                effects: vec![MobEffectInstanceSummary {
                    effect_id: 1,
                    amplifier: 255,
                    duration: -5,
                    ambient: true,
                    show_particles: false,
                    show_icon: true,
                    hidden_effect: Some(Box::new(MobEffectDetailsSummary {
                        amplifier: 0,
                        duration: 10,
                        ambient: false,
                        show_particles: true,
                        show_icon: false,
                        hidden_effect: None,
                    })),
                }],
                probability_bits: nan_bits,
            },
            ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids: Vec::new(),
                effect_tag: Some("minecraft:beneficial".to_string()),
            },
            ConsumeEffectSummary::ClearAllEffects,
            ConsumeEffectSummary::TeleportRandomly {
                diameter_bits: (-0.0_f32).to_bits(),
            },
            ConsumeEffectSummary::PlaySound {
                sound: SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:consume".to_string()),
                    fixed_range_bits: Some(f32::INFINITY.to_bits()),
                },
            },
        ],
        ..DataComponentPatchSummary::default()
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    let encoded = encoded.into_inner();

    let mut expected = Encoder::new();
    expected.write_f32(f32::from_bits(nan_bits));
    expected.write_var_i32(ItemUseAnimationSummary::Spear.id());
    expected.write_var_i32(9);
    expected.write_bool(false);
    expected.write_var_i32(5);
    expected.write_var_i32(
        bbb_facts::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_APPLY_EFFECTS_ID,
    );
    expected.write_var_i32(1);
    expected.write_var_i32(1);
    expected.write_var_i32(255);
    expected.write_var_i32(-5);
    expected.write_bool(true);
    expected.write_bool(false);
    expected.write_bool(true);
    expected.write_bool(true);
    expected.write_var_i32(0);
    expected.write_var_i32(10);
    expected.write_bool(false);
    expected.write_bool(true);
    expected.write_bool(false);
    expected.write_bool(false);
    expected.write_f32(f32::from_bits(nan_bits));
    expected.write_var_i32(
        bbb_facts::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID,
    );
    expected.write_var_i32(0);
    expected.write_string("minecraft:beneficial");
    expected.write_var_i32(
        bbb_facts::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_CLEAR_ALL_EFFECTS_ID,
    );
    expected.write_var_i32(
        bbb_facts::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_TELEPORT_RANDOMLY_ID,
    );
    expected.write_f32(-0.0);
    expected
        .write_var_i32(bbb_facts::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID);
    expected.write_var_i32(0);
    expected.write_string("example:consume");
    expected.write_bool(true);
    expected.write_f32(f32::INFINITY);
    assert_eq!(encoded, expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.consumable, summary.consumable);
    assert_eq!(decoded.consumable_sound, summary.consumable_sound);
    assert_eq!(
        decoded.consumable_has_consume_particles,
        summary.consumable_has_consume_particles
    );
    assert_eq!(decoded.consumable_effects, summary.consumable_effects);
}

#[test]
fn death_protection_descriptor_round_trips_direct_holders_and_empty_lists() {
    let summary = DataComponentPatchSummary {
        death_protection_effects: vec![
            ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids: vec![0, 3],
                effect_tag: None,
            },
            ConsumeEffectSummary::PlaySound {
                sound: SoundEventSummary {
                    registry_id: Some(12),
                    sound_id: None,
                    fixed_range_bits: None,
                },
            },
        ],
        ..DataComponentPatchSummary::default()
    };
    let descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID).unwrap();
    let mut encoded = Encoder::new();
    descriptor.encode(&summary, &mut encoded).unwrap();
    let encoded = encoded.into_inner();

    let mut expected = Encoder::new();
    expected.write_var_i32(2);
    expected.write_var_i32(
        bbb_facts::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_REMOVE_EFFECTS_ID,
    );
    expected.write_var_i32(3);
    expected.write_var_i32(0);
    expected.write_var_i32(3);
    expected
        .write_var_i32(bbb_facts::consume_effect_types::VANILLA_CONSUME_EFFECT_TYPE_PLAY_SOUND_ID);
    expected.write_var_i32(13);
    assert_eq!(encoded, expected.into_inner());

    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&encoded);
    descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(
        decoded.death_protection_effects,
        summary.death_protection_effects
    );

    let mut empty = Encoder::new();
    descriptor
        .encode(&DataComponentPatchSummary::default(), &mut empty)
        .unwrap();
    assert_eq!(empty.into_inner(), [0]);
}

#[test]
fn consume_effect_descriptors_enforce_bounds_and_canonical_values() {
    fn mob_effect() -> MobEffectInstanceSummary {
        MobEffectInstanceSummary {
            effect_id: 0,
            amplifier: 0,
            duration: 20,
            ambient: false,
            show_particles: true,
            show_icon: true,
            hidden_effect: None,
        }
    }

    fn valid_consumable() -> DataComponentPatchSummary {
        DataComponentPatchSummary {
            consumable: Some(ConsumableSummary {
                consume_seconds: 1.6,
                animation: ItemUseAnimationSummary::Eat,
            }),
            consumable_sound: Some(SoundEventSummary {
                registry_id: Some(0),
                sound_id: None,
                fixed_range_bits: None,
            }),
            consumable_has_consume_particles: Some(true),
            consumable_effects: vec![ConsumeEffectSummary::ClearAllEffects],
            ..DataComponentPatchSummary::default()
        }
    }

    let consumable_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID).unwrap();
    let death_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID).unwrap();
    let mut cases = Vec::new();
    cases.push((
        DataComponentPatchSummary::default(),
        "missing consumable component value",
    ));
    let mut missing_sound = valid_consumable();
    missing_sound.consumable_sound = None;
    cases.push((missing_sound, "missing sound holder"));
    let mut missing_particles = valid_consumable();
    missing_particles.consumable_has_consume_particles = None;
    cases.push((missing_particles, "missing particle flag"));
    let mut malformed_sound = valid_consumable();
    malformed_sound.consumable_sound = Some(SoundEventSummary {
        registry_id: Some(0),
        sound_id: Some("minecraft:entity.generic.eat".to_string()),
        fixed_range_bits: None,
    });
    cases.push((malformed_sound, "sound event holder projections"));
    let mut invalid_effect_id = valid_consumable();
    invalid_effect_id.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
        effects: vec![MobEffectInstanceSummary {
            effect_id: i32::try_from(crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len())
                .unwrap(),
            ..mob_effect()
        }],
        probability_bits: 1.0_f32.to_bits(),
    }];
    cases.push((invalid_effect_id, "invalid vanilla mob effect holder"));
    let mut invalid_amplifier = valid_consumable();
    invalid_amplifier.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
        effects: vec![MobEffectInstanceSummary {
            amplifier: 256,
            ..mob_effect()
        }],
        probability_bits: 1.0_f32.to_bits(),
    }];
    cases.push((invalid_amplifier, "amplifier 256 is outside"));
    let mut hidden_effect = None;
    for _ in 0..=MAX_MOB_EFFECT_DETAILS_DEPTH {
        hidden_effect = Some(Box::new(MobEffectDetailsSummary {
            amplifier: 0,
            duration: 1,
            ambient: false,
            show_particles: true,
            show_icon: true,
            hidden_effect,
        }));
    }
    let mut excessive_depth = valid_consumable();
    excessive_depth.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
        effects: vec![MobEffectInstanceSummary {
            hidden_effect,
            ..mob_effect()
        }],
        probability_bits: 1.0_f32.to_bits(),
    }];
    cases.push((excessive_depth, "exceeded max depth"));
    let mut noncanonical_tag = valid_consumable();
    noncanonical_tag.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
        effect_registry_ids: Vec::new(),
        effect_tag: Some("beneficial".to_string()),
    }];
    cases.push((noncanonical_tag, "noncanonical tag resource location"));
    let mut negative_holder = valid_consumable();
    negative_holder.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
        effect_registry_ids: vec![-1],
        effect_tag: None,
    }];
    cases.push((negative_holder, "invalid mob effect holder id -1"));

    for (summary, expected_error) in cases {
        let mut encoded = Encoder::new();
        let error = consumable_descriptor
            .encode(&summary, &mut encoded)
            .unwrap_err();
        assert!(error.to_string().contains(expected_error), "{error}");
    }

    let mut maximal_outer = valid_consumable();
    maximal_outer.consumable_effects =
        vec![ConsumeEffectSummary::ClearAllEffects; MAX_DATA_COMPONENT_LIST_ITEMS];
    consumable_descriptor
        .encode(&maximal_outer, &mut Encoder::new())
        .unwrap();
    let mut maximal_apply = valid_consumable();
    maximal_apply.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
        effects: vec![mob_effect(); MAX_DATA_COMPONENT_LIST_ITEMS],
        probability_bits: 1.0_f32.to_bits(),
    }];
    consumable_descriptor
        .encode(&maximal_apply, &mut Encoder::new())
        .unwrap();
    let mut maximal_holder_set = valid_consumable();
    maximal_holder_set.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
        effect_registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS],
        effect_tag: None,
    }];
    consumable_descriptor
        .encode(&maximal_holder_set, &mut Encoder::new())
        .unwrap();

    for oversized in [
        {
            let mut value = valid_consumable();
            value.consumable_effects =
                vec![ConsumeEffectSummary::ClearAllEffects; MAX_DATA_COMPONENT_LIST_ITEMS + 1];
            value
        },
        {
            let mut value = valid_consumable();
            value.consumable_effects = vec![ConsumeEffectSummary::ApplyEffects {
                effects: vec![mob_effect(); MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                probability_bits: 1.0_f32.to_bits(),
            }];
            value
        },
        {
            let mut value = valid_consumable();
            value.consumable_effects = vec![ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids: vec![0; MAX_DATA_COMPONENT_LIST_ITEMS + 1],
                effect_tag: None,
            }];
            value
        },
    ] {
        let mut encoded = Encoder::new();
        assert!(matches!(
            consumable_descriptor.encode(&oversized, &mut encoded),
            Err(ProtocolError::PacketTooLarge(actual, max))
                if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                    && max == MAX_DATA_COMPONENT_LIST_ITEMS
        ));
    }

    let oversized_death = DataComponentPatchSummary {
        death_protection_effects: vec![
            ConsumeEffectSummary::ClearAllEffects;
            MAX_DATA_COMPONENT_LIST_ITEMS + 1
        ],
        ..DataComponentPatchSummary::default()
    };
    let mut encoded = Encoder::new();
    assert!(matches!(
        death_descriptor.encode(&oversized_death, &mut encoded),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_DATA_COMPONENT_LIST_ITEMS + 1
                && max == MAX_DATA_COMPONENT_LIST_ITEMS
    ));
}

#[test]
fn weapon_interaction_descriptors_round_trip_exact_wire_values() {
    let nan_bits = 0x7fc0_1234;
    let piercing = PiercingWeaponSummary {
        deals_knockback: false,
        dismounts: true,
        sound: Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some("example:pierce".to_string()),
            fixed_range_bits: Some(nan_bits),
        }),
        hit_sound: Some(SoundEventSummary {
            registry_id: Some(7),
            sound_id: None,
            fixed_range_bits: None,
        }),
    };
    let kinetic = KineticWeaponSummary {
        contact_cooldown_ticks: -1,
        delay_ticks: -2,
        dismount_conditions: Some(KineticWeaponConditionSummary {
            max_duration_ticks: -3,
            min_speed_bits: (-0.0_f32).to_bits(),
            min_relative_speed_bits: f32::INFINITY.to_bits(),
        }),
        knockback_conditions: None,
        damage_conditions: Some(KineticWeaponConditionSummary {
            max_duration_ticks: 4,
            min_speed_bits: nan_bits,
            min_relative_speed_bits: 0.25_f32.to_bits(),
        }),
        forward_movement_bits: nan_bits,
        damage_multiplier_bits: (-0.0_f32).to_bits(),
        sound: None,
        hit_sound: Some(SoundEventSummary {
            registry_id: None,
            sound_id: Some("example:hit".to_string()),
            fixed_range_bits: None,
        }),
    };
    let swing = SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::Stab,
        duration: -4,
    };
    let summary = DataComponentPatchSummary {
        piercing_weapon: Some(piercing.clone()),
        kinetic_weapon: Some(kinetic.clone()),
        swing_animation: Some(swing),
        ..DataComponentPatchSummary::default()
    };

    let piercing_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID).unwrap();
    let kinetic_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID).unwrap();
    let swing_descriptor =
        vanilla_data_component_descriptor(VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID).unwrap();

    let mut piercing_bytes = Encoder::new();
    piercing_descriptor
        .encode(&summary, &mut piercing_bytes)
        .unwrap();
    let piercing_bytes = piercing_bytes.into_inner();
    let mut expected = Encoder::new();
    expected.write_bool(false);
    expected.write_bool(true);
    expected.write_bool(true);
    expected.write_var_i32(0);
    expected.write_string("example:pierce");
    expected.write_bool(true);
    expected.write_f32(f32::from_bits(nan_bits));
    expected.write_bool(true);
    expected.write_var_i32(8);
    assert_eq!(piercing_bytes, expected.into_inner());
    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&piercing_bytes);
    piercing_descriptor
        .decode(&mut decoder, &mut decoded)
        .unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.piercing_weapon, Some(piercing));

    let mut kinetic_bytes = Encoder::new();
    kinetic_descriptor
        .encode(&summary, &mut kinetic_bytes)
        .unwrap();
    let kinetic_bytes = kinetic_bytes.into_inner();
    let mut expected = Encoder::new();
    expected.write_var_i32(-1);
    expected.write_var_i32(-2);
    expected.write_bool(true);
    expected.write_var_i32(-3);
    expected.write_f32(-0.0);
    expected.write_f32(f32::INFINITY);
    expected.write_bool(false);
    expected.write_bool(true);
    expected.write_var_i32(4);
    expected.write_f32(f32::from_bits(nan_bits));
    expected.write_f32(0.25);
    expected.write_f32(f32::from_bits(nan_bits));
    expected.write_f32(-0.0);
    expected.write_bool(false);
    expected.write_bool(true);
    expected.write_var_i32(0);
    expected.write_string("example:hit");
    expected.write_bool(false);
    assert_eq!(kinetic_bytes, expected.into_inner());
    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&kinetic_bytes);
    kinetic_descriptor
        .decode(&mut decoder, &mut decoded)
        .unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.kinetic_weapon, Some(kinetic));

    let mut swing_bytes = Encoder::new();
    swing_descriptor.encode(&summary, &mut swing_bytes).unwrap();
    let swing_bytes = swing_bytes.into_inner();
    let mut expected = Encoder::new();
    expected.write_var_i32(SwingAnimationTypeSummary::Stab.id());
    expected.write_var_i32(-4);
    assert_eq!(swing_bytes, expected.into_inner());
    let mut decoded = DataComponentPatchSummary::default();
    let mut decoder = Decoder::new(&swing_bytes);
    swing_descriptor.decode(&mut decoder, &mut decoded).unwrap();
    assert!(decoder.is_empty());
    assert_eq!(decoded.swing_animation, Some(swing));
}
