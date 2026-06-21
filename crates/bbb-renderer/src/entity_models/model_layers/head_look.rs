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

/// Vanilla half-amplitude leg swing for a single leg part: `leg.xRot =
/// cos(walkAnimationPos * 0.6662 [+ π]) * 1.4 * walkAnimationSpeed * 0.5`. The
/// `EntityModel` bipeds that are not `HumanoidModel` — `IllagerModel` (non-riding
/// branch), `VillagerModel`, and `WitchModel` — all apply this same extra `0.5`
/// amplitude factor that `HumanoidModel` does not, so they share this pose helper
/// rather than reusing [`humanoid_leg_swing_pose`]. The phase rule is the same: the
/// right leg (part offset `x < 0`) is in phase and the left leg a half-cycle out of
/// phase, both legs sitting at `z = 0`. The base leg pose carries no `xRot`, so it
/// is set (not accumulated). The `IllagerModel` riding sit pose (fixed `-1.4137167`
/// with leg yaw/roll splay) is a separate deferred pose.
pub(in crate::entity_models) fn half_amplitude_leg_swing_pose(
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
            angle.cos() * 1.4 * walk_animation_speed * 0.5,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `HoglinModel.setupAnim` leg swing for a single leg part: `leg.xRot =
/// cos(walkAnimationPos [+ π]) * 1.2 * walkAnimationSpeed`. `HoglinModel` is a custom
/// `EntityModel` (zoglin shares it) with its own formula — amplitude `1.2` (not the
/// `QuadrupedModel` `1.4`) and no `0.6662` frequency factor. The right-front and
/// left-hind legs are in phase (`cos(pos)`) and the left-front and right-hind a
/// half-cycle out (`cos(pos + π)`); that diagonal pairing is exactly the legs whose
/// part offset satisfies `x * z > 0` (right-front is `x < 0, z < 0`; left-hind is
/// `x > 0, z > 0`), the opposite sign from the `QuadrupedModel` rule. The base leg
/// pose carries no `xRot`, so it is set (not accumulated). The ear sway and the
/// headbutt head tilt are separate deferred animations.
pub(in crate::entity_models) fn hoglin_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let [x, _, z] = base.offset;
    let angle = if x * z > 0.0 {
        walk_animation_pos
    } else {
        walk_animation_pos + std::f32::consts::PI
    };
    PartPose {
        offset: base.offset,
        rotation: [
            angle.cos() * 1.2 * walk_animation_speed,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `EndermanModel.setupAnim` leg swing for a single leg part. `EndermanModel
/// extends HumanoidModel`, so `super.setupAnim` first sets `leg.xRot =
/// cos(walkAnimationPos * 0.6662 [+ π]) * 1.4 * walkAnimationSpeed`, then the enderman
/// halves it (`*= 0.5`) and clamps it to `[-0.4, 0.4]`. The phase rule is the
/// `HumanoidModel` one (the right leg, part offset `x < 0`, in phase; both legs at
/// `z = 0`). The base leg pose carries no `xRot`, so it is set (not accumulated). The
/// arm halve/clamp, the carried-block arm pose, and the creepy attack pose are
/// separate deferred animations.
pub(in crate::entity_models) fn enderman_leg_swing_pose(
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
    let x_rot = (angle.cos() * 1.4 * walk_animation_speed * 0.5).clamp(-0.4, 0.4);
    PartPose {
        offset: base.offset,
        rotation: [x_rot, base.rotation[1], base.rotation[2]],
    }
}

/// Vanilla `RavagerModel.setupAnim` leg swing for a single leg part: `leg.xRot =
/// cos(walkAnimationPos * 0.6662 [+ π]) * 0.4 * walkAnimationSpeed`. `RavagerModel`
/// is a custom `EntityModel`, but the leg swing follows the `QuadrupedModel` phase
/// (the right-hind/left-front pair in phase, resolved from `x * z < 0`) with a
/// shorter `0.4` amplitude (vanilla `legRot = 0.4 * walkAnimationSpeed`, no `1.4`
/// factor). The base leg pose carries no `xRot`, so it is set (not accumulated). The
/// neck/mouth attack/stun/roar poses are separate deferred event animations.
pub(in crate::entity_models) fn ravager_leg_swing_pose(
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
            angle.cos() * 0.4 * walk_animation_speed,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `Mth.triangleWave(index, period)`: a triangle wave in `[-1, 1]`,
/// `(|index % period - period/2| - period/4) / (period/4)`.
fn triangle_wave(index: f32, period: f32) -> f32 {
    ((index % period - period * 0.5).abs() - period * 0.25) / (period * 0.25)
}

/// The four swinging limb parts of the iron golem, for [`iron_golem_walk_pose`].
pub(in crate::entity_models) enum IronGolemWalkPart {
    RightArm,
    LeftArm,
    RightLeg,
    LeftLeg,
}

/// Vanilla `IronGolemModel.setupAnim` walking swing for one limb part, driven by
/// `Mth.triangleWave(walkAnimationPos, 13)`. Legs: `xRot = ±1.5 * triangleWave *
/// speed`. Arms (the default branch, when not attacking and not offering a flower):
/// `xRot = (-0.2 ± 1.5 * triangleWave) * speed`. The base pose carries no `xRot`, so
/// it is set (not accumulated). The attack swing and the offer-flower arm pose are
/// separate deferred event animations driven by server-authoritative timers.
pub(in crate::entity_models) fn iron_golem_walk_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
    part: IronGolemWalkPart,
) -> PartPose {
    let wave = triangle_wave(walk_animation_pos, 13.0);
    let x_rot = match part {
        IronGolemWalkPart::RightLeg => -1.5 * wave * walk_animation_speed,
        IronGolemWalkPart::LeftLeg => 1.5 * wave * walk_animation_speed,
        IronGolemWalkPart::RightArm => (-0.2 + 1.5 * wave) * walk_animation_speed,
        IronGolemWalkPart::LeftArm => (-0.2 - 1.5 * wave) * walk_animation_speed,
    };
    PartPose {
        offset: base.offset,
        rotation: [x_rot, base.rotation[1], base.rotation[2]],
    }
}

/// The iron golem body-layer part indices paired with their walk roles: the head and
/// body occupy `0`/`1`, then the right/left arm and right/left leg. The arms sit at
/// part offset `x = 0`, so the role is fixed by slot rather than offset sign.
pub(in crate::entity_models) const fn iron_golem_walk_part_roles() -> [(usize, IronGolemWalkPart); 4]
{
    [
        (2, IronGolemWalkPart::RightArm),
        (3, IronGolemWalkPart::LeftArm),
        (4, IronGolemWalkPart::RightLeg),
        (5, IronGolemWalkPart::LeftLeg),
    ]
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
