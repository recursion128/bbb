use super::PartPose;

/// Plain `QuadrupedModel` head-part index for the cow body layer. The cow mesh
/// lists the head first for both the adult and the baby layer.
pub(in crate::entity_models) const fn cow_head_part_index(_baby: bool) -> usize {
    0
}

/// Plain `QuadrupedModel` head-part index for the pig body layer. The adult pig
/// layer lists the head first; the baby pig layer lists the body first, so the
/// head is second (matching the vanilla baby quadruped mesh part order, the same
/// ordering as [`super::sheep_head_part_index`]).
pub(in crate::entity_models) const fn pig_head_part_index(baby: bool) -> usize {
    if baby {
        1
    } else {
        0
    }
}

/// True when a plain `QuadrupedModel` head has no look turn (head aligned with
/// the body and level), so callers can borrow the static parts unchanged instead
/// of cloning to apply [`quadruped_head_look_pose`].
pub(in crate::entity_models) fn head_look_at_rest(head_yaw_deg: f32, head_pitch_deg: f32) -> bool {
    head_yaw_deg == 0.0 && head_pitch_deg == 0.0
}

/// Vanilla `QuadrupedModel.setupAnim` head look: `head.xRot = xRot * π/180` and
/// `head.yRot = yRot * π/180`, where `xRot` is the head pitch and `yRot` is the
/// net head yaw (`Mth.wrapDegrees(headRot - bodyRot)`). The base head pose
/// carries no rotation, so the look angles are set (not accumulated), matching
/// the vanilla assignments. Pig and cow extend `QuadrupedModel` without
/// overriding `setupAnim`, so this is their full head animation.
pub(in crate::entity_models) fn quadruped_head_look_pose(
    base: PartPose,
    head_yaw_deg: f32,
    head_pitch_deg: f32,
) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [
            head_pitch_deg.to_radians(),
            head_yaw_deg.to_radians(),
            base.rotation[2],
        ],
    }
}
