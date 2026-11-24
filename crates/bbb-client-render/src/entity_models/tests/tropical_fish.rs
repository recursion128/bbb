use super::*;

#[test]
fn tropical_fish_tail_sway_matches_vanilla_setup_anim() {
    // Both layers: `tail.yRot = -amplitude * 0.45 * sin(0.6 * ageInTicks)`, amplitude 1.0
    // in water / 1.5 out (identical to `CodModel.setupAnim`). Zero at age 0.
    assert_eq!(tropical_fish_tail_yrot(0.0, true), 0.0);
    assert_eq!(tropical_fish_tail_yrot(0.0, false), 0.0);

    let age = 5.0_f32;
    let s = (0.6 * age).sin();
    assert!((tropical_fish_tail_yrot(age, true) - (-0.45 * s)).abs() < 1.0e-6);
    assert!((tropical_fish_tail_yrot(age, false) - (-1.5 * 0.45 * s)).abs() < 1.0e-6);
    // The beached fish thrashes harder.
    assert!(tropical_fish_tail_yrot(age, false).abs() > tropical_fish_tail_yrot(age, true).abs());
    // It matches cod's tail sway exactly.
    assert_eq!(
        tropical_fish_tail_yrot(age, true),
        cod_tail_fin_yrot(age, true)
    );
}

#[test]
fn tropical_fish_shape_from_vanilla_base_id() {
    // `TropicalFish.Base` ids: SMALL(0) / LARGE(1); anything else decodes to small.
    assert_eq!(
        TropicalFishModelShape::from_vanilla_base_id(0),
        TropicalFishModelShape::Small
    );
    assert_eq!(
        TropicalFishModelShape::from_vanilla_base_id(1),
        TropicalFishModelShape::Large
    );
    assert_eq!(
        TropicalFishModelShape::from_vanilla_base_id(7),
        TropicalFishModelShape::Small
    );
}

#[test]
fn tropical_fish_shape_from_vanilla_pattern_id() {
    // Pattern ids are packed as `base.id | index << 8`. The default KOB id `0` is small.
    assert_eq!(
        TropicalFishModelShape::from_vanilla_pattern_id(0),
        TropicalFishModelShape::Small
    );
    // FLOPPER = LARGE base (id 1), index 0 → packed pattern `0x0001`.
    assert_eq!(
        TropicalFishModelShape::from_vanilla_pattern_id(0x0001),
        TropicalFishModelShape::Large
    );
    // CLAYFISH = LARGE base, index 5 → `0x0501`; BETTY index 4 → `0x0401`.
    assert_eq!(
        TropicalFishModelShape::from_vanilla_pattern_id(0x0501),
        TropicalFishModelShape::Large
    );
    // SPOTTY = SMALL base, index 5 → `0x0500` stays small.
    assert_eq!(
        TropicalFishModelShape::from_vanilla_pattern_id(0x0500),
        TropicalFishModelShape::Small
    );
    // An out-of-range index (6) is not a valid pattern, so `byId` falls back to KOB (small).
    assert_eq!(
        TropicalFishModelShape::from_vanilla_pattern_id(0x0601),
        TropicalFishModelShape::Small
    );
}

#[test]
fn tropical_fish_textured_layer_passes_match_vanilla_renderer() {
    // Each body shape renders a cutout base layer (`#main`) plus the `TropicalFishPatternLayer`
    // overlay (`#pattern`, the body mesh inflated by `FISH_PATTERN_DEFORMATION`). The base is
    // tinted by `getModelTint = state.baseColor`, the overlay by `state.patternColor`. The vestigial
    // `parts` slices are nulled — emit builds the unified model and renders its tree.
    for (shape, texture, pattern, pattern_texture) in [
        (
            TropicalFishModelShape::Small,
            TROPICAL_FISH_SMALL_TEXTURE_REF,
            TropicalFishPattern::Brinely,
            TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
        ),
        (
            TropicalFishModelShape::Large,
            TROPICAL_FISH_LARGE_TEXTURE_REF,
            TropicalFishPattern::Betty,
            TROPICAL_FISH_BETTY_PATTERN_TEXTURE_REF,
        ),
    ] {
        let passes = tropical_fish_textured_layer_passes(
            shape,
            EntityDyeColor::Orange,
            pattern,
            EntityDyeColor::Cyan,
        );
        assert_eq!(passes.len(), 2);

        assert_eq!(
            passes[0].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[0].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[0].texture, texture);
        // `getModelTint = state.baseColor`: the base layer is tinted by the base dye's diffuse
        // color, not left white.
        assert_eq!(
            passes[0].tint,
            EntityDyeColor::Orange.texture_diffuse_color()
        );
        assert_eq!(passes[0].order, 0);

        assert_eq!(
            passes[1].render_type,
            EntityModelLayerRenderType::EntityCutout
        );
        assert_eq!(passes[1].render_type.vanilla_name(), "entityCutout");
        assert_eq!(passes[1].texture, pattern_texture);
        // The overlay is tinted by the pattern dye's diffuse color and drawn after the base.
        assert_eq!(passes[1].tint, EntityDyeColor::Cyan.texture_diffuse_color());
        assert_eq!(passes[1].order, 1);
    }
}

#[test]
fn tropical_fish_textured_mesh_uses_vanilla_geometry_and_animates() {
    let (atlas, _) = build_entity_model_texture_atlas(&tropical_fish_texture_images()).unwrap();

    // Each shape now emits the base body plus the inflated pattern overlay (same cube counts),
    // so the textured vertex count doubles. Small (kob): 2×5 cubes → 240 vertices; large
    // (flopper): 2×6 → 288.
    let small = EntityModelInstance::tropical_fish(
        810,
        [0.0, 64.0, 0.0],
        0.0,
        TropicalFishModelShape::Small,
        EntityDyeColor::Orange,
        TropicalFishPattern::Brinely,
        EntityDyeColor::Cyan,
    )
    .with(|s| {
        s.in_water = true;
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let small_still = entity_model_textured_meshes(&[small], &atlas);
    assert_tropical_fish_submission_pair(
        &small_still,
        small,
        EntityDyeColor::Orange.texture_diffuse_color(),
        EntityDyeColor::Cyan.texture_diffuse_color(),
    );
    assert_ne!(small.render_state.overlay_coords(), [0.0, 10.0]);
    assert_eq!(small_still.cutout.vertices.len(), 240);

    let large = EntityModelInstance::tropical_fish(
        811,
        [0.0, 64.0, 0.0],
        0.0,
        TropicalFishModelShape::Large,
        EntityDyeColor::Orange,
        TropicalFishPattern::Betty,
        EntityDyeColor::Cyan,
    )
    .with(|s| {
        s.in_water = true;
        s.light_coords = (6_u32 << 4) | (12_u32 << 20);
        s.white_overlay_progress = 0.8;
        s.has_red_overlay = true;
    });
    let large_still = entity_model_textured_meshes(&[large], &atlas);
    assert_tropical_fish_submission_pair(
        &large_still,
        large,
        EntityDyeColor::Orange.texture_diffuse_color(),
        EntityDyeColor::Cyan.texture_diffuse_color(),
    );
    assert_eq!(large_still.cutout.vertices.len(), 288);

    // The tail sway / body wiggle reorient the mesh as the age advances.
    let swimming_instance = small.with(|s| s.age_in_ticks = 7.0);
    let swimming = entity_model_textured_meshes(&[swimming_instance], &atlas);
    assert_tropical_fish_submission_pair(
        &swimming,
        swimming_instance,
        EntityDyeColor::Orange.texture_diffuse_color(),
        EntityDyeColor::Cyan.texture_diffuse_color(),
    );
    assert_eq!(
        small_still.cutout.vertices.len(),
        swimming.cutout.vertices.len()
    );
    assert_ne!(small_still.cutout.vertices, swimming.cutout.vertices);

    // A beached fish flops onto its side.
    let beached_instance = small.with(|s| s.in_water = false);
    let beached = entity_model_textured_meshes(&[beached_instance], &atlas);
    assert_tropical_fish_submission_pair(
        &beached,
        beached_instance,
        EntityDyeColor::Orange.texture_diffuse_color(),
        EntityDyeColor::Cyan.texture_diffuse_color(),
    );
    assert_ne!(small_still.cutout.vertices, beached.cutout.vertices);
}

#[test]
fn tropical_fish_pattern_missing_texture_preserves_other_geometry() {
    // Vanilla `TropicalFishPatternLayer` uses the shared colored-cutout copy helper at order(1);
    // missing atlas data suppresses only the folded pattern geometry.
    let images = [
        TROPICAL_FISH_SMALL_TEXTURE_REF,
        TROPICAL_FISH_LARGE_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect::<Vec<_>>();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    for (id, shape, base_texture, pattern, pattern_texture) in [
        (
            812,
            TropicalFishModelShape::Small,
            TROPICAL_FISH_SMALL_TEXTURE_REF,
            TropicalFishPattern::Brinely,
            TROPICAL_FISH_BRINELY_PATTERN_TEXTURE_REF,
        ),
        (
            813,
            TropicalFishModelShape::Large,
            TROPICAL_FISH_LARGE_TEXTURE_REF,
            TropicalFishPattern::Betty,
            TROPICAL_FISH_BETTY_PATTERN_TEXTURE_REF,
        ),
    ] {
        let instance = EntityModelInstance::tropical_fish(
            id,
            [0.0, 64.0, 0.0],
            0.0,
            shape,
            EntityDyeColor::Orange,
            pattern,
            EntityDyeColor::Cyan,
        )
        .with(|s| {
            s.in_water = true;
            s.light_coords = (6_u32 << 4) | (12_u32 << 20);
            s.white_overlay_progress = 0.8;
            s.has_red_overlay = true;
        });
        let meshes = entity_model_textured_meshes(&[instance], &atlas);

        let mut shifted_atlas = atlas.clone();
        let uv = &mut shifted_atlas
            .entries
            .iter_mut()
            .find(|entry| entry.texture == base_texture)
            .unwrap()
            .uv;
        uv.min[0] += 0.125;
        uv.max[0] += 0.125;
        uv.min[1] += 0.25;
        uv.max[1] += 0.25;
        let shifted = entity_model_textured_meshes(&[instance], &shifted_atlas);
        assert_eq!(meshes.cutout.vertices.len(), shifted.cutout.vertices.len());
        for (original, shifted) in meshes.cutout.vertices.iter().zip(&shifted.cutout.vertices) {
            assert!((shifted.uv[0] - original.uv[0] - 0.125).abs() < 1.0e-5);
            assert!((shifted.uv[1] - original.uv[1] - 0.25).abs() < 1.0e-5);
        }
        let base_tint = EntityDyeColor::Orange.texture_diffuse_color();

        let pattern_tint = EntityDyeColor::Cyan.texture_diffuse_color();

        assert!(
            !meshes.cutout.vertices.is_empty(),
            "missing {path} keeps the base mesh",
            path = pattern_texture.path
        );
        assert!(meshes
            .cutout
            .vertices
            .iter()
            .all(|vertex| vertex.tint == base_tint
                && vertex.light == instance.render_state.shader_light()
                && vertex.overlay == instance.render_state.overlay_coords()));
        assert!(!meshes
            .cutout
            .vertices
            .iter()
            .any(|vertex| vertex.tint == pattern_tint));
        assert!(meshes.translucent.vertices.is_empty());
        assert!(meshes.eyes.vertices.is_empty());
    }
}

fn assert_tropical_fish_submission_pair(
    meshes: &EntityModelTexturedMeshes,
    instance: EntityModelInstance,
    base_tint: [f32; 4],
    pattern_tint: [f32; 4],
) {
    assert!(meshes.translucent.vertices.is_empty());
    assert!(meshes.eyes.vertices.is_empty());

    assert!(meshes
        .cutout
        .vertices
        .iter()
        .any(|vertex| vertex.tint == base_tint));
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .filter(|vertex| vertex.tint == base_tint)
        .all(
            |vertex| vertex.light == instance.render_state.shader_light()
                && vertex.overlay == instance.render_state.overlay_coords()
        ));
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .any(|vertex| vertex.tint == pattern_tint));
    assert!(meshes
        .cutout
        .vertices
        .iter()
        .filter(|vertex| vertex.tint == pattern_tint)
        .all(
            |vertex| vertex.light == instance.render_state.shader_light()
                && vertex.overlay == [0.0, instance.render_state.overlay_coords()[1]]
        ));
}

fn tropical_fish_texture_images() -> Vec<EntityModelTextureImage> {
    texture_refs_matching(|path| path.starts_with("textures/entity/fish/tropical_"))
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

#[test]
fn tropical_fish_pattern_from_vanilla_pattern_id() {
    // `TropicalFish.Pattern.byId(patternId)`, keyed on `packedId = base.id | index << 8`
    // (`SMALL=0`/`LARGE=1`, index `0..=5`); unknown ids fall back to KOB.
    let expected = [
        TropicalFishPattern::Kob,
        TropicalFishPattern::Sunstreak,
        TropicalFishPattern::Snooper,
        TropicalFishPattern::Dasher,
        TropicalFishPattern::Brinely,
        TropicalFishPattern::Spotty,
        TropicalFishPattern::Flopper,
        TropicalFishPattern::Stripey,
        TropicalFishPattern::Glitter,
        TropicalFishPattern::Blockfish,
        TropicalFishPattern::Betty,
        TropicalFishPattern::Clayfish,
    ];
    for (descriptor, expected) in
        bbb_protocol::tropical_fish_patterns::VANILLA_TROPICAL_FISH_PATTERNS
            .iter()
            .zip(expected)
    {
        assert_eq!(
            TropicalFishPattern::from_vanilla_pattern_id(descriptor.packed_id),
            expected
        );
        assert_eq!(expected.pattern_index(), descriptor.pattern_index);
        assert_eq!(
            expected.shape(),
            TropicalFishModelShape::from_vanilla_base_id(descriptor.base_id)
        );
    }
    // Unknown low-byte base (0xAB) and out-of-range index (6 → 0x0601) both fall back to KOB.
    assert_eq!(
        TropicalFishPattern::from_vanilla_pattern_id(0x00AB),
        TropicalFishPattern::Kob
    );
    assert_eq!(
        TropicalFishPattern::from_vanilla_pattern_id(0x0601),
        TropicalFishPattern::Kob
    );
}

#[test]
fn tropical_fish_pattern_texture_refs_match_vanilla() {
    // Each pattern selects `tropical_{a,b}_pattern_{index + 1}.png`: the six small patterns ride
    // `tropical_a`, the six large `tropical_b`.
    let cases = [
        (
            TropicalFishPattern::Kob,
            "textures/entity/fish/tropical_a_pattern_1.png",
            TropicalFishModelShape::Small,
            0,
        ),
        (
            TropicalFishPattern::Sunstreak,
            "textures/entity/fish/tropical_a_pattern_2.png",
            TropicalFishModelShape::Small,
            1,
        ),
        (
            TropicalFishPattern::Snooper,
            "textures/entity/fish/tropical_a_pattern_3.png",
            TropicalFishModelShape::Small,
            2,
        ),
        (
            TropicalFishPattern::Dasher,
            "textures/entity/fish/tropical_a_pattern_4.png",
            TropicalFishModelShape::Small,
            3,
        ),
        (
            TropicalFishPattern::Brinely,
            "textures/entity/fish/tropical_a_pattern_5.png",
            TropicalFishModelShape::Small,
            4,
        ),
        (
            TropicalFishPattern::Spotty,
            "textures/entity/fish/tropical_a_pattern_6.png",
            TropicalFishModelShape::Small,
            5,
        ),
        (
            TropicalFishPattern::Flopper,
            "textures/entity/fish/tropical_b_pattern_1.png",
            TropicalFishModelShape::Large,
            0,
        ),
        (
            TropicalFishPattern::Stripey,
            "textures/entity/fish/tropical_b_pattern_2.png",
            TropicalFishModelShape::Large,
            1,
        ),
        (
            TropicalFishPattern::Glitter,
            "textures/entity/fish/tropical_b_pattern_3.png",
            TropicalFishModelShape::Large,
            2,
        ),
        (
            TropicalFishPattern::Blockfish,
            "textures/entity/fish/tropical_b_pattern_4.png",
            TropicalFishModelShape::Large,
            3,
        ),
        (
            TropicalFishPattern::Betty,
            "textures/entity/fish/tropical_b_pattern_5.png",
            TropicalFishModelShape::Large,
            4,
        ),
        (
            TropicalFishPattern::Clayfish,
            "textures/entity/fish/tropical_b_pattern_6.png",
            TropicalFishModelShape::Large,
            5,
        ),
    ];
    for (pattern, path, shape, index) in cases {
        let texture = tropical_fish_pattern_texture_ref(pattern);
        assert_eq!(texture.path, path);
        assert_eq!(texture.size, [32, 32]);
        assert_eq!(pattern.shape(), shape);
        assert_eq!(pattern.pattern_index(), index);
    }
}
