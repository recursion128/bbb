use super::*;

#[test]
fn strider_adult_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AdultStriderModel.createBodyLayer` (atlas 64×128).
    assert_eq!(
        STRIDER_BODY[0],
        ModelCubeDesc {
            min: [-8.0, -6.0, -8.0],
            size: [16.0, 14.0, 16.0],
            color: STRIDER_MAROON,
        }
    );
    assert_eq!(STRIDER_RIGHT_LEG[0].min, [-2.0, 0.0, -2.0]);
    assert_eq!(STRIDER_RIGHT_LEG[0].size, [4.0, 16.0, 4.0]);
    assert_eq!(STRIDER_LEFT_LEG[0].size, [4.0, 16.0, 4.0]);

    // Bristles are zero-thickness 12×0×16 planes; the right one's box starts at -12 (mirrored).
    assert_eq!(STRIDER_RIGHT_BRISTLE[0].min, [-12.0, 0.0, 0.0]);
    assert_eq!(STRIDER_RIGHT_BRISTLE[0].size, [12.0, 0.0, 16.0]);
    assert_eq!(STRIDER_LEFT_BRISTLE[0].min, [0.0, 0.0, 0.0]);
    assert_eq!(STRIDER_LEFT_BRISTLE[0].size, [12.0, 0.0, 16.0]);

    // Offsets and bristle rest rolls.
    assert_eq!(STRIDER_BODY_BASE_Y, 2.0);
    assert_eq!(STRIDER_LEG_BASE_Y, 8.0);
    assert_eq!(STRIDER_RIGHT_LEG_X, -4.0);
    assert_eq!(STRIDER_LEFT_LEG_X, 4.0);
    assert_eq!(STRIDER_RIGHT_TOP_BRISTLE_POSE.offset, [-8.0, -5.0, -8.0]);
    assert_eq!(STRIDER_RIGHT_MIDDLE_BRISTLE_POSE.offset, [-8.0, -1.0, -8.0]);
    assert_eq!(STRIDER_RIGHT_BOTTOM_BRISTLE_POSE.offset, [-8.0, 4.0, -8.0]);
    assert_eq!(STRIDER_LEFT_TOP_BRISTLE_POSE.offset, [8.0, -6.0, -8.0]);
    assert_eq!(STRIDER_LEFT_MIDDLE_BRISTLE_POSE.offset, [8.0, -2.0, -8.0]);
    assert_eq!(STRIDER_LEFT_BOTTOM_BRISTLE_POSE.offset, [8.0, 3.0, -8.0]);
    assert!((STRIDER_RIGHT_TOP_BRISTLE_POSE.rotation[2] - -0.872_664_63).abs() < 1.0e-6);
    assert!((STRIDER_RIGHT_MIDDLE_BRISTLE_POSE.rotation[2] - -1.134_464).abs() < 1.0e-6);
    assert!((STRIDER_RIGHT_BOTTOM_BRISTLE_POSE.rotation[2] - -1.221_730_5).abs() < 1.0e-6);
    assert!((STRIDER_LEFT_TOP_BRISTLE_POSE.rotation[2] - 0.872_664_63).abs() < 1.0e-6);
    assert!((STRIDER_LEFT_MIDDLE_BRISTLE_POSE.rotation[2] - 1.134_464).abs() < 1.0e-6);
    assert!((STRIDER_LEFT_BOTTOM_BRISTLE_POSE.rotation[2] - 1.221_730_5).abs() < 1.0e-6);
}

#[test]
fn strider_baby_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `BabyStriderModel.createBodyLayer` (atlas 32×32).
    assert_eq!(STRIDER_BABY_BODY[0].min, [-3.5, -3.75, -4.0]);
    assert_eq!(STRIDER_BABY_BODY[0].size, [7.0, 7.0, 8.0]);
    assert_eq!(STRIDER_BABY_RIGHT_LEG[0].size, [2.0, 4.0, 2.0]);
    assert_eq!(STRIDER_BABY_LEFT_LEG[0].size, [2.0, 4.0, 2.0]);

    // Baby bristles are zero-thickness 7×3×0 planes that flap on `xRot` (no rest roll).
    assert_eq!(STRIDER_BABY_BRISTLE[0].min, [-3.5, -2.5, 0.0]);
    assert_eq!(STRIDER_BABY_BRISTLE[0].size, [7.0, 3.0, 0.0]);

    assert_eq!(STRIDER_BABY_BODY_BASE_Y, 17.25);
    assert_eq!(STRIDER_BABY_LEG_BASE_Y, 20.0);
    assert_eq!(STRIDER_BABY_RIGHT_LEG_X, -1.5);
    assert_eq!(STRIDER_BABY_LEFT_LEG_X, 1.5);
    assert_eq!(STRIDER_BABY_FRONT_BRISTLE_POSE.offset, [0.0, -4.25, -2.0]);
    assert_eq!(STRIDER_BABY_MIDDLE_BRISTLE_POSE.offset, [0.0, -4.25, 0.0]);
    assert_eq!(STRIDER_BABY_BACK_BRISTLE_POSE.offset, [0.0, -4.25, 2.0]);
}

#[test]
fn strider_setup_anim_curves_match_vanilla() {
    // `animationSpeed = min(walkAnimationSpeed, 0.25)`.
    assert!((strider_animation_speed(0.1) - 0.1).abs() < 1.0e-6);
    assert!(
        (strider_animation_speed(0.8) - 0.25).abs() < 1.0e-6,
        "clamped"
    );

    let (pos, speed, age) = (3.0_f32, 0.2_f32, 11.0_f32);

    // `body.zRot = 0.1·sin(pos·1.5)·4·speed`.
    let expected_body_z = 0.1 * (pos * 1.5).sin() * 4.0 * speed;
    assert!((strider_body_z_rot(pos, speed) - expected_body_z).abs() < 1.0e-6);

    // Leg swing/roll: left uses phase 0, right uses phase π.
    let expected_left_x = (pos * 0.75).sin() * 2.0 * speed;
    let expected_right_x = (pos * 0.75 + std::f32::consts::PI).sin() * 2.0 * speed;
    assert!((strider_leg_x_rot(pos, speed, false) - expected_left_x).abs() < 1.0e-6);
    assert!((strider_leg_x_rot(pos, speed, true) - expected_right_x).abs() < 1.0e-6);
    let expected_left_z = (std::f32::consts::PI / 18.0) * (pos * 0.75).cos() * speed;
    assert!((strider_leg_z_rot(pos, speed, false) - expected_left_z).abs() < 1.0e-6);

    // Body bob (adult mul 2, baby mul 1).
    let expected_adult_body_y = 2.0 - 2.0 * (pos * 1.5).cos() * 2.0 * speed;
    assert!((strider_body_y(2.0, 2.0, pos, speed) - expected_adult_body_y).abs() < 1.0e-6);
    let expected_baby_body_y = 17.25 - 1.0 * (pos * 1.5).cos() * 2.0 * speed;
    assert!((strider_body_y(17.25, 1.0, pos, speed) - expected_baby_body_y).abs() < 1.0e-6);

    // Leg lift: right uses phase 0, left uses phase π (opposite of the swing).
    let expected_right_y = 8.0 + 2.0 * (pos * 0.75).sin() * 2.0 * speed;
    let expected_left_y = 8.0 + 2.0 * (pos * 0.75 + std::f32::consts::PI).sin() * 2.0 * speed;
    assert!((strider_leg_y(8.0, pos, speed, true) - expected_right_y).abs() < 1.0e-6);
    assert!((strider_leg_y(8.0, pos, speed, false) - expected_left_y).abs() < 1.0e-6);

    // Bristle flow + per-bristle ripple.
    let flow = (pos * 1.5 + std::f32::consts::PI).cos() * speed;
    assert!((strider_bristle_flow(pos, speed) - flow).abs() < 1.0e-6);
    assert!(
        (strider_bristle_top_flow(flow, age) - (flow * 0.6 + 0.1 * (age * 0.4).sin())).abs()
            < 1.0e-6
    );
    assert!(
        (strider_bristle_middle_flow(flow, age) - (flow * 1.2 + 0.1 * (age * 0.2).sin())).abs()
            < 1.0e-6
    );
    assert!(
        (strider_bristle_bottom_flow(flow, age) - (flow * 1.3 + 0.05 * (age * -0.4).sin())).abs()
            < 1.0e-6
    );
}

#[test]
fn strider_adult_mesh_uses_vanilla_body_layer_geometry() {
    // Nine cubes (body, two legs, six bristles) → 54 faces / 216 vertices.
    let strider = entity_model_mesh(&[EntityModelInstance::strider(
        700,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_eq!(strider.opaque_faces, 54);
    assert_eq!(strider.vertices.len(), 216);
    assert_eq!(strider.indices.len(), 324);
    assert!(strider
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(STRIDER_MAROON, 1.0)));
    assert!(strider
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(STRIDER_LEG, 1.0)));
}

#[test]
fn strider_baby_mesh_uses_vanilla_body_layer_geometry() {
    // Six cubes (body, two legs, three bristles) → 36 faces / 144 vertices.
    let baby = entity_model_mesh(&[EntityModelInstance::strider(
        701,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    assert_eq!(baby.opaque_faces, 36);
    assert_eq!(baby.vertices.len(), 144);
    assert_eq!(baby.indices.len(), 216);

    // The baby is a different (smaller) model than the adult, not a scaled copy.
    let adult = entity_model_mesh(&[EntityModelInstance::strider(
        702,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert_ne!(baby.vertices.len(), adult.vertices.len());
}

#[test]
fn strider_body_tracks_look_angles() {
    // The body tracks the projected look yaw/pitch.
    let base = EntityModelInstance::strider(703, [0.0, 64.0, 0.0], 0.0, false);
    let forward = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(40.0, -25.0)]);
    assert_eq!(forward.vertices.len(), looking.vertices.len());
    assert_ne!(
        forward.vertices, looking.vertices,
        "the body tracks the look"
    );
}

#[test]
fn strider_walk_animates_legs_body_and_bristles() {
    // A standing strider differs from a walking one: the legs swing/lift, the body sways/bobs,
    // and the bristles flow.
    let base = EntityModelInstance::strider(704, [0.0, 64.0, 0.0], 0.0, false);
    let standing = entity_model_mesh(&[base]);
    let walking = entity_model_mesh(&[base.with_walk_animation(3.0, 0.2)]);
    assert_eq!(standing.vertices.len(), walking.vertices.len());
    assert_ne!(standing.vertices, walking.vertices, "the walk animates");

    // The bristles also ripple on `ageInTicks` even when standing still.
    let idle_rippled =
        entity_model_mesh(&[base.with_age_in_ticks(13.0).with_walk_animation(3.0, 0.2)]);
    assert_ne!(
        walking.vertices, idle_rippled.vertices,
        "bristles ripple with age"
    );
}

#[test]
fn strider_texture_refs_match_vanilla_renderer() {
    assert_eq!(
        EntityModelKind::Strider { baby: false }.model_key(),
        "strider"
    );
    assert_eq!(
        EntityModelKind::Strider { baby: true }.model_key(),
        "strider_baby"
    );
    assert_eq!(
        EntityModelKind::Strider { baby: false }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/strider/strider.png",
            size: [64, 128],
        })
    );
    assert_eq!(
        EntityModelKind::Strider { baby: true }.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/strider/strider_baby.png",
            size: [32, 32],
        })
    );
}
