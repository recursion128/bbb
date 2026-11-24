use super::*;

#[test]
fn sniffer_textured_render_matches_vanilla_renderer() {
    let passes = sniffer_textured_layer_passes(false);
    let baby_passes = sniffer_textured_layer_passes(true);
    assert_eq!(passes.len(), 1);
    assert_eq!(baby_passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, SNIFFER_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
    assert_eq!(baby_passes[0].render_type, passes[0].render_type);
    assert_eq!(baby_passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(baby_passes[0].texture, SNIFFLET_TEXTURE_REF);
    assert_eq!(baby_passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(baby_passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(baby_passes[0].order, 0);
    assert!(entity_model_texture_refs().contains(&SNIFFER_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&SNIFFLET_TEXTURE_REF));

    let images: Vec<EntityModelTextureImage> = [SNIFFER_TEXTURE_REF, SNIFFLET_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::sniffer(900, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (5_u32 << 4) | (14_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());

    let mesh = &meshes.cutout;

    assert!(!mesh.vertices.is_empty());
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));

    let baby_instance = EntityModelInstance::snifflet(901, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.light_coords = (2_u32 << 4) | (9_u32 << 20));
    let baby_meshes = entity_model_textured_meshes(&[baby_instance], &atlas);

    assert!(!baby_meshes.cutout.vertices.is_empty());
    assert_ne!(meshes.cutout.vertices, baby_meshes.cutout.vertices);
}
