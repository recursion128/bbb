use super::*;

#[test]
fn first_person_view_lag_follows_turns_settles_and_applies_to_items_and_bare_arms() {
    let root = unique_item_model_temp_dir("first-person-view-lag");
    write_flat_item_runtime_fixture(&root, &["hand_item"]);
    let runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = world_with_level_dimension("minecraft:overworld");
    world.set_local_player_pose(bbb_client_world::LocalPlayerPoseState::default());
    let mut presentation = FirstPersonHandPresentation::fully_raised_for_test(&world);
    presentation.advance_for_ticks(&world, Some(&runtime), false, 1);
    assert_eq!(
        presentation.view_rotation_transform(&world, 1.0),
        Mat4::IDENTITY
    );

    world.set_local_player_pose(bbb_client_world::LocalPlayerPoseState {
        x_rot: 30.0,
        y_rot: 90.0,
        ..Default::default()
    });
    presentation.advance_for_ticks(&world, Some(&runtime), false, 1);
    let lag =
        Mat4::from_rotation_x(1.5_f32.to_radians()) * Mat4::from_rotation_y(4.5_f32.to_radians());
    assert!(presentation
        .view_rotation_transform(&world, 1.0)
        .abs_diff_eq(lag, 1.0e-6));
    assert!(presentation
        .view_rotation_transform(&world, 0.0)
        .abs_diff_eq(
            Mat4::from_rotation_x(3.0_f32.to_radians())
                * Mat4::from_rotation_y(9.0_f32.to_radians()),
            1.0e-6,
        ));

    let camera = CameraPose {
        position: [0.0, 0.0, 0.0],
        y_rot: 90.0,
        x_rot: 30.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let player = EntityModelInstance::player_with_skin(
        1,
        [0.0, 64.0, 0.0],
        0.0,
        EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
        PlayerModelPartVisibility::from_vanilla_mask(0x7f),
    );
    let arms = first_person_player_arms(
        &world,
        &presentation,
        Some(&runtime),
        Some(&player),
        Some(camera),
        1.0,
    );
    assert!(arms[0].transform.abs_diff_eq(
        first_person_render_player_arm_transform(
            first_person_camera_world_transform(camera) * lag,
            false,
            presentation.interpolated_inverse_height(
                InteractionHand::MainHand,
                Some(&runtime),
                1.0
            ),
            0.0,
        ),
        1.0e-5,
    ));

    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: runtime.item_protocol_id("minecraft:hand_item"),
            count: 1,
            component_patch: Default::default(),
        },
    });
    let mut item_presentation = FirstPersonHandPresentation::fully_raised_for_test(&world);
    world.set_local_player_pose(Default::default());
    item_presentation.advance_for_ticks(&world, Some(&runtime), false, 1);
    world.set_local_player_pose(bbb_client_world::LocalPlayerPoseState {
        x_rot: 30.0,
        y_rot: 90.0,
        ..Default::default()
    });
    item_presentation.advance_for_ticks(&world, Some(&runtime), false, 1);
    let registry_keys = ItemRegistryKeySet::from_world(&world);
    let mut map_cache = MapTextureExtractionCache::default();
    map_cache.begin_frame();
    let moving = first_person_item_models_with_map_cache(
        &world,
        &item_presentation,
        Some(&runtime),
        &TerrainTextureState::default(),
        None,
        &registry_keys,
        &mut map_cache,
        Some(camera),
        1.0,
        1.0,
    );
    let still = first_person_item_models(
        &world,
        Some(&runtime),
        &TerrainTextureState::default(),
        Some(camera),
        1.0,
    );
    let camera_world = first_person_camera_world_transform(camera);
    let expected = first_person_item_arm_transform_from_camera_world(
        camera_world * lag,
        false,
        item_presentation.interpolated_inverse_height(
            InteractionHand::MainHand,
            Some(&runtime),
            1.0,
        ),
    ) * first_person_item_arm_transform_from_camera_world(camera_world, false, 0.0)
        .inverse()
        * still.residency.instances[0].transform;
    assert!(moving.residency.instances[0]
        .transform
        .abs_diff_eq(expected, 1.0e-5));
    item_presentation.advance_for_ticks(&world, Some(&runtime), false, 12);
    assert!(item_presentation
        .view_rotation_transform(&world, 1.0)
        .abs_diff_eq(Mat4::IDENTITY, 2.0e-5,));

    world.set_local_player_pose(bbb_client_world::LocalPlayerPoseState {
        y_rot: 179.0,
        ..Default::default()
    });
    let mut wrapped = FirstPersonHandPresentation::fully_raised_for_test(&world);
    wrapped.advance_for_ticks(&world, Some(&runtime), false, 1);
    world.set_local_player_pose(bbb_client_world::LocalPlayerPoseState {
        y_rot: -179.0,
        ..Default::default()
    });
    wrapped.advance_for_ticks(&world, Some(&runtime), false, 1);
    assert!(wrapped
        .view_rotation_transform(&world, 0.5)
        .abs_diff_eq(Mat4::from_rotation_y(0.15_f32.to_radians()), 1.0e-6,));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_riptide_replaces_swing_until_auto_spin_finishes() {
    let root = unique_item_model_temp_dir("first-person-riptide");
    write_flat_item_runtime_fixture(&root, &["trident"]);
    let runtime = NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap())
        .unwrap()
        .with_vanilla_item_registry_for_test();
    let mut world = world_with_level_dimension("minecraft:overworld");
    insert_entity_block_test_chunk(&mut world);
    set_entity_block_test_block(
        &mut world,
        BlockPos { x: 0, y: -64, z: 0 },
        bbb_client_world::vanilla_block_state_id(
            "minecraft:water",
            &BTreeMap::from([("level".to_string(), "0".to_string())]),
        )
        .unwrap(),
    );
    world.set_local_player_pose(bbb_client_world::LocalPlayerPoseState {
        position: Vec3d {
            x: 0.0,
            y: -64.0,
            z: 0.0,
        },
        ..Default::default()
    });
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
            item_id: Some(item_types::VANILLA_ITEM_TRIDENT_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                ],
                enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 2,
                }],
                ..Default::default()
            }
            .into(),
        },
    });
    let camera = CameraPose {
        position: [0.0, 0.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let idle = first_person_item_models(
        &world,
        Some(&runtime),
        &TerrainTextureState::default(),
        Some(camera),
        1.0,
    );
    world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
    world.advance_local_using_item_ticks(10);
    assert!(world.release_local_using_item());
    assert_eq!(
        world
            .local_player_auto_spin_attack()
            .unwrap()
            .ticks_remaining,
        20
    );
    let spin = first_person_item_models(
        &world,
        Some(&runtime),
        &TerrainTextureState::default(),
        Some(camera),
        1.0,
    );
    let attach = first_person_item_arm_transform_from_camera_world(
        first_person_camera_world_transform(camera),
        false,
        0.0,
    );
    let expected = attach
        * Mat4::from_translation(Vec3::new(-0.4, 0.8, 0.3))
        * Mat4::from_rotation_y(65.0_f32.to_radians())
        * Mat4::from_rotation_z(-85.0_f32.to_radians())
        * attach.inverse()
        * idle.residency.instances[0].transform;
    assert!(spin.residency.instances[0]
        .transform
        .abs_diff_eq(expected, 1.0e-5));
    for _ in 0..20 {
        world.advance_local_player_input(bbb_client_world::LocalPlayerInputState::default(), 0.05);
    }
    assert_eq!(
        world
            .local_player_auto_spin_attack()
            .unwrap()
            .ticks_remaining,
        0
    );
    let stopped = first_person_item_models(
        &world,
        Some(&runtime),
        &TerrainTextureState::default(),
        Some(camera),
        1.0,
    );
    assert!(stopped.residency.instances[0]
        .transform
        .abs_diff_eq(idle.residency.instances[0].transform, 1.0e-5,));
    std::fs::remove_dir_all(root).unwrap();
}
