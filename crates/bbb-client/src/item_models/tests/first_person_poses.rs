use super::*;

#[test]
fn first_person_player_arms_render_empty_main_hand_with_player_skin_light_and_parts() {
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 20.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let world = WorldStore::new();
    let parts =
        PlayerModelPartVisibility::from_vanilla_mask(PlayerModelPartVisibility::RIGHT_SLEEVE_MASK);
    let skin = EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve);
    let player = EntityModelInstance::player_with_skin(1, [0.0, 64.0, 0.0], 0.0, skin, parts)
        .with(|s| s.light_coords = (5_u32 << 4) | (11_u32 << 20));

    let presentation = FirstPersonHandPresentation::fully_raised_for_test(&world);
    let arms =
        first_person_player_arms(&world, &presentation, None, Some(&player), Some(pose), 1.0);

    assert_eq!(arms.len(), 1);
    assert!(!arms[0].left);
    assert_eq!(arms[0].skin, skin);
    assert!(arms[0].sleeve_visible);
    assert_eq!(arms[0].light, [5.0 / 15.0, 11.0 / 15.0]);
    assert!(arms[0].transform.abs_diff_eq(
        first_person_render_player_arm_transform(
            first_person_camera_world_transform(pose),
            false,
            0.0,
            0.0,
        ),
        1.0e-5,
    ));
    let mut lowered_presentation = presentation.clone();
    lowered_presentation.item_used(InteractionHand::MainHand);
    let lowered_arms = first_person_player_arms(
        &world,
        &lowered_presentation,
        None,
        Some(&player),
        Some(pose),
        1.0,
    );
    assert!(lowered_arms[0].transform.abs_diff_eq(
        first_person_render_player_arm_transform(
            first_person_camera_world_transform(pose),
            false,
            1.0,
            0.0,
        ),
        1.0e-5,
    ));

    let mut sleeping_world = world_with_level_dimension("minecraft:overworld");
    sleeping_world.apply_add_entity(AddEntity {
        id: 1,
        uuid: Uuid::from_u128(1),
        entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: Vec3d::default(),
        delta_movement: Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    let accessors = bbb_client_world::vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert!(sleeping_world.apply_set_entity_data(SetEntityData {
        id: 1,
        values: vec![EntityDataValue {
            data_id: accessors.pose_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
            value: EntityDataValueKind::Pose(
                bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID
            ),
        }],
    }));
    assert!(
        first_person_player_arms(
            &sleeping_world,
            &FirstPersonHandPresentation::fully_raised_for_test(&sleeping_world),
            None,
            Some(&player),
            Some(pose),
            1.0,
        )
        .is_empty(),
        "vanilla renderItemInHand skips the hands while the camera player sleeps",
    );

    assert!(
        first_person_player_arms(
            &world,
            &presentation,
            None,
            Some(&player.with_invisible(true)),
            Some(pose),
            1.0,
        )
        .is_empty(),
        "vanilla renderArmWithItem skips empty-hand player arms while invisible",
    );
    assert!(
        first_person_player_arms(&world, &presentation, None, Some(&player), None, 1.0).is_empty()
    );
}

#[test]
fn first_person_player_arms_render_two_handed_and_one_handed_map_arms() {
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 20.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let camera_world = first_person_camera_world_transform(pose);
    let parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::LEFT_SLEEVE_MASK | PlayerModelPartVisibility::RIGHT_SLEEVE_MASK,
    );
    let player = EntityModelInstance::player_with_parts(1, [0.0, 64.0, 0.0], 0.0, false, parts);
    let mut map = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    map.component_patch.map_id = Some(7);

    let mut main_map_world = WorldStore::new();
    main_map_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: map.clone(),
    });
    let main_presentation = FirstPersonHandPresentation::fully_raised_for_test(&main_map_world);
    let main_map_arms = first_person_player_arms(
        &main_map_world,
        &main_presentation,
        None,
        Some(&player),
        Some(pose),
        1.0,
    );
    assert_eq!(main_map_arms.len(), 2);
    assert_eq!(
        main_map_arms.iter().map(|arm| arm.left).collect::<Vec<_>>(),
        vec![false, true],
    );
    assert!(main_map_arms[0].sleeve_visible);
    assert!(main_map_arms[1].sleeve_visible);
    assert!(main_map_arms[0].transform.abs_diff_eq(
        first_person_two_handed_map_arm_transform(camera_world, pose.x_rot, 0.0, 0.0, false),
        1.0e-5,
    ));
    assert!(main_map_arms[1].transform.abs_diff_eq(
        first_person_two_handed_map_arm_transform(camera_world, pose.x_rot, 0.0, 0.0, true),
        1.0e-5,
    ));

    let mut offhand_map_world = WorldStore::new();
    offhand_map_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: map,
    });
    let offhand_presentation =
        FirstPersonHandPresentation::fully_raised_for_test(&offhand_map_world);
    let offhand_map_arms = first_person_player_arms(
        &offhand_map_world,
        &offhand_presentation,
        None,
        Some(&player),
        Some(pose),
        1.0,
    );
    assert_eq!(
        offhand_map_arms
            .iter()
            .map(|arm| arm.left)
            .collect::<Vec<_>>(),
        vec![false, true],
        "offhand maps leave the empty main hand visible and render one map hand",
    );
    assert!(offhand_map_arms[0].transform.abs_diff_eq(
        first_person_render_player_arm_transform(camera_world, false, 0.0, 0.0),
        1.0e-5,
    ));
    assert!(offhand_map_arms[1].transform.abs_diff_eq(
        first_person_one_handed_map_arm_transform(camera_world, true, 0.0, 0.0),
        1.0e-5,
    ));
}

#[test]
fn first_person_item_models_render_idle_spyglass_and_hide_when_scoping() {
    let root = unique_item_model_temp_dir("first-person-spyglass-use");
    write_flat_item_runtime_fixture(&root, &["spyglass", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let renderable_spyglass = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:spyglass"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let off_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:off_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: renderable_spyglass,
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut scoping_world = WorldStore::new();
    scoping_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: Some(item_types::VANILLA_ITEM_SPYGLASS_ID),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        },
    });
    scoping_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: off_item,
    });
    scoping_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    assert!(
        first_person_item_models(
            &scoping_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        )
        .flat_meshes
        .is_empty(),
        "vanilla ItemInHandRenderer skips hands/items while player.isScoping"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_block_use_pose_for_shield_and_blocks_attacks() {
    let root = unique_item_model_temp_dir("first-person-block-use");
    write_flat_item_runtime_fixture(&root, &["shield", "guard_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut shield = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:shield"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    shield.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];
    shield.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::None,
        duration: 0,
    });
    let mut guard_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:guard_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    guard_item.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID];
    let mut consumable_guard = guard_item.clone();
    consumable_guard
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID);
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut shield_world = world_with_item_runtime_defaults(&item_runtime);
    shield_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: shield,
    });
    let idle_shield = first_person_item_models(
        &shield_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(idle_shield.flat_meshes.len(), 1);
    shield_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    let blocking_shield = first_person_item_models(
        &shield_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(blocking_shield.flat_meshes.len(), 1);
    assert_eq!(
        idle_shield.flat_meshes[0], blocking_shield.flat_meshes[0],
        "vanilla ShieldItem BLOCK use keeps only the base arm transform"
    );

    let mut idle_guard_world = WorldStore::new();
    idle_guard_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: guard_item.clone(),
    });
    let idle_guard = first_person_item_models(
        &idle_guard_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(idle_guard.flat_meshes.len(), 1);

    let mut blocking_guard_world = WorldStore::new();
    blocking_guard_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: guard_item,
    });
    blocking_guard_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    let blocking_guard = first_person_item_models(
        &blocking_guard_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(blocking_guard.flat_meshes.len(), 1);
    assert_ne!(
        idle_guard.flat_meshes[0], blocking_guard.flat_meshes[0],
        "non-shield BLOCK use applies ItemInHandRenderer's fixed BLOCK transform"
    );

    assert_eq!(
        first_person_stack_block_use_kind(&WorldStore::new(), &consumable_guard, &item_runtime),
        None,
        "CONSUMABLE wins over BLOCKS_ATTACKS in Item.getUseAnimation"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_consumable_eat_and_drink_use_pose() {
    let root = unique_item_model_temp_dir("first-person-consumable-use");
    write_flat_item_runtime_fixture(&root, &["snack"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut snack = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:snack"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    snack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID];
    snack.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut eating_world = WorldStore::new();
    eating_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack.clone(),
    });
    eating_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    eating_world.advance_local_using_item_ticks(12);
    let eating = first_person_item_models(
        &eating_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(eating.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], eating.flat_meshes[0],
        "consumable EAT applies ItemInHandRenderer.applyEatTransform before the arm transform"
    );

    let mut drink = snack;
    drink.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Drink,
    });
    let mut drinking_world = WorldStore::new();
    drinking_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: drink,
    });
    drinking_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    drinking_world.advance_local_using_item_ticks(12);
    let drinking = first_person_item_models(
        &drinking_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(drinking.flat_meshes.len(), 1);
    assert_eq!(
        eating.flat_meshes[0], drinking.flat_meshes[0],
        "vanilla EAT and DRINK share the first-person applyEatTransform matrix"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_default_consumable_use_pose() {
    let root = unique_item_model_temp_dir("first-person-default-consumable-use");
    write_default_consumable_item_runtime_fixture(&root, "snack");
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let snack = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:snack"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = world_with_item_runtime_defaults(&item_runtime);
    assert_eq!(
        idle_world
            .effective_item_stack(&snack)
            .component_patch
            .consumable,
        Some(ConsumableSummary {
            consume_seconds: 1.6,
            animation: ItemUseAnimationSummary::Eat,
        })
    );
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut eating_world = world_with_item_runtime_defaults(&item_runtime);
    eating_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack,
    });
    eating_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    eating_world.advance_local_using_item_ticks(12);
    let eating = first_person_item_models(
        &eating_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(eating.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], eating.flat_meshes[0],
        "default item prototype CONSUMABLE applies vanilla EAT first-person use pose"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_custom_consumable_non_eat_use_animations() {
    let root = unique_item_model_temp_dir("first-person-custom-consumable-use");
    write_flat_item_runtime_fixture(&root, &["snack", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let snack_bow = first_person_test_consumable_stack(
        &item_runtime,
        "minecraft:snack",
        ItemUseAnimationSummary::Bow,
        1.6,
    );
    assert_eq!(
        first_person_stack_supported_use_animation(&WorldStore::new(), &snack_bow, &item_runtime),
        Some(FirstPersonUseAnimation::Bow {
            use_duration_ticks: 32.0,
        })
    );
    assert_eq!(
        first_person_stack_supported_use_animation(
            &WorldStore::new(),
            &first_person_test_consumable_stack(
                &item_runtime,
                "minecraft:snack",
                ItemUseAnimationSummary::Trident,
                1.6,
            ),
            &item_runtime,
        ),
        Some(FirstPersonUseAnimation::Trident {
            use_duration_ticks: 32.0,
        })
    );
    assert_eq!(
        first_person_stack_supported_use_animation(
            &WorldStore::new(),
            &first_person_test_consumable_stack(
                &item_runtime,
                "minecraft:snack",
                ItemUseAnimationSummary::Brush,
                1.6,
            ),
            &item_runtime,
        ),
        Some(FirstPersonUseAnimation::Brush {
            use_duration_ticks: 32.0,
        })
    );
    assert_eq!(
        first_person_stack_supported_use_animation(
            &WorldStore::new(),
            &first_person_test_consumable_stack(
                &item_runtime,
                "minecraft:snack",
                ItemUseAnimationSummary::Bundle,
                1.6,
            ),
            &item_runtime,
        ),
        Some(FirstPersonUseAnimation::Bundle)
    );
    for animation in [
        ItemUseAnimationSummary::None,
        ItemUseAnimationSummary::Crossbow,
        ItemUseAnimationSummary::Spyglass,
        ItemUseAnimationSummary::TootHorn,
    ] {
        assert_eq!(
            first_person_stack_supported_use_animation(
                &WorldStore::new(),
                &first_person_test_consumable_stack(
                    &item_runtime,
                    "minecraft:snack",
                    animation,
                    1.6,
                ),
                &item_runtime,
            ),
            Some(FirstPersonUseAnimation::None),
            "generic {animation:?} has no ItemInHandRenderer switch case"
        );
    }
    assert_eq!(
        first_person_stack_supported_use_animation(
            &WorldStore::new(),
            &first_person_test_consumable_stack(
                &item_runtime,
                "minecraft:snack",
                ItemUseAnimationSummary::Spear,
                1.6,
            ),
            &item_runtime,
        ),
        None,
        "SPEAR needs the kinetic SpearAnimations.firstPersonUse path"
    );

    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let off_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:off_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack_bow.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut using_world = WorldStore::new();
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: snack_bow,
    });
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: off_item,
    });
    using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    using_world.advance_local_using_item_ticks(12);
    let using = first_person_item_models(
        &using_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(
        using.flat_meshes.len(),
        2,
        "generic BOW animation is not an Items.BOW/CROSSBOW hand-selection special case"
    );
    assert_ne!(
        idle.flat_meshes[0], using.flat_meshes[0],
        "generic BOW consumables still apply the vanilla draw transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_goat_horn_base_use_pose() {
    let root = unique_item_model_temp_dir("first-person-goat-horn-use");
    write_flat_item_runtime_fixture(&root, &["goat_horn"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut goat_horn = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:goat_horn"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    goat_horn.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];
    goat_horn.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::None,
        duration: 0,
    });
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: goat_horn.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut tooting_world = WorldStore::new();
    tooting_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: goat_horn,
    });
    tooting_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    let tooting = first_person_item_models(
        &tooting_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(tooting.flat_meshes.len(), 1);
    assert_eq!(
        idle.flat_meshes[0], tooting.flat_meshes[0],
        "vanilla TOOT_HORN first-person use keeps only the base arm transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_brush_use_pose() {
    let root = unique_item_model_temp_dir("first-person-brush-use");
    write_flat_item_runtime_fixture(&root, &["brush"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let brush = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:brush"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut edible_brush = brush.clone();
    edible_brush.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID];
    edible_brush.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    assert_eq!(
        first_person_stack_supported_use_animation(
            &WorldStore::new(),
            &edible_brush,
            &item_runtime
        ),
        Some(FirstPersonUseAnimation::Brush {
            use_duration_ticks: VANILLA_BRUSH_USE_DURATION_TICKS,
        }),
        "BrushItem.getUseAnimation overrides stack CONSUMABLE data"
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: brush.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut brushing_world = WorldStore::new();
    brushing_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: brush,
    });
    brushing_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    brushing_world.advance_local_using_item_ticks(12);
    let brushing = first_person_item_models(
        &brushing_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(brushing.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], brushing.flat_meshes[0],
        "BRUSH use applies ItemInHandRenderer.applyBrushTransform after the arm transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_bundle_use_swing() {
    let root = unique_item_model_temp_dir("first-person-bundle-use");
    write_flat_item_runtime_fixture(&root, &["bundle"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let bundle = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:bundle"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut edible_bundle = bundle.clone();
    edible_bundle.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID];
    edible_bundle.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    assert_eq!(
        first_person_stack_supported_use_animation(
            &WorldStore::new(),
            &edible_bundle,
            &item_runtime
        ),
        Some(FirstPersonUseAnimation::Bundle),
        "BundleItem.getUseAnimation overrides stack CONSUMABLE data"
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut world = world_with_level_dimension("minecraft:overworld");
    world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: bundle,
    });
    world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    let still = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(still.flat_meshes.len(), 1);

    assert!(world.apply_entity_animation(EntityAnimation { id: 1, action: 0 }));
    world.advance_entity_client_animations(2);
    let swinging = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(swinging.flat_meshes.len(), 1);
    assert_ne!(
        still.flat_meshes[0], swinging.flat_meshes[0],
        "BUNDLE use applies ItemInHandRenderer.swingArm while the item is being used"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_trident_use_pose() {
    let root = unique_item_model_temp_dir("first-person-trident-use");
    write_flat_item_runtime_fixture(&root, &["trident"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let trident = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:trident"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut edible_trident = trident.clone();
    edible_trident.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID];
    edible_trident.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    assert_eq!(
        first_person_stack_supported_use_animation(
            &WorldStore::new(),
            &edible_trident,
            &item_runtime
        ),
        Some(FirstPersonUseAnimation::Trident {
            use_duration_ticks: VANILLA_TRIDENT_USE_DURATION_TICKS,
        }),
        "TridentItem.getUseAnimation overrides stack CONSUMABLE data"
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: trident.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut using_world = WorldStore::new();
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: trident,
    });
    using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    using_world.advance_local_using_item_ticks(12);
    let using = first_person_item_models(
        &using_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(using.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], using.flat_meshes[0],
        "TRIDENT use applies ItemInHandRenderer's throw-charge transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_bow_use_pose_and_hand_selection() {
    let root = unique_item_model_temp_dir("first-person-bow-use");
    write_flat_item_runtime_fixture(&root, &["bow", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let bow = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:bow"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let off_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:off_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut edible_bow = bow.clone();
    edible_bow.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID];
    edible_bow.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    assert_eq!(
        first_person_stack_supported_use_animation(&WorldStore::new(), &edible_bow, &item_runtime),
        Some(FirstPersonUseAnimation::Bow {
            use_duration_ticks: VANILLA_BOW_USE_DURATION_TICKS,
        }),
        "BowItem.getUseAnimation overrides stack CONSUMABLE data"
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: bow.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut using_world = WorldStore::new();
    using_world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item: bow });
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: off_item,
    });
    using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    using_world.advance_local_using_item_ticks(12);
    let using = first_person_item_models(
        &using_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(
        using.flat_meshes.len(),
        1,
        "vanilla renders only the used hand while drawing a bow"
    );
    assert_ne!(
        idle.flat_meshes[0], using.flat_meshes[0],
        "BOW use applies ItemInHandRenderer's draw transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_use_duration_range_dispatch_reads_local_use_ticks() {
    // Vanilla `ItemInHandRenderer.renderArmWithItem` passes the local player
    // as item owner for first-person stack rendering; `UseDuration.get` then
    // reads that local player's active use item and elapsed ticks.
    let root = unique_item_model_temp_dir("first-person-use-duration-range-dispatch");
    write_use_duration_selector_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = first_person_test_consumable_stack(
        &item_runtime,
        "minecraft:use_duration_selector",
        ItemUseAnimationSummary::None,
        2.0,
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let render = |using_ticks: Option<u32>| {
        let mut world = WorldStore::new();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: stack.clone(),
        });
        if let Some(ticks) = using_ticks {
            world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
            world.advance_local_using_item_ticks(ticks);
        }
        let mut models = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            0.5,
        );
        assert_eq!(models.flat_meshes.len(), 1);
        models.flat_meshes.remove(0)
    };

    let idle = render(None);
    let using_start = render(Some(0));
    let using_mid = render(Some(13));
    let using_full = render(Some(18));

    assert_ne!(idle, using_start);
    assert_ne!(using_start, using_mid);
    assert_ne!(using_mid, using_full);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_crossbow_use_and_charged_pose() {
    let root = unique_item_model_temp_dir("first-person-crossbow-use");
    write_flat_item_runtime_fixture(&root, &["crossbow", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let crossbow = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:crossbow"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let off_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:off_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut edible_crossbow = crossbow.clone();
    edible_crossbow.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID];
    edible_crossbow.component_patch.consumable = Some(ConsumableSummary {
        consume_seconds: 1.6,
        animation: ItemUseAnimationSummary::Eat,
    });
    assert_eq!(
        first_person_stack_supported_use_animation(
            &WorldStore::new(),
            &edible_crossbow,
            &item_runtime
        ),
        Some(FirstPersonUseAnimation::Crossbow {
            use_duration_ticks: VANILLA_CROSSBOW_USE_DURATION_TICKS,
        }),
        "CrossbowItem.getUseAnimation overrides stack CONSUMABLE data"
    );
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut idle_world = WorldStore::new();
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: crossbow.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut using_world = WorldStore::new();
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: crossbow.clone(),
    });
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: off_item,
    });
    using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    using_world.advance_local_using_item_ticks(12);
    let using = first_person_item_models(
        &using_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(
        using.flat_meshes.len(),
        1,
        "vanilla renders only the used hand while drawing a crossbow"
    );
    assert_ne!(
        idle.flat_meshes[0], using.flat_meshes[0],
        "uncharged CROSSBOW use applies ItemInHandRenderer's draw transform"
    );

    let mut quick_charge_crossbow = crossbow.clone();
    quick_charge_crossbow.component_patch.enchantments =
        vec![bbb_protocol::packets::ItemEnchantmentSummary {
            holder_id: 1,
            level: 2,
        }];
    let mut quick_charge_world = world_with_level_dimension("minecraft:overworld");
    quick_charge_world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
    record_enchantment_registry(&mut quick_charge_world);
    quick_charge_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: quick_charge_crossbow,
    });
    quick_charge_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    quick_charge_world.advance_local_using_item_ticks(12);
    let quick_charge = first_person_item_models(
        &quick_charge_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(quick_charge.flat_meshes.len(), 1);
    assert_ne!(
        using.flat_meshes[0], quick_charge.flat_meshes[0],
        "synchronized Quick Charge changes the first-person draw transform"
    );

    let mut charged_crossbow = crossbow;
    charged_crossbow.component_patch.charged_projectiles_items = vec![ItemStackTemplateSummary {
        item_id: 99,
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    }];
    let mut charged_world = WorldStore::new();
    charged_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: charged_crossbow,
    });
    let charged = first_person_item_models(
        &charged_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(charged.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], charged.flat_meshes[0],
        "charged main-hand crossbows apply the vanilla idle hold offset"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_stack_supported_use_animation_resolves_spear_kinetic_weapon() {
    let root = unique_item_model_temp_dir("first-person-spear-use-animation");
    write_flat_item_runtime_fixture(&root, &["wooden_spear", "snack"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let world = world_with_item_runtime_defaults(&item_runtime);
    let kinetic_weapon = SpearKineticWeapon {
        delay_ticks: 15.0,
        dismount_duration_ticks: 100.0,
        knockback_duration_ticks: 200.0,
        damage_duration_ticks: 300.0,
        forward_movement: 0.38,
    };
    let wooden_spear = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:wooden_spear"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    assert_eq!(
        first_person_stack_supported_use_animation(&world, &wooden_spear, &item_runtime),
        Some(FirstPersonUseAnimation::Spear {
            kinetic_weapon,
            use_duration_ticks: VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS,
        })
    );

    let mut removed_kinetic = wooden_spear.clone();
    removed_kinetic
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID);
    assert_eq!(
        first_person_stack_supported_use_animation(&world, &removed_kinetic, &item_runtime),
        None,
        "removing the prototype KINETIC_WEAPON component disables vanilla SPEAR use"
    );

    let mut overridden = wooden_spear.clone();
    overridden
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID);
    let kinetic = world
        .item_profiles()
        .component_source(
            wooden_spear.item_id,
            &wooden_spear.component_patch,
            VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        )
        .kinetic_weapon
        .clone();
    let mut kinetic = kinetic.unwrap();
    kinetic.delay_ticks = 7;
    kinetic.dismount_conditions = None;
    kinetic.forward_movement_bits = 0.75_f32.to_bits();
    overridden.component_patch.kinetic_weapon = Some(kinetic);
    assert_eq!(
        first_person_stack_supported_use_animation(&world, &overridden, &item_runtime),
        Some(FirstPersonUseAnimation::Spear {
            kinetic_weapon: SpearKineticWeapon {
                delay_ticks: 7.0,
                dismount_duration_ticks: 0.0,
                forward_movement: 0.75,
                ..kinetic_weapon
            },
            use_duration_ticks: VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS,
        }),
    );
    overridden
        .component_patch
        .removed_type_ids
        .push(VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID);
    assert_eq!(
        first_person_stack_supported_use_animation(&world, &overridden, &item_runtime),
        None
    );

    let snack_spear = first_person_test_consumable_stack(
        &item_runtime,
        "minecraft:snack",
        ItemUseAnimationSummary::Spear,
        1.6,
    );
    assert_eq!(
        first_person_stack_supported_use_animation(&world, &snack_spear, &item_runtime),
        None,
        "generic SPEAR consumables need readable kinetic weapon data"
    );

    let wooden_spear_consumable = first_person_test_consumable_stack(
        &item_runtime,
        "minecraft:wooden_spear",
        ItemUseAnimationSummary::Spear,
        1.6,
    );
    assert_eq!(
        first_person_stack_supported_use_animation(&world, &wooden_spear_consumable, &item_runtime),
        Some(FirstPersonUseAnimation::Spear {
            kinetic_weapon,
            use_duration_ticks: 32.0,
        }),
        "consumable SPEAR keeps the consumable use duration while using the item kinetic component"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_spear_kinetic_use_pose_and_feedback() {
    let root = unique_item_model_temp_dir("first-person-spear-kinetic-use");
    write_flat_item_runtime_fixture(&root, &["wooden_spear"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let wooden_spear = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:wooden_spear"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let mut idle_world = world_with_level_dimension("minecraft:overworld");
    crate::resource_reload::apply_item_runtime_world_defaults(&item_runtime, &mut idle_world)
        .unwrap();
    idle_world.rebuild_item_component_defaults().unwrap();
    idle_world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
    idle_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: wooden_spear.clone(),
    });
    let idle = first_person_item_models(
        &idle_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(idle.flat_meshes.len(), 1);

    let mut using_world = world_with_level_dimension("minecraft:overworld");
    crate::resource_reload::apply_item_runtime_world_defaults(&item_runtime, &mut using_world)
        .unwrap();
    using_world.rebuild_item_component_defaults().unwrap();
    using_world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: wooden_spear.clone(),
    });
    using_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    using_world.advance_local_using_item_ticks(20);
    let using = first_person_item_models(
        &using_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(using.flat_meshes.len(), 1);
    assert_ne!(
        idle.flat_meshes[0], using.flat_meshes[0],
        "SPEAR use skips applyItemArmTransform and applies SpearAnimations.firstPersonUse"
    );

    let mut feedback_world = world_with_level_dimension("minecraft:overworld");
    crate::resource_reload::apply_item_runtime_world_defaults(&item_runtime, &mut feedback_world)
        .unwrap();
    feedback_world.rebuild_item_component_defaults().unwrap();
    feedback_world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
    feedback_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: wooden_spear,
    });
    feedback_world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    feedback_world.advance_local_using_item_ticks(20);
    assert!(feedback_world.apply_entity_event(EntityEvent {
        entity_id: 1,
        event_id: 2,
    }));
    feedback_world.advance_entity_client_animations(2);
    let feedback = first_person_item_models(
        &feedback_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        0.5,
    );
    assert_eq!(feedback.flat_meshes.len(), 1);
    assert_eq!(
        feedback_world.local_player_ticks_since_kinetic_hit_feedback(0.5),
        2.5
    );
    assert_ne!(
        using.flat_meshes[0], feedback.flat_meshes[0],
        "local kinetic hit feedback contributes the vanilla first-person spear kick"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_stab_swing_animation_for_spear_and_stack_patch() {
    let root = unique_item_model_temp_dir("first-person-stab-swing");
    write_flat_item_runtime_fixture(&root, &["stab_item", "wooden_spear"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut patched_stab = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:stab_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    patched_stab.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];
    patched_stab.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::Stab,
        duration: 13,
    });
    let default_spear = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:wooden_spear"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let assert_stab_moves = |item: ItemStackSummary, label: &str| {
        let mut world = world_with_level_dimension("minecraft:overworld");
        crate::resource_reload::apply_item_runtime_world_defaults(&item_runtime, &mut world)
            .unwrap();
        world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
        world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item });

        let still = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(still.flat_meshes.len(), 1, "{label} should render");

        assert!(world.apply_entity_animation(EntityAnimation { id: 1, action: 0 }));
        world.advance_entity_client_animations(2);
        let swinging = first_person_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        );
        assert_eq!(
            swinging.flat_meshes.len(),
            1,
            "{label} should keep rendering"
        );
        assert_ne!(
            still.flat_meshes[0], swinging.flat_meshes[0],
            "{label} should receive vanilla SpearAnimations.firstPersonAttack"
        );
    };
    assert_stab_moves(patched_stab, "stack-patch STAB");
    assert_stab_moves(default_spear, "default spear STAB");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_swing_animation_none_and_removal_use_stack_components() {
    let world = WorldStore::new();
    let mut none_stack = ItemStackSummary {
        item_id: Some(1),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    none_stack.component_patch.added_type_ids =
        vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];
    none_stack.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::None,
        duration: 6,
    });
    assert_eq!(
        first_person_stack_swing_animation(&world, &none_stack),
        FirstPersonSwingAnimation::None
    );

    none_stack.component_patch.removed_type_ids =
        vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];
    assert_eq!(
        first_person_stack_swing_animation(&world, &none_stack),
        FirstPersonSwingAnimation::Whack,
        "removing swing_animation falls back to vanilla SwingAnimation.DEFAULT WHACK",
    );
}

#[test]
fn first_person_swing_animation_uses_custom_prototype_type() {
    let mut world = WorldStore::new();
    world.set_default_item_component_codec_values(
        [(0, "none", 5), (1, "whack", 7), (2, "stab", 9)]
            .into_iter()
            .map(|(item_id, animation_type, duration)| {
                (
                    item_id,
                    BTreeMap::from([(
                        VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
                        serde_json::json!({"type": animation_type, "duration": duration}),
                    )]),
                )
            })
            .collect(),
    );
    world.rebuild_item_component_defaults().unwrap();

    for (item_id, expected) in [
        (0, FirstPersonSwingAnimation::None),
        (1, FirstPersonSwingAnimation::Whack),
        (2, FirstPersonSwingAnimation::Stab),
    ] {
        let stack = ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        };
        assert_eq!(first_person_stack_swing_animation(&world, &stack), expected);
    }
}
