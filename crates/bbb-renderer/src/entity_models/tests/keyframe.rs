use super::*;

const RAD: f32 = std::f32::consts::PI / 180.0;

#[test]
fn keyframe_vec_helpers_match_vanilla() {
    // Vanilla `KeyframeAnimations.posVec` negates the y axis.
    assert_eq!(pos_vec(1.0, 2.0, 3.0), [1.0, -2.0, 3.0]);
    // `degreeVec` converts degrees to radians.
    let deg = degree_vec(90.0, 45.0, -180.0);
    assert!((deg[0] - 90.0 * RAD).abs() < 1.0e-6);
    assert!((deg[1] - 45.0 * RAD).abs() < 1.0e-6);
    assert!((deg[2] - -180.0 * RAD).abs() < 1.0e-6);
}

#[test]
fn keyframe_elapsed_seconds_wraps_only_when_looping() {
    let looping = AnimationDefinition {
        length_seconds: 0.5,
        looping: true,
        bones: &[],
    };
    let once = AnimationDefinition {
        length_seconds: 0.5,
        looping: false,
        bones: &[],
    };
    assert!((keyframe_elapsed_seconds(&looping, 0.6) - 0.1).abs() < 1.0e-6);
    assert!((keyframe_elapsed_seconds(&looping, 1.25) - 0.25).abs() < 1.0e-6);
    assert!((keyframe_elapsed_seconds(&once, 0.6) - 0.6).abs() < 1.0e-6);
}

#[test]
fn sample_keyframe_channel_linear_matches_vanilla() {
    // A 3-keyframe triangle wave: 0 → 10 → 0 over 0..1s, all LINEAR.
    const KEYS: [Keyframe; 3] = [
        keyframe(0.0, [0.0, 0.0, 0.0], KeyframeInterpolation::Linear),
        keyframe(0.5, [10.0, 0.0, 0.0], KeyframeInterpolation::Linear),
        keyframe(1.0, [0.0, 0.0, 0.0], KeyframeInterpolation::Linear),
    ];

    // At each keyframe the value is exact; between them it lerps on the clamped alpha.
    assert_eq!(sample_keyframe_channel(&KEYS, 0.0, 1.0)[0], 0.0);
    assert_eq!(sample_keyframe_channel(&KEYS, 0.25, 1.0)[0], 5.0);
    assert_eq!(sample_keyframe_channel(&KEYS, 0.5, 1.0)[0], 10.0);
    assert_eq!(sample_keyframe_channel(&KEYS, 0.75, 1.0)[0], 5.0);
    assert_eq!(sample_keyframe_channel(&KEYS, 1.0, 1.0)[0], 0.0);

    // The target scale multiplies the result (vanilla `point0.lerp(point1, alpha).mul(scale)`).
    assert_eq!(sample_keyframe_channel(&KEYS, 0.25, 2.0)[0], 10.0);

    // Before the first / after the last keyframe the value clamps to the end keyframe.
    assert_eq!(sample_keyframe_channel(&KEYS, -1.0, 1.0)[0], 0.0);
    assert_eq!(sample_keyframe_channel(&KEYS, 2.0, 1.0)[0], 0.0);
}

#[test]
fn sample_keyframe_channel_catmullrom_matches_vanilla() {
    // The breeze head bob: a 3-keyframe `0 → 1 → 0` spline, all CATMULLROM.
    const KEYS: [Keyframe; 3] = [
        keyframe(0.0, [0.0, 0.0, 0.0], KeyframeInterpolation::CatmullRom),
        keyframe(1.0, [0.0, 1.0, 0.0], KeyframeInterpolation::CatmullRom),
        keyframe(2.0, [0.0, 0.0, 0.0], KeyframeInterpolation::CatmullRom),
    ];

    // The spline passes through each keyframe's `postTarget` exactly.
    assert!((sample_keyframe_channel(&KEYS, 0.0, 1.0)[1] - 0.0).abs() < 1.0e-6);
    assert!((sample_keyframe_channel(&KEYS, 1.0, 1.0)[1] - 1.0).abs() < 1.0e-6);
    assert!((sample_keyframe_channel(&KEYS, 2.0, 1.0)[1] - 0.0).abs() < 1.0e-6);

    // Midway through the first segment, vanilla `Mth.catmullrom(0.5, p0=0, p1=0, p2=1, p3=0)`:
    // `0.5·(0 + 0.5 + 4·0.25 − 3·0.125) = 0.5·1.125 = 0.5625` (`p0` clamps to `keyframes[0]`).
    let mid = sample_keyframe_channel(&KEYS, 0.5, 1.0)[1];
    assert!((mid - 0.5625).abs() < 1.0e-6, "got {mid}");

    // The target scale multiplies the cubic result.
    assert!((sample_keyframe_channel(&KEYS, 0.5, 2.0)[1] - 1.125).abs() < 1.0e-6);
}

#[test]
fn sample_bone_offsets_reads_bat_flying_definition() {
    // At t=0 the flying body holds `degreeVec(40, 0, 0)` and the wings ±85° yaw.
    let (body_pos, body_rot) = sample_bone_offsets(&BAT_FLYING, "body", 0.0, 1.0);
    assert_eq!(body_pos, [0.0, 0.0, 0.0]);
    assert!((body_rot[0] - 40.0 * RAD).abs() < 1.0e-6);

    let (_, right_wing_rot) = sample_bone_offsets(&BAT_FLYING, "right_wing", 0.0, 1.0);
    assert!((right_wing_rot[1] - 85.0 * RAD).abs() < 1.0e-6);
    let (_, left_wing_rot) = sample_bone_offsets(&BAT_FLYING, "left_wing", 0.0, 1.0);
    assert!((left_wing_rot[1] - -85.0 * RAD).abs() < 1.0e-6);

    // The flying head position peaks at `posVec(0, 2, 0)` (y negated) at t=0.125.
    let (head_pos, _) = sample_bone_offsets(&BAT_FLYING, "head", 0.125, 1.0);
    assert!((head_pos[1] - -2.0).abs() < 1.0e-6);

    // A bone with no channel in the definition contributes no offset.
    let (missing_pos, missing_rot) = sample_bone_offsets(&BAT_FLYING, "nonexistent", 0.0, 1.0);
    assert_eq!(missing_pos, [0.0, 0.0, 0.0]);
    assert_eq!(missing_rot, [0.0, 0.0, 0.0]);
}

#[test]
fn sample_bone_offsets_reads_bat_resting_definition() {
    // `BAT_RESTING` is a static single-keyframe pose: the head and body flip 180° about X (and
    // shift `posVec(0, 0.5, 0)`, y negated to -0.5) so the bat hangs upside down, the wings
    // fold (±10° yaw plus `posVec(0, 0, 1)`), and the wing tips fold hard (∓120° yaw).
    let (head_pos, head_rot) = sample_bone_offsets(&BAT_RESTING, "head", 0.0, 1.0);
    assert!((head_rot[0] - 180.0 * RAD).abs() < 1.0e-6);
    assert_eq!(head_rot[1], 0.0);
    assert!((head_pos[1] - -0.5).abs() < 1.0e-6);
    let (body_pos, body_rot) = sample_bone_offsets(&BAT_RESTING, "body", 0.0, 1.0);
    assert!((body_rot[0] - 180.0 * RAD).abs() < 1.0e-6);
    assert!((body_pos[1] - -0.5).abs() < 1.0e-6);

    let (right_wing_pos, right_wing_rot) =
        sample_bone_offsets(&BAT_RESTING, "right_wing", 0.0, 1.0);
    assert!((right_wing_rot[1] - -10.0 * RAD).abs() < 1.0e-6);
    assert!((right_wing_pos[2] - 1.0).abs() < 1.0e-6);
    let (_, left_wing_rot) = sample_bone_offsets(&BAT_RESTING, "left_wing", 0.0, 1.0);
    assert!((left_wing_rot[1] - 10.0 * RAD).abs() < 1.0e-6);
    let (_, right_tip_rot) = sample_bone_offsets(&BAT_RESTING, "right_wing_tip", 0.0, 1.0);
    assert!((right_tip_rot[1] - -120.0 * RAD).abs() < 1.0e-6);
    let (_, left_tip_rot) = sample_bone_offsets(&BAT_RESTING, "left_wing_tip", 0.0, 1.0);
    assert!((left_tip_rot[1] - 120.0 * RAD).abs() < 1.0e-6);

    // The single keyframe makes the pose static — a later time samples the same values.
    let (_, head_rot_late) = sample_bone_offsets(&BAT_RESTING, "head", 0.4, 1.0);
    assert!((head_rot_late[0] - 180.0 * RAD).abs() < 1.0e-6);
}
