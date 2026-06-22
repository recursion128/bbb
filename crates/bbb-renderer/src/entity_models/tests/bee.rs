use super::*;

use std::f32::consts::PI;

#[test]
fn bee_geometry_matches_vanilla_26_1_body_layers() {
    // Adult `AdultBeeModel.createBodyLayer` (atlas 64×64): the empty bone parents a 7×7×10 body.
    assert_eq!(BEE_BODY[0].min, [-3.5, -4.0, -5.0]);
    assert_eq!(BEE_BODY[0].size, [7.0, 7.0, 10.0]);
    assert_eq!(BEE_BONE_POSE.offset, [0.0, 19.0, 0.0]);
    // The stinger is a zero-thickness plane, the legs zero-depth planes.
    assert_eq!(BEE_STINGER[0].size, [0.0, 1.0, 2.0]);
    assert_eq!(BEE_FRONT_LEGS[0].size, [7.0, 2.0, 0.0]);
    // The wings carry the vanilla `CubeDeformation(0.001)`.
    assert_eq!(BEE_RIGHT_WING[0].min, [-9.001, -0.001, -0.001]);
    assert_eq!(BEE_RIGHT_WING[0].size, [9.002, 0.002, 6.002]);
    assert_eq!(BEE_RIGHT_WING_POSE.rotation, [0.0, -0.2618, 0.0]);
    assert_eq!(BEE_LEFT_WING_POSE.rotation, [0.0, 0.2618, 0.0]);

    // Baby `BabyBeeModel.createBodyLayer` (atlas 32×32): the bone itself carries two cubes, there
    // are no antennae, and the wings sit at a `0.2182` pitch.
    assert_eq!(BEE_BABY_BONE.len(), 2);
    assert_eq!(BEE_BABY_BODY[0].size, [4.0, 4.0, 5.0]);
    assert_eq!(BEE_BABY_BONE_POSE.offset, [0.0, 19.6667, -1.8567]);
    assert_eq!(BEE_BABY_RIGHT_WING_POSE.rotation, [0.2182, 0.3491, 0.0]);
    assert_eq!(BEE_BABY_LEFT_WING_POSE.rotation, [0.2182, -0.3491, 0.0]);
}

#[test]
fn bee_animation_helpers_match_vanilla_setup_anim() {
    // `rightWing.zRot = cos(ageInTicks · 120.32113°) · π · 0.15` (peaks at `t=0`).
    assert!((bee_wing_z_rot(0.0) - PI * 0.15).abs() < 1.0e-6);
    // `bobUpAndDown` reads `speed = cos(ageInTicks · 0.18)` (1 at `t=0`).
    assert!((bee_bob_speed(0.0) - 1.0).abs() < 1.0e-6);
    // `bone.xRot = 0.1 + speed · π · 0.025`.
    assert!((bee_bone_x_rot(0.0) - (0.1 + PI * 0.025)).abs() < 1.0e-6);
    // `bone.y -= cos(ageInTicks · 0.18) · 0.9`.
    assert!((bee_bone_y_delta(0.0) - -0.9).abs() < 1.0e-6);
    // `frontLeg.xRot = -speed · π · 0.1 + π/8`, `backLeg.xRot = -speed · π · 0.05 + π/4`.
    assert!((bee_front_leg_x_rot(0.0) - (-PI * 0.1 + PI / 8.0)).abs() < 1.0e-6);
    assert!((bee_back_leg_x_rot(0.0) - (-PI * 0.05 + PI / 4.0)).abs() < 1.0e-6);
    // Adult-only antenna bob: `antenna.xRot = speed · π · 0.03`.
    assert!((bee_antenna_x_rot(0.0) - PI * 0.03).abs() < 1.0e-6);
    // The airborne middle leg holds `π/4`.
    assert!((BEE_MID_LEG_FLYING_X_ROT - PI / 4.0).abs() < 1.0e-6);
}

#[test]
fn bee_mesh_uses_vanilla_body_layer_geometry() {
    // Adult: body, stinger, two antennae, two wings, three leg planes → 9 cubes / 54 faces / 216
    // vertices.
    let adult = entity_model_mesh(&[EntityModelInstance::bee(930, [0.0, 64.0, 0.0], 0.0, false)]);
    assert_eq!(adult.opaque_faces, 54);
    assert_eq!(adult.vertices.len(), 216);
    assert!(adult
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(BEE_YELLOW, 1.0)));

    // Baby: bone (two cubes), body, stinger, two wings, three leg planes → also 9 cubes.
    let baby = entity_model_mesh(&[EntityModelInstance::bee(931, [0.0, 64.0, 0.0], 0.0, true)]);
    assert_eq!(baby.opaque_faces, 54);
    assert_eq!(baby.vertices.len(), 216);
}

#[test]
fn bee_flaps_while_airborne_and_rests_on_ground() {
    // Airborne (the constructor default, `on_ground = false`): the wing flap and bob re-pose the
    // mesh as the age advances.
    let flying = EntityModelInstance::bee(932, [0.0, 64.0, 0.0], 0.0, false);
    let early = entity_model_mesh(&[flying]);
    let later = entity_model_mesh(&[flying.with_age_in_ticks(3.0)]);
    assert_eq!(early.vertices.len(), later.vertices.len());
    assert_ne!(early.vertices, later.vertices, "the wings flap with age");

    // On the ground the model holds its bind pose, so it is static across ages and differs from
    // the airborne flap.
    let grounded = flying.with_on_ground(true);
    let grounded_early = entity_model_mesh(&[grounded]);
    let grounded_later = entity_model_mesh(&[grounded.with_age_in_ticks(3.0)]);
    assert_eq!(
        grounded_early.vertices, grounded_later.vertices,
        "a grounded bee rests at its bind pose"
    );
    assert_ne!(early.vertices, grounded_early.vertices);
}

#[test]
fn bee_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Bee { baby: false }.model_key(), "bee");
    assert_eq!(EntityModelKind::Bee { baby: true }.model_key(), "bee_baby");
    assert_eq!(
        EntityModelKind::Bee { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/bee/bee.png",
            size: [64, 64],
        })
    );
    assert_eq!(
        EntityModelKind::Bee { baby: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/bee/bee_baby.png",
            size: [32, 32],
        })
    );
}
