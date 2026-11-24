use super::{
    apply_crossbow_charge_pose, apply_crossbow_hold_pose, apply_half_amplitude_leg_swing,
    apply_head_look, apply_humanoid_weapon_swing_down, apply_zombie_arms_held_out_named,
    humanoid_arm_swing_pose, PartPose,
};
use crate::entity_models::catalog::IllagerModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `IllagerModel.setupAnim` SPELLCASTING arm pose for one separate arm. The arm holds its
/// base offset (`rightArm.x = -5`/`leftArm.x = 5`, `z = 0` — both already the bind offset), pitches
/// `xRot = cos(ageInTicks · 0.6662) · 0.25`, and rolls outward `zRot = ±3π/4` (right `+`, left `−`),
/// with `yRot = 0`.
pub(in crate::entity_models) fn illager_spellcast_arm_pose(
    base: PartPose,
    age_in_ticks: f32,
    is_right: bool,
) -> PartPose {
    let three_quarter_pi = std::f32::consts::PI * 3.0 / 4.0;
    PartPose {
        offset: base.offset,
        rotation: [
            (age_in_ticks * 0.6662).cos() * 0.25,
            0.0,
            if is_right {
                three_quarter_pi
            } else {
                -three_quarter_pi
            },
        ],
    }
}

/// Vanilla `IllagerModel.setupAnim` `CELEBRATING` arm pose for one separate arm (the evoker/vindicator
/// raid victory dance). Both arms hold their bind offset (`rightArm.x = -5`/`leftArm.x = 5`, `z = 0`),
/// bob `xRot = cos(ageInTicks · 0.6662) · 0.05`, and raise outward by an asymmetric roll — the right arm
/// higher (`zRot = 2.670354`), the left to `-3π/4` — with `yRot = 0`.
pub(in crate::entity_models) fn illager_celebrate_arm_pose(
    base: PartPose,
    age_in_ticks: f32,
    is_right: bool,
) -> PartPose {
    PartPose {
        offset: base.offset,
        rotation: [
            (age_in_ticks * 0.6662).cos() * 0.05,
            0.0,
            if is_right {
                2.670354
            } else {
                -std::f32::consts::PI * 3.0 / 4.0
            },
        ],
    }
}

/// Vanilla `IllagerModel.setupAnim` `BOW_AND_ARROW` arm pose (the illusioner drawing its bow). Unlike
/// the symmetric `HumanoidModel`/skeleton aim, the illager braces the off (left) hand across the bow:
/// the right arm aims down the head look (`xRot = -π/2 + head.xRot`, `yRot = -0.1 + head.yRot`, `zRot`
/// preserved), the left arm holds the bow level and rolled in (`xRot = -0.9424779 + head.xRot`,
/// `yRot = head.yRot - 0.4`, `zRot = π/2`).
fn apply_illager_bow_aim(root: &mut ModelPart, head_yaw_degrees: f32, head_pitch_degrees: f32) {
    let head_yaw = head_yaw_degrees.to_radians();
    let head_pitch = head_pitch_degrees.to_radians();
    let right = root.child_mut("right_arm");
    right.pose.rotation = [
        -std::f32::consts::FRAC_PI_2 + head_pitch,
        -0.1 + head_yaw,
        right.pose.rotation[2],
    ];
    let left = root.child_mut("left_arm");
    left.pose.rotation = [
        -0.9424779 + head_pitch,
        head_yaw - 0.4,
        std::f32::consts::FRAC_PI_2,
    ];
}

/// Whether a pillager levels its crossbow this frame (vanilla `Pillager.getArmPose` returning
/// `CROSSBOW_HOLD`): it holds a crossbow and is not mid-draw (`isChargingCrossbow()`, which instead
/// takes the higher-priority `CROSSBOW_CHARGE` pull-back pose).
fn illager_is_holding_crossbow(instance: &EntityModelInstance) -> bool {
    let illager_state = *instance.render_state.kind.illager();
    illager_state.holds_crossbow && !illager_state.is_charging_crossbow
}

/// The resolved illager arm pose for a frame, mirroring each family's vanilla `getArmPose()` for the
/// poses bbb renders. `Crossed` shows the static folded `arms` part; every other pose uses the uncrossed
/// separate arms. An aggressive vindicator now chops (`Attacking`) and an aggressive illusioner draws its
/// bow (`BowAndArrow`) — `is_aggressive` is projected for both. A charging pillager pulls its crossbow
/// back (the `Swing` layout's `CROSSBOW_CHARGE` sub-pose, driven by the projected draw ticks).
#[derive(Clone, Copy, PartialEq, Eq)]
enum IllagerArmPose {
    /// Folded `arms` part — vanilla `CROSSED` (idle evoker/vindicator/illusioner).
    Crossed,
    /// Pillager separate-arm walk swing, optionally overridden by the `CROSSBOW_HOLD` level.
    Swing,
    /// Evoker/illusioner `SPELLCASTING` raised arms.
    Spellcast,
    /// Illusioner `BOW_AND_ARROW` draw.
    BowAndArrow,
    /// Evoker/vindicator `CELEBRATING` victory dance.
    Celebrating,
    /// Vindicator `ATTACKING` weapon swing-down (`AnimationUtils.swingWeaponDown`, armed main hand).
    Attacking,
}

impl IllagerArmPose {
    /// Whether this pose renders the uncrossed separate-arm layout (vanilla `crossedArms = pose ==
    /// CROSSED` toggles the `arms` part off and the separate arms on for every non-`Crossed` pose).
    fn uses_separate_arms(self) -> bool {
        self != IllagerArmPose::Crossed
    }
}

/// Resolves the illager arm pose from the projected render state, mirroring each family's vanilla
/// `getArmPose()` for the supported poses. The pillager uses crossbow charge/hold first, otherwise
/// `ATTACKING` when aggressive and `NEUTRAL` as the uncrossed swing layout; the spellcasters cast first
/// (priority), else the evoker celebrates and the illusioner draws its bow when aggressive; the vindicator
/// chops its axe when aggressive (`ATTACKING`, priority over `CELEBRATING` per vanilla), else celebrates,
/// else stays `Crossed`.
fn resolve_illager_arm_pose(
    instance: &EntityModelInstance,
    family: IllagerModelFamily,
) -> IllagerArmPose {
    let rs = &instance.render_state;
    match family {
        IllagerModelFamily::Pillager => {
            if rs.kind.illager().is_charging_crossbow || illager_is_holding_crossbow(instance) {
                IllagerArmPose::Swing
            } else if rs.is_aggressive {
                IllagerArmPose::Attacking
            } else {
                IllagerArmPose::Swing
            }
        }
        IllagerModelFamily::Evoker => {
            if rs.kind.illager().spellcasting {
                IllagerArmPose::Spellcast
            } else if rs.kind.illager().celebrating {
                IllagerArmPose::Celebrating
            } else {
                IllagerArmPose::Crossed
            }
        }
        IllagerModelFamily::Illusioner => {
            if rs.kind.illager().spellcasting {
                IllagerArmPose::Spellcast
            } else if rs.is_aggressive {
                IllagerArmPose::BowAndArrow
            } else {
                IllagerArmPose::Crossed
            }
        }
        IllagerModelFamily::Vindicator => {
            if rs.is_aggressive {
                // Vanilla `Vindicator.getArmPose`: aggressive -> ATTACKING. `IllagerModel.setupAnim`
                // later chooses the empty-hand or armed branch from the rendered main-hand item state.
                IllagerArmPose::Attacking
            } else if rs.kind.illager().celebrating {
                IllagerArmPose::Celebrating
            } else {
                IllagerArmPose::Crossed
            }
        }
    }
}

/// Mutable illager model, mirroring vanilla `IllagerModel`/`SpellcasterIllagerModel` shared by the
/// evoker, vindicator, illusioner, and pillager. The unified tree is built for the resolved
/// [`IllagerArmPose`] with the vanilla `IllagerModel` child names. `setup_anim` looks the head
/// ([`apply_head_look`] on `head`) and swings the legs at the villager-family half amplitude
/// ([`apply_half_amplitude_leg_swing`]), then applies the resolved arm pose: the pillager swings its
/// separate arms ([`humanoid_arm_swing_pose`]) and either pulls back a charging crossbow
/// ([`apply_crossbow_charge_pose`]), levels a held one ([`apply_crossbow_hold_pose`]), or attacks without
/// a crossbow (`ATTACKING`);
/// a casting evoker/illusioner raises the `SPELLCASTING` arms ([`illager_spellcast_arm_pose`]); an
/// aggressive illusioner draws its bow ([`apply_illager_bow_aim`]); an aggressive vindicator either
/// reaches with empty zombie arms ([`apply_zombie_arms_held_out_named`]) or chops its axe
/// ([`apply_humanoid_weapon_swing_down`]); a celebrating evoker/vindicator dances
/// ([`illager_celebrate_arm_pose`]). Riding illagers use vanilla's fixed seated leg pose before
/// those arm-pose branches run.
pub(in crate::entity_models) fn illager_model(
    instance: &EntityModelInstance,
    family: IllagerModelFamily,
) -> BakedRoot {
    let pose = resolve_illager_arm_pose(instance, family);
    let layer = match family {
        IllagerModelFamily::Evoker => ModelLayer::EVOKER,
        IllagerModelFamily::Illusioner => ModelLayer::ILLUSIONER,
        IllagerModelFamily::Pillager => ModelLayer::PILLAGER,
        IllagerModelFamily::Vindicator => ModelLayer::VINDICATOR,
    };
    let mut root = layer.bake();
    let render_state = &instance.render_state;
    apply_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
    );
    let limb_swing = render_state.walk_animation_pos;
    let limb_swing_amount = render_state.walk_animation_speed;
    if render_state.is_riding {
        apply_illager_riding_pose(&mut root, pose.uses_separate_arms());
    } else {
        apply_half_amplitude_leg_swing(&mut root, limb_swing, limb_swing_amount);
    }
    let age = render_state.age_in_ticks;
    match pose {
        // The folded `arms` part is static — vanilla never animates it (it swings the invisible
        // separate arms), so the crossed illager only swings its legs.
        IllagerArmPose::Crossed => {}
        IllagerArmPose::Swing => {
            if !render_state.is_riding {
                for name in ["right_arm", "left_arm"] {
                    let arm = root.child_mut(name);
                    arm.pose = humanoid_arm_swing_pose(arm.pose, limb_swing, limb_swing_amount);
                }
            }
            // Vanilla `Pillager.getArmPose`: `CROSSBOW_CHARGE` (drawing) takes priority over
            // `CROSSBOW_HOLD` (holding a level crossbow), and both take priority over `ATTACKING`.
            // A charging pillager pulls the crossbow back ([`apply_crossbow_charge_pose`] over the
            // projected draw ticks), overwriting the walk swing; otherwise a held crossbow levels along
            // the head look.
            if render_state.kind.illager().is_charging_crossbow {
                apply_crossbow_charge_pose(
                    &mut root,
                    render_state.crossbow_charge_duration_ticks,
                    render_state.crossbow_charge_ticks,
                );
            } else if illager_is_holding_crossbow(instance) {
                apply_crossbow_hold_pose(&mut root, render_state.head_yaw, render_state.head_pitch);
            }
        }
        IllagerArmPose::Spellcast => {
            // Vanilla overwrites both separate arms' rotations with the spellcasting pose.
            let right = root.child_mut("right_arm");
            right.pose = illager_spellcast_arm_pose(right.pose, age, true);
            let left = root.child_mut("left_arm");
            left.pose = illager_spellcast_arm_pose(left.pose, age, false);
        }
        IllagerArmPose::BowAndArrow => {
            apply_illager_bow_aim(&mut root, render_state.head_yaw, render_state.head_pitch);
        }
        IllagerArmPose::Celebrating => {
            let right = root.child_mut("right_arm");
            right.pose = illager_celebrate_arm_pose(right.pose, age, true);
            let left = root.child_mut("left_arm");
            left.pose = illager_celebrate_arm_pose(left.pose, age, false);
        }
        IllagerArmPose::Attacking => {
            // Illager ATTACKING (vindicator and pillager without a crossbow): vanilla checks the
            // rendered main-hand item. Empty hands use the same held-out `animateZombieArms` as zombies;
            // armed hands raise the weapon overhead and chop with `swingWeaponDown`. IllagerModel is
            // not a HumanoidModel, so there is no body twist in either branch.
            if render_state.kind.illager().main_hand_empty {
                apply_zombie_arms_held_out_named(&mut root, true, render_state.attack_anim, age);
            } else {
                apply_humanoid_weapon_swing_down(&mut root, render_state.attack_anim, age);
            }
        }
    }
    // Vanilla `IllagerModel` hides the hat except on the illusioner (`IllusionerRenderer`), and
    // toggles the folded arms against the separate arms.
    root.part_mut("hat").unwrap().visible = family == IllagerModelFamily::Illusioner;
    let crossed = !pose.uses_separate_arms();
    root.child_mut("arms").visible = crossed;
    root.child_mut("left_arm").visible = !crossed;
    root.child_mut("right_arm").visible = !crossed;
    root
}

fn apply_illager_riding_pose(root: &mut ModelPart, has_separate_arms: bool) {
    use std::f32::consts::PI;

    if has_separate_arms {
        for name in ["right_arm", "left_arm"] {
            root.child_mut(name).pose.rotation = [-PI / 5.0, 0.0, 0.0];
        }
    }
    root.child_mut("right_leg").pose.rotation = [-1.4137167, PI / 10.0, 0.07853982];
    root.child_mut("left_leg").pose.rotation = [-1.4137167, -PI / 10.0, -0.07853982];
}
