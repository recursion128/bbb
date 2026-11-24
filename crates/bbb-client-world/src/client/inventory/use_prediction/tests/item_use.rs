use super::*;

#[test]
fn generic_empty_and_componentless_item_use_predict_pass() {
    let mut world = WorldStore::new();
    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    );

    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    );
}

#[test]
fn generic_use_preserves_component_order_and_unknown_item_boundary() {
    let mut world = WorldStore::new();
    world.set_default_item_component_codec_values(BTreeMap::from([(
        VANILLA_ITEM_STONE_ID,
        BTreeMap::from([(
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
            serde_json::json!({}),
        )]),
    )]));
    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(true)
        ))
    );

    world.set_default_item_component_codec_values(BTreeMap::from([(
        VANILLA_ITEM_STONE_ID,
        BTreeMap::from([
            (
                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                serde_json::json!({"slot": "chest"}),
            ),
            (
                VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
                serde_json::json!({}),
            ),
        ]),
    )]));
    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::MissingComponentValue(VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
    );

    set_main_hand(&mut world, 99_999);
    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::UnknownItemType(Some(99_999))
    );
}

#[test]
fn generic_consumable_predicts_food_gate_and_using_duration() {
    let mut world = WorldStore::new();
    world.items.commit_item_component_defaults(BTreeMap::from([(
        VANILLA_ITEM_APPLE_ID,
        ProtocolDataComponentPatchSummary {
            added_type_ids: vec![
                VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            ],
            consumable: Some(ConsumableSummary {
                consume_seconds: 1.6,
                ..ConsumableSummary::default()
            }),
            consumable_sound: Some(bbb_protocol::packets::SoundEventSummary {
                registry_id: None,
                sound_id: Some("example:crunch".into()),
                fixed_range_bits: None,
            }),
            food_nutrition: Some(4),
            food_saturation_bits: Some(2.4_f32.to_bits()),
            food_can_always_eat: false,
            ..Default::default()
        },
    )]));
    set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);

    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
    );
    world.apply_player_health(PlayerHealth {
        health: 20.0,
        food: 10,
        saturation: 5.0,
    });
    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(true)
        ))
    );
    world.set_local_player_pose(Default::default());
    world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    world.advance_local_using_item_ticks_with_actions(8);
    assert!(world
        .take_pending_local_interaction_sound_actions()
        .is_empty());
    world.advance_local_using_item_ticks_with_actions(5);
    let actions = world.take_pending_local_interaction_sound_actions();
    assert_eq!(actions.len(), 2);
    let Some(crate::LocalInteractionSoundEffect::Positioned(sound)) =
        world.apply_local_interaction_sound_action(actions[0], &mut Default::default())
    else {
        panic!("expected the apple's consume sound");
    };
    assert_eq!(sound.sound.location.as_deref(), Some("example:crunch"));

    let mut instant = stack(VANILLA_ITEM_APPLE_ID);
    instant
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID);
    instant.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 0.04,
        ..Default::default()
    });
    let hand = InteractionHand::OffHand;
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: instant,
    });
    assert_eq!(
        world.predict_local_item_use(hand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::Consume { hand }),
            particle_action: Some(LocalInteractionParticleAction::Consume { hand, count: 16 }),
            ..LocalInteractionSuccess::consume(false)
        }))
    );
}

#[test]
fn place_on_water_places_lily_pad_with_inventory_sound_and_rollback() {
    let water_pos = BlockPos { x: 8, y: 7, z: 8 };
    let placed_pos = BlockPos { y: 8, ..water_pos };
    let air = block_state_id("minecraft:air", []);
    let water = block_state_id("minecraft:water", [("level", "0")]);
    let lily_pad = block_state_id("minecraft:lily_pad", []);
    let mut world = tool_use_world(air);
    install_place_on_water_support_tags(
        &mut world,
        "minecraft:supports_lily_pad",
        vec![2],
        Vec::new(),
    );
    set_block(&mut world, water_pos, water);
    let mut held = stack(VANILLA_ITEM_LILY_PAD_ID);
    held.count = 2;
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: held,
    });

    assert_eq!(
        world.predict_local_item_use_with_pov(
            InteractionHand::MainHand,
            Some(item_use_block_hit(water_pos, ProtocolDirection::Up)),
        ),
        LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::PlaceOnWaterBlockItem)
    );
    assert_eq!(world.probe_block(placed_pos).unwrap().block_state_id, air);
    assert_eq!(player_inventory_item(&world, 0).count, 2);

    assert_eq!(
        world.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(water_pos, ProtocolDirection::Up)),
            40,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::block_place(
                placed_pos,
                "minecraft:block.lily_pad.place",
                1.0,
                0.8,
            )
        ))
    );
    assert_eq!(
        world.probe_block(placed_pos).unwrap().block_state_id,
        lily_pad
    );
    assert_eq!(player_inventory_item(&world, 0).count, 1);
    assert_eq!(world.local_block_predictions().len(), 1);
    assert_eq!(world.local_block_predictions()[0].sequence, 40);
    assert_eq!(world.sync_ended_local_block_predictions(40), 1);
    assert_eq!(world.probe_block(placed_pos).unwrap().block_state_id, air);

    let mut random = crate::LevelEventSoundRandomState::default();
    let effect = world
        .apply_local_interaction_sound_action(
            LocalInteractionSoundAction::BlockPlace {
                pos: placed_pos,
                sound_resource_id: "minecraft:block.lily_pad.place",
                volume_bits: 1.0_f32.to_bits(),
                pitch_bits: 0.8_f32.to_bits(),
            },
            &mut random,
        )
        .unwrap();
    let crate::LocalInteractionSoundEffect::Positioned(sound) = effect else {
        panic!("lily pad placement sound should be positioned");
    };
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.lily_pad.place")
    );
    assert_eq!(sound.source, "block");
    assert_eq!((sound.volume, sound.pitch), (1.0, 0.8));
}

#[test]
fn place_on_water_uses_live_support_tags_and_exact_collision_shapes() {
    let support_pos = BlockPos { x: 8, y: 7, z: 8 };
    let placed_pos = BlockPos {
        y: support_pos.y + 1,
        ..support_pos
    };
    let air = block_state_id("minecraft:air", []);
    let water = block_state_id("minecraft:water", [("level", "0")]);
    let ice = block_state_id("minecraft:ice", []);
    let frogspawn = block_state_id("minecraft:frogspawn", []);

    let mut missing_tags = tool_use_world(air);
    set_block(&mut missing_tags, support_pos, water);
    set_main_hand(&mut missing_tags, VANILLA_ITEM_LILY_PAD_ID);
    assert_eq!(
        missing_tags.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(support_pos, ProtocolDirection::Up)),
            41,
            false,
        ),
        LocalItemUsePrediction::MissingRegistryTag("minecraft:supports_lily_pad")
    );
    assert_eq!(
        missing_tags.probe_block(placed_pos).unwrap().block_state_id,
        air
    );

    let mut ice_support = tool_use_world(air);
    install_place_on_water_support_tags(
        &mut ice_support,
        "minecraft:supports_lily_pad",
        Vec::new(),
        vec![VANILLA_BLOCK_ICE_ID],
    );
    set_block(&mut ice_support, support_pos, ice);
    set_main_hand(&mut ice_support, VANILLA_ITEM_LILY_PAD_ID);
    assert!(matches!(
        ice_support.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(support_pos, ProtocolDirection::Up)),
            42,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));

    let mut blocked_lily = tool_use_world(air);
    install_place_on_water_support_tags(
        &mut blocked_lily,
        "minecraft:supports_lily_pad",
        vec![2],
        Vec::new(),
    );
    set_block(&mut blocked_lily, support_pos, water);
    set_main_hand(&mut blocked_lily, VANILLA_ITEM_LILY_PAD_ID);
    add_entity_at(
        &mut blocked_lily,
        90,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: f64::from(placed_pos.x) + 0.5,
            y: f64::from(placed_pos.y),
            z: f64::from(placed_pos.z) + 0.5,
        },
    );
    assert_eq!(
        blocked_lily.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(support_pos, ProtocolDirection::Up)),
            43,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
    );

    let mut non_blocking_item = tool_use_world(air);
    install_place_on_water_support_tags(
        &mut non_blocking_item,
        "minecraft:supports_lily_pad",
        vec![2],
        Vec::new(),
    );
    set_block(&mut non_blocking_item, support_pos, water);
    set_main_hand(&mut non_blocking_item, VANILLA_ITEM_LILY_PAD_ID);
    add_entity_at(
        &mut non_blocking_item,
        91,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: f64::from(placed_pos.x) + 0.5,
            y: f64::from(placed_pos.y),
            z: f64::from(placed_pos.z) + 0.5,
        },
    );
    assert!(matches!(
        non_blocking_item.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(support_pos, ProtocolDirection::Up)),
            44,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));

    let mut marker_armor_stand = tool_use_world(air);
    install_place_on_water_support_tags(
        &mut marker_armor_stand,
        "minecraft:supports_lily_pad",
        vec![2],
        Vec::new(),
    );
    set_block(&mut marker_armor_stand, support_pos, water);
    set_main_hand(&mut marker_armor_stand, VANILLA_ITEM_LILY_PAD_ID);
    add_entity_at(
        &mut marker_armor_stand,
        92,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: f64::from(placed_pos.x) + 0.5,
            y: f64::from(placed_pos.y),
            z: f64::from(placed_pos.z) + 0.5,
        },
    );
    let armor_stand_accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    )
    .unwrap();
    assert!(marker_armor_stand.apply_set_entity_data(SetEntityData {
        id: 92,
        values: vec![EntityDataValue {
            data_id: armor_stand_accessors.client_flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(armor_stand_accessors.marker_mask),
        }],
    }));
    assert!(matches!(
        marker_armor_stand.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(support_pos, ProtocolDirection::Up)),
            45,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));

    let mut frogspawn_world = tool_use_world(air);
    install_place_on_water_support_tags(
        &mut frogspawn_world,
        "minecraft:supports_frogspawn",
        vec![2],
        Vec::new(),
    );
    set_block(&mut frogspawn_world, support_pos, water);
    set_main_hand(&mut frogspawn_world, VANILLA_ITEM_FROGSPAWN_ID);
    add_entity_at(
        &mut frogspawn_world,
        93,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d {
            x: f64::from(placed_pos.x) + 0.5,
            y: f64::from(placed_pos.y),
            z: f64::from(placed_pos.z) + 0.5,
        },
    );
    assert_eq!(
        frogspawn_world.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(support_pos, ProtocolDirection::Up)),
            46,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::block_place(
                placed_pos,
                "minecraft:block.frogspawn.place",
                1.0,
                0.8,
            )
        ))
    );
    assert_eq!(
        frogspawn_world
            .probe_block(placed_pos)
            .unwrap()
            .block_state_id,
        frogspawn
    );
}

#[test]
fn place_on_water_preserves_replace_clicked_and_empty_fluid_gates() {
    let support_pos = BlockPos { x: 8, y: 7, z: 8 };
    let shifted_pos = BlockPos {
        y: support_pos.y + 1,
        ..support_pos
    };
    let air = block_state_id("minecraft:air", []);
    let water = block_state_id("minecraft:water", [("level", "0")]);
    let one_snow = block_state_id("minecraft:snow", [("layers", "1")]);
    let two_snow = block_state_id("minecraft:snow", [("layers", "2")]);
    let lily_pad = block_state_id("minecraft:lily_pad", []);

    let prepare = |shifted_state| {
        let mut world = tool_use_world(air);
        install_place_on_water_support_tags(
            &mut world,
            "minecraft:supports_lily_pad",
            vec![2],
            Vec::new(),
        );
        set_block(&mut world, support_pos, water);
        set_block(&mut world, shifted_pos, shifted_state);
        set_main_hand(&mut world, VANILLA_ITEM_LILY_PAD_ID);
        world
    };

    let mut one_layer = prepare(one_snow);
    assert!(matches!(
        one_layer.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(support_pos, ProtocolDirection::Up)),
            46,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        one_layer.probe_block(shifted_pos).unwrap().block_state_id,
        lily_pad
    );

    let mut two_layers = prepare(two_snow);
    assert_eq!(
        two_layers.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(support_pos, ProtocolDirection::Up)),
            47,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        two_layers.probe_block(shifted_pos).unwrap().block_state_id,
        two_snow
    );

    let mut occupied_fluid = prepare(water);
    assert_eq!(
        occupied_fluid.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(support_pos, ProtocolDirection::Up)),
            48,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert!(occupied_fluid.local_block_predictions().is_empty());
}

#[test]
fn hanging_painting_uses_placeable_registry_and_applied_variant_dimensions() {
    let clicked = BlockPos { x: 8, y: 7, z: 8 };
    let air = block_state_id("minecraft:air", []);
    let stone = block_state_id("minecraft:stone", []);
    let install_placeable = |world: &mut WorldStore| {
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:painting_variant".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:placeable".to_string(),
                    entries: vec![0],
                }],
            }],
        });
    };
    let prepare = |support| {
        let mut world = tool_use_world(air);
        set_block(&mut world, clicked, support);
        set_main_hand(&mut world, VANILLA_ITEM_PAINTING_ID);
        world
    };

    let mut missing_tag = prepare(stone);
    assert_eq!(
        missing_tag.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            72,
        ),
        LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::HangingEntityItem)
    );

    let mut empty_tag = prepare(stone);
    empty_tag.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:painting_variant".to_string(),
            tags: Vec::new(),
        }],
    });
    assert_eq!(
        empty_tag.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            73,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );

    let mut placeable = prepare(stone);
    install_placeable(&mut placeable);
    assert_eq!(
        placeable.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            74,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(player_inventory_item(&placeable, 0).item_id, None);

    let mut no_support = prepare(air);
    install_placeable(&mut no_support);
    assert_eq!(
        no_support.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            75,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
    assert_eq!(
        player_inventory_item(&no_support, 0).item_id,
        Some(VANILLA_ITEM_PAINTING_ID)
    );

    let mut forced_large = prepare(stone);
    install_placeable(&mut forced_large);
    forced_large.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: Some(VANILLA_ITEM_PAINTING_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID],
                painting_variant_direct: Some(PaintingVariantSummary {
                    width: 2,
                    height: 2,
                    asset_id: "example:large".to_string(),
                    title: None,
                    title_styled: None,
                    title_nbt: None,
                    author: None,
                    author_styled: None,
                    author_nbt: None,
                }),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    });
    assert_eq!(
        forced_large.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            76,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
    assert_eq!(
        player_inventory_item(&forced_large, 0).item_id,
        Some(VANILLA_ITEM_PAINTING_ID)
    );
}

#[test]
fn hanging_overlap_matches_direction_and_same_painting_type_rules() {
    let clicked = BlockPos { x: 8, y: 7, z: 8 };
    let target = BlockPos {
        x: clicked.x + 1,
        ..clicked
    };
    let air = block_state_id("minecraft:air", []);
    let stone = block_state_id("minecraft:stone", []);
    let mut world = tool_use_world(air);
    set_block(&mut world, clicked, stone);
    set_main_hand(&mut world, VANILLA_ITEM_ITEM_FRAME_ID);
    world.apply_add_entity(AddEntity {
        id: 99,
        uuid: Uuid::from_u128(99),
        entity_type_id: VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
        position: Vec3d {
            x: f64::from(target.x),
            y: f64::from(target.y),
            z: f64::from(target.z),
        },
        delta_movement: Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: ProtocolDirection::East.id().into(),
    });
    let accessor = vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
    )
    .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 99,
        values: vec![EntityDataValue {
            data_id: accessor.direction_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
            value: EntityDataValueKind::Direction(i32::from(ProtocolDirection::East.id())),
        }],
    }));

    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            77,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
    assert_eq!(
        player_inventory_item(&world, 0).item_id,
        Some(VANILLA_ITEM_ITEM_FRAME_ID)
    );

    let mut painting_world = tool_use_world(air);
    for y in 7..=8 {
        for z in 7..=8 {
            set_block(&mut painting_world, BlockPos { x: 8, y, z }, stone);
        }
    }
    painting_world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:painting_variant".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:placeable".to_string(),
                entries: vec![0],
            }],
        }],
    });
    painting_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: Some(VANILLA_ITEM_PAINTING_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID],
                painting_variant_direct: Some(PaintingVariantSummary {
                    width: 2,
                    height: 2,
                    asset_id: "example:large".to_string(),
                    title: None,
                    title_styled: None,
                    title_nbt: None,
                    author: None,
                    author_styled: None,
                    author_nbt: None,
                }),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    });
    painting_world.apply_add_entity(AddEntity {
        id: 100,
        uuid: Uuid::from_u128(100),
        entity_type_id: VANILLA_ENTITY_TYPE_PAINTING_ID,
        position: Vec3d {
            x: f64::from(target.x),
            y: f64::from(target.y),
            z: f64::from(target.z),
        },
        delta_movement: Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: ProtocolDirection::North.id().into(),
    });
    let direction_accessor = vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PAINTING_ID,
    )
    .unwrap();
    let variant_data_id =
        crate::entities::vanilla_entity_metadata_painting_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PAINTING_ID,
        )
        .unwrap();
    assert!(painting_world.apply_set_entity_data(SetEntityData {
        id: 100,
        values: vec![
            EntityDataValue {
                data_id: direction_accessor.direction_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID,
                value: EntityDataValueKind::Direction(i32::from(ProtocolDirection::North.id(),)),
            },
            EntityDataValue {
                data_id: variant_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
                value: EntityDataValueKind::PaintingVariant(
                    bbb_protocol::packets::PaintingVariantData {
                        registry: Some(
                            bbb_protocol::packets::EntityDataRegistryReference::unresolved(0),
                        ),
                        direct: None,
                    },
                ),
            },
        ],
    }));

    assert_eq!(
        painting_world.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            78,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
    assert_eq!(
        player_inventory_item(&painting_world, 0).item_id,
        Some(VANILLA_ITEM_PAINTING_ID)
    );
}

#[test]
fn ender_eye_and_spawn_egg_use_exact_pov_results() {
    let hit = |pos: ProtocolBlockPos| LocalItemUsePovResult::Block {
        location: Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        },
        hit: BlockHitResult {
            pos,
            direction: ProtocolDirection::Up,
            cursor_x: 0.5,
            cursor_y: 0.5,
            cursor_z: 0.5,
            inside: false,
            world_border_hit: false,
        },
    };
    let mut frame_world = tool_use_world(block_state_id(
        "minecraft:end_portal_frame",
        [("eye", "false"), ("facing", "north")],
    ));
    set_main_hand(&mut frame_world, VANILLA_ITEM_ENDER_EYE_ID);
    assert_eq!(
        frame_world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::EnderEyeItem)
    );
    assert_eq!(
        frame_world.predict_local_item_use_with_pov(
            InteractionHand::MainHand,
            Some(hit(ProtocolBlockPos { x: 0, y: 0, z: 0 })),
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(
        frame_world.predict_local_item_use_with_pov(
            InteractionHand::MainHand,
            Some(LocalItemUsePovResult::Miss),
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(LocalInteractionSuccess {
            swing_source: LocalInteractionSwingSource::Server,
            was_item_interaction: true,
            starts_using_item: true,
            extra_swing: None,
            sound_action: None,
            particle_action: None,
        }))
    );

    set_main_hand(&mut frame_world, VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID);
    assert_eq!(
        frame_world.predict_local_item_use_with_pov(
            InteractionHand::MainHand,
            Some(LocalItemUsePovResult::Miss),
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(
        frame_world.predict_local_item_use_with_pov(
            InteractionHand::MainHand,
            Some(hit(ProtocolBlockPos { x: 0, y: 0, z: 0 })),
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
}

#[test]
fn bottle_use_fills_source_water_and_applies_filled_result_inventory_edges() {
    let hit = |pos: ProtocolBlockPos| LocalItemUsePovResult::Block {
        hit: BlockHitResult {
            pos,
            direction: ProtocolDirection::Up,
            cursor_x: 0.5,
            cursor_y: 0.5,
            cursor_z: 0.5,
            inside: false,
            world_border_hit: false,
        },
        location: Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        },
    };
    let pos = ProtocolBlockPos { x: 0, y: 0, z: 0 };
    let water = block_state_id("minecraft:water", [("level", "0")]);
    let water_potion =
        bbb_protocol::potion_types::vanilla_potion_for_resource_id("minecraft:water")
            .expect("generated water potion");

    let mut world = tool_use_world(water);
    set_main_hand(&mut world, VANILLA_ITEM_GLASS_BOTTLE_ID);
    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BottleItem)
    );
    assert_eq!(
        world.predict_local_item_use_with_pov(
            InteractionHand::MainHand,
            Some(LocalItemUsePovResult::Miss),
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(
        world.predict_local_item_use_with_pov(InteractionHand::MainHand, Some(hit(pos)),),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::bottle_fill(false)
        ))
    );
    let filled = player_inventory_item(&world, 0);
    assert_eq!(filled.item_id, Some(VANILLA_ITEM_POTION_ID));
    assert_eq!(filled.count, 1);
    assert_eq!(
        filled.component_patch.potion_contents.potion_id,
        Some(water_potion.id)
    );
    assert_eq!(
        filled.component_patch.added_type_ids,
        vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID]
    );

    let mut dry = tool_use_world(block_state_id("minecraft:stone", []));
    set_main_hand(&mut dry, VANILLA_ITEM_GLASS_BOTTLE_ID);
    assert_eq!(
        dry.predict_local_item_use_with_pov(InteractionHand::MainHand, Some(hit(pos)),),
        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(
        player_inventory_item(&dry, 0).item_id,
        Some(VANILLA_ITEM_GLASS_BOTTLE_ID)
    );

    let mut full = tool_use_world(water);
    for slot in 0..36 {
        full.apply_set_player_inventory(SetPlayerInventory {
            slot,
            item: stack(VANILLA_ITEM_STONE_ID),
        });
    }
    full.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_GLASS_BOTTLE_ID)
        },
    });
    assert_eq!(
        full.predict_local_item_use_with_pov(InteractionHand::MainHand, Some(hit(pos)),),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::bottle_fill(true)
        ))
    );
    assert_eq!(player_inventory_item(&full, 0).count, 1);
    assert!(full
        .inventory()
        .player_slots
        .iter()
        .all(|slot| slot.item.item_id != Some(VANILLA_ITEM_POTION_ID)));
}

#[test]
fn trident_use_checks_breakage_riptide_and_water_before_starting_use() {
    let consume = LocalItemUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::consume(true),
    ));
    let mut dry = WorldStore::new();
    set_main_hand(&mut dry, VANILLA_ITEM_TRIDENT_ID);
    assert_eq!(
        dry.predict_local_item_use(InteractionHand::MainHand),
        consume
    );

    dry.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: Some(VANILLA_ITEM_TRIDENT_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                ],
                max_damage: Some(250),
                damage: Some(249),
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    });
    assert_eq!(
        dry.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
    );

    let install_riptide = |world: &mut WorldStore| {
        world.record_registry_data(RegistryData {
            registry: "minecraft:enchantment".to_string(),
            entries: vec![RegistryDataEntry {
                id: "minecraft:riptide".to_string(),
                raw_data: None,
            }],
            raw_payload_len: 0,
        });
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_TRIDENT_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID],
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 1,
                    }],
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
        });
    };
    install_riptide(&mut dry);
    assert_eq!(
        dry.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
    );

    let mut water = tool_use_world(block_state_id("minecraft:water", [("level", "0")]));
    water.set_local_player_pose(crate::LocalPlayerPoseState::default());
    install_riptide(&mut water);
    assert_eq!(
        water.predict_local_item_use(InteractionHand::MainHand),
        consume
    );
}

#[test]
fn riptide_release_pushes_moves_and_starts_private_auto_spin_state() {
    let mut world = tool_use_world(block_state_id("minecraft:water", [("level", "0")]));
    world.record_registry_data(RegistryData {
        registry: "minecraft:enchantment".to_string(),
        entries: vec![RegistryDataEntry {
            id: "minecraft:riptide".to_string(),
            raw_data: None,
        }],
        raw_payload_len: 0,
    });
    let trident = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_TRIDENT_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID],
            enchantments: vec![ItemEnchantmentSummary {
                holder_id: 0,
                level: 2,
            }],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: trident.clone(),
    });
    world.set_local_player_pose(crate::LocalPlayerPoseState {
        on_ground: true,
        y_rot: 90.0,
        x_rot: -30.0,
        ..crate::LocalPlayerPoseState::default()
    });
    world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    world.advance_local_using_item_ticks(10);

    assert!(world.release_local_using_item());
    let pose = world.local_player_pose().unwrap();
    let strength = 2.25_f32;
    let y_rot = 90.0_f32.to_radians();
    let x_rot = -30.0_f32.to_radians();
    assert!(
        (pose.delta_movement.x - f64::from(-y_rot.sin() * x_rot.cos() * strength)).abs() < 1.0e-6
    );
    assert!((pose.delta_movement.y - f64::from(-x_rot.sin() * strength)).abs() < 1.0e-6);
    assert!(
        (pose.delta_movement.z - f64::from(y_rot.cos() * x_rot.cos() * strength)).abs() < 1.0e-6
    );
    assert!((pose.position.y - f64::from(1.1999999_f32)).abs() < 1.0e-9);
    assert!(!pose.on_ground);
    assert_eq!(
        world.local_player_auto_spin_attack(),
        Some(&crate::LocalPlayerAutoSpinAttackState {
            ticks_remaining: 20,
            damage_bits: 8.0_f32.to_bits(),
            item: trident.clone(),
        })
    );
    assert!(world
        .take_pending_local_interaction_sound_actions()
        .is_empty());
    assert_eq!(
        world.local_item_in_hand(InteractionHand::MainHand),
        Some(&trident)
    );

    world.advance_local_player_input(crate::LocalPlayerInputState::default(), 0.05);
    assert_eq!(
        world
            .local_player_auto_spin_attack()
            .map(|state| state.ticks_remaining),
        Some(19)
    );
}

#[test]
fn riptide_release_rechecks_damage_wetness_and_passenger_state() {
    let install_riptide = |world: &mut WorldStore, damaged: bool| {
        world.record_registry_data(RegistryData {
            registry: "minecraft:enchantment".to_string(),
            entries: vec![RegistryDataEntry {
                id: "minecraft:riptide".to_string(),
                raw_data: None,
            }],
            raw_payload_len: 0,
        });
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_TRIDENT_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: if damaged {
                        vec![
                            VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                            VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                        ]
                    } else {
                        vec![VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID]
                    },
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 1,
                    }],
                    max_damage: damaged.then_some(250),
                    damage: damaged.then_some(249),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
        });
        world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        world.advance_local_using_item_ticks(10);
    };

    let mut dry = WorldStore::new();
    dry.set_local_player_pose(crate::LocalPlayerPoseState::default());
    install_riptide(&mut dry, false);
    assert!(dry.release_local_using_item());
    assert!(dry.local_player_auto_spin_attack().is_none());

    let mut damaged = tool_use_world(block_state_id("minecraft:water", [("level", "0")]));
    damaged.set_local_player_pose(crate::LocalPlayerPoseState::default());
    install_riptide(&mut damaged, true);
    assert!(damaged.release_local_using_item());
    assert!(damaged.local_player_auto_spin_attack().is_none());

    let mut passenger = tool_use_world_with_login(
        block_state_id("minecraft:water", [("level", "0")]),
        Some(("minecraft:overworld", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
    );
    add_entity(&mut passenger, 7, VANILLA_ENTITY_TYPE_PLAYER_ID);
    add_entity(&mut passenger, 8, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    assert!(passenger.apply_set_passengers(SetPassengers {
        vehicle_id: 8,
        passenger_ids: vec![7],
    }));
    passenger.set_local_player_pose(crate::LocalPlayerPoseState::default());
    install_riptide(&mut passenger, false);
    assert!(passenger.release_local_using_item());
    assert!(passenger.local_player_auto_spin_attack().is_none());
    assert!(passenger
        .take_pending_local_interaction_sound_actions()
        .is_empty());
}

#[test]
fn instrument_use_resolves_named_session_holder_and_applies_item_cooldown() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        crate::INSTRUMENT_REGISTRY,
        0,
        vec![
            crate::RegistryPacketEntry::stub("example:other_horn"),
            crate::RegistryPacketEntry::with_known_pack_data_for_test(
                "example:network_horn",
                br#"{
                        "sound_event":"minecraft:item.goat_horn.sound.0",
                        "use_duration":7.05,
                        "range":128.0,
                        "description":"Network horn"
                    }"#
                .to_vec(),
            ),
        ],
    );
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: Some(VANILLA_ITEM_GOAT_HORN_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID],
                instrument: bbb_protocol::packets::InstrumentComponentSummary {
                    registry_id: Some(1),
                    ..Default::default()
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    });
    let instrument = world
        .local_instrument_use_state(world.local_item_in_hand(InteractionHand::MainHand).unwrap())
        .unwrap();
    assert_eq!(
        instrument.sound_event.registry_id,
        bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
            "minecraft:item.goat_horn.sound.0"
        )
    );
    assert_eq!(instrument.sound_event.sound_id, None);

    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::instrument_use(InteractionHand::MainHand)
        ))
    );
    assert_eq!(
        world
            .cooldown("minecraft:goat_horn")
            .map(|cooldown| (cooldown.duration, cooldown.remaining_ticks)),
        Some((141, 141))
    );
    assert_eq!(world.counters().cooldown_packets, 0);
    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    );
}

#[test]
fn instrument_use_resolves_direct_payload_sound_and_custom_cooldown_group() {
    let mut expected_seed = crate::SoundSeedRandomState::default();
    let expected_seed = expected_seed.next_long();
    let mut world = tool_use_world_with_login(
        block_state_id("minecraft:air", []),
        Some(("minecraft:overworld", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
    );
    world.set_local_player_pose(crate::LocalPlayerPoseState {
        position: Vec3d {
            x: 1.25,
            y: 64.0,
            z: -3.5,
        },
        ..crate::LocalPlayerPoseState::default()
    });
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: direct_instrument_stack(3.25, 96.0, Some("example:horns")),
    });

    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::instrument_use(InteractionHand::MainHand)
        ))
    );
    assert_eq!(
        world
            .cooldown("example:horns")
            .map(|cooldown| cooldown.duration),
        Some(65)
    );
    let effect = world
        .apply_local_interaction_sound_action(
            LocalInteractionSoundAction::InstrumentUse {
                hand: InteractionHand::MainHand,
            },
            &mut crate::LevelEventSoundRandomState::default(),
        )
        .unwrap();
    let crate::LocalInteractionSoundEffect::Entity { state, position } = effect else {
        panic!("instrument use must produce an entity-bound sound");
    };
    assert_eq!(state.sound.kind, "direct");
    assert_eq!(state.sound.location.as_deref(), Some("example:direct_horn"));
    assert_eq!(state.sound.fixed_range, Some(24.0));
    assert_eq!(state.source, "record");
    assert_eq!(state.entity_id, 7);
    assert_eq!(state.volume, 6.0);
    assert_eq!(state.pitch, 1.0);
    assert_eq!(state.seed, expected_seed);
    assert!(!state.looping);
    assert!(state.linear_attenuation);
    assert!(!state.tickable);
    assert_eq!(
        position,
        Some(Vec3d {
            x: 1.25,
            y: 64.0,
            z: -3.5,
        })
    );
    assert_eq!(world.last_sound_entity(), Some(&state));
}

#[test]
fn instrument_use_without_effective_component_fails_and_invalid_payload_stays_explicit() {
    let mut world = WorldStore::new();
    set_main_hand(&mut world, VANILLA_ITEM_GOAT_HORN_ID);
    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
    );

    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: Some(VANILLA_ITEM_GOAT_HORN_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID],
                instrument: bbb_protocol::packets::InstrumentComponentSummary {
                    registry_id: Some(0),
                    direct: direct_instrument_stack(1.0, 16.0, None)
                        .component_patch
                        .instrument
                        .direct
                        .clone(),
                },
                ..DataComponentPatchSummary::default()
            }
            .into(),
        },
    });
    assert_eq!(
        world.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::MissingComponentValue(VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID)
    );
    assert!(world.cooldowns().is_empty());
}

#[test]
fn potion_mud_conversion_preserves_live_tag_holder_and_face_gates() {
    let clicked = BlockPos { x: 8, y: 8, z: 8 };
    let dirt = block_state_id("minecraft:dirt", []);
    let mut world = tool_use_world(block_state_id("minecraft:air", []));
    set_block(&mut world, clicked, dirt);
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: potion_stack(0, 1),
    });

    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            1,
        ),
        LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::PotionItem)
    );

    install_convertable_to_mud_tag(&mut world);
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: potion_stack(1, 1),
    });
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            2,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: potion_stack(0, 1),
    });
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Down,
            InteractionHand::MainHand,
            false,
            3,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(world.probe_block(clicked).unwrap().block_state_id, dirt);
    assert_eq!(
        player_inventory_item(&world, 0).item_id,
        Some(VANILLA_ITEM_POTION_ID)
    );
    assert!(world.local_block_predictions().is_empty());
}

#[test]
fn potion_filled_result_tracks_creative_and_full_inventory_branches() {
    let clicked = BlockPos { x: 8, y: 8, z: 8 };
    let dirt = block_state_id("minecraft:dirt", []);
    let mut creative = tool_use_world(block_state_id("minecraft:air", []));
    install_convertable_to_mud_tag(&mut creative);
    set_block(&mut creative, clicked, dirt);
    creative.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    creative.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: potion_stack(0, 1),
    });
    assert_eq!(
        creative.predict_local_block_use(
            clicked,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            false,
            4,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction_with_drop(false)
        ))
    );
    assert_eq!(
        player_inventory_item(&creative, 0).item_id,
        Some(VANILLA_ITEM_POTION_ID)
    );
    assert_eq!(
        player_inventory_item(&creative, 1).item_id,
        Some(VANILLA_ITEM_GLASS_BOTTLE_ID)
    );
    assert_eq!(creative.sync_ended_local_block_predictions(4), 1);
    assert_eq!(creative.probe_block(clicked).unwrap().block_state_id, dirt);
    assert_eq!(
        creative.predict_local_block_use(
            clicked,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            false,
            5,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction_with_drop(false)
        ))
    );
    assert_eq!(player_inventory_item(&creative, 1).count, 1);
    assert!(creative
        .inventory()
        .player_slots
        .iter()
        .all(|slot| slot.slot == 1 || slot.item.item_id != Some(VANILLA_ITEM_GLASS_BOTTLE_ID)));

    let mut stacked = tool_use_world(block_state_id("minecraft:air", []));
    install_convertable_to_mud_tag(&mut stacked);
    set_block(&mut stacked, clicked, dirt);
    stacked.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: potion_stack(0, 2),
    });
    assert_eq!(
        stacked.predict_local_block_use(
            clicked,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            false,
            6,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction_with_drop(false)
        ))
    );
    assert_eq!(player_inventory_item(&stacked, 0).count, 1);
    assert_eq!(
        player_inventory_item(&stacked, 1).item_id,
        Some(VANILLA_ITEM_GLASS_BOTTLE_ID)
    );

    let mut full = tool_use_world(block_state_id("minecraft:air", []));
    install_convertable_to_mud_tag(&mut full);
    set_block(&mut full, clicked, dirt);
    for slot in 0..36 {
        full.apply_set_player_inventory(SetPlayerInventory {
            slot,
            item: stack(VANILLA_ITEM_STONE_ID),
        });
    }
    full.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: potion_stack(0, 2),
    });
    assert_eq!(
        full.predict_local_block_use(
            clicked,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            false,
            7,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction_with_drop(true)
        ))
    );
    assert_eq!(player_inventory_item(&full, 0).count, 1);
    assert!(full
        .inventory()
        .player_slots
        .iter()
        .all(|slot| { slot.item.item_id != Some(VANILLA_ITEM_GLASS_BOTTLE_ID) }));
}

#[test]
fn bonemeal_generated_owners_with_sturdy_faces_are_evaluated() {
    let air = block_state_id("minecraft:air", []);
    let mut world = tool_use_world(air);
    let pos = BlockPos { x: 8, y: 8, z: 8 };
    let state_count = world.registries().block_state_count();
    let mut reached_evaluators = [false; 6];
    let mut missing_evaluators = Vec::new();
    for state_id in 0..state_count {
        let Some(state) = world
            .registries()
            .block_state(i32::try_from(state_id).unwrap())
            .cloned()
        else {
            continue;
        };
        let Some(block_type) = vanilla_block_type_for_resource_id(&state.name) else {
            continue;
        };
        let Some(owner) = block_type.bonemeal_target_override else {
            continue;
        };
        set_block(&mut world, pos, state.id);
        let probe = world.probe_block(pos).unwrap();
        if world
            .debug_block_sturdy_faces(&probe)
            .into_iter()
            .any(|face| face)
        {
            let evaluator = match owner {
                VanillaBonemealTargetOverride::BonemealableFeaturePlacerBlock => 0,
                VanillaBonemealTargetOverride::GrassBlock => 1,
                VanillaBonemealTargetOverride::NetherrackBlock => 2,
                VanillaBonemealTargetOverride::NyliumBlock => 3,
                VanillaBonemealTargetOverride::RootedDirtBlock => 4,
                VanillaBonemealTargetOverride::MossyCarpetBlock => 5,
                VanillaBonemealTargetOverride::AzaleaBlock
                | VanillaBonemealTargetOverride::MushroomBlock
                | VanillaBonemealTargetOverride::SaplingBlock => continue,
                _ => {
                    missing_evaluators
                        .push(format!("{} state {} ({owner:?})", state.name, state.id));
                    continue;
                }
            };
            reached_evaluators[evaluator] = true;
        }
    }
    assert!(
        missing_evaluators.is_empty(),
        "sturdy bonemeal states without evaluators: {missing_evaluators:#?}"
    );
    assert_eq!(reached_evaluators, [true; 6]);
}

#[test]
fn hoe_and_shovel_use_on_predict_source_owned_client_result_gates() {
    let grass = block_state_id("minecraft:grass_block", [("snowy", "false")]);
    let mut world = tool_use_world(grass);
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let hoe_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::BlockPositioned {
                pos,
                sound_resource_id: "minecraft:item.hoe.till",
                volume_bits: 1.0_f32.to_bits(),
                pitch_bits: 1.0_f32.to_bits(),
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        },
    ));
    let shovel_flatten_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::BlockPositioned {
                pos,
                sound_resource_id: "minecraft:item.shovel.flatten",
                volume_bits: 1.0_f32.to_bits(),
                pitch_bits: 1.0_f32.to_bits(),
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        },
    ));
    set_main_hand(&mut world, VANILLA_ITEM_IRON_HOE_ID);
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            6,
        ),
        hoe_success
    );
    assert_eq!(world.probe_block(pos).unwrap().block_state_id, grass);
    assert!(world.local_block_predictions().is_empty());
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Down,
            InteractionHand::MainHand,
            false,
            7,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    set_main_hand(&mut world, VANILLA_ITEM_IRON_SHOVEL_ID);
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            8,
        ),
        shovel_flatten_success
    );
    assert_eq!(world.probe_block(pos).unwrap().block_state_id, grass);
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Down,
            InteractionHand::MainHand,
            false,
            9,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    set_main_hand(&mut world, VANILLA_ITEM_IRON_AXE_ID);
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            10,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    let rooted_dirt = block_state_id("minecraft:rooted_dirt", []);
    let mut rooted_world = tool_use_world(rooted_dirt);
    assert!(rooted_world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 0 },
        block_state_id: block_state_id("minecraft:stone", []),
    }));
    set_main_hand(&mut rooted_world, VANILLA_ITEM_IRON_HOE_ID);
    assert_eq!(
        rooted_world.predict_local_block_use(
            pos,
            ProtocolDirection::Down,
            InteractionHand::MainHand,
            false,
            11,
        ),
        hoe_success
    );
    assert_eq!(
        rooted_world.probe_block(pos).unwrap().block_state_id,
        rooted_dirt
    );

    let campfire = block_state_id(
        "minecraft:campfire",
        [
            ("facing", "north"),
            ("lit", "true"),
            ("signal_fire", "false"),
            ("waterlogged", "false"),
        ],
    );
    let mut campfire_world = tool_use_world(campfire);
    set_main_hand(&mut campfire_world, VANILLA_ITEM_IRON_SHOVEL_ID);
    assert_eq!(
        campfire_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            12,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(
        campfire_world.probe_block(pos).unwrap().block_state_id,
        campfire
    );
}

#[test]
fn item_use_on_respects_adventure_can_place_on_before_tool_owner() {
    let oak_log = block_state_id("minecraft:oak_log", [("axis", "y")]);
    let mut world = tool_use_world(oak_log);
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID,
        param: bbb_protocol::game_types::VANILLA_GAME_TYPE_ADVENTURE_ID as f32,
    });
    set_main_hand(&mut world, VANILLA_ITEM_IRON_AXE_ID);
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            13,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(world.probe_block(pos).unwrap().block_state_id, oak_log);

    let oak_log_holder = world
        .registry_holder_id_by_resource_id("minecraft:block", "minecraft:oak_log")
        .unwrap();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
                ],
                can_place_on_persistent: Some(AdventureModePredicateDataSummary {
                    predicates: vec![BlockPredicateSummary {
                        blocks: Some(HolderSetSummary {
                            registry_ids: vec![oak_log_holder],
                            tag: None,
                        }),
                        ..BlockPredicateSummary::default()
                    }],
                }),
                ..DataComponentPatchSummary::default()
            }
            .into(),
            ..stack(VANILLA_ITEM_IRON_AXE_ID)
        },
    });
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            14,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::BlockPositioned {
                    pos,
                    sound_resource_id: "minecraft:item.axe.strip",
                    volume_bits: 1.0_f32.to_bits(),
                    pitch_bits: 1.0_f32.to_bits(),
                }),
                ..LocalInteractionSuccess::client_item_interaction()
            }
        ))
    );
    assert_eq!(
        world.probe_block(pos).unwrap().block_name.as_deref(),
        Some("minecraft:stripped_oak_log")
    );
}

#[test]
fn spawn_egg_and_firework_use_on_predict_client_short_circuits() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let mut world = tool_use_world(block_state_id("minecraft:stone", []));
    let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ));

    set_main_hand(&mut world, VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID);
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            1,
        ),
        client_success
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );

    set_main_hand(&mut world, VANILLA_ITEM_FIREWORK_ROCKET_ID);
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            2,
        ),
        client_success
    );
    add_entity(&mut world, 7, VANILLA_ENTITY_TYPE_PLAYER_ID);
    world.local_player_id = Some(7);
    let accessors = crate::entities::vanilla_entity_metadata_entity_accessors().unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 7,
        values: vec![EntityDataValue {
            data_id: accessors.shared_flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(
                accessors.shared_flags_default | accessors.fall_flying_mask,
            ),
        }],
    }));
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            3,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
}

#[test]
fn lodestone_compass_use_on_predicts_inventory_and_drop_edges_without_local_sound() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let lodestone = block_state_id("minecraft:lodestone", []);
    let expected_success = |dropped| {
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction_with_drop(dropped),
        ))
    };

    let mut singleton = tool_use_world_with_login(
        lodestone,
        Some(("example:dimension", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
    );
    let mut removed_tracker = stack(VANILLA_ITEM_COMPASS_ID);
    removed_tracker
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID);
    singleton.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: removed_tracker,
    });
    let result = singleton.predict_local_block_use(
        pos,
        ProtocolDirection::Up,
        InteractionHand::MainHand,
        false,
        1,
    );
    assert_eq!(result, expected_success(false));
    let locked = player_inventory_item(&singleton, 0);
    assert_eq!(locked.count, 1);
    assert!(locked
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));
    assert!(!locked
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));
    assert_eq!(
        locked.component_patch.lodestone_target,
        Some(LodestoneTargetSummary {
            dimension: "example:dimension".to_string(),
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
        })
    );
    assert_eq!(locked.component_patch.lodestone_tracked, Some(true));
    assert!(
        singleton.client_audio().last_sound.is_none(),
        "CompassItem passes null as the excluded player, so ClientLevel does not play locally"
    );

    assert!(singleton.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
        block_state_id: block_state_id("minecraft:stone", []),
    }));
    assert_eq!(
        singleton.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            2,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    let mut stacked = tool_use_world_with_login(
        lodestone,
        Some(("minecraft:overworld", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
    );
    stacked.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_COMPASS_ID)
        },
    });
    assert_eq!(
        stacked.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            3,
        ),
        expected_success(false)
    );
    assert_eq!(player_inventory_item(&stacked, 0).count, 1);
    assert!(!player_inventory_item(&stacked, 0)
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));
    assert_eq!(player_inventory_item(&stacked, 1).count, 1);
    assert!(player_inventory_item(&stacked, 1)
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));

    let mut creative = tool_use_world_with_login(
        lodestone,
        Some(("minecraft:overworld", VANILLA_GAME_TYPE_CREATIVE_ID as i8)),
    );
    creative.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_COMPASS_ID)
        },
    });
    assert_eq!(
        creative.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            4,
        ),
        expected_success(false)
    );
    assert_eq!(player_inventory_item(&creative, 0).count, 2);
    assert_eq!(player_inventory_item(&creative, 1).count, 1);
    assert!(player_inventory_item(&creative, 1)
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));

    let mut full = tool_use_world_with_login(
        lodestone,
        Some(("minecraft:overworld", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
    );
    for slot in 0..36 {
        full.apply_set_player_inventory(SetPlayerInventory {
            slot,
            item: stack(VANILLA_ITEM_STONE_ID),
        });
    }
    full.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_COMPASS_ID)
        },
    });
    assert_eq!(
        full.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            5,
        ),
        expected_success(true)
    );
    assert_eq!(player_inventory_item(&full, 0).count, 1);
    assert!(full.inventory().player_slots.iter().all(|slot| {
        !slot
            .item
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID)
    }));
}

#[test]
fn age_lock_particle_timer_expires_after_forty_client_ticks() {
    let mut state = crate::entities::EntityClientAnimationState::default();
    state.start_age_lock_particle_timer();
    assert_eq!(state.age_lock_particle_timer_ticks(), 40);
    for expected in (0..40).rev() {
        state.advance_age_lock_particle_timer_tick();
        assert_eq!(state.age_lock_particle_timer_ticks(), expected);
    }
}
