use super::*;

fn count_cubes(parts: &[ModelPartDesc]) -> usize {
    parts
        .iter()
        .map(|part| part.cubes.len() + count_cubes(part.children))
        .sum()
}

#[test]
fn parrot_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `ParrotModel.createBodyLayer` (atlas 32×32): seven sibling root parts — body, tail,
    // two wings, head (parenting head2, the two beak halves, and the crest feather), two legs.
    assert_eq!(PARROT_PARTS.len(), 7);

    // `body` (3×6×3) pitched by 0.4937 rad.
    let body = &PARROT_PARTS[0];
    assert_eq!(body.pose.offset, [0.0, 16.5, -3.0]);
    assert_eq!(body.pose.rotation, [0.4937, 0.0, 0.0]);
    assert_eq!(body.cubes[0].size, [3.0, 6.0, 3.0]);

    // `tail` (3×4×1) pitched by 1.015 rad.
    let tail = &PARROT_PARTS[1];
    assert_eq!(tail.pose.offset, [0.0, 21.07, 1.16]);
    assert_eq!(tail.pose.rotation, [1.015, 0.0, 0.0]);
    assert_eq!(tail.cubes[0].size, [3.0, 4.0, 1.0]);

    // The two 1×5×3 wings: mirrored pivots, both flipped yRot = -π.
    let left_wing = &PARROT_PARTS[2];
    let right_wing = &PARROT_PARTS[3];
    assert_eq!(left_wing.pose.offset, [1.5, 16.94, -2.76]);
    assert_eq!(
        left_wing.pose.rotation,
        [-0.6981, -std::f32::consts::PI, 0.0]
    );
    assert_eq!(right_wing.pose.offset, [-1.5, 16.94, -2.76]);
    assert_eq!(right_wing.cubes[0].size, [1.0, 5.0, 3.0]);

    // `head` (2×3×2) at offset (0, 15.69, -2.76), parenting four cubes.
    let head = &PARROT_PARTS[4];
    assert_eq!(head.pose.offset, [0.0, 15.69, -2.76]);
    assert_eq!(head.cubes[0].size, [2.0, 3.0, 2.0]);
    assert_eq!(head.children.len(), 4);
    // head2 2×1×4, beak1 / beak2 1×2×1, the crest feather 0×5×4 pitched by -0.2214 rad.
    assert_eq!(head.children[0].cubes[0].size, [2.0, 1.0, 4.0]);
    assert_eq!(head.children[1].cubes[0].size, [1.0, 2.0, 1.0]);
    assert_eq!(head.children[3].pose.rotation, [-0.2214, 0.0, 0.0]);
    assert_eq!(head.children[3].cubes[0].size, [0.0, 5.0, 4.0]);

    // The two 1×2×1 legs at the mirrored pivots, both pitched by -0.0299 rad.
    let left_leg = &PARROT_PARTS[5];
    let right_leg = &PARROT_PARTS[6];
    assert_eq!(left_leg.pose.offset, [1.0, 22.0, -1.05]);
    assert_eq!(left_leg.pose.rotation, [-0.0299, 0.0, 0.0]);
    assert_eq!(right_leg.pose.offset, [-1.0, 22.0, -1.05]);

    // Eleven cubes total.
    assert_eq!(count_cubes(&PARROT_PARTS), 11);
}

#[test]
fn parrot_sitting_pose_matches_vanilla_prepare() {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_6};

    // Standing keeps the bind pose unchanged.
    assert_eq!(parrot_pose_parts(false), PARROT_PARTS.to_vec());

    // SITTING = `ParrotModel.prepare(SITTING)`: every part raises `y += 1.9`, the tail pitches
    // `xRot += π/6`, the wings tuck to `zRot = ±0.0873`, and the legs fold `xRot += π/2`.
    let sitting = parrot_pose_parts(true);
    for (i, part) in sitting.iter().enumerate() {
        assert!(
            (part.pose.offset[1] - (PARROT_PARTS[i].pose.offset[1] + 1.9)).abs() < 1.0e-6,
            "part {i} should raise y by 1.9"
        );
    }
    // tail (index 1): xRot = 1.015 + π/6.
    assert!((sitting[1].pose.rotation[0] - (1.015 + FRAC_PI_6)).abs() < 1.0e-6);
    // wings (2 left, 3 right): zRot set to ∓0.0873.
    assert!((sitting[2].pose.rotation[2] - (-0.0873)).abs() < 1.0e-6);
    assert!((sitting[3].pose.rotation[2] - 0.0873).abs() < 1.0e-6);
    // legs (5 left, 6 right): xRot = -0.0299 + π/2.
    assert!((sitting[5].pose.rotation[0] - (-0.0299 + FRAC_PI_2)).abs() < 1.0e-6);
    assert!((sitting[6].pose.rotation[0] - (-0.0299 + FRAC_PI_2)).abs() < 1.0e-6);
    // `prepare(SITTING)` only translates the head (index 4); the look rotation is applied later in
    // `emit_parrot_model`, so the pose helper itself leaves the head rotation at bind.
    assert_eq!(sitting[4].pose.rotation, PARROT_PARTS[4].pose.rotation);
}

#[test]
fn parrot_head_look_turns_only_the_head_subtree() {
    // Vanilla `ParrotModel.setupAnim` sets `head.xRot/yRot` from the look angles before the
    // per-pose switch, so the head and its beak/crest children turn while the body, tail, wings,
    // and legs hold. Depth-first emit order: body/tail/wings `[0, 96)`, the head plus its four
    // children `[96, 216)`, then the two legs `[216, 264)`. Only the head subtree moves.
    let rest = EntityModelInstance::parrot(990, [0.0, 64.0, 0.0], 0.0);
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
    let standing = entity_model_mesh(&[EntityModelInstance::parrot(981, [0.0, 64.0, 0.0], 0.0)]);
    let sitting = entity_model_mesh(&[
        EntityModelInstance::parrot(982, [0.0, 64.0, 0.0], 0.0).with_parrot_sitting(true)
    ]);
    assert_eq!(standing.vertices.len(), sitting.vertices.len());
    assert_ne!(
        standing.vertices, sitting.vertices,
        "the sitting parrot perches lower with folded legs"
    );
}

#[test]
fn parrot_mesh_uses_vanilla_body_layer_geometry() {
    // The body carries the body tint; the two beak halves carry the beak tint.
    let parrot = entity_model_mesh(&[EntityModelInstance::parrot(980, [0.0, 64.0, 0.0], 0.0)]);
    assert!(parrot
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PARROT_BODY, 1.0)));
    assert!(parrot
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(PARROT_BEAK, 1.0)));
}
