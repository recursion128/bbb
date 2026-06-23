use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn slime_cubes_match_vanilla_26_1_body_layers() {
    // Vanilla `SlimeModel` (atlas 64×32): the inner body `cube` (cutout) plus the two eyes and the
    // `mouth`, and the translucent 8³ outer shell `cube`. Each unified cube carries the colored tint
    // and the textured `uv_size`/`texOffs`; all parts sit at the identity pose.
    assert_eq!(
        SLIME_INNER_CUBE[0],
        ModelCube::new(
            [-3.0, 17.0, -3.0],
            [6.0, 6.0, 6.0],
            SLIME_GREEN,
            [6.0, 6.0, 6.0],
            [0.0, 16.0],
            false,
        )
    );
    assert_eq!(SLIME_RIGHT_EYE[0].color, SLIME_FEATURE_DARK);
    assert_eq!(SLIME_RIGHT_EYE[0].min, [-3.25, 18.0, -3.5]);
    assert_eq!(SLIME_RIGHT_EYE[0].tex, [32.0, 0.0]);
    assert_eq!(SLIME_LEFT_EYE[0].color, SLIME_FEATURE_DARK);
    assert_eq!(SLIME_LEFT_EYE[0].min, [1.25, 18.0, -3.5]);
    assert_eq!(SLIME_LEFT_EYE[0].tex, [32.0, 4.0]);
    assert_eq!(SLIME_MOUTH[0].color, SLIME_FEATURE_DARK);
    assert_eq!(SLIME_MOUTH[0].min, [0.0, 21.0, -3.5]);
    assert_eq!(SLIME_MOUTH[0].tex, [32.0, 8.0]);
    assert_eq!(
        SLIME_OUTER_CUBE[0],
        ModelCube::new(
            [-4.0, 16.0, -4.0],
            [8.0, 8.0, 8.0],
            SLIME_GREEN,
            [8.0, 8.0, 8.0],
            [0.0, 0.0],
            false,
        )
    );
}

#[test]
fn magma_cube_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla `LavaSlimeModel.createBodyLayer` (atlas 64×64): eight stacked 8×1×8 outer segments
    // climbing the atlas `v` ladder, plus the inner 4³ `inside_cube` core.
    let segment_tex = [
        [0.0, 0.0],
        [0.0, 9.0],
        [0.0, 18.0],
        [0.0, 27.0],
        [32.0, 0.0],
        [32.0, 9.0],
        [32.0, 18.0],
        [32.0, 27.0],
    ];
    for (index, tex) in segment_tex.into_iter().enumerate() {
        let cube = magma_cube_segment_cube(index);
        assert_eq!(
            cube,
            ModelCube::new(
                [-4.0, 16.0 + index as f32, -4.0],
                [8.0, 1.0, 8.0],
                MAGMA_CUBE_ORANGE,
                [8.0, 1.0, 8.0],
                tex,
                false,
            )
        );
    }
    assert_eq!(
        MAGMA_CUBE_INSIDE_CUBE[0],
        ModelCube::new(
            [-2.0, 18.0, -2.0],
            [4.0, 4.0, 4.0],
            MAGMA_CUBE_CORE,
            [4.0, 4.0, 4.0],
            [24.0, 40.0],
            false,
        )
    );
}

#[test]
fn slime_and_magma_cube_layer_passes_match_vanilla_renderers() {
    let slime = slime_textured_layer_passes();
    assert_eq!(slime.len(), 2);
    assert_eq!(slime[0].kind, EntityModelLayerKind::SlimeBase);
    assert_eq!(slime[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(slime[0].model_layer, MODEL_LAYER_SLIME);
    assert_eq!(slime[0].texture, SLIME_TEXTURE_REF);
    // The vestigial `parts` slices are nulled; emit builds the `SlimeModel`/`SlimeOuterModel` trees.
    assert!(slime[0].parts.is_empty());
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
    assert!(slime[1].parts.is_empty());
    assert_eq!(slime[1].visibility, EntityModelLayerVisibility::All);
    assert_eq!(slime[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((slime[1].collector_order, slime[1].submit_sequence), (1, 1));

    let magma = magma_cube_textured_layer_passes();
    assert_eq!(magma.len(), 1);
    assert_eq!(magma[0].kind, EntityModelLayerKind::MagmaCubeBase);
    assert_eq!(magma[0].render_type, EntityModelLayerRenderType::Cutout);
    assert_eq!(magma[0].model_layer, MODEL_LAYER_MAGMA_CUBE);
    assert_eq!(magma[0].texture, MAGMA_CUBE_TEXTURE_REF);
    assert!(magma[0].parts.is_empty());
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
