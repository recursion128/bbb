use super::*;

#[test]
fn entity_texture_atlas_stitches_official_cow_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&cow_texture_images()).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 384);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/cow/cow_temperate.png",
            "textures/entity/cow/cow_temperate_baby.png",
            "textures/entity/cow/cow_warm.png",
            "textures/entity/cow/cow_warm_baby.png",
            "textures/entity/cow/cow_cold.png",
            "textures/entity/cow/cow_cold_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 64.0 / 384.0]);
    assert_close2(layout.entries[3].uv.min, [0.0, 192.0 / 384.0]);
    assert_close2(layout.entries[3].uv.max, [1.0, 256.0 / 384.0]);
    assert_close2(layout.entries[4].uv.min, [0.0, 256.0 / 384.0]);
    assert_close2(layout.entries[4].uv.max, [1.0, 320.0 / 384.0]);
    let warm_baby_first_pixel = rgba_offset(layout.width, 192, 0, "test").unwrap();
    assert_eq!(
        &rgba[warm_baby_first_pixel..warm_baby_first_pixel + 4],
        &[3; 4]
    );
    let cold_first_pixel = rgba_offset(layout.width, 256, 0, "test").unwrap();
    assert_eq!(&rgba[cold_first_pixel..cold_first_pixel + 4], &[4; 4]);
}

#[test]
fn cow_textured_mesh_uses_vanilla_uvs_tints_and_variant_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&cow_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[
            EntityModelInstance::cow_variant(
                601,
                [0.0, 64.0, 0.0],
                0.0,
                CowModelVariant::Temperate,
                false,
            ),
            EntityModelInstance::cow_variant(
                602,
                [1.0, 64.0, 0.0],
                0.0,
                CowModelVariant::Cold,
                false,
            ),
            EntityModelInstance::cow_variant(
                603,
                [2.0, 64.0, 0.0],
                0.0,
                CowModelVariant::Warm,
                true,
            ),
        ],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 180);
    assert_eq!(mesh.vertices.len(), 720);
    assert_eq!(mesh.indices.len(), 1080);
    assert_close2(mesh.vertices[0].uv, [14.0 / 64.0, 0.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[240].uv, [14.0 / 64.0, 256.0 / 384.0]);
    assert_eq!(mesh.vertices[240].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[504].uv, [11.0 / 64.0, 210.0 / 384.0]);
    assert_eq!(mesh.vertices[504].tint, [1.0, 1.0, 1.0, 1.0]);
    let (min, max) = textured_mesh_extents(&mesh);
    assert_close3(min, [-0.375, 64.001, -0.65625]);
    assert_close3(max, [2.25, 65.5635, 1.0]);
}

#[test]
fn cow_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ADULT_COW_HEAD,
        [
            ModelCubeDesc {
                min: [-4.0, -4.0, -6.0],
                size: [8.0, 8.0, 6.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-3.0, 1.0, -7.0],
                size: [6.0, 3.0, 1.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-5.0, -5.0, -5.0],
                size: [1.0, 3.0, 1.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [4.0, -5.0, -5.0],
                size: [1.0, 3.0, 1.0],
                color: COW_BROWN,
            },
        ]
    );
    assert_eq!(ADULT_COW_PARTS.len(), 6);
    assert_part(
        &ADULT_COW_PARTS[0],
        [0.0, 4.0, -8.0],
        [0.0, 0.0, 0.0],
        ADULT_COW_HEAD.as_slice(),
    );
    assert_part(
        &ADULT_COW_PARTS[1],
        [0.0, 5.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_COW_BODY.as_slice(),
    );
    for (part, expected_offset) in ADULT_COW_PARTS[2..].iter().zip([
        [-4.0, 12.0, 7.0],
        [4.0, 12.0, 7.0],
        [-4.0, 12.0, -5.0],
        [4.0, 12.0, -5.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_COW_LEG.as_slice(),
        );
    }
}

#[test]
fn cow_warm_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        WARM_COW_HEAD,
        [
            ModelCubeDesc {
                min: [-4.0, -4.0, -6.0],
                size: [8.0, 8.0, 6.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-3.0, 1.0, -7.0],
                size: [6.0, 3.0, 1.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-8.0, -3.0, -5.0],
                size: [4.0, 2.0, 2.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-8.0, -5.0, -5.0],
                size: [2.0, 2.0, 2.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [4.0, -3.0, -5.0],
                size: [4.0, 2.0, 2.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [6.0, -5.0, -5.0],
                size: [2.0, 2.0, 2.0],
                color: COW_BROWN,
            },
        ]
    );

    assert_eq!(WARM_COW_PARTS.len(), 6);
    assert_part(
        &WARM_COW_PARTS[0],
        [0.0, 4.0, -8.0],
        [0.0, 0.0, 0.0],
        WARM_COW_HEAD.as_slice(),
    );
    assert_part(
        &WARM_COW_PARTS[1],
        [0.0, 5.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_COW_BODY.as_slice(),
    );
}

#[test]
fn cow_cold_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        COLD_COW_BODY,
        [
            ModelCubeDesc {
                min: [-6.5, -10.5, -7.5],
                size: [13.0, 19.0, 11.0],
                color: COW_COLD_FUR,
            },
            ModelCubeDesc {
                min: [-6.0, -10.0, -7.0],
                size: [12.0, 18.0, 10.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-2.0, 2.0, -8.0],
                size: [4.0, 6.0, 1.0],
                color: COW_BROWN,
            },
        ]
    );
    assert_eq!(
        COLD_COW_HEAD_CHILDREN,
        [
            ModelPartDesc {
                pose: PartPose {
                    offset: [-4.5, -2.5, -3.5],
                    rotation: [1.5708, 0.0, 0.0],
                },
                cubes: &COLD_COW_RIGHT_HORN,
                children: &[],
            },
            ModelPartDesc {
                pose: PartPose {
                    offset: [5.5, -2.5, -5.0],
                    rotation: [1.5708, 0.0, 0.0],
                },
                cubes: &COLD_COW_LEFT_HORN,
                children: &[],
            },
        ]
    );

    assert_eq!(COLD_COW_PARTS.len(), 6);
    assert_part_tree(
        &COLD_COW_PARTS[0],
        [0.0, 4.0, -8.0],
        [0.0, 0.0, 0.0],
        COLD_COW_HEAD.as_slice(),
        COLD_COW_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &COLD_COW_PARTS[1],
        [0.0, 5.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        COLD_COW_BODY.as_slice(),
    );

    assert_eq!(
        cow_model_parts(CowModelVariant::Temperate, false),
        ADULT_COW_PARTS.as_slice()
    );
    assert_eq!(
        cow_model_parts(CowModelVariant::Warm, false),
        WARM_COW_PARTS.as_slice()
    );
    assert_eq!(
        cow_model_parts(CowModelVariant::Cold, false),
        COLD_COW_PARTS.as_slice()
    );
    assert_eq!(
        cow_model_parts(CowModelVariant::Cold, true),
        BABY_COW_PARTS.as_slice()
    );
}

#[test]
fn cow_adult_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow(92, [0.0, 64.0, 0.0], 0.0, false)]);

    assert_eq!(mesh.opaque_faces, 60);
    assert_eq!(mesh.vertices.len(), 240);
    assert_eq!(mesh.indices.len(), 360);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.375, 64.001, -0.625]);
    assert_close3(max, [0.375, 65.5635, 0.9375]);
}

#[test]
fn cow_warm_adult_model_mesh_uses_vanilla_warm_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow_variant(
        94,
        [0.0, 64.0, 0.0],
        0.0,
        CowModelVariant::Warm,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 72);
    assert_eq!(mesh.vertices.len(), 288);
    assert_eq!(mesh.indices.len(), 432);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.5, 64.001, -0.625]);
    assert_close3(max, [0.5, 65.5635, 0.9375]);
}

#[test]
fn cow_cold_adult_model_mesh_uses_vanilla_cold_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow_variant(
        95,
        [0.0, 64.0, 0.0],
        0.0,
        CowModelVariant::Cold,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 66);
    assert_eq!(mesh.vertices.len(), 264);
    assert_eq!(mesh.indices.len(), 396);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.40625, 64.001, -0.65625]);
    assert_close3(max, [0.40625, 65.501, 1.0]);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(COW_COLD_FUR, 0.78)));
}

#[test]
fn cow_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_COW_HEAD,
        [
            ModelCubeDesc {
                min: [-3.0, -4.569, -4.8333],
                size: [6.0, 6.0, 5.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [3.0, -5.569, -3.8333],
                size: [1.0, 2.0, 1.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-4.0, -5.569, -3.8333],
                size: [1.0, 2.0, 1.0],
                color: COW_BROWN,
            },
            ModelCubeDesc {
                min: [-2.0, -1.569, -5.8333],
                size: [4.0, 3.0, 1.0],
                color: COW_BROWN,
            },
        ]
    );
    assert_eq!(BABY_COW_PARTS.len(), 6);
    assert_part(
        &BABY_COW_PARTS[0],
        [0.0, 13.569, -5.1667],
        [0.0, 0.0, 0.0],
        BABY_COW_HEAD.as_slice(),
    );
    assert_part(
        &BABY_COW_PARTS[1],
        [3.0, 19.0, -5.0],
        [0.0, 0.0, 0.0],
        BABY_COW_BODY.as_slice(),
    );
    for (part, expected_offset) in BABY_COW_PARTS[2..].iter().zip([
        [-2.5, 18.0, -3.5],
        [2.5, 18.0, -3.5],
        [-2.5, 18.0, 3.5],
        [2.5, 18.0, 3.5],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_COW_LEG.as_slice(),
        );
    }
}

#[test]
fn cow_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::cow(93, [0.0, 64.0, 0.0], 0.0, true)]);

    assert_eq!(mesh.opaque_faces, 54);
    assert_eq!(mesh.vertices.len(), 216);
    assert_eq!(mesh.indices.len(), 324);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.001, 0.6875]);
}

#[test]
fn cow_texture_refs_match_vanilla_renderers() {
    let cow_cases = [
        (
            CowModelVariant::Temperate,
            false,
            "cow_temperate",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_temperate.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Temperate,
            true,
            "cow_temperate_baby",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_temperate_baby.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Warm,
            false,
            "cow_warm",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_warm.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Warm,
            true,
            "cow_warm_baby",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_warm_baby.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Cold,
            false,
            "cow_cold",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_cold.png",
                size: [64, 64],
            },
        ),
        (
            CowModelVariant::Cold,
            true,
            "cow_cold_baby",
            EntityModelTextureRef {
                path: "textures/entity/cow/cow_cold_baby.png",
                size: [64, 64],
            },
        ),
    ];
    for (variant, baby, model_key, texture) in cow_cases {
        let kind = EntityModelKind::Cow { variant, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn cow_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let temperate = cow_textured_layer_passes(CowModelVariant::Temperate, false);
    assert_eq!(temperate.len(), 1);
    assert_eq!(temperate[0].kind, EntityModelLayerKind::CowBase);
    assert_eq!(temperate[0].model_layer, MODEL_LAYER_COW);
    assert_eq!(temperate[0].texture, COW_TEMPERATE_TEXTURE_REF);
    assert_eq!(temperate[0].parts, ADULT_COW_TEXTURED_PARTS.as_slice());
    assert_eq!(temperate[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (temperate[0].collector_order, temperate[0].submit_sequence),
        (0, 0)
    );

    let temperate_baby = cow_textured_layer_passes(CowModelVariant::Temperate, true);
    assert_eq!(temperate_baby[0].model_layer, MODEL_LAYER_COW_BABY);
    assert_eq!(temperate_baby[0].texture, COW_TEMPERATE_BABY_TEXTURE_REF);
    assert_eq!(temperate_baby[0].parts, BABY_COW_TEXTURED_PARTS.as_slice());

    let warm = cow_textured_layer_passes(CowModelVariant::Warm, false);
    assert_eq!(warm[0].model_layer, MODEL_LAYER_WARM_COW);
    assert_eq!(warm[0].texture, COW_WARM_TEXTURE_REF);
    assert_eq!(warm[0].parts, WARM_COW_TEXTURED_PARTS.as_slice());

    let warm_baby = cow_textured_layer_passes(CowModelVariant::Warm, true);
    assert_eq!(warm_baby[0].model_layer, MODEL_LAYER_WARM_COW_BABY);
    assert_eq!(warm_baby[0].texture, COW_WARM_BABY_TEXTURE_REF);
    assert_eq!(warm_baby[0].parts, BABY_COW_TEXTURED_PARTS.as_slice());

    let cold = cow_textured_layer_passes(CowModelVariant::Cold, false);
    assert_eq!(cold[0].model_layer, MODEL_LAYER_COLD_COW);
    assert_eq!(cold[0].texture, COW_COLD_TEXTURE_REF);
    assert_eq!(cold[0].parts, COLD_COW_TEXTURED_PARTS.as_slice());

    let cold_baby = cow_textured_layer_passes(CowModelVariant::Cold, true);
    assert_eq!(cold_baby[0].model_layer, MODEL_LAYER_COLD_COW_BABY);
    assert_eq!(cold_baby[0].texture, COW_COLD_BABY_TEXTURE_REF);
    assert_eq!(cold_baby[0].parts, BABY_COW_TEXTURED_PARTS.as_slice());
}

#[test]
fn cow_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_COW, "minecraft:cow#main");
    assert_eq!(MODEL_LAYER_COW_BABY, "minecraft:cow_baby#main");
    assert_eq!(MODEL_LAYER_WARM_COW, "minecraft:warm_cow#main");
    assert_eq!(MODEL_LAYER_WARM_COW_BABY, "minecraft:warm_cow_baby#main");
    assert_eq!(MODEL_LAYER_COLD_COW, "minecraft:cold_cow#main");
    assert_eq!(MODEL_LAYER_COLD_COW_BABY, "minecraft:cold_cow_baby#main");
    assert_eq!(
        ADULT_COW_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -4.0, -6.0],
            size: [8.0, 8.0, 6.0],
            uv_size: [8.0, 8.0, 6.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ADULT_COW_TEXTURED_BODY[1],
        TexturedModelCubeDesc {
            min: [-2.0, 2.0, -8.0],
            size: [4.0, 6.0, 1.0],
            uv_size: [4.0, 6.0, 1.0],
            tex: [52.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        WARM_COW_TEXTURED_HEAD[4],
        TexturedModelCubeDesc {
            min: [4.0, -3.0, -5.0],
            size: [4.0, 2.0, 2.0],
            uv_size: [4.0, 2.0, 2.0],
            tex: [27.0, 0.0],
            mirror: true,
        }
    );
    assert_eq!(
        COLD_COW_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-6.5, -10.5, -7.5],
            size: [13.0, 19.0, 11.0],
            uv_size: [12.0, 18.0, 10.0],
            tex: [20.0, 32.0],
            mirror: false,
        }
    );
    assert_eq!(
        COLD_COW_TEXTURED_HEAD_CHILDREN[0],
        TexturedModelPartDesc {
            pose: COLD_COW_HEAD_CHILDREN[0].pose,
            cubes: &COLD_COW_TEXTURED_RIGHT_HORN,
            children: &[],
        }
    );
    assert_eq!(
        BABY_COW_TEXTURED_HEAD[2],
        TexturedModelCubeDesc {
            min: [-4.0, -5.569, -3.8333],
            size: [1.0, 2.0, 1.0],
            uv_size: [1.0, 2.0, 1.0],
            tex: [4.0, 29.0],
            mirror: true,
        }
    );
    assert_eq!(
        BABY_COW_TEXTURED_LEFT_HIND_LEG[0],
        TexturedModelCubeDesc {
            min: [-1.5, 0.0, -1.5],
            size: [3.0, 6.0, 3.0],
            uv_size: [3.0, 6.0, 3.0],
            tex: [34.0, 27.0],
            mirror: false,
        }
    );
}

fn cow_texture_images() -> Vec<EntityModelTextureImage> {
    cow_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
