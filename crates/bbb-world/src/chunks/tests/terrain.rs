use super::{
    synthetic_local_palette_chunk_packet, synthetic_sparse_vertical_chunk_packet,
    terrain_cell_index, test_block_state_id,
};
use crate::{BlockPos, ChunkPos, WorldDimension, WorldStore};

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
fn extracted_terrain_carries_exact_solid_render_facts() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    store
        .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
        .unwrap();
    for (x, resource_id, expected) in [
        (34, "minecraft:stone", true),
        (35, "minecraft:glass", false),
    ] {
        assert!(
            store.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos { x, y: 1, z: -45 },
                block_state_id: test_block_state_id(resource_id, &[]),
            })
        );
        let cell = store
            .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .cells
            .into_iter()
            .find(|cell| cell.block_name.as_deref() == Some(resource_id))
            .unwrap();
        assert_eq!(cell.solid_render, expected);
        assert_eq!(
            store
                .probe_block(BlockPos { x, y: 1, z: -45 })
                .unwrap()
                .block_name
                .as_deref(),
            Some(resource_id)
        );
    }
}

#[test]
fn render_extraction_crops_empty_vertical_sections_but_keeps_neighbor_sampling_bands() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: -32,
        height: 80,
    });
    store
        .insert_level_chunk_with_light(synthetic_sparse_vertical_chunk_packet(Some(2)))
        .unwrap();

    let render = store
        .extract_terrain_render_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    assert_eq!(render.min_y, -16);
    assert_eq!(render.height, 48);
    assert_eq!(render.cells.len(), 16 * 48 * 16);

    let full = store
        .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    let full_offset = usize::try_from(render.min_y - full.min_y).unwrap() * 16 * 16;
    assert_eq!(
        render.cells,
        full.cells[full_offset..full_offset + render.cells.len()]
    );
}

#[test]
fn render_extraction_keeps_one_empty_section_for_an_empty_neighborhood() {
    let mut store = WorldStore::with_dimension(WorldDimension {
        min_y: -32,
        height: 80,
    });
    store
        .insert_level_chunk_with_light(synthetic_sparse_vertical_chunk_packet(None))
        .unwrap();

    let render = store
        .extract_terrain_render_chunk(ChunkPos { x: 2, z: -3 })
        .unwrap();
    assert_eq!(render.min_y, -32);
    assert_eq!(render.height, 16);
    assert!(render
        .cells
        .iter()
        .all(|cell| matches!(cell.material, crate::TerrainMaterialClass::Empty)));
}
