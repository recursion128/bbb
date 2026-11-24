use super::*;

#[test]
fn hash_ops_crc32c_int_matches_guava_for_map_id() {
    assert_eq!(crc32c(b"123456789"), 0xe306_9283);
    assert_eq!(hash_ops_crc32c_int(7), -1_726_626_450);
}

#[test]
fn inventory_state_serde_shape_survives_world_store_json_round_trip() {
    let mut store = WorldStore::new();
    store.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: 36,
        item: item_stack(42, 3),
    });
    store.apply_set_cursor_item(ProtocolSetCursorItem {
        item: item_stack(7, 1),
    });
    store.apply_open_screen(ProtocolOpenScreen {
        container_id: 5,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });

    let value = serde_json::to_value(&store).unwrap();
    let inventory = value
        .get("inventory")
        .and_then(|value| value.as_object())
        .expect("world store serializes the sub-store under the `inventory` key");
    let mut keys: Vec<&str> = inventory.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "cursor_item",
            "inventory_menu",
            "local_inventory_open",
            "open_container",
            "open_container_revision",
            "player_slots",
        ]
    );
    let open_container = inventory
        .get("open_container")
        .and_then(|value| value.as_object())
        .expect("open container serializes as an object");
    let mut container_keys: Vec<&str> = open_container.keys().map(String::as_str).collect();
    container_keys.sort_unstable();
    assert_eq!(
        container_keys,
        [
            "container_id",
            "data_values",
            "menu_type_id",
            "merchant_offers",
            "mount",
            "slots",
            "state_id",
            "title",
        ]
    );

    let restored: WorldStore = serde_json::from_value(value).unwrap();
    assert_eq!(restored.inventory(), store.inventory());
}

#[test]
fn persistent_optional_item_stack_codec_roundtrips_and_filters_transient_values() {
    use bbb_protocol::{
        data_component_types::{
            VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID, VANILLA_DATA_COMPONENT_LORE_TYPE_ID,
        },
        ComponentStyle, NbtSummaryValue, StyledTextRun,
    };

    let store = WorldStore::new();
    let stack = ProtocolItemStackSummary {
        item_id: Some(1),
        count: 32,
        component_patch: ProtocolDataComponentPatchSummary {
            added_type_ids: vec![
                VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
            ],
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_LORE_TYPE_ID],
            max_stack_size: Some(64),
            custom_name: Some("Saved".to_string()),
            custom_name_styled: Some(vec![StyledTextRun {
                text: "Saved".to_string(),
                style: ComponentStyle::default(),
            }]),
            custom_name_nbt: Some(NbtSummaryValue::string("Saved")),
            ..ProtocolDataComponentPatchSummary::default()
        }
        .into(),
    };

    let encoded = store.persistent_item_stack_to_nbt(&stack).unwrap();
    assert_eq!(
        store.persistent_item_stack_from_nbt(&encoded).unwrap(),
        stack
    );
    assert_eq!(
        store
            .persistent_item_stack_to_nbt(&ProtocolItemStackSummary::empty())
            .unwrap(),
        NbtSummaryValue::Compound(Vec::new())
    );

    let mut mixed = stack.clone();
    mixed
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID);
    mixed.component_patch.creative_slot_lock = true;
    assert_eq!(store.persistent_item_stack_to_nbt(&mixed).unwrap(), encoded);

    let transient = ProtocolItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: ProtocolDataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID],
            creative_slot_lock: true,
            ..ProtocolDataComponentPatchSummary::default()
        }
        .into(),
    };
    let transient_encoded = store.persistent_item_stack_to_nbt(&transient).unwrap();
    assert_eq!(
        store
            .persistent_item_stack_from_nbt(&transient_encoded)
            .unwrap(),
        ProtocolItemStackSummary {
            component_patch: ProtocolDataComponentPatchSummary::default().into(),
            ..transient
        }
    );
    assert!(store
        .persistent_item_stack_to_nbt(&ProtocolItemStackSummary {
            item_id: Some(1),
            count: 100,
            ..ProtocolItemStackSummary::empty()
        })
        .is_err());
}

#[test]
fn world_store_item_hash_delegates_to_registry_codec_with_session_registries() {
    use bbb_protocol::{
        data_component_types::{
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
            VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
            VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID, VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
        },
        packets::{
            AdventureModePredicateDataSummary, BlockPredicateSummary, BlocksAttacksSummary,
            ConsumableSummary, ConsumeEffectSummary, DamageReductionSummary,
            DataComponentPredicateTypeSummary, HolderSetSummary, ItemDamageFunctionSummary,
            ItemEnchantmentSummary, ItemUseAnimationSummary, KineticWeaponConditionSummary,
            KineticWeaponSummary, MobEffectDetailsSummary, MobEffectInstanceSummary,
            PartialDataComponentPredicateSummary, PotionContentsSummary, SoundEventSummary,
            TrimMaterialSummary, TrimPatternSummary,
        },
        NbtSummaryEntry, NbtSummaryValue,
    };
    use bbb_registry_codec::item_hash::{
        hash_boolean, hash_component, hash_empty_map, hash_int, hash_map, hash_record, hash_string,
    };

    let mut store = WorldStore::new();
    let mut enchantments = crate::RegistryContentState::new("minecraft:enchantment");
    enchantments.entries = vec![crate::RegistryPacketEntry::stub("example:sharp")];
    store
        .registries
        .contents
        .insert("minecraft:enchantment".to_string(), enchantments);
    store.record_registry_entries(
        "minecraft:dialog",
        0,
        vec![crate::RegistryPacketEntry::with_known_pack_data_for_test(
            "example:notice",
            br#"{"type":"minecraft:notice","title":"Referenced"}"#.to_vec(),
        )],
    );
    let ctx = InventoryCtx {
        items: &store.items,
        local_player: &store.local_player,
        entities: &store.entities,
        recipes: &store.recipes,
        registries: &store.registries,
    };

    // Official Vanilla 26.1 digests via `WorldStore::registries` (`ItemHashLookup`).
    let haste_id = store
        .registries
        .holder_id_by_resource_id("minecraft:mob_effect", "minecraft:haste")
        .unwrap();
    let speed_id = store
        .registries
        .holder_id_by_resource_id("minecraft:mob_effect", "minecraft:speed")
        .unwrap();
    let poison_id = store
        .registries
        .holder_id_by_resource_id("minecraft:mob_effect", "minecraft:poison")
        .unwrap();
    let drink_sound_id = store
        .registries
        .holder_id_by_resource_id("minecraft:sound_event", "minecraft:entity.generic.drink")
        .unwrap();
    let stone_block_id = store
        .registries
        .holder_id_by_resource_id("minecraft:block", "minecraft:stone")
        .unwrap();

    let patch = ProtocolDataComponentPatchSummary {
        added_type_ids: vec![
            VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
            VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
            VANILLA_DATA_COMPONENT_TRIM_TYPE_ID,
            VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
        ],
        enchantments: vec![ItemEnchantmentSummary {
            holder_id: 0,
            level: 3,
        }],
        potion_contents: PotionContentsSummary {
            custom_effects: vec![MobEffectInstanceSummary {
                effect_id: haste_id,
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
            ..Default::default()
        },
        food_nutrition: Some(7),
        food_saturation_bits: Some((-0.0_f32).to_bits()),
        food_can_always_eat: true,
        consumable: Some(ConsumableSummary {
            consume_seconds: 0.375,
            animation: ItemUseAnimationSummary::Drink,
        }),
        consumable_sound: Some(SoundEventSummary {
            registry_id: Some(drink_sound_id),
            sound_id: None,
            fixed_range_bits: None,
        }),
        consumable_has_consume_particles: Some(false),
        consumable_effects: vec![
            ConsumeEffectSummary::ApplyEffects {
                effects: vec![
                    MobEffectInstanceSummary {
                        effect_id: speed_id,
                        amplifier: 200,
                        duration: 321,
                        ambient: true,
                        show_particles: false,
                        show_icon: true,
                        hidden_effect: Some(Box::new(MobEffectDetailsSummary {
                            amplifier: 7,
                            duration: 42,
                            ambient: false,
                            show_particles: true,
                            show_icon: false,
                            hidden_effect: None,
                        })),
                    },
                    MobEffectInstanceSummary {
                        effect_id: poison_id,
                        amplifier: 1,
                        duration: -1,
                        ambient: false,
                        show_particles: true,
                        show_icon: true,
                        hidden_effect: None,
                    },
                ],
                probability_bits: 0.375_f32.to_bits(),
            },
            ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids: vec![poison_id, speed_id],
                effect_tag: None,
            },
            ConsumeEffectSummary::ClearAllEffects,
            ConsumeEffectSummary::TeleportRandomly {
                diameter_bits: 8.5_f32.to_bits(),
            },
            ConsumeEffectSummary::PlaySound {
                sound: SoundEventSummary {
                    registry_id: None,
                    sound_id: Some("bbb:consume".into()),
                    fixed_range_bits: Some(24.5_f32.to_bits()),
                },
            },
        ],
        blocks_attacks: Some(BlocksAttacksSummary {
            block_delay_seconds_bits: 0.375_f32.to_bits(),
            disable_cooldown_scale_bits: 1.25_f32.to_bits(),
            damage_reductions: vec![
                DamageReductionSummary {
                    horizontal_blocking_angle_bits: 135.0_f32.to_bits(),
                    damage_types: None,
                    base_bits: (-0.0_f32).to_bits(),
                    factor_bits: 0.625_f32.to_bits(),
                },
                DamageReductionSummary {
                    horizontal_blocking_angle_bits: 45.0_f32.to_bits(),
                    damage_types: None,
                    base_bits: 2.5_f32.to_bits(),
                    factor_bits: 0.25_f32.to_bits(),
                },
            ],
            item_damage: ItemDamageFunctionSummary {
                threshold_bits: 4.0_f32.to_bits(),
                base_bits: (-0.0_f32).to_bits(),
                factor_bits: 0.75_f32.to_bits(),
            },
            bypassed_by: None,
            block_sound: None,
            disable_sound: None,
        }),
        kinetic_weapon: Some(KineticWeaponSummary {
            contact_cooldown_ticks: 17,
            delay_ticks: 2,
            dismount_conditions: Some(KineticWeaponConditionSummary {
                max_duration_ticks: 11,
                min_speed_bits: (-0.0_f32).to_bits(),
                min_relative_speed_bits: 0.25_f32.to_bits(),
            }),
            knockback_conditions: None,
            damage_conditions: None,
            forward_movement_bits: (-0.0_f32).to_bits(),
            damage_multiplier_bits: 1.5_f32.to_bits(),
            sound: None,
            hit_sound: None,
        }),
        armor_trim_material_direct: Some(Box::new(TrimMaterialSummary {
            asset_name: "opal".to_string(),
            override_armor_assets: BTreeMap::new(),
            description: "Opal".to_string(),
            description_styled: None,
            description_nbt: Some(Box::new(NbtSummaryValue::string("Opal"))),
        })),
        armor_trim_pattern_direct: Some(Box::new(TrimPatternSummary {
            asset_id: "example:spiral".to_string(),
            description: "Spiral".to_string(),
            description_styled: None,
            description_nbt: Some(Box::new(NbtSummaryValue::string("Spiral"))),
            decal: false,
        })),
        can_place_on_persistent: Some(AdventureModePredicateDataSummary {
            predicates: vec![BlockPredicateSummary {
                blocks: Some(HolderSetSummary {
                    registry_ids: vec![stone_block_id],
                    tag: None,
                }),
                partial_components: vec![PartialDataComponentPredicateSummary {
                    predicate_type: DataComponentPredicateTypeSummary::AnyComponent(
                        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID,
                    ),
                    wire_value: Some(NbtSummaryValue::Compound(Vec::new())),
                }],
                ..Default::default()
            }],
        }),
        ..Default::default()
    };

    let hashed = hashed_component_patch_from_summary(ctx, &patch).unwrap();
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID],
        hash_map(vec![(hash_string("example:sharp"), hash_int(3))]) as i32
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID] as u32,
        0xd688_8158
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_FOOD_TYPE_ID] as u32,
        0x9758_c10d
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID] as u32,
        0x507c_982f
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID] as u32,
        0xfea9_40b3
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID] as u32,
        0xab78_a599
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_TRIM_TYPE_ID],
        hash_record([
            (
                "material",
                hash_record([
                    ("asset_name", hash_string("opal")),
                    ("description", hash_string("Opal")),
                ]),
            ),
            (
                "pattern",
                hash_record([
                    ("asset_id", hash_string("example:spiral")),
                    ("description", hash_string("Spiral")),
                    ("decal", hash_boolean(false)),
                ]),
            ),
        ]) as i32
    );
    assert_eq!(
        hashed.added_components[&VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID],
        hash_record([
            ("blocks", hash_string("minecraft:stone")),
            (
                "predicates",
                hash_map(vec![(hash_string("minecraft:lock"), hash_empty_map())]),
            ),
        ]) as i32
    );

    // Synchronized `minecraft:dialog` registry resolution works via `RegistrySet`.
    let named_dialog = NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::string("Open"),
        },
        NbtSummaryEntry {
            name: "click_event".into(),
            value: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "action".into(),
                    value: NbtSummaryValue::string("show_dialog"),
                },
                NbtSummaryEntry {
                    name: "dialog".into(),
                    value: NbtSummaryValue::string("example:notice"),
                },
            ]),
        },
    ]);
    assert!(hash_component(ctx.registries, &named_dialog).is_some());

    // Transient components stay unhashable / server-authoritative.
    let transient = ProtocolDataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID],
        creative_slot_lock: true,
        ..Default::default()
    };
    assert_eq!(hashed_component_patch_from_summary(ctx, &transient), None);
}
