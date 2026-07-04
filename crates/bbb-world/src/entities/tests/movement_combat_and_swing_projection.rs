use super::*;

#[test]
fn wolf_wet_shade_follows_vanilla_get_wet_shade_timer() {
    // Vanilla `Wolf.tick` marks the wolf wet while `isInWaterOrRain()`, then `getWetShade(partialTick)`
    // returns `0.75 + lerp(shakeAnimO, shakeAnim) * 0.125`, clamped to `1.0`, while wet. The client-side
    // drying timer advances `shakeAnim += 0.05` after the wolf leaves water and reaches white at 40 dry
    // ticks; f32 accumulation clears the wet state once the previous value compares `>= 2.0`.
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
    const AIR_BLOCK_STATE_ID: i32 = 0;

    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    store.apply_add_entity(ProtocolAddEntity {
        id: 82,
        uuid: default_entity_uuid(),
        entity_type_id: VANILLA_ENTITY_TYPE_WOLF_ID,
        position: ProtocolVec3d {
            x: 8.5,
            y: 2.0,
            z: 8.5,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 99,
    });
    assert!(
        store.apply_entity_position_sync(ProtocolEntityPositionSync {
            id: 82,
            position: ProtocolVec3d {
                x: 8.5,
                y: 2.0,
                z: 8.5,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: true,
        })
    );

    let fill = |store: &mut WorldStore, block_state_id: i32| {
        for y in 1..=3 {
            assert!(store.apply_block_update(ProtocolBlockUpdate {
                pos: ProtocolBlockPos { x: 8, y, z: 8 },
                block_state_id,
            }));
        }
    };
    let wolf_source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 82)
            .unwrap()
    };
    let shade = |store: &WorldStore, partial: f32| wolf_source(store, partial).wolf_wet_shade;
    let shake = |store: &WorldStore, partial: f32| wolf_source(store, partial).wolf_shake_anim;

    assert_eq!(
        shake(&store, 1.0),
        0.0,
        "an unticked dry wolf has no shakeAnim"
    );

    assert_eq!(
        shade(&store, 1.0),
        1.0,
        "an unticked dry wolf is not tinted"
    );

    fill(&mut store, SOURCE_WATER_BLOCK_STATE_ID);
    store.advance_entity_client_animations(1);
    assert!(
        (shade(&store, 1.0) - 0.75).abs() < 1.0e-6,
        "a wet wolf starts at the vanilla 0.75 shade floor: {}",
        shade(&store, 1.0)
    );
    assert_eq!(
        shake(&store, 1.0),
        0.0,
        "a wolf still in water has not started the drying shake"
    );

    fill(&mut store, AIR_BLOCK_STATE_ID);
    store.advance_entity_client_animations(1);
    assert_eq!(
        shake(&store, 0.0),
        0.0,
        "partial 0 reads shakeAnimO before the first dry tick increment"
    );
    assert!(
        (shake(&store, 1.0) - 0.05).abs() < 1.0e-6,
        "partial 1 reads shakeAnim after one 0.05 increment: {}",
        shake(&store, 1.0)
    );
    assert!(
        (shade(&store, 0.0) - 0.75).abs() < 1.0e-6,
        "partial 0 reads shakeAnimO before the first dry tick increment: {}",
        shade(&store, 0.0)
    );
    let first_dry_tick = 0.75 + 0.05 * 0.125;
    assert!(
        (shade(&store, 1.0) - first_dry_tick).abs() < 1.0e-6,
        "partial 1 reads shakeAnim after one 0.05 increment: {}",
        shade(&store, 1.0)
    );

    store.advance_entity_client_animations(39);
    assert!(
        (shake(&store, 1.0) - 2.0).abs() < 1.0e-6,
        "forty dry ticks reach vanilla's shakeAnim cap before clearing: {}",
        shake(&store, 1.0)
    );
    assert!(
        (shade(&store, 1.0) - 1.0).abs() < 1.0e-6,
        "forty dry ticks reach white before the wet state is dropped"
    );
    store.advance_entity_client_animations(2);
    assert_eq!(
        shade(&store, 1.0),
        1.0,
        "the cleared dry state keeps the default white shade"
    );
    assert_eq!(
        shake(&store, 1.0),
        0.0,
        "the cleared dry state resets shakeAnim"
    );
}

#[test]
fn wolf_head_roll_follows_vanilla_interested_angle_ease() {
    // Vanilla `Wolf.tick`: `interestedAngleO = interestedAngle`, then the synced
    // `DATA_INTERESTED_ID` target eases the current angle by 0.4/tick. `getHeadRollAngle`
    // lerps those two endpoints and scales the result by `0.15π`.
    const WOLF_INTERESTED_DATA_ID: u8 = 20;
    const WOLF_HEAD_ROLL_SCALE: f32 = 0.15 * std::f32::consts::PI;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        183,
        VANILLA_ENTITY_TYPE_WOLF_ID,
    ));

    let head_roll = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 183)
            .unwrap()
            .wolf_head_roll_angle
    };
    let assert_close = |actual: f32, expected: f32, label: &str| {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "{label}: expected {expected}, got {actual}"
        );
    };

    assert_eq!(head_roll(&store, 1.0), 0.0);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 183,
        values: vec![protocol_bool_data(WOLF_INTERESTED_DATA_ID, true)],
    }));
    store.advance_entity_client_animations(1);
    assert_close(
        head_roll(&store, 0.5),
        0.2 * WOLF_HEAD_ROLL_SCALE,
        "first interested tick lerps 0.0 -> 0.4 at partial 0.5",
    );

    store.advance_entity_client_animations(1);
    assert_close(
        head_roll(&store, 1.0),
        0.64 * WOLF_HEAD_ROLL_SCALE,
        "second interested tick eases 0.4 -> 0.64",
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 183,
        values: vec![protocol_bool_data(WOLF_INTERESTED_DATA_ID, false)],
    }));
    store.advance_entity_client_animations(1);
    assert_close(
        head_roll(&store, 0.5),
        0.512 * WOLF_HEAD_ROLL_SCALE,
        "interest clearing lerps 0.64 -> 0.384 at partial 0.5",
    );
}

#[test]
fn living_swim_amount_follows_vanilla_pose_swimming_ease_for_drowned() {
    // Vanilla `LivingEntity.updateSwimAmount`: save `swimAmountO`, then if
    // `isVisuallySwimming()` (`Pose.SWIMMING`) add `0.09` up to `1.0`, else subtract
    // `0.09` down to `0.0`. `getSwimAmount(partialTick)` lerps the two endpoints.
    const ENTITY_DATA_POSE_ID: u8 = 6;
    const POSE_STANDING: i32 = 0;
    const POSE_SWIMMING: i32 = 3;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        184,
        VANILLA_ENTITY_TYPE_DROWNED_ID,
    ));

    let swim_amount = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 184)
            .unwrap()
            .swim_amount
    };

    assert_eq!(swim_amount(&store, 1.0), 0.0);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 184,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_SWIMMING)],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        swim_amount(&store, 0.0),
        0.0,
        "partial 0 reads swimAmountO before the first swimming tick"
    );
    assert!(
        (swim_amount(&store, 1.0) - 0.09).abs() < 1.0e-6,
        "partial 1 reads swimAmount after one +0.09 tick: {}",
        swim_amount(&store, 1.0)
    );

    store.advance_entity_client_animations(11);
    assert_eq!(
        swim_amount(&store, 1.0),
        1.0,
        "swimAmount clamps at vanilla's fully-swimming value"
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 184,
        values: vec![protocol_pose_data(ENTITY_DATA_POSE_ID, POSE_STANDING)],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(
        swim_amount(&store, 0.0),
        1.0,
        "partial 0 reads the previous fully-swimming value while easing out"
    );
    assert!(
        (swim_amount(&store, 1.0) - 0.91).abs() < 1.0e-6,
        "partial 1 subtracts the vanilla 0.09 step: {}",
        swim_amount(&store, 1.0)
    );

    store.advance_entity_client_animations(12);
    assert_eq!(
        swim_amount(&store, 1.0),
        0.0,
        "after enough dry ticks the swim amount returns to rest"
    );
}

#[test]
fn entity_model_sources_project_on_ground_from_movement() {
    // Vanilla `Entity.onGround()`: the scene projects the entity's last synced movement ground
    // flag (combined with `isInWater` to drive the `TurtleRenderer` walk/swim branch). It
    // defaults to `false` until a movement packet sets it.

    let on_ground = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == 60)
            .unwrap()
            .on_ground
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
    ));
    assert!(
        !on_ground(&store),
        "a freshly spawned entity defaults to not on ground"
    );

    assert!(store.apply_entity_move(ProtocolEntityMove {
        id: 60,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: true,
    }));
    assert!(
        on_ground(&store),
        "a grounded movement packet projects on_ground"
    );
}

#[test]
fn entity_model_sources_project_is_moving_from_velocity() {
    // Vanilla `DolphinRenderState.isMoving` (`getDeltaMovement().horizontalDistanceSqr() > 1e-7`):
    // the scene projects the entity's synced velocity into the swim-animation gate.

    let is_moving = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == 61)
            .unwrap()
            .is_moving
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    ));
    assert!(
        !is_moving(&store),
        "a freshly spawned entity defaults to not moving"
    );

    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 61,
        delta_movement: ProtocolVec3d {
            x: 0.1,
            y: 0.5,
            z: -0.1,
        },
    }));
    assert!(
        is_moving(&store),
        "a horizontal velocity above 1e-7 projects is_moving"
    );

    // A purely vertical velocity (`horizontalDistanceSqr == 0`) is not moving.
    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 61,
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.5,
            z: 0.0,
        },
    }));
    assert!(
        !is_moving(&store),
        "a purely vertical velocity is not horizontally moving"
    );
}

#[test]
fn entity_model_sources_project_hurt_overlay_for_ten_ticks() {
    let red_overlay = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == 40)
            .unwrap()
            .has_red_overlay
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        40,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(!red_overlay(&store));

    // Vanilla animateHurt sets hurtTime = hurtDuration = 10, so hasRedOverlay
    // stays true through the next 9 client ticks and clears on the 10th.
    assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 40, yaw: 0.0 }));
    assert!(red_overlay(&store));
    store.advance_entity_client_animations(9);
    assert!(red_overlay(&store));
    store.advance_entity_client_animations(1);
    assert!(!red_overlay(&store));

    // A damage event re-triggers the same hurtTime countdown.
    assert!(store.apply_damage_event(ProtocolDamageEvent {
        entity_id: 40,
        source_type_id: 0,
        source_cause_id: 0,
        source_direct_id: 0,
        source_position: None,
    }));
    assert!(red_overlay(&store));
}

#[test]
fn entity_model_sources_project_kinetic_hit_feedback_from_living_entity_event() {
    const KINETIC_HIT_EVENT_ID: i8 = 2;

    let feedback = |store: &WorldStore, entity_id: i32, partial_tick: f32| {
        store
            .entity_model_sources_at_partial_tick(partial_tick)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .expect("entity source")
            .ticks_since_kinetic_hit_feedback
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        41,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        42,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));

    assert_eq!(feedback(&store, 41, 0.5), 0.0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 41,
        event_id: KINETIC_HIT_EVENT_ID,
    }));
    // Vanilla `LivingEntity.handleEntityEvent(2)` calls `onKineticHit`, and
    // `getTicksSinceLastKineticHitFeedback(partialTicks)` reads gameTime delta + partial.
    assert_eq!(feedback(&store, 41, 0.25), 0.25);
    store.advance_entity_client_animations(3);
    assert_eq!(feedback(&store, 41, 0.5), 3.5);

    store.advance_entity_client_animations(7);
    assert_eq!(feedback(&store, 41, 0.0), 10.0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 41,
        event_id: KINETIC_HIT_EVENT_ID,
    }));
    assert_eq!(
        feedback(&store, 41, 0.0),
        10.0,
        "vanilla ignores repeated kinetic feedback until elapsed > 10 ticks"
    );
    store.advance_entity_client_animations(1);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 41,
        event_id: KINETIC_HIT_EVENT_ID,
    }));
    assert_eq!(feedback(&store, 41, 0.0), 0.0);

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 42,
        event_id: KINETIC_HIT_EVENT_ID,
    }));
    assert_eq!(
        feedback(&store, 42, 0.5),
        0.0,
        "event 2 is LivingEntity-specific"
    );
}

#[test]
fn entity_model_sources_project_attack_swing_ramp() {
    let attack = |store: &WorldStore, partial: f32| {
        let source = store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 50)
            .unwrap();
        (source.attack_anim, source.attack_arm_off_hand)
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert_eq!(attack(&store, 1.0), (0.0, false));

    // Vanilla `ClientboundAnimate` action 0 = swing main hand → `LivingEntity.swing` arms the
    // 6-tick ramp. `updateSwingTime` then ramps `attackAnim` 0, 1/6, .. 5/6 over ticks 1..6 (the
    // current-tick value is read at partialTick 1.0; partialTick 0.0 yields the previous tick's).
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 50, action: 0 }));
    store.advance_entity_client_animations(1); // tick 1: swingTime 0 → 0
    assert_eq!(attack(&store, 1.0).0, 0.0);
    store.advance_entity_client_animations(1); // tick 2: swingTime 1 → 1/6
    assert!((attack(&store, 1.0).0 - 1.0 / 6.0).abs() < 1e-6);

    store.advance_entity_client_animations(4); // through tick 6: swingTime 5 → 5/6 (prev 4/6)
    assert!((attack(&store, 1.0).0 - 5.0 / 6.0).abs() < 1e-6);
    assert!((attack(&store, 0.0).0 - 4.0 / 6.0).abs() < 1e-6);
    // The partial tick lerps between the previous and current attackAnim (vanilla getAttackAnim).
    assert!((attack(&store, 0.5).0 - 0.75).abs() < 1e-6);

    store.advance_entity_client_animations(1); // tick 7: swingTime hits 6 → reset, swinging stops
    assert_eq!(attack(&store, 1.0).0, 0.0);
    store.advance_entity_client_animations(1); // tick 8: the decayed swing state is dropped
    assert_eq!(attack(&store, 1.0), (0.0, false));

    // Action 3 = off-hand swing → the off (left) arm is flagged.
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 50, action: 3 }));
    store.advance_entity_client_animations(2);
    assert!(attack(&store, 1.0).1, "off-hand swing flags the left arm");
}

#[test]
fn local_player_attack_swing_samples_local_entity() {
    let mut store = WorldStore::new();
    assert_eq!(store.local_player_attack_swing(1.0), None);

    store.apply_login(&protocol_play_login(61));
    assert_eq!(
        store.local_player_attack_swing(1.0),
        None,
        "login alone does not fabricate an entity animation component"
    );
    store.apply_add_entity(protocol_add_entity_with_type(
        61,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert_eq!(
        store.local_player_attack_swing(1.0),
        Some(LocalPlayerAttackSwingState {
            attack_anim: 0.0,
            off_hand: false,
        })
    );

    // Vanilla `renderHandsWithItems` samples `LocalPlayer.getAttackAnim(partialTick)` and
    // `swingingArm`; action 3 selects the off hand while the usual 6-tick WHACK ramp lerps by partial.
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 61, action: 3 }));
    store.advance_entity_client_animations(2);
    let swing = store.local_player_attack_swing(0.5).unwrap();
    assert!(swing.off_hand);
    assert!((swing.attack_anim - 1.0 / 12.0).abs() < 1e-6);
}

#[test]
fn entity_model_sources_use_held_item_default_swing_duration() {
    const SPEAR_ITEM_ID: i32 = 42;

    let attack = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 51)
            .unwrap()
            .attack_anim
    };

    let mut store = WorldStore::new();
    store.set_default_item_swing_animation_durations(BTreeMap::from([(SPEAR_ITEM_ID, 13)]));
    store.apply_add_entity(protocol_add_entity_with_type(
        51,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_equipment(ProtocolSetEquipment {
        entity_id: 51,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id: Some(SPEAR_ITEM_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            },
        }],
    }));

    // Vanilla `Item.Properties.spear(... attackDuration = 0.65F ...)` installs
    // `SwingAnimation(STAB, (int)(0.65 * 20))`, so a wooden spear swing is still
    // active after the default 6-tick WHACK would already have reset.
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 51, action: 0 }));
    store.advance_entity_client_animations(7);
    assert!((attack(&store, 1.0) - 6.0 / 13.0).abs() < 1e-6);
    assert!((attack(&store, 0.0) - 5.0 / 13.0).abs() < 1e-6);

    store.advance_entity_client_animations(6);
    assert!((attack(&store, 1.0) - 12.0 / 13.0).abs() < 1e-6);
    store.advance_entity_client_animations(1);
    assert_eq!(attack(&store, 1.0), 0.0);
}

#[test]
fn entity_model_sources_use_item_stack_swing_animation_patch_duration() {
    const SPEAR_ITEM_ID: i32 = 42;

    let attack = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 52)
            .unwrap()
            .attack_anim
    };
    let equip = |item: ItemStackSummary| ProtocolSetEquipment {
        entity_id: 52,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item,
        }],
    };

    let mut patched = ItemStackSummary {
        item_id: Some(SPEAR_ITEM_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    patched.component_patch.added = 1;
    patched.component_patch.added_type_ids = vec![40];
    patched.component_patch.swing_animation = Some(SwingAnimationSummary {
        animation_type: SwingAnimationTypeSummary::Stab,
        duration: 17,
    });

    let mut removed = ItemStackSummary {
        item_id: Some(SPEAR_ITEM_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    removed.component_patch.removed_type_ids = vec![40];

    let mut store = WorldStore::new();
    store.set_default_item_swing_animation_durations(BTreeMap::from([(SPEAR_ITEM_ID, 13)]));
    store.apply_add_entity(protocol_add_entity_with_type(
        52,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));

    // Vanilla `ItemStack.getSwingAnimation()` reads the stack component before the
    // item prototype default, so a server-synchronized patch duration wins.
    assert!(store.apply_set_equipment(equip(patched)));
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 52, action: 0 }));
    store.advance_entity_client_animations(7);
    assert!((attack(&store, 1.0) - 6.0 / 17.0).abs() < 1e-6);

    // Removing the component from a spear falls through to `SwingAnimation.DEFAULT`
    // (`WHACK`, 6 ticks), not the spear prototype's 13-tick STAB default.
    assert!(store.apply_set_equipment(equip(removed)));
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 52, action: 0 }));
    store.advance_entity_client_animations(6);
    assert!((attack(&store, 1.0) - 5.0 / 6.0).abs() < 1e-6);
    store.advance_entity_client_animations(1);
    assert_eq!(attack(&store, 1.0), 0.0);
}

#[test]
fn entity_model_sources_apply_mob_effect_swing_duration_modifiers() {
    const VANILLA_MOB_EFFECT_HASTE_ID: i32 = 2;
    const VANILLA_MOB_EFFECT_MINING_FATIGUE_ID: i32 = 3;
    const VANILLA_MOB_EFFECT_CONDUIT_POWER_ID: i32 = 28;
    const SPEAR_ITEM_ID: i32 = 42;

    let attack = |store: &WorldStore, entity_id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap()
            .attack_anim
    };
    let item = || ItemStackSummary {
        item_id: Some(SPEAR_ITEM_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let equip = |entity_id: i32| ProtocolSetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: item(),
        }],
    };
    let effect = |entity_id: i32, effect_id: i32, amplifier: i32| UpdateMobEffect {
        entity_id,
        effect_id,
        amplifier,
        duration_ticks: 200,
        flags: MobEffectFlags::default(),
    };

    let mut store = WorldStore::new();
    store.set_default_item_swing_animation_durations(BTreeMap::from([(SPEAR_ITEM_ID, 13)]));

    store.apply_add_entity(protocol_add_entity_with_type(
        53,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_equipment(equip(53)));
    assert!(store.apply_update_mob_effect(effect(53, VANILLA_MOB_EFFECT_HASTE_ID, 1)));
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 53, action: 0 }));
    store.advance_entity_client_animations(7);
    assert!((attack(&store, 53, 1.0) - 6.0 / 11.0).abs() < 1e-6);

    store.apply_add_entity(protocol_add_entity_with_type(
        54,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_equipment(equip(54)));
    assert!(store.apply_update_mob_effect(effect(54, VANILLA_MOB_EFFECT_MINING_FATIGUE_ID, 2,)));
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 54, action: 0 }));
    store.advance_entity_client_animations(7);
    assert!((attack(&store, 54, 1.0) - 6.0 / 19.0).abs() < 1e-6);

    store.apply_add_entity(protocol_add_entity_with_type(
        55,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_equipment(equip(55)));
    assert!(store.apply_update_mob_effect(effect(55, VANILLA_MOB_EFFECT_HASTE_ID, 0)));
    assert!(store.apply_update_mob_effect(effect(55, VANILLA_MOB_EFFECT_MINING_FATIGUE_ID, 5,)));
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 55, action: 0 }));
    store.advance_entity_client_animations(7);
    assert!((attack(&store, 55, 1.0) - 6.0 / 12.0).abs() < 1e-6);

    store.apply_add_entity(protocol_add_entity_with_type(
        56,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_equipment(equip(56)));
    assert!(store.apply_update_mob_effect(effect(56, VANILLA_MOB_EFFECT_HASTE_ID, 0)));
    assert!(store.apply_update_mob_effect(effect(56, VANILLA_MOB_EFFECT_CONDUIT_POWER_ID, 3,)));
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 56, action: 0 }));
    store.advance_entity_client_animations(7);
    assert!((attack(&store, 56, 1.0) - 6.0 / 9.0).abs() < 1e-6);
}

#[test]
fn entity_model_sources_refresh_in_flight_swing_duration_from_runtime_state_changes() {
    const PLAIN_ITEM_ID: i32 = 41;
    const SPEAR_ITEM_ID: i32 = 42;
    const VANILLA_MOB_EFFECT_HASTE_ID: i32 = 2;

    let attack = |store: &WorldStore, entity_id: i32, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .unwrap()
            .attack_anim
    };
    let item = |item_id| ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    let equip = |entity_id: i32, item_id: i32| ProtocolSetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: item(item_id),
        }],
    };
    let effect = |entity_id: i32, amplifier: i32| UpdateMobEffect {
        entity_id,
        effect_id: VANILLA_MOB_EFFECT_HASTE_ID,
        amplifier,
        duration_ticks: 200,
        flags: MobEffectFlags::default(),
    };

    let mut store = WorldStore::new();
    store.set_default_item_swing_animation_durations(BTreeMap::from([(SPEAR_ITEM_ID, 13)]));

    // `LivingEntity.updateSwingTime` re-reads `getCurrentSwingDuration()` every tick,
    // so swapping to a longer current hand item keeps the in-flight swing alive.
    store.apply_add_entity(protocol_add_entity_with_type(
        57,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_equipment(equip(57, PLAIN_ITEM_ID)));
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 57, action: 0 }));
    store.advance_entity_client_animations(3);
    assert!(store.apply_set_equipment(equip(57, SPEAR_ITEM_ID)));
    store.advance_entity_client_animations(4);
    assert!((attack(&store, 57, 1.0) - 6.0 / 13.0).abs() < 1e-6);

    // Swapping back to the default 6-tick item shortens the same swing and reaches reset.
    store.apply_add_entity(protocol_add_entity_with_type(
        58,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_equipment(equip(58, SPEAR_ITEM_ID)));
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 58, action: 0 }));
    store.advance_entity_client_animations(3);
    assert!(store.apply_set_equipment(equip(58, PLAIN_ITEM_ID)));
    store.advance_entity_client_animations(4);
    assert_eq!(attack(&store, 58, 1.0), 0.0);

    // Runtime effect updates and removals also affect the current duration source.
    store.apply_add_entity(protocol_add_entity_with_type(
        59,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_equipment(equip(59, SPEAR_ITEM_ID)));
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 59, action: 0 }));
    store.advance_entity_client_animations(3);
    assert!(store.apply_update_mob_effect(effect(59, 1)));
    store.advance_entity_client_animations(4);
    assert!((attack(&store, 59, 1.0) - 6.0 / 11.0).abs() < 1e-6);

    store.apply_add_entity(protocol_add_entity_with_type(
        60,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_equipment(equip(60, SPEAR_ITEM_ID)));
    assert!(store.apply_update_mob_effect(effect(60, 1)));
    assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 60, action: 0 }));
    store.advance_entity_client_animations(3);
    assert!(store.apply_remove_mob_effect(ProtocolRemoveMobEffect {
        entity_id: 60,
        effect_id: VANILLA_MOB_EFFECT_HASTE_ID,
    }));
    store.advance_entity_client_animations(4);
    assert!((attack(&store, 60, 1.0) - 6.0 / 13.0).abs() < 1e-6);
}
