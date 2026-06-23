use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn allay_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `AllayModel.createBodyLayer` (atlas 32×32). Head is a plain 5³ box. Each unified cube
    // carries both the colored geometry/tint and the textured `uv_size` / `texOffs` / `mirror`.
    assert_eq!(
        ALLAY_HEAD[0],
        ModelCube::new(
            [-2.5, -5.0, -2.5],
            [5.0, 5.0, 5.0],
            ALLAY_BLUE,
            [5.0, 5.0, 5.0],
            [0.0, 0.0],
            false,
        )
    );

    // Body: the plain `texOffs(0, 10)` 3×4×2 box plus the `texOffs(0, 16)` 3×5×2 box inset by
    // `CubeDeformation(-0.2)` (min +0.2, size -0.4); the inset cube keeps the 3×5×2 base uv_size.
    assert_eq!(ALLAY_BODY.len(), 2);
    assert_eq!(ALLAY_BODY[0].min, [-1.5, 0.0, -1.0]);
    assert_eq!(ALLAY_BODY[0].size, [3.0, 4.0, 2.0]);
    assert_eq!(ALLAY_BODY[0].tex, [0.0, 10.0]);
    assert_eq!(ALLAY_BODY[0].uv_size, [3.0, 4.0, 2.0]);
    assert_eq!(ALLAY_BODY[1].min, [-1.3, 0.2, -0.8]);
    assert_eq!(ALLAY_BODY[1].size, [2.6, 4.6, 1.6]);
    assert_eq!(ALLAY_BODY[1].tex, [0.0, 16.0]);
    assert_eq!(ALLAY_BODY[1].uv_size, [3.0, 5.0, 2.0]);

    // Arms: 1×4×2 boxes inset by `CubeDeformation(-0.01)` (min +0.01, size -0.02), uv_size keeps the
    // 1×4×2 base box. The right and left arms differ in their box origin (`-0.75` vs `-0.25`) and
    // `texOffs(23, 0)` / `texOffs(23, 6)`.
    assert_eq!(ALLAY_RIGHT_ARM[0].min, [-0.74, -0.49, -0.99]);
    assert_eq!(ALLAY_RIGHT_ARM[0].size, [0.98, 3.98, 1.98]);
    assert_eq!(ALLAY_RIGHT_ARM[0].uv_size, [1.0, 4.0, 2.0]);
    assert_eq!(ALLAY_RIGHT_ARM[0].tex, [23.0, 0.0]);
    assert_eq!(ALLAY_LEFT_ARM[0].min, [-0.24, -0.49, -0.99]);
    assert_eq!(ALLAY_LEFT_ARM[0].size, [0.98, 3.98, 1.98]);
    assert_eq!(ALLAY_LEFT_ARM[0].uv_size, [1.0, 4.0, 2.0]);
    assert_eq!(ALLAY_LEFT_ARM[0].tex, [23.0, 6.0]);

    // Wings: zero-thickness 0×5×8 planes whose box starts at y=1, both `texOffs(16, 14)` with NO
    // mirror (unlike the vex).
    assert_eq!(ALLAY_WING[0].min, [0.0, 1.0, 0.0]);
    assert_eq!(ALLAY_WING[0].size, [0.0, 5.0, 8.0]);
    assert_eq!(ALLAY_WING[0].uv_size, [0.0, 5.0, 8.0]);
    assert_eq!(ALLAY_WING[0].tex, [16.0, 14.0]);
    assert!(!ALLAY_WING[0].mirror);

    // Part offsets: the model root sits at +23.5, head at -3.99, body at -4.0, arms ±1.75,
    // wings ±0.5 and forward 0.6.
    assert_eq!(ALLAY_ROOT_BASE_Y, 23.5);
    assert_eq!(ALLAY_HEAD_POSE.offset, [0.0, -3.99, 0.0]);
    assert_eq!(ALLAY_BODY_POSE.offset, [0.0, -4.0, 0.0]);
    assert_eq!(ALLAY_RIGHT_ARM_POSE.offset, [-1.75, 0.5, 0.0]);
    assert_eq!(ALLAY_LEFT_ARM_POSE.offset, [1.75, 0.5, 0.0]);
    assert_eq!(ALLAY_RIGHT_WING_POSE.offset, [-0.5, 0.0, 0.6]);
    assert_eq!(ALLAY_LEFT_WING_POSE.offset, [0.5, 0.0, 0.6]);
}

#[test]
fn allay_setup_anim_constants_and_curves_match_vanilla() {
    // `flyingFactor = min(walkAnimationSpeed / 0.3, 1)`.
    assert!((allay_flying_factor(0.0) - 0.0).abs() < 1.0e-6);
    assert!((allay_flying_factor(0.15) - 0.5).abs() < 1.0e-6);
    assert!((allay_flying_factor(0.3) - 1.0).abs() < 1.0e-6);
    assert!(
        (allay_flying_factor(0.6) - 1.0).abs() < 1.0e-6,
        "clamped to 1"
    );

    // `flapAmount = cos(ageInTicks·20° + walkAnimationPos)·π·0.15 + walkAnimationSpeed`.
    assert!(
        (allay_wing_flap_amount(0.0, 0.0, 0.0) - std::f32::consts::PI * 0.15).abs() < 1.0e-6,
        "at age 0 the flap is the peak amplitude"
    );
    let (age, pos, speed) = (9.0_f32, 0.3_f32, 0.1_f32);
    let expected_flap =
        (age * 20.0_f32.to_radians() + pos).cos() * std::f32::consts::PI * 0.15 + speed;
    assert!((allay_wing_flap_amount(age, pos, speed) - expected_flap).abs() < 1.0e-6);

    // `wing.xRot = 0.43633232·(1 - flyingFactor)`, `body.xRot = flyingFactor·π/4`.
    assert!((allay_wing_rest_x_rot(0.0) - 0.436_332_32).abs() < 1.0e-6);
    assert!(
        (allay_wing_rest_x_rot(0.3) - 0.0).abs() < 1.0e-6,
        "flat while flying"
    );
    assert!((allay_body_x_rot(0.0) - 0.0).abs() < 1.0e-6);
    assert!((allay_body_x_rot(0.3) - std::f32::consts::FRAC_PI_4).abs() < 1.0e-6);

    // `root.y = 23.5 + cos(ageInTicks·9°)·0.25·(1 - flyingFactor)`; idle peak at age 0 is 23.75,
    // and the bob vanishes once flying.
    assert!((allay_root_y(0.0, 0.0) - 23.75).abs() < 1.0e-6);
    assert!((allay_root_y(0.0, 0.3) - 23.5).abs() < 1.0e-6);
    let expected_root = 23.5 + (age * 9.0_f32.to_radians()).cos() * 0.25;
    assert!((allay_root_y(age, 0.0) - expected_root).abs() < 1.0e-6);

    // `armIdleBobAmount = 0.43633232 - cos(idleBobSpeed + 3π/2)·π·0.075·(1 - flyingFactor)`;
    // at age 0 `cos(3π/2) = 0` so the arms rest exactly on `0.43633232`.
    assert!((allay_arm_idle_bob_amount(0.0, 0.0) - 0.436_332_32).abs() < 1.0e-6);
    let expected_arm = 0.436_332_32
        - (age * 9.0_f32.to_radians() + std::f32::consts::PI * 1.5).cos()
            * std::f32::consts::PI
            * 0.075;
    assert!((allay_arm_idle_bob_amount(age, 0.0) - expected_arm).abs() < 1.0e-6);
    // While flying the arm bob collapses to the rest angle.
    assert!((allay_arm_idle_bob_amount(age, 0.3) - 0.436_332_32).abs() < 1.0e-6);
}

#[test]
fn allay_mesh_uses_vanilla_body_layer_geometry() {
    // Seven cubes (head, two body boxes, two arms, two wings) → 42 faces / 168 vertices.
    let allay = entity_model_mesh(&[EntityModelInstance::allay(800, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(allay.opaque_faces, 42);
    assert_eq!(allay.vertices.len(), 168);
    assert_eq!(allay.indices.len(), 252);
    assert!(allay
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ALLAY_BLUE, 1.0)));
}

#[test]
fn allay_head_tracks_look_angles() {
    // The head re-poses with the projected look yaw/pitch; everything else is unchanged.
    let base = EntityModelInstance::allay(801, [0.0, 64.0, 0.0], 0.0);
    let forward = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(40.0, -25.0)]);
    assert_eq!(forward.vertices.len(), looking.vertices.len());
    assert_ne!(
        forward.vertices, looking.vertices,
        "the head tracks the look"
    );
}

#[test]
fn allay_wings_and_arms_animate_with_age() {
    // A still allay (age 0) differs from one advanced in age: the wings flap and the arms bob.
    let base = EntityModelInstance::allay(802, [0.0, 64.0, 0.0], 0.0);
    let still = entity_model_mesh(&[base]);
    let flapping = entity_model_mesh(&[base.with_age_in_ticks(7.0)]);
    assert_eq!(still.vertices.len(), flapping.vertices.len());
    assert_ne!(still.vertices, flapping.vertices, "the wings flap with age");
}

#[test]
fn allay_flying_pose_differs_from_idle() {
    // An idle allay (walkAnimationSpeed 0) bobs vertically and holds its wings/arms; a flying
    // allay (walkAnimationSpeed ≥ 0.3) tilts its body, flattens its wings, and stops bobbing.
    let idle = EntityModelInstance::allay(803, [0.0, 64.0, 0.0], 0.0);
    let flying = idle.with_walk_animation(0.0, 0.3);
    let idle_mesh = entity_model_mesh(&[idle]);
    let flying_mesh = entity_model_mesh(&[flying]);
    assert_eq!(idle_mesh.vertices.len(), flying_mesh.vertices.len());
    assert_ne!(
        idle_mesh.vertices, flying_mesh.vertices,
        "the flying pose tilts the body and flattens the wings"
    );
}

#[test]
fn allay_texture_ref_matches_vanilla_renderer() {
    let kind = EntityModelKind::Allay;
    assert_eq!(kind.model_key(), "allay");
    assert_eq!(
        kind.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/allay/allay.png",
            size: [32, 32],
        })
    );
}

#[test]
fn allay_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&allay_texture_images()).unwrap();
    // Allay renders into the translucent mesh (`RenderTypes::entityTranslucent`). Seven cubes →
    // 42 faces / 168 vertices, with nothing on the cutout or eyes passes.
    let base = EntityModelInstance::allay(850, [0.0, 64.0, 0.0], 0.0);
    let meshes = entity_model_textured_meshes(&[base], &atlas);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.translucent.cutout_faces, 42);
    assert_eq!(meshes.translucent.vertices.len(), 168);
    assert_eq!(meshes.translucent.indices.len(), 252);
    assert!(meshes
        .translucent
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    // The head re-poses with the projected look yaw/pitch.
    let looking = entity_model_textured_meshes(&[base.with_head_look(40.0, -25.0)], &atlas);
    assert_ne!(meshes.translucent.vertices, looking.translucent.vertices);

    // The wings flap and the arms bob as the age advances.
    let flapping = entity_model_textured_meshes(&[base.with_age_in_ticks(7.0)], &atlas);
    assert_ne!(meshes.translucent.vertices, flapping.translucent.vertices);

    // The flying pose (walkAnimationSpeed ≥ 0.3) differs from the idle pose.
    let flying = entity_model_textured_meshes(&[base.with_walk_animation(0.0, 0.3)], &atlas);
    assert_ne!(meshes.translucent.vertices, flying.translucent.vertices);
}

fn allay_texture_images() -> Vec<EntityModelTextureImage> {
    allay_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
