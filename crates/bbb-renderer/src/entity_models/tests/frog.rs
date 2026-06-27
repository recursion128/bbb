use super::*;

#[test]
fn frog_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `FrogModel.createBodyLayer` (atlas 48×48): the mesh root holds one `root` part at
    // `offset(0, 24, 0)` parenting `body` and the two legs.
    assert_eq!(FROG_ROOT_POSE.offset, [0.0, 24.0, 0.0]);

    // `body`: the `texOffs(3,1)` 7×3×9 box + the `texOffs(23,22)` 7×0×9 underside plane, parenting
    // head / tongue / two arms.
    assert_eq!(FROG_BODY_POSE.offset, [0.0, -2.0, 4.0]);
    assert_eq!(FROG_BODY_CUBES.len(), 2);
    assert_eq!(FROG_BODY_CUBES[0].min, [-3.5, -2.0, -8.0]);
    assert_eq!(FROG_BODY_CUBES[0].size, [7.0, 3.0, 9.0]);
    assert_eq!(FROG_BODY_CUBES[0].tex, [3.0, 1.0]);
    assert_eq!(FROG_BODY_CUBES[1].size, [7.0, 0.0, 9.0]);
    assert_eq!(FROG_BODY_CUBES[1].tex, [23.0, 22.0]);

    // `head` (`texOffs(23,13)` 7×0×9 plane + `texOffs(0,13)` 7×3×9 box) parents the `eyes` pivot.
    assert_eq!(FROG_HEAD_POSE.offset, [0.0, -2.0, -1.0]);
    assert_eq!(FROG_HEAD_CUBES.len(), 2);
    assert_eq!(FROG_HEAD_CUBES[0].tex, [23.0, 13.0]);
    assert_eq!(FROG_HEAD_CUBES[1].min, [-3.5, -2.0, -7.0]);
    assert_eq!(FROG_HEAD_CUBES[1].tex, [0.0, 13.0]);

    // The `eyes` empty pivot parents the two 3×2×3 eyes at ±X; the right eye `texOffs(0,0)`, the
    // left `texOffs(0,5)` (distinct UV regions, not mirrors).
    assert_eq!(FROG_EYES_POSE.offset, [-0.5, 0.0, 2.0]);
    assert_eq!(FROG_LEFT_EYE_POSE.offset, [-1.5, -3.0, -6.5]);
    assert_eq!(FROG_RIGHT_EYE_POSE.offset, [2.5, -3.0, -6.5]);
    assert_eq!(FROG_RIGHT_EYE_CUBES[0].min, [-1.5, -1.0, -1.5]);
    assert_eq!(FROG_RIGHT_EYE_CUBES[0].size, [3.0, 2.0, 3.0]);
    assert_eq!(FROG_RIGHT_EYE_CUBES[0].tex, [0.0, 0.0]);
    assert_eq!(FROG_LEFT_EYE_CUBES[0].size, [3.0, 2.0, 3.0]);
    assert_eq!(FROG_LEFT_EYE_CUBES[0].tex, [0.0, 5.0]);

    // The tongue plane `texOffs(17,13)` and the croaking pouch `texOffs(26,5)` (its `uv_size` is the
    // integer pre-deformation `addBox` dims `(7, 2, 3)`).
    assert_eq!(FROG_TONGUE_CUBES[0].tex, [17.0, 13.0]);
    assert_eq!(FROG_CROAKING_BODY_CUBES[0].tex, [26.0, 5.0]);
    assert_eq!(FROG_CROAKING_BODY_CUBES[0].uv_size, [7.0, 2.0, 3.0]);

    // The arms (2×3×3) each parent an 8×0×8 webbed hand; the hands differ in Z origin and UV. The
    // arms draw distinct UV regions: the left `texOffs(0,32)`, the right `texOffs(0,38)`.
    assert_eq!(FROG_LEFT_ARM_POSE.offset, [4.0, -1.0, -6.5]);
    assert_eq!(FROG_RIGHT_ARM_POSE.offset, [-4.0, -1.0, -6.5]);
    assert_eq!(FROG_LEFT_ARM_CUBES[0].tex, [0.0, 32.0]);
    assert_eq!(FROG_RIGHT_ARM_CUBES[0].tex, [0.0, 38.0]);
    assert_eq!(FROG_LEFT_HAND_CUBES[0].min, [-4.0, 0.01, -4.0]);
    assert_eq!(FROG_LEFT_HAND_CUBES[0].tex, [18.0, 40.0]);
    assert_eq!(FROG_RIGHT_HAND_CUBES[0].min, [-4.0, 0.01, -5.0]);
    assert_eq!(FROG_RIGHT_HAND_CUBES[0].tex, [2.0, 40.0]);

    // The legs (differ in X origin and UV: left `texOffs(14,25)`, right `texOffs(0,25)`) each parent
    // an 8×0×8 foot plane; the feet draw distinct UV regions (left `texOffs(2,32)`, right
    // `texOffs(18,32)`).
    assert_eq!(FROG_LEFT_LEG_POSE.offset, [3.5, -3.0, 4.0]);
    assert_eq!(FROG_LEFT_LEG_CUBES[0].min, [-1.0, 0.0, -2.0]);
    assert_eq!(FROG_LEFT_LEG_CUBES[0].tex, [14.0, 25.0]);
    assert_eq!(FROG_RIGHT_LEG_CUBES[0].min, [-2.0, 0.0, -2.0]);
    assert_eq!(FROG_RIGHT_LEG_CUBES[0].tex, [0.0, 25.0]);
    assert_eq!(FROG_LEFT_FOOT_CUBES[0].size, [8.0, 0.0, 8.0]);
    assert_eq!(FROG_LEFT_FOOT_CUBES[0].tex, [2.0, 32.0]);
    assert_eq!(FROG_RIGHT_FOOT_CUBES[0].tex, [18.0, 32.0]);
}

#[test]
fn frog_mesh_uses_vanilla_body_layer_geometry() {
    // 15 cubes → 90 faces / 360 vertices / 540 indices.
    let frog = entity_model_mesh(&[EntityModelInstance::frog(
        950,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )]);
    assert_eq!(frog.opaque_faces, 90);
    assert_eq!(frog.vertices.len(), 360);
    assert_eq!(frog.indices.len(), 540);
    // The body uses the frog body color; the eyes use their own tint.
    assert!(frog
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(FROG_BODY, 1.0)));
    assert!(frog
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(FROG_EYE, 1.0)));
}

#[test]
fn frog_walk_animation_matches_vanilla_definition() {
    // Vanilla `FrogAnimation.FROG_WALK`: 1.25 s looping, animating the body (rotation only), the two
    // arms, and the two legs (each rotation + position). 49 keyframes total.
    assert_eq!(FROG_WALK.length_seconds, 1.25);
    assert!(FROG_WALK.looping);
    assert_eq!(FROG_WALK.bones.len(), 5);
    let keyframes: usize = FROG_WALK
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 49);

    // The body has only a rotation channel: at t=0 it yaws +5° (`degreeVec(0, 5, 0)`).
    let (body_pos, body_rot) = sample_bone_offsets(&FROG_WALK, "body", 0.0, 1.0);
    assert_eq!(body_pos, [0.0, 0.0, 0.0]);
    assert_eq!(body_rot[0], 0.0);
    assert!((body_rot[1] - 5.0_f32.to_radians()).abs() < 1.0e-6);

    // Linear interpolation: the body rotation midway through [0, 0.2917] is the lerp of
    // `degreeVec(0, 5, 0)` and `degreeVec(-7.5, 0.33, 7.5)`.
    let (_, mid) = sample_bone_offsets(&FROG_WALK, "body", 0.2917 / 2.0, 1.0);
    let a = [0.0_f32.to_radians(), 5.0_f32.to_radians(), 0.0];
    let b = [
        (-7.5_f32).to_radians(),
        0.33_f32.to_radians(),
        7.5_f32.to_radians(),
    ];
    for axis in 0..3 {
        let expected = a[axis] + (b[axis] - a[axis]) * 0.5;
        assert!(
            (mid[axis] - expected).abs() < 1.0e-5,
            "axis {axis}: {} vs {expected}",
            mid[axis]
        );
    }

    // The target scale linearly attenuates the amplitude.
    let (_, half) = sample_bone_offsets(&FROG_WALK, "body", 0.0, 0.5);
    assert!((half[1] - 5.0_f32.to_radians() * 0.5).abs() < 1.0e-6);
}

#[test]
fn frog_swim_animation_matches_vanilla_definition() {
    // Vanilla `FrogAnimation.FROG_SWIM`: 1.04167 s looping, animating the body (rotation only), the
    // two arms, and the two legs (each rotation + position). 52 keyframes total.
    assert_eq!(FROG_SWIM.length_seconds, 1.04167);
    assert!(FROG_SWIM.looping);
    assert_eq!(FROG_SWIM.bones.len(), 5);
    let bones: Vec<&str> = FROG_SWIM.bones.iter().map(|bone| bone.bone).collect();
    assert_eq!(
        bones,
        ["body", "left_arm", "right_arm", "left_leg", "right_leg"]
    );
    let keyframes: usize = FROG_SWIM
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 52);

    // Spot-check vanilla frames: body pitches +10° at t=0.3333, the left arm starts at
    // `(90°, 22.5°, 0°)`, and `posVec(0, -0.64, 2)` flips Y to a `+0.64` offset.
    let (_, body_rot) = sample_bone_offsets(&FROG_SWIM, "body", 0.3333, 1.0);
    assert!((body_rot[0] - 10.0_f32.to_radians()).abs() < 1.0e-6);
    let (left_arm_pos, left_arm_rot) = sample_bone_offsets(&FROG_SWIM, "left_arm", 0.0, 1.0);
    assert!((left_arm_rot[0] - 90.0_f32.to_radians()).abs() < 1.0e-6);
    assert!((left_arm_rot[1] - 22.5_f32.to_radians()).abs() < 1.0e-6);
    assert_eq!(left_arm_pos, [0.0, 0.64, 2.0]);
}

#[test]
fn frog_walk_moves_the_limbs_off_the_walk_cycle() {
    // A still frog (walk speed 0) samples the cycle at amplitude 0, collapsing to the bind pose; a
    // walking frog samples the FROG_WALK offsets, animating the body, arms, and legs. The vertex
    // count is preserved (no parts appear or vanish).
    let still = entity_model_mesh(&[EntityModelInstance::frog(
        70,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )]);
    let walking = entity_model_mesh(&[EntityModelInstance::frog(
        71,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_walk_animation(6.0, 1.0)]);
    assert_eq!(still.vertices.len(), walking.vertices.len());
    assert_ne!(
        still.vertices, walking.vertices,
        "the walking frog animates its limbs"
    );

    // The still frog equals the plain bind-pose emit (amplitude 0 ⇒ no offsets).
    let bind = entity_model_mesh(&[EntityModelInstance::frog(
        72,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )]);
    assert_eq!(still.vertices, bind.vertices);
}

#[test]
fn frog_in_water_uses_swim_walk_cycle() {
    // Vanilla `FrogModel.setupAnim` chooses `FROG_SWIM.applyWalk(..., 1.0, 2.5)` when
    // `FrogRenderState.isSwimming` is true, otherwise `FROG_WALK.applyWalk(..., 1.5, 2.5)`.
    let walking = EntityModelInstance::frog(73, [0.0, 64.0, 0.0], 0.0, FrogModelVariant::Temperate)
        .with_walk_animation(0.0, 1.0);
    let dry = entity_model_mesh(&[walking.clone()]);
    let wet = entity_model_mesh(&[walking.with_in_water(true)]);
    assert_eq!(dry.vertices.len(), wet.vertices.len());
    assert_ne!(
        dry.vertices, wet.vertices,
        "an in-water moving frog samples FROG_SWIM instead of FROG_WALK"
    );

    // At zero walk speed, both applyWalk calls scale to amplitude 0 and collapse to the bind pose.
    let still = EntityModelInstance::frog(74, [0.0, 64.0, 0.0], 0.0, FrogModelVariant::Temperate);
    assert_eq!(
        entity_model_mesh(&[still.clone()]).vertices,
        entity_model_mesh(&[still.with_in_water(true)]).vertices
    );
}

#[test]
fn frog_croak_animation_matches_vanilla_definition() {
    // Vanilla `FrogAnimation.FROG_CROAK`: 3.0s, NOT looping, one `croaking_body` bone with a
    // POSITION channel (6 keyframes) and a SCALE channel (16 keyframes).
    assert_eq!(FROG_CROAK.length_seconds, 3.0);
    assert!(!FROG_CROAK.looping);
    assert_eq!(FROG_CROAK.bones.len(), 1);
    assert_eq!(FROG_CROAK.bones[0].bone, "croaking_body");
    let keyframes: usize = FROG_CROAK.bones[0]
        .channels
        .iter()
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 22);

    // At t=0 the pouch is at its bind position and collapsed (`scaleVec(0, 0, 0)` ⇒ scale `[0,0,0]`).
    let (pos0, _, scale0) = sample_bone_offsets_with_scale(&FROG_CROAK, "croaking_body", 0.0, 1.0);
    assert_eq!(pos0, [0.0, 0.0, 0.0]);
    assert_eq!(keyframe_animated_scale(scale0), [0.0, 0.0, 0.0]);

    // Once inflated the pouch lifts `+1` y (`posVec` negates y, so the offset is `-1`) and at the
    // `0.5417` keyframe puffs to `scaleVec(1.3, 2.1, 1.6)` ⇒ scale `[1.3, 2.1, 1.6]`.
    let (pos_up, _, _) = sample_bone_offsets_with_scale(&FROG_CROAK, "croaking_body", 1.0, 1.0);
    assert!((pos_up[1] - -1.0).abs() < 1.0e-6);
    let (_, _, puff) = sample_bone_offsets_with_scale(&FROG_CROAK, "croaking_body", 0.5417, 1.0);
    let puffed = keyframe_animated_scale(puff);
    assert!((puffed[0] - 1.3).abs() < 1.0e-4);
    assert!((puffed[1] - 2.1).abs() < 1.0e-4);
    assert!((puffed[2] - 1.6).abs() < 1.0e-4);
}

#[test]
fn frog_croak_shows_and_poses_the_pouch_off_the_hidden_bind_pose() {
    // A non-croaking frog (`-1.0` sentinel) hides the `croaking_body` pouch, so the mesh is the 15
    // visible cubes (90 faces) and matches the plain bind pose.
    let resting = entity_model_mesh(&[EntityModelInstance::frog(
        950,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )]);
    assert_eq!(resting.opaque_faces, 90);

    // A croaking frog shows the pouch, adding one cube (6 faces / 24 vertices), and re-poses it.
    let croaking = entity_model_mesh(&[EntityModelInstance::frog(
        951,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_croak_seconds(0.5417)]);
    assert_eq!(
        croaking.opaque_faces, 96,
        "the croaking frog reveals the puffed pouch cube"
    );
    assert_eq!(croaking.vertices.len(), resting.vertices.len() + 24);

    // Sampling the animation at a different time re-poses the pouch (the scale puffs and collapses).
    let early = entity_model_mesh(&[EntityModelInstance::frog(
        952,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_croak_seconds(0.4167)]);
    assert_eq!(early.opaque_faces, 96);
    assert_ne!(
        early.vertices, croaking.vertices,
        "the pouch puffs out as the croak animation advances"
    );

    // An explicit `-1.0` (the not-croaking sentinel) leaves the pouch hidden, equal to the rest mesh.
    let cleared = entity_model_mesh(&[EntityModelInstance::frog(
        953,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_croak_seconds(-1.0)]);
    assert_eq!(cleared.vertices, resting.vertices);
}

#[test]
fn frog_tongue_animation_matches_vanilla_definition() {
    // Vanilla `FrogAnimation.FROG_TONGUE`: 0.5s, NOT looping, two bones — `head` (ROTATION 4kf +
    // SCALE 4kf) and `tongue` (ROTATION 4kf + SCALE 3kf).
    assert_eq!(FROG_TONGUE.length_seconds, 0.5);
    assert!(!FROG_TONGUE.looping);
    assert_eq!(FROG_TONGUE.bones.len(), 2);
    assert_eq!(FROG_TONGUE.bones[0].bone, "head");
    assert_eq!(FROG_TONGUE.bones[1].bone, "tongue");

    // At the `0.1667` keyframe the tongue lashes forward (`scaleVec(0.5, 1, 5)` ⇒ x squashes to 0.5,
    // z extends to 5×).
    let (_, _, lash) = sample_bone_offsets_with_scale(&FROG_TONGUE, "tongue", 0.1667, 1.0);
    let lashed = keyframe_animated_scale(lash);
    assert!(
        (lashed[0] - 0.5).abs() < 1.0e-4,
        "x squashes to 0.5: {lashed:?}"
    );
    assert!(
        (lashed[2] - 5.0).abs() < 1.0e-4,
        "z extends to 5: {lashed:?}"
    );

    // Mid-lash the head dips to `-60°` xRot (`degreeVec`).
    let (_, head_rot, _) = sample_bone_offsets_with_scale(&FROG_TONGUE, "head", 0.25, 1.0);
    assert!(
        (head_rot[0] - (-60.0_f32).to_radians()).abs() < 1.0e-4,
        "the head dips: {head_rot:?}"
    );
}

#[test]
fn frog_tongue_lashes_the_head_and_tongue_off_the_rest_pose() {
    // A non-tongue-using frog (`-1.0` sentinel) holds the head/tongue at the bind pose.
    let resting = entity_model_mesh(&[EntityModelInstance::frog(
        954,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )]);

    // Mid-lash, the head dips and the tongue extends forward, re-posing those parts (no cubes
    // added/removed — the tongue is one of the 15 base cubes).
    let lashing = entity_model_mesh(&[EntityModelInstance::frog(
        955,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_tongue_seconds(0.1667)]);
    assert_eq!(resting.vertices.len(), lashing.vertices.len());
    assert_ne!(
        resting.vertices, lashing.vertices,
        "the head dips and the tongue lashes forward"
    );

    // An explicit `-1.0` (the not-using-tongue sentinel) equals the rest mesh.
    let cleared = entity_model_mesh(&[EntityModelInstance::frog(
        956,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_tongue_seconds(-1.0)]);
    assert_eq!(cleared.vertices, resting.vertices);
}

#[test]
fn frog_jump_animation_matches_vanilla_definition() {
    // Vanilla `FrogAnimation.FROG_JUMP`: 0.5s, NOT looping, five bones (body, the two arms, the two
    // legs), each with a ROTATION and a POSITION channel of two constant keyframes (10 channels, 20
    // keyframes total).
    assert_eq!(FROG_JUMP.length_seconds, 0.5);
    assert!(!FROG_JUMP.looping);
    assert_eq!(FROG_JUMP.bones.len(), 5);
    let bones: Vec<&str> = FROG_JUMP.bones.iter().map(|bone| bone.bone).collect();
    assert_eq!(
        bones,
        ["body", "left_arm", "right_arm", "left_leg", "right_leg"]
    );
    let keyframes: usize = FROG_JUMP
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 20);

    // The static hold pose: the body tips back `-22.5°`, the arms tuck back `-56.14°` and lift `+1`
    // y (`posVec` negates y, so the offset is `-1`), and the legs cock `45°`. Sampling anywhere in
    // `[0, 0.5]` returns the same constant pose.
    const RAD: f32 = std::f32::consts::PI / 180.0;
    let (body_pos, body_rot) = sample_bone_offsets(&FROG_JUMP, "body", 0.0, 1.0);
    assert_eq!(body_pos, [0.0, 0.0, 0.0]);
    assert!((body_rot[0] - -22.5 * RAD).abs() < 1.0e-6);
    let (arm_pos, arm_rot) = sample_bone_offsets(&FROG_JUMP, "left_arm", 0.5, 1.0);
    assert!((arm_pos[1] - -1.0).abs() < 1.0e-6);
    assert!((arm_rot[0] - -56.14 * RAD).abs() < 1.0e-4);
    let (_, leg_rot) = sample_bone_offsets(&FROG_JUMP, "right_leg", 0.25, 1.0);
    assert!((leg_rot[0] - 45.0 * RAD).abs() < 1.0e-6);
}

#[test]
fn frog_jump_reposes_the_limbs_off_the_bind_pose() {
    // A non-jumping frog (`-1.0` sentinel) renders the plain bind/walk-rest pose.
    let resting = entity_model_mesh(&[EntityModelInstance::frog(
        960,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )]);

    // A long-jumping frog tips the body and tucks the limbs into the hold pose: the same 15 cubes
    // (no pouch), but re-posed off the bind pose.
    let jumping = entity_model_mesh(&[EntityModelInstance::frog(
        961,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_jump_seconds(0.0)]);
    assert_eq!(jumping.opaque_faces, resting.opaque_faces);
    assert_eq!(jumping.vertices.len(), resting.vertices.len());
    assert_ne!(
        jumping.vertices, resting.vertices,
        "the long-jumping frog re-poses its body, arms, and legs"
    );

    // The hold pose is constant across the 0.5s window, so a later sample matches.
    let later = entity_model_mesh(&[EntityModelInstance::frog(
        962,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_jump_seconds(0.25)]);
    assert_eq!(later.vertices, jumping.vertices);

    // An explicit `-1.0` (the not-jumping sentinel) leaves the frog at the bind pose.
    let cleared = entity_model_mesh(&[EntityModelInstance::frog(
        963,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_jump_seconds(-1.0)]);
    assert_eq!(cleared.vertices, resting.vertices);
}

#[test]
fn frog_idle_water_animation_matches_vanilla_definition() {
    // Vanilla `FrogAnimation.FROG_IDLE_WATER`: 3.0s, LOOPING, five bones (body, the two arms, the
    // two legs). `body` has only a ROTATION channel; the four limbs each have a ROTATION and a
    // POSITION channel — nine channels, each with three CATMULLROM keyframes (27 keyframes total).
    assert_eq!(FROG_IDLE_WATER.length_seconds, 3.0);
    assert!(FROG_IDLE_WATER.looping);
    assert_eq!(FROG_IDLE_WATER.bones.len(), 5);
    let bones: Vec<&str> = FROG_IDLE_WATER.bones.iter().map(|bone| bone.bone).collect();
    assert_eq!(
        bones,
        ["body", "left_arm", "right_arm", "left_leg", "right_leg"]
    );
    let keyframes: usize = FROG_IDLE_WATER
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 27);

    // Spot-check the start frame (`t = 0`): the body holds at zero, the left arm splays `-22.5°` z
    // and offsets `-1` x, and the right leg swings out `22.5°` x / `22.5°` y and sits `+1` z.
    const RAD: f32 = std::f32::consts::PI / 180.0;
    let (body_pos, body_rot) = sample_bone_offsets(&FROG_IDLE_WATER, "body", 0.0, 1.0);
    assert_eq!(body_pos, [0.0, 0.0, 0.0]);
    assert_eq!(body_rot, [0.0, 0.0, 0.0]);
    let (arm_pos, arm_rot) = sample_bone_offsets(&FROG_IDLE_WATER, "left_arm", 0.0, 1.0);
    assert!((arm_pos[0] - -1.0).abs() < 1.0e-6);
    assert!((arm_rot[2] - -22.5 * RAD).abs() < 1.0e-6);
    let (leg_pos, leg_rot) = sample_bone_offsets(&FROG_IDLE_WATER, "right_leg", 0.0, 1.0);
    assert!((leg_pos[2] - 1.0).abs() < 1.0e-6);
    assert!((leg_rot[0] - 22.5 * RAD).abs() < 1.0e-6);
    assert!((leg_rot[1] - 22.5 * RAD).abs() < 1.0e-6);
}

#[test]
fn frog_idle_water_reposes_the_limbs_off_the_bind_pose() {
    // A dry/moving frog (`-1.0` sentinel) renders the plain bind/walk-rest pose (no swim-idle).
    let resting = entity_model_mesh(&[EntityModelInstance::frog(
        970,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )]);

    // An in-water idling frog hovers its limbs into the looping idle pose: the same 15 cubes (no
    // pouch), but re-posed off the bind pose. Even at `t = 0` the arms/legs carry a nonzero offset.
    let idling = entity_model_mesh(&[EntityModelInstance::frog(
        971,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_swim_idle_seconds(0.0)]);
    assert_eq!(idling.opaque_faces, resting.opaque_faces);
    assert_eq!(idling.vertices.len(), resting.vertices.len());
    assert_ne!(
        idling.vertices, resting.vertices,
        "the in-water idling frog hovers its body, arms, and legs"
    );

    // A different phase mid-cycle re-poses the hover again (the sway tracks the elapsed seconds).
    let idling_mid = entity_model_mesh(&[EntityModelInstance::frog(
        972,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_swim_idle_seconds(1.5)]);
    assert_ne!(idling.vertices, idling_mid.vertices);

    // The animation loops at 3.0s, so sampling one full period later returns the start pose.
    let idling_wrapped = entity_model_mesh(&[EntityModelInstance::frog(
        973,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_swim_idle_seconds(3.0)]);
    assert_eq!(idling_wrapped.vertices, idling.vertices);

    // An explicit `-1.0` (the dry/moving sentinel) leaves the frog at the bind pose.
    let cleared = entity_model_mesh(&[EntityModelInstance::frog(
        974,
        [0.0, 64.0, 0.0],
        0.0,
        FrogModelVariant::Temperate,
    )
    .with_frog_swim_idle_seconds(-1.0)]);
    assert_eq!(cleared.vertices, resting.vertices);
}

#[test]
fn frog_textured_render_matches_vanilla_renderer() {
    // Each of the three temperature variants binds its own base texture (atlas 48×48); all share
    // one `FrogModel` geometry. The three refs match `FrogRenderer.getTextureLocation`'s asset paths.
    for (variant, texture) in [
        (FrogModelVariant::Temperate, FROG_TEMPERATE_TEXTURE_REF),
        (FrogModelVariant::Warm, FROG_WARM_TEXTURE_REF),
        (FrogModelVariant::Cold, FROG_COLD_TEXTURE_REF),
    ] {
        let passes = frog_textured_layer_passes(variant);
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].kind, EntityModelLayerKind::FrogBase);
        assert_eq!(passes[0].model_layer, MODEL_LAYER_FROG);
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
        assert_eq!(
            EntityModelKind::Frog { variant }.vanilla_texture_ref(),
            Some(texture)
        );
        assert!(entity_model_texture_refs().contains(&texture));
    }
    assert_eq!(
        frog_entity_texture_refs(),
        &[
            FROG_TEMPERATE_TEXTURE_REF,
            FROG_WARM_TEXTURE_REF,
            FROG_COLD_TEXTURE_REF
        ]
    );

    let images: Vec<EntityModelTextureImage> = frog_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::frog(950, [0.0, 64.0, 0.0], 0.0, FrogModelVariant::Warm)
        .with_light_coords((8_u32 << 4) | (12_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.texture, FROG_WARM_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, entity_model_root_transform(instance));
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, instance.render_state.overlay_coords());
    assert_ne!(submit.overlay, [0.0, 10.0]);
    let mesh = &meshes.cutout;

    assert!(
        !mesh.vertices.is_empty(),
        "the frog emits textured geometry"
    );
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));
}
