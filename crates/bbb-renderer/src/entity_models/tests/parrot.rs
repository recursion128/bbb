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

    // Standing with a neutral gaze keeps every part at its bind pose: `setup_anim` applies only the
    // head look (identity at rest) and the walk swing (identity at rest).
    let mut standing = ParrotModel::new();
    standing.prepare(&EntityModelInstance::parrot(0, [0.0, 64.0, 0.0], 0.0));
    let standing_root = standing.root_mut();
    for (name, pose) in parts {
        let part = standing_root.child_mut(name);
        assert_eq!(part.pose.offset, pose.offset, "part {name} offset");
        assert_eq!(part.pose.rotation, pose.rotation, "part {name} rotation");
    }

    // SITTING = `ParrotModel.prepare(SITTING)`: every part raises `y += 1.9`, the tail pitches
    // `xRot += π/6`, the wings tuck to `zRot = ±0.0873`, and the legs fold `xRot += π/2`.
    let mut sitting = ParrotModel::new();
    sitting
        .prepare(&EntityModelInstance::parrot(0, [0.0, 64.0, 0.0], 0.0).with_parrot_sitting(true));
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
    // [0, 24), wings [48, 96), and head subtree [96, 216) hold. The wing flap / body bob need the
    // un-projected `flapAngle`, so the wings stay put.
    let rest = entity_model_mesh(&[EntityModelInstance::parrot(992, [0.0, 64.0, 0.0], 0.0)]);
    let walking = entity_model_mesh(&[
        EntityModelInstance::parrot(993, [0.0, 64.0, 0.0], 0.0).with_walk_animation(2.0, 1.0)
    ]);
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
        "the wings hold (flap is deferred)"
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
    let sit_rest = entity_model_mesh(&[
        EntityModelInstance::parrot(994, [0.0, 64.0, 0.0], 0.0).with_parrot_sitting(true)
    ]);
    let sit_walk = entity_model_mesh(&[EntityModelInstance::parrot(995, [0.0, 64.0, 0.0], 0.0)
        .with_parrot_sitting(true)
        .with_walk_animation(2.0, 1.0)]);
    assert_eq!(
        sit_rest.vertices, sit_walk.vertices,
        "a perched parrot is inert under walk animation"
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
