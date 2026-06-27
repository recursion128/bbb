use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn chicken_model_cubes_and_poses_match_vanilla_26_1_layers() {
    // Adult layout: head (offset (0, 15, -4)) carrying the beak + red_thing, body (FRAC_PI_2 pitch),
    // then the right/left legs and wings.
    assert_eq!(CHICKEN_HEAD_POSE.offset, [0.0, 15.0, -4.0]);
    assert_eq!(CHICKEN_HEAD_POSE.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(ADULT_CHICKEN_HEAD[0].min, [-2.0, -6.0, -2.0]);
    assert_eq!(ADULT_CHICKEN_HEAD[0].size, [4.0, 6.0, 3.0]);
    assert_eq!(ADULT_CHICKEN_BEAK[0].min, [-2.0, -4.0, -4.0]);
    assert_eq!(ADULT_CHICKEN_BEAK[0].size, [4.0, 2.0, 2.0]);
    assert_eq!(ADULT_CHICKEN_RED_THING[0].min, [-1.0, -2.0, -3.0]);
    assert_eq!(ADULT_CHICKEN_RED_THING[0].size, [2.0, 2.0, 2.0]);

    assert_eq!(CHICKEN_BODY_POSE.offset, [0.0, 16.0, 0.0]);
    assert_eq!(
        CHICKEN_BODY_POSE.rotation,
        [std::f32::consts::FRAC_PI_2, 0.0, 0.0]
    );
    assert_eq!(ADULT_CHICKEN_BODY[0].min, [-3.0, -4.0, -3.0]);
    assert_eq!(ADULT_CHICKEN_BODY[0].size, [6.0, 8.0, 6.0]);

    assert_eq!(CHICKEN_RIGHT_LEG_POSE.offset, [-2.0, 19.0, 1.0]);
    assert_eq!(CHICKEN_LEFT_LEG_POSE.offset, [1.0, 19.0, 1.0]);
    assert_eq!(ADULT_CHICKEN_LEG[0].size, [3.0, 5.0, 3.0]);

    // Cold variant: the head and body carry an extra fluff cube tinted CHICKEN_WING.
    assert_eq!(COLD_CHICKEN_HEAD.len(), 2);
    assert_eq!(
        COLD_CHICKEN_HEAD[1],
        ModelCube::new(
            [-3.0, -7.0, -2.015],
            [6.0, 3.0, 4.0],
            CHICKEN_WING,
            [6.0, 3.0, 4.0],
            [44.0, 0.0],
            false,
        )
    );
    assert_eq!(
        COLD_CHICKEN_BODY[1],
        ModelCube::new(
            [0.0, 3.0, -1.0],
            [0.0, 3.0, 5.0],
            CHICKEN_WING,
            [0.0, 3.0, 5.0],
            [38.0, 9.0],
            false,
        )
    );

    // Baby layout: a squat body (beak baked in), then the legs and wings.
    assert_eq!(BABY_CHICKEN_BODY_POSE.offset, [0.0, 20.25, -1.25]);
    assert_eq!(BABY_CHICKEN_BODY[0].size, [4.0, 4.0, 4.0]);
    assert_eq!(BABY_CHICKEN_LEFT_LEG_POSE.offset, [1.0, 22.0, 0.5]);
    assert_eq!(BABY_CHICKEN_LEFT_WING_POSE.offset, [-2.0, 20.0, 0.0]);
    assert_eq!(BABY_CHICKEN_LEFT_WING[0].size, [1.0, 0.0, 2.0]);
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
    assert_eq!(
        adult_temperate[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(
        adult_temperate[0].render_type.vanilla_name(),
        "entityCutout"
    );
    assert_eq!(adult_temperate[0].model_layer, MODEL_LAYER_CHICKEN);
    assert_eq!(adult_temperate[0].texture, CHICKEN_TEMPERATE_TEXTURE_REF);
    assert_eq!(adult_temperate[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        (adult_temperate[0].order, adult_temperate[0].submit_sequence),
        (0, 0)
    );

    let adult_warm = chicken_textured_layer_passes(ChickenModelVariant::Warm, false);
    assert_eq!(adult_warm[0].model_layer, MODEL_LAYER_CHICKEN);
    assert_eq!(adult_warm[0].texture, CHICKEN_WARM_TEXTURE_REF);

    let adult_cold = chicken_textured_layer_passes(ChickenModelVariant::Cold, false);
    assert_eq!(adult_cold[0].model_layer, MODEL_LAYER_COLD_CHICKEN);
    assert_eq!(adult_cold[0].texture, CHICKEN_COLD_TEXTURE_REF);

    let baby_warm = chicken_textured_layer_passes(ChickenModelVariant::Warm, true);
    assert_eq!(baby_warm[0].model_layer, MODEL_LAYER_CHICKEN_BABY);
    assert_eq!(baby_warm[0].texture, CHICKEN_WARM_BABY_TEXTURE_REF);
}

#[test]
fn chicken_cubes_match_vanilla_model_layer_uv_sources() {
    // Each unified cube carries the colored tint and the textured UV (`texOffs` / `uv_size`); no
    // CubeDeformation, so `uv_size == size` and no cube mirrors.
    assert_eq!(MODEL_LAYER_CHICKEN, "minecraft:chicken#main");
    assert_eq!(MODEL_LAYER_CHICKEN_BABY, "minecraft:chicken_baby#main");
    assert_eq!(MODEL_LAYER_COLD_CHICKEN, "minecraft:cold_chicken#main");
    assert_eq!(
        ADULT_CHICKEN_HEAD[0],
        ModelCube::new(
            [-2.0, -6.0, -2.0],
            [4.0, 6.0, 3.0],
            CHICKEN_WHITE,
            [4.0, 6.0, 3.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(ADULT_CHICKEN_BEAK[0].tex, [14.0, 0.0]);
    assert_eq!(ADULT_CHICKEN_RED_THING[0].tex, [14.0, 4.0]);
    assert_eq!(ADULT_CHICKEN_BODY[0].tex, [0.0, 9.0]);
    assert_eq!(COLD_CHICKEN_HEAD[1].tex, [44.0, 0.0]);
    assert_eq!(COLD_CHICKEN_BODY[1].tex, [38.0, 9.0]);
    assert_eq!(ADULT_CHICKEN_LEG[0].tex, [26.0, 0.0]);
    assert_eq!(ADULT_CHICKEN_RIGHT_WING[0].tex, [24.0, 13.0]);
    assert_eq!(ADULT_CHICKEN_LEFT_WING[0].tex, [24.0, 13.0]);
    assert_eq!(BABY_CHICKEN_BODY[0].tex, [0.0, 0.0]);
    assert_eq!(BABY_CHICKEN_BODY[1].tex, [10.0, 8.0]);
    assert_eq!(BABY_CHICKEN_RIGHT_LEG[1].tex, [0.0, 0.0]);
    assert_eq!(BABY_CHICKEN_LEFT_LEG[0].tex, [2.0, 2.0]);
    assert_eq!(BABY_CHICKEN_RIGHT_WING[0].tex, [6.0, 8.0]);
    assert_eq!(BABY_CHICKEN_LEFT_WING[0].tex, [4.0, 8.0]);
    // uv_size mirrors the cube size and no cube mirrors, for every chicken cube.
    for cube in ADULT_CHICKEN_HEAD
        .iter()
        .chain(ADULT_CHICKEN_BEAK.iter())
        .chain(ADULT_CHICKEN_RED_THING.iter())
        .chain(ADULT_CHICKEN_BODY.iter())
        .chain(COLD_CHICKEN_HEAD.iter())
        .chain(COLD_CHICKEN_BODY.iter())
        .chain(ADULT_CHICKEN_LEG.iter())
        .chain(ADULT_CHICKEN_RIGHT_WING.iter())
        .chain(ADULT_CHICKEN_LEFT_WING.iter())
        .chain(BABY_CHICKEN_BODY.iter())
        .chain(BABY_CHICKEN_LEFT_LEG.iter())
        .chain(BABY_CHICKEN_RIGHT_LEG.iter())
        .chain(BABY_CHICKEN_RIGHT_WING.iter())
        .chain(BABY_CHICKEN_LEFT_WING.iter())
    {
        assert_eq!(cube.uv_size, cube.size);
        assert!(!cube.mirror);
    }
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
    let instances = [
        EntityModelInstance::chicken_variant(
            401,
            [0.0, 64.0, 0.0],
            0.0,
            ChickenModelVariant::Temperate,
            false,
        )
        .with_light_coords((6_u32 << 4) | (10_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true),
        EntityModelInstance::chicken_variant(
            402,
            [1.0, 64.0, 0.0],
            0.0,
            ChickenModelVariant::Cold,
            false,
        )
        .with_light_coords((6_u32 << 4) | (10_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true),
        EntityModelInstance::chicken_variant(
            403,
            [2.0, 64.0, 0.0],
            0.0,
            ChickenModelVariant::Warm,
            true,
        )
        .with_light_coords((6_u32 << 4) | (10_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true),
    ];
    let meshes = entity_model_textured_meshes(&instances, &atlas);
    assert_chicken_submissions_match_vanilla(&meshes, &instances);
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 156);
    assert_eq!(mesh.vertices.len(), 624);
    assert_eq!(mesh.indices.len(), 936);
    assert_close2(mesh.vertices[0].uv, [7.0 / 64.0, 0.0]);
    assert_eq!(mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[192].uv, [7.0 / 64.0, 96.0 / 144.0]);
    assert_eq!(mesh.vertices[192].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_close2(mesh.vertices[432].uv, [8.0 / 64.0, 80.0 / 144.0]);
    assert_eq!(mesh.vertices[432].tint, [1.0, 1.0, 1.0, 1.0]);
    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instances[0].render_state.shader_light()
        && vertex.overlay == instances[0].render_state.overlay_coords()));
    assert_ne!(instances[0].render_state.overlay_coords(), [0.0, 10.0]);
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
        let still_instance = base.with_walk_animation(2.5, 0.0);
        let walking_instance = base.with_walk_animation(0.0, 1.0);
        let resting = entity_model_textured_meshes(&[base], &atlas);
        let still = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_chicken_submissions_match_vanilla(&resting, &[base]);
        assert_chicken_submissions_match_vanilla(&still, &[still_instance]);
        assert_chicken_submissions_match_vanilla(&walking, &[walking_instance]);

        assert_eq!(
            resting.cutout.vertices, still.cutout.vertices,
            "{:?} is inert",
            base.kind
        );
        assert_eq!(
            resting.cutout.vertices.len(),
            walking.cutout.vertices.len(),
            "{:?} leg swing keeps the vertex count",
            base.kind
        );
        let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
        let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{:?} feet should lift off the ground (textured)",
            base.kind
        );
    }
}

fn assert_chicken_submissions_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instances: &[EntityModelInstance],
) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), instances.len());

    for (submit, instance) in meshes.submissions.iter().zip(instances) {
        let instance = *instance;
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.texture, instance.kind.vanilla_texture_ref().unwrap());
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, entity_model_root_transform(instance));
        assert_eq!((submit.order, submit.submit_sequence), (0, 0));
        assert_eq!(submit.light, instance.render_state.shader_light());
        assert_eq!(submit.overlay, instance.render_state.overlay_coords());
    }
}

#[test]
fn chicken_leg_swing_matches_vanilla_humanoid_phase() {
    // Vanilla ChickenModel.setupAnim: rightLeg.xRot = cos(pos*0.6662)*1.4*speed,
    // leftLeg.xRot = cos(pos*0.6662+π)*1.4*speed — the HumanoidModel phase. The right leg
    // (offset x < 0) is in phase, the left leg (x > 0) out of phase. Only xRot moves. The adult,
    // cold, and headless-baby layers share the same phase logic (the adult legs sit at x = ±,
    // the baby legs are swapped in x but the phase still follows the sign).
    let pos = 1.3_f32;
    let speed = 0.7_f32;
    let phase = pos * 0.6662;
    let leg_poses = [
        CHICKEN_RIGHT_LEG_POSE,
        CHICKEN_LEFT_LEG_POSE,
        BABY_CHICKEN_RIGHT_LEG_POSE,
        BABY_CHICKEN_LEFT_LEG_POSE,
    ];
    for base in leg_poses {
        let posed = humanoid_leg_swing_pose(base, pos, speed);
        let expected = if base.offset[0] < 0.0 {
            phase.cos() * 1.4 * speed
        } else {
            (phase + std::f32::consts::PI).cos() * 1.4 * speed
        };
        assert!(
            (posed.rotation[0] - expected).abs() < 1e-6,
            "leg at x {} xRot",
            base.offset[0]
        );
        assert_eq!(posed.offset, base.offset, "leg offset");
        assert_eq!(posed.rotation[1], base.rotation[1], "leg yRot");
        assert_eq!(posed.rotation[2], base.rotation[2], "leg zRot");
    }
}

#[test]
fn chicken_flaps_its_wings_when_airborne() {
    // Vanilla `ChickenModel.setupAnim`: `flapAngle = (sin(flap) + 1) * flapSpeed`,
    // applied as `rightWing.zRot = flapAngle` / `leftWing.zRot = -flapAngle`. With
    // `flapSpeed == 0` (a grounded/still chicken) the wings hold the bind pose; a
    // non-zero flap re-poses them. The adult, cold, and headless-baby layers all carry
    // the named wings. Colored path.
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

        // flapSpeed == 0 holds the wings however far the flap phase has advanced.
        let held = entity_model_mesh(&[base.with_chicken_flap(2.5).with_chicken_flap_speed(0.0)]);
        assert_eq!(
            rest.vertices, held.vertices,
            "{:?} wings hold the bind pose at flapSpeed 0",
            base.kind
        );

        // A live flap (non-zero speed + a phase off the sin zero) re-poses the wings.
        let flapping =
            entity_model_mesh(&[base.with_chicken_flap(1.0).with_chicken_flap_speed(1.0)]);
        assert_ne!(
            rest.vertices, flapping.vertices,
            "{:?} a flapping wing differs from the bind pose",
            base.kind
        );
        assert_eq!(
            rest.vertices.len(),
            flapping.vertices.len(),
            "{:?} the wing flap keeps the vertex count",
            base.kind
        );
    }
}

#[test]
fn chicken_wing_flap_and_leg_swing_are_independent() {
    // The wing flap (`flap`/`flapSpeed`) and the leg swing (`walkAnimationPos`/`Speed`)
    // come from separate render-state channels and pose disjoint parts, so each drives
    // a distinct mesh change.
    let base = EntityModelInstance::chicken(26, [0.0, 64.0, 0.0], 0.0, false);
    let rest = entity_model_mesh(&[base]);

    let only_walk = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    let only_flap = entity_model_mesh(&[base.with_chicken_flap(1.0).with_chicken_flap_speed(1.0)]);
    let both = entity_model_mesh(&[base
        .with_walk_animation(0.0, 1.0)
        .with_chicken_flap(1.0)
        .with_chicken_flap_speed(1.0)]);

    assert_ne!(
        rest.vertices, only_walk.vertices,
        "the leg swing moves legs"
    );
    assert_ne!(
        rest.vertices, only_flap.vertices,
        "the wing flap moves wings"
    );
    assert_ne!(
        only_walk.vertices, both.vertices,
        "adding the wing flap changes the walking mesh too"
    );
    assert_ne!(
        only_flap.vertices, both.vertices,
        "adding the leg swing changes the flapping mesh too"
    );
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
