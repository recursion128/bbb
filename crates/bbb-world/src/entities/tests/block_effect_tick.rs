use super::*;

const ENTITY_ID: i32 = 4401;

#[test]
fn loaded_cobweb_applies_to_direct_item_tick_and_reports_narrow_work() {
    let mut world = item_world_with_block("minecraft:cobweb");

    let work = world.advance_entity_client_animations_with_observation(1);
    let state = world
        .entities
        .block_effects_debug_state(ENTITY_ID)
        .expect("item block effect state");

    assert_eq!(work.block_effect_entities, 1);
    assert!(work.block_effect_blocks_visited >= 1);
    assert_eq!(work.block_effect_callbacks_applied, 1);
    assert_eq!(
        state.stuck_speed_multiplier,
        EntityVec3 {
            x: 0.25,
            y: 0.05,
            z: 0.25,
        }
    );
    assert_eq!(state.fall_distance, 0.0);
    assert_eq!(state.movement_this_tick, 0);
    assert!(state.final_movements >= 1);
}

#[test]
fn retained_publication_keeps_private_block_effect_generation_isolated() {
    let mut live = item_world_with_block("minecraft:cobweb");
    let generation_a = live.publication_snapshot();
    let state_a_before = generation_a
        .entities
        .block_effects_debug_state(ENTITY_ID)
        .expect("generation A state");

    live.advance_entity_client_animations(1);
    let state_b = live
        .entities
        .block_effects_debug_state(ENTITY_ID)
        .expect("generation B state");

    assert_ne!(state_b, state_a_before);
    assert_eq!(
        generation_a.entities.block_effects_debug_state(ENTITY_ID),
        Some(state_a_before)
    );
    assert_eq!(state_a_before.stuck_speed_multiplier, EntityVec3::default());
    assert_eq!(
        state_b.stuck_speed_multiplier,
        EntityVec3 {
            x: 0.25,
            y: 0.05,
            z: 0.25,
        }
    );
}

#[test]
fn unloaded_world_skips_movement_history_and_terrain_callbacks() {
    let mut world = WorldStore::new();
    world.apply_add_entity(item_packet());

    let work = world.advance_entity_client_animations_with_observation(1);
    let state = world
        .entities
        .block_effects_debug_state(ENTITY_ID)
        .expect("item block effect state");

    assert_eq!(work.block_effect_duration, std::time::Duration::ZERO);
    assert_eq!(work.block_effect_entities, 0);
    assert_eq!(work.block_effect_blocks_visited, 0);
    assert_eq!(work.block_effect_callbacks_applied, 0);
    assert_eq!(state.movement_this_tick, 0);
    assert_eq!(state.final_movements, 0);
}

#[test]
fn same_protocol_id_recreation_discards_private_block_effect_runtime() {
    let mut world = item_world_with_block("minecraft:cobweb");
    world.advance_entity_client_animations(1);
    assert_ne!(
        world
            .entities
            .block_effects_debug_state(ENTITY_ID)
            .expect("mutated state")
            .stuck_speed_multiplier,
        EntityVec3::default()
    );

    world.apply_add_entity(ProtocolAddEntity {
        uuid: Uuid::from_u128(0xfeed),
        ..item_packet()
    });

    let reset = world
        .entities
        .block_effects_debug_state(ENTITY_ID)
        .expect("replacement state");
    assert_eq!(reset.stuck_speed_multiplier, EntityVec3::default());
    assert_eq!(reset.movement_this_tick, 0);
    assert_eq!(reset.final_movements, 0);
}

#[test]
fn boat_runs_the_two_vanilla_callback_passes() {
    let mut world =
        world_with_entity_and_block(VANILLA_ENTITY_TYPE_ACACIA_BOAT_ID, "minecraft:cobweb");

    let work = world.advance_entity_client_animations_with_observation(1);

    assert_eq!(work.block_effect_entities, 2);
    assert_eq!(work.block_effect_callbacks_applied, 2);
}

#[test]
fn attached_firework_skips_and_detached_firework_runs_callback_pass() {
    let mut world =
        world_with_entity_and_block(VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID, "minecraft:cobweb");
    let accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
    )
    .expect("generated firework accessors");
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: ENTITY_ID,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.attached_to_target_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
            value: EntityDataValueKind::OptionalUnsignedInt(Some(7)),
        }],
    }));
    assert_eq!(
        world
            .advance_entity_client_animations_with_observation(1)
            .block_effect_entities,
        0
    );

    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: ENTITY_ID,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.attached_to_target_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
            value: EntityDataValueKind::OptionalUnsignedInt(None),
        }],
    }));
    let detached = world.advance_entity_client_animations_with_observation(1);
    assert_eq!(detached.block_effect_entities, 1);
    assert_eq!(detached.block_effect_callbacks_applied, 1);
}

#[test]
fn generated_fire_immunity_prevents_client_private_ignition() {
    let mut world = world_with_entity_and_block(VANILLA_ENTITY_TYPE_TNT_ID, "minecraft:fire");

    let work = world.advance_entity_client_animations_with_observation(1);
    let state = world
        .entities
        .block_effects_debug_state(ENTITY_ID)
        .expect("TNT block effect state");

    assert_eq!(work.block_effect_entities, 1);
    assert_eq!(work.block_effect_callbacks_applied, 1);
    assert_eq!(state.remaining_fire_ticks, 0);
}

#[test]
fn powder_snow_freeze_mutation_is_visible_only_in_next_generation() {
    let mut live = item_world_with_block("minecraft:powder_snow");
    let generation_a = live.publication_snapshot();

    live.advance_entity_client_animations(1);

    assert_eq!(generation_a.entities.ticks_frozen(ENTITY_ID), Some(0));
    assert_eq!(live.entities.ticks_frozen(ENTITY_ID), Some(1));
    assert!(
        live.entities
            .block_effects_debug_state(ENTITY_ID)
            .expect("item block effect state")
            .in_powder_snow
    );
}

#[test]
fn in_ground_arrow_keeps_private_fire_without_base_tick() {
    let mut world = world_with_entity_and_block(VANILLA_ENTITY_TYPE_ARROW_ID, "minecraft:fire");
    set_arrow_in_ground(&mut world, ENTITY_ID);

    let work = world.advance_entity_client_animations_with_observation(1);
    assert!(work.block_effect_callbacks_applied >= 1);
    let ignited = world
        .entities
        .block_effects_debug_state(ENTITY_ID)
        .expect("stuck arrow state")
        .remaining_fire_ticks;
    assert!(ignited > 0);

    // The stuck branch never reaches `super.tick()`, so the private timer
    // survives the tick boundary instead of being cleared by `baseTick`.
    world.advance_entity_client_animations(1);
    assert!(
        world
            .entities
            .block_effects_debug_state(ENTITY_ID)
            .expect("stuck arrow state")
            .remaining_fire_ticks
            > 0
    );
}

#[test]
fn flight_arrow_base_tick_clears_ignited_fire_after_callbacks() {
    let mut world = world_with_entity_and_block(VANILLA_ENTITY_TYPE_ARROW_ID, "minecraft:fire");

    let work = world.advance_entity_client_animations_with_observation(1);

    // The fire callback ran against the explicit flight segment, and the
    // end-of-tick `super.tick()` client `baseTick` then cleared the timer.
    assert!(work.block_effect_callbacks_applied >= 1);
    assert_eq!(
        world
            .entities
            .block_effects_debug_state(ENTITY_ID)
            .expect("flying arrow state")
            .remaining_fire_ticks,
        0
    );
}

#[test]
fn returning_trident_no_physics_field_skips_block_effects() {
    // The fixture trident (ENTITY_ID) is the stuck control; the returning
    // trident spawns beside it in the same fire column with its owner link.
    let mut world = world_with_entity_and_block(VANILLA_ENTITY_TYPE_TRIDENT_ID, "minecraft:fire");
    set_arrow_in_ground(&mut world, ENTITY_ID);
    let owner_id = ENTITY_ID + 1;
    world.apply_add_entity(ProtocolAddEntity {
        id: owner_id,
        uuid: Uuid::from_u128(0xa11ce),
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: ProtocolVec3d {
            x: 12.0,
            y: 2.0,
            z: 8.5,
        },
        ..item_packet()
    });
    let returning_id = ENTITY_ID + 2;
    world.apply_add_entity(ProtocolAddEntity {
        id: returning_id,
        uuid: Uuid::from_u128(0xc0117501),
        entity_type_id: VANILLA_ENTITY_TYPE_TRIDENT_ID,
        data: owner_id,
        ..item_packet()
    });
    let arrow_accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    )
    .expect("generated abstract arrow accessors");
    let trident_accessors = vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_TRIDENT_ID,
    )
    .expect("generated trident accessors");
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: returning_id,
        values: vec![
            ProtocolEntityDataValue {
                data_id: arrow_accessors.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(arrow_accessors.no_physics_mask),
            },
            ProtocolEntityDataValue {
                data_id: trident_accessors.loyalty_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(2),
            },
        ],
    }));

    let work = world.advance_entity_client_animations_with_observation(1);

    // The returning trident set the client `noPhysics` field this tick, so
    // `isAffectedByBlocks` skipped its callback while the stuck control
    // trident in the same fire column still ignited.
    assert!(work.block_effect_callbacks_applied >= 1);
    assert_eq!(
        world
            .entities
            .block_effects_debug_state(returning_id)
            .expect("returning trident state")
            .remaining_fire_ticks,
        0
    );
    assert!(
        world
            .entities
            .block_effects_debug_state(ENTITY_ID)
            .expect("stuck control trident state")
            .remaining_fire_ticks
            > 0
    );
}

#[test]
fn rain_clears_stuck_arrow_fire_at_the_arrow_tick_head() {
    let mut world = world_with_entity_and_block(VANILLA_ENTITY_TYPE_ARROW_ID, "minecraft:fire");
    set_arrow_in_ground(&mut world, ENTITY_ID);
    world.advance_entity_client_animations(1);
    assert!(
        world
            .entities
            .block_effects_debug_state(ENTITY_ID)
            .expect("stuck arrow state")
            .remaining_fire_ticks
            > 0
    );

    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 8, y: 2, z: 8 },
        block_state_id: crate::vanilla_air_block_state_id(),
    }));
    world.record_registry_entries(
        "minecraft:worldgen/biome",
        0,
        vec![crate::registries::RegistryPacketEntry::with_raw_data(
            "minecraft:plains",
            crate::registries::test_biome_element_nbt(0.8, true),
        )],
    );
    world.level = Some(crate::WorldLevelInfo {
        dimension: "minecraft:overworld".to_string(),
        dimension_type_id: 0,
        dimension_type_name: Some("minecraft:overworld".to_string()),
        biome_zoom_seed: 0,
        default_clock_resource_id: None,
        timeline_resource_ids: Vec::new(),
        has_sky_light: true,
        has_ceiling: false,
        cardinal_lighting: crate::WorldCardinalLighting::Default,
        skybox: crate::WorldSkybox::Overworld,
        ambient_light: 0.0,
        environment_attributes: crate::ClientEnvironmentAttributes::default(),
        last_death_location: None,
        sea_level: 63,
        is_debug: false,
        is_flat: false,
    });
    world.weather.rain_level = 1.0;

    // `AbstractArrow.tick` head: `if (this.isInWaterOrRain()) this.clearFire();`
    // runs even for the stuck branch that never reaches `baseTick`.
    world.advance_entity_client_animations(1);
    assert_eq!(
        world
            .entities
            .block_effects_debug_state(ENTITY_ID)
            .expect("rained-on arrow state")
            .remaining_fire_ticks,
        0
    );
}

fn set_arrow_in_ground(world: &mut WorldStore, entity_id: i32) {
    let entity_type_id = world
        .entities
        .entity_type_id(entity_id)
        .expect("arrow entity type");
    let accessors =
        vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(entity_type_id)
            .expect("generated abstract arrow accessors");
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: entity_id,
        values: vec![ProtocolEntityDataValue {
            data_id: accessors.in_ground_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
}

fn item_world_with_block(block_name: &str) -> WorldStore {
    world_with_entity_and_block(VANILLA_ENTITY_TYPE_ITEM_ID, block_name)
}

fn world_with_entity_and_block(entity_type_id: i32, block_name: &str) -> WorldStore {
    let mut world = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    world.insert_decoded_chunk(empty_test_chunk());
    let block_state_id = crate::BlockStateRegistry::vanilla_26_1()
        .iter()
        .find(|state| state.name == block_name)
        .unwrap_or_else(|| panic!("missing Vanilla block state {block_name}"))
        .id;
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 8, y: 2, z: 8 },
        block_state_id,
    }));
    world.apply_add_entity(ProtocolAddEntity {
        entity_type_id,
        ..item_packet()
    });
    world
}

fn item_packet() -> ProtocolAddEntity {
    ProtocolAddEntity {
        id: ENTITY_ID,
        uuid: default_entity_uuid(),
        entity_type_id: VANILLA_ENTITY_TYPE_ITEM_ID,
        position: ProtocolVec3d {
            x: 8.5,
            y: 2.0,
            z: 8.5,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}
