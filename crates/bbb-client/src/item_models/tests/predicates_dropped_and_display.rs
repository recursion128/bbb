use super::*;

#[test]
fn held_generated_item_main_hand_select_uses_owner_main_arm_context() {
    // Vanilla `MainHand.get` returns `owner.getMainArm()`. Keep the attach
    // transform and physical hand fixed here so the mesh difference proves
    // the selected generated-item texture branch changed.
    let root = unique_item_model_temp_dir("held-main-hand-select");
    write_main_hand_select_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let world = WorldStore::new();
    let stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let bake = |owner_main_hand_left| {
        let mut residency = ItemModelResidencyFrame::default();
        crate::item_models::bake_item_stack(
            &crate::item_models::ItemBakeEnv {
                world: &world,
                item_runtime: &item_runtime,
                variant_textures: None,
                registry_keys: Default::default(),
            },
            &stack,
            BlockModelDisplayContext::ThirdPersonRightHand,
            &crate::item_models::ItemOwner {
                main_hand_left: owner_main_hand_left,
                ..Default::default()
            },
            &crate::item_models::ItemPlacement::attached(
                Mat4::IDENTITY,
                BlockModelDisplayContext::ThirdPersonRightHand,
                false,
                BLOCK_THIRD_PERSON_FALLBACK,
                GENERATED_THIRD_PERSON_FALLBACK,
            ),
            &mut residency,
        );
        assert_eq!(residency.assets.len(), 1);
        residency.assets.remove(0)
    };

    let fallback = bake(None);
    let right = bake(Some(false));
    let left = bake(Some(true));

    assert_ne!(fallback, right);
    assert_ne!(fallback, left);
    assert_ne!(right, left);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn held_generated_item_context_entity_type_select_uses_owner_kind() {
    // Vanilla `ContextEntityType.get` returns the living owner entity type
    // key. Keep the transform fixed so mesh changes prove the generated
    // item texture branch changed, not the hand pose.
    let root = unique_item_model_temp_dir("held-context-entity-type-select");
    write_context_entity_type_select_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let world = WorldStore::new();
    let stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let bake = |context_entity_type| {
        let mut residency = ItemModelResidencyFrame::default();
        crate::item_models::bake_item_stack(
            &crate::item_models::ItemBakeEnv {
                world: &world,
                item_runtime: &item_runtime,
                variant_textures: None,
                registry_keys: Default::default(),
            },
            &stack,
            BlockModelDisplayContext::ThirdPersonRightHand,
            &crate::item_models::ItemOwner {
                entity_type: context_entity_type,
                main_hand_left: Some(false),
                ..Default::default()
            },
            &crate::item_models::ItemPlacement::attached(
                Mat4::IDENTITY,
                BlockModelDisplayContext::ThirdPersonRightHand,
                false,
                BLOCK_THIRD_PERSON_FALLBACK,
                GENERATED_THIRD_PERSON_FALLBACK,
            ),
            &mut residency,
        );
        assert_eq!(residency.assets.len(), 1);
        residency.assets.remove(0)
    };

    let skin = EntityPlayerSkin::Default(EntityDefaultPlayerSkin::SlimAlex);
    let parts = PlayerModelPartVisibility::from_vanilla_mask(0);
    let player_kind = EntityModelKind::Player { skin, parts };
    assert_eq!(
        entity_model_context_entity_type(player_kind),
        Some("minecraft:player")
    );
    assert_eq!(
        entity_model_context_entity_type(EntityModelKind::Witch),
        Some("minecraft:witch")
    );

    let fallback = bake(None);
    let player = bake(entity_model_context_entity_type(player_kind));
    let witch = bake(entity_model_context_entity_type(EntityModelKind::Witch));

    assert_ne!(fallback, player);
    assert_ne!(fallback, witch);
    assert_ne!(player, witch);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn non_living_item_billboards_use_null_context_entity_type_select() {
    // Vanilla `ItemModelResolver.updateForNonLiving` carries an `ItemOwner`,
    // but `SelectItemModel.update` narrows it through `asLivingEntity()`.
    // ItemEntity, Snowball, and FireworkRocket therefore all select the null
    // fallback rather than a case for the rendered entity's own type.
    let root = unique_item_model_temp_dir("non-living-context-entity-type-select");
    write_context_entity_type_select_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let fallback = item_runtime
        .resolve_item_model(
            &stack,
            &bbb_client_render::item_model::ItemModelContext::new(BlockModelDisplayContext::Gui),
        )
        .and_then(|model| model.flat_icon())
        .expect("fallback icon");
    let fallback_uv = fallback.layers[0].uv;
    let mut world = WorldStore::new();

    const ITEM_ENTITY_ID: i32 = 620;
    world.apply_add_entity(protocol_add_entity(
        ITEM_ENTITY_ID,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    let item_data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ITEM_ID,
    )
    .expect("generated ItemEntity item accessor");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: ITEM_ENTITY_ID,
        values: vec![EntityDataValue {
            data_id: item_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(stack.clone()),
        }],
    }));

    const SNOWBALL_ENTITY_ID: i32 = 621;
    world.apply_add_entity(protocol_add_entity(
        SNOWBALL_ENTITY_ID,
        VANILLA_ENTITY_TYPE_SNOWBALL_ID,
    ));
    let snowball_item_data_id =
        bbb_client_world::vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
            VANILLA_ENTITY_TYPE_SNOWBALL_ID,
            "ITEM_STACK",
        )
        .expect("generated Snowball item accessor")
        .data_id;
    assert!(world.apply_set_entity_data(SetEntityData {
        id: SNOWBALL_ENTITY_ID,
        values: vec![EntityDataValue {
            data_id: snowball_item_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(stack.clone()),
        }],
    }));

    const FIREWORK_ENTITY_ID: i32 = 622;
    world.apply_add_entity(protocol_add_entity(
        FIREWORK_ENTITY_ID,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    ));
    let firework_accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    )
    .expect("generated FireworkRocket metadata accessors");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: FIREWORK_ENTITY_ID,
        values: vec![
            EntityDataValue {
                data_id: firework_accessors.item_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                value: EntityDataValueKind::ItemStack(stack),
            },
            EntityDataValue {
                data_id: firework_accessors.shot_at_angle_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));

    let mut candidates = Vec::new();
    world.fill_entity_render_candidates_at_partial_tick(1.0, &mut candidates);
    let dropped: Vec<_> = world.item_entity_stacks().collect();
    let billboards = crate::item_entities::item_entity_billboards_from_world(
        &world,
        &candidates,
        &dropped,
        Some(&item_runtime),
        &BTreeSet::new(),
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    assert_eq!(billboards.len(), 3);
    assert!(billboards
        .iter()
        .all(|billboard| billboard.layers[0].uv.min == fallback_uv.min
            && billboard.layers[0].uv.max == fallback_uv.max));
    assert!(billboards.iter().any(|billboard| {
        billboard.position[1] == 64.25
            && billboard.orientation
                == bbb_client_render::ItemEntityBillboardOrientation::CameraFacing
    }));
    assert!(billboards.iter().any(|billboard| {
        billboard.position[1] == 64.0
            && billboard.orientation
                == bbb_client_render::ItemEntityBillboardOrientation::CameraFacing
    }));
    assert!(billboards.iter().any(|billboard| {
        billboard.orientation
            == bbb_client_render::ItemEntityBillboardOrientation::FireworkShotAtAngle
    }));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn held_generated_item_context_dimension_select_uses_world_level() {
    // Vanilla `ContextDimension.get` returns `level.dimension()` when the
    // item resolver is called with a `ClientLevel`. The held-item path uses
    // the real world level, so the same stack can select different
    // generated textures in different dimensions.
    let root = unique_item_model_temp_dir("held-context-dimension-select");
    write_context_dimension_select_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let terrain_textures = TerrainTextureState::default();
    const ENTITY_ID: i32 = 610;
    let bake = |dimension: Option<&str>| {
        let mut world = dimension.map_or_else(WorldStore::new, world_with_level_dimension);
        world.apply_add_entity(protocol_add_entity(
            ENTITY_ID,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
        ));
        assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
        let instance = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
        let models = held_item_models(&[instance], &world, Some(&item_runtime), &terrain_textures);
        assert_eq!(models.flat_meshes.len(), 1);
        models.flat_meshes[0].clone()
    };

    let fallback = bake(None);
    let overworld = bake(Some("minecraft:overworld"));
    let nether = bake(Some("minecraft:the_nether"));

    assert_ne!(fallback, overworld);
    assert_ne!(fallback, nether);
    assert_ne!(overworld, nether);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn held_generated_item_trim_material_select_uses_world_registry() {
    // Vanilla `TrimMaterialProperty.get` reads the stack's TRIM component
    // and unwraps the synced trim-material registry key. Held generated
    // items therefore need the same world registry context as GUI/dropped
    // item consumers.
    let root = unique_item_model_temp_dir("held-trim-material-select");
    write_trim_material_select_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let terrain_textures = TerrainTextureState::default();
    const ENTITY_ID: i32 = 611;
    let bake = |with_registry: bool, material_id: Option<i32>| {
        let mut world = WorldStore::new();
        if with_registry {
            record_trim_material_registry(&mut world);
        }
        world.apply_add_entity(protocol_add_entity(
            ENTITY_ID,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
        ));
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: ENTITY_ID,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary {
                    item_id: Some(0),
                    count: 1,
                    component_patch: DataComponentPatchSummary {
                        armor_trim_material_id: material_id,
                        ..DataComponentPatchSummary::default()
                    }
                    .into(),
                },
            }],
        }));
        let instance = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
        let models = held_item_models(&[instance], &world, Some(&item_runtime), &terrain_textures);
        assert_eq!(models.flat_meshes.len(), 1);
        models.flat_meshes[0].clone()
    };

    let fallback = bake(false, Some(0));
    let quartz = bake(true, Some(0));
    let diamond = bake(true, Some(2));

    assert_ne!(fallback, quartz);
    assert_ne!(fallback, diamond);
    assert_ne!(quartz, diamond);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn humanoid_held_generated_item_using_item_condition_matches_used_hand() {
    // Vanilla `IsUsingItem.get` is true only when the submitted stack is the
    // owner's active `getUseItem()` stack, not merely because the owner is
    // using some item in the other hand.
    let root = unique_item_model_temp_dir("held-using-item-condition");
    write_using_item_condition_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 607;
    world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    let terrain_textures = TerrainTextureState::default();
    let base = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);

    let idle_models = held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
    let main_using_models = held_item_models(
        &[base.with(|s| (s.is_using_item, s.use_item_off_hand) = (true, false))],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let off_using_models = held_item_models(
        &[base.with(|s| (s.is_using_item, s.use_item_off_hand) = (true, true))],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );

    assert_eq!(idle_models.flat_meshes.len(), 1);
    assert_eq!(main_using_models.flat_meshes.len(), 1);
    assert_eq!(off_using_models.flat_meshes.len(), 1);
    assert_ne!(idle_models.flat_meshes[0], main_using_models.flat_meshes[0]);
    assert_eq!(idle_models.flat_meshes[0], off_using_models.flat_meshes[0]);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn humanoid_held_generated_item_use_duration_reads_owner_use_ticks() {
    // Vanilla `ItemModelResolver.updateForLiving` passes the owning living
    // entity into the item model; `UseDuration.get` then reads the active
    // `getUseItem()` stack's elapsed use ticks.
    let root = unique_item_model_temp_dir("held-use-duration-range-dispatch");
    write_bow_use_duration_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 608;
    world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    let terrain_textures = TerrainTextureState::default();
    let base = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);

    let bake = |instance: EntityModelInstance| {
        let models = held_item_models(&[instance], &world, Some(&item_runtime), &terrain_textures);
        assert_eq!(models.flat_meshes.len(), 1);
        models.flat_meshes[0].clone()
    };

    let idle = bake(base);
    let using_start = bake(
        base.with(|s| (s.is_using_item, s.use_item_off_hand) = (true, false))
            .with(|s| s.crossbow_charge_ticks = 0.0),
    );
    let using_pulling = bake(
        base.with(|s| (s.is_using_item, s.use_item_off_hand) = (true, false))
            .with(|s| s.crossbow_charge_ticks = 13.5),
    );
    let offhand_using = bake(
        base.with(|s| (s.is_using_item, s.use_item_off_hand) = (true, true))
            .with(|s| s.crossbow_charge_ticks = 13.5),
    );

    assert_ne!(idle, using_start);
    assert_ne!(using_start, using_pulling);
    assert_eq!(idle, offhand_using);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn humanoid_held_generated_crossbow_pull_applies_quick_charge_registry() {
    // Vanilla `CrossbowPull.get` divides elapsed use ticks by
    // `CrossbowItem.getChargeDuration`, which applies Quick Charge through
    // the synced enchantment registry before choosing the item-model
    // texture.
    let root = unique_item_model_temp_dir("held-crossbow-quick-charge-range-dispatch");
    write_crossbow_pull_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    const ENTITY_ID: i32 = 609;
    let quick_charge_crossbow = ItemStackSummary {
        item_id: Some(0),
        count: 1,
        component_patch: DataComponentPatchSummary {
            enchantments: vec![bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: 1,
                level: 2,
            }],
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let mut default_world = WorldStore::new();
    default_world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(default_world.apply_set_equipment(SetEquipment {
        entity_id: ENTITY_ID,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: quick_charge_crossbow,
        }],
    }));
    let mut quick_charge_world = default_world.clone();
    record_enchantment_registry(&mut quick_charge_world);
    let terrain_textures = TerrainTextureState::default();
    let instance = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.is_using_item, s.use_item_off_hand) = (true, false))
        .with(|s| s.crossbow_charge_ticks = 10.0);

    let default_models = held_item_models(
        &[instance],
        &default_world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let quick_charge_models = held_item_models(
        &[instance],
        &quick_charge_world,
        Some(&item_runtime),
        &terrain_textures,
    );

    assert_eq!(default_models.flat_meshes.len(), 1);
    assert_eq!(quick_charge_models.flat_meshes.len(), 1);
    assert!(!default_models.flat_meshes[0].is_empty());
    assert!(!quick_charge_models.flat_meshes[0].is_empty());
    assert_ne!(
        default_models.flat_meshes[0],
        quick_charge_models.flat_meshes[0]
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn illusioner_item_in_hand_layer_uses_vanilla_visibility_and_clones() {
    // Vanilla `IllusionerRenderer` installs a conditional `ItemInHandLayer`: idle illusioners do not
    // submit held items, while casting/aggressive illusioners submit inside the invisible clone loop.
    let root = unique_item_model_temp_dir("illusioner-held-item-clones");
    write_flat_item_runtime_fixture(&root, &["bow"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 604;
    world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
    ));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));

    let offsets = [
        [1.25, 0.0, -0.75],
        [-1.0, 0.5, 0.5],
        [0.5, 0.0, 1.0],
        [-1.5, 0.0, -1.25],
    ];
    let base = EntityModelInstance::illager(
        ENTITY_ID,
        [4.0, 64.0, -2.0],
        30.0,
        IllagerModelFamily::Illusioner,
    )
    .with(|s| (s.age_in_ticks, s.kind.illager_mut().clone_offsets) = (8.0, offsets));
    let terrain_textures = TerrainTextureState::default();

    let idle_models = held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
    let aggressive_models = held_item_models(
        &[base.with(|s| s.is_aggressive = true)],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let invisible_aggressive_models = held_item_models(
        &[base.with(|s| s.is_aggressive = true).with_invisible(true)],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let invisible_casting_models = held_item_models(
        &[base
            .with(|s| s.kind.illager_mut().spellcasting = true)
            .with_invisible(true)],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );

    assert!(idle_models.block_meshes.is_empty());
    assert!(idle_models.flat_meshes.is_empty());
    assert_eq!(aggressive_models.flat_meshes.len(), 1);
    assert_eq!(invisible_aggressive_models.flat_meshes.len(), 4);
    assert_eq!(invisible_casting_models.flat_meshes.len(), 4);
    for models in [
        &aggressive_models,
        &invisible_aggressive_models,
        &invisible_casting_models,
    ] {
        assert!(models.flat_meshes.iter().all(|mesh| !mesh.is_empty()));
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn illusioner_custom_head_item_branch_submits_per_invisible_clone() {
    // The generic item branch of vanilla `CustomHeadLayer` is inside `LivingEntityRenderer.submit`,
    // so `IllusionerRenderer`'s invisible clone loop runs it once for each clone root.
    let root = unique_item_model_temp_dir("illusioner-custom-head-item-clones");
    write_flat_item_runtime_fixture(&root, &["head_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 605;
    world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
    ));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::Head, 0)));

    let offsets = [
        [1.25, 0.0, -0.75],
        [-1.0, 0.5, 0.5],
        [0.5, 0.0, 1.0],
        [-1.5, 0.0, -1.25],
    ];
    let base = EntityModelInstance::illager(
        ENTITY_ID,
        [4.0, 64.0, -2.0],
        30.0,
        IllagerModelFamily::Illusioner,
    )
    .with(|s| (s.age_in_ticks, s.kind.illager_mut().clone_offsets) = (8.0, offsets));
    let terrain_textures = TerrainTextureState::default();

    let visible_models = held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
    let invisible_models = held_item_models(
        &[base.with_invisible(true)],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );

    assert!(visible_models.block_meshes.is_empty());
    assert!(invisible_models.block_meshes.is_empty());
    assert!(visible_models.flat_translucent_meshes.is_empty());
    assert!(invisible_models.flat_translucent_meshes.is_empty());
    assert_eq!(visible_models.flat_meshes.len(), 1);
    assert_eq!(invisible_models.flat_meshes.len(), 4);
    assert!(visible_models
        .flat_meshes
        .iter()
        .all(|mesh| !mesh.is_empty()));
    assert!(invisible_models
        .flat_meshes
        .iter()
        .all(|mesh| !mesh.is_empty()));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn allay_item_in_hand_layer_bakes_main_and_offhand_items() {
    // Vanilla `AllayRenderer` adds the standard `ItemInHandLayer`. Its `AllayModel.translateToHand`
    // supplies a special allay hand transform, but the item states still use the ordinary third-person
    // right/left hand display contexts from `ArmedEntityRenderState`.
    let root = unique_item_model_temp_dir("allay-held-items");
    write_flat_item_runtime_fixture(&root, &["main_item", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 603;
    world.apply_add_entity(protocol_add_entity(ENTITY_ID, VANILLA_ENTITY_TYPE_ALLAY_ID));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::OffHand, 1)));

    let base =
        EntityModelInstance::allay(ENTITY_ID, [0.0, 64.0, 0.0], 0.0).with(|s| s.age_in_ticks = 7.0);
    let holding = base.with(|s| s.kind.allay_mut().holding_item_progress = 1.0);
    let terrain_textures = TerrainTextureState::default();

    let idle_models = held_item_models(&[base], &world, Some(&item_runtime), &terrain_textures);
    let holding_models =
        held_item_models(&[holding], &world, Some(&item_runtime), &terrain_textures);

    for models in [&idle_models, &holding_models] {
        assert_eq!(models.flat_meshes.len(), 2);
        assert!(models.flat_meshes.iter().all(|mesh| !mesh.is_empty()));
    }
    assert_ne!(
        idle_models.flat_meshes[0], holding_models.flat_meshes[0],
        "holdingAnimationProgress feeds AllayModel.translateToHand through right_arm.xRot"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ominous_item_spawner_base_transform_scales_in_and_spins() {
    let early = ominous_item_spawner_base_transform([1.0, 2.0, 3.0], 9.0);
    assert!((early.transform_point3(Vec3::ZERO) - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-6);
    assert!((early.transform_vector3(Vec3::X) - Vec3::new(0.18, 0.0, 0.0)).length() < 1e-6);

    let full_size = ominous_item_spawner_base_transform([0.0, 0.0, 0.0], 60.0);
    assert!((full_size.transform_vector3(Vec3::X).length() - 1.0).abs() < 1e-6);
    assert!(
        (full_size.transform_vector3(Vec3::X) - Vec3::X).length() > 1.0,
        "60 ticks spins by 240 degrees, not identity"
    );
}

#[test]
fn ominous_item_spawner_models_emit_item_cluster_meshes() {
    let root = unique_item_model_temp_dir("ominous-item-spawner");
    write_flat_item_runtime_fixture(&root, &["omen_drop"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let item_id = item_runtime
        .item_protocol_id("minecraft:omen_drop")
        .expect("fixture item registered");
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        801,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
    ));
    let item_data_id = vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
    )
    .expect("generated OminousItemSpawner item accessor");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 801,
        values: vec![EntityDataValue {
            data_id: item_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(ItemStackSummary {
                item_id: Some(item_id),
                count: 3,
                component_patch: DataComponentPatchSummary::default().into(),
            }),
        }],
    }));
    world.advance_entity_client_animations(25);

    let models = ominous_item_spawner_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        None,
        0.5,
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    assert_eq!(models.residency.assets.len(), 1);
    assert_eq!(models.residency.instances.len(), 2);
    assert!(models.handled_entity_ids.contains(&801));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn dropped_item_models_submit_selected_special_geometry() {
    let root = unique_item_model_temp_dir("dropped-special-item");
    write_special_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let item_id = item_runtime
        .item_protocol_id("minecraft:special_trident")
        .expect("fixture item registered");
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(802, VANILLA_ENTITY_TYPE_ITEM_ID));
    let item_data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ITEM_ID,
    )
    .expect("generated ItemEntity item accessor");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 802,
        values: vec![EntityDataValue {
            data_id: item_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            }),
        }],
    }));

    let models = dropped_item_models(
        &world,
        &world.item_entity_stacks().collect::<Vec<_>>(),
        Some(&item_runtime),
        None,
        None,
        0.0,
        &crate::item_models::ItemRegistryKeySet::default(),
    );

    assert!(models.residency.assets.is_empty());
    assert!(models.residency.instances.is_empty());
    assert!(matches!(
        models.residency.special_items.as_slice(),
        [bbb_client_render::SpecialItemInstanceRequest {
            model: bbb_client_render::SpecialItemModel::Trident { .. },
            ..
        }]
    ));
    assert!(models.handled_entity_ids.contains(&802));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn dropped_composite_item_preserves_ordinary_special_leaf_order() {
    let root = unique_item_model_temp_dir("dropped-composite-special-item");
    write_composite_special_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let item_id = item_runtime
        .item_protocol_id("minecraft:composite_special")
        .expect("fixture item registered");
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(803, VANILLA_ENTITY_TYPE_ITEM_ID));
    let item_data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ITEM_ID,
    )
    .expect("generated ItemEntity item accessor");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 803,
        values: vec![EntityDataValue {
            data_id: item_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(ItemStackSummary {
                item_id: Some(item_id),
                count: 2,
                component_patch: DataComponentPatchSummary::default().into(),
            }),
        }],
    }));

    let models = dropped_item_models(
        &world,
        &world.item_entity_stacks().collect::<Vec<_>>(),
        Some(&item_runtime),
        None,
        None,
        0.0,
        &crate::item_models::ItemRegistryKeySet::default(),
    );

    assert_eq!(
        models
            .residency
            .instances
            .iter()
            .map(|instance| instance.submit_order)
            .collect::<Vec<_>>(),
        vec![0, 2, 3, 5]
    );
    assert_eq!(
        models
            .residency
            .special_items
            .iter()
            .map(|instance| instance.submit_order)
            .collect::<Vec<_>>(),
        vec![1, 4]
    );
    assert!(models.handled_entity_ids.contains(&803));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn smithing_shared_hand_item_bake_preserves_world_and_pip_special_targets() {
    let root = unique_item_model_temp_dir("hand-special-item");
    write_special_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let stack = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:special_trident"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut residency = ItemModelResidencyFrame::default();

    crate::item_models::bake_item_stack(
        &crate::item_models::ItemBakeEnv {
            world: &WorldStore::new(),
            item_runtime: &item_runtime,
            variant_textures: None,
            registry_keys: Default::default(),
        },
        &stack,
        BlockModelDisplayContext::ThirdPersonRightHand,
        &crate::item_models::ItemOwner {
            entity_type: Some("minecraft:player"),
            main_hand_left: Some(false),
            ..Default::default()
        },
        &crate::item_models::ItemPlacement::attached(
            Mat4::IDENTITY,
            BlockModelDisplayContext::ThirdPersonRightHand,
            false,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
        ),
        &mut residency,
    );

    assert!(residency.assets.is_empty());
    assert!(residency.instances.is_empty());
    let [request] = residency.special_items.as_slice() else {
        panic!("selected special leaf should submit one trident request");
    };
    assert!(matches!(
        &request.model,
        bbb_client_render::SpecialItemModel::Trident { .. }
    ));
    assert_eq!(request.target, ItemModelDrawTarget::World);
    assert_eq!(
        request.transform,
        display_matrix(
            &BlockModelDisplayTransform {
                rotation: [0.0; 3],
                translation: [4.0 / 16.0, 0.0, 0.0],
                scale: [1.0; 3],
            },
            false,
        )
    );

    crate::item_models::bake_item_stack(
        &crate::item_models::ItemBakeEnv {
            world: &WorldStore::new(),
            item_runtime: &item_runtime,
            variant_textures: None,
            registry_keys: Default::default(),
        },
        &stack,
        BlockModelDisplayContext::ThirdPersonLeftHand,
        &crate::item_models::ItemOwner::default(),
        &crate::item_models::ItemPlacement {
            attach: Mat4::IDENTITY,
            left_hand: true,
            block_fallback: BLOCK_THIRD_PERSON_FALLBACK,
            generated_fallback: GENERATED_THIRD_PERSON_FALLBACK,
            light: ITEM_MODEL_FULL_BRIGHT_LIGHT,
            target: ItemModelDrawTarget::EntityPreview(4),
            outline_color: None,
        },
        &mut residency,
    );
    assert_eq!(residency.special_items.len(), 2);
    assert_eq!(
        residency.special_items[1].target,
        ItemModelDrawTarget::EntityPreview(4)
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_pickup_particle_item_models_emit_ordinary_stack_meshes() {
    let root = unique_item_model_temp_dir("item-pickup-particle");
    write_flat_item_runtime_fixture(&root, &["picked_up"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let item_id = item_runtime
        .item_protocol_id("minecraft:picked_up")
        .expect("fixture item registered");
    let state = ItemPickupParticleRenderState {
        source_entity_id: 77,
        item: ParticleItemOptionState {
            item_id,
            count: 18,
            component_patch_len: 0,
        },
        component_patch: None,
        position: [1.25, 65.5, -3.0],
        age_ticks: 9.5,
        bob_offset: 0.25,
        light: [6.0 / 15.0, 12.0 / 15.0],
    };

    let world = WorldStore::new();
    let models = item_pickup_particle_item_models(
        &world,
        &[state],
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        None,
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    assert_eq!(models.residency.assets.len(), 1);
    assert_eq!(models.residency.instances.len(), 3);
    assert!(models.handled_entity_ids.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_pickup_particle_component_rich_bake_matches_dropped_item_bake() {
    // A component-rich pickup stack must bake through the SAME item-model
    // projection as a dropped item entity. Here the ITEM_MODEL component
    // overrides the root model (vanilla `ItemModelResolver.appendItemLayers`),
    // so the resolved item texture differs from the plain item. With identical
    // geometry inputs (position / age / entity id / light / count / seed) the
    // pickup carried bake must be byte-identical to the dropped-item bake.
    const ENTITY_ID: i32 = 555;
    const POSITION: [f32; 3] = [0.0, 64.0, 0.0];
    const AGE_TICKS: f32 = 9.5;
    const COUNT: i32 = 1;

    let root = unique_item_model_temp_dir("item-pickup-component-rich");
    write_flat_item_runtime_fixture(&root, &["pickup_tool", "override_skin"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let item_id = item_runtime
        .item_protocol_id("minecraft:pickup_tool")
        .expect("fixture item registered");

    // ITEM_MODEL override -> the resolved icon uses `override_skin`'s atlas rect
    // instead of `pickup_tool`'s, changing the baked mesh.
    let component_patch = DataComponentPatchSummary {
        item_model: Some("minecraft:override_skin".to_string()),
        ..DataComponentPatchSummary::default()
    };
    let stack = ItemStackSummary {
        item_id: Some(item_id),
        count: COUNT,
        component_patch: component_patch.clone().into(),
    };

    // Dropped-item bake: a real item entity carrying the component-rich stack.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(ENTITY_ID, VANILLA_ENTITY_TYPE_ITEM_ID));
    let item_data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ITEM_ID,
    )
    .expect("generated ItemEntity item accessor");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: ENTITY_ID,
        values: vec![EntityDataValue {
            data_id: item_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(stack.clone()),
        }],
    }));
    let dropped = dropped_item_models(
        &world,
        &world.item_entity_stacks().collect::<Vec<_>>(),
        Some(&item_runtime),
        None,
        None,
        AGE_TICKS,
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    let bob_offset = world.item_entity_stacks().next().unwrap().bob_offset;

    // Pickup carried bake: the same stack carried by the pickup channel, with
    // geometry inputs aligned to the dropped item entity.
    let component_rich_state = ItemPickupParticleRenderState {
        source_entity_id: ENTITY_ID,
        item: ParticleItemOptionState {
            item_id,
            count: COUNT,
            component_patch_len: 1,
        },
        component_patch: Some(Arc::new(component_patch.clone())),
        position: POSITION,
        age_ticks: AGE_TICKS,
        bob_offset,
        light: [1.0, 1.0],
    };
    let pickup = item_pickup_particle_item_models(
        &world,
        std::slice::from_ref(&component_rich_state),
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        None,
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    assert_eq!(dropped.residency.assets.len(), 1);
    assert_eq!(dropped.residency.instances.len(), 1);
    // Equality: the component-rich pickup selects the same immutable asset and
    // transform/tint as the dropped item; only its render target differs.
    assert_eq!(pickup.residency.assets, dropped.residency.assets);
    assert_eq!(
        pickup.residency.instances[0].transform,
        dropped.residency.instances[0].transform
    );
    assert_eq!(
        pickup.residency.instances[0].tint,
        dropped.residency.instances[0].tint
    );

    // Consumption proof: dropping the patch selects `pickup_tool`'s own model,
    // producing a different mesh -- so the patch demonstrably drives the bake.
    let plain_state = ItemPickupParticleRenderState {
        component_patch: None,
        item: ParticleItemOptionState {
            component_patch_len: 0,
            ..component_rich_state.item
        },
        ..component_rich_state
    };
    let plain = item_pickup_particle_item_models(
        &world,
        &[plain_state],
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        None,
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    assert_eq!(plain.residency.assets.len(), 1);
    assert_ne!(pickup.residency.assets, plain.residency.assets);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn component_select_prototype_and_dynamic_registry_reach_native_item_consumers() {
    const ITEM_ENTITY_ID: i32 = 880;
    const SPAWNER_ENTITY_ID: i32 = 881;
    const MATCH_ASSET: &str = "minecraft:item/component_select_match";
    const FALLBACK_ASSET: &str = "minecraft:item/component_select_fallback";

    let root = unique_item_model_temp_dir("component-select-native-consumers");
    write_component_select_registry_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let item_id = item_runtime
        .item_protocol_id("minecraft:component_selector")
        .expect("fixture item registered");
    let raw_stack = ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut world = world_with_component_select_prototype(item_id);

    let effective_stack = world.effective_item_stack(&raw_stack);
    assert_eq!(
        effective_stack
            .component_patch
            .damage_resistant_type_tag
            .as_deref(),
        Some("example:fire")
    );
    assert_eq!(
        effective_stack
            .component_patch
            .damage_resistant_type_registry_ids,
        [0]
    );

    // A context-free consumer cannot remap the named holder set and therefore
    // deterministically uses the fallback. Supplying the current world owner
    // round-trips the case key through the session registry and selects it.
    let context_free_icon = item_runtime
        .resolve_item_model(
            effective_stack.as_ref(),
            &bbb_client_render::item_model::ItemModelContext::new(BlockModelDisplayContext::Gui),
        )
        .and_then(|model| model.flat_icon())
        .expect("context-free fallback icon");
    let context_free_layers = item_runtime.generated_item_layers_for_icon(&context_free_icon);
    assert_eq!(context_free_layers[0].asset_id.as_ref(), FALLBACK_ASSET);
    let registry_layers = item_runtime
        .resolve_item_model(
            effective_stack.as_ref(),
            &bbb_client_render::item_model::ItemModelContext {
                registry_access: Some((world.registry_access_revision(), world.registries())),
                ..bbb_client_render::item_model::ItemModelContext::new(
                    BlockModelDisplayContext::Ground,
                )
            },
        )
        .and_then(|model| model.flat_icon())
        .map(|icon| item_runtime.generated_item_layers_for_icon(&icon))
        .unwrap_or_default();
    assert_eq!(registry_layers[0].asset_id.as_ref(), MATCH_ASSET);

    world.apply_add_entity(protocol_add_entity(
        ITEM_ENTITY_ID,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    let item_data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ITEM_ID,
    )
    .expect("generated ItemEntity item accessor");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: ITEM_ENTITY_ID,
        values: vec![EntityDataValue {
            data_id: item_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(raw_stack.clone()),
        }],
    }));
    let dropped = dropped_item_models(
        &world,
        &world.item_entity_stacks().collect::<Vec<_>>(),
        Some(&item_runtime),
        None,
        None,
        0.0,
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    assert_eq!(dropped.residency.assets[0].identity.model_id(), MATCH_ASSET);

    world.apply_add_entity(protocol_add_entity(
        SPAWNER_ENTITY_ID,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
    ));
    let spawner_item_data_id =
        vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
        )
        .expect("generated OminousItemSpawner item accessor");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: SPAWNER_ENTITY_ID,
        values: vec![EntityDataValue {
            data_id: spawner_item_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(raw_stack.clone()),
        }],
    }));
    let spawner = ominous_item_spawner_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        None,
        0.0,
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    assert_eq!(spawner.residency.assets[0].identity.model_id(), MATCH_ASSET);

    let projectile_entity_id = 882;
    world.apply_add_entity(protocol_add_entity(
        projectile_entity_id,
        VANILLA_ENTITY_TYPE_SNOWBALL_ID,
    ));
    let projectile_item_data_id =
        bbb_client_world::vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
            VANILLA_ENTITY_TYPE_SNOWBALL_ID,
            "ITEM_STACK",
        )
        .expect("generated Snowball item-stack accessor")
        .data_id;
    assert!(world.apply_set_entity_data(SetEntityData {
        id: projectile_entity_id,
        values: vec![EntityDataValue {
            data_id: projectile_item_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(raw_stack.clone()),
        }],
    }));
    let mut candidates = Vec::new();
    world.fill_entity_render_candidates_at_partial_tick(1.0, &mut candidates);
    let dropped: Vec<_> = world.item_entity_stacks().collect();
    let billboards = crate::item_entities::item_entity_billboards_from_world(
        &world,
        &candidates,
        &dropped,
        Some(&item_runtime),
        &BTreeSet::from([ITEM_ENTITY_ID]),
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    let projectile_billboard = billboards
        .iter()
        .find(|billboard| billboard.position == [0.0, 64.0, 0.0])
        .expect("thrown-item billboard");
    assert_eq!(
        projectile_billboard.layers[0].uv.min,
        registry_layers[0].rect.min
    );
    assert_eq!(
        projectile_billboard.layers[0].uv.max,
        registry_layers[0].rect.max
    );

    let pickup = item_pickup_particle_item_models(
        &world,
        &[ItemPickupParticleRenderState {
            source_entity_id: ITEM_ENTITY_ID,
            item: ParticleItemOptionState {
                item_id,
                count: 1,
                component_patch_len: 0,
            },
            component_patch: None,
            position: [0.0, 64.0, 0.0],
            age_ticks: 0.0,
            bob_offset: 0.0,
            light: [1.0; 2],
        }],
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        None,
        &crate::item_models::ItemRegistryKeySet::default(),
    );
    assert_eq!(pickup.residency.assets[0].identity.model_id(), MATCH_ASSET);

    let mut held_residency = ItemModelResidencyFrame::default();
    crate::item_models::bake_item_stack(
        &crate::item_models::ItemBakeEnv {
            world: &world,
            item_runtime: &item_runtime,
            variant_textures: None,
            registry_keys: Default::default(),
        },
        &raw_stack,
        BlockModelDisplayContext::ThirdPersonRightHand,
        &crate::item_models::ItemOwner {
            entity_type: Some("minecraft:player"),
            main_hand_left: Some(false),
            ..Default::default()
        },
        &crate::item_models::ItemPlacement::attached(
            Mat4::IDENTITY,
            BlockModelDisplayContext::ThirdPersonRightHand,
            false,
            BLOCK_THIRD_PERSON_FALLBACK,
            GENERATED_THIRD_PERSON_FALLBACK,
        ),
        &mut held_residency,
    );
    assert_eq!(held_residency.assets[0].identity.model_id(), MATCH_ASSET);

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn smithing_preview_projects_generated_hand_and_head_items_to_indexed_pip_geometry() {
    use bbb_protocol::{menu_types::VANILLA_MENU_TYPE_SMITHING_ID, RichTextComponent};

    let root = unique_item_model_temp_dir("smithing-preview-generated");
    write_flat_item_runtime_fixture(&root, &["preview_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let item_id = item_runtime
        .item_protocol_id("minecraft:preview_item")
        .expect("fixture item id");
    let stack = ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut world = WorldStore::new();
    world.apply_open_screen(OpenScreen {
        container_id: 8,
        menu_type_id: VANILLA_MENU_TYPE_SMITHING_ID,
        title: RichTextComponent::plain("Smithing"),
    });
    let mut items = vec![ItemStackSummary::empty(); 40];
    items[3] = stack;
    world.apply_container_set_content(ContainerSetContent {
        container_id: 8,
        state_id: 1,
        items,
        carried_item: ItemStackSummary::empty(),
    });

    let entity = EntityModelInstance::armor_stand(
        -1,
        [0.0; 3],
        210.0,
        false,
        true,
        false,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (0.0, 25.0));
    let preview = |slot, display_context| HudEntityPreview {
        entity,
        lighting: GuiItemLightingEntry::EntityInUi,
        rect: HudEntityPreviewRect {
            x: 121,
            y: 20,
            width: 40,
            height: 60,
        },
        scissor: None,
        translation: [0.0, 1.0, 0.0],
        rotation: [0.0, 0.0, 1.0, 0.0],
        override_camera_rotation: None,
        scale: 25.0,
        depth_isolated: true,
        after_background_layer: None,
        item_layers: vec![HudEntityPreviewItemLayer {
            slot,
            display_context,
            item_id,
            count: 1,
            foil: false,
            light_coords: ENTITY_FULL_BRIGHT_LIGHT_COORDS,
            overlay: ITEM_MODEL_NO_OVERLAY,
            order: 0,
            submit_sequence: 1,
        }],
    };
    let screen = |preview| HudInventoryScreen {
        width: 176,
        height: 166,
        background_layers: Vec::new(),
        foreground_layers: Vec::new(),
        fill_layers: Vec::new(),
        slots: Vec::new(),
        floating_items: Vec::new(),
        foreground_items: Vec::new(),
        ghost_items: Vec::new(),
        entity_previews: vec![preview],
        map_previews: Vec::new(),
        banner_pattern_previews: Vec::new(),
        text_labels: Vec::new(),
        scroll_area: None,
        preedit_overlay: None,
        hovered_slot_id: None,
        tooltip: None,
    };
    let extract = |preview| {
        smithing_entity_preview_item_models(
            &world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            None,
            Some(&screen(preview)),
            &crate::item_models::ItemRegistryKeySet::default(),
        )
    };

    let hand = extract(preview(
        HudEntityPreviewItemSlot::LeftHand,
        HudEntityPreviewItemDisplayContext::ThirdPersonLeftHand,
    ));
    let head = extract(preview(
        HudEntityPreviewItemSlot::Head,
        HudEntityPreviewItemDisplayContext::Head,
    ));
    assert_eq!(hand.instances.len(), 1);
    assert_eq!(head.instances.len(), 1);
    assert_eq!(
        hand.instances[0].target,
        ItemModelDrawTarget::EntityPreview(0)
    );
    assert_eq!(
        head.instances[0].target,
        ItemModelDrawTarget::EntityPreview(0)
    );
    assert_eq!(hand.instances[0].light, ITEM_MODEL_FULL_BRIGHT_LIGHT);
    assert_eq!(head.instances[0].overlay, ITEM_MODEL_NO_OVERLAY);

    let hand_attach = humanoid_hand_attach_transforms(&entity, true);
    let head_attach = custom_head_item_transforms(&entity);
    assert_eq!(hand_attach.len(), 1);
    assert_eq!(head_attach.len(), 1);
    let inventory = world.inventory();
    let container = inventory
        .open_container
        .as_ref()
        .expect("smithing container");
    let result_stack = inventory
        .container_slot(container, 3)
        .expect("result slot")
        .item;
    let hand_display = item_runtime
        .item_display_transform_for_stack(
            result_stack,
            BlockModelDisplayContext::ThirdPersonLeftHand,
        )
        .unwrap_or(GENERATED_THIRD_PERSON_FALLBACK);
    let head_display = item_runtime
        .item_display_transform_for_stack(result_stack, BlockModelDisplayContext::Head)
        .unwrap_or(GENERATED_HEAD_FALLBACK);
    let expected_hand = hand_attach[0] * display_matrix(&hand_display, true);
    let expected_head = head_attach[0] * display_matrix(&head_display, false);
    assert!(hand.instances[0]
        .transform
        .abs_diff_eq(expected_hand, 1.0e-6));
    assert!(head.instances[0]
        .transform
        .abs_diff_eq(expected_head, 1.0e-6));
    assert!(
        !hand.instances[0]
            .transform
            .abs_diff_eq(head.instances[0].transform, 1.0e-6),
        "hand and head placement must remain distinct"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn display_matrix_centers_the_model_at_the_translation() {
    // The display transform is about the model center (`T(-0.5)`), so the unit-cube center
    // (0.5,0.5,0.5) lands exactly at the (world-unit) translation regardless of rotation/scale.
    let generated =
        display_matrix(&GENERATED_THIRD_PERSON_FALLBACK, false).transform_point3(Vec3::splat(0.5));
    assert!((generated - Vec3::new(0.0, 3.0 / 16.0, 1.0 / 16.0)).length() < 1e-6);
    let block =
        display_matrix(&BLOCK_THIRD_PERSON_FALLBACK, false).transform_point3(Vec3::splat(0.5));
    assert!((block - Vec3::new(0.0, 2.5 / 16.0, 0.0)).length() < 1e-6);

    let generated_head =
        display_matrix(&GENERATED_HEAD_FALLBACK, false).transform_point3(Vec3::splat(0.5));
    assert!((generated_head - Vec3::new(0.0, 13.0 / 16.0, 7.0 / 16.0)).length() < 1e-6);
    let block_head = display_matrix(&BLOCK_HEAD_FALLBACK, false).transform_point3(Vec3::splat(0.5));
    assert!((block_head - Vec3::ZERO).length() < 1e-6);
}

#[test]
fn item_stack_foil_mode_projects_vanilla_special_context_scale() {
    let root = unique_item_model_temp_dir("special-foil-mode");
    write_flat_item_runtime_fixture(&root, &["clock", "spyglass"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let foiled_stack = |resource_id: &str| ItemStackSummary {
        item_id: item_runtime.item_protocol_id(resource_id),
        count: 1,
        component_patch: DataComponentPatchSummary {
            enchantment_glint_override: Some(true),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let plain_clock = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:clock"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };

    assert_eq!(
        item_stack_foil_mode(
            &foiled_stack("minecraft:clock"),
            &item_runtime,
            BlockModelDisplayContext::Ground
        ),
        ItemModelFoil::Special {
            decal_pose_scale: 1.0
        }
    );
    assert_eq!(
        item_stack_foil_mode(
            &foiled_stack("minecraft:clock"),
            &item_runtime,
            BlockModelDisplayContext::Gui
        ),
        ItemModelFoil::Special {
            decal_pose_scale: 0.5
        }
    );
    assert_eq!(
        item_stack_foil_mode(
            &foiled_stack("minecraft:clock"),
            &item_runtime,
            BlockModelDisplayContext::FirstPersonRightHand
        ),
        ItemModelFoil::Special {
            decal_pose_scale: 0.75
        }
    );
    assert_eq!(
        item_stack_foil_mode(
            &foiled_stack("minecraft:spyglass"),
            &item_runtime,
            BlockModelDisplayContext::Ground
        ),
        ItemModelFoil::Standard
    );
    assert_eq!(
        item_stack_foil_mode(
            &plain_clock,
            &item_runtime,
            BlockModelDisplayContext::Ground
        ),
        ItemModelFoil::None
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn left_hand_display_mirror_negates_x_translation_and_yz_rotation() {
    // Vanilla `ItemTransform.apply(applyLeftHandFix=true)` negates translation.x, rotation.y, and
    // rotation.z. A handheld-style transform with a Y rotation mirrors the model across the X plane.
    let handheld = BlockModelDisplayTransform {
        rotation: [0.0, -90.0, 55.0],
        translation: [1.0 / 16.0, 4.0 / 16.0, 0.5 / 16.0],
        scale: [0.85, 0.85, 0.85],
    };
    let right = display_matrix(&handheld, false);
    let left = display_matrix(&handheld, true);
    // The model center sits at the mirrored translation (x negated, y/z unchanged).
    let right_center = right.transform_point3(Vec3::splat(0.5));
    let left_center = left.transform_point3(Vec3::splat(0.5));
    assert!((right_center - Vec3::new(1.0 / 16.0, 4.0 / 16.0, 0.5 / 16.0)).length() < 1e-6);
    assert!((left_center - Vec3::new(-1.0 / 16.0, 4.0 / 16.0, 0.5 / 16.0)).length() < 1e-6);
    // The left fix mirrors the rotation across the X plane: `rotationXYZ(rx,-ry,-rz)` =
    // `M·rotationXYZ(rx,ry,rz)·M` (M = diag(-1,1,1)), so the rotated +X basis direction reflects to
    // `(x, -y, -z)` of the right-hand one.
    let right_dir = right.transform_vector3(Vec3::X);
    let left_dir = left.transform_vector3(Vec3::X);
    assert!((left_dir - Vec3::new(right_dir.x, -right_dir.y, -right_dir.z)).length() < 1e-6);
}
