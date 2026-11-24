use super::*;

#[test]
fn entity_model_instances_project_chicken_adult_and_baby_models() {
    let mut world = WorldStore::new();
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
    world.apply_add_entity(protocol_add_entity(
        85,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [5.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 27,
        values: vec![baby_data(VANILLA_ENTITY_TYPE_CHICKEN_ID, true)],
    }));

    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);

    assert_eq!(
        without_bounds(&instances),
        aged(
            vec![
                EntityModelInstance::chicken(26, [1.0, 64.0, -2.0], 0.0, false)
                    .with(|s| s.kind = EntityKindRenderState::Chicken(Default::default())),
                EntityModelInstance::chicken(27, [3.0, 64.0, -2.0], 0.0, true)
                    .with(|s| s.kind = EntityKindRenderState::Chicken(Default::default()))
                    .with(|s| s.age_scale = 0.5),
                EntityModelInstance::new(85, EntityModelKind::Minecart, [5.0, 64.0, -2.0], 0.0,)
                    .with(|s| s.kind = EntityKindRenderState::Minecart(Default::default())),
            ],
            1.0,
        )
    );
}

#[test]
fn entity_model_instances_project_warden_heart_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        96,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
        [1.0, 64.0, -2.0],
    ));

    // A warden between heartbeats projects no heart pulse, so the heart overlay's alpha is 0.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(resting[0].render_state.kind.warden().heart_animation, 0.0);

    // With no synced anger, vanilla `Warden.getHeartBeatDelay()` is the calm 40, so the heartbeat
    // (`tickCount % 40 == 0`) first fires on the 40th client tick: `heartAnimation = 10`, then
    // `heartAnimationO = 10; heartAnimation--`, leaving the pair (10, 9). At partialTick 1.0
    // `getHeartAnimation` lerps to 9/10.
    world.advance_entity_client_animations(40);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(
        instances[0].render_state.kind.warden().heart_animation,
        9.0 / 10.0,
        "the projected heartbeat pulse drives the warden heart emissive overlay's alpha"
    );
}

#[test]
fn entity_model_instances_project_squid_out_of_water_tentacle_and_body_tilt_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        95,
        VANILLA_ENTITY_TYPE_SQUID_ID,
        [1.0, 64.0, -2.0],
    ));
    // The out-of-water branch ignores horizontal swim velocity for pose, but the motion packet
    // keeps this test on the same projection path as the in-water branch.
    assert!(world.apply_entity_position_sync(EntityPositionSync {
        id: 95,
        position: Vec3d {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        delta_movement: Vec3d {
            x: 0.2,
            y: -0.1,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        on_ground: false,
    }));

    // A floating squid at rest projects the bind pose into the render state.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let resting = resting
        .iter()
        .find(|instance| instance.entity_id == 95)
        .unwrap();
    assert_eq!(resting.render_state.kind.squid().tentacle_angle, 0.0);
    assert_eq!(resting.render_state.kind.squid().x_body_rot, 0.0);
    assert_eq!(resting.render_state.kind.squid().z_body_rot, 0.0);

    // One tick out of water uses the suffocating branch: tentacles flex with
    // `abs(sin(tentacleMovement))`, xBodyRot eases toward -90 degrees, and zBodyRot is untouched.
    world.advance_entity_client_animations(1);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let squid = instances
        .iter()
        .find(|instance| instance.entity_id == 95)
        .unwrap();
    assert!(
        squid.render_state.kind.squid().tentacle_angle > 0.0,
        "the projected tentacle angle drives SquidModel.setupAnim: {}",
        squid.render_state.kind.squid().tentacle_angle
    );
    assert!(
        squid.render_state.kind.squid().x_body_rot < 0.0,
        "an out-of-water squid projects a negative body pitch: {}",
        squid.render_state.kind.squid().x_body_rot
    );
    assert_eq!(
        squid.render_state.kind.squid().z_body_rot,
        0.0,
        "out of water leaves the swim roll untouched"
    );
}

#[test]
fn entity_model_instances_project_squid_body_yaw_from_world_animation() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_rotation(
        96,
        VANILLA_ENTITY_TYPE_SQUID_ID,
        [1.0, 64.0, -2.0],
        20.0,
        5.0,
        30.0,
    ));

    // Vanilla `LivingEntity.recreateFromPacket` seeds squid `yBodyRot` from
    // the head yaw. A dry squid does not refine that yaw in `Squid.aiStep`, so
    // the native instance must use 30 as `bodyRot` and keep the head yaw
    // relative to that projected body yaw, not the synced transform yaw 20.
    world.advance_entity_client_animations(1);
    let instances = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    let squid = instances
        .iter()
        .find(|instance| instance.entity_id == 96)
        .unwrap();
    assert_eq!(squid.render_state.body_rot, 30.0);
    assert_eq!(squid.render_state.head_yaw, 0.0);
    assert_eq!(squid.render_state.head_pitch, 5.0);
    assert!(
        squid.render_state.kind.squid().x_body_rot < 0.0,
        "the same world animation state still feeds SquidRenderer body pitch"
    );
}

#[test]
fn entity_model_instances_project_evoker_fangs_bite_progress_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        98,
        VANILLA_ENTITY_TYPE_EVOKER_FANGS_ID,
        [1.0, 64.0, -2.0],
    ));

    let bite = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .iter()
            .find(|instance| instance.entity_id == 98)
            .map(|instance| instance.render_state.kind.evoker_fangs().bite_progress)
    };

    // A fang that has not started its attack is hidden underground: biteProgress 0.
    assert_eq!(bite(&world), Some(0.0));

    // Vanilla `EvokerFangs.handleEntityEvent`: event 4 starts the attack, and the
    // `lifeTicks` countdown drives the biteProgress ramp above 0, flowing through
    // EntityRenderSource into the renderer EntityRenderState
    // (`EvokerFangsModel.setupAnim` jaw snap / rise / vanish).
    assert!(world.apply_entity_event(EntityEvent {
        entity_id: 98,
        event_id: 4,
    }));
    world.advance_entity_client_animations(3);
    let progress = bite(&world).expect("the attacking fang projects an instance");
    assert!(
        progress > 0.0,
        "the projected bite ramp drives EvokerFangsModel.setupAnim: {progress}"
    );
}

#[test]
fn entity_model_instances_project_camel_dash_seconds_from_world() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        99,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        [1.0, 64.0, -2.0],
    ));

    let dash = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .iter()
            .find(|instance| instance.entity_id == 99)
            .map(|instance| {
                (
                    instance.render_state.kind.camel().dash_seconds,
                    instance.render_state.kind.camel().idle_seconds,
                    instance.render_state.kind.camel().jump_cooldown,
                )
            })
    };

    // A non-dashing camel projects the stopped-animation sentinel.
    assert_eq!(dash(&world), Some((-1.0, -1.0, 0.0)));

    // Vanilla `Camel.setupAnimationStates`: the synced `DASH` boolean starts `dashAnimationState`,
    // and the elapsed seconds flow through EntityRenderSource into the renderer EntityRenderState
    // (`CamelModel.setupAnim` looping `CAMEL_DASH` gallop).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 99,
        values: vec![camel_dash_data(true)],
    }));
    world.advance_entity_client_animations(2);
    let progress = dash(&world).expect("the dashing camel projects an instance");
    assert!(
        progress.0 >= 0.0,
        "the projected dash timer drives CamelModel.setupAnim: {progress:?}"
    );
    assert!(
        (progress.1 - 0.1).abs() < 1.0e-6,
        "the projected idle timer drives CamelModel.setupAnim: {progress:?}"
    );
    assert_eq!(
        progress.2, 52.0,
        "the projected dash cooldown drives CamelModel.applyHeadRotation"
    );
}

#[test]
fn entity_model_instances_project_polar_bear_standing_scale() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        80,
        VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        81,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [3.0, 64.0, -2.0],
    ));

    // A polar bear on all fours and any other entity carry a zero scale.
    let resting = entity_model_instances_from_world_at_partial_tick(&world, None, 1.0);
    assert_eq!(resting[0].render_state.kind.polar_bear().stand_scale, 0.0);
    assert_eq!(resting[1].render_state.kind.polar_bear().stand_scale, 0.0);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 80,
        values: vec![polar_bear_standing_data(true)],
    }));
    world.advance_entity_client_animations(1);

    // Vanilla PolarBearRenderer.extractRenderState reads
    // getStandingAnimationScale(partialTick); after one tick that is
    // lerp(0.5, 0, 1) / 6.
    let standing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5);
    assert_eq!(
        standing[0].render_state.kind.polar_bear().stand_scale,
        0.5 / 6.0
    );
    assert_eq!(standing[1].render_state.kind.polar_bear().stand_scale, 0.0);
}

#[test]
fn entity_model_instances_project_shulker_peek() {
    let accessors = shulker_metadata_accessors();
    // Vanilla Shulker.DATA_PEEK_ID (BYTE), a 0..=100 percentage; the client peek state
    // advances 0.05/tick toward raw·0.01 and the render state reads the partial-tick lerp
    // `Shulker.getClientPeekAmount` (`Mth.lerp(partialTick, currentPeekAmountO, current)`).
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        82,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        83,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [3.0, 64.0, -2.0],
    ));

    let peek = |world: &WorldStore, id: i32, partial: f32| {
        entity_model_instances_from_world_at_partial_tick(world, None, partial)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .shulker()
            .peek
    };

    // A closed shulker and every other entity carry a zero peek (the closed/bind pose).
    assert_eq!(peek(&world, 82, 1.0), 0.0);
    assert_eq!(peek(&world, 83, 1.0), 0.0);

    // Open the lid fully (raw 100 → target 1.0), then advance one tick: the client peek steps
    // 0.05 from 0. At partial-tick 0.5 the render state lerps `0 + 0.5·(0.05 − 0) = 0.025`.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 82,
        values: vec![protocol_byte_data(accessors.peek_data_id, 100)],
    }));
    world.advance_entity_client_animations(1);
    assert!((peek(&world, 82, 0.5) - 0.025).abs() < 1.0e-6);
    // The chicken has no peek state, so it stays at the closed/bind pose.
    assert_eq!(peek(&world, 83, 0.5), 0.0);
}

#[test]
fn entity_model_instances_project_shulker_attach_face() {
    let accessors = shulker_metadata_accessors();
    // Vanilla Shulker.DATA_ATTACH_FACE_ID (DIRECTION) feeds
    // `ShulkerRenderState.attachFace`, whose default is `Direction.DOWN`.
    const DIRECTION_NORTH: i32 = 2;
    const DIRECTION_NEGATIVE_ONE: i32 = -1;

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        82,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        83,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [3.0, 64.0, -2.0],
    ));

    let attach_face = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .shulker()
            .attach_face
    };

    assert_eq!(attach_face(&world, 82), EntityAttachmentFace::Down);
    assert_eq!(attach_face(&world, 83), EntityAttachmentFace::Down);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 82,
        values: vec![protocol_direction_data(
            accessors.attach_face_data_id,
            DIRECTION_NORTH,
        )],
    }));
    assert_eq!(attach_face(&world, 82), EntityAttachmentFace::North);
    assert_eq!(attach_face(&world, 83), EntityAttachmentFace::Down);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 82,
        values: vec![protocol_direction_data(
            accessors.attach_face_data_id,
            DIRECTION_NEGATIVE_ONE,
        )],
    }));
    // Vanilla `Direction.BY_ID` uses positive-modulo wrap, so -1 wraps to EAST.
    assert_eq!(attach_face(&world, 82), EntityAttachmentFace::East);
}

#[test]
fn entity_model_instances_project_death_animation_time() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        82,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));

    // A living entity at rest carries deathTime 0 and no red overlay.
    let alive = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    assert_eq!(alive[0].render_state.death_time, 0.0);
    assert!(!alive[0].render_state.has_red_overlay);

    // Vanilla isDeadOrDying(): health <= 0 starts the death counter; tickDeath
    // increments it each client tick, projected (plus the partial tick) as
    // LivingEntityRenderState.deathTime and driving the red overlay.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 82,
        values: vec![living_entity_health_data(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            0.0,
        )],
    }));
    world.advance_entity_client_animations(2);
    let dying = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);
    assert_eq!(dying[0].render_state.death_time, 2.25);
    assert!(dying[0].render_state.has_red_overlay);
}

#[test]
fn entity_model_instances_project_creaking_teardown_flicker_without_death_overlay() {
    let (tearing_down_data_id, tearing_down_default) =
        bbb_client_world::vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CREAKING_ID,
        )
        .expect("generated Creaking tearing-down accessor");
    assert!(!tearing_down_default);

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        822,
        VANILLA_ENTITY_TYPE_CREAKING_ID,
        [1.0, 64.0, -2.0],
    ));
    // Keep synced `isActive()` true to prove teardown ignores it in favor of the local flicker.
    // Health zero starts the shared death/red-overlay source state exactly as the server's
    // `CreakingHeartBlockEntity.removeProtector` does alongside `setTearingDown()`.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 822,
        values: vec![
            protocol_bool_data(creaking_active_data_id(), true),
            protocol_bool_data(tearing_down_data_id, true),
            living_entity_health_data(VANILLA_ENTITY_TYPE_CREAKING_ID, 0.0),
        ],
    }));

    // First teardown tick: deathTime 1 schedules the false-eye 0..=0 interval and toggles on.
    world.advance_entity_client_animations(1);
    let first = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 822)
        .unwrap();
    assert_eq!(first.kind, EntityModelKind::Creaking { eyes_glowing: true });
    assert_eq!(first.render_state.death_time, 0.0);
    assert!(!first.render_state.has_red_overlay);

    // Second tick schedules from the glowing 2..=8 interval and toggles off. Synced active remains
    // true, so a false kind here proves native projection selected `hasGlowingEyes()`.
    world.advance_entity_client_animations(1);
    let second = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 822)
        .unwrap();
    assert_eq!(
        second.kind,
        EntityModelKind::Creaking {
            eyes_glowing: false
        }
    );
    assert_eq!(second.render_state.death_time, 0.0);
    assert!(!second.render_state.has_red_overlay);
}

#[test]
fn entity_model_instances_fold_freeze_shake_into_body_rot() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        83,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));

    // A living entity that is not frozen solid has an unshaken body yaw.
    let warm = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0);
    assert_eq!(warm[0].render_state.body_rot, 0.0);

    // Vanilla Entity.isFullyFrozen(): ticksFrozen >= 140. setupRotations then
    // adds cos(floor(ageInTicks) * 3.25) * π * 0.4 to the body yaw; the shake
    // uses the floored (integer) tick count, so it does not lerp with partial.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 83,
        values: vec![entity_ticks_frozen_data(140)],
    }));
    world.advance_entity_client_animations(2);
    let frozen = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25);
    let expected_shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;
    assert!((frozen[0].render_state.body_rot - expected_shake).abs() < 1e-6);
    // The head turn relative to the body is unchanged by the shake.
    assert_eq!(frozen[0].render_state.head_yaw, 0.0);
}

#[test]
fn entity_model_instances_shake_zombie_family_while_converting() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        84,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        85,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
        [3.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        86,
        VANILLA_ENTITY_TYPE_HUSK_ID,
        [5.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        87,
        VANILLA_ENTITY_TYPE_DROWNED_ID,
        [7.0, 64.0, -2.0],
    ));
    world.advance_entity_client_animations(2);

    let shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;
    let body_rot = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .body_rot
    };

    // A non-converting zombie / zombie villager does not shake.
    assert_eq!(body_rot(&world, 84), 0.0);
    assert_eq!(body_rot(&world, 85), 0.0);

    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 84,
        values: vec![protocol_zombie_villager_converting_data(true)],
    }));
    assert_eq!(
        body_rot(&world, 84),
        0.0,
        "Zombie does not own ZombieVillager.DATA_CONVERTING_ID"
    );

    // AbstractZombieRenderer.isShaking ORs in DATA_DROWNED_CONVERSION_ID (18) for the
    // registered Zombie, Husk, and Drowned renderers.
    for (id, entity_type_id) in [
        (84, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        (86, VANILLA_ENTITY_TYPE_HUSK_ID),
        (87, VANILLA_ENTITY_TYPE_DROWNED_ID),
    ] {
        assert!(world.apply_set_entity_data(SetEntityData {
            id,
            values: vec![protocol_zombie_drowned_conversion_data(
                entity_type_id,
                true
            )],
        }));
        assert!((body_rot(&world, id) - shake).abs() < 1e-6);
    }

    // ZombieVillagerRenderer does not inherit AbstractZombieRenderer, so inherited
    // Zombie.DATA_DROWNED_CONVERSION_ID does not affect its shake projection.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 85,
        values: vec![protocol_zombie_drowned_conversion_data(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            true,
        )],
    }));
    assert_eq!(body_rot(&world, 85), 0.0);

    // ZombieVillagerRenderer independently ORs in DATA_CONVERTING_ID (19).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 85,
        values: vec![protocol_zombie_villager_converting_data(true)],
    }));
    assert!((body_rot(&world, 85) - shake).abs() < 1e-6);
}

#[test]
fn entity_model_instances_shake_striders_while_suffocating() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        86,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        87,
        VANILLA_ENTITY_TYPE_PIG_ID,
        [3.0, 64.0, -2.0],
    ));
    // Keep this renderer-only fixture's synchronized warm/cold state under
    // explicit test control. Vanilla Strider.tick recomputes suffocation in
    // air unless noAI is set, which is now part of the native client tick.
    let mob_accessors = bbb_client_world::vanilla_entity_metadata_mob_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_STRIDER_ID,
    )
    .expect("generated Strider Mob accessors");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 86,
        values: vec![protocol_byte_data(
            mob_accessors.flags_data_id,
            mob_accessors.no_ai_mask as i8,
        )],
    }));
    world.advance_entity_client_animations(2);

    let strider = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.25)
            .into_iter()
            .find(|instance| instance.entity_id == 86)
            .unwrap()
    };

    // A warm strider does not shake and keeps the non-cold texture variant.
    let warm = strider(&world);
    assert_eq!(warm.render_state.body_rot, 0.0);
    assert_eq!(
        warm.kind,
        EntityModelKind::Strider {
            baby: false,
            cold: false
        }
    );

    // StriderRenderer.isShaking ORs in StriderRenderState.isSuffocating,
    // which is extracted from DATA_SUFFOCATING (19). The same flag also
    // selects the cold texture.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 86,
        values: vec![strider_suffocating_data(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            true,
        )],
    }));
    let shaking = strider(&world);
    let expected_shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;
    assert!((shaking.render_state.body_rot - expected_shake).abs() < 1e-6);
    assert_eq!(shaking.render_state.head_yaw, 0.0);
    assert_eq!(
        shaking.kind,
        EntityModelKind::Strider {
            baby: false,
            cold: true
        }
    );

    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 87,
        values: vec![strider_suffocating_data(
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            true,
        )],
    }));
    let pig_body_rot = entity_model_instances_from_world_at_partial_tick(&world, None, 0.25)
        .into_iter()
        .find(|instance| instance.entity_id == 87)
        .unwrap()
        .render_state
        .body_rot;
    assert_eq!(
        pig_body_rot, 0.0,
        "Pig does not own Strider.DATA_SUFFOCATING"
    );
}

#[test]
fn entity_model_instances_shake_skeletons_while_freeze_converting() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        87,
        VANILLA_ENTITY_TYPE_SKELETON_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        88,
        VANILLA_ENTITY_TYPE_STRAY_ID,
        [3.0, 64.0, -2.0],
    ));
    world.advance_entity_client_animations(2);

    let body_rot = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == 87)
            .unwrap()
            .render_state
            .body_rot
    };
    assert_eq!(body_rot(&world), 0.0);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 87,
        values: vec![skeleton_freeze_conversion_data(
            VANILLA_ENTITY_TYPE_SKELETON_ID,
            true,
        )],
    }));
    let expected_shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;
    assert!((body_rot(&world) - expected_shake).abs() < 1e-6);

    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 88,
        values: vec![skeleton_freeze_conversion_data(
            VANILLA_ENTITY_TYPE_SKELETON_ID,
            true,
        )],
    }));
    let stray_body_rot = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 88)
        .unwrap()
        .render_state
        .body_rot;
    assert_eq!(
        stray_body_rot, 0.0,
        "Stray does not own Skeleton.DATA_STRAY_CONVERSION_ID"
    );
}

#[test]
fn entity_model_instances_shake_piglins_and_hoglins_while_zombifying() {
    fn dimension_world(dimension_type_id: i32, dimension: &str) -> WorldStore {
        let mut world = WorldStore::new();
        let mut login = protocol_play_login(10);
        login.levels = vec![dimension.to_string()];
        login.common_spawn_info.dimension_type_id = dimension_type_id;
        login.common_spawn_info.dimension = dimension.to_string();
        world.apply_login(&login);
        insert_loaded_entity_chunk(&mut world, ChunkPos { x: 0, z: -1 });
        world
    }

    fn add_piglin_family(world: &mut WorldStore) {
        for (id, entity_type_id, position) in [
            (87, VANILLA_ENTITY_TYPE_PIGLIN_ID, [1.0, 64.0, -2.0]),
            (88, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, [3.0, 64.0, -2.0]),
            (89, VANILLA_ENTITY_TYPE_HOGLIN_ID, [5.0, 64.0, -2.0]),
            (90, VANILLA_ENTITY_TYPE_ZOGLIN_ID, [7.0, 64.0, -2.0]),
            (
                91,
                VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
                [9.0, 64.0, -2.0],
            ),
        ] {
            world.apply_add_entity(protocol_add_entity(id, entity_type_id, position));
        }
        world.advance_entity_client_animations(2);
    }

    fn body_rot(world: &WorldStore, id: i32) -> f32 {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .body_rot
    }

    let shake = (2.0_f32 * 3.25).cos() * std::f32::consts::PI * 0.4;

    let mut overworld = dimension_world(0, "minecraft:overworld");
    add_piglin_family(&mut overworld);

    // Vanilla `PiglinRenderer` and `HoglinRenderer` OR `isConverting()` into
    // `isShaking`. The built-in Overworld keeps
    // `EnvironmentAttributes.PIGLINS_ZOMBIFY` at its default `true`.
    for id in [87, 88, 89] {
        assert!((body_rot(&overworld, id) - shake).abs() < 1e-6);
    }
    // ZoglinRenderer and ZombifiedPiglinRenderer do not expose a conversion override.
    for id in [90, 91] {
        assert_eq!(body_rot(&overworld, id), 0.0);
    }

    // Synced immune-to-zombification metadata suppresses the conversion shake.
    assert!(overworld.apply_set_entity_data(SetEntityData {
        id: 87,
        values: vec![zombification_immunity_data(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
            true,
        )],
    }));
    assert!(overworld.apply_set_entity_data(SetEntityData {
        id: 88,
        values: vec![zombification_immunity_data(
            VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
            true,
        )],
    }));
    assert!(overworld.apply_set_entity_data(SetEntityData {
        id: 89,
        values: vec![zombification_immunity_data(
            VANILLA_ENTITY_TYPE_HOGLIN_ID,
            true,
        )],
    }));
    for id in [87, 88, 89] {
        assert_eq!(body_rot(&overworld, id), 0.0);
    }

    let mut nether = dimension_world(1, "minecraft:the_nether");
    add_piglin_family(&mut nether);
    // Vanilla built-in Nether sets `PIGLINS_ZOMBIFY` to false.
    for id in [87, 88, 89, 90, 91] {
        assert_eq!(body_rot(&nether, id), 0.0);
    }

    let mut custom = WorldStore::new();
    install_piglins_zombify_dimension(&mut custom, false);
    custom.apply_login(&protocol_play_login(10));
    add_piglin_family(&mut custom);
    for id in [87, 88, 89, 90, 91] {
        assert_eq!(body_rot(&custom, id), 0.0);
    }
}

#[test]
fn entity_model_instances_project_auto_spin_attack() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        86,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [1.0, 64.0, -2.0],
    ));
    // A non-living entity never carries the living-entity flags byte.
    world.apply_add_entity(protocol_add_entity(
        87,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        [3.0, 64.0, -2.0],
    ));

    let auto_spin = |world: &WorldStore, id: i32, partial: f32| {
        entity_model_instances_from_world_at_partial_tick(world, None, partial)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .auto_spin_age_ticks
    };

    // A living entity at rest is not spinning.
    assert_eq!(auto_spin(&world, 86, 0.0), None);

    // Vanilla LivingEntity.isAutoSpinAttack(): DATA_LIVING_ENTITY_FLAGS & 4.
    // setupRotations then reads the lerped ageInTicks (tickCount + partial).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 86,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            true,
            false,
            true,
        )],
    }));
    world.advance_entity_client_animations(3);
    assert_eq!(auto_spin(&world, 86, 0.5), Some(3.5));

    // Clearing the spin bit (other living flags still set) stops the spin.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 86,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            true,
            false,
            false,
        )],
    }));
    assert_eq!(auto_spin(&world, 86, 0.5), None);

    // The living-entity gate keeps a non-living entity from ever spinning, even
    // if a stray flags byte is present.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 87,
        values: vec![living_entity_flags_data(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            false,
            false,
            true,
        )],
    }));
    assert_eq!(auto_spin(&world, 87, 0.5), None);
}

#[test]
fn entity_model_instances_project_boat_rowing_and_damage_times() {
    const ADVANCE: f32 = std::f32::consts::PI / 8.0;
    let boat_accessors = abstract_boat_metadata_accessors();
    let vehicle_accessors = vehicle_metadata_accessors(VANILLA_ENTITY_TYPE_OAK_BOAT_ID);

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        90,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        91,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        [2.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 90,
        passenger_ids: vec![91],
    }));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 90,
        values: vec![
            protocol_int_data(vehicle_accessors.hurt_time_data_id, 10),
            protocol_int_data(vehicle_accessors.hurt_dir_data_id, -1),
            protocol_float_data(vehicle_accessors.damage_data_id, 20.0),
            protocol_bool_data(boat_accessors.paddle_left_data_id, true),
            protocol_bool_data(boat_accessors.paddle_right_data_id, true),
            protocol_int_data(boat_accessors.bubble_time_data_id, 60),
        ],
    }));

    world.advance_entity_client_animations(2);
    let render_state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5)
        .into_iter()
        .find(|instance| instance.entity_id == 90)
        .unwrap()
        .render_state;

    assert!((render_state.kind.boat().rowing_time_left - ADVANCE * 1.5).abs() < 1.0e-6);
    assert!((render_state.kind.boat().rowing_time_right - ADVANCE * 1.5).abs() < 1.0e-6);
    assert!((render_state.kind.vehicle_hurt().time - 7.5).abs() < 1.0e-6);
    assert_eq!(render_state.kind.vehicle_hurt().dir, -1);
    assert!((render_state.kind.vehicle_hurt().damage_time - 17.5).abs() < 1.0e-6);
    let first_bubble_angle = 10.0 * (0.5_f32).sin() * 0.05;
    let second_bubble_angle = 10.0 * (1.0_f32).sin() * 0.1;
    let expected_bubble_angle =
        first_bubble_angle + (second_bubble_angle - first_bubble_angle) * 0.5;
    assert!((render_state.kind.boat().bubble_angle - expected_bubble_angle).abs() < 1.0e-6);
}

#[test]
fn entity_model_instances_project_minecart_damage_times() {
    let accessors = vehicle_metadata_accessors(VANILLA_ENTITY_TYPE_MINECART_ID);

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        93,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 93,
        values: vec![
            protocol_int_data(accessors.hurt_time_data_id, 10),
            protocol_int_data(accessors.hurt_dir_data_id, -1),
            protocol_float_data(accessors.damage_data_id, 20.0),
        ],
    }));

    world.advance_entity_client_animations(2);
    let render_state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5)
        .into_iter()
        .find(|instance| instance.entity_id == 93)
        .unwrap()
        .render_state;

    assert!((render_state.kind.vehicle_hurt().time - 7.5).abs() < 1.0e-6);
    assert_eq!(render_state.kind.vehicle_hurt().dir, -1);
    assert!((render_state.kind.vehicle_hurt().damage_time - 17.5).abs() < 1.0e-6);
}

#[test]
fn entity_model_instances_project_tnt_minecart_fuse_remaining_ticks() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        95,
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(world.apply_entity_event(EntityEvent {
        entity_id: 95,
        event_id: 10,
    }));
    world.advance_entity_client_animations(76);

    let render_state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5)
        .into_iter()
        .find(|instance| instance.entity_id == 95)
        .unwrap()
        .render_state;

    assert_eq!(
        render_state.kind.minecart().tnt_fuse_remaining_in_ticks,
        4.5
    );
}

#[test]
fn entity_model_instances_project_minecart_new_render_from_track_steps() {
    let mut world = WorldStore::new();
    world.apply_update_enabled_features(bbb_protocol::packets::UpdateEnabledFeatures {
        features: vec![
            "minecraft:vanilla".to_string(),
            "minecraft:minecart_improvements".to_string(),
        ],
    });
    world.apply_add_entity(protocol_add_entity(
        94,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        [1.0, 64.0, -2.0],
    ));
    assert!(
        world.apply_move_minecart_along_track(MoveMinecartAlongTrack {
            entity_id: 94,
            lerp_steps: vec![MinecartStep {
                position: Vec3d {
                    x: 1.75,
                    y: 64.2,
                    z: -2.75,
                },
                movement: Vec3d {
                    x: 0.4,
                    y: 0.0,
                    z: -0.4,
                },
                y_rot: 90.0,
                x_rot: 5.0,
                weight: 1.25,
            }],
        })
    );
    world.advance_entity_client_animations(1);

    let instance = entity_model_instances_from_world_at_partial_tick(&world, None, 0.5)
        .into_iter()
        .find(|instance| instance.entity_id == 94)
        .unwrap();
    assert!(instance.render_state.kind.minecart().new_render);
    assert!((instance.position[0] - 1.125).abs() < 1.0e-6);
    assert!((instance.position[1] - 64.03333).abs() < 1.0e-5);
    assert!((instance.position[2] + 2.125).abs() < 1.0e-6);
    assert!((instance.render_state.body_rot - 15.0).abs() < 1.0e-6);
    assert!((instance.render_state.head_pitch - 0.8333333).abs() < 1.0e-6);
}

#[test]
fn entity_model_instances_project_aggressive_for_zombie_family() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        90,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [1.0, 64.0, -2.0],
    ));

    let aggressive = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .is_aggressive
    };

    // A calm zombie projects is_aggressive = false.
    assert!(!aggressive(&world, 90));

    // Setting Mob.isAggressive through the generated accessor/mask projects through to the held-out
    // arm render state.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 90,
        values: vec![mob_aggressive_data(VANILLA_ENTITY_TYPE_ZOMBIE_ID)],
    }));
    assert!(aggressive(&world, 90));
}

#[test]
fn entity_model_instances_project_aggressive_for_pillager_attacking_pose() {
    // Vanilla `Pillager.getArmPose`: after crossbow charge/hold checks, an aggressive pillager returns
    // `ATTACKING`. Shared `IllagerRenderer.extractRenderState` still copies `isAggressive` for the
    // evoker, although the evoker arm-pose resolver ignores that field.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        91,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        92,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        [2.0, 64.0, -2.0],
    ));

    let aggressive = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .is_aggressive
    };

    assert!(!aggressive(&world, 91));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 91,
        values: vec![mob_aggressive_data(VANILLA_ENTITY_TYPE_PILLAGER_ID)],
    }));
    assert!(aggressive(&world, 91));

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 92,
        values: vec![mob_aggressive_data(VANILLA_ENTITY_TYPE_EVOKER_ID)],
    }));
    assert!(aggressive(&world, 92));
}

#[test]
fn entity_model_instances_project_enderman_carrying_and_creepy() {
    let accessors = vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    )
    .expect("generated EnderMan render metadata accessors");

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        94,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
        [1.0, 64.0, -2.0],
    ));

    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    // A freshly spawned enderman carries nothing and is not creepy.
    let calm = state(&world, 94);
    assert!(!calm.kind.enderman().carrying);
    assert!(!calm.kind.enderman().creepy);

    // A present carried block (`DATA_CARRY_STATE` set) and `DATA_CREEPY` project through
    // to the held-out arm pose and the creepy head/hat shift.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 94,
        values: vec![
            EntityDataValue {
                data_id: accessors.carried_block_state_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_STATE_ID,
                value: EntityDataValueKind::OptionalBlockState(Some(10)),
            },
            protocol_bool_data(accessors.creepy_data_id, true),
        ],
    }));
    let primed = state(&world, 94);
    assert!(primed.kind.enderman().carrying);
    assert!(primed.kind.enderman().creepy);
}

#[test]
fn entity_model_instances_project_bat_resting_and_flying_start_phase() {
    // Vanilla Bat.DATA_ID_FLAGS (BYTE) and its named resting bit.
    let (_, default, resting_mask) =
        bbb_client_world::vanilla_entity_metadata_bat_flags_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_BAT_ID,
        )
        .expect("generated Bat flags accessor must resolve");
    assert_eq!(default, 0);
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        95,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [1.0, 64.0, -2.0],
    ));

    let state = |world: &WorldStore, id: i32, partial_tick: f32| {
        entity_model_instances_from_world_at_partial_tick(world, None, partial_tick)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };

    // Before the first client tick the flying AnimationState is stopped. The first tick starts it
    // at elapsed zero, and native forwards the partial-tick sample into renderer state.
    let initial = state(&world, 95, 0.0);
    assert!(!initial.kind.bat().resting);
    assert_eq!(initial.kind.bat().flying_seconds, -1.0);
    world.advance_entity_client_animations(1);
    let flying = state(&world, 95, 0.5);
    assert!(!flying.kind.bat().resting);
    assert_eq!(flying.kind.bat().flying_seconds, 0.025);

    // Setting Bat.isResting via the named mask projects through to the hanging-pose state.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 95,
        values: vec![bat_flags_data(resting_mask)],
    }));
    assert!(state(&world, 95, 0.0).kind.bat().resting);
    world.advance_entity_client_animations(1);
    let resting = state(&world, 95, 0.0);
    assert!(resting.kind.bat().resting);
    assert_eq!(resting.kind.bat().flying_seconds, -1.0);

    // Taking off restarts the timer on the next bat tick instead of resuming the old absolute-age
    // phase.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 95,
        values: vec![bat_flags_data(default)],
    }));
    world.advance_entity_client_animations(1);
    let restarted = state(&world, 95, 0.0);
    assert!(!restarted.kind.bat().resting);
    assert_eq!(restarted.kind.bat().flying_seconds, 0.0);
}

#[test]
fn entity_model_instances_project_vex_charging() {
    // Vanilla Vex.DATA_FLAGS_ID (BYTE) and its named FLAG_IS_CHARGING bit.
    let (_, default, charging_mask) =
        bbb_client_world::vanilla_entity_metadata_vex_flags_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_VEX_ID,
        )
        .expect("generated Vex flags accessor must resolve");
    assert_eq!(default, 0);
    const PLAIN_ITEM_ID: i32 = 702;

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        97,
        VANILLA_ENTITY_TYPE_VEX_ID,
        [1.0, 64.0, -2.0],
    ));
    // A bat reuses data id 16 for its OWN resting flag — used below to prove the vex
    // charging projection is gated to the vex and never leaks onto another type.
    world.apply_add_entity(protocol_add_entity(
        98,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [2.0, 64.0, -2.0],
    ));

    let state = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
    };
    let equip =
        |entity_id: i32, slot: EquipmentSlot, item_id: Option<i32>, count: i32| SetEquipment {
            entity_id,
            slots: vec![EquipmentSlotUpdate {
                slot,
                item: ItemStackSummary {
                    item_id,
                    count,
                    component_patch: DataComponentPatchSummary::default().into(),
                },
            }],
        };

    // An idle vex projects vex_charging = false.
    let idle = state(&world, 97);
    assert!(!idle.kind.vex().charging);
    assert!(!idle.kind.vex().right_hand_item_non_empty);
    assert!(!idle.kind.vex().left_hand_item_non_empty);

    // Setting Vex.isCharging through the named mask projects through to the charging pose.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 97,
        values: vec![vex_flags_data(charging_mask)],
    }));
    assert!(state(&world, 97).kind.vex().charging);

    // Vanilla `ArmedEntityRenderState` checks RIGHT/LEFT hand item-state emptiness. bbb's current
    // Vex projection maps default RIGHT main hand to RIGHT and offhand to LEFT.
    assert!(world.apply_set_equipment(equip(97, EquipmentSlot::MainHand, Some(PLAIN_ITEM_ID), 1)));
    let main_hand_item = state(&world, 97);
    assert!(main_hand_item.kind.vex().right_hand_item_non_empty);
    assert!(!main_hand_item.kind.vex().left_hand_item_non_empty);

    assert!(world.apply_set_equipment(equip(97, EquipmentSlot::OffHand, Some(PLAIN_ITEM_ID), 1)));
    let both_hands = state(&world, 97);
    assert!(both_hands.kind.vex().right_hand_item_non_empty);
    assert!(both_hands.kind.vex().left_hand_item_non_empty);

    assert!(world.apply_set_equipment(equip(97, EquipmentSlot::MainHand, None, 0)));
    let offhand_only = state(&world, 97);
    assert!(!offhand_only.kind.vex().right_hand_item_non_empty);
    assert!(offhand_only.kind.vex().left_hand_item_non_empty);

    // Left-handed Vex swaps which visual arm holds canonical main/off hand items.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 97,
        values: vec![mob_left_handed_data(VANILLA_ENTITY_TYPE_VEX_ID)],
    }));
    let left_handed_offhand = state(&world, 97);
    assert!(left_handed_offhand.main_arm_left);
    assert!(left_handed_offhand.kind.vex().right_hand_item_non_empty);
    assert!(!left_handed_offhand.kind.vex().left_hand_item_non_empty);

    assert!(world.apply_set_equipment(equip(97, EquipmentSlot::OffHand, None, 0)));
    assert!(world.apply_set_equipment(equip(97, EquipmentSlot::MainHand, Some(PLAIN_ITEM_ID), 1)));
    let left_handed_main = state(&world, 97);
    assert!(!left_handed_main.kind.vex().right_hand_item_non_empty);
    assert!(left_handed_main.kind.vex().left_hand_item_non_empty);

    // The same flag byte set on a non-vex (bat) does NOT project vex_charging — the
    // derivation is gated by the generated accessor's Vex membership. The same held items also do
    // not project Vex hand state.
    assert!(world.apply_set_equipment(equip(98, EquipmentSlot::MainHand, Some(PLAIN_ITEM_ID), 1)));
    assert!(world.apply_set_equipment(equip(98, EquipmentSlot::OffHand, Some(PLAIN_ITEM_ID), 1)));
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 98,
        values: vec![vex_flags_collision_data(charging_mask)],
    }));
    let bat = state(&world, 98);
    assert!(!bat.kind.vex().charging);
    assert!(!bat.kind.vex().right_hand_item_non_empty);
    assert!(!bat.kind.vex().left_hand_item_non_empty);
}

#[test]
fn entity_model_instances_project_turtle_has_egg() {
    // Vanilla Turtle.HAS_EGG (AgeableMob 16/17 then Turtle's BOOLEAN data id 18) and
    // TurtleRenderer.extractRenderState: state.hasEgg = !entity.isBaby() && entity.hasEgg().
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        140,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        [1.0, 64.0, -2.0],
    ));
    // A second turtle (made a baby below), plus a non-turtle (bat) that reuses data id 18 for
    // its own flag — used to prove the egg projection is gated to adult turtles.
    world.apply_add_entity(protocol_add_entity(
        141,
        VANILLA_ENTITY_TYPE_TURTLE_ID,
        [2.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        142,
        VANILLA_ENTITY_TYPE_BAT_ID,
        [3.0, 64.0, -2.0],
    ));

    let has_egg = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .turtle()
            .has_egg
    };

    // An adult turtle without the flag projects turtle_has_egg = false.
    assert!(!has_egg(&world, 140));

    // Setting Turtle.HAS_EGG (data id 18) on the adult projects the egg belly.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 140,
        values: vec![turtle_has_egg_data(true)],
    }));
    assert!(has_egg(&world, 140));

    // A baby turtle with HAS_EGG set stays false (gated on !isBaby()).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 141,
        values: vec![
            baby_data(VANILLA_ENTITY_TYPE_TURTLE_ID, true),
            turtle_has_egg_data(true),
        ],
    }));
    assert!(!has_egg(&world, 141));

    // The same flag on a non-turtle (bat) does NOT project turtle_has_egg.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 142,
        values: vec![turtle_has_egg_collision_data(true)],
    }));
    assert!(!has_egg(&world, 142));
}

#[test]
fn entity_model_instances_project_pillager_charging_crossbow() {
    // Vanilla Pillager.IS_CHARGING_CROSSBOW and Pillager.getArmPose: a charging pillager renders
    // CROSSBOW_CHARGE, suppressing CROSSBOW_HOLD. SpellcasterIllager owns a different accessor at
    // the reused wire slot, so generated Pillager membership must gate the projection.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        160,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        161,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        [2.0, 64.0, -2.0],
    ));
    let (charging_data_id, charging_default) =
        bbb_client_world::vanilla_entity_metadata_pillager_charging_crossbow_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
        )
        .expect("generated Pillager charging accessor");
    assert!(!charging_default);

    let charging = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .illager()
            .is_charging_crossbow
    };

    // A pillager without the flag is not charging.
    assert!(!charging(&world, 160));

    // Setting the generated IS_CHARGING_CROSSBOW accessor projects the charge state.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 160,
        values: vec![protocol_bool_data(charging_data_id, true)],
    }));
    assert!(charging(&world, 160));

    // The same wire slot on an evoker does NOT project charging.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 161,
        values: vec![protocol_bool_data(charging_data_id, true)],
    }));
    assert!(!charging(&world, 161));
}

#[test]
fn entity_model_instances_project_pillager_crossbow_hold_from_either_hand() {
    // Vanilla `Pillager.getArmPose` uses `isHolding(Items.CROSSBOW)`, and `LivingEntity.isHolding`
    // checks both hands before the aggressive ATTACKING fallback.
    const CROSSBOW_ID: i32 = 0;

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
    let equip = |entity_id: i32, slot: EquipmentSlot| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot,
            item: ItemStackSummary {
                item_id: Some(CROSSBOW_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }],
    };
    let holds_crossbow = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, Some(&runtime), 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .illager()
            .holds_crossbow
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        162,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        163,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        [2.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        164,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        [3.0, 64.0, -2.0],
    ));

    assert!(!holds_crossbow(&world, 162));
    assert!(world.apply_set_equipment(equip(162, EquipmentSlot::MainHand)));
    assert!(holds_crossbow(&world, 162));

    assert!(world.apply_set_equipment(equip(163, EquipmentSlot::OffHand)));
    assert!(holds_crossbow(&world, 163));

    assert!(world.apply_set_equipment(equip(164, EquipmentSlot::OffHand)));
    assert!(!holds_crossbow(&world, 164));
}

#[test]
fn illager_celebrating_matches_generated_accessor_and_arm_pose_catalogs() {
    // Raider.IS_CELEBRATING supplies the instance flag; the nearest getArmPose implementation
    // decides whether that flag can reach CELEBRATING. Compare their intersection over every type.
    for entry in VANILLA_ENTITY_TYPES {
        let accessor =
            bbb_client_world::vanilla_entity_metadata_raider_celebrating_accessor_for_entity_type(
                entry.id,
            );
        let values = accessor
            .map(|(data_id, default)| {
                assert!(!default, "{} celebrating default", entry.resource_id);
                vec![protocol_bool_data(data_id, true)]
            })
            .unwrap_or_default();
        let expected = accessor.is_some()
            && bbb_client_world::vanilla_entity_type_illager_arm_pose_uses_celebrating(entry.id);
        assert_eq!(
            illager_celebrating(entry.id, &values),
            expected,
            "{}",
            entry.resource_id
        );
    }
    assert!(!illager_celebrating(-1, &[]));
}

#[test]
fn entity_model_instances_project_illager_main_hand_empty() {
    // Vanilla `IllagerModel.setupAnim` ATTACKING chooses empty-hand zombie arms vs armed weapon
    // swing from `state.getMainHandItemState().isEmpty()`. Native projects that from canonical
    // equipment so renderer can choose the right branch.
    const PLAIN_ITEM_ID: i32 = 701;

    let main_hand = |entity_id: i32, item_id: Option<i32>, count: i32| SetEquipment {
        entity_id,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id,
                count,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }],
    };
    let projected = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .illager()
            .main_hand_empty
    };

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        173,
        VANILLA_ENTITY_TYPE_VINDICATOR_ID,
        [1.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        174,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        [2.0, 64.0, -2.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        175,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [3.0, 64.0, -2.0],
    ));

    assert!(projected(&world, 173));
    assert!(projected(&world, 174));
    assert!(!projected(&world, 175));

    assert!(world.apply_set_equipment(main_hand(173, Some(PLAIN_ITEM_ID), 1)));
    assert!(!projected(&world, 173));

    assert!(world.apply_set_equipment(main_hand(173, None, 0)));
    assert!(projected(&world, 173));
}

#[test]
fn entity_model_instances_project_piglin_dancing() {
    // Vanilla Piglin.isDancing() (BOOLEAN data id 19) and Piglin.getArmPose → DANCING: a regular
    // piglin dances by a soul campfire. The brute and zombified piglin never return DANCING, so the
    // projection is gated to the regular piglin type.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        180,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [1.0, 64.0, -3.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        181,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        [2.0, 64.0, -3.0],
    ));

    let dancing = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .piglin()
            .dancing
    };

    // No flag → not dancing.
    assert!(!dancing(&world, 180));

    // Piglin.DATA_IS_DANCING projects the dance for the regular piglin.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 180,
        values: vec![protocol_bool_data(piglin_dancing_data_id(), true)],
    }));
    assert!(dancing(&world, 180));

    // The same encoded value on a piglin brute does NOT project dancing because the generated
    // accessor is not defined on it.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 181,
        values: vec![protocol_bool_data(piglin_dancing_data_id(), true)],
    }));
    assert!(!dancing(&world, 181));
}

#[test]
fn piglin_is_charging_crossbow_is_gated_to_the_regular_piglin() {
    // Vanilla Piglin.DATA_IS_CHARGING_CROSSBOW (BOOLEAN): getArmPose returns CROSSBOW_CHARGE
    // (the pull-back draw) while true, suppressing CROSSBOW_HOLD. Only the regular piglin defines the
    // accessor, so the projection is type-gated.
    let (charging_data_id, charging_default) =
        bbb_client_world::vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
        )
        .expect("generated Piglin charging accessor");
    assert!(!charging_default);
    let charging = vec![protocol_bool_data(charging_data_id, true)];
    assert!(piglin_is_charging_crossbow(
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        &charging
    ));
    assert!(!piglin_is_charging_crossbow(
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        &charging
    ));
    assert!(!piglin_is_charging_crossbow(
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        &[]
    ));
}

#[test]
fn entity_model_instances_project_piglin_crossbow_charge() {
    // Vanilla Piglin.getArmPose → CROSSBOW_CHARGE while isChargingCrossbow(): the
    // regular piglin draws its crossbow. Unlike CROSSBOW_HOLD this needs no held-item resolution (the
    // flag alone drives it), so it projects without an item runtime. Gated to the regular piglin: the
    // brute never charges because the generated accessor is not defined on it.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        190,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [1.0, 64.0, -4.0],
    ));
    world.apply_add_entity(protocol_add_entity(
        191,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        [2.0, 64.0, -4.0],
    ));
    let (charging_data_id, charging_default) =
        bbb_client_world::vanilla_entity_metadata_piglin_charging_crossbow_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_PIGLIN_ID,
        )
        .expect("generated Piglin charging accessor");
    assert!(!charging_default);

    let charging = |world: &WorldStore, id: i32| {
        entity_model_instances_from_world_at_partial_tick(world, None, 0.0)
            .into_iter()
            .find(|instance| instance.entity_id == id)
            .unwrap()
            .render_state
            .kind
            .piglin()
            .crossbow_charge
    };

    // No flag → not drawing.
    assert!(!charging(&world, 190));

    // Piglin.DATA_IS_CHARGING_CROSSBOW (id 18) projects the draw for the regular piglin — no item
    // runtime needed.
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 190,
        values: vec![protocol_bool_data(charging_data_id, true)],
    }));
    assert!(charging(&world, 190));

    // The same data id on a piglin brute does NOT project a draw.
    assert!(!world.apply_set_entity_data(SetEntityData {
        id: 191,
        values: vec![protocol_bool_data(charging_data_id, true)],
    }));
    assert!(!charging(&world, 191));
}

#[test]
fn entity_model_instances_piglin_crossbow_hold_needs_a_resolved_held_item() {
    // The CROSSBOW_HOLD pose needs the held item resolved through the item registry to confirm a
    // charged crossbow; without an item runtime it can never level, so the projection defaults off.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        182,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        [3.0, 64.0, -3.0],
    ));
    let crossbow_hold = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 182)
        .unwrap()
        .render_state
        .kind
        .piglin()
        .crossbow_hold;
    assert!(!crossbow_hold);
}

#[test]
fn entity_model_instances_drowned_throw_trident_needs_a_resolved_held_item() {
    // The THROW_TRIDENT pose needs the held item resolved through the item registry to confirm a
    // trident; without an item runtime it can never raise the trident, so the projection defaults off
    // even for an aggressive drowned.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        230,
        VANILLA_ENTITY_TYPE_DROWNED_ID,
        [3.0, 64.0, -8.0],
    ));
    // Aggressive (so only the missing item runtime, not the flag, gates the pose off).
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 230,
        values: vec![mob_aggressive_data(VANILLA_ENTITY_TYPE_DROWNED_ID)],
    }));
    let throwing = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 230)
        .unwrap()
        .render_state
        .kind
        .drowned()
        .throw_trident;
    assert!(!throwing);
}

#[test]
fn entity_model_instances_project_witch_item_and_potion_state() {
    // `WitchRenderState.isHoldingItem` only needs a non-empty main hand, so it projects without the
    // item runtime. `isHoldingPotion` needs registry resolution (`Items.POTION`), so it stays false
    // without the runtime.
    const POTION_ID: i32 = 0;

    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        244,
        VANILLA_ENTITY_TYPE_WITCH_ID,
        [3.0, 64.0, -4.0],
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 244,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::MainHand,
            item: ItemStackSummary {
                item_id: Some(POTION_ID),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }],
    }));

    let state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 244)
        .unwrap()
        .render_state;
    assert!(state.kind.witch().holding_item);
    assert!(!state.kind.witch().holding_potion);

    let registry: bbb_client_assets::ItemRegistryCatalog =
        serde_json::from_value(serde_json::json!({
            "resource_ids": ["minecraft:potion"],
            "protocol_ids": { "minecraft:potion": POTION_ID }
        }))
        .unwrap();
    let runtime = NativeItemRuntime::for_test_with_registry_and_equipment_assets(
        registry,
        bbb_client_assets::EquipmentAssetCatalog::default(),
    );
    let state = entity_model_instances_from_world_at_partial_tick(&world, Some(&runtime), 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 244)
        .unwrap()
        .render_state;
    assert!(state.kind.witch().holding_item);
    assert!(state.kind.witch().holding_potion);
}

#[test]
fn entity_model_instances_project_copper_golem_holding_item_from_either_hand() {
    // `CopperGolemModel.setupAnim` checks both rendered hand item states before clamping the arms into
    // the held-item pose, so an off-hand-only item is enough.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        245,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, -3.0],
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 245,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::OffHand,
            item: ItemStackSummary {
                item_id: Some(6),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }],
    }));

    let state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 245)
        .unwrap()
        .render_state;
    assert!(state.kind.copper_golem().holding_item);
}

#[test]
fn entity_model_instances_project_copper_golem_idle_seconds_from_world_timer() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        247,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, -2.0],
    ));

    let idle_seconds = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 247)
            .unwrap()
            .render_state
            .kind
            .copper_golem()
            .idle_seconds
    };

    assert_eq!(idle_seconds(&world), -1.0);
    world.advance_entity_client_animations(240);
    let after_timeout = idle_seconds(&world);
    assert!(
        after_timeout >= 0.0,
        "native projection carries the world copper golem idle timer: {after_timeout}"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 247,
        values: vec![protocol_copper_golem_state_data(
            COPPER_GOLEM_STATE_GETTING_ITEM_ID
        )],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(idle_seconds(&world), -1.0);

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 247,
        values: vec![protocol_copper_golem_state_data(COPPER_GOLEM_STATE_IDLE_ID)],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(idle_seconds(&world), -1.0);
}

#[test]
fn entity_model_instances_project_copper_golem_get_item_seconds_from_world_timer() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        248,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, -1.0],
    ));

    let get_item_seconds = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 248)
            .unwrap()
            .render_state
            .kind
            .copper_golem()
            .get_item_seconds
    };

    assert_eq!(get_item_seconds(&world), -1.0);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 248,
        values: vec![protocol_copper_golem_state_data(
            COPPER_GOLEM_STATE_GETTING_ITEM_ID
        )],
    }));
    world.advance_entity_client_animations(1);
    let after_one = get_item_seconds(&world);
    assert!(
        after_one >= 0.0,
        "native projection carries the copper golem GETTING_ITEM interaction timer: {after_one}"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 248,
        values: vec![protocol_copper_golem_state_data(COPPER_GOLEM_STATE_IDLE_ID)],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(get_item_seconds(&world), -1.0);
}

#[test]
fn entity_model_instances_project_copper_golem_get_no_item_seconds_from_world_timer() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        249,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, 0.0],
    ));

    let get_no_item_seconds = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 249)
            .unwrap()
            .render_state
            .kind
            .copper_golem()
            .get_no_item_seconds
    };

    assert_eq!(get_no_item_seconds(&world), -1.0);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 249,
        values: vec![protocol_copper_golem_state_data(
            COPPER_GOLEM_STATE_GETTING_NO_ITEM_ID
        )],
    }));
    world.advance_entity_client_animations(1);
    let after_one = get_no_item_seconds(&world);
    assert!(
        after_one >= 0.0,
        "native projection carries the copper golem GETTING_NO_ITEM interaction timer: {after_one}"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 249,
        values: vec![protocol_copper_golem_state_data(COPPER_GOLEM_STATE_IDLE_ID)],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(get_no_item_seconds(&world), -1.0);
}

#[test]
fn entity_model_instances_project_copper_golem_drop_item_seconds_from_world_timer() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        250,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, 1.0],
    ));

    let drop_item_seconds = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 250)
            .unwrap()
            .render_state
            .kind
            .copper_golem()
            .drop_item_seconds
    };

    assert_eq!(drop_item_seconds(&world), -1.0);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 250,
        values: vec![protocol_copper_golem_state_data(
            COPPER_GOLEM_STATE_DROPPING_ITEM_ID
        )],
    }));
    world.advance_entity_client_animations(1);
    let after_one = drop_item_seconds(&world);
    assert!(
        after_one >= 0.0,
        "native projection carries the copper golem DROPPING_ITEM interaction timer: {after_one}"
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 250,
        values: vec![protocol_copper_golem_state_data(COPPER_GOLEM_STATE_IDLE_ID)],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(drop_item_seconds(&world), -1.0);
}

#[test]
fn entity_model_instances_project_copper_golem_drop_no_item_seconds_from_world_timer() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        251,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        [3.0, 64.0, 2.0],
    ));

    let drop_no_item_seconds = |world: &WorldStore| {
        entity_model_instances_from_world_at_partial_tick(world, None, 1.0)
            .into_iter()
            .find(|instance| instance.entity_id == 251)
            .unwrap()
            .render_state
            .kind
            .copper_golem()
            .drop_no_item_seconds
    };

    assert_eq!(drop_no_item_seconds(&world), -1.0);
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 251,
        values: vec![protocol_copper_golem_state_data(
            COPPER_GOLEM_STATE_DROPPING_NO_ITEM_ID
        )],
    }));
    world.advance_entity_client_animations(1);
    let after_one = drop_no_item_seconds(&world);
    assert!(
            after_one >= 0.0,
            "native projection carries the copper golem DROPPING_NO_ITEM interaction timer: {after_one}"
        );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 251,
        values: vec![protocol_copper_golem_state_data(COPPER_GOLEM_STATE_IDLE_ID)],
    }));
    world.advance_entity_client_animations(1);
    assert_eq!(drop_no_item_seconds(&world), -1.0);
}

#[test]
fn entity_model_instances_custom_head_skull_needs_item_runtime() {
    // `LivingEntityRenderer.extractRenderState` needs the HEAD stack resolved as an AbstractSkullBlock.
    // Without the item registry runtime, bbb must not guess from the protocol id.
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        246,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        [3.0, 64.0, -2.0],
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 246,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Head,
            item: ItemStackSummary {
                item_id: Some(7),
                count: 1,
                component_patch: DataComponentPatchSummary::default().into(),
            },
        }],
    }));

    let state = entity_model_instances_from_world_at_partial_tick(&world, None, 0.0)
        .into_iter()
        .find(|instance| instance.entity_id == 246)
        .unwrap()
        .render_state;
    assert_eq!(state.custom_head_skull, None);
}
