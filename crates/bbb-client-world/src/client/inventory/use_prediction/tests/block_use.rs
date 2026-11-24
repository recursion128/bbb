use super::*;

#[test]
fn place_on_water_generated_owners_have_exact_block_profiles() {
    let owner_ids = VANILLA_ITEM_TYPES
        .iter()
        .filter(|item| item.use_override == Some(VanillaItemUseOverride::PlaceOnWaterBlockItem))
        .map(|item| item.id)
        .collect::<Vec<_>>();
    assert_eq!(
        owner_ids,
        vec![VANILLA_ITEM_LILY_PAD_ID, VANILLA_ITEM_FROGSPAWN_ID]
    );
    assert_eq!(
        vanilla_block_type(VANILLA_BLOCK_LILY_PAD_ID)
            .unwrap()
            .can_survive_override,
        Some(VanillaCanSurviveOverride::VegetationBlock)
    );
    assert_eq!(
        vanilla_block_type(VANILLA_BLOCK_FROGSPAWN_ID)
            .unwrap()
            .can_survive_override,
        Some(VanillaCanSurviveOverride::FrogspawnBlock)
    );
}

#[test]
fn solid_bucket_generated_owner_places_powder_snow_with_custom_sound_and_empty_bucket() {
    let owner_ids = VANILLA_ITEM_TYPES
        .iter()
        .filter(|item| item.use_on_override == Some(VanillaItemUseOnOverride::SolidBucketItem))
        .map(|item| item.id)
        .collect::<Vec<_>>();
    assert_eq!(owner_ids, vec![VANILLA_ITEM_POWDER_SNOW_BUCKET_ID]);

    let clicked = BlockPos { x: 8, y: 7, z: 8 };
    let target = BlockPos {
        y: clicked.y + 1,
        ..clicked
    };
    let stone = block_state_id("minecraft:stone", []);
    let powder_snow = block_state_id("minecraft:powder_snow", []);
    let mut world = tool_use_world(block_state_id("minecraft:air", []));
    set_block(&mut world, clicked, stone);
    set_main_hand(&mut world, VANILLA_ITEM_POWDER_SNOW_BUCKET_ID);

    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            59,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::block_place(
                target,
                "minecraft:item.bucket.empty_powder_snow",
                1.0,
                0.8,
            )
        ))
    );
    assert_eq!(
        world.probe_block(target).unwrap().block_state_id,
        powder_snow
    );
    assert_eq!(
        player_inventory_item(&world, 0).item_id,
        Some(VANILLA_ITEM_BUCKET_ID)
    );
    assert_eq!(world.local_block_predictions().len(), 1);
    assert_eq!(world.local_block_predictions()[0].sequence, 59);
    assert_eq!(world.sync_ended_local_block_predictions(59), 1);
}

#[test]
fn solid_bucket_uses_block_place_target_and_feature_gates_before_mutation() {
    let clicked = BlockPos { x: 8, y: 7, z: 8 };
    let relative = BlockPos {
        z: clicked.z - 1,
        ..clicked
    };
    let air = block_state_id("minecraft:air", []);
    let stone = block_state_id("minecraft:stone", []);
    let one_snow = block_state_id("minecraft:snow", [("layers", "1")]);
    let two_snow = block_state_id("minecraft:snow", [("layers", "2")]);
    let powder_snow = block_state_id("minecraft:powder_snow", []);
    let prepare = |clicked_state, relative_state| {
        let mut world = tool_use_world(air);
        set_block(&mut world, clicked, clicked_state);
        set_block(&mut world, relative, relative_state);
        set_main_hand(&mut world, VANILLA_ITEM_POWDER_SNOW_BUCKET_ID);
        world
    };

    let mut replace_clicked = prepare(one_snow, stone);
    assert!(matches!(
        replace_clicked.predict_local_block_use(
            clicked,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            true,
            60,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        replace_clicked.probe_block(clicked).unwrap().block_state_id,
        powder_snow
    );

    let mut redirect = prepare(two_snow, air);
    assert!(matches!(
        redirect.predict_local_block_use(
            clicked,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            true,
            61,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        redirect.probe_block(clicked).unwrap().block_state_id,
        two_snow
    );
    assert_eq!(
        redirect.probe_block(relative).unwrap().block_state_id,
        powder_snow
    );

    let mut blocked = prepare(stone, stone);
    assert_eq!(
        blocked.predict_local_block_use(
            clicked,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            true,
            62,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        player_inventory_item(&blocked, 0).item_id,
        Some(VANILLA_ITEM_POWDER_SNOW_BUCKET_ID)
    );
    assert!(blocked.local_block_predictions().is_empty());

    let mut disabled = prepare(stone, air);
    disabled.apply_update_enabled_features(UpdateEnabledFeatures {
        features: Vec::new(),
    });
    assert_eq!(
        disabled.predict_local_block_use(
            clicked,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            true,
            63,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        player_inventory_item(&disabled, 0).item_id,
        Some(VANILLA_ITEM_POWDER_SNOW_BUCKET_ID)
    );
    assert!(disabled.local_block_predictions().is_empty());
}

#[test]
fn solid_bucket_empty_placement_shape_ignores_entities_and_instabuild_keeps_item() {
    let clicked = BlockPos { x: 8, y: 7, z: 8 };
    let target = BlockPos {
        y: clicked.y + 1,
        ..clicked
    };
    let stone = block_state_id("minecraft:stone", []);
    let powder_snow = block_state_id("minecraft:powder_snow", []);
    let mut world = tool_use_world(block_state_id("minecraft:air", []));
    set_block(&mut world, clicked, stone);
    add_entity_at(
        &mut world,
        97,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: f64::from(target.x) + 0.5,
            y: f64::from(target.y) + 0.5,
            z: f64::from(target.z) + 0.5,
        },
    );
    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    set_main_hand(&mut world, VANILLA_ITEM_POWDER_SNOW_BUCKET_ID);

    assert!(matches!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            64,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        world.probe_block(target).unwrap().block_state_id,
        powder_snow
    );
    assert_eq!(
        player_inventory_item(&world, 0).item_id,
        Some(VANILLA_ITEM_POWDER_SNOW_BUCKET_ID)
    );
    assert!(world
        .inventory()
        .player_slots
        .iter()
        .all(|slot| slot.item.item_id != Some(VANILLA_ITEM_BUCKET_ID)));
}

#[test]
fn hanging_item_frame_requires_wall_and_clear_block_but_ignores_ordinary_entities() {
    let clicked = BlockPos { x: 8, y: 7, z: 8 };
    let target = BlockPos {
        x: clicked.x + 1,
        ..clicked
    };
    let air = block_state_id("minecraft:air", []);
    let stone = block_state_id("minecraft:stone", []);
    let repeater = block_state_id(
        "minecraft:repeater",
        [
            ("delay", "1"),
            ("facing", "north"),
            ("locked", "false"),
            ("powered", "false"),
        ],
    );
    let prepare = |support, target_state| {
        let mut world = tool_use_world(air);
        set_block(&mut world, clicked, support);
        set_block(&mut world, target, target_state);
        set_main_hand(&mut world, VANILLA_ITEM_ITEM_FRAME_ID);
        world
    };

    let mut no_support = prepare(air, air);
    assert_eq!(
        no_support.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            68,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
    assert_eq!(
        player_inventory_item(&no_support, 0).item_id,
        Some(VANILLA_ITEM_ITEM_FRAME_ID)
    );

    let mut collision = prepare(stone, stone);
    assert_eq!(
        collision.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            69,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );

    let mut diode = prepare(repeater, air);
    assert!(matches!(
        diode.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            70,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                swing_source: LocalInteractionSwingSource::Client,
                ..
            }
        ))
    ));

    let mut ordinary_entity = prepare(stone, air);
    add_entity_at(
        &mut ordinary_entity,
        98,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        Vec3d {
            x: f64::from(target.x) + 0.5,
            y: f64::from(target.y) + 0.5,
            z: f64::from(target.z) + 0.5,
        },
    );
    assert!(matches!(
        ordinary_entity.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            71,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                swing_source: LocalInteractionSwingSource::Client,
                ..
            }
        ))
    ));

    let border_clicked = BlockPos { x: 1, y: 7, z: 0 };
    let mut border_collision = tool_use_world(air);
    set_block(&mut border_collision, border_clicked, stone);
    set_main_hand(&mut border_collision, VANILLA_ITEM_ITEM_FRAME_ID);
    border_collision.apply_set_border_size(bbb_protocol::packets::SetBorderSize { size: 4.0 });
    assert_eq!(
        border_collision.predict_local_block_use(
            border_clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            true,
            72,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
}

#[test]
fn armor_stand_uses_replace_clicked_and_empty_context_block_collision() {
    let clicked = BlockPos { x: 8, y: 7, z: 8 };
    let relative = BlockPos {
        z: clicked.z - 1,
        ..clicked
    };
    let air = block_state_id("minecraft:air", []);
    let stone = block_state_id("minecraft:stone", []);
    let one_snow = block_state_id("minecraft:snow", [("layers", "1")]);
    let two_snow = block_state_id("minecraft:snow", [("layers", "2")]);
    let powder_snow = block_state_id("minecraft:powder_snow", []);
    let prepare = |clicked_state, relative_state| {
        let mut world = tool_use_world(air);
        set_block(&mut world, clicked, clicked_state);
        set_block(&mut world, relative, relative_state);
        set_main_hand(&mut world, VANILLA_ITEM_ARMOR_STAND_ID);
        world
    };

    let mut replace_clicked = prepare(one_snow, stone);
    assert!(matches!(
        replace_clicked.predict_local_block_use(
            clicked,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            true,
            51,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));

    let mut redirect = prepare(two_snow, stone);
    assert_eq!(
        redirect.predict_local_block_use(
            clicked,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            true,
            52,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        player_inventory_item(&redirect, 0).item_id,
        Some(VANILLA_ITEM_ARMOR_STAND_ID)
    );

    let mut powder = prepare(stone, air);
    set_block(
        &mut powder,
        BlockPos {
            y: clicked.y + 1,
            ..clicked
        },
        powder_snow,
    );
    assert!(matches!(
        powder.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            53,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
}

#[test]
fn empty_bucket_pickup_predicts_source_mutation_inventory_sound_and_rollback() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let water = block_state_id("minecraft:water", [("level", "0")]);
    let air = block_state_id("minecraft:air", []);
    let mut world = tool_use_world(water);
    set_main_hand(&mut world, VANILLA_ITEM_BUCKET_ID);

    assert_eq!(
        world.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            17,
            false,
        ),
        bucket_success_with(
            LocalInteractionSoundAction::BucketFill {
                sound_resource_id: "minecraft:item.bucket.fill",
            },
            None,
        )
    );
    assert_eq!(world.probe_block(pos).unwrap().block_state_id, air);
    assert_eq!(
        player_inventory_item(&world, 0).item_id,
        Some(VANILLA_ITEM_WATER_BUCKET_ID)
    );
    assert_eq!(world.local_block_predictions().len(), 1);
    assert_eq!(world.sync_ended_local_block_predictions(17), 1);
    assert_eq!(world.probe_block(pos).unwrap().block_state_id, water);

    let flowing = block_state_id("minecraft:water", [("level", "1")]);
    let mut flowing_world = tool_use_world(flowing);
    set_main_hand(&mut flowing_world, VANILLA_ITEM_BUCKET_ID);
    assert_eq!(
        flowing_world.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            18,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        player_inventory_item(&flowing_world, 0).item_id,
        Some(VANILLA_ITEM_BUCKET_ID)
    );
    assert!(flowing_world.local_block_predictions().is_empty());
}

#[test]
fn bucket_pickup_and_placement_follow_waterlogged_container_and_sneak_paths() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let above = BlockPos { y: 1, ..pos };
    let dry_slab = block_state_id(
        "minecraft:oak_slab",
        [("type", "bottom"), ("waterlogged", "false")],
    );
    let wet_slab = block_state_id(
        "minecraft:oak_slab",
        [("type", "bottom"), ("waterlogged", "true")],
    );
    let water = block_state_id("minecraft:water", [("level", "0")]);

    let mut pickup = tool_use_world(wet_slab);
    set_main_hand(&mut pickup, VANILLA_ITEM_BUCKET_ID);
    assert!(matches!(
        pickup.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            19,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(pickup.probe_block(pos).unwrap().block_state_id, dry_slab);
    assert_eq!(
        player_inventory_item(&pickup, 0).item_id,
        Some(VANILLA_ITEM_WATER_BUCKET_ID)
    );

    let mut container = tool_use_world(dry_slab);
    set_main_hand(&mut container, VANILLA_ITEM_WATER_BUCKET_ID);
    assert_eq!(
        container.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            20,
            false,
        ),
        bucket_success_with(
            LocalInteractionSoundAction::BucketEmpty {
                pos,
                sound_resource_id: "minecraft:item.bucket.empty",
                neutral_source: false,
            },
            None,
        )
    );
    assert_eq!(container.probe_block(pos).unwrap().block_state_id, dry_slab);
    assert_eq!(
        player_inventory_item(&container, 0).item_id,
        Some(VANILLA_ITEM_BUCKET_ID)
    );
    assert!(container.local_block_predictions().is_empty());

    let mut sneaking = tool_use_world(dry_slab);
    set_main_hand(&mut sneaking, VANILLA_ITEM_WATER_BUCKET_ID);
    assert!(matches!(
        sneaking.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            21,
            true,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(sneaking.probe_block(pos).unwrap().block_state_id, dry_slab);
    assert_eq!(sneaking.probe_block(above).unwrap().block_state_id, water);
    assert_eq!(sneaking.local_block_predictions().len(), 1);

    let wet_ladder = block_state_id(
        "minecraft:ladder",
        [("facing", "north"), ("waterlogged", "true")],
    );
    let dry_ladder = block_state_id(
        "minecraft:ladder",
        [("facing", "north"), ("waterlogged", "false")],
    );
    let ladder_support = BlockPos { z: 1, ..pos };
    let stone = block_state_id("minecraft:stone", []);
    let mut supported_ladder = tool_use_world(wet_ladder);
    set_block(&mut supported_ladder, ladder_support, stone);
    set_main_hand(&mut supported_ladder, VANILLA_ITEM_BUCKET_ID);
    assert!(matches!(
        supported_ladder.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            28,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        supported_ladder.probe_block(pos).unwrap().block_state_id,
        dry_ladder
    );
    assert_eq!(
        player_inventory_item(&supported_ladder, 0).item_id,
        Some(VANILLA_ITEM_WATER_BUCKET_ID)
    );

    let air = block_state_id("minecraft:air", []);
    let mut detached_ladder = tool_use_world(wet_ladder);
    set_main_hand(&mut detached_ladder, VANILLA_ITEM_BUCKET_ID);
    assert!(matches!(
        detached_ladder.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            29,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        detached_ladder.probe_block(pos).unwrap().block_state_id,
        air
    );
    assert_eq!(
        detached_ladder.local_block_predictions()[0].server_block_state_id,
        wet_ladder
    );
}

#[test]
fn bucket_pickup_generated_can_survive_owners_all_have_exact_evaluators() {
    let mut owners = Vec::new();
    let mut missing_evaluators = Vec::new();
    for block in VANILLA_BLOCK_TYPES.iter().filter(|block| {
        block.bucket_pickup_override == Some(VanillaBucketPickupOverride::SimpleWaterloggedBlock)
    }) {
        if let Some(owner) = block.can_survive_override {
            if !owners.contains(&owner) {
                owners.push(owner);
            }
            if !block_placement::survival::block_can_survive_owner_supported(
                owner,
                block.resource_id,
            ) {
                missing_evaluators.push((block.resource_id, owner));
            }
        }
    }

    assert!(
        missing_evaluators.is_empty(),
        "waterlogged bucket-pickup owners without evaluators: {missing_evaluators:?}"
    );
    assert_eq!(owners.len(), 19);
}

#[test]
fn bucket_pickup_uses_static_dynamic_and_collision_support_facts() {
    let pos = BlockPos { x: 8, y: 8, z: 8 };
    let below = BlockPos { y: 7, ..pos };
    let air = block_state_id("minecraft:air", []);
    let cauldron = block_state_id("minecraft:cauldron", []);
    let center_only_piston = block_state_id(
        "minecraft:piston",
        [("extended", "true"), ("facing", "north")],
    );
    let bamboo = block_state_id(
        "minecraft:bamboo",
        [("age", "0"), ("leaves", "none"), ("stage", "0")],
    );
    let scaffolding_support = block_state_id(
        "minecraft:scaffolding",
        [
            ("bottom", "false"),
            ("distance", "0"),
            ("waterlogged", "false"),
        ],
    );
    let pointed_support = block_state_id(
        "minecraft:pointed_dripstone",
        [
            ("thickness", "base"),
            ("vertical_direction", "up"),
            ("waterlogged", "false"),
        ],
    );
    let leaves = block_state_id(
        "minecraft:oak_leaves",
        [
            ("distance", "1"),
            ("persistent", "true"),
            ("waterlogged", "false"),
        ],
    );
    let wet_rail = block_state_id(
        "minecraft:rail",
        [("shape", "north_south"), ("waterlogged", "true")],
    );
    let dry_rail = block_state_id(
        "minecraft:rail",
        [("shape", "north_south"), ("waterlogged", "false")],
    );
    let wet_candle = block_state_id(
        "minecraft:candle",
        [("candles", "1"), ("lit", "false"), ("waterlogged", "true")],
    );
    let dry_candle = block_state_id(
        "minecraft:candle",
        [("candles", "1"), ("lit", "false"), ("waterlogged", "false")],
    );
    let wet_pickle = block_state_id(
        "minecraft:sea_pickle",
        [("pickles", "1"), ("waterlogged", "true")],
    );
    let dry_pickle = block_state_id(
        "minecraft:sea_pickle",
        [("pickles", "1"), ("waterlogged", "false")],
    );
    let wet_lichen = block_state_id(
        "minecraft:glow_lichen",
        [
            ("down", "true"),
            ("east", "false"),
            ("north", "false"),
            ("south", "false"),
            ("up", "false"),
            ("waterlogged", "true"),
            ("west", "false"),
        ],
    );
    let dry_lichen = block_state_id(
        "minecraft:glow_lichen",
        [
            ("down", "true"),
            ("east", "false"),
            ("north", "false"),
            ("south", "false"),
            ("up", "false"),
            ("waterlogged", "false"),
            ("west", "false"),
        ],
    );

    for (sequence, wet, support, expected) in [
        (30, wet_rail, cauldron, dry_rail),
        (31, wet_candle, center_only_piston, dry_candle),
        (32, wet_pickle, bamboo, dry_pickle),
        (33, wet_lichen, leaves, dry_lichen),
        (34, wet_rail, center_only_piston, air),
        (35, wet_candle, cauldron, air),
        (36, wet_rail, scaffolding_support, dry_rail),
        (37, wet_candle, pointed_support, dry_candle),
    ] {
        let mut world = tool_use_world(air);
        set_block(&mut world, pos, wet);
        set_block(&mut world, below, support);
        set_main_hand(&mut world, VANILLA_ITEM_BUCKET_ID);
        assert!(
            matches!(
                world.predict_local_item_use_with_pov_and_sequence(
                    InteractionHand::MainHand,
                    Some(item_use_block_hit(pos, ProtocolDirection::Up)),
                    sequence,
                    false,
                ),
                LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
            ),
            "bucket pickup case {sequence} should succeed"
        );
        assert_eq!(
            world.probe_block(pos).unwrap().block_state_id,
            expected,
            "bucket pickup case {sequence} predicted the wrong block state"
        );
    }
}

#[test]
fn bucket_pickup_survival_uses_live_tags_and_structural_neighbors() {
    let pos = BlockPos { x: 8, y: 8, z: 8 };
    let below = BlockPos { y: 7, ..pos };
    let east = BlockPos { x: 9, ..pos };
    let air = block_state_id("minecraft:air", []);
    let dirt = block_state_id("minecraft:dirt", []);
    let wet_small_dripleaf = block_state_id(
        "minecraft:small_dripleaf",
        [
            ("facing", "north"),
            ("half", "lower"),
            ("waterlogged", "true"),
        ],
    );
    let dry_small_dripleaf = block_state_id(
        "minecraft:small_dripleaf",
        [
            ("facing", "north"),
            ("half", "lower"),
            ("waterlogged", "false"),
        ],
    );

    let mut missing_tag = tool_use_world(air);
    set_block(&mut missing_tag, pos, wet_small_dripleaf);
    set_block(&mut missing_tag, below, dirt);
    set_main_hand(&mut missing_tag, VANILLA_ITEM_BUCKET_ID);
    assert_eq!(
        missing_tag.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            36,
            false,
        ),
        LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BucketItem)
    );
    assert_eq!(
        missing_tag.probe_block(pos).unwrap().block_state_id,
        wet_small_dripleaf
    );

    let mut supported_tag = tool_use_world(air);
    install_block_tag(
        &mut supported_tag,
        "minecraft:supports_small_dripleaf",
        vec![VANILLA_BLOCK_DIRT_ID],
    );
    set_block(&mut supported_tag, pos, wet_small_dripleaf);
    set_block(&mut supported_tag, below, dirt);
    set_main_hand(&mut supported_tag, VANILLA_ITEM_BUCKET_ID);
    assert!(matches!(
        supported_tag.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            37,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        supported_tag.probe_block(pos).unwrap().block_state_id,
        dry_small_dripleaf
    );

    let mut rejected_tag = tool_use_world(air);
    install_block_tag(
        &mut rejected_tag,
        "minecraft:supports_small_dripleaf",
        Vec::new(),
    );
    set_block(&mut rejected_tag, pos, wet_small_dripleaf);
    set_block(&mut rejected_tag, below, dirt);
    set_main_hand(&mut rejected_tag, VANILLA_ITEM_BUCKET_ID);
    assert!(matches!(
        rejected_tag.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            38,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(rejected_tag.probe_block(pos).unwrap().block_state_id, air);

    let wet_scaffolding = block_state_id(
        "minecraft:scaffolding",
        [
            ("bottom", "false"),
            ("distance", "1"),
            ("waterlogged", "true"),
        ],
    );
    let dry_scaffolding = block_state_id(
        "minecraft:scaffolding",
        [
            ("bottom", "false"),
            ("distance", "1"),
            ("waterlogged", "false"),
        ],
    );
    let neighbour_scaffolding = block_state_id(
        "minecraft:scaffolding",
        [
            ("bottom", "false"),
            ("distance", "0"),
            ("waterlogged", "false"),
        ],
    );
    let distance_seven_below_scaffolding = block_state_id(
        "minecraft:scaffolding",
        [
            ("bottom", "true"),
            ("distance", "7"),
            ("waterlogged", "false"),
        ],
    );
    let mut scaffolding = tool_use_world(air);
    set_block(&mut scaffolding, pos, wet_scaffolding);
    set_block(&mut scaffolding, below, distance_seven_below_scaffolding);
    set_block(&mut scaffolding, east, neighbour_scaffolding);
    set_main_hand(&mut scaffolding, VANILLA_ITEM_BUCKET_ID);
    assert!(matches!(
        scaffolding.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            39,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        scaffolding.probe_block(pos).unwrap().block_state_id,
        dry_scaffolding
    );

    let wet_dripstone = block_state_id(
        "minecraft:pointed_dripstone",
        [
            ("thickness", "tip"),
            ("vertical_direction", "up"),
            ("waterlogged", "true"),
        ],
    );
    let dry_dripstone = block_state_id(
        "minecraft:pointed_dripstone",
        [
            ("thickness", "tip"),
            ("vertical_direction", "up"),
            ("waterlogged", "false"),
        ],
    );
    let supporting_dripstone = block_state_id(
        "minecraft:pointed_dripstone",
        [
            ("thickness", "base"),
            ("vertical_direction", "up"),
            ("waterlogged", "false"),
        ],
    );
    let mut dripstone = tool_use_world(air);
    set_block(&mut dripstone, pos, wet_dripstone);
    set_block(&mut dripstone, below, supporting_dripstone);
    set_main_hand(&mut dripstone, VANILLA_ITEM_BUCKET_ID);
    assert!(matches!(
        dripstone.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            40,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        dripstone.probe_block(pos).unwrap().block_state_id,
        dry_dripstone
    );

    let above = BlockPos { y: 9, ..pos };
    let stone = block_state_id("minecraft:stone", []);
    let wet_lantern = block_state_id(
        "minecraft:lantern",
        [("hanging", "true"), ("waterlogged", "true")],
    );
    let dry_lantern = block_state_id(
        "minecraft:lantern",
        [("hanging", "true"), ("waterlogged", "false")],
    );
    for (sequence, unstable_entries, expected) in [
        (41, Vec::new(), dry_lantern),
        (42, vec![VANILLA_BLOCK_STONE_ID], air),
    ] {
        let mut lantern = tool_use_world(air);
        install_block_tag(
            &mut lantern,
            "minecraft:unstable_bottom_center",
            unstable_entries,
        );
        set_block(&mut lantern, pos, wet_lantern);
        set_block(&mut lantern, above, stone);
        set_main_hand(&mut lantern, VANILLA_ITEM_BUCKET_ID);
        assert!(matches!(
            lantern.predict_local_item_use_with_pov_and_sequence(
                InteractionHand::MainHand,
                Some(item_use_block_hit(pos, ProtocolDirection::Down)),
                sequence,
                false,
            ),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        assert_eq!(lantern.probe_block(pos).unwrap().block_state_id, expected);
    }
}

#[test]
fn filled_buckets_predict_replaceable_lava_mob_sound_and_creative_inventory() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let above = BlockPos { y: 1, ..pos };
    let stone = block_state_id("minecraft:stone", []);
    let short_grass = block_state_id("minecraft:short_grass", []);
    let lava = block_state_id("minecraft:lava", [("level", "0")]);
    let water = block_state_id("minecraft:water", [("level", "0")]);

    let mut lava_world = tool_use_world(stone);
    set_block(&mut lava_world, above, short_grass);
    set_main_hand(&mut lava_world, VANILLA_ITEM_LAVA_BUCKET_ID);
    assert_eq!(
        lava_world.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            22,
            false,
        ),
        bucket_success_with(
            LocalInteractionSoundAction::BucketEmpty {
                pos: above,
                sound_resource_id: "minecraft:item.bucket.empty_lava",
                neutral_source: false,
            },
            None,
        )
    );
    assert_eq!(lava_world.probe_block(above).unwrap().block_state_id, lava);
    assert_eq!(
        player_inventory_item(&lava_world, 0).item_id,
        Some(VANILLA_ITEM_BUCKET_ID)
    );

    let mut mob_world = tool_use_world(stone);
    set_main_hand(&mut mob_world, VANILLA_ITEM_COD_BUCKET_ID);
    assert_eq!(
        mob_world.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            23,
            false,
        ),
        bucket_success_with(
            LocalInteractionSoundAction::BucketEmpty {
                pos: above,
                sound_resource_id: "minecraft:item.bucket.empty_fish",
                neutral_source: true,
            },
            None,
        )
    );
    assert_eq!(mob_world.probe_block(above).unwrap().block_state_id, water);
    assert_eq!(
        player_inventory_item(&mob_world, 0).item_id,
        Some(VANILLA_ITEM_BUCKET_ID)
    );

    let mut creative = tool_use_world_with_login(
        stone,
        Some(("minecraft:overworld", VANILLA_GAME_TYPE_CREATIVE_ID as i8)),
    );
    creative.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    set_main_hand(&mut creative, VANILLA_ITEM_WATER_BUCKET_ID);
    assert!(matches!(
        creative.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            24,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        player_inventory_item(&creative, 0).item_id,
        Some(VANILLA_ITEM_WATER_BUCKET_ID)
    );
    assert_eq!(creative.probe_block(above).unwrap().block_state_id, water);

    let mut infinite_survival = tool_use_world(stone);
    infinite_survival.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: false,
        flying: false,
        can_fly: false,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    set_main_hand(&mut infinite_survival, VANILLA_ITEM_WATER_BUCKET_ID);
    assert!(matches!(
        infinite_survival.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            26,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        player_inventory_item(&infinite_survival, 0).item_id,
        Some(VANILLA_ITEM_WATER_BUCKET_ID)
    );
    assert!(infinite_survival
        .inventory()
        .player_slots
        .iter()
        .all(|slot| slot.item.item_id != Some(VANILLA_ITEM_BUCKET_ID)));

    let mut finite_creative = tool_use_world_with_login(
        stone,
        Some(("minecraft:overworld", VANILLA_GAME_TYPE_CREATIVE_ID as i8)),
    );
    finite_creative.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: false,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    set_main_hand(&mut finite_creative, VANILLA_ITEM_WATER_BUCKET_ID);
    assert!(matches!(
        finite_creative.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            27,
            false,
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        player_inventory_item(&finite_creative, 0).item_id,
        Some(VANILLA_ITEM_BUCKET_ID)
    );
}

#[test]
fn water_bucket_evaporation_consumes_bucket_without_block_mutation() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let above = BlockPos { y: 1, ..pos };
    let stone = block_state_id("minecraft:stone", []);
    let air = block_state_id("minecraft:air", []);
    let mut world = tool_use_world_with_login(
        stone,
        Some(("minecraft:the_nether", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
    );
    set_main_hand(&mut world, VANILLA_ITEM_WATER_BUCKET_ID);

    assert_eq!(
        world.predict_local_item_use_with_pov_and_sequence(
            InteractionHand::MainHand,
            Some(item_use_block_hit(pos, ProtocolDirection::Up)),
            25,
            false,
        ),
        bucket_success_with(
            LocalInteractionSoundAction::BucketWaterEvaporate { pos: above },
            Some(LocalInteractionParticleAction::BucketWaterEvaporate { pos: above }),
        )
    );
    assert_eq!(world.probe_block(above).unwrap().block_state_id, air);
    assert_eq!(
        player_inventory_item(&world, 0).item_id,
        Some(VANILLA_ITEM_BUCKET_ID)
    );
    assert!(world.local_block_predictions().is_empty());
}

#[test]
fn bucket_sound_actions_resolve_vanilla_sources_positions_and_evaporation_pitch() {
    let pos = BlockPos { x: -3, y: 70, z: 5 };
    let player_position = Vec3d {
        x: 1.25,
        y: 64.0,
        z: -2.5,
    };
    let mut world = WorldStore::new();
    world.set_local_player_pose(crate::LocalPlayerPoseState {
        position: player_position,
        ..crate::LocalPlayerPoseState::default()
    });
    let mut random = crate::LevelEventSoundRandomState::default();

    let fill = world
        .apply_local_interaction_sound_action(
            LocalInteractionSoundAction::BucketFill {
                sound_resource_id: "minecraft:item.bucket.fill_lava",
            },
            &mut random,
        )
        .unwrap();
    let crate::LocalInteractionSoundEffect::Positioned(fill) = fill else {
        panic!("bucket fill should be positioned at the local player");
    };
    assert_eq!(
        fill.sound.location.as_deref(),
        Some("minecraft:item.bucket.fill_lava")
    );
    assert_eq!(fill.source, "player");
    assert_eq!(fill.position, player_position);
    assert_eq!((fill.volume, fill.pitch), (1.0, 1.0));

    let empty = world
        .apply_local_interaction_sound_action(
            LocalInteractionSoundAction::BucketEmpty {
                pos,
                sound_resource_id: "minecraft:item.bucket.empty_fish",
                neutral_source: true,
            },
            &mut random,
        )
        .unwrap();
    let crate::LocalInteractionSoundEffect::Positioned(empty) = empty else {
        panic!("bucket empty should be block-positioned");
    };
    assert_eq!(empty.source, "neutral");
    assert_eq!(
        empty.position,
        Vec3d {
            x: -2.5,
            y: 70.5,
            z: 5.5,
        }
    );

    let mut expected_random = crate::LevelEventSoundRandomState::default();
    let expected_pitch = 2.6 + (expected_random.next_float() - expected_random.next_float()) * 0.8;
    let evaporate = world
        .apply_local_interaction_sound_action(
            LocalInteractionSoundAction::BucketWaterEvaporate { pos },
            &mut crate::LevelEventSoundRandomState::default(),
        )
        .unwrap();
    let crate::LocalInteractionSoundEffect::Positioned(evaporate) = evaporate else {
        panic!("bucket evaporation should be block-positioned");
    };
    assert_eq!(
        evaporate.sound.location.as_deref(),
        Some("minecraft:block.fire.extinguish")
    );
    assert_eq!(evaporate.source, "block");
    assert_eq!(evaporate.volume, 0.5);
    assert!((evaporate.pitch - expected_pitch).abs() < 1.0e-6);
}

#[test]
fn water_potion_mud_conversion_predicts_block_and_filled_result() {
    let mut world = tool_use_world(block_state_id("minecraft:air", []));
    install_convertable_to_mud_tag(&mut world);
    let clicked = BlockPos { x: 8, y: 8, z: 8 };
    let dirt = block_state_id("minecraft:dirt", []);
    let mud = block_state_id("minecraft:mud", []);
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
            31,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction_with_drop(false)
        ))
    );
    assert_eq!(world.probe_block(clicked).unwrap().block_state_id, mud);
    let hand = player_inventory_item(&world, 0);
    assert_eq!(hand.item_id, Some(VANILLA_ITEM_GLASS_BOTTLE_ID));
    assert_eq!(hand.count, 1);
    assert_eq!(world.local_block_predictions().len(), 1);
    assert_eq!(world.local_block_predictions()[0].sequence, 31);
}

#[test]
fn bone_meal_water_growth_predicts_client_result_without_local_mutation() {
    let mut world = tool_use_world(block_state_id("minecraft:air", []));
    set_main_hand(&mut world, VANILLA_ITEM_BONE_MEAL_ID);
    let clicked = BlockPos { x: 8, y: 8, z: 8 };
    let water = BlockPos { x: 9, y: 8, z: 8 };
    let above_clicked = BlockPos { x: 8, y: 9, z: 8 };
    let source_water = block_state_id("minecraft:water", [("level", "0")]);
    let stone = block_state_id("minecraft:stone", []);
    set_block(&mut world, clicked, stone);
    set_block(&mut world, water, source_water);

    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            false,
            1,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(world.probe_block(clicked).unwrap().block_state_id, stone);
    assert_eq!(
        world.probe_block(water).unwrap().block_state_id,
        source_water
    );
    assert_eq!(player_inventory_item(&world, 0).count, 1);

    let grass = block_state_id("minecraft:grass_block", [("snowy", "false")]);
    set_block(&mut world, clicked, grass);
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            false,
            2,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    set_block(&mut world, above_clicked, stone);
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            false,
            3,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );

    set_block(
        &mut world,
        water,
        block_state_id("minecraft:water", [("level", "1")]),
    );
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            false,
            4,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(player_inventory_item(&world, 0).count, 1);
}

#[test]
fn bone_meal_netherrack_target_uses_live_nylium_tag_and_skylight_rule() {
    let mut world = tool_use_world(block_state_id("minecraft:air", []));
    set_main_hand(&mut world, VANILLA_ITEM_BONE_MEAL_ID);
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:nylium".to_string(),
                entries: vec![VANILLA_BLOCK_CRIMSON_NYLIUM_ID],
            }],
        }],
    });
    let clicked = BlockPos { x: 8, y: 8, z: 8 };
    let water = BlockPos { x: 9, y: 8, z: 8 };
    let neighbour = BlockPos { x: 7, y: 8, z: 8 };
    let above_clicked = BlockPos { x: 8, y: 9, z: 8 };
    set_block(
        &mut world,
        clicked,
        block_state_id("minecraft:netherrack", []),
    );
    set_block(
        &mut world,
        water,
        block_state_id("minecraft:water", [("level", "0")]),
    );
    set_block(
        &mut world,
        neighbour,
        block_state_id("minecraft:crimson_nylium", []),
    );

    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            false,
            1,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    set_block(
        &mut world,
        above_clicked,
        block_state_id("minecraft:tinted_glass", []),
    );
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            false,
            2,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );

    set_block(
        &mut world,
        above_clicked,
        block_state_id("minecraft:glass", []),
    );
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            false,
            3,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
}

#[test]
fn bone_meal_mossy_carpet_target_tracks_replaceability_and_side_support() {
    let mut world = tool_use_world(block_state_id("minecraft:air", []));
    set_main_hand(&mut world, VANILLA_ITEM_BONE_MEAL_ID);
    let clicked = BlockPos { x: 8, y: 8, z: 8 };
    let support = BlockPos { x: 8, y: 9, z: 7 };
    set_block(
        &mut world,
        clicked,
        block_state_id(
            "minecraft:pale_moss_carpet",
            [
                ("bottom", "true"),
                ("east", "none"),
                ("north", "low"),
                ("south", "none"),
                ("west", "none"),
            ],
        ),
    );
    set_block(
        &mut world,
        BlockPos { x: 8, y: 7, z: 8 },
        block_state_id("minecraft:water", [("level", "0")]),
    );
    set_block(&mut world, support, block_state_id("minecraft:stone", []));

    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Down,
            InteractionHand::MainHand,
            false,
            1,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    set_block(
        &mut world,
        BlockPos { x: 8, y: 9, z: 8 },
        block_state_id("minecraft:stone", []),
    );
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Down,
            InteractionHand::MainHand,
            false,
            2,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );

    set_block(
        &mut world,
        BlockPos { x: 8, y: 9, z: 8 },
        block_state_id("minecraft:air", []),
    );
    set_block(&mut world, support, block_state_id("minecraft:air", []));
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Down,
            InteractionHand::MainHand,
            false,
            3,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
}

#[test]
fn bone_meal_non_sturdy_and_missing_water_targets_preserve_pass_boundary() {
    let mut world = tool_use_world(block_state_id("minecraft:air", []));
    set_main_hand(&mut world, VANILLA_ITEM_BONE_MEAL_ID);
    let crop = BlockPos { x: 8, y: 8, z: 8 };
    set_block(
        &mut world,
        crop,
        block_state_id("minecraft:wheat", [("age", "0")]),
    );
    set_block(
        &mut world,
        BlockPos { x: 9, y: 8, z: 8 },
        block_state_id("minecraft:water", [("level", "0")]),
    );
    assert_eq!(
        world.predict_local_block_use(
            crop,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            false,
            1,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    let edge = BlockPos { x: 15, y: 8, z: 8 };
    set_block(&mut world, edge, block_state_id("minecraft:stone", []));
    assert_eq!(
        world.predict_local_block_use(
            edge,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            false,
            2,
        ),
        LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::BoneMealItem)
    );
}

#[test]
fn axe_use_on_predicts_blocking_guard_and_local_state_transforms() {
    let oak_log = block_state_id("minecraft:oak_log", [("axis", "x")]);
    let mut world = tool_use_world(oak_log);
    set_main_hand(&mut world, VANILLA_ITEM_IRON_AXE_ID);
    set_offhand(
        &mut world,
        ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID],
                ..DataComponentPatchSummary::default()
            }
            .into(),
            ..stack(VANILLA_ITEM_SHIELD_ID)
        },
    );
    let pos = BlockPos { x: 0, y: 0, z: 0 };

    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            1,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(world.probe_block(pos).unwrap().block_state_id, oak_log);

    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            2,
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
    let stripped = world.probe_block(pos).unwrap();
    assert_eq!(
        stripped.block_name.as_deref(),
        Some("minecraft:stripped_oak_log")
    );
    assert_eq!(
        stripped.block_properties.get("axis").map(String::as_str),
        Some("x")
    );
    assert_eq!(world.local_block_predictions().len(), 1);
    assert_eq!(world.local_block_predictions()[0].sequence, 2);
    assert_eq!(
        world.local_block_predictions()[0].server_block_state_id,
        oak_log
    );
}

#[test]
fn client_owned_item_use_on_predicts_fixed_and_block_gated_results() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ));
    let consume = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::consume(true),
    ));
    let pass = LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
    let mut world = tool_use_world(block_state_id("minecraft:stone", []));

    for (item_id, expected) in [
        (VANILLA_ITEM_BRUSH_ID, consume),
        (VANILLA_ITEM_DEBUG_STICK_ID, client_success),
        (VANILLA_ITEM_LEAD_ID, pass),
        (VANILLA_ITEM_LILY_PAD_ID, pass),
    ] {
        set_main_hand(&mut world, item_id);
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                1,
            ),
            expected
        );
    }

    let mut frame_world = tool_use_world(block_state_id(
        "minecraft:end_portal_frame",
        [("eye", "false"), ("facing", "north")],
    ));
    set_main_hand(&mut frame_world, VANILLA_ITEM_ENDER_EYE_ID);
    assert_eq!(
        frame_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            2,
        ),
        client_success
    );
    assert_eq!(
        frame_world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );
    assert!(frame_world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
        block_state_id: block_state_id(
            "minecraft:end_portal_frame",
            [("eye", "true"), ("facing", "north")],
        ),
    }));
    assert_eq!(
        frame_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            3,
        ),
        pass
    );

    let mut map_world = tool_use_world(block_state_id(
        "minecraft:white_banner",
        [("rotation", "0")],
    ));
    set_main_hand(&mut map_world, VANILLA_ITEM_FILLED_MAP_ID);
    assert_eq!(
        map_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            4,
        ),
        LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::MapItem)
    );
    map_world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:banners".to_string(),
                entries: vec![VANILLA_BLOCK_WHITE_BANNER_ID],
            }],
        }],
    });
    assert_eq!(
        map_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            5,
        ),
        client_success
    );
    assert!(map_world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
        block_state_id: block_state_id("minecraft:stone", []),
    }));
    assert_eq!(
        map_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            6,
        ),
        pass
    );
}

#[test]
fn ignition_items_light_blocks_with_exact_client_mutation_and_consumption() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let unlit_campfire = block_state_id(
        "minecraft:campfire",
        [
            ("facing", "north"),
            ("lit", "false"),
            ("signal_fire", "false"),
            ("waterlogged", "false"),
        ],
    );
    let lit_campfire = block_state_id(
        "minecraft:campfire",
        [
            ("facing", "north"),
            ("lit", "true"),
            ("signal_fire", "false"),
            ("waterlogged", "false"),
        ],
    );
    let mut flint = tool_use_world(unlit_campfire);
    set_main_hand(&mut flint, VANILLA_ITEM_FLINT_AND_STEEL_ID);
    assert_eq!(
        flint.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            1,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::ignition(Some(
                LocalInteractionSoundAction::FlintAndSteelUse { pos }
            ))
        ))
    );
    assert_eq!(flint.probe_block(pos).unwrap().block_state_id, lit_campfire);
    assert_eq!(player_inventory_item(&flint, 0).count, 1);

    let mut charge = tool_use_world(unlit_campfire);
    charge.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_FIRE_CHARGE_ID)
        },
    });
    assert_eq!(
        charge.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            2,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(
        charge.probe_block(pos).unwrap().block_state_id,
        lit_campfire
    );
    assert_eq!(player_inventory_item(&charge, 0).count, 1);

    let mut creative = tool_use_world_with_login(
        unlit_campfire,
        Some(("minecraft:overworld", VANILLA_GAME_TYPE_CREATIVE_ID as i8)),
    );
    creative.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_FIRE_CHARGE_ID)
        },
    });
    assert!(matches!(
        creative.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            3,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(player_inventory_item(&creative, 0).count, 2);

    let lightable_states = [
        (
            block_state_id(
                "minecraft:candle",
                [("candles", "1"), ("lit", "false"), ("waterlogged", "false")],
            ),
            block_state_id(
                "minecraft:candle",
                [("candles", "1"), ("lit", "true"), ("waterlogged", "false")],
            ),
        ),
        (
            block_state_id("minecraft:candle_cake", [("lit", "false")]),
            block_state_id("minecraft:candle_cake", [("lit", "true")]),
        ),
    ];
    for (unlit, lit) in lightable_states {
        let mut world = tool_use_world(unlit);
        set_main_hand(&mut world, VANILLA_ITEM_FLINT_AND_STEEL_ID);
        assert!(matches!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                4,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        assert_eq!(world.probe_block(pos).unwrap().block_state_id, lit);
    }
}

#[test]
fn ignition_items_predict_ordinary_and_soul_fire_or_fail_closed() {
    let clicked = BlockPos { x: 8, y: 0, z: 8 };
    let fire_pos = BlockPos { x: 8, y: 1, z: 8 };
    let mut ordinary = tool_use_world_with_login(
        block_state_id("minecraft:air", []),
        Some(("minecraft:the_end", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
    );
    set_block(
        &mut ordinary,
        clicked,
        block_state_id("minecraft:oak_planks", []),
    );
    set_main_hand(&mut ordinary, VANILLA_ITEM_FLINT_AND_STEEL_ID);
    assert_eq!(
        ordinary.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            3,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::ignition(Some(
                LocalInteractionSoundAction::FlintAndSteelUse { pos: fire_pos }
            ))
        ))
    );
    assert_eq!(
        ordinary.probe_block(fire_pos).unwrap().block_state_id,
        block_state_id(
            "minecraft:fire",
            [
                ("age", "0"),
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("up", "false"),
                ("west", "false"),
            ],
        )
    );

    let mut soul = tool_use_world_with_login(
        block_state_id("minecraft:air", []),
        Some(("minecraft:the_end", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
    );
    set_block(
        &mut soul,
        clicked,
        block_state_id("minecraft:soul_sand", []),
    );
    set_main_hand(&mut soul, VANILLA_ITEM_FLINT_AND_STEEL_ID);
    assert!(matches!(
        soul.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            4,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        soul.probe_block(fire_pos).unwrap().block_state_id,
        block_state_id("minecraft:soul_fire", [])
    );

    let mut missing_level = tool_use_world(block_state_id("minecraft:air", []));
    set_block(
        &mut missing_level,
        clicked,
        block_state_id("minecraft:stone", []),
    );
    set_main_hand(&mut missing_level, VANILLA_ITEM_FLINT_AND_STEEL_ID);
    assert_eq!(
        missing_level.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            false,
            5,
        ),
        LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::FlintAndSteelItem),
        "without level identity the client must not guess portal eligibility"
    );

    let mut fail = tool_use_world_with_login(
        block_state_id("minecraft:air", []),
        Some(("minecraft:the_end", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
    );
    set_block(&mut fail, clicked, block_state_id("minecraft:stone", []));
    set_main_hand(&mut fail, VANILLA_ITEM_FLINT_AND_STEEL_ID);
    assert_eq!(
        fail.predict_local_block_use(
            clicked,
            ProtocolDirection::East,
            InteractionHand::MainHand,
            false,
            6,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
}

#[test]
fn ignition_creates_empty_obsidian_portal_with_vanilla_axis_priority() {
    let air = block_state_id("minecraft:air", []);
    let obsidian = block_state_id("minecraft:obsidian", []);
    let mut world = tool_use_world_with_login(
        air,
        Some(("minecraft:overworld", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
    );
    // Axis.X uses WEST as PortalShape.rightDir. The two-wide interior is
    // x=7,6 with sides at x=8,5 and a three-block interior height.
    for x in 5..=8 {
        set_block(&mut world, BlockPos { x, y: 0, z: 8 }, obsidian);
        set_block(&mut world, BlockPos { x, y: 4, z: 8 }, obsidian);
    }
    for y in 1..=3 {
        set_block(&mut world, BlockPos { x: 5, y, z: 8 }, obsidian);
        set_block(&mut world, BlockPos { x: 8, y, z: 8 }, obsidian);
    }
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_FIRE_CHARGE_ID)
        },
    });
    assert_eq!(
        world.predict_local_block_use(
            BlockPos { x: 7, y: 0, z: 8 },
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            7,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    let portal = block_state_id("minecraft:nether_portal", [("axis", "x")]);
    for x in [6, 7] {
        for y in 1..=3 {
            assert_eq!(
                world
                    .probe_block(BlockPos { x, y, z: 8 })
                    .unwrap()
                    .block_state_id,
                portal,
                "missing portal prediction at x={x}, y={y}"
            );
        }
    }
    assert_eq!(player_inventory_item(&world, 0).count, 1);
    assert_eq!(world.local_block_predictions().len(), 6);
}
