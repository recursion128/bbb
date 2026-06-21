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

/// Indices of the right and left ears within the hoglin head's children (both the adult
/// and baby layers list the right ear first, then the left, then — for the adult — the
/// two horns).
pub(in crate::entity_models) const HOGLIN_RIGHT_EAR_CHILD_INDEX: usize = 0;
pub(in crate::entity_models) const HOGLIN_LEFT_EAR_CHILD_INDEX: usize = 1;

/// Vanilla `HoglinModel.setupAnim` ear sway for one ear: `rightEar.zRot = -2π/9 -
/// walkAnimationSpeed * sin(walkAnimationPos)`, `leftEar.zRot = +2π/9 + walkAnimationSpeed
/// * sin(walkAnimationPos)`. Vanilla writes the absolute angle from the literal `2π/9`, so
/// this *sets* `zRot = ±(2π/9 + speed * sin(pos))` (right `−`, left `+`) rather than adding
/// onto the layer's rest splay; only `zRot` changes. `left` selects the side. The adult
/// ears already rest at `±2π/9`, so this matches their rest pose; the baby layer rests its
/// ears at a wider angle that vanilla overrides to `±2π/9`, so the baby ears must always be
/// re-posed through this (even standing).
pub(in crate::entity_models) fn hoglin_ear_sway_pose(
    base: PartPose,
    left: bool,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let ear_z = std::f32::consts::PI * 2.0 / 9.0;
    let magnitude = ear_z + walk_animation_speed * walk_animation_pos.sin();
    let z_rot = if left { magnitude } else { -magnitude };
    PartPose {
        offset: base.offset,
        rotation: [base.rotation[0], base.rotation[1], z_rot],
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

/// Vanilla `HumanoidModel.setupAnim` walking arm swing for a single arm part: sets
/// `arm.xRot = cos(walkAnimationPos * 0.6662 [+ π]) * 2.0 * walkAnimationSpeed * 0.5`
/// (amplitude `1.0`, shorter than the `1.4` leg swing). The right arm (part offset
/// `x < 0`) is the half-cycle out of phase (`cos(... + π)`) and the left arm in phase —
/// the opposite phasing to the same-side leg, the natural walking counter-swing. The
/// base arm pose carries no `xRot`, so it is set (not accumulated). Vanilla also divides
/// by `state.speedValue` (`1.0` except in the deferred crouch/swim/elytra poses) and
/// layers the `ageInTicks` idle bob and the held-item/attack/crouch/swim arm poses on
/// top — all deferred because the client does not yet track that state.
pub(in crate::entity_models) fn humanoid_arm_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let phase = walk_animation_pos * 0.6662;
    let angle = if base.offset[0] < 0.0 {
        phase + std::f32::consts::PI
    } else {
        phase
    };
    PartPose {
        offset: base.offset,
        rotation: [
            angle.cos() * 2.0 * walk_animation_speed * 0.5,
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

/// Vanilla `EndermanModel.setupAnim` arm swing for a single arm part. `EndermanModel
/// extends HumanoidModel`, so `super.setupAnim` first sets the inherited arm
/// counter-swing ([`humanoid_arm_swing_pose`]: `arm.xRot = cos(walkAnimationPos *
/// 0.6662 [+ π]) * 2.0 * walkAnimationSpeed * 0.5`, the right arm — part offset `x < 0`
/// — out of phase), then the enderman halves it (`*= 0.5`) and clamps it to
/// `[-0.4, 0.4]` exactly as it does the legs. The base arm pose carries no `xRot`, so
/// it is set (not accumulated). The carried-block arm pose (`xRot = -0.5`, `zRot =
/// ±0.05`) and the creepy attack head/hat shift are separate deferred animations gated
/// on state the client does not yet track.
pub(in crate::entity_models) fn enderman_arm_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let swung = humanoid_arm_swing_pose(base, walk_animation_pos, walk_animation_speed);
    let x_rot = (swung.rotation[0] * 0.5).clamp(-0.4, 0.4);
    PartPose {
        offset: base.offset,
        rotation: [x_rot, base.rotation[1], base.rotation[2]],
    }
}

/// Vanilla `WolfModel.setupAnim` tail wag for the tail part. In its non-angry branch the
/// wolf sets `tail.yRot = cos(walkAnimationPos * 0.6662) * 1.4 * walkAnimationSpeed` — the
/// same `QuadrupedModel` swing amplitude as the legs, with no phase offset, so the tail
/// sweeps side to side in step with the gait. The caller takes this branch only for a
/// non-angry wolf; an angry one holds its tail straight and raised
/// ([`wolf_angry_tail_pose`]). The base tail pose carries the layer's resting `xRot` droop
/// `π/5` — which is exactly the `tailAngle` vanilla writes for an untamed wolf, so a wild
/// wolf needs no `xRot` override; the tame/health `tailAngle` droop is still deferred (it
/// needs the wolf's health). The wag sets only the `yRot` axis and preserves the others.
pub(in crate::entity_models) fn wolf_tail_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let y_rot = (walk_animation_pos * 0.6662).cos() * 1.4 * walk_animation_speed;
    PartPose {
        offset: base.offset,
        rotation: [base.rotation[0], y_rot, base.rotation[2]],
    }
}

/// Vanilla `WolfModel.setupAnim` angry tail: an angry wolf zeroes the wag (`tail.yRot = 0`)
/// and `getTailAngle()` returns the constant `1.5393804` (≈ 88°), so `tail.xRot` is *set*
/// to that raised angle — overriding the layer's `π/5` wild rest droop. The offset and
/// `zRot` are preserved. Driven by the `isAngry` render state (which the client tracks); the
/// tame/health droop and the sitting/water-shake poses remain deferred.
pub(in crate::entity_models) const WOLF_ANGRY_TAIL_X_ROT: f32 = 1.5393804;

pub(in crate::entity_models) fn wolf_angry_tail_pose(base: PartPose) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [WOLF_ANGRY_TAIL_X_ROT, 0.0, base.rotation[2]],
    }
}

/// Tail part index in the wolf body layers. The adult layer lists head/body/mane at
/// `0`/`1`/`2`, the four legs at `[3, 4, 5, 6]`, and the tail last at `7`; the baby layer
/// drops the mane, so the tail is at `6`.
pub(in crate::entity_models) const fn wolf_tail_part_index(baby: bool) -> usize {
    if baby {
        6
    } else {
        7
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

/// Vanilla `SpiderModel.setupAnim` walking swing for one leg part. With
/// `animationPos = walkAnimationPos * 0.6662`, vanilla computes a horizontal `swing`
/// `-(cos(animationPos * 2 + phase) * 0.4) * walkAnimationSpeed` accumulated onto
/// `yRot`, and a vertical `step` `|sin(animationPos + phase) * 0.4| *
/// walkAnimationSpeed` accumulated onto `zRot`; the right legs add both (`+=`), the
/// left legs subtract both (`-=`). `phase` is the per-leg-pair offset (`0`, `π`,
/// `π/2`, `3π/2` for hind, middle-hind, middle-front, front), and `side_sign` is
/// `+1` for the right legs and `-1` for the left. Both terms are accumulated onto the
/// leg's resting `yRot`/`zRot` (the splay set in the body layer); `xRot` is untouched.
pub(in crate::entity_models) fn spider_leg_swing_pose(
    base: PartPose,
    phase: f32,
    side_sign: f32,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let animation_pos = walk_animation_pos * 0.6662;
    let swing = -((animation_pos * 2.0 + phase).cos() * 0.4) * walk_animation_speed;
    let step = (animation_pos + phase).sin().abs() * 0.4 * walk_animation_speed;
    PartPose {
        offset: base.offset,
        rotation: [
            base.rotation[0],
            base.rotation[1] + side_sign * swing,
            base.rotation[2] + side_sign * step,
        ],
    }
}

/// The eight spider leg part indices paired with their `(phase, side_sign)` swing
/// roles. `SpiderModel.createSpiderBodyLayer` lists the legs after head/body0/body1 in
/// right/left pairs from hind to front: hind (`3`/`4`, phase `0`), middle-hind
/// (`5`/`6`, phase `π`), middle-front (`7`/`8`, phase `π/2`), front (`9`/`10`, phase
/// `3π/2`). Right legs swing `+`, left legs swing `-`.
pub(in crate::entity_models) fn spider_leg_swing_roles() -> [(usize, f32, f32); 8] {
    use std::f32::consts::{FRAC_PI_2, PI};
    [
        (3, 0.0, 1.0),
        (4, 0.0, -1.0),
        (5, PI, 1.0),
        (6, PI, -1.0),
        (7, FRAC_PI_2, 1.0),
        (8, FRAC_PI_2, -1.0),
        (9, PI * 1.5, 1.0),
        (10, PI * 1.5, -1.0),
    ]
}

/// Vanilla `AbstractEquineModel.setupAnim` walking leg swing (the non-standing branch).
/// With `legAnim = cos(walkAnimationPos * 0.6662 + π) * walkAnimationSpeed`, the front
/// legs swing `±0.8 * legAnim` and the hind legs `±0.5 * legAnim` — a horse-specific
/// gait (front amplitude `0.8`, hind `0.5`) rather than the uniform `1.4`
/// `QuadrupedModel` swing. The signs are front-left `+0.8`, front-right `-0.8`,
/// hind-left `-0.5`, hind-right `+0.5`: the front legs have `z < 0` and the left legs
/// `x > 0`, so the sign is `+` when `(x > 0) == (z < 0)`. The base leg pose carries no
/// `xRot`, so it is set (not accumulated). In water vanilla scales the frequency by
/// `0.2`; that and the standing/eating/feeding poses are deferred (they depend on state
/// the client does not yet track). The head look/bob is applied separately by
/// [`equine_head_look_pose`] and the tail walk lift by [`equine_tail_swing_pose`].
pub(in crate::entity_models) fn equine_leg_swing_pose(
    base: PartPose,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let leg_anim =
        (walk_animation_pos * 0.6662 + std::f32::consts::PI).cos() * walk_animation_speed;
    let [x, _, z] = base.offset;
    let front = z < 0.0;
    let amplitude = if front { 0.8 } else { 0.5 };
    let sign = if (x > 0.0) == front { 1.0 } else { -1.0 };
    PartPose {
        offset: base.offset,
        rotation: [
            sign * amplitude * leg_anim,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `AbstractEquineModel.setupAnim` head (`head_parts`) look, in its default
/// (non-standing, non-eating, non-feeding) branch — the branch a free-standing horse
/// always takes. The net head yaw is clamped to `±20°` (a horse turns its head less than
/// the body) and applied as `head_parts.yRot = clamp(yRot, -20, 20) * π/180`; the head
/// pitch is added onto the layer's `π/6` neck tilt as `head_parts.xRot = π/6 + xRot *
/// π/180`, with a walk-driven bob `+= cos(walkAnimationPos * 0.8) * 0.15 *
/// walkAnimationSpeed` folded in when `walkAnimationSpeed > 0.2`. The rest `head_parts`
/// xRot is exactly that `π/6` tilt, so at a level head and no fast gait the pose equals
/// the rest pose. `HorseModel`/`BabyHorseModel` and the adult `DonkeyModel`/mule take this
/// unchanged; the baby donkey/mule (which forces `xRot = -30°`), the ridden/stand/eat/feed
/// poses, and the in-water gait are deferred. The tail walk lift is applied by
/// [`equine_tail_swing_pose`]; only its `ageInTicks`-driven `yRot` wag stays deferred.
pub(in crate::entity_models) fn equine_head_look_pose(
    base: PartPose,
    head_yaw_deg: f32,
    head_pitch_deg: f32,
    walk_animation_pos: f32,
    walk_animation_speed: f32,
) -> PartPose {
    let clamped_yaw = head_yaw_deg.clamp(-20.0, 20.0);
    let mut head_rot_x = head_pitch_deg.to_radians();
    if walk_animation_speed > 0.2 {
        head_rot_x += (walk_animation_pos * 0.8).cos() * 0.15 * walk_animation_speed;
    }
    PartPose {
        offset: base.offset,
        rotation: [
            std::f32::consts::FRAC_PI_6 + head_rot_x,
            clamped_yaw.to_radians(),
            base.rotation[2],
        ],
    }
}

/// Vanilla `AbstractEquineModel.setupAnim` tail walk animation (the default branch). The
/// tail's `xRot` is *set* to `getTailXRotOffset() + π/6 + walkAnimationSpeed * 0.75`, so a
/// running equine lifts its tail. The per-model `getTailXRotOffset` (`0` for the adult
/// horse/donkey/mule, `−π/2` for the baby horse) also overrides the baby layer's wider
/// rest angle: vanilla runs `setupAnim` every frame, so a standing baby horse renders its
/// tail at `−π/2 + π/6 = −1.0472`, not the layer's `−0.7418`. The tail base also
/// translates `y += walkAnimationSpeed * ageScale` and `z += walkAnimationSpeed * 2 *
/// ageScale`, where `ageScale` is `getAgeScale()` (`1.0` for adults, `0.5` for babies).
/// The `tail.yRot` wag (`cos(ageInTicks * 0.7)` under `animateTail`) needs `ageInTicks` the
/// client does not track and is deferred, so `yRot`/`zRot` are preserved here.
pub(in crate::entity_models) fn equine_tail_swing_pose(
    base: PartPose,
    tail_x_rot_offset: f32,
    walk_animation_speed: f32,
    age_scale: f32,
) -> PartPose {
    PartPose {
        offset: [
            base.offset[0],
            base.offset[1] + walk_animation_speed * age_scale,
            base.offset[2] + walk_animation_speed * 2.0 * age_scale,
        ],
        rotation: [
            tail_x_rot_offset + std::f32::consts::FRAC_PI_6 + walk_animation_speed * 0.75,
            base.rotation[1],
            base.rotation[2],
        ],
    }
}

/// Vanilla `SnowGolemModel.setupAnim` upper-body twist: the middle snow ball turns a
/// quarter of the head yaw, `upperBody.yRot = headYaw * π/180 * 0.25`. The base upper
/// body carries no rotation, so the twist is set (not accumulated); `xRot`/`zRot` and
/// the offset are preserved. The returned value is also the arm orbit angle for
/// [`snow_golem_arm_pose`].
pub(in crate::entity_models) fn snow_golem_upper_body_yrot(head_yaw_deg: f32) -> f32 {
    head_yaw_deg.to_radians() * 0.25
}

/// Applies the [`snow_golem_upper_body_yrot`] twist to the upper-body part pose.
pub(in crate::entity_models) fn snow_golem_upper_body_pose(
    base: PartPose,
    upper_body_yrot: f32,
) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [base.rotation[0], upper_body_yrot, base.rotation[2]],
    }
}

/// Vanilla `SnowGolemModel.setupAnim` arm orbit. The two stick arms ride the twisting
/// upper body: `leftArm.yRot = upperBodyYRot`, `leftArm.x = cos(upperBodyYRot) * 5`,
/// `leftArm.z = -sin(upperBodyYRot) * 5`; the right arm adds `π` to the yaw and negates
/// both `x` and `z`. The arm `y` offset and the drooping `zRot` (`±1.0` from the body
/// layer) are preserved; the base `x`/`z` offsets are overwritten by the orbit even at
/// rest (so a forward-facing snow golem still pulls its arms to `z = 0`).
pub(in crate::entity_models) fn snow_golem_arm_pose(
    base: PartPose,
    upper_body_yrot: f32,
    right: bool,
) -> PartPose {
    let (sin, cos) = upper_body_yrot.sin_cos();
    let (x, z, y_rot) = if right {
        (
            -cos * 5.0,
            sin * 5.0,
            upper_body_yrot + std::f32::consts::PI,
        )
    } else {
        (cos * 5.0, -sin * 5.0, upper_body_yrot)
    };
    PartPose {
        offset: [x, base.offset[1], z],
        rotation: [base.rotation[0], y_rot, base.rotation[2]],
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
