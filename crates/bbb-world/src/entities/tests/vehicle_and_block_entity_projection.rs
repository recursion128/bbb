use super::*;

#[test]
fn entity_model_sources_project_narrow_render_state_from_model_targets() {
    const AGEABLE_BABY_DATA_ID: u8 = 16;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        35,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        135.0,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 35,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));

    let sources = store.entity_model_sources_at_partial_tick(1.0);

    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].entity_id, 35);
    assert_eq!(sources[0].entity_type_id, VANILLA_ENTITY_TYPE_CHICKEN_ID);
    assert_eq!(
        sources[0].position,
        EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        }
    );
    assert_eq!(sources[0].y_rot, 135.0);
    assert_eq!(sources[0].age_ticks, 0);
    assert_eq!(
        sources[0].data_values,
        vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)]
    );

    store.advance_entity_client_animations(3);
    let sources = store.entity_model_sources_at_partial_tick(0.5);
    assert_eq!(sources[0].age_ticks, 3);
}

#[test]
fn entity_model_source_single_entity_matches_list_entry() {
    const AGEABLE_BABY_DATA_ID: u8 = 16;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_y_rot(
        35,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
        135.0,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 35,
        values: vec![protocol_bool_data(AGEABLE_BABY_DATA_ID, true)],
    }));
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.advance_entity_client_animations(3);

    let partial_tick = 0.5;
    let list = store.entity_model_sources_at_partial_tick(partial_tick);
    assert_eq!(list.len(), 2);

    // The narrow single-entity API must return exactly the list entry for the same id, so the two
    // code paths cannot drift.
    for entry in &list {
        let single = store
            .entity_model_source_at_partial_tick(entry.entity_id, partial_tick)
            .expect("single-entity source present for a listed entity");
        assert_eq!(&single, entry);
    }

    // An id that was never spawned has no list entry, so the single API resolves to None.
    assert_eq!(
        store.entity_model_source_at_partial_tick(404, partial_tick),
        None
    );
}

#[test]
fn entity_model_sources_project_boat_rowing_times_from_paddles_and_passengers() {
    const BOAT_PADDLE_LEFT_DATA_ID: u8 = 11;
    const BOAT_PADDLE_RIGHT_DATA_ID: u8 = 12;
    const ADVANCE: f32 = std::f32::consts::PI / 8.0;

    let assert_close = |actual: f32, expected: f32, label: &str| {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "{label}: expected {expected}, got {actual}"
        );
    };
    let rowing = |store: &WorldStore, partial_tick: f32| -> (f32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(partial_tick)
            .into_iter()
            .find(|source| source.entity_id == 20)
            .expect("boat source");
        (source.boat_rowing_time_left, source.boat_rowing_time_right)
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        20,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        99,
        VANILLA_ENTITY_TYPE_CHICKEN_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 20,
        values: vec![protocol_bool_data(BOAT_PADDLE_LEFT_DATA_ID, true)],
    }));

    store.advance_entity_client_animations(1);
    assert_eq!(rowing(&store, 1.0), (0.0, 0.0));

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![99],
    }));
    store.advance_entity_client_animations(1);
    assert_close(rowing(&store, 0.0).0, 0.0, "left start");
    assert_close(rowing(&store, 0.5).0, ADVANCE * 0.5, "left mid");
    assert_close(rowing(&store, 1.0).0, ADVANCE, "left end");
    assert_eq!(rowing(&store, 1.0).1, 0.0);

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 20,
        values: vec![
            protocol_bool_data(BOAT_PADDLE_LEFT_DATA_ID, true),
            protocol_bool_data(BOAT_PADDLE_RIGHT_DATA_ID, true),
        ],
    }));
    store.advance_entity_client_animations(1);
    assert_close(rowing(&store, 1.0).0, ADVANCE * 2.0, "left second tick");
    assert_close(rowing(&store, 1.0).1, ADVANCE, "right first tick");

    assert!(store.apply_set_passengers(ProtocolSetPassengers {
        vehicle_id: 20,
        passenger_ids: vec![],
    }));
    store.advance_entity_client_animations(1);
    assert_eq!(rowing(&store, 1.0), (0.0, 0.0));
}

#[test]
fn entity_model_sources_project_boat_damage_roll_from_vehicle_metadata() {
    const VEHICLE_HURT_TIME_DATA_ID: u8 = 8;
    const VEHICLE_HURT_DIR_DATA_ID: u8 = 9;
    const VEHICLE_DAMAGE_DATA_ID: u8 = 10;
    const BOAT_PADDLE_LEFT_DATA_ID: u8 = 11;

    let assert_close = |actual: f32, expected: f32, label: &str| {
        assert!(
            (actual - expected).abs() < 1.0e-6,
            "{label}: expected {expected}, got {actual}"
        );
    };
    let damage = |store: &WorldStore, partial_tick: f32| -> (f32, i32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(partial_tick)
            .into_iter()
            .find(|source| source.entity_id == 21)
            .expect("boat source");
        (
            source.boat_hurt_time,
            source.boat_hurt_dir,
            source.boat_damage_time,
        )
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        21,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 21,
        values: vec![
            protocol_int_data(VEHICLE_HURT_TIME_DATA_ID, 10),
            protocol_int_data(VEHICLE_HURT_DIR_DATA_ID, -1),
            protocol_float_data(VEHICLE_DAMAGE_DATA_ID, 20.0),
        ],
    }));

    let initial = damage(&store, 0.5);
    assert_close(initial.0, 9.5, "initial hurt time");
    assert_eq!(initial.1, -1);
    assert_close(initial.2, 19.5, "initial damage time");

    store.advance_entity_client_animations(1);
    let ticked = damage(&store, 0.5);
    assert_close(ticked.0, 8.5, "ticked hurt time");
    assert_eq!(ticked.1, -1);
    assert_close(ticked.2, 18.5, "ticked damage time");

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 21,
        values: vec![protocol_bool_data(BOAT_PADDLE_LEFT_DATA_ID, true)],
    }));
    let after_paddle_update = damage(&store, 0.5);
    assert_close(after_paddle_update.0, 8.5, "paddle update keeps hurt time");
    assert_eq!(after_paddle_update.1, -1);
    assert_close(
        after_paddle_update.2,
        18.5,
        "paddle update keeps damage time",
    );

    store.advance_entity_client_animations(20);
    assert_eq!(damage(&store, 1.0), (0.0, 1, 0.0));
}

#[test]
fn entity_model_sources_project_minecart_damage_roll_from_vehicle_metadata() {
    const VEHICLE_HURT_TIME_DATA_ID: u8 = 8;
    const VEHICLE_HURT_DIR_DATA_ID: u8 = 9;
    const VEHICLE_DAMAGE_DATA_ID: u8 = 10;

    let damage = |store: &WorldStore, partial_tick: f32| -> (f32, i32, f32) {
        let source = store
            .entity_model_sources_at_partial_tick(partial_tick)
            .into_iter()
            .find(|source| source.entity_id == 22)
            .expect("minecart source");
        (
            source.boat_hurt_time,
            source.boat_hurt_dir,
            source.boat_damage_time,
        )
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        22,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 22,
        values: vec![
            protocol_int_data(VEHICLE_HURT_TIME_DATA_ID, 10),
            protocol_int_data(VEHICLE_HURT_DIR_DATA_ID, -1),
            protocol_float_data(VEHICLE_DAMAGE_DATA_ID, 20.0),
        ],
    }));

    assert_eq!(damage(&store, 0.5), (9.5, -1, 19.5));
    store.advance_entity_client_animations(1);
    assert_eq!(damage(&store, 0.5), (8.5, -1, 18.5));
    store.advance_entity_client_animations(20);
    assert_eq!(damage(&store, 1.0), (0.0, 1, 0.0));
}

#[test]
fn entity_model_sources_project_tnt_minecart_fuse_from_prime_event() {
    let fuse = |store: &WorldStore, entity_id: i32, partial_tick: f32| -> f32 {
        store
            .entity_model_sources_at_partial_tick(partial_tick)
            .into_iter()
            .find(|source| source.entity_id == entity_id)
            .expect("minecart source")
            .minecart_tnt_fuse_remaining_in_ticks
    };

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        28,
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        29,
        VANILLA_ENTITY_TYPE_MINECART_ID,
    ));

    assert_eq!(fuse(&store, 28, 0.0), -1.0);
    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 28,
        event_id: 10,
    }));
    // Vanilla `MinecartTNT.handleEntityEvent(10)` primes `fuse = 80`, and
    // `TntMinecartRenderer.extractRenderState` consumes `fuse - partialTick + 1.0`.
    assert_eq!(fuse(&store, 28, 0.0), 81.0);
    assert_eq!(fuse(&store, 28, 0.5), 80.5);

    assert!(store.apply_entity_event(ProtocolEntityEvent {
        entity_id: 29,
        event_id: 10,
    }));
    assert_eq!(
        fuse(&store, 29, 0.0),
        -1.0,
        "event 10 is TNT-minecart-specific"
    );

    store.advance_entity_client_animations(1);
    assert_eq!(fuse(&store, 28, 0.0), 80.0);
    assert_eq!(fuse(&store, 28, 0.5), 79.5);
    store.advance_entity_client_animations(75);
    assert_eq!(fuse(&store, 28, 0.0), 5.0);
    assert_eq!(fuse(&store, 28, 0.5), 4.5);
    store.advance_entity_client_animations(10);
    assert_eq!(fuse(&store, 28, 0.0), 1.0);
    assert_eq!(fuse(&store, 28, 1.0), 0.0);
}

#[test]
fn minecart_display_block_state_projects_defaults_and_custom_metadata() {
    const MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID: u8 = 11;
    const MINECART_DISPLAY_OFFSET_DATA_ID: u8 = 12;
    const FURNACE_MINECART_FUEL_DATA_ID: u8 = 13;

    let mut store = WorldStore::new();
    for (id, entity_type_id) in [
        (23, VANILLA_ENTITY_TYPE_MINECART_ID),
        (24, VANILLA_ENTITY_TYPE_CHEST_MINECART_ID),
        (25, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID),
        (26, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID),
        (27, VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID),
        (28, VANILLA_ENTITY_TYPE_TNT_MINECART_ID),
        (29, VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID),
        (30, VANILLA_ENTITY_TYPE_COW_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
    }

    assert_eq!(store.minecart_display_block_state(23), None);
    assert_eq!(store.minecart_display_block_state(30), None);
    assert_eq!(
        store.minecart_display_block_state(24),
        Some(MinecartDisplayBlockState {
            block: EntityBlockModelState {
                name: "minecraft:chest".to_string(),
                properties: BTreeMap::from([
                    ("facing".to_string(), "north".to_string()),
                    ("type".to_string(), "single".to_string()),
                    ("waterlogged".to_string(), "false".to_string()),
                ]),
            },
            display_offset: 8,
        })
    );
    assert_eq!(
        store.minecart_display_block_state(25),
        Some(MinecartDisplayBlockState {
            block: EntityBlockModelState {
                name: "minecraft:furnace".to_string(),
                properties: BTreeMap::from([
                    ("facing".to_string(), "north".to_string()),
                    ("lit".to_string(), "false".to_string()),
                ]),
            },
            display_offset: 6,
        })
    );
    assert_eq!(
        store.minecart_display_block_state(26),
        Some(MinecartDisplayBlockState {
            block: EntityBlockModelState {
                name: "minecraft:hopper".to_string(),
                properties: BTreeMap::from([
                    ("enabled".to_string(), "true".to_string()),
                    ("facing".to_string(), "down".to_string()),
                ]),
            },
            display_offset: 1,
        })
    );
    assert_eq!(
        store.minecart_display_block_state(27),
        Some(MinecartDisplayBlockState {
            block: EntityBlockModelState {
                name: "minecraft:command_block".to_string(),
                properties: BTreeMap::from([
                    ("conditional".to_string(), "false".to_string()),
                    ("facing".to_string(), "north".to_string()),
                ]),
            },
            display_offset: 6,
        })
    );
    assert_eq!(
        store.minecart_display_block_state(28),
        Some(MinecartDisplayBlockState {
            block: EntityBlockModelState {
                name: "minecraft:tnt".to_string(),
                properties: BTreeMap::from([("unstable".to_string(), "false".to_string())]),
            },
            display_offset: 6,
        })
    );
    assert_eq!(
        store.minecart_display_block_state(29),
        Some(MinecartDisplayBlockState {
            block: EntityBlockModelState {
                name: "minecraft:spawner".to_string(),
                properties: BTreeMap::new(),
            },
            display_offset: 6,
        })
    );

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 25,
        values: vec![protocol_bool_data(FURNACE_MINECART_FUEL_DATA_ID, true)],
    }));
    assert_eq!(
        store
            .minecart_display_block_state(25)
            .unwrap()
            .block
            .properties
            .get("lit"),
        Some(&"true".to_string())
    );

    let grass_props = BTreeMap::from([("snowy".to_string(), "false".to_string())]);
    let grass_id = crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:grass_block", &grass_props)
        .expect("vanilla 26.1 grass block state exists")
        .id;
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 23,
        values: vec![
            protocol_optional_block_state_data(
                MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
                Some(grass_id),
            ),
            protocol_int_data(MINECART_DISPLAY_OFFSET_DATA_ID, 3),
        ],
    }));
    assert_eq!(
        store.minecart_display_block_state(23),
        Some(MinecartDisplayBlockState {
            block: EntityBlockModelState {
                name: "minecraft:grass_block".to_string(),
                properties: grass_props,
            },
            display_offset: 3,
        })
    );
}

#[test]
fn falling_block_state_projects_add_entity_block_state_data() {
    let grass_props = BTreeMap::from([("snowy".to_string(), "false".to_string())]);
    let grass_id = crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:grass_block", &grass_props)
        .expect("vanilla 26.1 grass block state exists")
        .id;
    let air_id = crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:air", &BTreeMap::new())
        .expect("vanilla 26.1 air block state exists")
        .id;
    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type_data(
        131,
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        grass_id,
    ));
    store.apply_add_entity(protocol_add_entity_with_type_data(
        132,
        VANILLA_ENTITY_TYPE_COW_ID,
        grass_id,
    ));
    store.apply_add_entity(protocol_add_entity_with_type_data(
        133,
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        air_id,
    ));
    store.apply_add_entity(protocol_add_entity_with_type_data(
        134,
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        -1,
    ));

    assert_eq!(
        store.falling_block_state(131),
        Some(FallingBlockModelState {
            block_state_id: grass_id,
            block: EntityBlockModelState {
                name: "minecraft:grass_block".to_string(),
                properties: grass_props,
            },
        })
    );
    assert_eq!(store.falling_block_state(132), None);
    assert_eq!(store.falling_block_state(133), None);
    assert_eq!(store.falling_block_state(134), None);
}

#[test]
fn primed_tnt_projects_block_state_and_fuse_metadata() {
    const PRIMED_TNT_FUSE_DATA_ID: u8 = 8;
    const PRIMED_TNT_BLOCK_STATE_DATA_ID: u8 = 9;

    let mut store = WorldStore::new();
    store.apply_add_entity(protocol_add_entity_with_type(
        133,
        VANILLA_ENTITY_TYPE_TNT_ID,
    ));
    store.apply_add_entity(protocol_add_entity_with_type(
        134,
        VANILLA_ENTITY_TYPE_COW_ID,
    ));

    assert_eq!(store.primed_tnt_fuse_remaining_in_ticks(134, 0.0), None);
    assert_eq!(
        store.primed_tnt_block_state(133),
        Some(EntityBlockModelState {
            name: "minecraft:tnt".to_string(),
            properties: BTreeMap::from([("unstable".to_string(), "false".to_string())]),
        })
    );
    assert_eq!(
        store.primed_tnt_fuse_remaining_in_ticks(133, 0.5),
        Some(80.5)
    );

    let grass_props = BTreeMap::from([("snowy".to_string(), "false".to_string())]);
    let grass_id = crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:grass_block", &grass_props)
        .expect("vanilla 26.1 grass block state exists")
        .id;
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 133,
        values: vec![
            protocol_int_data(PRIMED_TNT_FUSE_DATA_ID, 4),
            protocol_block_state_data(PRIMED_TNT_BLOCK_STATE_DATA_ID, grass_id),
        ],
    }));

    assert_eq!(
        store.primed_tnt_block_state(133),
        Some(EntityBlockModelState {
            name: "minecraft:grass_block".to_string(),
            properties: grass_props,
        })
    );
    assert_eq!(
        store.primed_tnt_fuse_remaining_in_ticks(133, 0.5),
        Some(4.5)
    );

    let air_id = crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:air", &BTreeMap::new())
        .expect("vanilla 26.1 air block state exists")
        .id;
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 133,
        values: vec![protocol_block_state_data(
            PRIMED_TNT_BLOCK_STATE_DATA_ID,
            air_id,
        )],
    }));
    assert_eq!(store.primed_tnt_block_state(133), None);
}

#[test]
fn minecart_display_blocks_expand_model_culling_bounds() {
    const MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID: u8 = 11;
    const MINECART_DISPLAY_OFFSET_DATA_ID: u8 = 12;

    fn assert_close(actual: f32, expected: f32, label: &str) {
        assert!(
            (actual - expected).abs() < 0.00001,
            "{label}: actual={actual}, expected={expected}"
        );
    }

    fn target_bounds(targets: &[EntityModelTargetState], entity_id: i32) -> EntityPickBoundsState {
        targets
            .iter()
            .find(|target| target.entity_id == entity_id)
            .unwrap_or_else(|| panic!("missing model target {entity_id}"))
            .bounds
    }

    let mut store = WorldStore::new();
    for (id, entity_type_id) in [
        (23, VANILLA_ENTITY_TYPE_MINECART_ID),
        (24, VANILLA_ENTITY_TYPE_TNT_MINECART_ID),
        (25, VANILLA_ENTITY_TYPE_CHEST_MINECART_ID),
        (26, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID),
    ] {
        store.apply_add_entity(protocol_add_entity_with_type(id, entity_type_id));
    }

    let targets = store
        .entities
        .model_targets_at_partial_tick(1.0, &store.registries);
    let plain = target_bounds(&targets, 23);
    let tnt = target_bounds(&targets, 24);
    let chest = target_bounds(&targets, 25);
    let hopper = target_bounds(&targets, 26);

    assert_close(tnt.min[1], plain.min[1], "tnt min y");
    assert_close(
        tnt.max[1] - plain.max[1],
        6.0 * 0.75 / 16.0,
        "tnt display-block culling expansion",
    );
    assert_close(
        chest.max[1] - plain.max[1],
        8.0 * 0.75 / 16.0,
        "chest display-block culling expansion",
    );
    assert_close(
        hopper.max[1] - plain.max[1],
        1.0 * 0.75 / 16.0,
        "hopper display-block culling expansion",
    );

    let grass_props = BTreeMap::from([("snowy".to_string(), "false".to_string())]);
    let grass_id = crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:grass_block", &grass_props)
        .expect("vanilla 26.1 grass block state exists")
        .id;
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 23,
        values: vec![
            protocol_optional_block_state_data(
                MINECART_CUSTOM_DISPLAY_BLOCK_DATA_ID,
                Some(grass_id),
            ),
            protocol_int_data(MINECART_DISPLAY_OFFSET_DATA_ID, -4),
        ],
    }));

    let targets = store
        .entities
        .model_targets_at_partial_tick(1.0, &store.registries);
    let custom = target_bounds(&targets, 23);
    assert_close(custom.max[1], plain.max[1], "negative offset max y");
    assert_close(
        custom.min[1] - plain.min[1],
        -4.0 * 0.75 / 16.0,
        "negative display-block culling expansion",
    );
}
