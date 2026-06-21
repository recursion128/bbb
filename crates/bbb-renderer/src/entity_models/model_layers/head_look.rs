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

/// Head-part index for standalone single-head body layers whose mesh lists the
/// head first: creeper, spider/cave spider, enderman, iron golem, snow golem,
/// and wolf (whose part 0 is the head pivot with the head/ears as children).
/// Each of these models applies the plain `setupAnim` head look.
pub(in crate::entity_models) const fn head_first_part_index() -> usize {
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

/// `PolarBearModel` head-part index. The adult body layer lists the head first;
/// the baby body layer lists the body first, so the head is second.
pub(in crate::entity_models) const fn polar_bear_head_part_index(baby: bool) -> usize {
    if baby {
        1
    } else {
        0
    }
}

/// `HoglinModel` head-part index. The adult body layer lists the body first (head
/// second); the baby body layer lists the head first.
pub(in crate::entity_models) const fn hoglin_head_part_index(baby: bool) -> usize {
    if baby {
        0
    } else {
        1
    }
}

/// `RavagerModel` neck-part index. The ravager body layer lists the neck first;
/// vanilla nests the head inside the neck (`neck.getChild("head")`).
pub(in crate::entity_models) const fn ravager_neck_part_index() -> usize {
    0
}

/// Index of the head within the ravager neck's children. The neck has a single
/// child, the head, which in turn parents the horns and mouth.
pub(in crate::entity_models) const fn ravager_head_child_index() -> usize {
    0
}

/// True when a head has no look turn (head aligned with the body and level), so
/// callers can borrow the static parts unchanged instead of cloning to apply
/// [`head_look_pose`].
pub(in crate::entity_models) fn head_look_at_rest(head_yaw_deg: f32, head_pitch_deg: f32) -> bool {
    head_yaw_deg == 0.0 && head_pitch_deg == 0.0
}

/// True when a yaw-only head has no turn, so callers (e.g. the hoglin, whose
/// `head.xRot` is the fixed headbutt-resting tilt rather than a look pitch) can
/// borrow the static parts unchanged instead of cloning to apply
/// [`head_look_yaw_pose`].
pub(in crate::entity_models) fn head_yaw_at_rest(head_yaw_deg: f32) -> bool {
    head_yaw_deg == 0.0
}

/// Vanilla yaw-only head look: sets `head.yRot = yRot * π/180` while leaving the
/// base `head.xRot` (and `head.zRot`) untouched. Used by `HoglinModel`, whose
/// `setupAnim` keeps `head.xRot` at the headbutt animation value (the fixed
/// `HOGLIN_HEAD_X_ROT` tilt at rest) instead of following the look pitch.
pub(in crate::entity_models) fn head_look_yaw_pose(base: PartPose, head_yaw_deg: f32) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [
            base.rotation[0],
            head_yaw_deg.to_radians(),
            base.rotation[2],
        ],
    }
}

/// True when the limb swing is at rest (`walkAnimationSpeed == 0`), so callers can
/// borrow the static leg parts unchanged instead of cloning to apply
/// [`quadruped_leg_swing_pose`].
pub(in crate::entity_models) fn limb_swing_at_rest(walk_animation_speed: f32) -> bool {
    walk_animation_speed == 0.0
}

/// Vanilla `QuadrupedModel.setupAnim` leg swing for a single leg part: sets
/// `leg.xRot = cos(walkAnimationPos * 0.6662 [+ π]) * 1.4 * walkAnimationSpeed`.
/// Vanilla puts the right-hind and left-front legs in phase (`cos(...)`) and the
/// left-hind and right-front legs a half-cycle out of phase (`cos(... + π)`). That
/// diagonal pairing is exactly the legs whose part offset satisfies `x * z < 0`
/// (right is `x < 0`, hind is `z > 0`), so the phase is resolved from the leg's
/// offset and is correct whatever order a model lists its legs in. The base leg
/// pose carries no `xRot`, so it is set (not accumulated), matching the vanilla
/// assignment.
pub(in crate::entity_models) fn quadruped_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let phase = walk_animation_pos * 0.6662;
    let [x, _, z] = base.offset;
    let angle = if x * z < 0.0 {
        phase
    } else {
        phase + std::f32::consts::PI
    };
    PartPose {
        offset: base.offset,
        rotation: [
            angle.cos() * 1.4 * walk_animation_speed,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `HumanoidModel.setupAnim` leg swing for a single leg part: sets
/// `leg.xRot = cos(walkAnimationPos * 0.6662 [+ π]) * 1.4 * walkAnimationSpeed`.
/// The right leg (part offset `x < 0`) is in phase (`cos(...)`) and the left leg a
/// half-cycle out of phase (`cos(... + π)`) — the legs swing oppositely, each
/// coordinated against the same-side arm. Both legs sit at `z = 0`, so the phase is
/// resolved from the `x` sign alone (the `QuadrupedModel` `x * z` rule would be
/// ambiguous). The base leg pose carries no `xRot`, so it is set (not accumulated),
/// matching the vanilla assignment. `state.speedValue` is `1.0` for every entity
/// that is not elytra fall-flying (a deferred pose) so it is omitted; the constant
/// `±0.005` leg yaw/roll splay vanilla always applies is ~0.3° and is omitted too.
pub(in crate::entity_models) fn humanoid_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let phase = walk_animation_pos * 0.6662;
    let angle = if base.offset[0] < 0.0 {
        phase
    } else {
        phase + std::f32::consts::PI
    };
    PartPose {
        offset: base.offset,
        rotation: [
            angle.cos() * 1.4 * walk_animation_speed,
            base.rotation[1],
            base.rotation[2],
        ],
    }
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
