use super::*;

#[test]
fn item_entity_stacks_filters_and_preserves_protocol_order() {
    let mut store = WorldStore::new();

    let mut first = protocol_add_entity_with_type(30, VANILLA_ENTITY_TYPE_ITEM_ID);
    first.position = ProtocolVec3d {
        x: 3.25,
        y: 65.5,
        z: -7.75,
    };
    store.apply_add_entity(first);
    store.apply_add_entity(protocol_add_entity_with_type(20, 7));
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        40,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        50,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 30,
        values: vec![item_stack_entity_data(item_stack(42, 3))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 20,
        values: vec![item_stack_entity_data(item_stack(99, 1))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![item_stack_entity_data(item_stack(51, 2))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 40,
        values: vec![item_stack_entity_data(ProtocolItemStackSummary::empty())],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![item_stack_entity_data(item_stack(77, 0))],
    }));

    let items = store.item_entity_stacks();
    assert_eq!(
        items.iter().map(|item| item.entity_id).collect::<Vec<_>>(),
        vec![30, 10]
    );
    assert_eq!(
        items[0].position,
        EntityVec3 {
            x: 3.25,
            y: 65.5,
            z: -7.75,
        }
    );
    assert_eq!(items[0].stack, item_stack(42, 3));
    assert_eq!(items[0].light, ENTITY_LIGHT_PROBE_FULL_BRIGHT);
    assert_eq!(
        items[1].position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(items[1].stack, item_stack(51, 2));
    assert_eq!(items[1].light, ENTITY_LIGHT_PROBE_FULL_BRIGHT);
}

#[test]
fn item_stacks_for_entity_types_collects_thrown_item_projectiles() {
    // The thrown-item projectiles carry their displayed item in the same `DATA_ITEM_STACK` (id 8) as
    // the dropped item, so the type-filtered accessor reads them for the billboard layer. Snowball is
    // type id 120, egg 39; a plain item (71) is excluded when those types are requested.
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(60, 120));
    store.apply_add_entity(protocol_add_entity_with_type(61, 39));
    store.apply_add_entity(protocol_add_entity_with_type(
        62,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![item_stack_entity_data(item_stack(880, 1))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 61,
        values: vec![item_stack_entity_data(item_stack(881, 1))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 62,
        values: vec![item_stack_entity_data(item_stack(882, 1))],
    }));

    let projectiles = store.item_stacks_for_entity_types(&[120, 39]);
    assert_eq!(
        projectiles
            .iter()
            .map(|item| item.entity_id)
            .collect::<Vec<_>>(),
        vec![60, 61]
    );
    assert_eq!(projectiles[0].stack, item_stack(880, 1));
    assert_eq!(projectiles[1].stack, item_stack(881, 1));

    // The dropped item (type 71) is untouched by the projectile-only query.
    assert!(!projectiles.iter().any(|item| item.entity_id == 62));
}

#[test]
fn firework_rocket_item_states_project_item_stack_pose_and_attachment_gate() {
    // Vanilla `FireworkRocketEntity` declares the item stack at id 8, attached target at id 9
    // (`OptionalInt`/`OptionalUnsignedInt`), and the shot-at-angle pose flag at id 10.
    const FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID: u8 = 9;
    const FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID: u8 = 10;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        70,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        71,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        72,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        73,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![item_stack_entity_data(item_stack(900, 1))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 71,
        values: vec![
            item_stack_entity_data(item_stack(901, 1)),
            protocol_bool_data(FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID, true),
        ],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 72,
        values: vec![
            item_stack_entity_data(item_stack(902, 1)),
            protocol_optional_unsigned_int_data(
                FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID,
                Some(5)
            ),
            protocol_bool_data(FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID, true),
        ],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 73,
        values: vec![item_stack_entity_data(item_stack(903, 1))],
    }));

    let fireworks = store.firework_rocket_item_states();
    assert_eq!(
        fireworks
            .iter()
            .map(|firework| firework.entity_id)
            .collect::<Vec<_>>(),
        vec![70, 71]
    );
    assert_eq!(fireworks[0].stack, item_stack(900, 1));
    assert!(!fireworks[0].shot_at_angle);
    assert_eq!(fireworks[1].stack, item_stack(901, 1));
    assert!(fireworks[1].shot_at_angle);
    assert!(fireworks
        .iter()
        .all(|firework| firework.light == ENTITY_LIGHT_PROBE_FULL_BRIGHT));
}

#[test]
fn firework_rocket_explosion_particle_state_projects_event_context() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        74,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        75,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        76,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        77,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));
    assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
        id: 75,
        delta_movement: ProtocolVec3d {
            x: 0.2,
            y: 0.4,
            z: -0.6,
        },
    }));

    let mut empty_stack = item_stack(904, 1);
    empty_stack.component_patch = DataComponentPatchSummary {
        fireworks_explosions_count: Some(0),
        ..DataComponentPatchSummary::default()
    };
    let explosion = FireworkExplosionSummary {
        shape: FireworkExplosionShapeSummary::Burst,
        colors: vec![0x112233],
        fade_colors: vec![0x445566],
        has_trail: true,
        has_twinkle: false,
    };
    let mut decoded_stack = item_stack(905, 1);
    decoded_stack.component_patch = DataComponentPatchSummary {
        fireworks_explosions_count: Some(1),
        fireworks_explosions: vec![explosion.clone()],
        ..DataComponentPatchSummary::default()
    };
    let mut declared_only_stack = item_stack(906, 1);
    declared_only_stack.component_patch = DataComponentPatchSummary {
        fireworks_explosions_count: Some(1),
        ..DataComponentPatchSummary::default()
    };

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 74,
        values: vec![item_stack_entity_data(empty_stack)],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 75,
        values: vec![item_stack_entity_data(decoded_stack)],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 76,
        values: vec![item_stack_entity_data(declared_only_stack)],
    }));

    let empty = store.firework_rocket_explosion_particle_state(74).unwrap();
    assert_eq!(empty.entity_id, 74);
    assert_eq!(
        empty.position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert!(!empty.has_explosions);
    assert!(empty.explosions.is_empty());

    let decoded = store.firework_rocket_explosion_particle_state(75).unwrap();
    assert_eq!(decoded.entity_id, 75);
    assert_eq!(
        decoded.delta_movement,
        EntityVec3 {
            x: 0.2,
            y: 0.4,
            z: -0.6,
        }
    );
    assert!(decoded.has_explosions);
    assert_eq!(decoded.explosions, vec![explosion]);

    let declared_only = store.firework_rocket_explosion_particle_state(76).unwrap();
    assert!(declared_only.has_explosions);
    assert!(declared_only.explosions.is_empty());
    assert!(store.firework_rocket_explosion_particle_state(77).is_none());
}

#[test]
fn ominous_item_spawner_item_states_project_item_stack_and_age_ticks() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        80,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        81,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        82,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 80,
        values: vec![item_stack_entity_data(item_stack(910, 3))],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 81,
        values: vec![item_stack_entity_data(ProtocolItemStackSummary::empty())],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 82,
        values: vec![item_stack_entity_data(item_stack(912, 1))],
    }));

    store.advance_entity_client_animations(7);

    let items = store.ominous_item_spawner_item_states_at_partial_tick(0.25);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].entity_id, 80);
    assert_eq!(items[0].stack, item_stack(910, 3));
    assert_eq!(items[0].age_ticks, 7.25);
}

#[test]
fn take_item_entity_shrinks_item_stacks_and_removes_entities() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(30, 7));

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![item_stack_entity_data(item_stack(42, 5))],
    }));

    assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
        item_id: 10,
        player_id: 99,
        amount: 2,
    }));
    let item_entity = store.probe_entity(10).unwrap();
    assert_eq!(
        item_entity.data_values,
        vec![item_stack_entity_data(item_stack(42, 3))]
    );

    assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
        item_id: 10,
        player_id: 99,
        amount: 3,
    }));
    assert!(store.probe_entity(10).is_none());

    assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
        item_id: 20,
        player_id: 99,
        amount: 1,
    }));
    assert!(store.probe_entity(20).is_some());

    assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
        item_id: 30,
        player_id: 99,
        amount: 1,
    }));
    assert!(store.probe_entity(30).is_none());
    assert!(!store.apply_take_item_entity(ProtocolTakeItemEntity {
        item_id: 999,
        player_id: 99,
        amount: 1,
    }));

    assert_eq!(store.entity_count(), 1);
    assert_eq!(store.counters().take_item_entities_received, 5);
    assert_eq!(store.counters().take_item_entities_applied, 4);
    assert_eq!(store.counters().take_item_entities_ignored, 1);
    assert_eq!(store.counters().item_entity_stack_shrinks, 2);
    assert_eq!(store.counters().take_item_entities_removed, 2);
    assert_eq!(store.counters().entities_removed, 2);
    assert_eq!(store.counters().entities_tracked, 1);
}

#[test]
fn experience_orb_icon_thresholds_match_vanilla() {
    let cases = [
        (-1, 0),
        (0, 0),
        (2, 0),
        (3, 1),
        (6, 1),
        (7, 2),
        (16, 2),
        (17, 3),
        (36, 3),
        (37, 4),
        (72, 4),
        (73, 5),
        (148, 5),
        (149, 6),
        (306, 6),
        (307, 7),
        (616, 7),
        (617, 8),
        (1236, 8),
        (1237, 9),
        (2476, 9),
        (2477, 10),
    ];
    for (value, icon) in cases {
        assert_eq!(experience_orb_icon(value), icon, "value {value}");
    }
}

#[test]
fn take_item_entity_pickup_particle_state_captures_experience_orb_icon() {
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 10,
        values: vec![ProtocolEntityDataValue {
            data_id: VANILLA_EXPERIENCE_ORB_VALUE_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(149),
        }],
    }));

    let state = store
        .take_item_entity_pickup_particle_state(10, 20)
        .expect("experience orb pickup particle state");

    assert_eq!(state.item_entity_id, 10);
    assert_eq!(
        state.item_entity_type_id,
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID
    );
    assert_eq!(state.item_stack, None);
    assert_eq!(state.experience_orb_icon, Some(6));
    assert_eq!(state.projectile_model, None);
    assert_eq!(
        take_item_entity_pickup_light(
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            TerrainLight { block: 6, sky: 12 }
        ),
        TerrainLight { block: 13, sky: 12 }
    );
    assert_eq!(
        take_item_entity_pickup_light(
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            TerrainLight { block: 14, sky: 3 }
        ),
        TerrainLight { block: 15, sky: 3 }
    );
    assert_eq!(
        take_item_entity_pickup_light(
            VANILLA_ENTITY_TYPE_ITEM_ID,
            TerrainLight { block: 6, sky: 12 }
        ),
        TerrainLight { block: 6, sky: 12 }
    );
}

#[test]
fn take_item_entity_pickup_particle_state_captures_projectile_models() {
    // Vanilla `ClientboundTakeItemEntity` spawns `ItemPickupParticle` for any
    // picked entity; arrows and tridents render their projectile models through
    // `ArrowRenderer` / `TippableArrowRenderer` / `SpectralArrowRenderer` /
    // `ThrownTridentRenderer` using the extracted `yRot`/`xRot`.
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        10,
        VANILLA_ENTITY_TYPE_ARROW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        11,
        VANILLA_ENTITY_TYPE_ARROW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        12,
        VANILLA_ENTITY_TYPE_ARROW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        13,
        VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        14,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        15,
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        16,
        VANILLA_ENTITY_TYPE_ITEM_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    // Tipped arrow: vanilla `TippableArrowRenderer.isTipped = getColor() > 0`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 11,
        values: vec![ProtocolEntityDataValue {
            data_id: VANILLA_ARROW_EFFECT_COLOR_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(0x38_5DC6),
        }],
    }));
    // Color 0 is NOT tipped (vanilla uses a strict `> 0`).
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 12,
        values: vec![ProtocolEntityDataValue {
            data_id: VANILLA_ARROW_EFFECT_COLOR_DATA_ID,
            serializer_id: 1,
            value: EntityDataValueKind::Int(0),
        }],
    }));
    // Enchanted trident: vanilla `ThrownTrident.ID_FOIL`.
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 15,
        values: vec![ProtocolEntityDataValue {
            data_id: VANILLA_TRIDENT_FOIL_DATA_ID,
            serializer_id: 8,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 16,
        values: vec![item_stack_entity_data(item_stack(42, 5))],
    }));

    let state = store
        .take_item_entity_pickup_particle_state(10, 20)
        .expect("untipped arrow pickup particle state");
    assert_eq!(
        state.projectile_model,
        Some(TakeItemEntityPickupProjectileModel::Arrow { tipped: false })
    );
    assert_eq!(state.item_stack, None);
    assert_eq!(state.experience_orb_icon, None);
    // The extracted render-state rotations travel with the descriptor
    // (`protocol_add_entity_with_type` spawns with yRot 20 / xRot -10).
    assert_eq!(state.item_y_rot, 20.0);
    assert_eq!(state.item_x_rot, -10.0);

    let tipped = store
        .take_item_entity_pickup_particle_state(11, 20)
        .expect("tipped arrow pickup particle state");
    assert_eq!(
        tipped.projectile_model,
        Some(TakeItemEntityPickupProjectileModel::Arrow { tipped: true })
    );

    let zero_color = store
        .take_item_entity_pickup_particle_state(12, 20)
        .expect("zero-color arrow pickup particle state");
    assert_eq!(
        zero_color.projectile_model,
        Some(TakeItemEntityPickupProjectileModel::Arrow { tipped: false })
    );

    let spectral = store
        .take_item_entity_pickup_particle_state(13, 20)
        .expect("spectral arrow pickup particle state");
    assert_eq!(
        spectral.projectile_model,
        Some(TakeItemEntityPickupProjectileModel::SpectralArrow)
    );

    let trident = store
        .take_item_entity_pickup_particle_state(14, 20)
        .expect("trident pickup particle state");
    assert_eq!(
        trident.projectile_model,
        Some(TakeItemEntityPickupProjectileModel::Trident { foil: false })
    );

    let foil_trident = store
        .take_item_entity_pickup_particle_state(15, 20)
        .expect("foil trident pickup particle state");
    assert_eq!(
        foil_trident.projectile_model,
        Some(TakeItemEntityPickupProjectileModel::Trident { foil: true })
    );

    // Item entities keep the dedicated stack channel; no carried projectile.
    let item = store
        .take_item_entity_pickup_particle_state(16, 20)
        .expect("item pickup particle state");
    assert_eq!(item.projectile_model, None);
    assert_eq!(item.item_stack, Some(item_stack(42, 5)));
}
