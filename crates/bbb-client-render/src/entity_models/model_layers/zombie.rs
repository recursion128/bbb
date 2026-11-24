use super::{
    apply_humanoid_arm_bob_named, apply_humanoid_attack_animation, apply_humanoid_head_look,
    apply_humanoid_limb_walk, apply_humanoid_mob_spear_arm_poses,
    apply_humanoid_no_swing_attack_animation, apply_humanoid_stab_attack_animation,
    apply_zombie_arms_held_out_named, humanoid_arm_bob_pose, humanoid_rot_lerp_rad,
};
use crate::entity_models::catalog::ZombieVariantModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Vanilla `ZombieModel.setupAnim` (`super.setupAnim` then `AnimationUtils.animateZombieArms`),
/// shared by the plain zombie and every zombie variant (husk, drowned, zombie villager): look the
/// head ([`apply_head_look`] on `head`), then either run the vanilla STAB swing branch for an attack-arm
/// spear or override the arms with the held-out `animateZombieArms` pose
/// ([`apply_zombie_arms_held_out_named`], `isAggressive`-driven and swung by `attack_anim`).
fn apply_zombie_family_anim(root: &mut ModelPart, instance: &EntityModelInstance) {
    let render_state = &instance.render_state;
    apply_humanoid_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
        render_state.is_fall_flying,
        render_state.swim_amount,
    );
    apply_humanoid_limb_walk(
        root,
        render_state.walk_animation_pos,
        render_state.walk_animation_speed,
        render_state.humanoid_speed_value,
    );
    apply_humanoid_mob_spear_arm_poses(
        root,
        render_state.head_yaw,
        render_state.head_pitch,
        render_state.humanoid_mob_main_hand_spear_pose,
        render_state.humanoid_mob_off_hand_spear_pose,
        render_state.swim_amount,
    );
    if render_state.main_hand_swing_is_stab {
        apply_humanoid_stab_attack_animation(
            root,
            render_state.attack_anim,
            render_state.attack_arm_off_hand,
            render_state.age_scale,
        );
    } else if render_state.main_hand_swing_is_none {
        apply_humanoid_no_swing_attack_animation(
            root,
            render_state.attack_anim,
            render_state.attack_arm_off_hand,
            render_state.age_scale,
        );
    } else {
        apply_humanoid_attack_animation(
            root,
            render_state.attack_anim,
            render_state.attack_arm_off_hand,
            render_state.head_pitch,
            render_state.age_scale,
        );
    }
    // This is HumanoidModel's shared post-pose/post-attack bob. The zombie subclass then either
    // overwrites the arms with its held-out pose or, for STAB, preserves them and bobs once more.
    apply_humanoid_arm_bob_named(root, render_state.age_in_ticks, true, true);
    if render_state.main_hand_swing_is_stab {
        // Vanilla `AnimationUtils.animateZombieArms` still runs after `super.setupAnim`, but the STAB branch
        // skips the held-out arm rewrite and only applies `bobArms`.
        for name in ["right_arm", "left_arm"] {
            let arm = root.child_mut(name);
            arm.pose = humanoid_arm_bob_pose(arm.pose, render_state.age_in_ticks);
        }
    } else {
        apply_zombie_arms_held_out_named(
            root,
            render_state.is_aggressive,
            render_state.attack_anim,
            render_state.age_in_ticks,
        );
    }
}

/// Vanilla `DrownedModel.setupAnim` `THROW_TRIDENT` override: after the held-out zombie arms, the main
/// (right) arm raises the trident straight overhead to throw (`rightArm.xRot = rightArm.xRot * 0.5 - π`,
/// `rightArm.yRot = 0`). Only the main arm is posed (`getMainArm() == arm`); left-handed mobs are not
/// projected, so this always poses the right arm.
fn apply_drowned_throw_trident(root: &mut ModelPart) {
    let right = root.child_mut("right_arm");
    right.pose.rotation[0] = right.pose.rotation[0] * 0.5 - std::f32::consts::PI;
    right.pose.rotation[1] = 0.0;
}

/// Vanilla `DrownedModel.setupAnim` swim override, applied after the trident pose. `swimAmount`
/// blends both arms toward the folded-back swimming pose, adds the out-of-phase arm/leg sine wave,
/// rolls the arms, and zeroes the head pitch.
fn apply_drowned_swim_pose(root: &mut ModelPart, instance: &EntityModelInstance) {
    let swim_amount = instance.render_state.swim_amount;
    if swim_amount <= 0.0 {
        return;
    }

    let age_wave = (0.1 * instance.render_state.age_in_ticks).sin();
    let right = root.child_mut("right_arm");
    right.pose.rotation[0] = humanoid_rot_lerp_rad(
        swim_amount,
        right.pose.rotation[0],
        -std::f32::consts::PI * 4.0 / 5.0,
    ) + swim_amount * 0.35 * age_wave;
    right.pose.rotation[2] = humanoid_rot_lerp_rad(swim_amount, right.pose.rotation[2], -0.15);

    let left = root.child_mut("left_arm");
    left.pose.rotation[0] = humanoid_rot_lerp_rad(
        swim_amount,
        left.pose.rotation[0],
        -std::f32::consts::PI * 4.0 / 5.0,
    ) - swim_amount * 0.35 * age_wave;
    left.pose.rotation[2] = humanoid_rot_lerp_rad(swim_amount, left.pose.rotation[2], 0.15);

    root.child_mut("left_leg").pose.rotation[0] -= swim_amount * 0.55 * age_wave;
    root.child_mut("right_leg").pose.rotation[0] += swim_amount * 0.55 * age_wave;
    root.child_mut("head").pose.rotation[0] = 0.0;
}

/// Mutable zombie model, mirroring vanilla `ZombieModel` (an `AbstractZombieModel` over `HumanoidModel`).
/// The unified tree is built for the selected `baby` layout with the vanilla child names. `setup_anim`
/// runs the shared [`apply_zombie_family_anim`].
pub(in crate::entity_models) fn zombie_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = if baby {
        ModelLayer::ZOMBIE_BABY
    } else {
        ModelLayer::ZOMBIE
    };
    let mut root = layer.bake();
    apply_zombie_family_anim(&mut root, instance);
    root
}

pub(in crate::entity_models) fn giant_model(instance: &EntityModelInstance) -> BakedRoot {
    let mut root = ModelLayer::GIANT.bake();
    apply_zombie_family_anim(&mut root, instance);
    root
}

/// Mutable zombie-variant model, mirroring vanilla `HuskRenderer`/`DrownedRenderer`/
/// `ZombieVillagerRenderer` — all of which inherit `ZombieModel.setupAnim`. The unified tree is
/// selected by `family`/`baby`: the husk and drowned reuse the plain zombie body, the zombie villager
/// builds its own robed tree; `setup_anim` runs the shared [`apply_zombie_family_anim`], then an aggressive
/// trident-holding drowned raises the trident to throw ([`apply_drowned_throw_trident`]) and then applies
/// the `swimAmount` limb override.
pub(in crate::entity_models) fn zombie_variant_model(
    family: ZombieVariantModelFamily,
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = match (family, baby) {
        (ZombieVariantModelFamily::Husk, false) => ModelLayer::HUSK,
        (ZombieVariantModelFamily::Husk, true) => ModelLayer::HUSK_BABY,
        (ZombieVariantModelFamily::Drowned, false) => ModelLayer::DROWNED,
        (ZombieVariantModelFamily::Drowned, true) => ModelLayer::DROWNED_BABY,
        (ZombieVariantModelFamily::ZombieVillager, false) => ModelLayer::ZOMBIE_VILLAGER,
        (ZombieVariantModelFamily::ZombieVillager, true) => ModelLayer::ZOMBIE_VILLAGER_BABY,
    };
    let mut root = layer.bake();
    apply_zombie_family_anim(&mut root, instance);
    // Vanilla `DrownedModel.setupAnim` raises the trident to throw after the held-out zombie arms.
    // Only the drowned sets this flag (the husk / zombie villager never throw).
    if instance.render_state.kind.drowned().throw_trident {
        apply_drowned_throw_trident(&mut root);
    }
    if family == ZombieVariantModelFamily::Drowned {
        apply_drowned_swim_pose(&mut root, instance);
    }
    root
}

/// The drowned outer-layer overlay model, mirroring vanilla `DrownedOuterLayer`: a second white
/// `coloredCutoutModelCopyLayerRender` copy of the inflated drowned body drawn with
/// `drowned_outer_layer.png` (adult) / `drowned_outer_layer_baby.png` (baby). It is a `DrownedModel`
/// (adult `DrownedModel.createBodyLayer(0.25)`) / `BabyDrownedModel` (baby
/// `BabyZombieModel.createBodyLayer(0.25)`) in its own right, so it is posed by the exact same animator
/// as the base (the shared `ZombieModel.setupAnim` plus the drowned trident throw and swim override),
/// keeping the inflated shell glued to the limbs.
pub(in crate::entity_models) fn drowned_outer_model(
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let layer = if baby {
        ModelLayer::DROWNED_BABY_OUTER
    } else {
        ModelLayer::DROWNED_OUTER
    };
    let mut root = layer.bake();
    apply_zombie_family_anim(&mut root, instance);
    if instance.render_state.kind.drowned().throw_trident {
        apply_drowned_throw_trident(&mut root);
    }
    apply_drowned_swim_pose(&mut root, instance);
    root
}
