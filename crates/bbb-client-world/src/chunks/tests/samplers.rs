use super::*;

#[test]
fn block_terrain_revisions_follow_three_dimensional_section_boundaries() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 48,
    });
    let chunk = ChunkPos { x: 2, z: -3 };
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet_with_sections(3))
        .unwrap();
    let inserted = store.terrain_revision();

    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 34,
            y: 17,
            z: -45,
        },
        block_state_id: 0,
    }));
    let interior = store.terrain_revision();
    assert_eq!(
        terrain_section_revisions(&store, chunk),
        [inserted, interior, inserted]
    );
    assert_eq!(
        terrain_section_revisions(&store, ChunkPos { x: 1, z: -3 }),
        [inserted; 3]
    );

    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 32,
            y: 15,
            z: -48,
        },
        block_state_id: 9,
    }));
    let corner = store.terrain_revision();
    for pos in [
        chunk,
        ChunkPos { x: 1, z: -3 },
        ChunkPos { x: 2, z: -4 },
        ChunkPos { x: 1, z: -4 },
    ] {
        assert_eq!(
            terrain_section_revisions(&store, pos),
            [corner, corner, inserted]
        );
    }
    assert_eq!(
        terrain_section_revisions(&store, ChunkPos { x: 3, z: -3 }),
        [inserted; 3]
    );
}

#[test]
fn samples_terrain_light_from_packet_layers() {
    let dimension = WorldDimension {
        min_y: 0,
        height: 16,
    };
    let index = section_block_index(2, 1, 3);
    let mut sky = [0; LIGHT_ARRAY_BYTES];
    let mut block = [0; LIGHT_ARRAY_BYTES];
    set_light_nibble(&mut sky, index, 12);
    set_light_nibble(&mut block, index, 7);
    let mut light = LightData::default();
    light.set_sky_packed(1, sky);
    light.set_block_packed(1, block);

    assert_eq!(
        sample_terrain_light(&light, dimension, 2, 1, 3),
        TerrainLight { sky: 12, block: 7 }
    );
}

#[test]
fn terrain_light_empty_masks_override_fallback() {
    let dimension = WorldDimension {
        min_y: 0,
        height: 16,
    };
    let mut light = LightData::default();
    light.set_sky_empty(1);
    light.set_block_empty(1);

    assert_eq!(
        sample_terrain_light(&light, dimension, 2, 1, 3),
        TerrainLight::DARK
    );
}

#[test]
fn terrain_light_respects_no_sky_light_below_world_dark_and_inherited_sky_slice() {
    let dimension = WorldDimension {
        min_y: 0,
        height: 64,
    };
    // Dimensions without sky light (Nether/End) report sky = 0 everywhere,
    // including `section_refs` consumed by terrain meshing.
    let mut nether_light = LightData::default();
    nether_light.set_has_sky_light(false);
    assert_eq!(
        sample_terrain_light(&nether_light, dimension, 2, 10, 3),
        TerrainLight::DARK
    );
    assert_eq!(
        nether_light.section_refs(1).0,
        bbb_render_types::TerrainSectionLightRef::Uniform(0)
    );

    // Below `min_y - 16` reports dark instead of full bright.
    let mut overworld = LightData::default();
    assert_eq!(
        sample_terrain_light(&overworld, dimension, 2, -17, 3),
        TerrainLight::DARK
    );

    // Uninitialized Overworld sky section below a Packed section inherits the
    // bottom `(x, 0, z)` slice of the nearest stored section above, while
    // sections above the highest stored section stay sky = 15.
    let mut above_sky = [0u8; LIGHT_ARRAY_BYTES];
    let shaded_bottom = section_block_index(2, 0, 3);
    let lit_bottom = section_block_index(4, 0, 5);
    let upper_only = section_block_index(2, 7, 3);
    set_light_nibble(&mut above_sky, shaded_bottom, 0);
    set_light_nibble(&mut above_sky, lit_bottom, 11);
    set_light_nibble(&mut above_sky, upper_only, 15);
    // Light section index 3 corresponds to section_y = 2 (y in 32..48).
    // Section index 1 (y in 0..16) and index 2 (y in 16..32) stay Uninitialized.
    overworld.set_sky_packed(3, above_sky);

    assert_eq!(sample_terrain_light(&overworld, dimension, 2, 5, 3).sky, 0);
    assert_eq!(
        sample_terrain_light(&overworld, dimension, 4, 21, 5).sky,
        11
    );
    assert_eq!(
        sample_terrain_light(&overworld, dimension, 2, 50, 3).sky,
        15
    );
    let (section1_sky, _) = overworld.section_refs(1);
    assert_eq!(section1_sky.get(section_block_index(2, 5, 3)), 0);
    assert_eq!(section1_sky.get(section_block_index(4, 9, 5)), 11);

    // Uninitialized section below an Empty sky section inherits Uniform(0).
    let mut roof_empty = LightData::default();
    roof_empty.set_sky_empty(3);
    assert_eq!(sample_terrain_light(&roof_empty, dimension, 2, 5, 3).sky, 0);
    assert_eq!(
        roof_empty.section_refs(1).0,
        bbb_render_types::TerrainSectionLightRef::Uniform(0)
    );
}

#[test]
fn applies_light_update_to_existing_chunk_sections() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let index = section_block_index(2, 1, 3);
    let mut sky = vec![0; LIGHT_ARRAY_BYTES];
    let mut block = vec![0; LIGHT_ARRAY_BYTES];
    set_light_nibble(&mut sky, index, 4);
    set_light_nibble(&mut block, index, 13);

    let applied = apply_light_update_now(
        &mut store,
        ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(&[0b10], &[0b10], &[], &[], vec![sky], vec![block]),
        },
    )
    .unwrap();

    assert!(applied);
    assert_eq!(store.counters().light_updates_received, 1);
    assert_eq!(store.counters().light_updates_applied, 1);
    assert_eq!(store.counters().light_updates_ignored, 0);
    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    assert_eq!(
        terrain.cells[terrain_cell_index(2, 1, 3, 16)].light,
        TerrainLight { sky: 4, block: 13 }
    );

    let applied = apply_light_update_now(
        &mut store,
        ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(&[], &[], &[], &[0b10], Vec::new(), Vec::new()),
        },
    )
    .unwrap();

    assert!(applied);
    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    assert_eq!(
        terrain.cells[terrain_cell_index(2, 1, 3, 16)].light,
        TerrainLight { sky: 4, block: 0 }
    );
}

#[test]
fn debug_sky_light_section_status_tracks_data_and_neighbor_sections() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    assert_eq!(store.debug_sky_light_section_status([2, 0, -3]), "0");
    assert_eq!(store.debug_sky_light_section_status([1, 0, -3]), "1");
    assert_eq!(store.debug_sky_light_section_status([0, 0, -3]), "2");
    assert_eq!(
        store.debug_sky_light_section_occupancy([2, 0, -3]),
        [true, true]
    );
    assert_eq!(
        store.debug_sky_light_section_occupancy([1, 0, -3]),
        [false, true]
    );
    assert_eq!(
        store.debug_sky_light_section_occupancy([0, 0, -3]),
        [false, false]
    );
}

#[test]
fn debug_light_sampling_reads_the_light_engine_vertical_padding() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let index = section_block_index(2, 15, 3);
    let mut sky = vec![0; LIGHT_ARRAY_BYTES];
    let mut block = vec![0; LIGHT_ARRAY_BYTES];
    set_light_nibble(&mut sky, index, 4);
    set_light_nibble(&mut block, index, 13);
    apply_light_update_now(
        &mut store,
        ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(&[0b1], &[0b1], &[], &[], vec![sky], vec![block]),
        },
    )
    .unwrap();
    let below_world = BlockPos {
        x: 34,
        y: -1,
        z: -45,
    };

    assert_eq!(store.sample_block_light(below_world), None);
    assert_eq!(
        store.sample_debug_light(below_world),
        Some(TerrainLight { sky: 4, block: 13 })
    );
    assert_eq!(
        store.sample_debug_light(BlockPos {
            y: -17,
            ..below_world
        }),
        None
    );
}

#[test]
fn samples_motion_blocking_height_from_chunk_heightmap() {
    let dimension = WorldDimension {
        min_y: 0,
        height: 16,
    };
    let mut packet = synthetic_local_palette_chunk_packet();
    packet.chunk_data.heightmaps = vec![motion_blocking_heightmap(dimension, &[(2, 3, 2)])];
    let mut store = WorldStore::with_dimension(dimension);
    store.insert_level_chunk_with_light(packet).unwrap();

    // Chunk (2, -3) local (2, 3) maps to world column (34, -45).
    assert_eq!(store.sample_motion_blocking_height(34, -45), Some(2));
    assert_eq!(store.sample_motion_blocking_height(9999, 9999), None);
}

#[test]
fn samples_requested_heightmap_kind_from_chunk_heightmap() {
    let dimension = WorldDimension {
        min_y: 0,
        height: 16,
    };
    let mut packet = synthetic_local_palette_chunk_packet();
    packet.chunk_data.heightmaps = vec![ChunkHeightmapData {
        kind_id: VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        data: motion_blocking_heightmap(dimension, &[(2, 3, 5)]).data,
    }];
    let mut store = WorldStore::with_dimension(dimension);
    store.insert_level_chunk_with_light(packet).unwrap();

    assert_eq!(
        store.sample_heightmap_first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, 34, -45),
        Some(5)
    );
    assert_eq!(
        store.sample_heightmap_first_available(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, 34, -45),
        None
    );
}

#[test]
fn chunk_heightmap_sampler_requires_loaded_chunk() {
    let store = WorldStore::new();

    assert!(store
        .chunk_heightmap_sampler(ChunkPos { x: 2, z: -3 })
        .is_none());
}

#[test]
fn chunk_heightmap_sampler_exposes_default_final_heightmaps() {
    let dimension = WorldDimension {
        min_y: -32,
        height: 16,
    };
    let mut store = WorldStore::with_dimension(dimension);
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    let sampler = store
        .chunk_heightmap_sampler(ChunkPos { x: 2, z: -3 })
        .unwrap();

    let final_kind_ids = VANILLA_HEIGHTMAP_TYPES
        .iter()
        .filter(|entry| entry.usage != HeightmapUsage::Worldgen)
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    assert_eq!(
        sampler.effective_kind_ids().collect::<Vec<_>>(),
        final_kind_ids
    );
    for kind_id in final_kind_ids {
        assert_eq!(sampler.first_available(kind_id, 2, 3), Some(-32));
    }
    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID, 2, 3),
        None
    );
    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID, 2, 3),
        None
    );
    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, 16, 3),
        None
    );
    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, 2, 16),
        None
    );
}

#[test]
fn chunk_heightmap_sampler_orders_and_samples_all_vanilla_kinds() {
    let dimension = WorldDimension {
        min_y: -32,
        height: 16,
    };
    let mut packet = synthetic_local_palette_chunk_packet();
    packet.chunk_data.heightmaps = VANILLA_HEIGHTMAP_TYPES
        .iter()
        .map(|entry| {
            let kind_id = entry.id;
            let first_available = dimension.min_y + kind_id + 1;
            let entries = if kind_id == VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID {
                vec![(2, 3, first_available), (3, 2, dimension.min_y + 15)]
            } else {
                vec![(2, 3, first_available)]
            };
            heightmap_data(kind_id, dimension, &entries)
        })
        .collect();
    let mut store = WorldStore::with_dimension(dimension);
    store.insert_level_chunk_with_light(packet).unwrap();

    let sampler = store
        .chunk_heightmap_sampler(ChunkPos { x: 2, z: -3 })
        .unwrap();

    assert_eq!(
        sampler.effective_kind_ids().collect::<Vec<_>>(),
        VANILLA_HEIGHTMAP_TYPES
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>()
    );
    for kind_id in VANILLA_HEIGHTMAP_TYPES.iter().map(|entry| entry.id) {
        assert_eq!(
            sampler.first_available(kind_id, 2, 3),
            Some(dimension.min_y + kind_id + 1)
        );
    }
    // Vanilla's local column index is x + z * 16, not z + x * 16.
    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID, 3, 2),
        Some(-17)
    );
    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID, 2, 3),
        Some(-31)
    );
}

#[test]
fn chunk_heightmap_sampler_uses_last_duplicate_kind() {
    let dimension = WorldDimension {
        min_y: -32,
        height: 16,
    };
    let mut packet = synthetic_local_palette_chunk_packet();
    packet.chunk_data.heightmaps = vec![
        heightmap_data(
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            dimension,
            &[(2, 3, -30)],
        ),
        heightmap_data(
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            dimension,
            &[(2, 3, -25)],
        ),
    ];
    let mut store = WorldStore::with_dimension(dimension);
    store.insert_level_chunk_with_light(packet).unwrap();

    let sampler = store
        .chunk_heightmap_sampler(ChunkPos { x: 2, z: -3 })
        .unwrap();

    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, 2, 3),
        Some(-25)
    );
    assert_eq!(
        store.sample_heightmap_first_available(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, 34, -45),
        Some(-25)
    );
}

#[test]
fn chunk_heightmap_sampler_does_not_replace_malformed_actual_maps() {
    let dimension = WorldDimension {
        min_y: -32,
        height: 16,
    };
    let mut packet = synthetic_local_palette_chunk_packet();
    packet.chunk_data.heightmaps = vec![
        ChunkHeightmapData {
            kind_id: VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
            data: vec![0],
        },
        ChunkHeightmapData {
            kind_id: VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            data: vec![0],
        },
    ];
    let mut store = WorldStore::with_dimension(dimension);
    store.insert_level_chunk_with_light(packet).unwrap();

    let sampler = store
        .chunk_heightmap_sampler(ChunkPos { x: 2, z: -3 })
        .unwrap();

    assert_eq!(
        sampler.effective_kind_ids().collect::<Vec<_>>(),
        vec![
            VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
            VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
            VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
        ]
    );
    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID, 2, 3),
        None
    );
    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, 2, 3),
        None
    );
    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, 2, 3),
        Some(-32)
    );
}

#[test]
fn motion_blocking_height_ignores_missing_or_malformed_heightmap() {
    let dimension = WorldDimension {
        min_y: 0,
        height: 16,
    };
    let mut wrong_kind = synthetic_local_palette_chunk_packet();
    wrong_kind.chunk_data.heightmaps = vec![ChunkHeightmapData {
        kind_id: VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        data: motion_blocking_heightmap(dimension, &[(2, 3, 2)]).data,
    }];
    let mut store = WorldStore::with_dimension(dimension);
    store.insert_level_chunk_with_light(wrong_kind).unwrap();
    assert_eq!(store.sample_motion_blocking_height(34, -45), None);

    let mut malformed = synthetic_local_palette_chunk_packet();
    malformed.chunk_data.heightmaps = vec![ChunkHeightmapData {
        kind_id: VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        data: vec![0],
    }];
    let mut store = WorldStore::with_dimension(dimension);
    store.insert_level_chunk_with_light(malformed).unwrap();
    assert_eq!(store.sample_motion_blocking_height(34, -45), None);
}

#[test]
fn light_update_for_missing_chunk_is_counted_but_not_applied() {
    let mut store = WorldStore::new();

    let applied = apply_light_update_now(
        &mut store,
        ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(&[], &[], &[], &[], Vec::new(), Vec::new()),
        },
    )
    .unwrap();

    assert!(!applied);
    assert_eq!(store.counters().light_updates_received, 1);
    assert_eq!(store.counters().light_updates_applied, 0);
    assert_eq!(store.counters().light_updates_ignored, 1);
}

#[test]
fn biome_update_for_missing_chunk_is_counted_but_not_applied() {
    let mut store = WorldStore::new();

    let applied = store
        .apply_biome_update(ProtocolChunksBiomes {
            chunks: vec![ProtocolChunkBiomeData {
                pos: ProtocolChunkPos { x: 2, z: -3 },
                raw_biomes: Vec::new(),
            }],
        })
        .unwrap();

    assert_eq!(applied, 0);
    assert_eq!(store.counters().biome_updates_received, 1);
    assert_eq!(store.counters().biome_updates_applied, 0);
    assert_eq!(store.counters().biome_updates_ignored, 1);
}

#[test]
fn chunk_biome_sampler_reads_neighbourhood_and_truncates_unloaded_columns() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    let sampler = store.chunk_biome_sampler(ChunkPos { x: 2, z: -3 });

    // In-chunk sample matches the per-block biome probe (single-value biome 4).
    assert_eq!(sampler.biome_id_at(34, 1, -45), Some(4));
    assert_eq!(
        sampler.biome_id_at(34, 1, -45),
        store
            .probe_block(BlockPos {
                x: 34,
                y: 1,
                z: -45,
            })
            .unwrap()
            .biome_id,
    );

    // A column that reaches into the adjacent (unloaded) chunk truncates to
    // None instead of fabricating a biome sample.
    assert_eq!(sampler.biome_id_at(48, 1, -45), None);
    // Out of the vertical range.
    assert_eq!(sampler.biome_id_at(34, 16, -45), None);
    // Beyond the pre-resolved 3x3 neighbourhood.
    assert_eq!(sampler.biome_id_at(64, 1, -45), None);
}

#[test]
fn extracts_all_terrain_chunks() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    assert_eq!(store.chunk_positions(), vec![ChunkPos { x: 2, z: -3 }]);
    let chunks = store.extract_terrain_chunks();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].pos, ChunkPos { x: 2, z: -3 });
    assert_eq!(chunks[0].summary().opaque_blocks, 4096);
}
