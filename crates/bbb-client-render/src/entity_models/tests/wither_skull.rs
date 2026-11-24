use super::*;

#[test]
fn wither_skull_textured_render_matches_vanilla_renderer() {
    let normal_pass = wither_skull_textured_layer_passes(false)[0];
    assert_eq!(normal_pass.texture, WITHER_TEXTURE_REF);
    assert_eq!(
        normal_pass.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(normal_pass.render_type.vanilla_name(), "entityTranslucent");
    assert_eq!(normal_pass.visibility, EntityModelLayerVisibility::All);
    assert_eq!(normal_pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(normal_pass.order, 0);
    let dangerous_pass = wither_skull_textured_layer_passes(true)[0];
    assert_eq!(dangerous_pass.texture, WITHER_INVULNERABLE_TEXTURE_REF);
    assert_eq!(
        dangerous_pass.render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(
        dangerous_pass.render_type.vanilla_name(),
        "entityTranslucent"
    );
    assert_eq!(dangerous_pass.visibility, EntityModelLayerVisibility::All);
    assert_eq!(dangerous_pass.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(dangerous_pass.order, 0);
    assert!(entity_model_texture_refs().contains(&WITHER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&WITHER_INVULNERABLE_TEXTURE_REF));

    let images: Vec<EntityModelTextureImage> =
        [WITHER_TEXTURE_REF, WITHER_INVULNERABLE_TEXTURE_REF]
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
                EntityModelTextureImage::new(*texture, vec![index as u8; len])
            })
            .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let normal = EntityModelInstance::wither_skull(820, [1.0, 64.0, -2.0], 37.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -18.0))
        .with(|s| {
            s.light_coords = (6_u32 << 4) | (10_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let dangerous =
        EntityModelInstance::wither_skull_with_dangerous(820, [1.0, 64.0, -2.0], 37.0, true)
            .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -18.0))
            .with(|s| {
                s.light_coords = (6_u32 << 4) | (10_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            });
    let normal_meshes = entity_model_textured_meshes(&[normal], &atlas);
    let dangerous_meshes = entity_model_textured_meshes(&[dangerous], &atlas);
    assert!(normal_meshes.cutout.vertices.is_empty());

    let normal_mesh = &normal_meshes.translucent;
    let dangerous_mesh = &dangerous_meshes.translucent;
    assert!(!normal_mesh.vertices.is_empty());
    assert_eq!(normal_mesh.vertices.len(), dangerous_mesh.vertices.len());
    assert_ne!(
        normal_mesh.vertices, dangerous_mesh.vertices,
        "dangerous skulls select the invulnerable texture atlas entry"
    );
    assert!(normal_mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == normal.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]));
    assert!(dangerous_mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == dangerous.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]));
}

#[test]
fn wither_skull_missing_texture_preserves_other_geometry() {
    let base_len =
        usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![0u8; base_len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance =
        EntityModelInstance::wither_skull_with_dangerous(820, [1.0, 64.0, -2.0], 37.0, true)
            .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -18.0))
            .with(|s| {
                s.light_coords = (6_u32 << 4) | (10_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}
