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
        assert_eq!(arrow_textured_layer_passes(texture)[0].texture, texture_ref);
        assert_eq!(
            arrow_textured_layer_passes(texture)[0].render_type,
            EntityModelLayerRenderType::EntityCutoutCull
        );
        assert_eq!(
            arrow_textured_layer_passes(texture)[0]
                .render_type
                .vanilla_name(),
            "entityCutoutCull"
        );
        assert_eq!(arrow_textured_layer_passes(texture)[0].tint, [1.0; 4]);
        assert_eq!(arrow_textured_layer_passes(texture)[0].order, 0);
        assert_eq!(arrow_textured_layer_passes(texture)[0].submit_sequence, 0);
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
    assert!(!meshes.cutout.vertices.is_empty());
    assert!(meshes
        .cutout
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
        .cutout
        .vertices
        .iter()
        .all(|vertex| vertex.light == submit.light && vertex.overlay == submit.overlay));

    let shaken_meshes = entity_model_textured_meshes(&[instance.with_arrow_shake(4.5)], &atlas);
    assert_eq!(shaken_meshes.submissions[0].transform, submit.transform);
    assert_eq!(
        shaken_meshes.cutout.vertices.len(),
        meshes.cutout.vertices.len()
    );
    assert!(
        meshes
            .cutout
            .vertices
            .iter()
            .zip(&shaken_meshes.cutout.vertices)
            .any(|(base, shaken)| base.position != shaken.position),
        "impact shake should pose arrow geometry without changing the renderer root transform"
    );
}
