use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn warden_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `WardenModel.createBodyLayer` (atlas 128×128): the mesh root holds one `bone` part
    // at `offset(0, 24, 0)` parenting the body and the two legs.
    assert_eq!(WARDEN_PARTS.len(), 1);
    let bone = &WARDEN_PARTS[0];
    assert_eq!(bone.pose.offset, [0.0, 24.0, 0.0]);
    assert!(bone.cubes.is_empty());
    assert_eq!(bone.children.len(), 3);

    // `body` (offset (0, -21, 0)): one 18×21×11 box parenting two ribcages, the head, and arms.
    let body = &bone.children[0];
    assert_eq!(body.pose.offset, [0.0, -21.0, 0.0]);
    assert_eq!(body.cubes.len(), 1);
    assert_eq!(body.cubes[0].min, [-9.0, -13.0, -4.0]);
    assert_eq!(body.cubes[0].size, [18.0, 21.0, 11.0]);
    assert_eq!(body.children.len(), 5);

    // The two 9×21×0 ribcage planes.
    let right_ribcage = &body.children[0];
    let left_ribcage = &body.children[1];
    assert_eq!(right_ribcage.pose.offset, [-7.0, -2.0, -4.0]);
    assert_eq!(right_ribcage.cubes[0].min, [-2.0, -11.0, -0.1]);
    assert_eq!(left_ribcage.pose.offset, [7.0, -2.0, -4.0]);
    assert_eq!(left_ribcage.cubes[0].min, [-7.0, -11.0, -0.1]);
    assert_eq!(right_ribcage.cubes[0].size, [9.0, 21.0, 0.0]);

    // `head` (16×16×10) parents the two 16×16×0 tendril planes.
    let head = &body.children[2];
    assert_eq!(head.pose.offset, [0.0, -13.0, 0.0]);
    assert_eq!(head.cubes[0].size, [16.0, 16.0, 10.0]);
    assert_eq!(head.children.len(), 2);
    assert_eq!(head.children[0].pose.offset, [-8.0, -12.0, 0.0]);
    assert_eq!(head.children[0].cubes[0].min, [-16.0, -13.0, 0.0]);
    assert_eq!(head.children[1].cubes[0].min, [0.0, -13.0, 0.0]);
    assert_eq!(head.children[1].cubes[0].size, [16.0, 16.0, 0.0]);

    // The two 8×28×8 arms.
    let right_arm = &body.children[3];
    let left_arm = &body.children[4];
    assert_eq!(right_arm.pose.offset, [-13.0, -13.0, 1.0]);
    assert_eq!(left_arm.pose.offset, [13.0, -13.0, 1.0]);
    assert_eq!(right_arm.cubes[0].size, [8.0, 28.0, 8.0]);

    // The two 6×13×6 legs (differing only in X origin).
    let right_leg = &bone.children[1];
    let left_leg = &bone.children[2];
    assert_eq!(right_leg.pose.offset, [-5.9, -13.0, 0.0]);
    assert_eq!(right_leg.cubes[0].min, [-3.1, 0.0, -3.0]);
    assert_eq!(left_leg.pose.offset, [5.9, -13.0, 0.0]);
    assert_eq!(left_leg.cubes[0].min, [-2.9, 0.0, -3.0]);

    // Ten cubes total.
    assert_eq!(count_cubes(&WARDEN_PARTS), 10);
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
    let body_bind = WARDEN_PARTS[0].children[WARDEN_BODY_BONE_CHILD_INDEX].pose;
    let body = warden_idle_body_pose(body_bind, age);
    assert!((body.rotation[0] - (body_bind.rotation[0] + 0.025 * s.cos())).abs() < 1.0e-6);
    assert_eq!(body.rotation[1], body_bind.rotation[1]);
    assert!((body.rotation[2] - (body_bind.rotation[2] + 0.025 * s.sin())).abs() < 1.0e-6);
    assert_eq!(body.offset, body_bind.offset);

    // The head pose: the look sets xRot/yRot, then the idle roll adds xRot += 0.06·sin(s) and
    // zRot += 0.06·cos(s).
    let head_bind = WARDEN_PARTS[0].children[WARDEN_BODY_BONE_CHILD_INDEX].children
        [WARDEN_HEAD_BODY_CHILD_INDEX]
        .pose;
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
