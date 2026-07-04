use super::*;

#[test]
fn entity_model_sources_project_creeper_swelling_fuse() {
    const CREEPER_SWELL_DIR_DATA_ID: u8 = 16;

    // Read at partial tick 1.0 so getSwelling returns the current swell.
    let swelling = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 50)
            .unwrap()
            .creeper_swelling
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_CREEPER_ID,
    ));
    // Default swell direction is -1 (resting): the fuse stays at zero.
    assert_eq!(swelling(&store), 0.0);
    store.advance_entity_client_animations(5);
    assert_eq!(swelling(&store), 0.0);

    // A positive swell direction advances the fuse one step per client tick;
    // getSwelling divides the lerped swell by maxSwell - 2 = 28.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: CREEPER_SWELL_DIR_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(1),
        }],
    }));
    store.advance_entity_client_animations(3);
    assert_eq!(swelling(&store), 3.0 / 28.0);

    // Flipping the direction back to -1 drains the fuse toward zero again.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: CREEPER_SWELL_DIR_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(-1),
        }],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(swelling(&store), 2.0 / 28.0);
}

#[test]
fn entity_model_sources_project_squid_tentacle_and_body_animation() {
    const SQUID_RESET_MOVEMENT_EVENT_ID: i8 = 19;
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;

    let source = |store: &WorldStore, partial: f32| {
        store
            .entity_model_sources_at_partial_tick(partial)
            .into_iter()
            .find(|source| source.entity_id == 70)
            .unwrap()
    };

    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    store.apply_add_entity(ProtocolAddEntity {
        id: 70,
        uuid: default_entity_uuid(),
        entity_type_id: VANILLA_ENTITY_TYPE_SQUID_ID,
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
    for y in 1..=3 {
        assert!(store.apply_block_update(ProtocolBlockUpdate {
            pos: ProtocolBlockPos { x: 8, y, z: 8 },
            block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
        }));
    }
    // Give the squid a horizontal+downward velocity so the body pitch turns away
    // from zero (vanilla `xBodyRot` is driven by `atan2(horizontal, dm.y)`).
    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 70,
        delta_movement: ProtocolVec3d {
            x: 0.2,
            y: -0.1,
            z: 0.0,
        },
    }));

    // A floating squid that has never been ticked is frozen at the bind pose.
    let resting = source(&store, 1.0);
    assert_eq!(resting.squid_tentacle_angle, 0.0);
    assert_eq!(resting.squid_x_body_rot, 0.0);
    assert_eq!(resting.squid_y_body_rot, 0.0);
    assert_eq!(resting.squid_z_body_rot, 0.0);

    // A few ticks in (still early in the half-cycle, `scale < 0.75`) the tentacle
    // flex is already off the bind pose, but the body roll has not yet engaged:
    // vanilla only sets `rotateSpeed = 1` once `scale > 0.75`.
    store.advance_entity_client_animations(5);
    let after_five = source(&store, 1.0);
    assert!(
        after_five.squid_tentacle_angle > 0.0,
        "the tentacle angle leaves the bind pose: {}",
        after_five.squid_tentacle_angle
    );
    assert!(
        after_five.squid_x_body_rot < 0.0,
        "a diving squid pitches its body negative: {}",
        after_five.squid_x_body_rot
    );
    let expected_y_body_rot = -90.0 * (1.0 - 0.9_f32.powi(5));
    assert!(
        (after_five.squid_y_body_rot - expected_y_body_rot).abs() < 1.0e-5,
        "in water, Squid.aiStep eases yBodyRot toward -atan2(dm.x, dm.z): expected {expected_y_body_rot}, got {}",
        after_five.squid_y_body_rot
    );

    // Advance deep into the half-cycle so `scale > 0.75` engages `rotateSpeed = 1`,
    // after which the body roll accumulates each tick (`zBodyRot += π·rotateSpeed·1.5`).
    store.advance_entity_client_animations(18);
    let after_roll = source(&store, 1.0);
    assert!(
        after_roll.squid_z_body_rot > 0.0,
        "the body roll accumulates once the half-cycle passes 0.75: {}",
        after_roll.squid_z_body_rot
    );

    store.advance_entity_client_animations(1);
    let after_more = source(&store, 1.0);
    assert!(
        after_more.squid_z_body_rot > after_roll.squid_z_body_rot,
        "the body roll keeps advancing across ticks"
    );

    // The lerped getters track the partial tick between the old and current
    // endpoints: at partial 0.0 the projection equals last tick's value (the
    // half-way point of the lerp at 0.5 sits strictly between the two endpoints).
    let at_zero = source(&store, 0.0).squid_z_body_rot;
    let at_half = source(&store, 0.5).squid_z_body_rot;
    let at_one = source(&store, 1.0).squid_z_body_rot;
    assert!(
        at_zero < at_half && at_half < at_one,
        "partial tick lerps the roll: {at_zero} < {at_half} < {at_one}"
    );
    assert!(
        (at_half - (at_zero + (at_one - at_zero) * 0.5)).abs() < 1.0e-4,
        "the projection is a linear lerp between the endpoints"
    );
    let yaw_at_zero = source(&store, 0.0).squid_y_body_rot;
    let yaw_at_half = source(&store, 0.5).squid_y_body_rot;
    let yaw_at_one = source(&store, 1.0).squid_y_body_rot;
    assert!(
        yaw_at_zero > yaw_at_half && yaw_at_half > yaw_at_one,
        "partial tick rot-lerps the movement-derived body yaw: {yaw_at_zero} > {yaw_at_half} > {yaw_at_one}"
    );
    assert!(
        (yaw_at_half - (yaw_at_zero + (yaw_at_one - yaw_at_zero) * 0.5)).abs() < 1.0e-4,
        "the body yaw projection follows vanilla Mth.rotLerp for this non-wrapping case"
    );

    // Entity event 19 (`Squid.handleEntityEvent`) resets `tentacleMovement` to 0.
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 70,
        event_id: SQUID_RESET_MOVEMENT_EVENT_ID,
    }));
    // After the reset, the next tick restarts the half-cycle from near zero, so the
    // tentacle angle is small (`sin(scale²·π)·π·0.25` with `scale` just above 0).
    store.advance_entity_client_animations(1);
    let after_reset = source(&store, 1.0);
    assert!(
        after_reset.squid_tentacle_angle < after_five.squid_tentacle_angle,
        "the event-19 reset rewinds the tentacle cycle"
    );
}

#[test]
fn squid_out_of_water_branch_flexes_tentacles_and_pitches_down() {
    let source = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 70)
            .unwrap()
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_SQUID_ID,
    ));

    store.advance_entity_client_animations(1);
    let after_one = source(&store);
    let tentacle_speed = super::animations::SquidAnimationState::new(70).tentacle_speed;
    let expected_tentacle_angle = tentacle_speed.sin().abs() * std::f32::consts::PI * 0.25;

    assert!(
        (after_one.squid_tentacle_angle - expected_tentacle_angle).abs() < 1.0e-6,
        "out of water uses abs(sin(tentacleMovement)) * pi * 0.25"
    );
    assert!(
        (after_one.squid_x_body_rot - -1.8).abs() < 1.0e-6,
        "out of water eases xBodyRot toward -90 by 0.02"
    );
    assert_eq!(
        after_one.squid_z_body_rot, 0.0,
        "out of water leaves the swim roll untouched"
    );
    assert_eq!(
        after_one.squid_y_body_rot, 30.0,
        "out of water leaves the yBodyRot seeded from the add-entity head yaw untouched"
    );
}

#[test]
fn squid_tentacle_speed_is_seeded_by_entity_id() {
    // The per-tick tentacle advance equals `tentacleSpeed`, so after one tick the
    // tentacle movement (read indirectly via the angle the half-cycle produces) is
    // a deterministic function of the id-seeded speed. Two squids with different
    // ids advance at different rates, while a glow squid is seeded the same way.
    let tentacle_angle_after_one_tick = |id: i32, entity_type_id: i32| {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
        store.advance_entity_client_animations(1);
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .squid_tentacle_angle
    };

    let squid_a = tentacle_angle_after_one_tick(7, VANILLA_ENTITY_TYPE_SQUID_ID);
    let squid_b = tentacle_angle_after_one_tick(1000, VANILLA_ENTITY_TYPE_SQUID_ID);
    assert!(squid_a > 0.0 && squid_b > 0.0);
    assert!(
        (squid_a - squid_b).abs() > 1.0e-6,
        "different ids seed different tentacle speeds: {squid_a} vs {squid_b}"
    );

    // A glow squid uses the same id-seeded animation (vanilla `GlowSquid extends Squid`).
    let glow = tentacle_angle_after_one_tick(7, VANILLA_ENTITY_TYPE_GLOW_SQUID_ID);
    assert!(
        (glow - squid_a).abs() < 1.0e-6,
        "the glow squid shares the squid animation seeding for the same id"
    );
}

#[test]
fn guardian_tail_animation_speed_branches_match_vanilla_ai_step() {
    // Vanilla `Guardian.aiStep` ramps `clientSideTailAnimationSpeed` differently per
    // tick depending on `isInWater()` and the synced `isMoving()` (`DATA_ID_MOVING`),
    // then integrates `clientSideTailAnimation += speed`. The projected
    // `guardian_tail_animation` advances by that per-tick speed, so its one-tick delta
    // pins which branch ran:
    //   - out of water  → speed = 2.0   (the frantic flop)
    //   - in water, moving, from rest (speed < 0.5) → speed snaps to 4.0
    //   - in water, idle → speed eases toward 0.125 (≈ 0.025 from rest, by 0.2)
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;

    let tail = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 80)
            .unwrap()
            .guardian_tail_animation
    };

    // A guardian standing in a tall water column (submerged) and flagged moving.
    let make_store = |moving: bool, in_water: bool| {
        let mut store = WorldStore::with_dimension(crate::WorldDimension {
            min_y: 0,
            height: 16,
        });
        store.insert_decoded_chunk(empty_test_chunk());
        store.apply_add_entity(ProtocolAddEntity {
            id: 80,
            uuid: default_entity_uuid(),
            entity_type_id: VANILLA_ENTITY_TYPE_GUARDIAN_ID,
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
        if in_water {
            // Fill the column the guardian's AABB occupies so `world_aabb_in_water`
            // sees a submerged box.
            for y in 1..=4 {
                assert!(store.apply_block_update(ProtocolBlockUpdate {
                    pos: ProtocolBlockPos { x: 8, y, z: 8 },
                    block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
                }));
            }
        }
        if moving {
            assert!(store.apply_set_entity_data(ProtocolSetEntityData {
                id: 80,
                values: vec![protocol_bool_data(16, true)],
            }));
        }
        store
    };

    // In water + moving: from the rest speed (`0`, which is `< 0.5`) the first tick
    // snaps the speed to `4.0`, so the tail jumps by 4 per tick.
    let mut wet_moving = make_store(true, true);
    wet_moving.advance_entity_client_animations(1);
    let after_one = tail(&wet_moving);
    assert!(
        (after_one - 4.0).abs() < 1.0e-4,
        "in-water moving guardian snaps its tail speed to 4.0 from rest: {after_one}"
    );

    // In water + idle: the speed eases toward `0.125` by `0.2` (`0 + (0.125 - 0)*0.2 =
    // 0.025`), a slow hover wave — far slower than either other branch.
    let mut wet_idle = make_store(false, true);
    wet_idle.advance_entity_client_animations(1);
    let idle_one = tail(&wet_idle);
    assert!(
        (idle_one - 0.025).abs() < 1.0e-4,
        "in-water idle guardian eases its tail speed toward 0.125 (0.025 from rest): {idle_one}"
    );

    // Out of water: the speed is forced to `2.0` regardless of the moving flag.
    let mut dry = make_store(true, false);
    dry.advance_entity_client_animations(1);
    let dry_one = tail(&dry);
    assert!(
        (dry_one - 2.0).abs() < 1.0e-4,
        "an out-of-water guardian flops its tail at speed 2.0: {dry_one}"
    );

    // The three branches advance the tail at distinctly different rates.
    assert!(after_one > dry_one && dry_one > idle_one);
}

#[test]
fn guardian_spikes_withdrawal_branches_match_vanilla_ai_step() {
    // Vanilla `Guardian.aiStep` eases `clientSideSpikesAnimation` (spawn `0`): in water toward `1`
    // while idle (by `0.06`, spikes extend) or toward `0` while moving (by `0.25`, spikes retract);
    // out of water it randomizes — deferred, so the value is HELD. `GuardianRenderState.spikesAnimation`
    // lerps it, and `setupAnim` turns it into `withdrawal = (1 - it) · 0.55`.
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;
    const AIR_BLOCK_STATE_ID: i32 = 0;

    let spikes = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 80)
            .unwrap()
            .guardian_spikes_animation
    };

    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    store.apply_add_entity(ProtocolAddEntity {
        id: 80,
        uuid: default_entity_uuid(),
        entity_type_id: VANILLA_ENTITY_TYPE_GUARDIAN_ID,
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
    let fill = |store: &mut WorldStore, block_state_id: i32| {
        for y in 1..=4 {
            assert!(store.apply_block_update(ProtocolBlockUpdate {
                pos: ProtocolBlockPos { x: 8, y, z: 8 },
                block_state_id,
            }));
        }
    };

    // An unticked guardian projects the fully-extended rest pose (withdrawal `0` ⇒ spikesAnimation 1).
    assert_eq!(spikes(&store), 1.0);

    // In water + idle: from the spawn `0` the spikes ease UP toward `1` by `0.06` — first tick `0.06`.
    fill(&mut store, SOURCE_WATER_BLOCK_STATE_ID);
    store.advance_entity_client_animations(1);
    assert!(
        (spikes(&store) - 0.06).abs() < 1.0e-5,
        "in-water idle eases the spikes toward 1 by 0.06: {}",
        spikes(&store)
    );
    // They keep climbing while idle.
    store.advance_entity_client_animations(9);
    let extended = spikes(&store);
    assert!(
        extended > 0.06 && extended < 1.0,
        "the idle spikes keep extending toward 1: {extended}"
    );

    // Flag the guardian moving (synced `DATA_ID_MOVING`, idx 16): in water the spikes now RETRACT,
    // easing toward `0` by `0.25` — one tick gives `0.75 · extended`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 80,
        values: vec![protocol_bool_data(16, true)],
    }));
    store.advance_entity_client_animations(1);
    let retracting = spikes(&store);
    assert!(
        (retracting - extended * 0.75).abs() < 1.0e-5,
        "in-water moving retracts the spikes toward 0 by 0.25: {retracting} vs {extended}"
    );

    // Out of water (drain the column): vanilla randomizes, which is deferred, so the value is HELD at
    // the last frame regardless of the still-set moving flag.
    fill(&mut store, AIR_BLOCK_STATE_ID);
    store.advance_entity_client_animations(1);
    assert!(
        (spikes(&store) - retracting).abs() < 1.0e-5,
        "out of water the spikes hold their last value (random flicker deferred): {} vs {retracting}",
        spikes(&store)
    );
}

#[test]
fn entity_model_sources_project_guardian_attack_beam() {
    // Vanilla `GuardianRenderer.extractRenderState`: a guardian whose synced `DATA_ID_ATTACK_TARGET`
    // (idx 17) names a live target projects the world eye→target vector and the ramping attack timing;
    // with no target it projects no beam.
    const GUARDIAN_ATTACK_TARGET_DATA_ID: u8 = 17;

    let add_at = |id: i32, type_id: i32, x: f64| ProtocolAddEntity {
        id,
        uuid: default_entity_uuid(),
        entity_type_id: type_id,
        position: ProtocolVec3d { x, y: 64.0, z: 0.0 },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    };

    let mut store = WorldStore::new();
    // Guardian at the origin; target zombie 10 blocks east (+X).
    store.apply_add_entity(add_at(70, VANILLA_ENTITY_TYPE_GUARDIAN_ID, 0.0));
    store.apply_add_entity(add_at(71, VANILLA_ENTITY_TYPE_ZOMBIE_ID, 10.0));

    let beam = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == 70)
            .unwrap()
            .guardian_beam
    };

    // No active attack target → no beam.
    assert!(beam(&store).is_none());

    // Lock onto the zombie (id 71) and ramp the client-side attack time over five ticks.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![protocol_int_data(GUARDIAN_ATTACK_TARGET_DATA_ID, 71)],
    }));
    store.advance_entity_client_animations(5);
    let projected = beam(&store).expect("a guardian locked onto a live target beams");

    // The beam points east (+X) toward the target and is level (no Z drift, small Y from eye/center).
    assert!(
        projected.eye_to_target[0] > 8.0,
        "beam points +X toward target: {:?}",
        projected.eye_to_target
    );
    assert!(projected.eye_to_target[2].abs() < 0.01);
    assert!(projected.eye_height > 0.0);
    // Five client ticks ramp `clientSideAttackTime` to 5; at partial 0, `attackTime = 5` and
    // `attackScale = 5 / 80` (the guardian's `getAttackDuration`).
    assert!((projected.attack_time - 5.0).abs() < 1.0e-4);
    assert!((projected.attack_scale - 5.0 / 80.0).abs() < 1.0e-4);

    // Clearing the target stops the beam (and resets the counter, vanilla `onSyncedDataUpdated`).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![protocol_int_data(GUARDIAN_ATTACK_TARGET_DATA_ID, 0)],
    }));
    assert!(beam(&store).is_none());
}

#[test]
fn entity_model_sources_project_end_crystal_beam_target() {
    // Vanilla `EndCrystal.DATA_BEAM_TARGET` is Optional<BlockPos> id 8. `EndCrystalRenderer`
    // projects `Vec3.atCenterOf(target) - entity.getPosition(partialTicks)` into
    // `EndCrystalRenderState.beamOffset`.
    const END_CRYSTAL_BEAM_TARGET_DATA_ID: u8 = 8;

    let add_at = |id: i32, type_id: i32| ProtocolAddEntity {
        id,
        uuid: default_entity_uuid(),
        entity_type_id: type_id,
        position: ProtocolVec3d {
            x: 10.0,
            y: 64.0,
            z: -3.0,
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    };
    let beam_target = |target: Option<ProtocolBlockPos>| ProtocolEntityDataValue {
        data_id: END_CRYSTAL_BEAM_TARGET_DATA_ID,
        serializer_id: 11,
        value: EntityDataValueKind::OptionalBlockPos(target),
    };
    let beam = |store: &WorldStore, id: i32| {
        store
            .entity_model_sources_at_partial_tick(0.0)
            .into_iter()
            .find(|source| source.entity_id == id)
            .unwrap()
            .end_crystal_beam
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(add_at(180, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID));
    store.apply_add_entity(add_at(181, VANILLA_ENTITY_TYPE_BAT_ID));
    assert!(beam(&store, 180).is_none());

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 180,
        values: vec![beam_target(Some(ProtocolBlockPos {
            x: 14,
            y: 67,
            z: -10,
        }))],
    }));
    assert_eq!(beam(&store, 180).unwrap().beam_offset, [4.5, 3.5, -6.5]);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 180,
        values: vec![beam_target(None)],
    }));
    assert!(beam(&store, 180).is_none());

    // The same data id on a non-crystal is ignored.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 181,
        values: vec![beam_target(Some(ProtocolBlockPos {
            x: 14,
            y: 67,
            z: -10,
        }))],
    }));
    assert!(beam(&store, 181).is_none());
}

#[test]
fn entity_model_sources_project_ender_dragon_nearest_crystal_beam() {
    // Vanilla `EnderDragon.checkCrystals` tracks the nearest end crystal intersecting
    // `getBoundingBox().inflate(32)`. `EnderDragonRenderer.extractRenderState` then writes
    // `beamOffset = crystal.getPosition(partialTicks) + getY(crystal.time + partialTicks)
    // - dragon.getPosition(partialTicks)`.

    let add_at = |id: i32, type_id: i32, position: [f64; 3]| ProtocolAddEntity {
        id,
        uuid: default_entity_uuid(),
        entity_type_id: type_id,
        position: ProtocolVec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        delta_movement: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    };
    let dragon_beam = |store: &WorldStore, id: i32, partial_ticks: f32| {
        let position = store.entities.transform(id).unwrap().position;
        store
            .entities
            .model_source(
                id,
                position,
                partial_ticks,
                &store.registries,
                &store.items.default_item_max_damage,
                &store.items.default_item_armor_materials,
                &store.items.default_item_equipment_slots,
                &store.items.default_llama_body_decor_colors,
                &store.items.default_nautilus_body_armor_materials,
                &store.items.default_horse_body_armor_materials,
                &store.items.default_wolf_body_armor_materials,
            )
            .unwrap()
            .ender_dragon_beam
    };
    let vanilla_crystal_y = |age: f32| {
        let hh = (age * 0.2).sin() / 2.0 + 0.5;
        (hh * hh + hh) * 0.4 - 1.4
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(add_at(
        190,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        [0.0, 64.0, 0.0],
    ));
    assert!(dragon_beam(&store, 190, 0.25).is_none());

    // A crystal outside the vanilla inflated search box is ignored.
    store.apply_add_entity(add_at(
        191,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        [42.0, 65.0, 0.0],
    ));
    assert!(dragon_beam(&store, 190, 0.25).is_none());

    // Add two in-range crystals; the nearer one supplies the beam offset.
    store.apply_add_entity(add_at(
        192,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        [10.0, 66.0, 0.0],
    ));
    store.apply_add_entity(add_at(
        193,
        VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        [6.0, 65.0, 0.0],
    ));
    store.apply_add_entity(add_at(194, VANILLA_ENTITY_TYPE_BAT_ID, [3.0, 65.0, 0.0]));
    store.advance_entity_client_animations(10);

    let beam = dragon_beam(&store, 190, 0.25).expect("dragon has an in-range healing crystal");
    let expected_y = 1.0 + vanilla_crystal_y(10.25);
    assert!(
        (beam.beam_offset[0] - 6.0).abs() < 1.0e-5,
        "{:?}",
        beam.beam_offset
    );
    assert!(
        (beam.beam_offset[1] - expected_y).abs() < 1.0e-5,
        "{:?} vs {expected_y}",
        beam.beam_offset
    );
    assert!(beam.beam_offset[2].abs() < 1.0e-5);

    // Non-dragons do not project the dragon-owned healing beam even when crystals are tracked.
    assert!(dragon_beam(&store, 194, 0.25).is_none());
}

#[test]
fn frog_swim_idle_activates_only_in_water_and_idle() {
    // Vanilla `Frog.tick` (client): `swimIdleAnimationState.animateWhen(isInWater() &&
    // !walkAnimation.isMoving(), tickCount)`. The projected `frog_swim_idle_seconds` is `>= 0` while
    // the timer runs (in water, not moving) and the `-1.0` stopped sentinel otherwise. The frog
    // reads the previous tick's `WalkAnimationState.isMoving()` before the current tick's
    // `updateWalkAnimation` advances, matching vanilla's tick order.
    const SOURCE_WATER_BLOCK_STATE_ID: i32 = 86;

    let source = |store: &WorldStore| {
        store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 81)
            .unwrap()
    };
    let swim_idle = |store: &WorldStore| source(store).frog_swim_idle_seconds;
    let walk = |store: &WorldStore| {
        let source = source(store);
        (source.walk_animation_position, source.walk_animation_speed)
    };
    let sync_position = |store: &mut WorldStore, x: f64, z: f64| {
        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id: 81,
                position: ProtocolVec3d { x, y: 2.0, z },
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
    };

    // A frog standing in a tall water column (submerged) or out of water.
    let make_store = |in_water: bool| {
        let mut store = WorldStore::with_dimension(crate::WorldDimension {
            min_y: 0,
            height: 16,
        });
        store.insert_decoded_chunk(empty_test_chunk());
        store.apply_add_entity(ProtocolAddEntity {
            id: 81,
            uuid: default_entity_uuid(),
            entity_type_id: VANILLA_ENTITY_TYPE_FROG_ID,
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
        if in_water {
            // Fill the column the frog's AABB occupies so `world_aabb_in_water` sees a submerged box.
            for y in 1..=4 {
                assert!(store.apply_block_update(ProtocolBlockUpdate {
                    pos: ProtocolBlockPos { x: 8, y, z: 8 },
                    block_state_id: SOURCE_WATER_BLOCK_STATE_ID,
                }));
            }
        }
        store
    };

    // In water and idle: the swim-idle timer starts on the first tick (`start_age == age_ticks`),
    // so at partial `1.0` the elapsed seconds are `(0 + 1.0)/20 = 0.05` and climb `1/20 = 0.05` per
    // tick thereafter — non-negative, the active branch.
    let mut wet = make_store(true);
    wet.advance_entity_client_animations(1);
    assert!(
        (swim_idle(&wet) - 0.05).abs() < 1.0e-6,
        "an in-water idle frog activates its swim-idle: {}",
        swim_idle(&wet)
    );
    wet.advance_entity_client_animations(2);
    assert!(
        (swim_idle(&wet) - 0.15).abs() < 1.0e-6,
        "the swim-idle elapsed seconds climb 1/20 per tick: {}",
        swim_idle(&wet)
    );

    // Moving in water: the tick that observes the position delta still reads the previous
    // non-moving walk speed, then `Frog.updateWalkAnimation` records the movement. The following
    // tick sees `walkAnimation.isMoving()` and stops the idle animation.
    let mut wet_moving = make_store(true);
    wet_moving.advance_entity_client_animations(1);
    sync_position(&mut wet_moving, 8.52, 8.5);
    wet_moving.advance_entity_client_animations(1);
    let (_, moving_speed) = walk(&wet_moving);
    assert!(
        (moving_speed - 0.2).abs() < 1.0e-5,
        "frog movement uses targetSpeed=min(distance*25,1): {moving_speed}"
    );
    assert!(
        (swim_idle(&wet_moving) - 0.10).abs() < 1.0e-6,
        "the movement tick still reads the previous idle walk speed: {}",
        swim_idle(&wet_moving)
    );
    wet_moving.advance_entity_client_animations(1);
    assert_eq!(
        swim_idle(&wet_moving),
        -1.0,
        "a moving in-water frog stops its swim-idle on the next tick"
    );

    // Out of water: the gate is false, the timer never starts, so the `-1.0` sentinel holds.
    let mut dry = make_store(false);
    dry.advance_entity_client_animations(3);
    assert_eq!(
        swim_idle(&dry),
        -1.0,
        "an out-of-water frog never activates its swim-idle"
    );

    // Leaving the water stops the animation: drain the column the wet frog idles in, then tick.
    for y in 1..=4 {
        assert!(wet.apply_block_update(ProtocolBlockUpdate {
            pos: ProtocolBlockPos { x: 8, y, z: 8 },
            block_state_id: 0,
        }));
    }
    wet.advance_entity_client_animations(1);
    assert_eq!(
        swim_idle(&wet),
        -1.0,
        "a frog that leaves the water stops its swim-idle (back to the sentinel)"
    );
}

#[test]
fn frog_walk_animation_uses_vanilla_update_override() {
    const VANILLA_POSE_LONG_JUMPING_ID: i32 = 6;

    let walk = |store: &WorldStore| -> (f32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(1.0)
            .into_iter()
            .find(|source| source.entity_id == 82)
            .unwrap();
        (source.walk_animation_position, source.walk_animation_speed)
    };
    let sync_position = |store: &mut WorldStore, x: f64| {
        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id: 82,
                position: ProtocolVec3d { x, y: 64.0, z: 0.0 },
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
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        82,
        VANILLA_ENTITY_TYPE_FROG_ID,
    ));
    sync_position(&mut store, 0.0);
    store.advance_entity_client_animations(1);
    assert_eq!(walk(&store), (0.0, 0.0));

    // `Frog.updateWalkAnimation`: targetSpeed = min(distance * 25, 1). A 0.02-block
    // step gives target 0.5, so the vanilla 0.4 low-pass reaches speed/position 0.2.
    sync_position(&mut store, 0.02);
    store.advance_entity_client_animations(1);
    let (pos, speed) = walk(&store);
    assert!((speed - 0.2).abs() < 1.0e-5, "frog walk speed: {speed}");
    assert!((pos - 0.2).abs() < 1.0e-5, "frog walk pos: {pos}");

    // While `jumpAnimationState.isStarted()` (`Pose.LONG_JUMPING`), vanilla forces
    // the target speed to 0, so the existing speed decays by the same 0.4 factor.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 82,
        values: vec![protocol_pose_data(6, VANILLA_POSE_LONG_JUMPING_ID)],
    }));
    sync_position(&mut store, 0.04);
    store.advance_entity_client_animations(1);
    let (jump_pos, jump_speed) = walk(&store);
    assert!(
        (jump_speed - 0.12).abs() < 1.0e-5,
        "long-jumping frog walk speed decays toward zero: {jump_speed}"
    );
    assert!(
        (jump_pos - 0.32).abs() < 1.0e-5,
        "long-jumping frog walk position keeps integrating the decayed speed: {jump_pos}"
    );
}

#[test]
fn squid_tentacle_speed_matches_java_random_for_known_id() {
    // Vanilla `Squid` constructor: `random.setSeed(getId()); tentacleSpeed = 1 /
    // (random.nextFloat() + 1) * 0.2`. Pinned against the Java LCG: for id 0 the
    // first `nextFloat()` is 0.730_967_76 (matching the audio module's LCG test),
    // so `tentacleSpeed = 1 / 1.730_967_76 * 0.2 = 0.115_542_31`.
    let state = super::animations::SquidAnimationState::new(0);
    assert!(
        (state.tentacle_speed - 0.115_542_31).abs() < 1.0e-7,
        "id-0 tentacle speed must match the Java Random formula: {}",
        state.tentacle_speed
    );
}
