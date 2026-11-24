use super::*;

#[test]
fn encodes_set_creative_mode_slot_tooltip_style_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID],
                tooltip_style: Some("example:rare".to_string()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_TOOLTIP_STYLE_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "tooltip_style component value")
            .unwrap(),
    );
    assert_eq!(value.read_string(32_767).unwrap(), "example:rare");
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_pot_decorations_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID],
                pot_decorations_item_ids: vec![VANILLA_ITEM_STONE_ID],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "pot_decorations component value")
            .unwrap(),
    );
    assert_eq!(value.read_len().unwrap(), 4);
    assert_eq!(value.read_var_i32().unwrap(), VANILLA_ITEM_STONE_ID);
    for _ in 0..3 {
        assert_eq!(value.read_var_i32().unwrap(), VANILLA_ITEM_BRICK_ID);
    }
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_provides_banner_patterns_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID],
                provides_banner_pattern_registry_ids: vec![0, 42],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "provides_banner_patterns component value")
            .unwrap(),
    );
    assert_eq!(value.read_var_i32().unwrap(), 3);
    assert_eq!(value.read_var_i32().unwrap(), 0);
    assert_eq!(value.read_var_i32().unwrap(), 42);
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_block_state_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID],
                block_state_properties: BTreeMap::from([
                    ("powered".into(), "true".into()),
                    ("facing".into(), "north".into()),
                ])
                .into(),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "block_state component value")
            .unwrap(),
    );
    assert_eq!(value.read_len().unwrap(), 2);
    assert_eq!(value.read_string(32767).unwrap(), "facing");
    assert_eq!(value.read_string(32767).unwrap(), "north");
    assert_eq!(value.read_string(32767).unwrap(), "powered");
    assert_eq!(value.read_string(32767).unwrap(), "true");
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_consume_effect_family_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID,
                ],
                consumable: Some(ConsumableSummary {
                    consume_seconds: 1.0,
                    animation: ItemUseAnimationSummary::Eat,
                }),
                consumable_sound: Some(SoundEventSummary {
                    registry_id: Some(0),
                    sound_id: None,
                    fixed_range_bits: None,
                }),
                consumable_has_consume_particles: Some(true),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            2, 0, // patch counts
            24, 8, // consumable type and delimited value length
            0x3f, 0x80, 0, 0, // consume seconds = 1.0
            1, // eat animation
            1, // sound registry holder 0
            1, // has consume particles
            0, // no consume effects
            36, 1, // death_protection type and delimited value length
            0, // no death effects
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_weapon_interaction_family_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID,
                    VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
                    VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                ],
                piercing_weapon: Some(PiercingWeaponSummary {
                    deals_knockback: true,
                    dismounts: false,
                    sound: None,
                    hit_sound: None,
                }),
                kinetic_weapon: Some(KineticWeaponSummary {
                    contact_cooldown_ticks: 10,
                    delay_ticks: 0,
                    dismount_conditions: None,
                    knockback_conditions: None,
                    damage_conditions: None,
                    forward_movement_bits: 0.0_f32.to_bits(),
                    damage_multiplier_bits: 1.0_f32.to_bits(),
                    sound: None,
                    hit_sound: None,
                }),
                swing_animation: Some(SwingAnimationSummary {
                    animation_type: SwingAnimationTypeSummary::Stab,
                    duration: 13,
                }),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            3, 0, // patch counts
            38, 4, // piercing_weapon type and delimited value length
            1, 0, // deals knockback, does not dismount
            0, 0, // no sound or hit sound
            39, 15, // kinetic_weapon type and delimited value length
            10, 0, // contact cooldown and delay
            0, 0, 0, // no dismount, knockback, or damage conditions
            0, 0, 0, 0, // forward movement = 0.0
            0x3f, 0x80, 0, 0, // damage multiplier = 1.0
            0, 0, // no sound or hit sound
            40, 2, // swing_animation type and delimited value length
            2, 13, // stab, duration
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_trim_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_TRIM_TYPE_ID],
                armor_trim_material_id: Some(0),
                armor_trim_pattern_id: Some(17),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            56, 2, // trim type and delimited value length
            1, 18, // material holder 0 and pattern holder 17, both offset by one
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_debug_stick_state_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID,
                ],
                debug_stick_properties: vec![crate::packets::DebugStickPropertySummary {
                    block_registry_id: crate::block_types::VANILLA_BLOCK_OAK_LOG_ID,
                    property: "axis".to_string(),
                }],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        crate::data_component_types::VANILLA_DATA_COMPONENT_DEBUG_STICK_STATE_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "debug_stick_state value")
            .unwrap(),
    );
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut value).unwrap(),
        Some(NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "minecraft:oak_log".into(),
            value: NbtSummaryValue::String("axis".into()),
        }]))
    );
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_lock_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
                ],
                lock_predicate: Some(crate::packets::PersistentItemPredicateSummary {
                    items: Some(crate::packets::HolderSetSummary {
                        registry_ids: vec![crate::item_types::VANILLA_ITEM_STONE_ID],
                        tag: None,
                    }),
                    count_min: Some(2),
                    count_max: Some(2),
                    ..crate::packets::PersistentItemPredicateSummary::default()
                }),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        crate::data_component_types::VANILLA_DATA_COMPONENT_LOCK_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(decoder.read_exact(value_len, "lock value").unwrap());
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut value).unwrap(),
        Some(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "items".into(),
                value: NbtSummaryValue::String("minecraft:stone".into()),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(2),
            },
        ]))
    );
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_direct_provides_trim_material_component() {
    let description_nbt = NbtSummaryValue::String("Ruby".into());
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID],
                provides_trim_material_direct: Some(Box::new(TrimMaterialSummary {
                    asset_name: "ruby".to_string(),
                    override_armor_assets: BTreeMap::from([(
                        "minecraft:netherite".to_string(),
                        "ruby_darker".to_string(),
                    )]),
                    description: "Ruby".to_string(),
                    description_styled: Some(
                        vec![StyledTextRun {
                            text: "Ruby".to_string(),
                            style: Default::default(),
                        }]
                        .into_boxed_slice(),
                    ),
                    description_nbt: Some(Box::new(description_nbt.clone())),
                })),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_PROVIDES_TRIM_MATERIAL_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "provides_trim_material component value")
            .unwrap(),
    );
    assert_eq!(value.read_var_i32().unwrap(), 0);
    assert_eq!(value.read_string(usize::MAX).unwrap(), "ruby");
    assert_eq!(value.read_len().unwrap(), 1);
    assert_eq!(
        value.read_string(usize::MAX).unwrap(),
        "minecraft:netherite"
    );
    assert_eq!(value.read_string(usize::MAX).unwrap(), "ruby_darker");
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut value).unwrap(),
        Some(description_nbt)
    );
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_direct_break_sound_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID],
                break_sound: Some(SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("example:item.custom_break".to_string()),
                    fixed_range_bits: Some(12.5_f32.to_bits()),
                }),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), 36);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_len().unwrap(), 0);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DATA_COMPONENT_BREAK_SOUND_TYPE_ID
    );
    let value_len = decoder.read_len().unwrap();
    let mut value = Decoder::new(
        decoder
            .read_exact(value_len, "break_sound component value")
            .unwrap(),
    );
    assert_eq!(value.read_var_i32().unwrap(), 0);
    assert_eq!(
        value.read_string(usize::MAX).unwrap(),
        "example:item.custom_break"
    );
    assert!(value.read_bool().unwrap());
    assert_eq!(value.read_f32().unwrap().to_bits(), 12.5_f32.to_bits());
    assert!(value.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_creative_mode_slot_map_id_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID],
                map_id: Some(42),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            46, 1,  // map_id type and delimited value length
            42, // map id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_map_decorations_component() {
    let frame = bbb_facts::vanilla_map_decoration_type_for_resource_id("minecraft:frame").unwrap();
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID],
                map_decorations: Some(vec![ItemMapDecorationSummary {
                    id: "home".into(),
                    decoration_type_id: frame.type_id,
                    decoration_type_resource_id: frame.resource_id.to_string(),
                    x_bits: 1.5_f64.to_bits(),
                    z_bits: (-2.25_f64).to_bits(),
                    rotation_bits: 90.0_f32.to_bits(),
                }]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            47, 73, // map_decorations type and delimited codec-NBT length
            10, // compound root
            10, 0, 4, b'h', b'o', b'm', b'e', // `home` compound
            8, 0, 4, b't', b'y', b'p', b'e', 0, 15, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f',
            b't', b':', b'f', b'r', b'a', b'm', b'e', // type
            6, 0, 1, b'x', 0x3f, 0xf8, 0, 0, 0, 0, 0, 0, // x = 1.5d
            6, 0, 1, b'z', 0xc0, 0x02, 0, 0, 0, 0, 0, 0, // z = -2.25d
            5, 0, 8, b'r', b'o', b't', b'a', b't', b'i', b'o', b'n', 0x42, 0xb4, 0, 0, 0,
            0, // inner and outer compound ends
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_recipes_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID],
                recipe_keys: Some(vec!["minecraft:bread".to_string()]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            66, 23, // recipes type and delimited codec-NBT length
            9, 8, 0, 0, 0, 1, // list root, string element type, one entry
            0, 15, b'm', b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':', b'b', b'r', b'e',
            b'a', b'd',
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_map_post_processing_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID],
                map_post_processing: Some(MapPostProcessingSummary::Scale),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            48, 1, // map_post_processing type and delimited value length
            1, // SCALE
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_dye_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DYE_TYPE_ID],
                dye_color_id: Some(VANILLA_DYE_COLOR_RED_ID),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            43, 1,  // dye type and delimited value length
            14, // RED
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_base_color_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BASE_COLOR_TYPE_ID],
                base_color_id: Some(VANILLA_DYE_COLOR_RED_ID),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            73, 1,  // base_color type and delimited value length
            14, // RED
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_ominous_bottle_amplifier_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID],
                ominous_bottle_amplifier: Some(3),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            63, 1, // ominous bottle amplifier type and delimited value length
            3, // Bad Omen IV amplifier
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_note_block_sound_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_NOTE_BLOCK_SOUND_TYPE_ID],
                note_block_sound: Some("bbb:chime".to_string()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            71, 10, // note_block_sound type and delimited value length
            9, 98, 98, 98, 58, 99, 104, 105, 109, 101, // bbb:chime
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_villager_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_VILLAGER_VARIANT_TYPE_ID],
                villager_variant_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            81, 1, // villager/variant type and delimited value length
            2, // PLAINS holder id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_wolf_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID],
                wolf_variant_id: Some(8),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            82, 1, // wolf/variant type and delimited value length
            8, // STRIPED holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_wolf_sound_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_WOLF_SOUND_VARIANT_TYPE_ID],
                wolf_sound_variant_id: Some(6),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            83, 1, // wolf/sound_variant type and delimited value length
            6, // CUTE holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_wolf_collar_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
                    VANILLA_DYE_COLOR_RED_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            84, 1,  // wolf/collar type and delimited value length
            14, // RED id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_fox_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_FOX_VARIANT_TYPE_ID,
                    VANILLA_FOX_VARIANT_SNOW_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            85, 1, // fox/variant type and delimited value length
            1, // SNOW id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_pig_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PIG_VARIANT_TYPE_ID],
                pig_variant_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            93, 1, // pig/variant type and delimited value length
            2, // COLD holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_pig_sound_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PIG_SOUND_VARIANT_TYPE_ID],
                pig_sound_variant_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            94, 1, // pig/sound_variant type and delimited value length
            1, // BIG holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_cow_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_COW_VARIANT_TYPE_ID],
                cow_variant_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            95, 1, // cow/variant type and delimited value length
            2, // COLD holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_cow_sound_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_COW_SOUND_VARIANT_TYPE_ID],
                cow_sound_variant_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            96, 1, // cow/sound_variant type and delimited value length
            1, // MOODY holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_chicken_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CHICKEN_VARIANT_TYPE_ID],
                chicken_variant_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            97, 1, // chicken/variant type and delimited value length
            2, // COLD holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_chicken_sound_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CHICKEN_SOUND_VARIANT_TYPE_ID],
                chicken_sound_variant_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            98, 1, // chicken/sound_variant type and delimited value length
            1, // PICKY holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_zombie_nautilus_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_ZOMBIE_NAUTILUS_VARIANT_TYPE_ID],
                zombie_nautilus_variant_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            99, 1, // zombie_nautilus/variant type and delimited value length
            1, // WARM holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_frog_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_FROG_VARIANT_TYPE_ID],
                frog_variant_id: Some(2),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            100, 1, // frog/variant type and delimited value length
            2, // COLD holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_cat_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CAT_VARIANT_TYPE_ID],
                cat_variant_id: Some(10),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            105, 1,  // cat/variant type and delimited value length
            10, // ALL_BLACK holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_cat_sound_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CAT_SOUND_VARIANT_TYPE_ID],
                cat_sound_variant_id: Some(1),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            106, 1, // cat/sound_variant type and delimited value length
            1, // ROYAL holder id in the vanilla fallback catalog
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_cat_collar_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_CAT_COLLAR_TYPE_ID,
                    VANILLA_DYE_COLOR_RED_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            107, 1,  // cat/collar type and delimited value length
            14, // RED id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_salmon_size_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SALMON_SIZE_TYPE_ID,
                    VANILLA_SALMON_VARIANT_LARGE_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            86, 1, // salmon/size type and delimited value length
            2, // LARGE id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_parrot_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_PARROT_VARIANT_TYPE_ID,
                    VANILLA_PARROT_VARIANT_GRAY_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            87, 1, // parrot/variant type and delimited value length
            4, // GRAY id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_tropical_fish_pattern_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_TYPE_ID],
                tropical_fish_pattern_id: Some(257),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            88, 2, // tropical_fish/pattern type and delimited value length
            0x81, 0x02, // STRIPEY sparse packed id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_tropical_fish_base_color_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_TROPICAL_FISH_BASE_COLOR_TYPE_ID],
                tropical_fish_base_color_id: Some(11),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            89, 1,  // tropical_fish/base_color type and delimited value length
            11, // BLUE dye id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_tropical_fish_pattern_color_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_TROPICAL_FISH_PATTERN_COLOR_TYPE_ID],
                tropical_fish_pattern_color_id: Some(9),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            90, 1, // tropical_fish/pattern_color type and delimited value length
            9, // CYAN dye id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_mooshroom_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_MOOSHROOM_VARIANT_TYPE_ID,
                    VANILLA_MOOSHROOM_VARIANT_BROWN_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            91, 1, // mooshroom/variant type and delimited value length
            1, // BROWN id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_rabbit_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_RABBIT_VARIANT_TYPE_ID,
                    VANILLA_RABBIT_VARIANT_EVIL_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            92, 1,  // rabbit/variant type and delimited value length
            99, // EVIL sparse id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_horse_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_HORSE_VARIANT_TYPE_ID,
                    VANILLA_HORSE_VARIANT_DARK_BROWN_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            101, 1, // horse/variant type and delimited value length
            6, // DARK_BROWN id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_llama_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_LLAMA_VARIANT_TYPE_ID,
                    VANILLA_LLAMA_VARIANT_GRAY_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            103, 1, // llama/variant type and delimited value length
            3, // GRAY id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_axolotl_variant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_AXOLOTL_VARIANT_TYPE_ID,
                    VANILLA_AXOLOTL_VARIANT_BLUE_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            104, 1, // axolotl/variant type and delimited value length
            4, // BLUE id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_sheep_color_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SHEEP_COLOR_TYPE_ID,
                    VANILLA_DYE_COLOR_RED_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            108, 1,  // sheep color type and delimited value length
            14, // RED id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_shulker_color_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID],
                builtin_enum_component_ids: BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_SHULKER_COLOR_TYPE_ID,
                    VANILLA_DYE_COLOR_RED_ID,
                )]),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            109, 1,  // shulker color type and delimited value length
            14, // RED id
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_dyed_color_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID],
                dyed_color: Some(0x12_34_56),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            44, 4, // dyed_color type and delimited value length
            0, 0x12, 0x34, 0x56, // fixed-width RGB int
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_map_color_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_COLOR_TYPE_ID],
                map_color: Some(0x45_67_89),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            45, 4, // map_color type and delimited value length
            0, 0x45, 0x67, 0x89, // fixed-width RGB int
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_firework_family_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_FIREWORK_EXPLOSION_TYPE_ID,
                    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
                ],
                firework_explosion: Some(FireworkExplosionSummary {
                    shape: FireworkExplosionShapeSummary::Star,
                    colors: vec![0x11_22_33],
                    has_trail: true,
                    has_twinkle: false,
                    ..Default::default()
                }),
                fireworks_flight_duration: Some(2),
                fireworks_explosions: vec![FireworkExplosionSummary {
                    shape: FireworkExplosionShapeSummary::Burst,
                    colors: vec![-1],
                    fade_colors: vec![0x44_55_66],
                    has_trail: false,
                    has_twinkle: true,
                }],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            2, 0, // patch counts
            68, 9, // firework_explosion type and delimited value length
            2, 1, 0, 0x11, 0x22, 0x33, // star and one color
            0, 1, 0, // no fade colors, trail, no twinkle
            69, 15, // fireworks type and delimited value length
            2, 1, // flight duration and one explosion
            4, 1, 0xff, 0xff, 0xff, 0xff, // burst and one signed color
            1, 0, 0x44, 0x55, 0x66, // one fade color
            0, 1, // no trail, twinkle
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_potion_effect_family_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
                    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_SUSPICIOUS_STEW_EFFECTS_TYPE_ID,
                ],
                potion_contents: PotionContentsSummary {
                    potion_id: Some(3),
                    custom_color: Some(0x11_22_33),
                    custom_effects: vec![MobEffectInstanceSummary {
                        effect_id: 2,
                        amplifier: 1,
                        duration: 200,
                        ambient: false,
                        show_particles: true,
                        show_icon: false,
                        hidden_effect: None,
                    }],
                    custom_name: Some("swift".into()),
                },
                potion_duration_scale_bits: Some(0.125_f32.to_bits()),
                suspicious_stew_effects: vec![
                    SuspiciousStewEffectSummary {
                        effect_id: 0,
                        duration: 160,
                    },
                    SuspiciousStewEffectSummary {
                        effect_id: 1,
                        duration: -1,
                    },
                ],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            3, 0, // patch counts
            51, 23, // potion_contents type and delimited value length
            1, 3, // present potion holder
            1, 0, 0x11, 0x22, 0x33, // present custom color
            1,    // one custom effect
            2, 1, 0xc8, 1, // effect holder, amplifier, duration
            0, 1, 0, 0, // ambient, particles, icon, no hidden effect
            1, 5, b's', b'w', b'i', b'f', b't', // present custom name
            52, 4, 0x3e, 0, 0, 0, // potion duration scale = 0.125
            53, 10, // suspicious_stew_effects type and delimited value length
            2, 0, 0xa0, 1, // two entries, first holder and duration
            1, 0xff, 0xff, 0xff, 0xff, 0x0f, // second holder and -1 duration
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_blocks_attacks_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID],
                blocks_attacks: Some(BlocksAttacksSummary {
                    block_delay_seconds_bits: 0.25_f32.to_bits(),
                    disable_cooldown_scale_bits: 1.0_f32.to_bits(),
                    damage_reductions: Vec::new(),
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
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            37, 24, // blocks_attacks type and delimited value length
            0x3e, 0x80, 0, 0, // block delay = 0.25
            0x3f, 0x80, 0, 0, // disable cooldown scale = 1.0
            0, // no damage reductions
            0x3f, 0x80, 0, 0, // item damage threshold = 1.0
            0, 0, 0, 0, // item damage base = 0.0
            0x3f, 0x80, 0, 0, // item damage factor = 1.0
            0, 0, 0, // no bypassed-by set or sounds
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_damage_resistant_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID],
                damage_resistant_type_registry_ids: vec![2, 4],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            27, 3, // damage_resistant type and delimited value length
            3, 2, 4, // two direct damage type holders
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_repairable_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID],
                repairable_item_registry_ids: vec![1, 193],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            33, 4, // repairable type and delimited value length
            3, 1, 0xc1, 0x01, // two direct item holders
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_weapon_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID],
                weapon_item_damage_per_attack: Some(3),
                weapon_disable_blocking_seconds_bits: Some(5.0_f32.to_bits()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            29, 5, // weapon type and delimited value length
            3, 0x40, 0xa0, 0, 0, // damage per attack and five seconds
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_container_loot_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID],
                container_loot: true,
                container_loot_table: Some("minecraft:empty".to_string()),
                container_loot_seed: Some(0),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            79, 32, // container_loot type and delimited codec-NBT length
            10, // compound root
            8, 0, 10, b'l', b'o', b'o', b't', b'_', b't', b'a', b'b', b'l', b'e', 0, 15, b'm',
            b'i', b'n', b'e', b'c', b'r', b'a', b'f', b't', b':', b'e', b'm', b'p', b't', b'y',
            0, // loot_table string and compound end
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_written_book_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID],
                written_book: Some(WrittenBookContentSummary {
                    title: "Guide".into(),
                    title_filter: None,
                    author: "Alex".into(),
                    generation: 1,
                    pages: vec!["Raw".to_string()],
                    page_filters: vec![Some("Safe".to_string())],
                    resolved: true,
                }),
                written_book_pages_nbt: vec![Some(NbtSummaryValue::String("Raw".into()))],
                written_book_page_filters_nbt: vec![Some(NbtSummaryValue::String("Safe".into()))],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            55, 29, // written_book_content type and delimited value length
            5, b'G', b'u', b'i', b'd', b'e', 0, // raw title, no filtered title
            4, b'A', b'l', b'e', b'x', // author
            1,    // generation
            1,    // page count
            8, 0, 3, b'R', b'a', b'w', // raw page Component NBT
            1, 8, 0, 4, b'S', b'a', b'f', b'e', // filtered page
            1,    // resolved
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_lore_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_LORE_TYPE_ID],
                lore: vec!["First".to_string(), "Second".to_string()],
                lore_styled: vec![
                    vec![crate::StyledTextRun {
                        text: "First".to_string(),
                        style: crate::ComponentStyle::default(),
                    }],
                    vec![crate::StyledTextRun {
                        text: "Second".to_string(),
                        style: crate::ComponentStyle::default(),
                    }],
                ],
                lore_nbt: vec![
                    NbtSummaryValue::String("First".into()),
                    NbtSummaryValue::String("Second".into()),
                ],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            11, 18, // lore type and delimited value length
            2,  // line count
            8, 0, 5, b'F', b'i', b'r', b's', b't', // first component
            8, 0, 6, b'S', b'e', b'c', b'o', b'n', b'd', // second component
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_name_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                    VANILLA_DATA_COMPONENT_ITEM_NAME_TYPE_ID,
                ],
                custom_name: Some("Custom".to_string()),
                custom_name_styled: Some(vec![crate::StyledTextRun {
                    text: "Custom".to_string(),
                    style: crate::ComponentStyle::default(),
                }]),
                custom_name_nbt: Some(NbtSummaryValue::String("Custom".into())),
                item_name: Some("Item".to_string()),
                item_name_styled: Some(vec![crate::StyledTextRun {
                    text: "Item".to_string(),
                    style: crate::ComponentStyle::default(),
                }]),
                item_name_nbt: Some(NbtSummaryValue::String("Item".into())),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            2, 0, // patch counts
            6, 9, // custom_name type and delimited value length
            8, 0, 6, b'C', b'u', b's', b't', b'o', b'm', // custom component
            9, 7, // item_name type and delimited value length
            8, 0, 4, b'I', b't', b'e', b'm', // item component
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_use_effects_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID],
                use_effects: Some(crate::packets::UseEffectsSummary {
                    can_sprint: true,
                    interact_vibrations: false,
                    speed_multiplier: 0.25,
                }),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            5, 6, // use_effects type and delimited value length
            1, 0, // can sprint; does not emit interaction vibrations
            0x3e, 0x80, 0, 0, // speed multiplier = 0.25
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_minimum_attack_charge_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID],
                minimum_attack_charge_bits: Some(0.75_f32.to_bits()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            7, 4, // minimum_attack_charge type and delimited value length
            0x3f, 0x40, 0, 0, // minimum attack charge = 0.75
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_damage_type_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID],
                damage_type_id: Some(300),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            8, 2, // damage_type component and delimited value length
            0xac, 2, // session holder id 300
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_item_enchantment_components() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                    VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
                ],
                enchantments: vec![crate::packets::ItemEnchantmentSummary {
                    holder_id: 37,
                    level: 4,
                }],
                stored_enchantments: vec![crate::packets::ItemEnchantmentSummary {
                    holder_id: 13,
                    level: 5,
                }],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            2, 0, // patch counts
            13, 3, // enchantments type and delimited value length
            1, 37, 4, // one level-four enchantment
            42, 3, // stored_enchantments type and delimited value length
            1, 13, 5, // one stored level-five enchantment
        ]
    );
}

#[test]
fn creative_slot_enchantment_levels_follow_the_persistent_range() {
    for type_id in [
        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
    ] {
        for level in [-1, 0, 1, 255, 256] {
            let mut patch = DataComponentPatchSummary {
                added_type_ids: vec![type_id],
                ..DataComponentPatchSummary::default()
            };
            let entries = vec![crate::packets::ItemEnchantmentSummary {
                holder_id: 13,
                level,
            }];
            if type_id == VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID {
                patch.enchantments = entries;
            } else {
                patch.stored_enchantments = entries;
            }
            let result = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
                slot_num: 36,
                item: ItemStackSummary {
                    item_id: Some(42),
                    count: 1,
                    component_patch: patch.into(),
                },
            });
            if (1..=255).contains(&level) {
                let (_, payload) = result.unwrap();
                if level == 1 {
                    assert!(payload.ends_with(&[1, 13, 1]));
                } else {
                    assert!(payload.ends_with(&[1, 13, 0xff, 1]));
                }
            } else {
                assert!(result.is_err(), "{type_id} level {level}");
            }
        }
    }
}

#[test]
fn encodes_set_creative_mode_slot_writable_book_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID],
                writable_book_pages: vec!["raw".into(), "second".into()],
                writable_book_page_filters: vec![Some("safe".into()), None],
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            54, 19, // writable_book_content type and delimited value length
            2,  // page count
            3, b'r', b'a', b'w', // raw first page
            1, 4, b's', b'a', b'f', b'e', // filtered first page
            6, b's', b'e', b'c', b'o', b'n', b'd', // raw second page
            0,    // no filtered second page
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_profile_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    crate::data_component_types::VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID,
                ],
                profile: Some(ResolvableProfileSummary {
                    kind: ResolvableProfileKindSummary::GameProfile,
                    uuid: Some(Uuid::from_u128(1)),
                    name: Some("Alex".to_string()),
                    properties: Vec::new(),
                    profile_textures: None,
                    skin_patch: PlayerSkinPatchSummary::default(),
                }),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            70, 27, // profile type and value length
            1,  // Either.left(GameProfile)
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, // UUID
            4, b'A', b'l', b'e', b'x', // name
            0,    // property count
            0, 0, 0, 0, // empty skin patch
        ]
    );
}

#[test]
fn encodes_set_creative_mode_slot_drop_empty_stack() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: -1,
        item: ItemStackSummary::empty(),
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i16().unwrap(), -1);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());
}

#[test]
fn set_creative_mode_slot_normalizes_air_item_to_empty_stack() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(crate::item_types::VANILLA_ITEM_AIR_ID),
            count: 99,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                max_stack_size: Some(99),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(payload, [0, 36, 0]);
}

#[test]
fn encodes_set_creative_mode_slot_adventure_mode_predicate_component() {
    let (id, payload) = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID],
                can_place_on_persistent: Some(AdventureModePredicateDataSummary {
                    predicates: vec![BlockPredicateSummary {
                        blocks: Some(HolderSetSummary {
                            registry_ids: vec![1],
                            tag: None,
                        }),
                        ..BlockPredicateSummary::default()
                    }],
                }),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT);
    assert_eq!(
        payload,
        [
            0, 36, // slot
            1, 42, // count and item holder
            1, 0, // patch counts
            14, 8, // can_place_on type and delimited value length
            1, // predicate count
            1, // blocks present
            2, 1, // one direct block holder
            0, // no state predicate
            0, // no nbt predicate
            0, // no exact component predicates
            0, // no partial component predicates
        ]
    );
}
