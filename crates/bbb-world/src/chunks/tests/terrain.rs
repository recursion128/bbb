use super::{synthetic_local_palette_chunk_packet, terrain_cell_index};
use crate::{ChunkPos, WorldDimension, WorldStore};

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
