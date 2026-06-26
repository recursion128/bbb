use super::*;

use crate::entity_models::model::{EntityModel, ModelCube};

#[test]
fn illager_model_parts_match_vanilla_26_1_body_layer() {
    // The unified cubes carry both render paths' geometry: the colored debug tint and the textured
    // `uv_size`/`texOffs`/`mirror`. The hat / body robe overlay deformed cubes inflate their geometry
    // but keep the base box as `uv_size`.
    assert_eq!(
        ILLAGER_HEAD[0],
        ModelCube::new(
            [-4.0, -10.0, -4.0],
            [8.0, 10.0, 8.0],
            ILLAGER_ROBE,
            [8.0, 10.0, 8.0],
            [0.0, 0.0],
            false,
        )
    );
    assert_eq!(
        ILLAGER_HAT[0],
        ModelCube::new(
            [-4.45, -10.45, -4.45],
            [8.9, 12.9, 8.9],
            ILLAGER_HAT_COLOR,
            [8.0, 12.0, 8.0],
            [32.0, 0.0],
            false,
        )
    );
    assert_eq!(
        ILLAGER_BODY[1],
        ModelCube::new(
            [-4.5, -0.5, -3.5],
            [9.0, 21.0, 7.0],
            ILLAGER_ROBE,
            [8.0, 20.0, 6.0],
            [0.0, 38.0],
            false,
        )
    );
    assert_eq!(ILLAGER_NOSE[0].tex, [24.0, 0.0]);
    assert_eq!(ILLAGER_CROSSED_ARMS[0].tex, [44.0, 22.0]);
    assert_eq!(ILLAGER_CROSSED_ARMS[1].tex, [40.0, 38.0]);
    assert!(ILLAGER_LEFT_SHOULDER[0].mirror);
    assert_eq!(ILLAGER_RIGHT_LEG[0].size, [4.0, 12.0, 4.0]);
    assert!(!ILLAGER_RIGHT_LEG[0].mirror);
    assert!(ILLAGER_LEFT_LEG[0].mirror);
    assert_eq!(ILLAGER_RIGHT_ARM[0].tex, [40.0, 46.0]);
    assert!(!ILLAGER_RIGHT_ARM[0].mirror);
    assert!(ILLAGER_LEFT_ARM[0].mirror);
}

#[test]
fn illager_model_meshes_use_vanilla_scaled_body_layer_geometry() {
    let evoker = entity_model_mesh(&[EntityModelInstance::illager(
        46,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Evoker,
    )]);
    assert_eq!(evoker.opaque_faces, 54);
    assert_eq!(evoker.vertices.len(), 216);
    assert_eq!(evoker.indices.len(), 324);
    let (evoker_min, evoker_max) = mesh_extents(&evoker);
    assert_close3(evoker_min, [-0.46875, 64.00094, -0.23437501]);
    assert_close3(evoker_max, [0.46875003, 65.993126, 0.3839772]);

    let illusioner = entity_model_mesh(&[EntityModelInstance::illager(
        68,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Illusioner,
    )]);
    assert_eq!(illusioner.opaque_faces, 60);
    assert_eq!(illusioner.vertices.len(), 240);
    assert_eq!(illusioner.indices.len(), 360);
    let (illusioner_min, illusioner_max) = mesh_extents(&illusioner);
    assert_close3(illusioner_min, [-0.46875, 64.00094, -0.26074222]);
    assert_close3(illusioner_max, [0.46875003, 66.01949, 0.3839772]);

    let pillager = entity_model_mesh(&[EntityModelInstance::illager(
        103,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Pillager,
    )]);
    assert_eq!(pillager.opaque_faces, 48);
    assert_eq!(pillager.vertices.len(), 192);
    assert_eq!(pillager.indices.len(), 288);
    let (pillager_min, pillager_max) = mesh_extents(&pillager);
    assert_close3(pillager_min, [-0.46875, 64.00094, -0.23437501]);
    assert_close3(pillager_max, [0.46875, 65.993126, 0.3515625]);

    let vindicator = entity_model_mesh(&[EntityModelInstance::illager(
        140,
        [0.0, 64.0, 0.0],
        0.0,
        IllagerModelFamily::Vindicator,
    )]);
    assert_eq!(vindicator.vertices, evoker.vertices);
    assert_eq!(vindicator.indices, evoker.indices);
}

#[test]
fn illager_texture_refs_match_vanilla_renderers() {
    let cases = [
        (
            IllagerModelFamily::Evoker,
            "evoker",
            EntityModelTextureRef {
                path: "textures/entity/illager/evoker.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Illusioner,
            "illusioner",
            EntityModelTextureRef {
                path: "textures/entity/illager/illusioner.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Pillager,
            "pillager",
            EntityModelTextureRef {
                path: "textures/entity/illager/pillager.png",
                size: [64, 64],
            },
        ),
        (
            IllagerModelFamily::Vindicator,
            "vindicator",
            EntityModelTextureRef {
                path: "textures/entity/illager/vindicator.png",
                size: [64, 64],
            },
        ),
    ];

    for (family, model_key, texture) in cases {
        let kind = EntityModelKind::Illager { family };
        assert_eq!(kind.model_key(), model_key);
        assert_eq!(kind.vanilla_texture_ref(), Some(texture));
    }
}

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
fn illager_family_swings_its_legs_when_walking() {
    // IllagerModel is not a HumanoidModel but its non-riding setupAnim swings the
    // legs `cos(pos * 0.6662 [+ π]) * 1.4 * speed * 0.5`. A standing illager is inert;
    // a walking one lifts its feet (a shorter model) and splays its legs along Z, for
    // every family (the crossed-arms evoker/vindicator/illusioner lists legs at
    // [3, 4], the uncrossed pillager at [2, 3]). The pillager also swings its separate arms
    // (see `pillager_swings_its_arms_when_walking`); the arm-pose overrides and the riding
    // sit pose are deferred.
    let families = [
        ("evoker", IllagerModelFamily::Evoker),
        ("vindicator", IllagerModelFamily::Vindicator),
        ("illusioner", IllagerModelFamily::Illusioner),
        ("pillager", IllagerModelFamily::Pillager),
    ];
    for (name, family) in families {
        let base = EntityModelInstance::illager(200, [0.0, 64.0, 0.0], 0.0, family);
        let rest = entity_model_mesh(&[base]);
        let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
        assert_eq!(rest.vertices, still.vertices, "{name}: rest is inert");

        let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
        assert_ne!(rest.vertices, walking.vertices, "{name}: walking differs");

        let (rest_min, rest_max) = mesh_extents(&rest);
        let (walk_min, walk_max) = mesh_extents(&walking);
        assert!(
            (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
            "{name}: a walking illager's feet should lift off the ground"
        );
        assert!(
            (walk_max[2] - walk_min[2]) > (rest_max[2] - rest_min[2]) + 0.02,
            "{name}: a walking illager's legs should splay along Z"
        );
    }
}

#[test]
fn riding_illagers_use_vanilla_seated_pose() {
    use std::f32::consts::PI;

    // Vanilla `IllagerRenderer.extractRenderState`: `state.isRiding = entity.isPassenger()`.
    // `IllagerModel.setupAnim` then uses the fixed seated pose instead of the walk swing.
    let riding =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager)
            .with_is_riding(true)
            .with_walk_animation(0.0, 1.0);
    let mut model = IllagerModel::new(&riding, IllagerModelFamily::Pillager);
    model.prepare(&riding);

    let right_arm = model.root_mut().child_mut("right_arm").pose.rotation;
    assert!((right_arm[0] - (-PI / 5.0)).abs() < 1e-6);
    assert!(right_arm[1].abs() < 1e-6 && right_arm[2].abs() < 1e-6);
    let left_arm = model.root_mut().child_mut("left_arm").pose.rotation;
    assert!((left_arm[0] - (-PI / 5.0)).abs() < 1e-6);
    assert!(left_arm[1].abs() < 1e-6 && left_arm[2].abs() < 1e-6);

    let right_leg = model.root_mut().child_mut("right_leg").pose.rotation;
    assert!((right_leg[0] - -1.4137167).abs() < 1e-6);
    assert!((right_leg[1] - PI / 10.0).abs() < 1e-6);
    assert!((right_leg[2] - 0.07853982).abs() < 1e-6);
    let left_leg = model.root_mut().child_mut("left_leg").pose.rotation;
    assert!((left_leg[0] - -1.4137167).abs() < 1e-6);
    assert!((left_leg[1] - -PI / 10.0).abs() < 1e-6);
    assert!((left_leg[2] - -0.07853982).abs() < 1e-6);

    // The seated branch replaces walking legs, but vanilla still runs the arm-pose branch afterwards.
    let attacking =
        EntityModelInstance::illager(140, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Vindicator)
            .with_is_riding(true)
            .with_walk_animation(0.0, 1.0)
            .with_is_aggressive(true);
    let mut attacking_model = IllagerModel::new(&attacking, IllagerModelFamily::Vindicator);
    attacking_model.prepare(&attacking);
    let attacking_right_arm = attacking_model
        .root_mut()
        .child_mut("right_arm")
        .pose
        .rotation;
    assert!(
        (attacking_right_arm[0] - (-PI / 5.0)).abs() > 0.5,
        "ATTACKING runs after the seated arm preset"
    );
    let attacking_right_leg = attacking_model
        .root_mut()
        .child_mut("right_leg")
        .pose
        .rotation;
    assert!((attacking_right_leg[0] - -1.4137167).abs() < 1e-6);
    assert!((attacking_right_leg[1] - PI / 10.0).abs() < 1e-6);
    assert!((attacking_right_leg[2] - 0.07853982).abs() < 1e-6);
}

#[test]
fn pillager_swings_its_arms_when_walking() {
    // Vanilla `IllagerModel.setupAnim` swings the separate arms with the `HumanoidModel`
    // amplitude `cos(pos * 0.6662 [+ π]) * 2.0 * speed * 0.5` (right arm a half-cycle out of
    // phase). The pillager renders the uncrossed layout head/body/leg/leg/right_arm/left_arm
    // (192 verts, 8 cubes), so the two arm cubes occupy vertices [144, 192). A walking
    // pillager swings them; a standing one keeps them at rest.
    let base =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager);
    let rest = entity_model_mesh(&[base]);
    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_eq!(rest.vertices.len(), 192);
    assert_ne!(
        rest.vertices[144..192],
        walking.vertices[144..192],
        "the pillager swings its separate arms when walking"
    );
    let still = entity_model_mesh(&[base.with_walk_animation(2.5, 0.0)]);
    assert_eq!(
        rest.vertices[144..192],
        still.vertices[144..192],
        "a standing pillager's arms are inert"
    );
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
            .with_head_look(yaw, pitch)
            .with_main_hand_holds_crossbow(true);
    let mut model = IllagerModel::new(&holding, IllagerModelFamily::Pillager);
    model.prepare(&holding);

    let right = model.root_mut().child_mut("right_arm");
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
    let left = model.root_mut().child_mut("left_arm");
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
    let charging = holding.with_is_charging_crossbow(true);
    let mut charging_model = IllagerModel::new(&charging, IllagerModelFamily::Pillager);
    charging_model.prepare(&charging);
    assert!(
        (charging_model
            .root_mut()
            .child_mut("right_arm")
            .pose
            .rotation[0]
            - (-0.97079635))
            .abs()
            < 1e-6,
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
                .with_is_charging_crossbow(true)
                .with_crossbow_charge_ticks(ticks);
        let mut model = IllagerModel::new(&instance, IllagerModelFamily::Pillager);
        model.prepare(&instance);
        let right = model.root_mut().child_mut("right_arm").pose;
        let left = model.root_mut().child_mut("left_arm").pose;
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
fn pillager_crossbow_hold_reposes_only_the_arms() {
    // End-to-end through the dispatch: a pillager holding its crossbow re-poses the two arm cubes
    // ([144, 192), per `pillager_swings_its_arms_when_walking`) while the head/body/legs ([0, 144))
    // stay byte-identical, and charging re-poses only the arms too (the distinct CROSSBOW_CHARGE draw).
    let base =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager)
            .with_head_look(15.0, -10.0);
    let rest = entity_model_mesh(&[base]);
    let holding = entity_model_mesh(&[base.with_main_hand_holds_crossbow(true)]);
    assert_eq!(rest.vertices.len(), 192);
    assert_eq!(
        rest.vertices[0..144],
        holding.vertices[0..144],
        "the head, body and legs do not hold the crossbow"
    );
    assert_ne!(
        rest.vertices[144..192],
        holding.vertices[144..192],
        "both arms level the crossbow"
    );
    let charging = entity_model_mesh(&[base
        .with_main_hand_holds_crossbow(true)
        .with_is_charging_crossbow(true)
        .with_crossbow_charge_ticks(12.0)]);
    assert_eq!(
        rest.vertices[0..144],
        charging.vertices[0..144],
        "the head, body and legs do not move during the crossbow draw"
    );
    assert_ne!(
        rest.vertices[144..192],
        charging.vertices[144..192],
        "a charging pillager pulls the crossbow back with its arms"
    );
    assert_ne!(
        holding.vertices[144..192],
        charging.vertices[144..192],
        "the charge draw pose differs from the level hold pose"
    );
}

#[test]
fn illager_textured_parts_match_vanilla_body_layer_uv_sources() {
    assert_eq!(MODEL_LAYER_EVOKER, "minecraft:evoker#main");
    assert_eq!(MODEL_LAYER_ILLUSIONER, "minecraft:illusioner#main");
    assert_eq!(MODEL_LAYER_PILLAGER, "minecraft:pillager#main");
    assert_eq!(MODEL_LAYER_VINDICATOR, "minecraft:vindicator#main");

    // The unified cubes carry the textured UV sources (`uv_size`/`texOffs`/`mirror`) merged into the
    // colored geometry. Vanilla `IllagerModel.createBodyLayer` UVs (64x64).
    assert_eq!(ILLAGER_HEAD[0].tex, [0.0, 0.0]); // head texOffs(0, 0)
    assert_eq!(ILLAGER_HEAD[0].uv_size, [8.0, 10.0, 8.0]);
    assert_eq!(ILLAGER_NOSE[0].tex, [24.0, 0.0]); // nose texOffs(24, 0)
    assert_eq!(ILLAGER_BODY[0].tex, [16.0, 20.0]); // body texOffs(16, 20)
    assert_eq!(ILLAGER_BODY[1].tex, [0.0, 38.0]); // robe overlay texOffs(0, 38)
    assert_eq!(ILLAGER_BODY[1].uv_size, [8.0, 20.0, 6.0]); // base box, not the 0.5 inflation
    assert_eq!(ILLAGER_CROSSED_ARMS[0].tex, [44.0, 22.0]); // arms texOffs(44, 22)
    assert_eq!(ILLAGER_CROSSED_ARMS[1].tex, [40.0, 38.0]); // arms texOffs(40, 38)
    assert_eq!(ILLAGER_LEFT_SHOULDER[0].tex, [44.0, 22.0]); // left shoulder mirror
    assert!(ILLAGER_LEFT_SHOULDER[0].mirror);
    assert_eq!(ILLAGER_RIGHT_LEG[0].tex, [0.0, 22.0]); // right leg texOffs(0, 22)
    assert!(!ILLAGER_RIGHT_LEG[0].mirror);
    assert_eq!(ILLAGER_LEFT_LEG[0].tex, [0.0, 22.0]); // left leg mirror
    assert!(ILLAGER_LEFT_LEG[0].mirror);

    // The illusioner head hat: `texOffs(32, 0)` over the base 8x12x8 box, inflated geometry.
    assert_eq!(ILLAGER_HAT[0].tex, [32.0, 0.0]);
    assert_eq!(ILLAGER_HAT[0].uv_size, [8.0, 12.0, 8.0]);
    assert_eq!(ILLAGER_HAT[0].size, [8.9, 12.9, 8.9]);

    // The two separate swinging arms (pillager / spellcasting) at `texOffs(40, 46)`.
    assert_eq!(ILLAGER_RIGHT_ARM[0].tex, [40.0, 46.0]);
    assert!(!ILLAGER_RIGHT_ARM[0].mirror);
    assert_eq!(ILLAGER_LEFT_ARM[0].tex, [40.0, 46.0]);
    assert!(ILLAGER_LEFT_ARM[0].mirror);
}

#[test]
fn illager_textured_layer_passes_match_vanilla_renderer() {
    let cases = [
        (
            IllagerModelFamily::Evoker,
            "minecraft:evoker#main",
            EVOKER_TEXTURE_REF,
        ),
        (
            IllagerModelFamily::Illusioner,
            "minecraft:illusioner#main",
            ILLUSIONER_TEXTURE_REF,
        ),
        (
            IllagerModelFamily::Pillager,
            "minecraft:pillager#main",
            PILLAGER_TEXTURE_REF,
        ),
        (
            IllagerModelFamily::Vindicator,
            "minecraft:vindicator#main",
            VINDICATOR_TEXTURE_REF,
        ),
    ];
    for (family, model_layer, texture) in cases {
        let passes = illager_textured_layer_passes(family);
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0].kind, EntityModelLayerKind::IllagerBase);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].model_layer, model_layer);
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert!(entity_model_texture_refs().contains(&texture));
    }
    assert_eq!(
        illager_entity_texture_refs(),
        &[
            EVOKER_TEXTURE_REF,
            ILLUSIONER_TEXTURE_REF,
            PILLAGER_TEXTURE_REF,
            VINDICATOR_TEXTURE_REF,
        ]
    );
}

#[test]
fn illager_textured_mesh_matches_colored_geometry_and_swings() {
    let (atlas, _) = build_entity_model_texture_atlas(&illager_texture_images()).unwrap();
    let families = [
        IllagerModelFamily::Evoker,
        IllagerModelFamily::Illusioner,
        IllagerModelFamily::Pillager,
        IllagerModelFamily::Vindicator,
    ];
    for family in families {
        let instances = [EntityModelInstance::illager(
            45,
            [0.0, 64.0, 0.0],
            0.0,
            family,
        )];
        let colored = entity_model_mesh(&instances);
        let textured = entity_model_textured_mesh(&instances, &atlas);
        // The textured illager shares the colored geometry exactly (VILLAGER_LIKE_SCALE root).
        assert_eq!(textured.cutout_faces, colored.opaque_faces, "{family:?}");
        assert_eq!(
            textured.vertices.len(),
            colored.vertices.len(),
            "{family:?}"
        );
        assert!(textured
            .vertices
            .iter()
            .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
        let (cmin, cmax) = mesh_extents(&colored);
        let (tmin, tmax) = textured_mesh_extents(&textured);
        assert_close3(tmin, cmin);
        assert_close3(tmax, cmax);

        // Walking re-poses the legs on both render paths.
        let walking = [instances[0].with_walk_animation(0.0, 1.0)];
        let textured_walk = entity_model_textured_mesh(&walking, &atlas);
        assert_ne!(textured.vertices, textured_walk.vertices, "{family:?} legs");
    }

    // The pillager swings its visible separate arms; the crossed-arm families hold still.
    let pillager =
        EntityModelInstance::illager(103, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Pillager);
    let pillager_rest = entity_model_textured_mesh(&[pillager], &atlas);
    let pillager_walk =
        entity_model_textured_mesh(&[pillager.with_walk_animation(0.0, 1.0)], &atlas);
    assert_eq!(pillager_rest.vertices.len(), 192);
    assert_ne!(
        pillager_rest.vertices[144..192],
        pillager_walk.vertices[144..192],
        "the textured pillager swings its separate arms"
    );

    let evoker =
        EntityModelInstance::illager(46, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Evoker);
    let evoker_rest = entity_model_textured_mesh(&[evoker], &atlas);
    let evoker_walk = entity_model_textured_mesh(&[evoker.with_walk_animation(0.0, 1.0)], &atlas);
    assert_eq!(evoker_rest.vertices.len(), 216);
    assert_eq!(
        evoker_rest.vertices[96..168],
        evoker_walk.vertices[96..168],
        "the textured crossed arms part stays still when walking"
    );

    // The textured spellcasting evoker swaps to the uncrossed layout (216 → 192 verts), matching
    // the colored path; an idle illusioner keeps its hat (216 verts when casting).
    let evoker_cast = entity_model_textured_mesh(&[evoker.with_illager_spellcasting(true)], &atlas);
    assert_eq!(evoker_cast.cutout_faces, 48);
    assert_eq!(evoker_cast.vertices.len(), 192);
    assert_ne!(evoker_rest.vertices, evoker_cast.vertices);
    let illusioner =
        EntityModelInstance::illager(68, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Illusioner);
    let illusioner_cast =
        entity_model_textured_mesh(&[illusioner.with_illager_spellcasting(true)], &atlas);
    assert_eq!(illusioner_cast.vertices.len(), 216);
}

fn illager_texture_images() -> Vec<EntityModelTextureImage> {
    illager_entity_texture_refs()
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
    let right = illager_spellcast_arm_pose(ILLAGER_RIGHT_ARM_POSE, 0.0, true);
    assert_eq!(right.offset, [-5.0, 2.0, 0.0]);
    assert!((right.rotation[0] - 0.25).abs() < 1.0e-6); // cos(0) * 0.25
    assert_eq!(right.rotation[1], 0.0);
    assert!((right.rotation[2] - PI * 3.0 / 4.0).abs() < 1.0e-6);

    let left = illager_spellcast_arm_pose(ILLAGER_LEFT_ARM_POSE, 0.0, false);
    assert_eq!(left.offset, [5.0, 2.0, 0.0]);
    assert!((left.rotation[2] - (-PI * 3.0 / 4.0)).abs() < 1.0e-6);

    // The arm pitch animates with `ageInTicks`.
    let aged = illager_spellcast_arm_pose(ILLAGER_RIGHT_ARM_POSE, 5.0, true);
    assert!((aged.rotation[0] - (5.0_f32 * 0.6662).cos() * 0.25).abs() < 1.0e-6);
}

#[test]
fn spellcasting_evoker_raises_separate_arms() {
    // A casting evoker swaps the crossed `arms` part (idle: 9 cubes / 216 verts) for the
    // uncrossed layout with two separate raised arms (8 cubes / 192 verts, like a pillager),
    // so the mesh both shrinks by one cube and re-poses.
    let base = EntityModelInstance::illager(46, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Evoker);
    let idle = entity_model_mesh(&[base]);
    let casting = entity_model_mesh(&[base.with_illager_spellcasting(true)]);
    assert_eq!(idle.vertices.len(), 216);
    assert_eq!(
        casting.vertices.len(),
        192,
        "the crossed arms part is hidden and two separate arms render"
    );
    assert_ne!(idle.vertices, casting.vertices);
}

#[test]
fn spellcasting_illusioner_keeps_its_hat() {
    // A casting illusioner swaps to the uncrossed layout (idle 10 cubes / 240 verts → 9 cubes /
    // 216 verts) but keeps its hat — so it stays one cube larger than a casting evoker (192).
    let base =
        EntityModelInstance::illager(68, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Illusioner);
    let idle = entity_model_mesh(&[base]);
    let casting = entity_model_mesh(&[base.with_illager_spellcasting(true)]);
    assert_eq!(idle.vertices.len(), 240);
    assert_eq!(casting.vertices.len(), 216, "the hatted head is kept");
    assert_ne!(idle.vertices, casting.vertices);
}

#[test]
fn non_spellcaster_illagers_ignore_the_spellcasting_flag() {
    // Vindicator and pillager are not spellcasters, so the flag must not change their pose.
    for family in [IllagerModelFamily::Vindicator, IllagerModelFamily::Pillager] {
        let base = EntityModelInstance::illager(140, [0.0, 64.0, 0.0], 0.0, family);
        let idle = entity_model_mesh(&[base]);
        let flagged = entity_model_mesh(&[base.with_illager_spellcasting(true)]);
        assert_eq!(
            idle.vertices, flagged.vertices,
            "the spellcasting flag is gated to evoker/illusioner"
        );
    }
}

#[test]
fn illager_celebrate_arm_pose_matches_vanilla() {
    use std::f32::consts::PI;

    // Vanilla `IllagerModel.setupAnim` CELEBRATING: both arms hold the bind offset (`x = ∓5`, `z = 0`),
    // bob `xRot = cos(ageInTicks · 0.6662) · 0.05`, raise asymmetrically (right `zRot = 2.670354`, left
    // `zRot = -3π/4`), with `yRot = 0`.
    let right = illager_celebrate_arm_pose(ILLAGER_RIGHT_ARM_POSE, 0.0, true);
    assert_eq!(right.offset, [-5.0, 2.0, 0.0]);
    assert!((right.rotation[0] - 0.05).abs() < 1.0e-6); // cos(0) * 0.05
    assert_eq!(right.rotation[1], 0.0);
    assert!((right.rotation[2] - 2.670354).abs() < 1.0e-6);

    let left = illager_celebrate_arm_pose(ILLAGER_LEFT_ARM_POSE, 0.0, false);
    assert_eq!(left.offset, [5.0, 2.0, 0.0]);
    assert!((left.rotation[2] - (-PI * 3.0 / 4.0)).abs() < 1.0e-6);

    // The arm pitch bobs with `ageInTicks`.
    let aged = illager_celebrate_arm_pose(ILLAGER_RIGHT_ARM_POSE, 5.0, true);
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
            .with_head_look(yaw, pitch)
            .with_is_aggressive(true);
    let mut model = IllagerModel::new(&drawing, IllagerModelFamily::Illusioner);
    model.prepare(&drawing);

    let right = model.root_mut().child_mut("right_arm");
    assert!(
        (right.pose.rotation[0] - (-FRAC_PI_2 + pitch_rad)).abs() < 1.0e-6,
        "right arm aims down the head pitch: {}",
        right.pose.rotation[0]
    );
    assert!((right.pose.rotation[1] - (-0.1 + yaw_rad)).abs() < 1.0e-6);
    let left = model.root_mut().child_mut("left_arm");
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
        let celebrating = entity_model_mesh(&[base.with_illager_celebrating(true)]);
        assert_eq!(idle.vertices.len(), 216);
        assert_eq!(
            celebrating.vertices.len(),
            192,
            "the crossed arms hide and two separate arms dance"
        );
        assert_ne!(idle.vertices, celebrating.vertices);

        let celebrating_instance = base.with_illager_celebrating(true);
        let mut model = IllagerModel::new(&celebrating_instance, family);
        model.prepare(&celebrating_instance);
        assert!(
            (model.root_mut().child_mut("right_arm").pose.rotation[2] - 2.670354).abs() < 1.0e-6,
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
        entity_model_mesh(&[pillager.with_illager_celebrating(true)]).vertices,
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
            .with_illager_celebrating(true)
            .with_is_aggressive(true);
    let mut model = IllagerModel::new(&attacking, IllagerModelFamily::Vindicator);
    model.prepare(&attacking);

    let right = model.root_mut().child_mut("right_arm").pose.rotation;
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
    let left = model.root_mut().child_mut("left_arm").pose.rotation;
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
    let mid = attacking.with_attack_anim(0.5);
    let mut swinging = IllagerModel::new(&mid, IllagerModelFamily::Vindicator);
    swinging.prepare(&mid);
    let chopping = swinging.root_mut().child_mut("right_arm").pose.rotation[0];
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
fn empty_hand_attacking_vindicator_uses_zombie_arms() {
    use std::f32::consts::PI;

    // Vanilla `IllagerModel.setupAnim` ATTACKING branch: an empty main hand uses
    // `AnimationUtils.animateZombieArms(left, right, true, state)` instead of the armed
    // `swingWeaponDown` chop.
    let attacking =
        EntityModelInstance::illager(140, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Vindicator)
            .with_is_aggressive(true)
            .with_illager_main_hand_empty(true);
    let mut model = IllagerModel::new(&attacking, IllagerModelFamily::Vindicator);
    model.prepare(&attacking);

    let right = model.root_mut().child_mut("right_arm").pose.rotation;
    assert!(
        (right[0] - (-PI / 1.5)).abs() < 1.0e-6,
        "empty-hand attacking right arm reaches forward: {}",
        right[0]
    );
    assert!((right[1] - -0.1).abs() < 1.0e-6);
    assert!((right[2] - 0.1).abs() < 1.0e-6);
    let left = model.root_mut().child_mut("left_arm").pose.rotation;
    assert!((left[0] - (-PI / 1.5)).abs() < 1.0e-6);
    assert!((left[1] - 0.1).abs() < 1.0e-6);
    assert!((left[2] - -0.1).abs() < 1.0e-6);

    let mid = attacking.with_attack_anim(0.5);
    let mut swinging = IllagerModel::new(&mid, IllagerModelFamily::Vindicator);
    swinging.prepare(&mid);
    let swinging_right = swinging.root_mut().child_mut("right_arm").pose.rotation;
    let attack_y = (0.5_f32 * PI).sin();
    let attack_x = ((1.0 - 0.5 * 0.5) * PI).sin();
    assert!(
        (swinging_right[0] - (-PI / 1.5 + attack_y * 1.2 - attack_x * 0.4)).abs() < 1.0e-6,
        "empty-hand attack uses zombie swing xRot: {}",
        swinging_right[0]
    );
    assert!((swinging_right[1] - -(0.1 - attack_y * 0.6)).abs() < 1.0e-6);
}

#[test]
fn crossed_arm_illagers_keep_their_arms_still_when_walking() {
    // The evoker/vindicator/illusioner show the static crossed `arms` part: vanilla swings
    // the *invisible* separate arms, so the visible crossed part holds still. The evoker
    // layout is head/body/crossed_arm/leg/leg (216 verts, 9 cubes): the crossed arm part
    // (3 cubes) occupies vertices [96, 168) and the two legs [168, 216). A walking evoker
    // swings only its legs.
    let base = EntityModelInstance::illager(46, [0.0, 64.0, 0.0], 0.0, IllagerModelFamily::Evoker);
    let rest = entity_model_mesh(&[base]);
    let walking = entity_model_mesh(&[base.with_walk_animation(0.0, 1.0)]);
    assert_eq!(rest.vertices.len(), 216);
    assert_eq!(
        rest.vertices[96..168],
        walking.vertices[96..168],
        "the crossed arms part stays still when walking"
    );
    assert_ne!(
        rest.vertices[168..216],
        walking.vertices[168..216],
        "the legs still swing when walking"
    );
}
