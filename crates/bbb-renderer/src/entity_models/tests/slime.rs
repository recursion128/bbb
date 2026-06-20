use super::*;

#[test]
fn slime_and_magma_cube_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(SLIME_PARTS.len(), 5);
    assert_part(
        &SLIME_PARTS[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        SLIME_INNER_CUBE.as_slice(),
    );
    assert_part(
        &SLIME_PARTS[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        SLIME_RIGHT_EYE.as_slice(),
    );
    assert_part(
        &SLIME_PARTS[2],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        SLIME_LEFT_EYE.as_slice(),
    );
    assert_part(
        &SLIME_PARTS[3],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        SLIME_MOUTH.as_slice(),
    );
    assert_part(
        &SLIME_PARTS[4],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        SLIME_OUTER_CUBE.as_slice(),
    );

    let magma_segments = [
        MAGMA_CUBE_SEGMENT_0.as_slice(),
        MAGMA_CUBE_SEGMENT_1.as_slice(),
        MAGMA_CUBE_SEGMENT_2.as_slice(),
        MAGMA_CUBE_SEGMENT_3.as_slice(),
        MAGMA_CUBE_SEGMENT_4.as_slice(),
        MAGMA_CUBE_SEGMENT_5.as_slice(),
        MAGMA_CUBE_SEGMENT_6.as_slice(),
        MAGMA_CUBE_SEGMENT_7.as_slice(),
    ];
    for (index, (part, cubes)) in MAGMA_CUBE_PARTS[..8].iter().zip(magma_segments).enumerate() {
        assert_part(part, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], cubes);
        assert_eq!(part.cubes[0].min, [-4.0, 16.0 + index as f32, -4.0]);
        assert_eq!(part.cubes[0].size, [8.0, 1.0, 8.0]);
    }
    assert_part(
        &MAGMA_CUBE_PARTS[8],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        MAGMA_CUBE_INSIDE_CUBE.as_slice(),
    );
}

#[test]
fn slime_and_magma_cube_meshes_use_vanilla_size_scaling() {
    let slime = entity_model_mesh(&[EntityModelInstance::slime(117, [0.0, 64.0, 0.0], 0.0, 1)]);
    assert_eq!(slime.opaque_faces, 30);
    assert_eq!(slime.vertices.len(), 120);
    assert_eq!(slime.indices.len(), 180);
    let (slime_min, slime_max) = mesh_extents(&slime);
    assert_close3(slime_min, [-0.24975, 64.0, -0.24975]);
    assert_close3(slime_max, [0.24975, 64.4995, 0.24975]);

    let large_slime =
        entity_model_mesh(&[EntityModelInstance::slime(117, [0.0, 64.0, 0.0], 0.0, 4)]);
    assert_eq!(large_slime.opaque_faces, slime.opaque_faces);
    let (large_slime_min, large_slime_max) = mesh_extents(&large_slime);
    assert_close3(large_slime_min, [-0.999, 64.00299, -0.999]);
    assert_close3(large_slime_max, [0.999, 66.00099, 0.999]);

    let magma_cube = entity_model_mesh(&[EntityModelInstance::magma_cube(
        80,
        [0.0, 64.0, 0.0],
        0.0,
        3,
    )]);
    assert_eq!(magma_cube.opaque_faces, 54);
    assert_eq!(magma_cube.vertices.len(), 216);
    assert_eq!(magma_cube.indices.len(), 324);
    let (magma_min, magma_max) = mesh_extents(&magma_cube);
    assert_close3(magma_min, [-0.75, 64.003, -0.75]);
    assert_close3(magma_max, [0.75, 65.503, 0.75]);
}

#[test]
fn slime_and_magma_cube_texture_refs_match_vanilla_renderers() {
    assert_eq!(EntityModelKind::Slime { size: 4 }.model_key(), "slime");
    assert_eq!(
        EntityModelKind::Slime { size: 4 }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/slime/slime.png",
            size: [64, 32],
        })
    );
    assert_eq!(
        EntityModelKind::MagmaCube { size: 3 }.model_key(),
        "magma_cube"
    );
    assert_eq!(
        EntityModelKind::MagmaCube { size: 3 }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/slime/magmacube.png",
            size: [64, 64],
        })
    );
}
