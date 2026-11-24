use super::*;

use crate::entity_models::model_layers::parrot_model;

#[test]
fn parrot_sitting_pose_matches_vanilla_prepare() {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_6};

    // The seven named root parts and their bind poses, in vanilla `addOrReplaceChild` order.
    let parts = [
        ("body", layer_pose(ModelLayer::PARROT, "body")),
        ("tail", layer_pose(ModelLayer::PARROT, "tail")),
        ("left_wing", layer_pose(ModelLayer::PARROT, "left_wing")),
        ("right_wing", layer_pose(ModelLayer::PARROT, "right_wing")),
        ("head", layer_pose(ModelLayer::PARROT, "head")),
        ("left_leg", layer_pose(ModelLayer::PARROT, "left_leg")),
        ("right_leg", layer_pose(ModelLayer::PARROT, "right_leg")),
    ];

    // Standing with a neutral gaze and no flap (`flapAngle == 0`): the head look is identity, the walk
    // swing is identity, and the bob vanishes, so every part holds its bind offset. The one rotation
    // change is the wings — vanilla's STANDING fall-through sets `zRot = ±(0.0873 + flapAngle)`, so a
    // grounded parrot's wings settle to `zRot = ±0.0873` rather than the bind `zRot = 0`.
    let mut standing = parrot_model(
        &EntityModelInstance::parrot(0, [0.0, 64.0, 0.0], 0.0, ParrotModelVariant::RedBlue)
            .with(|s| s.on_ground = true),
    );
    let standing_root = &mut standing;
    for (name, pose) in parts {
        let part = standing_root.child_mut(name);
        assert_eq!(part.pose.offset, pose.offset, "part {name} offset");
    }
    // Non-wing parts hold their bind rotation; the wings tuck to ±0.0873.
    for (name, pose) in parts.iter().filter(|(name, _)| !name.ends_with("wing")) {
        assert_eq!(
            standing_root.child_mut(name).pose.rotation,
            pose.rotation,
            "part {name} rotation"
        );
    }
    assert!((standing_root.child_mut("left_wing").pose.rotation[2] - (-0.0873)).abs() < 1.0e-6);
    assert!((standing_root.child_mut("right_wing").pose.rotation[2] - 0.0873).abs() < 1.0e-6);

    // SITTING = `ParrotModel.prepare(SITTING)`: every part raises `y += 1.9`, the tail pitches
    // `xRot += π/6`, the wings tuck to `zRot = ±0.0873`, and the legs fold `xRot += π/2`.
    let mut sitting = parrot_model(
        &EntityModelInstance::parrot(0, [0.0, 64.0, 0.0], 0.0, ParrotModelVariant::RedBlue)
            .with(|s| s.kind.parrot_mut().sitting = true),
    );
    let root = &mut sitting;
    for (name, pose) in parts {
        assert!(
            (root.child_mut(name).pose.offset[1] - (pose.offset[1] + 1.9)).abs() < 1.0e-6,
            "part {name} should raise y by 1.9"
        );
    }
    // tail: xRot = 1.015 + π/6.
    assert!((root.child_mut("tail").pose.rotation[0] - (1.015 + FRAC_PI_6)).abs() < 1.0e-6);
    // wings: zRot set to ∓0.0873.
    assert!((root.child_mut("left_wing").pose.rotation[2] - (-0.0873)).abs() < 1.0e-6);
    assert!((root.child_mut("right_wing").pose.rotation[2] - 0.0873).abs() < 1.0e-6);
    // legs: xRot = -0.0299 + π/2.
    assert!((root.child_mut("left_leg").pose.rotation[0] - (-0.0299 + FRAC_PI_2)).abs() < 1.0e-6);
    assert!((root.child_mut("right_leg").pose.rotation[0] - (-0.0299 + FRAC_PI_2)).abs() < 1.0e-6);
    // `prepare(SITTING)` only translates the head; with a neutral gaze the head look leaves the head
    // rotation at bind.
    assert_eq!(
        root.child_mut("head").pose.rotation,
        layer_pose(ModelLayer::PARROT, "head").rotation
    );
}

#[test]
fn parrot_party_pose_matches_vanilla_prepare_and_setup_anim() {
    use std::f32::consts::PI;

    let age = 1.25_f32;
    let flap = 0.6_f32;
    let x_pos = age.cos();
    let y_pos = age.sin();
    let mut party = parrot_model(
        &EntityModelInstance::parrot(989, [0.0, 64.0, 0.0], 0.0, ParrotModelVariant::RedBlue)
            .with(|s| {
                s.kind.parrot_mut().party = true;
                s.kind.parrot_mut().sitting = true;
            })
            .with(|s| (s.head_yaw, s.head_pitch) = (40.0, -25.0))
            .with(|s| {
                s.age_in_ticks = age;
                s.kind.parrot_mut().flap_angle = flap;
            }),
    );
    let root = &mut party;

    // `ParrotModel.getPose` checks PARTY before SITTING, so the sitting y-offset/fold never applies.
    assert_eq!(
        root.child_mut("left_leg").pose.offset,
        layer_pose(ModelLayer::PARROT, "left_leg").offset
    );
    assert_eq!(
        root.child_mut("right_leg").pose.offset,
        layer_pose(ModelLayer::PARROT, "right_leg").offset
    );
    assert!((root.child_mut("left_leg").pose.rotation[2] - (-PI / 9.0)).abs() < 1.0e-6);
    assert!((root.child_mut("right_leg").pose.rotation[2] - (PI / 9.0)).abs() < 1.0e-6);

    // PARTY overwrites the head look (`xRot/yRot = 0`) and rolls the head by `sin(age) * 0.4`.
    let head = root.child_mut("head");
    assert!(
        (head.pose.offset[0] - (layer_pose(ModelLayer::PARROT, "head").offset[0] + x_pos)).abs()
            < 1.0e-6
    );
    assert!(
        (head.pose.offset[1] - (layer_pose(ModelLayer::PARROT, "head").offset[1] + y_pos)).abs()
            < 1.0e-6
    );
    assert_eq!(head.pose.rotation[0], 0.0);
    assert_eq!(head.pose.rotation[1], 0.0);
    assert!((head.pose.rotation[2] - y_pos * 0.4).abs() < 1.0e-6);

    let body = root.child_mut("body");
    assert!(
        (body.pose.offset[0] - (layer_pose(ModelLayer::PARROT, "body").offset[0] + x_pos)).abs()
            < 1.0e-6
    );
    assert!(
        (body.pose.offset[1] - (layer_pose(ModelLayer::PARROT, "body").offset[1] + y_pos)).abs()
            < 1.0e-6
    );

    let left_wing = root.child_mut("left_wing");
    assert!(
        (left_wing.pose.offset[0]
            - (layer_pose(ModelLayer::PARROT, "left_wing").offset[0] + x_pos))
            .abs()
            < 1.0e-6
    );
    assert!(
        (left_wing.pose.offset[1]
            - (layer_pose(ModelLayer::PARROT, "left_wing").offset[1] + y_pos))
            .abs()
            < 1.0e-6
    );
    assert!((left_wing.pose.rotation[2] - (-0.0873 - flap)).abs() < 1.0e-6);

    let right_wing = root.child_mut("right_wing");
    assert!(
        (right_wing.pose.offset[0]
            - (layer_pose(ModelLayer::PARROT, "right_wing").offset[0] + x_pos))
            .abs()
            < 1.0e-6
    );
    assert!(
        (right_wing.pose.offset[1]
            - (layer_pose(ModelLayer::PARROT, "right_wing").offset[1] + y_pos))
            .abs()
            < 1.0e-6
    );
    assert!((right_wing.pose.rotation[2] - (0.0873 + flap)).abs() < 1.0e-6);

    let tail = root.child_mut("tail");
    assert!(
        (tail.pose.offset[0] - (layer_pose(ModelLayer::PARROT, "tail").offset[0] + x_pos)).abs()
            < 1.0e-6
    );
    assert!(
        (tail.pose.offset[1] - (layer_pose(ModelLayer::PARROT, "tail").offset[1] + y_pos)).abs()
            < 1.0e-6
    );
    assert_eq!(
        tail.pose.rotation,
        layer_pose(ModelLayer::PARROT, "tail").rotation
    );
}

#[test]
fn parrot_walk_swing_matches_vanilla_setup_anim() {
    use std::f32::consts::PI;

    let pos = 2.0_f32;
    let speed = 0.75_f32;
    let phase = pos * 0.6662;

    // The left leg (offset x = +1.0) swings in phase, the right (x = -1.0) a half-cycle out, both
    // added onto the baked -0.0299 pitch.
    let left = parrot_leg_swing_pose(layer_pose(ModelLayer::PARROT, "left_leg"), pos, speed);
    let right = parrot_leg_swing_pose(layer_pose(ModelLayer::PARROT, "right_leg"), pos, speed);
    assert!((left.rotation[0] - (-0.0299 + phase.cos() * 1.4 * speed)).abs() < 1.0e-6);
    assert!((right.rotation[0] - (-0.0299 + (phase + PI).cos() * 1.4 * speed)).abs() < 1.0e-6);
    // The pivot and the other rotation axes are untouched.
    assert_eq!(
        left.offset,
        layer_pose(ModelLayer::PARROT, "left_leg").offset
    );
    assert_eq!(
        left.rotation[1],
        layer_pose(ModelLayer::PARROT, "left_leg").rotation[1]
    );
    assert_eq!(
        left.rotation[2],
        layer_pose(ModelLayer::PARROT, "left_leg").rotation[2]
    );

    // The tail adds cos(phase)·0.3·speed onto the baked 1.015 pitch.
    let tail = parrot_tail_swing_pose(layer_pose(ModelLayer::PARROT, "tail"), pos, speed);
    assert!((tail.rotation[0] - (1.015 + phase.cos() * 0.3 * speed)).abs() < 1.0e-6);

    // At rest (speed 0) every swing collapses to the baked pose.
    assert_eq!(
        parrot_leg_swing_pose(layer_pose(ModelLayer::PARROT, "left_leg"), pos, 0.0).rotation,
        layer_pose(ModelLayer::PARROT, "left_leg").rotation
    );
    assert_eq!(
        parrot_tail_swing_pose(layer_pose(ModelLayer::PARROT, "tail"), pos, 0.0).rotation,
        layer_pose(ModelLayer::PARROT, "tail").rotation
    );
}

#[test]
fn parrot_flying_pitches_legs_back_versus_standing() {
    // Vanilla `ParrotModel.prepare(FLYING)` pitches both legs `xRot += 2π/9`, and FLYING skips the
    // STANDING leg walk swing. With no flap the body/tail/wings/head match between a grounded and an
    // airborne parrot (both settle the wings to ±0.0873, bob is 0), so the only difference is the legs.
    let standing = entity_model_mesh(&[EntityModelInstance::parrot(
        72,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with(|s| s.on_ground = true)]);
    let flying = entity_model_mesh(&[EntityModelInstance::parrot(
        73,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with(|s| s.on_ground = false)]);
    assert_eq!(standing.vertices.len(), flying.vertices.len());
    const STILL: &[&str] = &["body", "tail", "left_wing", "right_wing", "head"];
    const LEGS: &[&str] = &["left_leg", "right_leg"];
    let model = ModelLayer::PARROT.bake();
    assert_eq!(
        parts_vertices(&standing.vertices, &model, STILL),
        parts_vertices(&flying.vertices, &model, STILL),
        "the body, tail, wings, and head match between standing and flying"
    );
    assert_ne!(
        parts_vertices(&standing.vertices, &model, LEGS),
        parts_vertices(&flying.vertices, &model, LEGS),
        "the flying parrot pitches its legs back (prepare(FLYING))"
    );
}

#[test]
fn parrot_textured_render_matches_vanilla_renderer() {
    // The five `Parrot.Variant` colours share one model and differ only by texture
    // (`ParrotRenderer.getVariantTexture`); `RED_BLUE` is the vanilla `DEFAULT`. Note the `Gray`
    // variant's file is the British-spelled `parrot_grey.png`.
    let variant_textures = [
        (ParrotModelVariant::RedBlue, "parrot_red_blue.png"),
        (ParrotModelVariant::Blue, "parrot_blue.png"),
        (ParrotModelVariant::Green, "parrot_green.png"),
        (ParrotModelVariant::YellowBlue, "parrot_yellow_blue.png"),
        (ParrotModelVariant::Gray, "parrot_grey.png"),
    ];
    for (variant, file) in variant_textures {
        let expected = EntityModelTextureRef {
            path: match variant {
                ParrotModelVariant::RedBlue => "textures/entity/parrot/parrot_red_blue.png",
                ParrotModelVariant::Blue => "textures/entity/parrot/parrot_blue.png",
                ParrotModelVariant::Green => "textures/entity/parrot/parrot_green.png",
                ParrotModelVariant::YellowBlue => "textures/entity/parrot/parrot_yellow_blue.png",
                ParrotModelVariant::Gray => "textures/entity/parrot/parrot_grey.png",
            },
            size: [32, 32],
        };
        assert!(expected.path.ends_with(file));
        let passes = parrot_textured_layer_passes(variant);
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].texture, expected);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(passes[0].order, 0);
        assert!(entity_model_texture_refs().contains(&expected));
    }

    // `Parrot.Variant.byId` clamps out-of-range ids to the nearest endpoint.
    assert_eq!(ParrotModelVariant::from_id(0), ParrotModelVariant::RedBlue);
    assert_eq!(ParrotModelVariant::from_id(4), ParrotModelVariant::Gray);
    assert_eq!(
        ParrotModelVariant::from_id(-99),
        ParrotModelVariant::RedBlue
    );
    assert_eq!(ParrotModelVariant::from_id(99), ParrotModelVariant::Gray);
    for (id, variant) in variant_textures
        .iter()
        .map(|(variant, _)| *variant)
        .enumerate()
    {
        assert_eq!(variant.vanilla_id(), id as i32);
    }

    let images: Vec<EntityModelTextureImage> = [
        PARROT_RED_BLUE_TEXTURE_REF,
        PARROT_BLUE_TEXTURE_REF,
        PARROT_GREEN_TEXTURE_REF,
        PARROT_YELLOW_BLUE_TEXTURE_REF,
        PARROT_GRAY_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    // Each colour emits textured geometry tinted white.
    for (variant, _) in variant_textures {
        let instance = EntityModelInstance::parrot(900, [0.0, 64.0, 0.0], 0.0, variant).with(|s| {
            s.light_coords = (6_u32 << 4) | (13_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());

        let mesh = &meshes.cutout;

        assert!(
            !mesh.vertices.is_empty(),
            "parrot {variant:?} emits geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
        assert!(mesh.vertices.iter().all(|vertex| vertex.light
            == instance.render_state.shader_light()
            && vertex.overlay == instance.render_state.overlay_coords()));
    }

    let party_instance =
        EntityModelInstance::parrot(901, [1.0, 64.0, -2.0], 15.0, ParrotModelVariant::RedBlue)
            .with(|s| {
                s.kind.parrot_mut().party = true;
                s.kind.parrot_mut().sitting = true;
            })
            .with(|s| (s.head_yaw, s.head_pitch) = (30.0, -20.0))
            .with(|s| {
                s.age_in_ticks = 1.25;
                s.kind.parrot_mut().flap_angle = 0.6;
                s.light_coords = (4_u32 << 4) | (11_u32 << 20);
            });
    let party = entity_model_textured_meshes(&[party_instance], &atlas);

    let standing_instance =
        EntityModelInstance::parrot(902, [1.0, 64.0, -2.0], 15.0, ParrotModelVariant::RedBlue)
            .with(|s| {
                s.on_ground = true;
                s.age_in_ticks = 1.25;
                s.kind.parrot_mut().flap_angle = 0.6;
                s.light_coords = (4_u32 << 4) | (11_u32 << 20);
            });
    let standing = entity_model_textured_meshes(&[standing_instance], &atlas);
    assert_eq!(party.cutout.vertices.len(), standing.cutout.vertices.len());
    assert_ne!(
        party.cutout.vertices, standing.cutout.vertices,
        "PARTY changes the posed parrot geometry without changing vanilla base submission metadata"
    );
}
