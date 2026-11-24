use super::*;

#[test]
fn creaking_textured_render_matches_vanilla_renderer() {
    // An inactive creaking renders just the cutout base body.
    let dormant = creaking_textured_layer_passes(false);
    assert_eq!(dormant.len(), 1);
    assert_eq!(
        dormant[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(dormant[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(dormant[0].texture, CREAKING_TEXTURE_REF);
    assert_eq!(dormant[0].visibility, EntityModelLayerVisibility::All);
    assert_eq!(dormant[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(dormant[0].order, 0);
    // An active creaking adds the emissive `creaking_eyes.png` eyes overlay.
    let glowing = creaking_textured_layer_passes(true);
    assert_eq!(glowing.len(), 2);
    assert_eq!(glowing[1].render_type, EntityModelLayerRenderType::Eyes);

    assert_eq!(glowing[1].texture, CREAKING_EYES_TEXTURE_REF);
    assert_eq!(
        glowing[1].visibility,
        EntityModelLayerVisibility::RetainedParts(&["head"])
    );
    assert_eq!(glowing[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(glowing[1].order, 1);
    assert!(entity_model_texture_refs().contains(&CREAKING_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&CREAKING_EYES_TEXTURE_REF));

    let images: Vec<EntityModelTextureImage> = [CREAKING_TEXTURE_REF, CREAKING_EYES_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::creaking(900, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());

    let mesh = &meshes.cutout;

    assert!(!mesh.vertices.is_empty());
    assert_eq!(meshes.eyes.vertices.len(), 4 * 24);
    assert!(
        meshes.eyes.vertices.len() < mesh.vertices.len(),
        "vanilla CreakingModel.createEyesLayer retains only the head subtree"
    );
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
    assert!(meshes.eyes.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, instance.render_state.overlay_coords()[1]]));
}

#[test]
fn creaking_eyes_missing_texture_preserves_other_geometry() {
    let images: Vec<EntityModelTextureImage> = [CREAKING_TEXTURE_REF, CREAKING_EYES_TEXTURE_REF]
        .iter()
        .enumerate()
        .filter_map(|(index, texture)| {
            if *texture == CREAKING_EYES_TEXTURE_REF {
                return None;
            }
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            Some(EntityModelTextureImage::new(
                *texture,
                vec![index as u8; len],
            ))
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::creaking(901, [0.0, 64.0, 0.0], 0.0, true).with(|s| {
        s.light_coords = (3_u32 << 4) | (13_u32 << 20);
        s.white_overlay_progress = 0.7;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(!meshes.cutout.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.eyes.indices.is_empty());
}
