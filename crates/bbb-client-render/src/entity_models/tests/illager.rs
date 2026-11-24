use super::*;
use crate::entity_models::model_layers::illager_model;

#[test]
fn half_amplitude_leg_swing_pose_applies_vanilla_half_amplitude() {
    // Vanilla IllagerModel.setupAnim (non-riding): rightLeg.xRot =
    // cos(pos * 0.6662) * 1.4 * speed * 0.5 (in phase), leftLeg.xRot =
    // cos(pos * 0.6662 + π) * 1.4 * speed * 0.5 (out of phase). The extra 0.5 factor
    // (vs HumanoidModel's 1.4 * speed) is what makes the illager-specific pose. The
    // illager body layers place the right leg at offset x = -2 and the left at x = +2.
    let right = half_amplitude_leg_swing_pose(
        PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        0.0,
        1.0,
    );
    let left = half_amplitude_leg_swing_pose(
        PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        0.0,
        1.0,
    );
    assert!(
        (right.rotation[0] - 0.7).abs() < 1e-6,
        "right leg in phase at half amplitude: {}",
        right.rotation[0]
    );
    assert!(
        (left.rotation[0] + 0.7).abs() < 1e-6,
        "left leg out of phase at half amplitude: {}",
        left.rotation[0]
    );

    // A general (pos, speed) reproduces cos(pos * 0.6662 [+ π]) * 1.4 * speed * 0.5.
    let phase = 1.5_f32 * 0.6662;
    let right = half_amplitude_leg_swing_pose(
        PartPose {
            offset: [-2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        1.5,
        0.5,
    );
    let left = half_amplitude_leg_swing_pose(
        PartPose {
            offset: [2.0, 12.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
        },
        1.5,
        0.5,
    );
    assert!((right.rotation[0] - phase.cos() * 1.4 * 0.5 * 0.5).abs() < 1e-6);
    assert!(
        (left.rotation[0] - (phase + std::f32::consts::PI).cos() * 1.4 * 0.5 * 0.5).abs() < 1e-6
    );
}

#[test]
fn riding_illagers_use_vanilla_seated_pose() {
    use std::f32::consts::PI;

    // Vanilla `IllagerRenderer.extractRenderState`: `state.isRiding = entity.isPassenger()`.
    // `IllagerModel.setupAnim` then uses the fixed seated pose instead of the walk swing.
    let riding =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager)
            .with(|s| s.is_riding = true)
            .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let mut model = illager_model(&riding, IllagerModelFamily::Pillager);

    let right_arm = model.child_mut("right_arm").pose.rotation;
    assert!((right_arm[0] - (-PI / 5.0)).abs() < 1e-6);
    assert!(right_arm[1].abs() < 1e-6 && right_arm[2].abs() < 1e-6);
    let left_arm = model.child_mut("left_arm").pose.rotation;
    assert!((left_arm[0] - (-PI / 5.0)).abs() < 1e-6);
    assert!(left_arm[1].abs() < 1e-6 && left_arm[2].abs() < 1e-6);

    let right_leg = model.child_mut("right_leg").pose.rotation;
    assert!((right_leg[0] - -1.4137167).abs() < 1e-6);
    assert!((right_leg[1] - PI / 10.0).abs() < 1e-6);
    assert!((right_leg[2] - 0.07853982).abs() < 1e-6);
    let left_leg = model.child_mut("left_leg").pose.rotation;
    assert!((left_leg[0] - -1.4137167).abs() < 1e-6);
    assert!((left_leg[1] - -PI / 10.0).abs() < 1e-6);
    assert!((left_leg[2] - -0.07853982).abs() < 1e-6);

    // The seated branch replaces walking legs, but vanilla still runs the arm-pose branch afterwards.
    let attacking =
        EntityModelInstance::illager(140, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Vindicator)
            .with(|s| s.is_riding = true)
            .with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0))
            .with(|s| s.is_aggressive = true);
    let mut attacking_model = illager_model(&attacking, IllagerModelFamily::Vindicator);
    let attacking_right_arm = attacking_model.child_mut("right_arm").pose.rotation;
    assert!(
        (attacking_right_arm[0] - (-PI / 5.0)).abs() > 0.5,
        "ATTACKING runs after the seated arm preset"
    );
    let attacking_right_leg = attacking_model.child_mut("right_leg").pose.rotation;
    assert!((attacking_right_leg[0] - -1.4137167).abs() < 1e-6);
    assert!((attacking_right_leg[1] - PI / 10.0).abs() < 1e-6);
    assert!((attacking_right_leg[2] - 0.07853982).abs() < 1e-6);
}

#[test]
fn riding_illager_arm_poses_still_override_the_seated_arms() {
    use std::f32::consts::{FRAC_PI_2, PI};

    // Vanilla `IllagerModel.setupAnim` writes the riding preset first, then runs the
    // arm-pose branch. The legs stay seated, while spell / bow / crossbow / celebrate
    // arms replace the `-PI / 5` seated arm pitch.
    let spellcasting =
        EntityModelInstance::illager(46, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Evoker).with(
            |s| {
                s.is_riding = true;
                s.kind.illager_mut().spellcasting = true;
            },
        );
    let mut spell_model = illager_model(&spellcasting, IllagerModelFamily::Evoker);
    assert_illager_seated_legs(&mut spell_model);
    let spell_right = spell_model.child_mut("right_arm").pose.rotation;
    assert!((spell_right[0] - 0.25).abs() < 1e-6);
    assert!((spell_right[2] - PI * 3.0 / 4.0).abs() < 1e-6);

    let celebrating =
        EntityModelInstance::illager(140, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Vindicator)
            .with(|s| {
                s.is_riding = true;
                s.kind.illager_mut().celebrating = true;
            });
    let mut celebrating_model = illager_model(&celebrating, IllagerModelFamily::Vindicator);
    assert_illager_seated_legs(&mut celebrating_model);
    let celebrating_right = celebrating_model.child_mut("right_arm").pose.rotation;
    assert!((celebrating_right[0] - 0.05).abs() < 1e-6);
    assert!((celebrating_right[2] - 2.670354).abs() < 1e-6);

    let yaw = 25.0_f32;
    let pitch = -15.0_f32;
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();
    let crossbow_hold =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager)
            .with(|s| s.is_riding = true)
            .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
            .with(|s| s.kind.illager_mut().holds_crossbow = true);
    let mut hold_model = illager_model(&crossbow_hold, IllagerModelFamily::Pillager);
    assert_illager_seated_legs(&mut hold_model);
    let hold_right = hold_model.child_mut("right_arm").pose.rotation;
    assert!((hold_right[0] - (-FRAC_PI_2 + pitch_rad + 0.1)).abs() < 1e-6);
    assert!((hold_right[1] - (-0.3 + yaw_rad)).abs() < 1e-6);

    let crossbow_charge = crossbow_hold.with(|s| {
        s.kind.illager_mut().is_charging_crossbow = true;
        s.crossbow_charge_ticks = 12.5;
    });
    let mut charge_model = illager_model(&crossbow_charge, IllagerModelFamily::Pillager);
    assert_illager_seated_legs(&mut charge_model);
    let charge_right = charge_model.child_mut("right_arm").pose.rotation;
    let charge_left = charge_model.child_mut("left_arm").pose.rotation;
    assert!((charge_right[0] - (-0.97079635)).abs() < 1e-6);
    assert!((charge_right[1] - (-0.8)).abs() < 1e-6);
    assert!(charge_left[1] > 0.4 && charge_left[1] < 0.85);
    let quick_charge = crossbow_charge.with(|s| s.crossbow_charge_duration_ticks = 15.0);
    let mut quick_model = illager_model(&quick_charge, IllagerModelFamily::Pillager);
    let quick_left = quick_model.child_mut("left_arm").pose.rotation;
    assert!(
        (quick_left[1] - (0.4 + (0.85 - 0.4) * (12.5 / 15.0))).abs() < 1e-6,
        "a synchronized 15-tick duration advances the pillager arm draw"
    );

    let bow_aim =
        EntityModelInstance::illager(68, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Illusioner)
            .with(|s| s.is_riding = true)
            .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
            .with(|s| s.is_aggressive = true);
    let mut bow_model = illager_model(&bow_aim, IllagerModelFamily::Illusioner);
    assert_illager_seated_legs(&mut bow_model);
    let bow_right = bow_model.child_mut("right_arm").pose.rotation;
    let bow_left = bow_model.child_mut("left_arm").pose.rotation;
    assert!((bow_right[0] - (-FRAC_PI_2 + pitch_rad)).abs() < 1e-6);
    assert!((bow_right[1] - (-0.1 + yaw_rad)).abs() < 1e-6);
    assert!((bow_left[2] - FRAC_PI_2).abs() < 1e-6);
}

fn assert_illager_seated_legs(model: &mut ModelPart) {
    use std::f32::consts::PI;

    let right_leg = model.child_mut("right_leg").pose.rotation;
    assert!((right_leg[0] - -1.4137167).abs() < 1e-6);
    assert!((right_leg[1] - PI / 10.0).abs() < 1e-6);
    assert!((right_leg[2] - 0.07853982).abs() < 1e-6);
    let left_leg = model.child_mut("left_leg").pose.rotation;
    assert!((left_leg[0] - -1.4137167).abs() < 1e-6);
    assert!((left_leg[1] - -PI / 10.0).abs() < 1e-6);
    assert!((left_leg[2] - -0.07853982).abs() < 1e-6);
}

#[test]
fn pillager_holds_its_crossbow_level_with_the_head_look() {
    use std::f32::consts::FRAC_PI_2;
    // Vanilla `IllagerModel.setupAnim` `CROSSBOW_HOLD` (`AnimationUtils.animateCrossbowHold`,
    // `holdingInRightArm = true`): the right (holding) arm levels the crossbow along the head look
    // (`xRot = -π/2 + head.xRot + 0.1`, `yRot = -0.3 + head.yRot`), the left (shooting) arm reaches
    // across (`xRot = -1.5 + head.xRot`, `yRot = 0.6 + head.yRot`); the walk swing zeroed `zRot`.
    let yaw = 25.0_f32;
    let pitch = -15.0_f32;
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();

    let holding =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager)
            .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
            .with(|s| s.kind.illager_mut().holds_crossbow = true);
    let mut model = illager_model(&holding, IllagerModelFamily::Pillager);

    let right = model.child_mut("right_arm");
    assert!(
        (right.pose.rotation[0] - (-FRAC_PI_2 + pitch_rad + 0.1)).abs() < 1e-6,
        "right (holding) arm levels the crossbow: {}",
        right.pose.rotation[0]
    );
    assert!(
        (right.pose.rotation[1] - (-0.3 + yaw_rad)).abs() < 1e-6,
        "right arm yaws -0.3 off the head: {}",
        right.pose.rotation[1]
    );
    assert!(
        right.pose.rotation[2].abs() < 1e-6,
        "zRot preserved at 0 from the swing"
    );
    let left = model.child_mut("left_arm");
    assert!(
        (left.pose.rotation[0] - (-1.5 + pitch_rad)).abs() < 1e-6,
        "left (shooting) arm reaches across: {}",
        left.pose.rotation[0]
    );
    assert!(
        (left.pose.rotation[1] - (0.6 + yaw_rad)).abs() < 1e-6,
        "left arm yaws 0.6 off the head: {}",
        left.pose.rotation[1]
    );

    // Charging takes the higher-priority `CROSSBOW_CHARGE` pull-back pose instead of the level hold:
    // the right (holding) arm braces (`xRot = -0.97079635`, not the hold's `-π/2 + pitch + 0.1`).
    let charging = holding.with(|s| s.kind.illager_mut().is_charging_crossbow = true);
    let mut charging_model = illager_model(&charging, IllagerModelFamily::Pillager);
    assert!(
        (charging_model.child_mut("right_arm").pose.rotation[0] - (-0.97079635)).abs() < 1e-6,
        "a charging pillager braces the crossbow to draw, not the level hold"
    );
}

#[test]
fn pillager_pulls_its_crossbow_back_while_charging() {
    use std::f32::consts::FRAC_PI_2;
    // Vanilla `AnimationUtils.animateCrossbowCharge` (`holdingInRightArm = true`): the right (holding)
    // arm braces (`yRot = -0.8`, `xRot = -0.97079635`); the left (pulling) arm draws the string back as
    // `ticksUsingItem / maxChargeDuration` (= 25) climbs — `yRot` lerps `0.4 → 0.85`, `xRot` lerps
    // `-0.97079635 → -π/2`.
    const HOLD_X: f32 = -0.97079635;
    let charge = |ticks: f32| {
        let instance =
            EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager)
                .with(|s| {
                    s.kind.illager_mut().is_charging_crossbow = true;
                    s.crossbow_charge_ticks = ticks;
                });
        let mut model = illager_model(&instance, IllagerModelFamily::Pillager);
        let right = model.child_mut("right_arm").pose;
        let left = model.child_mut("left_arm").pose;
        (right, left)
    };

    // At the start of the draw (ticks 0) the pulling arm sits at the brace.
    let (right0, left0) = charge(0.0);
    assert!(
        (right0.rotation[1] - (-0.8)).abs() < 1e-6 && (right0.rotation[0] - HOLD_X).abs() < 1e-6,
        "the holding arm braces: {:?}",
        right0.rotation
    );
    assert!(
        (left0.rotation[1] - 0.4).abs() < 1e-6 && (left0.rotation[0] - HOLD_X).abs() < 1e-6,
        "the pulling arm starts at the brace: {:?}",
        left0.rotation
    );

    // Fully drawn (ticks ≥ 25) the pulling arm reaches the far draw.
    let (_right_full, left_full) = charge(25.0);
    assert!(
        (left_full.rotation[1] - 0.85).abs() < 1e-6
            && (left_full.rotation[0] - (-FRAC_PI_2)).abs() < 1e-6,
        "the pulling arm reaches full draw: {:?}",
        left_full.rotation
    );

    // Mid-draw is strictly between the two (the string is being pulled back).
    let (_right_mid, left_mid) = charge(12.5);
    assert!(
        left_mid.rotation[1] > 0.4 && left_mid.rotation[1] < 0.85,
        "mid-draw the pulling arm is partway back: {}",
        left_mid.rotation[1]
    );
    // Past the max the draw clamps (no overshoot).
    let (_r, left_over) = charge(40.0);
    assert!(
        (left_over.rotation[1] - 0.85).abs() < 1e-6,
        "over-charge clamps to full draw: {}",
        left_over.rotation[1]
    );
}

#[test]
fn illager_textured_layer_passes_match_vanilla_renderer() {
    let cases = [
        (IllagerModelFamily::Evoker, EVOKER_TEXTURE_REF),
        (IllagerModelFamily::Illusioner, ILLUSIONER_TEXTURE_REF),
        (IllagerModelFamily::Pillager, PILLAGER_TEXTURE_REF),
        (IllagerModelFamily::Vindicator, VINDICATOR_TEXTURE_REF),
    ];
    for (family, texture) in cases {
        let passes = illager_textured_layer_passes(family);
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(passes[0].order, 0);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert!(entity_model_texture_refs().contains(&texture));
    }
}

#[test]
fn illager_textured_mesh_geometry_and_swings() {
    let (atlas, _) = build_entity_model_texture_atlas(&illager_texture_images()).unwrap();
    let families = [
        IllagerModelFamily::Evoker,
        IllagerModelFamily::Illusioner,
        IllagerModelFamily::Pillager,
        IllagerModelFamily::Vindicator,
    ];
    for family in families {
        let instances = [
            EntityModelInstance::illager(45, [0.0, 64.0, 0.0], 0.0, family).with(|s| {
                s.light_coords = (5_u32 << 4) | (12_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            }),
        ];
        let textured_meshes = entity_model_textured_meshes(&instances, &atlas);
        assert_illager_textured_buckets(&textured_meshes);
        let textured = &textured_meshes.cutout;
        assert!(textured
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
        assert!(textured.vertices.iter().all(|vertex| vertex.light
            == instances[0].render_state.shader_light()
            && vertex.overlay == instances[0].render_state.overlay_coords()));
        assert_ne!(instances[0].render_state.overlay_coords(), [0.0, 10.0]);

        // Walking re-poses the legs on both render paths.
        let walking =
            [instances[0].with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0))];
        let textured_walk = entity_model_textured_meshes(&walking, &atlas);
        assert_illager_textured_buckets(&textured_walk);
        assert_ne!(
            textured.vertices, textured_walk.cutout.vertices,
            "{family:?} legs"
        );
    }

    // The pillager swings its visible separate arms; the crossed-arm families hold still.
    let pillager =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager);
    let pillager_walk_instance =
        pillager.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let pillager_rest = entity_model_textured_meshes(&[pillager], &atlas);
    let pillager_walk = entity_model_textured_meshes(&[pillager_walk_instance], &atlas);
    assert_illager_textured_buckets(&pillager_rest);
    assert_illager_textured_buckets(&pillager_walk);
    assert_eq!(pillager_rest.cutout.vertices.len(), 192);
    let posed = illager_model(&pillager, IllagerModelFamily::Pillager);
    let arms = ["right_arm", "left_arm"];
    assert_ne!(
        parts_vertices(&pillager_rest.cutout.vertices, &posed, &arms),
        parts_vertices(&pillager_walk.cutout.vertices, &posed, &arms),
        "the textured pillager swings its separate arms"
    );

    let evoker =
        EntityModelInstance::illager(46, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Evoker);
    let evoker_walk_instance =
        evoker.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let evoker_rest = entity_model_textured_meshes(&[evoker], &atlas);
    let evoker_walk = entity_model_textured_meshes(&[evoker_walk_instance], &atlas);
    assert_illager_textured_buckets(&evoker_rest);
    assert_illager_textured_buckets(&evoker_walk);
    assert_eq!(evoker_rest.cutout.vertices.len(), 216);
    assert_eq!(
        evoker_rest.cutout.vertices[96..168],
        evoker_walk.cutout.vertices[96..168],
        "the textured crossed arms part stays still when walking"
    );

    // The textured spellcasting evoker swaps to the uncrossed layout (216 → 192 verts); an idle illusioner keeps its hat (216 verts when casting).
    let evoker_cast_instance = evoker.with(|s| s.kind.illager_mut().spellcasting = true);
    let evoker_cast = entity_model_textured_meshes(&[evoker_cast_instance], &atlas);
    assert_illager_textured_buckets(&evoker_cast);
    assert_eq!(evoker_cast.cutout.cutout_faces, 48);
    assert_eq!(evoker_cast.cutout.vertices.len(), 192);
    assert_ne!(evoker_rest.cutout.vertices, evoker_cast.cutout.vertices);
    let illusioner =
        EntityModelInstance::illager(68, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Illusioner);
    let illusioner_cast_instance = illusioner.with(|s| s.kind.illager_mut().spellcasting = true);
    let illusioner_cast = entity_model_textured_meshes(&[illusioner_cast_instance], &atlas);
    assert_illager_textured_buckets(&illusioner_cast);
    assert_eq!(illusioner_cast.cutout.vertices.len(), 216);
}

#[test]
fn invisible_illusioner_submits_visible_clone_bodies() {
    let (atlas, _) = build_entity_model_texture_atlas(&illager_texture_images()).unwrap();
    let offsets = [
        [1.5, 0.0, -0.5],
        [-1.0, 1.0, 0.75],
        [0.25, 0.0, 1.25],
        [-1.5, 0.0, -1.25],
    ];
    let instance =
        EntityModelInstance::illager(68, [4.0, 64.0, -2.0], 30.0, IllagerModelFamily::Illusioner)
            .with_invisible(true)
            .with_appears_glowing(true)
            .with(|s| {
                s.age_in_ticks = 8.0;
                s.light_coords = (6_u32 << 4) | (10_u32 << 20);
                s.has_red_overlay = true;
                s.kind.illager_mut().clone_offsets = offsets;
            });

    let textured = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(textured.cutout.vertices.len(), 240 * 4);
    assert!(textured.translucent.vertices.is_empty());
    assert_eq!(textured.outline.vertices.len(), 240 * 4);
    assert!(textured.outline_cull.vertices.is_empty());

    let evoker =
        EntityModelInstance::illager(46, [4.0, 64.0, -2.0], 30.0, IllagerModelFamily::Evoker)
            .with_invisible(true)
            .with_appears_glowing(true);
    let evoker_outline = entity_model_textured_meshes(&[evoker], &atlas);

    assert!(evoker_outline.cutout.vertices.is_empty());
    assert!(!evoker_outline.outline.vertices.is_empty());
}

fn assert_illager_textured_buckets(meshes: &EntityModelTexturedMeshes) {
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

fn illager_texture_images() -> Vec<EntityModelTextureImage> {
    [
        EVOKER_TEXTURE_REF,
        ILLUSIONER_TEXTURE_REF,
        PILLAGER_TEXTURE_REF,
        VINDICATOR_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}

#[test]
fn illager_spellcast_arm_pose_matches_vanilla() {
    use std::f32::consts::PI;

    // Vanilla `IllagerModel.setupAnim` SPELLCASTING: `rightArm.x = -5` / `leftArm.x = 5`,
    // `z = 0` (both the bind offset), `xRot = cos(ageInTicks · 0.6662) · 0.25`, and
    // `zRot = ±3π/4` (right `+`, left `−`), `yRot = 0`. The offset is unchanged from the base.
    let right = illager_spellcast_arm_pose(layer_pose(ModelLayer::EVOKER, "right_arm"), 0.0, true);
    assert_eq!(right.offset, [-5.0, 2.0, 0.0]);
    assert!((right.rotation[0] - 0.25).abs() < 1.0e-6); // cos(0) * 0.25
    assert_eq!(right.rotation[1], 0.0);
    assert!((right.rotation[2] - PI * 3.0 / 4.0).abs() < 1.0e-6);

    let left = illager_spellcast_arm_pose(layer_pose(ModelLayer::EVOKER, "left_arm"), 0.0, false);
    assert_eq!(left.offset, [5.0, 2.0, 0.0]);
    assert!((left.rotation[2] - (-PI * 3.0 / 4.0)).abs() < 1.0e-6);

    // The arm pitch animates with `ageInTicks`.
    let aged = illager_spellcast_arm_pose(layer_pose(ModelLayer::EVOKER, "right_arm"), 5.0, true);
    assert!((aged.rotation[0] - (5.0_f32 * 0.6662).cos() * 0.25).abs() < 1.0e-6);
}

#[test]
fn illager_celebrate_arm_pose_matches_vanilla() {
    use std::f32::consts::PI;

    // Vanilla `IllagerModel.setupAnim` CELEBRATING: both arms hold the bind offset (`x = ∓5`, `z = 0`),
    // bob `xRot = cos(ageInTicks · 0.6662) · 0.05`, raise asymmetrically (right `zRot = 2.670354`, left
    // `zRot = -3π/4`), with `yRot = 0`.
    let right = illager_celebrate_arm_pose(layer_pose(ModelLayer::EVOKER, "right_arm"), 0.0, true);
    assert_eq!(right.offset, [-5.0, 2.0, 0.0]);
    assert!((right.rotation[0] - 0.05).abs() < 1.0e-6); // cos(0) * 0.05
    assert_eq!(right.rotation[1], 0.0);
    assert!((right.rotation[2] - 2.670354).abs() < 1.0e-6);

    let left = illager_celebrate_arm_pose(layer_pose(ModelLayer::EVOKER, "left_arm"), 0.0, false);
    assert_eq!(left.offset, [5.0, 2.0, 0.0]);
    assert!((left.rotation[2] - (-PI * 3.0 / 4.0)).abs() < 1.0e-6);

    // The arm pitch bobs with `ageInTicks`.
    let aged = illager_celebrate_arm_pose(layer_pose(ModelLayer::EVOKER, "right_arm"), 5.0, true);
    assert!((aged.rotation[0] - (5.0_f32 * 0.6662).cos() * 0.05).abs() < 1.0e-6);
}

#[test]
fn aggressive_illusioner_draws_its_bow() {
    use std::f32::consts::FRAC_PI_2;

    // Vanilla `Illusioner.getArmPose`: `!isCastingSpell() && isAggressive()` → BOW_AND_ARROW. The
    // illager braces the off hand across the bow: right arm `xRot = -π/2 + head.xRot`,
    // `yRot = -0.1 + head.yRot`; left arm `xRot = -0.9424779 + head.xRot`, `yRot = head.yRot - 0.4`,
    // `zRot = π/2`.
    let yaw = 20.0_f32;
    let pitch = -12.0_f32;
    let yaw_rad = yaw.to_radians();
    let pitch_rad = pitch.to_radians();
    let drawing =
        EntityModelInstance::illager(68, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Illusioner)
            .with(|s| (s.head_yaw, s.head_pitch) = (yaw, pitch))
            .with(|s| s.is_aggressive = true);
    let mut model = illager_model(&drawing, IllagerModelFamily::Illusioner);

    let right = model.child_mut("right_arm");
    assert!(
        (right.pose.rotation[0] - (-FRAC_PI_2 + pitch_rad)).abs() < 1.0e-6,
        "right arm aims down the head pitch: {}",
        right.pose.rotation[0]
    );
    assert!((right.pose.rotation[1] - (-0.1 + yaw_rad)).abs() < 1.0e-6);
    let left = model.child_mut("left_arm");
    assert!((left.pose.rotation[0] - (-0.9424779 + pitch_rad)).abs() < 1.0e-6);
    assert!((left.pose.rotation[1] - (yaw_rad - 0.4)).abs() < 1.0e-6);
    assert!(
        (left.pose.rotation[2] - FRAC_PI_2).abs() < 1.0e-6,
        "the braced off arm rolls to π/2"
    );

    // The mesh swaps the crossed arms (240 hatted verts) for the uncrossed hatted layout (216); an
    // idle illusioner stays crossed.
    let idle = entity_model_mesh(&[EntityModelInstance::illager(
        68,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Illusioner,
    )]);
    let aggressive = entity_model_mesh(&[drawing]);
    assert_eq!(idle.vertices.len(), 240);
    assert_eq!(
        aggressive.vertices.len(),
        216,
        "the crossed arms part is hidden and the separate arms draw the bow"
    );
}

#[test]
fn celebrating_raiders_raise_the_victory_arms() {
    // Vanilla `SpellcasterIllager`/`Vindicator.getArmPose`: not casting / not aggressive and
    // `isCelebrating()` → CELEBRATING. The evoker and vindicator swap their crossed arms (216) for the
    // uncrossed separate-arm dance (192) and roll the right arm up to `2.670354`.
    for (id, family) in [
        (46, IllagerModelFamily::Evoker),
        (140, IllagerModelFamily::Vindicator),
    ] {
        let base = EntityModelInstance::illager(id, [0.0, 64.0, 0.0], 0.0, family);
        let idle = entity_model_mesh(&[base]);
        let celebrating =
            entity_model_mesh(&[base.with(|s| s.kind.illager_mut().celebrating = true)]);
        assert_eq!(idle.vertices.len(), 216);
        assert_eq!(
            celebrating.vertices.len(),
            192,
            "the crossed arms hide and two separate arms dance"
        );
        assert_ne!(idle.vertices, celebrating.vertices);

        let celebrating_instance = base.with(|s| s.kind.illager_mut().celebrating = true);
        let mut model = illager_model(&celebrating_instance, family);
        assert!(
            (model.child_mut("right_arm").pose.rotation[2] - 2.670354).abs() < 1.0e-6,
            "the right arm raises into the victory roll"
        );
    }

    // The celebrate flag is gated to the evoker/vindicator: a celebrating pillager keeps its
    // crossbow-swing arms (it never returns CELEBRATING), and an illusioner is unaffected (it draws
    // a bow when aggressive, crosses otherwise).
    let pillager =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager);
    assert_eq!(
        entity_model_mesh(&[pillager]).vertices,
        entity_model_mesh(&[pillager.with(|s| s.kind.illager_mut().celebrating = true)]).vertices,
        "the pillager ignores the celebrate flag"
    );
}

#[test]
fn aggressive_vindicator_chops_with_its_axe() {
    use std::f32::consts::PI;

    // Vanilla `Vindicator.getArmPose`: aggressive → ATTACKING. `IllagerModel.setupAnim` takes the armed
    // `AnimationUtils.swingWeaponDown` (mainArm = RIGHT): the right arm raises overhead
    // (`xRot = -1.8849558 + cos(age·0.09)·0.15`) and chops with `+= sin(t·π)·2.2 - sin((1-(1-t)²)·π)·0.4`,
    // the left arm trails (`xRot = cos(age·0.19)·0.5 + sin(t·π)·1.2 - …·0.4`), both yawing apart `±π/20`.
    // Aggression outranks celebrating, so the celebrate flag is ignored. `age = 0`, `t = 0` here.
    let attacking =
        EntityModelInstance::illager(140, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Vindicator)
            .with(|s| {
                s.kind.illager_mut().celebrating = true;
                s.is_aggressive = true;
            });
    let mut model = illager_model(&attacking, IllagerModelFamily::Vindicator);

    let right = model.child_mut("right_arm").pose.rotation;
    assert!(
        (right[0] - (-1.8849558 + 0.15)).abs() < 1.0e-6,
        "the right arm raises overhead: {}",
        right[0]
    );
    assert!((right[1] - PI / 20.0).abs() < 1.0e-6);
    // `swingWeaponDown` zeroes `zRot`, then `bobArms` rolls it back out (`+cos(0)·0.05 + 0.05 = 0.1`).
    assert!(
        (right[2] - 0.1).abs() < 1.0e-6,
        "the bob rolls zRot: {}",
        right[2]
    );
    let left = model.child_mut("left_arm").pose.rotation;
    assert!(
        (left[0] - 0.5).abs() < 1.0e-6,
        "the off arm trails at cos(0)·0.5: {}",
        left[0]
    );
    assert!((left[1] - (-PI / 20.0)).abs() < 1.0e-6);
    assert!(
        (left[2] - (-0.1)).abs() < 1.0e-6,
        "the off arm bob mirrors: {}",
        left[2]
    );

    // Mid-swing (`t = 0.5`) the chop drives the right arm forward from its raised rest.
    let raised = right[0];
    let mid = attacking.with(|s| s.attack_anim = 0.5);
    let mut swinging = illager_model(&mid, IllagerModelFamily::Vindicator);
    let chopping = swinging.child_mut("right_arm").pose.rotation[0];
    let attack2 = (0.5_f32 * PI).sin();
    let attack = ((1.0 - 0.5 * 0.5) * PI).sin();
    assert!(
        (chopping - (-1.8849558 + 0.15 + (attack2 * 2.2 - attack * 0.4))).abs() < 1.0e-6,
        "the axe chops down with the attack swing: {chopping}"
    );
    assert!(chopping > raised, "the chop swings the arm forward");

    // The aggressive vindicator swaps the crossed arms (216) for the uncrossed two-arm layout (192).
    assert_eq!(entity_model_mesh(&[attacking]).vertices.len(), 192);
}

#[test]
fn aggressive_pillager_attacks_when_not_holding_crossbow() {
    use std::f32::consts::PI;

    // Vanilla `Pillager.getArmPose`: `CROSSBOW_CHARGE` and `CROSSBOW_HOLD` win first; otherwise an
    // aggressive pillager returns `ATTACKING` and `IllagerModel.setupAnim` uses the same armed
    // `swingWeaponDown` branch as a vindicator when the rendered main hand is non-empty.
    let attacking =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager)
            .with(|s| s.is_aggressive = true);
    let mut model = illager_model(&attacking, IllagerModelFamily::Pillager);

    let right = model.child_mut("right_arm").pose.rotation;
    assert!(
        (right[0] - (-1.8849558 + 0.15)).abs() < 1.0e-6,
        "the right arm raises overhead: {}",
        right[0]
    );
    assert!((right[1] - PI / 20.0).abs() < 1.0e-6);
    assert!(
        (right[2] - 0.1).abs() < 1.0e-6,
        "the bob rolls zRot: {}",
        right[2]
    );

    let calm =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager);
    assert_ne!(
        entity_model_mesh(&[attacking]).vertices,
        entity_model_mesh(&[calm]).vertices,
        "aggressive pillager should not stay on the neutral arm swing"
    );

    let holding_crossbow = calm.with(|s| s.kind.illager_mut().holds_crossbow = true);
    assert_eq!(
        entity_model_mesh(&[holding_crossbow.with(|s| s.is_aggressive = true)]).vertices,
        entity_model_mesh(&[holding_crossbow]).vertices,
        "CROSSBOW_HOLD outranks ATTACKING"
    );
    let charging_crossbow = calm.with(|s| s.kind.illager_mut().is_charging_crossbow = true);
    assert_eq!(
        entity_model_mesh(&[charging_crossbow.with(|s| s.is_aggressive = true)]).vertices,
        entity_model_mesh(&[charging_crossbow]).vertices,
        "CROSSBOW_CHARGE outranks ATTACKING"
    );
}

#[test]
fn empty_hand_attacking_vindicator_uses_zombie_arms() {
    use std::f32::consts::PI;

    // Vanilla `IllagerModel.setupAnim` ATTACKING branch: an empty main hand uses
    // `AnimationUtils.animateZombieArms(left, right, true, state)` instead of the armed
    // `swingWeaponDown` chop.
    let attacking =
        EntityModelInstance::illager(140, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Vindicator)
            .with(|s| {
                s.is_aggressive = true;
                s.kind.illager_mut().main_hand_empty = true;
            });
    let mut model = illager_model(&attacking, IllagerModelFamily::Vindicator);

    let right = model.child_mut("right_arm").pose.rotation;
    assert!(
        (right[0] - (-PI / 1.5)).abs() < 1.0e-6,
        "empty-hand attacking right arm reaches forward: {}",
        right[0]
    );
    assert!((right[1] - -0.1).abs() < 1.0e-6);
    assert!((right[2] - 0.1).abs() < 1.0e-6);
    let left = model.child_mut("left_arm").pose.rotation;
    assert!((left[0] - (-PI / 1.5)).abs() < 1.0e-6);
    assert!((left[1] - 0.1).abs() < 1.0e-6);
    assert!((left[2] - -0.1).abs() < 1.0e-6);

    let mid = attacking.with(|s| s.attack_anim = 0.5);
    let mut swinging = illager_model(&mid, IllagerModelFamily::Vindicator);
    let swinging_right = swinging.child_mut("right_arm").pose.rotation;
    let attack_y = (0.5_f32 * PI).sin();
    let attack_x = ((1.0 - 0.5 * 0.5) * PI).sin();
    assert!(
        (swinging_right[0] - (-PI / 1.5 + attack_y * 1.2 - attack_x * 0.4)).abs() < 1.0e-6,
        "empty-hand attack uses zombie swing xRot: {}",
        swinging_right[0]
    );
    assert!((swinging_right[1] - -(0.1 - attack_y * 0.6)).abs() < 1.0e-6);
}
