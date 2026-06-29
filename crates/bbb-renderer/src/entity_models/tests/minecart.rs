use super::*;

use crate::entity_models::model::ModelCube;

#[test]
fn minecart_cubes_match_vanilla_26_1_body_layer() {
    // Vanilla MinecartModel.createBodyLayer(): a 20x16x2 floor panel laid flat plus four
    // 16x8x2 wall panels boxed in. No setupAnim, so the cart is static. Each unified cube carries
    // the colored tint (`MINECART_GRAY`) and the textured UV (`texOffs` / `uv_size` / `mirror`) in
    // one struct.
    //
    // bottom: texOffs(0, 10), addBox(-10, -8, -1, 20, 16, 2). The floor samples texOffs(0, 10).
    assert_eq!(
        MINECART_BOTTOM[0],
        ModelCube::new(
            [-10.0, -8.0, -1.0],
            [20.0, 16.0, 2.0],
            MINECART_GRAY,
            [20.0, 16.0, 2.0],
            [0.0, 10.0],
            false,
        )
    );
    // The four walls share one texOffs(0, 0) box(-8, -9, -1, 16x8x2), not mirrored.
    assert_eq!(
        MINECART_WALL[0],
        ModelCube::new(
            [-8.0, -9.0, -1.0],
            [16.0, 8.0, 2.0],
            MINECART_GRAY,
            [16.0, 8.0, 2.0],
            [0.0, 0.0],
            false,
        )
    );
}

#[test]
fn minecart_layer_passes_match_vanilla_renderer() {
    let passes = minecart_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].kind, EntityModelLayerKind::MinecartBase);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].model_layer, MODEL_LAYER_MINECART);
    assert_eq!(passes[0].texture, MINECART_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((passes[0].order, passes[0].submit_sequence), (0, 0));
}

#[test]
fn minecart_texture_ref_matches_vanilla_renderer() {
    assert_eq!(EntityModelKind::Minecart.model_key(), "minecart");
    assert_eq!(
        EntityModelKind::Minecart.vanilla_texture_ref(),
        Some(EntityModelTextureRef {
            path: "textures/entity/minecart/minecart.png",
            size: [64, 32],
        })
    );
    assert!(entity_model_texture_refs().contains(&MINECART_TEXTURE_REF));
    assert_eq!(minecart_entity_texture_refs(), &[MINECART_TEXTURE_REF]);
}

#[test]
fn minecart_mesh_uses_vanilla_body_layer_geometry() {
    let mesh = entity_model_mesh(&[EntityModelInstance::minecart(1, [0.0, 64.0, 0.0], 0.0)]);
    // Five cubes => 30 faces, 120 verts, 180 indices.
    assert_eq!(mesh.opaque_faces, 30);
    assert_eq!(mesh.vertices.len(), 120);
    assert_eq!(mesh.indices.len(), 180);
}

#[test]
fn minecart_jitter_matches_vanilla_offset_seed_formula() {
    assert_close3(
        minecart_render_jitter_offset(1),
        [0.00175, 0.00075, 0.00175],
    );
    assert_close3(
        minecart_render_jitter_offset(2),
        [0.00175, -0.00125, 0.00025],
    );
    assert_close3(
        minecart_render_jitter_offset(41),
        [-0.00175, 0.00175, 0.00025],
    );

    for offset in [-1, 0, 1, 2, 41, i32::MAX]
        .into_iter()
        .map(minecart_render_jitter_offset)
    {
        assert!(offset.iter().all(|component| component.abs() <= 0.00175));
    }
}

#[test]
fn minecart_root_transform_matches_vanilla_old_render_without_rail() {
    let instance =
        EntityModelInstance::minecart(41, [2.0, 64.0, -3.0], 45.0).with_head_look(0.0, -10.0);
    let expected = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_translation(Vec3::from_array([-0.00175, 0.00175, 0.00025]))
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0_f32 - instance.render_state.body_rot).to_radians())
        * Mat4::from_rotation_z((-instance.render_state.head_pitch).to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));

    assert_close_transform(minecart_model_root_transform(instance), expected);
    assert_ne!(
        minecart_model_root_transform(instance),
        entity_model_root_transform(instance)
    );
}

#[test]
fn minecart_new_render_root_transform_uses_vanilla_order() {
    let old_render =
        EntityModelInstance::minecart(41, [2.0, 64.0, -3.0], 45.0).with_head_look(0.0, -10.0);
    let new_render = old_render.with_minecart_new_render(true);
    let expected = Mat4::from_translation(Vec3::from_array(new_render.position))
        * Mat4::from_translation(Vec3::from_array([-0.00175, 0.00175, 0.00025]))
        * Mat4::from_rotation_y(new_render.render_state.body_rot.to_radians())
        * Mat4::from_rotation_z((-new_render.render_state.head_pitch).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));

    assert_close_transform(minecart_model_root_transform(new_render), expected);
    assert_ne!(
        minecart_model_root_transform(new_render),
        minecart_model_root_transform(old_render)
    );
}

#[test]
fn minecart_hurt_roll_matches_vanilla_vehicle_damage_formula() {
    let damaged = EntityModelInstance::minecart(41, [2.0, 64.0, -3.0], 45.0)
        .with_head_look(0.0, -10.0)
        .with_minecart_hurt_time(7.5)
        .with_minecart_hurt_dir(-1)
        .with_minecart_damage_time(17.5);
    let roll = 7.5_f32.sin() * 7.5 * 17.5 / 10.0 * -1.0;
    assert!((minecart_damage_roll_degrees(damaged) - roll).abs() < 1.0e-6);

    let expected = Mat4::from_translation(Vec3::from_array(damaged.position))
        * Mat4::from_translation(Vec3::from_array([-0.00175, 0.00175, 0.00025]))
        * Mat4::from_translation(Vec3::new(0.0, 0.375, 0.0))
        * Mat4::from_rotation_y((180.0_f32 - damaged.render_state.body_rot).to_radians())
        * Mat4::from_rotation_z((-damaged.render_state.head_pitch).to_radians())
        * Mat4::from_rotation_x(roll.to_radians())
        * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0));
    assert_close_transform(minecart_model_root_transform(damaged), expected);

    let settled = damaged.with_minecart_hurt_time(0.0);
    assert_eq!(minecart_damage_roll_degrees(settled), 0.0);
}

#[test]
fn minecart_textured_mesh_matches_colored_geometry_and_vanilla_uvs() {
    let (atlas, _) = build_entity_model_texture_atlas(&minecart_texture_images()).unwrap();
    let instance = EntityModelInstance::minecart(1, [0.0, 64.0, 0.0], 0.0)
        .with_light_coords((6_u32 << 4) | (13_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.texture, MINECART_TEXTURE_REF);
    assert_eq!(submit.render_type, EntityModelLayerRenderType::EntityCutout);
    assert_eq!(submit.render_type.vanilla_name(), "entityCutout");
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, minecart_model_root_transform(instance));
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, [0.0, 10.0]);
    assert_ne!(submit.overlay, instance.render_state.overlay_coords());
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    let textured = &meshes.cutout;
    assert_eq!(textured.cutout_faces, 30);
    assert_eq!(textured.vertices.len(), 120);
    assert_eq!(textured.indices.len(), 180);
    assert!(textured
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(textured
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));
    // The textured cart shares the colored cart's geometry exactly (same parts and transform).
    let colored = entity_model_mesh(&[EntityModelInstance::minecart(1, [0.0, 64.0, 0.0], 0.0)]);
    let (cmin, cmax) = mesh_extents(&colored);
    let (tmin, tmax) = textured_mesh_extents(textured);
    assert_close3(tmin, cmin);
    assert_close3(tmax, cmax);
}

fn minecart_texture_images() -> Vec<EntityModelTextureImage> {
    minecart_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_close_transform(actual: Mat4, expected: Mat4) {
    for (actual, expected) in actual
        .to_cols_array()
        .into_iter()
        .zip(expected.to_cols_array())
    {
        assert!(
            (actual - expected).abs() < 1.0e-5,
            "expected {expected}, got {actual}"
        );
    }
}
