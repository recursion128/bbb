use super::{
    apply_crossbow_charge_pose, apply_crossbow_hold_pose, apply_humanoid_arm_bob_named,
    apply_humanoid_attack_animation, apply_humanoid_head_look, apply_humanoid_limb_walk,
    apply_humanoid_mob_spear_arm_poses, apply_humanoid_no_swing_attack_animation,
    apply_humanoid_stab_attack_animation, apply_humanoid_weapon_swing_down,
    apply_zombie_arms_held_out_named, humanoid_arm_bob_pose, piglin_ear_flap_pose,
    PIGLIN_ADULT_EAR_ANGLE, PIGLIN_BABY_EAR_ANGLE,
};
use crate::entity_models::catalog::PiglinModelFamily;
use crate::entity_models::instances::EntityModelInstance;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::{BakedRoot, ModelLayer};

/// Whether a piglin renders the baby layer. The brute reuses the adult model even as a baby, so only
/// the (zombified) piglin uses the smaller baby layout. Drives the part tree, ear children, and the
/// default ear angle.
fn piglin_baby_layout(family: PiglinModelFamily, baby: bool) -> bool {
    baby && family != PiglinModelFamily::PiglinBrute
}

/// `AbstractPiglinModel.getDefaultEarAngleInDegrees()` (in radians): `5°` baby, `30°` adult/brute.
fn piglin_default_ear_angle(baby_layout: bool) -> f32 {
    if baby_layout {
        PIGLIN_BABY_EAR_ANGLE
    } else {
        PIGLIN_ADULT_EAR_ANGLE
    }
}

/// Vanilla `PiglinModel.setupAnim`'s `DANCING` branch (the soul-campfire celebration): the ears sway
/// (`±π/6 ∓ trig(dancePos·30)·10°`, overwriting the idle flap), the head bobs side-to-side and up
/// (`x += sin(dancePos·10)`, `y += sin(dancePos·40) + 0.4`), both arms raise overhead and wag
/// (`rightArm.zRot = (70° + cos(dancePos·40)·10°)`, the left mirrored, `y += sin(dancePos·40)·0.5 ∓ 0.5`),
/// and the body bounces (`y += sin(dancePos·40)·0.35`). `dancePos = ageInTicks / 60`. Runs after the
/// inherited walk + ear flap, overwriting the ear/arm rotations and adding the bob offsets onto the
/// reset bind pose.
fn apply_piglin_dance(root: &mut ModelPart, age_in_ticks: f32) {
    use std::f32::consts::PI;
    let dance_pos = age_in_ticks / 60.0;
    let deg = PI / 180.0;
    let bob = (dance_pos * 40.0).sin();
    {
        let head = root.child_mut("head");
        head.child_mut("right_ear").pose.rotation[2] =
            PI / 6.0 + deg * (dance_pos * 30.0).sin() * 10.0;
        head.child_mut("left_ear").pose.rotation[2] =
            -PI / 6.0 - deg * (dance_pos * 30.0).cos() * 10.0;
        head.pose.offset[0] += (dance_pos * 10.0).sin();
        head.pose.offset[1] += bob + 0.4;
    }
    let right_arm_zrot = deg * (70.0 + (dance_pos * 40.0).cos() * 10.0);
    {
        let right_arm = root.child_mut("right_arm");
        right_arm.pose.rotation[2] = right_arm_zrot;
        right_arm.pose.offset[1] += bob * 0.5 - 0.5;
    }
    {
        let left_arm = root.child_mut("left_arm");
        left_arm.pose.rotation[2] = -right_arm_zrot;
        left_arm.pose.offset[1] += bob * 0.5 + 0.5;
    }
    root.child_mut("body").pose.offset[1] += bob * 0.35;
}

/// Vanilla `PiglinModel.holdWeaponHigh` (mainArm = RIGHT): the main (right) arm raises overhead
/// (`xRot = -1.8`), holding the melee weapon high while not mid-swing. Only the main arm's pitch is
/// overwritten — the walk-swing yaw/roll and idle bob from `super.setupAnim` stay, and the off arm keeps
/// its walk swing — mirroring vanilla's single-field write. Left-handed mobs (`mainArm = LEFT`, which
/// would raise the left arm) are not projected, so this always raises the right arm.
fn hold_weapon_high(root: &mut ModelPart) {
    root.child_mut("right_arm").pose.rotation[0] = -1.8;
}

/// Vanilla `PiglinModel.setupAnim` `ADMIRING_ITEM` (mainArm = RIGHT): the head tilts down to look at the
/// admired item (`head.xRot = 0.5`, `head.yRot = 0`, overwriting the head look) and the OFF (left) arm
/// lifts the offhand item up (`leftArm.xRot = -0.9`, `leftArm.yRot = 0.5`, overwriting the walk swing; the
/// roll/`zRot` from the idle bob is kept, matching vanilla's single-axis writes). The main (right) arm
/// keeps its walk swing. Left-handed mobs (which would pose the right arm) are not projected, so this
/// always poses the left arm.
fn apply_piglin_admire(root: &mut ModelPart) {
    {
        let head = root.child_mut("head");
        head.pose.rotation[0] = 0.5;
        head.pose.rotation[1] = 0.0;
    }
    let left = root.child_mut("left_arm");
    left.pose.rotation[0] = -0.9;
    left.pose.rotation[1] = 0.5;
}

/// Mutable piglin model, mirroring vanilla `AbstractPiglinModel extends HumanoidModel` (the piglin,
/// piglin brute, and zombified piglin). The unified tree is built for the `family`/`baby` layout with
/// the vanilla child names. `setup_anim` runs `super.setupAnim` — the head look ([`apply_head_look`]
/// on `head`), the humanoid limb swing ([`apply_humanoid_limb_walk`]), arm poses/attack, and trailing
/// bob ([`apply_humanoid_arm_bob_named`]) — then it always
/// flaps the two ears ([`piglin_ear_flap_pose`], head children). Zombified piglins overwrite the inherited
/// arm swing with the held-out `animateZombieArms` pose ([`apply_zombie_arms_held_out_named`]). A regular
/// piglin holding a charged crossbow then levels it ([`apply_crossbow_hold_pose`], the `CROSSBOW_HOLD`
/// pose); an aggressive piglin/brute holding a melee weapon raises and swings it ([`hold_weapon_high`] /
/// [`apply_humanoid_weapon_swing_down`], the `ATTACKING_WITH_MELEE_WEAPON` pose); a regular piglin with a
/// piglin-loved offhand item admires it ([`apply_piglin_admire`], the `ADMIRING_ITEM` pose); and a dancing
/// regular piglin runs the `DANCING` pose ([`apply_piglin_dance`]). The family recolor/texture is supplied
/// by the caller; held items defer.
pub(in crate::entity_models) fn piglin_model(
    family: PiglinModelFamily,
    baby: bool,
    instance: &EntityModelInstance,
) -> BakedRoot {
    let baby_layout = piglin_baby_layout(family, baby);
    let layer = match (family, baby_layout) {
        (PiglinModelFamily::Piglin, false) => ModelLayer::PIGLIN,
        (PiglinModelFamily::Piglin, true) => ModelLayer::PIGLIN_BABY,
        (PiglinModelFamily::PiglinBrute, _) => ModelLayer::PIGLIN_BRUTE,
        (PiglinModelFamily::ZombifiedPiglin, false) => ModelLayer::ZOMBIFIED_PIGLIN,
        (PiglinModelFamily::ZombifiedPiglin, true) => ModelLayer::ZOMBIFIED_PIGLIN_BABY,
    };
    let mut root = layer.bake();
    let render_state = &instance.render_state;
    let limb_swing = render_state.walk_animation_pos;
    let limb_swing_amount = render_state.walk_animation_speed;
    let age_in_ticks = render_state.age_in_ticks;
    apply_humanoid_head_look(
        root.child_mut("head"),
        render_state.head_yaw,
        render_state.head_pitch,
        render_state.is_fall_flying,
        render_state.swim_amount,
    );
    apply_humanoid_limb_walk(
        &mut root,
        limb_swing,
        limb_swing_amount,
        render_state.humanoid_speed_value,
    );
    apply_humanoid_mob_spear_arm_poses(
        &mut root,
        render_state.head_yaw,
        render_state.head_pitch,
        render_state.humanoid_mob_main_hand_spear_pose,
        render_state.humanoid_mob_off_hand_spear_pose,
        render_state.swim_amount,
    );
    // Dynamic dispatch of `HumanoidModel.setupAttackAnimation` happens before the inherited
    // arm bob. A live Piglin melee swing replaces the generic attack with `swingWeaponDown`
    // (which includes its own bob); every other branch uses the item swing type normally.
    if family != PiglinModelFamily::ZombifiedPiglin
        && render_state.kind.piglin().attacking_with_melee
        && render_state.attack_anim > 0.0
    {
        apply_humanoid_weapon_swing_down(&mut root, render_state.attack_anim, age_in_ticks);
    } else if render_state.main_hand_swing_is_stab {
        apply_humanoid_stab_attack_animation(
            &mut root,
            render_state.attack_anim,
            render_state.attack_arm_off_hand,
            render_state.age_scale,
        );
    } else if render_state.main_hand_swing_is_none {
        apply_humanoid_no_swing_attack_animation(
            &mut root,
            render_state.attack_anim,
            render_state.attack_arm_off_hand,
            render_state.age_scale,
        );
    } else {
        apply_humanoid_attack_animation(
            &mut root,
            render_state.attack_anim,
            render_state.attack_arm_off_hand,
            render_state.head_pitch,
            render_state.age_scale,
        );
    }
    apply_humanoid_arm_bob_named(&mut root, age_in_ticks, true, true);
    // Flap the two ears (head children) every frame.
    let default_ear_angle = piglin_default_ear_angle(baby_layout);
    let head = root.child_mut("head");
    let left = head.child_mut("left_ear");
    left.pose = piglin_ear_flap_pose(
        left.pose,
        true,
        default_ear_angle,
        age_in_ticks,
        limb_swing,
        limb_swing_amount,
    );
    let right = head.child_mut("right_ear");
    right.pose = piglin_ear_flap_pose(
        right.pose,
        false,
        default_ear_angle,
        age_in_ticks,
        limb_swing,
        limb_swing_amount,
    );
    if family == PiglinModelFamily::ZombifiedPiglin {
        // Vanilla `ZombifiedPiglinModel.setupAnim` runs `AnimationUtils.animateZombieArms` after
        // `AbstractPiglinModel`: WHACK replaces the inherited arm pose, while STAB skips that rewrite
        // and only applies the trailing `bobArms`.
        if render_state.main_hand_swing_is_stab {
            for name in ["right_arm", "left_arm"] {
                let arm = root.child_mut(name);
                arm.pose = humanoid_arm_bob_pose(arm.pose, age_in_ticks);
            }
        } else {
            apply_zombie_arms_held_out_named(
                &mut root,
                render_state.is_aggressive,
                render_state.attack_anim,
                age_in_ticks,
            );
        }
    }
    // Vanilla `PiglinModel.setupAnim` `CROSSBOW_HOLD`: a regular piglin holding a charged crossbow
    // levels it along the head look, overwriting the walk arm swing. Mutually exclusive with DANCING
    // (the projection gates it off while dancing/charging), and applied before the dance block so a
    // dancing piglin (higher priority) still wins.
    if render_state.kind.piglin().crossbow_hold {
        apply_crossbow_hold_pose(&mut root, render_state.head_yaw, render_state.head_pitch);
    }
    // Vanilla `PiglinModel.setupAnim` `CROSSBOW_CHARGE`: a regular piglin drawing its crossbow pulls it
    // back, the left (pulling) arm lerping from rest to full draw over the projected use-item ticks,
    // overwriting the walk arm swing. Higher priority than CROSSBOW_HOLD (applied after it so it wins
    // when both somehow fire), below ATTACKING / ADMIRING / DANCING (the projection gates it off under
    // those, and they apply later so they still win).
    if render_state.kind.piglin().crossbow_charge {
        apply_crossbow_charge_pose(
            &mut root,
            render_state.crossbow_charge_duration_ticks,
            render_state.crossbow_charge_ticks,
        );
    }
    // Vanilla `PiglinModel` `ATTACKING_WITH_MELEE_WEAPON`: a piglin/brute that is aggressive and holds
    // a melee weapon raises it overhead at rest (`holdWeaponHigh`, in `setupAnim`) and chops it down
    // across an attack swing (`AnimationUtils.swingWeaponDown`, in `setupAttackAnimation`), both
    // overwriting the walk arm swing. Higher priority than CROSSBOW_HOLD (and item-exclusive with it),
    // applied after it; lower than DANCING, applied before the dance block so a dancing piglin wins.
    if render_state.kind.piglin().attacking_with_melee && render_state.attack_anim <= 0.0 {
        hold_weapon_high(&mut root);
    }
    // Vanilla `PiglinModel.setupAnim` ADMIRING_ITEM: a regular piglin with a piglin-loved offhand item
    // tilts its head down and lifts the off arm to show it. Higher priority than CROSSBOW_HOLD /
    // ATTACKING (the projection gates those off while admiring), below DANCING (applied after).
    if render_state.kind.piglin().admiring {
        apply_piglin_admire(&mut root);
    }
    // Vanilla `PiglinModel.setupAnim` DANCING runs after the inherited walk + ear flap, overwriting
    // the ear/arm rotations and bobbing the head/body. Only the regular piglin dances (the
    // projection gates `piglin_dancing` to it; the brute and zombified piglin never set it).
    if render_state.kind.piglin().dancing {
        apply_piglin_dance(&mut root, age_in_ticks);
    }
    root
}
