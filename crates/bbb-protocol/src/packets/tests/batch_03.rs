use super::*;

#[test]
fn set_creative_mode_slot_rejects_values_rejected_by_the_untrusted_stack_codec() {
    let stack_with_patch = |count, component_patch| SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(42),
            count,
            component_patch,
        },
    };
    let max_stack_size_patch = |value| DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
        max_stack_size: Some(value),
        ..DataComponentPatchSummary::default()
    };
    let max_damage_patch = |value| DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID],
        max_damage: Some(value),
        ..DataComponentPatchSummary::default()
    };
    let damage_patch = |value| DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID],
        damage: Some(value),
        ..DataComponentPatchSummary::default()
    };
    let damage_type_patch = |holder_id| DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID],
        damage_type_id: Some(holder_id),
        ..DataComponentPatchSummary::default()
    };
    let use_effects_patch = |speed_multiplier| DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID],
        use_effects: Some(crate::packets::UseEffectsSummary {
            can_sprint: false,
            interact_vibrations: true,
            speed_multiplier,
        }),
        ..DataComponentPatchSummary::default()
    };
    let minimum_attack_charge_patch = |value: f32| DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID],
        minimum_attack_charge_bits: Some(value.to_bits()),
        ..DataComponentPatchSummary::default()
    };
    let enchantments_patch = |type_id, level| {
        let mut patch = DataComponentPatchSummary {
            added_type_ids: vec![type_id],
            ..DataComponentPatchSummary::default()
        };
        let value = vec![crate::packets::ItemEnchantmentSummary {
            holder_id: 0,
            level,
        }];
        if type_id == VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID {
            patch.enchantments = value;
        } else {
            patch.stored_enchantments = value;
        }
        patch
    };
    let fireworks_patch = |value| DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID],
        fireworks_flight_duration: Some(value),
        ..DataComponentPatchSummary::default()
    };
    let potion_duration_scale_patch = |bits| DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID],
        potion_duration_scale_bits: Some(bits),
        ..DataComponentPatchSummary::default()
    };
    let adventure_mode_patch = |predicates: Vec<BlockPredicateSummary>| DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID],
        can_place_on_persistent: Some(AdventureModePredicateDataSummary { predicates }),
        ..DataComponentPatchSummary::default()
    };

    for packet in [
        stack_with_patch(100, Default::default()),
        stack_with_patch(1, max_stack_size_patch(0).into()),
        stack_with_patch(1, max_stack_size_patch(100).into()),
        stack_with_patch(1, max_damage_patch(0).into()),
        stack_with_patch(1, max_damage_patch(-1).into()),
        stack_with_patch(1, damage_patch(-1).into()),
        stack_with_patch(1, damage_type_patch(-1).into()),
        stack_with_patch(1, use_effects_patch(-0.0).into()),
        stack_with_patch(1, use_effects_patch(-1.0).into()),
        stack_with_patch(1, use_effects_patch(1.000_000_1).into()),
        stack_with_patch(1, use_effects_patch(f32::INFINITY).into()),
        stack_with_patch(1, use_effects_patch(f32::NAN).into()),
        stack_with_patch(1, minimum_attack_charge_patch(-0.0).into()),
        stack_with_patch(1, minimum_attack_charge_patch(-1.0).into()),
        stack_with_patch(1, minimum_attack_charge_patch(1.000_000_1).into()),
        stack_with_patch(1, minimum_attack_charge_patch(f32::INFINITY).into()),
        stack_with_patch(1, minimum_attack_charge_patch(f32::NAN).into()),
        stack_with_patch(
            1,
            enchantments_patch(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, -1).into(),
        ),
        stack_with_patch(
            1,
            enchantments_patch(VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, 256).into(),
        ),
        stack_with_patch(1, fireworks_patch(-1).into()),
        stack_with_patch(1, fireworks_patch(256).into()),
        stack_with_patch(1, potion_duration_scale_patch((-0.0_f32).to_bits()).into()),
        stack_with_patch(1, potion_duration_scale_patch((-1.0_f32).to_bits()).into()),
        stack_with_patch(
            1,
            potion_duration_scale_patch(f32::INFINITY.to_bits()).into(),
        ),
        stack_with_patch(1, potion_duration_scale_patch(f32::NAN.to_bits()).into()),
        stack_with_patch(1, adventure_mode_patch(Vec::new()).into()),
        stack_with_patch(
            1,
            adventure_mode_patch(vec![BlockPredicateSummary {
                state: Some(vec![
                    StatePropertyMatcherSummary {
                        name: "axis".into(),
                        value: StatePropertyValueMatcherSummary::Exact("x".into()),
                    },
                    StatePropertyMatcherSummary {
                        name: "axis".into(),
                        value: StatePropertyValueMatcherSummary::Exact("y".into()),
                    },
                ]),
                ..BlockPredicateSummary::default()
            }])
            .into(),
        ),
    ] {
        assert!(encode_play_set_creative_mode_slot(&packet).is_err());
    }
}

#[test]
fn set_creative_mode_slot_accepts_untrusted_stack_codec_boundaries() {
    for (flight_duration, duration_scale_bits, bounded_float, enchantment_level) in [
        (0, 0.0_f32.to_bits(), 0.0, 1),
        (255, f32::MAX.to_bits(), 1.0, 255),
    ] {
        encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(42),
                count: 99,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![
                        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                        VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
                        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
                        VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                        VANILLA_DATA_COMPONENT_STORED_ENCHANTMENTS_TYPE_ID,
                        VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
                        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
                    ],
                    max_stack_size: Some(99),
                    max_damage: Some(1),
                    damage: Some(0),
                    use_effects: Some(crate::packets::UseEffectsSummary {
                        can_sprint: false,
                        interact_vibrations: true,
                        speed_multiplier: bounded_float,
                    }),
                    minimum_attack_charge_bits: Some(bounded_float.to_bits()),
                    enchantments: vec![crate::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: enchantment_level,
                    }],
                    stored_enchantments: vec![crate::packets::ItemEnchantmentSummary {
                        holder_id: 0,
                        level: enchantment_level,
                    }],
                    fireworks_flight_duration: Some(flight_duration),
                    potion_duration_scale_bits: Some(duration_scale_bits),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
        })
        .unwrap();
    }
}

#[test]
fn set_creative_mode_slot_enforces_persistent_counts_for_recursive_item_components() {
    fn recursive_patch(component_type_id: i32, count: i32) -> DataComponentPatchSummary {
        let template = crate::packets::ItemStackTemplateSummary {
            item_id: 42,
            count,
            component_patch: Default::default(),
        };
        let mut patch = DataComponentPatchSummary {
            added_type_ids: vec![component_type_id],
            ..DataComponentPatchSummary::default()
        };
        match component_type_id {
            crate::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID => {
                patch.use_remainder = Some(Box::new(template));
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID => {
                patch.charged_projectiles_items = vec![template];
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID => {
                patch.bundle_contents_items = vec![template];
            }
            crate::data_component_types::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID => {
                patch.container.slots = vec![crate::packets::ItemContainerSlotSummary {
                    slot: 0,
                    item: template,
                }];
                patch.container.slot_count = Some(1);
            }
            _ => unreachable!("recursive component type"),
        }
        patch
    }

    let component_type_ids = [
        crate::data_component_types::VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
        crate::data_component_types::VANILLA_DATA_COMPONENT_CHARGED_PROJECTILES_TYPE_ID,
        crate::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID,
        crate::data_component_types::VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID,
    ];
    for component_type_id in component_type_ids {
        for count in [1, 99] {
            encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
                slot_num: 36,
                item: ItemStackSummary {
                    item_id: Some(42),
                    count: 1,
                    component_patch: recursive_patch(component_type_id, count).into(),
                },
            })
            .unwrap();
        }
        for count in [-1, 0, 100] {
            let error = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
                slot_num: 36,
                item: ItemStackSummary {
                    item_id: Some(42),
                    count: 1,
                    component_patch: recursive_patch(component_type_id, count).into(),
                },
            })
            .unwrap_err();
            assert!(
                error.to_string().contains(&format!(
                    "nested item stack count {count} must be in 1..=99"
                )),
                "{error}"
            );
        }
    }
}

#[test]
fn set_creative_mode_slot_rejects_an_out_of_range_builtin_item_holder() {
    let invalid_item_id = crate::item_types::VANILLA_ITEM_TYPES.len() as i32;
    let error = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
        slot_num: 36,
        item: ItemStackSummary {
            item_id: Some(invalid_item_id),
            count: 1,
            component_patch: Default::default(),
        },
    })
    .unwrap_err();

    assert!(error
        .to_string()
        .contains(&format!("invalid vanilla item holder id {invalid_item_id}")));
}

#[test]
fn set_creative_mode_slot_rejects_persistent_and_wire_asymmetric_components() {
    use crate::data_component_types::{
        VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
        VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
        VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID,
        VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
    };
    use crate::packets::data_components::{
        AdventureModePredicateDataSummary, BlockPredicateSummary, ConsumeEffectSummary,
        DataComponentPatchSummary, PlayerSkinPatchSummary, ResolvableProfileKindSummary,
        ResolvableProfileSummary,
    };

    let check_rejected = |patch: DataComponentPatchSummary, label: &str| {
        let result = encode_play_set_creative_mode_slot(&SetCreativeModeSlot {
            slot_num: 36,
            item: ItemStackSummary {
                item_id: Some(42),
                count: 1,
                component_patch: patch.into(),
            },
        });
        assert!(result.is_err(), "expected creative rejection for {label}");
    };

    // Negative food nutrition
    check_rejected(
        DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_FOOD_TYPE_ID],
            food_nutrition: Some(-1),
            food_saturation_bits: Some(1.0_f32.to_bits()),
            ..DataComponentPatchSummary::default()
        },
        "negative food nutrition",
    );
    // NaN teleport_randomly diameter
    check_rejected(
        DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID],
            death_protection_effects: vec![ConsumeEffectSummary::TeleportRandomly {
                diameter_bits: f32::NAN.to_bits(),
            }],
            ..DataComponentPatchSummary::default()
        },
        "NaN teleport_randomly diameter",
    );
    // Negative weapon damage (wire decode accepts, wire encode rejects)
    check_rejected(
        DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID],
            weapon_item_damage_per_attack: Some(-1),
            weapon_disable_blocking_seconds_bits: Some(0.0_f32.to_bits()),
            ..DataComponentPatchSummary::default()
        },
        "negative weapon damage",
    );
    // Out-of-range ominous_bottle_amplifier
    check_rejected(
        DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID],
            ominous_bottle_amplifier: Some(5),
            ..DataComponentPatchSummary::default()
        },
        "ominous_bottle_amplifier=5",
    );
    // Invalid profile player name
    check_rejected(
        DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID],
            profile: Some(ResolvableProfileSummary {
                kind: ResolvableProfileKindSummary::Partial,
                uuid: None,
                name: Some("bad name!".to_string()),
                properties: Vec::new(),
                profile_textures: None,
                skin_patch: PlayerSkinPatchSummary::default(),
            }),
            ..DataComponentPatchSummary::default()
        },
        "invalid profile player name",
    );
    // Out-of-range max_stack_size nested inside can_place_on exact_components
    check_rejected(
        DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID],
            can_place_on_persistent: Some(AdventureModePredicateDataSummary {
                predicates: vec![BlockPredicateSummary {
                    exact_components: Some(Box::new(DataComponentPatchSummary {
                        added_type_ids: vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID],
                        max_stack_size: Some(0),
                        ..DataComponentPatchSummary::default()
                    })),
                    ..BlockPredicateSummary::default()
                }],
            }),
            ..DataComponentPatchSummary::default()
        },
        "nested exact_components max_stack_size=0",
    );
}

#[test]
fn large_system_chat_and_team_and_attribute_packets_round_trip_above_legacy_caps() {
    use crate::packets::{
        encode_play_clientbound_set_player_team, encode_play_clientbound_system_chat,
        encode_play_clientbound_update_attributes, AttributeModifier, AttributeSnapshot,
        PlayerTeamMethod, SetPlayerTeam, SystemChat, UpdateAttributes,
    };

    // > 64 KiB text component composed of multiple valid NBT strings.
    let entries: Vec<_> = (0..12)
        .map(|_| crate::NbtSummaryValue::string("a".repeat(8000)))
        .collect();
    let big_component =
        crate::RichTextComponent::try_from_component_nbt(crate::NbtSummaryValue::List(entries))
            .unwrap();
    let (id, payload) = encode_play_clientbound_system_chat(&SystemChat {
        content: Box::new(big_component),
        overlay: false,
    })
    .unwrap();
    assert!(payload.len() > 64 * 1024);
    assert!(decode_play_clientbound(id, &payload).is_ok());

    // > 8192 team members
    let players: Vec<crate::JavaString> = (0..8200).map(|i| format!("p{i}").into()).collect();
    let (team_id, team_payload) = encode_play_clientbound_set_player_team(&SetPlayerTeam {
        name: "huge".into(),
        method: PlayerTeamMethod::Join.into(),
        parameters: None,
        players,
    })
    .unwrap();
    assert!(decode_play_clientbound(team_id, &team_payload).is_ok());

    // > 1024 attribute modifiers
    let modifiers: Vec<AttributeModifier> = (0..1030)
        .map(|i| AttributeModifier {
            id: format!("minecraft:mod_{i}"),
            amount: 1.0,
            operation_id: 0,
        })
        .collect();
    let (attr_id, attr_payload) = encode_play_clientbound_update_attributes(&UpdateAttributes {
        entity_id: 1,
        attributes: vec![AttributeSnapshot {
            attribute_id: 0,
            base: 20.0,
            modifiers,
        }],
    })
    .unwrap();
    assert!(decode_play_clientbound(attr_id, &attr_payload).is_ok());
}

#[test]
fn play_serverbound_inventory_packet_ids_match_vanilla_26_1_registration_order() {
    assert_eq!(ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED, 3);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_BUTTON_CLICK, 17);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_CLICK, 18);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_CLOSE, 19);
    assert_eq!(ids::play::SERVERBOUND_CONTAINER_SLOT_STATE_CHANGED, 20);
    assert_eq!(ids::play::SERVERBOUND_DEBUG_SUBSCRIPTION_REQUEST, 23);
    assert_eq!(ids::play::SERVERBOUND_EDIT_BOOK, 24);
    assert_eq!(ids::play::SERVERBOUND_RENAME_ITEM, 48);
    assert_eq!(ids::play::SERVERBOUND_SEEN_ADVANCEMENTS, 50);
    assert_eq!(ids::play::SERVERBOUND_SELECT_TRADE, 51);
    assert_eq!(ids::play::SERVERBOUND_SET_BEACON, 52);
    assert_eq!(ids::play::SERVERBOUND_SET_CREATIVE_MODE_SLOT, 56);
    assert_eq!(ids::play::SERVERBOUND_SIGN_UPDATE, 61);
}

#[test]
fn play_serverbound_interaction_packet_ids_match_vanilla_26_1_registration_order() {
    assert_eq!(ids::play::SERVERBOUND_ATTACK, 1);
    assert_eq!(ids::play::SERVERBOUND_BLOCK_ENTITY_TAG_QUERY, 2);
    assert_eq!(ids::play::SERVERBOUND_CHANGE_DIFFICULTY, 4);
    assert_eq!(ids::play::SERVERBOUND_CHANGE_GAME_MODE, 5);
    assert_eq!(ids::play::SERVERBOUND_CHAT_ACK, 6);
    assert_eq!(ids::play::SERVERBOUND_CHAT_COMMAND, 7);
    assert_eq!(ids::play::SERVERBOUND_CHAT_COMMAND_SIGNED, 8);
    assert_eq!(ids::play::SERVERBOUND_CHAT, 9);
    assert_eq!(ids::play::SERVERBOUND_ENTITY_TAG_QUERY, 25);
    assert_eq!(ids::play::SERVERBOUND_INTERACT, 26);
    assert_eq!(ids::play::SERVERBOUND_LOCK_DIFFICULTY, 29);
    assert_eq!(ids::play::SERVERBOUND_PADDLE_BOAT, 35);
    assert_eq!(ids::play::SERVERBOUND_PICK_ITEM_FROM_ENTITY, 37);
    assert_eq!(ids::play::SERVERBOUND_PING_REQUEST, 38);
    assert_eq!(ids::play::SERVERBOUND_SPECTATE_ENTITY, 62);
    assert_eq!(ids::play::SERVERBOUND_TELEPORT_TO_ENTITY, 64);
}

#[test]
fn play_serverbound_player_state_packet_ids_match_vanilla_26_1_registration_order() {
    assert_eq!(ids::play::SERVERBOUND_PLACE_RECIPE, 39);
    assert_eq!(ids::play::SERVERBOUND_PLAYER_ABILITIES, 40);
    assert_eq!(ids::play::SERVERBOUND_PLAYER_ACTION, 41);
    assert_eq!(ids::play::SERVERBOUND_PLAYER_COMMAND, 42);
    assert_eq!(ids::play::SERVERBOUND_PLAYER_INPUT, 43);
    assert_eq!(ids::play::SERVERBOUND_PLAYER_LOADED, 44);
    assert_eq!(ids::play::SERVERBOUND_RECIPE_BOOK_CHANGE_SETTINGS, 46);
    assert_eq!(ids::play::SERVERBOUND_RECIPE_BOOK_SEEN_RECIPE, 47);
}

#[test]
fn encodes_container_inventory_packets() {
    let (id, payload) = encode_play_container_button_click(ContainerButtonClick {
        container_id: 7,
        button_id: 2,
    });
    assert_eq!(id, ids::play::SERVERBOUND_CONTAINER_BUTTON_CLICK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert!(decoder.is_empty());

    let mut added_components = BTreeMap::new();
    added_components.insert(10, 0x0102_0304);
    let mut removed_components = BTreeSet::new();
    removed_components.insert(20);
    let mut changed_slots = BTreeMap::new();
    changed_slots.insert(
        5,
        HashedStack::Item(HashedItemStack {
            item_id: 42,
            count: 64,
            components: HashedComponentPatch {
                added_components,
                removed_components,
            },
        }),
    );
    let (id, payload) = encode_play_container_click(ContainerClick {
        container_id: 7,
        state_id: 33,
        slot_num: 5,
        button_num: 1,
        input: ContainerInput::Pickup,
        changed_slots,
        carried_item: HashedStack::empty(),
    })
    .unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_CONTAINER_CLICK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert_eq!(decoder.read_var_i32().unwrap(), 33);
    assert_eq!(decoder.read_i16().unwrap(), 5);
    assert_eq!(decoder.read_i8().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_i16().unwrap(), 5);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_var_i32().unwrap(), 64);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 10);
    assert_eq!(decoder.read_i32().unwrap(), 0x0102_0304);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 20);
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_container_close(ContainerCloseRequest { container_id: 7 });
    assert_eq!(id, ids::play::SERVERBOUND_CONTAINER_CLOSE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_container_slot_state_changed(ContainerSlotStateChanged {
        slot_id: 12,
        container_id: 7,
        new_state: true,
    });
    assert_eq!(id, ids::play::SERVERBOUND_CONTAINER_SLOT_STATE_CHANGED);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 12);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn container_click_rejects_vanilla_bounded_collections_before_encoding() {
    let click = |changed_slots, carried_item| ContainerClick {
        container_id: 7,
        state_id: 33,
        slot_num: 5,
        button_num: 0,
        input: ContainerInput::Pickup,
        changed_slots,
        carried_item,
    };

    let changed_slots = (0..=128).map(|slot| (slot, HashedStack::empty())).collect();
    assert!(encode_play_container_click(click(changed_slots, HashedStack::empty())).is_err());

    let added_components = (0..=256).map(|id| (id, id)).collect();
    let oversized_added = HashedStack::Item(HashedItemStack {
        item_id: 42,
        count: 1,
        components: HashedComponentPatch {
            added_components,
            removed_components: BTreeSet::new(),
        },
    });
    assert!(encode_play_container_click(click(BTreeMap::new(), oversized_added)).is_err());

    let removed_components = (0..=256).collect();
    let oversized_removed = HashedStack::Item(HashedItemStack {
        item_id: 42,
        count: 1,
        components: HashedComponentPatch {
            added_components: BTreeMap::new(),
            removed_components,
        },
    });
    assert!(encode_play_container_click(click(BTreeMap::new(), oversized_removed)).is_err());

    let invalid_item_id = crate::item_types::VANILLA_ITEM_TYPES.len() as i32;
    let invalid_item = HashedStack::Item(HashedItemStack {
        item_id: invalid_item_id,
        count: 1,
        components: HashedComponentPatch::default(),
    });
    let error = encode_play_container_click(click(BTreeMap::new(), invalid_item)).unwrap_err();
    assert!(error
        .to_string()
        .contains(&format!("invalid vanilla item holder id {invalid_item_id}")));
}

#[test]
fn encodes_place_recipe_packet() {
    let (id, payload) = encode_play_place_recipe(PlaceRecipeCommand {
        container_id: 7,
        recipe_index: 123,
        use_max_items: true,
    });
    assert_eq!(id, ids::play::SERVERBOUND_PLACE_RECIPE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_recipe_book_change_settings_packet() {
    let (id, payload) = encode_play_recipe_book_change_settings(RecipeBookChangeSettingsCommand {
        book_type: RecipeBookType::BlastFurnace,
        open: true,
        filtering: false,
    });
    assert_eq!(id, ids::play::SERVERBOUND_RECIPE_BOOK_CHANGE_SETTINGS);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert!(decoder.read_bool().unwrap());
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_recipe_book_seen_recipe_packet() {
    let (id, payload) = encode_play_recipe_book_seen_recipe(RecipeBookSeenRecipeCommand {
        recipe: RecipeDisplayId { index: 321 },
    });
    assert_eq!(id, ids::play::SERVERBOUND_RECIPE_BOOK_SEEN_RECIPE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 321);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_change_difficulty_packet() {
    let (id, payload) = encode_play_change_difficulty(ChangeDifficultyCommand {
        difficulty: Difficulty::Hard,
    });

    assert_eq!(id, ids::play::SERVERBOUND_CHANGE_DIFFICULTY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_change_game_mode_packet() {
    let (id, payload) = encode_play_change_game_mode(ChangeGameModeCommand {
        game_mode: GameType::Spectator,
    });

    assert_eq!(id, ids::play::SERVERBOUND_CHANGE_GAME_MODE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_chat_acknowledgement_packet() {
    let (id, payload) = encode_play_chat_acknowledgement(ChatAcknowledgement { offset: 65 });

    assert_eq!(id, ids::play::SERVERBOUND_CHAT_ACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 65);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_lock_difficulty_packet() {
    let (id, payload) = encode_play_lock_difficulty(LockDifficultyCommand { locked: true });

    assert_eq!(id, ids::play::SERVERBOUND_LOCK_DIFFICULTY);
    let mut decoder = Decoder::new(&payload);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_block_entity_tag_query_packet() {
    let pos = BlockPos {
        x: -5,
        y: 70,
        z: 12,
    };
    let (id, payload) = encode_play_block_entity_tag_query(BlockEntityTagQuery {
        transaction_id: 7,
        pos,
    });

    assert_eq!(id, ids::play::SERVERBOUND_BLOCK_ENTITY_TAG_QUERY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert_eq!(
        decoder.read_i64().unwrap(),
        encode_block_pos(pos.x, pos.y, pos.z)
    );
    assert!(decoder.is_empty());
}

#[test]
fn encodes_entity_tag_query_packet() {
    let (id, payload) = encode_play_entity_tag_query(EntityTagQuery {
        transaction_id: 8,
        entity_id: 1234,
    });

    assert_eq!(id, ids::play::SERVERBOUND_ENTITY_TAG_QUERY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 8);
    assert_eq!(decoder.read_var_i32().unwrap(), 1234);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_edit_book_unsigned_packet() {
    let (id, payload) = encode_play_edit_book(&EditBook {
        slot: 3,
        pages: vec!["page 1".to_string(), "page 2".to_string()],
        title: None,
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_EDIT_BOOK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert_eq!(decoder.read_string(1024).unwrap(), "page 1");
    assert_eq!(decoder.read_string(1024).unwrap(), "page 2");
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_edit_book_signed_packet() {
    let (id, payload) = encode_play_edit_book(&EditBook {
        slot: 5,
        pages: vec!["final page".to_string()],
        title: Some("Field Notes".to_string()),
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_EDIT_BOOK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 5);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_string(1024).unwrap(), "final page");
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_string(32).unwrap(), "Field Notes");
    assert!(decoder.is_empty());
}

#[test]
fn edit_book_rejects_vanilla_collection_and_utf16_limits() {
    let packet = |pages, title| EditBook {
        slot: 3,
        pages,
        title,
    };
    assert!(encode_play_edit_book(&packet(vec![String::new(); 101], None)).is_err());
    assert!(encode_play_edit_book(&packet(vec!["😀".repeat(513)], None)).is_err());
    assert!(encode_play_edit_book(&packet(vec![String::new()], Some("😀".repeat(17)))).is_err());
}

#[test]
fn encodes_rename_item_packet() {
    let (id, payload) = encode_play_rename_item(&RenameItem {
        name: "Sharp Pick".to_string(),
    });

    assert_eq!(id, ids::play::SERVERBOUND_RENAME_ITEM);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "Sharp Pick");
    assert!(decoder.is_empty());
}

#[test]
fn encodes_seen_advancements_opened_tab_packet() {
    let (id, payload) = encode_play_seen_advancements(&SeenAdvancements::OpenedTab {
        tab: "minecraft:story/root".to_string(),
    });

    assert_eq!(id, ids::play::SERVERBOUND_SEEN_ADVANCEMENTS);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:story/root");
    assert!(decoder.is_empty());
}

#[test]
fn encodes_seen_advancements_closed_screen_packet() {
    let (id, payload) = encode_play_seen_advancements(&SeenAdvancements::ClosedScreen);

    assert_eq!(id, ids::play::SERVERBOUND_SEEN_ADVANCEMENTS);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_select_trade_packet() {
    let (id, payload) = encode_play_select_trade(SelectTradeCommand { item: 2 });
    assert_eq!(id, ids::play::SERVERBOUND_SELECT_TRADE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_beacon_packet_with_effects() {
    let (id, payload) = encode_play_set_beacon(SetBeacon {
        primary_effect: Some(1),
        secondary_effect: Some(10),
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_BEACON);
    let mut decoder = Decoder::new(&payload);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 10);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_set_beacon_packet_without_secondary_effect() {
    let (id, payload) = encode_play_set_beacon(SetBeacon {
        primary_effect: Some(3),
        secondary_effect: None,
    })
    .unwrap();

    assert_eq!(id, ids::play::SERVERBOUND_SET_BEACON);
    let mut decoder = Decoder::new(&payload);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn set_beacon_rejects_holders_outside_the_builtin_mob_effect_registry() {
    let invalid_effect_id = crate::mob_effect_types::VANILLA_MOB_EFFECT_TYPES.len() as i32;
    let error = encode_play_set_beacon(SetBeacon {
        primary_effect: Some(1),
        secondary_effect: Some(invalid_effect_id),
    })
    .unwrap_err();

    assert!(error.to_string().contains(&format!(
        "invalid vanilla mob effect holder id {invalid_effect_id}"
    )));
}

#[test]
fn encodes_select_bundle_item_packet() {
    let (id, payload) = encode_play_select_bundle_item(SelectBundleItem {
        slot_id: 7,
        selected_item_index: 2,
    });
    assert_eq!(id, ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED);
    assert_eq!(payload, vec![0x07, 0x02]);
}

#[test]
fn encodes_select_bundle_item_unselect_packet() {
    let (id, payload) = encode_play_select_bundle_item(SelectBundleItem {
        slot_id: 7,
        selected_item_index: -1,
    });
    assert_eq!(id, ids::play::SERVERBOUND_BUNDLE_ITEM_SELECTED);
    assert_eq!(payload, vec![0x07, 0xff, 0xff, 0xff, 0xff, 0x0f]);
}

#[test]
fn encodes_player_input_flags() {
    let (id, payload) = encode_play_player_input(PlayerInput {
        forward: true,
        backward: false,
        left: true,
        right: false,
        jump: true,
        shift: true,
        sprint: false,
    });

    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_INPUT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_u8().unwrap(), 0b0011_0101);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_player_abilities_flying_bit() {
    let (id, payload) = encode_play_player_abilities(PlayerAbilitiesCommand { flying: true });
    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_ABILITIES);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_u8().unwrap(), 0x02);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_player_abilities(PlayerAbilitiesCommand { flying: false });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_u8().unwrap(), 0x00);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_player_command_actions() {
    let (id, payload) = encode_play_player_command(PlayerCommand {
        entity_id: 5,
        action: PlayerCommandAction::StopSleeping,
        data: 0,
    });
    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_COMMAND);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 5);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_player_command(PlayerCommand {
        entity_id: 1234,
        action: PlayerCommandAction::StartSprinting,
        data: 0,
    });
    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_COMMAND);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1234);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_player_command(PlayerCommand {
        entity_id: -7,
        action: PlayerCommandAction::StopSprinting,
        data: 0,
    });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), -7);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_player_command(PlayerCommand {
        entity_id: 99,
        action: PlayerCommandAction::StartRidingJump,
        data: 20,
    });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 99);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(decoder.read_var_i32().unwrap(), 20);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_player_command(PlayerCommand {
        entity_id: 42,
        action: PlayerCommandAction::OpenInventory,
        data: 0,
    });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_var_i32().unwrap(), 5);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_player_command(PlayerCommand {
        entity_id: 42,
        action: PlayerCommandAction::StartFallFlying,
        data: 0,
    });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_var_i32().unwrap(), 6);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_player_action_packet() {
    let (id, payload) = encode_play_player_action(PlayerAction {
        action: PlayerActionKind::StartDestroyBlock,
        pos: BlockPos {
            x: 34,
            y: -12,
            z: -45,
        },
        direction: Direction::North,
        sequence: 7,
    });

    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_ACTION);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        chunks::decode_block_pos(decoder.read_i64().unwrap()),
        BlockPos {
            x: 34,
            y: -12,
            z: -45,
        }
    );
    assert_eq!(decoder.read_u8().unwrap(), 2);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_all_player_action_kinds() {
    let pos = BlockPos {
        x: -12,
        y: 319,
        z: 45,
    };
    for entry in bbb_facts::player_action_kinds::VANILLA_PLAYER_ACTION_KINDS {
        let sequence = entry.ordinal + 10;
        let (id, payload) = encode_play_player_action(PlayerAction {
            action: entry.kind,
            pos,
            direction: Direction::East,
            sequence,
        });

        assert_eq!(id, ids::play::SERVERBOUND_PLAYER_ACTION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), entry.ordinal);
        assert_eq!(
            chunks::decode_block_pos(decoder.read_i64().unwrap()),
            pos,
            "{}",
            entry.constant_name
        );
        assert_eq!(decoder.read_u8().unwrap(), 5, "{}", entry.constant_name);
        assert_eq!(
            decoder.read_var_i32().unwrap(),
            sequence,
            "{}",
            entry.constant_name
        );
        assert!(decoder.is_empty(), "{}", entry.constant_name);
    }
}

#[test]
fn encodes_stab_player_action_packet() {
    let (id, payload) = encode_play_player_action(PlayerAction {
        action: PlayerActionKind::Stab,
        pos: BlockPos { x: 0, y: 0, z: 0 },
        direction: Direction::Down,
        sequence: 0,
    });

    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_ACTION);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 7);
    assert_eq!(
        chunks::decode_block_pos(decoder.read_i64().unwrap()),
        BlockPos { x: 0, y: 0, z: 0 }
    );
    assert_eq!(decoder.read_u8().unwrap(), 0);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_chat_command_packet() {
    let (id, payload) = encode_play_chat_command(&ChatCommand {
        command: "give @p minecraft:stone".to_string(),
    });

    assert_eq!(id, ids::play::SERVERBOUND_CHAT_COMMAND);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(
        decoder.read_string(32767).unwrap(),
        "give @p minecraft:stone"
    );
    assert!(decoder.is_empty());
}

#[test]
fn encodes_signed_chat_command_packet_with_empty_argument_signatures() {
    let mut packet = ChatCommandSigned::unsigned_arguments(
        "say hello",
        1_717_986_918_300,
        0x0102_0304_0506_0708,
    );
    packet.last_seen_messages = LastSeenMessagesUpdate {
        offset: 3,
        acknowledged: (1 << 1) | (1 << 9),
        checksum: 0x42,
    };

    let (id, payload) = encode_play_chat_command_signed(&packet);

    assert_eq!(id, ids::play::SERVERBOUND_CHAT_COMMAND_SIGNED);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "say hello");
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_i64().unwrap(), 0x0102_0304_0506_0708);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(
        decoder.read_exact(3, "last seen bitset").unwrap(),
        &[0x02, 0x02, 0x00]
    );
    assert_eq!(decoder.read_u8().unwrap(), 0x42);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_signed_chat_command_argument_signatures() {
    let packet = ChatCommandSigned {
        command: "tell Steve hello".to_string(),
        timestamp_millis: 1_717_986_918_300,
        salt: 0x0102_0304_0506_0708,
        argument_signatures: ArgumentSignatures {
            entries: vec![ArgumentSignature {
                name: "message".to_string(),
                signature: MessageSignature {
                    bytes: vec![0xab; 256],
                },
            }],
        },
        last_seen_messages: LastSeenMessagesUpdate::default(),
    };

    let (id, payload) = encode_play_chat_command_signed(&packet);

    assert_eq!(id, ids::play::SERVERBOUND_CHAT_COMMAND_SIGNED);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "tell Steve hello");
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_i64().unwrap(), 0x0102_0304_0506_0708);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_string(16).unwrap(), "message");
    assert_eq!(
        decoder.read_exact(256, "argument signature").unwrap(),
        vec![0xab; 256].as_slice()
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        decoder.read_exact(3, "last seen bitset").unwrap(),
        &[0, 0, 0]
    );
    assert_eq!(decoder.read_u8().unwrap(), 1);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_unsigned_chat_message_packet() {
    let (id, payload) = encode_play_chat_message(&ChatMessage::unsigned(
        "hello",
        1_717_986_918_300,
        0x0102_0304_0506_0708,
    ));

    assert_eq!(id, ids::play::SERVERBOUND_CHAT);
    assert_eq!(
        payload,
        vec![
            0x05, b'h', b'e', b'l', b'l', b'o', 0x00, 0x00, 0x01, 0x8f, 0xff, 0xff, 0xff, 0x9c,
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        ]
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(256).unwrap(), "hello");
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_i64().unwrap(), 0x0102_0304_0506_0708);
    assert!(!decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        decoder.read_exact(3, "last seen bitset").unwrap(),
        &[0, 0, 0]
    );
    assert_eq!(decoder.read_u8().unwrap(), 1);
    assert!(decoder.is_empty());
}

#[test]
#[should_panic(expected = "chat message exceeds vanilla maximum of 256 characters")]
fn rejects_oversized_unsigned_chat_message_packet() {
    let _ = encode_play_chat_message(&ChatMessage::unsigned(
        "a".repeat(257),
        1_717_986_918_300,
        0x0102_0304_0506_0708,
    ));
}

#[test]
fn encodes_signed_chat_message_packet() {
    let mut packet = ChatMessage::unsigned("hello", 1_717_986_918_300, 0x0102_0304_0506_0708);
    packet.signature = Some(MessageSignature {
        bytes: vec![0xab; 256],
    });

    let (id, payload) = encode_play_chat_message(&packet);

    assert_eq!(id, ids::play::SERVERBOUND_CHAT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(256).unwrap(), "hello");
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_i64().unwrap(), 0x0102_0304_0506_0708);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(
        decoder.read_exact(256, "chat message signature").unwrap(),
        vec![0xab; 256]
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        decoder.read_exact(3, "last seen bitset").unwrap(),
        &[0, 0, 0]
    );
    assert_eq!(decoder.read_u8().unwrap(), 1);
    assert!(decoder.is_empty());
}

#[test]
#[should_panic(expected = "chat message signature must be exactly 256 bytes")]
fn rejects_invalid_chat_message_signature_length() {
    let mut packet = ChatMessage::unsigned("hello", 1_717_986_918_300, 7);
    packet.signature = Some(MessageSignature {
        bytes: vec![0xab; 255],
    });

    let _ = encode_play_chat_message(&packet);
}

#[test]
fn encodes_chat_message_last_seen_update_bitset_little_endian() {
    let (id, payload) = encode_play_chat_message(&ChatMessage {
        message: "hello".to_string(),
        timestamp_millis: 1_717_986_918_300,
        salt: 0x0102_0304_0506_0708,
        signature: None,
        last_seen_messages: LastSeenMessagesUpdate {
            offset: 3,
            acknowledged: (1 << 0) | (1 << 8) | (1 << 19),
            checksum: 0x7f,
        },
    });

    assert_eq!(id, ids::play::SERVERBOUND_CHAT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(256).unwrap(), "hello");
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_i64().unwrap(), 0x0102_0304_0506_0708);
    assert!(!decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(
        decoder.read_exact(3, "last seen bitset").unwrap(),
        &[0x01, 0x01, 0x08]
    );
    assert_eq!(decoder.read_u8().unwrap(), 0x7f);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_chat_session_update_packet() {
    let session_id = Uuid::from_u128(0x12345678_90ab_cdef_1020_304050607080);
    let (id, payload) = encode_play_chat_session_update(&ChatSessionUpdate {
        session_id,
        expires_at_epoch_millis: 1_717_986_918_300,
        public_key: vec![0x30, 0x82, 0x01, 0x22],
        key_signature: vec![0xab; 256],
    });

    assert_eq!(id, ids::play::SERVERBOUND_CHAT_SESSION_UPDATE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_uuid().unwrap(), session_id);
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert_eq!(decoder.read_var_i32().unwrap(), 4);
    assert_eq!(
        decoder.read_exact(4, "profile public key").unwrap(),
        &[0x30, 0x82, 0x01, 0x22]
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 256);
    assert_eq!(
        decoder
            .read_exact(256, "profile public key signature")
            .unwrap(),
        vec![0xab; 256]
    );
    assert!(decoder.is_empty());
}

#[test]
#[should_panic(expected = "profile public key exceeds vanilla maximum of 512 bytes")]
fn rejects_oversized_chat_session_public_key() {
    let _ = encode_play_chat_session_update(&ChatSessionUpdate {
        session_id: Uuid::nil(),
        expires_at_epoch_millis: 0,
        public_key: vec![0; 513],
        key_signature: Vec::new(),
    });
}

#[test]
#[should_panic(expected = "profile public key signature exceeds vanilla maximum of 4096 bytes")]
fn rejects_oversized_chat_session_key_signature() {
    let _ = encode_play_chat_session_update(&ChatSessionUpdate {
        session_id: Uuid::nil(),
        expires_at_epoch_millis: 0,
        public_key: Vec::new(),
        key_signature: vec![0; 4097],
    });
}

#[test]
fn encodes_entity_interaction_packets() {
    let (id, payload) = encode_play_attack_entity(AttackEntity { entity_id: 123 });
    assert_eq!(id, ids::play::SERVERBOUND_ATTACK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_interact_entity(InteractEntity {
        entity_id: 123,
        hand: InteractionHand::OffHand,
        location: Vec3d::default(),
        using_secondary_action: true,
    });
    assert_eq!(id, ids::play::SERVERBOUND_INTERACT);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_u8().unwrap(), 0);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_interact_entity(InteractEntity {
        entity_id: 5,
        hand: InteractionHand::MainHand,
        location: Vec3d {
            x: 1.0,
            y: 0.0,
            z: -1.0,
        },
        using_secondary_action: false,
    });
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 5);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        decoder.read_exact(6, "lp_vec3").unwrap(),
        &[0xf1, 0xff, 0x00, 0x00, 0xff, 0xff]
    );
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_jigsaw_generate_packet() {
    let packet = JigsawGenerate {
        pos: BlockPos {
            x: -12,
            y: 64,
            z: 9,
        },
        levels: 20,
        keep_jigsaws: true,
    };
    let (id, payload) = encode_play_jigsaw_generate(packet);

    assert_eq!(id, ids::play::SERVERBOUND_JIGSAW_GENERATE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(
        decoder.read_i64().unwrap(),
        encode_block_pos(packet.pos.x, packet.pos.y, packet.pos.z)
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 20);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_swing_hand() {
    let (id, payload) = encode_play_swing(InteractionHand::MainHand);
    assert_eq!(id, ids::play::SERVERBOUND_SWING);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    let (_, payload) = encode_play_swing(InteractionHand::OffHand);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_use_item_on_packet() {
    let (id, payload) = encode_play_use_item_on(UseItemOn {
        hand: InteractionHand::MainHand,
        hit: BlockHitResult {
            pos: BlockPos {
                x: 34,
                y: -12,
                z: -45,
            },
            direction: Direction::Up,
            cursor_x: 0.25,
            cursor_y: 1.0,
            cursor_z: 0.75,
            inside: true,
            world_border_hit: false,
        },
        sequence: 11,
    });

    assert_eq!(id, ids::play::SERVERBOUND_USE_ITEM_ON);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        chunks::decode_block_pos(decoder.read_i64().unwrap()),
        BlockPos {
            x: 34,
            y: -12,
            z: -45,
        }
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_f32().unwrap(), 0.25);
    assert_eq!(decoder.read_f32().unwrap(), 1.0);
    assert_eq!(decoder.read_f32().unwrap(), 0.75);
    assert!(decoder.read_bool().unwrap());
    assert!(!decoder.read_bool().unwrap());
    assert_eq!(decoder.read_var_i32().unwrap(), 11);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_use_item_packet() {
    let (id, payload) = encode_play_use_item(UseItem {
        hand: InteractionHand::OffHand,
        sequence: 12,
        y_rot: 180.0,
        x_rot: -30.0,
    });

    assert_eq!(id, ids::play::SERVERBOUND_USE_ITEM);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_var_i32().unwrap(), 12);
    assert_eq!(decoder.read_f32().unwrap(), 180.0);
    assert_eq!(decoder.read_f32().unwrap(), -30.0);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_pick_item_from_block_packet() {
    let (id, payload) = encode_play_pick_item_from_block(PickItemFromBlock {
        pos: BlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        include_data: true,
    });

    assert_eq!(id, ids::play::SERVERBOUND_PICK_ITEM_FROM_BLOCK);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(
        chunks::decode_block_pos(decoder.read_i64().unwrap()),
        BlockPos {
            x: -5,
            y: 70,
            z: 12,
        }
    );
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_pick_item_from_entity_packet() {
    let (id, payload) = encode_play_pick_item_from_entity(PickItemFromEntity {
        entity_id: 123,
        include_data: true,
    });

    assert_eq!(id, ids::play::SERVERBOUND_PICK_ITEM_FROM_ENTITY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_sign_update_packet() {
    let (id, payload) = encode_play_sign_update(&SignUpdate {
        pos: BlockPos {
            x: -5,
            y: 70,
            z: 12,
        },
        is_front_text: false,
        lines: [
            "line 0".to_string(),
            "line 1".to_string(),
            "line 2".to_string(),
            "line 3".to_string(),
        ],
    });

    assert_eq!(id, ids::play::SERVERBOUND_SIGN_UPDATE);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(
        chunks::decode_block_pos(decoder.read_i64().unwrap()),
        BlockPos {
            x: -5,
            y: 70,
            z: 12,
        }
    );
    assert!(!decoder.read_bool().unwrap());
    assert_eq!(decoder.read_string(384).unwrap(), "line 0");
    assert_eq!(decoder.read_string(384).unwrap(), "line 1");
    assert_eq!(decoder.read_string(384).unwrap(), "line 2");
    assert_eq!(decoder.read_string(384).unwrap(), "line 3");
    assert!(decoder.is_empty());
}

#[test]
fn encodes_spectator_entity_packets() {
    let (id, payload) = encode_play_spectate_entity(SpectateEntity { entity_id: 1234 });
    assert_eq!(id, ids::play::SERVERBOUND_SPECTATE_ENTITY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 1234);
    assert!(decoder.is_empty());

    let uuid = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
    let (id, payload) = encode_play_teleport_to_entity(TeleportToEntity { uuid });
    assert_eq!(id, ids::play::SERVERBOUND_TELEPORT_TO_ENTITY);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_uuid().unwrap(), uuid);
    assert!(decoder.is_empty());
}

#[test]
fn decodes_award_stats() {
    assert_eq!(ids::play::CLIENTBOUND_AWARD_STATS, 3);

    let mut payload = Encoder::new();
    payload.write_var_i32(2);
    payload.write_var_i32(0);
    payload.write_var_i32(34);
    payload.write_var_i32(12);
    payload.write_var_i32(8);
    payload.write_var_i32(5);
    payload.write_var_i32(1);
    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_AWARD_STATS, &payload.into_inner()).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::AwardStats(AwardStats {
            stats: vec![
                StatUpdate {
                    stat_type_id: 0,
                    value_id: 34,
                    amount: 12,
                },
                StatUpdate {
                    stat_type_id: 8,
                    value_id: 5,
                    amount: 1,
                },
            ],
        })
    );

    let mut trailing = Encoder::new();
    trailing.write_var_i32(0);
    trailing.write_u8(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_AWARD_STATS, &trailing.into_inner())
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after award stats packet")
    );
}

#[test]
fn decodes_and_encodes_move_vehicle_packets() {
    let mut payload = Encoder::new();
    payload.write_f64(12.5);
    payload.write_f64(65.25);
    payload.write_f64(-8.75);
    payload.write_f32(135.0);
    payload.write_f32(-12.5);
    let payload = payload.into_inner();
    assert_eq!(payload.len(), 32);

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_MOVE_VEHICLE, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::MoveVehicle(MoveVehicle {
            position: Vec3d {
                x: 12.5,
                y: 65.25,
                z: -8.75,
            },
            y_rot: 135.0,
            x_rot: -12.5,
        })
    );

    let (id, payload) = encode_play_move_vehicle(MoveVehicleCommand {
        position: Vec3d {
            x: 12.5,
            y: 65.25,
            z: -8.75,
        },
        y_rot: 135.0,
        x_rot: -12.5,
        on_ground: true,
    });
    assert_eq!(id, ids::play::SERVERBOUND_MOVE_VEHICLE);
    assert_eq!(id, 34);
    assert_eq!(payload.len(), 33);
    assert_eq!(payload[32], 1);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_f64().unwrap(), 12.5);
    assert_eq!(decoder.read_f64().unwrap(), 65.25);
    assert_eq!(decoder.read_f64().unwrap(), -8.75);
    assert_eq!(decoder.read_f32().unwrap(), 135.0);
    assert_eq!(decoder.read_f32().unwrap(), -12.5);
    assert!(decoder.read_bool().unwrap());
    assert!(decoder.is_empty());

    let mut trailing = Encoder::new();
    trailing.write_f64(12.5);
    trailing.write_f64(65.25);
    trailing.write_f64(-8.75);
    trailing.write_f32(135.0);
    trailing.write_f32(-12.5);
    trailing.write_u8(0);
    let error =
        decode_play_clientbound(ids::play::CLIENTBOUND_MOVE_VEHICLE, &trailing.into_inner())
            .unwrap_err();
    assert!(error
        .to_string()
        .contains("trailing bytes after move vehicle packet"));
}

#[test]
fn encodes_paddle_boat() {
    let (id, payload) = encode_play_paddle_boat(PaddleBoat {
        left: true,
        right: false,
    });
    assert_eq!(id, ids::play::SERVERBOUND_PADDLE_BOAT);
    assert_eq!(id, 35);

    let mut decoder = Decoder::new(&payload);
    assert!(decoder.read_bool().unwrap());
    assert!(!decoder.read_bool().unwrap());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_play_ping_request() {
    let (id, payload) = encode_play_ping_request(1_717_986_918_300);
    assert_eq!(id, ids::play::SERVERBOUND_PING_REQUEST);
    assert_eq!(id, 38);
    assert_eq!(payload.len(), 8);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i64().unwrap(), 1_717_986_918_300);
    assert!(decoder.is_empty());
}

#[test]
fn encodes_play_connection_lifecycle_packets() {
    let (id, payload) = encode_play_keep_alive(-123_456_789_012_345);
    assert_eq!(id, ids::play::SERVERBOUND_KEEP_ALIVE);
    assert_eq!(id, 28);
    assert_eq!(payload.len(), 8);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i64().unwrap(), -123_456_789_012_345);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_pong(-1_234_567);
    assert_eq!(id, ids::play::SERVERBOUND_PONG);
    assert_eq!(id, 45);
    assert_eq!(payload.len(), 4);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_i32().unwrap(), -1_234_567);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_configuration_acknowledged();
    assert_eq!(id, ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED);
    assert_eq!(id, 16);
    assert!(payload.is_empty());
}

#[test]
fn decodes_play_keep_alive_and_ping_strictly() {
    let mut keep_alive = Encoder::new();
    keep_alive.write_i64(-123_456_789_012_345);
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_KEEP_ALIVE, &keep_alive.into_inner())
            .unwrap(),
        PlayClientbound::KeepAlive {
            id: -123_456_789_012_345,
        }
    );
    let mut keep_alive = Encoder::new();
    keep_alive.write_i64(-123_456_789_012_345);
    keep_alive.write_u8(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_KEEP_ALIVE, &keep_alive.into_inner())
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after play keep alive packet")
    );

    let mut ping = Encoder::new();
    ping.write_i32(-1_234_567);
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_PING, &ping.into_inner()).unwrap(),
        PlayClientbound::Ping { id: -1_234_567 }
    );
    let mut ping = Encoder::new();
    ping.write_i32(-1_234_567);
    ping.write_u8(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_PING, &ping.into_inner())
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after play ping packet")
    );
}

#[test]
fn encodes_player_loaded() {
    let (id, payload) = encode_play_player_loaded();
    assert_eq!(id, ids::play::SERVERBOUND_PLAYER_LOADED);
    assert_eq!(id, 44);
    assert!(payload.is_empty());
}

#[test]
fn decodes_chunk_batch_and_encodes_client_play_status_packets() {
    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_CHUNK_BATCH_START, &[]).unwrap();
    assert_eq!(packet, PlayClientbound::ChunkBatchStart);

    let mut payload = Encoder::new();
    payload.write_var_i32(9);
    let packet = decode_play_clientbound(
        ids::play::CLIENTBOUND_CHUNK_BATCH_FINISHED,
        &payload.into_inner(),
    )
    .unwrap();
    assert_eq!(
        packet,
        PlayClientbound::ChunkBatchFinished { batch_size: 9 }
    );

    let error =
        decode_play_clientbound(ids::play::CLIENTBOUND_CHUNK_BATCH_START, &[0]).unwrap_err();
    assert!(error
        .to_string()
        .contains("trailing bytes after chunk batch start packet"));

    let mut trailing_finished = Encoder::new();
    trailing_finished.write_var_i32(9);
    trailing_finished.write_u8(0);
    let error = decode_play_clientbound(
        ids::play::CLIENTBOUND_CHUNK_BATCH_FINISHED,
        &trailing_finished.into_inner(),
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("trailing bytes after chunk batch finished packet"));

    let (id, payload) = encode_play_chunk_batch_received(9.0);
    assert_eq!(id, ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED);
    assert_eq!(payload.len(), 4);
    assert_eq!(Decoder::new(&payload).read_f32().unwrap(), 9.0);

    let (id, payload) = encode_play_client_tick_end();
    assert_eq!(id, ids::play::SERVERBOUND_CLIENT_TICK_END);
    assert!(payload.is_empty());

    let (id, payload) = encode_play_client_information_default().unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_CLIENT_INFORMATION);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(16).unwrap(), "en_us");
    assert_eq!(decoder.read_i8().unwrap(), 10);
}

#[test]
fn encodes_client_information_from_struct() {
    let information = ClientInformation {
        language: "zh_cn".to_string(),
        view_distance: 12,
        chat_visibility: ClientChatVisibility::System,
        chat_colors: false,
        displayed_skin_parts: 0x15,
        main_hand: ClientMainHand::Left,
        text_filtering_enabled: true,
        allows_listing: true,
        particle_status: ClientParticleStatus::Minimal,
    };

    let (id, payload) = encode_client_information(&information).unwrap();
    assert_eq!(id, ids::configuration::SERVERBOUND_CLIENT_INFORMATION);
    assert_client_information_payload(&payload, &information);

    let (id, play_payload) = encode_play_client_information(&information).unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_CLIENT_INFORMATION);
    assert_eq!(play_payload, payload);

    let mut maximum_language = information.clone();
    maximum_language.language = "a".repeat(16);
    encode_client_information(&maximum_language).unwrap();
    encode_play_client_information(&maximum_language).unwrap();

    let mut oversized_language = information;
    oversized_language.language = "a".repeat(17);
    assert!(encode_client_information(&oversized_language).is_err());
    assert!(encode_play_client_information(&oversized_language).is_err());
    oversized_language.language = "😀".repeat(9);
    assert!(encode_client_information(&oversized_language).is_err());
    assert!(encode_play_client_information(&oversized_language).is_err());
}

#[test]
fn encodes_configuration_brand_custom_payload() {
    let (id, payload) = encode_configuration_brand_custom_payload("bbb-client").unwrap();
    assert_eq!(id, ids::configuration::SERVERBOUND_CUSTOM_PAYLOAD);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:brand");
    assert_eq!(decoder.read_string(32767).unwrap(), "bbb-client");
    assert!(decoder.is_empty());

    encode_configuration_brand_custom_payload(&"a".repeat(32_767)).unwrap();
    assert!(encode_configuration_brand_custom_payload(&"a".repeat(32_768)).is_err());
    assert!(encode_configuration_brand_custom_payload(&"😀".repeat(16_384)).is_err());
}

#[test]
fn encodes_play_brand_custom_payload() {
    let (id, payload) = encode_play_custom_payload(&ServerboundCustomPayload::Brand {
        brand: "bbb-client".to_string(),
    })
    .unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_CUSTOM_PAYLOAD);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:brand");
    assert_eq!(decoder.read_string(32767).unwrap(), "bbb-client");
    assert!(decoder.is_empty());
}

#[test]
fn encodes_custom_click_action_in_configuration_and_play() {
    let action = CustomClickAction {
        id: "dialog/submit".to_string(),
        payload: Some(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "byte".into(),
                value: NbtSummaryValue::Byte(-7),
            },
            NbtSummaryEntry {
                name: "short".into(),
                value: NbtSummaryValue::Short(-300),
            },
            NbtSummaryEntry {
                name: "int".into(),
                value: NbtSummaryValue::Int(70_000),
            },
            NbtSummaryEntry {
                name: "long".into(),
                value: NbtSummaryValue::Long(-9_000_000_000),
            },
            NbtSummaryEntry {
                name: "float".into(),
                value: NbtSummaryValue::Float(1.25f32.to_bits()),
            },
            NbtSummaryEntry {
                name: "double".into(),
                value: NbtSummaryValue::Double((-2.5f64).to_bits()),
            },
            NbtSummaryEntry {
                name: "bytes".into(),
                value: NbtSummaryValue::ByteArray(vec![-1, 0, 1]),
            },
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::String("A\0😀".into()),
            },
            NbtSummaryEntry {
                name: "values".into(),
                value: NbtSummaryValue::List(vec![
                    NbtSummaryValue::Int(3),
                    NbtSummaryValue::Int(5),
                ]),
            },
            NbtSummaryEntry {
                name: "nested".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "ok".into(),
                    value: NbtSummaryValue::Byte(1),
                }]),
            },
            NbtSummaryEntry {
                name: "ints".into(),
                value: NbtSummaryValue::IntArray(vec![-1, 0, 1]),
            },
            NbtSummaryEntry {
                name: "longs".into(),
                value: NbtSummaryValue::LongArray(vec![-1, 0, 1]),
            },
        ])),
    };
    let (configuration_id, configuration_payload) =
        encode_configuration_custom_click_action(&action).unwrap();
    let (play_id, play_payload) = encode_play_custom_click_action(&action).unwrap();

    assert_eq!(
        configuration_id,
        ids::configuration::SERVERBOUND_CUSTOM_CLICK_ACTION
    );
    assert_eq!(play_id, ids::play::SERVERBOUND_CUSTOM_CLICK_ACTION);
    assert_eq!(configuration_payload, play_payload);

    let mut decoder = Decoder::new(&play_payload);
    assert_eq!(
        decoder.read_string(32_767).unwrap(),
        "minecraft:dialog/submit"
    );
    let tag_len = decoder.read_len().unwrap();
    let mut tag_decoder = Decoder::new(decoder.read_exact(tag_len, "custom click tag").unwrap());
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut tag_decoder).unwrap(),
        action.payload
    );
    assert!(tag_decoder.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn encodes_absent_custom_click_action_tag_as_length_prefixed_end_tag() {
    let action = CustomClickAction {
        id: "bbb:none".to_string(),
        payload: None,
    };
    let (_, payload) = encode_play_custom_click_action(&action).unwrap();
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32_767).unwrap(), "bbb:none");
    assert_eq!(decoder.read_len().unwrap(), 1);
    assert_eq!(decoder.read_u8().unwrap(), 0);
    assert!(decoder.is_empty());
}

#[test]
fn rejects_invalid_oversized_and_excessively_deep_custom_click_actions() {
    let invalid_id = CustomClickAction {
        id: "BBB:submit".to_string(),
        payload: None,
    };
    assert!(encode_play_custom_click_action(&invalid_id).is_err());

    let oversized = CustomClickAction {
        id: "bbb:submit".to_string(),
        payload: Some(NbtSummaryValue::ByteArray(vec![
            0;
            super::client_common::MAX_CUSTOM_CLICK_ACTION_TAG_BYTES
        ])),
    };
    assert!(matches!(
        encode_play_custom_click_action(&oversized),
        Err(ProtocolError::PacketTooLarge(_, max))
            if max == super::client_common::MAX_CUSTOM_CLICK_ACTION_TAG_BYTES
    ));

    let mut too_deep = NbtSummaryValue::Compound(Vec::new());
    for _ in 0..super::client_common::MAX_CUSTOM_CLICK_ACTION_TAG_DEPTH {
        too_deep = NbtSummaryValue::Compound(vec![NbtSummaryEntry {
            name: "child".into(),
            value: too_deep,
        }]);
    }
    let too_deep = CustomClickAction {
        id: "bbb:submit".to_string(),
        payload: Some(too_deep),
    };
    assert!(encode_configuration_custom_click_action(&too_deep)
        .unwrap_err()
        .to_string()
        .contains("network nbt exceeded max depth"));
}

#[test]
fn encodes_heterogeneous_custom_click_action_list_with_vanilla_wrappers() {
    let action = CustomClickAction {
        id: "bbb:submit".to_string(),
        payload: Some(NbtSummaryValue::List(vec![
            NbtSummaryValue::Int(1),
            NbtSummaryValue::String("two".into()),
            NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::String("three".into()),
            }]),
        ])),
    };
    let (_, payload) = encode_play_custom_click_action(&action).unwrap();
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_string(32_767).unwrap(), "bbb:submit");
    let tag_len = decoder.read_len().unwrap();
    let mut tag_decoder = Decoder::new(decoder.read_exact(tag_len, "custom click tag").unwrap());
    assert_eq!(
        decode_nbt_summary_from_decoder(&mut tag_decoder).unwrap(),
        action.payload
    );
    assert!(tag_decoder.is_empty());
    assert!(decoder.is_empty());
}

#[test]
fn play_custom_payload_rejects_invalid_ids_and_oversized_brand() {
    assert!(
        encode_play_custom_payload(&ServerboundCustomPayload::Unknown {
            id: "BBB:debug".to_string(),
            raw_payload: Vec::new(),
        })
        .is_err()
    );
    assert!(
        encode_play_custom_payload(&ServerboundCustomPayload::Brand {
            brand: "😀".repeat(16_384),
        })
        .is_err()
    );
}

#[test]
fn encodes_play_debug_subscription_request_as_validated_deterministic_set() {
    use crate::debug_subscription_types::VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID;

    let (id, payload) = encode_play_debug_subscription_request(&[
        15,
        VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID,
        8,
        15,
    ])
    .unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_DEBUG_SUBSCRIPTION_REQUEST);

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 8);
    assert_eq!(decoder.read_var_i32().unwrap(), 15);
    assert!(decoder.is_empty());

    let (id, payload) = encode_play_debug_subscription_request(&[]).unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_DEBUG_SUBSCRIPTION_REQUEST);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert!(decoder.is_empty());

    assert!(encode_play_debug_subscription_request(&[-1])
        .unwrap_err()
        .to_string()
        .contains("invalid debug subscription id -1"));
    assert!(encode_play_debug_subscription_request(&[16])
        .unwrap_err()
        .to_string()
        .contains("invalid debug subscription id 16"));
}

#[test]
fn rejects_oversized_play_unknown_custom_payload() {
    let err = encode_play_custom_payload(&ServerboundCustomPayload::Unknown {
        id: "bbb:debug".to_string(),
        raw_payload: vec![0; 32768],
    })
    .unwrap_err();

    assert!(matches!(err, ProtocolError::PacketTooLarge(32768, 32767)));
}

#[test]
fn encodes_configuration_accept_code_of_conduct_as_unit_packet() {
    let (id, payload) = encode_configuration_accept_code_of_conduct();
    assert_eq!(id, ids::configuration::SERVERBOUND_ACCEPT_CODE_OF_CONDUCT);
    assert!(payload.is_empty());
}

#[test]
fn decodes_transfer_packets_in_configuration_and_play() {
    let mut payload = Encoder::new();
    payload.write_string("next.example.com");
    payload.write_var_i32(25566);
    let payload = payload.into_inner();

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_TRANSFER, &payload)
            .unwrap(),
        ConfigurationClientbound::Transfer(Transfer {
            host: "next.example.com".into(),
            port: 25566,
        })
    );
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_TRANSFER, &payload).unwrap(),
        PlayClientbound::Transfer(Transfer {
            host: "next.example.com".into(),
            port: 25566,
        })
    );

    let mut trailing = payload;
    trailing.push(0);
    for error in [
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_TRANSFER, &trailing)
            .unwrap_err(),
        decode_play_clientbound(ids::play::CLIENTBOUND_TRANSFER, &trailing).unwrap_err(),
    ] {
        assert!(error
            .to_string()
            .contains("trailing bytes after transfer packet"));
    }
}

#[test]
fn decodes_and_encodes_cookie_packets() {
    let mut request_payload = Encoder::new();
    request_payload.write_string("session");
    let request_payload = request_payload.into_inner();

    assert_eq!(
        decode_login_clientbound(ids::login::CLIENTBOUND_COOKIE_REQUEST, &request_payload).unwrap(),
        LoginClientbound::CookieRequest(CookieRequest {
            key: "minecraft:session".to_string(),
        })
    );
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_COOKIE_REQUEST,
            &request_payload,
        )
        .unwrap(),
        ConfigurationClientbound::CookieRequest(CookieRequest {
            key: "minecraft:session".to_string(),
        })
    );
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_COOKIE_REQUEST, &request_payload).unwrap(),
        PlayClientbound::CookieRequest(CookieRequest {
            key: "minecraft:session".to_string(),
        })
    );

    let mut trailing_request = request_payload.clone();
    trailing_request.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_COOKIE_REQUEST, &trailing_request)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after cookie request packet")
    );

    let mut store_payload = Encoder::new();
    store_payload.write_string("session");
    store_payload.write_var_i32(3);
    store_payload.write_bytes(&[1, 2, 3]);
    let store_payload = store_payload.into_inner();

    let store = StoreCookie {
        key: "minecraft:session".to_string(),
        payload: vec![1, 2, 3],
    };
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_STORE_COOKIE,
            &store_payload,
        )
        .unwrap(),
        ConfigurationClientbound::StoreCookie(store.clone())
    );
    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_STORE_COOKIE, &store_payload).unwrap(),
        PlayClientbound::StoreCookie(store)
    );

    let mut trailing_store = store_payload.clone();
    trailing_store.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_STORE_COOKIE, &trailing_store)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after store cookie packet")
    );

    let (id, payload) = encode_login_cookie_response("bbb:session", Some(&[1, 2, 3])).unwrap();
    assert_eq!(id, ids::login::SERVERBOUND_COOKIE_RESPONSE);
    assert_cookie_response_payload(&payload, "bbb:session", Some(&[1, 2, 3]));

    let (id, payload) = encode_configuration_cookie_response("bbb:missing", None).unwrap();
    assert_eq!(id, ids::configuration::SERVERBOUND_COOKIE_RESPONSE);
    assert_cookie_response_payload(&payload, "bbb:missing", None);

    let (id, payload) = encode_play_cookie_response("bbb:session", Some(&[4, 5])).unwrap();
    assert_eq!(id, ids::play::SERVERBOUND_COOKIE_RESPONSE);
    assert_cookie_response_payload(&payload, "bbb:session", Some(&[4, 5]));

    let maximum_payload = vec![7; 5120];
    let (_, payload) = encode_login_cookie_response("session", Some(&maximum_payload)).unwrap();
    assert_cookie_response_payload(&payload, "minecraft:session", Some(&maximum_payload));
    assert!(encode_configuration_cookie_response("bbb:session", Some(&vec![7; 5121])).is_err());
    assert!(encode_play_cookie_response("INVALID:session", None).is_err());
}

#[test]
fn login_unknown_packet_preserves_unsupported_diagnostics() {
    assert_eq!(
        decode_login_clientbound(0x7f, &[1, 2, 3]).unwrap(),
        LoginClientbound::Unknown {
            packet_id: 0x7f,
            len: 3,
        }
    );
}

#[test]
fn rejects_invalid_cookie_keys() {
    let mut request_payload = Encoder::new();
    request_payload.write_string("bbb:Session");
    let request_payload = request_payload.into_inner();

    let err = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_COOKIE_REQUEST,
        &request_payload,
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));

    let mut store_payload = Encoder::new();
    store_payload.write_string("bbb:Session");
    store_payload.write_var_i32(1);
    store_payload.write_u8(0);
    let store_payload = store_payload.into_inner();

    let err =
        decode_play_clientbound(ids::play::CLIENTBOUND_STORE_COOKIE, &store_payload).unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));
}

#[test]
fn configuration_store_cookie_enforces_vanilla_payload_limit() {
    let mut payload = Encoder::new();
    payload.write_string("bbb:session");
    payload.write_var_i32(5120);
    payload.write_bytes(&vec![7; 5120]);
    assert!(matches!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_STORE_COOKIE,
            &payload.into_inner()
        )
        .unwrap(),
        ConfigurationClientbound::StoreCookie(StoreCookie { payload, .. }) if payload.len() == 5120
    ));

    let mut payload = Encoder::new();
    payload.write_string("bbb:session");
    payload.write_var_i32(5121);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_STORE_COOKIE,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::PacketTooLarge(5121, 5120)));
}

#[test]
fn decodes_configuration_common_packets() {
    let mut custom_payload = Encoder::new();
    custom_payload.write_string("minecraft:brand");
    custom_payload.write_string("bbb-client");
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_CUSTOM_PAYLOAD,
            &custom_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::CustomPayload(CustomPayload {
            id: "minecraft:brand".to_string(),
            payload: CustomPayloadBody::Brand {
                brand: "bbb-client".to_string(),
            },
        })
    );

    let mut unknown_custom_payload = Encoder::new();
    unknown_custom_payload.write_string("bbb:configuration_probe");
    unknown_custom_payload.write_bytes(&[0xaa, 0xbb, 0xcc]);
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_CUSTOM_PAYLOAD,
            &unknown_custom_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::CustomPayload(CustomPayload {
            id: "bbb:configuration_probe".to_string(),
            payload: CustomPayloadBody::Unknown {
                raw_payload: vec![0xaa, 0xbb, 0xcc],
            },
        })
    );

    let mut oversized_custom_payload = Encoder::new();
    oversized_custom_payload.write_string("bbb:oversized");
    oversized_custom_payload.write_bytes(&vec![0; 1_048_577]);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_CUSTOM_PAYLOAD,
        &oversized_custom_payload.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(1_048_577, 1_048_576)
    ));

    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_DISCONNECT,
            &nbt_string_root("Configuration closed"),
        )
        .unwrap(),
        ConfigurationClientbound::Disconnect {
            reason: "Configuration closed".to_string(),
            styled_reason: vec![StyledTextRun {
                text: "Configuration closed".to_string(),
                style: ComponentStyle::default(),
            }],
            raw_reason: nbt_string_root("Configuration closed"),
        }
    );

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_RESET_CHAT, &[]).unwrap(),
        ConfigurationClientbound::ResetChat
    );

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_FINISH_CONFIGURATION, &[])
            .unwrap(),
        ConfigurationClientbound::Finish
    );
    let mut keep_alive = Encoder::new();
    keep_alive.write_i64(-123_456_789);
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_KEEP_ALIVE,
            &keep_alive.into_inner()
        )
        .unwrap(),
        ConfigurationClientbound::KeepAlive { id: -123_456_789 }
    );
    let mut trailing_keep_alive = Encoder::new();
    trailing_keep_alive.write_i64(-123_456_789);
    trailing_keep_alive.write_u8(0);
    assert!(decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_KEEP_ALIVE,
        &trailing_keep_alive.into_inner(),
    )
    .unwrap_err()
    .to_string()
    .contains("trailing bytes after configuration keep alive packet"));
    let mut ping = Encoder::new();
    ping.write_i32(0x1234_5678);
    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_PING, &ping.into_inner())
            .unwrap(),
        ConfigurationClientbound::Ping { id: 0x1234_5678 }
    );
    let mut trailing_ping = Encoder::new();
    trailing_ping.write_i32(0x1234_5678);
    trailing_ping.write_u8(0);
    assert!(decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_PING,
        &trailing_ping.into_inner(),
    )
    .unwrap_err()
    .to_string()
    .contains("trailing bytes after configuration ping packet"));

    assert_eq!(
        encode_configuration_finish(),
        (
            ids::configuration::SERVERBOUND_FINISH_CONFIGURATION,
            Vec::new()
        )
    );
    let (packet_id, payload) = encode_configuration_keep_alive(-123_456_789);
    assert_eq!(packet_id, ids::configuration::SERVERBOUND_KEEP_ALIVE);
    assert_eq!(Decoder::new(&payload).read_i64().unwrap(), -123_456_789);
    let (packet_id, payload) = encode_configuration_pong(0x1234_5678);
    assert_eq!(packet_id, ids::configuration::SERVERBOUND_PONG);
    assert_eq!(Decoder::new(&payload).read_i32().unwrap(), 0x1234_5678);

    let pack_id = Uuid::from_u128(0x11111111_2222_3333_4444_555555555555);
    let mut push_payload = Encoder::new();
    push_payload.write_uuid(pack_id);
    push_payload.write_string("https://example.invalid/server-pack.zip");
    push_payload.write_string("0123456789abcdef0123456789abcdef01234567");
    push_payload.write_bool(true);
    push_payload.write_bool(false);
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_RESOURCE_PACK_PUSH,
            &push_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::ResourcePackPush(ResourcePackPush {
            id: pack_id,
            url: "https://example.invalid/server-pack.zip".to_string(),
            hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
            required: true,
            prompt: None,
        })
    );

    let mut pop_payload = Encoder::new();
    pop_payload.write_bool(true);
    pop_payload.write_uuid(pack_id);
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_RESOURCE_PACK_POP,
            &pop_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::ResourcePackPop(ResourcePackPop { id: Some(pack_id) })
    );
    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_RESOURCE_PACK_POP, &[0])
            .unwrap(),
        ConfigurationClientbound::ResourcePackPop(ResourcePackPop { id: None })
    );

    let mut oversized_hash = Encoder::new();
    oversized_hash.write_uuid(pack_id);
    oversized_hash.write_string("https://example.invalid/server-pack.zip");
    oversized_hash.write_string(&"a".repeat(41));
    oversized_hash.write_bool(false);
    oversized_hash.write_bool(false);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_RESOURCE_PACK_PUSH,
        &oversized_hash.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::StringTooLong { max: 40, .. }
    ));

    let mut features_payload = Encoder::new();
    features_payload.write_var_i32(3);
    features_payload.write_string("minecraft:update_1_21");
    features_payload.write_string("vanilla");
    features_payload.write_string("minecraft:vanilla");
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_UPDATE_ENABLED_FEATURES,
            &features_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::UpdateEnabledFeatures(UpdateEnabledFeatures {
            features: vec![
                "minecraft:update_1_21".to_string(),
                "minecraft:vanilla".to_string(),
            ],
        })
    );

    let mut hostile_features = Encoder::new();
    hostile_features.write_var_i32(i32::MAX);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_UPDATE_ENABLED_FEATURES,
        &hostile_features.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::PacketTooLarge(_, _)));

    let mut truncated_features = Encoder::new();
    truncated_features.write_var_i32(16);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_UPDATE_ENABLED_FEATURES,
        &truncated_features.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::UnexpectedEof { .. }));

    let mut invalid_features_payload = Encoder::new();
    invalid_features_payload.write_var_i32(1);
    invalid_features_payload.write_string("minecraft:Vanilla");
    let err = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_UPDATE_ENABLED_FEATURES,
        &invalid_features_payload.into_inner(),
    )
    .unwrap_err();
    assert!(err.to_string().contains("invalid resource location"));

    let mut known_packs_payload = Encoder::new();
    known_packs_payload.write_var_i32(1);
    known_packs_payload.write_string("minecraft");
    known_packs_payload.write_string("core");
    known_packs_payload.write_string("26.1");
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_SELECT_KNOWN_PACKS,
            &known_packs_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::SelectKnownPacks {
            known_packs: vec![KnownPack {
                namespace: "minecraft".to_string(),
                id: "core".to_string(),
                version: "26.1".to_string(),
            }],
        }
    );

    let mut hostile_known_packs = Encoder::new();
    hostile_known_packs.write_var_i32(i32::MAX);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_SELECT_KNOWN_PACKS,
        &hostile_known_packs.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::PacketTooLarge(_, _)));

    let mut truncated_known_packs = Encoder::new();
    truncated_known_packs.write_var_i32(16);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_SELECT_KNOWN_PACKS,
        &truncated_known_packs.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(error, ProtocolError::UnexpectedEof { .. }));

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_CLEAR_DIALOG, &[])
            .unwrap(),
        ConfigurationClientbound::ClearDialog
    );
    let error =
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_CLEAR_DIALOG, &[0])
            .unwrap_err();
    assert!(error
        .to_string()
        .contains("trailing bytes after clear dialog packet"));

    let mut show_dialog_payload = Encoder::new();
    show_dialog_payload.write_bytes(&nbt_compound_with_string("type", "minecraft:notice"));
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_SHOW_DIALOG,
            &show_dialog_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::ShowDialog(ShowDialog {
            dialog: DialogHolder::ContextFree {
                dialog: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::String("minecraft:notice".into()),
                }]),
            },
        })
    );

    let mut code_payload = Encoder::new();
    code_payload.write_string("Keep the server friendly.");
    assert_eq!(
        decode_configuration_clientbound(
            ids::configuration::CLIENTBOUND_CODE_OF_CONDUCT,
            &code_payload.into_inner(),
        )
        .unwrap(),
        ConfigurationClientbound::CodeOfConduct {
            text: "Keep the server friendly.".to_string(),
        }
    );

    let mut oversized_code = Encoder::new();
    oversized_code.write_string(&"x".repeat(32_768));
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_CODE_OF_CONDUCT,
        &oversized_code.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::StringTooLong { max: 32_767, .. }
    ));
}

#[test]
fn encodes_selected_known_packs_with_vanilla_limit_and_field_order() {
    let packs = vec![
        KnownPack {
            namespace: "minecraft".to_string(),
            id: "core".to_string(),
            version: "26.1".to_string(),
        },
        KnownPack {
            namespace: "example".to_string(),
            id: "feature".to_string(),
            version: "1".to_string(),
        },
    ];
    let (id, payload) = encode_select_known_packs(&packs).unwrap();
    assert_eq!(id, ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS);
    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    for pack in &packs {
        assert_eq!(decoder.read_string(32_767).unwrap(), pack.namespace);
        assert_eq!(decoder.read_string(32_767).unwrap(), pack.id);
        assert_eq!(decoder.read_string(32_767).unwrap(), pack.version);
    }
    assert!(decoder.is_empty());

    let sixty_four = vec![packs[0].clone(); 64];
    encode_select_known_packs(&sixty_four).unwrap();
    let sixty_five = vec![packs[0].clone(); 65];
    assert!(encode_select_known_packs(&sixty_five)
        .unwrap_err()
        .to_string()
        .contains("maximum 64"));

    let oversized = [KnownPack {
        namespace: "😀".repeat(16_384),
        id: "core".to_string(),
        version: "26.1".to_string(),
    }];
    assert!(encode_select_known_packs(&oversized)
        .unwrap_err()
        .to_string()
        .contains("maximum is 32767"));
}

#[test]
fn decodes_configuration_registry_data_entries() {
    let raw_plains = nbt_compound_with_string("name", "minecraft:plains");
    let raw_temperature = nbt_list_of_floats(&[0.8, 0.4]);
    let mut payload = Encoder::new();
    payload.write_string("worldgen/biome");
    payload.write_var_i32(3);
    payload.write_string("plains");
    payload.write_bool(true);
    payload.write_bytes(&raw_plains);
    payload.write_string("minecraft:the_void");
    payload.write_bool(false);
    payload.write_string("bbb:raw_numeric_root");
    payload.write_bool(true);
    payload.write_bytes(&raw_temperature);
    let payload = payload.into_inner();

    assert_eq!(
        decode_configuration_clientbound(ids::configuration::CLIENTBOUND_REGISTRY_DATA, &payload)
            .unwrap(),
        ConfigurationClientbound::RegistryData(RegistryData {
            registry: "minecraft:worldgen/biome".to_string(),
            entries: vec![
                RegistryDataEntry {
                    id: "minecraft:plains".to_string(),
                    raw_data: Some(raw_plains),
                },
                RegistryDataEntry {
                    id: "minecraft:the_void".to_string(),
                    raw_data: None,
                },
                RegistryDataEntry {
                    id: "bbb:raw_numeric_root".to_string(),
                    raw_data: Some(raw_temperature),
                },
            ],
            raw_payload_len: payload.len(),
        })
    );

    for (registry, id, expected_registry, expected_id) in [
        (
            ":worldgen/biome",
            ":plains",
            "minecraft:worldgen/biome",
            "minecraft:plains",
        ),
        (
            "bbb_0-.:folder/path_0-.",
            "bbb_0-.:entry/path_0-.",
            "bbb_0-.:folder/path_0-.",
            "bbb_0-.:entry/path_0-.",
        ),
    ] {
        let mut payload = Encoder::new();
        payload.write_string(registry);
        payload.write_var_i32(1);
        payload.write_string(id);
        payload.write_bool(false);
        let payload = payload.into_inner();
        assert_eq!(
            decode_configuration_clientbound(
                ids::configuration::CLIENTBOUND_REGISTRY_DATA,
                &payload
            )
            .unwrap(),
            ConfigurationClientbound::RegistryData(RegistryData {
                registry: expected_registry.to_string(),
                entries: vec![RegistryDataEntry {
                    id: expected_id.to_string(),
                    raw_data: None,
                }],
                raw_payload_len: payload.len(),
            })
        );
    }
}

#[test]
fn rejects_configuration_registry_data_entry_count_before_allocation() {
    let mut payload = Encoder::new();
    payload.write_string("minecraft:worldgen/biome");
    payload.write_var_i32(131_073);
    let error = decode_configuration_clientbound(
        ids::configuration::CLIENTBOUND_REGISTRY_DATA,
        &payload.into_inner(),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::PacketTooLarge(131_073, 131_072)
    ));
}

#[test]
fn edit_book_raw_utf16_enforces_unit_limits_before_encoding() {
    let mut packet = EditBook {
        slot: 0,
        pages: vec![vec![0xd800; 1024]],
        title: Some(vec![0xdc00; 32]),
    };
    assert!(encode_play_edit_book_utf16(&packet).is_ok());
    packet.pages[0].push(0xd800);
    assert!(encode_play_edit_book_utf16(&packet).is_err());
    packet.pages[0].pop();
    packet.title.as_mut().unwrap().push(0xdc00);
    assert!(encode_play_edit_book_utf16(&packet).is_err());
    packet.title = None;
    packet.pages = vec![vec![]; 101];
    assert!(encode_play_edit_book_utf16(&packet).is_err());
}
