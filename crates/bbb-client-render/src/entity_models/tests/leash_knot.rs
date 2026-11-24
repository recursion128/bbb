use super::*;

#[test]
fn leash_knot_layer_passes_match_vanilla_renderer() {
    let passes = leash_knot_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, LEASH_KNOT_TEXTURE_REF);
    assert_eq!(passes[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
}

#[test]
fn leash_knot_texture_ref_matches_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&LEASH_KNOT_TEXTURE_REF));
}

#[test]
fn leash_knot_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let images: Vec<EntityModelTextureImage> = [LEASH_KNOT_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::leash_knot(760, [1.0, 64.0, -2.0], 45.0).with(|s| {
        s.light_coords = (4_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());

    assert_eq!(meshes.cutout.cutout_faces, 6);
    assert_eq!(meshes.cutout.vertices.len(), 24);
    assert_eq!(meshes.cutout.indices.len(), 36);
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));
}
