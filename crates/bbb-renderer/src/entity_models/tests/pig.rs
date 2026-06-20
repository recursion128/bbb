use super::*;

#[test]
fn pig_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        ADULT_PIG_HEAD,
        [
            ModelCubeDesc {
                min: [-4.0, -4.0, -8.0],
                size: [8.0, 8.0, 8.0],
                color: PIG_PINK,
            },
            ModelCubeDesc {
                min: [-2.0, 0.0, -9.0],
                size: [4.0, 3.0, 1.0],
                color: PIG_PINK,
            },
        ]
    );
    assert_eq!(
        ADULT_PIG_BODY[0],
        ModelCubeDesc {
            min: [-5.0, -10.0, -7.0],
            size: [10.0, 16.0, 8.0],
            color: PIG_PINK,
        }
    );
    assert_eq!(
        ADULT_PIG_LEG[0],
        ModelCubeDesc {
            min: [-2.0, 0.0, -2.0],
            size: [4.0, 6.0, 4.0],
            color: PIG_PINK,
        }
    );

    assert_eq!(ADULT_PIG_PARTS.len(), 6);
    assert_part(
        &ADULT_PIG_PARTS[0],
        [0.0, 12.0, -6.0],
        [0.0, 0.0, 0.0],
        ADULT_PIG_HEAD.as_slice(),
    );
    assert_part(
        &ADULT_PIG_PARTS[1],
        [0.0, 11.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_PIG_BODY.as_slice(),
    );

    for (part, expected_offset) in ADULT_PIG_PARTS[2..].iter().zip([
        [-3.0, 18.0, 7.0],
        [3.0, 18.0, 7.0],
        [-3.0, 18.0, -5.0],
        [3.0, 18.0, -5.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_PIG_LEG.as_slice(),
        );
    }
}

#[test]
fn pig_cold_adult_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        COLD_PIG_BODY,
        [
            ModelCubeDesc {
                min: [-5.0, -10.0, -7.0],
                size: [10.0, 16.0, 8.0],
                color: PIG_PINK,
            },
            ModelCubeDesc {
                min: [-5.5, -10.5, -7.5],
                size: [11.0, 17.0, 9.0],
                color: PIG_COLD_FUR,
            },
        ]
    );

    assert_eq!(COLD_PIG_PARTS.len(), 6);
    assert_part(
        &COLD_PIG_PARTS[0],
        [0.0, 12.0, -6.0],
        [0.0, 0.0, 0.0],
        ADULT_PIG_HEAD.as_slice(),
    );
    assert_part(
        &COLD_PIG_PARTS[1],
        [0.0, 11.0, 2.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        COLD_PIG_BODY.as_slice(),
    );

    for (part, expected_offset) in COLD_PIG_PARTS[2..].iter().zip([
        [-3.0, 18.0, 7.0],
        [3.0, 18.0, 7.0],
        [-3.0, 18.0, -5.0],
        [3.0, 18.0, -5.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            ADULT_PIG_LEG.as_slice(),
        );
    }

    assert_eq!(
        pig_model_parts(PigModelVariant::Temperate, false),
        ADULT_PIG_PARTS.as_slice()
    );
    assert_eq!(
        pig_model_parts(PigModelVariant::Warm, false),
        ADULT_PIG_PARTS.as_slice()
    );
    assert_eq!(
        pig_model_parts(PigModelVariant::Cold, false),
        COLD_PIG_PARTS.as_slice()
    );
    assert_eq!(
        pig_model_parts(PigModelVariant::Cold, true),
        BABY_PIG_PARTS.as_slice()
    );
}

#[test]
fn pig_adult_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::pig(
        90,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.3125, 64.001, -0.5625]);
    assert_close3(max, [0.3125, 65.001, 0.9375]);
}

#[test]
fn pig_cold_adult_model_mesh_uses_vanilla_cold_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::pig(
        92,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Cold,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 48);
    assert_eq!(mesh.vertices.len(), 192);
    assert_eq!(mesh.indices.len(), 288);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.34375, 64.001, -0.5625]);
    assert_close3(max, [0.34375, 65.001, 0.9375]);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PIG_COLD_FUR, 0.78)));
}

#[test]
fn pig_baby_model_parts_match_vanilla_26_1_body_layer() {
    assert_eq!(
        BABY_PIG_BODY[0],
        ModelCubeDesc {
            min: [-3.5, -3.0, -4.5],
            size: [7.0, 6.0, 9.0],
            color: PIG_PINK,
        }
    );
    assert_eq!(
        BABY_PIG_HEAD,
        [
            ModelCubeDesc {
                min: [-3.525, -5.025, -5.025],
                size: [7.05, 6.05, 6.05],
                color: PIG_PINK,
            },
            ModelCubeDesc {
                min: [-1.515, -1.99, -6.015],
                size: [3.03, 2.03, 1.03],
                color: PIG_PINK,
            },
        ]
    );
    assert_eq!(
        BABY_PIG_LEG[0],
        ModelCubeDesc {
            min: [-1.0, 0.0, -1.0],
            size: [2.0, 2.0, 2.0],
            color: PIG_PINK,
        }
    );

    assert_eq!(BABY_PIG_PARTS.len(), 6);
    assert_part(
        &BABY_PIG_PARTS[0],
        [0.0, 19.0, 0.5],
        [0.0, 0.0, 0.0],
        BABY_PIG_BODY.as_slice(),
    );
    assert_part(
        &BABY_PIG_PARTS[1],
        [0.0, 19.0, -2.0],
        [0.0, 0.0, 0.0],
        BABY_PIG_HEAD.as_slice(),
    );

    for (part, expected_offset) in BABY_PIG_PARTS[2..].iter().zip([
        [2.5, 22.0, -3.0],
        [-2.5, 22.0, -3.0],
        [2.5, 22.0, 4.0],
        [-2.5, 22.0, 4.0],
    ]) {
        assert_part(
            part,
            expected_offset,
            [0.0, 0.0, 0.0],
            BABY_PIG_LEG.as_slice(),
        );
    }
}

#[test]
fn pig_baby_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::pig(
        91,
        [0.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Warm,
        true,
    )]);

    assert_eq!(mesh.opaque_faces, 42);
    assert_eq!(mesh.vertices.len(), 168);
    assert_eq!(mesh.indices.len(), 252);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.2203125, 64.001, -0.3125]);
    assert_close3(max, [0.2203125, 64.62756, 0.5009375]);
}

#[test]
fn pig_texture_refs_match_vanilla_variant_assets() {
    let cases = [
        (
            PigModelVariant::Temperate,
            false,
            "pig_temperate",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_temperate.png",
                size: [64, 64],
            },
        ),
        (
            PigModelVariant::Temperate,
            true,
            "pig_temperate_baby",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_temperate_baby.png",
                size: [32, 32],
            },
        ),
        (
            PigModelVariant::Warm,
            false,
            "pig_warm",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_warm.png",
                size: [64, 64],
            },
        ),
        (
            PigModelVariant::Warm,
            true,
            "pig_warm_baby",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_warm_baby.png",
                size: [32, 32],
            },
        ),
        (
            PigModelVariant::Cold,
            false,
            "pig_cold",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_cold.png",
                size: [64, 64],
            },
        ),
        (
            PigModelVariant::Cold,
            true,
            "pig_cold_baby",
            EntityModelTextureRef {
                path: "textures/entity/pig/pig_cold_baby.png",
                size: [32, 32],
            },
        ),
    ];

    for (variant, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Pig { variant, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn pig_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let temperate = pig_textured_layer_passes(PigModelVariant::Temperate, false);
    assert_eq!(temperate.len(), 1);
    assert_eq!(temperate[0].kind, EntityModelLayerKind::PigBase);
    assert_eq!(temperate[0].model_layer, MODEL_LAYER_PIG);
    assert_eq!(temperate[0].texture, PIG_TEMPERATE_TEXTURE_REF);
    assert_eq!(temperate[0].parts, ADULT_PIG_TEXTURED_PARTS.as_slice());
    assert_eq!(temperate[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (temperate[0].collector_order, temperate[0].submit_sequence),
        (0, 0)
    );

    let warm_baby = pig_textured_layer_passes(PigModelVariant::Warm, true);
    assert_eq!(warm_baby[0].model_layer, MODEL_LAYER_PIG_BABY);
    assert_eq!(warm_baby[0].texture, PIG_WARM_BABY_TEXTURE_REF);
    assert_eq!(warm_baby[0].parts, BABY_PIG_TEXTURED_PARTS.as_slice());

    let cold_adult = pig_textured_layer_passes(PigModelVariant::Cold, false);
    assert_eq!(cold_adult[0].model_layer, MODEL_LAYER_COLD_PIG);
    assert_eq!(cold_adult[0].texture, PIG_COLD_TEXTURE_REF);
    assert_eq!(cold_adult[0].parts, COLD_PIG_TEXTURED_PARTS.as_slice());
}

#[test]
fn pig_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_PIG, "minecraft:pig#main");
    assert_eq!(MODEL_LAYER_PIG_BABY, "minecraft:pig_baby#main");
    assert_eq!(MODEL_LAYER_COLD_PIG, "minecraft:cold_pig#main");
    assert_eq!(ADULT_PIG_TEXTURED_PARTS.len(), 6);
    assert_eq!(COLD_PIG_TEXTURED_PARTS.len(), 6);
    assert_eq!(BABY_PIG_TEXTURED_PARTS.len(), 6);
    assert_eq!(
        ADULT_PIG_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-4.0, -4.0, -8.0],
            size: [8.0, 8.0, 8.0],
            uv_size: [8.0, 8.0, 8.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(ADULT_PIG_TEXTURED_HEAD[1].tex, [16.0, 16.0]);
    assert_eq!(ADULT_PIG_TEXTURED_BODY[0].tex, [28.0, 8.0]);
    assert_eq!(
        COLD_PIG_TEXTURED_BODY[1],
        TexturedModelCubeDesc {
            min: [-5.5, -10.5, -7.5],
            size: [11.0, 17.0, 9.0],
            uv_size: [10.0, 16.0, 8.0],
            tex: [28.0, 32.0],
            mirror: false,
        }
    );
    assert_eq!(
        BABY_PIG_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-3.525, -5.025, -5.025],
            size: [7.05, 6.05, 6.05],
            uv_size: [7.0, 6.0, 6.0],
            tex: [0.0, 15.0],
            mirror: false,
        }
    );
    assert_eq!(BABY_PIG_TEXTURED_HEAD[1].tex, [6.0, 27.0]);
    assert_eq!(BABY_PIG_TEXTURED_RIGHT_FRONT_LEG[0].tex, [23.0, 0.0]);
    assert_eq!(BABY_PIG_TEXTURED_RIGHT_HIND_LEG[0].tex, [23.0, 4.0]);
    assert_eq!(ADULT_PIG_TEXTURED_PARTS[0].pose, ADULT_PIG_PARTS[0].pose);
    assert_eq!(COLD_PIG_TEXTURED_PARTS[1].pose, COLD_PIG_PARTS[1].pose);
    assert_eq!(BABY_PIG_TEXTURED_PARTS[1].pose, BABY_PIG_PARTS[1].pose);
}

#[test]
fn entity_texture_atlas_stitches_official_pig_png_slots() {
    let images = pig_texture_images();

    let (layout, rgba) = build_entity_model_texture_atlas(&images).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 288);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/pig/pig_temperate.png",
            "textures/entity/pig/pig_temperate_baby.png",
            "textures/entity/pig/pig_warm.png",
            "textures/entity/pig/pig_warm_baby.png",
            "textures/entity/pig/pig_cold.png",
            "textures/entity/pig/pig_cold_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 64.0 / 288.0]);
    assert_close2(layout.entries[1].uv.min, [0.0, 64.0 / 288.0]);
    assert_close2(layout.entries[1].uv.max, [0.5, 96.0 / 288.0]);
    assert_close2(layout.entries[4].uv.min, [0.0, 192.0 / 288.0]);
    assert_close2(layout.entries[4].uv.max, [1.0, 256.0 / 288.0]);
    let warm_baby_first_pixel = rgba_offset(layout.width, 160, 0, "test").unwrap();
    assert_eq!(
        &rgba[warm_baby_first_pixel..warm_baby_first_pixel + 4],
        &[3; 4]
    );
    let cold_first_pixel = rgba_offset(layout.width, 192, 0, "test").unwrap();
    assert_eq!(&rgba[cold_first_pixel..cold_first_pixel + 4], &[4; 4]);
}

#[test]
fn pig_textured_mesh_uses_vanilla_uvs_tints_and_variant_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&pig_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[
            EntityModelInstance::pig(
                501,
                [0.0, 64.0, 0.0],
                0.0,
                PigModelVariant::Temperate,
                false,
            ),
            EntityModelInstance::pig(502, [1.0, 64.0, 0.0], 0.0, PigModelVariant::Cold, false),
            EntityModelInstance::pig(503, [2.0, 64.0, 0.0], 0.0, PigModelVariant::Warm, true),
        ],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 132);
    assert_eq!(mesh.vertices.len(), 528);
    assert_eq!(mesh.indices.len(), 792);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[168].uv, [16.0 / 64.0, 192.0 / 288.0]);
    assert_eq!(mesh.vertices[168].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[360].uv, [16.0 / 64.0, 160.0 / 288.0]);
    assert_eq!(mesh.vertices[360].tint, [1.0, 1.0, 1.0, 1.0]);
    let (min, max) = textured_mesh_extents(&mesh);
    assert!(max[0] - min[0] > 2.0);
    assert_close3([min[1], max[1], max[2] - min[2]], [64.001, 65.001, 1.5]);
}

fn pig_texture_images() -> Vec<EntityModelTextureImage> {
    pig_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
