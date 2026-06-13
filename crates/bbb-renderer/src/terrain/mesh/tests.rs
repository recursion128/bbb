use super::super::{
    build_opaque_chunk_mesh, build_opaque_terrain_meshes, build_opaque_terrain_meshes_with_atlas,
    build_terrain_mesh_layers_with_atlas, build_terrain_meshes_with_atlas, TerrainBox,
    TerrainCross, TerrainFace, TerrainLight, TerrainMesh, TerrainQuad, TerrainTint,
    TerrainTransparency, TerrainUvRect, TerrainVertex,
};
use super::*;

#[test]
fn meshes_single_opaque_block_as_six_faces() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    let snapshot = TerrainChunkSnapshot::new(2, -3, 0, 1, cells);

    let mesh = build_opaque_chunk_mesh(&snapshot);
    assert_eq!(mesh.source_sections, 1);
    assert_eq!(mesh.opaque_faces, 6);
    assert_eq!(mesh.culled_faces, 0);
    assert_eq!(mesh.vertices.len(), 24);
    assert_eq!(mesh.indices.len(), 36);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.block_state_id == 1));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.position[0] >= 33.0 && vertex.position[2] <= -45.0));
}

#[test]
fn culls_internal_faces_between_opaque_blocks() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    cells[cell_index(2, 0, 2, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let mesh = build_opaque_chunk_mesh(&snapshot);
    assert_eq!(mesh.source_sections, 1);
    assert_eq!(mesh.opaque_faces, 10);
    assert_eq!(mesh.culled_faces, 2);
    assert_eq!(mesh.vertices.len(), 40);
    assert_eq!(mesh.indices.len(), 60);
}

#[test]
fn culls_internal_faces_between_fluid_blocks() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_texture(86, TerrainMaterialClass::Fluid, 0);
    cells[cell_index(2, 0, 2, 1)] = TerrainCell::with_texture(86, TerrainMaterialClass::Fluid, 0);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let layers = build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
    assert_eq!(layers.translucent.len(), 1);
    assert_eq!(layers.translucent[0].translucent_faces, 10);
    assert_eq!(layers.translucent[0].culled_faces, 2);
    assert_eq!(layers.translucent[0].vertices.len(), 40);
    assert_eq!(layers.translucent[0].indices.len(), 60);
}

#[test]
fn fluid_box_extends_to_above_fluid_without_internal_top_face() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] =
        TerrainCell::with_shape(86, TerrainMaterialClass::Fluid, 0, fluid_box_shape(14));
    cells[cell_index(1, 1, 2, 2)] =
        TerrainCell::with_shape(87, TerrainMaterialClass::Fluid, 0, fluid_box_shape(14));
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 2, cells);

    let layers = build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
    let mesh = &layers.translucent[0];

    assert_eq!(mesh.translucent_faces, 10);
    assert_eq!(mesh.culled_faces, 2);
    assert!(mesh
        .vertices
        .iter()
        .filter(|vertex| vertex.block_state_id == 86)
        .any(|vertex| vertex.position[1] == 1.0));
    let upper_top = face_vertices(mesh, 87, [0.0, 1.0, 0.0]);
    assert_eq!(upper_top.len(), 4);
    assert!(upper_top
        .iter()
        .all(|vertex| vertex.position[1] > 1.0 && vertex.position[1] < 1.875));
}

#[test]
fn fluid_faces_use_flat_vanilla_light_without_ambient_occlusion() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 3 * 16];
    cells[cell_index(1, 1, 2, 3)] =
        TerrainCell::with_shape(86, TerrainMaterialClass::Fluid, 0, fluid_box_shape(16))
            .with_light(TerrainLight { sky: 4, block: 2 })
            .with_ambient_occlusion(true);
    cells[cell_index(1, 2, 2, 3)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 15, block: 1 });
    cells[cell_index(1, 0, 2, 3)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 3, block: 10 });
    cells[cell_index(0, 2, 2, 3)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 15 });
    cells[cell_index(1, 2, 1, 3)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 15 });
    cells[cell_index(0, 2, 1, 3)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 15 });
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 3, cells);

    let layers = build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
    let mesh = &layers.translucent[0];

    assert_face_light(mesh, 86, [0.0, 1.0, 0.0], [2.0 / 15.0, 1.0]);
    assert_face_light(mesh, 86, [0.0, 0.0, -1.0], [2.0 / 15.0, 1.0]);
    assert_face_light(mesh, 86, [0.0, -1.0, 0.0], [10.0 / 15.0, 4.0 / 15.0]);
    assert!(face_vertices(mesh, 86, [0.0, 1.0, 0.0])
        .iter()
        .all(|vertex| vertex.ambient_occlusion == 1.0));
}

#[test]
fn fluid_top_face_averages_corner_heights_from_adjacent_fluids() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(86, TerrainMaterialClass::Fluid, 0, fluid_box_shape(8));
    cells[cell_index(1, 0, 1, 1)] =
        TerrainCell::with_shape(87, TerrainMaterialClass::Fluid, 0, fluid_box_shape(16));
    cells[cell_index(0, 0, 2, 1)] =
        TerrainCell::with_shape(88, TerrainMaterialClass::Fluid, 0, fluid_box_shape(16));
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let layers = build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
    let top_vertices = face_vertices(&layers.translucent[0], 86, [0.0, 1.0, 0.0]);

    assert_float_eq(vertex_at_xz(&top_vertices, 1.0, 2.0).position[1], 1.0);
    assert_float_eq(vertex_at_xz(&top_vertices, 2.0, 2.0).position[1], 1.0);
    assert_float_eq(vertex_at_xz(&top_vertices, 1.0, 3.0).position[1], 1.0);
    assert_float_eq(vertex_at_xz(&top_vertices, 2.0, 3.0).position[1], 1.0 / 6.0);
}

#[test]
fn fluid_side_uvs_follow_corner_heights() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(86, TerrainMaterialClass::Fluid, 0, fluid_box_shape(8));
    cells[cell_index(0, 0, 2, 1)] =
        TerrainCell::with_shape(88, TerrainMaterialClass::Fluid, 0, fluid_box_shape(16));
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let layers = build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
    let north = face_vertices(&layers.translucent[0], 86, [0.0, 0.0, -1.0]);

    let east_bottom = vertex_at(&north, [2.0, 0.0, 2.0]);
    let east_top = vertex_at_approx(&north, [2.0, 1.0 / 6.0, 2.0]);
    let west_top = vertex_at(&north, [1.0, 1.0, 2.0]);
    let west_bottom = vertex_at(&north, [1.0, 0.0, 2.0]);

    assert_eq!(east_bottom.uv, [0.5, 0.5]);
    assert_float_eq(east_top.uv[0], 0.5);
    assert_float_eq(east_top.uv[1], 5.0 / 12.0);
    assert_eq!(west_top.uv, [0.0, 0.0]);
    assert_eq!(west_bottom.uv, [0.0, 0.5]);
}

#[test]
fn culls_faces_between_adjacent_chunk_snapshots() {
    let left = single_block_snapshot(0, 0, 15, 0, 2);
    let right = single_block_snapshot(1, 0, 0, 0, 2);

    let meshes = build_opaque_terrain_meshes(&[left, right]);
    assert_eq!(meshes.len(), 2);
    assert_eq!(
        meshes.iter().map(|mesh| mesh.opaque_faces).sum::<usize>(),
        10
    );
    assert_eq!(
        meshes.iter().map(|mesh| mesh.culled_faces).sum::<usize>(),
        2
    );
    assert_eq!(
        meshes.iter().map(|mesh| mesh.vertices.len()).sum::<usize>(),
        40
    );
    assert_eq!(
        meshes.iter().map(|mesh| mesh.indices.len()).sum::<usize>(),
        60
    );
}

#[test]
fn culls_fluid_faces_between_adjacent_chunk_snapshots() {
    let left = single_fluid_snapshot(0, 0, 15, 0, 2);
    let right = single_fluid_snapshot(1, 0, 0, 0, 2);

    let layers = build_terrain_mesh_layers_with_atlas(&[left, right], &TerrainTextureAtlas::unit());
    assert_eq!(layers.translucent.len(), 2);
    assert_eq!(
        layers
            .translucent
            .iter()
            .map(|mesh| mesh.translucent_faces)
            .sum::<usize>(),
        10
    );
    assert_eq!(
        layers
            .translucent
            .iter()
            .map(|mesh| mesh.culled_faces)
            .sum::<usize>(),
        2
    );
}

#[test]
fn maps_face_uvs_into_texture_atlas_rect() {
    let mut snapshot = single_block_snapshot(0, 0, 1, 0, 2);
    snapshot.cells[cell_index(1, 0, 2, 1)].texture_indices = [1; 6];
    let atlas = TerrainTextureAtlas {
        rects: vec![
            TerrainUvRect::UNIT,
            TerrainUvRect {
                min: [0.25, 0.5],
                max: [0.5, 0.75],
            },
        ],
        fallback_index: 0,
    };

    let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(mesh.vertices[0].uv, [0.25, 0.5]);
    assert_eq!(mesh.vertices[1].uv, [0.5, 0.5]);
    assert_eq!(mesh.vertices[2].uv, [0.5, 0.75]);
    assert_eq!(mesh.vertices[3].uv, [0.25, 0.75]);
}

#[test]
fn maps_different_faces_to_different_texture_rects() {
    let mut snapshot = single_block_snapshot(0, 0, 1, 0, 2);
    snapshot.cells[cell_index(1, 0, 2, 1)].texture_indices = [1, 2, 0, 0, 0, 0];
    let atlas = TerrainTextureAtlas {
        rects: vec![
            TerrainUvRect::UNIT,
            TerrainUvRect {
                min: [0.0, 0.0],
                max: [0.25, 0.25],
            },
            TerrainUvRect {
                min: [0.5, 0.5],
                max: [0.75, 0.75],
            },
        ],
        fallback_index: 0,
    };

    let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(mesh.vertices[0].uv, [0.0, 0.0]);
    assert_eq!(mesh.vertices[4].uv, [0.5, 0.5]);
}

#[test]
fn combined_terrain_mesh_includes_cutout_faces() {
    let mut snapshot = single_block_snapshot(0, 0, 1, 0, 2);
    snapshot.cells[cell_index(1, 0, 2, 1)].material = TerrainMaterialClass::Cutout;

    let opaque = build_opaque_chunk_mesh(&snapshot);
    assert_eq!(opaque.opaque_faces, 0);
    assert_eq!(opaque.cutout_faces, 0);

    let combined = build_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(combined.opaque_faces, 0);
    assert_eq!(combined.cutout_faces, 6);
    assert_eq!(combined.vertices.len(), 24);
    assert_eq!(combined.indices.len(), 36);
}

#[test]
fn cross_cutout_mesh_emits_vanilla_cross_quads() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(
        2,
        TerrainMaterialClass::Cutout,
        0,
        TerrainRenderShape::Cross {
            shade: false,
            light_emission: 0,
        },
    );
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let mesh = build_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(mesh.opaque_faces, 0);
    assert_eq!(mesh.cutout_faces, 4);
    assert_eq!(mesh.culled_faces, 0);
    assert_eq!(mesh.vertices.len(), 16);
    assert_eq!(mesh.indices.len(), 24);
    assert!(mesh.vertices.iter().all(|vertex| vertex.shade == 1.0));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.position == [1.0, 0.0, 2.0]));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.position == [2.0, 1.0, 3.0]));
}

#[test]
fn cross_cutout_faces_are_not_neighbor_culled() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(
        2,
        TerrainMaterialClass::Cutout,
        0,
        TerrainRenderShape::Cross {
            shade: true,
            light_emission: 0,
        },
    );
    cells[cell_index(2, 0, 2, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let layers = build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());

    assert_eq!(layers.opaque[0].opaque_faces, 6);
    assert_eq!(layers.cutout[0].cutout_faces, 4);
    assert_eq!(layers.cutout[0].culled_faces, 0);
}

#[test]
fn cross_layers_preserve_emissive_light() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell {
        block_state_id: 2,
        material: TerrainMaterialClass::Cutout,
        texture_indices: [0; 6],
        ambient_occlusion: true,
        light: TerrainLight { sky: 4, block: 2 },
        tint: [TerrainTint::WHITE; 6],
        render_shape: TerrainRenderShape::Crosses(vec![
            TerrainCross {
                texture_indices: [0; 6],
                tint: [TerrainTint::WHITE; 6],
                face_transparency: [TerrainTransparency::OPAQUE; 6],
                shade: false,
                light_emission: 0,
            },
            TerrainCross {
                texture_indices: [0; 6],
                tint: [TerrainTint::WHITE; 6],
                face_transparency: [TerrainTransparency::OPAQUE; 6],
                shade: false,
                light_emission: 15,
            },
        ]),
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    };
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let mesh = build_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(mesh.cutout_faces, 8);
    assert_eq!(mesh.vertices.len(), 32);
    assert_eq!(mesh.vertices[0].light, [2.0 / 15.0, 4.0 / 15.0]);
    assert_eq!(mesh.vertices[16].light, [1.0, 4.0 / 15.0]);
    assert!(mesh.vertices.iter().all(|vertex| vertex.shade == 1.0));
}

#[test]
fn box_model_mesh_uses_bounds_and_face_uv_crop() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(3, TerrainMaterialClass::Opaque, 1, slab_box_shape());
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);
    let atlas = TerrainTextureAtlas {
        rects: vec![
            TerrainUvRect::UNIT,
            TerrainUvRect {
                min: [0.25, 0.25],
                max: [0.75, 0.75],
            },
        ],
        fallback_index: 0,
    };

    let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(mesh.opaque_faces, 6);
    assert_eq!(mesh.vertices.len(), 24);
    assert!(mesh.vertices.iter().any(|vertex| vertex.position[1] == 0.5));
    assert!(!mesh.vertices.iter().any(|vertex| vertex.position[1] == 1.0));
    assert!(mesh.vertices.iter().any(|vertex| vertex.uv == [0.25, 0.5]));
}

#[test]
fn box_model_mesh_rotates_face_uv_crop() {
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    face_uvs[TerrainFace::Down.index()] = [4, 4, 8, 12];
    let mut face_uv_rotations = [0; 6];
    face_uv_rotations[TerrainFace::Down.index()] = 1;
    let shape = TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, 16, 16],
        face_present: [true; 6],
        face_uvs,
        face_uv_rotations,
        face_shade: [false; 6],
        face_light_emission: [0; 6],
        face_cull: [None; 6],
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    };
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(3, TerrainMaterialClass::Opaque, 1, shape);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(mesh.vertices[0].uv, [0.5, 0.25]);
    assert_eq!(mesh.vertices[0].shade, 1.0);
    assert_eq!(mesh.vertices[1].uv, [0.5, 0.75]);
    assert_eq!(mesh.vertices[2].uv, [0.25, 0.75]);
    assert_eq!(mesh.vertices[3].uv, [0.25, 0.25]);
}

#[test]
fn box_model_culls_only_faces_marked_by_cullface() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(3, TerrainMaterialClass::Opaque, 0, slab_box_shape());
    cells[cell_index(1, 0, 1, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    cells[cell_index(1, 0, 3, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(mesh.culled_faces, 4);
    assert_eq!(mesh.opaque_faces, 14);
}

#[test]
fn box_model_uses_cullface_direction_not_rendered_face_direction() {
    let mut face_present = [false; 6];
    face_present[TerrainFace::North.index()] = true;
    let mut face_cull = [None; 6];
    face_cull[TerrainFace::North.index()] = Some(TerrainFace::South);
    let shape = TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, 16, 16],
        face_present,
        face_uvs: [[0, 0, 16, 16]; 6],
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull,
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    };
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(3, TerrainMaterialClass::Opaque, 0, shape);
    cells[cell_index(1, 0, 3, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(mesh.culled_faces, 2);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.block_state_id != 3));
}

#[test]
fn multi_box_model_skips_absent_faces() {
    let mut upper = TerrainBox {
        from: [8, 8, 0],
        to: [16, 16, 16],
        face_present: [true; 6],
        face_uvs: [[0, 0, 16, 16]; 6],
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull: [None; 6],
        texture_indices: [0; 6],
        tint: [TerrainTint::WHITE; 6],
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    };
    upper.face_present[TerrainFace::Down.index()] = false;
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell {
        block_state_id: 4,
        material: TerrainMaterialClass::Opaque,
        texture_indices: [0; 6],
        ambient_occlusion: true,
        light: TerrainLight::FULL_BRIGHT,
        tint: [TerrainTint::WHITE; 6],
        render_shape: TerrainRenderShape::Boxes(vec![
            TerrainBox {
                from: [0, 0, 0],
                to: [16, 8, 16],
                face_present: [true; 6],
                face_uvs: [[0, 0, 16, 16]; 6],
                face_uv_rotations: [0; 6],
                face_shade: [true; 6],
                face_light_emission: [0; 6],
                face_cull: [None; 6],
                texture_indices: [0; 6],
                tint: [TerrainTint::WHITE; 6],
                face_transparency: [TerrainTransparency::OPAQUE; 6],
            },
            upper,
        ]),
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    };
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &TerrainTextureAtlas::unit())
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(mesh.opaque_faces, 11);
    assert_eq!(mesh.vertices.len(), 44);
    assert_eq!(mesh.indices.len(), 66);
}

#[test]
fn boxes_use_per_box_texture_and_tint() {
    let grass_tint = TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59);
    let foliage_tint = TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18);
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell {
        block_state_id: 4,
        material: TerrainMaterialClass::Opaque,
        texture_indices: [0; 6],
        ambient_occlusion: true,
        light: TerrainLight::FULL_BRIGHT,
        tint: [TerrainTint::WHITE; 6],
        render_shape: TerrainRenderShape::Boxes(vec![
            TerrainBox {
                from: [0, 0, 0],
                to: [16, 8, 16],
                face_present: [true; 6],
                face_uvs: [[0, 0, 16, 16]; 6],
                face_uv_rotations: [0; 6],
                face_shade: [true; 6],
                face_light_emission: [0; 6],
                face_cull: [None; 6],
                texture_indices: [1; 6],
                tint: [grass_tint; 6],
                face_transparency: [TerrainTransparency::OPAQUE; 6],
            },
            TerrainBox {
                from: [0, 8, 0],
                to: [16, 16, 16],
                face_present: [true; 6],
                face_uvs: [[0, 0, 16, 16]; 6],
                face_uv_rotations: [0; 6],
                face_shade: [true; 6],
                face_light_emission: [0; 6],
                face_cull: [None; 6],
                texture_indices: [2; 6],
                tint: [foliage_tint; 6],
                face_transparency: [TerrainTransparency::OPAQUE; 6],
            },
        ]),
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    };
    let atlas = TerrainTextureAtlas {
        rects: vec![
            TerrainUvRect::UNIT,
            TerrainUvRect {
                min: [0.25, 0.25],
                max: [0.5, 0.5],
            },
            TerrainUvRect {
                min: [0.5, 0.5],
                max: [0.75, 0.75],
            },
        ],
        fallback_index: 0,
    };
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let mesh = build_opaque_terrain_meshes_with_atlas(&[snapshot], &atlas)
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(mesh.vertices[0].uv, [0.25, 0.25]);
    assert_eq!(mesh.vertices[0].tint, grass_tint.as_shader_tint());
    assert_eq!(mesh.vertices[24].uv, [0.5, 0.5]);
    assert_eq!(mesh.vertices[24].tint, foliage_tint.as_shader_tint());
}

#[test]
fn mesh_vertices_carry_terrain_light() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0)
        .with_light(TerrainLight { sky: 9, block: 6 });
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let mesh = build_opaque_chunk_mesh(&snapshot);

    assert_eq!(mesh.vertices[0].light, [6.0 / 15.0, 9.0 / 15.0]);
}

#[test]
fn ambient_occlusion_smooths_cubic_face_vertex_light() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = TerrainCell::with_texture(42, TerrainMaterialClass::Opaque, 0)
        .with_light(TerrainLight { sky: 1, block: 1 });
    cells[cell_index(1, 1, 2, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 12, block: 12 });
    cells[cell_index(0, 1, 2, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 8, block: 4 });
    cells[cell_index(1, 1, 1, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 4, block: 8 });
    cells[cell_index(0, 1, 1, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 0, block: 0 });
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 2, cells);

    let mesh = build_opaque_chunk_mesh(&snapshot);
    let top_vertices = face_vertices(&mesh, 42, [0.0, 1.0, 0.0]);

    assert_float_eq(
        vertex_at(&top_vertices, [1.0, 1.0, 2.0]).light[0],
        9.0 / 15.0,
    );
    assert_float_eq(
        vertex_at(&top_vertices, [1.0, 1.0, 2.0]).light[1],
        9.0 / 15.0,
    );
}

#[test]
fn ambient_occlusion_flag_disables_light_smoothing() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = TerrainCell::with_texture(42, TerrainMaterialClass::Opaque, 0)
        .with_light(TerrainLight { sky: 5, block: 6 })
        .with_ambient_occlusion(false);
    cells[cell_index(1, 1, 2, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 15, block: 15 });
    cells[cell_index(0, 1, 2, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 15, block: 15 });
    cells[cell_index(1, 1, 1, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 15, block: 15 });
    cells[cell_index(0, 1, 1, 2)] =
        TerrainCell::EMPTY.with_light(TerrainLight { sky: 15, block: 15 });
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 2, cells);

    let mesh = build_opaque_chunk_mesh(&snapshot);
    let top_vertices = face_vertices(&mesh, 42, [0.0, 1.0, 0.0]);

    assert!(top_vertices
        .iter()
        .all(|vertex| vertex.light == [6.0 / 15.0, 5.0 / 15.0]));
}

#[test]
fn mesh_vertices_carry_face_tint() {
    let mut face_tints = [TerrainTint::WHITE; 6];
    face_tints[TerrainFace::Down.index()] = TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59);
    face_tints[TerrainFace::Up.index()] = TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18);
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0).with_tint(face_tints);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let mesh = build_opaque_chunk_mesh(&snapshot);

    assert_eq!(
        mesh.vertices[0].tint,
        TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59).as_shader_tint()
    );
    assert_eq!(
        mesh.vertices[4].tint,
        TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18).as_shader_tint()
    );
}

#[test]
fn cube_vertices_carry_vanilla_default_cardinal_shade() {
    let snapshot = single_block_snapshot(0, 0, 1, 0, 2);

    let mesh = build_opaque_chunk_mesh(&snapshot);

    assert_face_shade(&mesh, [0.0, -1.0, 0.0], 0.5);
    assert_face_shade(&mesh, [0.0, 1.0, 0.0], 1.0);
    assert_face_shade(&mesh, [0.0, 0.0, -1.0], 0.8);
    assert_face_shade(&mesh, [0.0, 0.0, 1.0], 0.8);
    assert_face_shade(&mesh, [-1.0, 0.0, 0.0], 0.6);
    assert_face_shade(&mesh, [1.0, 0.0, 0.0], 0.6);
}

#[test]
fn ambient_occlusion_darkens_cubic_face_corners_from_outer_neighbors() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = TerrainCell::with_texture(42, TerrainMaterialClass::Opaque, 0);
    cells[cell_index(0, 1, 2, 2)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    cells[cell_index(1, 1, 1, 2)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    cells[cell_index(0, 1, 1, 2)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 2, cells);

    let mesh = build_opaque_chunk_mesh(&snapshot);
    let top_vertices = face_vertices(&mesh, 42, [0.0, 1.0, 0.0]);

    assert_eq!(top_vertices.len(), 4);
    assert_float_eq(
        vertex_at(&top_vertices, [1.0, 1.0, 2.0]).ambient_occlusion,
        0.4,
    );
    assert_float_eq(
        vertex_at(&top_vertices, [2.0, 1.0, 3.0]).ambient_occlusion,
        1.0,
    );
}

#[test]
fn ambient_occlusion_flag_disables_corner_darkening() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 2 * 16];
    cells[cell_index(1, 0, 2, 2)] = TerrainCell::with_texture(42, TerrainMaterialClass::Opaque, 0)
        .with_ambient_occlusion(false);
    cells[cell_index(0, 1, 2, 2)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    cells[cell_index(1, 1, 1, 2)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    cells[cell_index(0, 1, 1, 2)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 2, cells);

    let mesh = build_opaque_chunk_mesh(&snapshot);
    let top_vertices = face_vertices(&mesh, 42, [0.0, 1.0, 0.0]);

    assert_eq!(top_vertices.len(), 4);
    assert!(top_vertices
        .iter()
        .all(|vertex| vertex.ambient_occlusion == 1.0));
}

#[test]
fn layer_builder_splits_opaque_and_cutout_meshes() {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    cells[cell_index(3, 0, 2, 1)] = TerrainCell::with_texture(2, TerrainMaterialClass::Cutout, 0);
    cells[cell_index(5, 0, 2, 1)] =
        TerrainCell::with_texture(3, TerrainMaterialClass::Translucent, 0);
    cells[cell_index(7, 0, 2, 1)] = TerrainCell::with_texture(4, TerrainMaterialClass::Fluid, 0);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let layers = build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());
    assert_eq!(layers.source_sections, 1);
    assert_eq!(layers.opaque.len(), 1);
    assert_eq!(layers.cutout.len(), 1);
    assert_eq!(layers.translucent.len(), 1);
    assert_eq!(layers.opaque[0].opaque_faces, 6);
    assert_eq!(layers.opaque[0].cutout_faces, 0);
    assert_eq!(layers.opaque[0].translucent_faces, 0);
    assert_eq!(layers.cutout[0].opaque_faces, 0);
    assert_eq!(layers.cutout[0].cutout_faces, 6);
    assert_eq!(layers.cutout[0].translucent_faces, 0);
    assert_eq!(layers.translucent[0].opaque_faces, 0);
    assert_eq!(layers.translucent[0].cutout_faces, 0);
    assert_eq!(layers.translucent[0].translucent_faces, 12);
}

#[test]
fn forced_translucent_cube_faces_emit_in_translucent_layer() {
    let mut face_transparency = [TerrainTransparency::OPAQUE; 6];
    face_transparency[TerrainFace::Up.index()] = TerrainTransparency::TRANSLUCENT;
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0)
        .with_face_transparency(face_transparency);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let layers = build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());

    assert_eq!(layers.opaque[0].opaque_faces, 5);
    assert_eq!(layers.translucent[0].translucent_faces, 1);
    assert_eq!(layers.cutout[0].vertices.len(), 0);
}

#[test]
fn transparent_cube_faces_emit_in_cutout_layer() {
    let mut face_transparency = [TerrainTransparency::OPAQUE; 6];
    face_transparency[TerrainFace::Up.index()] = TerrainTransparency {
        has_transparent: true,
        has_translucent: false,
    };
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0)
        .with_face_transparency(face_transparency);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let layers = build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());

    assert_eq!(layers.opaque[0].opaque_faces, 5);
    assert_eq!(layers.cutout[0].cutout_faces, 1);
    assert_eq!(layers.translucent[0].vertices.len(), 0);
}

#[test]
fn quad_shape_emits_custom_vertices_in_texture_layer() {
    let quad = TerrainQuad {
        corners: [
            [0.0, 0.0, 0.0],
            [16.0, 0.0, 0.0],
            [16.0, 16.0, 0.0],
            [0.0, 16.0, 0.0],
        ],
        normal: [0.0, 0.0, -1.0],
        uvs: [[0.0, 0.0], [0.5, 0.0], [0.5, 1.0], [0.0, 1.0]],
        cull: None,
        texture_index: 0,
        tint: TerrainTint::from_rgb_u8(128, 255, 64),
        transparency: TerrainTransparency {
            has_transparent: true,
            has_translucent: false,
        },
        shade: false,
        light_emission: 15,
    };
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] = TerrainCell::with_shape(
        7,
        TerrainMaterialClass::Opaque,
        0,
        TerrainRenderShape::Quads(vec![quad]),
    );
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let layers = build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());

    assert_eq!(layers.opaque[0].vertices.len(), 0);
    assert_eq!(layers.cutout[0].cutout_faces, 1);
    assert_eq!(layers.cutout[0].vertices.len(), 4);
    assert_eq!(layers.cutout[0].vertices[0].position, [1.0, 0.0, 2.0]);
    assert_eq!(layers.cutout[0].vertices[2].position, [2.0, 1.0, 2.0]);
    assert_eq!(layers.cutout[0].vertices[0].uv, [0.0, 0.0]);
    assert_eq!(layers.cutout[0].vertices[0].shade, 1.0);
    assert_eq!(layers.cutout[0].vertices[0].light[0], 1.0);
    assert_eq!(layers.translucent[0].vertices.len(), 0);
}

#[test]
fn forced_translucent_box_faces_emit_in_translucent_layer() {
    let mut shape = slab_box_shape();
    let TerrainRenderShape::Box {
        face_transparency, ..
    } = &mut shape
    else {
        panic!("slab helper builds a box");
    };
    face_transparency[TerrainFace::Up.index()] = TerrainTransparency::TRANSLUCENT;
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(1, 0, 2, 1)] =
        TerrainCell::with_shape(3, TerrainMaterialClass::Opaque, 1, shape);
    let snapshot = TerrainChunkSnapshot::new(0, 0, 0, 1, cells);

    let layers = build_terrain_mesh_layers_with_atlas(&[snapshot], &TerrainTextureAtlas::unit());

    assert_eq!(layers.opaque[0].opaque_faces, 5);
    assert_eq!(layers.translucent[0].translucent_faces, 1);
}

fn single_block_snapshot(
    chunk_x: i32,
    chunk_z: i32,
    x: usize,
    y: usize,
    z: usize,
) -> TerrainChunkSnapshot {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(x, y, z, 1)] = TerrainCell::with_texture(1, TerrainMaterialClass::Opaque, 0);
    TerrainChunkSnapshot::new(chunk_x, chunk_z, 0, 1, cells)
}

fn single_fluid_snapshot(
    chunk_x: i32,
    chunk_z: i32,
    x: usize,
    y: usize,
    z: usize,
) -> TerrainChunkSnapshot {
    let mut cells = vec![TerrainCell::EMPTY; 16 * 1 * 16];
    cells[cell_index(x, y, z, 1)] = TerrainCell::with_texture(86, TerrainMaterialClass::Fluid, 0);
    TerrainChunkSnapshot::new(chunk_x, chunk_z, 0, 1, cells)
}

fn slab_box_shape() -> TerrainRenderShape {
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    face_uvs[TerrainFace::North.index()] = [0, 8, 16, 16];
    face_uvs[TerrainFace::South.index()] = [0, 8, 16, 16];
    face_uvs[TerrainFace::West.index()] = [0, 8, 16, 16];
    face_uvs[TerrainFace::East.index()] = [0, 8, 16, 16];
    let mut face_cull = all_face_cull();
    face_cull[TerrainFace::Up.index()] = None;
    TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, 8, 16],
        face_present: [true; 6],
        face_uvs,
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull,
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    }
}

fn fluid_box_shape(height: u8) -> TerrainRenderShape {
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    let side_v0 = 16 - height;
    face_uvs[TerrainFace::North.index()] = [0, side_v0, 16, 16];
    face_uvs[TerrainFace::South.index()] = [0, side_v0, 16, 16];
    face_uvs[TerrainFace::West.index()] = [0, side_v0, 16, 16];
    face_uvs[TerrainFace::East.index()] = [0, side_v0, 16, 16];
    TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, height, 16],
        face_present: [true; 6],
        face_uvs,
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull: all_face_cull(),
        face_transparency: [TerrainTransparency::OPAQUE; 6],
    }
}

fn all_face_cull() -> [Option<TerrainFace>; 6] {
    TerrainFace::ALL.map(Some)
}

fn face_vertices(mesh: &TerrainMesh, block_state_id: i32, normal: [f32; 3]) -> Vec<TerrainVertex> {
    mesh.vertices
        .iter()
        .copied()
        .filter(|vertex| vertex.block_state_id == block_state_id && vertex.normal == normal)
        .collect()
}

fn vertex_at(vertices: &[TerrainVertex], position: [f32; 3]) -> TerrainVertex {
    vertices
        .iter()
        .copied()
        .find(|vertex| vertex.position == position)
        .expect("vertex exists at position")
}

fn vertex_at_approx(vertices: &[TerrainVertex], position: [f32; 3]) -> TerrainVertex {
    vertices
        .iter()
        .copied()
        .find(|vertex| {
            vertex
                .position
                .iter()
                .zip(position)
                .all(|(actual, expected)| (*actual - expected).abs() < 0.0001)
        })
        .expect("vertex exists near position")
}

fn vertex_at_xz(vertices: &[TerrainVertex], x: f32, z: f32) -> TerrainVertex {
    vertices
        .iter()
        .copied()
        .find(|vertex| vertex.position[0] == x && vertex.position[2] == z)
        .expect("vertex exists at x/z")
}

fn assert_float_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "expected {expected}, got {actual}"
    );
}

fn assert_face_shade(mesh: &TerrainMesh, normal: [f32; 3], expected: f32) {
    let vertices = face_vertices(mesh, 1, normal);
    assert_eq!(vertices.len(), 4);
    assert!(vertices
        .iter()
        .all(|vertex| (vertex.shade - expected).abs() < 0.0001));
}

fn assert_face_light(
    mesh: &TerrainMesh,
    block_state_id: i32,
    normal: [f32; 3],
    expected: [f32; 2],
) {
    let vertices = face_vertices(mesh, block_state_id, normal);
    assert_eq!(vertices.len(), 4);
    assert!(vertices.iter().all(|vertex| {
        (vertex.light[0] - expected[0]).abs() < 0.0001
            && (vertex.light[1] - expected[1]).abs() < 0.0001
    }));
}
