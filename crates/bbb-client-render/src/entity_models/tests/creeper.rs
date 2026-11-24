use super::*;

#[test]
fn creeper_textured_layer_passes_match_vanilla_renderer_model_layer() {
    let passes = creeper_textured_layer_passes();
    assert_eq!(passes.len(), 2);
    assert_eq!(
        passes[0].render_type,
        EntityModelLayerRenderType::EntityCutout
    );
    assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
    assert_eq!(passes[0].texture, CREEPER_TEXTURE_REF);
    assert_eq!(passes[0].tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(passes[0].order, 0);
    let grey = 128.0 / 255.0;
    assert_eq!(
        passes[1].render_type,
        EntityModelLayerRenderType::EnergySwirl
    );
    assert_eq!(passes[1].render_type.vanilla_name(), "energySwirl");
    assert_eq!(passes[1].texture, CREEPER_ARMOR_TEXTURE_REF);
    assert_eq!(passes[1].tint, [grey, grey, grey, 1.0]);
    assert_eq!(passes[1].order, 1);
}

#[test]
fn entity_texture_atlas_stitches_official_creeper_png_slot() {
    let (layout, _rgba) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();

    // The creeper now stitches two textures: the base creeper.png and the `CreeperPowerLayer`
    // energy-swirl creeper_armor.png (the charged-creeper overlay).
    assert_eq!(layout.entries.len(), 2);
    let paths: Vec<&str> = layout
        .entries
        .iter()
        .map(|entry| entry.texture.path)
        .collect();
    assert!(paths.contains(&"textures/entity/creeper/creeper.png"));
    assert!(paths.contains(&"textures/entity/creeper/creeper_armor.png"));
    // Each slot is the official 64×32 texture, so its uv sub-rect covers 64×32 texels of the atlas
    // (regardless of how the two are packed).
    for entry in &layout.entries {
        let width = (entry.uv.max[0] - entry.uv.min[0]) * layout.width as f32;
        let height = (entry.uv.max[1] - entry.uv.min[1]) * layout.height as f32;
        assert!((width - 64.0).abs() < 0.5, "slot is 64 texels wide");
        assert!((height - 32.0).abs() < 0.5, "slot is 32 texels tall");
    }
}

#[test]
fn charged_creeper_emits_scrolling_energy_swirl() {
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();

    // An uncharged creeper has no `CreeperPowerLayer`, so it emits no additive swirl geometry.
    let plain = entity_model_textured_meshes(
        &[EntityModelInstance::new(
            960,
            EntityModelKind::Creeper,
            [0.0, 64.0, 0.0],
            0.0,
        )],
        &atlas,
    );
    assert!(
        plain.scroll_additive.vertices.is_empty(),
        "no energy swirl when not powered"
    );

    assert_creeper_textured_buckets(
        &plain,
        EntityModelInstance::new(960, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0),
    );

    // A charged creeper draws the inflated `CREEPER_ARMOR` model (6 cubes → 144 vertices) into the
    // additive scroll mesh, every vertex tinted by the vanilla `0xFF808080` half-grey.
    let grey = 128.0 / 255.0;
    let powered = EntityModelInstance::new(961, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.kind.creeper_mut().powered = true);
    let rest = entity_model_textured_meshes(&[powered], &atlas);

    assert_creeper_textured_buckets(&rest, powered);

    assert_eq!(rest.scroll_additive.vertices.len(), 144);
    assert!(rest
        .scroll_additive
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [grey, grey, grey, 1.0]));
    assert!(rest.scroll_additive.vertices.iter().all(|vertex| {
        vertex.light == powered.render_state.shader_light() && vertex.overlay == [0.0, 10.0]
    }));

    // The inflated armor floats outside the body: its X half-extent (head 8→12 wide, so split ±6 →
    // 0.375 block) exceeds the base creeper's 0.25, confirming the `CubeDeformation(2.0)` inflate.
    assert!(rest
        .scroll_additive
        .vertices
        .iter()
        .any(|vertex| vertex.position[0].abs() > 0.3));

    // Vanilla `EnergySwirlLayer` scrolls both axes by `(ageInTicks · 0.01) % 1`; the creeper has no
    // age-driven body animation, so only the local UVs move.
    let age = 20.0_f32;
    let scrolled = entity_model_textured_meshes(
        &[
            EntityModelInstance::new(961, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0).with(
                |s| {
                    s.kind.creeper_mut().powered = true;
                    s.age_in_ticks = age;
                },
            ),
        ],
        &atlas,
    );
    let expected = (age * 0.01).rem_euclid(1.0);
    assert!(expected > 0.0);
    for (rest_vertex, scrolled_vertex) in rest
        .scroll_additive
        .vertices
        .iter()
        .zip(&scrolled.scroll_additive.vertices)
    {
        assert!(
            (scrolled_vertex.local_uv[0] - (rest_vertex.local_uv[0] + expected)).abs() < 1.0e-6
        );
        assert!(
            (scrolled_vertex.local_uv[1] - (rest_vertex.local_uv[1] + expected)).abs() < 1.0e-6
        );
    }
}

#[test]
fn charged_creeper_energy_swirl_missing_armor_texture_preserves_other_geometry() {
    // `EnergySwirlLayer` is dispatch-owned, but it must still produce its vanilla submission before
    // the backend tries to fold the scrolled geometry into the additive mesh bucket.
    let len =
        usize::try_from(CREEPER_TEXTURE_REF.size[0] * CREEPER_TEXTURE_REF.size[1] * 4).unwrap();
    let images = vec![EntityModelTextureImage::new(
        CREEPER_TEXTURE_REF,
        vec![0u8; len],
    )];
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    let light_coords = (5_u32 << 4) | (11_u32 << 20);
    let powered = EntityModelInstance::new(962, EntityModelKind::Creeper, [1.0, 64.0, 2.0], 45.0)
        .with(|s| {
            s.kind.creeper_mut().powered = true;
            s.light_coords = light_coords;
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });

    let meshes = entity_model_textured_meshes(&[powered], &atlas);

    assert_creeper_textured_buckets(&meshes, powered);
    assert_eq!(
        meshes.cutout.vertices.len(),
        144,
        "base creeper still renders from the available base texture"
    );
    assert!(
        meshes.scroll_additive.vertices.is_empty(),
        "missing creeper_armor.png suppresses only folded energy-swirl geometry"
    );
}

#[test]
fn creeper_textured_mesh_uses_vanilla_uvs_tints_and_body_layer_bounds() {
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let instance = EntityModelInstance::new(910, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let meshes = entity_model_textured_meshes(&[instance], &atlas);

    assert_creeper_textured_buckets(&meshes, instance);
    assert!(meshes.scroll_additive.vertices.is_empty());
    let mesh = &meshes.cutout;

    assert_eq!(mesh.cutout_faces, 36);
    assert_eq!(mesh.vertices.len(), 144);
    assert_eq!(mesh.indices.len(), 216);
    assert_close2(mesh.vertices[0].uv, [16.0 / 64.0, 0.0]);
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.tint == [1.0, 1.0, 1.0, 1.0]));
    let (min, max) = textured_mesh_extents(mesh);
    assert_close3(min, [-0.25, 64.001, -0.375]);
    assert_close3(max, [0.25, 65.626, 0.375]);
}

#[test]
fn creeper_textured_mesh_applies_head_look() {
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let base = EntityModelInstance::new(199, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let yawed_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (45.0, 0.0));
    let pitched_instance = base.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let yawed = entity_model_textured_meshes(&[yawed_instance], &atlas);
    let pitched = entity_model_textured_meshes(&[pitched_instance], &atlas);

    assert_creeper_textured_buckets(&resting, base);
    assert_creeper_textured_buckets(&yawed, yawed_instance);
    assert_creeper_textured_buckets(&pitched, pitched_instance);
    assert!(resting.scroll_additive.vertices.is_empty());
    assert!(yawed.scroll_additive.vertices.is_empty());
    assert!(pitched.scroll_additive.vertices.is_empty());
    assert_eq!(resting.cutout.vertices.len(), yawed.cutout.vertices.len());
    assert_ne!(resting.cutout.vertices, yawed.cutout.vertices);
    assert_ne!(yawed.cutout.vertices, pitched.cutout.vertices);
}

#[test]
fn creeper_textured_mesh_swings_legs_when_walking() {
    // The real creeper render path (texture-backed) swings the same `QuadrupedModel`
    // legs. A standing creeper is byte-identical however far the swing has advanced;
    // a walking one lifts its feet.
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let base = EntityModelInstance::new(251, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let still_instance = base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (2.5, 0.0));
    let walking_instance =
        base.with(|s| (s.walk_animation_pos, s.walk_animation_speed) = (0.0, 1.0));
    let resting = entity_model_textured_meshes(&[base], &atlas);
    let still = entity_model_textured_meshes(&[still_instance], &atlas);
    let walking = entity_model_textured_meshes(&[walking_instance], &atlas);

    assert_creeper_textured_buckets(&resting, base);
    assert_creeper_textured_buckets(&still, still_instance);
    assert_creeper_textured_buckets(&walking, walking_instance);
    assert!(resting.scroll_additive.vertices.is_empty());
    assert!(still.scroll_additive.vertices.is_empty());
    assert!(walking.scroll_additive.vertices.is_empty());

    assert_eq!(
        resting.cutout.vertices, still.cutout.vertices,
        "a standing creeper is inert"
    );
    assert_eq!(
        resting.cutout.vertices.len(),
        walking.cutout.vertices.len(),
        "leg swing keeps the vertex count"
    );
    assert_ne!(
        resting.cutout.vertices, walking.cutout.vertices,
        "a walking creeper differs"
    );

    let (rest_min, rest_max) = textured_mesh_extents(&resting.cutout);
    let (walk_min, walk_max) = textured_mesh_extents(&walking.cutout);
    assert!(
        (walk_max[1] - walk_min[1]) < (rest_max[1] - rest_min[1]) - 0.02,
        "a walking creeper's feet should lift off the ground"
    );
}

#[test]
fn creeper_textured_mesh_inflates_as_it_primes_to_explode() {
    // The real creeper render path (texture-backed) inflates with the same
    // `CreeperRenderer.scale` swell. An unswollen creeper is byte-identical; a fully primed
    // one grows on the X/Z plane by `(1 + 0.4) * wobble`.
    let (atlas, _) = build_entity_model_texture_atlas(&creeper_texture_images()).unwrap();
    let base = EntityModelInstance::new(253, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let unswollen_instance = base.with(|s| s.kind.creeper_mut().swelling = 0.0);
    let calm = entity_model_textured_meshes(&[base], &atlas);
    let unswollen = entity_model_textured_meshes(&[unswollen_instance], &atlas);

    assert_creeper_textured_buckets(&calm, base);
    assert_creeper_textured_buckets(&unswollen, unswollen_instance);
    assert!(calm.scroll_additive.vertices.is_empty());
    assert!(unswollen.scroll_additive.vertices.is_empty());
    assert_eq!(
        calm.cutout.vertices, unswollen.cutout.vertices,
        "an unswollen creeper is unscaled"
    );

    let swelling = 1.0_f32;
    let primed_instance = base.with(|s| s.kind.creeper_mut().swelling = swelling);
    let primed = entity_model_textured_meshes(&[primed_instance], &atlas);

    assert_creeper_textured_buckets(&primed, primed_instance);
    assert!(primed.scroll_additive.vertices.is_empty());
    assert_eq!(
        calm.cutout.vertices.len(),
        primed.cutout.vertices.len(),
        "the swell scale keeps the vertex count"
    );
    assert_ne!(
        calm.cutout.vertices, primed.cutout.vertices,
        "a priming creeper is rescaled"
    );

    let wobble = 1.0 + (swelling * 100.0).sin() * swelling * 0.01;
    let expected_s = 1.4 * wobble;
    let (calm_min, calm_max) = textured_mesh_extents(&calm.cutout);
    let (primed_min, primed_max) = textured_mesh_extents(&primed.cutout);
    let ratio = (primed_max[0] - primed_min[0]) / (calm_max[0] - calm_min[0]);
    assert!(
        (ratio - expected_s).abs() < 1.0e-4,
        "X extent ratio {ratio} should match swell factor {expected_s}"
    );
}

fn assert_creeper_textured_buckets(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_cutout.vertices.is_empty());
    assert!(meshes.dynamic_player_skin_translucent.vertices.is_empty());
    assert!(meshes.dynamic_player_texture_cutout.vertices.is_empty());
    assert!(meshes
        .dynamic_player_texture_translucent
        .vertices
        .is_empty());
    assert!(meshes.scroll.vertices.is_empty());

    assert!(meshes.cutout.vertices.iter().all(|vertex| vertex.light
        == instance.render_state.shader_light()
        && vertex.overlay == instance.render_state.overlay_coords()));
}

fn creeper_texture_images() -> Vec<EntityModelTextureImage> {
    [CREEPER_TEXTURE_REF, CREEPER_ARMOR_TEXTURE_REF]
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}
