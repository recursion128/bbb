use super::*;

#[test]
fn breeze_texture_ref_matches_vanilla_renderer() {
    // Vanilla `BreezeEyesLayer`'s always-on emissive glow adds the `breeze_eyes.png`
    // entityTranslucentEmissive overlay, and `BreezeWindLayer` adds the 128×128
    // `breeze_wind.png` swirling wind body.
    assert!(entity_model_texture_refs().contains(&BREEZE_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&BREEZE_EYES_TEXTURE_REF));
    assert!(entity_model_texture_refs().contains(&BREEZE_WIND_TEXTURE_REF));
    let passes = breeze_textured_layer_passes();
    assert_eq!(passes.len(), 3);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityTranslucent
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityTranslucent");
    assert_eq!(passes[0].texture, BREEZE_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
    assert_eq!(
        passes[1].render_type,
        EntityModelLayerRenderType::BreezeWind
    );
    assert_eq!(passes[1].render_type.vanilla_name(), "breezeWind");
    assert_eq!(passes[1].texture, BREEZE_WIND_TEXTURE_REF);
    assert_eq!(passes[1].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[1].order, 1);
    assert_eq!(
        passes[2].render_type,
        EntityModelLayerRenderType::EntityTranslucentEmissive
    );
    assert_eq!(
        passes[2].render_type.vanilla_name(),
        "entityTranslucentEmissive"
    );
    assert_eq!(passes[2].texture, BREEZE_EYES_TEXTURE_REF);
    assert_eq!(passes[2].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[2].order, 1);
}

#[test]
fn breeze_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&breeze_texture_images()).unwrap();

    // The breeze base body draws into the translucent mesh (vanilla `RenderTypes::entityTranslucent`).
    // Head and its retained `eyes` (two cubes each) plus three rods → 7 cubes / 42 faces / 168
    // vertices, nothing on the cutout pass, white tint.
    let base = EntityModelInstance::breeze(960, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (5_u32 << 4) | (11_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[base], &atlas);

    assert!(meshes.cutout.vertices.is_empty());
    assert_eq!(meshes.translucent.cutout_faces, 42);
    assert_eq!(meshes.translucent.vertices.len(), 168);
    assert!(meshes
        .translucent
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]
            && vertex.light == base.render_state.shader_light()
            && vertex.overlay == base.render_state.overlay_coords()));

    // Vanilla `BreezeEyesLayer`'s always-on emissive glow renders the `createEyesLayer` model (only
    // the two `eyes` cubes) into entityTranslucentEmissive with `breeze_eyes.png`.

    assert_eq!(meshes.translucent_emissive.cutout_faces, 12);
    assert_eq!(meshes.translucent_emissive.vertices.len(), 48);
    assert!(meshes
        .translucent_emissive
        .vertices
        .iter()
        .all(|vertex| vertex.light == base.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]));

    // The looping IDLE re-poses both meshes with age and loops every 40 ticks.
    let later = entity_model_textured_meshes(&[base.with(|s| s.age_in_ticks = 7.0)], &atlas);
    assert_ne!(meshes.translucent.vertices, later.translucent.vertices);
    assert_ne!(
        meshes.translucent_emissive.vertices,
        later.translucent_emissive.vertices
    );
    let one_cycle = entity_model_textured_meshes(&[base.with(|s| s.age_in_ticks = 40.0)], &atlas);
    assert_eq!(meshes.translucent.vertices, one_cycle.translucent.vertices);
    assert_eq!(
        meshes.translucent_emissive.vertices,
        one_cycle.translucent_emissive.vertices
    );
}

#[test]
fn breeze_wind_body_folds_into_scrolling_overlay() {
    let (atlas, _) = build_entity_model_texture_atlas(&breeze_texture_images()).unwrap();

    // Vanilla `BreezeWindLayer` renders the separate `wind_body` shell chain with the translucent,
    // U-scrolling `breezeWind` render type. The wind body's 7 cubes (`wind_bottom` 1 + `wind_mid` 3 +
    // `wind_top` 3) → 42 faces / 168 vertices fold into the scroll mesh; nothing on the additive swirl.
    let base = EntityModelInstance::breeze(970, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (2_u32 << 4) | (14_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let meshes = entity_model_textured_meshes(&[base], &atlas);

    assert_eq!(meshes.scroll.vertices.len(), 168);
    assert_eq!(meshes.scroll.indices.len(), 42 * 6);
    assert!(meshes
        .scroll
        .vertices
        .iter()
        .all(|vertex| vertex.light == base.render_state.shader_light()
            && vertex.overlay == [0.0, 10.0]));
    assert!(meshes.scroll_additive.vertices.is_empty());

    // The wind body sways with the looping IDLE and its U coordinate scrolls by `ageInTicks · 0.02`;
    // age 7 differs from age 0 in both pose and scroll.
    let later = entity_model_textured_meshes(&[base.with(|s| s.age_in_ticks = 7.0)], &atlas);
    assert_ne!(meshes.scroll.vertices, later.scroll.vertices);

    // The IDLE pose loops every 40 ticks, but the U-scroll keeps advancing (`40 · 0.02 = 0.8 ≠ 0`), so
    // the wind body at age 40 carries the same pose phase yet a different scrolled local UV.
    let one_cycle = entity_model_textured_meshes(&[base.with(|s| s.age_in_ticks = 40.0)], &atlas);
    assert_ne!(
        meshes.scroll.vertices, one_cycle.scroll.vertices,
        "the wind body's U-scroll advances past the looped pose"
    );

    // Each pose-driven action also re-poses the wind shell chain (the `wind_*` channels of SHOOT etc.).
    let shooting = entity_model_textured_meshes(
        &[base.with(|s| s.kind.breeze_mut().shoot_seconds = 0.25)],
        &atlas,
    );
    assert_ne!(
        meshes.scroll.vertices, shooting.scroll.vertices,
        "the shoot action re-poses the wind body"
    );
}

#[test]
fn breeze_wind_missing_wind_texture_texture_preserves_other_geometry() {
    let images: Vec<_> = breeze_texture_images()
        .into_iter()
        .filter(|image| image.texture != BREEZE_WIND_TEXTURE_REF)
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let base = EntityModelInstance::breeze(971, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (2_u32 << 4) | (14_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[base], &atlas);

    assert!(!meshes.translucent.vertices.is_empty());

    assert!(
        meshes.scroll.vertices.is_empty(),
        "missing breeze_wind.png suppresses only folded wind-body geometry"
    );
    assert!(meshes.scroll.indices.is_empty());
}

#[test]
fn breeze_eyes_missing_eyes_texture_texture_preserves_other_geometry() {
    let images: Vec<_> = breeze_texture_images()
        .into_iter()
        .filter(|image| image.texture != BREEZE_EYES_TEXTURE_REF)
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let base = EntityModelInstance::breeze(972, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (3_u32 << 4) | (13_u32 << 20);
        s.white_overlay_progress = 0.75;
        s.has_red_overlay = true;
    });

    let meshes = entity_model_textured_meshes(&[base], &atlas);

    assert!(!meshes.translucent.vertices.is_empty());

    assert!(!meshes.scroll.vertices.is_empty());

    assert!(
        meshes.translucent_emissive.vertices.is_empty(),
        "missing breeze_eyes.png suppresses only folded entityTranslucentEmissive geometry"
    );
    assert!(meshes.translucent_emissive.indices.is_empty());
}

fn breeze_texture_images() -> Vec<EntityModelTextureImage> {
    [
        BREEZE_TEXTURE_REF,
        BREEZE_EYES_TEXTURE_REF,
        BREEZE_WIND_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}
