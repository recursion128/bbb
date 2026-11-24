use super::*;

#[test]
fn falling_block_and_tnt_apply_gravity_drag_and_client_fuse_tick() {
    let mut store = WorldStore::new();
    for (id, entity_type_id) in [
        (59, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID),
        (60, VANILLA_ENTITY_TYPE_TNT_ID),
    ] {
        store.apply_add_entity(projectile(
            id,
            entity_type_id,
            ProtocolVec3d {
                x: 0.5,
                y: 0.2,
                z: -0.25,
            },
        ));
    }
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let requested = EntityVec3 {
        x: 0.5,
        y: 0.16,
        z: -0.25,
    };
    for id in [59, 60] {
        let transform = store.probe_entity_transform(id).unwrap();
        assert_vec3_near(
            transform.position,
            EntityVec3 {
                x: 1.5,
                y: 64.16,
                z: -2.25,
            },
        );
        assert_vec3_near(
            transform.delta_movement,
            scale(requested, f64::from(0.98_f32)),
        );
        assert_eq!(transform.on_ground, Some(false));
    }
    assert_eq!(
        store.primed_tnt_fuse_remaining_in_ticks(60, 1.0),
        Some(79.0)
    );
    assert_eq!(
        store.primed_tnt_smoke_particle_states(),
        vec![super::super::super::PrimedTntSmokeParticleState {
            entity_id: 60,
            position: EntityVec3 {
                x: 1.5,
                y: 64.16,
                z: -2.25,
            },
        }]
    );
    assert_eq!(
        generation_a.primed_tnt_fuse_remaining_in_ticks(60, 1.0),
        Some(80.0),
        "retained publication keeps the pre-tick fuse"
    );
}

#[test]
fn item_entity_applies_client_gravity_drag_and_retained_generation_isolation() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        63,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        ProtocolVec3d {
            x: 0.5,
            y: 0.2,
            z: -0.25,
        },
    ));
    set_nonempty_item_stack(&mut store, 63);
    assert_eq!(
        store
            .entities
            .client_movement_fluid_inputs(&HashSet::new())
            .len(),
        1
    );
    assert_eq!(
        store
            .entities
            .client_movement_collision_inputs(
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &HashSet::new(),
                0.0023333333333333335,
                0,
            )
            .len(),
        1
    );
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let requested = EntityVec3 {
        x: 0.5,
        y: 0.16,
        z: -0.25,
    };
    let transform = store.probe_entity_transform(63).unwrap();
    assert_vec3_near(
        transform.position,
        EntityVec3 {
            x: 1.5,
            y: 64.16,
            z: -2.25,
        },
    );
    assert_vec3_near(
        transform.delta_movement,
        EntityVec3 {
            x: requested.x * f64::from(0.98_f32),
            y: requested.y * f64::from(0.98_f32),
            z: requested.z * f64::from(0.98_f32),
        },
    );
    assert_eq!(
        generation_a.probe_entity_transform(63).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        "the retained item generation keeps its pre-tick transform"
    );
}

#[test]
fn experience_orb_skips_first_client_tick_then_follows_nearby_player() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        64,
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        ProtocolVec3d::default(),
    ));
    let mut player = projectile(65, VANILLA_ENTITY_TYPE_PLAYER_ID, ProtocolVec3d::default());
    player.position.x = 5.0;
    store.apply_add_entity(player);

    store.advance_entity_client_animations(1);
    assert_eq!(
        store.probe_entity_transform(64).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        "Vanilla clears firstTick without running ExperienceOrb base motion"
    );
    assert_eq!(
        store
            .entities
            .client_movement_collision_inputs(
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &HashSet::new(),
                0.0023333333333333335,
                0,
            )
            .len(),
        1,
        "the second orb tick must enter collision/world-fact extraction"
    );
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let attraction = EntityVec3 {
        x: 4.0,
        y: f64::from(1.62_f32) * 0.5,
        z: 0.0,
    };
    let power = 1.0 - length_squared(attraction).sqrt() / 8.0;
    let requested = add(
        EntityVec3 {
            x: 0.0,
            y: -0.03,
            z: 0.0,
        },
        scale(normalize(attraction), power * power * 0.1),
    );
    let transform = store.probe_entity_transform(64).unwrap();
    assert_vec3_near(
        transform.position,
        add(
            generation_a.probe_entity_transform(64).unwrap().position,
            requested,
        ),
    );
    assert_vec3_near(
        transform.delta_movement,
        scale(requested, f64::from(0.98_f32)),
    );
    assert_eq!(
        generation_a.probe_entity_transform(64).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        "the retained orb generation remains isolated from follow motion"
    );
    assert_eq!(
        generation_a
            .entities
            .experience_orb_movement(64)
            .unwrap()
            .following_player_uuid,
        None
    );
    assert_eq!(
        store
            .entities
            .experience_orb_movement(64)
            .unwrap()
            .following_player_uuid,
        Some(Uuid::from_u128(65)),
        "the live generation alone retains the selected Player instance"
    );
}

#[test]
fn experience_orb_target_selection_preserves_vanilla_retention_and_range_rules() {
    let origin = EntityVec3::default();
    let dead_current = EntityExperienceOrbPlayerTarget {
        entity_id: 1,
        uuid: Uuid::from_u128(1),
        position: EntityVec3 {
            x: 8.0,
            y: 0.0,
            z: 0.0,
        },
        attraction_position: EntityVec3::default(),
        is_dead_or_dying: true,
    };
    let first_nearest = EntityExperienceOrbPlayerTarget {
        entity_id: 2,
        uuid: Uuid::from_u128(2),
        position: EntityVec3 {
            x: 3.0,
            y: 0.0,
            z: 0.0,
        },
        attraction_position: EntityVec3::default(),
        is_dead_or_dying: false,
    };
    let equal_nearest = EntityExperienceOrbPlayerTarget {
        entity_id: 3,
        uuid: Uuid::from_u128(3),
        ..first_nearest
    };

    assert_eq!(
        select_experience_orb_player_target(
            origin,
            Some(dead_current.uuid),
            &[dead_current, first_nearest],
        ),
        Some(dead_current),
        "an existing non-spectator target remains retained at exactly 8 blocks, even if dead"
    );
    assert_eq!(
        select_experience_orb_player_target(
            origin,
            None,
            &[dead_current, first_nearest, equal_nearest],
        ),
        Some(first_nearest),
        "a new target excludes dead/exact-range players and preserves first-player ties"
    );
}

#[test]
fn item_and_orb_collision_responses_use_block_friction_and_vertical_bounce() {
    let collision = EntityClientMovementCollisionResult {
        pre_move_delta: EntityVec3 {
            x: 0.25,
            y: -0.1,
            z: -0.5,
        },
        requested_movement: EntityVec3 {
            x: 0.25,
            y: -0.1,
            z: -0.5,
        },
        clipped_movement: EntityVec3 {
            x: 0.25,
            y: 0.0,
            z: -0.5,
        },
        supporting_block: Some(BlockPos { x: 1, y: 63, z: -2 }),
        ground_friction: 0.98,
        movement_skipped: false,
        experience_orb_update: None,
        fishing_hook_update: None,
        fishing_hook_reverse_deflection: None,
    };
    let mut support = EntitySupportingBlock::default();
    let mut item = test_transform(EntityVec3::default());
    advance_item_entity(&mut item, &mut support, collision);
    assert_eq!(item.on_ground, Some(true));
    assert_eq!(support.main, collision.supporting_block);
    assert_vec3_near(
        item.delta_movement,
        EntityVec3 {
            x: 0.25 * f64::from(0.98_f32 * 0.98_f32),
            y: -0.1 * f64::from(0.98_f32) * -0.5,
            z: -0.5 * f64::from(0.98_f32 * 0.98_f32),
        },
    );

    let mut orb_movement = experience_orb_movement_from_uuid(Uuid::from_u128(4));
    let mut orb_support = EntitySupportingBlock::default();
    let mut orb = test_transform(EntityVec3::default());
    advance_experience_orb(&mut orb_movement, &mut orb, &mut orb_support, collision);
    assert_eq!(orb.on_ground, Some(true));
    assert_vec3_near(
        orb.delta_movement,
        EntityVec3 {
            x: 0.25 * f64::from(0.98_f32 * 0.98_f32),
            y: 0.04,
            z: -0.5 * f64::from(0.98_f32 * 0.98_f32),
        },
    );
}

#[test]
fn same_id_readd_regenerates_exact_world_fact_family_markers() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        66,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        ProtocolVec3d::default(),
    ));
    assert_eq!(
        store
            .entities
            .client_movement_fluid_inputs(&HashSet::new())
            .len(),
        1
    );

    store.apply_add_entity(projectile(
        66,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        ProtocolVec3d::default(),
    ));
    assert!(store
        .entities
        .client_movement_fluid_inputs(&HashSet::new())
        .is_empty());
    assert!(store
        .entities
        .client_movement_collision_inputs(
            &HashMap::new(),
            &HashMap::new(),
            &HashMap::new(),
            &HashSet::new(),
            0.0023333333333333335,
            0,
        )
        .is_empty());

    store.apply_add_entity(projectile(
        66,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        ProtocolVec3d::default(),
    ));
    assert_eq!(
        store
            .entities
            .experience_orb_player_targets(&HashSet::new())
            .len(),
        1
    );
}

#[test]
fn item_water_and_experience_orb_lava_inputs_match_vanilla_float_paths() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        68,
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        ProtocolVec3d::default(),
    ));
    // Vanilla's first client orb tick only clears firstTick.
    store.advance_entity_client_animations(1);

    store.apply_add_entity(projectile(
        67,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        ProtocolVec3d {
            x: 0.5,
            y: 0.2,
            z: -0.25,
        },
    ));
    set_nonempty_item_stack(&mut store, 67);

    let mut fluid_by_id = HashMap::new();
    fluid_by_id.insert(
        67,
        crate::client::EntityFluidContactState {
            water_height: 0.2,
            ..crate::client::EntityFluidContactState::default()
        },
    );
    fluid_by_id.insert(
        68,
        crate::client::EntityFluidContactState {
            block_fluid_kind: Some(crate::TerrainFluidKind::Lava),
            ..crate::client::EntityFluidContactState::default()
        },
    );
    let inputs = store.entities.client_movement_collision_inputs(
        &fluid_by_id,
        &HashMap::new(),
        &HashMap::new(),
        &HashSet::new(),
        0.0023333333333333335,
        0,
    );
    let item = inputs.iter().find(|input| input.entity_id == 67).unwrap();
    assert_vec3_near(
        item.pre_move_delta,
        EntityVec3 {
            x: 0.5 * f64::from(0.99_f32),
            y: 0.2,
            z: -0.25 * f64::from(0.99_f32),
        },
    );

    let mut expected_seed = experience_orb_movement_from_uuid(Uuid::from_u128(68)).random_seed;
    let expected_orb = EntityVec3 {
        x: f64::from(
            (java_random_next_float(&mut expected_seed)
                - java_random_next_float(&mut expected_seed))
                * 0.2,
        ),
        y: f64::from(0.2_f32),
        z: f64::from(
            (java_random_next_float(&mut expected_seed)
                - java_random_next_float(&mut expected_seed))
                * 0.2,
        ),
    };
    let orb = inputs.iter().find(|input| input.entity_id == 68).unwrap();
    assert_eq!(orb.pre_move_delta, expected_orb);
    assert_eq!(
        orb.experience_orb_update.unwrap().random_seed,
        expected_seed
    );
}

#[test]
fn item_world_collision_uses_generated_ice_friction_only_for_ground_motion() {
    let mut store = nonliving_collision_world();
    let ice = crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:ice", &Default::default())
        .expect("vanilla ice block state")
        .id;
    assert!(
        store.apply_block_update(bbb_protocol::packets::BlockUpdate {
            pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
            block_state_id: ice,
        })
    );
    let mut item = projectile(
        69,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        ProtocolVec3d {
            x: 0.25,
            y: -0.1,
            z: 0.0,
        },
    );
    item.position = ProtocolVec3d {
        x: 0.5,
        y: 1.05,
        z: 0.5,
    };
    store.apply_add_entity(item);
    set_nonempty_item_stack(&mut store, 69);

    store.advance_entity_client_animations(1);

    let transform = store.probe_entity_transform(69).unwrap();
    assert_eq!(transform.on_ground, Some(true));
    assert!((transform.position.y - 1.0).abs() <= 1.0e-9);
    assert_vec3_near(
        transform.delta_movement,
        EntityVec3 {
            x: 0.25 * f64::from(0.98_f32 * 0.98_f32),
            y: -0.14 * f64::from(0.98_f32) * -0.5,
            z: 0.0,
        },
    );
}

#[test]
fn settled_item_preserves_vanilla_four_tick_movement_skip_schedule() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        70,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        ProtocolVec3d::default(),
    ));
    set_nonempty_item_stack(&mut store, 70);
    store
        .entities
        .with_transform_mut(70, |transform| transform.on_ground = Some(true));

    store.advance_entity_client_animations(1);

    let first = store.probe_entity_transform(70).unwrap();
    assert_eq!(
        first.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(
        first.delta_movement,
        EntityVec3 {
            x: 0.0,
            y: -0.04,
            z: 0.0,
        },
        "the skipped tick retains gravity in deltaMovement without moving"
    );

    store.advance_entity_client_animations(1);
    assert!(store.probe_entity_transform(70).unwrap().position.y < 64.0);
}

#[test]
fn experience_orb_no_collision_gate_includes_collidable_entities() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        71,
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        ProtocolVec3d::default(),
    ));
    store.apply_add_entity(projectile(
        72,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        ProtocolVec3d::default(),
    ));

    store.advance_entity_client_animations(1);
    store.advance_entity_client_animations(1);

    assert_eq!(
        store.probe_entity_transform(71).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        "an orb intersecting a collidable entity does not apply air gravity"
    );
}

#[test]
fn nonliving_collision_response_matches_falling_and_tnt_ground_branches() {
    let metadata = EntityMetadata::default();
    let requested = EntityVec3 {
        x: 0.5,
        y: -0.04,
        z: -0.25,
    };
    let collision = Some(EntityClientMovementCollisionResult {
        pre_move_delta: requested,
        requested_movement: requested,
        clipped_movement: EntityVec3 {
            x: 0.5,
            y: 0.0,
            z: -0.25,
        },
        supporting_block: None,
        ground_friction: 0.6,
        movement_skipped: false,
        experience_orb_update: None,
        fishing_hook_update: None,
        fishing_hook_reverse_deflection: None,
    });

    let mut falling = test_transform(EntityVec3 {
        x: 0.5,
        y: 0.0,
        z: -0.25,
    });
    advance_falling_block(&metadata, &mut falling, collision);
    assert_eq!(falling.on_ground, Some(true));
    assert_vec3_near(
        falling.position,
        EntityVec3 {
            x: 1.5,
            y: 64.0,
            z: -2.25,
        },
    );
    assert_vec3_near(
        falling.delta_movement,
        scale(requested, f64::from(0.98_f32)),
    );

    let mut tnt_metadata = EntityMetadata::default();
    let mut tnt = test_transform(EntityVec3 {
        x: 0.5,
        y: 0.0,
        z: -0.25,
    });
    let accessors =
        vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(VANILLA_ENTITY_TYPE_TNT_ID)
            .unwrap();
    advance_primed_tnt(
        &mut tnt_metadata,
        &mut tnt,
        accessors.fuse_data_id,
        accessors.fuse_default,
        collision,
    );
    let drag = f64::from(0.98_f32);
    assert_eq!(tnt.on_ground, Some(true));
    assert_vec3_near(
        tnt.delta_movement,
        EntityVec3 {
            x: requested.x * drag * 0.7,
            y: requested.y * drag * -0.5,
            z: requested.z * drag * 0.7,
        },
    );
    assert_eq!(
        entity_data_int(
            &tnt_metadata.data_values,
            accessors.fuse_data_id,
            accessors.fuse_default,
        ),
        79
    );
}

#[test]
fn nonliving_world_collision_inputs_clip_the_canonical_tick() {
    let mut store = nonliving_collision_world();
    for (id, entity_type_id) in [
        (61, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID),
        (62, VANILLA_ENTITY_TYPE_TNT_ID),
    ] {
        let mut entity = projectile(
            id,
            entity_type_id,
            ProtocolVec3d {
                x: 0.0,
                y: -0.1,
                z: 0.0,
            },
        );
        entity.position = ProtocolVec3d {
            x: 0.5,
            y: 1.05,
            z: 0.5,
        };
        store.apply_add_entity(entity);
    }

    store.advance_entity_client_animations(1);

    let requested_y = -0.14;
    let falling = store.probe_entity_transform(61).unwrap();
    assert_eq!(falling.on_ground, Some(true));
    assert!((falling.position.y - 1.0).abs() <= 1.0e-9);
    assert!((falling.delta_movement.y - requested_y * f64::from(0.98_f32)).abs() <= 1.0e-12);
    let tnt = store.probe_entity_transform(62).unwrap();
    assert_eq!(tnt.on_ground, Some(true));
    assert!((tnt.position.y - 1.0).abs() <= 1.0e-9);
    assert!((tnt.delta_movement.y - requested_y * f64::from(0.98_f32) * -0.5).abs() <= 1.0e-12);
}

#[test]
fn llama_spit_and_unattached_firework_apply_subclass_free_flight() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        60,
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID,
        ProtocolVec3d {
            x: 0.4,
            y: 0.2,
            z: -0.6,
        },
    ));
    store.apply_add_entity(projectile(
        61,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        ProtocolVec3d {
            x: 0.2,
            y: 0.8,
            z: -0.4,
        },
    ));
    store.apply_add_entity(projectile(
        62,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        ProtocolVec3d {
            x: 0.2,
            y: 0.8,
            z: -0.4,
        },
    ));
    let firework_accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 62,
        values: vec![EntityDataValue {
            data_id: firework_accessors.shot_at_angle_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));

    store.advance_entity_client_animations(1);

    let llama_inertia = f64::from(0.99_f32);
    assert_vec3_near(
        store.probe_entity_transform(60).unwrap().position,
        EntityVec3 {
            x: 1.4,
            y: 64.2,
            z: -2.6,
        },
    );
    assert_vec3_near(
        store.probe_entity_transform(60).unwrap().delta_movement,
        EntityVec3 {
            x: 0.4 * llama_inertia,
            y: 0.2 * llama_inertia - 0.06,
            z: -0.6 * llama_inertia,
        },
    );
    assert_vec3_near(
        store.probe_entity_transform(61).unwrap().position,
        EntityVec3 {
            x: 1.23,
            y: 64.84,
            z: -2.46,
        },
    );
    assert_vec3_near(
        store.probe_entity_transform(62).unwrap().position,
        EntityVec3 {
            x: 1.2,
            y: 64.8,
            z: -2.4,
        },
    );
    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert!(matches!(
        effects.as_slice(),
        [
            EntityClientMovementEffectState::Particle(
                EntityClientMovementParticleState::FireworkRocket(
                    FireworkRocketTrailParticleState { entity_id: 61, .. }
                )
            ),
            EntityClientMovementEffectState::Particle(
                EntityClientMovementParticleState::FireworkRocket(
                    FireworkRocketTrailParticleState { entity_id: 62, .. }
                )
            )
        ]
    ));

    store.advance_entity_client_animations(1);
    let second_tick_effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(second_tick_effects.len(), 2);
    assert!(second_tick_effects
        .iter()
        .all(|effect| matches!(effect, EntityClientMovementEffectState::Particle(_))));
}

#[test]
fn attached_firework_uses_stable_target_lookup_and_boosts_fall_flying_target() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        70,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        ProtocolVec3d {
            x: 0.2,
            y: 0.1,
            z: -0.3,
        },
    ));
    store.apply_add_entity(projectile(
        71,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        ProtocolVec3d::default(),
    ));
    let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    let firework_accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![EntityDataValue {
            data_id: entity_accessors.shared_flags_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(entity_accessors.fall_flying_mask),
        }],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 71,
        values: vec![EntityDataValue {
            data_id: firework_accessors.attached_to_target_data_id,
            serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
            value: EntityDataValueKind::OptionalUnsignedInt(Some(70)),
        }],
    }));
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let expected_movement = EntityVec3 {
        x: 0.1,
        y: 0.05,
        z: 0.7,
    };
    assert_vec3_near(
        store.probe_entity_transform(70).unwrap().delta_movement,
        expected_movement,
    );
    let rocket = store.probe_entity_transform(71).unwrap();
    assert_vec3_near(
        rocket.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
    );
    assert_vec3_near(rocket.delta_movement, expected_movement);
    assert_vec3_near(
        generation_a
            .probe_entity_transform(70)
            .unwrap()
            .delta_movement,
        EntityVec3 {
            x: 0.2,
            y: 0.1,
            z: -0.3,
        },
    );
    assert_eq!(
        store
            .drain_entity_client_movement_effect_states()
            .filter_map(|state| match state {
                EntityClientMovementEffectState::Particle(
                    EntityClientMovementParticleState::FireworkRocket(state),
                ) => Some(state.entity_id),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![71]
    );
}
