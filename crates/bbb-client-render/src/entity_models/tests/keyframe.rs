use crate::entity_models::keyframe::*;
use crate::entity_models::model_data_26_1::frog_animation;

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
fn keyframe_walk_sample_matches_vanilla_apply_walk() {
    // Vanilla `KeyframeAnimation.applyWalk(pos, speed, speedFactor, scaleFactor)`:
    //   time_ms = (long)(pos·50·speedFactor); seconds = (time_ms/1000) % length when looping;
    //   target_scale = min(speed·scaleFactor, 1). FROG_WALK is length 1.25, looping.
    let (seconds, scale) = keyframe_walk_sample(&frog_animation::FROG_WALK, 0.5, 0.3, 1.5, 2.5);
    // 0.5·50·1.5 = 37.5 → (long)37 → 0.037 s (no wrap); min(0.3·2.5, 1) = 0.75.
    assert!((seconds - 0.037).abs() < 1.0e-6, "seconds was {seconds}");
    assert!((scale - 0.75).abs() < 1.0e-6, "scale was {scale}");

    // The millisecond truncation drops the fraction: 0.999·50·1.5 = 74.925 → (long)74 → 0.074.
    let (s2, _) = keyframe_walk_sample(&frog_animation::FROG_WALK, 0.999, 0.0, 1.5, 2.5);
    assert!((s2 - 0.074).abs() < 1.0e-6, "seconds was {s2}");

    // The looping wrap: 20·50·1.5 = 1500 ms = 1.5 s, wrapped by the 1.25 s length → 0.25 s.
    let (s3, _) = keyframe_walk_sample(&frog_animation::FROG_WALK, 20.0, 1.0, 1.5, 2.5);
    assert!((s3 - 0.25).abs() < 1.0e-6, "seconds was {s3}");

    // The amplitude clamps at 1.0.
    let (_, clamped) = keyframe_walk_sample(&frog_animation::FROG_WALK, 0.0, 1.0, 1.5, 2.5);
    assert_eq!(clamped, 1.0);
}
