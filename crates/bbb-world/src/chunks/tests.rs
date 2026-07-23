use super::{
    decode_block_entity_data_record, sample_terrain_light, BlockEntityRecord, ChunkState,
    LightData, NbtPayloadSummary, PaletteKind, LIGHT_ARRAY_BYTES,
};
use crate::{
    section_block_index, BlockPos, ChunkPos, TerrainFluidKind, TerrainFluidState, TerrainLight,
    TerrainMaterialClass, WorldDimension, WorldStore,
};

use crate::LocalPlayerPoseState;
use bbb_protocol::codec::Encoder;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use bbb_protocol::heightmap_types::{
    HeightmapUsage, VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID, VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
    VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID, VANILLA_HEIGHTMAP_TYPES,
    VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
};
use bbb_protocol::packets::{
    AddEntity as ProtocolAddEntity, BlockChangedAck as ProtocolBlockChangedAck,
    BlockEntityData as ProtocolBlockEntityData, BlockPos as ProtocolBlockPos,
    BlockUpdate as ProtocolBlockUpdate, ChunkBiomeData as ProtocolChunkBiomeData,
    ChunkHeightmapData, ChunkPos as ProtocolChunkPos, ChunksBiomes as ProtocolChunksBiomes,
    LevelChunkBlockEntity, LevelChunkData, LevelChunkWithLight, LightUpdate as ProtocolLightUpdate,
    LightUpdateData as ProtocolLightUpdateData,
    PlayerPositionUpdate as ProtocolPlayerPositionUpdate,
    SectionBlocksUpdate as ProtocolSectionBlocksUpdate,
    SetChunkCacheCenter as ProtocolSetChunkCacheCenter,
    SetChunkCacheRadius as ProtocolSetChunkCacheRadius, Vec3d as ProtocolVec3d,
};
use std::collections::BTreeMap;
use uuid::Uuid;

mod terrain;
mod terrain_fluids;
mod terrain_materials;

#[test]
fn decodes_level_chunk_with_light_structure() {
    let packet = synthetic_level_chunk_packet();
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    let pos = store.insert_level_chunk_with_light(packet).unwrap();
    assert_eq!(store.queued_light_update_count(), 1);
    assert!(store.probe_chunk(pos).unwrap().light.sky_updates.is_empty());
    assert_eq!(store.poll_queued_light_updates(), 1);
    let chunk = store.probe_chunk(pos).unwrap();

    assert_eq!(pos, ChunkPos { x: 1, z: -2 });
    assert_eq!(store.first_chunk(), Some(ChunkPos { x: 1, z: -2 }));
    assert_eq!(chunk.state, ChunkState::Decoded);
    assert_eq!(chunk.heightmaps.len(), 1);
    assert_eq!(
        chunk.heightmaps[0].kind_id,
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID
    );
    assert_eq!(chunk.sections.len(), 1);
    assert_eq!(
        chunk.sections[0].block_states.palette_kind,
        PaletteKind::SingleValue
    );
    assert_eq!(chunk.sections[0].block_states.palette_global_ids, vec![0]);
    assert_eq!(chunk.sections[0].biomes.entry_count, 64);
    assert_eq!(chunk.block_entities.len(), 1);
    assert_eq!(chunk.block_entities[0].local_x, 10);
    assert_eq!(chunk.block_entities[0].local_z, 11);
    assert!(chunk.block_entities[0].nbt.is_none());
    assert_eq!(chunk.light.sky_updates, vec![vec![1; LIGHT_ARRAY_BYTES]]);
    assert_eq!(store.counters().chunks_decoded, 1);
    assert_eq!(store.counters().sections_decoded, 1);
}

#[test]
fn decodes_skull_note_block_sound_from_initial_chunk_data() {
    let mut packet = synthetic_level_chunk_packet();
    packet
        .chunk_data
        .block_entities
        .push(LevelChunkBlockEntity {
            packed_xz: 0x12,
            y: 7,
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID,
            raw_nbt: nbt_compound_with_string("note_block_sound", "bbb:initial/chime"),
        });
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_level_chunk_with_light(packet).unwrap();

    assert_eq!(
        store.skull_note_block_sound_at(BlockPos {
            x: 17,
            y: 7,
            z: -30
        }),
        Some("bbb:initial/chime")
    );
}

#[test]
fn terrain_revisions_separate_chunk_content_from_neighbor_mesh_invalidation() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    let chunk = ChunkPos { x: 2, z: -3 };
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    let inserted = store.terrain_revision();
    assert!(inserted > 0);
    assert_eq!(store.terrain_chunk_content_revision(chunk), inserted);
    for dz in -1..=1 {
        for dx in -1..=1 {
            assert_eq!(
                store.terrain_chunk_mesh_revision(ChunkPos {
                    x: chunk.x + dx,
                    z: chunk.z + dz,
                }),
                inserted
            );
        }
    }

    let west = ChunkPos {
        x: chunk.x - 1,
        z: chunk.z,
    };
    let east = ChunkPos {
        x: chunk.x + 1,
        z: chunk.z,
    };
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 34,
            y: 1,
            z: -45,
        },
        block_state_id: 0,
    }));
    let interior = store.terrain_revision();
    assert!(interior > inserted);
    assert_eq!(store.terrain_chunk_content_revision(chunk), interior);
    assert_eq!(store.terrain_chunk_mesh_revision(chunk), interior);
    assert_eq!(store.terrain_chunk_mesh_revision(west), inserted);
    assert_eq!(store.terrain_chunk_mesh_revision(east), inserted);

    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 32,
            y: 1,
            z: -45,
        },
        block_state_id: 9,
    }));
    let edge = store.terrain_revision();
    assert!(edge > interior);
    assert_eq!(store.terrain_chunk_content_revision(chunk), edge);
    assert_eq!(store.terrain_chunk_mesh_revision(chunk), edge);
    assert_eq!(store.terrain_chunk_mesh_revision(west), edge);
    assert_eq!(store.terrain_chunk_mesh_revision(east), inserted);
}

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
fn standalone_light_revisions_touch_only_masked_section_neighbors() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 48,
    });
    let chunk = ChunkPos { x: 2, z: -3 };
    let diagonal = ChunkPos { x: 1, z: -4 };
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet_with_sections(3))
        .unwrap();
    let inserted = store.terrain_revision();
    let diagonal_snapshot = store.terrain_chunk_snapshot_revision(diagonal);

    assert!(store
        .apply_light_update(ProtocolLightUpdate {
            chunk_x: chunk.x,
            chunk_z: chunk.z,
            // Light-section index 1 is absolute section y=0 because the
            // light engine includes one section below minSectionY.
            light_data: light_update_data(
                &[0b10],
                &[],
                &[],
                &[],
                vec![vec![7; LIGHT_ARRAY_BYTES]],
                Vec::new(),
            ),
        })
        .unwrap());
    let relit = store.terrain_revision();
    assert_eq!(
        terrain_section_revisions(&store, chunk),
        [relit, relit, inserted]
    );
    assert_eq!(
        terrain_section_revisions(&store, diagonal),
        [relit, relit, inserted]
    );
    assert_eq!(store.terrain_chunk_snapshot_revision(chunk), relit);
    assert_eq!(
        store.terrain_chunk_snapshot_revision(diagonal),
        diagonal_snapshot
    );

    assert!(store
        .apply_light_update(ProtocolLightUpdate {
            chunk_x: chunk.x,
            chunk_z: chunk.z,
            light_data: light_update_data(&[], &[], &[], &[], Vec::new(), Vec::new()),
        })
        .unwrap());
    assert_eq!(store.terrain_revision(), relit);
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
fn full_chunk_biome_reload_and_unload_invalidate_full_neighbor_height() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 32,
    });
    let chunk = ChunkPos { x: 2, z: -3 };
    let diagonal = ChunkPos { x: 1, z: -4 };
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet_with_sections(2))
        .unwrap();

    let replacement = store.probe_chunk(chunk).unwrap().clone();
    let mut diagonal_column = replacement.clone();
    diagonal_column.pos = diagonal;
    store.insert_decoded_chunk(diagonal_column);
    store.insert_decoded_chunk(replacement);
    let reloaded = store.terrain_revision();
    assert_eq!(terrain_section_revisions(&store, chunk), [reloaded; 2]);
    assert_eq!(terrain_section_revisions(&store, diagonal), [reloaded; 2]);
    assert_eq!(store.terrain_chunk_snapshot_revision(diagonal), reloaded);

    store
        .apply_biome_update(ProtocolChunksBiomes {
            chunks: vec![ProtocolChunkBiomeData {
                pos: ProtocolChunkPos { x: 2, z: -3 },
                raw_biomes: repeated_single_biome_payload(7, 2),
            }],
        })
        .unwrap();
    let rebiomed = store.terrain_revision();
    assert_eq!(terrain_section_revisions(&store, chunk), [rebiomed; 2]);
    assert_eq!(terrain_section_revisions(&store, diagonal), [rebiomed; 2]);
    assert_eq!(store.terrain_chunk_snapshot_revision(diagonal), rebiomed);

    assert!(store.forget_chunk(chunk));
    let unloaded = store.terrain_revision();
    assert_eq!(terrain_section_revisions(&store, chunk), [0; 2]);
    assert_eq!(terrain_section_revisions(&store, diagonal), [unloaded; 2]);
    assert_eq!(store.terrain_chunk_snapshot_revision(diagonal), unloaded);
}

#[test]
fn light_and_chunk_removal_invalidate_neighbor_meshes_without_fabricating_content() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    let chunk = ChunkPos { x: 2, z: -3 };
    let neighbor = ChunkPos { x: 1, z: -4 };
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let mut neighbor_column = store.probe_chunk(chunk).unwrap().clone();
    neighbor_column.pos = neighbor;
    store.insert_decoded_chunk(neighbor_column);
    let inserted = store.terrain_revision();
    let neighbor_content = store.terrain_chunk_content_revision(neighbor);

    assert!(store
        .apply_light_update(ProtocolLightUpdate {
            chunk_x: chunk.x,
            chunk_z: chunk.z,
            light_data: light_update_data(
                &[0b10],
                &[],
                &[],
                &[],
                vec![vec![5; LIGHT_ARRAY_BYTES]],
                Vec::new(),
            ),
        })
        .unwrap());
    let relit = store.terrain_revision();
    assert!(relit > inserted);
    assert_eq!(store.terrain_chunk_content_revision(chunk), relit);
    assert_eq!(
        store.terrain_chunk_content_revision(neighbor),
        neighbor_content
    );
    assert_eq!(store.terrain_chunk_mesh_revision(neighbor), relit);

    assert!(store.forget_chunk(chunk));
    let forgotten = store.terrain_revision();
    assert!(forgotten > relit);
    assert_eq!(store.terrain_chunk_content_revision(chunk), 0);
    assert_eq!(store.terrain_chunk_mesh_revision(chunk), 0);
    assert_eq!(
        store.terrain_chunk_content_revision(neighbor),
        neighbor_content
    );
    assert_eq!(store.terrain_chunk_mesh_revision(neighbor), forgotten);
}

#[test]
fn terrain_revision_tracks_view_selection_and_level_clear_lifecycle() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    let initial = store.terrain_revision();

    store.apply_set_chunk_cache_center(ProtocolSetChunkCacheCenter {
        chunk_x: 2,
        chunk_z: -3,
    });
    let recentered = store.terrain_revision();
    assert!(recentered > initial);

    let chunk = ChunkPos { x: 2, z: -3 };
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let inserted = store.terrain_revision();
    assert!(inserted > recentered);
    assert_eq!(store.terrain_chunk_content_revision(chunk), inserted);

    store.clear_client_level();
    assert!(store.terrain_revision() > inserted);
    assert_eq!(store.terrain_chunk_content_revision(chunk), 0);
    assert_eq!(store.terrain_chunk_mesh_revision(chunk), 0);
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
fn samples_terrain_light_from_packet_layers() {
    let dimension = WorldDimension {
        min_y: 0,
        height: 16,
    };
    let index = section_block_index(2, 1, 3);
    let mut sky = vec![0; LIGHT_ARRAY_BYTES];
    let mut block = vec![0; LIGHT_ARRAY_BYTES];
    set_light_nibble(&mut sky, index, 12);
    set_light_nibble(&mut block, index, 7);
    let light = LightData {
        sky_y_mask: vec![0b10],
        block_y_mask: vec![0b10],
        empty_sky_y_mask: Vec::new(),
        empty_block_y_mask: Vec::new(),
        sky_updates: vec![sky],
        block_updates: vec![block],
    };

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
    let light = LightData {
        sky_y_mask: Vec::new(),
        block_y_mask: Vec::new(),
        empty_sky_y_mask: vec![0b10],
        empty_block_y_mask: vec![0b10],
        sky_updates: Vec::new(),
        block_updates: Vec::new(),
    };

    assert_eq!(
        sample_terrain_light(&light, dimension, 2, 1, 3),
        TerrainLight::DARK
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

    let applied = store
        .apply_light_update(ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(&[0b10], &[0b10], &[], &[], vec![sky], vec![block]),
        })
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

    let applied = store
        .apply_light_update(ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(&[], &[], &[], &[0b10], Vec::new(), Vec::new()),
        })
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
fn queued_light_updates_preserve_fifo_order_and_vanilla_poll_budget() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    assert_eq!(store.poll_queued_light_updates(), 1);

    for value in 0..11u8 {
        store
            .queue_light_update(ProtocolLightUpdate {
                chunk_x: 2,
                chunk_z: -3,
                light_data: light_update_data(
                    &[0b10],
                    &[],
                    &[],
                    &[],
                    vec![vec![value; LIGHT_ARRAY_BYTES]],
                    Vec::new(),
                ),
            })
            .unwrap();
    }

    assert_eq!(store.poll_queued_light_updates(), 10);
    assert_eq!(store.queued_light_update_count(), 1);
    assert_eq!(
        store
            .probe_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .light
            .sky_updates[0][0],
        9
    );
    assert_eq!(store.poll_queued_light_updates(), 1);
    assert_eq!(
        store
            .probe_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .light
            .sky_updates[0][0],
        10
    );

    for _ in 0..1_000 {
        store
            .queue_light_update(ProtocolLightUpdate {
                chunk_x: 2,
                chunk_z: -3,
                light_data: light_update_data(&[], &[], &[], &[], Vec::new(), Vec::new()),
            })
            .unwrap();
    }
    assert_eq!(store.poll_queued_light_updates(), 1_000);
    assert_eq!(store.queued_light_update_count(), 0);
    assert_eq!(store.counters().light_updates_received, 1_011);
    assert_eq!(store.counters().light_updates_applied, 1_011);
}

#[test]
fn light_validation_rejects_wrong_dimension_domains_before_mutation() {
    let dimension = WorldDimension {
        min_y: 0,
        height: 16,
    };
    let mut store = WorldStore::with_dimension(dimension);
    let err = store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet_with_sections(2))
        .unwrap_err();
    assert!(matches!(
        err,
        crate::WorldDecodeError::ChunkSectionCountMismatch {
            expected: 1,
            actual: 2
        }
    ));
    assert!(store.chunk_positions().is_empty());
    assert_eq!(store.queued_light_update_count(), 0);
    assert_eq!(store.counters().chunks_received, 0);

    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    assert_eq!(store.poll_queued_light_updates(), 1);
    let before = store.probe_chunk(ChunkPos { x: 2, z: -3 }).unwrap().clone();
    let err = store
        .queue_light_update(ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(
                &[0b1000],
                &[],
                &[],
                &[],
                vec![vec![1; LIGHT_ARRAY_BYTES]],
                Vec::new(),
            ),
        })
        .unwrap_err();
    assert!(matches!(
        err,
        crate::WorldDecodeError::LightMaskOutsideDimension {
            field: "sky_y_mask",
            section: 3,
            section_count: 3
        }
    ));
    assert_eq!(store.queued_light_update_count(), 0);
    assert_eq!(store.counters().light_updates_received, 0);
    assert_eq!(store.probe_chunk(before.pos).unwrap().light, before.light);
}

#[test]
fn forget_and_level_clear_discard_pending_light_without_stale_application() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    store
        .queue_light_update(ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(&[], &[], &[], &[], Vec::new(), Vec::new()),
        })
        .unwrap();
    assert_eq!(store.queued_light_update_count(), 2);
    assert_eq!(store.publication_snapshot().queued_light_update_count(), 0);
    assert_eq!(store.queued_light_update_count(), 2);
    assert!(store.forget_chunk(ChunkPos { x: 2, z: -3 }));
    assert_eq!(store.queued_light_update_count(), 0);
    assert_eq!(store.counters().light_updates_received, 1);
    assert_eq!(store.counters().light_updates_applied, 0);
    assert_eq!(store.counters().light_updates_ignored, 1);

    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    store
        .queue_light_update(ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(&[], &[], &[], &[], Vec::new(), Vec::new()),
        })
        .unwrap();
    store.clear_client_level();
    assert_eq!(store.queued_light_update_count(), 0);
    assert_eq!(store.poll_queued_light_updates(), 0);
    assert_eq!(store.counters().light_updates_received, 2);
    assert_eq!(store.counters().light_updates_applied, 0);
    assert_eq!(store.counters().light_updates_ignored, 2);
}

#[test]
fn samples_block_light_at_world_position_and_reports_unloaded() {
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
    store
        .apply_light_update(ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(&[0b10], &[0b10], &[], &[], vec![sky], vec![block]),
        })
        .unwrap();

    // Chunk (2, -3) local (2, 1, 3) maps to world block (34, 1, -45).
    assert_eq!(
        store.sample_block_light(BlockPos {
            x: 34,
            y: 1,
            z: -45,
        }),
        Some(TerrainLight { sky: 4, block: 13 })
    );
    // Unloaded chunk and out-of-world height report no data so callers can apply
    // the entity full-bright fallback.
    assert_eq!(
        store.sample_block_light(BlockPos {
            x: 9999,
            y: 1,
            z: 9999,
        }),
        None
    );
    assert_eq!(
        store.sample_block_light(BlockPos {
            x: 34,
            y: 999,
            z: -45,
        }),
        None
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
    store
        .apply_light_update(ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(&[0b1], &[0b1], &[], &[], vec![sky], vec![block]),
        })
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
fn block_update_maintains_distinct_motion_blocking_heightmaps() {
    let dimension = WorldDimension {
        min_y: 0,
        height: 16,
    };
    let mut packet = synthetic_local_palette_chunk_packet();
    packet.chunk_data.heightmaps = vec![
        motion_blocking_heightmap(dimension, &[(2, 3, 2)]),
        heightmap_data(
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
            dimension,
            &[(2, 3, 2)],
        ),
    ];
    let mut store = WorldStore::with_dimension(dimension);
    store.insert_level_chunk_with_light(packet).unwrap();
    let pos = ProtocolBlockPos {
        x: 34,
        y: 1,
        z: -45,
    };

    // Leaves block movement, so the ordinary map rises; `MOTION_BLOCKING_NO_LEAVES` keeps the
    // stone at y=1 as its highest qualifying block.
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { y: 3, ..pos },
        block_state_id: 255, // minecraft:oak_leaves
    }));
    assert_eq!(store.sample_motion_blocking_height(34, -45), Some(4));
    assert_eq!(
        store.sample_heightmap_first_available(
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
            34,
            -45,
        ),
        Some(2)
    );

    // Replacing the leaf with stone raises both maps. Replacing that stone with a leaf makes only
    // the no-leaves map scan back to the next qualifying block at y=2, matching the two
    // `LevelChunk.setBlockState` updates.
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { y: 3, ..pos },
        block_state_id: 9,
    }));
    assert_eq!(
        store.sample_heightmap_first_available(
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
            34,
            -45,
        ),
        Some(4)
    );
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { y: 3, ..pos },
        block_state_id: 255,
    }));
    assert_eq!(store.sample_motion_blocking_height(34, -45), Some(4));
    assert_eq!(
        store.sample_heightmap_first_available(
            VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
            34,
            -45,
        ),
        Some(3)
    );
}

#[test]
fn block_update_materializes_omitted_motion_blocking_heightmaps() {
    let dimension = WorldDimension {
        min_y: 0,
        height: 16,
    };
    let mut store = WorldStore::with_dimension(dimension);
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 34,
            y: 3,
            z: -45,
        },
        block_state_id: 255, // minecraft:oak_leaves
    }));
    let sampler = store
        .chunk_heightmap_sampler(ChunkPos { x: 2, z: -3 })
        .unwrap();
    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, 2, 3),
        Some(4)
    );
    assert_eq!(
        sampler.first_available(VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID, 2, 3),
        Some(0)
    );
}

#[test]
fn light_update_for_missing_chunk_is_counted_but_not_applied() {
    let mut store = WorldStore::new();

    let applied = store
        .apply_light_update(ProtocolLightUpdate {
            chunk_x: 2,
            chunk_z: -3,
            light_data: light_update_data(&[], &[], &[], &[], Vec::new(), Vec::new()),
        })
        .unwrap();

    assert!(!applied);
    assert_eq!(store.counters().light_updates_received, 1);
    assert_eq!(store.counters().light_updates_applied, 0);
    assert_eq!(store.counters().light_updates_ignored, 1);
}

#[test]
fn applies_biome_update_to_existing_chunk_sections() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    let applied = store
        .apply_biome_update(ProtocolChunksBiomes {
            chunks: vec![ProtocolChunkBiomeData {
                pos: ProtocolChunkPos { x: 2, z: -3 },
                raw_biomes: single_biome_payload(7),
            }],
        })
        .unwrap();

    assert_eq!(applied, 1);
    assert_eq!(store.counters().biome_updates_received, 1);
    assert_eq!(store.counters().biome_updates_applied, 1);
    assert_eq!(
        store
            .probe_block(BlockPos {
                x: 34,
                y: 1,
                z: -45,
            })
            .unwrap()
            .biome_id,
        Some(7)
    );
    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    assert_eq!(
        terrain.cells[terrain_cell_index(2, 1, 3, 16)].biome_id,
        Some(7)
    );
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
fn chunk_cache_updates_track_view_state() {
    let mut store = WorldStore::new();

    assert_eq!(store.chunk_cache_center(), None);
    assert_eq!(store.chunk_cache_radius(), None);

    let view = store.apply_set_chunk_cache_center(ProtocolSetChunkCacheCenter {
        chunk_x: 11,
        chunk_z: -9,
    });

    assert_eq!(view.center, Some(ChunkPos { x: 11, z: -9 }));
    assert_eq!(view.radius, None);
    assert_eq!(store.chunk_cache_center(), Some(ChunkPos { x: 11, z: -9 }));
    assert_eq!(store.counters().chunk_cache_center_updates_received, 1);

    let view = store.apply_set_chunk_cache_radius(ProtocolSetChunkCacheRadius { radius: 12 });

    assert_eq!(view.center, Some(ChunkPos { x: 11, z: -9 }));
    assert_eq!(view.radius, Some(12));
    assert_eq!(store.chunk_view(), view);
    assert_eq!(store.chunk_cache_radius(), Some(12));
    assert_eq!(store.counters().chunk_cache_radius_updates_received, 1);
}

#[test]
fn chunk_cache_view_prunes_and_rejects_columns_outside_vanilla_storage_range() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    for x in [0, 5, 6, 20] {
        let mut packet = synthetic_level_chunk_packet();
        packet.x = x;
        packet.z = 0;
        store.insert_level_chunk_with_light(packet).unwrap();
    }
    assert_eq!(store.chunk_positions().len(), 4);

    store.apply_set_chunk_cache_radius(ProtocolSetChunkCacheRadius { radius: 2 });
    assert_eq!(store.chunk_positions().len(), 4, "center is not known yet");
    store.apply_set_chunk_cache_center(ProtocolSetChunkCacheCenter {
        chunk_x: 0,
        chunk_z: 0,
    });
    assert!(store.probe_chunk(ChunkPos { x: 0, z: 0 }).is_some());
    assert!(store.probe_chunk(ChunkPos { x: 5, z: 0 }).is_some());
    assert!(store.probe_chunk(ChunkPos { x: 6, z: 0 }).is_none());
    assert!(store.probe_chunk(ChunkPos { x: 20, z: 0 }).is_none());

    let mut packet = synthetic_level_chunk_packet();
    packet.x = -6;
    packet.z = 0;
    store.insert_level_chunk_with_light(packet).unwrap();
    assert!(store.probe_chunk(ChunkPos { x: -6, z: 0 }).is_none());
    assert_eq!(store.counters().chunk_forgets_received, 0);
    assert_eq!(store.counters().chunks_forgotten, 0);

    store.apply_set_chunk_cache_center(ProtocolSetChunkCacheCenter {
        chunk_x: 10,
        chunk_z: 0,
    });
    assert!(store.probe_chunk(ChunkPos { x: 0, z: 0 }).is_none());
    assert!(store.probe_chunk(ChunkPos { x: 5, z: 0 }).is_some());
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
fn applies_single_block_update_and_reuploads_palette() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    let applied = store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 34,
            y: 1,
            z: -45,
        },
        block_state_id: 0,
    });

    assert!(applied);
    assert_eq!(store.counters().block_updates_received, 1);
    assert_eq!(store.counters().block_updates_applied, 1);
    assert_eq!(store.counters().block_updates_ignored, 0);

    let probe = store
        .probe_block(BlockPos {
            x: 34,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(probe.block_state_id, 0);
    assert_eq!(probe.block_name.as_deref(), Some("minecraft:air"));
    assert_eq!(probe.material, TerrainMaterialClass::Empty);
    assert_eq!(probe.block_palette_kind, PaletteKind::Global);
    assert_eq!(probe.block_palette_index, None);

    let chunk = store.probe_chunk(ChunkPos { x: 2, z: -3 }).unwrap();
    assert_eq!(chunk.sections[0].non_empty_block_count, 4095);
    let summary = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap()
        .summary();
    assert_eq!(summary.empty_blocks, 1);
    assert_eq!(summary.opaque_blocks, 4095);
}

#[test]
fn local_destroy_prediction_defers_server_block_update_until_ack() {
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
    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 0);
    assert_eq!(store.local_block_predictions().len(), 1);
    assert_eq!(store.counters().local_block_predictions_created, 1);
    assert_eq!(store.counters().local_block_predictions_tracked, 1);

    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 34,
            y: 1,
            z: -45,
        },
        block_state_id: 9,
    }));
    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 0);
    assert_eq!(store.local_block_predictions()[0].server_block_state_id, 9);
    assert_eq!(
        store
            .counters()
            .local_block_predictions_reconciled_by_update,
        1
    );

    store.apply_block_changed_ack(ProtocolBlockChangedAck { sequence: 7 });
    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 9);
    assert!(store.local_block_predictions().is_empty());
    assert_eq!(
        store.counters().local_block_predictions_reconciled_by_ack,
        1
    );
    assert_eq!(store.counters().local_block_predictions_tracked, 0);
}

#[test]
fn repeated_local_predictions_retain_first_server_state_and_player_position() {
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
    let first_position = ProtocolVec3d {
        x: 34.5,
        y: 2.0,
        z: -44.5,
    };
    store.set_local_player_pose(LocalPlayerPoseState {
        position: first_position,
        ..LocalPlayerPoseState::default()
    });

    assert!(store.predict_local_block_state(pos, 1, 7));
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 36.5,
            y: 2.0,
            z: -44.5,
        },
        ..LocalPlayerPoseState::default()
    });
    assert!(store.predict_local_block_state(pos, 0, 8));

    assert_eq!(store.local_block_predictions().len(), 1);
    assert_eq!(store.counters().local_block_predictions_created, 1);
    assert_eq!(store.local_block_predictions()[0].sequence, 8);
    assert_eq!(store.local_block_predictions()[0].server_block_state_id, 9);
    assert_eq!(
        store.local_block_predictions()[0].predicted_block_state_id,
        0
    );
    assert_eq!(
        store.local_block_predictions()[0].player_position,
        Some(first_position)
    );

    store.apply_block_changed_ack(ProtocolBlockChangedAck { sequence: 7 });
    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 0);
    assert_eq!(store.local_block_predictions().len(), 1);

    store.apply_block_changed_ack(ProtocolBlockChangedAck { sequence: 8 });
    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 9);
    assert!(store.local_block_predictions().is_empty());
}

#[test]
fn one_prediction_sequence_tracks_each_changed_position() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let grass = BlockPos {
        x: 34,
        y: 1,
        z: -45,
    };
    let granite = BlockPos {
        x: 35,
        y: 1,
        z: -45,
    };

    assert!(store.predict_local_block_state(grass, 0, 7));
    assert!(store.predict_local_block_state(granite, 1, 7));

    assert_eq!(store.local_block_predictions().len(), 2);
    assert_eq!(store.counters().local_block_predictions_created, 2);
    assert!(store.local_block_predictions().iter().any(|prediction| {
        prediction.pos == grass
            && prediction.server_block_state_id == 9
            && prediction.predicted_block_state_id == 0
    }));
    assert!(store.local_block_predictions().iter().any(|prediction| {
        prediction.pos == granite
            && prediction.server_block_state_id == 5
            && prediction.predicted_block_state_id == 1
    }));

    store.apply_block_changed_ack(ProtocolBlockChangedAck { sequence: 7 });
    assert_eq!(store.probe_block(grass).unwrap().block_state_id, 9);
    assert_eq!(store.probe_block(granite).unwrap().block_state_id, 5);
    assert!(store.local_block_predictions().is_empty());
}

#[test]
fn local_destroy_prediction_rejected_ack_snaps_colliding_player_to_prediction_position() {
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
    let predicted_position = ProtocolVec3d {
        x: 34.5,
        y: 2.0,
        z: -44.5,
    };
    store.set_local_player_pose(LocalPlayerPoseState {
        position: predicted_position,
        ..LocalPlayerPoseState::default()
    });

    assert!(store.predict_local_destroy_block(pos, 7));
    assert_eq!(
        store.local_block_predictions()[0].player_position,
        Some(predicted_position)
    );
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 34.5,
            y: 1.2,
            z: -44.5,
        },
        ..LocalPlayerPoseState::default()
    });

    store.apply_block_changed_ack(ProtocolBlockChangedAck { sequence: 7 });

    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 9);
    assert_eq!(
        store.local_player_pose().unwrap().position,
        predicted_position
    );
}

#[test]
fn teleport_prevents_older_prediction_rejection_from_moving_player_back() {
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
    let pre_teleport_position = ProtocolVec3d {
        x: 34.5,
        y: 2.0,
        z: -44.5,
    };
    store.set_local_player_pose(LocalPlayerPoseState {
        position: pre_teleport_position,
        ..LocalPlayerPoseState::default()
    });
    store.set_local_prediction_sequence(7);
    assert!(store.predict_local_destroy_block(pos, 7));

    let teleported_position = ProtocolVec3d {
        x: 34.5,
        y: 1.2,
        z: -44.5,
    };
    store.apply_player_position(ProtocolPlayerPositionUpdate {
        id: 23,
        position: teleported_position,
        delta_movement: ProtocolVec3d::default(),
        y_rot: 0.0,
        x_rot: 0.0,
        relatives_mask: 0,
    });
    store.apply_block_changed_ack(ProtocolBlockChangedAck { sequence: 7 });

    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 9);
    assert_eq!(
        store.local_player_pose().unwrap().position,
        teleported_position
    );

    assert_eq!(store.next_local_prediction_sequence(), 8);
    store.set_local_player_pose(LocalPlayerPoseState {
        position: pre_teleport_position,
        ..LocalPlayerPoseState::default()
    });
    assert!(store.predict_local_destroy_block(pos, 8));
    store.set_local_player_pose(LocalPlayerPoseState {
        position: teleported_position,
        ..LocalPlayerPoseState::default()
    });
    store.apply_block_changed_ack(ProtocolBlockChangedAck { sequence: 8 });

    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 9);
    assert_eq!(
        store.local_player_pose().unwrap().position,
        pre_teleport_position
    );
}

#[test]
fn local_destroy_prediction_rejected_ack_keeps_non_colliding_player_position() {
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
    store.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 34.5,
            y: 2.0,
            z: -44.5,
        },
        ..LocalPlayerPoseState::default()
    });
    assert!(store.predict_local_destroy_block(pos, 7));

    let non_colliding_position = ProtocolVec3d {
        x: 36.5,
        y: 1.2,
        z: -44.5,
    };
    store.set_local_player_pose(LocalPlayerPoseState {
        position: non_colliding_position,
        ..LocalPlayerPoseState::default()
    });

    store.apply_block_changed_ack(ProtocolBlockChangedAck { sequence: 7 });

    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 9);
    assert_eq!(
        store.local_player_pose().unwrap().position,
        non_colliding_position
    );
}

#[test]
fn local_destroy_prediction_accepts_matching_server_update_on_ack() {
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
    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 0);

    assert_eq!(
        store.apply_section_blocks_update(ProtocolSectionBlocksUpdate {
            section_x: 2,
            section_y: 0,
            section_z: -3,
            updates: vec![ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 34,
                    y: 1,
                    z: -45,
                },
                block_state_id: 0,
            }],
        }),
        1
    );
    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 0);
    assert_eq!(store.local_block_predictions()[0].server_block_state_id, 0);

    store.apply_block_changed_ack(ProtocolBlockChangedAck { sequence: 7 });
    assert_eq!(store.probe_block(pos).unwrap().block_state_id, 0);
    assert!(store.local_block_predictions().is_empty());
    assert_eq!(
        store
            .counters()
            .local_block_predictions_reconciled_by_update,
        1
    );
    assert_eq!(
        store.counters().local_block_predictions_reconciled_by_ack,
        1
    );
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
fn applies_section_blocks_update() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    let applied = store.apply_section_blocks_update(ProtocolSectionBlocksUpdate {
        section_x: 2,
        section_y: 0,
        section_z: -3,
        updates: vec![
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 34,
                    y: 1,
                    z: -45,
                },
                block_state_id: 0,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 35,
                    y: 1,
                    z: -45,
                },
                block_state_id: 0,
            },
        ],
    });

    assert_eq!(applied, 2);
    assert_eq!(store.counters().block_updates_received, 2);
    assert_eq!(store.counters().block_updates_applied, 2);
    assert_eq!(store.counters().block_updates_ignored, 0);

    let summary = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap()
        .summary();
    assert_eq!(summary.empty_blocks, 2);
    assert_eq!(summary.opaque_blocks, 4094);
    assert_eq!(
        store
            .probe_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .sections[0]
            .non_empty_block_count,
        4094
    );
}

#[test]
fn block_updates_follow_vanilla_block_entity_lifecycle() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    let stone = test_block_state_id("minecraft:stone", &[]);
    let sign = test_block_state_id(
        "minecraft:oak_sign",
        &[("rotation", "0"), ("waterlogged", "false")],
    );
    let moving_piston = test_block_state_id(
        "minecraft:moving_piston",
        &[("facing", "north"), ("type", "normal")],
    );
    let chest_north = test_block_state_id(
        "minecraft:chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    let chest_south = test_block_state_id(
        "minecraft:chest",
        &[
            ("facing", "south"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    let copper_chest = test_block_state_id(
        "minecraft:copper_chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    let waxed_oxidized_copper_chest = test_block_state_id(
        "minecraft:waxed_oxidized_copper_chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    let copper_statue = test_block_state_id(
        "minecraft:copper_golem_statue",
        &[
            ("copper_golem_pose", "standing"),
            ("facing", "north"),
            ("waterlogged", "false"),
        ],
    );
    let waxed_oxidized_copper_statue = test_block_state_id(
        "minecraft:waxed_oxidized_copper_golem_statue",
        &[
            ("copper_golem_pose", "standing"),
            ("facing", "north"),
            ("waterlogged", "false"),
        ],
    );
    let update = |store: &mut WorldStore, pos: BlockPos, block_state_id| {
        assert!(store.apply_block_update(ProtocolBlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
        }));
    };
    let apply_data =
        |store: &mut WorldStore, pos: BlockPos, block_entity_type_id, raw_nbt: Vec<u8>| {
            assert!(store
                .apply_block_entity_data(ProtocolBlockEntityData {
                    pos: ProtocolBlockPos {
                        x: pos.x,
                        y: pos.y,
                        z: pos.z,
                    },
                    block_entity_type_id,
                    raw_nbt,
                })
                .unwrap());
        };

    let removed = BlockPos {
        x: 33,
        y: 2,
        z: -46,
    };
    update(&mut store, removed, sign);
    let sign_nbt = nbt_compound_with_string("id", "minecraft:sign");
    apply_data(
        &mut store,
        removed,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
        sign_nbt,
    );
    update(&mut store, removed, stone);
    assert_eq!(store.block_entity_type_id_at(removed), None);

    let same_block = BlockPos {
        x: 34,
        y: 2,
        z: -46,
    };
    update(&mut store, same_block, chest_north);
    assert_eq!(
        store.block_entity_type_id_at(same_block),
        Some(bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID)
    );
    assert_eq!(store.block_entity_raw_nbt_at(same_block), Some(&[][..]));
    let chest_nbt = nbt_compound_with_string("id", "minecraft:chest");
    apply_data(
        &mut store,
        same_block,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
        chest_nbt.clone(),
    );
    update(&mut store, same_block, chest_south);
    assert_eq!(
        store.block_entity_raw_nbt_at(same_block),
        Some(chest_nbt.as_slice())
    );
    update(&mut store, same_block, sign);
    assert_eq!(
        store.block_entity_type_id_at(same_block),
        Some(bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID)
    );
    assert_eq!(store.block_entity_raw_nbt_at(same_block), Some(&[][..]));

    let mismatched = BlockPos {
        x: 35,
        y: 2,
        z: -46,
    };
    update(&mut store, mismatched, moving_piston);
    apply_data(
        &mut store,
        mismatched,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_PISTON_ID,
        nbt_compound_with_string("id", "minecraft:piston"),
    );
    update(&mut store, mismatched, chest_north);
    assert_eq!(
        store.block_entity_type_id_at(mismatched),
        Some(bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID)
    );
    assert_eq!(store.block_entity_raw_nbt_at(mismatched), Some(&[][..]));

    let preserved_chest = BlockPos {
        x: 36,
        y: 2,
        z: -46,
    };
    update(&mut store, preserved_chest, copper_chest);
    apply_data(
        &mut store,
        preserved_chest,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
        chest_nbt.clone(),
    );
    update(&mut store, preserved_chest, waxed_oxidized_copper_chest);
    assert_eq!(
        store.block_entity_raw_nbt_at(preserved_chest),
        Some(chest_nbt.as_slice())
    );
    update(&mut store, preserved_chest, chest_north);
    assert_eq!(
        store.block_entity_type_id_at(preserved_chest),
        Some(bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID)
    );
    assert_eq!(
        store.block_entity_raw_nbt_at(preserved_chest),
        Some(&[][..])
    );

    let preserved_statue = BlockPos {
        x: 37,
        y: 2,
        z: -46,
    };
    update(&mut store, preserved_statue, copper_statue);
    let statue_nbt = nbt_compound_with_string("id", "minecraft:copper_golem_statue");
    apply_data(
        &mut store,
        preserved_statue,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_COPPER_GOLEM_STATUE_ID,
        statue_nbt.clone(),
    );
    update(&mut store, preserved_statue, waxed_oxidized_copper_statue);
    assert_eq!(
        store.block_entity_raw_nbt_at(preserved_statue),
        Some(statue_nbt.as_slice())
    );
}

#[test]
fn section_block_updates_apply_in_order_and_reject_impossible_positions() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    let pos = ProtocolBlockPos {
        x: 38,
        y: 3,
        z: -46,
    };
    let chest = test_block_state_id(
        "minecraft:chest",
        &[
            ("facing", "north"),
            ("type", "single"),
            ("waterlogged", "false"),
        ],
    );
    let stone = test_block_state_id("minecraft:stone", &[]);
    let revision = store.terrain_revision();

    assert_eq!(
        store.apply_section_blocks_update(ProtocolSectionBlocksUpdate {
            section_x: 2,
            section_y: 0,
            section_z: -3,
            updates: vec![
                ProtocolBlockUpdate {
                    pos,
                    block_state_id: chest,
                },
                ProtocolBlockUpdate {
                    pos,
                    block_state_id: stone,
                },
            ],
        }),
        2
    );
    let pos = BlockPos {
        x: 38,
        y: 3,
        z: -46,
    };
    assert_eq!(store.probe_block(pos).unwrap().block_state_id, stone);
    assert_eq!(store.block_entity_type_id_at(pos), None);
    assert_eq!(store.terrain_revision(), revision + 2);

    let revision = store.terrain_revision();
    assert_eq!(
        store.apply_section_blocks_update(ProtocolSectionBlocksUpdate {
            section_x: 2,
            section_y: 0,
            section_z: -3,
            updates: vec![ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 48,
                    y: 3,
                    z: -46
                },
                block_state_id: chest,
            }],
        }),
        0
    );
    assert_eq!(store.terrain_revision(), revision);
    assert_eq!(store.counters().block_updates_received, 3);
    assert_eq!(store.counters().block_updates_applied, 2);
    assert_eq!(store.counters().block_updates_ignored, 1);
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
fn block_updates_for_missing_targets_are_counted_but_not_applied() {
    let mut store = WorldStore::new();

    assert!(!store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: 34,
            y: 1,
            z: -45,
        },
        block_state_id: 0,
    }));

    let applied = store.apply_section_blocks_update(ProtocolSectionBlocksUpdate {
        section_x: 2,
        section_y: 0,
        section_z: -3,
        updates: vec![
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 34,
                    y: 1,
                    z: -45,
                },
                block_state_id: 0,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 35,
                    y: 1,
                    z: -45,
                },
                block_state_id: 0,
            },
        ],
    });

    assert_eq!(applied, 0);
    assert_eq!(store.counters().block_updates_received, 3);
    assert_eq!(store.counters().block_updates_applied, 0);
    assert_eq!(store.counters().block_updates_ignored, 3);
}

#[test]
fn applies_block_entity_data_update() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
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
            sign_text: None,
            vault_shared_data: None,
            decorated_pot_sherds: None,
            banner_patterns: None,
            end_gateway: None,
            spawner: None,
            command_block: None,
            structure_block: None,
            jigsaw_block: None,
            test_block: None,
            test_instance_block: None,
            note_block_sound: None,
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
}

#[test]
fn block_entity_data_updates_every_known_matching_type_and_ignores_mismatches_before_decode() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    for block_entity_type in bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPES {
        let local_x = block_entity_type.id.rem_euclid(16) as u8;
        let local_z = block_entity_type.id.div_euclid(16) as u8;
        store.chunks[0]
            .block_entities
            .push(BlockEntityRecord::empty(
                local_x,
                7,
                local_z,
                block_entity_type.id,
            ));
    }

    for block_entity_type in bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPES {
        let pos = block_entity_matrix_pos(block_entity_type.id);
        assert!(store
            .apply_block_entity_data(ProtocolBlockEntityData {
                pos,
                block_entity_type_id: block_entity_type.id,
                raw_nbt: vec![10, 0],
            })
            .unwrap());
        assert_eq!(
            store.block_entity_raw_nbt_at(BlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            }),
            Some(&[10, 0][..])
        );
    }

    let type_count =
        i32::try_from(bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPES.len()).unwrap();
    for block_entity_type in bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPES {
        let pos = block_entity_matrix_pos(block_entity_type.id);
        let mismatched_type_id = (block_entity_type.id + 1).rem_euclid(type_count);
        assert!(!store
            .apply_block_entity_data(ProtocolBlockEntityData {
                pos,
                block_entity_type_id: mismatched_type_id,
                // Invalid for BlockEntityData, but the handler must not touch
                // it after the existing-type identity check fails.
                raw_nbt: vec![0],
            })
            .unwrap());
        assert_eq!(
            store.block_entity_type_id_at(BlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            }),
            Some(block_entity_type.id)
        );
    }

    let first_type = bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPES[0];
    let first_pos = block_entity_matrix_pos(first_type.id);
    assert!(store
        .apply_block_entity_data(ProtocolBlockEntityData {
            pos: first_pos,
            block_entity_type_id: first_type.id,
            raw_nbt: vec![10, 1, 0, 0],
        })
        .is_err());
    assert_eq!(
        store.block_entity_raw_nbt_at(BlockPos {
            x: first_pos.x,
            y: first_pos.y,
            z: first_pos.z,
        }),
        Some(&[10, 0][..])
    );

    let counters = store.counters();
    assert_eq!(
        counters.block_entity_updates_received,
        2 * type_count as usize + 1
    );
    assert_eq!(counters.block_entity_updates_applied, type_count as usize);
    assert_eq!(counters.block_entity_updates_ignored, type_count as usize);
    assert_eq!(counters.world_apply_errors, 1);
}

#[test]
fn typed_block_entity_projections_are_selected_only_by_generated_type_identity() {
    use bbb_protocol::block_entity_types::{
        VANILLA_BLOCK_ENTITY_TYPE_BANNER_ID, VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
        VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID, VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID,
        VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID, VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
        VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID,
    };

    let sign_nbt = sign_text_nbt(["front", "", "", ""], ["back", "", "", ""]);
    assert!(
        decode_test_block_entity_record(VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID, sign_nbt.clone())
            .sign_text
            .is_some()
    );
    assert!(
        decode_test_block_entity_record(VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID, sign_nbt)
            .sign_text
            .is_none()
    );

    let vault_nbt = vault_shared_data_nbt(&[Uuid::nil()], Some(3.0));
    assert!(
        decode_test_block_entity_record(VANILLA_BLOCK_ENTITY_TYPE_VAULT_ID, vault_nbt.clone())
            .vault_shared_data
            .is_some()
    );
    assert!(
        decode_test_block_entity_record(VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID, vault_nbt)
            .vault_shared_data
            .is_none()
    );

    let pot_nbt = nbt_compound_with_empty_list("sherds", 8);
    assert!(decode_test_block_entity_record(
        VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
        pot_nbt.clone()
    )
    .decorated_pot_sherds
    .is_some());
    assert!(
        decode_test_block_entity_record(VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID, pot_nbt)
            .decorated_pot_sherds
            .is_none()
    );

    let banner_nbt = nbt_compound_with_empty_list("patterns", 10);
    assert!(decode_test_block_entity_record(
        VANILLA_BLOCK_ENTITY_TYPE_BANNER_ID,
        banner_nbt.clone()
    )
    .banner_patterns
    .is_some());
    assert!(
        decode_test_block_entity_record(VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID, banner_nbt)
            .banner_patterns
            .is_none()
    );

    let gateway_nbt = nbt_compound_with_long("Age", 42);
    assert!(decode_test_block_entity_record(
        VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID,
        gateway_nbt.clone()
    )
    .end_gateway
    .is_some());
    assert!(
        decode_test_block_entity_record(VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID, gateway_nbt)
            .end_gateway
            .is_none()
    );

    let spawner_nbt = nbt_compound_with_short("Delay", 12);
    assert!(decode_test_block_entity_record(
        VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
        spawner_nbt.clone()
    )
    .spawner
    .is_some());
    assert!(
        decode_test_block_entity_record(VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID, spawner_nbt)
            .spawner
            .is_none()
    );
}

#[test]
fn applies_sign_block_entity_text_update() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
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
        block_state_id: test_block_state_id(
            "minecraft:oak_sign",
            &[("rotation", "0"), ("waterlogged", "false")],
        ),
    }));

    let raw_nbt = sign_text_nbt(
        ["Front 1", "Front 2", "Front 3", "Front 4"],
        ["Back 1", "Back 2", "Back 3", "Back 4"],
    );
    assert!(store
        .apply_block_entity_data(ProtocolBlockEntityData {
            pos,
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
            raw_nbt,
        })
        .unwrap());

    assert_eq!(
        store.sign_text_lines(
            BlockPos {
                x: 33,
                y: 7,
                z: -46
            },
            true
        ),
        Some([
            "Front 1".to_string(),
            "Front 2".to_string(),
            "Front 3".to_string(),
            "Front 4".to_string(),
        ])
    );
    assert_eq!(
        store.sign_text_lines(
            BlockPos {
                x: 33,
                y: 7,
                z: -46
            },
            false
        ),
        Some([
            "Back 1".to_string(),
            "Back 2".to_string(),
            "Back 3".to_string(),
            "Back 4".to_string(),
        ])
    );
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

fn synthetic_level_chunk_packet() -> LevelChunkWithLight {
    let mut sections = Encoder::new();
    sections.write_i16(0);
    sections.write_i16(0);
    sections.write_u8(0);
    sections.write_var_i32(0);
    sections.write_u8(0);
    sections.write_var_i32(0);
    let sections = sections.into_inner();

    LevelChunkWithLight {
        x: 1,
        z: -2,
        chunk_data: LevelChunkData {
            heightmaps: vec![ChunkHeightmapData {
                kind_id: VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                data: vec![42],
            }],
            section_data: sections,
            block_entities: vec![LevelChunkBlockEntity {
                packed_xz: 0xab,
                y: 64,
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
                raw_nbt: vec![0],
            }],
        },
        light_data: light_update_data(
            &[0b10],
            &[0b100],
            &[],
            &[],
            vec![vec![1; LIGHT_ARRAY_BYTES]],
            vec![vec![3; LIGHT_ARRAY_BYTES]],
        ),
    }
}

fn synthetic_local_palette_chunk_packet() -> LevelChunkWithLight {
    synthetic_local_palette_chunk_packet_with_sections(1)
}

fn synthetic_local_palette_chunk_packet_with_sections(section_count: usize) -> LevelChunkWithLight {
    let mut sections = Encoder::new();
    for _ in 0..section_count {
        sections.write_i16(4096);
        sections.write_i16(0);
        write_local_block_palette(&mut sections);
        sections.write_u8(0);
        sections.write_var_i32(4);
    }
    let sections = sections.into_inner();

    LevelChunkWithLight {
        x: 2,
        z: -3,
        chunk_data: LevelChunkData {
            heightmaps: Vec::new(),
            section_data: sections,
            block_entities: Vec::new(),
        },
        light_data: light_update_data(&[], &[], &[], &[], Vec::new(), Vec::new()),
    }
}

fn terrain_section_revisions<const N: usize>(store: &WorldStore, pos: ChunkPos) -> [u64; N] {
    store
        .terrain_chunk_section_mesh_revisions(pos)
        .map(|section| section.revision)
        .collect::<Vec<_>>()
        .try_into()
        .ok()
        .expect("dimension has the expected section count")
}

fn motion_blocking_heightmap(
    dimension: WorldDimension,
    entries: &[(u8, u8, i32)],
) -> ChunkHeightmapData {
    heightmap_data(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, dimension, entries)
}

fn heightmap_data(
    kind_id: i32,
    dimension: WorldDimension,
    entries: &[(u8, u8, i32)],
) -> ChunkHeightmapData {
    let bits = heightmap_bits_for_dimension(dimension);
    let mut values = vec![0u64; 16 * 16];
    for &(local_x, local_z, first_available) in entries {
        let index = usize::from(local_x) + usize::from(local_z) * 16;
        values[index] = u64::try_from(first_available - dimension.min_y).unwrap();
    }
    ChunkHeightmapData {
        kind_id,
        data: pack_fixed_values(&values, bits)
            .into_iter()
            .map(|value| value as i64)
            .collect(),
    }
}

fn heightmap_bits_for_dimension(dimension: WorldDimension) -> usize {
    let value = u64::try_from(dimension.height).unwrap() + 1;
    (u64::BITS - (value - 1).leading_zeros()).max(1) as usize
}

fn write_local_block_palette(out: &mut Encoder) {
    let target_index = section_block_index(2, 1, 3);
    let mut values = vec![0u64; 4096];
    values[target_index] = 2;

    out.write_u8(2);
    out.write_var_i32(3);
    out.write_var_i32(5);
    out.write_var_i32(7);
    out.write_var_i32(9);
    for value in pack_fixed_values(&values, 2) {
        out.write_i64(value as i64);
    }
}

fn single_biome_payload(biome_id: i32) -> Vec<u8> {
    let mut payload = Encoder::new();
    payload.write_u8(0);
    payload.write_var_i32(biome_id);
    payload.into_inner()
}

fn repeated_single_biome_payload(biome_id: i32, section_count: usize) -> Vec<u8> {
    let section = single_biome_payload(biome_id);
    section.repeat(section_count)
}

fn nbt_compound_with_string(name: &str, value: &str) -> Vec<u8> {
    let mut payload = vec![10, 8];
    payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
    payload.extend_from_slice(name.as_bytes());
    payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
    payload.extend_from_slice(value.as_bytes());
    payload.push(0);
    payload
}

fn nbt_compound_with_empty_list(name: &str, element_type: u8) -> Vec<u8> {
    let mut payload = vec![10, 9];
    write_nbt_string(&mut payload, name);
    payload.push(element_type);
    payload.extend_from_slice(&0i32.to_be_bytes());
    payload.push(0);
    payload
}

fn nbt_compound_with_long(name: &str, value: i64) -> Vec<u8> {
    let mut payload = vec![10, 4];
    write_nbt_string(&mut payload, name);
    payload.extend_from_slice(&value.to_be_bytes());
    payload.push(0);
    payload
}

fn nbt_compound_with_short(name: &str, value: i16) -> Vec<u8> {
    let mut payload = vec![10, 2];
    write_nbt_string(&mut payload, name);
    payload.extend_from_slice(&value.to_be_bytes());
    payload.push(0);
    payload
}

fn decode_test_block_entity_record(
    block_entity_type_id: i32,
    raw_nbt: Vec<u8>,
) -> BlockEntityRecord {
    decode_block_entity_data_record(
        0,
        0,
        0,
        block_entity_type_id,
        raw_nbt,
        bbb_protocol::packets::JigsawJoint::Rollable,
    )
    .unwrap()
}

fn block_entity_matrix_pos(block_entity_type_id: i32) -> ProtocolBlockPos {
    ProtocolBlockPos {
        x: 32 + block_entity_type_id.rem_euclid(16),
        y: 7,
        z: -48 + block_entity_type_id.div_euclid(16),
    }
}

fn sign_text_nbt(front: [&str; 4], back: [&str; 4]) -> Vec<u8> {
    let mut payload = vec![10];
    write_sign_text_side(&mut payload, "front_text", front);
    write_sign_text_side(&mut payload, "back_text", back);
    payload.push(0);
    payload
}

fn vault_shared_data_nbt(players: &[Uuid], connected_particles_range: Option<f64>) -> Vec<u8> {
    let mut payload = vec![10, 10];
    write_nbt_string(&mut payload, "shared_data");
    if !players.is_empty() {
        payload.push(9);
        write_nbt_string(&mut payload, "connected_players");
        payload.push(11);
        payload.extend_from_slice(&(players.len() as i32).to_be_bytes());
        for player in players {
            write_nbt_uuid_int_array(&mut payload, *player);
        }
    }
    if let Some(range) = connected_particles_range {
        payload.push(6);
        write_nbt_string(&mut payload, "connected_particles_range");
        payload.extend_from_slice(&range.to_be_bytes());
    }
    payload.push(0);
    payload.push(0);
    payload
}

fn write_sign_text_side(out: &mut Vec<u8>, name: &str, lines: [&str; 4]) {
    out.push(10);
    write_nbt_string(out, name);
    out.push(9);
    write_nbt_string(out, "messages");
    out.push(8);
    out.extend_from_slice(&4i32.to_be_bytes());
    for line in lines {
        write_nbt_string(out, line);
    }
    out.push(0);
}

fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u16).to_be_bytes());
    out.extend_from_slice(value.as_bytes());
}

fn write_nbt_uuid_int_array(out: &mut Vec<u8>, uuid: Uuid) {
    let value = uuid.as_u128();
    let ints = [
        (value >> 96) as u32,
        (value >> 64) as u32,
        (value >> 32) as u32,
        value as u32,
    ];
    out.extend_from_slice(&4i32.to_be_bytes());
    for value in ints {
        out.extend_from_slice(&(value as i32).to_be_bytes());
    }
}

fn vault_block_state_id(facing: &str) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("ominous".to_string(), "false".to_string()),
        ("vault_state".to_string(), "active".to_string()),
    ]);
    crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties("minecraft:vault", &properties)
        .unwrap()
        .id
}

fn test_block_state_id(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect();
    crate::registries::BlockStateRegistry::vanilla_26_1()
        .find_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("missing vanilla block state {name} {properties:?}"))
        .id
}

fn vault_test_player(id: i32, uuid: Uuid, position: ProtocolVec3d) -> ProtocolAddEntity {
    ProtocolAddEntity {
        id,
        uuid,
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position,
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    }
}

fn assert_close_f64(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 0.0001,
        "expected {actual} to be close to {expected}"
    );
}

fn pack_fixed_values(values: &[u64], bits_per_entry: usize) -> Vec<u64> {
    let values_per_long = 64 / bits_per_entry;
    let mut packed = vec![0; values.len().div_ceil(values_per_long)];
    let mask = (1u64 << bits_per_entry) - 1;
    for (index, value) in values.iter().copied().enumerate() {
        let cell_index = index / values_per_long;
        let bit_index = (index - cell_index * values_per_long) * bits_per_entry;
        packed[cell_index] |= (value & mask) << bit_index;
    }
    packed
}

fn set_light_nibble(layer: &mut [u8], nibble_index: usize, value: u8) {
    let byte = layer.get_mut(nibble_index / 2).unwrap();
    let shift = (nibble_index % 2) * 4;
    *byte = (*byte & !(0x0f << shift)) | ((value & 0x0f) << shift);
}

fn terrain_cell_index(x: usize, y: usize, z: usize, height: usize) -> usize {
    assert!(x < 16);
    assert!(y < height);
    assert!(z < 16);
    ((y * 16) + z) * 16 + x
}

fn light_update_data(
    sky_y_mask: &[i64],
    block_y_mask: &[i64],
    empty_sky_y_mask: &[i64],
    empty_block_y_mask: &[i64],
    sky_updates: Vec<Vec<u8>>,
    block_updates: Vec<Vec<u8>>,
) -> ProtocolLightUpdateData {
    ProtocolLightUpdateData {
        sky_y_mask: sky_y_mask.to_vec(),
        block_y_mask: block_y_mask.to_vec(),
        empty_sky_y_mask: empty_sky_y_mask.to_vec(),
        empty_block_y_mask: empty_block_y_mask.to_vec(),
        sky_updates,
        block_updates,
    }
}
