use super::*;

#[test]
fn vex_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `VexModel.createBodyLayer` (atlas 32×32). Head is a plain 5³ box.
    assert_eq!(
        VEX_HEAD[0],
        ModelCubeDesc {
            min: [-2.5, -5.0, -2.5],
            size: [5.0, 5.0, 5.0],
            color: VEX_GREY,
        }
    );

    // Body: the plain `texOffs(0, 10)` box plus the `texOffs(0, 16)` box inset by
    // `CubeDeformation(-0.2)` (min +0.2, size -0.4).
    assert_eq!(VEX_BODY.len(), 2);
    assert_eq!(VEX_BODY[0].min, [-1.5, 0.0, -1.0]);
    assert_eq!(VEX_BODY[0].size, [3.0, 4.0, 2.0]);
    assert_eq!(VEX_BODY[1].min, [-1.3, 1.2, -0.8]);
    assert_eq!(VEX_BODY[1].size, [2.6, 4.6, 1.6]);

    // Arms: 2×4×2 boxes inset by `CubeDeformation(-0.1)` (min +0.1, size -0.2). The right and
    // left arms differ only in their box origin (`-1.25` vs `-0.75`).
    assert_eq!(VEX_RIGHT_ARM[0].min, [-1.15, -0.4, -0.9]);
    assert_eq!(VEX_RIGHT_ARM[0].size, [1.8, 3.8, 1.8]);
    assert_eq!(VEX_LEFT_ARM[0].min, [-0.65, -0.4, -0.9]);
    assert_eq!(VEX_LEFT_ARM[0].size, [1.8, 3.8, 1.8]);

    // Wings: zero-thickness 0×5×8 planes.
    assert_eq!(VEX_WING[0].size, [0.0, 5.0, 8.0]);

    // Part offsets: the model root sits at -2.5, head/body at +20, arms ±1.75, wings ±0.5.
    assert_eq!(VEX_ROOT_POSE.offset, [0.0, -2.5, 0.0]);
    assert_eq!(VEX_HEAD_POSE.offset, [0.0, 20.0, 0.0]);
    assert_eq!(VEX_BODY_POSE.offset, [0.0, 20.0, 0.0]);
    assert_eq!(VEX_RIGHT_ARM_POSE.offset, [-1.75, 0.25, 0.0]);
    assert_eq!(VEX_LEFT_ARM_POSE.offset, [1.75, 0.25, 0.0]);
    assert_eq!(VEX_LEFT_WING_POSE.offset, [0.5, 1.0, 1.0]);
    assert_eq!(VEX_RIGHT_WING_POSE.offset, [-0.5, 1.0, 1.0]);
}

#[test]
fn vex_setup_anim_constants_and_curves_match_vanilla() {
    // Non-charging idle: `body.xRot = π/20`, arm rest roll `π/5`, wing pitch/roll `0.47123888`.
    assert!((VEX_BODY_X_ROT - std::f32::consts::PI / 20.0).abs() < 1.0e-6);
    assert!((VEX_ARM_REST_Z_ROT - std::f32::consts::PI / 5.0).abs() < 1.0e-6);
    assert!((VEX_WING_X_ROT - 0.471_238_88).abs() < 1.0e-6);
    assert!((VEX_WING_Z_ROT - 0.471_238_88).abs() < 1.0e-6);

    // `movingArmZBob = cos(ageInTicks · 5.5°) · 0.1`; at age 0 it is the peak `0.1`.
    assert!((vex_moving_arm_z_bob(0.0) - 0.1).abs() < 1.0e-6);
    let age = 9.0_f32;
    assert!((vex_moving_arm_z_bob(age) - (age * 5.5_f32.to_radians()).cos() * 0.1).abs() < 1.0e-6);

    // `leftWing.yRot = 1.0995574 + cos(ageInTicks · 45.836624°) · 16.2°`.
    let expected_rest = 1.099_557_4 + 16.2_f32.to_radians();
    assert!((vex_left_wing_y_rot(0.0) - expected_rest).abs() < 1.0e-6);
    let expected_age =
        1.099_557_4 + (age * 45.836_624_f32.to_radians()).cos() * 16.2_f32.to_radians();
    assert!((vex_left_wing_y_rot(age) - expected_age).abs() < 1.0e-6);

    // Charging (both hands empty, `VexModel.setArmsCharging`): both arms pitch to
    // `xRot = -1.2217305`, yaw to `±π/12`, and roll to `∓0.47123888 ∓ bob`.
    assert!((VEX_ARM_CHARGING_X_ROT - (-1.221_730_5)).abs() < 1.0e-6);
    assert!((VEX_ARM_CHARGING_Y_ROT - std::f32::consts::PI / 12.0).abs() < 1.0e-6);
    assert!((VEX_ARM_CHARGING_Z_ROT - 0.471_238_88).abs() < 1.0e-6);
}

#[test]
fn vex_charging_levels_the_body_and_raises_the_arms() {
    // Vanilla `VexModel.setupAnim`: `if (isCharging) { body.xRot = 0; setArmsCharging(...) }`.
    // The idle pose tilts the body `π/20` and rolls the arms to `±π/5`; charging levels the
    // body and pitches the arms to `-1.2217305`, so the posed mesh must differ from idle while
    // keeping the same vertex count (only rotations change). Compared at the same age so the
    // bob and wing flap are identical between the two — the difference is purely the charge.
    let idle = EntityModelInstance::vex(960, [0.0, 64.0, 0.0], 0.0);
    let charging = idle.with_vex_charging(true);
    let idle_mesh = entity_model_mesh(&[idle]);
    let charging_mesh = entity_model_mesh(&[charging]);
    assert_eq!(idle_mesh.vertices.len(), charging_mesh.vertices.len());
    assert_ne!(
        idle_mesh.vertices, charging_mesh.vertices,
        "charging levels the body and raises the arms"
    );

    // An idle vex with `with_vex_charging(false)` is identical to the default idle (the default
    // render state is not-charging), confirming the flag is what flips the pose.
    let still_idle = entity_model_mesh(&[idle.with_vex_charging(false)]);
    assert_eq!(idle_mesh.vertices, still_idle.vertices);
}

#[test]
fn vex_textured_charging_levels_the_body_and_raises_the_arms() {
    let (atlas, _) = build_entity_model_texture_atlas(&vex_texture_images()).unwrap();
    let idle = EntityModelInstance::vex(961, [0.0, 64.0, 0.0], 0.0);
    let idle_mesh = entity_model_textured_meshes(&[idle], &atlas);
    let charging_mesh = entity_model_textured_meshes(&[idle.with_vex_charging(true)], &atlas);
    assert_eq!(
        idle_mesh.translucent.vertices.len(),
        charging_mesh.translucent.vertices.len()
    );
    assert_ne!(
        idle_mesh.translucent.vertices, charging_mesh.translucent.vertices,
        "charging levels the body and raises the arms on the textured path too"
    );
}

#[test]
fn vex_mesh_uses_vanilla_body_layer_geometry() {
    // Seven cubes (head, two body boxes, two arms, two wings) → 42 faces / 168 vertices.
    let vex = entity_model_mesh(&[EntityModelInstance::vex(900, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(vex.opaque_faces, 42);
    assert_eq!(vex.vertices.len(), 168);
    assert_eq!(vex.indices.len(), 252);
    assert!(vex
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(VEX_GREY, 1.0)));
}

#[test]
fn vex_head_tracks_look_angles() {
    // The head re-poses with the projected look yaw/pitch; everything else is unchanged.
    let base = EntityModelInstance::vex(901, [0.0, 64.0, 0.0], 0.0);
    let forward = entity_model_mesh(&[base]);
    let looking = entity_model_mesh(&[base.with_head_look(40.0, -25.0)]);
    assert_eq!(forward.vertices.len(), looking.vertices.len());
    assert_ne!(
        forward.vertices, looking.vertices,
        "the head tracks the look"
    );
}

#[test]
fn vex_wings_and_arms_animate_with_age() {
    // A still vex (age 0) differs from one advanced in age: the wings flap and the arms bob.
    let base = EntityModelInstance::vex(902, [0.0, 64.0, 0.0], 0.0);
    let still = entity_model_mesh(&[base]);
    let flapping = entity_model_mesh(&[base.with_age_in_ticks(7.0)]);
    assert_eq!(still.vertices.len(), flapping.vertices.len());
    assert_ne!(still.vertices, flapping.vertices, "the wings flap with age");
}

#[test]
fn vex_texture_ref_matches_vanilla_renderer() {
    let kind = EntityModelKind::Vex;
    assert_eq!(kind.model_key(), "vex");
    assert_eq!(
        kind.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/illager/vex.png",
            size: [32, 32],
        })
    );
}

#[test]
fn vex_textured_cubes_match_vanilla_body_layer_uvs() {
    // The textured cubes mirror the colored geometry. `CubeDeformation` inflates the box but
    // the `uv_size` keeps the BASE box, exactly as vanilla bakes it.
    assert_eq!(VEX_TEXTURED_HEAD[0].min, [-2.5, -5.0, -2.5]);
    assert_eq!(VEX_TEXTURED_HEAD[0].size, [5.0, 5.0, 5.0]);
    assert_eq!(VEX_TEXTURED_HEAD[0].uv_size, [5.0, 5.0, 5.0]);
    assert_eq!(VEX_TEXTURED_HEAD[0].tex, [0.0, 0.0]);

    // Body box 0 is `texOffs(0, 10)` 3×4×2; box 1 is `texOffs(0, 16)` with the 3×5×2 base box
    // inset by `CubeDeformation(-0.2)`.
    assert_eq!(VEX_TEXTURED_BODY[0].tex, [0.0, 10.0]);
    assert_eq!(VEX_TEXTURED_BODY[0].uv_size, [3.0, 4.0, 2.0]);
    assert_eq!(VEX_TEXTURED_BODY[1].tex, [0.0, 16.0]);
    assert_eq!(VEX_TEXTURED_BODY[1].size, [2.6, 4.6, 1.6]);
    assert_eq!(VEX_TEXTURED_BODY[1].uv_size, [3.0, 5.0, 2.0]);

    // Arms: `texOffs(23, 0)` / `texOffs(23, 6)`, 2×4×2 base box inset by `CubeDeformation(-0.1)`.
    assert_eq!(VEX_TEXTURED_RIGHT_ARM[0].tex, [23.0, 0.0]);
    assert_eq!(VEX_TEXTURED_RIGHT_ARM[0].uv_size, [2.0, 4.0, 2.0]);
    assert_eq!(VEX_TEXTURED_LEFT_ARM[0].tex, [23.0, 6.0]);
    assert_eq!(VEX_TEXTURED_LEFT_ARM[0].uv_size, [2.0, 4.0, 2.0]);

    // Both wings share `texOffs(16, 14)`; only the left wing's UV is mirrored.
    assert_eq!(VEX_TEXTURED_LEFT_WING[0].tex, [16.0, 14.0]);
    assert!(VEX_TEXTURED_LEFT_WING[0].mirror);
    assert_eq!(VEX_TEXTURED_RIGHT_WING[0].tex, [16.0, 14.0]);
    assert!(!VEX_TEXTURED_RIGHT_WING[0].mirror);
}

#[test]
fn vex_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&vex_texture_images()).unwrap();
    // Vex renders into the translucent mesh (`RenderTypes::entityTranslucent`). Seven cubes →
    // 42 faces / 168 vertices, with nothing on the cutout or eyes passes.
    let base = EntityModelInstance::vex(950, [0.0, 64.0, 0.0], 0.0);
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
    assert_eq!(
        meshes.translucent.vertices.len(),
        looking.translucent.vertices.len()
    );
    assert_ne!(meshes.translucent.vertices, looking.translucent.vertices);

    // The wings flap and the arms bob as the age advances.
    let flapping = entity_model_textured_meshes(&[base.with_age_in_ticks(7.0)], &atlas);
    assert_ne!(meshes.translucent.vertices, flapping.translucent.vertices);
}

fn vex_texture_images() -> Vec<EntityModelTextureImage> {
    vex_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
