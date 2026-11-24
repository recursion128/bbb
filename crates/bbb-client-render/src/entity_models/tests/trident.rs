use super::*;

#[test]
fn trident_layer_passes_and_texture_ref_match_vanilla_renderer() {
    let passes = trident_textured_layer_passes();
    assert_eq!(passes.len(), 2);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, TRIDENT_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
    assert_eq!(
        passes[1].render_type,
        EntityModelLayerRenderType::EntityGlint
    );
    assert_eq!(passes[1].render_type.vanilla_name(), "entityGlint");
    assert_eq!(passes[1].texture, ENCHANTED_GLINT_ITEM_TEXTURE_REF);
    assert_eq!(passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[1].order, 1);

    assert!(entity_model_texture_refs().contains(&TRIDENT_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&TRIDENT_RIPTIDE_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&ENCHANTED_GLINT_ITEM_TEXTURE_REF));
}

#[test]
fn trident_textured_mesh_uses_vanilla_uvs_and_geometry() {
    let images: Vec<EntityModelTextureImage> = [
        TRIDENT_TEXTURE_REF,
        TRIDENT_RIPTIDE_TEXTURE_REF,
        ENCHANTED_GLINT_ITEM_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::trident(1350, [1.0, 64.0, -2.0], 37.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -18.0))
        .with(|s| {
            s.light_coords = (5_u32 << 4) | (11_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    let mesh = meshes.cutout;
    assert_eq!(mesh.cutout_faces, 30);
    assert_eq!(mesh.vertices.len(), 120);
    assert_eq!(mesh.indices.len(), 180);
    assert!(mesh.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));
}

#[test]
fn foiled_trident_emits_vanilla_entity_glint_geometry() {
    let images: Vec<EntityModelTextureImage> = [
        TRIDENT_TEXTURE_REF,
        TRIDENT_RIPTIDE_TEXTURE_REF,
        ENCHANTED_GLINT_ITEM_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::trident(1350, [1.0, 64.0, -2.0], 37.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -18.0))
        .with(|s| {
            s.light_coords = (5_u32 << 4) | (11_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
            s.kind.trident_mut().foil = true;
        });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.cutout.cutout_faces, 30);
    assert_eq!(meshes.cutout.vertices.len(), 120);
    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
    assert_eq!(meshes.entity_glint.vertices.len(), 120);
    assert!(meshes.entity_glint.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == instance.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]
    }));
    assert!(meshes.armor_entity_glint.vertices.is_empty());
}

#[test]
fn trident_pickup_particle_mesh_uses_item_pickup_group_submission() {
    // The item-pickup carried trident (vanilla `ItemPickupParticleGroup.State.submit`
    // -> `ThrownTridentRenderer`): the base model pass and, for `isFoil`, the
    // order(1) glint submit are both baked into the blended particles target
    // with the pass forced to `EntityTranslucent`, the world-space interpolated
    // pickup transform passed through, and the frozen pickup light applied.
    use crate::ParticleItemPickupProjectileKind;

    let images: Vec<EntityModelTextureImage> = [
        TRIDENT_TEXTURE_REF,
        TRIDENT_RIPTIDE_TEXTURE_REF,
        ENCHANTED_GLINT_ITEM_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let transform =
        Mat4::from_translation(Vec3::new(1.0, 65.0, -2.0)) * Mat4::from_rotation_z(0.25);

    let plain = projectile_pickup_particle_textured_meshes(
        &[ProjectilePickupParticleRenderInstance {
            transform,
            kind: ParticleItemPickupProjectileKind::Trident { foil: false },
            light: [0.4, 0.8],
        }],
        &atlas,
    );
    assert!(plain.cutout.vertices.is_empty());
    assert!(plain.entity_glint.vertices.is_empty());

    assert_eq!(plain.translucent.vertices.len(), 120);

    let foiled = projectile_pickup_particle_textured_meshes(
        &[ProjectilePickupParticleRenderInstance {
            transform,
            kind: ParticleItemPickupProjectileKind::Trident { foil: true },
            light: [0.4, 0.8],
        }],
        &atlas,
    );

    assert_eq!(foiled.translucent.vertices.len(), 240);

    assert!(foiled.translucent.vertices.iter().all(|vertex| {
        vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == [0.4, 0.8]
            && vertex.overlay == ENTITY_VERTEX_NO_OVERLAY
    }));
}

#[test]
fn foiled_trident_missing_texture_preserves_other_geometry() {
    let base_len =
        usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![0u8; base_len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance = EntityModelInstance::trident(1350, [1.0, 64.0, -2.0], 37.0)
        .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -18.0))
        .with(|s| {
            s.light_coords = (5_u32 << 4) | (11_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
            s.kind.trident_mut().foil = true;
        });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.scroll.vertices.is_empty());
    assert!(meshes.scroll_additive.vertices.is_empty());
    assert!(meshes.entity_glint.vertices.is_empty());
    assert!(meshes.armor_entity_glint.vertices.is_empty());
}
