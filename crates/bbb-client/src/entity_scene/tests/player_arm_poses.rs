use super::*;

#[test]
fn entity_model_instances_project_player_spear_use_kinetic_state() {
    // Vanilla `AvatarRenderer.getArmPose` returns `SPEAR` while a player uses a spear, and
    // `SpearAnimations` reads the effective stack `KineticWeapon` component. The same
    // pose has `affectsOffhandPose`, so an off-hand spear use suppresses the main-hand ITEM fallback.
    const WOODEN_SPEAR_ID: i32 = 0;
    const IRON_SPEAR_ID: i32 = 1;
    const PLAIN_ITEM_ID: i32 = 2;

    let registry: bbb_client_assets::ItemRegistryCatalog =
        serde_json::from_value(serde_json::json!({
            "resource_ids": [
                "minecraft:wooden_spear",
                "minecraft:iron_spear",
                "minecraft:stick"
            ],
            "protocol_ids": {
                "minecraft:wooden_spear": WOODEN_SPEAR_ID,
                "minecraft:iron_spear": IRON_SPEAR_ID,
                "minecraft:stick": PLAIN_ITEM_ID
            },
            "default_component_codec_values": vanilla_kinetic_weapon_fixture(&[
                "minecraft:wooden_spear",
                "minecraft:iron_spear"
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
    let using = |id: i32, off_hand: bool| SetEntityData {
        id,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            off_hand,
            false,
        )],
    };
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    let mut world = WorldStore::new();
    install_kinetic_runtime_defaults(&mut world, &runtime);
    world.apply_add_entity(protocol_add_entity(
        241,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(241, EquipmentSlot::MainHand, WOODEN_SPEAR_ID)));
    assert!(world.apply_set_entity_data(using(241, false)));
    let main = state(&world, 241);
    assert_eq!(
        main.kind.player().using_spear,
        Some(SpearKineticWeapon {
            delay_ticks: 15.0,
            dismount_duration_ticks: 100.0,
            knockback_duration_ticks: 200.0,
            damage_duration_ticks: 300.0,
            forward_movement: 0.38,
        })
    );
    assert!(!main.kind.player().main_hand_item_pose);
    assert!(!main.kind.player().main_hand_spear_pose);
    assert!(!main.kind.player().off_hand_spear_pose);

    assert!(world.apply_entity_event(EntityEvent {
        entity_id: 241,
        event_id: 2,
    }));
    world.advance_entity_client_animations(2);
    let hit_feedback =
        entity_model_instances_from_world_at_partial_tick(&world, Some(&runtime), 0.5)
            .into_iter()
            .find(|instance| instance.entity_id == 241)
            .unwrap()
            .render_state;
    assert_eq!(hit_feedback.ticks_since_kinetic_hit_feedback, 2.5);

    world.apply_add_entity(protocol_add_entity(
        242,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(242, EquipmentSlot::MainHand, PLAIN_ITEM_ID)));
    assert!(world.apply_set_equipment(equip(242, EquipmentSlot::OffHand, IRON_SPEAR_ID)));
    assert!(world.apply_set_entity_data(using(242, true)));
    let off = state(&world, 242);
    assert!(off.use_item_off_hand);
    assert_eq!(
        off.kind.player().using_spear,
        Some(SpearKineticWeapon {
            delay_ticks: 12.0,
            dismount_duration_ticks: 50.0,
            knockback_duration_ticks: 135.0,
            damage_duration_ticks: 225.0,
            forward_movement: 0.38,
        })
    );
    assert!(!off.kind.player().main_hand_item_pose);
    assert!(!off.kind.player().off_hand_item_pose);
    assert!(!off.kind.player().main_hand_spear_pose);
    assert!(!off.kind.player().off_hand_spear_pose);
}

#[test]
fn entity_model_instances_project_player_held_spear_arm_pose() {
    // Vanilla `AvatarRenderer.getArmPose` returns `SPEAR` for a held spear even when it is not being
    // used. In the normal right-handed `HumanoidModel.setupAnim` dispatch, the off-hand pose runs first;
    // an off-hand `SPEAR` has `affectsOffhandPose`, so it suppresses the main-hand ITEM pose. A charged
    // main-hand `CROSSBOW_HOLD` is the two-handed exception: it forces the off hand to ITEM first. A
    // main-hand spear does not suppress a plain off-hand ITEM because the off hand is posed first.
    const WOODEN_SPEAR_ID: i32 = 0;
    const IRON_SPEAR_ID: i32 = 1;
    const PLAIN_ITEM_ID: i32 = 2;
    const CROSSBOW_ID: i32 = 3;

    let registry: bbb_client_assets::ItemRegistryCatalog =
        serde_json::from_value(serde_json::json!({
            "resource_ids": [
                "minecraft:wooden_spear",
                "minecraft:iron_spear",
                "minecraft:stick",
                "minecraft:crossbow"
            ],
            "protocol_ids": {
                "minecraft:wooden_spear": WOODEN_SPEAR_ID,
                "minecraft:iron_spear": IRON_SPEAR_ID,
                "minecraft:stick": PLAIN_ITEM_ID,
                "minecraft:crossbow": CROSSBOW_ID
            },
            "default_component_codec_values": vanilla_kinetic_weapon_fixture(&[
                "minecraft:wooden_spear",
                "minecraft:iron_spear"
            ])
        }))
        .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_client_assets::EquipmentAssetCatalog::default(),
    );
    let equip = |entity_id: i32, slot: EquipmentSlot, item_id: i32, charged: bool| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    charged_projectiles_items: if charged {
                        vec![bbb_protocol::packets::ItemStackTemplateSummary {
                            item_id: PLAIN_ITEM_ID,
                            count: 1,
                            component_patch: DataComponentPatchSummary::default().into(),
                        }]
                    } else {
                        Vec::new()
                    },
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
        }],
    };
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    let mut world = WorldStore::new();
    install_kinetic_runtime_defaults(&mut world, &runtime);
    world.apply_add_entity(protocol_add_entity(
        243,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [3.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(243, EquipmentSlot::MainHand, WOODEN_SPEAR_ID, false,)));
    assert!(world.apply_set_equipment(equip(243, EquipmentSlot::OffHand, PLAIN_ITEM_ID, false,)));
    let main_spear = state(&world, 243);
    assert!(main_spear.kind.player().main_hand_spear_pose);
    assert!(!main_spear.kind.player().off_hand_spear_pose);
    assert!(!main_spear.humanoid_mob_main_hand_spear_pose);
    assert!(!main_spear.humanoid_mob_off_hand_spear_pose);
    assert!(!main_spear.kind.player().main_hand_item_pose);
    assert!(
        main_spear.kind.player().off_hand_item_pose,
        "a main-hand spear does not suppress the plain off-hand ITEM pose"
    );

    world.apply_add_entity(protocol_add_entity(
        244,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [4.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(244, EquipmentSlot::MainHand, PLAIN_ITEM_ID, false,)));
    assert!(world.apply_set_equipment(equip(244, EquipmentSlot::OffHand, IRON_SPEAR_ID, false,)));
    let off_spear = state(&world, 244);
    assert!(!off_spear.kind.player().main_hand_spear_pose);
    assert!(off_spear.kind.player().off_hand_spear_pose);
    assert!(!off_spear.humanoid_mob_main_hand_spear_pose);
    assert!(!off_spear.humanoid_mob_off_hand_spear_pose);
    assert!(
        !off_spear.kind.player().main_hand_item_pose,
        "off-hand SPEAR affectsOffhandPose skips the main-hand ITEM pose"
    );
    assert!(!off_spear.kind.player().off_hand_item_pose);

    world.apply_add_entity(protocol_add_entity(
        245,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [5.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(245, EquipmentSlot::MainHand, CROSSBOW_ID, true,)));
    assert!(world.apply_set_equipment(equip(245, EquipmentSlot::OffHand, IRON_SPEAR_ID, false,)));
    let off_spear_over_crossbow = state(&world, 245);
    assert!(!off_spear_over_crossbow.kind.player().off_hand_spear_pose);
    assert!(
        off_spear_over_crossbow.kind.player().crossbow_hold,
        "main-hand CROSSBOW_HOLD isTwoHanded forces a non-empty off hand to ITEM first"
    );
    assert!(
            off_spear_over_crossbow.kind.player().off_hand_item_pose,
            "the forced off-hand ITEM pose replaces the held SPEAR pose before CROSSBOW_HOLD overwrites it"
        );
}

#[test]
fn entity_model_instances_project_humanoid_mob_held_spear_arm_pose() {
    // Vanilla `HumanoidMobRenderer.getArmPose` returns `SPEAR` for a non-player humanoid mob holding an
    // item tagged `minecraft:spears`. `AbstractZombieRenderer` additionally checks the opposite hand's
    // STAB swing component, so a zombie-family mob with a spear in either hand marks both arm poses.
    const WOODEN_SPEAR_ID: i32 = 0;
    const IRON_SPEAR_ID: i32 = 1;
    const PLAIN_ITEM_ID: i32 = 2;

    let registry: bbb_client_assets::ItemRegistryCatalog =
        serde_json::from_value(serde_json::json!({
            "resource_ids": [
                "minecraft:wooden_spear",
                "minecraft:iron_spear",
                "minecraft:stick"
            ],
            "protocol_ids": {
                "minecraft:wooden_spear": WOODEN_SPEAR_ID,
                "minecraft:iron_spear": IRON_SPEAR_ID,
                "minecraft:stick": PLAIN_ITEM_ID
            },
            "default_component_codec_values": vanilla_kinetic_weapon_fixture(&[
                "minecraft:wooden_spear",
                "minecraft:iron_spear"
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
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    let mut world = WorldStore::new();
    install_kinetic_runtime_defaults(&mut world, &runtime);
    world.apply_add_entity(protocol_add_entity(
        246,
        VANILLA_ENTITY_TYPE_SKELETON_ID,
        [6.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(246, EquipmentSlot::MainHand, WOODEN_SPEAR_ID)));
    let skeleton_main = state(&world, 246);
    assert!(skeleton_main.humanoid_mob_main_hand_spear_pose);
    assert!(!skeleton_main.humanoid_mob_off_hand_spear_pose);
    assert!(!skeleton_main.kind.player().main_hand_spear_pose);

    world.apply_add_entity(protocol_add_entity(
        247,
        VANILLA_ENTITY_TYPE_SKELETON_ID,
        [7.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(247, EquipmentSlot::MainHand, PLAIN_ITEM_ID)));
    assert!(world.apply_set_equipment(equip(247, EquipmentSlot::OffHand, IRON_SPEAR_ID)));
    let skeleton_off = state(&world, 247);
    assert!(!skeleton_off.humanoid_mob_main_hand_spear_pose);
    assert!(skeleton_off.humanoid_mob_off_hand_spear_pose);

    world.apply_add_entity(protocol_add_entity(
        248,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [8.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(248, EquipmentSlot::MainHand, WOODEN_SPEAR_ID)));
    let zombie_main = state(&world, 248);
    assert!(zombie_main.humanoid_mob_main_hand_spear_pose);
    assert!(
        zombie_main.humanoid_mob_off_hand_spear_pose,
        "AbstractZombieRenderer checks the opposite main-hand STAB component for the off arm"
    );

    world.apply_add_entity(protocol_add_entity(
        249,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [9.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(249, EquipmentSlot::OffHand, IRON_SPEAR_ID)));
    let zombie_off = state(&world, 249);
    assert!(
        zombie_off.humanoid_mob_main_hand_spear_pose,
        "AbstractZombieRenderer checks the opposite off-hand STAB component for the main arm"
    );
    assert!(zombie_off.humanoid_mob_off_hand_spear_pose);

    world.apply_add_entity(protocol_add_entity(
        250,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [10.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(250, EquipmentSlot::MainHand, WOODEN_SPEAR_ID)));
    let piglin_main = state(&world, 250);
    assert!(piglin_main.humanoid_mob_main_hand_spear_pose);
    assert!(
            !piglin_main.humanoid_mob_off_hand_spear_pose,
            "PiglinRenderer inherits the base same-hand HumanoidMobRenderer pose, not the zombie opposite-hand override"
        );

    world.apply_add_entity(protocol_add_entity(
        251,
        VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
        [11.0, 64.0, -9.0],
    ));
    assert!(world.apply_set_equipment(equip(251, EquipmentSlot::OffHand, IRON_SPEAR_ID)));
    let zombified_off = state(&world, 251);
    assert!(!zombified_off.humanoid_mob_main_hand_spear_pose);
    assert!(zombified_off.humanoid_mob_off_hand_spear_pose);
}

#[test]
fn entity_model_instances_spyglass_pose_needs_a_resolved_spyglass() {
    // The SPYGLASS use-item pose needs the using-hand item resolved through the item registry to
    // confirm a spyglass; without an item runtime it can never resolve, so the projection defaults
    // off even for a player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        250,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, -10.0],
    ));
    // Flag the player as using an item (so only the missing runtime, not the flag, gates the pose off).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 250,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let spyglass = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 250)
        .unwrap()
        .render_state
        .kind
        .player()
        .using_spyglass;
    assert!(!spyglass);
}

#[test]
fn entity_model_instances_horn_pose_needs_a_resolved_goat_horn() {
    // The TOOT_HORN use-item pose needs the using-hand item resolved through the item registry to
    // confirm a goat horn; without an item runtime it can never resolve, so the projection defaults
    // off even for a player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        251,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 251,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let tooting = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 251)
        .unwrap()
        .render_state
        .kind
        .player()
        .tooting_horn;
    assert!(!tooting);
}

#[test]
fn entity_model_instances_brush_pose_needs_a_resolved_brush() {
    // The BRUSH use-item pose needs the using-hand item resolved through the item registry to confirm
    // a brush; without an item runtime it can never resolve, so the projection defaults off even for a
    // player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        252,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [3.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 252,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let brushing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 252)
        .unwrap()
        .render_state
        .kind
        .player()
        .brushing;
    assert!(!brushing);
}

#[test]
fn entity_model_instances_block_pose_uses_shield_or_blocks_attacks_component() {
    // The BLOCK use-item pose needs a non-consumable `BLOCKS_ATTACKS` item. Vanilla shields resolve from
    // the item registry, while datapack/patch-granted blockers are visible in `added_type_ids` and work
    // without an item runtime.
    const PLAIN_ITEM_ID: i32 = 730;

    let mut world = WorldStore::new();
    let equip = |entity_id: i32, added_type_ids: Vec<i32>| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id: Some(PLAIN_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added_type_ids,
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
        }],
    };
    let set_using = |id: i32| SetEntityData {
        id,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    };
    let blocking = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .player()
            .blocking
    };

    world.apply_add_entity(protocol_add_entity(
        253,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [4.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(set_using(253)));
    assert!(!blocking(&world, 253));

    world.apply_add_entity(protocol_add_entity(
        254,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [5.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(
        254,
        vec![VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID]
    )));
    assert!(world.apply_set_entity_data(set_using(254)));
    assert!(blocking(&world, 254));

    // `Item.getUseAnimation` checks CONSUMABLE before BLOCKS_ATTACKS, so a stack adding both routes to
    // EAT/DRINK rather than the BLOCK pose.
    world.apply_add_entity(protocol_add_entity(
        255,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [6.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(
        255,
        vec![
            VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID
        ]
    )));
    assert!(world.apply_set_entity_data(set_using(255)));
    assert!(!blocking(&world, 255));
}

#[test]
fn entity_model_instances_throw_trident_pose_needs_a_resolved_trident() {
    // The THROW_TRIDENT use-item pose needs the using-hand item resolved through the item registry to
    // confirm a trident; without an item runtime it can never resolve, so the projection defaults off
    // even for a player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        254,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [5.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 254,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let throwing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 254)
        .unwrap()
        .render_state
        .kind
        .player()
        .throwing_trident;
    assert!(!throwing);
}

#[test]
fn entity_model_instances_bow_draw_pose_needs_a_resolved_bow() {
    // The BOW_AND_ARROW use-item pose needs the using-hand item resolved through the item registry to
    // confirm a bow; without an item runtime it can never resolve, so the projection defaults off even
    // for a player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        255,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [6.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 255,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let drawing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 255)
        .unwrap()
        .render_state
        .kind
        .player()
        .drawing_bow;
    assert!(!drawing);
}

#[test]
fn entity_model_instances_crossbow_charge_pose_needs_a_resolved_crossbow() {
    // The CROSSBOW_CHARGE use-item pose needs the using-hand item resolved through the item registry to
    // confirm a crossbow; without an item runtime it can never resolve, so the projection defaults off
    // even for a player flagged as using an item.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        256,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [7.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 256,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    let charging = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 256)
        .unwrap()
        .render_state
        .kind
        .player()
        .charging_crossbow;
    assert!(!charging);
}

#[test]
fn entity_model_instances_project_synchronized_crossbow_charge_duration() {
    const CROSSBOW_ID: i32 = 0;
    const ENTITY_ID: i32 = 2_560;
    let registry: bbb_client_assets::ItemRegistryCatalog =
        serde_json::from_value(serde_json::json!({
            "resource_ids": ["minecraft:crossbow"],
            "protocol_ids": { "minecraft:crossbow": CROSSBOW_ID }
        }))
        .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_client_assets::EquipmentAssetCatalog::default(),
    );
    let mut world = WorldStore::new();
    world.record_registry_entries(
        "minecraft:enchantment",
        0,
        vec![RegistryPacketEntry::with_raw_data(
            "example:patient_loading",
            crossbow_charge_time_enchantment_nbt(0.75),
        )],
    );
    world.begin_registry_configuration();
    world.finish_registry_configuration().unwrap();
    world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [7.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: ENTITY_ID,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id: Some(CROSSBOW_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: 0,
                        level: 1,
                    }],
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
        }],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: ENTITY_ID,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));

    let state = entity_model_instances_from_world_at_partial_tick(&world, Some(&runtime), 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == ENTITY_ID)
        .unwrap()
        .render_state;
    assert!(state.kind.player().charging_crossbow);
    assert_eq!(state.crossbow_charge_duration_ticks, 15.0);
}

#[test]
fn entity_model_instances_crossbow_hold_pose_needs_a_resolved_charged_crossbow() {
    // The CROSSBOW_HOLD pose needs the held item resolved through the item registry to confirm a CHARGED
    // crossbow; without an item runtime neither the main-hand nor off-hand projection can resolve, so
    // both default off even for a (non-swinging) player.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        257,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [8.0, 64.0, -10.0],
    ));
    let render_state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 257)
        .unwrap()
        .render_state;
    assert!(!render_state.kind.player().crossbow_hold);
    assert!(!render_state.kind.player().crossbow_hold_off_hand);
}

#[test]
fn entity_model_instances_project_off_hand_bow_and_crossbow_player_poses() {
    // Vanilla `AvatarRenderer.getArmPose` selects the BOW/CROSSBOW use pose from the used hand, and
    // `CROSSBOW_HOLD` before the use-item branch from either hand. The renderer owns the exact arm math;
    // this test proves native projects the correct using-hand/off-hand booleans from resolved items.
    const BOW_ID: i32 = 0;
    const CROSSBOW_ID: i32 = 1;

    let registry: bbb_client_assets::ItemRegistryCatalog =
        serde_json::from_value(serde_json::json!({
            "resource_ids": [
                "minecraft:bow",
                "minecraft:crossbow"
            ],
            "protocol_ids": {
                "minecraft:bow": BOW_ID,
                "minecraft:crossbow": CROSSBOW_ID
            }
        }))
        .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_client_assets::EquipmentAssetCatalog::default(),
    );
    let equip = |entity_id: i32, slot: EquipmentSlot, item_id: i32, charged: bool| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    charged_projectiles_items: if charged {
                        vec![bbb_protocol::packets::ItemStackTemplateSummary {
                            item_id: BOW_ID,
                            count: 1,
                            component_patch: DataComponentPatchSummary::default().into(),
                        }]
                    } else {
                        Vec::new()
                    },
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            },
        }],
    };
    let use_off_hand = |id: i32| SetEntityData {
        id,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            true,
            false,
        )],
    };
    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    let mut world = WorldStore::new();

    world.apply_add_entity(protocol_add_entity(
        258,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [9.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(258, EquipmentSlot::OffHand, BOW_ID, false)));
    assert!(world.apply_set_entity_data(use_off_hand(258)));
    let drawing_bow = state(&world, 258);
    assert!(drawing_bow.kind.player().drawing_bow);
    assert!(drawing_bow.use_item_off_hand);
    assert!(!drawing_bow.kind.player().off_hand_item_pose);

    world.apply_add_entity(protocol_add_entity(
        259,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [10.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(259, EquipmentSlot::OffHand, CROSSBOW_ID, false)));
    assert!(world.apply_set_entity_data(use_off_hand(259)));
    let charging_crossbow = state(&world, 259);
    assert!(charging_crossbow.kind.player().charging_crossbow);
    assert!(charging_crossbow.use_item_off_hand);
    assert!(!charging_crossbow.kind.player().crossbow_hold_off_hand);

    world.apply_add_entity(protocol_add_entity(
        260,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [11.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(260, EquipmentSlot::OffHand, CROSSBOW_ID, true)));
    let off_hand_hold = state(&world, 260);
    assert!(!off_hand_hold.kind.player().crossbow_hold);
    assert!(off_hand_hold.kind.player().crossbow_hold_off_hand);
    assert!(!off_hand_hold.kind.player().off_hand_item_pose);

    world.apply_add_entity(protocol_add_entity(
        261,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [12.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(261, EquipmentSlot::MainHand, CROSSBOW_ID, true)));
    assert!(world.apply_set_equipment(equip(261, EquipmentSlot::OffHand, CROSSBOW_ID, true)));
    let main_hand_hold = state(&world, 261);
    assert!(main_hand_hold.kind.player().crossbow_hold);
    assert!(!main_hand_hold.kind.player().crossbow_hold_off_hand);

    world.apply_add_entity(protocol_add_entity(
        262,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [13.0, 64.0, -10.0],
    ));
    assert!(world.apply_set_equipment(equip(262, EquipmentSlot::MainHand, CROSSBOW_ID, true)));
    assert!(world.apply_set_equipment(equip(262, EquipmentSlot::OffHand, BOW_ID, false)));
    assert!(world.apply_set_entity_data(use_off_hand(262)));
    let main_hold_over_off_bow = state(&world, 262);
    assert!(main_hold_over_off_bow.kind.player().crossbow_hold);
    assert!(
        !main_hold_over_off_bow.kind.player().drawing_bow,
        "main-hand CROSSBOW_HOLD forces the using off-hand bow pose down to ITEM"
    );
    assert!(main_hold_over_off_bow.kind.player().off_hand_item_pose);
}

#[test]
fn entity_model_instances_project_player_main_hand_item_pose() {
    // Vanilla `AvatarRenderer.getArmPose` fallback `ITEM`: a player holding a plain main-hand item, not
    // using it, lowers/halves the arm. Gated to the player kind, a non-empty main hand, and not using an
    // item; an empty hand, a using-item player, or a non-player mob never reaches the `ITEM` fallback.
    // Any item id resolves the same way without a runtime — only "main hand non-empty" drives the gate.
    const PLAIN_ITEM_ID: i32 = 710;

    let plain_main_hand = |entity_id: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
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
            .main_hand_item_pose
    };

    let mut world = WorldStore::new();
    // A player holding a plain item in the main hand reaches the ITEM fallback pose.
    world.apply_add_entity(protocol_add_entity(
        260,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [1.0, 64.0, 8.0],
    ));
    assert!(world.apply_set_equipment(plain_main_hand(260)));
    assert!(posing(&world, 260));

    // The same player with an empty main hand has no item to pose.
    world.apply_add_entity(protocol_add_entity(
        261,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [2.0, 64.0, 8.0],
    ));
    assert!(!posing(&world, 261));

    // A player USING a NON-special main-hand item (here a plain item — `EAT`/`DRINK` or any tool) still
    // falls through to the `ITEM` fallback (vanilla `getArmPose` only special-cases bow/crossbow/trident/
    // shield/spyglass/horn/brush; everything else -> `ITEM`). A SPECIAL using item would suppress it, but
    // that needs the item runtime to resolve, so the no-runtime case treats any using item as non-special.
    world.apply_add_entity(protocol_add_entity(
        262,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        [3.0, 64.0, 8.0],
    ));
    assert!(world.apply_set_equipment(plain_main_hand(262)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 262,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            true,
            false,
            false,
        )],
    }));
    assert!(posing(&world, 262));

    // A non-player mob holding an item never returns ITEM (`HumanoidMobRenderer.getArmPose`).
    world.apply_add_entity(protocol_add_entity(
        263,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [4.0, 64.0, 8.0],
    ));
    assert!(world.apply_set_equipment(plain_main_hand(263)));
    assert!(!posing(&world, 263));
}
