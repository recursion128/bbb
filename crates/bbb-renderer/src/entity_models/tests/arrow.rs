use super::*;

use crate::entity_models::colored::arrow_model_root_transform;
use crate::entity_models::model::EntityModel;

#[test]
fn arrow_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `ArrowModel.createBodyLayer` (atlas 32×32): the `back` arrowhead plane plus the two
    // crossed fletching planes, now carried as a unified `ArrowModel` tree with textured UVs.

    // `back`: the 0×5×5 plane at offset (-11, 0, 0), pitched π/4, with `withScale(0.8)` baked → 0×4×4.
    assert_eq!(ARROW_BACK_POSE.offset, [-11.0, 0.0, 0.0]);
    assert_eq!(
        ARROW_BACK_POSE.rotation,
        [std::f32::consts::FRAC_PI_4, 0.0, 0.0]
    );
    assert!((ARROW_BACK_CUBE.size[1] - 5.0 * 0.8).abs() < 1.0e-6);
    assert_eq!(ARROW_BACK_CUBE.min, [0.0, -2.0, -2.0]);
    assert_eq!(ARROW_BACK_CUBE.size, [0.0, 4.0, 4.0]);
    // UV: texOffs(0,0), the integer addBox dims [0,5,5].
    assert_eq!(ARROW_BACK_CUBE.tex, [0.0, 0.0]);
    assert_eq!(ARROW_BACK_CUBE.uv_size, [0.0, 5.0, 5.0]);

    // `cross_1` / `cross_2`: the shared 16×4×0 plane at pitches π/4 and 3π/4.
    assert_eq!(ARROW_CROSS_1_POSE.offset, [0.0, 0.0, 0.0]);
    assert_eq!(
        ARROW_CROSS_1_POSE.rotation,
        [std::f32::consts::FRAC_PI_4, 0.0, 0.0]
    );
    assert_eq!(
        ARROW_CROSS_2_POSE.rotation,
        [3.0 * std::f32::consts::FRAC_PI_4, 0.0, 0.0]
    );
    assert_eq!(ARROW_CROSS_CUBE.min, [-12.0, -2.0, 0.0]);
    assert_eq!(ARROW_CROSS_CUBE.size, [16.0, 4.0, 0.0]);
    // UV: texOffs(0,0); the vanilla `texScaleV = 0.8` is baked into the UV height (4 × 0.8 = 3.2).
    assert_eq!(ARROW_CROSS_CUBE.tex, [0.0, 0.0]);
    assert_eq!(ARROW_CROSS_CUBE.uv_size, [16.0, 3.2, 0.0]);
}

#[test]
fn arrow_mesh_uses_vanilla_body_layer_geometry() {
    // 3 planes → 18 faces / 72 vertices / 108 indices; the shaft cross and the head carry their tints.
    let arrow = entity_model_mesh(&[EntityModelInstance::arrow(
        60,
        [0.0, 64.0, 0.0],
        0.0,
        ArrowModelTexture::Normal,
    )]);
    assert_eq!(arrow.opaque_faces, 18);
    assert_eq!(arrow.vertices.len(), 72);
    assert_eq!(arrow.indices.len(), 108);
    assert!(arrow
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ARROW_SHAFT, 1.0)));
    assert!(arrow
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(ARROW_HEAD, 1.0)));
}

#[test]
fn arrow_impact_shake_rolls_root_like_vanilla_setup_anim() {
    let base = EntityModelInstance::arrow(60, [0.0, 64.0, 0.0], 0.0, ArrowModelTexture::Normal);
    let mut model = ArrowModel::new();

    model.prepare(&base);
    assert_eq!(model.root().pose.rotation[2], 0.0);

    let shaken = base.with_arrow_shake(4.5);
    model.prepare(&shaken);
    let expected = arrow_shake_z_rot(4.5);
    assert!(
        (model.root().pose.rotation[2] - expected).abs() < 1.0e-6,
        "root zRot expected {expected}, got {}",
        model.root().pose.rotation[2]
    );

    model.prepare(&base);
    assert_eq!(
        model.root().pose.rotation[2],
        0.0,
        "prepare must reset the previous frame's shake pose"
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
        assert_eq!(pass.kind, EntityModelLayerKind::ArrowBase);
        assert_eq!(pass.model_layer, MODEL_LAYER_ARROW);
        assert_eq!(pass.texture, texture_ref);
        assert_eq!(
            pass.render_type,
            EntityModelLayerRenderType::EntityCutoutCull
        );
        assert_eq!(pass.render_type.vanilla_name(), "entityCutoutCull");
        assert_eq!(pass.visibility, EntityModelLayerVisibility::All);
        assert_eq!(pass.tint, [1.0; 4]);
        assert_eq!(pass.order, 0);
        assert_eq!(pass.submit_sequence, 0);
        assert_eq!(
            EntityModelKind::Arrow { texture }.vanilla_texture_ref(),
            Some(texture_ref)
        );
        assert!(entity_model_texture_refs().contains(&texture_ref));
    }
    assert_eq!(
        arrow_entity_texture_refs(),
        &[
            ARROW_TEXTURE_REF,
            ARROW_TIPPED_TEXTURE_REF,
            ARROW_SPECTRAL_TEXTURE_REF
        ]
    );

    let len = usize::try_from(ARROW_TEXTURE_REF.size[0] * ARROW_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ARROW_TIPPED_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance =
        EntityModelInstance::arrow(60, [0.0, 64.0, 0.0], 35.0, ArrowModelTexture::Tipped)
            .with_head_look(0.0, -12.0)
            .with_light_coords((4_u32 << 4) | (12_u32 << 20))
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true);
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
    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.texture, ARROW_TIPPED_TEXTURE_REF);
    assert_eq!(
        submit.render_type,
        EntityModelLayerRenderType::EntityCutoutCull
    );
    assert_eq!(submit.render_type.vanilla_name(), "entityCutoutCull");
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.order, 0);
    assert_eq!(submit.submit_sequence, 0);
    assert_eq!(submit.transform, arrow_model_root_transform(instance));
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, [0.0, 10.0]);
    assert_ne!(submit.overlay, instance.render_state.overlay_coords());
    assert!(meshes
        .cutout_cull
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));

    let shaken_meshes = entity_model_textured_meshes(&[instance.with_arrow_shake(4.5)], &atlas);
    assert_eq!(shaken_meshes.submissions[0].transform, submit.transform);
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

    let images: Vec<EntityModelTextureImage> = arrow_entity_texture_refs()
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

        assert!(meshes.cutout.vertices.is_empty());
        assert!(meshes.cutout_cull.vertices.is_empty());
        assert!(
            !meshes.translucent.vertices.is_empty(),
            "{kind:?} emits textured translucent geometry"
        );
        assert_eq!(meshes.submissions.len(), 1);
        let submit = meshes.submissions[0];
        assert_eq!(
            submit.render_type,
            EntityModelLayerRenderType::EntityTranslucent
        );
        assert_eq!(submit.render_type.vanilla_name(), "entityTranslucent");
        assert_eq!(submit.texture, texture_ref);
        assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(submit.transform, transform);
        assert_eq!(submit.light, [0.4, 0.8]);
        assert_eq!(submit.overlay, ENTITY_VERTEX_NO_OVERLAY);
        assert_eq!((submit.order, submit.submit_sequence), (0, 0));
        assert!(meshes.translucent.vertices.iter().all(|vertex| {
            vertex.tint == submit.tint
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
    let submit = meshes.submissions[0];

    assert_eq!(
        submit.render_type,
        EntityModelLayerRenderType::EntityCutoutCull
    );
    assert!(submit.render_type.outline_cull());
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
fn arrow_submission_survives_missing_texture_atlas_entry() {
    // Vanilla `ArrowRenderer.submit` records the `entityCutoutCull` submit before atlas lookup;
    // missing texture data suppresses only the folded geometry.
    let base_len =
        usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![0u8; base_len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let instance =
        EntityModelInstance::arrow(60, [0.0, 64.0, 0.0], 35.0, ArrowModelTexture::Spectral)
            .with_head_look(0.0, -12.0)
            .with_light_coords((4_u32 << 4) | (12_u32 << 20))
            .with_white_overlay_progress(0.8)
            .with_has_red_overlay(true);

    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.texture, ARROW_SPECTRAL_TEXTURE_REF);
    assert_eq!(
        submit.render_type,
        EntityModelLayerRenderType::EntityCutoutCull
    );
    assert_eq!(submit.render_type.vanilla_name(), "entityCutoutCull");
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(submit.transform, arrow_model_root_transform(instance));
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert_eq!(submit.light, instance.render_state.shader_light());
    assert_eq!(submit.overlay, [0.0, 10.0]);
    assert!(meshes.cutout.vertices.is_empty());
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
}
