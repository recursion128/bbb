use super::*;

#[test]
fn warden_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `WardenModel.createBodyLayer` (atlas 128×128): the mesh root holds one `bone` part
    // at `offset(0, 24, 0)` parenting the body and the two legs.
    assert_eq!(WARDEN_BONE_POSE.offset, [0.0, 24.0, 0.0]);

    // `body` (offset (0, -21, 0)): one 18×21×11 box parenting two ribcages, the head, and arms.
    assert_eq!(WARDEN_BODY_POSE.offset, [0.0, -21.0, 0.0]);
    assert_eq!(WARDEN_BODY_CUBES.len(), 1);
    assert_eq!(WARDEN_BODY_CUBES[0].min, [-9.0, -13.0, -4.0]);
    assert_eq!(WARDEN_BODY_CUBES[0].size, [18.0, 21.0, 11.0]);

    // The two 9×21×0 ribcage planes.
    assert_eq!(WARDEN_RIGHT_RIBCAGE_POSE.offset, [-7.0, -2.0, -4.0]);
    assert_eq!(WARDEN_RIGHT_RIBCAGE_CUBES[0].min, [-2.0, -11.0, -0.1]);
    assert_eq!(WARDEN_LEFT_RIBCAGE_POSE.offset, [7.0, -2.0, -4.0]);
    assert_eq!(WARDEN_LEFT_RIBCAGE_CUBES[0].min, [-7.0, -11.0, -0.1]);
    assert_eq!(WARDEN_RIGHT_RIBCAGE_CUBES[0].size, [9.0, 21.0, 0.0]);

    // `head` (16×16×10) parents the two 16×16×0 tendril planes.
    assert_eq!(WARDEN_HEAD_POSE.offset, [0.0, -13.0, 0.0]);
    assert_eq!(WARDEN_HEAD_CUBES[0].size, [16.0, 16.0, 10.0]);
    assert_eq!(WARDEN_RIGHT_TENDRIL_POSE.offset, [-8.0, -12.0, 0.0]);
    assert_eq!(WARDEN_RIGHT_TENDRIL_CUBES[0].min, [-16.0, -13.0, 0.0]);
    assert_eq!(WARDEN_LEFT_TENDRIL_CUBES[0].min, [0.0, -13.0, 0.0]);
    assert_eq!(WARDEN_LEFT_TENDRIL_CUBES[0].size, [16.0, 16.0, 0.0]);

    // The two 8×28×8 arms.
    assert_eq!(WARDEN_RIGHT_ARM_POSE.offset, [-13.0, -13.0, 1.0]);
    assert_eq!(WARDEN_LEFT_ARM_POSE.offset, [13.0, -13.0, 1.0]);
    assert_eq!(WARDEN_ARM_CUBES[0].size, [8.0, 28.0, 8.0]);

    // The two 6×13×6 legs (differing only in X origin).
    assert_eq!(WARDEN_RIGHT_LEG_POSE.offset, [-5.9, -13.0, 0.0]);
    assert_eq!(WARDEN_RIGHT_LEG_CUBES[0].min, [-3.1, 0.0, -3.0]);
    assert_eq!(WARDEN_LEFT_LEG_POSE.offset, [5.9, -13.0, 0.0]);
    assert_eq!(WARDEN_LEFT_LEG_CUBES[0].min, [-2.9, 0.0, -3.0]);
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
