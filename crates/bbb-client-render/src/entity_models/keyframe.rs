//! Vanilla 26.1 `net.minecraft.client.animation` keyframe-animation framework
//! (`KeyframeAnimation` / `AnimationChannel` / `AnimationDefinition` / `Keyframe`).
//!
//! An [`AnimationDefinition`] is a `&'static` table of per-bone [`AnimationChannel`]s, each a
//! sorted list of [`Keyframe`]s sampled at a time to produce a position or rotation offset that
//! is added to the bone's bind pose (vanilla `ModelPart::offsetPos` / `offsetRotation`). The
//! sampling reproduces `KeyframeAnimation.Entry.apply` exactly: a binary search for the
//! surrounding keyframes, a clamped alpha, and the interpolation.
//!
//! The [`KeyframeInterpolation::Linear`] and [`KeyframeInterpolation::CatmullRom`] interpolations
//! and the position/rotation/scale targets are implemented ([`AnimationTarget::Scale`] mirrors
//! `ModelPart::offsetScale`, the croaking frog's first consumer).

use super::model::ModelPart;

/// Vanilla `AnimationChannel.Interpolations` (`LINEAR` and the cubic `CATMULLROM`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::entity_models) enum KeyframeInterpolation {
    Linear,
    CatmullRom,
}

/// Vanilla `AnimationChannel.Target` (the subset implemented so far). `Position` mirrors
/// `ModelPart::offsetPos`, `Rotation` mirrors `ModelPart::offsetRotation`, and `Scale` mirrors
/// `ModelPart::offsetScale` (its `scaleVec` value is an offset added to the part's `1.0` base
/// scale).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::entity_models) enum AnimationTarget {
    Position,
    Rotation,
    Scale,
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

/// Vanilla `KeyframeAnimation.applyWalk` sample time and amplitude: the time is
/// `(long)(animationPos·50·speedFactor)` milliseconds and the amplitude `min(speed·scaleFactor, 1)`.
pub(in crate::entity_models) fn keyframe_walk_sample(
    definition: &AnimationDefinition,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
    speed_factor: f32,
    scale_factor: f32,
) -> (f32, f32) {
    let millis = (walk_animation_pos * 50.0 * speed_factor) as i64;
    let seconds = keyframe_elapsed_seconds(definition, millis as f32 / 1000.0);
    let scale = (walk_animation_speed * scale_factor).min(1.0);
    (seconds, scale)
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
    let first_at_or_after = keyframes.partition_point(|frame| {
        !matches!(
            seconds.partial_cmp(&frame.timestamp),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        )
    });
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

    // Vanilla picks the interpolation from the NEXT keyframe (`keyframes[next].interpolation`).
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
        KeyframeInterpolation::CatmullRom => {
            // Vanilla CATMULLROM samples the four surrounding `postTarget`s (the prev/next pair
            // plus the clamped neighbours `prev - 1` and `next + 1`) through `Mth.catmullrom`.
            let point0 = keyframes[prev.saturating_sub(1)].post_target;
            let point1 = previous_frame.post_target;
            let point2 = next_frame.post_target;
            let point3 = keyframes[(next + 1).min(keyframes.len() - 1)].post_target;
            [
                catmullrom(alpha, point0[0], point1[0], point2[0], point3[0]) * target_scale,
                catmullrom(alpha, point0[1], point1[1], point2[1], point3[1]) * target_scale,
                catmullrom(alpha, point0[2], point1[2], point2[2], point3[2]) * target_scale,
            ]
        }
    }
}

fn lerp(a: f32, b: f32, alpha: f32) -> f32 {
    a + (b - a) * alpha
}

/// Vanilla `Mth.catmullrom(alpha, p0, p1, p2, p3)`: the Catmull-Rom spline through `p1`/`p2` with
/// `p0`/`p3` as the surrounding control points.
fn catmullrom(alpha: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    0.5 * (2.0 * p1
        + (p2 - p0) * alpha
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * alpha * alpha
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * alpha * alpha * alpha)
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
    let (position, rotation, _) =
        sample_bone_offsets_with_scale(definition, bone, seconds, target_scale);
    (position, rotation)
}

/// Like [`sample_bone_offsets`] but also returns the bone's scale offset (vanilla
/// `ModelPart::offsetScale`'s `scaleVec` value, a per-axis offset added to the part's `1.0` base
/// scale via [`keyframe_animated_scale`]). `[0, 0, 0]` for a bone with no `SCALE` channel — the
/// identity offset, so the part holds its `[1, 1, 1]` base scale.
pub(in crate::entity_models) fn sample_bone_offsets_with_scale(
    definition: &AnimationDefinition,
    bone: &str,
    seconds: f32,
    target_scale: f32,
) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let mut position = [0.0; 3];
    let mut rotation = [0.0; 3];
    let mut scale = [0.0; 3];
    for bone_animation in definition.bones {
        if bone_animation.bone != bone {
            continue;
        }
        for channel in bone_animation.channels {
            let value = sample_keyframe_channel(channel.keyframes, seconds, target_scale);
            match channel.target {
                AnimationTarget::Position => position = value,
                AnimationTarget::Rotation => rotation = value,
                AnimationTarget::Scale => scale = value,
            }
        }
    }
    (position, rotation, scale)
}

/// Vanilla `KeyframeAnimation.apply`: adds every channel sampled at `seconds_since_start` onto the
/// part `ModelPart.createPartLookup` resolves for its bone.
pub(in crate::entity_models) fn animate(
    root: &mut ModelPart,
    definition: &AnimationDefinition,
    seconds_since_start: f32,
    target_scale: f32,
) {
    let seconds = keyframe_elapsed_seconds(definition, seconds_since_start);
    for bone in definition.bones {
        let part = root.part_mut(bone.bone).unwrap_or_else(|| {
            panic!(
                "cannot animate {}, which does not exist in model",
                bone.bone
            )
        });
        for channel in bone.channels {
            let value = sample_keyframe_channel(channel.keyframes, seconds, target_scale);
            let target = match channel.target {
                AnimationTarget::Position => &mut part.pose.offset,
                AnimationTarget::Rotation => &mut part.pose.rotation,
                AnimationTarget::Scale => &mut part.scale,
            };
            for axis in 0..3 {
                target[axis] += value[axis];
            }
        }
    }
}

/// Vanilla `KeyframeAnimation.applyWalk`.
pub(in crate::entity_models) fn animate_walk(
    root: &mut ModelPart,
    definition: &AnimationDefinition,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
    speed_factor: f32,
    scale_factor: f32,
) {
    let (seconds, scale) = keyframe_walk_sample(
        definition,
        walk_animation_pos,
        walk_animation_speed,
        speed_factor,
        scale_factor,
    );
    animate(root, definition, seconds, scale);
}

/// Combine a bind pose with the keyframe position/rotation offsets (vanilla `ModelPart::offsetPos` /
/// `offsetRotation` add to the bind pose). Shared by the keyframe-animated entities' `setup_anim`.
pub(in crate::entity_models) fn keyframe_animated_pose(
    bind: super::geometry::PartPose,
    position: [f32; 3],
    rotation: [f32; 3],
) -> super::geometry::PartPose {
    super::geometry::PartPose {
        offset: [
            bind.offset[0] + position[0],
            bind.offset[1] + position[1],
            bind.offset[2] + position[2],
        ],
        rotation: [
            bind.rotation[0] + rotation[0],
            bind.rotation[1] + rotation[1],
            bind.rotation[2] + rotation[2],
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOOPING: AnimationDefinition = AnimationDefinition {
        length_seconds: 3.0,
        looping: true,
        bones: &[],
    };
    const ONCE: AnimationDefinition = AnimationDefinition {
        length_seconds: 3.0,
        looping: false,
        bones: &[],
    };

    #[test]
    fn keyframe_elapsed_seconds_wraps_a_looping_definition_by_its_length() {
        // Vanilla `KeyframeAnimation.getElapsedSeconds`: a looping definition wraps the elapsed time
        // by its length (3.0s here), a non-looping one runs straight through.
        assert!((keyframe_elapsed_seconds(&LOOPING, 1.5) - 1.5).abs() < 1.0e-6);
        assert!((keyframe_elapsed_seconds(&LOOPING, 3.0) - 0.0).abs() < 1.0e-6);
        assert!((keyframe_elapsed_seconds(&LOOPING, 3.25) - 0.25).abs() < 1.0e-6);
        assert!((keyframe_elapsed_seconds(&LOOPING, 7.0) - 1.0).abs() < 1.0e-6);
        // A non-looping definition is unwrapped (the sampling clamps at the end keyframe).
        assert!((keyframe_elapsed_seconds(&ONCE, 5.0) - 5.0).abs() < 1.0e-6);
    }
}
