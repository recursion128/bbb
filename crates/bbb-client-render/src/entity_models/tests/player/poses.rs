use super::*;
use crate::entity_models::model_layers::player_model;

#[test]
fn player_flight_swim_root_transform_matches_avatar_setup_rotation_order() {
    let base = EntityModelInstance::player(980, [0.0; 3], 0.0, false);
    let flip = Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));
    let avatar_scale = Mat4::from_scale(Vec3::splat(0.9375));
    let model_offset = Mat4::from_translation(Vec3::new(0.0, -1.501, 0.0));
    let initial_yaw = Mat4::from_rotation_y(std::f32::consts::PI);

    let flying = base
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, 10.0))
        .with(|s| {
            s.is_fall_flying = true;
            s.kind.player_mut().fall_flying_time_in_ticks = 5.0;
            s.kind.player_mut().should_apply_flying_y_rot = true;
            s.kind.player_mut().flying_y_rot = 0.3;
        });
    let expected_flying = initial_yaw
        * Mat4::from_rotation_x((-25.0_f32).to_radians())
        * Mat4::from_rotation_y(0.3)
        * flip
        * avatar_scale
        * model_offset;
    assert_player_mat4_close(player_model_root_transform(&flying), expected_flying);

    let swimming = base
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, 20.0))
        .with(|s| {
            s.swim_amount = 0.5;
            s.in_water = true;
            s.is_visually_swimming = true;
        });
    let expected_swimming = initial_yaw
        * Mat4::from_rotation_x((-55.0_f32).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, -1.0, 0.3))
        * flip
        * avatar_scale
        * model_offset;
    assert_player_mat4_close(player_model_root_transform(&swimming), expected_swimming);

    let spinning = base.with(|s| s.auto_spin_age_ticks = Some(4.0));
    let expected_spinning_flight = initial_yaw
        * Mat4::from_rotation_x((-90.0_f32).to_radians())
        * Mat4::from_rotation_y((-300.0_f32).to_radians())
        * Mat4::from_rotation_y(1.0)
        * flip
        * avatar_scale
        * model_offset;
    assert_player_mat4_close(
        player_model_root_transform(&spinning.with(|s| {
            s.is_fall_flying = true;
            s.kind.player_mut().fall_flying_time_in_ticks = 20.0;
            s.kind.player_mut().should_apply_flying_y_rot = true;
            s.kind.player_mut().flying_y_rot = 1.0;
        })),
        expected_spinning_flight,
    );
}

#[test]
fn player_flight_swim_head_and_speed_value_match_humanoid_setup_anim() {
    let base = EntityModelInstance::player(981, [0.0; 3], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, 30.0))
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (1.0, 0.8));
    let mut ordinary = player_model(false, &base);
    let ordinary_right_arm = ordinary.child_mut("right_arm").pose.rotation[0];
    let ordinary_right_leg = ordinary.child_mut("right_leg").pose.rotation[0];
    let mut flying = player_model(
        false,
        &base.with(|s| {
            s.is_fall_flying = true;
            s.humanoid_speed_value = 8.0;
        }),
    );
    assert!(
        (flying.child_mut("head").pose.rotation[0] + std::f32::consts::FRAC_PI_4).abs() < 1.0e-6
    );
    assert!(
        (flying.child_mut("right_arm").pose.rotation[0] - ordinary_right_arm / 8.0).abs() < 1.0e-6
    );
    assert!(
        (flying.child_mut("right_leg").pose.rotation[0] - ordinary_right_leg / 8.0).abs() < 1.0e-6
    );
    let mut swimming = player_model(
        false,
        &base.with(|s| {
            s.swim_amount = 0.5;
            s.is_using_item = true;
        }),
    );
    assert!((swimming.child_mut("head").pose.rotation[0] - (-7.5_f32).to_radians()).abs() < 1.0e-6);
}

#[test]
fn player_flight_swim_arm_cycle_covers_all_three_vanilla_phases() {
    let base = EntityModelInstance::player(982, [0.0; 3], 0.0, false).with(|s| s.swim_amount = 1.0);
    let cases = [
        (
            7.0,
            [
                0.0,
                -std::f32::consts::PI,
                -std::f32::consts::PI
                    + 1.870_796_4 * (-65.0 * 7.0 + 7.0 * 7.0) / (-65.0 * 14.0 + 14.0 * 14.0),
            ],
            [
                0.0,
                std::f32::consts::PI,
                std::f32::consts::PI
                    - 1.870_796_4 * (-65.0 * 7.0 + 7.0 * 7.0) / (-65.0 * 14.0 + 14.0 * 14.0),
            ],
        ),
        (
            18.0,
            [
                std::f32::consts::FRAC_PI_4,
                -std::f32::consts::PI,
                5.012_389 - 1.870_796_4 * 0.5 - std::f32::consts::TAU,
            ],
            [
                std::f32::consts::FRAC_PI_4,
                std::f32::consts::PI,
                2.206_194_4,
            ],
        ),
        (
            24.0,
            [
                std::f32::consts::FRAC_PI_4,
                -std::f32::consts::PI,
                -std::f32::consts::PI,
            ],
            [
                std::f32::consts::FRAC_PI_4,
                std::f32::consts::PI,
                std::f32::consts::PI,
            ],
        ),
    ];
    for (animation_position, expected_left, expected_right) in cases {
        let mut model = player_model(
            false,
            &base.with(|s| {
                (s.walk_animation_pos, s.walk_animation_speed) = (animation_position, 0.0)
            }),
        );
        let left = model.child_mut("left_arm").pose.rotation;
        let right = model.child_mut("right_arm").pose.rotation;
        for axis in 0..3 {
            assert!(
                (left[axis] - expected_left[axis]).abs() < 1.0e-5,
                "phase {animation_position}, left axis {axis}: {} vs {}",
                left[axis],
                expected_left[axis]
            );
            assert!(
                (right[axis] - expected_right[axis]).abs() < 1.0e-5,
                "phase {animation_position}, right axis {axis}: {} vs {}",
                right[axis],
                expected_right[axis]
            );
        }
    }
}

#[test]
fn player_flight_swim_suppresses_using_and_attacking_arms_but_keeps_leg_kick() {
    let base = EntityModelInstance::player(983, [0.0; 3], 0.0, false)
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (18.0, 0.0))
        .with(|s| {
            s.swim_amount = 1.0;
            s.is_fall_flying = true;
            s.attack_anim = 0.4;
        });
    let mut attack = player_model(false, &base);
    let attack_right = attack.child_mut("right_arm").pose.rotation;
    let attack_left = attack.child_mut("left_arm").pose.rotation;
    let mut using = player_model(false, &base.with(|s| s.is_using_item = true));
    let using_right = using.child_mut("right_arm").pose.rotation;
    let using_left = using.child_mut("left_arm").pose.rotation;
    assert_eq!(
        attack_right, using_right,
        "the attacking right arm skips swim"
    );
    assert_ne!(attack_left, using_left, "the free left arm still swims");
    let mut dry = player_model(
        false,
        &base.with(|s| {
            s.swim_amount = 0.0;
            s.is_using_item = true;
        }),
    );
    assert_eq!(using_right, dry.child_mut("right_arm").pose.rotation);
    assert_eq!(using_left, dry.child_mut("left_arm").pose.rotation);
    assert_ne!(
        using.child_mut("right_leg").pose.rotation[0],
        dry.child_mut("right_leg").pose.rotation[0],
        "using an item suppresses only the arms; the swim leg kick remains"
    );
}

#[test]
fn player_textured_mesh_applies_vanilla_model_part_visibility_to_overlay_parts() {
    let (atlas, _) = build_entity_model_texture_atlas(&steve_player_texture_images()).unwrap();
    let hidden_instance = EntityModelInstance::player_with_parts(
        903,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        PLAYER_MODEL_PARTS_ALL_HIDDEN,
    );
    let hidden_meshes = entity_model_textured_meshes(&[hidden_instance], &atlas);
    assert_player_vertices_match_vanilla(&hidden_meshes, hidden_instance);
    let hidden = &hidden_meshes.cutout;
    assert_eq!(hidden.cutout_faces, 36);
    assert_eq!(hidden.vertices.len(), 144);
    assert_eq!(hidden.indices.len(), 216);

    let partial_parts = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::HAT_MASK | PlayerModelPartVisibility::RIGHT_SLEEVE_MASK,
    );
    let partial_instance =
        EntityModelInstance::player_with_parts(904, [0.0, 64.0, 0.0], 0.0, true, partial_parts);
    let partial_meshes = entity_model_textured_meshes(&[partial_instance], &atlas);
    assert_player_vertices_match_vanilla(&partial_meshes, partial_instance);
    let partial = &partial_meshes.cutout;
    assert_eq!(partial.cutout_faces, 48);
    assert_eq!(partial.vertices.len(), 192);
    assert_eq!(partial.indices.len(), 288);
    assert!(partial
        .vertices
        .iter()
        .any(|vertex| vertex.uv[1] >= 32.0 / 64.0));
}

#[test]
fn player_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(903, [0.0, 64.0, 0.0], 0.0, slim);
        let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
        let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
        let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
        let yawed_meshes = entity_model_textured_meshes(&[yawed_instance], &atlas);
        let pitched_meshes = entity_model_textured_meshes(&[pitched_instance], &atlas);
        assert_player_vertices_match_vanilla(&resting_meshes, base);
        assert_player_vertices_match_vanilla(&yawed_meshes, yawed_instance);
        assert_player_vertices_match_vanilla(&pitched_meshes, pitched_instance);
        let resting = &resting_meshes.cutout;
        let yawed = &yawed_meshes.cutout;
        let pitched = &pitched_meshes.cutout;

        // Head look turns the head part (index 0, shared across all passes)
        // without changing the vertex count.
        assert_eq!(resting.vertices.len(), yawed.vertices.len());
        assert_ne!(resting.vertices, yawed.vertices, "slim={slim}");
        assert_ne!(yawed.vertices, pitched.vertices, "slim={slim}");
    }
}

#[test]
fn player_attack_swing_twists_body_and_whacks_the_swinging_arm() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.setupAttackAnimation` (WHACK), at attackTime `t` with head pitch `p`:
    // the body twists `yRot = sin(sqrt(t) · 2π) · 0.2`, the arm anchors swing around it, and the
    // attacking arm whacks down. `setupAttackAnimation` runs last, so it accumulates onto the idle pose.
    let t = 0.4_f32;
    let pitch = -10.0_f32;
    let body_yrot = (t.sqrt() * PI * 2.0).sin() * 0.2;
    let base = EntityModelInstance::player(900, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, pitch));

    // Baseline: a non-swinging player keeps the body untwisted and the arms on the idle pose.
    let mut resting = player_model(false, &base);
    let resting_right_xrot = resting.child_mut("right_arm").pose.rotation[0];
    let resting_left_xrot = resting.child_mut("left_arm").pose.rotation[0];
    assert_eq!(resting.child_mut("body").pose.rotation[1], 0.0);

    // Main-hand swing twists the body and whacks the RIGHT arm.
    let mut main = player_model(false, &base.with(|s| s.attack_anim = t));
    assert!(
        (main.child_mut("body").pose.rotation[1] - body_yrot).abs() < 1e-6,
        "the body twists with the swing"
    );
    let right = main.child_mut("right_arm");
    // The arm anchor swings around the twisting body (vanilla `rightArm.x/z`), overwriting the bind.
    assert!((right.pose.offset[0] - (-body_yrot.cos() * 5.0)).abs() < 1e-6);
    assert!((right.pose.offset[2] - body_yrot.sin() * 5.0).abs() < 1e-6);
    // The whack drives the right arm well forward of its idle pitch.
    assert!(
        right.pose.rotation[0] < resting_right_xrot - 0.8,
        "the main hand whacks down: {} vs resting {resting_right_xrot}",
        right.pose.rotation[0]
    );
    // The off (left) arm is not whacked — only the shared body twist adds to its xRot.
    let main_left_xrot = main.child_mut("left_arm").pose.rotation[0];
    assert!(
        (main_left_xrot - (resting_left_xrot + body_yrot)).abs() < 1e-6,
        "the idle off arm only picks up the body twist on xRot"
    );

    // Off-hand swing negates the body twist and whacks the LEFT arm instead.
    let mut off = player_model(
        false,
        &base.with(|s| {
            s.attack_anim = t;
            s.attack_arm_off_hand = true;
        }),
    );
    assert!(
        (off.child_mut("body").pose.rotation[1] - (-body_yrot)).abs() < 1e-6,
        "the off-hand swing negates the body twist"
    );
    assert!(
        off.child_mut("left_arm").pose.rotation[0] < resting_left_xrot - 0.8,
        "the off hand whacks the left arm"
    );
    // The idle right arm in an off-hand swing is not whacked on its pitch (only yRot picks up the twist).
    assert!(
        (off.child_mut("right_arm").pose.rotation[0] - resting_right_xrot).abs() < 1e-6,
        "the idle main arm keeps its pitch during an off-hand swing"
    );
}

#[test]
fn player_none_swing_keeps_attack_prologue_without_whack() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.setupAttackAnimation` still runs its shared body/arm-anchor prologue for
    // `SwingAnimationType.NONE`, then breaks before the WHACK arm chop.
    let t = 0.4_f32;
    let body_yrot = (t.sqrt() * PI * 2.0).sin() * 0.2;
    let base = EntityModelInstance::player(904, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -10.0));
    let mut resting = player_model(false, &base);
    let resting_right = resting.child_mut("right_arm").pose;
    let resting_left = resting.child_mut("left_arm").pose;
    let mut none = player_model(
        false,
        &base.with(|s| {
            s.attack_anim = t;
            s.main_hand_swing_is_none = true;
        }),
    );
    let body = none.child_mut("body").pose;
    let right = none.child_mut("right_arm").pose;
    let left = none.child_mut("left_arm").pose;

    assert!((body.rotation[1] - body_yrot).abs() < 1.0e-6);
    assert!((right.offset[0] - (-body_yrot.cos() * 5.0)).abs() < 1.0e-6);
    assert!((right.offset[2] - body_yrot.sin() * 5.0).abs() < 1.0e-6);
    assert!((right.rotation[0] - resting_right.rotation[0]).abs() < 1.0e-6);
    assert!((right.rotation[1] - (resting_right.rotation[1] + body_yrot)).abs() < 1.0e-6);
    assert!((right.rotation[2] - resting_right.rotation[2]).abs() < 1.0e-6);
    assert!((left.rotation[0] - (resting_left.rotation[0] + body_yrot)).abs() < 1.0e-6);
    let mut whack = player_model(false, &base.with(|s| s.attack_anim = t));
    assert!(
        whack.child_mut("right_arm").pose.rotation[0] < right.rotation[0] - 0.8,
        "WHACK should chop the attack arm while NONE keeps the prologue-only pitch"
    );
}

#[test]
fn player_with_a_spear_lunges_instead_of_whacking() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.setupAttackAnimation` `STAB` (`SpearAnimations.thirdPersonAttackHand`): the
    // shared body twist + arm anchors run, but the attacking arm LUNGES on its pitch
    // (`xRot += (90·prepare − 120·attack + 30·retract)·π/180`) instead of whacking, and the body-twist
    // contributions on the arm rotations are undone (the off arm keeps its resting pitch).
    let t = 0.1_f32;
    let body_yrot = (t.sqrt() * PI * 2.0).sin() * 0.2;
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
        let prepare = in_out_sine(progress(t, 0.0, 0.05));
        let attack = progress(t, 0.05, 0.2).powi(2);
        let retract = in_out_expo(progress(t, 0.4, 1.0));
        (90.0 * prepare - 120.0 * attack + 30.0 * retract).to_radians()
    };

    let base = EntityModelInstance::player(905, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -10.0));
    let mut resting = player_model(false, &base);
    let resting_left_xrot = resting.child_mut("left_arm").pose.rotation[0];

    let spear_base = base.with(|s| s.kind.player_mut().main_hand_spear_pose = true);
    let mut spear_held = player_model(false, &spear_base);
    let spear_base_right_xrot = spear_held.child_mut("right_arm").pose.rotation[0];

    // A spear-swing first points the held spear along the head look, twists the body the same way the whack
    // does, then lunges the right arm from that SPEAR base pose.
    let spear = spear_base.with(|s| {
        s.attack_anim = t;
        s.main_hand_swing_is_stab = true;
    });
    let mut model = player_model(false, &spear);
    assert!(
        (model.child_mut("body").pose.rotation[1] - body_yrot).abs() < 1e-6,
        "the body still twists with the swing"
    );
    let right = model.child_mut("right_arm");
    let right_offset0 = right.pose.offset[0];
    let right_xrot = right.pose.rotation[0];
    assert!((right_offset0 - (-body_yrot.cos() * 5.0)).abs() < 1e-6);
    assert!(
        (right_xrot - (spear_base_right_xrot + stab)).abs() < 1e-6,
        "the main arm lunges from the held-spear pose: {right_xrot} vs {}",
        spear_base_right_xrot + stab
    );

    // Unlike the whack, the stab undoes the prologue's body-twist add on the off arm's pitch — it stays
    // at its resting pitch.
    let left_xrot = model.child_mut("left_arm").pose.rotation[0];
    assert!(
        (left_xrot - resting_left_xrot).abs() < 1e-6,
        "the off arm keeps its resting pitch during a stab (no body-twist add)"
    );

    // The same swing time with a non-spear (WHACK) drives a visibly different attacking-arm pitch.
    let mut whack = player_model(false, &base.with(|s| s.attack_anim = t));
    let whack_right_xrot = whack.child_mut("right_arm").pose.rotation[0];
    assert!(
        (whack_right_xrot - right_xrot).abs() > 0.3,
        "the spear lunge differs from the whack chop: stab {right_xrot} vs whack {whack_right_xrot}"
    );
}

#[test]
fn player_holding_a_spear_points_the_arm_along_head_look() {
    // Vanilla `AvatarRenderer.getArmPose` returns `SPEAR` for a held spear even when the player is not using
    // it. `SpearAnimations.thirdPersonHandUse` still applies the head-look base pose, but with
    // `ticksUsingItem <= 0` it skips all kinetic sway.
    let yaw = 25.0_f32;
    let pitch = -15.0_f32;
    let base = EntityModelInstance::player(906, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch));
    let mut idle = player_model(false, &base);
    let idle_right_zrot = idle.child_mut("right_arm").pose.rotation[2];
    let expected_x = (-std::f32::consts::FRAC_PI_2 + pitch.to_radians() + 0.8)
        .to_degrees()
        .clamp(-120.0, 30.0)
        .to_radians();
    let mut main = player_model(
        false,
        &base.with(|s| s.kind.player_mut().main_hand_spear_pose = true),
    );
    let right = main.child_mut("right_arm").pose;
    let expected_right_y = (-0.1 + yaw.to_radians())
        .to_degrees()
        .clamp(-60.0, 60.0)
        .to_radians();
    assert!((right.rotation[0] - expected_x).abs() < 1e-6);
    assert!((right.rotation[1] - expected_right_y).abs() < 1e-6);
    assert_eq!(
        right.rotation[2], idle_right_zrot,
        "held spear keeps only the folded-in idle bob roll, with no kinetic roll sway"
    );
    let mut off = player_model(
        false,
        &base.with(|s| s.kind.player_mut().off_hand_spear_pose = true),
    );
    let left = off.child_mut("left_arm").pose;
    let expected_left_y = (0.1 + yaw.to_radians())
        .to_degrees()
        .clamp(-60.0, 60.0)
        .to_radians();
    assert!((left.rotation[0] - expected_x).abs() < 1e-6);
    assert!((left.rotation[1] - expected_left_y).abs() < 1e-6);
}

#[test]
fn player_using_a_spear_applies_kinetic_hand_sway() {
    // Vanilla `SpearAnimations.thirdPersonHandUse`: the SPEAR arm pose points along the head look first,
    // then kinetic weapon timing adds y/z sway and the raise/lower/back pitch terms from `ticksUsingItem`.
    let kinetic = SpearKineticWeapon {
        delay_ticks: 12.0,
        dismount_duration_ticks: 50.0,
        knockback_duration_ticks: 135.0,
        damage_duration_ticks: 225.0,
        forward_movement: 0.38,
    };
    let ticks = 60.0;
    let yaw = 20.0_f32;
    let pitch = -10.0_f32;
    let base = EntityModelInstance::player(906, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
        .with(|s| s.kind.player_mut().using_spear = Some(kinetic));
    let mut steady = player_model(false, &base);
    let steady_right = steady.child_mut("right_arm").pose;
    let expected_y = (-0.1 + yaw.to_radians())
        .to_degrees()
        .clamp(-60.0, 60.0)
        .to_radians();
    let expected_x = (-std::f32::consts::FRAC_PI_2 + pitch.to_radians() + 0.8)
        .to_degrees()
        .clamp(-120.0, 30.0)
        .to_radians();
    assert!((steady_right.rotation[0] - expected_x).abs() < 1e-6);
    assert!((steady_right.rotation[1] - expected_y).abs() < 1e-6);
    let mut swaying = player_model(false, &base.with(|s| s.crossbow_charge_ticks = ticks));
    let swaying_right = swaying.child_mut("right_arm").pose;
    let params = kinetic.use_params(ticks);
    let x_delta = (-40.0 * params.raise_progress_start
        + 30.0 * params.raise_progress_middle
        + -20.0 * params.raise_progress_end
        + 20.0 * params.lower_progress
        + 10.0 * params.raise_back_progress
        + 0.6 * params.sway_scale_slow * params.sway_intensity)
        .to_radians();
    let y_delta = -params.sway_scale_fast.to_radians() * params.sway_intensity;
    let z_delta = -params.sway_scale_slow.to_radians() * params.sway_intensity * 0.5;
    assert!((swaying_right.rotation[0] - (steady_right.rotation[0] + x_delta)).abs() < 1e-6);
    assert!((swaying_right.rotation[1] - (steady_right.rotation[1] + y_delta)).abs() < 1e-6);
    assert!((swaying_right.rotation[2] - (steady_right.rotation[2] + z_delta)).abs() < 1e-6);
    let mut off = player_model(
        false,
        &base.with(|s| {
            s.use_item_off_hand = true;
            s.crossbow_charge_ticks = ticks;
        }),
    );
    let off_left = off.child_mut("left_arm").pose;
    assert!(
        (off_left.rotation[1] - (0.1 + yaw.to_radians() - y_delta)).abs() < 1e-6,
        "off-hand spear mirrors the kinetic yaw sway"
    );
}

#[test]
fn player_tooting_a_goat_horn_raises_it_to_the_mouth() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `TOOT_HORN`: the holding arm raises the horn —
    // `xRot = clamp(head.xRot, −1.2, 1.2) − 1.4835298`, `yRot = head.yRot ∓ π/6`. Unlike the spyglass it
    // keeps the idle bob, so the arm roll (`zRot`) is left non-zero.
    let yaw = 20.0_f32;
    let pitch = -10.0_f32;
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();
    let base = EntityModelInstance::player(921, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch));
    let mut main = player_model(
        false,
        &base.with(|s| s.kind.player_mut().tooting_horn = true),
    );
    let right = main.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (pitch_rad.clamp(-1.2, 1.2) - 1.4835298)).abs() < 1e-6,
        "the right arm raises the horn to the mouth: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (yaw_rad - PI / 6.0)).abs() < 1e-6,
        "the right arm yaws −π/6 off the head: {}",
        right.rotation[1]
    );
    assert_ne!(
        right.rotation[2], 0.0,
        "the horn arm keeps the idle bob roll (unlike the spyglass)"
    );

    // Off-hand horn raises the LEFT arm with the mirrored yaw.
    let mut off = player_model(
        false,
        &base.with(|s| {
            s.kind.player_mut().tooting_horn = true;
            s.use_item_off_hand = true;
        }),
    );
    let left = off.child_mut("left_arm").pose;
    assert!(
        (left.rotation[1] - (yaw_rad + PI / 6.0)).abs() < 1e-6,
        "the left arm yaws +π/6 off the head"
    );
}

#[test]
fn player_brushing_lowers_the_arm_to_the_block() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `BRUSH`: the holding arm lowers — `xRot =
    // arm.xRot · 0.5 − π/5`, `yRot = 0`. At rest (age 0, not walking) the arm pitch is `0`, so the brush
    // pose lands the arm at exactly `−π/5`.
    let base = EntityModelInstance::player(922, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -10.0));
    let mut main = player_model(false, &base.with(|s| s.kind.player_mut().brushing = true));
    let right = main.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI / 5.0)).abs() < 1e-6,
        "the right arm lowers to −π/5: {}",
        right.rotation[0]
    );
    assert_eq!(right.rotation[1], 0.0, "the brush pose zeroes the arm yaw");

    // A non-brushing player keeps its (much higher) idle arm pitch — the brush visibly lowers the arm.
    let mut idle = player_model(false, &base);
    assert!(
        idle.child_mut("right_arm").pose.rotation[0] > -PI / 5.0 + 0.3,
        "an idle player does not lower the arm to the block"
    );

    // Off-hand brushing lowers the LEFT arm instead.
    let mut off = player_model(
        false,
        &base.with(|s| {
            s.kind.player_mut().brushing = true;
            s.use_item_off_hand = true;
        }),
    );
    assert!(
        (off.child_mut("left_arm").pose.rotation[0] - (-PI / 5.0)).abs() < 1e-6,
        "the off hand lowers the left arm"
    );
}

#[test]
fn player_holding_an_off_hand_item_lowers_the_left_arm() {
    use std::f32::consts::PI;

    // Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM` (`HumanoidModel.poseLeftArm` ITEM
    // case): a player holding a plain off-hand item lowers/halves the OFF (left) walk swing before the
    // mirrored idle bob: `xRot = walkXRot · 0.5 − π/10 − bobX`.
    let age = 11.0_f32;
    let walk_pos = 0.7_f32;
    let walk_speed = 0.8_f32;
    let base = EntityModelInstance::player(940, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -10.0))
        .with(|s| s.age_in_ticks = age)
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (walk_pos, walk_speed));
    let walk_x = (walk_pos * 0.6662).cos() * walk_speed;
    let bob_x = (age * 0.067).sin() * 0.05;
    let expected_x = walk_x * 0.5 - PI / 10.0 - bob_x;
    let mut held = player_model(
        false,
        &base.with(|s| s.kind.player_mut().off_hand_item_pose = true),
    );
    let left = held.child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - expected_x).abs() < 1e-6,
        "the left arm halves walk before adding mirrored bob: {} vs {expected_x}",
        left.rotation[0]
    );
    assert_eq!(
        left.rotation[1], 0.0,
        "the off-hand ITEM pose zeroes the arm yaw"
    );

    // The off-hand ITEM pose is left-arm only; the right (main) arm is untouched.
    let right = held.child_mut("right_arm").pose;
    let mut bare = player_model(false, &base);
    assert_eq!(
        right.rotation,
        bare.child_mut("right_arm").pose.rotation,
        "the off-hand ITEM pose leaves the main (right) arm alone"
    );
}

#[test]
fn player_raising_a_shield_tucks_the_arm_along_the_head_look() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseBlockingArm` `BLOCK`: the raising arm tucks the shield forward —
    // `xRot = arm.xRot · 0.5 − 0.9424779 + clamp(head.xRot, −4π/9, 0.43633232)`,
    // `yRot = (right ? −π/6 : π/6) + clamp(head.yRot, −π/6, π/6)`. With a head look of `(yaw 10°, pitch −10°)`
    // both clamps pass through, and at rest (age 0) the arm pitch is `0`.
    let yaw = 10.0_f32.to_radians();
    let pitch = (-10.0_f32).to_radians();
    let base = EntityModelInstance::player(950, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (10.0, -10.0));
    let mut main = player_model(false, &base.with(|s| s.kind.player_mut().blocking = true));
    let right = main.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-0.9424779 + pitch)).abs() < 1e-5,
        "the right arm pitch tucks to −0.9424779 + head.xRot: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-PI / 6.0 + yaw)).abs() < 1e-5,
        "the right arm yaws −π/6 + head.yRot: {}",
        right.rotation[1]
    );

    // An idle (non-blocking) player keeps its much higher arm pitch — the shield visibly tucks the arm down.
    let mut idle = player_model(false, &base);
    assert!(
        idle.child_mut("right_arm").pose.rotation[0] > -0.9424779 + pitch + 0.3,
        "an idle player does not tuck the arm"
    );

    // Off-hand blocking tucks the LEFT arm instead, with the +π/6 base yaw.
    let mut off = player_model(
        false,
        &base.with(|s| {
            s.kind.player_mut().blocking = true;
            s.use_item_off_hand = true;
        }),
    );
    let left = off.child_mut("left_arm").pose;
    assert!(
        (left.rotation[1] - (PI / 6.0 + yaw)).abs() < 1e-5,
        "the off hand yaws +π/6 + head.yRot: {}",
        left.rotation[1]
    );
}

#[test]
fn player_charging_a_trident_raises_the_arm_overhead() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm`/`poseLeftArm` `THROW_TRIDENT`: the holding arm raises the trident
    // straight overhead — `xRot = arm.xRot · 0.5 − π`, `yRot = 0`. At rest (age 0) the arm pitch is `0`, so
    // it lands at exactly `−π`.
    let base = EntityModelInstance::player(960, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -10.0));
    let mut main = player_model(
        false,
        &base.with(|s| s.kind.player_mut().throwing_trident = true),
    );
    let right = main.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI)).abs() < 1e-6,
        "the right arm raises to −π: {}",
        right.rotation[0]
    );
    assert_eq!(
        right.rotation[1], 0.0,
        "the THROW_TRIDENT pose zeroes the arm yaw"
    );

    // An idle player keeps its much higher arm pitch — charging the throw visibly raises the arm overhead.
    let mut idle = player_model(false, &base);
    assert!(
        idle.child_mut("right_arm").pose.rotation[0] > -PI + 0.5,
        "an idle player does not raise the arm overhead"
    );

    // Off-hand charging raises the LEFT arm instead.
    let mut off = player_model(
        false,
        &base.with(|s| {
            s.kind.player_mut().throwing_trident = true;
            s.use_item_off_hand = true;
        }),
    );
    assert!(
        (off.child_mut("left_arm").pose.rotation[0] - (-PI)).abs() < 1e-6,
        "the off hand raises the left arm overhead"
    );
}

#[test]
fn player_drawing_a_bow_raises_both_arms_along_the_head_look() {
    use std::f32::consts::PI;

    // Vanilla `HumanoidModel.poseRightArm` `BOW_AND_ARROW`: both arms raise along the head look —
    // `rightArm.xRot = leftArm.xRot = −π/2 + head.xRot`, `rightArm.yRot = −0.1 + head.yRot`,
    // `leftArm.yRot = 0.1 + head.yRot + 0.4`. These are SET, overwriting the walk pitch/yaw.
    let yaw = 15.0_f32.to_radians();
    let pitch = (-20.0_f32).to_radians();
    let base = EntityModelInstance::player(970, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (15.0, -20.0))
        .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let mut drawing = player_model(
        false,
        &base.with(|s| s.kind.player_mut().drawing_bow = true),
    );
    let right = drawing.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI / 2.0 + pitch)).abs() < 1e-5,
        "the right arm raises to −π/2 + head.xRot: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-0.1 + yaw)).abs() < 1e-5,
        "the right arm yaws −0.1 + head.yRot: {}",
        right.rotation[1]
    );
    let left = drawing.child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-PI / 2.0 + pitch)).abs() < 1e-5,
        "the left arm raises to −π/2 + head.xRot: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (0.1 + yaw + 0.4)).abs() < 1e-5,
        "the left arm yaws 0.1 + head.yRot + 0.4: {}",
        left.rotation[1]
    );

    // Vanilla `HumanoidModel.poseLeftArm` mirrors the brace offset for off-hand bow draw.
    let mut off = player_model(
        false,
        &base.with(|s| {
            s.kind.player_mut().drawing_bow = true;
            s.use_item_off_hand = true;
        }),
    );
    let right = off.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-PI / 2.0 + pitch)).abs() < 1e-5,
        "the off-hand bow still raises the right arm to −π/2 + head.xRot: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-0.1 + yaw - 0.4)).abs() < 1e-5,
        "the off-hand bow yaws the right arm −0.1 + head.yRot − 0.4: {}",
        right.rotation[1]
    );
    let left = off.child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-PI / 2.0 + pitch)).abs() < 1e-5,
        "the off-hand bow raises the left arm to −π/2 + head.xRot: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (0.1 + yaw)).abs() < 1e-5,
        "the off-hand bow yaws the left arm 0.1 + head.yRot: {}",
        left.rotation[1]
    );

    // A non-drawing walking player keeps its swinging (much lower) arm pitch — the bow visibly raises both.
    let mut idle = player_model(false, &base);
    assert!(
        idle.child_mut("right_arm").pose.rotation[0] > -PI / 2.0 + pitch + 0.5,
        "a non-drawing player does not raise the arm to the bow stance"
    );
}

#[test]
fn player_charging_a_crossbow_braces_and_draws_the_string() {
    use std::f32::consts::FRAC_PI_2;

    // Vanilla `AnimationUtils.animateCrossbowCharge` (right-handed): the right arm braces the crossbow
    // (`yRot = −0.8`, `xRot = −0.97079635`) while the left arm pulls the string back, lerping its `xRot`
    // `−0.97079635 → −π/2` and `yRot` `0.4 → 0.85` over `ticksUsingItem / 25`. At half draw (12.5 / 25 =
    // 0.5) the left arm is halfway through that lerp. Reuses the same helper as the pillager/piglin.
    const HOLD_X: f32 = -0.97079635;
    let base = EntityModelInstance::player(980, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (15.0, -10.0))
        .with(|s| s.crossbow_charge_ticks = 12.5);
    let mut drawing = player_model(
        false,
        &base.with(|s| s.kind.player_mut().charging_crossbow = true),
    );
    let right = drawing.child_mut("right_arm").pose;
    assert!(
        (right.rotation[1] - (-0.8)).abs() < 1e-6,
        "the right arm braces at yRot −0.8: {}",
        right.rotation[1]
    );
    assert!(
        (right.rotation[0] - HOLD_X).abs() < 1e-6,
        "the right arm braces at xRot −0.97079635: {}",
        right.rotation[0]
    );
    let left = drawing.child_mut("left_arm").pose;
    assert!(
        (left.rotation[1] - (0.4 + (0.85 - 0.4) * 0.5)).abs() < 1e-6,
        "the left arm yaws halfway through the draw: {}",
        left.rotation[1]
    );
    assert!(
        (left.rotation[0] - (HOLD_X + (-FRAC_PI_2 - HOLD_X) * 0.5)).abs() < 1e-6,
        "the left arm pitches halfway through the string pull: {}",
        left.rotation[0]
    );
    let mut quick_charge = player_model(
        false,
        &base.with(|s| {
            s.kind.player_mut().charging_crossbow = true;
            s.crossbow_charge_duration_ticks = 15.0;
        }),
    );
    let quick_left = quick_charge.child_mut("left_arm").pose;
    let quick_alpha = 12.5 / 15.0;
    assert!(
        (quick_left.rotation[1] - (0.4 + (0.85 - 0.4) * quick_alpha)).abs() < 1e-6,
        "a synchronized 15-tick duration advances the arm draw: {}",
        quick_left.rotation[1]
    );

    // Vanilla `holdingInRightArm = false` mirrors the brace into the left arm and makes the right arm pull.
    let mut off = player_model(
        false,
        &base.with(|s| {
            s.kind.player_mut().charging_crossbow = true;
            s.use_item_off_hand = true;
        }),
    );
    let left = off.child_mut("left_arm").pose;
    assert!(
        (left.rotation[1] - 0.8).abs() < 1e-6,
        "the off-hand crossbow braces the left arm at yRot 0.8: {}",
        left.rotation[1]
    );
    assert!(
        (left.rotation[0] - HOLD_X).abs() < 1e-6,
        "the off-hand crossbow braces the left arm at xRot −0.97079635: {}",
        left.rotation[0]
    );
    let right = off.child_mut("right_arm").pose;
    assert!(
        (right.rotation[1] - (-(0.4 + (0.85 - 0.4) * 0.5))).abs() < 1e-6,
        "the off-hand crossbow yaws the right arm halfway through the mirrored draw: {}",
        right.rotation[1]
    );
    assert!(
        (right.rotation[0] - (HOLD_X + (-FRAC_PI_2 - HOLD_X) * 0.5)).abs() < 1e-6,
        "the off-hand crossbow pitches the right arm halfway through the string pull: {}",
        right.rotation[0]
    );

    // A non-charging player does not brace — the right arm keeps its (much higher) idle pitch.
    let mut idle = player_model(false, &base);
    assert!(
        idle.child_mut("right_arm").pose.rotation[1] > -0.8 + 0.3,
        "an idle player does not brace the crossbow"
    );
}

#[test]
fn player_holding_a_charged_crossbow_levels_it_along_the_head_look() {
    use std::f32::consts::FRAC_PI_2;

    // Vanilla `AnimationUtils.animateCrossbowHold` (right-handed): the right arm levels the crossbow
    // (`xRot = −π/2 + head.xRot + 0.1`, `yRot = −0.3 + head.yRot`) while the left reaches the trigger
    // (`xRot = −1.5 + head.xRot`, `yRot = 0.6 + head.yRot`). Reuses the same helper as the pillager.
    let yaw = 20.0_f32.to_radians();
    let pitch = (-10.0_f32).to_radians();
    let base = EntityModelInstance::player(990, [0.0, 64.0, 0.0], 0.0, false)
        .with(|s| (s.head_yaw, s.head_pitch) = (20.0, -10.0));
    let mut holding = player_model(
        false,
        &base.with(|s| s.kind.player_mut().crossbow_hold = true),
    );
    let right = holding.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-FRAC_PI_2 + pitch + 0.1)).abs() < 1e-6,
        "the right arm levels at −π/2 + head.xRot + 0.1: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-0.3 + yaw)).abs() < 1e-6,
        "the right arm yaws −0.3 + head.yRot: {}",
        right.rotation[1]
    );
    let left = holding.child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-1.5 + pitch)).abs() < 1e-6,
        "the left arm reaches the trigger at −1.5 + head.xRot: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (0.6 + yaw)).abs() < 1e-6,
        "the left arm yaws 0.6 + head.yRot: {}",
        left.rotation[1]
    );

    // Vanilla `AvatarRenderer.getArmPose(avatar, arm)` forces a non-empty off hand to ITEM when the
    // main-hand pose is the two-handed `CROSSBOW_HOLD`. `HumanoidModel.setupAnim` applies that off-hand pose
    // first and then overwrites both arms with the main-hand hold, so an off-hand SPEAR must not survive.
    let mut holding_over_spear = player_model(
        false,
        &base.with(|s| {
            s.kind.player_mut().off_hand_spear_pose = true;
            s.kind.player_mut().crossbow_hold = true;
        }),
    );
    let right = holding_over_spear.child_mut("right_arm").pose;
    assert!((right.rotation[0] - (-FRAC_PI_2 + pitch + 0.1)).abs() < 1e-6);
    assert!((right.rotation[1] - (-0.3 + yaw)).abs() < 1e-6);
    let left = holding_over_spear.child_mut("left_arm").pose;
    assert!((left.rotation[0] - (-1.5 + pitch)).abs() < 1e-6);
    assert!((left.rotation[1] - (0.6 + yaw)).abs() < 1e-6);

    // Vanilla `holdingInRightArm = false` mirrors `CROSSBOW_HOLD` for a charged off-hand crossbow.
    let mut off = player_model(
        false,
        &base.with(|s| s.kind.player_mut().crossbow_hold_off_hand = true),
    );
    let left = off.child_mut("left_arm").pose;
    assert!(
        (left.rotation[0] - (-FRAC_PI_2 + pitch + 0.1)).abs() < 1e-6,
        "the off-hand crossbow levels the left arm at −π/2 + head.xRot + 0.1: {}",
        left.rotation[0]
    );
    assert!(
        (left.rotation[1] - (0.3 + yaw)).abs() < 1e-6,
        "the off-hand crossbow yaws the left arm 0.3 + head.yRot: {}",
        left.rotation[1]
    );
    let right = off.child_mut("right_arm").pose;
    assert!(
        (right.rotation[0] - (-1.5 + pitch)).abs() < 1e-6,
        "the off-hand crossbow reaches the right arm to −1.5 + head.xRot: {}",
        right.rotation[0]
    );
    assert!(
        (right.rotation[1] - (-0.6 + yaw)).abs() < 1e-6,
        "the off-hand crossbow yaws the right arm −0.6 + head.yRot: {}",
        right.rotation[1]
    );
}

#[test]
fn player_textured_mesh_swings_legs_when_walking() {
    // The real player render path (texture-backed) swings the inherited
    // `HumanoidModel` legs (and the pants children that ride them) on the shared
    // visibility-filtered part array. A standing player is byte-identical however
    // far the swing position has advanced; a walking one lifts its feet. Checked
    // with all model parts visible (pants present) and with the pants hidden.
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    let no_pants = PlayerModelPartVisibility::from_vanilla_mask(
        PlayerModelPartVisibility::ALL_MASK
            & !PlayerModelPartVisibility::LEFT_PANTS_MASK
            & !PlayerModelPartVisibility::RIGHT_PANTS_MASK,
    );
    for slim in [false, true] {
        for (label, base) in [
            (
                "all_parts",
                EntityModelInstance::player(911, [0.0, 64.0, 0.0], 0.0, slim),
            ),
            (
                "no_pants",
                EntityModelInstance::player_with_parts(912, [0.0, 64.0, 0.0], 0.0, slim, no_pants),
            ),
        ] {
            let still_instance =
                base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
            let walking_instance =
                base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
            let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
            let still_meshes = entity_model_textured_meshes(&[still_instance], &atlas);
            let walking_meshes = entity_model_textured_meshes(&[walking_instance], &atlas);
            assert_player_vertices_match_vanilla(&resting_meshes, base);
            assert_player_vertices_match_vanilla(&still_meshes, still_instance);
            assert_player_vertices_match_vanilla(&walking_meshes, walking_instance);
            let resting = &resting_meshes.cutout;
            let still = &still_meshes.cutout;
            let walking = &walking_meshes.cutout;

            assert_eq!(
                resting.vertices, still.vertices,
                "slim={slim} {label}: a standing player is inert"
            );
            assert_eq!(
                resting.vertices.len(),
                walking.vertices.len(),
                "slim={slim} {label}: leg swing keeps the vertex count"
            );
            assert_ne!(
                resting.vertices, walking.vertices,
                "slim={slim} {label}: a walking player differs"
            );

            let (rest_min, rest_max) = textured_mesh_extents(resting);
            let (walk_min, walk_max) = textured_mesh_extents(walking);
            assert!(
                (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.1,
                "slim={slim} {label}: a walking player's feet should lift off the ground"
            );
        }
    }
}

#[test]
fn player_textured_mesh_swings_its_arms_when_walking() {
    // The real (texture-backed) player render path swings the arms on the same shared
    // visibility-filtered part array as the legs; the sleeve children ride the arm parts.
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(921, [0.0, 64.0, 0.0], 0.0, slim);
        let still_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
        let walking_instance =
            base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
        let resting_meshes = entity_model_textured_meshes(&[base], &atlas);
        let still_meshes = entity_model_textured_meshes(&[still_instance], &atlas);
        let walking_meshes = entity_model_textured_meshes(&[walking_instance], &atlas);
        assert_player_vertices_match_vanilla(&resting_meshes, base);
        assert_player_vertices_match_vanilla(&still_meshes, still_instance);
        assert_player_vertices_match_vanilla(&walking_meshes, walking_instance);
        let resting = &resting_meshes.cutout;
        let still = &still_meshes.cutout;
        let walking = &walking_meshes.cutout;
        assert_eq!(
            resting.vertices, still.vertices,
            "slim={slim}: inert when standing"
        );
        assert_eq!(resting.vertices.len(), 288, "slim={slim}");
        assert_eq!(
            player_parts(&resting.vertices, slim, &["head", "body"]),
            player_parts(&walking.vertices, slim, &["head", "body"]),
            "slim={slim}: head and body stay put"
        );
        assert_ne!(
            player_parts(&resting.vertices, slim, &["right_arm", "left_arm"]),
            player_parts(&walking.vertices, slim, &["right_arm", "left_arm"]),
            "slim={slim}: the arms swing"
        );
    }
}

#[test]
fn humanoid_arm_swing_pose_matches_vanilla_formula() {
    // Vanilla HumanoidModel.setupAnim: rightArm.xRot = cos(pos*0.6662 + π)*2.0*speed*0.5,
    // leftArm.xRot = cos(pos*0.6662)*2.0*speed*0.5 (amplitude 1.0). The right arm
    // (offset x < 0) is the out-of-phase one, opposite the same-side leg. Only xRot
    // moves. The right arm rests at x = -5, the left at x = +5.
    let pos = 1.3_f32;
    let speed = 0.7_f32;
    let phase = pos * 0.6662;
    let right = humanoid_arm_swing_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, pos, speed);
    let left = humanoid_arm_swing_pose(PLAYER_FIXTURE_LEFT_ARM_POSE, pos, speed);
    assert!(
        (right.rotation[0] - (phase + std::f32::consts::PI).cos() * 2.0 * speed * 0.5).abs() < 1e-6,
        "right arm out of phase"
    );
    assert!(
        (left.rotation[0] - phase.cos() * 2.0 * speed * 0.5).abs() < 1e-6,
        "left arm in phase"
    );
    // The arm swing is the opposite phase to the same-side leg (right arm uses +π, the
    // right leg uses none) and a shorter amplitude (1.0 vs 1.4).
    let right_leg = humanoid_leg_swing_pose(PLAYER_FIXTURE_RIGHT_LEG_POSE, pos, speed);
    assert!(
        (right.rotation[0] + right_leg.rotation[0] / 1.4).abs() < 1e-6,
        "right arm is the negated, scaled right-leg swing"
    );
    // Only xRot changes.
    assert_eq!(right.offset, PLAYER_FIXTURE_RIGHT_ARM_POSE.offset);
    assert_eq!(right.rotation[1], PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[1]);
    assert_eq!(right.rotation[2], PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[2]);
    // At rest (speed 0) the arms hold their body-layer pose.
    assert_eq!(
        humanoid_arm_swing_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, pos, 0.0),
        PLAYER_FIXTURE_RIGHT_ARM_POSE
    );
}

#[test]
fn humanoid_arm_bob_pose_matches_vanilla_formula() {
    // Vanilla HumanoidModel.setupAnim applies AnimationUtils.bobModelPart to both arms every
    // frame — bobModelPart(rightArm, age, 1.0), bobModelPart(leftArm, age, -1.0):
    //   arm.zRot += scale * (cos(age * 0.09)  * 0.05 + 0.05)
    //   arm.xRot += scale * (sin(age * 0.067) * 0.05)
    // The right arm rests at x = -5 (bob scale +1), the left at x = +5 (scale -1); the bob
    // accumulates onto the arm's rest pose.
    let age = 27.3_f32;
    let bob_x = (age * 0.067).sin() * 0.05;
    let bob_z = (age * 0.09).cos() * 0.05 + 0.05;
    let right = humanoid_arm_bob_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, age);
    let left = humanoid_arm_bob_pose(PLAYER_FIXTURE_LEFT_ARM_POSE, age);
    assert!(
        (right.rotation[0] - (PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[0] + bob_x)).abs() < 1e-6,
        "right arm bob xRot"
    );
    assert!(
        (right.rotation[2] - (PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[2] + bob_z)).abs() < 1e-6,
        "right arm bob zRot"
    );
    // The left arm uses the opposite sign (scale -1).
    assert!(
        (left.rotation[0] - (PLAYER_FIXTURE_LEFT_ARM_POSE.rotation[0] - bob_x)).abs() < 1e-6,
        "left arm bob xRot mirrored"
    );
    assert!(
        (left.rotation[2] - (PLAYER_FIXTURE_LEFT_ARM_POSE.rotation[2] - bob_z)).abs() < 1e-6,
        "left arm bob zRot mirrored"
    );
    // The bob preserves the offset and yRot.
    assert_eq!(right.offset, PLAYER_FIXTURE_RIGHT_ARM_POSE.offset);
    assert_eq!(right.rotation[1], PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[1]);
    // The xRot term vanishes at age 0 (sin 0 = 0) but the zRot baseline does not
    // (cos 0 = 1 gives ±0.1), so the arms never sit at the bare rest pose.
    let at_zero = humanoid_arm_bob_pose(PLAYER_FIXTURE_RIGHT_ARM_POSE, 0.0);
    assert!((at_zero.rotation[0] - PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[0]).abs() < 1e-6);
    assert!((at_zero.rotation[2] - (PLAYER_FIXTURE_RIGHT_ARM_POSE.rotation[2] + 0.1)).abs() < 1e-6);
}

#[test]
fn player_textured_arms_idle_bob_as_age_advances() {
    // The texture-backed player path applies the same idle bob on the shared
    // visibility-filtered part array, so a standing player's arms bob with ageInTicks while
    // the head, body, and legs are byte-identical across ages.
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(931, [0.0, 64.0, 0.0], 0.0, slim);
        let later_instance = base.with(|s| s.age_in_ticks = 27.3);
        let early_meshes = entity_model_textured_meshes(&[base], &atlas);
        let later_meshes = entity_model_textured_meshes(&[later_instance], &atlas);
        assert_player_vertices_match_vanilla(&early_meshes, base);
        assert_player_vertices_match_vanilla(&later_meshes, later_instance);
        let early = &early_meshes.cutout;
        let later = &later_meshes.cutout;
        assert_eq!(early.vertices.len(), 288, "slim={slim}");
        assert_eq!(
            player_parts(&early.vertices, slim, &["head", "body"]),
            player_parts(&later.vertices, slim, &["head", "body"]),
            "slim={slim}: head and body do not bob"
        );
        assert_ne!(
            player_parts(&early.vertices, slim, &["right_arm", "left_arm"]),
            player_parts(&later.vertices, slim, &["right_arm", "left_arm"]),
            "slim={slim}: the arms idle-bob with ageInTicks"
        );
        assert_eq!(
            player_parts(&early.vertices, slim, &["right_leg", "left_leg"]),
            player_parts(&later.vertices, slim, &["right_leg", "left_leg"]),
            "slim={slim}: the legs do not bob"
        );
    }
}

#[test]
fn player_textured_mesh_crouches_when_sneaking() {
    // The real player render path (texture-backed) applies the same crouch to the shared
    // visibility-filtered part array (the hat/jacket/sleeve/pants children ride the shifted
    // parts).
    let (atlas, _) = build_entity_model_texture_atlas(&player_texture_images()).unwrap();
    for slim in [false, true] {
        let base = EntityModelInstance::player(921, [0.0, 64.0, 0.0], 0.0, slim);
        let crouching_instance = base.with(|s| s.is_crouching = true);
        let standing_meshes = entity_model_textured_meshes(&[base], &atlas);
        let crouching_meshes = entity_model_textured_meshes(&[crouching_instance], &atlas);
        assert_player_vertices_match_vanilla(&standing_meshes, base);
        assert_player_vertices_match_vanilla(&crouching_meshes, crouching_instance);
        let standing = &standing_meshes.cutout;
        let crouching = &crouching_meshes.cutout;
        assert_eq!(standing.vertices.len(), crouching.vertices.len());
        assert_ne!(
            standing.vertices, crouching.vertices,
            "slim={slim}: a sneaking player poses differently"
        );

        let (stand_min, stand_max) = textured_mesh_extents(standing);
        let (crouch_min, crouch_max) = textured_mesh_extents(crouching);
        assert!(
            (crouch_max[1] - crouch_min[1]) < (stand_max[1] - stand_min[1]) - 0.1,
            "slim={slim}: a sneaking player is shorter"
        );
        assert!(
            (crouch_max[2] - crouch_min[2]) > (stand_max[2] - stand_min[2]) + 0.1,
            "slim={slim}: a sneaking player leans forward"
        );
    }
}

fn player_parts<V: Copy>(vertices: &[V], slim: bool, names: &[&str]) -> Vec<V> {
    parts_vertices(
        vertices,
        &crate::entity_models::model_layers::player_model_layer(slim).bake(),
        names,
    )
}
