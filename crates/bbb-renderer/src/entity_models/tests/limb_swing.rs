use super::*;

use std::borrow::Cow;
use std::f32::consts::PI;

#[test]
fn limb_swing_leg_rotations_match_vanilla_quadruped_setup_anim() {
    // Vanilla QuadrupedModel.setupAnim, leg part order [rightHind, leftHind,
    // rightFront, leftFront]:
    //   rightHind.xRot = cos(pos * 0.6662) * 1.4 * speed
    //   leftHind.xRot  = cos(pos * 0.6662 + π) * 1.4 * speed
    //   rightFront.xRot= cos(pos * 0.6662 + π) * 1.4 * speed
    //   leftFront.xRot = cos(pos * 0.6662) * 1.4 * speed
    let base =
        EntityModelInstance::quadruped(1, [0.0, 64.0, 0.0], 0.0, QuadrupedModelFamily::Cow, false);

    // A standing entity (walkAnimationSpeed == 0) keeps every leg at rest.
    assert_eq!(quadruped_leg_x_rotations(base), [0.0; 4]);

    // pos = 0 → cos(0) = 1, cos(π) = -1, at full amplitude (speed = 1.0): the
    // hind-right / front-left pair and the hind-left / front-right pair are exactly
    // out of phase.
    let rots = quadruped_leg_x_rotations(base.with_walk_animation(0.0, 1.0));
    assert!((rots[0] - 1.4).abs() < 1e-5, "right hind: {}", rots[0]);
    assert!((rots[1] + 1.4).abs() < 1e-5, "left hind: {}", rots[1]);
    assert!((rots[2] + 1.4).abs() < 1e-5, "right front: {}", rots[2]);
    assert!((rots[3] - 1.4).abs() < 1e-5, "left front: {}", rots[3]);

    // A general (pos, speed) reproduces the `cos(pos * 0.6662 [+ π]) * 1.4 * speed`
    // formula, including the 0.6662 frequency factor.
    let rots = quadruped_leg_x_rotations(base.with_walk_animation(1.5, 0.5));
    let phase = 1.5_f32 * 0.6662;
    let in_phase = phase.cos() * 1.4 * 0.5;
    let out_of_phase = (phase + PI).cos() * 1.4 * 0.5;
    assert!((rots[0] - in_phase).abs() < 1e-5);
    assert!((rots[1] - out_of_phase).abs() < 1e-5);
    assert!((rots[2] - out_of_phase).abs() < 1e-5);
    assert!((rots[3] - in_phase).abs() < 1e-5);
}

#[test]
fn limb_swing_is_inert_at_zero_speed() {
    // walkAnimationSpeed == 0 multiplies every leg rotation to zero, so the mesh is
    // byte-identical to the un-animated render however far the swing position has
    // advanced.
    let base =
        EntityModelInstance::quadruped(2, [0.0, 64.0, 0.0], 0.0, QuadrupedModelFamily::Cow, false);
    let rest = entity_model_mesh(&[base]);
    let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);

    assert_eq!(rest.vertices, still.vertices);
}

#[test]
fn limb_swing_lifts_quadruped_feet_off_the_ground() {
    // Vanilla QuadrupedModel.setupAnim swings each straight-down leg about the X
    // axis, and rotating a vertical leg in either direction raises its foot, so a
    // walking quadruped is shorter (its lowest point rises) than when standing.
    let base =
        EntityModelInstance::quadruped(3, [0.0, 64.0, 0.0], 0.0, QuadrupedModelFamily::Cow, false);
    let rest = entity_model_mesh(&[base]);
    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);

    assert_ne!(rest.vertices, walking.vertices);

    let (rest_min, rest_max) = mesh_extents(&rest);
    let (walk_min, walk_max) = mesh_extents(&walking);
    let rest_height = rest_max[1] - rest_min[1];
    let walk_height = walk_max[1] - walk_min[1];
    assert!(
        walk_height < rest_height - 0.3,
        "walking height {walk_height} should be well under standing height {rest_height}"
    );
}

#[test]
fn quadruped_limb_swing_parts_assign_vanilla_leg_phases_by_offset() {
    // The adult cow body layer lists its legs hind-first: indices [2, 3, 4, 5] are
    // [rightHind, leftHind, rightFront, leftFront] with offsets
    // [-4,_,7], [4,_,7], [-4,_,-5], [4,_,-5]. With walkAnimationPos = 0 and
    // walkAnimationSpeed = 1, vanilla sets rightHind/leftFront to cos(0)*1.4 = 1.4
    // and leftHind/rightFront to cos(π)*1.4 = -1.4.
    let posed = quadruped_limb_swing_parts(
        Cow::Borrowed(&ADULT_COW_PARTS),
        QUADRUPED_LEG_PART_INDICES,
        0.0,
        1.0,
    );
    assert!(
        (posed[2].pose.rotation[0] - 1.4).abs() < 1e-5,
        "right hind in phase"
    );
    assert!(
        (posed[3].pose.rotation[0] + 1.4).abs() < 1e-5,
        "left hind out of phase"
    );
    assert!(
        (posed[4].pose.rotation[0] + 1.4).abs() < 1e-5,
        "right front out of phase"
    );
    assert!(
        (posed[5].pose.rotation[0] - 1.4).abs() < 1e-5,
        "left front in phase"
    );
    // The head and body parts are untouched by the leg swing.
    assert_eq!(posed[0].pose.rotation, [0.0, 0.0, 0.0]);
    assert_eq!(posed[1].pose.rotation, ADULT_COW_PARTS[1].pose.rotation);

    // The baby cow body layer lists its legs front-first: indices [2, 3, 4, 5] are
    // [rightFront, leftFront, rightHind, leftHind]. The offset-based phase keeps the
    // diagonal pairing correct despite the different ordering.
    let posed = quadruped_limb_swing_parts(
        Cow::Borrowed(&BABY_COW_PARTS),
        QUADRUPED_LEG_PART_INDICES,
        0.0,
        1.0,
    );
    assert!(
        (posed[2].pose.rotation[0] + 1.4).abs() < 1e-5,
        "baby right front out of phase"
    );
    assert!(
        (posed[3].pose.rotation[0] - 1.4).abs() < 1e-5,
        "baby left front in phase"
    );
    assert!(
        (posed[4].pose.rotation[0] - 1.4).abs() < 1e-5,
        "baby right hind in phase"
    );
    assert!(
        (posed[5].pose.rotation[0] + 1.4).abs() < 1e-5,
        "baby left hind out of phase"
    );
}

#[test]
fn quadruped_limb_swing_parts_borrow_unchanged_at_rest() {
    // walkAnimationSpeed == 0 leaves the static parts untouched (a borrow, no clone).
    let posed = quadruped_limb_swing_parts(
        Cow::Borrowed(&ADULT_COW_PARTS),
        QUADRUPED_LEG_PART_INDICES,
        2.5,
        0.0,
    );
    assert!(matches!(posed, Cow::Borrowed(_)));
}

#[test]
fn dedicated_cow_model_swings_its_legs_when_walking() {
    // The dedicated CowModel path (EntityModelKind::Cow, used by real cows with
    // their variants/horns) consumes the projected limb swing: a standing cow is
    // byte-identical with or without a swing position, and a walking cow's feet lift
    // (its lowest point rises), shortening the model.
    for baby in [false, true] {
        let base = EntityModelInstance::cow(5, [0.0, 64.0, 0.0], 0.0, baby);
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "baby={baby}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(
            rest.vertices, walking.vertices,
            "baby={baby}: walking differs"
        );

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.1,
            "baby={baby}: walking cow's feet should lift off the ground"
        );
    }
}
