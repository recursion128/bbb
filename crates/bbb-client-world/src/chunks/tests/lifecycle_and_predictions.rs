use super::*;

#[test]
fn family_columns_follow_arrival_order_across_replacement_and_unload() {
    use crate::chunks::{BlockFamily, ChunkMap};

    let pos = |x| ChunkPos { x, z: 0 };
    let packet = synthetic_level_chunk_packet();
    let mut template = crate::chunks::decode_level_chunk_with_light(
        pos(0),
        packet.chunk_data,
        packet.light_data,
        WorldDimension {
            min_y: 0,
            height: 16,
        },
    )
    .unwrap();
    std::sync::Arc::make_mut(&mut template.block_entities).clear();
    let beacon = test_block_state_id("minecraft:beacon", &[]);
    let air = crate::vanilla_air_block_state_id();
    let column = |x, state_id| {
        let mut column = template.clone();
        column.pos = pos(x);
        let section = std::sync::Arc::make_mut(&mut column.sections[0]);
        section.non_empty_block_count = if state_id == air { 0 } else { 4096 };
        section.block_states.palette_global_ids[0] = state_id;
        column.into()
    };
    let beacons = |map: &ChunkMap| {
        map.with_family(BlockFamily::Beacon)
            .map(|column| column.pos.x)
            .collect::<Vec<_>>()
    };
    let mut map = ChunkMap::default();
    // Columns 37 and 36 straddle the first group boundary in arrival order.
    for x in (36..=100).rev() {
        let state = if matches!(x, 100 | 37 | 36) {
            beacon
        } else {
            air
        };
        map.push(column(x, state));
    }
    assert_eq!(beacons(&map), [100, 37, 36]);
    assert_eq!(map.families(), BlockFamily::Beacon.bit());
    let previous = map.clone();
    map.push(column(35, beacon));
    map.push(column(37, air));
    assert_eq!(beacons(&map), [100, 36, 35]);
    std::sync::Arc::make_mut(&mut map.get_mut(pos(36)).unwrap().sections[0])
        .block_states
        .palette_global_ids[0] = air;
    assert_eq!(beacons(&map), [100, 35]);
    map.remove(pos(99));
    assert_eq!(beacons(&map), [100, 35]);
    map.retain(|column| column.pos.x < 36);
    assert_eq!(beacons(&map), [35]);
    map.push(column(35, air));
    assert_eq!(map.families(), 0);
    assert_eq!(beacons(&previous), [100, 37, 36]);
}

#[test]
fn retained_publication_keeps_exact_section_revision_generation() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 32,
    });
    let chunk = ChunkPos { x: 2, z: -3 };
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet_with_sections(2))
        .unwrap();
    assert_eq!(store.poll_queued_light_updates(), 1);
    let generation_a = store.publication_snapshot();
    let inserted = store.terrain_revision();

    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 34,
            y: 17,
            z: -45,
        },
        block_state_id: 0,
    }));
    let changed = store.terrain_revision();
    let generation_b = store.publication_snapshot();

    assert_eq!(
        terrain_section_revisions(&generation_a, chunk),
        [inserted; 2]
    );
    assert_eq!(
        terrain_section_revisions(&generation_b, chunk),
        [inserted, changed]
    );
}

#[test]
fn probes_chunk_summary_from_world_store() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    let pos = store
        .insert_level_chunk_with_light(synthetic_level_chunk_packet())
        .unwrap();
    assert_eq!(store.poll_queued_light_updates(), 1);

    let summary = store.probe_chunk_summary(pos).unwrap();

    assert_eq!(summary.pos, ChunkPos { x: 1, z: -2 });
    assert_eq!(summary.state, ChunkState::Decoded);
    assert_eq!(summary.heightmaps, 1);
    assert_eq!(summary.sections, 1);
    assert_eq!(summary.block_entities, 1);
    assert_eq!(summary.sky_light_arrays, 1);
    assert_eq!(summary.block_light_arrays, 1);
    assert!(store
        .probe_chunk_summary(ChunkPos { x: 99, z: 99 })
        .is_none());
}

#[test]
fn first_chunk_tracks_first_decoded_chunk_and_survives_forget() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });

    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    store
        .insert_level_chunk_with_light(synthetic_level_chunk_packet())
        .unwrap();

    assert_eq!(store.first_chunk(), Some(ChunkPos { x: 2, z: -3 }));

    assert!(store.forget_chunk(ChunkPos { x: 2, z: -3 }));
    assert_eq!(store.first_chunk(), Some(ChunkPos { x: 2, z: -3 }));
}

#[test]
fn probes_block_state_from_local_palette() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    let probe = store
        .probe_block(BlockPos {
            x: 34,
            y: 1,
            z: -45,
        })
        .unwrap();

    assert_eq!(probe.chunk, ChunkPos { x: 2, z: -3 });
    assert_eq!(probe.local_x, 2);
    assert_eq!(probe.local_y, 1);
    assert_eq!(probe.local_z, 3);
    assert_eq!(probe.section_y, 0);
    assert_eq!(probe.section_index, 0);
    assert_eq!(probe.block_state_id, 9);
    assert_eq!(probe.block_name.as_deref(), Some("minecraft:grass_block"));
    assert_eq!(probe.material, TerrainMaterialClass::Opaque);
    assert_eq!(probe.block_properties.get("snowy").unwrap(), "false");
    assert_eq!(probe.block_palette_kind, PaletteKind::Local);
    assert_eq!(probe.block_palette_index, Some(2));
    assert_eq!(probe.biome_id, Some(4));
    assert_eq!(store.biome_id_at(probe.pos), Some(4));
    assert_eq!(
        store.biome_id_at(BlockPos {
            x: 34,
            y: 16,
            z: -45,
        }),
        None
    );
    assert_eq!(store.biome_id_at(BlockPos { x: 0, y: 1, z: 0 }), None);
    assert_eq!(probe.biome_palette_kind, PaletteKind::SingleValue);

    assert!(store
        .probe_block(BlockPos {
            x: 34,
            y: 16,
            z: -45,
        })
        .is_none());
    assert!(store.probe_block(BlockPos { x: 0, y: 1, z: 0 }).is_none());
}

#[test]
fn multi_block_prediction_validates_every_write_before_mutating() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let loaded = BlockPos {
        x: 34,
        y: 1,
        z: -45,
    };
    let unloaded = BlockPos {
        x: 500,
        y: 1,
        z: 500,
    };

    assert!(!store.predict_local_block_states(&[(loaded, 0), (unloaded, 1)], 7));
    assert_eq!(store.probe_block(loaded).unwrap().block_state_id, 9);
    assert!(store.local_block_predictions().is_empty());
}

#[test]
fn local_destroy_prediction_uses_legacy_fluid_block_state() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let pos = BlockPos {
        x: 34,
        y: 1,
        z: -45,
    };
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 34,
            y: 1,
            z: -45,
        },
        block_state_id: 13332,
    }));
    assert_eq!(
        store.probe_block(pos).unwrap().fluid,
        Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false))
    );

    assert!(store.predict_local_destroy_block(pos, 7));
    let predicted = store.probe_block(pos).unwrap();
    assert_eq!(predicted.block_name.as_deref(), Some("minecraft:water"));
    assert_eq!(
        predicted.block_properties.get("level").map(String::as_str),
        Some("0")
    );
    assert_eq!(predicted.material, TerrainMaterialClass::Fluid);
    assert_eq!(
        store.local_block_predictions()[0].server_block_state_id,
        13332
    );
    assert_eq!(
        store.local_block_predictions()[0].predicted_block_state_id,
        predicted.block_state_id
    );
}

#[test]
fn invalid_block_state_update_cannot_rewrite_pending_prediction() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let pos = BlockPos {
        x: 34,
        y: 1,
        z: -45,
    };
    assert!(store.predict_local_destroy_block(pos, 7));
    let known_server_state = store.local_block_predictions()[0].server_block_state_id;

    assert!(!store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: pos.x,
            y: pos.y,
            z: pos.z
        },
        block_state_id: bbb_protocol::block_state_types::VANILLA_BLOCK_STATE_COUNT,
    }));
    assert_eq!(
        store.local_block_predictions()[0].server_block_state_id,
        known_server_state
    );
    assert_eq!(
        store
            .counters()
            .local_block_predictions_reconciled_by_update,
        0
    );
    assert_eq!(store.counters().block_updates_ignored, 1);
}

#[test]
fn applies_block_entity_data_update() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    let empty_revision = store.stable_block_entity_render_revision();
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let pos = ProtocolBlockPos {
        x: 33,
        y: 7,
        z: -46,
    };
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos,
        block_state_id: test_block_state_id("minecraft:spawner", &[]),
    }));
    let before_data = store.stable_block_entity_render_revision();
    assert!(before_data.terrain > empty_revision.terrain);
    assert_eq!(
        store.clone().stable_block_entity_render_revision(),
        before_data
    );

    let raw_nbt = nbt_compound_with_string("id", "minecraft:chest");
    let applied = store
        .apply_block_entity_data(ProtocolBlockEntityData {
            pos,
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
            raw_nbt: raw_nbt.clone(),
        })
        .unwrap();

    assert!(applied);
    let after_data = store.stable_block_entity_render_revision();
    assert_eq!(after_data.terrain, before_data.terrain);
    assert_eq!(
        after_data.block_entity_data,
        before_data.block_entity_data + 1
    );
    assert_eq!(store.counters().block_entity_updates_received, 1);
    assert_eq!(store.counters().block_entity_updates_applied, 1);
    assert_eq!(store.counters().block_entity_updates_ignored, 0);

    let chunk = store.probe_chunk(ChunkPos { x: 2, z: -3 }).unwrap();
    assert_eq!(chunk.block_entities.len(), 1);
    assert_eq!(
        chunk.block_entities[0],
        BlockEntityRecord {
            local_x: 1,
            y: 7,
            local_z: 2,
            type_id: bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
            raw_nbt: raw_nbt.clone(),
            nbt: Some(NbtPayloadSummary {
                root_type: 10,
                byte_len: raw_nbt.len(),
            }),
            payload: crate::BlockEntityPayload::None,
        }
    );
    assert_eq!(
        store.block_entity_type_id_at(BlockPos {
            x: 33,
            y: 7,
            z: -46,
        }),
        Some(9)
    );
    assert_eq!(
        store.block_entity_raw_nbt_at(BlockPos {
            x: 33,
            y: 7,
            z: -46,
        }),
        Some(raw_nbt.as_slice())
    );
    assert_eq!(
        store.block_entity_type_id_at(BlockPos {
            x: 34,
            y: 7,
            z: -46,
        }),
        None
    );

    assert!(!store
        .apply_block_entity_data(ProtocolBlockEntityData {
            pos,
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_PISTON_ID,
            raw_nbt: vec![0],
        })
        .unwrap());
    let chunk = store.probe_chunk(ChunkPos { x: 2, z: -3 }).unwrap();
    assert_eq!(chunk.block_entities.len(), 1);
    assert_eq!(
        chunk.block_entities[0].type_id,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID
    );
    assert_eq!(
        store.block_entity_raw_nbt_at(BlockPos {
            x: 33,
            y: 7,
            z: -46,
        }),
        Some(raw_nbt.as_slice())
    );
    assert_eq!(
        store.block_entity_type_id_at(BlockPos {
            x: 33,
            y: 7,
            z: -46,
        }),
        Some(bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID)
    );
    assert_eq!(store.stable_block_entity_render_revision(), after_data);

    let missing_chunk_applied = store
        .apply_block_entity_data(ProtocolBlockEntityData {
            pos: ProtocolBlockPos {
                x: 800,
                y: 7,
                z: -46,
            },
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
            raw_nbt: vec![0],
        })
        .unwrap();
    assert!(!missing_chunk_applied);
    assert_eq!(store.counters().block_entity_updates_received, 3);
    assert_eq!(store.counters().block_entity_updates_applied, 1);
    assert_eq!(store.counters().block_entity_updates_ignored, 2);
    assert_eq!(store.stable_block_entity_render_revision(), after_data);
}

#[test]
fn applies_vault_shared_data_and_projects_connection_particles() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let vault_pos = ProtocolBlockPos {
        x: 33,
        y: 7,
        z: -46,
    };
    let connected_uuid = Uuid::from_u128(0x0011_2233_4455_6677_8899_aabb_ccdd_eeff);
    let distant_uuid = Uuid::from_u128(0xffee_ddcc_bbaa_9988_7766_5544_3322_1100);
    let raw_nbt = vault_shared_data_nbt(&[connected_uuid, distant_uuid], Some(3.0));

    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: vault_pos,
        block_state_id: vault_block_state_id("east"),
    }));
    assert!(store
        .apply_block_entity_data(ProtocolBlockEntityData {
            pos: vault_pos,
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
            raw_nbt,
        })
        .unwrap());
    store.apply_add_entity(vault_test_player(
        10,
        connected_uuid,
        ProtocolVec3d {
            x: 35.2,
            y: 7.0,
            z: -46.0,
        },
    ));
    store.apply_add_entity(vault_test_player(
        11,
        distant_uuid,
        ProtocolVec3d {
            x: 42.0,
            y: 7.0,
            z: -46.0,
        },
    ));

    let shared_data = store
        .vault_shared_data_at(BlockPos {
            x: 33,
            y: 7,
            z: -46,
        })
        .unwrap();
    assert_eq!(
        shared_data.connected_players,
        vec![connected_uuid, distant_uuid]
    );
    assert_eq!(shared_data.connected_particles_range, 3.0);

    let particles = store
        .vault_connection_particle_state(BlockPos {
            x: 33,
            y: 7,
            z: -46,
        })
        .unwrap();
    assert_eq!(particles.origin, [34.0, 8.75, -45.5]);
    assert_eq!(particles.targets.len(), 1);
    assert_eq!(particles.targets[0].entity_id, 10);
    assert_eq!(particles.targets[0].uuid, connected_uuid);
    assert_close_f64(particles.targets[0].target_position[0], 35.2);
    assert_close_f64(particles.targets[0].target_position[1], 7.9);
    assert_close_f64(particles.targets[0].target_position[2], -46.0);
}

#[test]
fn forgets_loaded_chunk() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    assert!(store.forget_chunk(ChunkPos { x: 2, z: -3 }));
    assert_eq!(store.counters().chunk_forgets_received, 1);
    assert_eq!(store.counters().chunks_forgotten, 1);
    assert_eq!(store.chunk_count(), 0);
    assert!(store.probe_chunk(ChunkPos { x: 2, z: -3 }).is_none());
    assert!(store
        .probe_block(BlockPos {
            x: 34,
            y: 1,
            z: -45,
        })
        .is_none());
    assert!(store.extract_terrain_chunks().is_empty());
}

#[test]
fn forget_missing_chunk_is_counted_but_not_applied() {
    let mut store = WorldStore::new();

    assert!(!store.forget_chunk(ChunkPos { x: 2, z: -3 }));
    assert_eq!(store.counters().chunk_forgets_received, 1);
    assert_eq!(store.counters().chunks_forgotten, 0);
    assert_eq!(store.counters().chunk_forgets_ignored, 1);
    assert_eq!(store.chunk_count(), 0);
}
