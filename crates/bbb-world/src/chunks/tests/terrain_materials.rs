use super::{synthetic_local_palette_chunk_packet, terrain_cell_index};
use crate::{
    BlockPos, ChunkPos, TerrainFluidKind, TerrainFluidState, TerrainMaterialClass, WorldDimension,
    WorldStore,
};

use bbb_protocol::packets::{
    BlockPos as ProtocolBlockPos, BlockUpdate as ProtocolBlockUpdate,
    SectionBlocksUpdate as ProtocolSectionBlocksUpdate,
};

#[test]
fn terrain_cells_classify_waterlogged_rails_as_cutout() {
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
                    z: -44,
                },
                block_state_id: 2187,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 35,
                    y: 1,
                    z: -44,
                },
                block_state_id: 2211,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 36,
                    y: 1,
                    z: -44,
                },
                block_state_id: 5727,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 37,
                    y: 1,
                    z: -44,
                },
                block_state_id: 11408,
            },
        ],
    });

    assert_eq!(applied, 4);

    let source_water = Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false));
    for (x, block_name) in [
        (34, "minecraft:powered_rail"),
        (35, "minecraft:detector_rail"),
        (36, "minecraft:rail"),
        (37, "minecraft:activator_rail"),
    ] {
        let probe = store.probe_block(BlockPos { x, y: 1, z: -44 }).unwrap();
        assert_eq!(probe.block_name.as_deref(), Some(block_name));
        assert_eq!(probe.material, TerrainMaterialClass::Cutout);
        assert_eq!(probe.fluid, source_water);
    }

    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    for local_x in 2..=5 {
        assert_eq!(
            terrain.cells[terrain_cell_index(local_x, 1, 4, 16)].fluid,
            source_water
        );
    }

    let summary = terrain.summary();
    assert_eq!(summary.fluid_state_blocks, 4);
    assert_eq!(summary.cutout_blocks, 4);
    assert_eq!(summary.opaque_blocks, 4092);
    assert_eq!(
        store
            .probe_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .sections[0]
            .fluid_count,
        4
    );
}

#[test]
fn terrain_cells_classify_ladders_and_torches_as_cutout() {
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
                    z: -43,
                },
                block_state_id: 5719,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 35,
                    y: 1,
                    z: -43,
                },
                block_state_id: 3370,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 36,
                    y: 1,
                    z: -43,
                },
                block_state_id: 3371,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 37,
                    y: 1,
                    z: -43,
                },
                block_state_id: 6885,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 38,
                    y: 1,
                    z: -43,
                },
                block_state_id: 6887,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 39,
                    y: 1,
                    z: -43,
                },
                block_state_id: 7006,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 40,
                    y: 1,
                    z: -43,
                },
                block_state_id: 7007,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 41,
                    y: 1,
                    z: -43,
                },
                block_state_id: 7011,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 42,
                    y: 1,
                    z: -43,
                },
                block_state_id: 7012,
            },
        ],
    });

    assert_eq!(applied, 9);

    let source_water = Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false));
    for (x, block_name, expected_fluid) in [
        (34, "minecraft:ladder", source_water),
        (35, "minecraft:torch", None),
        (36, "minecraft:wall_torch", None),
        (37, "minecraft:redstone_torch", None),
        (38, "minecraft:redstone_wall_torch", None),
        (39, "minecraft:soul_torch", None),
        (40, "minecraft:soul_wall_torch", None),
        (41, "minecraft:copper_torch", None),
        (42, "minecraft:copper_wall_torch", None),
    ] {
        let probe = store.probe_block(BlockPos { x, y: 1, z: -43 }).unwrap();
        assert_eq!(probe.block_name.as_deref(), Some(block_name));
        assert_eq!(probe.material, TerrainMaterialClass::Cutout);
        assert_eq!(probe.fluid, expected_fluid);
    }

    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    assert_eq!(
        terrain.cells[terrain_cell_index(2, 1, 5, 16)].fluid,
        source_water
    );
    for local_x in 3..=10 {
        assert_eq!(
            terrain.cells[terrain_cell_index(local_x, 1, 5, 16)].fluid,
            None
        );
    }

    let summary = terrain.summary();
    assert_eq!(summary.fluid_state_blocks, 1);
    assert_eq!(summary.cutout_blocks, 9);
    assert_eq!(summary.opaque_blocks, 4087);
    assert_eq!(
        store
            .probe_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .sections[0]
            .fluid_count,
        1
    );
}

#[test]
fn terrain_cells_classify_redstone_controls_as_cutout() {
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
                    z: -42,
                },
                block_state_id: 6772,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 35,
                    y: 1,
                    z: -42,
                },
                block_state_id: 6896,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 36,
                    y: 1,
                    z: -42,
                },
                block_state_id: 10676,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 37,
                    y: 1,
                    z: -42,
                },
                block_state_id: 10844,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 38,
                    y: 1,
                    z: -42,
                },
                block_state_id: 21467,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 39,
                    y: 1,
                    z: -42,
                },
                block_state_id: 22746,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 40,
                    y: 1,
                    z: -42,
                },
                block_state_id: 6796,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 41,
                    y: 1,
                    z: -42,
                },
                block_state_id: 6862,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 42,
                    y: 1,
                    z: -42,
                },
                block_state_id: 11231,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 43,
                    y: 1,
                    z: -42,
                },
                block_state_id: 11247,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 44,
                    y: 1,
                    z: -42,
                },
                block_state_id: 21047,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 45,
                    y: 1,
                    z: -42,
                },
                block_state_id: 22744,
            },
        ],
    });

    assert_eq!(applied, 12);

    for (x, block_name) in [
        (34, "minecraft:lever"),
        (35, "minecraft:stone_button"),
        (36, "minecraft:oak_button"),
        (37, "minecraft:pale_oak_button"),
        (38, "minecraft:crimson_button"),
        (39, "minecraft:polished_blackstone_button"),
        (40, "minecraft:stone_pressure_plate"),
        (41, "minecraft:oak_pressure_plate"),
        (42, "minecraft:light_weighted_pressure_plate"),
        (43, "minecraft:heavy_weighted_pressure_plate"),
        (44, "minecraft:crimson_pressure_plate"),
        (45, "minecraft:polished_blackstone_pressure_plate"),
    ] {
        let probe = store.probe_block(BlockPos { x, y: 1, z: -42 }).unwrap();
        assert_eq!(probe.block_name.as_deref(), Some(block_name));
        assert_eq!(probe.material, TerrainMaterialClass::Cutout);
        assert_eq!(probe.fluid, None);
    }

    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    for local_x in 2..=13 {
        assert_eq!(
            terrain.cells[terrain_cell_index(local_x, 1, 6, 16)].fluid,
            None
        );
    }

    let summary = terrain.summary();
    assert_eq!(summary.fluid_state_blocks, 0);
    assert_eq!(summary.cutout_blocks, 12);
    assert_eq!(summary.opaque_blocks, 4084);
    assert_eq!(
        store
            .probe_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .sections[0]
            .fluid_count,
        0
    );
}

#[test]
fn terrain_cells_classify_waterlogged_lanterns_as_cutout() {
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
                    z: -41,
                },
                block_state_id: 20837,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 35,
                    y: 1,
                    z: -41,
                },
                block_state_id: 20841,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 36,
                    y: 1,
                    z: -41,
                },
                block_state_id: 20845,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 37,
                    y: 1,
                    z: -41,
                },
                block_state_id: 20873,
            },
        ],
    });

    assert_eq!(applied, 4);

    let source_water = Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false));
    for (x, block_name) in [
        (34, "minecraft:lantern"),
        (35, "minecraft:soul_lantern"),
        (36, "minecraft:copper_lantern"),
        (37, "minecraft:waxed_oxidized_copper_lantern"),
    ] {
        let probe = store.probe_block(BlockPos { x, y: 1, z: -41 }).unwrap();
        assert_eq!(probe.block_name.as_deref(), Some(block_name));
        assert_eq!(probe.material, TerrainMaterialClass::Cutout);
        assert_eq!(probe.fluid, source_water);
    }

    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    for local_x in 2..=5 {
        assert_eq!(
            terrain.cells[terrain_cell_index(local_x, 1, 7, 16)].fluid,
            source_water
        );
    }

    let summary = terrain.summary();
    assert_eq!(summary.fluid_state_blocks, 4);
    assert_eq!(summary.cutout_blocks, 4);
    assert_eq!(summary.opaque_blocks, 4092);
    assert_eq!(
        store
            .probe_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .sections[0]
            .fluid_count,
        4
    );
}

#[test]
fn terrain_cells_classify_waterlogged_lightning_rods_as_cutout() {
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
                    z: -40,
                },
                block_state_id: 27543,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 35,
                    y: 1,
                    z: -40,
                },
                block_state_id: 27567,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 36,
                    y: 1,
                    z: -40,
                },
                block_state_id: 27639,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 37,
                    y: 1,
                    z: -40,
                },
                block_state_id: 27711,
            },
        ],
    });

    assert_eq!(applied, 4);

    let source_water = Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false));
    for (x, block_name) in [
        (34, "minecraft:lightning_rod"),
        (35, "minecraft:exposed_lightning_rod"),
        (36, "minecraft:waxed_lightning_rod"),
        (37, "minecraft:waxed_oxidized_lightning_rod"),
    ] {
        let probe = store.probe_block(BlockPos { x, y: 1, z: -40 }).unwrap();
        assert_eq!(probe.block_name.as_deref(), Some(block_name));
        assert_eq!(probe.material, TerrainMaterialClass::Cutout);
        assert_eq!(probe.fluid, source_water);
    }

    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    for local_x in 2..=5 {
        assert_eq!(
            terrain.cells[terrain_cell_index(local_x, 1, 8, 16)].fluid,
            source_water
        );
    }

    let summary = terrain.summary();
    assert_eq!(summary.fluid_state_blocks, 4);
    assert_eq!(summary.cutout_blocks, 4);
    assert_eq!(summary.opaque_blocks, 4092);
    assert_eq!(
        store
            .probe_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .sections[0]
            .fluid_count,
        4
    );
}

#[test]
fn terrain_cells_classify_dripstone_campfires_and_end_rods_as_cutout() {
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
                    z: -39,
                },
                block_state_id: 14636,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 35,
                    y: 1,
                    z: -39,
                },
                block_state_id: 20877,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 36,
                    y: 1,
                    z: -39,
                },
                block_state_id: 20909,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 37,
                    y: 1,
                    z: -39,
                },
                block_state_id: 27735,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 38,
                    y: 1,
                    z: -39,
                },
                block_state_id: 27754,
            },
        ],
    });

    assert_eq!(applied, 5);

    let source_water = Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false));
    for (x, block_name, fluid) in [
        (34, "minecraft:end_rod", None),
        (35, "minecraft:campfire", source_water),
        (36, "minecraft:soul_campfire", source_water),
        (37, "minecraft:pointed_dripstone", source_water),
        (38, "minecraft:pointed_dripstone", None),
    ] {
        let probe = store.probe_block(BlockPos { x, y: 1, z: -39 }).unwrap();
        assert_eq!(probe.block_name.as_deref(), Some(block_name));
        assert_eq!(probe.material, TerrainMaterialClass::Cutout);
        assert_eq!(probe.fluid, fluid);
    }

    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    assert_eq!(terrain.cells[terrain_cell_index(2, 1, 9, 16)].fluid, None);
    for local_x in 3..=5 {
        assert_eq!(
            terrain.cells[terrain_cell_index(local_x, 1, 9, 16)].fluid,
            source_water
        );
    }
    assert_eq!(terrain.cells[terrain_cell_index(6, 1, 9, 16)].fluid, None);

    let summary = terrain.summary();
    assert_eq!(summary.fluid_state_blocks, 3);
    assert_eq!(summary.cutout_blocks, 5);
    assert_eq!(summary.opaque_blocks, 4091);
    assert_eq!(
        store
            .probe_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .sections[0]
            .fluid_count,
        3
    );
}
