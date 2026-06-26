use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn polar_bear_model_parts_match_vanilla_26_1_body_layers() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`/`mirror`.
    assert_eq!(
        ADULT_POLAR_BEAR_HEAD[1],
        ModelCube::new(
            [-2.5, 1.0, -6.0],
            [5.0, 3.0, 3.0],
            POLAR_BEAR_WHITE,
            [5.0, 3.0, 3.0],
            [0.0, 44.0],
            false,
        )
    );
    // The right/left ear cubes mirror about the head.
    assert!(!ADULT_POLAR_BEAR_HEAD[2].mirror);
    assert!(ADULT_POLAR_BEAR_HEAD[3].mirror);
    assert_eq!(ADULT_POLAR_BEAR_BODY[0].size, [14.0, 14.0, 11.0]);
    assert_eq!(ADULT_POLAR_BEAR_HIND_LEG[0].size, [4.0, 10.0, 8.0]);
    assert_eq!(ADULT_POLAR_BEAR_FRONT_LEG[0].size, [4.0, 10.0, 6.0]);

    assert_eq!(
        BABY_POLAR_BEAR_HEAD[1],
        ModelCube::new(
            [-2.0, 0.375, -6.25],
            [4.0, 2.0, 2.0],
            POLAR_BEAR_WHITE,
            [4.0, 2.0, 2.0],
            [20.0, 3.0],
            false,
        )
    );
    assert_eq!(BABY_POLAR_BEAR_BODY[0].size, [8.0, 7.0, 12.0]);
    assert_eq!(BABY_POLAR_BEAR_RIGHT_HIND_LEG[0].size, [3.0, 3.0, 3.0]);
}

#[test]
fn polar_bear_meshes_use_vanilla_body_layers() {
    let adult = entity_model_mesh(&[EntityModelInstance::polar_bear(
        210,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 60);
    assert_eq!(adult.vertices.len(), 240);
    assert_eq!(adult.indices.len(), 360);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(POLAR_BEAR_WHITE, 0.78)));

    let baby = entity_model_mesh(&[EntityModelInstance::polar_bear(
        211,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 54);
    assert_eq!(baby.vertices.len(), 216);
    assert_eq!(baby.indices.len(), 324);

    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    assert!(adult_max[1] > baby_max[1]);
    assert!(adult_min[2] < baby_min[2]);
}

#[test]
fn polar_bear_texture_refs_match_vanilla_renderer() {
    let cases = [
        (
            false,
            "polar_bear",
            EntityModelTextureRef {
                path: "textures/entity/bear/polarbear.png",
                size: [128, 64],
            },
        ),
        (
            true,
            "polar_bear_baby",
            EntityModelTextureRef {
                path: "textures/entity/bear/polarbear_baby.png",
                size: [64, 64],
            },
        ),
    ];

    for (baby, model_key, texture) in cases {
        let kind = EntityModelKind::PolarBear { baby };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }

    assert_eq!(
        polar_bear_entity_texture_refs(),
        &[
            EntityModelTextureRef {
                path: "textures/entity/bear/polarbear.png",
                size: [128, 64],
            },
            EntityModelTextureRef {
                path: "textures/entity/bear/polarbear_baby.png",
                size: [64, 64],
            },
        ]
    );
    assert!(entity_model_texture_refs().contains(&POLAR_BEAR_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&POLAR_BEAR_BABY_TEXTURE_REF));
}

#[test]
fn polar_bear_textured_layer_passes_match_vanilla_renderer_model_choice() {
    let adult = polar_bear_textured_layer_passes(false);
    assert_eq!(adult.len(), 1);
    assert_eq!(adult[0].kind, EntityModelLayerKind::PolarBearBase);
    assert_eq!(
        adult[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[0].model_layer, MODEL_LAYER_POLAR_BEAR);
    assert_eq!(adult[0].texture, POLAR_BEAR_TEXTURE_REF);
    assert_eq!(adult[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((adult[0].order, adult[0].submit_sequence), (0, 0));

    let baby = polar_bear_textured_layer_passes(true);
    assert_eq!(baby.len(), 1);
    assert_eq!(baby[0].kind, EntityModelLayerKind::PolarBearBase);
    assert_eq!(
        baby[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(baby[0].model_layer, MODEL_LAYER_POLAR_BEAR_BABY);
    assert_eq!(baby[0].texture, POLAR_BEAR_BABY_TEXTURE_REF);
    assert_eq!(baby[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(baby[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((baby[0].order, baby[0].submit_sequence), (0, 0));
}

#[test]
fn polar_bear_textured_model_parts_match_vanilla_model_layer_uv_sources() {
    // The textured UV sources now live on the unified cubes (`uv_size`/`tex`/`mirror`).
    assert_eq!(MODEL_LAYER_POLAR_BEAR, "minecraft:polar_bear#main");
    assert_eq!(
        MODEL_LAYER_POLAR_BEAR_BABY,
        "minecraft:polar_bear_baby#main"
    );
    assert_eq!(ADULT_POLAR_BEAR_HEAD[1].tex, [0.0, 44.0]);
    assert_eq!(ADULT_POLAR_BEAR_HEAD[3].tex, [26.0, 0.0]);
    assert!(ADULT_POLAR_BEAR_HEAD[3].mirror);
    assert_eq!(ADULT_POLAR_BEAR_BODY[0].tex, [0.0, 19.0]);
    assert_eq!(ADULT_POLAR_BEAR_BODY[1].tex, [39.0, 0.0]);
    assert_eq!(ADULT_POLAR_BEAR_HIND_LEG[0].tex, [50.0, 22.0]);
    assert_eq!(ADULT_POLAR_BEAR_FRONT_LEG[0].tex, [50.0, 40.0]);

    assert_eq!(BABY_POLAR_BEAR_BODY[0].tex, [0.0, 9.0]);
    assert_eq!(BABY_POLAR_BEAR_HEAD[0].tex, [0.0, 0.0]);
    assert_eq!(BABY_POLAR_BEAR_HEAD[1].tex, [20.0, 3.0]);
    assert_eq!(BABY_POLAR_BEAR_HEAD[2].tex, [20.0, 0.0]);
    assert_eq!(BABY_POLAR_BEAR_HEAD[3].tex, [26.0, 0.0]);
    assert_eq!(BABY_POLAR_BEAR_RIGHT_HIND_LEG[0].tex, [0.0, 34.0]);
    assert_eq!(BABY_POLAR_BEAR_LEFT_HIND_LEG[0].tex, [12.0, 34.0]);
    assert_eq!(BABY_POLAR_BEAR_RIGHT_FRONT_LEG[0].tex, [0.0, 28.0]);
    assert_eq!(BABY_POLAR_BEAR_LEFT_FRONT_LEG[0].tex, [12.0, 28.0]);
}

#[test]
fn entity_texture_atlas_stitches_official_polar_bear_png_slots() {
    let (layout, rgba) = build_entity_model_texture_atlas(&polar_bear_texture_images()).unwrap();

    assert_eq!(layout.width, 128);
    assert_eq!(layout.height, 128);
    assert_eq!(
        layout
            .entries
            .iter()
            .map(|entry| entry.texture.path)
            .collect::<Vec<_>>(),
        vec![
            "textures/entity/bear/polarbear.png",
            "textures/entity/bear/polarbear_baby.png",
        ]
    );
    assert_close2(layout.entries[0].uv.min, [0.0, 0.0]);
    assert_close2(layout.entries[0].uv.max, [1.0, 0.5]);
    assert_close2(layout.entries[1].uv.min, [0.0, 0.5]);
    assert_close2(layout.entries[1].uv.max, [0.5, 1.0]);
    assert_eq!(&rgba[0..4], &[0; 4]);
    let baby_first_pixel = rgba_offset(layout.width, 64, 0, "polar bear baby atlas row").unwrap();
    assert_eq!(&rgba[baby_first_pixel..baby_first_pixel + 4], &[1; 4]);
}

#[test]
fn polar_bear_textured_mesh_uses_vanilla_uvs_tints_and_scale() {
    let (atlas, _) = build_entity_model_texture_atlas(&polar_bear_texture_images()).unwrap();
    let adult = EntityModelInstance::polar_bear(212, [0.0, 64.0, 0.0], 0.0, false);
    let adult_mesh = entity_model_textured_mesh(&[adult], &atlas);
    assert_eq!(adult_mesh.cutout_faces, 60);
    assert_eq!(adult_mesh.vertices.len(), 240);
    assert_eq!(adult_mesh.indices.len(), 360);
    assert_close2(adult_mesh.vertices[0].uv, [14.0 / 128.0, 0.0]);
    assert_eq!(adult_mesh.vertices[0].tint, [1.0, 1.0, 1.0, 1.0]);
    let (adult_textured_min, adult_textured_max) = textured_mesh_extents(&adult_mesh);
    let (adult_colored_min, adult_colored_max) = mesh_extents(&entity_model_mesh(&[adult]));
    assert_close3(adult_textured_min, adult_colored_min);
    assert_close3(adult_textured_max, adult_colored_max);

    let baby = EntityModelInstance::polar_bear(213, [0.0, 64.0, 0.0], 0.0, true);
    let baby_mesh = entity_model_textured_mesh(&[baby], &atlas);
    assert_eq!(baby_mesh.cutout_faces, 54);
    assert_eq!(baby_mesh.vertices.len(), 216);
    assert_eq!(baby_mesh.indices.len(), 324);
    assert_close2(baby_mesh.vertices[0].uv, [20.0 / 128.0, 73.0 / 128.0]);
    assert!(baby_mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (baby_textured_min, baby_textured_max) = textured_mesh_extents(&baby_mesh);
    let (baby_colored_min, baby_colored_max) = mesh_extents(&entity_model_mesh(&[baby]));
    assert_close3(baby_textured_min, baby_colored_min);
    assert_close3(baby_textured_max, baby_colored_max);
}

#[test]
fn polar_bear_textured_meshes_apply_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&polar_bear_texture_images()).unwrap();

    // Adult head is part 0 (4 cubes = first 96 vertices); body and legs follow and
    // must stay put under a head look.
    let adult = EntityModelInstance::polar_bear(214, [0.0, 64.0, 0.0], 0.0, false);
    let resting = entity_model_textured_mesh(&[adult], &atlas);
    let yawed = entity_model_textured_mesh(&[adult.with_head_look(50.0, 0.0)], &atlas);
    let pitched = entity_model_textured_mesh(&[adult.with_head_look(0.0, -20.0)], &atlas);
    assert_eq!(resting.vertices.len(), yawed.vertices.len());
    assert_ne!(resting.vertices[0..96], yawed.vertices[0..96]);
    assert_eq!(resting.vertices[96..], yawed.vertices[96..]);
    assert_ne!(yawed.vertices[0..96], pitched.vertices[0..96]);

    // Baby lists the body first (1 cube = first 24 vertices); head is index 1.
    let baby = EntityModelInstance::polar_bear(215, [0.0, 64.0, 0.0], 0.0, true);
    let baby_resting = entity_model_textured_mesh(&[baby], &atlas);
    let baby_looking = entity_model_textured_mesh(&[baby.with_head_look(50.0, -20.0)], &atlas);
    assert_ne!(baby_resting.vertices, baby_looking.vertices);
    assert_eq!(baby_resting.vertices[0..24], baby_looking.vertices[0..24]);

    // While rearing, the head look still applies, composed on top of the standing
    // pose and distinct from a flat (non-standing) look.
    let standing = EntityModelInstance::polar_bear_standing(216, [0.0, 64.0, 0.0], 0.0, false, 1.0);
    let standing_resting = entity_model_textured_mesh(&[standing], &atlas);
    let standing_looking =
        entity_model_textured_mesh(&[standing.with_head_look(50.0, -20.0)], &atlas);
    let flat_looking = entity_model_textured_mesh(&[adult.with_head_look(50.0, -20.0)], &atlas);
    assert_ne!(
        standing_resting.vertices[0..96],
        standing_looking.vertices[0..96]
    );
    assert_eq!(
        standing_resting.vertices[96..],
        standing_looking.vertices[96..]
    );
    assert_ne!(
        standing_looking.vertices[0..96],
        flat_looking.vertices[0..96]
    );
}

#[test]
fn polar_bear_standing_part_roles_cover_the_reared_parts() {
    // The rear-up moves the head, body, and both front legs (by name); the swing resolves these the
    // same for the adult and baby layouts.
    assert_eq!(
        polar_bear_standing_part_roles(),
        [
            ("head", PolarBearStandPart::Head),
            ("body", PolarBearStandPart::Body),
            ("right_front_leg", PolarBearStandPart::FrontLeg),
            ("left_front_leg", PolarBearStandPart::FrontLeg),
        ]
    );
}

#[test]
fn apply_polar_bear_standing_pose_matches_vanilla_setup_anim() {
    let pi = std::f32::consts::PI;

    // Adult (ageScale 1.0) at standScale 1.0 (squared = 1.0). The bind poses are the vanilla
    // `PolarBearModel.createBodyLayer` head/body/front-leg offsets.
    let adult_head_pose = PartPose {
        offset: [0.0, 10.0, -16.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let mut head = adult_head_pose;
    apply_polar_bear_standing_pose(&mut head, PolarBearStandPart::Head, false, 1.0);
    assert_eq!(head.offset, [0.0, 10.0 - 24.0, -16.0 + 13.0]);
    assert!((head.rotation[0] - pi * 0.15).abs() < 1e-6);
    assert_eq!([head.rotation[1], head.rotation[2]], [0.0, 0.0]);

    let mut body = PartPose {
        offset: [-2.0, 9.0, 12.0],
        rotation: [std::f32::consts::FRAC_PI_2, 0.0, 0.0],
    };
    apply_polar_bear_standing_pose(&mut body, PolarBearStandPart::Body, false, 1.0);
    assert_eq!(body.offset, [-2.0, 9.0 + 2.0, 12.0]);
    assert!((body.rotation[0] - (std::f32::consts::FRAC_PI_2 - pi * 0.35)).abs() < 1e-6);

    let mut front_leg = PartPose {
        offset: [-3.5, 14.0, -8.0],
        rotation: [0.0, 0.0, 0.0],
    };
    apply_polar_bear_standing_pose(&mut front_leg, PolarBearStandPart::FrontLeg, false, 1.0);
    assert_eq!(front_leg.offset, [-3.5, 14.0 - 20.0, -8.0 + 4.0]);
    assert!((front_leg.rotation[0] - (-pi * 0.45)).abs() < 1e-6);

    // standScale is squared: 0.5 -> 0.25 of the full delta.
    let mut quarter_head = adult_head_pose;
    apply_polar_bear_standing_pose(&mut quarter_head, PolarBearStandPart::Head, false, 0.5);
    assert_eq!(quarter_head.offset[1], 10.0 - 0.25 * 24.0);

    // Baby (ageScale 0.5) scales only the body/front-leg translation terms.
    let mut baby_body = PartPose {
        offset: [0.0, 17.5, 0.0],
        rotation: [0.0, 0.0, 0.0],
    };
    apply_polar_bear_standing_pose(&mut baby_body, PolarBearStandPart::Body, true, 1.0);
    assert_eq!(baby_body.offset[1], 17.5 + 0.5 * 2.0);

    let mut baby_front_leg = PartPose {
        offset: [-2.5, 21.5, -4.5],
        rotation: [0.0, 0.0, 0.0],
    };
    apply_polar_bear_standing_pose(&mut baby_front_leg, PolarBearStandPart::FrontLeg, true, 1.0);
    assert_eq!(baby_front_leg.offset[1], 21.5 - 0.5 * 20.0);
    assert_eq!(baby_front_leg.offset[2], -4.5 + 0.5 * 4.0);

    // The head translation does not use ageScale, so the baby head moves the
    // same absolute amount as the adult head.
    let mut baby_head = PartPose {
        offset: [0.0, 18.625, -5.75],
        rotation: [0.0, 0.0, 0.0],
    };
    apply_polar_bear_standing_pose(&mut baby_head, PolarBearStandPart::Head, true, 1.0);
    assert_eq!(baby_head.offset, [0.0, 18.625 - 24.0, -5.75 + 13.0]);
}

#[test]
fn polar_bear_standing_mesh_rears_head_body_and_front_legs() {
    let resting = entity_model_mesh(&[EntityModelInstance::polar_bear(
        220,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let standing = entity_model_mesh(&[EntityModelInstance::polar_bear_standing(
        220,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        1.0,
    )]);
    assert_eq!(resting.vertices.len(), 240);
    assert_eq!(standing.vertices.len(), 240);
    // Adult layout: head 0..96, body 96..144, hind legs 144..192, front legs 192..240.
    assert_eq!(resting.vertices[144..192], standing.vertices[144..192]);
    assert_ne!(resting.vertices[0..96], standing.vertices[0..96]);
    assert_ne!(resting.vertices[96..144], standing.vertices[96..144]);
    assert_ne!(resting.vertices[192..216], standing.vertices[192..216]);
    assert_ne!(resting.vertices[216..240], standing.vertices[216..240]);

    // standScale 0.0 is a no-op identical to the resting mesh.
    let neutral = entity_model_mesh(&[EntityModelInstance::polar_bear_standing(
        220,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        0.0,
    )]);
    assert_eq!(resting.vertices, neutral.vertices);

    // Baby layout: body 0..24, head 24..120, hind legs 120..168, front legs 168..216.
    let baby_resting = entity_model_mesh(&[EntityModelInstance::polar_bear(
        221,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    let baby_standing = entity_model_mesh(&[EntityModelInstance::polar_bear_standing(
        221,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        1.0,
    )]);
    assert_eq!(
        baby_resting.vertices[120..168],
        baby_standing.vertices[120..168]
    );
    assert_ne!(baby_resting.vertices[0..24], baby_standing.vertices[0..24]);
    assert_ne!(
        baby_resting.vertices[24..120],
        baby_standing.vertices[24..120]
    );
    assert_ne!(
        baby_resting.vertices[168..216],
        baby_standing.vertices[168..216]
    );
}

#[test]
fn polar_bear_swings_its_legs_when_walking() {
    // Vanilla `PolarBearModel extends QuadrupedModel`: `setupAnim` runs
    // `super.setupAnim` (the four-leg swing) before the standing pose. A standing
    // (not rearing) polar bear is inert; a walking one swings all four legs (head and
    // body untouched) and lifts its feet. Adult layout: head 0..96, body 96..144,
    // hind legs 144..192, front legs 192..240.
    let base = EntityModelInstance::polar_bear(230, [0.0, 64.0, 0.0], 0.0, false);
    let rest = entity_model_mesh(&[base]);
    let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
    assert_eq!(rest.vertices, still.vertices, "rest is inert");

    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_eq!(
        rest.vertices[0..96],
        walking.vertices[0..96],
        "head unmoved"
    );
    assert_eq!(
        rest.vertices[96..144],
        walking.vertices[96..144],
        "body unmoved"
    );
    assert_ne!(
        rest.vertices[144..192],
        walking.vertices[144..192],
        "hind legs swing"
    );
    assert_ne!(
        rest.vertices[192..240],
        walking.vertices[192..240],
        "front legs swing"
    );

    let (rest_min, rest_max) = mesh_extents(&rest);
    let (walk_min, walk_max) = mesh_extents(&walking);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking polar bear's feet should lift off the ground"
    );
}

#[test]
fn polar_bear_leg_swing_composes_with_the_standing_rear() {
    // Vanilla applies the leg swing in `super.setupAnim`, then the standing rear adds
    // `frontLeg.xRot -= standScale * π * 0.45` on top. So a walking, rearing bear's
    // front legs differ from a still, rearing bear's (the swing rides on top of the
    // standing delta), and the hind legs (untouched by the rear) differ too, while
    // the head and body (no swing) stay identical between the two.
    let standing = EntityModelInstance::polar_bear_standing(231, [0.0, 64.0, 0.0], 0.0, false, 1.0);
    let standing_still = entity_model_mesh(&[standing]);
    let standing_walking = entity_model_mesh(&[standing.with_walk_animation(0.0, 1.0)]);
    assert_eq!(
        standing_still.vertices[0..96],
        standing_walking.vertices[0..96],
        "standing head unaffected by the swing"
    );
    assert_eq!(
        standing_still.vertices[96..144],
        standing_walking.vertices[96..144],
        "standing body unaffected by the swing"
    );
    assert_ne!(
        standing_still.vertices[144..192],
        standing_walking.vertices[144..192],
        "standing hind legs still swing"
    );
    assert_ne!(
        standing_still.vertices[192..240],
        standing_walking.vertices[192..240],
        "standing front legs swing on top of the rear delta"
    );
}

#[test]
fn polar_bear_textured_mesh_swings_legs_when_walking() {
    // The real polar bear render path (texture-backed) swings the same
    // `QuadrupedModel` legs. A standing bear is byte-identical however far the swing
    // has advanced; a walking adult lifts its feet. Baby legs swing too (asserted via
    // the vertex difference, as the short legs stay inside the bounding box).
    let (atlas, _) = build_entity_model_texture_atlas(&polar_bear_texture_images()).unwrap();
    for (name, base, adult_size) in [
        (
            "adult",
            EntityModelInstance::polar_bear(232, [0.0, 64.0, 0.0], 0.0, false),
            true,
        ),
        (
            "baby",
            EntityModelInstance::polar_bear(233, [0.0, 64.0, 0.0], 0.0, true),
            false,
        ),
    ] {
        let resting = entity_model_textured_mesh(&[base], &atlas);
        let still = entity_model_textured_mesh(&[base.with_walk_animation(2.5, 0.0)], &atlas);
        let walking = entity_model_textured_mesh(&[base.with_walk_animation(0.0, 1.0)], &atlas);

        assert_eq!(
            resting.vertices, still.vertices,
            "{name}: a standing polar bear is inert"
        );
        assert_eq!(
            resting.vertices.len(),
            walking.vertices.len(),
            "{name}: leg swing keeps the vertex count"
        );
        assert_ne!(
            resting.vertices, walking.vertices,
            "{name}: a walking polar bear differs"
        );

        if adult_size {
            let (rest_min, rest_max) = textured_mesh_extents(&resting);
            let (walk_min, walk_max) = textured_mesh_extents(&walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
                "{name}: a walking polar bear's feet should lift off the ground"
            );
        }
    }
}

fn polar_bear_texture_images() -> Vec<EntityModelTextureImage> {
    polar_bear_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
