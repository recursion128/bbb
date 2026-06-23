use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn frog_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `FrogModel.createBodyLayer` (atlas 48×48): the mesh root holds one `root` part at
    // `offset(0, 24, 0)` parenting `body` and the two legs.
    assert_eq!(FROG_PARTS.len(), 1);
    let root = &FROG_PARTS[0];
    assert_eq!(root.pose.offset, [0.0, 24.0, 0.0]);
    assert!(root.cubes.is_empty());
    assert_eq!(root.children.len(), 3);

    // `body`: the 7×3×9 box + the 7×0×9 underside plane, parenting head / tongue / two arms.
    let body = &root.children[0];
    assert_eq!(body.pose.offset, [0.0, -2.0, 4.0]);
    assert_eq!(body.cubes.len(), 2);
    assert_eq!(body.cubes[0].min, [-3.5, -2.0, -8.0]);
    assert_eq!(body.cubes[0].size, [7.0, 3.0, 9.0]);
    assert_eq!(body.cubes[1].size, [7.0, 0.0, 9.0]);
    assert_eq!(body.children.len(), 4);

    // `head` (7×0×9 plane + 7×3×9 box) parents the `eyes` pivot.
    let head = &body.children[0];
    assert_eq!(head.pose.offset, [0.0, -2.0, -1.0]);
    assert_eq!(head.cubes.len(), 2);
    assert_eq!(head.cubes[1].min, [-3.5, -2.0, -7.0]);
    assert_eq!(head.children.len(), 1);

    // The `eyes` empty pivot parents the two 3×2×3 eyes at ±X.
    let eyes = &head.children[0];
    assert_eq!(eyes.pose.offset, [-0.5, 0.0, 2.0]);
    assert!(eyes.cubes.is_empty());
    assert_eq!(eyes.children.len(), 2);
    assert_eq!(eyes.children[0].pose.offset, [-1.5, -3.0, -6.5]);
    assert_eq!(eyes.children[1].pose.offset, [2.5, -3.0, -6.5]);
    assert_eq!(eyes.children[0].cubes[0].min, [-1.5, -1.0, -1.5]);
    assert_eq!(eyes.children[0].cubes[0].size, [3.0, 2.0, 3.0]);

    // The arms (2×3×3) each parent an 8×0×8 webbed hand; the hands differ only in Z origin.
    let left_arm = &body.children[2];
    let right_arm = &body.children[3];
    assert_eq!(left_arm.pose.offset, [4.0, -1.0, -6.5]);
    assert_eq!(right_arm.pose.offset, [-4.0, -1.0, -6.5]);
    assert_eq!(left_arm.children[0].cubes[0].min, [-4.0, 0.01, -4.0]);
    assert_eq!(right_arm.children[0].cubes[0].min, [-4.0, 0.01, -5.0]);

    // The legs (differ only in X origin) each parent an 8×0×8 foot plane.
    let left_leg = &root.children[1];
    let right_leg = &root.children[2];
    assert_eq!(left_leg.pose.offset, [3.5, -3.0, 4.0]);
    assert_eq!(left_leg.cubes[0].min, [-1.0, 0.0, -2.0]);
    assert_eq!(right_leg.cubes[0].min, [-2.0, 0.0, -2.0]);
    assert_eq!(left_leg.children[0].cubes[0].size, [8.0, 0.0, 8.0]);

    // Fifteen visible cubes total — the `croaking_body` is hidden at rest and so is omitted.
    assert_eq!(count_cubes(&FROG_PARTS), 15);
}

#[test]
fn frog_mesh_uses_vanilla_body_layer_geometry() {
    // 15 cubes → 90 faces / 360 vertices / 540 indices.
    let frog = entity_model_mesh(&[EntityModelInstance::frog(950, [0.0, 64.0, 0.0], 0.0)]);
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
fn frog_walk_moves_the_limbs_off_the_walk_cycle() {
    // A still frog (walk speed 0) samples the cycle at amplitude 0, collapsing to the bind pose; a
    // walking frog samples the FROG_WALK offsets, animating the body, arms, and legs. The vertex
    // count is preserved (no parts appear or vanish).
    let still = entity_model_mesh(&[EntityModelInstance::frog(70, [0.0, 64.0, 0.0], 0.0)]);
    let walking = entity_model_mesh(&[
        EntityModelInstance::frog(71, [0.0, 64.0, 0.0], 0.0).with_walk_animation(6.0, 1.0)
    ]);
    assert_eq!(still.vertices.len(), walking.vertices.len());
    assert_ne!(
        still.vertices, walking.vertices,
        "the walking frog animates its limbs"
    );

    // The still frog equals the plain bind-pose emit (amplitude 0 ⇒ no offsets).
    let bind = entity_model_mesh(&[EntityModelInstance::frog(72, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(still.vertices, bind.vertices);
}
