use super::*;
use crate::entity_models::model_layers::{zombie_model, zombie_variant_model};

use crate::entity_models::catalog::{
    zombie_villager_level_texture_ref, zombie_villager_profession_texture_ref,
    zombie_villager_type_texture_ref,
};

#[test]
fn zombie_arm_held_out_pose_matches_vanilla_resting_animate_zombie_arms() {
    // Vanilla AnimationUtils.animateZombieArms at attackTime = 0 (non-aggressive): both arms
    // drop forward to xRot = -π/2.25, splay to yRot ∓0.1 (right arm -0.1, left +0.1), zero
    // zRot, then take the idle bob. The right arm rests at x = -5, the left at x = +5.
    let right_arm_pose = PartPose {
        offset: [-5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let left_arm_pose = PartPose {
        offset: [5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let arm_drop = -std::f32::consts::PI / 2.25;
    let right = zombie_arm_held_out_pose(right_arm_pose, false, 0.0, 0.0);
    let left = zombie_arm_held_out_pose(left_arm_pose, false, 0.0, 0.0);
    // At ageInTicks 0 the bob's xRot term is sin(0) * 0.05 = 0, so xRot is the bare arm drop.
    assert!(
        (right.rotation[0] - arm_drop).abs() < 1e-6,
        "right arm drop: {}",
        right.rotation[0]
    );
    assert!((left.rotation[0] - arm_drop).abs() < 1e-6, "left arm drop");
    // The arms splay out (yRot is untouched by the bob): right -0.1, left +0.1.
    assert!((right.rotation[1] - (-0.1)).abs() < 1e-6, "right arm splay");
    assert!((left.rotation[1] - 0.1).abs() < 1e-6, "left arm splay");
    // zRot starts at the held-out 0 and carries only the idle-bob baseline at age 0
    // (right arm +0.1, left -0.1).
    assert!((right.rotation[2] - 0.1).abs() < 1e-6, "right arm bob zRot");
    assert!(
        (left.rotation[2] - (-0.1)).abs() < 1e-6,
        "left arm bob zRot"
    );
    // The pose is set absolutely (the deep arm drop overrides the inherited swing); the
    // offset is preserved.
    assert_eq!(right.offset, right_arm_pose.offset);

    // An aggressive mob (Mob.isAggressive) raises its arms higher: armDrop = -π/1.5, deeper
    // (more negative) than the calm -π/2.25. Only xRot changes; the yRot splay and the bob
    // are unchanged.
    let aggressive_arm_drop = -std::f32::consts::PI / 1.5;
    let aggressive_right = zombie_arm_held_out_pose(right_arm_pose, true, 0.0, 0.0);
    assert!(
        (aggressive_right.rotation[0] - aggressive_arm_drop).abs() < 1e-6,
        "aggressive right arm drop: {}",
        aggressive_right.rotation[0]
    );
    assert!(
        aggressive_right.rotation[0] < right.rotation[0],
        "aggressive arms are raised higher (deeper drop) than calm: {} vs {}",
        aggressive_right.rotation[0],
        right.rotation[0]
    );
    assert!(
        (aggressive_right.rotation[1] - (-0.1)).abs() < 1e-6,
        "splay unchanged"
    );
    assert!(
        (aggressive_right.rotation[2] - 0.1).abs() < 1e-6,
        "bob zRot unchanged"
    );
}

#[test]
fn zombie_arm_held_out_pose_swings_during_a_melee_strike() {
    use std::f32::consts::PI;
    // Vanilla AnimationUtils.animateZombieArms attack terms (attackTime > 0): attackYRot = sin(t·π)
    // lifts both arms' yRot toward center (yRot = ±(0.1 - attackYRot·0.6)) and the xRot chops
    // (`+= attackYRot·1.2 - sin((1-(1-t)²)·π)·0.4`), over the held-out arm drop.
    let right_arm_pose = PartPose {
        offset: [-5.0, 2.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
    };
    let t = 0.5_f32;
    let arm_drop = -PI / 2.25;
    let attack_y = (t * PI).sin();
    let attack_x = ((1.0 - (1.0 - t) * (1.0 - t)) * PI).sin();

    let resting = zombie_arm_held_out_pose(right_arm_pose, false, 0.0, 0.0);
    let swinging = zombie_arm_held_out_pose(right_arm_pose, false, t, 0.0);

    // The right arm chops up off the rest drop (xRot rises by attackYRot·1.2 - attackX·0.4).
    assert!(
        (swinging.rotation[0] - (arm_drop + attack_y * 1.2 - attack_x * 0.4)).abs() < 1e-6,
        "right arm xRot swings: {}",
        swinging.rotation[0]
    );
    assert!(
        swinging.rotation[0] > resting.rotation[0],
        "the strike lifts the arm off the held-out drop"
    );
    // The yRot pulls in toward center: right arm -(0.1 - attackYRot·0.6) = +0.2 at t = 0.5.
    assert!(
        (swinging.rotation[1] - (-(0.1 - attack_y * 0.6))).abs() < 1e-6,
        "right arm yRot pulls in: {}",
        swinging.rotation[1]
    );

    // attackTime 0 reproduces the resting held-out pose exactly.
    assert_eq!(
        zombie_arm_held_out_pose(right_arm_pose, false, 0.0, 0.0),
        resting
    );
}

#[test]
fn zombie_whack_swing_keeps_inherited_body_twist_and_arm_anchors() {
    use std::f32::consts::PI;

    // Vanilla `AbstractZombieModel.setupAnim`: `super.setupAnim` first runs
    // `HumanoidModel.setupAttackAnimation` for a WHACK swing, then `AnimationUtils.animateZombieArms`
    // overwrites the arm rotations. The body twist and arm anchor offsets from the inherited helper remain.
    let t = 0.1_f32;
    let mut model = zombie_model(
        false,
        &EntityModelInstance::zombie(63, [0.0, 64.0, 0.0], 0.0, false).with(|s| s.attack_anim = t),
    );

    let body_yrot = (t.sqrt() * PI * 2.0).sin() * 0.2;
    assert!(
        (model.child_mut("body").pose.rotation[1] - body_yrot).abs() < 1e-6,
        "WHACK keeps the inherited HumanoidModel body twist"
    );
    let right = model.child_mut("right_arm").pose;
    assert!((right.offset[0] - (-body_yrot.cos() * 5.0)).abs() < 1e-6);
    assert!((right.offset[2] - (body_yrot.sin() * 5.0)).abs() < 1e-6);

    let attack_y = (t * PI).sin();
    let attack_x = ((1.0 - (1.0 - t) * (1.0 - t)) * PI).sin();
    let held_out_x = -PI / 2.25 + attack_y * 1.2 - attack_x * 0.4;
    assert!(
        (right.rotation[0] - held_out_x).abs() < 1e-6,
        "animateZombieArms still overwrites the WHACK arm pitch"
    );
    assert!((right.rotation[1] - (-(0.1 - attack_y * 0.6))).abs() < 1e-6);
}

#[test]
fn zombie_stab_swing_uses_spear_lunge_instead_of_held_out_rewrite() {
    use std::f32::consts::PI;

    // Vanilla `AbstractZombieModel.setupAnim`: `super.setupAnim` runs
    // `HumanoidModel.setupAttackAnimation`. For `SwingAnimationType.STAB` this applies
    // `SpearAnimations.thirdPersonAttackHand`; the subsequent `AnimationUtils.animateZombieArms` sees STAB
    // and skips the held-out rewrite, keeping only its final `bobArms`.
    let t = 0.1_f32;
    let age = 11.0_f32;
    let base = EntityModelInstance::zombie(64, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -10.0))
        .with(|s| s.age_in_ticks = age);

    let stab_instance = base.with(|s| {
        s.attack_anim = t;
        s.main_hand_swing_is_stab = true;
        s.humanoid_mob_main_hand_spear_pose = true;
        s.humanoid_mob_off_hand_spear_pose = true;
    });
    let mut stab_model = zombie_model(false, &stab_instance);

    let body_yrot = (t.sqrt() * PI * 2.0).sin() * 0.2;
    assert!(
        (stab_model.child_mut("body").pose.rotation[1] - body_yrot).abs() < 1e-6,
        "STAB still runs the HumanoidModel attack body twist"
    );

    let progress =
        |time: f32, start: f32, end: f32| ((time - start) / (end - start)).clamp(0.0, 1.0);
    let in_out_sine = |x: f32| -((PI * x).cos() - 1.0) / 2.0;
    let in_out_expo = |x: f32| {
        if x < 0.5 {
            if x == 0.0 {
                0.0
            } else {
                2.0_f32.powf(20.0 * x - 10.0) / 2.0
            }
        } else if x == 1.0 {
            1.0
        } else {
            (2.0 - 2.0_f32.powf(-20.0 * x + 10.0)) / 2.0
        }
    };
    let stab_lunge = {
        let prepare = in_out_sine(progress(t, 0.0, 0.05));
        let attack = progress(t, 0.05, 0.2).powi(2);
        let retract = in_out_expo(progress(t, 0.4, 1.0));
        (90.0 * prepare - 120.0 * attack + 30.0 * retract).to_radians()
    };
    let bob_x = (age * 0.067).sin() * 0.05;
    let bob_z = (age * 0.09).cos() * 0.05 + 0.05;
    let right = stab_model.child_mut("right_arm").pose;
    assert!((right.offset[0] - (-body_yrot.cos() * 5.0)).abs() < 1e-6);
    assert!((right.offset[2] - (body_yrot.sin() * 5.0)).abs() < 1e-6);
    assert!(
        (right.rotation[0] - (stab_lunge + 2.0 * bob_x)).abs() < 1e-6,
        "right arm lunges then takes Humanoid + zombie-subclass bob: {} vs {}",
        right.rotation[0],
        stab_lunge + 2.0 * bob_x
    );
    assert!(
        right.rotation[1].abs() < 1e-6,
        "thirdPersonAttackHand cancels the body-yRot arm yaw addition"
    );
    assert!(
        (right.rotation[2] - 2.0 * bob_z).abs() < 1e-6,
        "HumanoidModel bob plus animateZombieArms STAB bob both remain"
    );

    let whack_instance = base.with(|s| s.attack_anim = t);
    let mut whack_model = zombie_model(false, &whack_instance);
    let whack_right = whack_model.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - whack_right.rotation[0]).abs() > 0.5,
        "STAB must not fall back to the zombie held-out melee rewrite"
    );
}

#[test]
fn zombie_stab_type_skips_held_out_rewrite_without_attack_progress() {
    let pitch = (-10.0_f32).to_radians();
    let spear_base_x = (-std::f32::consts::FRAC_PI_2 + pitch + 0.8)
        .to_degrees()
        .clamp(-120.0, 30.0)
        .to_radians();

    // `AnimationUtils.animateZombieArms` checks `swingAnimationType != STAB`, not attack progress. A zombie
    // whose attack-arm item is a spear therefore skips the held-out rewrite even at attackTime 0. The
    // zombie renderer marks both arm poses from the main-hand STAB component; right-handed setup applies
    // the off-hand SPEAR pose first and skips the main hand.
    let spear = EntityModelInstance::zombie(65, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -10.0))
        .with(|s| {
            s.main_hand_swing_is_stab = true;
            s.humanoid_mob_main_hand_spear_pose = true;
            s.humanoid_mob_off_hand_spear_pose = true;
        });
    let mut spear_model = zombie_model(false, &spear);

    let left = spear_model.child_mut("left_arm").pose;
    assert!((left.rotation[0] - spear_base_x).abs() < 1e-6);
    assert!((left.rotation[1] - 0.1).abs() < 1e-6);

    let right = spear_model.child_mut("right_arm").pose;
    assert!(
        right.rotation[0].abs() < 1e-6,
        "main-hand pose is skipped by the zombie off-hand SPEAR affectsOffhandPose dispatch"
    );
    let mut held_out = zombie_model(
        false,
        &EntityModelInstance::zombie(66, [0.0, 64.0, 0.0], 0.0, false),
    );
    assert!(
        (right.rotation[0] - held_out.child_mut("right_arm").pose.rotation[0]).abs() > 1.0,
        "STAB swingAnimationType must not fall back to animateZombieArms held-out rewrite"
    );
}

#[test]
fn drowned_raises_its_trident_to_throw() {
    use std::f32::consts::PI;
    // Vanilla `DrownedModel.setupAnim` THROW_TRIDENT: after the held-out zombie arms, the main (right)
    // arm raises the trident overhead (`rightArm.xRot = rightArm.xRot * 0.5 - π`, `rightArm.yRot = 0`).
    let drowned = |throwing: bool| {
        EntityModelInstance::zombie_variant(
            38,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Drowned,
            false,
        )
        .with(|s| {
            s.is_aggressive = true;
            s.age_in_ticks = 9.0;
            s.kind.drowned_mut().throw_trident = throwing;
        })
    };
    let mut not_throwing =
        zombie_variant_model(ZombieVariantModelFamily::Drowned, false, &drowned(false));
    let held_out_x = not_throwing.child_mut("right_arm").pose.rotation[0];
    let mut throwing =
        zombie_variant_model(ZombieVariantModelFamily::Drowned, false, &drowned(true));
    let right = throwing.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (held_out_x * 0.5 - PI)).abs() < 1.0e-6,
        "the throw raises the right arm overhead off the held-out drop: {} vs {}",
        right.rotation[0],
        held_out_x * 0.5 - PI
    );
    assert!(
        right.rotation[1].abs() < 1.0e-6,
        "the throwing arm faces forward (yRot = 0): {}",
        right.rotation[1]
    );
    // The raised arm points up-and-back (xRot near -π), well off the held-out forward drop.
    assert!(
        right.rotation[0] < -2.0,
        "the trident is raised overhead: {}",
        right.rotation[0]
    );

    // The throw visibly changes the mesh versus an aggressive drowned not throwing.
    assert_ne!(
        entity_model_mesh(&[drowned(false)]).vertices,
        entity_model_mesh(&[drowned(true)]).vertices,
        "a throwing drowned no longer holds the plain held-out arms"
    );
}

#[test]
fn drowned_swim_amount_reposes_limbs_after_trident_pose() {
    // Vanilla `DrownedModel.setupAnim` applies the THROW_TRIDENT arm first, then the swimAmount
    // override folds both arms toward `-4π/5`, adds opposite sine waves to arms/legs, rolls the arms,
    // and clears the head pitch.
    let rot_lerp_rad = |a: f32, from: f32, to: f32| {
        let mut diff = to - from;
        while diff < -std::f32::consts::PI {
            diff += 2.0 * std::f32::consts::PI;
        }
        while diff >= std::f32::consts::PI {
            diff -= 2.0 * std::f32::consts::PI;
        }
        from + a * diff
    };
    let base = EntityModelInstance::zombie_variant(
        39,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Drowned,
        false,
    )
    .with(|s| {
        s.is_aggressive = true;
        s.kind.drowned_mut().throw_trident = true;
    })
    .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -25.0))
    .with(|s| s.age_in_ticks = 10.0);
    let swim_amount = 0.5;
    let wave = (0.1_f32 * 10.0).sin();
    let mut dry_model = zombie_variant_model(ZombieVariantModelFamily::Drowned, false, &base);
    let dry_right = dry_model.child_mut("right_arm").pose.rotation;
    let dry_left = dry_model.child_mut("left_arm").pose.rotation;
    let dry_left_leg_x = dry_model.child_mut("left_leg").pose.rotation[0];
    let dry_right_leg_x = dry_model.child_mut("right_leg").pose.rotation[0];
    let mut swimming_model = zombie_variant_model(
        ZombieVariantModelFamily::Drowned,
        false,
        &base.with(|s| s.swim_amount = swim_amount),
    );
    let swim_right = swimming_model.child_mut("right_arm").pose.rotation;
    let swim_left = swimming_model.child_mut("left_arm").pose.rotation;

    assert!(
        (swim_right[0]
            - (rot_lerp_rad(swim_amount, dry_right[0], -std::f32::consts::PI * 4.0 / 5.0,)
                + swim_amount * 0.35 * wave))
            .abs()
            < 1.0e-6,
        "right arm xRot follows DrownedModel swim override"
    );
    assert!(
        (swim_left[0]
            - (rot_lerp_rad(swim_amount, dry_left[0], -std::f32::consts::PI * 4.0 / 5.0,)
                - swim_amount * 0.35 * wave))
            .abs()
            < 1.0e-6,
        "left arm xRot follows opposite swim wave"
    );
    assert!((swim_right[2] - rot_lerp_rad(swim_amount, dry_right[2], -0.15)).abs() < 1.0e-6);
    assert!((swim_left[2] - rot_lerp_rad(swim_amount, dry_left[2], 0.15)).abs() < 1.0e-6);
    assert!(
        (swimming_model.child_mut("left_leg").pose.rotation[0]
            - (dry_left_leg_x - swim_amount * 0.55 * wave))
            .abs()
            < 1.0e-6
    );
    assert!(
        (swimming_model.child_mut("right_leg").pose.rotation[0]
            - (dry_right_leg_x + swim_amount * 0.55 * wave))
            .abs()
            < 1.0e-6
    );
    assert_eq!(swimming_model.child_mut("head").pose.rotation[0], 0.0);
    assert_ne!(
        entity_model_mesh(&[base]).vertices,
        entity_model_mesh(&[base.with(|s| s.swim_amount = swim_amount)]).vertices,
        "the drowned mesh visibly re-poses while swimming"
    );
}

#[test]
fn zombie_textured_layer_passes_match_vanilla_renderer() {
    for (baby, texture) in [(false, ZOMBIE_TEXTURE_REF), (true, ZOMBIE_BABY_TEXTURE_REF)] {
        let passes = zombie_textured_layer_passes(baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    }
    assert!(entity_model_texture_refs().contains(&ZOMBIE_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ZOMBIE_BABY_TEXTURE_REF));
}

#[test]
fn zombie_textured_mesh_geometry_and_legs_swing() {
    let (atlas, _) = build_entity_model_texture_atlas(&zombie_texture_images()).unwrap();
    for baby in [false, true] {
        let instances = [zombie_submission_probe(EntityModelInstance::zombie(
            55,
            [0.0, 64.0, 0.0],
            0.0,
            baby,
        ))];
        let textured_meshes = entity_model_textured_meshes(&instances, &atlas);
        assert_zombie_family_textured_output(&textured_meshes, instances[0]);
        let textured = &textured_meshes.cutout;
        assert!(textured
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

        // Vanilla runs the leg swing every frame; advancing the walk animation re-poses the legs
        // while the projected zombie-arm branch applies.
        let walking = [instances[0]
            .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.0, 1.0))
            .with(|s| s.age_in_ticks = 8.0)];
        let textured_walk_meshes = entity_model_textured_meshes(&walking, &atlas);
        assert_zombie_family_textured_output(&textured_walk_meshes, walking[0]);
        let textured_walk = &textured_walk_meshes.cutout;
        assert_ne!(
            textured.vertices, textured_walk.vertices,
            "legs swing (baby={baby})"
        );
    }
}

fn zombie_texture_images() -> Vec<EntityModelTextureImage> {
    [ZOMBIE_TEXTURE_REF, ZOMBIE_BABY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn husk_textured_layer_passes_reuse_the_zombie_body_layer() {
    // Vanilla `HuskRenderer extends ZombieRenderer`: `ModelLayers.HUSK` is the shared
    // `humanoidBodyLayer` (the adult husk mesh is scaled at the root, not in its UVs) and
    // `HUSK_BABY` is the shared `babyZombieLayer`, so the husk's textured parts are byte-for-byte
    // the zombie's, with only the texture and the adult scale changing.
    assert_eq!(ModelLayer::HUSK, ModelLayer::HUSK);
    assert_eq!(ModelLayer::HUSK_BABY, ModelLayer::HUSK_BABY);

    // The husk reuses the unified `ZombieVariantModel` (plain-zombie) tree, so the layer-pass
    // geometry is vestigial.
    let adult = husk_textured_layer_passes(false);
    assert_eq!(adult.len(), 1);
    let baby = husk_textured_layer_passes(true);
    assert_eq!(baby.len(), 1);
}

#[test]
fn husk_textured_layer_passes_match_vanilla_renderer() {
    for (baby, texture) in [(false, HUSK_TEXTURE_REF), (true, HUSK_BABY_TEXTURE_REF)] {
        let passes = husk_textured_layer_passes(baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    }
    assert!(entity_model_texture_refs().contains(&HUSK_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&HUSK_BABY_TEXTURE_REF));
}

#[test]
fn husk_textured_mesh_geometry_and_legs_swing() {
    let (atlas, _) = build_entity_model_texture_atlas(&husk_texture_images()).unwrap();
    for baby in [false, true] {
        let instances = [zombie_submission_probe(
            EntityModelInstance::zombie_variant(
                56,
                [0.0, 64.0, 0.0],
                0.0,
                ZombieVariantModelFamily::Husk,
                baby,
            ),
        )];
        let textured_meshes = entity_model_textured_meshes(&instances, &atlas);
        assert_zombie_family_textured_output(&textured_meshes, instances[0]);
        let textured = &textured_meshes.cutout;
        assert!(textured
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

        // Vanilla runs the leg swing every frame; advancing the walk animation re-poses the legs
        // while the projected zombie-arm branch applies.
        let walking = [instances[0]
            .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.0, 1.0))
            .with(|s| s.age_in_ticks = 8.0)];
        let textured_walk_meshes = entity_model_textured_meshes(&walking, &atlas);
        assert_zombie_family_textured_output(&textured_walk_meshes, walking[0]);
        let textured_walk = &textured_walk_meshes.cutout;
        assert_ne!(
            textured.vertices, textured_walk.vertices,
            "legs swing (baby={baby})"
        );
    }
}

fn husk_texture_images() -> Vec<EntityModelTextureImage> {
    [HUSK_TEXTURE_REF, HUSK_BABY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn drowned_textured_layer_passes_match_vanilla_renderer() {
    // Adult: base body (`drowned.png`) + the always-on white `DrownedOuterLayer` overlay
    // (`drowned_outer_layer.png`). Baby: base body + the distinct baby outer layer.
    let adult = drowned_textured_layer_passes(false);
    assert_eq!(adult.len(), 2);
    assert_eq!(
        adult[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[0].texture, DROWNED_TEXTURE_REF);
    assert_eq!(adult[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult[0].order, 0);

    assert_eq!(
        adult[1].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(adult[1].texture, DROWNED_OUTER_LAYER_TEXTURE_REF);
    assert_eq!(adult[1].visibility, EntityModelLayerVisibility::All);
    assert_eq!(adult[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(adult[1].order, 1);

    let baby = drowned_textured_layer_passes(true);
    assert_eq!(baby.len(), 2);
    assert_eq!(baby[0].texture, DROWNED_BABY_TEXTURE_REF);
    assert_eq!(baby[1].texture, DROWNED_OUTER_LAYER_BABY_TEXTURE_REF);
    assert_eq!(baby[1].order, 1);

    assert!(entity_model_texture_refs().contains(&DROWNED_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&DROWNED_BABY_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&DROWNED_OUTER_LAYER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&DROWNED_OUTER_LAYER_BABY_TEXTURE_REF));
}

#[test]
fn drowned_textured_mesh_geometry_and_legs_swing() {
    let (atlas, _) = build_entity_model_texture_atlas(&drowned_texture_images()).unwrap();
    for baby in [false, true] {
        let instances = [zombie_submission_probe(
            EntityModelInstance::zombie_variant(
                57,
                [0.0, 64.0, 0.0],
                0.0,
                ZombieVariantModelFamily::Drowned,
                baby,
            ),
        )];
        let textured_meshes = entity_model_textured_meshes(&instances, &atlas);
        assert_zombie_family_textured_output(&textured_meshes, instances[0]);
        let textured = &textured_meshes.cutout;
        assert!(textured
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

        // Vanilla runs the leg swing every frame; advancing the walk animation re-poses the legs
        // (the held-out arms and the swim override share the same animator).
        let walking = [instances[0]
            .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.0, 1.0))
            .with(|s| s.age_in_ticks = 8.0)];
        let textured_walk_meshes = entity_model_textured_meshes(&walking, &atlas);
        assert_zombie_family_textured_output(&textured_walk_meshes, walking[0]);
        let textured_walk = &textured_walk_meshes.cutout;
        assert_ne!(
            textured.vertices, textured_walk.vertices,
            "legs swing (baby={baby})"
        );
    }
}

#[test]
fn drowned_textured_mesh_applies_swim_amount_pose_and_body_pitch() {
    let (atlas, _) = build_entity_model_texture_atlas(&drowned_texture_images()).unwrap();
    let base = EntityModelInstance::zombie_variant(
        59,
        [0.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Drowned,
        false,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (0.0, 25.0))
    .with(|s| {
        s.age_in_ticks = 10.0;
        s.bounding_box_height = 1.95;
    });
    let dry_meshes = entity_model_textured_meshes(&[base], &atlas);
    assert_zombie_family_textured_output(&dry_meshes, base);
    let dry = &dry_meshes.cutout;
    let swimming_instance = base.with(|s| s.swim_amount = 0.5);
    let swimming_meshes = entity_model_textured_meshes(&[swimming_instance], &atlas);
    assert_zombie_family_textured_output(&swimming_meshes, swimming_instance);
    let swimming = &swimming_meshes.cutout;
    assert_eq!(dry.cutout_faces, swimming.cutout_faces);
    assert_eq!(dry.vertices.len(), swimming.vertices.len());
    assert_ne!(
        dry.vertices, swimming.vertices,
        "swimAmount re-poses the textured drowned"
    );
    assert!(swimming
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    let pivoted_meshes = entity_model_textured_meshes(&[swimming_instance], &atlas);
    assert_zombie_family_textured_output(&pivoted_meshes, swimming_instance);
    let pivoted = &pivoted_meshes.cutout;
    let foot_pivot_instance = swimming_instance.with(|s| s.bounding_box_height = 0.0);
    let foot_pivot_meshes = entity_model_textured_meshes(&[foot_pivot_instance], &atlas);
    assert_zombie_family_textured_output(&foot_pivot_meshes, foot_pivot_instance);
    let foot_pivot = &foot_pivot_meshes.cutout;
    assert_ne!(
        pivoted.vertices, foot_pivot.vertices,
        "DrownedRenderer.setupRotations uses boundingBoxHeight / 2 as the swim pitch pivot"
    );
}

fn drowned_texture_images() -> Vec<EntityModelTextureImage> {
    [DROWNED_TEXTURE_REF, DROWNED_BABY_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn drowned_outer_layer_overlays_the_inflated_white_shell() {
    // Vanilla `DrownedOuterLayer`: an always-on white cutout copy of the inflated
    // `DrownedModel.createBodyLayer(0.25)` shell (`drowned_outer_layer.png`), posed by the same
    // animator as the base so it tracks the limbs. The seven inflated boxes add 42 cutout faces.
    let mut images = drowned_texture_images();
    for outer in [
        DROWNED_OUTER_LAYER_TEXTURE_REF,
        DROWNED_OUTER_LAYER_BABY_TEXTURE_REF,
    ] {
        let len = usize::try_from(outer.size[0] * outer.size[1] * 4).unwrap();
        images.push(EntityModelTextureImage::new(outer, vec![200u8; len]));
    }
    let (with_outer, _) = build_entity_model_texture_atlas(&images).unwrap();
    let (base_only, _) = build_entity_model_texture_atlas(&drowned_texture_images()).unwrap();

    let instances = [zombie_submission_probe(
        EntityModelInstance::zombie_variant(
            57,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Drowned,
            false,
        ),
    )];
    let base_meshes = entity_model_textured_meshes(&instances, &base_only);
    assert_zombie_family_textured_output(&base_meshes, instances[0]);
    let base = &base_meshes.cutout;
    let outer_meshes = entity_model_textured_meshes(&instances, &with_outer);
    assert_zombie_family_textured_output(&outer_meshes, instances[0]);
    let outer = &outer_meshes.cutout;
    let body_overlay = instances[0].render_state.overlay_coords();
    let layer_overlay = [0.0, body_overlay[1]];
    assert_ne!(body_overlay, layer_overlay);

    assert!(base.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == instances[0].render_state.shader_light()
            && vertex.overlay == body_overlay
    }));
    assert!(base
        .vertices
        .iter()
        .all(|vertex| vertex.overlay != layer_overlay));
    assert!(outer
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == body_overlay));
    assert!(outer
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == layer_overlay));

    // The outer adds exactly the inflated humanoid shell (7 boxes * 6 faces).
    assert_eq!(outer.cutout_faces, base.cutout_faces + 42);
    assert!(outer.vertices.len() > base.vertices.len());
    // The white outer copy keeps the [1, 1, 1, 1] tint.
    assert!(outer
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    // The 0.25 inflation makes the shell strictly larger than the base body in every axis (a
    // rotation-independent claim: every inflated cube grows by 0.5 about its own centre).
    let (bmin, bmax) = textured_mesh_extents(base);
    let (omin, omax) = textured_mesh_extents(outer);
    for axis in 0..3 {
        assert!(
            omax[axis] - omin[axis] > bmax[axis] - bmin[axis],
            "outer shell wider on axis {axis}"
        );
    }

    // The overlay is posed by the same animator, so advancing the walk re-poses it too.
    let walking = [instances[0]
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.0, 1.0))
        .with(|s| s.age_in_ticks = 8.0)];
    let outer_walk_meshes = entity_model_textured_meshes(&walking, &with_outer);
    assert_zombie_family_textured_output(&outer_walk_meshes, walking[0]);
    let outer_walk = &outer_walk_meshes.cutout;
    assert_ne!(
        outer.vertices, outer_walk.vertices,
        "outer shell tracks the limbs"
    );

    // The baby drowned carries its own distinct outer shell (`BabyDrownedModel.createBodyLayer(0.25)`),
    // which adds the same seven inflated boxes (42 faces) over the baby body and tracks it past the
    // base baby body in every direction.
    let baby = [zombie_submission_probe(
        EntityModelInstance::zombie_variant(
            58,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::Drowned,
            true,
        ),
    )];
    let baby_base_meshes = entity_model_textured_meshes(&baby, &base_only);
    assert_zombie_family_textured_output(&baby_base_meshes, baby[0]);
    let baby_base = &baby_base_meshes.cutout;
    let baby_outer_meshes = entity_model_textured_meshes(&baby, &with_outer);
    assert_zombie_family_textured_output(&baby_outer_meshes, baby[0]);
    let baby_outer = &baby_outer_meshes.cutout;
    let baby_body_overlay = baby[0].render_state.overlay_coords();
    let baby_layer_overlay = [0.0, baby_body_overlay[1]];
    assert_ne!(baby_body_overlay, baby_layer_overlay);

    assert!(baby_base.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == baby[0].render_state.shader_light()
            && vertex.overlay == baby_body_overlay
    }));
    assert!(baby_base
        .vertices
        .iter()
        .all(|vertex| vertex.overlay != baby_layer_overlay));
    assert!(baby_outer
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == baby_body_overlay));
    assert!(baby_outer
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == baby_layer_overlay));
    assert_eq!(baby_outer.cutout_faces, baby_base.cutout_faces + 42);
    assert!(baby_outer
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (bbmin, bbmax) = textured_mesh_extents(baby_base);
    let (bomin, bomax) = textured_mesh_extents(baby_outer);
    for axis in 0..3 {
        assert!(
            bomax[axis] - bomin[axis] > bbmax[axis] - bbmin[axis],
            "baby outer shell wider on axis {axis}"
        );
    }
}

#[test]
fn zombie_villager_textured_layer_passes_match_vanilla_renderer() {
    for (baby, texture) in [
        (false, ZOMBIE_VILLAGER_TEXTURE_REF),
        (true, ZOMBIE_VILLAGER_BABY_TEXTURE_REF),
    ] {
        let passes = zombie_villager_textured_layer_passes(baby);
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    }
    let farmer =
        VillagerModelData::new(VillagerModelType::Snow, VillagerModelProfession::Farmer, 4);
    let overlays = zombie_villager_data_textured_layer_passes(false, farmer);
    assert_eq!(overlays.len(), 3);
    assert_eq!(overlays[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(
        overlays[0].texture,
        zombie_villager_type_texture_ref(VillagerModelType::Snow, false)
    );
    assert_eq!(overlays[0].order, 1);
    assert_eq!(
        overlays[1].texture,
        zombie_villager_profession_texture_ref(VillagerModelProfession::Farmer).unwrap()
    );
    assert_eq!(overlays[1].order, 2);
    assert_eq!(overlays[2].texture, zombie_villager_level_texture_ref(4));
    assert_eq!(overlays[2].order, 3);

    let baby_overlays = zombie_villager_data_textured_layer_passes(true, farmer);
    assert_eq!(baby_overlays.len(), 1);
    assert_eq!(
        baby_overlays[0].texture,
        zombie_villager_type_texture_ref(VillagerModelType::Snow, true)
    );
    assert_eq!(baby_overlays[0].order, 1);
    assert!(entity_model_texture_refs().contains(&ZOMBIE_VILLAGER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ZOMBIE_VILLAGER_BABY_TEXTURE_REF));
}

#[test]
fn zombie_villager_textured_mesh_geometry_and_legs_swing() {
    let (atlas, _) = build_entity_model_texture_atlas(&zombie_villager_texture_images()).unwrap();
    for baby in [false, true] {
        let instances = [zombie_submission_probe(
            EntityModelInstance::zombie_variant(
                58,
                [0.0, 64.0, 0.0],
                0.0,
                ZombieVariantModelFamily::ZombieVillager,
                baby,
            ),
        )];
        let textured_meshes = entity_model_textured_meshes(&instances, &atlas);
        assert_zombie_family_textured_output(&textured_meshes, instances[0]);
        let textured = &textured_meshes.cutout;
        assert!(textured
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

        // Walking re-poses the legs on both render paths; both share the projected zombie-arm branch.
        let walking = [instances[0]
            .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.0, 1.0))
            .with(|s| s.age_in_ticks = 8.0)];
        let textured_walk_meshes = entity_model_textured_meshes(&walking, &atlas);
        assert_zombie_family_textured_output(&textured_walk_meshes, walking[0]);
        let textured_walk = &textured_walk_meshes.cutout;
        assert_ne!(
            textured.vertices, textured_walk.vertices,
            "legs swing (baby={baby})"
        );
    }
}

#[test]
fn zombie_villager_profession_layers_render_with_zombie_textures_and_no_hat_rule() {
    let textures = [
        ZOMBIE_VILLAGER_TEXTURE_REF,
        ZOMBIE_VILLAGER_TYPE_TEXTURE_REFS[4],
        ZOMBIE_VILLAGER_PROFESSION_TEXTURE_REFS[4],
        ZOMBIE_VILLAGER_LEVEL_TEXTURE_REFS[3],
    ];
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&textures)).unwrap();
    let instance = zombie_submission_probe(
        EntityModelInstance::zombie_variant(
            404,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::ZombieVillager,
            false,
        )
        .with(|s| {
            s.kind.villager_mut().model_data =
                VillagerModelData::new(VillagerModelType::Snow, VillagerModelProfession::Farmer, 4)
        }),
    );
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    let mesh = &meshes.cutout;

    // Base 60 faces + type layer without the head subtree 36 + profession 60 + level 60. The type
    // layer starts at the right arm (`texOffs(44, 22)`), the others at the head.
    assert_eq!(mesh.cutout_faces, 216);
    assert_eq!(mesh.vertices.len(), 864);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert_close2(
        mesh.vertices[240].uv,
        [52.0 / 64.0, 1.0 / 4.0 + 22.0 / 256.0],
    );
    assert_close2(mesh.vertices[384].uv, [16.0 / 64.0, 2.0 / 4.0]);
    assert_close2(mesh.vertices[624].uv, [16.0 / 64.0, 3.0 / 4.0]);

    assert_zombie_family_textured_output(&meshes, instance);
    let body_overlay = instance.render_state.overlay_coords();
    let layer_overlay = [0.0, body_overlay[1]];
    assert_ne!(body_overlay, layer_overlay);
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == body_overlay));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == layer_overlay));
}

#[test]
fn zombie_villager_profession_missing_textures_preserve_other_geometry() {
    // Vanilla `ZombieVillagerRenderer` installs `VillagerProfessionLayer` with the
    // `zombie_villager` texture path, so type/profession/level submits keep the same orders.
    let (atlas, _) = build_entity_model_texture_atlas(&texture_images(&[
        ZOMBIE_VILLAGER_TEXTURE_REF,
        ZOMBIE_VILLAGER_BABY_TEXTURE_REF,
    ]))
    .unwrap();

    let adult = zombie_submission_probe(
        EntityModelInstance::zombie_variant(
            405,
            [0.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::ZombieVillager,
            false,
        )
        .with(|s| {
            s.kind.villager_mut().model_data =
                VillagerModelData::new(VillagerModelType::Snow, VillagerModelProfession::Farmer, 4)
        }),
    );
    let adult_meshes = entity_model_textured_meshes(&[adult], &atlas);
    assert_zombie_family_folded_meshes_are_cutout_only(&adult_meshes);

    assert_eq!(adult_meshes.cutout.vertices.len(), 240);
    let adult_base_overlay = adult.render_state.overlay_coords();
    let adult_layer_overlay = [0.0, adult_base_overlay[1]];

    assert!(adult_meshes.cutout.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == adult.render_state.shader_light()
            && vertex.overlay == adult_base_overlay
    }));
    assert!(adult_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay != adult_layer_overlay));

    let baby = zombie_submission_probe(
        EntityModelInstance::zombie_variant(
            406,
            [2.0, 64.0, 0.0],
            0.0,
            ZombieVariantModelFamily::ZombieVillager,
            true,
        )
        .with(|s| {
            s.kind.villager_mut().model_data =
                VillagerModelData::new(VillagerModelType::Plains, VillagerModelProfession::Mason, 5)
        }),
    );
    let baby_meshes = entity_model_textured_meshes(&[baby], &atlas);
    assert_zombie_family_folded_meshes_are_cutout_only(&baby_meshes);

    assert_eq!(baby_meshes.cutout.vertices.len(), 240);
    let baby_base_overlay = baby.render_state.overlay_coords();
    let baby_layer_overlay = [0.0, baby_base_overlay[1]];

    assert!(baby_meshes.cutout.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == baby.render_state.shader_light()
            && vertex.overlay == baby_base_overlay
    }));
    assert!(baby_meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.overlay != baby_layer_overlay));
}

fn zombie_submission_probe(instance: EntityModelInstance) -> EntityModelInstance {
    instance.with(|s| {
        s.light_coords = (7_u32 << 4) | (9_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    })
}

fn assert_zombie_family_textured_output(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert_zombie_family_folded_meshes_are_cutout_only(meshes);
    let has_zero_white_overlay = match instance.kind {
        EntityModelKind::Zombie { .. }
        | EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            ..
        } => false,
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby,
        } => drowned_textured_layer_passes(baby)
            .iter()
            .any(|pass| matches!(pass.kind, EntityModelLayerKind::ZeroWhiteOverlay)),
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby,
        } => !zombie_villager_data_textured_layer_passes(
            baby,
            instance.render_state.kind.villager().model_data,
        )
        .is_empty(),
        _ => panic!("expected zombie-family instance"),
    };
    let base_overlay = instance.render_state.overlay_coords();
    let zero_white_overlay = [0.0, base_overlay[1]];

    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == instance.render_state.shader_light()));
    if has_zero_white_overlay && zero_white_overlay != base_overlay {
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .any(|vertex| vertex.overlay == base_overlay));
        assert!(meshes.cutout.vertices.iter().all(|vertex| {
            vertex.overlay == base_overlay || vertex.overlay == zero_white_overlay
        }));
    } else {
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .all(|vertex| vertex.overlay == base_overlay));
    }
}

fn assert_zombie_family_folded_meshes_are_cutout_only(meshes: &EntityModelTexturedMeshes) {
    assert!(!meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
}

fn zombie_villager_texture_images() -> Vec<EntityModelTextureImage> {
    texture_images(&[
        ZOMBIE_VILLAGER_TEXTURE_REF,
        ZOMBIE_VILLAGER_BABY_TEXTURE_REF,
    ])
}

fn texture_images(textures: &[EntityModelTextureRef]) -> Vec<EntityModelTextureImage> {
    textures
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
