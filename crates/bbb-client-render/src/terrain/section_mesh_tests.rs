use super::fixture::{TerrainCell, TerrainChunkFixture, TerrainFixtureWorld};
use super::*;

fn single_chunk(height: usize, cells: Vec<TerrainCell>) -> TerrainFixtureWorld {
    TerrainFixtureWorld::new(
        &[TerrainChunkFixture::new(3, -2, 0, height, cells)],
        TerrainTextureAtlas::unit(),
    )
}

fn opaque_cube() -> TerrainCell {
    TerrainCell::with_texture(TerrainMaterialClass::Opaque, 0)
}

#[test]
fn every_requested_section_gets_its_own_meshes() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 32 * 16];
    for y in [0, 16] {
        cells[mesh::cell_index(1, y, 1, 32)] = opaque_cube();
    }
    let layers = single_chunk(32, cells).build_all(0, [0.0; 3]);

    assert_eq!(layers.source_sections, 2);
    assert_eq!(
        layers
            .opaque
            .iter()
            .map(|section| (section.section_y, section.mesh.opaque_faces))
            .collect::<Vec<_>>(),
        vec![(0, 6), (1, 6)]
    );
    assert!(layers.opaque[1]
        .mesh
        .vertices
        .iter()
        .all(|vertex| (0.0..=1.0).contains(&vertex.local_position()[1])
            && (16.0..=17.0).contains(&vertex.world_position([48, 16, -32])[1])));
}

#[test]
fn section_subset_matches_the_same_full_build_sections() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 48 * 16];
    for y in [1, 17, 33] {
        cells[mesh::cell_index(1, y, 1, 48)] = opaque_cube();
    }
    let world = single_chunk(48, cells);
    let full = world.build_all(0, [0.0; 3]);
    let subset = world.build(0, [0.0; 3], &[0, 2]);

    assert_eq!(subset.source_sections, 2);
    assert_eq!(
        subset.opaque,
        vec![full.opaque[0].clone(), full.opaque[2].clone()]
    );
    assert_eq!(
        subset.cutout,
        vec![full.cutout[0].clone(), full.cutout[2].clone()]
    );
    assert_eq!(
        subset.translucent,
        vec![full.translucent[0].clone(), full.translucent[2].clone()]
    );
}

#[test]
fn section_visibility_sparse_and_empty_sections_are_all_visible() {
    let mut empty_cells = vec![TerrainCell::EMPTY; 16 * 16 * 16];
    empty_cells[0].light = TerrainLight { sky: 0, block: 0 };
    let empty_layers = single_chunk(16, empty_cells).build_all(0, [0.0; 3]);
    let empty_visibility = empty_layers.visibility[0];
    assert!(!empty_visibility.has_renderable_layers);
    for from in TerrainFace::ALL {
        for to in TerrainFace::ALL {
            assert!(empty_visibility.faces_can_see_each_other(from, to));
        }
    }

    let mut sparse_cells = vec![TerrainCell::EMPTY; 16 * 16 * 16];
    for cell in &mut sparse_cells[..255] {
        *cell = opaque_cube();
    }
    let sparse_layers = single_chunk(16, sparse_cells).build_all(0, [0.0; 3]);
    assert!(
        sparse_layers.visibility[0].faces_can_see_each_other(TerrainFace::West, TerrainFace::East)
    );
}

#[test]
fn section_visibility_distinguishes_fully_solid_and_partial_height_sections() {
    let layers = single_chunk(16, vec![opaque_cube(); 16 * 16 * 16]).build_all(0, [0.0; 3]);
    let visibility = layers.visibility[0];
    assert!(visibility.has_renderable_layers);
    for from in TerrainFace::ALL {
        for to in TerrainFace::ALL {
            assert!(!visibility.faces_can_see_each_other(from, to));
        }
    }
    let partial = single_chunk(1, vec![opaque_cube(); 16 * 16]).build_all(0, [0.0; 3]);
    assert!(partial.visibility[0].faces_can_see_each_other(TerrainFace::West, TerrainFace::East));
}

#[test]
fn section_visibility_solid_wall_separates_west_from_east() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 16 * 16];
    for y in 0..16 {
        for z in 0..16 {
            cells[mesh::cell_index(8, y, z, 16)] = opaque_cube();
        }
    }
    let layers = single_chunk(16, cells).build_all(0, [0.0; 3]);
    let visibility = layers.visibility[0];
    assert!(!visibility.faces_can_see_each_other(TerrainFace::West, TerrainFace::East));
    assert!(visibility.faces_can_see_each_other(TerrainFace::West, TerrainFace::North));
    assert!(visibility.faces_can_see_each_other(TerrainFace::East, TerrainFace::South));
}
