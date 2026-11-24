use super::*;

#[test]
fn llama_spit_textured_render_matches_vanilla_renderer() {
    let passes = llama_spit_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(passes[0].texture, LLAMA_SPIT_TEXTURE_REF);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
    assert!(entity_model_texture_refs().contains(&LLAMA_SPIT_TEXTURE_REF));

    let len = usize::try_from(LLAMA_SPIT_TEXTURE_REF.size[0] * LLAMA_SPIT_TEXTURE_REF.size[1] * 4)
        .unwrap();
    let images = vec![EntityModelTextureImage::new(
        LLAMA_SPIT_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::llama_spit(790, [0.0, 64.0, 0.0], 35.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -12.0))
        .with(|s| {
            s.light_coords = (7_u32 << 4) | (9_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());

    assert_eq!(meshes.cutout.cutout_faces, 42);
    assert_eq!(meshes.cutout.vertices.len(), 168);
    assert_eq!(meshes.cutout.indices.len(), 252);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));
}

#[test]
fn llama_spit_missing_texture_preserves_other_geometry() {
    let base_len =
        usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![0u8; base_len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::llama_spit(792, [2.0, 65.0, -4.0], 35.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -12.0))
        .with(|s| {
            s.light_coords = (7_u32 << 4) | (9_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}
