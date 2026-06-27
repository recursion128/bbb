use super::*;

#[test]
fn warden_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `WardenModel.createBodyLayer` (atlas 128×128): the mesh root holds one `bone` part
    // at `offset(0, 24, 0)` parenting the body and the two legs.
    assert_eq!(WARDEN_BONE_POSE.offset, [0.0, 24.0, 0.0]);

    // `body` (offset (0, -21, 0)): one 18×21×11 box (`texOffs(0,0)`) parenting two ribcages, the
    // head, and arms.
    assert_eq!(WARDEN_BODY_POSE.offset, [0.0, -21.0, 0.0]);
    assert_eq!(WARDEN_BODY_CUBES.len(), 1);
    assert_eq!(WARDEN_BODY_CUBES[0].min, [-9.0, -13.0, -4.0]);
    assert_eq!(WARDEN_BODY_CUBES[0].size, [18.0, 21.0, 11.0]);
    assert_eq!(WARDEN_BODY_CUBES[0].tex, [0.0, 0.0]);

    // The two 9×21×0 ribcage planes share `texOffs(90,11)`; the left is the vanilla `mirror()`.
    assert_eq!(WARDEN_RIGHT_RIBCAGE_POSE.offset, [-7.0, -2.0, -4.0]);
    assert_eq!(WARDEN_RIGHT_RIBCAGE_CUBES[0].min, [-2.0, -11.0, -0.1]);
    assert_eq!(WARDEN_RIGHT_RIBCAGE_CUBES[0].tex, [90.0, 11.0]);
    assert!(!WARDEN_RIGHT_RIBCAGE_CUBES[0].mirror);
    assert_eq!(WARDEN_LEFT_RIBCAGE_POSE.offset, [7.0, -2.0, -4.0]);
    assert_eq!(WARDEN_LEFT_RIBCAGE_CUBES[0].min, [-7.0, -11.0, -0.1]);
    assert_eq!(WARDEN_LEFT_RIBCAGE_CUBES[0].tex, [90.0, 11.0]);
    assert!(WARDEN_LEFT_RIBCAGE_CUBES[0].mirror);
    assert_eq!(WARDEN_RIGHT_RIBCAGE_CUBES[0].size, [9.0, 21.0, 0.0]);

    // `head` (16×16×10, `texOffs(0,32)`) parents the two 16×16×0 tendril planes (`texOffs(52,32)`
    // and `texOffs(58,0)` — distinct UV regions, not mirrors).
    assert_eq!(WARDEN_HEAD_POSE.offset, [0.0, -13.0, 0.0]);
    assert_eq!(WARDEN_HEAD_CUBES[0].size, [16.0, 16.0, 10.0]);
    assert_eq!(WARDEN_HEAD_CUBES[0].tex, [0.0, 32.0]);
    assert_eq!(WARDEN_RIGHT_TENDRIL_POSE.offset, [-8.0, -12.0, 0.0]);
    assert_eq!(WARDEN_RIGHT_TENDRIL_CUBES[0].min, [-16.0, -13.0, 0.0]);
    assert_eq!(WARDEN_RIGHT_TENDRIL_CUBES[0].tex, [52.0, 32.0]);
    assert_eq!(WARDEN_LEFT_TENDRIL_CUBES[0].min, [0.0, -13.0, 0.0]);
    assert_eq!(WARDEN_LEFT_TENDRIL_CUBES[0].size, [16.0, 16.0, 0.0]);
    assert_eq!(WARDEN_LEFT_TENDRIL_CUBES[0].tex, [58.0, 0.0]);

    // The two 8×28×8 arms add the identical box but draw distinct UV regions: the right
    // `texOffs(44,50)`, the left `texOffs(0,58)` (not mirrors).
    assert_eq!(WARDEN_RIGHT_ARM_POSE.offset, [-13.0, -13.0, 1.0]);
    assert_eq!(WARDEN_LEFT_ARM_POSE.offset, [13.0, -13.0, 1.0]);
    assert_eq!(WARDEN_RIGHT_ARM_CUBES[0].size, [8.0, 28.0, 8.0]);
    assert_eq!(WARDEN_RIGHT_ARM_CUBES[0].tex, [44.0, 50.0]);
    assert!(!WARDEN_RIGHT_ARM_CUBES[0].mirror);
    assert_eq!(WARDEN_LEFT_ARM_CUBES[0].size, [8.0, 28.0, 8.0]);
    assert_eq!(WARDEN_LEFT_ARM_CUBES[0].tex, [0.0, 58.0]);
    assert!(!WARDEN_LEFT_ARM_CUBES[0].mirror);

    // The two 6×13×6 legs differ in X origin and UV (right `texOffs(76,48)`, left `texOffs(76,76)`).
    assert_eq!(WARDEN_RIGHT_LEG_POSE.offset, [-5.9, -13.0, 0.0]);
    assert_eq!(WARDEN_RIGHT_LEG_CUBES[0].min, [-3.1, 0.0, -3.0]);
    assert_eq!(WARDEN_RIGHT_LEG_CUBES[0].tex, [76.0, 48.0]);
    assert_eq!(WARDEN_LEFT_LEG_POSE.offset, [5.9, -13.0, 0.0]);
    assert_eq!(WARDEN_LEFT_LEG_CUBES[0].min, [-2.9, 0.0, -3.0]);
    assert_eq!(WARDEN_LEFT_LEG_CUBES[0].tex, [76.0, 76.0]);
}

#[test]
fn warden_mesh_uses_vanilla_body_layer_geometry() {
    // 10 cubes → 60 faces / 240 vertices / 360 indices; the tendrils carry their own cyan tint.
    let warden = entity_model_mesh(&[EntityModelInstance::warden(920, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(warden.opaque_faces, 60);
    assert_eq!(warden.vertices.len(), 240);
    assert_eq!(warden.indices.len(), 360);
    assert!(warden
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WARDEN_BODY, 1.0)));
    assert!(warden
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WARDEN_TENDRIL, 1.0)));
}

#[test]
fn warden_head_look_turns_only_the_nested_head_subtree() {
    // Vanilla `WardenModel.animateHeadLookTarget` sets `head.xRot/yRot` from the look angles. The
    // head is `bone.body.head` (nested three deep), so the head box and its two tendrils turn while
    // the body, ribcages, arms, and legs hold. Depth-first emit order: the body and two ribcages
    // `[0, 72)`, the head and its two tendrils `[72, 144)`, then the two arms and two legs
    // `[144, 240)`.
    let rest = EntityModelInstance::warden(921, [0.0, 64.0, 0.0], 0.0);
    let looked = rest.with_head_look(40.0, -30.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let looked_mesh = entity_model_mesh(&[looked]);
    assert_eq!(rest_mesh.vertices.len(), looked_mesh.vertices.len());
    assert_eq!(
        rest_mesh.vertices[..72],
        looked_mesh.vertices[..72],
        "the body and ribcages stay put"
    );
    assert_ne!(
        rest_mesh.vertices[72..144],
        looked_mesh.vertices[72..144],
        "the head and its tendrils turn"
    );
    assert_eq!(
        rest_mesh.vertices[144..],
        looked_mesh.vertices[144..],
        "the arms and legs stay put"
    );
}

#[test]
fn warden_idle_and_head_pose_match_vanilla_setup_anim() {
    let age = 50.0_f32;
    let s = age * 0.1;

    // `animateIdlePose` body roll: xRot += 0.025·cos(s), zRot += 0.025·sin(s), yRot untouched.
    let body_bind = WARDEN_BODY_POSE;
    let body = warden_idle_body_pose(body_bind, age);
    assert!((body.rotation[0] - (body_bind.rotation[0] + 0.025 * s.cos())).abs() < 1.0e-6);
    assert_eq!(body.rotation[1], body_bind.rotation[1]);
    assert!((body.rotation[2] - (body_bind.rotation[2] + 0.025 * s.sin())).abs() < 1.0e-6);
    assert_eq!(body.offset, body_bind.offset);

    // The head pose: the look sets xRot/yRot, then the idle roll adds xRot += 0.06·sin(s) and
    // zRot += 0.06·cos(s).
    let head_bind = WARDEN_HEAD_POSE;
    let head = warden_head_pose(head_bind, 40.0, -30.0, age);
    assert!((head.rotation[0] - ((-30.0_f32).to_radians() + 0.06 * s.sin())).abs() < 1.0e-6);
    assert!((head.rotation[1] - 40.0_f32.to_radians()).abs() < 1.0e-6);
    assert!((head.rotation[2] - (head_bind.rotation[2] + 0.06 * s.cos())).abs() < 1.0e-6);
    assert_eq!(head.offset, head_bind.offset);
}

#[test]
fn warden_idle_wobble_sways_the_body_subtree_off_age() {
    // `animateIdlePose` is always on, so advancing `ageInTicks` rolls the body — carrying its whole
    // subtree (ribcages, head, tendrils, arms) — plus the head's own extra roll. Only the legs,
    // hung off the `bone` rather than the body, hold. Layout: body subtree `[0, 192)`, the two legs
    // `[192, 240)`.
    let rest = EntityModelInstance::warden(922, [0.0, 64.0, 0.0], 0.0);
    let rest_mesh = entity_model_mesh(&[rest]);
    let aged_mesh = entity_model_mesh(&[rest.with_age_in_ticks(50.0)]);
    assert_eq!(rest_mesh.vertices.len(), aged_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices[..192],
        aged_mesh.vertices[..192],
        "the body subtree wobbles with age"
    );
    assert_eq!(
        rest_mesh.vertices[192..],
        aged_mesh.vertices[192..],
        "the legs hold"
    );
}

#[test]
fn warden_walk_pose_matches_vanilla_animate_walk() {
    use std::f32::consts::{FRAC_PI_2, PI};
    // Vanilla `WardenModel.animateWalk(animationPos, animationSpeed)`:
    //   speedModifier        = min(0.5, 3·animationSpeed)
    //   adjustedPos          = animationPos·0.8662
    //   speedModifierWithMin = min(0.35, speedModifier)
    //   head.zRot += 0.3·sin(adjustedPos)·speedModifier
    //   head.xRot += 1.2·cos(adjustedPos + π/2)·speedModifierWithMin
    //   body.zRot  = 0.1·sin(adjustedPos)·speedModifier
    //   body.xRot  = 1.0·cos(adjustedPos)·speedModifierWithMin
    //   leftLeg.xRot  = 1.0·cos(adjustedPos)·speedModifier
    //   rightLeg.xRot = 1.0·cos(adjustedPos + π)·speedModifier
    //   leftArm.xRot  = -(0.8·cos(adjustedPos)·speedModifier)
    //   rightArm.xRot = -(0.8·sin(adjustedPos)·speedModifier)
    // `animationSpeed = 0.2` makes `3·0.2 = 0.6` clamp to `speedModifier = 0.5`, which in turn clamps
    // to `speedModifierWithMin = 0.35`, exercising both `min`s.
    let walk_pos = 3.0_f32;
    let walk_speed = 0.2_f32;
    let speed = (3.0 * walk_speed).min(0.5);
    let speed_with_min = speed.min(0.35);
    let adjusted = walk_pos * 0.8662;
    let cos = adjusted.cos();
    let sin = adjusted.sin();
    let pose = warden_walk_pose(walk_pos, walk_speed);
    assert!((pose.head_x_rot - 1.2 * (adjusted + FRAC_PI_2).cos() * speed_with_min).abs() < 1.0e-6);
    assert!((pose.head_z_rot - 0.3 * sin * speed).abs() < 1.0e-6);
    assert!((pose.body_x_rot - cos * speed_with_min).abs() < 1.0e-6);
    assert!((pose.body_z_rot - 0.1 * sin * speed).abs() < 1.0e-6);
    assert!((pose.left_leg_x_rot - cos * speed).abs() < 1.0e-6);
    assert!((pose.right_leg_x_rot - (adjusted + PI).cos() * speed).abs() < 1.0e-6);
    assert!((pose.left_arm_x_rot - -(0.8 * cos * speed)).abs() < 1.0e-6);
    assert!((pose.right_arm_x_rot - -(0.8 * sin * speed)).abs() < 1.0e-6);

    // A standing warden (`animationSpeed = 0`) zeroes every term, so it adds nothing on top of the
    // look/idle pose.
    let still = warden_walk_pose(7.5, 0.0);
    assert_eq!(
        [
            still.head_x_rot,
            still.head_z_rot,
            still.body_x_rot,
            still.body_z_rot,
            still.left_leg_x_rot,
            still.right_leg_x_rot,
            still.left_arm_x_rot,
            still.right_arm_x_rot
        ],
        [0.0; 8]
    );
}

#[test]
fn warden_walk_swings_the_legs_and_arms_off_walk_state() {
    // `animateWalk` is the only `setupAnim` motion that reaches the legs (hung off the `bone`, not
    // the body), so a walking warden re-poses the legs `[192, 240)` the idle wobble leaves alone, and
    // also swings the arms `[144, 192)`. A standing warden (`walkSpeed = 0`) adds nothing, matching
    // the look/idle-only rest. All three share the default `ageInTicks = 0`, so the idle wobble is
    // identical and only the walk differs.
    let rest = EntityModelInstance::warden(923, [0.0, 64.0, 0.0], 0.0);
    let standing = rest.with_walk_animation(0.0, 0.0);
    let walking = rest.with_walk_animation(4.0, 0.8);
    let rest_mesh = entity_model_mesh(&[rest]);
    let standing_mesh = entity_model_mesh(&[standing]);
    let walking_mesh = entity_model_mesh(&[walking]);
    assert_eq!(
        rest_mesh.vertices, standing_mesh.vertices,
        "walkSpeed 0 adds no walk"
    );
    assert_ne!(
        rest_mesh.vertices[144..192],
        walking_mesh.vertices[144..192],
        "the arms swing with the walk"
    );
    assert_ne!(
        rest_mesh.vertices[192..],
        walking_mesh.vertices[192..],
        "the legs swing with the walk"
    );
}

#[test]
fn warden_tendril_x_rot_matches_vanilla_animate_tendrils() {
    // Vanilla `WardenModel.animateTendrils`:
    //   tendrilXRot = tendrilAnimation · (float)(cos(ageInTicks · 2.25) · π · 0.1)
    // The `cos·π·0.1` factor is evaluated in double precision before the `(float)` cast.
    let age = 7.0_f32;
    let tendril = 0.6_f32;
    let expected = tendril * ((age as f64 * 2.25).cos() * std::f64::consts::PI * 0.1) as f32;
    assert!((warden_tendril_x_rot(tendril, age) - expected).abs() < 1.0e-7);
    // A resting warden (`tendrilAnimation = 0`) adds no sway, whatever the age.
    assert_eq!(warden_tendril_x_rot(0.0, age), 0.0);
}

#[test]
fn warden_tendrils_sway_with_the_tendril_pulse() {
    // Vanilla `WardenModel.animateTendrils` swings the two head tendrils' `xRot` by
    // `tendrilAnimation·cos(ageInTicks·2.25)·π·0.1` (left `+`, right `−`). The tendrils are the
    // head's two children, vertices `[96, 144)`; the head box `[72, 96)`, the body/ribcages
    // `[0, 72)`, and the arms/legs `[144, 240)` hold when the look/idle/walk are unchanged. A
    // resting warden (`tendrilAnimation = 0`) leaves the tendrils at bind.
    let rest = EntityModelInstance::warden(930, [0.0, 64.0, 0.0], 0.0);
    let pulsing = rest.with_tendril_animation(0.7);
    let rest_mesh = entity_model_mesh(&[rest]);
    let pulsing_mesh = entity_model_mesh(&[pulsing]);
    assert_eq!(rest_mesh.vertices.len(), pulsing_mesh.vertices.len());
    assert_ne!(
        rest_mesh.vertices[96..144],
        pulsing_mesh.vertices[96..144],
        "the two tendrils sway with the pulse"
    );
    assert_eq!(
        rest_mesh.vertices[..96],
        pulsing_mesh.vertices[..96],
        "the body, ribcages, and head box hold"
    );
    assert_eq!(
        rest_mesh.vertices[144..],
        pulsing_mesh.vertices[144..],
        "the arms and legs hold"
    );
}

#[test]
fn warden_combat_animations_match_vanilla_definitions() {
    // Vanilla `WardenAnimation` lengths / looping flags for the four triggered combat one-shots
    // `WardenModel.setupAnim` applies (all NOT looping — they hold their final frame at the end).
    assert_eq!(WARDEN_ATTACK.length_seconds, 0.33333);
    assert!(!WARDEN_ATTACK.looping);
    assert_eq!(WARDEN_ATTACK.bones.len(), 4);
    assert_eq!(WARDEN_SONIC_BOOM.length_seconds, 3.0);
    assert!(!WARDEN_SONIC_BOOM.looping);
    assert_eq!(WARDEN_SONIC_BOOM.bones.len(), 6);
    assert_eq!(WARDEN_ROAR.length_seconds, 4.2);
    assert!(!WARDEN_ROAR.looping);
    assert_eq!(WARDEN_ROAR.bones.len(), 4);
    assert_eq!(WARDEN_SNIFF.length_seconds, 4.16);
    assert!(!WARDEN_SNIFF.looping);
    assert_eq!(WARDEN_SNIFF.bones.len(), 4);

    // The roar rears the `body` to `degreeVec(47.5, 0, 0)` at its `3.0` keyframe.
    let (_, roar_body) = sample_bone_offsets(&WARDEN_ROAR, "body", 3.0, 1.0);
    assert!((roar_body[0] - 47.5_f32.to_radians()).abs() < 1.0e-4);

    // The sonic boom fans the `right_ribcage` open to `degreeVec(0, 125, 0)` at its `2.5` keyframe
    // (`posVec`/`degreeVec` axes mirror vanilla).
    let (_, boom_ribcage) = sample_bone_offsets(&WARDEN_SONIC_BOOM, "right_ribcage", 2.5, 1.0);
    assert!((boom_ribcage[1] - 125.0_f32.to_radians()).abs() < 1.0e-4);

    // The attack dips the `body` `-1` y / `-2` z (`posVec` negates y, so the offset is `+1`/`-2`) at
    // its `0.2083` keyframe.
    let (attack_body_pos, _) = sample_bone_offsets(&WARDEN_ATTACK, "body", 0.2083, 1.0);
    assert!((attack_body_pos[1] - 1.0).abs() < 1.0e-4);
    assert!((attack_body_pos[2] - -2.0).abs() < 1.0e-4);
}

#[test]
fn warden_combat_animations_re_pose_off_the_bind_pose() {
    // A warden with no triggered combat animation (all `-1.0` sentinels) renders at the look/idle/
    // walk (here resting) bind pose.
    let resting = entity_model_mesh(&[EntityModelInstance::warden(960, [0.0, 64.0, 0.0], 0.0)]);

    // A roaring warden re-poses off the bind pose (the body rears, head shakes, arms fling): the
    // mesh re-poses parts, it does not add or hide cubes.
    let roaring = entity_model_mesh(&[
        EntityModelInstance::warden(961, [0.0, 64.0, 0.0], 0.0).with_warden_roar_seconds(2.0)
    ]);
    assert_eq!(resting.vertices.len(), roaring.vertices.len());
    assert_ne!(
        resting.vertices, roaring.vertices,
        "the roaring warden leaves the bind pose"
    );

    // The attack and sonic boom at the same time pose differently from the roar and from each other.
    let attacking = entity_model_mesh(&[
        EntityModelInstance::warden(962, [0.0, 64.0, 0.0], 0.0).with_warden_attack_seconds(0.2)
    ]);
    let booming =
        entity_model_mesh(&[EntityModelInstance::warden(963, [0.0, 64.0, 0.0], 0.0)
            .with_warden_sonic_boom_seconds(2.0)]);
    assert_ne!(
        roaring.vertices, attacking.vertices,
        "the attack poses differently from the roar"
    );
    assert_ne!(
        attacking.vertices, booming.vertices,
        "the sonic boom poses differently from the attack"
    );
    assert_ne!(
        roaring.vertices, booming.vertices,
        "the sonic boom poses differently from the roar (only the boom fans the ribcages)"
    );

    // Sampling the roar at a different time advances the pose.
    let roaring_later =
        entity_model_mesh(&[
            EntityModelInstance::warden(964, [0.0, 64.0, 0.0], 0.0).with_warden_roar_seconds(3.0)
        ]);
    assert_ne!(
        roaring.vertices, roaring_later.vertices,
        "the roar advances as its elapsed seconds climb"
    );

    // The `-1.0` no-animation sentinel leaves the warden at the bind pose.
    let cleared = entity_model_mesh(&[
        EntityModelInstance::warden(965, [0.0, 64.0, 0.0], 0.0).with_warden_roar_seconds(-1.0)
    ]);
    assert_eq!(cleared.vertices, resting.vertices);
}

#[test]
fn warden_spawn_animations_match_vanilla_definitions() {
    // Vanilla `WardenAnimation.WARDEN_DIG` / `WARDEN_EMERGE`: the spawn/despawn one-shots (NOT
    // looping, so they hold their final frame), each animating all six bones (body, head, both arms,
    // both legs) with ROTATION + POSITION.
    assert_eq!(WARDEN_DIG.length_seconds, 5.0);
    assert!(!WARDEN_DIG.looping);
    assert_eq!(WARDEN_DIG.bones.len(), 6);
    assert_eq!(WARDEN_EMERGE.length_seconds, 6.68);
    assert!(!WARDEN_EMERGE.looping);
    assert_eq!(WARDEN_EMERGE.bones.len(), 6);

    // The emerge opens with the body 63 units underground: `posVec(0, -63, 0)` at its `0.0` keyframe
    // (`posVec` negates y, so the position offset is `+63`). The legs start there too.
    let (emerge_body_pos, _) = sample_bone_offsets(&WARDEN_EMERGE, "body", 0.0, 1.0);
    assert!((emerge_body_pos[1] - 63.0).abs() < 1.0e-4);
    let (emerge_right_leg_pos, _) = sample_bone_offsets(&WARDEN_EMERGE, "right_leg", 0.0, 1.0);
    assert!((emerge_right_leg_pos[1] - 63.0).abs() < 1.0e-4);

    // The dig's final body frame is the LINEAR `degreeVec(147.5, 0, 0)` at its `4.5` keyframe (the
    // warden pitched face-down into the ground).
    let (_, dig_body_rot) = sample_bone_offsets(&WARDEN_DIG, "body", 4.5, 1.0);
    assert!((dig_body_rot[0] - 147.5_f32.to_radians()).abs() < 1.0e-4);
    // The dig also swings the legs (only dig/emerge reach the legs): `right_leg` rears to
    // `degreeVec(113.27, 0, 0)` across its `0.5..3.3333` hold.
    let (_, dig_right_leg_rot) = sample_bone_offsets(&WARDEN_DIG, "right_leg", 0.7083, 1.0);
    assert!((dig_right_leg_rot[0] - 113.27_f32.to_radians()).abs() < 1.0e-4);
}

#[test]
fn warden_spawn_animations_re_pose_off_the_bind_pose_including_the_legs() {
    // A warden with no triggered animation (all `-1.0` sentinels) renders at the look/idle/walk
    // (here resting) bind pose; the legs occupy vertices `[192, 240)`.
    let resting = entity_model_mesh(&[EntityModelInstance::warden(970, [0.0, 64.0, 0.0], 0.0)]);

    // The emerge re-poses off the bind pose AND moves the legs (unlike the attack/sonic_boom/roar/
    // sniff combat one-shots, which carry no leg bone). It re-poses parts, it does not add/hide cubes.
    let emerging = entity_model_mesh(&[
        EntityModelInstance::warden(971, [0.0, 64.0, 0.0], 0.0).with_warden_emerge_seconds(1.0)
    ]);
    assert_eq!(resting.vertices.len(), emerging.vertices.len());
    assert_ne!(
        resting.vertices, emerging.vertices,
        "the emerging warden leaves the bind pose"
    );
    assert_ne!(
        resting.vertices[192..],
        emerging.vertices[192..],
        "the emerge swings the legs (the combat one-shots never do)"
    );

    // The dig poses differently from the emerge, and also moves the legs.
    let digging = entity_model_mesh(&[
        EntityModelInstance::warden(972, [0.0, 64.0, 0.0], 0.0).with_warden_dig_seconds(1.0)
    ]);
    assert_ne!(
        emerging.vertices, digging.vertices,
        "the dig poses differently from the emerge"
    );
    assert_ne!(
        resting.vertices[192..],
        digging.vertices[192..],
        "the dig swings the legs"
    );

    // Sampling the emerge at a later time advances the rise.
    let emerging_later =
        entity_model_mesh(&[
            EntityModelInstance::warden(973, [0.0, 64.0, 0.0], 0.0).with_warden_emerge_seconds(4.0)
        ]);
    assert_ne!(
        emerging.vertices, emerging_later.vertices,
        "the emerge advances as its elapsed seconds climb"
    );

    // The `-1.0` no-animation sentinel leaves the warden at the bind pose.
    let cleared = entity_model_mesh(&[
        EntityModelInstance::warden(974, [0.0, 64.0, 0.0], 0.0).with_warden_dig_seconds(-1.0)
    ]);
    assert_eq!(cleared.vertices, resting.vertices);
}

#[test]
fn warden_textured_render_matches_vanilla_renderer() {
    // The warden binds its base body texture (warden.png, atlas 128×128, whole model), then the five
    // `WardenEmissiveLayer`s as eyes-render-type passes (the eyes pipeline being emissive + alpha-
    // blended), each over its own `retainExactParts` subset: the always-on bioluminescent overlay
    // (head/arms/legs), the two pulsating-spots overlays (body/legs), the tendril overlay (the two
    // tendrils, reusing the base texture at the lerped `tendrilAnimation` alpha), and the heart overlay
    // (body only, warden_heart.png at the lerped `heartAnimation` alpha).
    let passes = warden_textured_layer_passes(0.0, 1.0, 0.7);
    assert_eq!(passes.len(), 6);
    assert_eq!(passes[0].kind, EntityModelLayerKind::WardenBase);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].model_layer, MODEL_LAYER_WARDEN);
    assert_eq!(passes[0].texture, WARDEN_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
    assert_eq!(passes[1].kind, EntityModelLayerKind::WardenBioluminescent);
    assert_eq!(passes[1].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[1].render_type.vanilla_name(), "eyes");
    assert_eq!(passes[1].model_layer, MODEL_LAYER_WARDEN_BIOLUMINESCENT);
    assert_eq!(passes[1].texture, WARDEN_BIOLUMINESCENT_TEXTURE_REF);
    assert_eq!(passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[1].order, passes[1].submit_sequence), (1, 1));
    assert_eq!(
        passes[1].visibility,
        EntityModelLayerVisibility::RetainedParts(&[
            "head",
            "left_arm",
            "right_arm",
            "left_leg",
            "right_leg"
        ])
    );
    assert_eq!(passes[2].kind, EntityModelLayerKind::WardenPulsatingSpots1);
    assert_eq!(passes[2].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[2].render_type.vanilla_name(), "eyes");
    assert_eq!(passes[2].model_layer, MODEL_LAYER_WARDEN_PULSATING_SPOTS);
    assert_eq!(passes[2].texture, WARDEN_PULSATING_SPOTS_1_TEXTURE_REF);
    assert_eq!((passes[2].order, passes[2].submit_sequence), (1, 2));
    assert_eq!(
        passes[2].visibility,
        EntityModelLayerVisibility::RetainedParts(&[
            "body",
            "head",
            "left_arm",
            "right_arm",
            "left_leg",
            "right_leg"
        ])
    );
    assert_eq!(passes[3].kind, EntityModelLayerKind::WardenPulsatingSpots2);
    assert_eq!(passes[3].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[3].render_type.vanilla_name(), "eyes");
    assert_eq!(passes[3].model_layer, MODEL_LAYER_WARDEN_PULSATING_SPOTS);
    assert_eq!(passes[3].texture, WARDEN_PULSATING_SPOTS_2_TEXTURE_REF);
    assert_eq!((passes[3].order, passes[3].submit_sequence), (1, 3));
    assert_eq!(passes[3].visibility, passes[2].visibility);
    // The tendril overlay reuses warden.png over the two tendril planes at `tendrilAnimation` (1.0 here).
    assert_eq!(passes[4].kind, EntityModelLayerKind::WardenTendrils);
    assert_eq!(passes[4].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[4].render_type.vanilla_name(), "eyes");
    assert_eq!(passes[4].model_layer, MODEL_LAYER_WARDEN_TENDRILS);
    assert_eq!(passes[4].texture, WARDEN_TEXTURE_REF);
    assert_eq!(passes[4].tint[3], 1.0);
    assert_eq!((passes[4].order, passes[4].submit_sequence), (1, 4));
    assert_eq!(
        passes[4].visibility,
        EntityModelLayerVisibility::RetainedParts(&["left_tendril", "right_tendril"])
    );
    // The heart overlay binds warden_heart.png over the body only at `heartAnimation` (0.7 here).
    assert_eq!(passes[5].kind, EntityModelLayerKind::WardenHeart);
    assert_eq!(passes[5].render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(passes[5].render_type.vanilla_name(), "eyes");
    assert_eq!(passes[5].model_layer, MODEL_LAYER_WARDEN_HEART);
    assert_eq!(passes[5].texture, WARDEN_HEART_TEXTURE_REF);
    assert_eq!(passes[5].tint[3], 0.7);
    assert_eq!((passes[5].order, passes[5].submit_sequence), (1, 5));
    assert_eq!(
        passes[5].visibility,
        EntityModelLayerVisibility::RetainedParts(&["body"])
    );

    // The pulsating alpha is `max(0, cos(ageInTicks · 0.045 + phase) · 0.25)`. At age 0 the first set
    // is at its peak 0.25 while the π-offset second set is clamped to 0; the two alternate over time.
    assert!((passes[2].tint[3] - 0.25).abs() < 1.0e-6);
    assert_eq!(passes[3].tint[3], 0.0);
    assert!((warden_pulsating_spots_alpha(0.0, 0.0) - 0.25).abs() < 1.0e-6);
    assert_eq!(warden_pulsating_spots_alpha(0.0, std::f32::consts::PI), 0.0);
    let half_period = std::f32::consts::PI / 0.045;
    assert_eq!(warden_pulsating_spots_alpha(half_period, 0.0), 0.0);
    assert!(
        (warden_pulsating_spots_alpha(half_period, std::f32::consts::PI) - 0.25).abs() < 1.0e-6
    );

    assert_eq!(
        EntityModelKind::Warden.vanilla_texture_ref(),
        Some(WARDEN_TEXTURE_REF)
    );
    for texture in [
        WARDEN_TEXTURE_REF,
        WARDEN_BIOLUMINESCENT_TEXTURE_REF,
        WARDEN_PULSATING_SPOTS_1_TEXTURE_REF,
        WARDEN_PULSATING_SPOTS_2_TEXTURE_REF,
        WARDEN_HEART_TEXTURE_REF,
    ] {
        assert!(entity_model_texture_refs().contains(&texture));
    }
    assert_eq!(
        warden_entity_texture_refs(),
        &[
            WARDEN_TEXTURE_REF,
            WARDEN_BIOLUMINESCENT_TEXTURE_REF,
            WARDEN_PULSATING_SPOTS_1_TEXTURE_REF,
            WARDEN_PULSATING_SPOTS_2_TEXTURE_REF,
            WARDEN_HEART_TEXTURE_REF
        ]
    );

    let images: Vec<EntityModelTextureImage> = warden_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let default_instance = EntityModelInstance::warden(920, [0.0, 64.0, 0.0], 0.0)
        .with_light_coords((5_u32 << 4) | (11_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let meshes = entity_model_textured_meshes(&[default_instance], &atlas);
    assert_warden_submissions_match_vanilla(&meshes, default_instance);
    // The base cutout pass draws the whole body (10 cubes → 240 vertices), every cube at the neutral
    // tint; the emissive overlays route to the eyes mesh, not here.
    assert_eq!(meshes.cutout.vertices.len(), 240);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == meshes.submissions[0].light
            && vertex.overlay == meshes.submissions[0].overlay));
    // Vanilla skips 0-alpha emissive layers. At age 0 with no heart/tendril pulse, only the
    // bioluminescent layer (5 cubes) and the first pulsating-spots layer (3 cubes) submit.
    assert_eq!(meshes.eyes.vertices.len(), 8 * 24);
    assert!(meshes.eyes.vertices.iter().all(|vertex| {
        vertex.light == default_instance.render_state.shader_light()
            && vertex.overlay == [0.0, default_instance.render_state.overlay_coords()[1]]
    }));

    let animated_instance = EntityModelInstance::warden(921, [0.0, 64.0, 0.0], 0.0)
        .with_light_coords((6_u32 << 4) | (10_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true)
        .with_age_in_ticks(half_period)
        .with_tendril_animation(1.0)
        .with_heart_animation(0.7);
    let animated_meshes = entity_model_textured_meshes(&[animated_instance], &atlas);
    assert_warden_submissions_match_vanilla(&animated_meshes, animated_instance);
    // Half a pulsating-spots period flips the active spots layer; the non-zero tendril and heart
    // pulses add their retained tendril (2 cubes) and body-heart (1 cube) submissions.
    assert_eq!(animated_meshes.eyes.vertices.len(), 11 * 24);
    assert!(animated_meshes.eyes.vertices.iter().all(|vertex| {
        vertex.light == animated_instance.render_state.shader_light()
            && vertex.overlay == [0.0, animated_instance.render_state.overlay_coords()[1]]
    }));
}

#[test]
fn warden_bioluminescent_submission_survives_missing_texture_atlas_entry() {
    // Vanilla records the always-on bioluminescent emissive layer at order(1); missing texture data
    // suppresses only that layer's folded retained-part geometry.
    let images: Vec<EntityModelTextureImage> = warden_entity_texture_refs()
        .iter()
        .enumerate()
        .filter_map(|(index, texture)| {
            if *texture == WARDEN_BIOLUMINESCENT_TEXTURE_REF {
                return None;
            }
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            Some(EntityModelTextureImage::new(
                *texture,
                vec![index as u8; len],
            ))
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::warden(922, [0.0, 64.0, 0.0], 0.0)
        .with_light_coords((2_u32 << 4) | (14_u32 << 20))
        .with_white_overlay_progress(0.7)
        .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_warden_submissions_match_vanilla(&meshes, instance);
    assert_eq!(meshes.submissions.len(), 3);
    let bioluminescent = meshes.submissions[1];
    assert_eq!(bioluminescent.render_type, EntityModelLayerRenderType::Eyes);
    assert_eq!(bioluminescent.render_type.vanilla_name(), "eyes");
    assert_eq!(bioluminescent.texture, WARDEN_BIOLUMINESCENT_TEXTURE_REF);
    assert_eq!(
        (bioluminescent.order, bioluminescent.submit_sequence),
        (1, 1)
    );
    assert_eq!(bioluminescent.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(
        bioluminescent.transform,
        entity_model_root_transform(instance)
    );
    assert_eq!(bioluminescent.light, instance.render_state.shader_light());
    assert_eq!(
        bioluminescent.overlay,
        [0.0, instance.render_state.overlay_coords()[1]]
    );

    assert_eq!(meshes.cutout.vertices.len(), 240);
    assert_eq!(
        meshes.eyes.vertices.len(),
        3 * 24,
        "missing bioluminescent texture suppresses only its five retained cubes"
    );
    assert!(meshes
        .eyes
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 0.25]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == [0.0, instance.render_state.overlay_coords()[1]]));
}

fn assert_warden_submissions_match_vanilla(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    let EntityModelKind::Warden = instance.kind else {
        panic!("expected warden instance");
    };
    let expected: Vec<_> = warden_textured_layer_passes(
        instance.render_state.age_in_ticks,
        instance.render_state.tendril_animation,
        instance.render_state.heart_animation,
    )
    .into_iter()
    .filter(|pass| pass.tint[3] > 1.0e-5)
    .collect();
    assert_eq!(meshes.submissions.len(), expected.len());
    let transform = entity_model_root_transform(instance);
    for (submit, pass) in meshes.submissions.iter().zip(expected.iter()) {
        assert_eq!(submit.render_type, pass.render_type);
        assert_eq!(
            submit.render_type.vanilla_name(),
            pass.render_type.vanilla_name()
        );
        assert_eq!(submit.texture, pass.texture);
        assert_eq!(submit.tint, pass.tint);
        assert_eq!(submit.transform, transform);
        assert_eq!(submit.light, instance.render_state.shader_light());
        if pass.kind == EntityModelLayerKind::WardenBase {
            assert_eq!(submit.overlay, instance.render_state.overlay_coords());
            assert_ne!(submit.overlay, [0.0, 10.0]);
        } else {
            assert_eq!(
                submit.overlay,
                [0.0, instance.render_state.overlay_coords()[1]]
            );
            assert_ne!(submit.overlay, instance.render_state.overlay_coords());
            assert_ne!(submit.overlay, [0.0, 10.0]);
        }
        assert_eq!(
            (submit.order, submit.submit_sequence),
            (pass.order, pass.submit_sequence)
        );
        assert!(submit.dynamic_player_skin.is_none());
        assert!(submit.dynamic_player_texture.is_none());
    }
}
