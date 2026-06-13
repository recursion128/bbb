use super::{
    sample_terrain_light, BlockEntityRecord, ChunkState, LightData, NbtPayloadSummary, PaletteKind,
    LIGHT_ARRAY_BYTES,
};
use crate::{
    section_block_index, BlockPos, ChunkPos, TerrainLight, TerrainMaterialClass, WorldDimension,
    WorldStore,
};

use bbb_protocol::codec::Encoder;
use bbb_protocol::packets::{
    BlockEntityData as ProtocolBlockEntityData, BlockPos as ProtocolBlockPos,
    BlockUpdate as ProtocolBlockUpdate, ChunkBiomeData as ProtocolChunkBiomeData,
    ChunkHeightmapData, ChunkPos as ProtocolChunkPos, ChunksBiomes as ProtocolChunksBiomes,
    LevelChunkBlockEntity, LevelChunkData, LevelChunkWithLight, LightUpdate as ProtocolLightUpdate,
    LightUpdateData as ProtocolLightUpdateData, SectionBlocksUpdate as ProtocolSectionBlocksUpdate,
    SetChunkCacheCenter as ProtocolSetChunkCacheCenter,
    SetChunkCacheRadius as ProtocolSetChunkCacheRadius,
};

#[test]
fn decodes_level_chunk_with_light_structure() {
    let packet = synthetic_level_chunk_packet();
    let mut store = WorldStore::new();
    let pos = store.insert_level_chunk_with_light(packet).unwrap();
    let chunk = store.probe_chunk(pos).unwrap();

    assert_eq!(pos, ChunkPos { x: 1, z: -2 });
    assert_eq!(chunk.state, ChunkState::Decoded);
    assert_eq!(chunk.heightmaps.len(), 1);
    assert_eq!(chunk.heightmaps[0].kind_id, 1);
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
fn extracts_terrain_chunk_summary() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    let summary = terrain.summary();
    assert_eq!(summary.total_blocks, 4096);
    assert_eq!(summary.opaque_blocks, 4096);
    assert_eq!(summary.empty_blocks, 0);
    assert_eq!(summary.cutout_blocks, 0);
    assert_eq!(
        terrain.cells[terrain_cell_index(2, 1, 3, 16)].biome_id,
        Some(4)
    );
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
fn applies_block_entity_data_update() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();

    let raw_nbt = nbt_compound_with_string("id", "minecraft:chest");
    let applied = store
        .apply_block_entity_data(ProtocolBlockEntityData {
            pos: ProtocolBlockPos {
                x: 33,
                y: 7,
                z: -46,
            },
            block_entity_type_id: 9,
            raw_nbt: raw_nbt.clone(),
        })
        .unwrap();

    assert!(applied);
    assert_eq!(store.counters().block_entity_updates_received, 1);
    assert_eq!(store.counters().block_entity_updates_applied, 1);

    let chunk = store.probe_chunk(ChunkPos { x: 2, z: -3 }).unwrap();
    assert_eq!(chunk.block_entities.len(), 1);
    assert_eq!(
        chunk.block_entities[0],
        BlockEntityRecord {
            local_x: 1,
            y: 7,
            local_z: 2,
            type_id: 9,
            nbt: Some(NbtPayloadSummary {
                root_type: 10,
                byte_len: raw_nbt.len(),
            }),
        }
    );

    let replacement_nbt = nbt_compound_with_string("id", "minecraft:furnace");
    assert!(store
        .apply_block_entity_data(ProtocolBlockEntityData {
            pos: ProtocolBlockPos {
                x: 33,
                y: 7,
                z: -46,
            },
            block_entity_type_id: 11,
            raw_nbt: replacement_nbt,
        })
        .unwrap());
    let chunk = store.probe_chunk(ChunkPos { x: 2, z: -3 }).unwrap();
    assert_eq!(chunk.block_entities.len(), 1);
    assert_eq!(chunk.block_entities[0].type_id, 11);

    let missing_chunk_applied = store
        .apply_block_entity_data(ProtocolBlockEntityData {
            pos: ProtocolBlockPos {
                x: 800,
                y: 7,
                z: -46,
            },
            block_entity_type_id: 9,
            raw_nbt: vec![0],
        })
        .unwrap();
    assert!(!missing_chunk_applied);
    assert_eq!(store.counters().block_entity_updates_received, 3);
    assert_eq!(store.counters().block_entity_updates_applied, 2);
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
                kind_id: 1,
                data: vec![42],
            }],
            section_data: sections,
            block_entities: vec![LevelChunkBlockEntity {
                packed_xz: 0xab,
                y: 64,
                block_entity_type_id: 7,
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
    let mut sections = Encoder::new();
    sections.write_i16(4096);
    sections.write_i16(0);
    write_local_block_palette(&mut sections);
    sections.write_u8(0);
    sections.write_var_i32(4);
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

fn nbt_compound_with_string(name: &str, value: &str) -> Vec<u8> {
    let mut payload = vec![10, 8];
    payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
    payload.extend_from_slice(name.as_bytes());
    payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
    payload.extend_from_slice(value.as_bytes());
    payload.push(0);
    payload
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
