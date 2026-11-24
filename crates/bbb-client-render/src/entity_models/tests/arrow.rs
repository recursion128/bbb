use super::*;
use crate::entity_models::model_layers::arrow_model;

#[test]
fn arrow_impact_shake_rolls_root_like_vanilla_setup_anim() {
    let base = EntityModelInstance::arrow(60, [0.0, 64.0, 0.0], 0.0, ArrowModelTexture::Normal);
    let model = arrow_model(&base);
    assert_eq!(model.pose.rotation[2], 0.0);

    let shaken = base.with(|s| s.kind.arrow_mut().shake = 4.5);
    drop(model);
    let model = arrow_model(&shaken);
    let expected = arrow_shake_z_rot(4.5);
    assert!(
        (model.pose.rotation[2] - expected).abs() < 1.0e-6,
        "root zRot expected {expected}, got {}",
        model.pose.rotation[2]
    );

    drop(model);
    let model = arrow_model(&base);
    assert_eq!(
        model.pose.rotation[2], 0.0,
        "the next frame must not keep the previous shake pose"
    );
    assert_eq!(arrow_shake_z_rot(0.0), 0.0);
}

#[test]
fn arrow_textured_render_matches_vanilla_renderer() {
    // One model, three images: normal / tipped (`getColor() > 0`) / spectral (the distinct entity).
    for (texture, texture_ref) in [
        (ArrowModelTexture::Normal, ARROW_TEXTURE_REF),
        (ArrowModelTexture::Tipped, ARROW_TIPPED_TEXTURE_REF),
        (ArrowModelTexture::Spectral, ARROW_SPECTRAL_TEXTURE_REF),
    ] {
        let pass = arrow_textured_layer_passes(texture)[0];
        assert_eq!(pass.texture, texture_ref);
        assert_eq!(
            pass.render_type,
            EntityModelLayerRenderType::EntityCutoutCull
        );
        assert_eq!(pass.render_type.vanilla_name(), "entityCutoutCull");
        assert_eq!(pass.visibility, EntityModelLayerVisibility::All);
        assert_eq!(pass.tint, [1.0; 4]);
        assert_eq!(pass.order, 0);
        assert!(entity_model_texture_refs().contains(&texture_ref));
    }

    let len = usize::try_from(ARROW_TEXTURE_REF.size[0] * ARROW_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ARROW_TIPPED_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance =
        EntityModelInstance::arrow(60, [0.0, 64.0, 0.0], 35.0, ArrowModelTexture::Tipped)
            .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -12.0))
            .with(|s| {
                s.light_coords = (4_u32 << 4) | (12_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            });
    let meshes = entity_model_textured_meshes(&[instance], &atlas);
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.cutout.vertices.is_empty());
    assert!(!meshes.cutout_cull.vertices.is_empty());
    assert!(meshes
        .cutout_cull
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));

    assert!(meshes.cutout_cull.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == [0.0, 10.0]));

    let shaken_meshes =
        entity_model_textured_meshes(&[instance.with(|s| s.kind.arrow_mut().shake = 4.5)], &atlas);

    assert_eq!(
        shaken_meshes.cutout_cull.vertices.len(),
        meshes.cutout_cull.vertices.len()
    );
    assert!(
        meshes
            .cutout_cull
            .vertices
            .iter()
            .zip(&shaken_meshes.cutout_cull.vertices)
            .any(|(base, shaken)| base.position != shaken.position),
        "impact shake should pose arrow geometry without changing the renderer root transform"
    );
}

#[test]
fn arrow_pickup_particle_mesh_uses_item_pickup_group_submission() {
    // The item-pickup carried arrow (vanilla `ItemPickupParticleGroup.State.submit`
    // -> `ArrowRenderer`): baked like the elder-guardian particle model with the
    // pass forced to `EntityTranslucent` for the blended particles target, the
    // world-space interpolated pickup transform passed through, and the frozen
    // pickup light applied per-vertex. The texture keeps the vanilla
    // normal / tipped / spectral selection.
    use crate::ParticleItemPickupProjectileKind;

    let images: Vec<EntityModelTextureImage> = [
        ARROW_TEXTURE_REF,
        ARROW_TIPPED_TEXTURE_REF,
        ARROW_SPECTRAL_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    for (kind, texture_ref) in [
        (ParticleItemPickupProjectileKind::Arrow, ARROW_TEXTURE_REF),
        (
            ParticleItemPickupProjectileKind::TippedArrow,
            ARROW_TIPPED_TEXTURE_REF,
        ),
        (
            ParticleItemPickupProjectileKind::SpectralArrow,
            ARROW_SPECTRAL_TEXTURE_REF,
        ),
    ] {
        let transform =
            Mat4::from_translation(Vec3::new(1.0, 65.0, -2.0)) * Mat4::from_rotation_y(0.5);
        let instance = ProjectilePickupParticleRenderInstance {
            transform,
            kind,
            light: [0.4, 0.8],
        };

        let meshes = projectile_pickup_particle_textured_meshes(&[instance], &atlas);

        let uv = atlas
            .entries
            .iter()
            .find(|entry| entry.texture == texture_ref)
            .unwrap()
            .uv;
        assert!(meshes.translucent.vertices.iter().all(|vertex| {
            vertex.uv[0] >= uv.min[0]
                && vertex.uv[0] <= uv.max[0]
                && vertex.uv[1] >= uv.min[1]
                && vertex.uv[1] <= uv.max[1]
        }));
        assert!(meshes.cutout.vertices.is_empty());
        assert!(meshes.cutout_cull.vertices.is_empty());
        assert!(
            !meshes.translucent.vertices.is_empty(),
            "{kind:?} emits textured translucent geometry"
        );

        assert!(meshes.translucent.vertices.iter().all(|vertex| {
            vertex.tint == [1.0, 1.0, 1.0, 1.0]
                && vertex.light == [0.4, 0.8]
                && vertex.overlay == ENTITY_VERTEX_NO_OVERLAY
        }));
    }
}

#[test]
fn glowing_arrow_outline_copy_uses_source_cull_bucket() {
    // Vanilla `RenderType.outline()` forwards `state.pipeline.isCull()`, so an
    // `entityCutoutCull` arrow outline derives `OUTLINE_CULL`.
    let len =
        usize::try_from(ARROW_TIPPED_TEXTURE_REF.size[0] * ARROW_TIPPED_TEXTURE_REF.size[1] * 4)
            .unwrap();
    let images = vec![EntityModelTextureImage::new(
        ARROW_TIPPED_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance =
        EntityModelInstance::arrow(60, [0.0, 64.0, 0.0], 35.0, ArrowModelTexture::Tipped)
            .with_outline_color(0xff33_66cc);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.outline.vertices.is_empty());
    assert_eq!(
        meshes.outline_cull.vertices.len(),
        meshes.cutout_cull.vertices.len()
    );
    assert_eq!(
        meshes.outline_cull.indices.len(),
        meshes.cutout_cull.indices.len()
    );
    assert_eq!(
        meshes.outline_cull.cutout_faces,
        meshes.cutout_cull.cutout_faces
    );
    let outline_tint = [
        0x33 as f32 / 255.0,
        0x66 as f32 / 255.0,
        0xcc as f32 / 255.0,
        1.0,
    ];
    assert!(meshes
        .outline_cull
        .vertices
        .iter()
        .all(|vertex| vertex.tint == outline_tint));
}

#[test]
fn arrow_missing_texture_preserves_other_geometry() {
    let base_len =
        usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![0u8; base_len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance =
        EntityModelInstance::arrow(60, [0.0, 64.0, 0.0], 35.0, ArrowModelTexture::Spectral)
            .with(|s| (s.head_yaw, s.head_pitch) = (0.0, -12.0))
            .with(|s| {
                s.light_coords = (4_u32 << 4) | (12_u32 << 20);
                s.white_overlay_progress = 0.8;
                s.has_red_overlay = true;
            });

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}
