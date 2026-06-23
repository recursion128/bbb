use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn adult_armadillo_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultArmadilloModel.createBodyLayer` (atlas 64×64): the root parents the body and
    // the four legs directly; the body parents the tail and head, and the head parents the head
    // cube and the two ear pivots.
    assert_eq!(ADULT_ARMADILLO_PARTS.len(), 5);

    // `body` (offset (0, 21, 4)): a `CubeDeformation(0.3)` shell over the bare 8×8×12 box.
    let body = &ADULT_ARMADILLO_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 21.0, 4.0]);
    assert_eq!(body.cubes.len(), 2);
    assert_eq!(body.cubes[0].min, [-4.3, -7.3, -10.3]);
    assert_eq!(body.cubes[0].size, [8.6, 8.6, 12.6]);
    assert_eq!(body.cubes[1].min, [-4.0, -7.0, -10.0]);
    assert_eq!(body.cubes[1].size, [8.0, 8.0, 12.0]);
    assert_eq!(body.children.len(), 2);

    // `tail`: the 1×6×1 plume, pitched down by 0.5061 rad.
    let tail = &body.children[0];
    assert_eq!(tail.pose.offset, [0.0, -3.0, 1.0]);
    assert_eq!(tail.pose.rotation, [0.5061, 0.0, 0.0]);
    assert_eq!(tail.cubes[0].size, [1.0, 6.0, 1.0]);

    // `head` (offset (0, -2, -11)): a bare pivot parenting the head cube and the two ears.
    let head = &body.children[1];
    assert_eq!(head.pose.offset, [0.0, -2.0, -11.0]);
    assert!(head.cubes.is_empty());
    assert_eq!(head.children.len(), 3);

    // `head_cube`: the 3×5×2 snout, pitched up by -0.3927 rad.
    let head_cube = &head.children[0];
    assert_eq!(head_cube.pose.rotation, [-0.3927, 0.0, 0.0]);
    assert_eq!(head_cube.cubes[0].size, [3.0, 5.0, 2.0]);

    // The two ear pivots and their rotated 2×5×0 ear planes.
    let right_ear = &head.children[1];
    let left_ear = &head.children[2];
    assert_eq!(right_ear.pose.offset, [-1.0, -1.0, 0.0]);
    assert_eq!(
        right_ear.children[0].pose.rotation,
        [0.1886, -0.3864, -0.0718]
    );
    assert_eq!(right_ear.children[0].cubes[0].min, [-2.0, -3.0, 0.0]);
    assert_eq!(left_ear.pose.offset, [1.0, -2.0, 0.0]);
    assert_eq!(left_ear.children[0].pose.rotation, [0.1886, 0.3864, 0.0718]);
    assert_eq!(left_ear.children[0].cubes[0].size, [2.0, 5.0, 0.0]);

    // The four 2×3×2 legs at the corner pivots.
    assert_eq!(ADULT_ARMADILLO_PARTS[1].pose.offset, [-2.0, 21.0, 4.0]);
    assert_eq!(ADULT_ARMADILLO_PARTS[2].pose.offset, [2.0, 21.0, 4.0]);
    assert_eq!(ADULT_ARMADILLO_PARTS[3].pose.offset, [-2.0, 21.0, -4.0]);
    assert_eq!(ADULT_ARMADILLO_PARTS[4].pose.offset, [2.0, 21.0, -4.0]);
    assert_eq!(ADULT_ARMADILLO_PARTS[1].cubes[0].size, [2.0, 3.0, 2.0]);

    // Ten cubes in the non-hiding rest pose (the shell-ball `cube` is a separate rolled-up part).
    assert_eq!(count_cubes(&ADULT_ARMADILLO_PARTS), 10);
}

#[test]
fn baby_armadillo_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyArmadilloModel.createBodyLayer` (atlas 64×64): smaller geometry, the ears
    // parented to the head cube, and the front legs at swapped X origins.
    assert_eq!(BABY_ARMADILLO_PARTS.len(), 5);

    let body = &BABY_ARMADILLO_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 20.0, 0.5]);
    assert_eq!(body.cubes[0].min, [-2.8, -2.3, -3.8]);
    assert_eq!(body.cubes[0].size, [5.6, 4.6, 7.6]);
    assert_eq!(body.cubes[1].size, [5.0, 4.0, 6.0]);

    // `tail` pivot (offset (0, 0, 3.4)) parents the 1×1×4 stub pitched by -1.0472 rad.
    let tail = &body.children[0];
    assert_eq!(tail.pose.offset, [0.0, 0.0, 3.4]);
    assert_eq!(tail.children[0].pose.rotation, [-1.0472, 0.0, 0.0]);
    assert_eq!(tail.children[0].cubes[0].size, [1.0, 1.0, 4.0]);

    // `head` pivot parents the head cube (pitched up 0.7417649 rad) which parents the two ears.
    let head = &body.children[1];
    assert_eq!(head.pose.offset, [0.0, 0.0, -3.2]);
    let head_cube = &head.children[0];
    assert_eq!(head_cube.pose.rotation, [0.7417649, 0.0, 0.0]);
    assert_eq!(head_cube.cubes[0].size, [2.0, 2.0, 4.0]);
    assert_eq!(head_cube.children.len(), 2);
    assert_eq!(
        head_cube.children[0].pose.rotation,
        [-0.4363, -0.1134, 0.0524]
    );
    assert_eq!(
        head_cube.children[1].pose.rotation,
        [-0.4363, 0.1134, -0.0524]
    );
    assert_eq!(head_cube.children[0].cubes[0].size, [2.0, 3.0, 0.0]);

    // The front legs carry vanilla's swapped X origins (right at +1.5, left at -1.5).
    assert_eq!(BABY_ARMADILLO_PARTS[1].pose.offset, [-1.5, 22.0, 2.5]);
    assert_eq!(BABY_ARMADILLO_PARTS[2].pose.offset, [1.5, 22.0, 2.5]);
    assert_eq!(BABY_ARMADILLO_PARTS[3].pose.offset, [1.5, 22.0, -1.5]);
    assert_eq!(BABY_ARMADILLO_PARTS[4].pose.offset, [-1.5, 22.0, -1.5]);
    assert_eq!(BABY_ARMADILLO_PARTS[1].cubes[0].size, [2.0, 2.0, 2.0]);

    assert_eq!(count_cubes(&BABY_ARMADILLO_PARTS), 10);
}

#[test]
fn armadillo_rolled_up_parts_match_vanilla_hiding_in_shell() {
    // Vanilla `ArmadilloModel.setupAnim` `isHidingInShell`: the body cubes (`skipDraw`), the tail,
    // and both hind legs hide; the head (+ ears), both front legs, and the `cube` ball show.
    // So the rolled-up tree is: body pivot (no cubes) → head only; the two front legs; the ball →
    // head_cube + 2 ears + 2 front legs + 1 ball = 6 cubes.
    assert_eq!(ADULT_ARMADILLO_ROLLED_PARTS.len(), 4);

    // Hiding body keeps its pivot offset but drops its own cubes and the tail child.
    let body = &ADULT_ARMADILLO_ROLLED_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 21.0, 4.0]);
    assert!(body.cubes.is_empty());
    assert_eq!(body.children.len(), 1);
    let head = &body.children[0];
    assert_eq!(head.pose.offset, [0.0, -2.0, -11.0]);
    assert_eq!(head.children.len(), 3); // head_cube + the two ears

    // The two FRONT legs (z = -4) stay; the hind legs (z = +4) are gone.
    assert_eq!(
        ADULT_ARMADILLO_ROLLED_PARTS[1].pose.offset,
        [-2.0, 21.0, -4.0]
    );
    assert_eq!(
        ADULT_ARMADILLO_ROLLED_PARTS[2].pose.offset,
        [2.0, 21.0, -4.0]
    );

    // The shell-ball `cube` (root child at (0, 24, 0)): a plain 10×10×10 box.
    let ball = &ADULT_ARMADILLO_ROLLED_PARTS[3];
    assert_eq!(ball.pose.offset, [0.0, 24.0, 0.0]);
    assert_eq!(ball.cubes[0].min, [-5.0, -10.0, -6.0]);
    assert_eq!(ball.cubes[0].size, [10.0, 10.0, 10.0]);

    // Six cubes total in the rolled-up pose.
    assert_eq!(count_cubes(&ADULT_ARMADILLO_ROLLED_PARTS), 6);

    // Baby: same swap; the ball is the 6×6×6 box + CubeDeformation(0.3) → min -3.3, size 6.6.
    assert_eq!(count_cubes(&BABY_ARMADILLO_ROLLED_PARTS), 6);
    let baby_ball = &BABY_ARMADILLO_ROLLED_PARTS[3];
    assert_eq!(baby_ball.pose.offset, [0.0, 20.7, 0.5]);
    assert_eq!(baby_ball.cubes[0].min, [-3.3, -3.3, -3.3]);
    assert_eq!(baby_ball.cubes[0].size, [6.6, 6.6, 6.6]);
}

#[test]
fn armadillo_rolled_up_mesh_swaps_to_the_shell_ball() {
    // The rolled-up mesh has six cubes (→ 36 faces / 144 vertices) versus the rest pose's ten,
    // and shows the shell-ball geometry while hiding the body box, tail, and hind legs.
    let rest = entity_model_mesh(&[EntityModelInstance::armadillo(
        72,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    let rolled = entity_model_mesh(&[EntityModelInstance::armadillo(
        73,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )]);
    assert_eq!(rest.vertices.len(), 240);
    assert_eq!(rolled.opaque_faces, 36);
    assert_eq!(rolled.vertices.len(), 144);

    // The 10×10×10 shell ball is wider in Z than the rest pose's bare body box at this scale, so
    // the rolled mesh reaches a distinct extent — proving the ball replaced the body.
    assert_ne!(rest.vertices, rolled.vertices);
}

#[test]
fn armadillo_mesh_selects_adult_or_baby_body_layer() {
    // Each rest pose has 10 cubes → 60 faces / 240 vertices / 360 indices; the soft head/ears/tail
    // carry the skin tint while the armored body/legs carry the shell tint.
    let adult = entity_model_mesh(&[EntityModelInstance::armadillo(
        70,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    assert_eq!(adult.opaque_faces, 60);
    assert_eq!(adult.vertices.len(), 240);
    assert_eq!(adult.indices.len(), 360);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ARMADILLO_SHELL, 1.0)));
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ARMADILLO_SKIN, 1.0)));

    let baby = entity_model_mesh(&[EntityModelInstance::armadillo(
        71,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
    )]);
    assert_eq!(baby.opaque_faces, 60);
    assert_eq!(baby.vertices.len(), 240);

    // The baby layer is geometrically smaller than the adult, so its mesh is more compact.
    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    let adult_span = adult_max[2] - adult_min[2];
    let baby_span = baby_max[2] - baby_min[2];
    assert!(
        baby_span < adult_span,
        "baby z-span {baby_span} should be smaller than adult {adult_span}"
    );
}

#[test]
fn armadillo_clamps_the_head_look_to_vanilla_bounds() {
    // Vanilla `ArmadilloModel.setupAnim` (not hiding) clamps the look: pitch (`xRot`) to [-22.5, 25]
    // and yaw (`yRot`) to [-32.5, 32.5] degrees before assigning `head.xRot/yRot`.
    assert_eq!(armadillo_clamped_head_look(50.0, 40.0), (32.5, 25.0));
    assert_eq!(armadillo_clamped_head_look(-50.0, -40.0), (-32.5, -22.5));
    assert_eq!(armadillo_clamped_head_look(10.0, 5.0), (10.0, 5.0));
}

#[test]
fn armadillo_head_follows_the_clamped_look_while_not_hiding() {
    // The head pivot is `body` (root child 0) → `head` (child 1, after the tail). Emitted depth-first,
    // the body's two cubes are vertices [0, 48), the tail [48, 72), then the head subtree — head cube
    // and the two ear planes — is [72, 144), and the four legs are [144, 240). A non-zero look (above
    // the clamp on yaw) re-poses only the head subtree; the body, tail, and legs stay at bind.
    let base = EntityModelInstance::armadillo(74, [0.0, 64.0, 0.0], 0.0, false, false);
    let rest = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(35.0, -20.0)]);
    assert_eq!(rest.vertices.len(), looking.vertices.len());
    assert_ne!(
        rest.vertices[72..144],
        looking.vertices[72..144],
        "the head, snout, and ears turn with the look"
    );
    assert_eq!(
        rest.vertices[..72],
        looking.vertices[..72],
        "the body and tail stay at bind"
    );
    assert_eq!(
        rest.vertices[144..],
        looking.vertices[144..],
        "the four legs stay at bind"
    );
}

#[test]
fn armadillo_ignores_the_look_while_hiding_in_shell() {
    // While `isHidingInShell`, `setupAnim` skips the head look entirely (the head is balled up), so a
    // rolled-up armadillo renders identically regardless of the look angles.
    let base = EntityModelInstance::armadillo(75, [0.0, 64.0, 0.0], 0.0, false, true);
    let rolled = entity_model_mesh(&[base]);
    let rolled_looking = entity_model_mesh(&[base.with_head_look(35.0, -20.0)]);
    assert_eq!(
        rolled.vertices, rolled_looking.vertices,
        "the rolled-up armadillo ignores the look"
    );
}

#[test]
fn armadillo_walk_animation_matches_vanilla_definition() {
    // Vanilla `ArmadilloAnimation.ARMADILLO_WALK`: 1.4583 s looping, animating body, tail, the four
    // legs, and the head — seven bones, 82 keyframes total.
    assert_eq!(ARMADILLO_WALK.length_seconds, 1.4583);
    assert!(ARMADILLO_WALK.looping);
    assert_eq!(ARMADILLO_WALK.bones.len(), 7);
    let keyframes: usize = ARMADILLO_WALK
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 82);

    // The hind legs start a half-cycle apart: at t=0 the right hind leg pitches `-50°` and the left
    // hind leg `+50°` (`degreeVec(±50, 0, 0)`).
    let (_, rhl_rot) = sample_bone_offsets(&ARMADILLO_WALK, "right_hind_leg", 0.0, 1.0);
    let (_, lhl_rot) = sample_bone_offsets(&ARMADILLO_WALK, "left_hind_leg", 0.0, 1.0);
    assert!((rhl_rot[0] - (-50.0_f32).to_radians()).abs() < 1.0e-5);
    assert!((lhl_rot[0] - 50.0_f32.to_radians()).abs() < 1.0e-5);

    // The right hind leg's position channel reaches `posVec(0, 0, -0.5)` at its t=0.25 keyframe.
    let (rhl_pos, _) = sample_bone_offsets(&ARMADILLO_WALK, "right_hind_leg", 0.25, 1.0);
    assert!((rhl_pos[2] - -0.5).abs() < 1.0e-5);

    // The `body` z-sway is CatmullRom: at its t=0.2917 keyframe it reaches `degreeVec(0, 0, 6.81)`.
    let (_, body_rot) = sample_bone_offsets(&ARMADILLO_WALK, "body", 0.2917, 1.0);
    assert!(
        (body_rot[2] - 6.81_f32.to_radians()).abs() < 1.0e-4,
        "body z-roll was {}",
        body_rot[2]
    );
}

#[test]
fn armadillo_walk_moves_the_limbs_and_composes_with_the_look() {
    // A still adult (walk speed 0) samples the cycle at amplitude 0, collapsing to the bind pose; a
    // walking adult samples ARMADILLO_WALK across the body, tail, four legs, and head. The vertex
    // count is preserved.
    let still = entity_model_mesh(&[EntityModelInstance::armadillo(
        76,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    let walking = entity_model_mesh(&[EntityModelInstance::armadillo(
        77,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )
    .with_walk_animation(5.0, 1.0)]);
    assert_eq!(still.vertices.len(), walking.vertices.len());
    assert_ne!(
        still.vertices, walking.vertices,
        "the walking armadillo rocks its body and legs"
    );

    // The head walk roll ADDS onto the look, so a walking + looking adult differs from one that only
    // walks — and ONLY across the head subtree [72, 144); the body, tail, and legs share the walk.
    let walking_looking = entity_model_mesh(&[EntityModelInstance::armadillo(
        78,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )
    .with_walk_animation(5.0, 1.0)
    .with_head_look(30.0, -15.0)]);
    assert_ne!(
        walking.vertices[72..144],
        walking_looking.vertices[72..144],
        "the look composes onto the walking head"
    );
    assert_eq!(
        walking.vertices[..72],
        walking_looking.vertices[..72],
        "the body and tail share the same walk regardless of the look"
    );
    assert_eq!(
        walking.vertices[144..],
        walking_looking.vertices[144..],
        "the legs share the same walk regardless of the look"
    );
}

#[test]
fn baby_armadillo_walk_animation_matches_vanilla_definition() {
    // Vanilla `BabyArmadilloAnimation.ARMADILLO_BABY_WALK`: 1.4583 s looping, the same seven bones and
    // 82 keyframes as the adult, with slightly different keyframe timestamps.
    assert_eq!(ARMADILLO_BABY_WALK.length_seconds, 1.4583);
    assert!(ARMADILLO_BABY_WALK.looping);
    assert_eq!(ARMADILLO_BABY_WALK.bones.len(), 7);
    let keyframes: usize = ARMADILLO_BABY_WALK
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(keyframes, 82);

    // The baby `body` z-sway is CatmullRom: at its t=0.3 keyframe it reaches `degreeVec(0, 0, 6.81)`.
    let (_, body_rot) = sample_bone_offsets(&ARMADILLO_BABY_WALK, "body", 0.3, 1.0);
    assert!(
        (body_rot[2] - 6.81_f32.to_radians()).abs() < 1.0e-4,
        "baby body z-roll was {}",
        body_rot[2]
    );

    // The hind legs start a half-cycle apart, same as the adult (`degreeVec(±50, 0, 0)` at t=0).
    let (_, rhl_rot) = sample_bone_offsets(&ARMADILLO_BABY_WALK, "right_hind_leg", 0.0, 1.0);
    let (_, lhl_rot) = sample_bone_offsets(&ARMADILLO_BABY_WALK, "left_hind_leg", 0.0, 1.0);
    assert!((rhl_rot[0] - (-50.0_f32).to_radians()).abs() < 1.0e-5);
    assert!((lhl_rot[0] - 50.0_f32.to_radians()).abs() < 1.0e-5);
}

#[test]
fn baby_armadillo_walk_moves_the_limbs_and_composes_with_the_look() {
    // The baby shares the adult's `body → tail/head` + four-leg topology, so its head subtree is the
    // same [72, 144) span. A still baby collapses to the bind pose; a walking baby samples
    // ARMADILLO_BABY_WALK, and the head walk roll composes onto the look.
    let still = entity_model_mesh(&[EntityModelInstance::armadillo(
        79,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
    )]);
    let walking = entity_model_mesh(&[EntityModelInstance::armadillo(
        80,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
    )
    .with_walk_animation(5.0, 1.0)]);
    assert_eq!(still.vertices.len(), walking.vertices.len());
    assert_ne!(
        still.vertices, walking.vertices,
        "the walking baby rocks its body and legs"
    );

    let walking_looking = entity_model_mesh(&[EntityModelInstance::armadillo(
        81,
        [0.0, 64.0, 0.0],
        0.0,
        true,
        false,
    )
    .with_walk_animation(5.0, 1.0)
    .with_head_look(30.0, -15.0)]);
    assert_ne!(
        walking.vertices[72..144],
        walking_looking.vertices[72..144],
        "the look composes onto the walking baby head"
    );
    assert_eq!(
        walking.vertices[..72],
        walking_looking.vertices[..72],
        "the body and tail share the same walk regardless of the look"
    );
    assert_eq!(
        walking.vertices[144..],
        walking_looking.vertices[144..],
        "the legs share the same walk regardless of the look"
    );
}
