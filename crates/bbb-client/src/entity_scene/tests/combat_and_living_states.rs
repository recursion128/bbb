use super::*;

#[test]
fn entity_model_instances_empty_hand_swing_defaults_to_whack() {
    // Vanilla `ItemStack.EMPTY.getSwingAnimation()` resolves `SwingAnimation.DEFAULT`.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        240,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -9.0],
    ));
    let stab = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 240)
        .unwrap()
        .render_state
        .main_hand_swing_is_stab;
    assert!(!stab);
}

#[test]
fn entity_model_instances_project_stab_from_attack_arm_item() {
    // Vanilla `ArmedEntityRenderState.extractArmedEntityRenderState` reads
    // `entity.getItemHeldByArm(state.attackArm).getSwingAnimation().type()`: an
    // off-hand spear swing is STAB even when the main hand is a plain item.
    const WOODEN_SPEAR_ID: i32 = 0;
    const PLAIN_ITEM_ID: i32 = 1;

    let registry: bbb_client_assets::ItemRegistryCatalog =
        serde_json::from_value(serde_json::json!({
            "resource_ids": [
                "minecraft:wooden_spear",
                "minecraft:stick"
            ],
            "protocol_ids": {
                "minecraft:wooden_spear": WOODEN_SPEAR_ID,
                "minecraft:stick": PLAIN_ITEM_ID
            },
            "default_component_codec_values": vanilla_kinetic_weapon_fixture(&[
                "minecraft:wooden_spear"
            ])
        }))
        .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_client_assets::EquipmentAssetCatalog::default(),
    );
    let equip = |entity_id: i32, slot: EquipmentSlot, item_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }],
    };
    let state = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 240)
            .unwrap()
            .render_state
    };

    let mut world = WorldStore::new();
    install_kinetic_runtime_defaults(&mut world, &runtime);
    world.apply_add_entity(protocol_add_entity(
        240,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(240, EquipmentSlot::MainHand, PLAIN_ITEM_ID)));
    assert!(world.apply_set_equipment(equip(240, EquipmentSlot::OffHand, WOODEN_SPEAR_ID)));
    assert!(world.apply_entity_animation(EntityAnimation { id: 240, action: 3 }));

    let off_hand_attack = state(&world);
    assert!(off_hand_attack.attack_arm_off_hand);
    assert!(off_hand_attack.main_hand_swing_is_stab);

    assert!(world.apply_entity_animation(EntityAnimation { id: 240, action: 0 }));
    let main_hand_attack = state(&world);
    assert!(!main_hand_attack.attack_arm_off_hand);
    assert!(
        !main_hand_attack.main_hand_swing_is_stab,
        "the main-hand plain item keeps WHACK even while the off hand holds a spear"
    );

    world.apply_add_entity(protocol_add_entity(
        241,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [2.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(241, EquipmentSlot::MainHand, WOODEN_SPEAR_ID,)));
    assert!(world.apply_entity_animation(EntityAnimation { id: 241, action: 0 }));
    let zombie_attack =
        entity_model_instances_from_world_at_partial_tick(&world, Some(&runtime), 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 241)
            .unwrap()
            .render_state;
    assert!(
        zombie_attack.main_hand_swing_is_stab,
        "ArmedEntityRenderState extracts attack-arm STAB for non-player humanoids too"
    );
}

#[test]
fn entity_model_instances_use_custom_prototype_swing_animation_types_for_both_attack_hands() {
    const NONE_ITEM_ID: i32 = 70;
    const STAB_ITEM_ID: i32 = 71;

    let mut world = WorldStore::new();
    install_swing_animation_defaults(
        &mut world,
        &[(NONE_ITEM_ID, "none", 5), (STAB_ITEM_ID, "stab", 9)],
    );
    world.apply_add_entity(protocol_add_entity(
        247,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 247,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: ItemStackSummary {
                    item_id: Some(NONE_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default().into(),
                },
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::OffHand,
                item: ItemStackSummary {
                    item_id: Some(STAB_ITEM_ID),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default().into(),
                },
            },
        ],
    }));
    let state = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 247)
            .unwrap()
            .render_state
    };

    assert!(world.apply_entity_animation(EntityAnimation { id: 247, action: 0 }));
    let main_attack = state(&world);
    assert!(main_attack.main_hand_swing_is_none);
    assert!(!main_attack.main_hand_swing_is_stab);

    assert!(world.apply_entity_animation(EntityAnimation { id: 247, action: 3 }));
    let off_attack = state(&world);
    assert!(!off_attack.main_hand_swing_is_none);
    assert!(off_attack.main_hand_swing_is_stab);
}

#[test]
fn entity_model_instances_project_stab_from_stack_swing_animation_patch() {
    // Vanilla `ItemStack.getSwingAnimation()` reads the stack component before
    // the item prototype. A patch-granted STAB therefore drives the attack-arm
    // STAB render state even on a non-spear item and without an item runtime.
    const PLAIN_ITEM_ID: i32 = 31;

    let mut item = ItemStackSummary {
        item_id: Some(PLAIN_ITEM_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    item.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];
    item.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::Stab,
        duration: 17,
    });

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        242,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 242,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item,
        }],
    }));
    assert!(world.apply_entity_animation(EntityAnimation { id: 242, action: 0 }));

    let state = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0)
        .into_iter()
        .find(|instance| instance.entity_id == 242)
        .unwrap()
        .render_state;
    assert!(state.main_hand_swing_is_stab);
    assert!(
            state.kind.player().main_hand_spear_pose,
            "AvatarRenderer returns ArmPose.SPEAR for a swinging STAB stack even when the item is not tagged spear"
        );
}

#[test]
fn entity_model_instances_stack_swing_animation_overrides_default_spear_stab() {
    // Removing or overriding `SWING_ANIMATION` on a spear stack changes
    // `getSwingAnimation().type()` away from the item prototype's STAB default.
    const WOODEN_SPEAR_ID: i32 = 0;

    let registry: bbb_client_assets::ItemRegistryCatalog =
        serde_json::from_value(serde_json::json!({
            "resource_ids": [
                "minecraft:wooden_spear"
            ],
            "protocol_ids": {
                "minecraft:wooden_spear": WOODEN_SPEAR_ID
            }
        }))
        .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_client_assets::EquipmentAssetCatalog::default(),
    );
    let equip = |entity_id: i32, item: ItemStackSummary| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item,
        }],
    };
    let state = |world: &WorldStore, entity_id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == entity_id)
            .unwrap()
            .render_state
    };

    let mut removed = ItemStackSummary {
        item_id: Some(WOODEN_SPEAR_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    removed.component_patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];

    let mut whack = ItemStackSummary {
        item_id: Some(WOODEN_SPEAR_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    whack.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];
    whack.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::Whack,
        duration: 6,
    });

    let mut none = ItemStackSummary {
        item_id: Some(WOODEN_SPEAR_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    none.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID];
    none.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::None,
        duration: 6,
    });

    let mut world = WorldStore::new();
    install_swing_animation_defaults(&mut world, &[(WOODEN_SPEAR_ID, "stab", 13)]);
    world.apply_add_entity(protocol_add_entity(
        243,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(243, removed)));
    assert!(world.apply_entity_animation(EntityAnimation { id: 243, action: 0 }));
    assert!(!state(&world, 243).main_hand_swing_is_stab);
    assert!(!state(&world, 243).main_hand_swing_is_none);

    world.apply_add_entity(protocol_add_entity(
        244,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(244, whack)));
    assert!(world.apply_entity_animation(EntityAnimation { id: 244, action: 0 }));
    assert!(!state(&world, 244).main_hand_swing_is_stab);
    assert!(!state(&world, 244).main_hand_swing_is_none);

    world.apply_add_entity(protocol_add_entity(
        245,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [3.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(245, none)));
    assert!(world.apply_entity_animation(EntityAnimation { id: 245, action: 0 }));
    assert!(!state(&world, 245).main_hand_swing_is_stab);
    assert!(state(&world, 245).main_hand_swing_is_none);
}

#[test]
fn entity_model_instances_project_player_off_hand_item_pose() {
    // Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM`: a player holding a plain off-hand
    // item lowers/halves the OFF arm. Gated to the player kind + a non-empty off hand, suppressed only
    // when USING the off hand (its use poses win); using the MAIN hand leaves the off hand on `ITEM`.
    const PLAIN_ITEM_ID: i32 = 720;

    let plain_off_hand = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::OffHand,
            item: ItemStackSummary {
                item_id: Some(PLAIN_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }],
    };
    let posing = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .player()
            .off_hand_item_pose
    };

    let mut world = WorldStore::new();
    // A player holding a plain off-hand item reaches the off-hand ITEM fallback pose.
    world.apply_add_entity(protocol_add_entity(
        270,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, 12.0],
    ));
    assert!(world.apply_set_equipment(plain_off_hand(270)));
    assert!(posing(&world, 270));

    // An empty off hand has no item to pose.
    world.apply_add_entity(protocol_add_entity(
        271,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, 12.0],
    ));
    assert!(!posing(&world, 271));

    // USING a NON-special off-hand item (here a plain item — `EAT`/`DRINK`) still falls through to the
    // off-hand `ITEM` fallback (only a special off-hand use item would route to its own pose, which needs
    // the runtime to resolve; the no-runtime case treats any using item as non-special).
    world.apply_add_entity(protocol_add_entity(
        272,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [3.0, 64.0, 12.0],
    ));
    assert!(world.apply_set_equipment(plain_off_hand(272)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 272,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            true,
            false,
        )],
    }));
    assert!(posing(&world, 272));

    // USING the MAIN hand leaves the off hand on its ITEM fallback (the off hand is not the using hand).
    world.apply_add_entity(protocol_add_entity(
        273,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [4.0, 64.0, 12.0],
    ));
    assert!(world.apply_set_equipment(plain_off_hand(273)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 273,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    assert!(posing(&world, 273));

    // A non-player mob never returns ITEM for the off hand either.
    world.apply_add_entity(protocol_add_entity(
        274,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [5.0, 64.0, 12.0],
    ));
    assert!(world.apply_set_equipment(plain_off_hand(274)));
    assert!(!posing(&world, 274));
}

#[test]
fn entity_model_instances_project_piglin_melee_attack_pose() {
    // Vanilla Piglin/PiglinBrute.getArmPose ATTACKING_WITH_MELEE_WEAPON: aggressive (Mob.isAggressive,
    // resolved through the generated Mob accessor/mask) AND isHoldingMeleeWeapon (main-hand item with
    // DataComponents.TOOL, wire type 28). Gated to the regular piglin + brute (the zombified piglin
    // uses its renderer zombie-arm pose); the regular piglin is also suppressed while DANCING.
    // Any item id resolves the same way — only the TOOL component drives the pose, not the item type.
    const MELEE_ITEM_ID: i32 = 700;

    // A main-hand stack carrying the `minecraft:tool` data component (wire type 28).
    let tool_main_hand = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id: Some(MELEE_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_TOOL_TYPE_ID],
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
        }],
    };
    let set_aggressive = |id: i32, entity_type_id: i32| SetEntityData {
        id,
        values: vec![mob_aggressive_data(entity_type_id)],
    };
    let attacking = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .piglin()
            .attacking_with_melee
    };

    let mut world = WorldStore::new();
    // Regular piglin (210), brute (211), zombified piglin (212): all aggressive, all holding a tool.
    for (id, type_id) in [
        (210, VANILLA_ENTITY_TYPE_PIGLIN_ID),
        (211, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID),
        (212, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID),
    ] {
        world.apply_add_entity(protocol_add_entity(id, type_id, [1.0, 64.0, -6.0]));
        assert!(world.apply_set_equipment(tool_main_hand(id)));
        assert!(world.apply_set_entity_data(set_aggressive(id, type_id)));
    }
    // The piglin and the brute raise/swing the melee weapon; the zombified piglin uses zombie arms.
    assert!(attacking(&world, 210));
    assert!(attacking(&world, 211));
    assert!(!attacking(&world, 212));

    // An aggressive piglin with an empty hand (no tool component) does not raise a weapon.
    world.apply_add_entity(protocol_add_entity(
        213,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [2.0, 64.0, -6.0],
    ));
    assert!(world.apply_set_entity_data(set_aggressive(213, VANILLA_ENTITY_TYPE_PIGLIN_ID,)));
    assert!(!attacking(&world, 213));

    // A non-aggressive piglin holding a tool does not attack.
    world.apply_add_entity(protocol_add_entity(
        214,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [3.0, 64.0, -6.0],
    ));
    assert!(world.apply_set_equipment(tool_main_hand(214)));
    assert!(!attacking(&world, 214));

    // A dancing piglin (higher priority) holding a tool while aggressive keeps DANCING, not the attack.
    world.apply_add_entity(protocol_add_entity(
        215,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [4.0, 64.0, -6.0],
    ));
    assert!(world.apply_set_equipment(tool_main_hand(215)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 215,
        values: vec![
            mob_aggressive_data(VANILLA_ENTITY_TYPE_PIGLIN_ID),
            protocol_bool_data(piglin_dancing_data_id(), true),
        ],
    }));
    assert!(!attacking(&world, 215));
}

#[test]
fn entity_model_instances_project_piglin_admiring_a_loved_offhand_item() {
    // Vanilla Piglin.getArmPose ADMIRING_ITEM = PiglinAi.isLovedItem(getOffhandItem()) =
    // offhand.is(ItemTags.PIGLIN_LOVED). Gated to the regular piglin (the brute has no admire branch);
    // higher priority than ATTACKING/CROSSBOW (suppresses them), below DANCING.
    const LOVED_ITEM_ID: i32 = 800; // stand-in for a `minecraft:piglin_loved` item (e.g. gold_ingot).
    const PLAIN_ITEM_ID: i32 = 801; // not in the tag.
    const TOOL_ITEM_ID: i32 = 802; // a melee weapon (TOOL component) for the suppression check.

    let equip = |entity_id: i32, slot: EquipmentSlot, item_id: i32, tool: bool| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids: if tool {
                        vec![VANILLA_DATA_COMPONENT_TOOL_TYPE_ID]
                    } else {
                        Vec::new()
                    },
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
        }],
    };
    let admiring = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .piglin()
            .admiring
    };

    let mut world = WorldStore::new();
    // The `minecraft:piglin_loved` item tag arrives via UpdateTags (gold_ingot etc.).
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: PIGLIN_LOVED_ITEM_TAG.to_string(),
                entries: vec![LOVED_ITEM_ID, TOOL_ITEM_ID],
            }],
        }],
    });

    // A regular piglin with a loved item in its OFFHAND admires it.
    world.apply_add_entity(protocol_add_entity(
        220,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [1.0, 64.0, -7.0],
    ));
    assert!(world.apply_set_equipment(equip(220, EquipmentSlot::OffHand, LOVED_ITEM_ID, false)));
    assert!(admiring(&world, 220));

    // A non-loved offhand item → no admiring.
    world.apply_add_entity(protocol_add_entity(
        221,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [2.0, 64.0, -7.0],
    ));
    assert!(world.apply_set_equipment(equip(221, EquipmentSlot::OffHand, PLAIN_ITEM_ID, false)));
    assert!(!admiring(&world, 221));

    // The piglin brute has no ADMIRING_ITEM branch, even with a loved offhand item.
    world.apply_add_entity(protocol_add_entity(
        222,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        [3.0, 64.0, -7.0],
    ));
    assert!(world.apply_set_equipment(equip(222, EquipmentSlot::OffHand, LOVED_ITEM_ID, false)));
    assert!(!admiring(&world, 222));

    // A dancing piglin (higher priority) does not admire.
    world.apply_add_entity(protocol_add_entity(
        223,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [4.0, 64.0, -7.0],
    ));
    assert!(world.apply_set_equipment(equip(223, EquipmentSlot::OffHand, LOVED_ITEM_ID, false)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 223,
        values: vec![protocol_bool_data(piglin_dancing_data_id(), true)],
    }));
    assert!(!admiring(&world, 223));

    // ADMIRING suppresses ATTACKING: an aggressive piglin with a tool main hand AND a loved offhand
    // admires (vanilla precedence ADMIRING > ATTACKING), so it does not swing.
    world.apply_add_entity(protocol_add_entity(
        224,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [5.0, 64.0, -7.0],
    ));
    assert!(world.apply_set_equipment(equip(224, EquipmentSlot::MainHand, TOOL_ITEM_ID, true)));
    assert!(world.apply_set_equipment(equip(224, EquipmentSlot::OffHand, LOVED_ITEM_ID, false)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 224,
        values: vec![mob_aggressive_data(VANILLA_ENTITY_TYPE_PIGLIN_ID)],
    }));
    let attacking_224 = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 224)
        .unwrap()
        .render_state
        .kind
        .piglin()
        .attacking_with_melee;
    assert!(
        admiring(&world, 224),
        "the loved offhand item makes it admire"
    );
    assert!(
        !attacking_224,
        "admiring (higher priority) suppresses the melee swing"
    );
}

#[test]
fn entity_model_instances_project_panda_unhappy_and_sneezing() {
    // Vanilla PandaRenderState projects unhappy/eat/sneeze counters and named flags from the Panda
    // metadata family; worried-gene pandas additionally become scared in a thundering level.
    let accessors = panda_metadata_accessors();
    let mut world = WorldStore::new();
    world.apply_login(&protocol_play_login(1));
    insert_loaded_entity_chunk(&mut world, ChunkPos { x: 0, z: -1 });
    world.apply_add_entity(protocol_add_entity(
        190,
        VANILLA_ENTITY_TYPE_PANDA_ID,
        [1.0, 64.0, -4.0],
    ));

    let panda = |world: &WorldStore, partial_tick: f32| {
        entity_model_instances_from_world_at_partial_tick(world, None, partial_tick)
            .into_iter()
            .find(|instance| instance.entity_id == 190)
            .unwrap()
            .render_state
    };

    // No data → content panda.
    let rest = panda(&world, 0.0);
    assert!(!rest.kind.panda().unhappy);
    assert!(!rest.kind.panda().sneezing);
    assert_eq!(rest.kind.panda().sneeze_time, 0);
    assert!(!rest.kind.panda().eating);
    assert!(!rest.kind.panda().sitting);
    assert!(!rest.kind.panda().scared);
    assert_eq!(rest.kind.panda().sit_amount, 0.0);
    assert_eq!(rest.kind.panda().lie_on_back_amount, 0.0);
    assert_eq!(rest.kind.panda().roll_amount, 0.0);
    assert_eq!(rest.kind.panda().roll_time, 0.0);

    // UNHAPPY_COUNTER > 0 projects the unhappy shake; the sneeze flag + counter project the dip; the
    // sitting bit + EAT_COUNTER project the held-item layer state. The same flags byte also feeds
    // the world-side sit/on-back/roll animation amounts that native copies into render state.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 190,
        values: vec![
            protocol_int_data(accessors.unhappy_counter_data_id, 12),
            protocol_int_data(accessors.sneeze_counter_data_id, 9),
            protocol_int_data(accessors.eat_counter_data_id, 4),
            protocol_byte_data(
                accessors.flags_data_id,
                accessors.sneezing_mask
                    | accessors.rolling_mask
                    | accessors.sitting_mask
                    | accessors.on_back_mask,
            ),
        ],
    }));
    world.advance_entity_client_animations(1);
    let active = panda(&world, 0.5);
    assert!(active.kind.panda().unhappy);
    assert!(active.kind.panda().sneezing);
    assert_eq!(active.kind.panda().sneeze_time, 9);
    assert!(active.kind.panda().eating);
    assert!(active.kind.panda().sitting);
    assert!(!active.kind.panda().scared);
    assert!((active.kind.panda().sit_amount - 0.075).abs() < 1.0e-6);
    assert!((active.kind.panda().lie_on_back_amount - 0.075).abs() < 1.0e-6);
    assert!((active.kind.panda().roll_amount - 0.075).abs() < 1.0e-6);
    assert!((active.kind.panda().roll_time - 1.5).abs() < 1.0e-6);

    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 2,
        param: 0.0,
    });
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 8,
        param: 0.9,
    });
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 190,
        values: vec![
            protocol_byte_data(accessors.main_gene_data_id, 2),
            protocol_byte_data(accessors.hidden_gene_data_id, 0),
        ],
    }));
    assert!(!panda(&world, 0.0).kind.panda().scared);
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: 8,
        param: 1.0,
    });
    assert!(panda(&world, 0.0).kind.panda().scared);

    // A zero unhappy counter is content again; clearing the flag stops the sneeze even with a counter.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 190,
        values: vec![
            protocol_int_data(accessors.unhappy_counter_data_id, 0),
            protocol_int_data(accessors.eat_counter_data_id, 0),
            protocol_byte_data(accessors.flags_data_id, 0),
        ],
    }));
    let calmed = panda(&world, 0.0);
    assert!(!calmed.kind.panda().unhappy);
    assert!(!calmed.kind.panda().sneezing);
    assert!(!calmed.kind.panda().eating);
    assert!(!calmed.kind.panda().sitting);
    assert!(calmed.kind.panda().scared);
}

#[test]
fn entity_model_instances_project_villager_unhappy() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        191,
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
        [1.0, 64.0, -4.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        192,
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        [2.0, 64.0, -4.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        193,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
        [3.0, 64.0, -4.0],
    ));

    let villager_unhappy = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .villager()
            .unhappy
    };

    assert!(!villager_unhappy(&world, 191));
    assert!(!villager_unhappy(&world, 192));

    for (id, entity_type_id) in [
        (191, VANILLA_ENTITY_TYPE_VILLAGER_ID),
        (192, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID),
    ] {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![abstract_villager_unhappy_data(entity_type_id, 12)],
        }));
    }
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 193,
        values: vec![abstract_villager_unhappy_collision_data(12)],
    }));
    assert!(villager_unhappy(&world, 191));
    assert!(villager_unhappy(&world, 192));
    assert!(
        !villager_unhappy(&world, 193),
        "zombie villagers do not use VillagerRenderState"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 191,
        values: vec![abstract_villager_unhappy_data(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
            0,
        )],
    }));
    assert!(!villager_unhappy(&world, 191));
}

#[test]
fn entity_model_instances_project_goat_ramming_head_tilt() {
    use std::f32::consts::PI;

    // Vanilla Goat.getRammingXHeadRot() = lowerHeadTick/20 · (baby ? 52.5 : 30)° · π/180, driven by
    // the world-projected ram counter (entity events 58/59). The baby max head pitch is steeper.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        200,
        VANILLA_ENTITY_TYPE_GOAT_ID,
        [1.0, 64.0, -5.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        201,
        VANILLA_ENTITY_TYPE_GOAT_ID,
        [2.0, 64.0, -5.0],
    ));
    // Make 201 a baby goat through the generated inherited accessor.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 201,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_GOAT_ID, true)],
    }));

    let ramming = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .goat()
            .ramming_x_head_rot
    };

    // At rest both goats project no head tilt.
    assert_eq!(ramming(&world, 200), 0.0);
    assert_eq!(ramming(&world, 201), 0.0);

    // Event 58 begins the ram; after 20 ticks the counter saturates at 20 (full tilt).
    for id in [200, 201] {
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: id,
            event_id: 58,
        }));
    }
    world.advance_entity_client_animations(20);
    // Adult: 30° → π/6; baby: 52.5°.
    assert!((ramming(&world, 200) - PI / 6.0).abs() < 1.0e-5);
    assert!((ramming(&world, 201) - 52.5_f32.to_radians()).abs() < 1.0e-5);
    assert!(
        ramming(&world, 201) > ramming(&world, 200),
        "the baby tilts its head further"
    );
}

#[test]
fn entity_model_instances_project_turtle_laying_egg() {
    // Vanilla Turtle.LAYING_EGG (BOOLEAN data id 19) and TurtleRenderer.extractRenderState:
    // state.isLayingEgg = entity.isLayingEgg(). Unlike hasEgg, this is NOT baby-gated (the
    // egg-laying amplitude lives in the shared TurtleModel).
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        150,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        [1.0, 64.0, -2.0],
    ));
    // A baby turtle (lays too), plus a non-turtle (bat) used to prove the type gating.
    world.apply_add_entity(protocol_add_entity(
        151,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        [2.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        152,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [3.0, 64.0, -2.0],
    ));

    let laying = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .turtle()
            .laying_egg
    };

    // A turtle that is not laying projects turtle_laying_egg = false.
    assert!(!laying(&world, 150));

    // Setting Turtle.LAYING_EGG (data id 19) projects through.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 150,
        values: vec![turtle_laying_egg_data(true)],
    }));
    assert!(laying(&world, 150));

    // A baby turtle DOES lay (no baby exclusion, unlike hasEgg).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 151,
        values: vec![
            baby_data(VANILLA_ENTITY_TYPE_TURTLE_ID, true),
            turtle_laying_egg_data(true),
        ],
    }));
    assert!(laying(&world, 151));

    // The same flag on a non-turtle (bat) does NOT project turtle_laying_egg.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 152,
        values: vec![turtle_laying_egg_collision_data(true)],
    }));
    assert!(!laying(&world, 152));
}

#[test]
fn entity_model_instances_project_end_crystal_shows_bottom() {
    // Vanilla EndCrystal.DATA_SHOW_BOTTOM (BOOLEAN, default true) and
    // EndCrystalRenderState.showsBottom = entity.showsBottom().
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        160,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        [1.0, 64.0, -2.0],
    ));
    // A non-crystal: the field defaults true (it is unused and never reads id 9).
    world.apply_add_entity(protocol_add_entity(
        161,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [2.0, 64.0, -2.0],
    ));

    let shows_bottom = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .end_crystal()
            .shows_bottom
    };

    // No synced value → the vanilla default `true` (the bottom slab is shown).
    assert!(shows_bottom(&world, 160));

    // Clearing DATA_SHOW_BOTTOM hides the base slab.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 160,
        values: vec![end_crystal_show_bottom_data(false)],
    }));
    assert!(!shows_bottom(&world, 160));

    // Re-setting it shows the base again.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 160,
        values: vec![end_crystal_show_bottom_data(true)],
    }));
    assert!(shows_bottom(&world, 160));

    // A non-crystal keeps the default `true` (unused).
    assert!(shows_bottom(&world, 161));
}

#[test]
fn entity_model_instances_project_end_crystal_beam_target() {
    // Vanilla EndCrystal.DATA_BEAM_TARGET (OPTIONAL_BLOCK_POS) projects as
    // EndCrystalRenderState.beamOffset = target block center - crystal position.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        162,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        [10.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        163,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [10.0, 64.0, -3.0],
    ));

    let beam = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .end_crystal()
            .beam
    };

    assert!(beam(&world, 162).is_none());

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 162,
        values: vec![end_crystal_beam_target_data(Some(
            bbb_protocol::packets::BlockPos {
                x: 14,
                y: 67,
                z: -10,
            },
        ))],
    }));
    assert_eq!(beam(&world, 162).unwrap().beam_offset, [4.5, 3.5, -6.5]);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 162,
        values: vec![end_crystal_beam_target_data(None)],
    }));
    assert!(beam(&world, 162).is_none());

    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 163,
        values: vec![end_crystal_beam_target_collision_data(Some(
            bbb_protocol::packets::BlockPos {
                x: 14,
                y: 67,
                z: -10,
            },
        ))],
    }));
    assert!(beam(&world, 163).is_none());
}

#[test]
fn entity_model_instances_project_bee_stinger() {
    // Vanilla Bee.DATA_FLAGS_ID and FLAG_HAS_STUNG are generated from Bee.java.
    let accessors = bee_metadata_accessors();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        96,
        VANILLA_ENTITY_TYPE_BEE_ID,
        [1.0, 64.0, -2.0],
    ));

    let has_stinger = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .bee()
            .has_stinger
    };

    // A bee that has not stung keeps its stinger.
    assert!(has_stinger(&world, 96));

    // Setting Bee.hasStung (DATA_FLAGS_ID & 4) hides the stinger cube.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 96,
        values: vec![protocol_byte_data(
            accessors.flags_data_id,
            accessors.has_stung_mask,
        )],
    }));
    assert!(!has_stinger(&world, 96));
}

#[test]
fn entity_model_instances_project_bee_angry_from_anger_end_time() {
    // Vanilla Bee.DATA_ANGER_END_TIME (LONG): isAngry = endTime > 0 && endTime - gameTime
    // > 0. The world has no time set here, so the game time defaults to 0.
    let accessors = bee_metadata_accessors();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        97,
        VANILLA_ENTITY_TYPE_BEE_ID,
        [1.0, 64.0, -2.0],
    ));

    let angry = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .bee()
            .angry
    };

    // A bee with no anger end time (default -1) is calm.
    assert!(!angry(&world, 97));

    // An anger end time in the future (game time 0) makes the bee angry.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 97,
        values: vec![protocol_long_data(accessors.anger_end_time_data_id, 200)],
    }));
    assert!(angry(&world, 97));

    // A zero/past end time is calm again.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 97,
        values: vec![protocol_long_data(accessors.anger_end_time_data_id, 0)],
    }));
    assert!(!angry(&world, 97));
}

#[test]
fn entity_model_instances_project_camel_class_sit_then_sit_pose_from_pose_change_tick() {
    // Vanilla Camel.LAST_POSE_CHANGE_TICK (LONG): the SIGN encodes sitting (< 0) and the
    // magnitude is the change tick. getPoseTime = gameTime - |LAST_POSE_CHANGE_TICK|. A camel that
    // sat at game tick 100 (so LAST_POSE_CHANGE_TICK = -100) plays CAMEL_SIT for getPoseTime < 40,
    // then CAMEL_SIT_POSE (which starts when the 40-tick sit-down window ends).
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        150,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        152,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        [2.0, 64.0, -2.0],
    ));
    // Sat down at game tick 100 (negative magnitude → sitting).
    for (id, entity_type_id) in [
        (150, VANILLA_ENTITY_TYPE_CAMEL_ID),
        (152, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID),
    ] {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![camel_last_pose_change_tick_data(entity_type_id, -100)],
        }));
    }

    let sit_seconds = |world: &WorldStore, id: i32| {
        let state = entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state;
        (
            state.kind.camel().sit_seconds,
            state.kind.camel().sit_pose_seconds,
            state.kind.camel().standup_seconds,
        )
    };

    // getPoseTime = 120 - 100 = 20 (< 40): inside the sit-down window, so CAMEL_SIT is active at
    // 20/20 = 1.0 s, and sit-pose / standup are the stopped sentinel.
    world.apply_world_time(PlayTime {
        game_time: 120,
        clock_updates: Vec::new(),
    });
    assert_eq!(sit_seconds(&world, 150), (1.0, -1.0, -1.0));
    assert_eq!(sit_seconds(&world, 152), (1.0, -1.0, -1.0));

    // getPoseTime = 160 - 100 = 60 (>= 40): past the sit-down window, so CAMEL_SIT_POSE takes over
    // at (60 - 40)/20 = 1.0 s, and sit / standup are stopped.
    world.apply_world_time(PlayTime {
        game_time: 160,
        clock_updates: Vec::new(),
    });
    assert_eq!(sit_seconds(&world, 150), (-1.0, 1.0, -1.0));
    assert_eq!(sit_seconds(&world, 152), (-1.0, 1.0, -1.0));

    // Standing back up at game tick 200 (positive magnitude → not sitting): getPoseTime = 210 -
    // 200 = 10 (< 52 STANDUP window, >= 0), so CAMEL_STANDUP is active at 10/20 = 0.5 s.
    for (id, entity_type_id) in [
        (150, VANILLA_ENTITY_TYPE_CAMEL_ID),
        (152, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID),
    ] {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![camel_last_pose_change_tick_data(entity_type_id, 200)],
        }));
    }
    world.apply_world_time(PlayTime {
        game_time: 210,
        clock_updates: Vec::new(),
    });
    assert_eq!(sit_seconds(&world, 150), (-1.0, -1.0, 0.5));
    assert_eq!(sit_seconds(&world, 152), (-1.0, -1.0, 0.5));

    // Long after standing up (getPoseTime = 300 - 200 = 100 >= 52): no transition, all stopped.
    world.apply_world_time(PlayTime {
        game_time: 300,
        clock_updates: Vec::new(),
    });
    assert_eq!(sit_seconds(&world, 150), (-1.0, -1.0, -1.0));
    assert_eq!(sit_seconds(&world, 152), (-1.0, -1.0, -1.0));
}

#[test]
fn camel_body_anchor_y_offset_is_exposed_for_native_attachment_projection() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        151,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 151,
        values: vec![
            entity_pose_data(VANILLA_ENTITY_POSE_SITTING_ID),
            camel_last_pose_change_tick_data(VANILLA_ENTITY_TYPE_CAMEL_ID, -100),
        ],
    }));
    world.apply_world_time(PlayTime {
        game_time: 160,
        clock_updates: Vec::new(),
    });
    let sitting_front = world
        .entity_body_anchor_y_offset(151, true, 0.0)
        .expect("camel body anchor y-offset");
    assert!(
        (sitting_front - 0.77).abs() < 1.0e-5,
        "sitting front anchor: {sitting_front}"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 151,
        values: vec![
            entity_pose_data(VANILLA_ENTITY_POSE_STANDING_ID),
            camel_last_pose_change_tick_data(VANILLA_ENTITY_TYPE_CAMEL_ID, 200),
        ],
    }));
    world.apply_world_time(PlayTime {
        game_time: 224,
        clock_updates: Vec::new(),
    });
    let standing_front_flex = world
        .entity_body_anchor_y_offset(151, true, 0.0)
        .expect("camel body anchor y-offset");
    assert!(
        (standing_front_flex - 1.508).abs() < 1.0e-5,
        "stand-up front flex anchor: {standing_front_flex}"
    );
}

#[test]
fn entity_model_instances_project_player_crouch_pose() {
    // Vanilla Entity.isCrouching reads the generated Entity pose accessor.

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        98,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -2.0],
    ));

    let crouching = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .is_crouching
    };
    let set_pose = |world: &mut WorldStore, id: i32, pose: i32| {
        world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![entity_pose_data(pose)],
        })
    };

    // A standing player is not crouching.
    assert!(!crouching(&world, 98));
    // Pose.CROUCHING projects the sneaking pose; standing again clears it.
    assert!(set_pose(&mut world, 98, VANILLA_ENTITY_POSE_CROUCHING_ID));
    assert!(crouching(&world, 98));
    assert!(set_pose(&mut world, 98, VANILLA_ENTITY_POSE_STANDING_ID));
    assert!(!crouching(&world, 98));
}

#[test]
fn entity_model_instances_project_upside_down_dinnerbone() {
    let mut world = WorldStore::new();
    // The sheep spawns with body yaw 100, then turns its head to 170: net yaw 70.
    world.apply_add_entity(protocol_add_entity_with_rotation(
        88,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
        30.0,
        -20.0,
        100.0,
    ));
    // A non-living entity (boat) is never flipped by LivingEntityRenderer.
    world.apply_add_entity(protocol_add_entity(
        89,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_rotate_head(bbb_protocol::packets::RotateHead {
        id: 88,
        y_head_rot: 170.0,
    }));
    world.advance_entity_client_animations(3);

    let render_state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    // A normally-named living entity is upright; its head look is unnegated.
    let upright = render_state(&world, 88);
    assert_eq!(upright.upside_down_height, None);
    assert_eq!(upright.head_yaw, 70.0);
    assert_eq!(upright.head_pitch, -20.0);

    // Vanilla LivingEntityRenderer.isUpsideDownName: the "Dinnerbone" name tag
    // flips the model; extractRenderState then negates the head yaw and pitch.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 88,
        values: vec![entity_custom_name_data(Some("Dinnerbone"))],
    }));
    let flipped = render_state(&world, 88);
    assert!(flipped
        .upside_down_height
        .is_some_and(|height| height > 0.0));
    assert_eq!(flipped.head_yaw, -70.0);
    assert_eq!(flipped.head_pitch, 20.0);

    // "Grumm" flips too; an unrelated name does not.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 88,
        values: vec![entity_custom_name_data(Some("Grumm"))],
    }));
    assert!(render_state(&world, 88).upside_down_height.is_some());
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 88,
        values: vec![entity_custom_name_data(Some("Steve"))],
    }));
    assert_eq!(render_state(&world, 88).upside_down_height, None);

    // A non-living entity named Dinnerbone is still never flipped.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 89,
        values: vec![entity_custom_name_data(Some("Dinnerbone"))],
    }));
    assert_eq!(render_state(&world, 89).upside_down_height, None);
}

#[test]
fn entity_model_instances_project_sleeping_pose() {
    let mut world = WorldStore::new();
    // A sheep facing body yaw 45 with no bed resolved (no chunk loaded).
    world.apply_add_entity(protocol_add_entity_with_rotation(
        93,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
        45.0,
        0.0,
        45.0,
    ));

    let sleeping = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .sleeping
    };

    // An awake entity is not laid down.
    assert_eq!(sleeping(&world, 93), None);

    // Vanilla Pose.SLEEPING with no resolvable bed falls back to the body yaw and
    // no head offset (setupRotations `angle = bodyRot`).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 93,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SLEEPING_ID)],
    }));
    assert_eq!(
        sleeping(&world, 93),
        Some(SleepingPose {
            yaw_angle: 45.0,
            bed_offset: [0.0, 0.0],
        })
    );
}

#[test]
fn entity_model_instances_project_scale_attribute() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        97,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        [1.0, 64.0, -2.0],
    ));

    let scale = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 97)
            .unwrap()
            .render_state
            .scale
    };

    // Default size projects scale 1.0.
    assert_eq!(scale(&world), 1.0);

    // Vanilla getScale() (the SCALE attribute) flows through to the render state.
    assert!(world.apply_update_attributes(UpdateAttributes {
        entity_id: 97,
        attributes: vec![AttributeSnapshot {
            attribute_id: VANILLA_ATTRIBUTE_SCALE_ID,
            base: 1.25,
            modifiers: Vec::new(),
        }],
    }));
    assert_eq!(scale(&world), 1.25);
}

#[test]
fn on_fire_model_source_reaches_dispatcher_flame_dimensions() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        910,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [0.0, 64.0, 0.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 910,
        values: vec![entity_on_fire_data()],
    }));

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 910)
        .unwrap();
    assert_eq!(instance.render_state.flame_dimensions, Some([0.4, 0.7]));
}

#[test]
fn entity_model_instances_project_full_bright_light_without_chunk_data() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        90,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        instances[0].render_state.light_coords,
        bbb_client_render::ENTITY_FULL_BRIGHT_LIGHT_COORDS
    );
}

#[test]
fn entity_model_instances_project_hurt_red_overlay_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        91,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));

    let calm = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(!calm[0].render_state.has_red_overlay);

    assert!(world.apply_hurt_animation(bbb_protocol::packets::HurtAnimation { id: 91, yaw: 0.0 }));
    let hurt = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(hurt[0].render_state.has_red_overlay);
}

#[test]
fn creeper_white_overlay_progress_matches_vanilla_strobe() {
    // (int)(step * 10) even -> 0.0, odd -> clamp(step, 0.5, 1.0).
    assert_eq!(creeper_white_overlay_progress(0.0), 0.0);
    assert_eq!(creeper_white_overlay_progress(0.15), 0.5); // bucket 1 (odd), clamped up
    assert_eq!(creeper_white_overlay_progress(0.55), 0.55); // bucket 5 (odd)
    assert_eq!(creeper_white_overlay_progress(0.6), 0.0); // bucket 6 (even)
}

#[test]
fn entity_model_instances_project_creeper_white_overlay_from_world() {
    let accessors = creeper_metadata_accessors();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        92,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        [1.0, 64.0, -2.0],
    ));
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(resting[0].render_state.white_overlay_progress, 0.0);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 92,
        values: vec![EntityDataValue {
            data_id: accessors.swell_dir_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(1),
        }],
    }));
    world.advance_entity_client_animations(5);

    // swell = 5, getSwelling(1.0) = 5/28; the strobe lands in an odd bucket
    // so the projected progress is the clamped swelling (>= 0.5).
    let swelling = 5.0 / 28.0;
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        instances[0].render_state.white_overlay_progress,
        creeper_white_overlay_progress(swelling)
    );
    assert!(instances[0].render_state.white_overlay_progress >= 0.5);
}

#[test]
fn entity_model_instances_project_creeper_swelling_from_world() {
    let accessors = creeper_metadata_accessors();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        93,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        [1.0, 64.0, -2.0],
    ));
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        resting[0].render_state.kind.creeper().swelling,
        0.0,
        "a calm creeper carries no swell, so CreeperRenderer.scale is the identity"
    );

    // Prime the creeper: swell direction = 1 advances the swell counter each tick, and
    // vanilla `Creeper.getSwelling(partialTick) = swell / SWELL_MAX (28)`.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 93,
        values: vec![EntityDataValue {
            data_id: accessors.swell_dir_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(1),
        }],
    }));
    world.advance_entity_client_animations(5);

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        instances[0].render_state.kind.creeper().swelling,
        5.0 / 28.0,
        "the projected swell drives the renderer inflate-and-flicker scale"
    );
}

#[test]
fn entity_model_instances_project_charged_creeper_from_world() {
    let accessors = creeper_metadata_accessors();

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        97,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
        [1.0, 64.0, -2.0],
    ));
    // A plain creeper is not powered, so it wears no CreeperPowerLayer energy swirl.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(!resting[0].render_state.kind.creeper().powered);

    // Vanilla `Creeper.DATA_IS_POWERED`: set true for a lightning-charged creeper.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 97,
        values: vec![EntityDataValue {
            data_id: accessors.powered_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(
        instances[0].render_state.kind.creeper().powered,
        "the charged creeper projects isPowered, gating the energy-swirl overlay"
    );
}

#[test]
fn entity_model_instances_project_powered_wither_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        145,
        VANILLA_ENTITY_TYPE_WITHER_ID,
        [3.0, 64.0, 1.0],
    ));
    // A wither with no synced health defaults to full (maxHealth 300), so it is not powered.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(!resting[0].render_state.kind.wither().powered);

    // A healthy wither (health 200/300 > 150) stays un-powered.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 145,
        values: vec![living_entity_health_data(
            VANILLA_ENTITY_TYPE_WITHER_ID,
            200.0
        )],
    }));
    let healthy = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(!healthy[0].render_state.kind.wither().powered);

    // Vanilla `WitherBoss.isPowered() = getHealth() <= getMaxHealth() / 2`: at or below half
    // health (120 ≤ 150) the `WitherArmorLayer` energy swirl ignites.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 145,
        values: vec![living_entity_health_data(
            VANILLA_ENTITY_TYPE_WITHER_ID,
            120.0
        )],
    }));
    let powered = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert!(
        powered[0].render_state.kind.wither().powered,
        "the half-health wither projects isPowered, gating the energy-swirl overlay"
    );
}

#[test]
fn entity_model_instances_project_wither_side_head_rotations() {
    // Vanilla WitherBoss.DATA_TARGET_B: the first side-head target id.

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        145,
        VANILLA_ENTITY_TYPE_WITHER_ID,
        [0.0, 64.0, 0.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        26,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [0.0, 64.0, 0.0],
    ));
    let target_eye = f64::from(world.probe_entity_camera_pose(26).unwrap().eye_height);
    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 26,
        position: Vec3d {
            x: 11.3,
            y: 66.2 - target_eye,
            z: 0.0,
        },
        delta_movement: Vec3d::default(),
        y_rot: 0.0,
        x_rot: 0.0,
        on_ground: true,
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 145,
        values: vec![protocol_wither_target_b_data(26)],
    }));
    world.advance_entity_client_animations(1);

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5);
    let wither = instances
        .iter()
        .find(|instance| instance.entity_id == 145)
        .expect("wither instance");
    assert!(wither.render_state.kind.wither().x_head_rots[0].abs() < 1.0e-5);
    assert_eq!(wither.render_state.kind.wither().x_head_rots[1], 0.0);
    assert_eq!(wither.render_state.kind.wither().y_head_rots, [-10.0, 0.0]);
}

fn protocol_wither_target_b_data(target_id: i32) -> EntityDataValue {
    let accessor = bbb_client_world::vanilla_entity_metadata_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_WITHER_ID,
        "DATA_TARGET_B",
    )
    .expect("generated WitherBoss target-B accessor must resolve");
    assert_eq!(
        accessor.owner,
        "net.minecraft.world.entity.boss.wither.WitherBoss"
    );
    protocol_int_data(accessor.data_id, target_id)
}

#[test]
fn entity_model_kind_uses_vanilla_chicken_variant_metadata() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_CHICKEN_ID, &[]),
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            &[protocol_chicken_variant_data(1)]
        ),
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Warm,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            &[
                protocol_chicken_variant_data(2),
                baby_data(VANILLA_ENTITY_TYPE_CHICKEN_ID, true),
            ]
        ),
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Cold,
            baby: true
        }
    );
}

#[test]
fn entity_model_instances_project_chicken_variants_from_world_registry_order() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:chicken_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:cold"),
            RegistryPacketEntry::stub("minecraft:temperate"),
            RegistryPacketEntry::stub("minecraft:warm"),
        ],
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            &[protocol_chicken_variant_data(99)]
        ),
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
    );
    world.apply_add_entity(protocol_add_entity(
        26,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        27,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 26,
        values: vec![protocol_chicken_variant_data(0)],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 27,
        values: vec![
            protocol_chicken_variant_data(2),
            baby_data(VANILLA_ENTITY_TYPE_CHICKEN_ID, true),
        ],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        without_bounds(&instances),
        aged(
            vec![
                EntityModelInstance::chicken_variant(
                    26,
                    [1.0, 64.0, -2.0],
                    0.0,
                    ChickenModelVariant::Cold,
                    false
                )
                .with(|s| s.kind = EntityKindRenderState::Chicken(Default::default())),
                EntityModelInstance::chicken_variant(
                    27,
                    [3.0, 64.0, -2.0],
                    0.0,
                    ChickenModelVariant::Warm,
                    true
                )
                .with(|s| s.kind = EntityKindRenderState::Chicken(Default::default()))
                .with(|s| s.age_scale = 0.5),
            ],
            1.0,
        )
    );
}

#[test]
fn entity_model_kind_uses_vanilla_cow_variant_metadata() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_COW_ID, &[]),
        EntityModelKind::Cow {
            variant: CowModelVariant::Temperate,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_COW_ID, &[protocol_cow_variant_data(1)]),
        EntityModelKind::Cow {
            variant: CowModelVariant::Warm,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_COW_ID,
            &[
                protocol_cow_variant_data(2),
                baby_data(VANILLA_ENTITY_TYPE_COW_ID, true),
            ]
        ),
        EntityModelKind::Cow {
            variant: CowModelVariant::Cold,
            baby: true
        }
    );
}

#[test]
fn entity_model_instances_project_cow_variants_from_world_registry_order() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:cow_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:cold"),
            RegistryPacketEntry::stub("minecraft:temperate"),
            RegistryPacketEntry::stub("minecraft:warm"),
        ],
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_COW_ID, &[protocol_cow_variant_data(99)]),
        EntityModelKind::Cow {
            variant: CowModelVariant::Temperate,
            baby: false
        }
    );
    world.apply_add_entity(protocol_add_entity(
        30,
        VANILLA_ENTITY_TYPE_COW_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        31,
        VANILLA_ENTITY_TYPE_COW_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 30,
        values: vec![protocol_cow_variant_data(0)],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 31,
        values: vec![
            protocol_cow_variant_data(2),
            baby_data(VANILLA_ENTITY_TYPE_COW_ID, true),
        ],
    }));

    let start_instances = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        start_instances, instances,
        "settled cows keep the same geometry"
    );
    assert_eq!(
        without_bounds(&instances),
        vec![
            EntityModelInstance::cow_variant(
                30,
                [1.0, 64.0, -2.0],
                0.0,
                CowModelVariant::Cold,
                false
            ),
            EntityModelInstance::cow_variant(
                31,
                [3.0, 64.0, -2.0],
                0.0,
                CowModelVariant::Warm,
                true
            )
            .with(|s| s.age_scale = 0.5),
        ]
    );
}

#[test]
fn entity_model_kind_uses_vanilla_pig_variant_metadata() {
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PIG_ID, &[]),
        EntityModelKind::Pig {
            variant: PigModelVariant::Temperate,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PIG_ID, &[protocol_pig_variant_data(1)]),
        EntityModelKind::Pig {
            variant: PigModelVariant::Warm,
            baby: false
        }
    );
    assert_eq!(
        entity_model_kind(
            VANILLA_ENTITY_TYPE_PIG_ID,
            &[
                protocol_pig_variant_data(2),
                baby_data(VANILLA_ENTITY_TYPE_PIG_ID, true),
            ]
        ),
        EntityModelKind::Pig {
            variant: PigModelVariant::Cold,
            baby: true
        }
    );
}

#[test]
fn entity_model_instances_project_pig_variants_from_world_registry_order() {
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:pig_variant",
        0,
        vec![
            RegistryPacketEntry::stub("minecraft:cold"),
            RegistryPacketEntry::stub("minecraft:temperate"),
            RegistryPacketEntry::stub("minecraft:warm"),
        ],
    );
    assert_eq!(
        entity_model_kind(VANILLA_ENTITY_TYPE_PIG_ID, &[protocol_pig_variant_data(99)]),
        EntityModelKind::Pig {
            variant: PigModelVariant::Temperate,
            baby: false
        }
    );
    world.apply_add_entity(protocol_add_entity(
        100,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        101,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 100,
        values: vec![protocol_pig_variant_data(0)],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 101,
        values: vec![
            protocol_pig_variant_data(2),
            baby_data(VANILLA_ENTITY_TYPE_PIG_ID, true),
        ],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        without_bounds(&instances),
        aged(
            vec![
                EntityModelInstance::pig(100, [1.0, 64.0, -2.0], 0.0, PigModelVariant::Cold, false,).with(|s| s.kind = EntityKindRenderState::Pig(Default::default())),
                EntityModelInstance::pig(101, [3.0, 64.0, -2.0], 0.0, PigModelVariant::Warm, true,).with(|s| s.kind = EntityKindRenderState::Pig(Default::default()))
                    .with(|s| s.age_scale = 0.5),
            ],
            1.0,
        )
    );
}
