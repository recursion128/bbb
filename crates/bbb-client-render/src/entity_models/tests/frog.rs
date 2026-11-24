use super::*;

#[test]
fn frog_textured_render_matches_vanilla_renderer() {
    // Each of the three temperature variants binds its own base texture (atlas 48×48); all share
    // one `FrogModel` geometry. The three refs match `FrogRenderer.getTextureLocation`'s asset paths.
    for (variant, texture) in [
        (FrogModelVariant::Temperate, FROG_TEMPERATE_TEXTURE_REF),
        (FrogModelVariant::Warm, FROG_WARM_TEXTURE_REF),
        (FrogModelVariant::Cold, FROG_COLD_TEXTURE_REF),
    ] {
        let passes = frog_textured_layer_passes(variant);
        assert_eq!(passes.len(), 1);
        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].texture, texture);
        assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
        assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(passes[0].order, 0);
        assert!(entity_model_texture_refs().contains(&texture));
    }

    let images: Vec<EntityModelTextureImage> = [
        FROG_TEMPERATE_TEXTURE_REF,
        FROG_WARM_TEXTURE_REF,
        FROG_COLD_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::frog(950, [0.0, 64.0, 0.0], 0.0, FrogModelVariant::Warm)
        .with(|s| {
            s.light_coords = (8_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());

    let mesh = &meshes.cutout;

    assert!(
        !mesh.vertices.is_empty(),
        "the frog emits textured geometry"
    );
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
}
