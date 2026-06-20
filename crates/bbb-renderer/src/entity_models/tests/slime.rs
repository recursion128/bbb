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
fn slime_and_magma_cube_textured_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(MODEL_LAYER_SLIME, "minecraft:slime#main");
    assert_eq!(MODEL_LAYER_SLIME_OUTER, "minecraft:slime#outer");
    assert_eq!(MODEL_LAYER_MAGMA_CUBE, "minecraft:magma_cube#main");
    assert_eq!(
        SLIME_INNER_TEXTURED_CUBE[0],
        TexturedModelCubeDesc {
            min: [-3.0, 17.0, -3.0],
            size: [6.0, 6.0, 6.0],
            uv_size: [6.0, 6.0, 6.0],
            tex: [0.0, 16.0],
            mirror: false,
        }
    );
    assert_eq!(
        SLIME_OUTER_TEXTURED_CUBE[0],
        TexturedModelCubeDesc {
            min: [-4.0, 16.0, -4.0],
            size: [8.0, 8.0, 8.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(SLIME_INNER_TEXTURED_PARTS.len(), 4);
    assert_eq!(SLIME_OUTER_TEXTURED_PARTS.len(), 1);
    assert_eq!(
        MAGMA_CUBE_TEXTURED_SEGMENT_4[0],
        TexturedModelCubeDesc {
            min: [-4.0, 20.0, -4.0],
            size: [8.0, 1.0, 8.0],
            uv_size: [8.0, 1.0, 8.0],
            tex: [32.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        MAGMA_CUBE_INSIDE_TEXTURED_CUBE[0],
        TexturedModelCubeDesc {
            min: [-2.0, 18.0, -2.0],
            size: [4.0, 4.0, 4.0],
            uv_size: [4.0, 4.0, 4.0],
            tex: [24.0, 40.0],
            mirror: false,
        }
    );
    assert_eq!(MAGMA_CUBE_TEXTURED_PARTS.len(), 9);
}

#[test]
fn slime_and_magma_cube_layer_passes_match_vanilla_renderers() {
    let slime = slime_textured_layer_passes();
    assert_eq!(slime.len(), 2);
    assert_eq!(slime[0].kind, EntityModelLayerKind::SlimeBase);
    assert_eq!(slime[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(slime[0].model_layer, MODEL_LAYER_SLIME);
    assert_eq!(slime[0].texture, SLIME_TEXTURE_REF);
    assert_eq!(slime[0].parts, SLIME_INNER_TEXTURED_PARTS.as_slice());
    assert_eq!(slime[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(slime[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((slime[0].collector_order, slime[0].submit_sequence), (0, 0));
    assert_eq!(slime[1].kind, EntityModelLayerKind::SlimeOuter);
    assert_eq!(
        slime[1].render_type,
        EntityModelLayerRenderType::Translucent
    );
    assert_eq!(slime[1].model_layer, MODEL_LAYER_SLIME_OUTER);
    assert_eq!(slime[1].texture, SLIME_TEXTURE_REF);
    assert_eq!(slime[1].parts, SLIME_OUTER_TEXTURED_PARTS.as_slice());
    assert_eq!(slime[1].visibility, EntityModelLayerVisibility::All);
    assert_eq!(slime[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((slime[1].collector_order, slime[1].submit_sequence), (1, 1));

    let magma = magma_cube_textured_layer_passes();
    assert_eq!(magma.len(), 1);
    assert_eq!(magma[0].kind, EntityModelLayerKind::MagmaCubeBase);
    assert_eq!(magma[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(magma[0].model_layer, MODEL_LAYER_MAGMA_CUBE);
    assert_eq!(magma[0].texture, MAGMA_CUBE_TEXTURE_REF);
    assert_eq!(magma[0].parts, MAGMA_CUBE_TEXTURED_PARTS.as_slice());
    assert_eq!(magma[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(magma[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((magma[0].collector_order, magma[0].submit_sequence), (0, 0));
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
fn entity_texture_atlas_stitches_official_slime_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&slime_texture_images()).unwrap();

    assert_eq!(
        slime_entity_texture_refs(),
        &[SLIME_TEXTURE_REF, MAGMA_CUBE_TEXTURE_REF]
    );
    assert!(entity_model_texture_refs().contains(&SLIME_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&MAGMA_CUBE_TEXTURE_REF));
    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 96);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/slime/slime.png",
            "textures/entity/slime/magmacube.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 32.0 / 96.0]);
    assert_close2(layout.entries[1].uv.min, [0.0, 32.0 / 96.0]);
    assert_close2(layout.entries[1].uv.max, [1.0, 1.0]);
    let magma_first_pixel = rgba_offset(layout.width, 32, 0, "test").unwrap();
    assert_eq!(&rgba[0..4], &[0; 4]);
    assert_eq!(&rgba[magma_first_pixel..magma_first_pixel + 4], &[1; 4]);
}

#[test]
fn slime_and_magma_cube_textured_meshes_use_vanilla_uvs_and_layer_buckets() {
    let (atlas, _) = build_entity_model_texture_atlas(&slime_texture_images()).unwrap();
    let slime = entity_model_textured_meshes(
        &[EntityModelInstance::slime(117, [0.0, 64.0, 0.0], 0.0, 1)],
        &atlas,
    );

    assert_eq!(slime.cutout.cutout_faces, 24);
    assert_eq!(slime.cutout.vertices.len(), 96);
    assert_eq!(slime.cutout.indices.len(), 144);
    assert_eq!(slime.translucent.cutout_faces, 6);
    assert_eq!(slime.translucent.vertices.len(), 24);
    assert_eq!(slime.translucent.indices.len(), 36);
    assert!(slime.eyes.vertices.is_empty());
    assert_close2(slime.cutout.vertices[0].uv, [12.0 / 64.0, 16.0 / 96.0]);
    assert_close2(slime.translucent.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(slime
        .cutout
        .vertices
        .iter()
        .chain(slime.translucent.vertices.iter())
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (slime_outer_min, slime_outer_max) = textured_mesh_extents(&slime.translucent);
    assert_close3(slime_outer_min, [-0.24975, 64.0, -0.24975]);
    assert_close3(slime_outer_max, [0.24975, 64.4995, 0.24975]);

    let magma = entity_model_textured_meshes(
        &[EntityModelInstance::magma_cube(
            80,
            [0.0, 64.0, 0.0],
            0.0,
            3,
        )],
        &atlas,
    );
    assert_eq!(magma.cutout.cutout_faces, 54);
    assert_eq!(magma.cutout.vertices.len(), 216);
    assert_eq!(magma.cutout.indices.len(), 324);
    assert!(magma.translucent.vertices.is_empty());
    assert!(magma.eyes.vertices.is_empty());
    assert_close2(magma.cutout.vertices[0].uv, [16.0 / 64.0, 32.0 / 96.0]);
    assert!(magma
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (magma_min, magma_max) = textured_mesh_extents(&magma.cutout);
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

fn slime_texture_images() -> Vec<EntityModelTextureImage> {
    slime_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
