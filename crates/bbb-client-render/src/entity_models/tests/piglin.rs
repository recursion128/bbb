use super::*;
use crate::entity_models::model_layers::piglin_model;

#[test]
fn zombified_piglin_uses_held_out_zombie_arms() {
    use std::f32::consts::PI;

    let arm_pose = |instance: EntityModelInstance| {
        let mut model = piglin_model(PiglinModelFamily::ZombifiedPiglin, false, &instance);
        (
            model.child_mut("right_arm").pose.rotation,
            model.child_mut("left_arm").pose.rotation,
        )
    };

    // Vanilla `ZombifiedPiglinModel.setupAnim` calls
    // `AnimationUtils.animateZombieArms(leftArm, rightArm, state.isAggressive, state)` after
    // `AbstractPiglinModel.setupAnim`, replacing the inherited arm swing.
    let calm = EntityModelInstance::piglin(
        96,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::ZombifiedPiglin,
        false,
    );
    let (calm_right, calm_left) = arm_pose(calm);
    assert!((calm_right[0] - (-PI / 2.25)).abs() < 1.0e-6);
    assert!((calm_right[1] - -0.1).abs() < 1.0e-6);
    assert!((calm_right[2] - 0.1).abs() < 1.0e-6);
    assert!((calm_left[0] - (-PI / 2.25)).abs() < 1.0e-6);
    assert!((calm_left[1] - 0.1).abs() < 1.0e-6);
    assert!((calm_left[2] - -0.1).abs() < 1.0e-6);

    let aggressive = calm.with(|s| s.is_aggressive = true);
    let (aggressive_right, _) = arm_pose(aggressive);
    assert!(
        (aggressive_right[0] - (-PI / 1.5)).abs() < 1.0e-6,
        "aggressive zombified piglins raise the held-out arms higher"
    );

    let mid = aggressive.with(|s| s.attack_anim = 0.5);
    let (mid_right, _) = arm_pose(mid);
    let attack_y = (0.5_f32 * PI).sin();
    let attack_x = ((1.0 - 0.5 * 0.5) * PI).sin();
    assert!(
        (mid_right[0] - (-PI / 1.5 + attack_y * 1.2 - attack_x * 0.4)).abs() < 1.0e-6,
        "attack swing uses zombie-arm xRot formula: {}",
        mid_right[0]
    );
    assert!((mid_right[1] - -(0.1 - attack_y * 0.6)).abs() < 1.0e-6);
}

#[test]
fn piglin_humanoid_mob_spear_pose_points_arm_along_head_look() {
    let yaw = 20.0_f32;
    let pitch = -10.0_f32;
    let yaw_rad = yaw.to_radians();
    let spear_pitch = (-std::f32::consts::FRAC_PI_2 + pitch.to_radians() + 0.8)
        .to_degrees()
        .clamp(-120.0, 30.0)
        .to_radians();
    let age = 11.0_f32;
    let bob_x = (age * 0.067).sin() * 0.05;
    let bob_z = (age * 0.09).cos() * 0.05 + 0.05;

    // Vanilla `PiglinRenderer` inherits `HumanoidMobRenderer.getArmPose`: when no higher-priority
    // `PiglinArmPose` overrides it, a held spear uses `HumanoidModel.ArmPose.SPEAR`.
    let spear =
        EntityModelInstance::piglin(160, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false)
            .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
            .with(|s| {
                s.age_in_ticks = age;
                s.humanoid_mob_main_hand_spear_pose = true;
            });
    let mut model = piglin_model(PiglinModelFamily::Piglin, false, &spear);
    let right = model.child_mut("right_arm").pose;
    assert!((right.rotation[0] - (spear_pitch + bob_x)).abs() < 1.0e-6);
    assert!((right.rotation[1] - (-0.1 + yaw_rad)).abs() < 1.0e-6);
    assert!(
        (right.rotation[2] - bob_z).abs() < 1.0e-6,
        "the inherited idle bob remains on top of the spear pose"
    );

    // The regular piglin's own arm-pose branches still run after the inherited HumanoidModel pose.
    let holding = spear.with(|s| s.kind.piglin_mut().crossbow_hold = true);
    let mut hold_model = piglin_model(PiglinModelFamily::Piglin, false, &holding);
    let held_right = hold_model.child_mut("right_arm").pose;
    assert!(
        (held_right.rotation[0] - spear_pitch).abs() > 0.5,
        "PiglinArmPose.CROSSBOW_HOLD overwrites the inherited SPEAR arm pose"
    );
}

#[test]
fn zombified_piglin_stab_type_skips_held_out_rewrite() {
    use std::f32::consts::PI;

    let t = 0.1_f32;
    let pitch = (-10.0_f32).to_radians();
    let spear_base_x = (-PI / 2.0 + pitch + 0.8)
        .to_degrees()
        .clamp(-120.0, 30.0)
        .to_radians();
    let progress =
        |time: f32, start: f32, end: f32| ((time - start) / (end - start)).clamp(0.0, 1.0);
    let prepare = -((PI * progress(t, 0.0, 0.05)).cos() - 1.0) / 2.0;
    let attack = progress(t, 0.05, 0.2).powi(2);
    let stab_lunge = (90.0 * prepare - 120.0 * attack).to_radians();
    let age = 11.0_f32;
    let bob_x = (age * 0.067).sin() * 0.05;
    let bob_z = (age * 0.09).cos() * 0.05 + 0.05;

    // `ZombifiedPiglinRenderer` inherits base `HumanoidMobRenderer`, not the zombie renderer's
    // opposite-hand override. A main-hand spear therefore poses the main/right arm, and
    // `AnimationUtils.animateZombieArms` sees STAB and skips the held-out rewrite.
    let spear = EntityModelInstance::piglin(
        161,
        [0.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::ZombifiedPiglin,
        false,
    )
    .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -10.0))
    .with(|s| {
        s.age_in_ticks = age;
        s.attack_anim = t;
        s.main_hand_swing_is_stab = true;
        s.humanoid_mob_main_hand_spear_pose = true;
    });
    let mut model = piglin_model(PiglinModelFamily::ZombifiedPiglin, false, &spear);
    let right = model.child_mut("right_arm").pose;
    assert!((right.rotation[0] - (spear_base_x + stab_lunge + 2.0 * bob_x)).abs() < 1.0e-6);
    assert!((right.rotation[1] - (-0.1)).abs() < 1.0e-6);
    assert!((right.rotation[2] - 2.0 * bob_z).abs() < 1.0e-6);
    let mut held_out = piglin_model(
        PiglinModelFamily::ZombifiedPiglin,
        false,
        &EntityModelInstance::piglin(
            162,
            [0.0, 64.0, 0.0],
            0.0,
            PiglinModelFamily::ZombifiedPiglin,
            false,
        ),
    );
    assert!(
        (right.rotation[0] - held_out.child_mut("right_arm").pose.rotation[0]).abs() > 1.0,
        "STAB swingAnimationType must skip the zombified-piglin held-out arm rewrite"
    );
}

#[test]
fn piglin_ear_flap_pose_matches_vanilla_formula() {
    // Vanilla `AbstractPiglinModel.setupAnim`:
    //   freq = ageInTicks * 0.1 + pos * 0.5;  amp = 0.08 + speed * 0.4;
    //   leftEar.zRot  = -default - cos(freq * 1.2) * amp;
    //   rightEar.zRot =  default + cos(freq)       * amp.
    // The default ear angle is 30° (adult) or 5° (baby), in radians.
    assert!((PIGLIN_ADULT_EAR_ANGLE - std::f32::consts::FRAC_PI_6).abs() < 1e-6);
    assert!((PIGLIN_BABY_EAR_ANGLE - 5.0 * std::f32::consts::PI / 180.0).abs() < 1e-9);

    let default = PIGLIN_ADULT_EAR_ANGLE;
    let left_base = PartPose {
        offset: [4.5, -6.0, 0.0],
        rotation: [0.0, 0.0, -default],
    };
    let right_base = PartPose {
        offset: [-4.5, -6.0, 0.0],
        rotation: [0.0, 0.0, default],
    };

    // Standing (age 0, pos 0, speed 0): freq 0, amp 0.08, so the ears carry the ±0.08
    // baseline flap on top of the default angle.
    let left = piglin_ear_flap_pose(left_base, true, default, 0.0, 0.0, 0.0);
    assert!(
        (left.rotation[2] - (-default - 0.08)).abs() < 1e-6,
        "{}",
        left.rotation[2]
    );
    let right = piglin_ear_flap_pose(right_base, false, default, 0.0, 0.0, 0.0);
    assert!(
        (right.rotation[2] - (default + 0.08)).abs() < 1e-6,
        "{}",
        right.rotation[2]
    );

    // A general (age, pos, speed) reproduces the formula, including the left ear's ×1.2
    // frequency and the speed-scaled amplitude.
    let (age, pos, speed) = (40.0_f32, 1.5_f32, 0.6_f32);
    let freq = age * 0.1 + pos * 0.5;
    let amp = 0.08 + speed * 0.4;
    let left = piglin_ear_flap_pose(left_base, true, default, age, pos, speed);
    assert!((left.rotation[2] - (-default - (freq * 1.2).cos() * amp)).abs() < 1e-6);
    let right = piglin_ear_flap_pose(right_base, false, default, age, pos, speed);
    assert!((right.rotation[2] - (default + freq.cos() * amp)).abs() < 1e-6);

    // The offset and the untouched xRot/yRot are preserved.
    assert_eq!(left.offset, left_base.offset);
    assert_eq!(left.rotation[0], left_base.rotation[0]);
    assert_eq!(left.rotation[1], left_base.rotation[1]);

    // The baby ear uses the 5° default angle.
    let baby_left = piglin_ear_flap_pose(left_base, true, PIGLIN_BABY_EAR_ANGLE, 0.0, 0.0, 0.0);
    assert!((baby_left.rotation[2] - (-PIGLIN_BABY_EAR_ANGLE - 0.08)).abs() < 1e-6);
}

#[test]
fn piglin_textured_layer_passes_match_vanilla_renderer() {
    let cases = [
        (PiglinModelFamily::Piglin, false, PIGLIN_TEXTURE_REF),
        (PiglinModelFamily::Piglin, true, PIGLIN_BABY_TEXTURE_REF),
        (
            PiglinModelFamily::PiglinBrute,
            false,
            PIGLIN_BRUTE_TEXTURE_REF,
        ),
        (
            PiglinModelFamily::ZombifiedPiglin,
            false,
            ZOMBIFIED_PIGLIN_TEXTURE_REF,
        ),
        (
            PiglinModelFamily::ZombifiedPiglin,
            true,
            ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF,
        ),
    ];
    for (family, baby, texture) in cases {
        let baby_layout = baby && family != PiglinModelFamily::PiglinBrute;
        let passes = piglin_textured_layer_passes(family, baby_layout);
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(passes[0].order, 0);
        assert!(entity_model_texture_refs().contains(&texture));
        // The unified `PiglinModel` tree drives the geometry, so the layer-pass parts are vestigial.
    }
    // The brute is never baby: its baby flag still selects the adult layer + brute texture.
    let brute_baby = piglin_textured_layer_passes(PiglinModelFamily::PiglinBrute, false);
    assert_eq!(brute_baby[0].texture, PIGLIN_BRUTE_TEXTURE_REF);
}

#[test]
fn piglin_textured_mesh_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&piglin_texture_images()).unwrap();
    let cases = [
        (PiglinModelFamily::Piglin, false),
        (PiglinModelFamily::Piglin, true),
        (PiglinModelFamily::PiglinBrute, false),
        (PiglinModelFamily::ZombifiedPiglin, false),
        (PiglinModelFamily::ZombifiedPiglin, true),
    ];
    for (family, baby) in cases {
        let instances = [
            EntityModelInstance::piglin(90, [0.0, 64.0, 0.0], 0.0, family, baby).with(|s| {
                s.light_coords = (5_u32 << 4) | (11_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            }),
        ];
        let textured_meshes = entity_model_textured_meshes(&instances, &atlas);
        assert_piglin_textured_output(&textured_meshes, &instances);
        let textured = &textured_meshes.cutout;
        assert!(textured
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

        // The ears flap as ageInTicks advances on the textured path too.
        let aged = [instances[0].with(|s| s.age_in_ticks = 31.4)];
        let textured_aged = entity_model_textured_meshes(&aged, &atlas);
        assert_piglin_textured_output(&textured_aged, &aged);
        assert_ne!(
            textured.vertices, textured_aged.cutout.vertices,
            "{family:?} ears"
        );
    }

    // Non-zombified piglins swing their arms when walking; the zombified piglin holds them out.
    let piglin =
        EntityModelInstance::piglin(90, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false);
    let piglin = piglin.with(|s| {
        s.light_coords = (6_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let piglin_walk_instance =
        piglin.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let piglin_rest = entity_model_textured_meshes(&[piglin], &atlas);
    let piglin_walk = entity_model_textured_meshes(&[piglin_walk_instance], &atlas);
    assert_piglin_textured_output(&piglin_rest, &[piglin]);
    assert_piglin_textured_output(&piglin_walk, &[piglin_walk_instance]);
    assert_ne!(
        piglin_rest.cutout.vertices, piglin_walk.cutout.vertices,
        "the textured piglin swings its arms/legs when walking"
    );
}

fn assert_piglin_textured_output(
    meshes: &EntityModelTexturedMeshes,
    instances: &[EntityModelInstance],
) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());

    if instances.len() == 1 {
        let instance = instances[0];
        assert!(meshes.cutout.vertices.iter().all(|vertex| {
            vertex.light == instance.render_state.shader_light()
                && vertex.overlay == instance.render_state.overlay_coords()
        }));
    }
}

#[test]
fn dancing_piglin_raises_its_arms_and_bobs() {
    use std::f32::consts::PI;

    // Vanilla `PiglinModel.setupAnim` DANCING (`Piglin.isDancing()` → DATA_IS_DANCING): the head/body
    // bob, both arms raise overhead (`rightArm.zRot = (70 + cos(dancePos·40)·10)°`, the left mirrored)
    // and wag with `ageInTicks`, and the ears sway. `dancePos = ageInTicks / 60`. We compare a dancing
    // model to an idle one at the same age so the bob is read as the delta over the shared bind pose.
    let deg = PI / 180.0;
    let age = 48.0_f32;
    let dance_pos = age / 60.0;
    let bob = (dance_pos * 40.0).sin();

    let base =
        EntityModelInstance::piglin(91, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false)
            .with(|s| s.age_in_ticks = age);
    let mut idle = piglin_model(PiglinModelFamily::Piglin, false, &base);
    let mut dancing = piglin_model(
        PiglinModelFamily::Piglin,
        false,
        &base.with(|s| s.kind.piglin_mut().dancing = true),
    );

    // Both arms raise overhead, the left mirroring the right's `zRot`.
    let right_zrot = deg * (70.0 + (dance_pos * 40.0).cos() * 10.0);
    let danced_right = dancing.child_mut("right_arm").pose;
    assert!(
        (danced_right.rotation[2] - right_zrot).abs() < 1.0e-6,
        "the right arm raises to ~80°: {}",
        danced_right.rotation[2]
    );
    let danced_left_zrot = dancing.child_mut("left_arm").pose.rotation[2];
    assert!(
        (danced_left_zrot + right_zrot).abs() < 1.0e-6,
        "the left arm mirrors the right: {danced_left_zrot}"
    );

    // The bob offsets add onto the bind pose (read as the dancing − idle delta).
    let idle_head = idle.child_mut("head").pose.offset;
    let danced_head = dancing.child_mut("head").pose.offset;
    assert!((danced_head[0] - idle_head[0] - (dance_pos * 10.0).sin()).abs() < 1.0e-6);
    assert!((danced_head[1] - idle_head[1] - (bob + 0.4)).abs() < 1.0e-6);

    let idle_ry = idle.child_mut("right_arm").pose.offset[1];
    let danced_ry = dancing.child_mut("right_arm").pose.offset[1];
    assert!((danced_ry - idle_ry - (bob * 0.5 - 0.5)).abs() < 1.0e-6);
    let idle_ly = idle.child_mut("left_arm").pose.offset[1];
    let danced_ly = dancing.child_mut("left_arm").pose.offset[1];
    assert!((danced_ly - idle_ly - (bob * 0.5 + 0.5)).abs() < 1.0e-6);
    let idle_by = idle.child_mut("body").pose.offset[1];
    let danced_by = dancing.child_mut("body").pose.offset[1];
    assert!((danced_by - idle_by - bob * 0.35).abs() < 1.0e-6);

    // The dance overwrites the idle ear flap with its own sway.
    let danced_right_ear = dancing
        .child_mut("head")
        .child_mut("right_ear")
        .pose
        .rotation[2];
    assert!(
        (danced_right_ear - (PI / 6.0 + deg * (dance_pos * 30.0).sin() * 10.0)).abs() < 1.0e-6,
        "the right ear sways with the dance: {danced_right_ear}"
    );

    // The full mesh visibly changes, and advancing the dance keeps animating it.
    let dancing_mesh = entity_model_mesh(&[base.with(|s| s.kind.piglin_mut().dancing = true)]);
    assert_ne!(
        entity_model_mesh(&[base]).vertices,
        dancing_mesh.vertices,
        "a dancing piglin no longer stands idle"
    );
    let later = entity_model_mesh(&[base.with(|s| {
        s.kind.piglin_mut().dancing = true;
        s.age_in_ticks = age + 7.0;
    })]);
    assert_ne!(
        dancing_mesh.vertices, later.vertices,
        "the dance keeps wagging as ageInTicks advances"
    );
}

#[test]
fn piglin_holds_its_crossbow_level_with_the_head_look() {
    use std::f32::consts::FRAC_PI_2;
    // Vanilla `PiglinModel.setupAnim` `CROSSBOW_HOLD` (`AnimationUtils.animateCrossbowHold`,
    // `holdingInRightArm = true`, the same pose as the pillager): a regular piglin holding a charged
    // crossbow levels it along the head look — right (holding) arm `xRot = -π/2 + head.xRot + 0.1`,
    // `yRot = -0.3 + head.yRot`; left (shooting) arm `xRot = -1.5 + head.xRot`, `yRot = 0.6 + head.yRot`.
    let yaw = 25.0_f32;
    let pitch = -15.0_f32;
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();

    let holding =
        EntityModelInstance::piglin(96, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false)
            .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
            .with(|s| s.kind.piglin_mut().crossbow_hold = true);
    let mut model = piglin_model(PiglinModelFamily::Piglin, false, &holding);

    let right = model.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-FRAC_PI_2 + pitch_rad + 0.1)).abs() < 1.0e-6,
        "right (holding) arm levels the crossbow: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-0.3 + yaw_rad)).abs() < 1.0e-6,
        "right arm yaws -0.3 off the head: {}",
        right.rotation[1]
    );
    let left = model.child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-1.5 + pitch_rad)).abs() < 1.0e-6,
        "left (shooting) arm reaches across: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (0.6 + yaw_rad)).abs() < 1.0e-6,
        "left arm yaws 0.6 off the head: {}",
        left.rotation[1]
    );

    // An idle piglin (no crossbow hold) keeps the walk swing — its arms are nowhere near the level pose.
    let idle =
        EntityModelInstance::piglin(96, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false)
            .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch));
    let mut idle_model = piglin_model(PiglinModelFamily::Piglin, false, &idle);
    assert!(
        (idle_model.child_mut("right_arm").pose.rotation[0] - (-FRAC_PI_2 + pitch_rad + 0.1)).abs()
            > 1.0,
        "an idle piglin does not level a crossbow"
    );
}

#[test]
fn piglin_pulls_its_crossbow_back_while_charging() {
    use std::f32::consts::FRAC_PI_2;
    // Vanilla `PiglinModel.setupAnim` `CROSSBOW_CHARGE` (`AnimationUtils.animateCrossbowCharge`,
    // `holdingInRightArm = true`, the same pose as the pillager): the right (holding) arm fixes at
    // `yRot = -0.8`, `xRot = -0.97079635`; the left (pulling) arm lerps `yRot 0.4 → 0.85` and
    // `xRot -0.97079635 → -π/2` over `ticksUsingItem / 25` (clamped). Only the regular piglin draws.
    const HOLD_X_ROT: f32 = -0.97079635;
    let arms = |ticks: f32| {
        let charging = EntityModelInstance::piglin(
            98,
            [0.0, 64.0, 0.0],
            0.0,
            PiglinModelFamily::Piglin,
            false,
        )
        .with(|s| (s.head_yaw, s.head_pitch) = (25.0, -15.0))
        .with(|s| {
            s.kind.piglin_mut().crossbow_charge = true;
            s.crossbow_charge_ticks = ticks;
        });
        let mut model = piglin_model(PiglinModelFamily::Piglin, false, &charging);
        (
            model.child_mut("right_arm").pose,
            model.child_mut("left_arm").pose,
        )
    };

    // At the start of the draw (ticks 0) the left arm sits at the rest end of the lerp.
    let (right0, left0) = arms(0.0);
    assert!(
        (right0.rotation[1] - (-0.8)).abs() < 1.0e-6
            && (right0.rotation[0] - HOLD_X_ROT).abs() < 1.0e-6,
        "the holding (right) arm braces the crossbow regardless of the draw: {right0:?}"
    );
    assert!(
        (left0.rotation[1] - 0.4).abs() < 1.0e-6 && (left0.rotation[0] - HOLD_X_ROT).abs() < 1.0e-6,
        "the pulling (left) arm starts at the rest end of the draw: {left0:?}"
    );

    // At full charge (ticks 25) the left arm reaches the fully-drawn end of the lerp.
    let (_, left_full) = arms(25.0);
    assert!(
        (left_full.rotation[1] - 0.85).abs() < 1.0e-6
            && (left_full.rotation[0] - (-FRAC_PI_2)).abs() < 1.0e-6,
        "the pulling arm reaches full draw at maxChargeDuration: {left_full:?}"
    );

    // Mid-draw the left arm sits strictly between the two ends, and over-charge clamps to full draw.
    let (_, left_mid) = arms(12.5);
    assert!(
        left_mid.rotation[1] > 0.4 && left_mid.rotation[1] < 0.85,
        "mid-draw the pulling arm is between rest and full: {}",
        left_mid.rotation[1]
    );
    let quick_charge =
        EntityModelInstance::piglin(98, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false)
            .with(|s| {
                s.kind.piglin_mut().crossbow_charge = true;
                s.crossbow_charge_ticks = 12.5;
                s.crossbow_charge_duration_ticks = 15.0;
            });
    let mut quick_model = piglin_model(PiglinModelFamily::Piglin, false, &quick_charge);
    let quick_left = quick_model.child_mut("left_arm").pose;
    assert!(
        (quick_left.rotation[1] - (0.4 + (0.85 - 0.4) * (12.5 / 15.0))).abs() < 1.0e-6,
        "a synchronized 15-tick duration advances the piglin arm draw"
    );
    let (_, left_over) = arms(40.0);
    assert!(
        (left_over.rotation[1] - 0.85).abs() < 1.0e-6,
        "an over-charged draw clamps to full: {}",
        left_over.rotation[1]
    );

    // The charge draw is a distinct pose from the level hold (the right arm yaws to -0.8, not -0.3+head).
    let holding =
        EntityModelInstance::piglin(98, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false)
            .with(|s| (s.head_yaw, s.head_pitch) = (25.0, -15.0))
            .with(|s| s.kind.piglin_mut().crossbow_hold = true);
    let mut hold_model = piglin_model(PiglinModelFamily::Piglin, false, &holding);
    let hold_right = hold_model.child_mut("right_arm").pose;
    assert!(
        (hold_right.rotation[1] - right0.rotation[1]).abs() > 0.4,
        "the charge draw repositions the arms differently from the level hold"
    );
}

#[test]
fn piglin_raises_and_swings_its_melee_weapon_when_attacking() {
    // Vanilla `PiglinModel` `ATTACKING_WITH_MELEE_WEAPON`: at rest (`attackTime == 0`) `holdWeaponHigh`
    // raises the main (right) arm overhead (`xRot = -1.8`, overwriting only the pitch); mid-swing
    // (`attackTime > 0`) `AnimationUtils.swingWeaponDown` chops it down. The brute shares the pose.
    for family in [PiglinModelFamily::Piglin, PiglinModelFamily::PiglinBrute] {
        let age = 11.0_f32;
        let held_high =
            EntityModelInstance::piglin(97, [0.0, 64.0, 0.0], 0.0, family, false).with(|s| {
                s.age_in_ticks = age;
                s.kind.piglin_mut().attacking_with_melee = true;
            });
        let mut model = piglin_model(family, false, &held_high);
        let right_x = model.child_mut("right_arm").pose.rotation[0];
        assert!(
            (right_x - (-1.8)).abs() < 1.0e-6,
            "a resting piglin holds the weapon high (right arm xRot = -1.8): {right_x}"
        );

        // Mid-swing the right arm chops down through `swingWeaponDown` (the vindicator-shared math),
        // landing well away from the static -1.8 raise and from the idle walk pose.
        let attack_anim = 0.5_f32;
        let swinging =
            EntityModelInstance::piglin(97, [0.0, 64.0, 0.0], 0.0, family, false).with(|s| {
                s.age_in_ticks = age;
                s.kind.piglin_mut().attacking_with_melee = true;
                s.attack_anim = attack_anim;
            });
        let mut swing_model = piglin_model(family, false, &swinging);
        let attack2 = (attack_anim * std::f32::consts::PI).sin();
        let attack =
            ((1.0 - (1.0 - attack_anim) * (1.0 - attack_anim)) * std::f32::consts::PI).sin();
        let expected_right = -1.8849558
            + (age * 0.09).cos() * 0.15
            + (attack2 * 2.2 - attack * 0.4)
            + (age * 0.067).sin() * 0.1; // swingWeaponDown bob + trailing HumanoidModel bob
        let swing_right = swing_model.child_mut("right_arm").pose.rotation[0];
        assert!(
            (swing_right - expected_right).abs() < 1.0e-5,
            "the swing chops the right arm down: got {swing_right}, expected {expected_right}"
        );

        // The attack visibly changes the mesh versus an idle piglin of the same family/age.
        let idle = EntityModelInstance::piglin(97, [0.0, 64.0, 0.0], 0.0, family, false)
            .with(|s| s.age_in_ticks = age);
        assert_ne!(
            entity_model_mesh(&[idle]).vertices,
            entity_model_mesh(&[swinging]).vertices,
            "an attacking piglin no longer stands idle"
        );
    }
}

#[test]
fn piglin_default_whack_swing_uses_humanoid_attack_animation() {
    use std::f32::consts::PI;

    let attack_anim = 0.5_f32;
    let head_pitch = -20.0_f32;
    let body_yrot = (attack_anim.sqrt() * PI * 2.0).sin() * 0.2;
    let swing = 1.0 - (1.0 - attack_anim).powi(4);
    let raise = (swing * PI).sin();
    let head_term = (attack_anim * PI).sin() * -(head_pitch.to_radians() - 0.7) * 0.75;

    for family in [PiglinModelFamily::Piglin, PiglinModelFamily::PiglinBrute] {
        let default_whack = EntityModelInstance::piglin(171, [0.0, 64.0, 0.0], 0.0, family, false)
            .with(|s| (s.head_yaw, s.head_pitch) = (0.0, head_pitch))
            .with(|s| s.attack_anim = attack_anim);
        let mut model = piglin_model(family, false, &default_whack);
        let body = model.child_mut("body").pose;
        let right = model.child_mut("right_arm").pose;

        assert!(
            (body.rotation[1] - body_yrot).abs() < 1.0e-6,
            "{family:?} default WHACK twists the body"
        );
        assert!((right.offset[0] - (-body_yrot.cos() * 5.0)).abs() < 1.0e-6);
        assert!((right.offset[2] - (body_yrot.sin() * 5.0)).abs() < 1.0e-6);
        assert!((right.rotation[0] - (-(raise * 1.2 + head_term))).abs() < 1.0e-6);
        assert!((right.rotation[1] - body_yrot * 3.0).abs() < 1.0e-6);
        assert!((right.rotation[2] - (0.1 - 0.4)).abs() < 1.0e-6);

        let age_scaled_whack = default_whack.with(|s| s.age_scale = 0.5);
        let mut age_scaled_model = piglin_model(family, false, &age_scaled_whack);
        let age_scaled_right = age_scaled_model.child_mut("right_arm").pose;
        assert!((age_scaled_right.offset[0] - (-body_yrot.cos() * 5.0 * 0.5)).abs() < 1.0e-6);
        assert!((age_scaled_right.offset[2] - (body_yrot.sin() * 5.0 * 0.5)).abs() < 1.0e-6);

        let melee = default_whack.with(|s| s.kind.piglin_mut().attacking_with_melee = true);
        let mut melee_model = piglin_model(family, false, &melee);
        assert!(
            melee_model
                .child_mut("body")
                .pose
                .rotation[1]
                .abs()
                < 1.0e-6,
            "ATTACKING_WITH_MELEE_WEAPON uses swingWeaponDown instead of default HumanoidModel WHACK"
        );
    }
}

#[test]
fn piglin_default_none_and_stab_swing_use_humanoid_swing_type() {
    use std::f32::consts::PI;

    let attack_anim = 0.1_f32;
    let body_yrot = (attack_anim.sqrt() * PI * 2.0).sin() * 0.2;
    let progress = |t: f32, a: f32, b: f32| ((t - a) / (b - a)).clamp(0.0, 1.0);
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
    let stab = {
        let prepare = in_out_sine(progress(attack_anim, 0.0, 0.05));
        let attack = progress(attack_anim, 0.05, 0.2).powi(2);
        let retract = in_out_expo(progress(attack_anim, 0.4, 1.0));
        (90.0 * prepare - 120.0 * attack + 30.0 * retract).to_radians()
    };

    for family in [PiglinModelFamily::Piglin, PiglinModelFamily::PiglinBrute] {
        let base = EntityModelInstance::piglin(172, [0.0, 64.0, 0.0], 0.0, family, false);
        let mut idle = piglin_model(family, false, &base);
        let idle_right = idle.child_mut("right_arm").pose;
        let mut none = piglin_model(
            family,
            false,
            &base.with(|s| {
                s.attack_anim = attack_anim;
                s.main_hand_swing_is_none = true;
            }),
        );
        let none_body = none.child_mut("body").pose;
        let none_right = none.child_mut("right_arm").pose;
        assert!((none_body.rotation[1] - body_yrot).abs() < 1.0e-6);
        assert!((none_right.offset[0] - (-body_yrot.cos() * 5.0)).abs() < 1.0e-6);
        assert!((none_right.offset[2] - body_yrot.sin() * 5.0).abs() < 1.0e-6);
        assert!((none_right.rotation[0] - idle_right.rotation[0]).abs() < 1.0e-6);
        let mut stab_model = piglin_model(
            family,
            false,
            &base.with(|s| {
                s.attack_anim = attack_anim;
                s.main_hand_swing_is_stab = true;
            }),
        );
        let stab_right = stab_model.child_mut("right_arm").pose;
        assert!((stab_right.offset[0] - (-body_yrot.cos() * 5.0)).abs() < 1.0e-6);
        assert!((stab_right.rotation[0] - (idle_right.rotation[0] + stab)).abs() < 1.0e-6);
        let mut whack = piglin_model(family, false, &base.with(|s| s.attack_anim = attack_anim));
        assert!(
            (whack.child_mut("right_arm").pose.rotation[0] - stab_right.rotation[0]).abs() > 0.3,
            "{family:?} STAB should not collapse to the default WHACK branch"
        );
    }
}

#[test]
fn piglin_admires_a_loved_offhand_item() {
    // Vanilla `PiglinModel.setupAnim` ADMIRING_ITEM (mainArm = RIGHT): head tilts down to the item
    // (`head.xRot = 0.5, head.yRot = 0`, overwriting the head look) and the off (left) arm lifts it
    // (`leftArm.xRot = -0.9, leftArm.yRot = 0.5`). The right arm keeps its walk swing.
    let admiring =
        EntityModelInstance::piglin(98, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false)
            .with(|s| (s.head_yaw, s.head_pitch) = (30.0, -20.0))
            .with(|s| s.kind.piglin_mut().admiring = true);
    let mut model = piglin_model(PiglinModelFamily::Piglin, false, &admiring);

    let head = model.child_mut("head").pose;
    assert!(
        (head.rotation[0] - 0.5).abs() < 1.0e-6 && head.rotation[1].abs() < 1.0e-6,
        "the head tilts down to the item, overwriting the head look: {:?}",
        head.rotation
    );
    let left = model.child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-0.9)).abs() < 1.0e-6 && (left.rotation[1] - 0.5).abs() < 1.0e-6,
        "the left arm lifts the admired item: {:?}",
        left.rotation
    );

    // An idle piglin keeps its head look (not the fixed admire tilt) — head yaw tracks the look.
    let idle =
        EntityModelInstance::piglin(98, [0.0, 64.0, 0.0], 0.0, PiglinModelFamily::Piglin, false)
            .with(|s| (s.head_yaw, s.head_pitch) = (30.0, -20.0));
    let mut idle_model = piglin_model(PiglinModelFamily::Piglin, false, &idle);
    assert!(
        idle_model.child_mut("head").pose.rotation[1].abs() > 1.0e-3,
        "an idle piglin's head still yaws with the look, not the fixed admire pose"
    );
}

fn piglin_texture_images() -> Vec<EntityModelTextureImage> {
    [
        PIGLIN_TEXTURE_REF,
        PIGLIN_BABY_TEXTURE_REF,
        PIGLIN_BRUTE_TEXTURE_REF,
        ZOMBIFIED_PIGLIN_TEXTURE_REF,
        ZOMBIFIED_PIGLIN_BABY_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}
