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
fn terrain_cells_include_vanilla_fluid_state() {
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
                block_state_id: 13332,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 35,
                    y: 1,
                    z: -45,
                },
                block_state_id: 12565,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 36,
                    y: 1,
                    z: -45,
                },
                block_state_id: 89,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 37,
                    y: 1,
                    z: -45,
                },
                block_state_id: 15294,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 38,
                    y: 1,
                    z: -45,
                },
                block_state_id: 2254,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 39,
                    y: 1,
                    z: -45,
                },
                block_state_id: 27047,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 40,
                    y: 1,
                    z: -45,
                },
                block_state_id: 7958,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 41,
                    y: 1,
                    z: -45,
                },
                block_state_id: 8118,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 42,
                    y: 1,
                    z: -45,
                },
                block_state_id: 8246,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 43,
                    y: 1,
                    z: -45,
                },
                block_state_id: 8276,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 44,
                    y: 1,
                    z: -45,
                },
                block_state_id: 5666,
            },
            ProtocolBlockUpdate {
                pos: ProtocolBlockPos {
                    x: 45,
                    y: 1,
                    z: -45,
                },
                block_state_id: 7128,
            },
        ],
    });

    assert_eq!(applied, 12);

    let source_water = Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, false));
    let falling_source_water = Some(TerrainFluidState::new(TerrainFluidKind::Water, 8, true));
    let slab = store
        .probe_block(BlockPos {
            x: 34,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(slab.block_name.as_deref(), Some("minecraft:oak_slab"));
    assert_eq!(slab.material, TerrainMaterialClass::Opaque);
    assert_eq!(slab.fluid, source_water);

    let light = store
        .probe_block(BlockPos {
            x: 35,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(light.block_name.as_deref(), Some("minecraft:light"));
    assert_eq!(light.material, TerrainMaterialClass::Invisible);
    assert_eq!(light.fluid, source_water);

    let water = store
        .probe_block(BlockPos {
            x: 36,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(water.block_name.as_deref(), Some("minecraft:water"));
    assert_eq!(water.material, TerrainMaterialClass::Fluid);
    assert_eq!(
        water.fluid,
        Some(TerrainFluidState::new(TerrainFluidKind::Water, 5, false))
    );

    let bubble_column = store
        .probe_block(BlockPos {
            x: 37,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(
        bubble_column.block_name.as_deref(),
        Some("minecraft:bubble_column")
    );
    assert_eq!(bubble_column.material, TerrainMaterialClass::Invisible);
    assert_eq!(bubble_column.fluid, source_water);

    let seagrass = store
        .probe_block(BlockPos {
            x: 38,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(seagrass.block_name.as_deref(), Some("minecraft:seagrass"));
    assert_eq!(seagrass.material, TerrainMaterialClass::Cutout);
    assert_eq!(seagrass.fluid, source_water);

    let copper_grate = store
        .probe_block(BlockPos {
            x: 39,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(
        copper_grate.block_name.as_deref(),
        Some("minecraft:copper_grate")
    );
    assert_eq!(copper_grate.material, TerrainMaterialClass::Cutout);
    assert_eq!(copper_grate.fluid, falling_source_water);

    let iron_bars = store
        .probe_block(BlockPos {
            x: 40,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(iron_bars.block_name.as_deref(), Some("minecraft:iron_bars"));
    assert_eq!(iron_bars.material, TerrainMaterialClass::Cutout);
    assert_eq!(iron_bars.fluid, source_water);

    let waxed_copper_bars = store
        .probe_block(BlockPos {
            x: 41,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(
        waxed_copper_bars.block_name.as_deref(),
        Some("minecraft:waxed_copper_bars")
    );
    assert_eq!(waxed_copper_bars.material, TerrainMaterialClass::Cutout);
    assert_eq!(waxed_copper_bars.fluid, source_water);

    let iron_chain = store
        .probe_block(BlockPos {
            x: 42,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(
        iron_chain.block_name.as_deref(),
        Some("minecraft:iron_chain")
    );
    assert_eq!(iron_chain.material, TerrainMaterialClass::Cutout);
    assert_eq!(iron_chain.fluid, source_water);

    let waxed_copper_chain = store
        .probe_block(BlockPos {
            x: 43,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(
        waxed_copper_chain.block_name.as_deref(),
        Some("minecraft:waxed_copper_chain")
    );
    assert_eq!(waxed_copper_chain.material, TerrainMaterialClass::Cutout);
    assert_eq!(waxed_copper_chain.fluid, source_water);

    let oak_door = store
        .probe_block(BlockPos {
            x: 44,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(oak_door.block_name.as_deref(), Some("minecraft:oak_door"));
    assert_eq!(oak_door.material, TerrainMaterialClass::Cutout);
    assert_eq!(oak_door.fluid, None);

    let oak_trapdoor = store
        .probe_block(BlockPos {
            x: 45,
            y: 1,
            z: -45,
        })
        .unwrap();
    assert_eq!(
        oak_trapdoor.block_name.as_deref(),
        Some("minecraft:oak_trapdoor")
    );
    assert_eq!(oak_trapdoor.material, TerrainMaterialClass::Cutout);
    assert_eq!(oak_trapdoor.fluid, source_water);

    let terrain = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    assert_eq!(
        terrain.cells[terrain_cell_index(2, 1, 3, 16)].fluid,
        source_water
    );
    assert_eq!(
        terrain.cells[terrain_cell_index(3, 1, 3, 16)].fluid,
        source_water
    );
    assert_eq!(
        terrain.cells[terrain_cell_index(4, 1, 3, 16)].fluid,
        Some(TerrainFluidState::new(TerrainFluidKind::Water, 5, false))
    );
    assert_eq!(
        terrain.cells[terrain_cell_index(5, 1, 3, 16)].fluid,
        source_water
    );
    assert_eq!(
        terrain.cells[terrain_cell_index(6, 1, 3, 16)].fluid,
        source_water
    );
    assert_eq!(
        terrain.cells[terrain_cell_index(7, 1, 3, 16)].fluid,
        falling_source_water
    );
    assert_eq!(
        terrain.cells[terrain_cell_index(8, 1, 3, 16)].fluid,
        source_water
    );
    assert_eq!(
        terrain.cells[terrain_cell_index(9, 1, 3, 16)].fluid,
        source_water
    );
    assert_eq!(
        terrain.cells[terrain_cell_index(10, 1, 3, 16)].fluid,
        source_water
    );
    assert_eq!(
        terrain.cells[terrain_cell_index(11, 1, 3, 16)].fluid,
        source_water
    );
    assert_eq!(terrain.cells[terrain_cell_index(12, 1, 3, 16)].fluid, None);
    assert_eq!(
        terrain.cells[terrain_cell_index(13, 1, 3, 16)].fluid,
        source_water
    );

    let summary = terrain.summary();
    assert_eq!(summary.fluid_state_blocks, 11);
    assert_eq!(summary.fluid_blocks, 1);
    assert_eq!(summary.invisible_blocks, 2);
    assert_eq!(summary.cutout_blocks, 8);
    assert_eq!(summary.opaque_blocks, 4085);
    assert_eq!(
        store
            .probe_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .sections[0]
            .fluid_count,
        11
    );
}
