use super::*;

#[test]
fn wind_charge_spin_matches_vanilla_setup_anim() {
    // `wind.yRot = age·16°·π/180` (set, overwriting the -π/4 bind); `windCharge.yRot = -that`.
    let age = 5.0_f32;
    let expected = (age * 16.0).to_radians();
    assert!((wind_charge_spin_yrot(age) - expected).abs() < 1.0e-6);
    assert_eq!(wind_charge_spin_yrot(0.0), 0.0);
}

#[test]
fn wind_charge_textured_render_matches_vanilla_renderer() {
    assert!(entity_model_texture_refs().contains(&WIND_CHARGE_TEXTURE_REF));
    let passes = wind_charge_textured_layer_passes();
    assert_eq!(passes.len(), 1);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::BreezeWind
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "breezeWind");
    assert_eq!(passes[0].texture, WIND_CHARGE_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);

    let len =
        usize::try_from(WIND_CHARGE_TEXTURE_REF.size[0] * WIND_CHARGE_TEXTURE_REF.size[1] * 4)
            .unwrap();
    let images = vec![EntityModelTextureImage::new(
        WIND_CHARGE_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    // Vanilla `WindChargeRenderer` draws the whole model with the scrolling `breezeWind` render type,
    // so the wind charge emits no cutout/eyes geometry — only the scroll mesh (3 cubes → 72 vertices).
    let instance = EntityModelInstance::wind_charge(180, [0.0, 64.0, 0.0], 0.0).with(|s| {
        s.light_coords = (3_u32 << 4) | (13_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let rest = entity_model_textured_meshes(&[instance], &atlas);

    assert!(rest.cutout.vertices.is_empty(), "no cutout pass");
    assert!(rest.eyes.vertices.is_empty(), "no eyes pass");
    assert_eq!(rest.scroll.vertices.len(), 72);
    assert!(rest
        .scroll
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    assert!(rest.scroll.vertices.iter().all(|vertex| {
        vertex.light == instance.render_state.shader_light() && vertex.overlay == [0.0, 10.0]
    }));
    // Every scroll vertex carries the wind charge texture's atlas sub-rect for the shader's wrap.
    let rect_min = rest.scroll.vertices[0].uv_rect_min;
    let rect_size = rest.scroll.vertices[0].uv_rect_size;
    assert!(rect_size[0] > 0.0 && rect_size[1] > 0.0);
    assert!(rest
        .scroll
        .vertices
        .iter()
        .all(|vertex| vertex.uv_rect_min == rect_min && vertex.uv_rect_size == rect_size));

    // Vanilla `WindChargeRenderer.xOffset(t) = t · 0.03`, taken `% 1.0`: advancing `ageInTicks` scrolls
    // every vertex's local U by that amount (V fixed). The local UV derives from `texOffs`, so it is
    // independent of the age-driven spin — vertex `i` keeps the same base UV at both ages.
    let age = 10.0_f32;
    let scrolled = entity_model_textured_meshes(
        &[EntityModelInstance::wind_charge(180, [0.0, 64.0, 0.0], 0.0)
            .with(|s| s.age_in_ticks = age)],
        &atlas,
    );
    let expected_offset = (age * 0.03).rem_euclid(1.0);
    assert!(expected_offset > 0.0);
    for (rest_vertex, scrolled_vertex) in rest.scroll.vertices.iter().zip(&scrolled.scroll.vertices)
    {
        assert!(
            (scrolled_vertex.local_uv[0] - (rest_vertex.local_uv[0] + expected_offset)).abs()
                < 1.0e-6,
            "the U coordinate scrolls by (age · 0.03) % 1"
        );
        assert_eq!(
            scrolled_vertex.local_uv[1], rest_vertex.local_uv[1],
            "only U scrolls; V is fixed"
        );
    }
}

#[test]
fn wind_charge_breeze_wind_missing_texture_preserves_other_geometry() {
    let base_len =
        usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![0u8; base_len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    let instance = EntityModelInstance::wind_charge(183, [2.0, 65.0, -4.0], 30.0);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert!(
        meshes.scroll.vertices.is_empty(),
        "missing wind_charge.png suppresses only folded scroll geometry"
    );
}
