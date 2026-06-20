use super::*;

#[test]
fn hoglin_model_parts_match_vanilla_26_1_body_layers() {
    assert_eq!(ADULT_HOGLIN_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_HOGLIN_PARTS[0],
        [0.0, 7.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_HOGLIN_BODY.as_slice(),
        ADULT_HOGLIN_BODY_CHILDREN.as_slice(),
    );
    assert_eq!(
        ADULT_HOGLIN_MANE[0],
        ModelCubeDesc {
            min: [-0.001, -0.001, -9.001],
            size: [0.002, 10.002, 19.002],
            color: HOGLIN_RED,
        }
    );
    assert_part(
        &ADULT_HOGLIN_BODY_CHILDREN[0],
        [0.0, -14.0, -7.0],
        [0.0, 0.0, 0.0],
        ADULT_HOGLIN_MANE.as_slice(),
    );
    assert_part_tree(
        &ADULT_HOGLIN_PARTS[1],
        [0.0, 2.0, -12.0],
        [HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        ADULT_HOGLIN_HEAD.as_slice(),
        ADULT_HOGLIN_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_HOGLIN_HEAD_CHILDREN[0],
        [-6.0, -2.0, -3.0],
        [0.0, 0.0, -HOGLIN_EAR_Z_ROT],
        ADULT_HOGLIN_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &ADULT_HOGLIN_HEAD_CHILDREN[1],
        [6.0, -2.0, -3.0],
        [0.0, 0.0, HOGLIN_EAR_Z_ROT],
        ADULT_HOGLIN_LEFT_EAR.as_slice(),
    );
    assert_part(
        &ADULT_HOGLIN_HEAD_CHILDREN[2],
        [-7.0, 2.0, -12.0],
        [0.0, 0.0, 0.0],
        ADULT_HOGLIN_HORN.as_slice(),
    );
    assert_part(
        &ADULT_HOGLIN_HEAD_CHILDREN[3],
        [7.0, 2.0, -12.0],
        [0.0, 0.0, 0.0],
        ADULT_HOGLIN_HORN.as_slice(),
    );
    for (part, expected_offset, expected_cubes) in [
        (
            &ADULT_HOGLIN_PARTS[2],
            [-4.0, 10.0, -8.5],
            ADULT_HOGLIN_FRONT_LEG.as_slice(),
        ),
        (
            &ADULT_HOGLIN_PARTS[3],
            [4.0, 10.0, -8.5],
            ADULT_HOGLIN_FRONT_LEG.as_slice(),
        ),
        (
            &ADULT_HOGLIN_PARTS[4],
            [-5.0, 13.0, 10.0],
            ADULT_HOGLIN_HIND_LEG.as_slice(),
        ),
        (
            &ADULT_HOGLIN_PARTS[5],
            [5.0, 13.0, 10.0],
            ADULT_HOGLIN_HIND_LEG.as_slice(),
        ),
    ] {
        assert_part(part, expected_offset, [0.0, 0.0, 0.0], expected_cubes);
    }

    assert_eq!(BABY_HOGLIN_PARTS.len(), 6);
    assert_part_tree(
        &BABY_HOGLIN_PARTS[0],
        [0.0, 13.0, -7.0],
        [BABY_HOGLIN_HEAD_X_ROT, 0.0, 0.0],
        BABY_HOGLIN_HEAD.as_slice(),
        BABY_HOGLIN_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &BABY_HOGLIN_HEAD_CHILDREN[0],
        [-5.0, -1.0, -1.5],
        [0.0, 0.0, -BABY_HOGLIN_EAR_Z_ROT],
        BABY_HOGLIN_RIGHT_EAR.as_slice(),
    );
    assert_part(
        &BABY_HOGLIN_HEAD_CHILDREN[1],
        [5.0, -1.0, -1.5],
        [0.0, 0.0, BABY_HOGLIN_EAR_Z_ROT],
        BABY_HOGLIN_LEFT_EAR.as_slice(),
    );
    assert_part(
        &BABY_HOGLIN_PARTS[1],
        [0.0, 24.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_HOGLIN_BODY.as_slice(),
    );
    assert_eq!(
        BABY_HOGLIN_BODY[0],
        ModelCubeDesc {
            min: [-4.02, -14.02, -7.02],
            size: [8.04, 8.04, 14.04],
            color: HOGLIN_RED,
        }
    );
    assert_eq!(
        BABY_HOGLIN_BODY[1],
        ModelCubeDesc {
            min: [-0.02, -18.02, -8.02],
            size: [0.04, 6.04, 11.04],
            color: HOGLIN_RED,
        }
    );
    for (part, expected_offset) in [
        (&BABY_HOGLIN_PARTS[2], [-2.5, 18.0, 4.5]),
        (&BABY_HOGLIN_PARTS[3], [2.5, 18.0, 4.5]),
        (&BABY_HOGLIN_PARTS[4], [-2.5, 18.0, -4.5]),
        (&BABY_HOGLIN_PARTS[5], [2.5, 18.0, -4.5]),
    ] {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_HOGLIN_LEG.as_slice(),
        );
    }
}

#[test]
fn hoglin_meshes_use_vanilla_body_layers_for_hoglins_and_zoglins() {
    let adult_hoglin = entity_model_mesh(&[EntityModelInstance::hoglin(
        220,
        [0.0, 64.0, 0.0],
        0.0,
        HoglinModelFamily::Hoglin,
        false,
    )]);
    assert_eq!(adult_hoglin.opaque_faces, 66);
    assert_eq!(adult_hoglin.vertices.len(), 264);
    assert_eq!(adult_hoglin.indices.len(), 396);
    assert!(adult_hoglin
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(HOGLIN_RED, 0.78)));

    let adult_zoglin = entity_model_mesh(&[EntityModelInstance::hoglin(
        221,
        [0.0, 64.0, 0.0],
        0.0,
        HoglinModelFamily::Zoglin,
        false,
    )]);
    assert_same_geometry(&adult_zoglin, &adult_hoglin);
    assert!(adult_zoglin
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ZOGLIN_GREEN, 0.78)));

    let baby_hoglin = entity_model_mesh(&[EntityModelInstance::hoglin(
        222,
        [0.0, 64.0, 0.0],
        0.0,
        HoglinModelFamily::Hoglin,
        true,
    )]);
    assert_eq!(baby_hoglin.opaque_faces, 66);
    assert_eq!(baby_hoglin.vertices.len(), 264);
    assert_eq!(baby_hoglin.indices.len(), 396);

    let baby_zoglin = entity_model_mesh(&[EntityModelInstance::hoglin(
        223,
        [0.0, 64.0, 0.0],
        0.0,
        HoglinModelFamily::Zoglin,
        true,
    )]);
    assert_same_geometry(&baby_zoglin, &baby_hoglin);

    let (adult_min, adult_max) = mesh_extents(&adult_hoglin);
    let (baby_min, baby_max) = mesh_extents(&baby_hoglin);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn hoglin_texture_refs_match_vanilla_renderers() {
    let cases = [
        (
            HoglinModelFamily::Hoglin,
            false,
            "hoglin",
            EntityModelTextureRef {
                path: "textures/entity/hoglin/hoglin.png",
                size: [128, 64],
            },
        ),
        (
            HoglinModelFamily::Hoglin,
            true,
            "hoglin_baby",
            EntityModelTextureRef {
                path: "textures/entity/hoglin/hoglin_baby.png",
                size: [64, 64],
            },
        ),
        (
            HoglinModelFamily::Zoglin,
            false,
            "zoglin",
            EntityModelTextureRef {
                path: "textures/entity/hoglin/zoglin.png",
                size: [128, 64],
            },
        ),
        (
            HoglinModelFamily::Zoglin,
            true,
            "zoglin_baby",
            EntityModelTextureRef {
                path: "textures/entity/hoglin/zoglin_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (family, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Hoglin { family, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }

    assert_eq!(
        hoglin_entity_texture_refs(),
        &[
            EntityModelTextureRef {
                path: "textures/entity/hoglin/hoglin.png",
                size: [128, 64],
            },
            EntityModelTextureRef {
                path: "textures/entity/hoglin/hoglin_baby.png",
                size: [64, 64],
            },
            EntityModelTextureRef {
                path: "textures/entity/hoglin/zoglin.png",
                size: [128, 64],
            },
            EntityModelTextureRef {
                path: "textures/entity/hoglin/zoglin_baby.png",
                size: [64, 64],
            },
        ]
    );
    assert!(entity_model_texture_refs().contains(&HOGLIN_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&HOGLIN_BABY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ZOGLIN_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ZOGLIN_BABY_TEXTURE_REF));
}

#[test]
fn hoglin_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let cases = [
        (
            HoglinModelFamily::Hoglin,
            false,
            MODEL_LAYER_HOGLIN,
            HOGLIN_TEXTURE_REF,
            ADULT_HOGLIN_TEXTURED_PARTS.as_slice(),
        ),
        (
            HoglinModelFamily::Hoglin,
            true,
            MODEL_LAYER_HOGLIN_BABY,
            HOGLIN_BABY_TEXTURE_REF,
            BABY_HOGLIN_TEXTURED_PARTS.as_slice(),
        ),
        (
            HoglinModelFamily::Zoglin,
            false,
            MODEL_LAYER_ZOGLIN,
            ZOGLIN_TEXTURE_REF,
            ADULT_HOGLIN_TEXTURED_PARTS.as_slice(),
        ),
        (
            HoglinModelFamily::Zoglin,
            true,
            MODEL_LAYER_ZOGLIN_BABY,
            ZOGLIN_BABY_TEXTURE_REF,
            BABY_HOGLIN_TEXTURED_PARTS.as_slice(),
        ),
    ];

    for (family, baby, model_layer, texture, parts) in cases {
        let passes = hoglin_textured_layer_passes(family, baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].kind, EntityModelLayerKind::HoglinBase);
        assert_eq!(passes[0].render_type, EntityModelLayerRenderType::Cutout);
        assert_eq!(passes[0].model_layer, model_layer);
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].parts, parts);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(
            (passes[0].collector_order, passes[0].submit_sequence),
            (0, 0)
        );
    }
}

#[test]
fn hoglin_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_HOGLIN, "minecraft:hoglin#main");
    assert_eq!(MODEL_LAYER_HOGLIN_BABY, "minecraft:hoglin_baby#main");
    assert_eq!(MODEL_LAYER_ZOGLIN, "minecraft:zoglin#main");
    assert_eq!(MODEL_LAYER_ZOGLIN_BABY, "minecraft:zoglin_baby#main");
    assert_eq!(ADULT_HOGLIN_TEXTURED_PARTS.len(), 6);
    assert_eq!(BABY_HOGLIN_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        ADULT_HOGLIN_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-8.0, -7.0, -13.0],
            size: [16.0, 14.0, 26.0],
            uv_size: [16.0, 14.0, 26.0],
            tex: [1.0, 1.0],
            mirror: false,
        }
    );
    assert_eq!(
        ADULT_HOGLIN_TEXTURED_MANE[0],
        TexturedModelCubeDesc {
            min: [-0.001, -0.001, -9.001],
            size: [0.002, 10.002, 19.002],
            uv_size: [0.0, 10.0, 19.0],
            tex: [90.0, 33.0],
            mirror: false,
        }
    );
    assert_eq!(ADULT_HOGLIN_TEXTURED_HEAD[0].tex, [61.0, 1.0]);
    assert_eq!(ADULT_HOGLIN_TEXTURED_RIGHT_EAR[0].tex, [1.0, 1.0]);
    assert_eq!(ADULT_HOGLIN_TEXTURED_LEFT_EAR[0].tex, [1.0, 6.0]);
    assert_eq!(ADULT_HOGLIN_TEXTURED_RIGHT_HORN[0].tex, [10.0, 13.0]);
    assert_eq!(ADULT_HOGLIN_TEXTURED_LEFT_HORN[0].tex, [1.0, 13.0]);
    assert_eq!(ADULT_HOGLIN_TEXTURED_RIGHT_FRONT_LEG[0].tex, [66.0, 42.0]);
    assert_eq!(ADULT_HOGLIN_TEXTURED_LEFT_FRONT_LEG[0].tex, [41.0, 42.0]);
    assert_eq!(ADULT_HOGLIN_TEXTURED_RIGHT_HIND_LEG[0].tex, [21.0, 45.0]);
    assert_eq!(ADULT_HOGLIN_TEXTURED_LEFT_HIND_LEG[0].tex, [0.0, 45.0]);
    assert_eq!(
        ADULT_HOGLIN_TEXTURED_PARTS[0].pose,
        ADULT_HOGLIN_PARTS[0].pose
    );
    assert_eq!(
        ADULT_HOGLIN_TEXTURED_BODY_CHILDREN[0].pose,
        ADULT_HOGLIN_BODY_CHILDREN[0].pose
    );
    assert_eq!(
        ADULT_HOGLIN_TEXTURED_HEAD_CHILDREN[3].pose,
        ADULT_HOGLIN_HEAD_CHILDREN[3].pose
    );

    assert_eq!(BABY_HOGLIN_TEXTURED_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(BABY_HOGLIN_TEXTURED_HEAD[1].tex, [44.0, 29.0]);
    assert_eq!(BABY_HOGLIN_TEXTURED_HEAD[2].tex, [52.0, 29.0]);
    assert_eq!(BABY_HOGLIN_TEXTURED_BODY[0].tex, [0.0, 16.0]);
    assert_eq!(BABY_HOGLIN_TEXTURED_BODY[0].uv_size, [8.0, 8.0, 14.0]);
    assert_eq!(BABY_HOGLIN_TEXTURED_BODY[1].tex, [24.0, 39.0]);
    assert_eq!(BABY_HOGLIN_TEXTURED_BODY[1].uv_size, [0.0, 6.0, 11.0]);
    assert_eq!(BABY_HOGLIN_TEXTURED_RIGHT_EAR[0].tex, [32.0, 5.0]);
    assert_eq!(BABY_HOGLIN_TEXTURED_LEFT_EAR[0].tex, [32.0, 0.0]);
    assert!(BABY_HOGLIN_TEXTURED_LEFT_EAR[0].mirror);
    assert_eq!(BABY_HOGLIN_TEXTURED_RIGHT_HIND_LEG[0].tex, [0.0, 47.0]);
    assert_eq!(BABY_HOGLIN_TEXTURED_LEFT_HIND_LEG[0].tex, [12.0, 47.0]);
    assert_eq!(BABY_HOGLIN_TEXTURED_RIGHT_FRONT_LEG[0].tex, [0.0, 38.0]);
    assert_eq!(BABY_HOGLIN_TEXTURED_LEFT_FRONT_LEG[0].tex, [12.0, 38.0]);
    assert_eq!(
        BABY_HOGLIN_TEXTURED_PARTS[0].pose,
        BABY_HOGLIN_PARTS[0].pose
    );
    assert_eq!(
        BABY_HOGLIN_TEXTURED_HEAD_CHILDREN[1].pose,
        BABY_HOGLIN_HEAD_CHILDREN[1].pose
    );
}

#[test]
fn entity_texture_atlas_stitches_official_hoglin_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&hoglin_texture_images()).unwrap();

    assert_eq!(layout.width, 128);
    assert_eq!(layout.height, 256);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/hoglin/hoglin.png",
            "textures/entity/hoglin/hoglin_baby.png",
            "textures/entity/hoglin/zoglin.png",
            "textures/entity/hoglin/zoglin_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 64.0 / 256.0]);
    assert_close2(layout.entries[1].uv.min, [0.0, 64.0 / 256.0]);
    assert_close2(layout.entries[1].uv.max, [0.5, 128.0 / 256.0]);
    assert_close2(layout.entries[2].uv.min, [0.0, 128.0 / 256.0]);
    assert_close2(layout.entries[2].uv.max, [1.0, 192.0 / 256.0]);
    assert_close2(layout.entries[3].uv.min, [0.0, 192.0 / 256.0]);
    assert_close2(layout.entries[3].uv.max, [0.5, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let baby_first_pixel = rgba_offset(layout.width, 64, 0, "hoglin baby atlas row").unwrap();
    assert_eq!(&rgba[baby_first_pixel..baby_first_pixel + 4], &[1; 4]);
    let zoglin_first_pixel = rgba_offset(layout.width, 128, 0, "zoglin atlas row").unwrap();
    assert_eq!(&rgba[zoglin_first_pixel..zoglin_first_pixel + 4], &[2; 4]);
    let zoglin_baby_first_pixel =
        rgba_offset(layout.width, 192, 0, "zoglin baby atlas row").unwrap();
    assert_eq!(
        &rgba[zoglin_baby_first_pixel..zoglin_baby_first_pixel + 4],
        &[3; 4]
    );
}

#[test]
fn hoglin_textured_mesh_uses_vanilla_uvs_tints_and_family_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&hoglin_texture_images()).unwrap();
    let adult_hoglin =
        EntityModelInstance::hoglin(224, [0.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, false);
    let adult_zoglin =
        EntityModelInstance::hoglin(225, [3.0, 64.0, 0.0], 0.0, HoglinModelFamily::Zoglin, false);
    let baby_hoglin =
        EntityModelInstance::hoglin(226, [6.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, true);
    let baby_zoglin =
        EntityModelInstance::hoglin(227, [9.0, 64.0, 0.0], 0.0, HoglinModelFamily::Zoglin, true);
    let mesh = entity_model_textured_mesh(
        &[adult_hoglin, adult_zoglin, baby_hoglin, baby_zoglin],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 264);
    assert_eq!(mesh.vertices.len(), 1056);
    assert_eq!(mesh.indices.len(), 1584);
    assert_close2(mesh.vertices[0].uv, [43.0 / 128.0, 1.0 / 256.0]);
    assert_close2(mesh.vertices[264].uv, [43.0 / 128.0, 129.0 / 256.0]);
    assert_close2(mesh.vertices[528].uv, [22.0 / 128.0, 64.0 / 256.0]);
    assert_close2(mesh.vertices[792].uv, [22.0 / 128.0, 192.0 / 256.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let adult_hoglin_mesh = entity_model_textured_mesh(&[adult_hoglin], &atlas);
    let (adult_textured_min, adult_textured_max) = textured_mesh_extents(&adult_hoglin_mesh);
    let (adult_colored_min, adult_colored_max) = mesh_extents(&entity_model_mesh(&[adult_hoglin]));
    assert_close3(adult_textured_min, adult_colored_min);
    assert_close3(adult_textured_max, adult_colored_max);

    let baby_hoglin_mesh = entity_model_textured_mesh(&[baby_hoglin], &atlas);
    let (baby_textured_min, baby_textured_max) = textured_mesh_extents(&baby_hoglin_mesh);
    let (baby_colored_min, baby_colored_max) = mesh_extents(&entity_model_mesh(&[baby_hoglin]));
    assert_close3(baby_textured_min, baby_colored_min);
    assert_close3(baby_textured_max, baby_colored_max);
}

fn hoglin_texture_images() -> Vec<EntityModelTextureImage> {
    hoglin_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
