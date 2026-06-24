use super::*;

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
fn arrow_textured_render_matches_vanilla_renderer() {
    // One model, three images: normal / tipped (`getColor() > 0`) / spectral (the distinct entity).
    for (texture, texture_ref) in [
        (ArrowModelTexture::Normal, ARROW_TEXTURE_REF),
        (ArrowModelTexture::Tipped, ARROW_TIPPED_TEXTURE_REF),
        (ArrowModelTexture::Spectral, ARROW_SPECTRAL_TEXTURE_REF),
    ] {
        assert_eq!(arrow_textured_layer_passes(texture)[0].texture, texture_ref);
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
    let mesh = entity_model_textured_mesh(
        &[EntityModelInstance::arrow(
            60,
            [0.0, 64.0, 0.0],
            0.0,
            ArrowModelTexture::Tipped,
        )],
        &atlas,
    );
    assert!(!mesh.vertices.is_empty());
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
}
