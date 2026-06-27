use super::*;

#[test]
fn adult_armadillo_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultArmadilloModel.createBodyLayer` (atlas 64×64): the root parents the body and
    // the four legs directly; the body parents the tail and head, and the head parents the head
    // cube and the two ear pivots.

    // `body` (offset (0, 21, 4)): a `CubeDeformation(0.3)` shell (`texOffs(0,20)`) over the bare
    // 8×8×12 box (`texOffs(0,40)`); both `uv_size` stay the integer dims (8, 8, 12).
    assert_eq!(ADULT_ARMADILLO_BODY_POSE.offset, [0.0, 21.0, 4.0]);
    assert_eq!(ADULT_ARMADILLO_BODY_CUBES.len(), 2);
    assert_eq!(ADULT_ARMADILLO_BODY_CUBES[0].min, [-4.3, -7.3, -10.3]);
    assert_eq!(ADULT_ARMADILLO_BODY_CUBES[0].size, [8.6, 8.6, 12.6]);
    assert_eq!(ADULT_ARMADILLO_BODY_CUBES[0].uv_size, [8.0, 8.0, 12.0]);
    assert_eq!(ADULT_ARMADILLO_BODY_CUBES[0].tex, [0.0, 20.0]);
    assert_eq!(ADULT_ARMADILLO_BODY_CUBES[1].min, [-4.0, -7.0, -10.0]);
    assert_eq!(ADULT_ARMADILLO_BODY_CUBES[1].size, [8.0, 8.0, 12.0]);
    assert_eq!(ADULT_ARMADILLO_BODY_CUBES[1].tex, [0.0, 40.0]);

    // `tail`: the 1×6×1 plume (`texOffs(44,53)`), pitched down by 0.5061 rad.
    assert_eq!(ADULT_ARMADILLO_TAIL_POSE.offset, [0.0, -3.0, 1.0]);
    assert_eq!(ADULT_ARMADILLO_TAIL_POSE.rotation, [0.5061, 0.0, 0.0]);
    assert_eq!(ADULT_ARMADILLO_TAIL_CUBES[0].size, [1.0, 6.0, 1.0]);
    assert_eq!(ADULT_ARMADILLO_TAIL_CUBES[0].tex, [44.0, 53.0]);

    // `head` (offset (0, -2, -11)): a bare pivot parenting the head cube and the two ears.
    assert_eq!(ADULT_ARMADILLO_HEAD_POSE.offset, [0.0, -2.0, -11.0]);

    // `head_cube`: the 3×5×2 snout (`texOffs(43,15)`), pitched up by -0.3927 rad.
    assert_eq!(ADULT_ARMADILLO_HEAD_CUBE_POSE.rotation, [-0.3927, 0.0, 0.0]);
    assert_eq!(ADULT_ARMADILLO_HEAD_CUBES[0].size, [3.0, 5.0, 2.0]);
    assert_eq!(ADULT_ARMADILLO_HEAD_CUBES[0].tex, [43.0, 15.0]);

    // The two ear pivots and their rotated 2×5×0 ear planes (`texOffs(43,10)` / `texOffs(47,10)`).
    assert_eq!(ADULT_ARMADILLO_RIGHT_EAR_POSE.offset, [-1.0, -1.0, 0.0]);
    assert_eq!(
        ADULT_ARMADILLO_RIGHT_EAR_CUBE_POSE.rotation,
        [0.1886, -0.3864, -0.0718]
    );
    assert_eq!(ADULT_ARMADILLO_RIGHT_EAR_CUBES[0].min, [-2.0, -3.0, 0.0]);
    assert_eq!(ADULT_ARMADILLO_RIGHT_EAR_CUBES[0].tex, [43.0, 10.0]);
    assert_eq!(ADULT_ARMADILLO_LEFT_EAR_POSE.offset, [1.0, -2.0, 0.0]);
    assert_eq!(
        ADULT_ARMADILLO_LEFT_EAR_CUBE_POSE.rotation,
        [0.1886, 0.3864, 0.0718]
    );
    assert_eq!(ADULT_ARMADILLO_LEFT_EAR_CUBES[0].size, [2.0, 5.0, 0.0]);
    assert_eq!(ADULT_ARMADILLO_LEFT_EAR_CUBES[0].tex, [47.0, 10.0]);

    // The four 2×3×2 legs at the corner pivots draw distinct UV regions (none are mirrors): the
    // right/left hind `texOffs(51,31)`/`(42,31)`, the right/left front `texOffs(51,43)`/`(42,43)`.
    assert_eq!(
        ADULT_ARMADILLO_RIGHT_HIND_LEG_POSE.offset,
        [-2.0, 21.0, 4.0]
    );
    assert_eq!(
        ADULT_ARMADILLO_RIGHT_HIND_LEG_CUBES[0].size,
        [2.0, 3.0, 2.0]
    );
    assert_eq!(ADULT_ARMADILLO_RIGHT_HIND_LEG_CUBES[0].tex, [51.0, 31.0]);
    assert_eq!(ADULT_ARMADILLO_LEFT_HIND_LEG_POSE.offset, [2.0, 21.0, 4.0]);
    assert_eq!(ADULT_ARMADILLO_LEFT_HIND_LEG_CUBES[0].tex, [42.0, 31.0]);
    assert_eq!(
        ADULT_ARMADILLO_RIGHT_FRONT_LEG_POSE.offset,
        [-2.0, 21.0, -4.0]
    );
    assert_eq!(ADULT_ARMADILLO_RIGHT_FRONT_LEG_CUBES[0].tex, [51.0, 43.0]);
    assert_eq!(
        ADULT_ARMADILLO_LEFT_FRONT_LEG_POSE.offset,
        [2.0, 21.0, -4.0]
    );
    assert_eq!(ADULT_ARMADILLO_LEFT_FRONT_LEG_CUBES[0].tex, [42.0, 43.0]);
}

#[test]
fn baby_armadillo_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyArmadilloModel.createBodyLayer` (atlas 64×64): smaller geometry, the ears
    // parented to the head cube, and the front legs at swapped X origins.
    // The shell box `texOffs(0,0)` keeps the integer dims (5, 4, 7); the bare box `texOffs(0,11)`.
    assert_eq!(BABY_ARMADILLO_BODY_POSE.offset, [0.0, 20.0, 0.5]);
    assert_eq!(BABY_ARMADILLO_BODY_CUBES[0].min, [-2.8, -2.3, -3.8]);
    assert_eq!(BABY_ARMADILLO_BODY_CUBES[0].size, [5.6, 4.6, 7.6]);
    assert_eq!(BABY_ARMADILLO_BODY_CUBES[0].uv_size, [5.0, 4.0, 7.0]);
    assert_eq!(BABY_ARMADILLO_BODY_CUBES[0].tex, [0.0, 0.0]);
    assert_eq!(BABY_ARMADILLO_BODY_CUBES[1].size, [5.0, 4.0, 6.0]);
    assert_eq!(BABY_ARMADILLO_BODY_CUBES[1].tex, [0.0, 11.0]);

    // `tail` pivot (offset (0, 0, 3.4)) parents the 1×1×4 stub (`texOffs(22,11)`) pitched by
    // -1.0472 rad.
    assert_eq!(BABY_ARMADILLO_TAIL_POSE.offset, [0.0, 0.0, 3.4]);
    assert_eq!(BABY_ARMADILLO_TAIL_CUBE_POSE.rotation, [-1.0472, 0.0, 0.0]);
    assert_eq!(BABY_ARMADILLO_TAIL_CUBES[0].size, [1.0, 1.0, 4.0]);
    assert_eq!(BABY_ARMADILLO_TAIL_CUBES[0].tex, [22.0, 11.0]);

    // `head` pivot parents the head cube (`texOffs(20,17)`, pitched up 0.7417649 rad) which parents
    // the two ears. Both ears share `texOffs(28,8)`; the right ear is added with `mirror()`.
    assert_eq!(BABY_ARMADILLO_HEAD_POSE.offset, [0.0, 0.0, -3.2]);
    assert_eq!(
        BABY_ARMADILLO_HEAD_CUBE_POSE.rotation,
        [0.7417649, 0.0, 0.0]
    );
    assert_eq!(BABY_ARMADILLO_HEAD_CUBES[0].size, [2.0, 2.0, 4.0]);
    assert_eq!(BABY_ARMADILLO_HEAD_CUBES[0].tex, [20.0, 17.0]);
    assert_eq!(
        BABY_ARMADILLO_RIGHT_EAR_POSE.rotation,
        [-0.4363, -0.1134, 0.0524]
    );
    assert_eq!(
        BABY_ARMADILLO_LEFT_EAR_POSE.rotation,
        [-0.4363, 0.1134, -0.0524]
    );
    assert_eq!(BABY_ARMADILLO_RIGHT_EAR_CUBES[0].size, [2.0, 3.0, 0.0]);
    assert_eq!(BABY_ARMADILLO_RIGHT_EAR_CUBES[0].tex, [28.0, 8.0]);
    assert!(BABY_ARMADILLO_RIGHT_EAR_CUBES[0].mirror);
    assert_eq!(BABY_ARMADILLO_LEFT_EAR_CUBES[0].tex, [28.0, 8.0]);
    assert!(!BABY_ARMADILLO_LEFT_EAR_CUBES[0].mirror);

    // The front legs carry vanilla's swapped X origins (right at +1.5, left at -1.5). The four legs
    // draw distinct UV regions AND mirror flags: the right/left hind `texOffs(20,27)` mirrored /
    // `texOffs(20,27)`, the right front `texOffs(20,23)`, the left front `texOffs(24,0)` mirrored.
    assert_eq!(
        BABY_ARMADILLO_RIGHT_FRONT_LEG_POSE.offset,
        [-1.5, 22.0, 2.5]
    );
    assert_eq!(BABY_ARMADILLO_LEFT_FRONT_LEG_POSE.offset, [1.5, 22.0, 2.5]);
    assert_eq!(BABY_ARMADILLO_RIGHT_HIND_LEG_POSE.offset, [1.5, 22.0, -1.5]);
    assert_eq!(BABY_ARMADILLO_LEFT_HIND_LEG_POSE.offset, [-1.5, 22.0, -1.5]);
    assert_eq!(BABY_ARMADILLO_RIGHT_HIND_LEG_CUBES[0].size, [2.0, 2.0, 2.0]);
    assert_eq!(BABY_ARMADILLO_RIGHT_HIND_LEG_CUBES[0].tex, [20.0, 27.0]);
    assert!(BABY_ARMADILLO_RIGHT_HIND_LEG_CUBES[0].mirror);
    assert_eq!(BABY_ARMADILLO_LEFT_HIND_LEG_CUBES[0].tex, [20.0, 27.0]);
    assert!(!BABY_ARMADILLO_LEFT_HIND_LEG_CUBES[0].mirror);
    assert_eq!(BABY_ARMADILLO_RIGHT_FRONT_LEG_CUBES[0].tex, [20.0, 23.0]);
    assert!(!BABY_ARMADILLO_RIGHT_FRONT_LEG_CUBES[0].mirror);
    assert_eq!(BABY_ARMADILLO_LEFT_FRONT_LEG_CUBES[0].tex, [24.0, 0.0]);
    assert!(BABY_ARMADILLO_LEFT_FRONT_LEG_CUBES[0].mirror);
}

#[test]
fn armadillo_rolled_up_parts_match_vanilla_hiding_in_shell() {
    // Vanilla `ArmadilloModel.setupAnim` `isHidingInShell`: the body cubes (`skipDraw`), the tail,
    // and both hind legs hide; the head (+ ears), both front legs, and the `cube` ball show.
    // So the rolled-up tree is: body pivot (no cubes) → head only; the two front legs; the ball →
    // head_cube + 2 ears + 2 front legs + 1 ball = 6 cubes. The hiding body keeps its pivot offset.
    assert_eq!(ADULT_ARMADILLO_BODY_POSE.offset, [0.0, 21.0, 4.0]);
    assert_eq!(ADULT_ARMADILLO_HEAD_POSE.offset, [0.0, -2.0, -11.0]);

    // The two FRONT legs (z = -4) stay; the hind legs (z = +4) are gone.
    assert_eq!(
        ADULT_ARMADILLO_RIGHT_FRONT_LEG_POSE.offset,
        [-2.0, 21.0, -4.0]
    );
    assert_eq!(
        ADULT_ARMADILLO_LEFT_FRONT_LEG_POSE.offset,
        [2.0, 21.0, -4.0]
    );

    // The shell-ball `cube` (root child at (0, 24, 0)): a plain 10×10×10 box.
    assert_eq!(ADULT_ARMADILLO_BALL_POSE.offset, [0.0, 24.0, 0.0]);
    assert_eq!(ADULT_ARMADILLO_BALL_CUBES[0].min, [-5.0, -10.0, -6.0]);
    assert_eq!(ADULT_ARMADILLO_BALL_CUBES[0].size, [10.0, 10.0, 10.0]);

    // Baby: same swap; the ball is the 6×6×6 box + CubeDeformation(0.3) → min -3.3, size 6.6.
    assert_eq!(BABY_ARMADILLO_BALL_POSE.offset, [0.0, 20.7, 0.5]);
    assert_eq!(BABY_ARMADILLO_BALL_CUBES[0].min, [-3.3, -3.3, -3.3]);
    assert_eq!(BABY_ARMADILLO_BALL_CUBES[0].size, [6.6, 6.6, 6.6]);
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
fn armadillo_roll_up_and_out_animations_match_vanilla_definitions() {
    // Vanilla `ArmadilloAnimation.ARMADILLO_ROLL_UP`: 0.5 s non-looping. The applied bone set is the
    // seven rest-tree bones (body, tail, head, four legs); the `cube` channel is omitted (the ball is
    // rendered statically). 58 keyframes across those bones.
    assert_eq!(ARMADILLO_ROLL_UP.length_seconds, 0.5);
    assert!(!ARMADILLO_ROLL_UP.looping);
    assert_eq!(ARMADILLO_ROLL_UP.bones.len(), 7);
    let roll_up_keyframes: usize = ARMADILLO_ROLL_UP
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(roll_up_keyframes, 58);
    // The head whips down to `degreeVec(-72.5, 0, 0)` at its t=0.25 keyframe as the armadillo balls up.
    let (_, head_rot) = sample_bone_offsets(&ARMADILLO_ROLL_UP, "head", 0.25, 1.0);
    assert!((head_rot[0] - (-72.5_f32).to_radians()).abs() < 1.0e-4);

    // Vanilla `ARMADILLO_ROLL_OUT`: 1.5 s non-looping, six applied bones (head, four legs, body — the
    // body channel is POSITION-only; the `cube` channel is omitted). 103 keyframes.
    assert_eq!(ARMADILLO_ROLL_OUT.length_seconds, 1.5);
    assert!(!ARMADILLO_ROLL_OUT.looping);
    assert_eq!(ARMADILLO_ROLL_OUT.bones.len(), 6);
    let roll_out_keyframes: usize = ARMADILLO_ROLL_OUT
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(roll_out_keyframes, 103);
    // The body springs up to `posVec(0, 5, 0)` at its t=1.25 keyframe as it un-balls (`pos_vec`
    // negates Y to the model's coordinate convention, so the offset is `-5`).
    let (body_pos, _) = sample_bone_offsets(&ARMADILLO_ROLL_OUT, "body", 1.25, 1.0);
    assert!((body_pos[1] - -5.0).abs() < 1.0e-4);

    // Vanilla `ARMADILLO_PEEK`: 2.5 s non-looping, six bones (head, four legs, shell `cube`) and
    // 82 keyframes. The shell `cube` is included because vanilla still applies peek while the
    // armadillo is hiding in its shell.
    assert_eq!(ARMADILLO_PEEK.length_seconds, 2.5);
    assert!(!ARMADILLO_PEEK.looping);
    assert_eq!(ARMADILLO_PEEK.bones.len(), 6);
    let peek_keyframes: usize = ARMADILLO_PEEK
        .bones
        .iter()
        .flat_map(|bone| bone.channels.iter())
        .map(|channel| channel.keyframes.len())
        .sum();
    assert_eq!(peek_keyframes, 82);
    let (_, head_rot) = sample_bone_offsets(&ARMADILLO_PEEK, "head", 1.3, 1.0);
    assert!((head_rot[2] - (-39.1287_f32).to_radians()).abs() < 1.0e-4);
    let (cube_pos, cube_rot) = sample_bone_offsets(&ARMADILLO_PEEK, "cube", 2.15, 1.0);
    assert!((cube_pos[1] - -1.7).abs() < 1.0e-4);
    assert!((cube_rot[0] - (-25.0_f32).to_radians()).abs() < 1.0e-4);
}

#[test]
fn armadillo_rolling_re_poses_off_the_bind_pose_before_the_ball_takes_over() {
    // During ROLLING's first ~5 ticks the armadillo is NOT yet hiding (`shouldHideInShell` flips at
    // tick 5), so it renders the full rest tree with the roll-up keyframe ADDED onto the walk/bind
    // pose — the body curls in. A rolling armadillo therefore re-poses off the bind pose without
    // changing the cube count (10 cubes / 240 vertices).
    let rest = entity_model_mesh(&[EntityModelInstance::armadillo(
        82,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    let rolling = entity_model_mesh(&[EntityModelInstance::armadillo(
        83,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )
    .with_armadillo_roll_up_seconds(0.2)]);
    assert_eq!(
        rest.vertices.len(),
        rolling.vertices.len(),
        "the roll-up re-poses parts, it does not add or hide cubes"
    );
    assert_ne!(
        rest.vertices, rolling.vertices,
        "the rolling armadillo curls off the bind pose before the ball takes over"
    );
    // Advancing the roll-up's elapsed seconds advances the curl.
    let rolling_later = entity_model_mesh(&[EntityModelInstance::armadillo(
        84,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )
    .with_armadillo_roll_up_seconds(0.4)]);
    assert_ne!(
        rolling.vertices, rolling_later.vertices,
        "the curl-in advances as the roll-up elapsed seconds climb"
    );
}

#[test]
fn armadillo_unrolling_re_poses_off_the_bind_pose() {
    // Once UNROLLING un-hides (inStateTicks >= 26) the rest tree shows again with the roll-out
    // keyframe ADDED, so the body un-curls off the bind pose (same cube count).
    let rest = entity_model_mesh(&[EntityModelInstance::armadillo(
        85,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    let unrolling = entity_model_mesh(&[EntityModelInstance::armadillo(
        86,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )
    .with_armadillo_roll_out_seconds(1.35)]);
    assert_eq!(rest.vertices.len(), unrolling.vertices.len());
    assert_ne!(
        rest.vertices, unrolling.vertices,
        "the unrolling armadillo un-curls off the bind pose"
    );
}

#[test]
fn armadillo_hidden_applies_visible_roll_keyframes() {
    // While `isHidingInShell` (the `rolled_up` tree), vanilla still applies roll-up / roll-out
    // keyframes after the visibility swap. bbb's rolled tree omits the hidden body/tail/hind legs
    // and the roll `cube` channels, but it still re-poses the visible head and front legs.
    let hidden = entity_model_mesh(&[EntityModelInstance::armadillo(
        87,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )]);
    let hidden_rolling = entity_model_mesh(&[EntityModelInstance::armadillo(
        88,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )
    .with_armadillo_roll_up_seconds(0.2)]);
    // Six cubes → 36 faces / 144 vertices: the shell ball, head, ears, two front legs.
    assert_eq!(hidden.vertices.len(), 144);
    assert_eq!(hidden.vertices.len(), hidden_rolling.vertices.len());
    assert_ne!(
        hidden.vertices, hidden_rolling.vertices,
        "hidden armadillo roll keyframes move the visible head/front legs"
    );
}

#[test]
fn armadillo_peek_re_poses_visible_parts_even_while_hidden() {
    // Vanilla `ArmadilloModel.setupAnim` applies `ARMADILLO_PEEK` after the hide/show swap. An
    // unhidden armadillo moves its head and all four legs; a hidden armadillo still moves the
    // visible head, front legs, and shell `cube`.
    let rest = entity_model_mesh(&[EntityModelInstance::armadillo(
        89,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )]);
    let peeking = entity_model_mesh(&[EntityModelInstance::armadillo(
        90,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        false,
    )
    .with_armadillo_peek_seconds(0.5)]);
    assert_eq!(rest.vertices.len(), peeking.vertices.len());
    assert_ne!(
        rest.vertices, peeking.vertices,
        "peek re-poses the expanded armadillo without changing cube count"
    );

    let hidden = entity_model_mesh(&[EntityModelInstance::armadillo(
        91,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )]);
    let hidden_peeking = entity_model_mesh(&[EntityModelInstance::armadillo(
        92,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
    )
    .with_armadillo_peek_seconds(2.15)]);
    assert_eq!(hidden.vertices.len(), hidden_peeking.vertices.len());
    assert_ne!(
        hidden.vertices, hidden_peeking.vertices,
        "peek moves the visible shell ball/head/front legs while hidden"
    );
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

#[test]
fn armadillo_textured_render_matches_vanilla_renderer() {
    // The adult and baby armadillo share the UV layout but bind their own 64×64 textures.
    for (baby, texture) in [
        (false, ARMADILLO_TEXTURE_REF),
        (true, ARMADILLO_BABY_TEXTURE_REF),
    ] {
        let passes = armadillo_textured_layer_passes(baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].kind, EntityModelLayerKind::ArmadilloBase);
        assert_eq!(
            passes[0].model_layer,
            if baby {
                MODEL_LAYER_ARMADILLO_BABY
            } else {
                MODEL_LAYER_ARMADILLO
            }
        );
        assert_eq!(passes[0].texture, texture);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
    }
    assert_eq!(
        EntityModelKind::Armadillo {
            baby: false,
            rolled_up: false,
        }
        .vanilla_texture_ref(),
        Some(ARMADILLO_TEXTURE_REF)
    );
    assert_eq!(
        EntityModelKind::Armadillo {
            baby: true,
            rolled_up: false,
        }
        .vanilla_texture_ref(),
        Some(ARMADILLO_BABY_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&ARMADILLO_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ARMADILLO_BABY_TEXTURE_REF));
    assert_eq!(
        armadillo_entity_texture_refs(),
        &[ARMADILLO_TEXTURE_REF, ARMADILLO_BABY_TEXTURE_REF]
    );

    let images: Vec<EntityModelTextureImage> = armadillo_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    // Both ages, both rolled/unrolled, emit textured geometry through a vanilla-shaped submission
    // before folding into the cutout mesh.
    for baby in [false, true] {
        for rolled_up in [false, true] {
            let instance =
                EntityModelInstance::armadillo(980, [0.0, 64.0, 0.0], 15.0, baby, rolled_up)
                    .with_armadillo_peek_seconds(if rolled_up { 2.15 } else { 0.5 })
                    .with_light_coords((5_u32 << 4) | (10_u32 << 20))
                    .with_white_overlay_progress(0.8)
                    .with_has_red_overlay(true);
            let meshes = entity_model_textured_meshes(&[instance], &atlas);
            assert!(
                !meshes.cutout.vertices.is_empty(),
                "baby={baby} rolled_up={rolled_up} emits textured geometry"
            );
            assert!(meshes
                .cutout
                .vertices
                .iter()
                .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
                    && vertex.light == instance.render_state.shader_light()
                    && vertex.overlay == instance.render_state.overlay_coords()));
            assert_eq!(meshes.submissions.len(), 1);
            let submit = meshes.submissions[0];
            assert_eq!(
                submit.texture,
                if baby {
                    ARMADILLO_BABY_TEXTURE_REF
                } else {
                    ARMADILLO_TEXTURE_REF
                }
            );
            assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
            assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
            assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
            assert_eq!((submit.order, submit.submit_sequence), (0, 0));
            assert_eq!(submit.light, instance.render_state.shader_light());
            assert_eq!(submit.overlay, instance.render_state.overlay_coords());
            assert_ne!(submit.overlay, [0.0, 10.0]);
            assert_eq!(submit.transform, entity_model_root_transform(instance));
        }
    }
}
