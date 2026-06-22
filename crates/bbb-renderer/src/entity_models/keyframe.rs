//! Vanilla 26.1 `net.minecraft.client.animation` keyframe-animation framework
//! (`KeyframeAnimation` / `AnimationChannel` / `AnimationDefinition` / `Keyframe`).
//!
//! An [`AnimationDefinition`] is a `&'static` table of per-bone [`AnimationChannel`]s, each a
//! sorted list of [`Keyframe`]s sampled at a time to produce a position or rotation offset that
//! is added to the bone's bind pose (vanilla `ModelPart::offsetPos` / `offsetRotation`). The
//! sampling reproduces `KeyframeAnimation.Entry.apply` exactly: a binary search for the
//! surrounding keyframes, a clamped alpha, and the interpolation.
//!
//! Only the [`KeyframeInterpolation::Linear`] interpolation and the position/rotation targets
//! are implemented so far — the cubic `CATMULLROM` interpolation and the `SCALE` target are
//! added when the first entity that uses them is wired (the bat uses neither).

/// Vanilla `AnimationChannel.Interpolations`. Only `LINEAR` is implemented so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::entity_models) enum KeyframeInterpolation {
    Linear,
}

/// Vanilla `AnimationChannel.Target` (the subset implemented so far). `Position` mirrors
/// `ModelPart::offsetPos` and `Rotation` mirrors `ModelPart::offsetRotation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::entity_models) enum AnimationTarget {
    Position,
    Rotation,
}

/// Vanilla `Keyframe(timestamp, preTarget, postTarget, interpolation)`. The single-target
/// `Keyframe` constructor (`pre == post`) is built with [`keyframe`].
#[derive(Clone, Copy, Debug)]
pub(in crate::entity_models) struct Keyframe {
    pub timestamp: f32,
    pub pre_target: [f32; 3],
    pub post_target: [f32; 3],
    pub interpolation: KeyframeInterpolation,
}

/// Vanilla `AnimationChannel(target, keyframes...)`.
#[derive(Clone, Copy, Debug)]
pub(in crate::entity_models) struct AnimationChannel {
    pub target: AnimationTarget,
    pub keyframes: &'static [Keyframe],
}

/// One bone's channels in an [`AnimationDefinition`] (vanilla `boneAnimations` maps a bone name
/// to a list of channels).
#[derive(Clone, Copy, Debug)]
pub(in crate::entity_models) struct BoneAnimation {
    pub bone: &'static str,
    pub channels: &'static [AnimationChannel],
}

/// Vanilla `AnimationDefinition(lengthInSeconds, looping, boneAnimations)`.
#[derive(Clone, Copy, Debug)]
pub(in crate::entity_models) struct AnimationDefinition {
    pub length_seconds: f32,
    pub looping: bool,
    pub bones: &'static [BoneAnimation],
}

/// Vanilla `KeyframeAnimations.posVec(x, y, z)` — the y axis is negated to match the model
/// coordinate space.
pub(in crate::entity_models) const fn pos_vec(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, -y, z]
}

/// Vanilla `KeyframeAnimations.degreeVec(x, y, z)` — degrees converted to radians.
pub(in crate::entity_models) const fn degree_vec(x: f32, y: f32, z: f32) -> [f32; 3] {
    const RAD: f32 = std::f32::consts::PI / 180.0;
    [x * RAD, y * RAD, z * RAD]
}

/// The single-target `Keyframe` constructor (`pre == post`).
pub(in crate::entity_models) const fn keyframe(
    timestamp: f32,
    target: [f32; 3],
    interpolation: KeyframeInterpolation,
) -> Keyframe {
    Keyframe {
        timestamp,
        pre_target: target,
        post_target: target,
        interpolation,
    }
}

/// Vanilla `KeyframeAnimation.getElapsedSeconds`: a looping animation wraps the elapsed time by
/// its length, a non-looping one runs straight through (the sampling clamps at the end keyframe).
pub(in crate::entity_models) fn keyframe_elapsed_seconds(
    definition: &AnimationDefinition,
    seconds_since_start: f32,
) -> f32 {
    if definition.looping {
        seconds_since_start.rem_euclid(definition.length_seconds)
    } else {
        seconds_since_start
    }
}

/// Sample one channel's keyframes at `seconds`, reproducing `KeyframeAnimation.Entry.apply`: a
/// binary search for the surrounding keyframes (`Mth.binarySearch(...) - 1`, clamped to `0`),
/// the clamped lerp alpha, and the interpolation, scaled by `target_scale`.
pub(in crate::entity_models) fn sample_keyframe_channel(
    keyframes: &[Keyframe],
    seconds: f32,
    target_scale: f32,
) -> [f32; 3] {
    // `Mth.binarySearch(0, len, i -> seconds <= keyframes[i].timestamp)` is the first index whose
    // timestamp is at or past `seconds`; `prev` is the one before it, clamped to `0`.
    let first_at_or_after = keyframes.partition_point(|frame| !(seconds <= frame.timestamp));
    let prev = first_at_or_after.saturating_sub(1);
    let next = (prev + 1).min(keyframes.len() - 1);
    let previous_frame = &keyframes[prev];
    let next_frame = &keyframes[next];

    let alpha = if next != prev {
        let delta = seconds - previous_frame.timestamp;
        (delta / (next_frame.timestamp - previous_frame.timestamp)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    match next_frame.interpolation {
        KeyframeInterpolation::Linear => {
            // Vanilla LINEAR: lerp(prev.postTarget, next.preTarget, alpha) * targetScale.
            let point0 = previous_frame.post_target;
            let point1 = next_frame.pre_target;
            [
                lerp(point0[0], point1[0], alpha) * target_scale,
                lerp(point0[1], point1[1], alpha) * target_scale,
                lerp(point0[2], point1[2], alpha) * target_scale,
            ]
        }
    }
}

fn lerp(a: f32, b: f32, alpha: f32) -> f32 {
    a + (b - a) * alpha
}

/// The position and rotation offsets a bone receives from an [`AnimationDefinition`] at
/// `seconds` (already wrapped by [`keyframe_elapsed_seconds`]). Bones with no matching channel
/// (or a channel for an unimplemented target) contribute no offset.
pub(in crate::entity_models) fn sample_bone_offsets(
    definition: &AnimationDefinition,
    bone: &str,
    seconds: f32,
    target_scale: f32,
) -> ([f32; 3], [f32; 3]) {
    let mut position = [0.0; 3];
    let mut rotation = [0.0; 3];
    for bone_animation in definition.bones {
        if bone_animation.bone != bone {
            continue;
        }
        for channel in bone_animation.channels {
            let value = sample_keyframe_channel(channel.keyframes, seconds, target_scale);
            match channel.target {
                AnimationTarget::Position => position = value,
                AnimationTarget::Rotation => rotation = value,
            }
        }
    }
    (position, rotation)
}
