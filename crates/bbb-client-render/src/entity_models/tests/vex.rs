use super::*;
use crate::entity_models::model_layers::vex_model;

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
    assert!((VEX_ARM_CHARGING_ITEM_X_ROT - std::f32::consts::PI * 7.0 / 6.0).abs() < 1.0e-6);
    assert!((VEX_ARM_CHARGING_Y_ROT - std::f32::consts::PI / 12.0).abs() < 1.0e-6);
    assert!((VEX_ARM_CHARGING_Z_ROT - 0.471_238_88).abs() < 1.0e-6);
}

fn posed_vex_arms(instance: EntityModelInstance) -> ([f32; 3], [f32; 3]) {
    let mut model = vex_model(&instance);
    let body = model.child_mut("root").child_mut("body");
    (
        body.child_mut("right_arm").pose.rotation,
        body.child_mut("left_arm").pose.rotation,
    )
}

#[test]
fn vex_charging_uses_held_item_arm_variant() {
    // Vanilla `setArmsCharging`: if either hand item state is non-empty, only those hands switch to
    // the held-item pitch (`π*7/6`). Empty hands retain the rest roll assigned before the charging
    // branch instead of taking the empty-hands lunge.
    let base = EntityModelInstance::vex(962, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.kind.vex_mut().charging = true;
        s.age_in_ticks = 0.0;
    });
    let bob = vex_moving_arm_z_bob(0.0);

    let (right_holding, left_empty) =
        posed_vex_arms(base.with(|s| s.kind.vex_mut().right_hand_item_non_empty = true));
    assert!((right_holding[0] - VEX_ARM_CHARGING_ITEM_X_ROT).abs() < 1.0e-6);
    assert!((right_holding[1] - VEX_ARM_CHARGING_Y_ROT).abs() < 1.0e-6);
    assert!((right_holding[2] - (-VEX_ARM_CHARGING_Z_ROT - bob)).abs() < 1.0e-6);
    assert_eq!(left_empty[0], 0.0);
    assert_eq!(left_empty[1], 0.0);
    assert!((left_empty[2] - -(VEX_ARM_REST_Z_ROT + bob)).abs() < 1.0e-6);

    let (right_holding, left_holding) = posed_vex_arms(base.with(|s| {
        s.kind.vex_mut().right_hand_item_non_empty = true;
        s.kind.vex_mut().left_hand_item_non_empty = true;
    }));
    assert!((right_holding[0] - VEX_ARM_CHARGING_ITEM_X_ROT).abs() < 1.0e-6);
    assert!((left_holding[0] - VEX_ARM_CHARGING_ITEM_X_ROT).abs() < 1.0e-6);
    assert!((left_holding[1] - (-VEX_ARM_CHARGING_Y_ROT)).abs() < 1.0e-6);
    assert!((left_holding[2] - (VEX_ARM_CHARGING_Z_ROT + bob)).abs() < 1.0e-6);
}

#[test]
fn vex_textured_charging_levels_the_body_and_raises_the_arms() {
    let (atlas, _) = build_entity_model_texture_atlas(&vex_texture_images()).unwrap();
    let idle = EntityModelInstance::vex(961, [0.0, 64.0, 0.0], 0.0);
    let idle_mesh = entity_model_textured_meshes(&[idle], &atlas);
    let charging_mesh =
        entity_model_textured_meshes(&[idle.with(|s| s.kind.vex_mut().charging = true)], &atlas);
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
fn vex_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&vex_texture_images()).unwrap();
    // Vanilla `VexModel` constructs with `RenderTypes::entityTranslucent`. The backend folds that into
    // the translucent mesh, but the submission must keep the vanilla render type, selected texture,
    // tint, transform, and default collector order.
    let base = EntityModelInstance::vex(950, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (15_u32 << 4) | (8_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[base], &atlas);

    // Seven cubes → 42 faces / 168 vertices, with nothing on the cutout or eyes passes.
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
    assert!(meshes
        .translucent
        .vertices
        .iter()
        .all(|vertex| vertex.light == base.render_state.shader_light()
            && vertex.overlay == base.render_state.overlay_coords()));
    assert_eq!(base.render_state.shader_light(), [1.0, 8.0 / 15.0]);
    assert_ne!(base.render_state.overlay_coords(), [0.0, 10.0]);

    // The head re-poses with the projected look yaw/pitch.
    let looking = entity_model_textured_meshes(
        &[base.with(|s| (s.head_yaw, s.head_pitch) = (40.0, -25.0))],
        &atlas,
    );
    assert_eq!(
        meshes.translucent.vertices.len(),
        looking.translucent.vertices.len()
    );
    assert_ne!(meshes.translucent.vertices, looking.translucent.vertices);

    // The wings flap and the arms bob as the age advances.
    let flapping = entity_model_textured_meshes(&[base.with(|s| s.age_in_ticks = 7.0)], &atlas);
    assert_ne!(meshes.translucent.vertices, flapping.translucent.vertices);
}

#[test]
fn vex_charging_dispatch_swaps_to_the_charging_texture() {
    // Vanilla `VexRenderer.getTextureLocation`: the charging vex draws `vex_charging.png` over the
    // same `VexModel`, so the geometry is byte-identical and only the sampled atlas region (the UVs)
    // changes. Build an atlas with distinct pixels for the two textures so the swap is observable.
    let images = [VEX_TEXTURE_REF, VEX_CHARGING_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![(index * 40) as u8; len])
        })
        .collect::<Vec<_>>();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    let idle = EntityModelInstance::vex(960, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (15_u32 << 4) | (8_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let charging = EntityModelInstance::new(
        961,
        EntityModelKind::Vex { charging: true },
        [0.0, 64.0, 0.0],
        0.0,
    )
    .with(|s| {
        s.light_coords = (15_u32 << 4) | (8_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let idle_mesh = entity_model_textured_meshes(&[idle], &atlas);
    let charging_mesh = entity_model_textured_meshes(&[charging], &atlas);

    // Same translucent geometry (identical positions), different sampled region (UVs differ).
    assert_eq!(
        idle_mesh.translucent.vertices.len(),
        charging_mesh.translucent.vertices.len()
    );
    assert!(idle_mesh
        .translucent
        .vertices
        .iter()
        .zip(&charging_mesh.translucent.vertices)
        .all(|(a, b)| a.position == b.position));
    assert!(idle_mesh
        .translucent
        .vertices
        .iter()
        .zip(&charging_mesh.translucent.vertices)
        .any(|(a, b)| a.uv != b.uv));
}

fn vex_texture_images() -> Vec<EntityModelTextureImage> {
    [VEX_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
