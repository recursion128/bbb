use super::*;
use crate::entity_models::model::EntityModel;

#[test]
fn parrot_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `ParrotModel.createBodyLayer` (atlas 32×32): seven named sibling root parts — body,
    // tail, two wings, head (parenting head2, the two beak halves, and the crest feather), two legs.

    // `body` (3×6×3) pitched by 0.4937 rad.
    assert_eq!(PARROT_BODY_POSE.offset, [0.0, 16.5, -3.0]);
    assert_eq!(PARROT_BODY_POSE.rotation, [0.4937, 0.0, 0.0]);
    assert_eq!(PARROT_BODY_CUBES[0].size, [3.0, 6.0, 3.0]);

    // `tail` (3×4×1) pitched by 1.015 rad.
    assert_eq!(PARROT_TAIL_POSE.offset, [0.0, 21.07, 1.16]);
    assert_eq!(PARROT_TAIL_POSE.rotation, [1.015, 0.0, 0.0]);
    assert_eq!(PARROT_TAIL_CUBES[0].size, [3.0, 4.0, 1.0]);

    // The two 1×5×3 wings: mirrored pivots, both flipped yRot = -π.
    assert_eq!(PARROT_LEFT_WING_POSE.offset, [1.5, 16.94, -2.76]);
    assert_eq!(
        PARROT_LEFT_WING_POSE.rotation,
        [-0.6981, -std::f32::consts::PI, 0.0]
    );
    assert_eq!(PARROT_RIGHT_WING_POSE.offset, [-1.5, 16.94, -2.76]);
    assert_eq!(PARROT_WING_CUBES[0].size, [1.0, 5.0, 3.0]);

    // `head` (2×3×2) at offset (0, 15.69, -2.76), parenting four cubes.
    assert_eq!(PARROT_HEAD_POSE.offset, [0.0, 15.69, -2.76]);
    assert_eq!(PARROT_HEAD_CUBES[0].size, [2.0, 3.0, 2.0]);
    // head2 2×1×4, beak1 / beak2 1×2×1, the crest feather 0×5×4 pitched by -0.2214 rad.
    assert_eq!(PARROT_HEAD2_CUBES[0].size, [2.0, 1.0, 4.0]);
    assert_eq!(PARROT_BEAK1_CUBES[0].size, [1.0, 2.0, 1.0]);
    assert_eq!(PARROT_FEATHER_POSE.rotation, [-0.2214, 0.0, 0.0]);
    assert_eq!(PARROT_FEATHER_CUBES[0].size, [0.0, 5.0, 4.0]);

    // The two 1×2×1 legs at the mirrored pivots, both pitched by -0.0299 rad.
    assert_eq!(PARROT_LEFT_LEG_POSE.offset, [1.0, 22.0, -1.05]);
    assert_eq!(PARROT_LEFT_LEG_POSE.rotation, [-0.0299, 0.0, 0.0]);
    assert_eq!(PARROT_RIGHT_LEG_POSE.offset, [-1.0, 22.0, -1.05]);
}

#[test]
fn parrot_sitting_pose_matches_vanilla_prepare() {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_6};

    // The seven named root parts and their bind poses, in vanilla `addOrReplaceChild` order.
    let parts = [
        ("body", PARROT_BODY_POSE),
        ("tail", PARROT_TAIL_POSE),
        ("left_wing", PARROT_LEFT_WING_POSE),
        ("right_wing", PARROT_RIGHT_WING_POSE),
        ("head", PARROT_HEAD_POSE),
        ("left_leg", PARROT_LEFT_LEG_POSE),
        ("right_leg", PARROT_RIGHT_LEG_POSE),
    ];

    // Standing with a neutral gaze and no flap (`flapAngle == 0`): the head look is identity, the walk
    // swing is identity, and the bob vanishes, so every part holds its bind offset. The one rotation
    // change is the wings — vanilla's STANDING fall-through sets `zRot = ±(0.0873 + flapAngle)`, so a
    // grounded parrot's wings settle to `zRot = ±0.0873` rather than the bind `zRot = 0`.
    let mut standing = ParrotModel::new();
    standing.prepare(
        &EntityModelInstance::parrot(0, [0.0, 64.0, 0.0], 0.0, ParrotModelVariant::RedBlue)
            .with_on_ground(true),
    );
    let standing_root = standing.root_mut();
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
    let mut sitting = ParrotModel::new();
    sitting.prepare(
        &EntityModelInstance::parrot(0, [0.0, 64.0, 0.0], 0.0, ParrotModelVariant::RedBlue)
            .with_parrot_sitting(true),
    );
    let root = sitting.root_mut();
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
        PARROT_HEAD_POSE.rotation
    );
}

#[test]
fn parrot_head_look_turns_only_the_head_subtree() {
    // Vanilla `ParrotModel.setupAnim` sets `head.xRot/yRot` from the look angles before the
    // per-pose switch, so the head and its beak/crest children turn while the body, tail, wings,
    // and legs hold. Depth-first emit order: body/tail/wings `[0, 96)`, the head plus its four
    // children `[96, 216)`, then the two legs `[216, 264)`. Only the head subtree moves.
    let rest = EntityModelInstance::parrot(990, [0.0, 64.0, 0.0], 0.0, ParrotModelVariant::RedBlue);
    let looked = rest.with_head_look(35.0, -25.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_eq!(
        rest_mesh.vertices[..96],
        looked_mesh.vertices[..96],
        "the body, tail, and wings stay put"
    );
    assert_ne!(
        rest_mesh.vertices[96..216],
        looked_mesh.vertices[96..216],
        "the head and its beak/crest children turn"
    );
    assert_eq!(
        rest_mesh.vertices[216..],
        looked_mesh.vertices[216..],
        "the legs stay put"
    );

    // The head look also applies on the sitting perch — only the un-projected PARTY pose would
    // overwrite it. The perched head is raised but still turns.
    let sit_rest = entity_model_mesh(&[rest.with_parrot_sitting(true)]);
    let sit_looked = entity_model_mesh(&[looked.with_parrot_sitting(true)]);
    assert_eq!(sit_rest.vertices[..96], sit_looked.vertices[..96]);
    assert_ne!(sit_rest.vertices[96..216], sit_looked.vertices[96..216]);
    assert_eq!(sit_rest.vertices[216..], sit_looked.vertices[216..]);
}

#[test]
fn parrot_sitting_mesh_differs_from_standing() {
    // The perched parrot re-poses every part (raise + fold), so its mesh differs from standing
    // while keeping the same 11-cube vertex count.
    let standing = entity_model_mesh(&[EntityModelInstance::parrot(
        981,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with_on_ground(true)]);
    let sitting = entity_model_mesh(&[EntityModelInstance::parrot(
        982,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with_parrot_sitting(true)]);
    assert_eq!(standing.vertices.len(), sitting.vertices.len());
    assert_ne!(
        standing.vertices, sitting.vertices,
        "the sitting parrot perches lower with folded legs"
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
    let left = parrot_leg_swing_pose(PARROT_LEFT_LEG_POSE, pos, speed);
    let right = parrot_leg_swing_pose(PARROT_RIGHT_LEG_POSE, pos, speed);
    assert!((left.rotation[0] - (-0.0299 + phase.cos() * 1.4 * speed)).abs() < 1.0e-6);
    assert!((right.rotation[0] - (-0.0299 + (phase + PI).cos() * 1.4 * speed)).abs() < 1.0e-6);
    // The pivot and the other rotation axes are untouched.
    assert_eq!(left.offset, PARROT_LEFT_LEG_POSE.offset);
    assert_eq!(left.rotation[1], PARROT_LEFT_LEG_POSE.rotation[1]);
    assert_eq!(left.rotation[2], PARROT_LEFT_LEG_POSE.rotation[2]);

    // The tail adds cos(phase)·0.3·speed onto the baked 1.015 pitch.
    let tail = parrot_tail_swing_pose(PARROT_TAIL_POSE, pos, speed);
    assert!((tail.rotation[0] - (1.015 + phase.cos() * 0.3 * speed)).abs() < 1.0e-6);

    // At rest (speed 0) every swing collapses to the baked pose.
    assert_eq!(
        parrot_leg_swing_pose(PARROT_LEFT_LEG_POSE, pos, 0.0).rotation,
        PARROT_LEFT_LEG_POSE.rotation
    );
    assert_eq!(
        parrot_tail_swing_pose(PARROT_TAIL_POSE, pos, 0.0).rotation,
        PARROT_TAIL_POSE.rotation
    );
}

#[test]
fn parrot_walk_swing_moves_only_the_legs_and_tail() {
    // A walking standing parrot swings its tail [24, 48) and both legs [216, 264) while the body
    // [0, 24), wings [48, 96), and head subtree [96, 216) hold. The walk swing does not touch the
    // wings, and with no flap (`flapAngle == 0`) the wing zRot settles to ±0.0873 for both meshes
    // (the bob also vanishes), so the wing slice matches between rest and walking.
    let rest = entity_model_mesh(&[EntityModelInstance::parrot(
        992,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with_on_ground(true)]);
    let walking = entity_model_mesh(&[EntityModelInstance::parrot(
        993,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with_on_ground(true)
    .with_walk_animation(2.0, 1.0)]);
    assert_eq!(rest.vertices.len(), walking.vertices.len());
    assert_eq!(
        rest.vertices[0..24],
        walking.vertices[0..24],
        "the body holds"
    );
    assert_ne!(
        rest.vertices[24..48],
        walking.vertices[24..48],
        "the tail swings"
    );
    assert_eq!(
        rest.vertices[48..96],
        walking.vertices[48..96],
        "the wings hold under the walk swing (no flap, so both settle to ±0.0873)"
    );
    assert_eq!(
        rest.vertices[96..216],
        walking.vertices[96..216],
        "the head holds"
    );
    assert_ne!(
        rest.vertices[216..264],
        walking.vertices[216..264],
        "both legs swing"
    );

    // A perched parrot skips the swing: the vanilla SITTING branch breaks before it.
    let sit_rest = entity_model_mesh(&[EntityModelInstance::parrot(
        994,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with_parrot_sitting(true)]);
    let sit_walk = entity_model_mesh(&[EntityModelInstance::parrot(
        995,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with_parrot_sitting(true)
    .with_walk_animation(2.0, 1.0)]);
    assert_eq!(
        sit_rest.vertices, sit_walk.vertices,
        "a perched parrot is inert under walk animation"
    );
}

#[test]
fn parrot_flaps_its_wings_when_airborne() {
    // Vanilla `ParrotModel.setupAnim` STANDING/FLYING fall-through: `leftWing.zRot = -0.0873 -
    // flapAngle`, `rightWing.zRot = 0.0873 + flapAngle`, plus the `flapAngle * 0.3` body/wing/leg bob.
    // With `flapAngle == 0` the wings settle to ±0.0873 (the rest splay); a live flap re-poses them
    // and lifts the bobbing parts. Tested airborne (FLYING) and grounded (STANDING) — both carry the
    // flap.
    for on_ground in [false, true] {
        let rest = entity_model_mesh(&[EntityModelInstance::parrot(
            70,
            [0.0, 64.0, 0.0],
            0.0,
            ParrotModelVariant::RedBlue,
        )
        .with_on_ground(on_ground)
        .with_parrot_flap_angle(0.0)]);
        let flapping = entity_model_mesh(&[EntityModelInstance::parrot(
            71,
            [0.0, 64.0, 0.0],
            0.0,
            ParrotModelVariant::RedBlue,
        )
        .with_on_ground(on_ground)
        .with_parrot_flap_angle(0.8)]);
        assert_eq!(
            rest.vertices.len(),
            flapping.vertices.len(),
            "the wing flap keeps the vertex count (on_ground={on_ground})"
        );
        assert_ne!(
            rest.vertices, flapping.vertices,
            "a non-zero flapAngle re-poses the wings and lifts the bob (on_ground={on_ground})"
        );
        // The flap moves the wings [48, 96): both wings carry `zRot = ±(0.0873 + flapAngle)`.
        assert_ne!(
            rest.vertices[48..96],
            flapping.vertices[48..96],
            "the wings flap (on_ground={on_ground})"
        );
    }
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
    .with_on_ground(true)]);
    let flying = entity_model_mesh(&[EntityModelInstance::parrot(
        73,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with_on_ground(false)]);
    assert_eq!(standing.vertices.len(), flying.vertices.len());
    // body/tail/wings/head [0, 216) hold; only the legs [216, 264) differ.
    assert_eq!(
        standing.vertices[0..216],
        flying.vertices[0..216],
        "the body, tail, wings, and head match between standing and flying"
    );
    assert_ne!(
        standing.vertices[216..264],
        flying.vertices[216..264],
        "the flying parrot pitches its legs back (prepare(FLYING))"
    );
}

#[test]
fn parrot_sitting_mesh_differs_from_standing_and_flying() {
    // A perched parrot re-poses every part (raise + fold), so its mesh differs from both the standing
    // and the flying parrot while keeping the same vertex count. SITTING ignores the flap.
    let standing = entity_model_mesh(&[EntityModelInstance::parrot(
        74,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with_on_ground(true)]);
    let flying = entity_model_mesh(&[EntityModelInstance::parrot(
        75,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with_on_ground(false)]);
    let sitting = entity_model_mesh(&[EntityModelInstance::parrot(
        76,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with_parrot_sitting(true)
    .with_parrot_flap_angle(0.8)]);
    assert_eq!(standing.vertices.len(), sitting.vertices.len());
    assert_ne!(
        standing.vertices, sitting.vertices,
        "the sitting parrot perches lower with folded legs"
    );
    assert_ne!(
        flying.vertices, sitting.vertices,
        "the sitting parrot differs from the flying parrot"
    );
    // A sitting parrot ignores the flap entirely (its pose lives wholly in prepare(SITTING)).
    let sitting_no_flap = entity_model_mesh(&[EntityModelInstance::parrot(
        77,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )
    .with_parrot_sitting(true)]);
    assert_eq!(
        sitting.vertices, sitting_no_flap.vertices,
        "a sitting parrot does not flap"
    );
}

#[test]
fn parrot_mesh_uses_vanilla_body_layer_geometry() {
    // The body carries the body tint; the two beak halves carry the beak tint.
    let parrot = entity_model_mesh(&[EntityModelInstance::parrot(
        980,
        [0.0, 64.0, 0.0],
        0.0,
        ParrotModelVariant::RedBlue,
    )]);
    assert!(parrot
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PARROT_BODY, 1.0)));
    assert!(parrot
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PARROT_BEAK, 1.0)));
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
        assert_eq!(passes[0].texture, expected);
        assert_eq!(
            EntityModelKind::Parrot { variant }.vanilla_texture_ref(),
            Some(expected)
        );
        assert!(entity_model_texture_refs().contains(&expected));
    }

    // `Parrot.Variant.byId` folds out-of-range ids back to the `RED_BLUE` default.
    assert_eq!(ParrotModelVariant::from_id(0), ParrotModelVariant::RedBlue);
    assert_eq!(ParrotModelVariant::from_id(4), ParrotModelVariant::Gray);
    assert_eq!(ParrotModelVariant::from_id(99), ParrotModelVariant::RedBlue);
    assert_eq!(parrot_entity_texture_refs().len(), 5);

    let images: Vec<EntityModelTextureImage> = parrot_entity_texture_refs()
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
        let instance = EntityModelInstance::parrot(900, [0.0, 64.0, 0.0], 0.0, variant);
        let meshes = entity_model_textured_meshes(&[instance], &atlas);
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());
        assert_eq!(meshes.submissions.len(), 1);
        let submit = meshes.submissions[0];
        assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
        assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
        assert_eq!(submit.texture, parrot_texture_ref(variant));
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, entity_model_root_transform(instance));
        assert_eq!((submit.order, submit.submit_sequence), (0, 0));
        let mesh = &meshes.cutout;

        assert!(
            !mesh.vertices.is_empty(),
            "parrot {variant:?} emits geometry"
        );
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    }
}
