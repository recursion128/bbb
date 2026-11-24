use super::*;

#[test]
fn shulker_bullet_textured_render_matches_vanilla_renderer() {
    let passes = shulker_bullet_textured_layer_passes();
    assert_eq!(passes.len(), 2);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(
        (passes[0].texture, passes[0].tint),
        (SHULKER_BULLET_TEXTURE_REF, [1.0, 1.0, 1.0, 1.0])
    );
    assert_eq!(passes[0].order, 0);
    assert_eq!(
        passes[1].render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(passes[1].render_type.vanilla_name(), "entityTranslucent");
    assert_eq!(
        (passes[1].texture, passes[1].tint),
        (SHULKER_BULLET_TEXTURE_REF, [1.0, 1.0, 1.0, 38.0 / 255.0])
    );
    assert_eq!(passes[1].order, 1);
    assert!(entity_model_texture_refs().contains(&SHULKER_BULLET_TEXTURE_REF));

    let len = usize::try_from(
        SHULKER_BULLET_TEXTURE_REF.size[0] * SHULKER_BULLET_TEXTURE_REF.size[1] * 4,
    )
    .unwrap();
    let images = vec![EntityModelTextureImage::new(
        SHULKER_BULLET_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::shulker_bullet(1130, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (8_u32 << 4) | (8_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    let mesh = &meshes.cutout;
    assert_eq!(mesh.vertices.len(), 72);
    assert_eq!(mesh.indices.len(), 108);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(mesh.vertices.iter().all(|vertex| {
        vertex.light == instance.render_state.shader_light() && vertex.overlay == [0.0, 10.0]
    }));
    assert_eq!(meshes.translucent.vertices.len(), 72);
    assert_eq!(meshes.translucent.indices.len(), 108);
    assert!(meshes
        .translucent
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 38.0 / 255.0]));
    assert!(meshes.translucent.vertices.iter().all(|vertex| {
        vertex.light == instance.render_state.shader_light() && vertex.overlay == [0.0, 10.0]
    }));

    let (base_min, base_max) = textured_mesh_extents(&meshes.cutout);
    let (shell_min, shell_max) = textured_mesh_extents(&meshes.translucent);
    let center = |min: [f32; 3], max: [f32; 3]| {
        [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ]
    };
    let size = |min: [f32; 3], max: [f32; 3]| [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    assert_close3(center(shell_min, shell_max), center(base_min, base_max));
    let base_size = size(base_min, base_max);
    assert_close3(
        size(shell_min, shell_max),
        [base_size[0] * 1.5, base_size[1] * 1.5, base_size[2] * 1.5],
    );
}

#[test]
fn shulker_bullet_missing_texture_preserves_other_geometry() {
    let base_len =
        usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![0u8; base_len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::shulker_bullet(1133, [2.0, 65.0, -4.0], 30.0).with(|s| {
        s.light_coords = (7_u32 << 4) | (10_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
}
