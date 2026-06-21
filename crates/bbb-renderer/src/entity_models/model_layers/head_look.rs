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

/// `HumanoidModel` head-part index for the zombie family body layers (zombie,
/// husk, drowned, and zombie villager). The adult layer lists the head first;
/// the baby layer lists the body first, so the head is second.
pub(in crate::entity_models) const fn zombie_head_part_index(baby: bool) -> usize {
    if baby {
        1
    } else {
        0
    }
}

/// `HumanoidModel` head-part index for the piglin family body layers (piglin,
/// piglin brute, zombified piglin). The adult layer lists the head first; the
/// baby layer lists the body first, so the head is second. `baby_layout` is
/// whether the baby part layout is in use — a baby piglin *brute* still renders
/// the adult layout.
pub(in crate::entity_models) const fn piglin_head_part_index(baby_layout: bool) -> usize {
    if baby_layout {
        1
    } else {
        0
    }
}

/// Head-part index for the `VillagerModel`/`IllagerModel`/`WitchModel` family.
/// The adult villager, wandering trader, witch, and illager body layers list the
/// head first; the baby villager layout lists the arms container and legs first,
/// so the head is at index 3. (Witch and illagers have no baby layout.)
pub(in crate::entity_models) const fn villager_head_part_index(baby: bool) -> usize {
    if baby {
        3
    } else {
        0
    }
}

/// `PlayerModel` head-part index. The wide and slim player body layers list the
/// head first; visibility filtering only toggles the overlay children, never the
/// base part order.
pub(in crate::entity_models) const fn player_head_part_index() -> usize {
    0
}

/// `SkeletonModel` head-part index. The skeleton, stray, wither-skeleton, and
/// bogged body layers list the head first.
pub(in crate::entity_models) const fn skeleton_head_part_index() -> usize {
    0
}

/// Parched-skeleton head-part index. The parched body layer lists the body
/// first, so the head is second.
pub(in crate::entity_models) const fn parched_head_part_index() -> usize {
    1
}

/// True when a head has no look turn (head aligned with the body and level), so
/// callers can borrow the static parts unchanged instead of cloning to apply
/// [`head_look_pose`].
pub(in crate::entity_models) fn head_look_at_rest(head_yaw_deg: f32, head_pitch_deg: f32) -> bool {
    head_yaw_deg == 0.0 && head_pitch_deg == 0.0
}

/// Vanilla head look shared by `QuadrupedModel.setupAnim` and
/// `HumanoidModel.setupAnim`: `head.xRot = xRot * π/180` and `head.yRot = yRot *
/// π/180`, where `xRot` is the head pitch and `yRot` is the net head yaw
/// (`Mth.wrapDegrees(headRot - bodyRot)`). The base head pose carries no
/// rotation, so the look angles are set (not accumulated), matching the vanilla
/// assignments. Pig, cow, and the zombie family extend these base models without
/// overriding the head animation, so this is their full head pose.
pub(in crate::entity_models) fn head_look_pose(
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
