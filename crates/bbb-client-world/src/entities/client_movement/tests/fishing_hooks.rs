use super::*;

#[test]
fn fishing_hook_free_flight_uses_client_gravity_inertia_and_generation_isolation() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        80,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        ProtocolVec3d::default(),
    ));
    let mut hook = projectile(
        81,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        ProtocolVec3d {
            x: 0.5,
            y: 0.2,
            z: -0.25,
        },
    );
    hook.data = 80;
    store.apply_add_entity(hook);
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let moved = EntityVec3 {
        x: 0.5,
        y: 0.17,
        z: -0.25,
    };
    assert_vec3_near(
        store.probe_entity_transform(81).unwrap().position,
        add(
            generation_a.probe_entity_transform(81).unwrap().position,
            moved,
        ),
    );
    assert_vec3_near(
        store.probe_entity_transform(81).unwrap().delta_movement,
        scale(moved, 0.92),
    );
    assert_eq!(
        generation_a
            .entities
            .fishing_hook_movement(81)
            .unwrap()
            .state,
        EntityFishingHookState::Flying
    );
    assert_eq!(
        generation_a.probe_entity_transform(81).unwrap().position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
}

#[test]
fn client_level_catch_up_interleaves_entity_ticks_with_moving_world_border() {
    let mut store = WorldStore::new();
    store.apply_set_border_lerp_size(ProtocolSetBorderLerpSize {
        old_size: 10.0,
        new_size: 4.0,
        lerp_time: 4,
    });
    store.apply_add_entity(projectile(
        87,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        ProtocolVec3d::default(),
    ));
    let mut hook = projectile(
        88,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        ProtocolVec3d {
            x: 0.82,
            y: 0.0,
            z: 0.0,
        },
    );
    hook.position.x = 2.0;
    hook.data = 87;
    store.apply_add_entity(hook);

    let (_, running_ticks) = store.advance_client_level_entity_ticks_with_observation(3);

    let transform = store.probe_entity_transform(88).unwrap();
    assert_eq!(running_ticks, 3);
    assert_eq!(store.world_border().current_size, 5.5);
    assert!(
        transform.position.x < 3.5744 && transform.delta_movement.x < 0.0,
        "the third entity tick must collide with the border advanced after the first two ticks: {transform:?}"
    );
}

#[test]
fn fishing_hook_metadata_update_retains_exact_target_and_biting_rng_order() {
    let mut store = WorldStore::new();
    for id in [82, 83] {
        store.apply_add_entity(projectile(
            id,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            ProtocolVec3d::default(),
        ));
    }
    let mut hook = projectile(
        84,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        ProtocolVec3d {
            x: 0.25,
            y: 0.5,
            z: -0.75,
        },
    );
    hook.data = 82;
    store.apply_add_entity(hook);
    let generation_a = store.publication_snapshot();
    let mut expected_seed = store
        .entities
        .fishing_hook_movement(84)
        .unwrap()
        .random_seed;
    let random = java_random_next_float(&mut expected_seed);
    let expected_y = f64::from(-0.4_f32 * (0.6_f32 + random * 0.4_f32));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 84,
        values: vec![
            EntityDataValue {
                data_id: crate::entities::VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(84),
            },
            EntityDataValue {
                data_id: crate::entities::VANILLA_FISHING_HOOK_BITING_DATA_ID,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    let movement = store.entities.fishing_hook_movement(84).unwrap();
    assert_eq!(movement.hooked_entity_uuid, Some(Uuid::from_u128(83)));
    assert!(movement.biting);
    assert_eq!(movement.random_seed, expected_seed);
    assert_eq!(
        store.probe_entity_transform(84).unwrap().delta_movement.y,
        expected_y
    );
    assert_eq!(
        generation_a
            .entities
            .fishing_hook_movement(84)
            .unwrap()
            .hooked_entity_uuid,
        None,
        "the retained generation must not observe metadata-derived target identity"
    );

    store.advance_entity_client_animations(1);
    assert_eq!(
        store.entities.fishing_hook_movement(84).unwrap().state,
        EntityFishingHookState::HookedInEntity
    );
    let transition_position = store.probe_entity_transform(84).unwrap().position;
    store.advance_entity_client_animations(1);
    let hooked_position = store.probe_entity_transform(84).unwrap().position;
    assert_eq!(hooked_position.x, 1.0);
    assert_eq!(hooked_position.z, -2.0);
    assert!(hooked_position.y > transition_position.y);

    store.apply_remove_entities(RemoveEntities {
        entity_ids: vec![83],
    });
    let mut reused_id = projectile(83, VANILLA_ENTITY_TYPE_ZOMBIE_ID, ProtocolVec3d::default());
    reused_id.uuid = Uuid::from_u128(830);
    store.apply_add_entity(reused_id);
    store.advance_entity_client_animations(1);
    assert_eq!(
        store.entities.fishing_hook_movement(84).unwrap().state,
        EntityFishingHookState::Flying
    );
    assert_eq!(
        store
            .entities
            .metadata_int_for_entity(
                84,
                crate::entities::VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
                crate::entities::VANILLA_FISHING_HOOK_DEFAULT_HOOKED_ENTITY,
            )
            .unwrap(),
        0,
        "a reused entity id with a different UUID detaches the old hooked target"
    );
}

#[test]
fn fishing_hook_entity_hit_uses_margin_and_owner_root_gate() {
    let target = EntityProjectileTarget {
        entity_id: 91,
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        uuid: uuid::Uuid::from_u128(91),
        root_vehicle_id: 91,
        position: EntityVec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        bounds: EntityPickBoundsState::from_base_size(0.6, 1.8, 0.1),
        alive: true,
        pickable: true,
        deflects_projectiles: false,
        redirectable_projectile: false,
        hurt_client: false,
        trident_hurt_client: false,
        hurt_client_rejects_ender_dragon_owner: false,
        incarnation: EntityIncarnation(91),
    };
    let from = EntityVec3 {
        x: 0.0,
        y: 0.9,
        z: 0.0,
    };
    let delta = EntityVec3 {
        x: 2.0,
        y: 0.0,
        z: 0.0,
    };
    assert_eq!(
        fishing_hook_entity_hit(90, from, delta, Some(91), false, 1, None, &[target]),
        None,
        "a hook that has not left its owner root cannot hit that graph"
    );
    let hit =
        fishing_hook_entity_hit(90, from, delta, Some(91), true, 22, None, &[target]).unwrap();
    assert!((hit.scale - 0.2).abs() <= 1.0e-6, "{}", hit.scale);

    let deflecting = EntityProjectileTarget {
        deflects_projectiles: true,
        ..target
    };
    assert!(
        fishing_hook_entity_hit(90, from, delta, Some(91), true, 22, None, &[deflecting],)
            .unwrap()
            .reverse_deflection
            .is_some()
    );
    assert_eq!(
        fishing_hook_entity_hit(
            90,
            from,
            delta,
            Some(91),
            true,
            22,
            Some(deflecting.entity_id),
            &[deflecting],
        )
        .unwrap()
        .reverse_deflection,
        None,
        "Projectile.lastDeflectedBy suppresses a repeated reverse from the same entity"
    );
    let other_deflector = EntityProjectileTarget {
        entity_id: 92,
        ..deflecting
    };
    assert!(
        fishing_hook_entity_hit(
            90,
            from,
            delta,
            Some(91),
            true,
            22,
            Some(deflecting.entity_id),
            &[other_deflector],
        )
        .unwrap()
        .reverse_deflection
        .is_some(),
        "a different deflector entity reverses the hook again"
    );
}

#[test]
fn fishing_hook_projectile_targets_include_dragon_parts_but_not_parent() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        100,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        ProtocolVec3d::default(),
    ));

    let targets = store.entities.projectile_targets(&HashSet::new());
    let parent = targets
        .iter()
        .find(|target| target.entity_id == 100)
        .unwrap();
    assert!(!parent.pickable);
    for part_id in 101..=108 {
        let part = targets
            .iter()
            .find(|target| target.entity_id == part_id)
            .unwrap_or_else(|| panic!("missing EnderDragonPart {part_id}"));
        assert!(part.pickable);
        assert!(part.alive);
        assert_eq!(part.root_vehicle_id, part_id);
    }
}

#[test]
fn entity_incarnation_changes_on_same_identity_readd_and_is_snapshot_isolated() {
    let mut store = WorldStore::new();
    let breeze = projectile(109, VANILLA_ENTITY_TYPE_BREEZE_ID, ProtocolVec3d::default());
    store.apply_add_entity(breeze.clone());
    let first = store
        .entities
        .projectile_targets(&HashSet::new())
        .into_iter()
        .find(|target| target.entity_id == 109)
        .unwrap()
        .incarnation;
    let generation_a = store.publication_snapshot();

    store.apply_add_entity(breeze);
    let replacement = store
        .entities
        .projectile_targets(&HashSet::new())
        .into_iter()
        .find(|target| target.entity_id == 109)
        .unwrap()
        .incarnation;
    let retained = generation_a
        .entities
        .projectile_targets(&HashSet::new())
        .into_iter()
        .find(|target| target.entity_id == 109)
        .unwrap()
        .incarnation;

    assert_ne!(replacement, first);
    assert_eq!(retained, first);

    store.apply_add_entity(projectile(
        110,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        ProtocolVec3d::default(),
    ));
    let player_before_respawn = store.entities.identity(110).unwrap().incarnation;
    let old = store.entities.take_record(110).unwrap();
    store
        .entities
        .respawn_local_player(110, old, false, false)
        .unwrap();
    assert_ne!(
        store.entities.identity(110).unwrap().incarnation,
        player_before_respawn,
        "respawn creates a new Vanilla entity object even when id and UUID stay stable"
    );
}

#[test]
fn fishing_hook_world_border_hit_reverses_and_damps_before_gravity() {
    let mut store = WorldStore::new();
    store.apply_set_border_size(ProtocolSetBorderSize { size: 4.0 });
    store.apply_add_entity(projectile(
        109,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        ProtocolVec3d::default(),
    ));
    let mut hook = projectile(
        110,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        ProtocolVec3d {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        },
    );
    hook.data = 109;
    store.apply_add_entity(hook);
    let mut expected_seed = store
        .entities
        .fishing_hook_movement(110)
        .unwrap()
        .random_seed;
    let _ = java_random_next_float(&mut expected_seed);

    store.advance_entity_client_animations(1);

    let transform = store.probe_entity_transform(110).unwrap();
    assert_vec3_near(
        transform.position,
        EntityVec3 {
            x: 0.0,
            y: 63.97,
            z: -2.0,
        },
    );
    assert_vec3_near(
        transform.delta_movement,
        EntityVec3 {
            x: -0.92,
            y: -0.0276,
            z: 0.0,
        },
    );
    assert_eq!(
        store
            .entities
            .fishing_hook_movement(110)
            .unwrap()
            .random_seed,
        expected_seed,
    );
    assert!(store
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());
}

#[test]
fn fishing_hook_breeze_hit_reverses_and_retains_deflector() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        111,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        ProtocolVec3d::default(),
    ));
    let mut hook = projectile(
        112,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        ProtocolVec3d {
            x: 4.0,
            y: 0.0,
            z: 0.0,
        },
    );
    hook.data = 111;
    store.apply_add_entity(hook);
    let mut breeze = projectile(113, VANILLA_ENTITY_TYPE_BREEZE_ID, ProtocolVec3d::default());
    breeze.position.x = 2.0;
    store.apply_add_entity(breeze);
    let generation_a = store.publication_snapshot();

    store.advance_entity_client_animations(1);

    let transform = store.probe_entity_transform(112).unwrap();
    assert_vec3_near(
        transform.position,
        EntityVec3 {
            x: -1.0,
            y: 63.97,
            z: -2.0,
        },
    );
    assert_vec3_near(
        transform.delta_movement,
        EntityVec3 {
            x: -1.84,
            y: -0.0276,
            z: 0.0,
        },
    );
    assert_eq!(
        generation_a
            .entities
            .fishing_hook_movement(112)
            .unwrap()
            .last_deflected_by,
        None,
    );
    assert_eq!(
        store
            .entities
            .fishing_hook_movement(112)
            .unwrap()
            .last_deflected_by,
        Some(113),
    );
}

#[test]
fn fishing_hook_water_transition_and_bobbing_advance_single_rng() {
    let mut store = WorldStore::new();
    store.apply_add_entity(projectile(
        92,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        ProtocolVec3d::default(),
    ));
    let mut hook = projectile(
        93,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
        ProtocolVec3d {
            x: 0.3,
            y: -0.2,
            z: 0.6,
        },
    );
    hook.data = 92;
    store.apply_add_entity(hook);
    let identity = store.entities.identity(93).unwrap();
    let bounds = store
        .entities
        .client_movement_fluid_inputs(&HashSet::new())
        .into_iter()
        .find(|input| input.entity_id == 93)
        .unwrap()
        .bounds;
    let movement = store.entities.fishing_hook_movement(93).unwrap();
    let water = crate::client::EntityFluidContactState {
        block_fluid_kind: Some(crate::TerrainFluidKind::Water),
        block_fluid_height: 0.8,
        ..crate::client::EntityFluidContactState::default()
    };

    let (transition_delta, skipped, mut update, entity_hit, collision_delta) =
        store.entities.prepare_fishing_hook_movement(
            &identity,
            EntityVec3 {
                x: 1.0,
                y: 64.5,
                z: -2.0,
            },
            EntityVec3 {
                x: 0.3,
                y: -0.2,
                z: 0.6,
            },
            false,
            water,
            movement,
            7,
            bounds,
            1,
            &[],
            &HashSet::new(),
        );
    assert!(skipped);
    assert_eq!(update.movement.state, EntityFishingHookState::Bobbing);
    assert_eq!(entity_hit, None);
    assert_eq!(collision_delta, None);
    assert_vec3_near(
        transition_delta,
        EntityVec3 {
            x: 0.09,
            y: -0.04,
            z: 0.18,
        },
    );

    update.movement.biting = true;
    let mut expected_seed = update.movement.random_seed;
    let regular = java_random_next_float(&mut expected_seed);
    let force = 64.5 - 0.04 - 64.0 - 0.8;
    let bite_first = java_random_next_float(&mut expected_seed);
    let bite_second = java_random_next_float(&mut expected_seed);
    let expected = EntityVec3 {
        x: transition_delta.x * 0.9,
        y: transition_delta.y
            - force * f64::from(regular) * 0.2
            - 0.1 * f64::from(bite_first) * f64::from(bite_second),
        z: transition_delta.z * 0.9,
    };
    let (bobbing_delta, skipped, update, _, collision_delta) =
        store.entities.prepare_fishing_hook_movement(
            &identity,
            EntityVec3 {
                x: 1.0,
                y: 64.5,
                z: -2.0,
            },
            transition_delta,
            false,
            water,
            update.movement,
            7,
            bounds,
            2,
            &[],
            &HashSet::new(),
        );
    assert!(!skipped);
    assert_eq!(collision_delta, None);
    assert_vec3_near(bobbing_delta, expected);
    assert_eq!(update.movement.random_seed, expected_seed);
}
