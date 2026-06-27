use super::*;

#[test]
fn wind_charge_geometry_matches_vanilla_26_1_body_layer() {
    // Vanilla `WindChargeModel.createBodyLayer` (atlas 64×32): the `bone` root (no cubes) parents the
    // `wind` shell (a fixed `yRot = -0.7854` ≈ -π/4, two boxes) and the `wind_charge` core box.

    // `wind`: the -π/4 bind rotation plus the `texOffs(15, 20)` 8×2×8 and `texOffs(0, 9)` 6×4×6 boxes.
    assert_eq!(WIND_CHARGE_WIND_POSE.rotation[0], 0.0);
    assert!((WIND_CHARGE_WIND_POSE.rotation[1] - (-std::f32::consts::FRAC_PI_4)).abs() < 1.0e-4);
    assert_eq!(WIND_CHARGE_WIND_POSE.rotation[2], 0.0);
    assert_eq!(WIND_CHARGE_WIND_CUBES.len(), 2);
    assert_eq!(WIND_CHARGE_WIND_CUBES[0].min, [-4.0, -1.0, -4.0]);
    assert_eq!(WIND_CHARGE_WIND_CUBES[0].size, [8.0, 2.0, 8.0]);
    assert_eq!(WIND_CHARGE_WIND_CUBES[0].tex, [15.0, 20.0]);
    assert_eq!(WIND_CHARGE_WIND_CUBES[1].min, [-3.0, -2.0, -3.0]);
    assert_eq!(WIND_CHARGE_WIND_CUBES[1].size, [6.0, 4.0, 6.0]);
    assert_eq!(WIND_CHARGE_WIND_CUBES[1].tex, [0.0, 9.0]);

    // `wind_charge`: the 4×4×4 core box at ZERO with no rotation, `texOffs(0, 0)`.
    assert_eq!(WIND_CHARGE_CORE_CUBES.len(), 1);
    assert_eq!(WIND_CHARGE_CORE_CUBES[0].min, [-2.0, -2.0, -2.0]);
    assert_eq!(WIND_CHARGE_CORE_CUBES[0].size, [4.0, 4.0, 4.0]);
    assert_eq!(WIND_CHARGE_CORE_CUBES[0].tex, [0.0, 0.0]);
}

#[test]
fn wind_charge_mesh_uses_vanilla_body_layer_geometry() {
    // 3 cubes → 18 faces / 72 vertices / 108 indices; the wind shell and the core carry their tints.
    let charge = entity_model_mesh(&[EntityModelInstance::wind_charge(180, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(charge.opaque_faces, 18);
    assert_eq!(charge.vertices.len(), 72);
    assert_eq!(charge.indices.len(), 108);
    assert!(charge
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WIND_CHARGE_WIND, 1.0)));
    assert!(charge
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(WIND_CHARGE_CORE, 1.0)));
}

#[test]
fn wind_charge_spin_matches_vanilla_setup_anim() {
    // `wind.yRot = age·16°·π/180` (set, overwriting the -π/4 bind); `windCharge.yRot = -that`.
    let age = 5.0_f32;
    let expected = (age * 16.0).to_radians();
    assert!((wind_charge_spin_yrot(age) - expected).abs() < 1.0e-6);
    assert_eq!(wind_charge_spin_yrot(0.0), 0.0);
}

#[test]
fn wind_charge_counter_spins_shell_and_core_with_age() {
    // Vanilla `WindChargeModel.setupAnim` sets `wind.yRot = age·16°` (overwriting the -π/4 bind) and
    // `windCharge.yRot = -age·16°`. So at age 0 the shell sits axis-aligned: its 8×2×8 slab
    // half-extent is 0.25 block, so no vertex passes |x| = 0.30. At age 2.8125 the shell turns 45°,
    // mapping its corner (0.25, 0.25) to (0.354, 0) — reaching ~0.354 on X.
    let rest = entity_model_mesh(&[EntityModelInstance::wind_charge(181, [0.0, 0.0, 0.0], 0.0)]);
    assert!(
        !rest.vertices.iter().any(|v| v.position[0].abs() > 0.30),
        "at age 0 the spin is zero, so the shell sits axis-aligned (the -π/4 bind is overwritten)"
    );

    let age = 45.0_f32 / 16.0; // age·16° = 45°
    let spun = entity_model_mesh(&[
        EntityModelInstance::wind_charge(182, [0.0, 0.0, 0.0], 0.0).with_age_in_ticks(age)
    ]);
    assert!(
        spun.vertices.iter().any(|v| v.position[0].abs() > 0.30),
        "the age-driven spin swings the shell off-axis"
    );
    assert_eq!(rest.vertices.len(), spun.vertices.len());
    assert_ne!(
        rest.vertices, spun.vertices,
        "both the shell and the counter-spun core turn with age"
    );
}

#[test]
fn wind_charge_textured_render_matches_vanilla_renderer() {
    assert_eq!(
        EntityModelKind::WindCharge.vanilla_texture_ref(),
        Some(WIND_CHARGE_TEXTURE_REF)
    );
    assert!(entity_model_texture_refs().contains(&WIND_CHARGE_TEXTURE_REF));
    assert_eq!(
        wind_charge_entity_texture_refs(),
        &[WIND_CHARGE_TEXTURE_REF]
    );

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
    let instance = EntityModelInstance::wind_charge(180, [0.0, 64.0, 0.0], 0.0)
        .with_light_coords((3_u32 << 4) | (13_u32 << 20))
        .with_white_overlay_progress(0.8)
        .with_has_red_overlay(true);
    let rest = entity_model_textured_meshes(&[instance], &atlas);
    assert_eq!(rest.submissions.len(), 1);
    assert_eq!(
        rest.submissions[0].render_type,
        EntityModelLayerRenderType::BreezeWind
    );
    assert_eq!(rest.submissions[0].texture, WIND_CHARGE_TEXTURE_REF);
    assert_eq!(rest.submissions[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(rest.submissions[0].order, 0);
    assert_eq!(rest.submissions[0].submit_sequence, 0);
    assert_eq!(
        rest.submissions[0].light,
        instance.render_state.shader_light()
    );
    assert_eq!(rest.submissions[0].overlay, [0.0, 10.0]);
    assert_ne!(
        rest.submissions[0].overlay,
        instance.render_state.overlay_coords()
    );
    assert_eq!(
        rest.submissions[0].transform,
        wind_charge_model_root_transform(instance)
    );
    assert!(rest.cutout.vertices.is_empty(), "no cutout pass");
    assert!(rest.eyes.vertices.is_empty(), "no eyes pass");
    assert_eq!(rest.scroll.vertices.len(), 72);
    assert!(rest
        .scroll
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
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
        &[EntityModelInstance::wind_charge(180, [0.0, 64.0, 0.0], 0.0).with_age_in_ticks(age)],
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
fn wind_charge_breeze_wind_submission_survives_missing_texture_atlas_entry() {
    // Residual scroll emits are submission-first: the vanilla `breezeWind(wind_charge.png)` submit is
    // recorded before atlas lookup, and missing texture data suppresses only the folded scroll mesh.
    let base_len =
        usize::try_from(ZOMBIE_TEXTURE_REF.size[0] * ZOMBIE_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        ZOMBIE_TEXTURE_REF,
        vec![0u8; base_len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    let instance = EntityModelInstance::wind_charge(183, [2.0, 65.0, -4.0], 30.0);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_eq!(meshes.submissions.len(), 1);
    let submit = meshes.submissions[0];
    assert_eq!(submit.render_type, EntityModelLayerRenderType::BreezeWind);
    assert_eq!(submit.texture, WIND_CHARGE_TEXTURE_REF);
    assert_eq!(submit.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!((submit.order, submit.submit_sequence), (0, 0));
    assert_eq!(submit.transform, wind_charge_model_root_transform(instance));
    assert!(
        meshes.scroll.vertices.is_empty(),
        "missing wind_charge.png suppresses only folded scroll geometry"
    );
}
