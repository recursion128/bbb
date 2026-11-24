use super::*;
use crate::entity_models::model_layers::magma_cube_model;

#[test]
fn slime_and_magma_cube_layer_passes_match_vanilla_renderers() {
    let slime = slime_textured_layer_passes();
    assert_eq!(slime.len(), 2);
    assert_eq!(
        slime[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(slime[0].texture, SLIME_TEXTURE_REF);
    // The vestigial `parts` slices are nulled; emit builds the `SlimeModel`/`SlimeOuterModel` trees.
    assert_eq!(slime[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(slime[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(slime[0].order, 0);
    assert_eq!(
        slime[1].render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(slime[1].texture, SLIME_TEXTURE_REF);
    assert_eq!(slime[1].visibility, EntityModelLayerVisibility::All);
    assert_eq!(slime[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(slime[1].order, 1);

    let magma = magma_cube_textured_layer_passes();
    assert_eq!(magma.len(), 1);
    assert_eq!(
        magma[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(magma[0].texture, MAGMA_CUBE_TEXTURE_REF);
    assert_eq!(magma[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(magma[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(magma[0].order, 0);
}

#[test]
fn magma_cube_squish_fans_the_segments_apart_vertically() {
    // Vanilla `LavaSlimeModel.setupAnim`: `cubeN.y = -(4 - N) * max(0, squish) * 1.7`.
    // Only a positive (jump-stretch) squish spreads the eight stacked lava slices —
    // the lower segments sink, the upper ones rise; the negative landing splat leaves
    // them flush. The overall body scale is handled by the root transform.
    let base = EntityModelInstance::magma_cube(80, [0.0, 64.0, 0.0], 0.0, 1);

    // Rest (squish 0): every segment holds its baked y (offset 0).
    let mut resting = magma_cube_model(&base);
    for name in ["cube0", "cube4", "cube7"] {
        assert_eq!(
            resting.child_mut(name).pose.offset[1],
            0.0,
            "{name} sits flush at rest"
        );
    }

    // Positive squish 0.5: cube0 sinks to -(4)·0.5·1.7, cube4 (the midpoint) stays
    // put, cube7 rises to -(4-7)·0.5·1.7.
    let mut stretched = magma_cube_model(&base.with(|s| s.kind.slime_mut().squish = 0.5));
    assert!(
        (stretched.child_mut("cube0").pose.offset[1] - (-4.0 * 0.5 * 1.7)).abs() < 1.0e-6,
        "lowest segment sinks: {}",
        stretched.child_mut("cube0").pose.offset[1]
    );
    assert_eq!(
        stretched.child_mut("cube4").pose.offset[1],
        0.0,
        "the midpoint segment stays put"
    );
    assert!(
        (stretched.child_mut("cube7").pose.offset[1] - (3.0 * 0.5 * 1.7)).abs() < 1.0e-6,
        "highest segment rises: {}",
        stretched.child_mut("cube7").pose.offset[1]
    );

    // The negative landing squish is clamped at 0, so the segments stay flush.
    let mut splat = magma_cube_model(&base.with(|s| s.kind.slime_mut().squish = -0.5));
    assert_eq!(
        splat.child_mut("cube0").pose.offset[1],
        0.0,
        "the landing splat keeps the segments flush"
    );
}

#[test]
fn entity_texture_atlas_stitches_official_slime_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&slime_texture_images()).unwrap();

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
    let slime_instance = EntityModelInstance::slime(117, [0.0, 64.0, 0.0], 0.0, 1).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let slime = entity_model_textured_meshes(&[slime_instance], &atlas);

    assert_eq!(slime.cutout.cutout_faces, 24);
    assert_eq!(slime.cutout.vertices.len(), 96);
    assert_eq!(slime.cutout.indices.len(), 144);
    assert_eq!(slime.translucent.cutout_faces, 6);
    assert_eq!(slime.translucent.vertices.len(), 24);
    assert_eq!(slime.translucent.indices.len(), 36);
    assert!(slime.eyes.vertices.is_empty());
    let cube = ModelLayer::SLIME.bake().part_vertex_range("cube").unwrap();
    assert_close2(
        slime.cutout.vertices[cube.start].uv,
        [12.0 / 64.0, 16.0 / 96.0],
    );
    assert_close2(slime.translucent.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(slime
        .cutout
        .vertices
        .iter()
        .chain(slime.translucent.vertices.iter())
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(slime.cutout.vertices.iter().all(|vertex| vertex.light
        == slime_instance.render_state.shader_light()
        && vertex.overlay == slime_instance.render_state.overlay_coords()));
    assert!(slime.translucent.vertices.iter().all(|vertex| vertex.light
        == slime_instance.render_state.shader_light()
        && vertex.overlay == [0.0, slime_instance.render_state.overlay_coords()[1]]));
    let (slime_outer_min, slime_outer_max) = textured_mesh_extents(&slime.translucent);
    assert_close3(slime_outer_min, [-0.24975, 64.0, -0.24975]);
    assert_close3(slime_outer_max, [0.24975, 64.4995, 0.24975]);

    let glowing_invisible_slime = slime_instance
        .with_invisible(true)
        .with_appears_glowing(true);
    let glowing_slime = entity_model_textured_meshes(&[glowing_invisible_slime], &atlas);

    assert!(glowing_slime.cutout.vertices.is_empty());
    assert!(glowing_slime.translucent.vertices.is_empty());
    assert!(glowing_slime.eyes.vertices.is_empty());

    let magma_instance = EntityModelInstance::magma_cube(80, [0.0, 64.0, 0.0], 0.0, 3).with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let magma = entity_model_textured_meshes(&[magma_instance], &atlas);

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
    assert!(magma.cutout.vertices.iter().all(|vertex| vertex.light
        == magma_instance.render_state.shader_light()
        && vertex.overlay == magma_instance.render_state.overlay_coords()));
    let (magma_min, magma_max) = textured_mesh_extents(&magma.cutout);
    assert_close3(magma_min, [-0.75, 64.003, -0.75]);
    assert_close3(magma_max, [0.75, 65.503, 0.75]);
}

fn slime_texture_images() -> Vec<EntityModelTextureImage> {
    [SLIME_TEXTURE_REF, MAGMA_CUBE_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
