use super::*;

#[test]
fn abstract_arrows_honor_synced_ground_physics_and_crit_flags() {
    let mut store = WorldStore::new();
    for (id, entity_type_id) in [
        (54, VANILLA_ENTITY_TYPE_ARROW_ID),
        (55, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID),
        (56, VANILLA_ENTITY_TYPE_TRIDENT_ID),
    ] {
        store.apply_add_entity(projectile(
            id,
            entity_type_id,
            ProtocolVec3d {
                x: 1.0,
                y: 0.5,
                z: 0.0,
            },
        ));
    }
    let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARROW_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 54,
        values: vec![EntityDataValue {
            data_id: accessors.flags_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(accessors.crit_mask),
        }],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![EntityDataValue {
            data_id: accessors.in_ground_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 56,
        values: vec![EntityDataValue {
            data_id: accessors.flags_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(accessors.no_physics_mask),
        }],
    }));
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let inertia = f64::from(0.99_f32);
    let flying = store.probe_entity_transform(54).unwrap();
    assert_vec3_near(
        flying.position,
        EntityVec3 {
            x: 2.0,
            y: 64.5,
            z: -2.0,
        },
    );
    assert_vec3_near(
        flying.delta_movement,
        EntityVec3 {
            x: inertia,
            y: 0.5 * inertia - 0.05,
            z: 0.0,
        },
    );
    assert_eq!(flying.y_rot, 18.0);
    assert_eq!(
        store.probe_entity_transform(55).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    let no_physics = store.probe_entity_transform(56).unwrap();
    assert_vec3_near(
        no_physics.delta_movement,
        EntityVec3 {
            x: inertia,
            y: 0.5 * inertia,
            z: 0.0,
        },
    );
    assert_eq!(no_physics.y_rot, -18.0);

    let effects = store
        .drain_entity_client_movement_effect_states()
        .collect::<Vec<_>>();
    assert_eq!(effects.len(), 4);
    for (index, effect) in effects.into_iter().enumerate() {
        assert_eq!(
            effect,
            particle_effect(EntityClientMovementParticleState::Simple(
                EntityClientMovementSimpleParticleState {
                    entity_id: 54,
                    kind: EntityClientMovementSimpleParticleKind::Crit,
                    position: EntityVec3 {
                        x: 1.0 + index as f64 / 4.0,
                        y: 64.0 + 0.5 * index as f64 / 4.0,
                        z: -2.0,
                    },
                    velocity: EntityVec3 {
                        x: -1.0,
                        y: -0.3,
                        z: 0.0,
                    },
                },
            ))
        );
    }
    assert_eq!(
        generation_a.probe_entity_transform(54).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
        "retained publication remains isolated from arrow movement"
    );
}

#[test]
fn abstract_arrow_water_inertia_precedes_bubbles_and_free_flight() {
    for (entity_id, entity_type_id, inertia) in [
        (57, VANILLA_ENTITY_TYPE_ARROW_ID, 0.6_f32),
        (58, VANILLA_ENTITY_TYPE_TRIDENT_ID, 0.99_f32),
    ] {
        let mut metadata = EntityMetadata::default();
        let mut animations = EntityClientAnimationState::default();
        let mut transform = test_transform(EntityVec3 {
            x: 0.5,
            y: -0.25,
            z: 1.0,
        });
        let mut effects = Vec::new();
        let accessors =
            vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(entity_type_id)
                .unwrap();

        let step = advance_abstract_arrow(
            entity_id,
            entity_type_id,
            &mut metadata,
            &mut animations,
            &mut transform,
            accessors,
            true,
            None,
            &mut effects,
        );
        let segment = step.first_effect;
        assert_vec3_near(
            segment.from,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        );
        assert_vec3_near(segment.to, transform.position);
        finish_abstract_arrow(
            &metadata,
            &mut transform,
            step.finish.expect("flight runs the inertia tail"),
            accessors,
        );

        let water_movement = scale(
            EntityVec3 {
                x: 0.5,
                y: -0.25,
                z: 1.0,
            },
            f64::from(inertia),
        );
        assert_vec3_near(
            transform.position,
            EntityVec3 {
                x: 1.5,
                y: 63.75,
                z: -1.0,
            },
        );
        assert_vec3_near(
            transform.delta_movement,
            EntityVec3 {
                y: water_movement.y - 0.05,
                ..water_movement
            },
        );
        assert_eq!(effects.len(), 4);
        assert!(effects.iter().all(|effect| {
            *effect
                == particle_effect(EntityClientMovementParticleState::Simple(
                    EntityClientMovementSimpleParticleState {
                        entity_id,
                        kind: EntityClientMovementSimpleParticleKind::Bubble,
                        position: EntityVec3 {
                            x: 1.0 - water_movement.x * 0.25,
                            y: 64.0 - water_movement.y * 0.25,
                            z: -2.0 - water_movement.z * 0.25,
                        },
                        velocity: water_movement,
                    },
                ))
        }));
    }
}

#[test]
fn abstract_arrow_margin_refinement_honors_blocks_and_player_friendly_fire() {
    let owner_uuid = Uuid::from_u128(70);
    let target_uuid = Uuid::from_u128(71);
    let source = EntityAbstractArrowCollisionSource {
        entity_id: 70,
        entity_type_id: VANILLA_ENTITY_TYPE_ARROW_ID,
        position: EntityVec3 {
            x: 0.0,
            y: 0.9,
            z: 0.0,
        },
        movement: EntityVec3 {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        },
        bounds: EntityPickBoundsState::from_base_size(0.5, 0.5, 0.0),
        on_ground: false,
        tick_count: 22,
        owner_uuid: Some(owner_uuid),
        owner_entity_type_id: Some(VANILLA_ENTITY_TYPE_PLAYER_ID),
        owner_root_vehicle_id: None,
        left_owner: true,
        trident_dealt_damage: false,
        piercing_ignore_entity_ids: Vec::new(),
    };
    let target = EntityProjectileTarget {
        entity_id: 71,
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        uuid: target_uuid,
        root_vehicle_id: 71,
        position: EntityVec3 {
            x: 1.0,
            y: 0.0,
            z: 0.35,
        },
        bounds: EntityPickBoundsState::from_base_size(0.6, 1.8, 0.1),
        alive: true,
        pickable: true,
        deflects_projectiles: false,
        redirectable_projectile: false,
        hurt_client: false,
        trident_hurt_client: false,
        hurt_client_rejects_ender_dragon_owner: false,
        incarnation: EntityIncarnation(71),
    };
    let batch = || EntityAbstractArrowCollisionBatch {
        sources: vec![source.clone()],
        targets: vec![target],
    };
    let miss_clip = |_: i32, _: EntityVec3, movement: EntityVec3, _: bool, _: f64| {
        crate::client::ProjectileBlockMovementClip {
            movement: [movement.x, movement.y, movement.z],
            world_border_hit: false,
            block_state_id: None,
        }
    };
    let friendly_filtered = resolve_abstract_arrow_collision_facts(
        batch(),
        |_| None,
        miss_clip,
        |owner, target| owner != owner_uuid || target != target_uuid,
    );
    assert!(friendly_filtered[&70].entity_hits.is_empty());

    let unobstructed =
        resolve_abstract_arrow_collision_facts(batch(), |_| None, miss_clip, |_, _| true);
    assert_eq!(unobstructed[&70].entity_hits.len(), 1);
    assert!(
        unobstructed[&70].entity_hits[0].location.z >= 0.05 - 1.0e-12,
        "the inflated-margin entry must be refined back to the exact target surface"
    );

    let crystal = EntityProjectileTarget {
        entity_type_id: VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
        hurt_client: true,
        hurt_client_rejects_ender_dragon_owner: true,
        ..target
    };
    let mut dragon_owned_source = source.clone();
    dragon_owned_source.owner_entity_type_id = Some(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID);
    let dragon_owned = resolve_abstract_arrow_collision_facts(
        EntityAbstractArrowCollisionBatch {
            sources: vec![dragon_owned_source],
            targets: vec![crystal],
        },
        |_| None,
        miss_clip,
        |_, _| true,
    );
    assert!(!dragon_owned[&70].entity_hits[0].hurt_client);

    let obstructed = resolve_abstract_arrow_collision_facts(
        batch(),
        |_| None,
        |_, _, movement, _, _| {
            let secondary = movement.z != 0.0;
            crate::client::ProjectileBlockMovementClip {
                movement: if secondary {
                    [0.0; 3]
                } else {
                    [movement.x, movement.y, movement.z]
                },
                world_border_hit: false,
                block_state_id: secondary.then_some(1),
            }
        },
        |_, _| true,
    );
    assert!(obstructed[&70].entity_hits.is_empty());
}

#[test]
fn abstract_arrow_block_hit_mutates_exact_saveable_client_state() {
    let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    )
    .unwrap();
    let mut metadata = EntityMetadata::default();
    set_entity_data_byte(&mut metadata, accessors.flags_data_id, accessors.crit_mask);
    set_entity_data_byte(&mut metadata, accessors.pierce_level_data_id, 4);
    let mut projectile = test_arrow_lifecycle();
    let mut runtime = test_arrow_runtime(72);
    runtime.piercing_ignore_entity_ids.extend([80, 81]);
    let mut animations = EntityClientAnimationState {
        kind: crate::entities::animations::EntityKindAnimation::Arrow(None),
        ..EntityClientAnimationState::default()
    };
    let mut transform = test_transform(EntityVec3 {
        x: 1.0,
        y: -0.25,
        z: 0.0,
    });
    let collision = EntityAbstractArrowCollisionFacts {
        initial_block_state_id: None,
        endpoint: EntityVec3 {
            x: 1.75,
            y: 63.75,
            z: -2.0,
        },
        world_border_hit: false,
        block_hit: Some(EntityAbstractArrowBlockHit {
            location: EntityVec3 {
                x: 1.75,
                y: 63.75,
                z: -2.0,
            },
            block_state_id: 9,
        }),
        entity_hits: Vec::new(),
    };
    let step = advance_abstract_arrow(
        72,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
        &mut metadata,
        &mut animations,
        &mut transform,
        accessors,
        false,
        Some(&collision),
        &mut Vec::new(),
    );
    assert_eq!(step.collision, Some(collision.clone()));
    apply_abstract_arrow_endpoint_hit(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
        &mut metadata,
        &mut projectile,
        &mut runtime,
        &mut animations,
        &mut transform,
        accessors,
        &collision,
    );
    assert_eq!(projectile.arrow_last_block_state_id, Some(9));
    assert!(projectile.arrow_hit_ground_sound_is_arrow);
    assert!(runtime.piercing_ignore_entity_ids.is_empty());
    assert!(entity_data_bool(
        &metadata.data_values,
        accessors.in_ground_data_id,
        accessors.in_ground_default
    ));
    assert_eq!(
        entity_data_byte(
            &metadata.data_values,
            accessors.flags_data_id,
            accessors.flags_default
        ) & accessors.crit_mask,
        0
    );
    assert_eq!(
        entity_data_byte(
            &metadata.data_values,
            accessors.pierce_level_data_id,
            accessors.pierce_level_default
        ),
        0
    );
    assert_eq!(
        animations.arrow_kind().unwrap().shake_time,
        7,
        "onHitBlock installs shake after this tick's decrement"
    );
    assert_vec3_near(
        transform.position,
        EntityVec3 {
            x: 1.75 - f64::from(0.05_f32),
            y: 63.75 + f64::from(0.05_f32),
            z: -2.0,
        },
    );
    assert_eq!(transform.delta_movement, EntityVec3::default());
}

#[test]
fn trident_initial_block_containment_enters_ground_branch_on_the_same_tick() {
    let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    )
    .unwrap();
    let mut metadata = EntityMetadata::default();
    let mut animations = EntityClientAnimationState {
        kind: crate::entities::animations::EntityKindAnimation::Arrow(None),
        ..EntityClientAnimationState::default()
    };
    let mut transform = test_transform(EntityVec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let collision = EntityAbstractArrowCollisionFacts {
        initial_block_state_id: Some(9),
        endpoint: transform.position,
        world_border_hit: false,
        block_hit: None,
        entity_hits: Vec::new(),
    };
    let step = advance_abstract_arrow(
        720,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
        &mut metadata,
        &mut animations,
        &mut transform,
        accessors,
        false,
        Some(&collision),
        &mut Vec::new(),
    );

    assert!(step.trident_initial_containment_grounded);
    assert!(step.finish.is_none());
    assert_eq!(transform.delta_movement, EntityVec3::default());
    assert!(entity_data_bool(
        &metadata.data_values,
        accessors.in_ground_data_id,
        accessors.in_ground_default,
    ));
    assert_eq!(animations.arrow_kind().unwrap().shake_time, 6);
}

#[test]
fn abstract_arrow_world_border_hit_reverses_before_inertia_and_gravity() {
    let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARROW_ID,
    )
    .unwrap();
    let mut metadata = EntityMetadata::default();
    let mut projectile = test_arrow_lifecycle();
    let mut runtime = test_arrow_runtime(721);
    let mut animations = EntityClientAnimationState::default();
    let mut transform = test_transform(EntityVec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let collision = EntityAbstractArrowCollisionFacts {
        initial_block_state_id: None,
        endpoint: EntityVec3 {
            x: 1.5,
            y: 64.0,
            z: -2.0,
        },
        world_border_hit: true,
        block_hit: None,
        entity_hits: Vec::new(),
    };
    let step = advance_abstract_arrow(
        721,
        VANILLA_ENTITY_TYPE_ARROW_ID,
        &mut metadata,
        &mut animations,
        &mut transform,
        accessors,
        false,
        Some(&collision),
        &mut Vec::new(),
    );
    apply_abstract_arrow_endpoint_hit(
        VANILLA_ENTITY_TYPE_ARROW_ID,
        &mut metadata,
        &mut projectile,
        &mut runtime,
        &mut animations,
        &mut transform,
        accessors,
        &collision,
    );
    finish_abstract_arrow(&metadata, &mut transform, step.finish.unwrap(), accessors);

    assert_vec3_near(
        transform.position,
        EntityVec3 {
            x: 1.5,
            y: 64.0,
            z: -2.0,
        },
    );
    assert_vec3_near(
        transform.delta_movement,
        EntityVec3 {
            x: -0.1 * f64::from(0.99_f32),
            y: -0.05,
            z: 0.0,
        },
    );
    assert_eq!(projectile.arrow_last_block_state_id, None);
}

#[test]
fn abstract_arrow_piercing_limit_discards_after_level_plus_one_targets() {
    let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARROW_ID,
    )
    .unwrap();
    let mut metadata = EntityMetadata::default();
    set_entity_data_byte(&mut metadata, accessors.pierce_level_data_id, 1);
    let mut projectile = test_arrow_lifecycle();
    let mut runtime = test_arrow_runtime(73);
    let mut animations = EntityClientAnimationState::default();
    let mut transform = test_transform(EntityVec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let entity_hits = (80..83)
        .map(|entity_id| EntityAbstractArrowEntityHit {
            entity_id,
            incarnation: EntityIncarnation(entity_id as u64),
            location: EntityVec3 {
                x: f64::from(entity_id - 79),
                y: 64.0,
                z: -2.0,
            },
            target_position: EntityVec3 {
                x: f64::from(entity_id - 79),
                y: 64.0,
                z: -2.0,
            },
            deflects_projectiles: false,
            redirectable_projectile: false,
            hurt_client: true,
        })
        .collect();
    let collision = EntityAbstractArrowCollisionFacts {
        initial_block_state_id: None,
        endpoint: EntityVec3 {
            x: 2.0,
            y: 64.0,
            z: -2.0,
        },
        world_border_hit: false,
        block_hit: None,
        entity_hits,
    };
    let step = advance_abstract_arrow(
        73,
        VANILLA_ENTITY_TYPE_ARROW_ID,
        &mut metadata,
        &mut animations,
        &mut transform,
        accessors,
        false,
        Some(&collision),
        &mut Vec::new(),
    );
    let collision_step = advance_abstract_arrow_collision_after_first_effect(
        VANILLA_ENTITY_TYPE_ARROW_ID,
        &mut metadata,
        &mut projectile,
        &mut runtime,
        &mut transform,
        accessors,
        step.collision.as_ref().unwrap(),
        None,
    );
    assert!(collision_step.discarded);
    assert_eq!(runtime.piercing_ignore_entity_ids, vec![80, 81]);
    assert_vec3_near(
        transform.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        },
    );
}

#[test]
fn abstract_arrow_client_hurt_discards_and_motion_correction_releases_ground_state() {
    let mut store = WorldStore::new();
    let mut arrow = projectile(
        74,
        VANILLA_ENTITY_TYPE_ARROW_ID,
        ProtocolVec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        },
    );
    arrow.position = ProtocolVec3d {
        x: 0.0,
        y: 64.0,
        z: 0.0,
    };
    store.apply_add_entity(arrow.clone());
    let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARROW_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 74,
        values: vec![EntityDataValue {
            data_id: accessors.in_ground_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 74,
        delta_movement: ProtocolVec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        },
    }));
    assert!(!store
        .entities
        .metadata_bool_for_entity(74, accessors.in_ground_data_id, accessors.in_ground_default,)
        .unwrap());

    let mut boat = projectile(
        75,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        ProtocolVec3d::default(),
    );
    boat.position = ProtocolVec3d {
        x: 1.5,
        y: 64.0,
        z: 0.0,
    };
    store.apply_add_entity(boat);
    store.advance_entity_client_animations(1);
    assert!(
        store.probe_entity(74).is_none(),
        "VehicleEntity.hurtClient accepts the simulated arrow hit, so a non-piercing arrow discards locally"
    );
    assert!(store.probe_entity(75).is_some());
}

#[test]
fn item_entity_projectile_hurt_respects_effective_damage_resistant_component() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        76,
        VANILLA_ENTITY_TYPE_ITEM_ID,
        ProtocolVec3d::default(),
    ));
    set_nonempty_item_stack(&mut store, 76);
    assert_eq!(
        store.item_entity_accepts_projectile_hurt_client(76),
        [true, true]
    );

    let arrow_damage_type_id = store
        .registry_holder_id_by_resource_id("minecraft:damage_type", "minecraft:arrow")
        .unwrap();
    let component_patch = bbb_protocol::packets::DataComponentPatchSummary {
        added_type_ids: vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        ],
        damage_resistant_type_registry_ids: vec![arrow_damage_type_id],
        ..Default::default()
    };
    let data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ITEM_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 76,
        values: vec![EntityDataValue {
            data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                count: 1,
                component_patch: component_patch.into(),
            }),
        }],
    }));
    assert_eq!(
        store.item_entity_accepts_projectile_hurt_client(76),
        [false, true]
    );
}

#[test]
fn abstract_arrow_hit_aim_deflects_redirectable_projectile_to_owner_look() {
    let mut store = WorldStore::new();
    let mut owner = projectile(77, VANILLA_ENTITY_TYPE_PLAYER_ID, ProtocolVec3d::default());
    owner.position = ProtocolVec3d {
        x: -5.0,
        y: 64.0,
        z: 0.0,
    };
    owner.x_rot = -10.0;
    owner.y_rot = 45.0;
    store.apply_add_entity(owner);

    let mut fireball = projectile(
        78,
        VANILLA_ENTITY_TYPE_FIREBALL_ID,
        ProtocolVec3d::default(),
    );
    fireball.position = ProtocolVec3d {
        x: 1.5,
        y: 64.0,
        z: 0.0,
    };
    store.apply_add_entity(fireball);

    let mut arrow = projectile(
        79,
        VANILLA_ENTITY_TYPE_ARROW_ID,
        ProtocolVec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        },
    );
    arrow.position = ProtocolVec3d {
        x: 0.0,
        y: 64.0,
        z: 0.0,
    };
    arrow.data = 77;
    store.apply_add_entity(arrow);

    store.advance_entity_client_animations(1);

    assert_vec3_near(
        store.probe_entity_transform(78).unwrap().delta_movement,
        calculate_view_vector(-10.0, 45.0),
    );
    assert!(
        store.probe_entity_transform(79).unwrap().delta_movement.x < 0.0,
        "the redirectable target still rejects client damage, so the source arrow reverses"
    );
}

#[test]
fn abstract_arrow_runtime_round_trip_rebinds_deflector_and_preserves_private_state() {
    let mut store = WorldStore::new();
    let breeze = projectile(80, VANILLA_ENTITY_TYPE_BREEZE_ID, ProtocolVec3d::default());
    let breeze_uuid = breeze.uuid;
    store.apply_add_entity(breeze);
    store.apply_add_entity(projectile(
        81,
        VANILLA_ENTITY_TYPE_ARROW_ID,
        ProtocolVec3d::default(),
    ));

    let mut arrow = store.probe_entity(81).unwrap();
    let runtime = arrow.abstract_arrow_runtime.as_mut().unwrap();
    runtime.random_seed = 0x1234_5678_9abc;
    runtime.last_deflected_by = Some(crate::entities::EntityRuntimeReferenceState {
        entity_id: 80,
        uuid: breeze_uuid,
    });
    runtime.piercing_ignore_entity_ids = vec![82, 83];
    store.entities.insert_or_replace(arrow.clone());

    let breeze_incarnation = store
        .entities
        .projectile_targets(&HashSet::new())
        .into_iter()
        .find(|target| target.entity_id == 80)
        .unwrap()
        .incarnation;
    let runtime = store.entities.abstract_arrow_runtime(81).unwrap();
    assert_eq!(
        runtime.last_deflected_by,
        Some(EntityInstanceReference {
            entity_id: 80,
            incarnation: breeze_incarnation,
        })
    );
    assert_eq!(store.probe_entity(81).unwrap(), arrow);
    assert_eq!(
        store.publication_snapshot().probe_entity(81).unwrap(),
        arrow
    );

    let encoded = serde_json::to_value(&store).unwrap();
    let restored: WorldStore = serde_json::from_value(encoded).unwrap();
    assert_eq!(restored.probe_entity(81).unwrap(), arrow);
    let restored_breeze_incarnation = restored
        .entities
        .projectile_targets(&HashSet::new())
        .into_iter()
        .find(|target| target.entity_id == 80)
        .unwrap()
        .incarnation;
    let restored_runtime = restored.entities.abstract_arrow_runtime(81).unwrap();
    assert_eq!(
        restored_runtime.last_deflected_by,
        Some(EntityInstanceReference {
            entity_id: 80,
            incarnation: restored_breeze_incarnation,
        })
    );
    assert!(restored_runtime.pending_last_deflected_by.is_none());
    assert_eq!(restored_runtime.random_seed, 0x1234_5678_9abc);
    assert_eq!(restored_runtime.piercing_ignore_entity_ids, vec![82, 83]);
}

#[test]
fn loyalty_trident_returns_to_live_owner_once_and_preserves_old_generation() {
    let mut store = WorldStore::new();
    let mut owner = projectile(90, VANILLA_ENTITY_TYPE_PLAYER_ID, ProtocolVec3d::default());
    owner.position = ProtocolVec3d {
        x: 10.0,
        y: 64.0,
        z: 0.0,
    };
    store.apply_add_entity(owner);
    let mut trident = projectile(
        91,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
        ProtocolVec3d {
            x: 0.2,
            y: 0.1,
            z: -0.3,
        },
    );
    trident.data = 90;
    store.apply_add_entity(trident);
    let arrow_accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    )
    .unwrap();
    let trident_accessors = vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 91,
        values: vec![
            EntityDataValue {
                data_id: arrow_accessors.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(arrow_accessors.no_physics_mask),
            },
            EntityDataValue {
                data_id: trident_accessors.loyalty_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(2),
            },
        ],
    }));
    let generation_a = store.publication_snapshot();
    let owner_eye_height = f64::from(store.probe_entity_camera_pose(90).unwrap().eye_height);
    let original_position = EntityVec3 {
        x: 1.0,
        y: 64.0,
        z: -2.0,
    };
    let to_owner = EntityVec3 {
        x: 9.0,
        y: owner_eye_height,
        z: 2.0,
    };
    let adjusted_position = EntityVec3 {
        y: original_position.y + to_owner.y * 0.03,
        ..original_position
    };
    let return_delta = add(
        scale(
            EntityVec3 {
                x: 0.2,
                y: 0.1,
                z: -0.3,
            },
            0.95,
        ),
        scale(normalize(to_owner), 0.1),
    );

    store.advance_entity_client_animations(1);

    let transform = store.probe_entity_transform(91).unwrap();
    assert_vec3_near(transform.position, add(adjusted_position, return_delta));
    assert_vec3_near(
        transform.delta_movement,
        scale(return_delta, f64::from(0.99_f32)),
    );
    let lifecycle = store
        .probe_entity(91)
        .unwrap()
        .projectile_lifecycle
        .unwrap();
    assert_eq!(lifecycle.trident_return_ticks, 1);

    assert_eq!(
        generation_a
            .probe_entity(91)
            .unwrap()
            .projectile_lifecycle
            .unwrap()
            .trident_return_ticks,
        0
    );
    assert_eq!(
        generation_a.probe_entity_transform(91).unwrap().position,
        original_position
    );

    store.advance_entity_client_animations(1);
    assert!(store
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());
    assert_eq!(
        store
            .probe_entity(91)
            .unwrap()
            .projectile_lifecycle
            .unwrap()
            .trident_return_ticks,
        2
    );
}

#[test]
fn silent_returning_trident_suppresses_sound_but_advances_return_counter() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        92,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        ProtocolVec3d::default(),
    ));
    let mut trident = projectile(93, VANILLA_ENTITY_TYPE_TRIDENT_ID, ProtocolVec3d::default());
    trident.data = 92;
    store.apply_add_entity(trident);
    let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
    let arrow_accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    )
    .unwrap();
    let trident_accessors = vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    )
    .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 93,
        values: vec![
            EntityDataValue {
                data_id: entity_accessors.silent_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            EntityDataValue {
                data_id: arrow_accessors.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(arrow_accessors.no_physics_mask),
            },
            EntityDataValue {
                data_id: trident_accessors.loyalty_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(1),
            },
        ],
    }));

    store.advance_entity_client_animations(1);

    assert!(store
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());
    assert_eq!(
        store
            .probe_entity(93)
            .unwrap()
            .projectile_lifecycle
            .unwrap()
            .trident_return_ticks,
        1
    );
}
