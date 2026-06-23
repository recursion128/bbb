use super::*;

#[test]
fn chicken_model_parts_match_vanilla_26_1_layers() {
    assert_eq!(ADULT_CHICKEN_PARTS.len(), 6);
    assert_part_tree(
        &ADULT_CHICKEN_PARTS[0],
        [0.0, 15.0, -4.0],
        [0.0, 0.0, 0.0],
        ADULT_CHICKEN_HEAD.as_slice(),
        ADULT_CHICKEN_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &ADULT_CHICKEN_HEAD_CHILDREN[0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_CHICKEN_BEAK.as_slice(),
    );
    assert_part(
        &ADULT_CHICKEN_HEAD_CHILDREN[1],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        ADULT_CHICKEN_RED_THING.as_slice(),
    );
    assert_part(
        &ADULT_CHICKEN_PARTS[1],
        [0.0, 16.0, 0.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        ADULT_CHICKEN_BODY.as_slice(),
    );
    assert_part(
        &ADULT_CHICKEN_PARTS[2],
        [-2.0, 19.0, 1.0],
        [0.0, 0.0, 0.0],
        ADULT_CHICKEN_LEG.as_slice(),
    );
    assert_part(
        &ADULT_CHICKEN_PARTS[3],
        [1.0, 19.0, 1.0],
        [0.0, 0.0, 0.0],
        ADULT_CHICKEN_LEG.as_slice(),
    );

    assert_eq!(COLD_CHICKEN_PARTS.len(), 6);
    assert_part_tree(
        &COLD_CHICKEN_PARTS[0],
        [0.0, 15.0, -4.0],
        [0.0, 0.0, 0.0],
        COLD_CHICKEN_HEAD.as_slice(),
        ADULT_CHICKEN_HEAD_CHILDREN.as_slice(),
    );
    assert_part(
        &COLD_CHICKEN_PARTS[1],
        [0.0, 16.0, 0.0],
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
        COLD_CHICKEN_BODY.as_slice(),
    );
    assert_eq!(
        COLD_CHICKEN_HEAD[1],
        ModelCubeDesc {
            min: [-3.0, -7.0, -2.015],
            size: [6.0, 3.0, 4.0],
            color: CHICKEN_WING,
        }
    );
    assert_eq!(
        COLD_CHICKEN_BODY[1],
        ModelCubeDesc {
            min: [0.0, 3.0, -1.0],
            size: [0.0, 3.0, 5.0],
            color: CHICKEN_WING,
        }
    );

    assert_eq!(BABY_CHICKEN_PARTS.len(), 5);
    assert_part(
        &BABY_CHICKEN_PARTS[0],
        [0.0, 20.25, -1.25],
        [0.0, 0.0, 0.0],
        BABY_CHICKEN_BODY.as_slice(),
    );
    assert_part(
        &BABY_CHICKEN_PARTS[1],
        [1.0, 22.0, 0.5],
        [0.0, 0.0, 0.0],
        BABY_CHICKEN_LEFT_LEG.as_slice(),
    );
    assert_part(
        &BABY_CHICKEN_PARTS[4],
        [-2.0, 20.0, 0.0],
        [0.0, 0.0, 0.0],
        BABY_CHICKEN_LEFT_WING.as_slice(),
    );

    assert_eq!(
        chicken_part_trees(ChickenModelVariant::Temperate, false).0,
        ADULT_CHICKEN_PARTS.as_slice()
    );
    assert_eq!(
        chicken_part_trees(ChickenModelVariant::Warm, false).0,
        ADULT_CHICKEN_PARTS.as_slice()
    );
    assert_eq!(
        chicken_part_trees(ChickenModelVariant::Cold, false).0,
        COLD_CHICKEN_PARTS.as_slice()
    );
    assert_eq!(
        chicken_part_trees(ChickenModelVariant::Cold, true).0,
        BABY_CHICKEN_PARTS.as_slice()
    );
}

#[test]
fn chicken_adult_model_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::chicken(
        26,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 48);
    assert_eq!(mesh.vertices.len(), 192);
    assert_eq!(mesh.indices.len(), 288);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.25]);
    assert_close3(max, [0.25, 64.9385, 0.5]);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(CHICKEN_RED, 0.78)));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(CHICKEN_BEAK, 0.78)));
}

#[test]
fn chicken_cold_adult_model_mesh_uses_vanilla_cold_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::chicken_variant(
        28,
        [0.0, 64.0, 0.0],
        0.0,
        ChickenModelVariant::Cold,
        false,
    )]);

    assert_eq!(mesh.opaque_faces, 60);
    assert_eq!(mesh.vertices.len(), 240);
    assert_eq!(mesh.indices.len(), 360);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.001, 0.5]);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(CHICKEN_WING, 0.78)));
}

#[test]
fn chicken_baby_model_mesh_uses_flat_vanilla_baby_parts() {
    let mesh = entity_model_mesh(&[EntityModelInstance::chicken(
        27,
        [0.0, 70.0, 0.0],
        0.0,
        true,
    )]);

    assert_eq!(mesh.opaque_faces, 48);
    assert_eq!(mesh.vertices.len(), 192);
    assert_eq!(mesh.indices.len(), 288);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [-0.1875, 70.001, -0.125]);
    assert_close3(max, [0.1875, 70.376, 0.1875]);
}

#[test]
fn chicken_texture_refs_match_vanilla_variant_assets() {
    let cases = [
        (
            ChickenModelVariant::Temperate,
            false,
            "chicken_temperate",
            EntityModelTextureRef {
                path: "textures/entity/chicken/chicken_temperate.png",
                size: [64, 32],
            },
        ),
        (
            ChickenModelVariant::Temperate,
            true,
            "chicken_temperate_baby",
            EntityModelTextureRef {
                path: "textures/entity/chicken/chicken_temperate_baby.png",
                size: [16, 16],
            },
        ),
        (
            ChickenModelVariant::Warm,
            false,
            "chicken_warm",
            EntityModelTextureRef {
                path: "textures/entity/chicken/chicken_warm.png",
                size: [64, 32],
            },
        ),
        (
            ChickenModelVariant::Warm,
            true,
            "chicken_warm_baby",
            EntityModelTextureRef {
                path: "textures/entity/chicken/chicken_warm_baby.png",
                size: [16, 16],
            },
        ),
        (
            ChickenModelVariant::Cold,
            false,
            "chicken_cold",
            EntityModelTextureRef {
                path: "textures/entity/chicken/chicken_cold.png",
                size: [64, 32],
            },
        ),
        (
            ChickenModelVariant::Cold,
            true,
            "chicken_cold_baby",
            EntityModelTextureRef {
                path: "textures/entity/chicken/chicken_cold_baby.png",
                size: [16, 16],
            },
        ),
    ];

    for (variant, baby, model_key, texture) in cases {
        let kind = EntityModelKind::Chicken { variant, baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

#[test]
fn chicken_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let adult_temperate = chicken_textured_layer_passes(ChickenModelVariant::Temperate, false);
    assert_eq!(adult_temperate.len(), 1);
    assert_eq!(adult_temperate[0].kind, EntityModelLayerKind::ChickenBase);
    assert_eq!(adult_temperate[0].model_layer, MODEL_LAYER_CHICKEN);
    assert_eq!(adult_temperate[0].texture, CHICKEN_TEMPERATE_TEXTURE_REF);
    assert_eq!(
        adult_temperate[0].parts,
        ADULT_CHICKEN_TEXTURED_PARTS.as_slice()
    );
    assert_eq!(adult_temperate[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult_temperate[0].collector_order, 0);
    assert_eq!(adult_temperate[0].submit_sequence, 0);

    let adult_warm = chicken_textured_layer_passes(ChickenModelVariant::Warm, false);
    assert_eq!(adult_warm[0].model_layer, MODEL_LAYER_CHICKEN);
    assert_eq!(adult_warm[0].texture, CHICKEN_WARM_TEXTURE_REF);
    assert_eq!(adult_warm[0].parts, ADULT_CHICKEN_TEXTURED_PARTS.as_slice());

    let adult_cold = chicken_textured_layer_passes(ChickenModelVariant::Cold, false);
    assert_eq!(adult_cold[0].model_layer, MODEL_LAYER_COLD_CHICKEN);
    assert_eq!(adult_cold[0].texture, CHICKEN_COLD_TEXTURE_REF);
    assert_eq!(adult_cold[0].parts, COLD_CHICKEN_TEXTURED_PARTS.as_slice());

    let baby_warm = chicken_textured_layer_passes(ChickenModelVariant::Warm, true);
    assert_eq!(baby_warm[0].model_layer, MODEL_LAYER_CHICKEN_BABY);
    assert_eq!(baby_warm[0].texture, CHICKEN_WARM_BABY_TEXTURE_REF);
    assert_eq!(baby_warm[0].parts, BABY_CHICKEN_TEXTURED_PARTS.as_slice());
}

#[test]
fn chicken_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_CHICKEN, "minecraft:chicken#main");
    assert_eq!(MODEL_LAYER_CHICKEN_BABY, "minecraft:chicken_baby#main");
    assert_eq!(MODEL_LAYER_COLD_CHICKEN, "minecraft:cold_chicken#main");
    assert_eq!(
        ADULT_CHICKEN_TEXTURED_HEAD[0],
        TexturedModelCubeDesc {
            min: [-2.0, -6.0, -2.0],
            size: [4.0, 6.0, 3.0],
            uv_size: [4.0, 6.0, 3.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ADULT_CHICKEN_TEXTURED_BEAK[0],
        TexturedModelCubeDesc {
            min: [-2.0, -4.0, -4.0],
            size: [4.0, 2.0, 2.0],
            uv_size: [4.0, 2.0, 2.0],
            tex: [14.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        ADULT_CHICKEN_TEXTURED_RED_THING[0],
        TexturedModelCubeDesc {
            min: [-1.0, -2.0, -3.0],
            size: [2.0, 2.0, 2.0],
            uv_size: [2.0, 2.0, 2.0],
            tex: [14.0, 4.0],
            mirror: false,
        }
    );
    assert_eq!(
        COLD_CHICKEN_TEXTURED_HEAD[1],
        TexturedModelCubeDesc {
            min: [-3.0, -7.0, -2.015],
            size: [6.0, 3.0, 4.0],
            uv_size: [6.0, 3.0, 4.0],
            tex: [44.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        COLD_CHICKEN_TEXTURED_BODY[1],
        TexturedModelCubeDesc {
            min: [0.0, 3.0, -1.0],
            size: [0.0, 3.0, 5.0],
            uv_size: [0.0, 3.0, 5.0],
            tex: [38.0, 9.0],
            mirror: false,
        }
    );
    assert_eq!(
        BABY_CHICKEN_TEXTURED_BODY[0],
        TexturedModelCubeDesc {
            min: [-2.0, -2.25, -0.75],
            size: [4.0, 4.0, 4.0],
            uv_size: [4.0, 4.0, 4.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
    assert_eq!(
        BABY_CHICKEN_TEXTURED_RIGHT_LEG[1],
        TexturedModelCubeDesc {
            min: [-0.5, 2.0, -1.0],
            size: [1.0, 0.0, 1.0],
            uv_size: [1.0, 0.0, 1.0],
            tex: [0.0, 0.0],
            mirror: false,
        }
    );
}

#[test]
fn entity_texture_atlas_stitches_official_chicken_png_slots() {
    let images = chicken_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect::<Vec<_>>();

    let (layout, rgba) = build_entity_model_texture_atlas(&images).unwrap();

    assert_eq!(layout.width, 64);
    assert_eq!(layout.height, 144);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/chicken/chicken_temperate.png",
            "textures/entity/chicken/chicken_temperate_baby.png",
            "textures/entity/chicken/chicken_warm.png",
            "textures/entity/chicken/chicken_warm_baby.png",
            "textures/entity/chicken/chicken_cold.png",
            "textures/entity/chicken/chicken_cold_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 32.0 / 144.0]);
    assert_close2(layout.entries[1].uv.min, [0.0, 32.0 / 144.0]);
    assert_close2(layout.entries[1].uv.max, [0.25, 48.0 / 144.0]);
    assert_close2(layout.entries[4].uv.min, [0.0, 96.0 / 144.0]);
    assert_close2(layout.entries[4].uv.max, [1.0, 128.0 / 144.0]);
    let warm_first_pixel = rgba_offset(layout.width, 48, 0, "test").unwrap();
    assert_eq!(&rgba[warm_first_pixel..warm_first_pixel + 4], &[2; 4]);
    let cold_baby_first_pixel = rgba_offset(layout.width, 128, 0, "test").unwrap();
    assert_eq!(
        &rgba[cold_baby_first_pixel..cold_baby_first_pixel + 4],
        &[5; 4]
    );
}

#[test]
fn chicken_textured_mesh_uses_vanilla_uvs_tints_and_variant_textures() {
    let (atlas, _) = build_entity_model_texture_atlas(&chicken_texture_images()).unwrap();
    let mesh = entity_model_textured_mesh(
        &[
            EntityModelInstance::chicken_variant(
                401,
                [0.0, 64.0, 0.0],
                0.0,
                ChickenModelVariant::Temperate,
                false,
            ),
            EntityModelInstance::chicken_variant(
                402,
                [1.0, 64.0, 0.0],
                0.0,
                ChickenModelVariant::Cold,
                false,
            ),
            EntityModelInstance::chicken_variant(
                403,
                [2.0, 64.0, 0.0],
                0.0,
                ChickenModelVariant::Warm,
                true,
            ),
        ],
        &atlas,
    );

    assert_eq!(mesh.cutout_faces, 156);
    assert_eq!(mesh.vertices.len(), 624);
    assert_eq!(mesh.indices.len(), 936);
    assert_close2(mesh.vertices[0].uv, [7.0 / 64.0, 0.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[192].uv, [7.0 / 64.0, 96.0 / 144.0]);
    assert_eq!(mesh.vertices[192].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[432].uv, [8.0 / 64.0, 80.0 / 144.0]);
    assert_eq!(mesh.vertices[432].tint, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn chicken_swings_its_legs_when_walking() {
    // Vanilla `ChickenModel.setupAnim` swings the two legs with the `HumanoidModel` phase
    // `cos(pos * 0.6662 [+ π]) * 1.4 * speed`. A standing chicken is inert; a walking one
    // lifts its feet off the ground (the adult, cold, and headless-baby layers all show
    // it). The chicken has no head look; the wing flap is deferred. Colored path.
    for base in [
        EntityModelInstance::chicken(26, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::chicken_variant(
            28,
            [0.0, 64.0, 0.0],
            0.0,
            ChickenModelVariant::Cold,
            false,
        ),
        EntityModelInstance::chicken(27, [0.0, 64.0, 0.0], 0.0, true),
    ] {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(
            rest.vertices, still.vertices,
            "{:?} rest is inert",
            base.kind
        );

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(
            rest.vertices, walking.vertices,
            "{:?} walking differs",
            base.kind
        );

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{:?} feet should lift off the ground",
            base.kind
        );
    }
}

#[test]
fn chicken_textured_mesh_swings_its_legs_when_walking() {
    // The real (texture-backed) chicken render path swings the same legs. A standing
    // chicken is byte-identical however far the swing has advanced; a walking one lifts
    // its feet while keeping the vertex count.
    let (atlas, _) = build_entity_model_texture_atlas(&chicken_texture_images()).unwrap();
    for base in [
        EntityModelInstance::chicken(426, [0.0, 64.0, 0.0], 0.0, false),
        EntityModelInstance::chicken_variant(
            428,
            [0.0, 64.0, 0.0],
            0.0,
            ChickenModelVariant::Cold,
            false,
        ),
        EntityModelInstance::chicken(427, [0.0, 64.0, 0.0], 0.0, true),
    ] {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
        let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

        assert_eq!(resting.vertices, still.vertices, "{:?} is inert", base.kind);
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{:?} leg swing keeps the vertex count",
            base.kind
        );
        let (rest_min, rest_max) = textured_mesh_extents(&resting);
        let (walk_min, walk_max) = textured_mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{:?} feet should lift off the ground (textured)",
            base.kind
        );
    }
}

#[test]
fn chicken_leg_swing_matches_vanilla_humanoid_phase() {
    // Vanilla ChickenModel.setupAnim: rightLeg.xRot = cos(pos*0.6662)*1.4*speed,
    // leftLeg.xRot = cos(pos*0.6662+π)*1.4*speed — the HumanoidModel phase. The adult and
    // cold layers list the legs at [2, 3]; the headless baby layer at [1, 2]. The right
    // leg (offset x < 0) is in phase, the left leg (x > 0) out of phase. Only xRot moves.
    let pos = 1.3_f32;
    let speed = 0.7_f32;
    let phase = pos * 0.6662;
    for (parts, baby) in [
        (ADULT_CHICKEN_PARTS.as_slice(), false),
        (COLD_CHICKEN_PARTS.as_slice(), false),
        (BABY_CHICKEN_PARTS.as_slice(), true),
    ] {
        for index in chicken_leg_part_indices(baby) {
            let base = parts[index].pose;
            let posed = humanoid_leg_swing_pose(base, pos, speed);
            let expected = if base.offset[0] < 0.0 {
                phase.cos() * 1.4 * speed
            } else {
                (phase + std::f32::consts::PI).cos() * 1.4 * speed
            };
            assert!(
                (posed.rotation[0] - expected).abs() < 1e-6,
                "leg {index} xRot"
            );
            assert_eq!(posed.offset, base.offset, "leg {index} offset");
            assert_eq!(posed.rotation[1], base.rotation[1], "leg {index} yRot");
            assert_eq!(posed.rotation[2], base.rotation[2], "leg {index} zRot");
        }
    }
}

fn chicken_texture_images() -> Vec<EntityModelTextureImage> {
    chicken_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
