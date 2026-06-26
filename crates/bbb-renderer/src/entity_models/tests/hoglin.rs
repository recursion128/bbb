use super::*;

use crate::entity_models::model::{EntityModel, ModelCube};

#[test]
fn hoglin_adult_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla `HoglinModel.createBodyLayer`: the body (parenting the mane) and the head (parenting
    // the two ears + two horns) plus four legs. Each unified cube carries the colored tint
    // (`HOGLIN_RED`) and the textured `uv_size`/`texOffs`/`mirror` in one struct; the mane keeps its
    // inflated colored geometry against the base textured `uv_size`.
    assert_eq!(
        ADULT_HOGLIN_BODY[0],
        ModelCube::new(
            [-8.0, -7.0, -13.0],
            [16.0, 14.0, 26.0],
            HOGLIN_RED,
            [16.0, 14.0, 26.0],
            [1.0, 1.0],
            false,
        )
    );
    assert_eq!(
        ADULT_HOGLIN_MANE[0],
        ModelCube::new(
            [-0.001, -0.001, -9.001],
            [0.002, 10.002, 19.002],
            HOGLIN_RED,
            [0.0, 10.0, 19.0],
            [90.0, 33.0],
            false,
        )
    );
    assert_eq!(ADULT_HOGLIN_HEAD[0].tex, [61.0, 1.0]);
    assert_eq!(ADULT_HOGLIN_RIGHT_EAR[0].tex, [1.0, 1.0]);
    assert_eq!(ADULT_HOGLIN_LEFT_EAR[0].tex, [1.0, 6.0]);
    assert_eq!(ADULT_HOGLIN_RIGHT_HORN[0].tex, [10.0, 13.0]);
    assert_eq!(ADULT_HOGLIN_LEFT_HORN[0].tex, [1.0, 13.0]);
    assert_eq!(ADULT_HOGLIN_RIGHT_FRONT_LEG[0].tex, [66.0, 42.0]);
    assert_eq!(ADULT_HOGLIN_LEFT_FRONT_LEG[0].tex, [41.0, 42.0]);
    assert_eq!(ADULT_HOGLIN_RIGHT_HIND_LEG[0].tex, [21.0, 45.0]);
    assert_eq!(ADULT_HOGLIN_LEFT_HIND_LEG[0].tex, [0.0, 45.0]);
    // Every adult cube tints with `HOGLIN_RED`.
    for cube in [
        ADULT_HOGLIN_BODY[0],
        ADULT_HOGLIN_HEAD[0],
        ADULT_HOGLIN_RIGHT_FRONT_LEG[0],
        ADULT_HOGLIN_LEFT_HIND_LEG[0],
    ] {
        assert_eq!(cube.color, HOGLIN_RED);
    }
}

#[test]
fn hoglin_baby_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla `BabyHoglinModel.createBodyLayer`: the head (parenting only the two ears) plus the body
    // and four legs. The body cubes are inflated (the colored geometry keeps the inflated box against
    // the base textured `uv_size`); the left ear is mirrored.
    assert_eq!(
        BABY_HOGLIN_BODY[0],
        ModelCube::new(
            [-4.02, -14.02, -7.02],
            [8.04, 8.04, 14.04],
            HOGLIN_RED,
            [8.0, 8.0, 14.0],
            [0.0, 16.0],
            false,
        )
    );
    assert_eq!(BABY_HOGLIN_BODY[1].uv_size, [0.0, 6.0, 11.0]);
    assert_eq!(BABY_HOGLIN_BODY[1].tex, [24.0, 39.0]);
    assert_eq!(BABY_HOGLIN_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(BABY_HOGLIN_HEAD[1].tex, [44.0, 29.0]);
    assert_eq!(BABY_HOGLIN_HEAD[2].tex, [52.0, 29.0]);
    assert_eq!(BABY_HOGLIN_RIGHT_EAR[0].tex, [32.0, 5.0]);
    assert_eq!(BABY_HOGLIN_LEFT_EAR[0].tex, [32.0, 0.0]);
    assert!(BABY_HOGLIN_LEFT_EAR[0].mirror);
    assert_eq!(BABY_HOGLIN_RIGHT_HIND_LEG[0].tex, [0.0, 47.0]);
    assert_eq!(BABY_HOGLIN_LEFT_HIND_LEG[0].tex, [12.0, 47.0]);
    assert_eq!(BABY_HOGLIN_RIGHT_FRONT_LEG[0].tex, [0.0, 38.0]);
    assert_eq!(BABY_HOGLIN_LEFT_FRONT_LEG[0].tex, [12.0, 38.0]);
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
        ),
        (
            HoglinModelFamily::Hoglin,
            true,
            MODEL_LAYER_HOGLIN_BABY,
            HOGLIN_BABY_TEXTURE_REF,
        ),
        (
            HoglinModelFamily::Zoglin,
            false,
            MODEL_LAYER_ZOGLIN,
            ZOGLIN_TEXTURE_REF,
        ),
        (
            HoglinModelFamily::Zoglin,
            true,
            MODEL_LAYER_ZOGLIN_BABY,
            ZOGLIN_BABY_TEXTURE_REF,
        ),
    ];

    for (family, baby, model_layer, texture) in cases {
        let passes = hoglin_textured_layer_passes(family, baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].kind, EntityModelLayerKind::HoglinBase);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].model_layer, model_layer);
        assert_eq!(passes[0].texture, texture);
        // The vestigial `parts` slice is nulled; emit builds `HoglinModel::new(baby)` and renders it.
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(
            (passes[0].collector_order, passes[0].submit_sequence),
            (0, 0)
        );
    }
}

#[test]
fn hoglin_model_layers_match_vanilla() {
    assert_eq!(MODEL_LAYER_HOGLIN, "minecraft:hoglin#main");
    assert_eq!(MODEL_LAYER_HOGLIN_BABY, "minecraft:hoglin_baby#main");
    assert_eq!(MODEL_LAYER_ZOGLIN, "minecraft:zoglin#main");
    assert_eq!(MODEL_LAYER_ZOGLIN_BABY, "minecraft:zoglin_baby#main");
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

#[test]
fn hoglin_textured_meshes_apply_yaw_only_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&hoglin_texture_images()).unwrap();
    // Vanilla `HoglinModel.setupAnim` turns the head in yaw only, keeping the
    // headbutt-rest pitch baked into the base pose, so a pitch-only look leaves the
    // mesh unchanged while a yaw look turns the head.
    for (id, baby) in [(228, false), (229, true)] {
        let base =
            EntityModelInstance::hoglin(id, [0.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, baby);
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let yawed = entity_model_textured_mesh(&[base.with_head_look(50.0, 0.0)], &atlas);
        let pitched = entity_model_textured_mesh(&[base.with_head_look(0.0, -20.0)], &atlas);
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(
            resting.vertices, yawed.vertices,
            "baby={baby} yaw turns head"
        );
        assert_eq!(
            resting.vertices, pitched.vertices,
            "baby={baby} pitch ignored"
        );
    }
}

#[test]
fn hoglin_leg_swing_pose_matches_vanilla_formula() {
    // Vanilla HoglinModel.setupAnim: rightFrontLeg.xRot = cos(pos) * 1.2 * speed,
    // leftFrontLeg.xRot = cos(pos + π) * 1.2 * speed, rightHindLeg = leftFrontLeg,
    // leftHindLeg = rightFrontLeg. The amplitude is 1.2 (not the QuadrupedModel 1.4)
    // and there is NO 0.6662 frequency factor. The right-front leg sits at offset
    // x = -4, z = -8.5 (x*z > 0 -> in phase) and the left-front at x = 4, z = -8.5
    // (x*z < 0 -> out of phase).
    let right_front_pose = PartPose {
        offset: [-4.0, 10.0, -8.5],
        rotation: [0.0, 0.0, 0.0],
    };
    let left_front_pose = PartPose {
        offset: [4.0, 10.0, -8.5],
        rotation: [0.0, 0.0, 0.0],
    };
    let right_hind_pose = PartPose {
        offset: [-5.0, 13.0, 10.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let left_hind_pose = PartPose {
        offset: [5.0, 13.0, 10.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let right_front = hoglin_leg_swing_pose(right_front_pose, 0.0, 1.0);
    let left_front = hoglin_leg_swing_pose(left_front_pose, 0.0, 1.0);
    assert!(
        (right_front.rotation[0] - 1.2).abs() < 1e-6,
        "right front in phase at amplitude 1.2: {}",
        right_front.rotation[0]
    );
    assert!(
        (left_front.rotation[0] + 1.2).abs() < 1e-6,
        "left front out of phase at amplitude 1.2: {}",
        left_front.rotation[0]
    );
    // The diagonal pair: right hind (x*z < 0) matches left front, and left hind (x*z > 0)
    // matches right front.
    let right_hind = hoglin_leg_swing_pose(right_hind_pose, 0.0, 1.0);
    let left_hind = hoglin_leg_swing_pose(left_hind_pose, 0.0, 1.0);
    assert!((right_hind.rotation[0] - left_front.rotation[0]).abs() < 1e-6);
    assert!((left_hind.rotation[0] - right_front.rotation[0]).abs() < 1e-6);

    // A general (pos, speed) reproduces cos(pos [+ π]) * 1.2 * speed, with no 0.6662.
    let right_front = hoglin_leg_swing_pose(right_front_pose, 1.5, 0.5);
    let left_front = hoglin_leg_swing_pose(left_front_pose, 1.5, 0.5);
    assert!((right_front.rotation[0] - 1.5_f32.cos() * 1.2 * 0.5).abs() < 1e-6);
    assert!(
        (left_front.rotation[0] - (1.5_f32 + std::f32::consts::PI).cos() * 1.2 * 0.5).abs() < 1e-6
    );
}

#[test]
fn hoglin_family_swings_its_legs_when_walking() {
    // `HoglinModel` (zoglin shares it) swings the four legs with its own
    // `cos(pos [+ π]) * 1.2 * speed` formula. A standing hoglin is inert; a walking
    // adult lifts its feet and splays its legs along Z; the baby's short legs swing
    // too but the motion stays inside its bounding box. The adult ear sway is covered by
    // `adult_hoglin_sways_its_ears_when_walking`; the headbutt holds its rest down-tilt here (no attack
    // timer), so `rest == still`. Colored path.
    for (name, base, adult_size) in [
        (
            "hoglin",
            EntityModelInstance::hoglin(
                240,
                [0.0, 64.0, 0.0],
                0.0,
                HoglinModelFamily::Hoglin,
                false,
            ),
            true,
        ),
        (
            "zoglin",
            EntityModelInstance::hoglin(
                241,
                [0.0, 64.0, 0.0],
                0.0,
                HoglinModelFamily::Zoglin,
                false,
            ),
            true,
        ),
        (
            "hoglin_baby",
            EntityModelInstance::hoglin(
                242,
                [0.0, 64.0, 0.0],
                0.0,
                HoglinModelFamily::Hoglin,
                true,
            ),
            false,
        ),
    ] {
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "{name}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(rest.vertices, walking.vertices, "{name}: walking differs");

        if adult_size {
            let (rest_min, rest_max) = mesh_extents(&rest);
            let (walk_min, walk_max) = mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking hoglin's feet should lift off the ground"
            );
            assert!(
                (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
                "{name}: a walking hoglin's legs should splay along Z"
            );
        }
    }
}

#[test]
fn hoglin_textured_mesh_swings_legs_when_walking() {
    // The real hoglin render path (texture-backed) swings the same legs. A standing
    // hoglin is byte-identical however far the swing has advanced; a walking adult
    // lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&hoglin_texture_images()).unwrap();
    for (name, base, adult_size) in [
        (
            "hoglin",
            EntityModelInstance::hoglin(
                243,
                [0.0, 64.0, 0.0],
                0.0,
                HoglinModelFamily::Hoglin,
                false,
            ),
            true,
        ),
        (
            "hoglin_baby",
            EntityModelInstance::hoglin(
                244,
                [0.0, 64.0, 0.0],
                0.0,
                HoglinModelFamily::Hoglin,
                true,
            ),
            false,
        ),
    ] {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
        let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

        assert_eq!(
            resting.vertices, still.vertices,
            "{name}: a standing hoglin is inert"
        );
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{name}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.vertices, walking.vertices,
            "{name}: a walking hoglin differs"
        );

        if adult_size {
            let (rest_min, rest_max) = textured_mesh_extents(&resting);
            let (walk_min, walk_max) = textured_mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking hoglin's feet should lift off the ground"
            );
        }
    }
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

#[test]
fn hoglin_ear_sway_pose_matches_vanilla_formula() {
    // Vanilla HoglinModel.setupAnim: rightEar.zRot = -2π/9 - speed * sin(pos),
    // leftEar.zRot = +2π/9 + speed * sin(pos). The adult ear poses rest at ∓2π/9, so the
    // sway adds ∓speed * sin(pos) onto each; only zRot changes.
    let right = PartPose {
        offset: [-6.0, -2.0, -3.0],
        rotation: [0.0, 0.0, -HOGLIN_EAR_Z_ROT],
    };
    let left = PartPose {
        offset: [6.0, -2.0, -3.0],
        rotation: [0.0, 0.0, HOGLIN_EAR_Z_ROT],
    };
    let ear_z = std::f32::consts::PI * 2.0 / 9.0;
    assert!(
        (right.rotation[2] + ear_z).abs() < 1e-6,
        "right ear rests at -2π/9"
    );
    assert!(
        (left.rotation[2] - ear_z).abs() < 1e-6,
        "left ear rests at +2π/9"
    );

    // At pos = π/2, speed = 1: sin = 1, so the sway magnitude is 1.0.
    let pos = std::f32::consts::FRAC_PI_2;
    let swayed_right = hoglin_ear_sway_pose(right, false, pos, 1.0);
    let swayed_left = hoglin_ear_sway_pose(left, true, pos, 1.0);
    assert!(
        (swayed_right.rotation[2] - (-ear_z - 1.0)).abs() < 1e-6,
        "right ear: {}",
        swayed_right.rotation[2]
    );
    assert!(
        (swayed_left.rotation[2] - (ear_z + 1.0)).abs() < 1e-6,
        "left ear: {}",
        swayed_left.rotation[2]
    );
    // Only zRot changes; the offset and other axes are preserved.
    assert_eq!(swayed_right.offset, right.offset);
    assert_eq!(swayed_right.rotation[0], right.rotation[0]);
    assert_eq!(swayed_right.rotation[1], right.rotation[1]);

    // A general (pos, speed): sway = speed * sin(pos).
    let pos = 1.3_f32;
    let speed = 0.6_f32;
    let sway = speed * pos.sin();
    let swayed_right = hoglin_ear_sway_pose(right, false, pos, speed);
    let swayed_left = hoglin_ear_sway_pose(left, true, pos, speed);
    assert!((swayed_right.rotation[2] - (-ear_z - sway)).abs() < 1e-6);
    assert!((swayed_left.rotation[2] - (ear_z + sway)).abs() < 1e-6);

    // sin(pos) == 0 (pos = 0) leaves the adult ears at their ±2π/9 rest splay.
    let swayed_right = hoglin_ear_sway_pose(right, false, 0.0, 1.0);
    assert_eq!(swayed_right.rotation[2], right.rotation[2]);

    // The baby ear poses rest at a wider angle (±BABY_HOGLIN_EAR_Z_ROT), but vanilla writes
    // the absolute from the literal 2π/9, so hoglin_ear_sway_pose ignores the base angle and
    // overrides the baby ears to ±2π/9 (± the sway).
    let baby_right = PartPose {
        offset: [-5.0, -1.0, -1.5],
        rotation: [0.0, 0.0, -BABY_HOGLIN_EAR_Z_ROT],
    };
    let baby_left = PartPose {
        offset: [5.0, -1.0, -1.5],
        rotation: [0.0, 0.0, BABY_HOGLIN_EAR_Z_ROT],
    };
    assert!(
        (baby_right.rotation[2] + BABY_HOGLIN_EAR_Z_ROT).abs() < 1e-6
            && (BABY_HOGLIN_EAR_Z_ROT - ear_z).abs() > 1e-3,
        "the baby layer ears rest at a wider angle than 2π/9"
    );
    let baby_rest_right = hoglin_ear_sway_pose(baby_right, false, 0.0, 1.0);
    let baby_rest_left = hoglin_ear_sway_pose(baby_left, true, 0.0, 1.0);
    assert!(
        (baby_rest_right.rotation[2] + ear_z).abs() < 1e-6,
        "baby right ear overridden to -2π/9 at rest: {}",
        baby_rest_right.rotation[2]
    );
    assert!(
        (baby_rest_left.rotation[2] - ear_z).abs() < 1e-6,
        "baby left ear overridden to +2π/9 at rest: {}",
        baby_rest_left.rotation[2]
    );
}

#[test]
fn adult_hoglin_sways_its_ears_when_walking() {
    // The adult hoglin/zoglin ears (children of the head) sway side to side as the gait
    // advances. In the body layer the head subtree emits head cubes + ears + horns, so the
    // ears occupy 24-vertex blocks [3, 5) = vertices [72, 120); the four legs occupy blocks
    // [7, 11) = vertices [168, 264). The ear sway is `speed * sin(pos)`, so it shows only
    // when sin(pos) != 0 — a walking hoglin at a sin-zero phase moves only its (cos-driven)
    // legs. Covers hoglin and zoglin in the colored path.
    for (name, family) in [
        ("hoglin", HoglinModelFamily::Hoglin),
        ("zoglin", HoglinModelFamily::Zoglin),
    ] {
        let base = EntityModelInstance::hoglin(250, [0.0, 64.0, 0.0], 0.0, family, false);
        let rest = entity_model_mesh(&[base]);

        // A sin-nonzero phase sways the ears and (cos-nonzero) swings the legs.
        let walking = entity_model_mesh(&[base.with_walk_animation(1.5, 1.0)]);
        assert_ne!(
            rest.vertices[72..120],
            walking.vertices[72..120],
            "{name}: ears sway when walking"
        );
        assert_ne!(
            rest.vertices[168..264],
            walking.vertices[168..264],
            "{name}: legs swing when walking"
        );
        // Head cubes, horns, body and mane stay put (only the ear children and legs move).
        assert_eq!(
            rest.vertices[0..72],
            walking.vertices[0..72],
            "{name}: body/mane/head cubes stay put"
        );
        assert_eq!(
            rest.vertices[120..168],
            walking.vertices[120..168],
            "{name}: horns stay put"
        );

        // At a sin-zero phase (pos = 0) the ears stay at rest; only the legs swing.
        let legs_only = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_eq!(
            rest.vertices[72..120],
            legs_only.vertices[72..120],
            "{name}: ears stay put when sin(pos) == 0"
        );
        assert_ne!(
            rest.vertices[168..264],
            legs_only.vertices[168..264],
            "{name}: legs still swing when sin(pos) == 0"
        );
    }
}

#[test]
fn hoglin_textured_mesh_sways_its_ears_when_walking() {
    // The texture-backed hoglin runs the same adult ear sway, emitting the head subtree in
    // the same order, so the ears occupy textured vertices [72, 120). A standing hoglin is
    // byte-identical; a walking one (sin-nonzero phase) sways its ears.
    let (atlas, _) = build_entity_model_texture_atlas(&hoglin_texture_images()).unwrap();
    let base =
        EntityModelInstance::hoglin(251, [0.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, false);
    let resting = entity_model_textured_mesh(&[base], &atlas);
    let walking = entity_model_textured_mesh(&[base.with_walk_animation(1.5, 1.0)], &atlas);
    assert_eq!(
        resting.vertices.len(),
        walking.vertices.len(),
        "the ear sway keeps the vertex count"
    );
    assert_ne!(
        resting.vertices[72..120],
        walking.vertices[72..120],
        "the textured ears sway when walking"
    );
    assert_eq!(
        resting.vertices[0..72],
        walking.vertices[0..72],
        "body/mane/head cubes stay put"
    );
}

#[test]
fn baby_hoglin_sways_its_ears_when_walking() {
    // The baby hoglin shares `HoglinModel.setupAnim`, so its ears sway too — and vanilla
    // overrides the baby layer's wider ear rest angle to ±2π/9, so even a standing baby's
    // ears sit at ±2π/9 (the renderer always re-poses them). The baby head subtree emits in
    // the same order, so the ears occupy 24-vertex blocks [3, 5) = vertices [72, 120) and
    // the legs blocks [7, 11) = vertices [168, 264).
    let base =
        EntityModelInstance::hoglin(252, [0.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, true);
    let rest = entity_model_mesh(&[base]);

    // A sin-nonzero phase sways the ears and swings the legs.
    let walking = entity_model_mesh(&[base.with_walk_animation(1.5, 1.0)]);
    assert_ne!(
        rest.vertices[72..120],
        walking.vertices[72..120],
        "baby ears sway when walking"
    );
    assert_ne!(
        rest.vertices[168..264],
        walking.vertices[168..264],
        "baby legs swing when walking"
    );

    // At a sin-zero phase the ears stay at their ±2π/9 rest; only the legs swing.
    let legs_only = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_eq!(
        rest.vertices[72..120],
        legs_only.vertices[72..120],
        "baby ears stay put when sin(pos) == 0"
    );
    assert_ne!(
        rest.vertices[168..264],
        legs_only.vertices[168..264],
        "baby legs still swing when sin(pos) == 0"
    );
}

#[test]
fn hoglin_headbutt_raises_the_head_from_its_rest_tilt() {
    use std::f32::consts::PI;

    // Vanilla `HoglinModel.animateHeadbutt`: head.xRot = lerp(1 - |10 - 2·tick|/10, 0.87266463, -π/9).
    // At rest (tick 0) the factor is 0, so the head holds its baked down-tilt; at the attack midpoint
    // (tick 5) the factor is 1, so the head rises to -π/9. The baby additionally lifts head.y by
    // factor·2.5.
    let rest_tilt = 0.87266463_f32;
    let base =
        EntityModelInstance::hoglin(243, [0.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, false);

    // Rest: head holds the down-tilt.
    let mut resting = HoglinModel::new(false);
    resting.prepare(&base);
    let resting_pitch = resting.root_mut().child_mut("head").pose.rotation[0];
    assert!(
        (resting_pitch - rest_tilt).abs() < 1.0e-6,
        "rest tilt: {resting_pitch}"
    );

    // Peak ram (tick 5 → factor 1): head rises to -π/9.
    let mut ramming = HoglinModel::new(false);
    ramming.prepare(&base.with_hoglin_attack_animation_tick(5));
    let ram_pitch = ramming.root_mut().child_mut("head").pose.rotation[0];
    assert!(
        (ram_pitch - (-PI / 9.0)).abs() < 1.0e-6,
        "peak ram pitch: {ram_pitch}"
    );
    assert!(
        ram_pitch < resting_pitch,
        "the ram raises the head from its rest tilt"
    );

    // Mid-ramp (tick 8 → factor = 1 - |10-16|/10 = 0.4): lerp between the two.
    let mut mid = HoglinModel::new(false);
    mid.prepare(&base.with_hoglin_attack_animation_tick(8));
    let mid_pitch = mid.root_mut().child_mut("head").pose.rotation[0];
    let factor = 1.0 - (10i32 - 2 * 8).abs() as f32 / 10.0;
    assert!(
        (mid_pitch - (rest_tilt + factor * (-PI / 9.0 - rest_tilt))).abs() < 1.0e-6,
        "mid-ramp pitch: {mid_pitch}"
    );

    // The baby lifts its head (head.y) at the ram peak; the adult does not.
    let baby_base =
        EntityModelInstance::hoglin(244, [0.0, 64.0, 0.0], 0.0, HoglinModelFamily::Hoglin, true);
    let mut baby_rest = HoglinModel::new(true);
    baby_rest.prepare(&baby_base);
    let baby_rest_y = baby_rest.root_mut().child_mut("head").pose.offset[1];
    let mut baby_ram = HoglinModel::new(true);
    baby_ram.prepare(&baby_base.with_hoglin_attack_animation_tick(5));
    let baby_ram_y = baby_ram.root_mut().child_mut("head").pose.offset[1];
    assert!(
        (baby_ram_y - baby_rest_y - 2.5).abs() < 1.0e-6,
        "the baby lifts its head by 2.5"
    );

    // The headbutt visibly re-poses the rendered mesh.
    assert_ne!(
        entity_model_mesh(&[base]).vertices,
        entity_model_mesh(&[base.with_hoglin_attack_animation_tick(5)]).vertices,
        "the headbutt raises the head in the mesh"
    );
}
